//! The nav subcommand driver, moved unchanged from main.rs.
use anyhow::Context;
use prism_review::cli::{NavArgs, NavQuery};
use std::io::Write;

/// Parse `file:line` CLI seed specs into `SeedSpec::Loc`, delegating to
/// `reasoning::seeds::parse_file_line_spec` for the actual normalization and
/// minimum-line validation. This stays a small wrapper here (rather than
/// calling `mcp::input::parse_taint_reaches` directly) because the whole
/// `mcp` module lives behind the `mcp` feature, while `nav` subcommands (and
/// the `reasoning` layer they call into) are built by default with no
/// feature flag -- but `parse_file_line_spec` itself is the single
/// implementation MCP's loc-seed parser also calls into, so `./app.py:2` and
/// line `0` are handled identically on both paths (F3, P6bc review). See
/// CLAUDE.md's MCP Adapter section.
fn parse_loc_seeds(
    specs: &[String],
) -> std::result::Result<Vec<prism::reasoning::seeds::SeedSpec>, String> {
    specs
        .iter()
        .map(|spec| prism::reasoning::seeds::parse_file_line_spec(spec))
        .collect()
}

pub(super) fn run_nav(nav: &NavArgs) -> anyhow::Result<()> {
    let mut nav_options = prism::api::NavOptions::default();
    nav_options.no_cache = nav.no_cache;
    nav_options.cache_dir = nav.cache_dir.clone();
    nav_options.owner = nav.owner.options()?;
    match &nav.query {
        NavQuery::NodesAt {
            repo,
            location,
            format,
        } => {
            let (file, line) = location
                .rsplit_once(':')
                .and_then(|(f, l)| l.parse::<usize>().ok().map(|n| (f.to_string(), n)))
                .ok_or_else(|| anyhow::anyhow!("--location must be file:line"))?;
            let session = prism::api::nav_session(repo, &nav_options)?;
            let ev = prism::navigation::queries::nodes_at(&session, &file, line);
            println!("{}", prism_review::output::navigation::render(&ev, format));
            Ok(())
        }
        NavQuery::SymbolSpans {
            repo,
            symbol,
            file,
            location,
            format,
        } => {
            let session = prism::api::nav_session(repo, &nav_options)?;
            match prism::navigation::queries::symbol_spans(
                &session,
                symbol.as_deref(),
                file.as_deref(),
                location.as_deref(),
            ) {
                Ok(result) => {
                    println!(
                        "{}",
                        prism_review::output::navigation::render_symbol_spans(&result, format)
                    );
                    Ok(())
                }
                Err(error) => {
                    let (rendered, code) =
                        prism_review::output::navigation::render_err(&error, format);
                    println!("{rendered}");
                    prism::profile::exit(code);
                }
            }
        }
        NavQuery::Callers {
            repo,
            symbol,
            file,
            location,
            depth,
            confidence,
            format,
        } => {
            let session = prism::api::nav_session(repo, &nav_options)?;
            let exact = confidence == "exact";
            match prism::navigation::queries::callers_with_confidence(
                &session,
                symbol.as_deref(),
                file.as_deref(),
                location.as_deref(),
                *depth,
                exact,
            ) {
                Ok(ev) => {
                    println!("{}", prism_review::output::navigation::render(&ev, format));
                    Ok(())
                }
                Err(e) => {
                    let (s, code) = prism_review::output::navigation::render_err(&e, format);
                    println!("{s}");
                    prism::profile::exit(code);
                }
            }
        }
        NavQuery::Callees {
            repo,
            symbol,
            file,
            location,
            depth,
            confidence,
            format,
        } => {
            let session = prism::api::nav_session(repo, &nav_options)?;
            let exact = confidence == "exact";
            match prism::navigation::queries::callees_with_confidence(
                &session,
                symbol.as_deref(),
                file.as_deref(),
                location.as_deref(),
                *depth,
                exact,
            ) {
                Ok(ev) => {
                    println!("{}", prism_review::output::navigation::render(&ev, format));
                    Ok(())
                }
                Err(e) => {
                    let (s, code) = prism_review::output::navigation::render_err(&e, format);
                    println!("{s}");
                    prism::profile::exit(code);
                }
            }
        }
        NavQuery::Ego {
            repo,
            symbol,
            file,
            location,
            hops,
            edges,
            format,
        } => {
            let session = prism::api::nav_session(repo, &nav_options)?;
            let edge_kinds: Vec<&str> = edges.split(',').collect();
            match prism::navigation::queries::ego_graph(
                &session,
                symbol.as_deref(),
                file.as_deref(),
                location.as_deref(),
                *hops,
                &edge_kinds,
            ) {
                Ok(ev) => {
                    println!("{}", prism_review::output::navigation::render(&ev, format));
                    Ok(())
                }
                Err(e) => {
                    let (s, code) = prism_review::output::navigation::render_err(&e, format);
                    println!("{s}");
                    prism::profile::exit(code);
                }
            }
        }
        NavQuery::ModuleDeps { repo, file, format } => {
            let session = prism::api::nav_session(repo, &nav_options)?;
            let ev = prism::navigation::module_graph::module_deps(&session, file);
            println!("{}", prism_review::output::navigation::render(&ev, format));
            Ok(())
        }
        NavQuery::RepoMap { repo, format } => {
            let session = prism::api::nav_session(repo, &nav_options)?;
            let ev = prism::navigation::module_graph::repo_map(&session);
            println!("{}", prism_review::output::navigation::render(&ev, format));
            Ok(())
        }
        NavQuery::Onboard { repo, format, out } => {
            let session = prism::api::nav_session(repo, &nav_options)?;
            let report = prism::navigation::onboarding::build_report(&session)?;
            let rendered = prism_review::output::onboarding::render(&report, format)?;
            if let Some(path) = out {
                let mut file = match std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(path)
                {
                    Ok(file) => file,
                    Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
                        anyhow::bail!(
                            "onboarding report target already exists: {}",
                            path.display()
                        )
                    }
                    Err(error) => {
                        return Err(error).with_context(|| {
                            format!("failed to create onboarding report {}", path.display())
                        })
                    }
                };
                if let Err(error) = file
                    .write_all(rendered.as_bytes())
                    .and_then(|_| file.sync_all())
                {
                    drop(file);
                    if let Err(cleanup_error) = std::fs::remove_file(path) {
                        anyhow::bail!(
                            "failed to write onboarding report {}: {error}; failed to remove partial report: {cleanup_error}",
                            path.display()
                        );
                    }
                    return Err(error).with_context(|| {
                        format!("failed to write onboarding report {}", path.display())
                    });
                }
            } else {
                print!("{rendered}");
            }
            Ok(())
        }
        NavQuery::CallStats { repo, dump_sites } => {
            let session = prism::api::nav_session(repo, &nav_options)?;
            if *dump_sites {
                for site in
                    prism::navigation::queries::call_site_dump_iter(session.index.call_graph())
                {
                    println!("{}", serde_json::to_string(&site)?);
                }
            } else {
                let mut stats = prism::navigation::queries::call_stats(session.index.call_graph());
                stats.as_object_mut().expect("call-stats object").insert(
                    "return_flow".into(),
                    serde_json::to_value(&session.index.cpg().return_flow_stats)?,
                );
                stats.as_object_mut().expect("call-stats object").insert(
                    "dfg_labels".into(),
                    serde_json::to_value(&session.index.cpg().dfg_label_stats)?,
                );
                println!("{}", serde_json::to_string_pretty(&stats)?);
            }
            Ok(())
        }
        NavQuery::DfgStats { repo, edges } => {
            let session = prism::api::nav_session(repo, &nav_options)?;
            if *edges {
                for edge in prism::navigation::queries::dfg_edge_dump(session.index.cpg()) {
                    println!("{}", serde_json::to_string(&edge)?);
                }
            } else {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&session.index.cpg().dfg_label_stats)?
                );
            }
            Ok(())
        }
        NavQuery::InterfaceManifest { repo } => {
            let session = prism::api::nav_session(repo, &nav_options)?;
            let manifest =
                prism::navigation::queries::interface_dispatch_manifest(session.index.call_graph());
            println!("{}", serde_json::to_string_pretty(&manifest)?);
            Ok(())
        }
        NavQuery::Functions { repo, format } => {
            let recs = prism::navigation::inventory::functions_inventory(repo)?;
            if format == "json" {
                println!("{}", serde_json::to_string_pretty(&recs)?);
            } else {
                for r in &recs {
                    println!(
                        "{}:{}-{} {} [{}]",
                        r.file,
                        r.start_line,
                        r.end_line,
                        r.name.as_deref().unwrap_or("<anon>"),
                        r.kind
                    );
                }
            }
            Ok(())
        }
        NavQuery::TaintReaches {
            repo,
            source,
            sink,
            format,
        } => {
            let session = prism::api::nav_session(repo, &nav_options)?;
            let sources = match parse_loc_seeds(source) {
                Ok(seeds) => seeds,
                Err(msg) => {
                    eprintln!("error: {msg}");
                    prism::profile::exit(2);
                }
            };
            let sinks = if sink.is_empty() {
                None
            } else {
                match parse_loc_seeds(sink) {
                    Ok(seeds) => Some(seeds),
                    Err(msg) => {
                        eprintln!("error: {msg}");
                        prism::profile::exit(2);
                    }
                }
            };
            match prism::reasoning::taint_reaches::taint_reaches(
                &session,
                &sources,
                sinks.as_deref(),
            ) {
                Ok(ev) => {
                    println!("{}", prism_review::output::navigation::render(&ev, format));
                    Ok(())
                }
                Err(e) => {
                    let (s, code) = prism_review::output::navigation::render_err(&e, format);
                    println!("{s}");
                    prism::profile::exit(code);
                }
            }
        }
    }
}
