use anyhow::{Context, Result};
use clap::Parser;
use prism::cli::{Cli, Command, NavArgs, NavQuery, ReviewArgs, TargetsArgs};
use prism::output;
use prism::slice::{MultiSliceResult, SliceConfig};
use std::collections::BTreeMap;
use std::fs;

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

fn main() -> Result<()> {
    let cli = Cli::parse();
    // Run the whole command on the large-stack pool. prism walks tree-sitter
    // ASTs recursively in parsing, CPG build, and the live-type scan; on deeply
    // nested files those overflow a default ~2 MiB thread/worker stack. install()
    // runs the command and every nested par_iter on big-stack workers, covering
    // all of run_nav/run_review (incl. direct changed-file / delta / contract
    // parses) in one place. See prism::build_pool.
    prism::build_pool::install(|| match &cli.command {
        Some(Command::Nav(nav)) => run_nav(nav),
        Some(Command::Targets(targets)) => run_targets(targets),
        None => run_review(&cli.review),
    })
}

fn run_nav(nav: &NavArgs) -> anyhow::Result<()> {
    let mut nav_options = prism::api::NavOptions::default();
    nav_options.no_cache = nav.no_cache;
    nav_options.cache_dir = nav.cache_dir.clone();
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
            println!("{}", prism::output::navigation::render(&ev, format));
            Ok(())
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
                    println!("{}", prism::output::navigation::render(&ev, format));
                    Ok(())
                }
                Err(e) => {
                    let (s, code) = prism::output::navigation::render_err(&e, format);
                    println!("{s}");
                    std::process::exit(code);
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
                    println!("{}", prism::output::navigation::render(&ev, format));
                    Ok(())
                }
                Err(e) => {
                    let (s, code) = prism::output::navigation::render_err(&e, format);
                    println!("{s}");
                    std::process::exit(code);
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
                    println!("{}", prism::output::navigation::render(&ev, format));
                    Ok(())
                }
                Err(e) => {
                    let (s, code) = prism::output::navigation::render_err(&e, format);
                    println!("{s}");
                    std::process::exit(code);
                }
            }
        }
        NavQuery::ModuleDeps { repo, file, format } => {
            let session = prism::api::nav_session(repo, &nav_options)?;
            let ev = prism::navigation::module_graph::module_deps(&session, file);
            println!("{}", prism::output::navigation::render(&ev, format));
            Ok(())
        }
        NavQuery::RepoMap { repo, format } => {
            let session = prism::api::nav_session(repo, &nav_options)?;
            let ev = prism::navigation::module_graph::repo_map(&session);
            println!("{}", prism::output::navigation::render(&ev, format));
            Ok(())
        }
        NavQuery::CallStats { repo, dump_sites } => {
            let session = prism::api::nav_session(repo, &nav_options)?;
            if *dump_sites {
                for site in prism::navigation::queries::call_site_dump(session.index.call_graph()) {
                    println!("{}", serde_json::to_string(&site)?);
                }
            } else {
                let mut stats = prism::navigation::queries::call_stats(session.index.call_graph());
                stats.as_object_mut().expect("call-stats object").insert(
                    "return_flow".into(),
                    serde_json::to_value(&session.index.cpg().return_flow_stats)?,
                );
                println!("{}", serde_json::to_string_pretty(&stats)?);
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
                    std::process::exit(2);
                }
            };
            let sinks = if sink.is_empty() {
                None
            } else {
                match parse_loc_seeds(sink) {
                    Ok(seeds) => Some(seeds),
                    Err(msg) => {
                        eprintln!("error: {msg}");
                        std::process::exit(2);
                    }
                }
            };
            match prism::reasoning::taint_reaches::taint_reaches(
                &session,
                &sources,
                sinks.as_deref(),
            ) {
                Ok(ev) => {
                    println!("{}", prism::output::navigation::render(&ev, format));
                    Ok(())
                }
                Err(e) => {
                    let (s, code) = prism::output::navigation::render_err(&e, format);
                    println!("{s}");
                    std::process::exit(code);
                }
            }
        }
    }
}

fn run_targets(args: &TargetsArgs) -> Result<()> {
    const FINDING_ALGORITHMS: &[&str] = &[
        "echo",
        "absence",
        "contract",
        "provenance",
        "membrane",
        "taint",
        "symmetry",
        "peer_consistency",
        "callback_dispatcher",
        "primitive",
    ];
    const EMPTY_ALGORITHMS: &[&str] = &["angle", "delta"];
    const ACCEPTED: &str = "echo, absence, contract, provenance, membrane, taint, symmetry, peer_consistency, callback_dispatcher, primitive, angle, delta";

    // The acceptance table is deliberately evaluated before canonicalizing the
    // repo, reading the diff, or building a CPG.
    let requested: Vec<String> = args
        .algorithm
        .split(',')
        .map(|name| name.trim().to_lowercase())
        .collect();
    for name in &requested {
        if name == "chop" || name == "conditioned" {
            anyhow::bail!(
                "targets: algorithm {name} requires --chop-source/--chop-sink (or --condition); use the top-level command"
            );
        }
        if !FINDING_ALGORITHMS.contains(&name.as_str())
            && !EMPTY_ALGORITHMS.contains(&name.as_str())
        {
            anyhow::bail!(
                "targets: algorithm {name} produces slice blocks, not findings; accepted: {ACCEPTED}"
            );
        }
        if name == "delta" && args.old_repo.is_none() {
            anyhow::bail!("targets: delta requires --old-repo");
        }
    }
    for name in &requested {
        if EMPTY_ALGORITHMS.contains(&name.as_str()) {
            eprintln!("targets: {name} produces no findings at this version");
        }
    }

    let algorithms = prism::api::parse_algorithms(&requested.join(","))?;
    let repo = fs::canonicalize(&args.repo)
        .with_context(|| format!("Failed to canonicalize repo: {:?}", args.repo))?;
    let diff_text = fs::read_to_string(&args.diff)
        .with_context(|| format!("Failed to read diff: {:?}", args.diff))?;

    let mut review_options = prism::api::ReviewOptions::new(&repo);
    review_options.files_filter = args.files.as_ref().map(|files| {
        files
            .split(',')
            .map(|file| file.trim().to_string())
            .collect()
    });
    review_options.compile_commands = args.compile_commands.clone();
    review_options.scoped_cpg = args.scoped_cpg;
    review_options.cache_dir = args.cache_dir.clone();
    review_options.no_cache = args.no_cache;

    let mut algorithm_params = prism::api::AlgorithmParams::default();
    algorithm_params.old_repo = args.old_repo.clone();
    let config = SliceConfig {
        algorithm: algorithms[0],
        scoped_cpg: args.scoped_cpg,
        ..SliceConfig::default()
    };
    let outcome = prism::api::review(
        &review_options,
        &diff_text,
        &algorithms,
        &config,
        &algorithm_params,
    )?;

    let mut run_warnings = outcome.inputs.parse_warnings.clone();
    run_warnings.extend(outcome.inputs.load_warnings.clone());
    run_warnings.extend(outcome.build_warnings.clone());
    let min_tier = if args.min_tier == "asserted" {
        prism::api::FindingTier::Asserted
    } else {
        prism::api::FindingTier::Candidate
    };
    // `TargetsMeta` is `#[non_exhaustive]` (§2.3.1): Default + field assignment,
    // the same shape an embedder must use.
    let mut meta = prism::targets::TargetsMeta::default();
    meta.algorithms_run = outcome.run.algorithms_run.clone();
    meta.repo_root = repo.clone();
    meta.repo_sha = repo_sha(&repo);
    meta.errors = outcome.run.errors.clone();
    meta.run_warnings = run_warnings;
    meta.min_severity_rank = output::severity_rank(&args.min_severity);
    meta.min_tier = min_tier;
    let document = prism::targets::project(&outcome.run.findings, &outcome.inputs, &meta);

    let mut bytes = serde_json::to_vec_pretty(&document)?;
    bytes.push(b'\n');
    if let Some(path) = &args.out {
        fs::write(path, &bytes).with_context(|| format!("Failed to write targets: {path:?}"))?;
    } else {
        use std::io::Write;
        std::io::stdout().lock().write_all(&bytes)?;
    }

    let exit_code = targets_exit_code(args.strict, &document.errors);
    if exit_code != 0 {
        std::process::exit(exit_code);
    }
    Ok(())
}

fn repo_sha(repo: &std::path::Path) -> Option<String> {
    let output = std::process::Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let sha = String::from_utf8(output.stdout).ok()?;
    let sha = sha.trim();
    (!sha.is_empty()).then(|| sha.to_string())
}

fn targets_exit_code(strict: bool, errors: &[prism::slice::AlgorithmError]) -> i32 {
    if strict && !errors.is_empty() {
        3
    } else {
        0
    }
}

fn run_review(cli: &ReviewArgs) -> Result<()> {
    if cli.list_algorithms {
        println!("Available algorithms:\n");
        println!("  Paper (arXiv:2505.17928):");
        println!("    originaldiff     Raw diff lines only");
        println!("    parentfunction   Entire enclosing function");
        println!("    leftflow         Backward data-flow from L-values (default)");
        println!("    fullflow         LeftFlow + R-value forward tracing");
        println!();
        println!("  Established taxonomy:");
        println!("    thin             Data deps only, no control flow context");
        println!("    barrier          Interprocedural with depth limits (--barrier-depth, --barrier-symbols)");
        println!("    chop             Source-to-sink paths (--chop-source, --chop-sink)");
        println!("    taint            Forward taint propagation (--taint-source)");
        println!("    relevant         Backward + alternate branch paths");
        println!("    conditioned      Slice under assumption (--condition)");
        println!("    delta            Behavioral diff between versions (--old-repo)");
        println!();
        println!("  Theoretical extensions:");
        println!("    spiral           Adaptive-depth concentric rings (--spiral-max-ring)");
        println!("    circular         Data flow cycle detection");
        println!("    quantum          Concurrent state enumeration (--quantum-var)");
        println!("    horizontal       Peer pattern consistency (--peer-pattern)");
        println!("    vertical         End-to-end feature path (--layers)");
        println!("    angle            Cross-cutting concern trace (--concern)");
        println!("    3d               Temporal-structural risk (--temporal-days)");
        println!();
        println!("  Novel extensions:");
        println!(
            "    absence          Missing counterparts (open without close, lock without unlock)"
        );
        println!("    resonance        Files that usually co-change but aren't in this diff");
        println!("    symmetry         Broken symmetry (serialize changed, deserialize not)");
        println!("    gradient         Continuous relevance scoring with distance decay");
        println!("    provenance       Trace data origin (user input, config, database, constant)");
        println!("    phantom          Recently deleted code the diff may depend on");
        println!("    membrane         Module boundary: who calls this API and will they break");
        println!(
            "    echo             Ripple effect: downstream callers missing new error handling"
        );
        println!(
            "    contract         Implicit behavioral contract extraction and violation detection"
        );
        println!(
            "    peer             Peer-signature guard divergence (C/C++ sibling NULL-guard clusters)"
        );
        println!(
            "    callback         Callback-dispatcher resolution (function-pointer registrations → invocation sites)"
        );
        println!(
            "    primitive        Security-primitive fingerprints (hash truncation, weak-hash-for-identity, shell-injection, TLS disabled, hardcoded secrets)"
        );
        return Ok(());
    }

    let algorithms_to_run = prism::api::parse_algorithms(&cli.algorithm)?;

    let multi_run = algorithms_to_run.len() > 1;

    let config = SliceConfig {
        algorithm: algorithms_to_run[0],
        max_branch_lines: cli.max_branch_lines,
        include_returns: !cli.no_returns,
        trace_callees: !cli.no_trace_callees,
        scoped_cpg: cli.scoped_cpg,
        diagram_node_cap: cli.diagram_node_cap,
        strict_diagrams: cli.strict_diagrams,
    };

    let repo = cli.repo.as_ref().context("--repo is required")?;
    let diff_path = cli.diff.as_ref().context("--diff is required")?;

    // Read diff input
    let diff_text =
        fs::read_to_string(diff_path).context(format!("Failed to read diff: {:?}", diff_path))?;

    let mut review_options = prism::api::ReviewOptions::new(repo);
    review_options.files_filter = cli
        .files
        .as_ref()
        .map(|f| f.split(',').map(|s| s.trim().to_string()).collect());
    review_options.compile_commands = cli.compile_commands.clone();
    review_options.scoped_cpg = cli.scoped_cpg;
    review_options.cache_dir = cli.cache_dir.clone();
    review_options.no_cache = cli.no_cache;
    if let Some(ref v) = cli.python_version {
        if let Some(lv) = prism::type_provider::LanguageVersion::parse(v) {
            review_options
                .language_versions
                .push((prism::languages::Language::Python, lv));
        }
    }
    if let Some(ref v) = cli.go_version {
        if let Some(lv) = prism::type_provider::LanguageVersion::parse(v) {
            review_options
                .language_versions
                .push((prism::languages::Language::Go, lv));
        }
    }
    if let Some(ref v) = cli.node_version {
        if let Some(lv) = prism::type_provider::LanguageVersion::parse(v) {
            review_options
                .language_versions
                .push((prism::languages::Language::JavaScript, lv));
        }
    }
    if let Some(ref v) = cli.typescript_version {
        if let Some(lv) = prism::type_provider::LanguageVersion::parse(v) {
            review_options
                .language_versions
                .push((prism::languages::Language::TypeScript, lv));
        }
    }
    if let Some(ref v) = cli.java_version {
        if let Some(lv) = prism::type_provider::LanguageVersion::parse(v) {
            review_options
                .language_versions
                .push((prism::languages::Language::Java, lv));
        }
    }
    if let Some(ref v) = cli.rust_version {
        if let Some(lv) = prism::type_provider::LanguageVersion::parse(v) {
            review_options
                .language_versions
                .push((prism::languages::Language::Rust, lv));
        }
    }

    let mut algorithm_params = prism::api::AlgorithmParams::default();
    algorithm_params.barrier_depth = cli.barrier_depth;
    algorithm_params.barrier_symbols = cli
        .barrier_symbols
        .split(',')
        .filter(|s| !s.is_empty())
        .map(|s| s.trim().to_string())
        .collect();
    algorithm_params.chop_source = cli.chop_source.clone();
    algorithm_params.chop_sink = cli.chop_sink.clone();
    algorithm_params.taint_sources = cli.taint_source.clone();
    algorithm_params.taint_return_flow = cli.taint_return_flow;
    algorithm_params.condition = cli.condition.clone();
    algorithm_params.old_repo = cli.old_repo.clone();
    algorithm_params.spiral_max_ring = cli.spiral_max_ring;
    algorithm_params.quantum_var = cli.quantum_var.clone();
    algorithm_params.peer_pattern = cli.peer_pattern.clone();
    algorithm_params.layers = cli.layers.clone();
    algorithm_params.concern = cli.concern.clone();
    algorithm_params.temporal_days = cli.temporal_days;

    let inputs = prism::api::load_review_inputs(&review_options, &diff_text)?;
    let built = prism::api::build_context(&inputs, &review_options)?;

    // --format callers: emit raw call graph without running any algorithm.
    if cli.format == "callers" {
        let callers_output = output::to_callers_output(&built.ctx, &inputs.diff, cli.caller_depth);
        println!("{}", serde_json::to_string_pretty(&callers_output)?);
        return Ok(());
    }

    if multi_run {
        // --- Multi-algorithm run ---
        let run = prism::api::run_review(
            &built.ctx,
            &inputs,
            &algorithms_to_run,
            &config,
            &algorithm_params,
            repo,
        );
        let results = run.results;
        let all_errors = run.errors;
        let algorithms_run = run.algorithms_run;
        let all_findings = run.findings;

        match cli.format.as_str() {
            "review" => {
                // Compact review-only path (P1 Change 3): severity floor +
                // block retention + dropped slice_lines/diff_lines. Distinct
                // from "json" below, which keeps the old byte-pinned shape.
                let min_rank = output::severity_rank(&cli.review_min_severity);
                let all_diagram_warnings: Vec<_> = results
                    .iter()
                    .flat_map(|r| r.diagram_warnings.iter().cloned())
                    .collect();
                let review_results: Vec<_> = results
                    .iter()
                    .map(|r| {
                        output::to_compact_review_output(
                            r,
                            &inputs.sources,
                            min_rank,
                            cli.review_full_slices,
                            cli.review_no_diagrams,
                        )
                    })
                    .collect();
                let mut filtered_all_findings: Vec<_> = all_findings
                    .iter()
                    .filter(|f| output::severity_rank(&f.severity) >= min_rank)
                    .cloned()
                    .collect();
                if cli.review_no_diagrams {
                    // Second copy (spec-review delta 1): all_findings is built
                    // independently of review_results above, not through
                    // to_compact_review_output, so it needs the same strip
                    // applied via the shared helper to avoid drifting from it.
                    output::strip_finding_diagrams(&mut filtered_all_findings);
                }
                let out = output::CompactMultiReviewOutput {
                    version: "1.0".to_string(),
                    algorithms_run,
                    results: review_results,
                    all_findings: filtered_all_findings,
                    errors: all_errors,
                    warnings: inputs.parse_warnings.clone(),
                    parse_quality: inputs.parse_quality.clone(),
                    diagram_warnings: all_diagram_warnings.clone(),
                };
                println!("{}", serde_json::to_string_pretty(&out)?);
                emit_warnings_to_stderr(&all_diagram_warnings);
                let exit_code = determine_exit_code(cli.strict_diagrams, &all_diagram_warnings);
                if exit_code != 0 {
                    std::process::exit(exit_code);
                }
            }
            "json" => {
                // json and review produce the same ReviewOutput structure so that
                // slice_text (rendered source code) is always present in structured output.
                let all_diagram_warnings: Vec<_> = results
                    .iter()
                    .flat_map(|r| r.diagram_warnings.iter().cloned())
                    .collect();
                let review_results: Vec<_> = results
                    .iter()
                    .map(|r| output::to_review_output(r, &inputs.sources))
                    .collect();
                let out = output::MultiReviewOutput {
                    version: "1.0".to_string(),
                    algorithms_run,
                    results: review_results,
                    all_findings,
                    errors: all_errors,
                    warnings: inputs.parse_warnings.clone(),
                    parse_quality: inputs.parse_quality.clone(),
                    diagram_warnings: all_diagram_warnings.clone(),
                };
                println!("{}", serde_json::to_string_pretty(&out)?);
                emit_warnings_to_stderr(&all_diagram_warnings);
                let exit_code = determine_exit_code(cli.strict_diagrams, &all_diagram_warnings);
                if exit_code != 0 {
                    std::process::exit(exit_code);
                }
            }
            "sarif" => {
                // SARIF 2.1 (design §2.2). Same trailer as "json"; the
                // serializer is total, so nothing here can fail but the
                // pretty-printer.
                let all_diagram_warnings: Vec<_> = results
                    .iter()
                    .flat_map(|r| r.diagram_warnings.iter().cloned())
                    .collect();
                let document = output::to_sarif(
                    &output::SarifInputs::new(&all_findings)
                        .errors(&all_errors)
                        .parse_warnings(&inputs.parse_warnings)
                        .load_warnings(&inputs.load_warnings)
                        .build_warnings(&built.warnings)
                        .algorithms_run(&algorithms_run)
                        .parse_quality(&inputs.parse_quality)
                        .files(&inputs.files)
                        .sources(&inputs.sources),
                );
                println!("{}", serde_json::to_string_pretty(&document)?);
                emit_warnings_to_stderr(&all_diagram_warnings);
                let exit_code = determine_exit_code(cli.strict_diagrams, &all_diagram_warnings);
                if exit_code != 0 {
                    std::process::exit(exit_code);
                }
            }
            "mermaid" => {
                let multi_result = MultiSliceResult {
                    version: "1.0".to_string(),
                    algorithms_run,
                    results,
                    findings: all_findings,
                    errors: all_errors,
                    warnings: inputs.parse_warnings.clone(),
                    parse_quality: inputs.parse_quality.clone(),
                    diagram_warnings: vec![],
                };
                let report = output::format_mermaid_report(&multi_result);
                println!("{}", report);
                let warnings = multi_result.aggregate_diagram_warnings();
                emit_warnings_to_stderr(&warnings);
                let exit_code = determine_exit_code(cli.strict_diagrams, &warnings);
                if exit_code != 0 {
                    std::process::exit(exit_code);
                }
            }
            _ => {
                for w in &inputs.parse_warnings {
                    eprintln!("WARNING: {}", w);
                }
                for result in &results {
                    println!("=== {} ===", result.algorithm.name());
                    print!(
                        "{}",
                        output::format_slice_result(&result.blocks, &inputs.sources)
                    );
                }
                let all_diagram_warnings: Vec<_> = results
                    .iter()
                    .flat_map(|r| r.diagram_warnings.iter().cloned())
                    .collect();
                emit_warnings_to_stderr(&all_diagram_warnings);
                let exit_code = determine_exit_code(cli.strict_diagrams, &all_diagram_warnings);
                if exit_code != 0 {
                    std::process::exit(exit_code);
                }
            }
        }
    } else {
        // --- Single-algorithm run ---
        let algorithm = algorithms_to_run[0];
        let mut result = prism::api::run_algorithm(
            algorithm,
            &built.ctx,
            &inputs,
            &config,
            &algorithm_params,
            repo,
        )?;
        result.warnings = inputs.parse_warnings.clone();

        match cli.format.as_str() {
            "review" => {
                // Compact review-only path (P1 Change 3): severity floor +
                // block retention + dropped slice_lines/diff_lines.
                let min_rank = output::severity_rank(&cli.review_min_severity);
                let review = output::to_compact_review_output(
                    &result,
                    &inputs.sources,
                    min_rank,
                    cli.review_full_slices,
                    cli.review_no_diagrams,
                );
                println!("{}", serde_json::to_string_pretty(&review)?);
                emit_warnings_to_stderr(&result.diagram_warnings);
                let exit_code = determine_exit_code(cli.strict_diagrams, &result.diagram_warnings);
                if exit_code != 0 {
                    std::process::exit(exit_code);
                }
            }
            "json" => {
                // json retains the old ReviewOutput structure byte-for-byte
                // (compatibility tests pin this shape — see nav_compat_test.rs).
                let review = output::to_review_output(&result, &inputs.sources);
                println!("{}", serde_json::to_string_pretty(&review)?);
                emit_warnings_to_stderr(&result.diagram_warnings);
                let exit_code = determine_exit_code(cli.strict_diagrams, &result.diagram_warnings);
                if exit_code != 0 {
                    std::process::exit(exit_code);
                }
            }
            "paper" => {
                let paper_output = output::to_paper_format(&result.blocks);
                println!("{}", serde_json::to_string_pretty(&paper_output)?);
                emit_warnings_to_stderr(&result.diagram_warnings);
                let exit_code = determine_exit_code(cli.strict_diagrams, &result.diagram_warnings);
                if exit_code != 0 {
                    std::process::exit(exit_code);
                }
            }
            "sarif" => {
                // Single-run: no AlgorithmError can exist (a failing algorithm
                // is a hard `?` above), so `errors` keeps the builder's empty
                // default; `result.warnings` already carries the parse warnings
                // assigned at the top of this branch.
                let algorithms_run = vec![algorithm.name().to_string()];
                let document = output::to_sarif(
                    &output::SarifInputs::new(&result.findings)
                        .parse_warnings(&result.warnings)
                        .load_warnings(&inputs.load_warnings)
                        .build_warnings(&built.warnings)
                        .algorithms_run(&algorithms_run)
                        .parse_quality(&inputs.parse_quality)
                        .files(&inputs.files)
                        .sources(&inputs.sources),
                );
                println!("{}", serde_json::to_string_pretty(&document)?);
                emit_warnings_to_stderr(&result.diagram_warnings);
                let exit_code = determine_exit_code(cli.strict_diagrams, &result.diagram_warnings);
                if exit_code != 0 {
                    std::process::exit(exit_code);
                }
            }
            "mermaid" => {
                let algorithms_run = vec![result.algorithm.name().to_string()];
                let multi_result = MultiSliceResult {
                    version: "1.0".to_string(),
                    algorithms_run,
                    results: vec![result],
                    findings: vec![],
                    errors: vec![],
                    warnings: vec![],
                    parse_quality: BTreeMap::new(),
                    diagram_warnings: vec![],
                };
                let report = output::format_mermaid_report(&multi_result);
                println!("{}", report);
                let warnings = multi_result.aggregate_diagram_warnings();
                emit_warnings_to_stderr(&warnings);
                let exit_code = determine_exit_code(cli.strict_diagrams, &warnings);
                if exit_code != 0 {
                    std::process::exit(exit_code);
                }
            }
            _ => {
                for w in &result.warnings {
                    eprintln!("WARNING: {}", w);
                }
                print!(
                    "{}",
                    output::format_slice_result(&result.blocks, &inputs.sources)
                );
                emit_warnings_to_stderr(&result.diagram_warnings);
                let exit_code = determine_exit_code(cli.strict_diagrams, &result.diagram_warnings);
                if exit_code != 0 {
                    std::process::exit(exit_code);
                }
            }
        }
    }

    Ok(())
}

fn emit_warnings_to_stderr(warnings: &[prism::slice::DiagramWarning]) {
    use std::io::Write;
    let mut err = std::io::stderr().lock();
    for w in warnings {
        let title = w.graph_title.as_deref().unwrap_or("(no title)");
        let _ = writeln!(
            err,
            "prism: diagram warning: {}/{} - {:?}: {}",
            w.algorithm, title, w.kind, w.detail
        );
    }
}

fn determine_exit_code(strict: bool, warnings: &[prism::slice::DiagramWarning]) -> i32 {
    if !strict {
        return 0;
    }
    if warnings.iter().any(|w| w.kind.is_bug()) {
        return 2;
    }
    0
}

#[cfg(test)]
mod cli_parse_tests {
    // `parse_diagram_cap`'s own unit tests moved with it to `src/cli.rs`
    // (spec §2.5.8 / §6 pure move): they now live in `prism::cli::tests`.

    /// Architecture pin: `run_slicing_inner` must be a public symbol in
    /// `prism::algorithms`.  The `run_algorithm` fallback path calls it so that
    /// the single `finalize_diagrams` call at the end of `run_algorithm` is the
    /// only one — if the fallback used `run_slicing` (which finalizes), then the
    /// trailing `finalize_diagrams` call would be a second invocation, duplicating
    /// all diagram warnings in JSON output and `## Diagnostics` sections.
    ///
    /// This test does not call the function at runtime; it merely references the
    /// path so that removing the symbol breaks compilation.
    #[test]
    fn run_slicing_inner_is_accessible_for_no_double_finalize_seam() {
        // Compile-time pin: the symbol must be reachable.
        let _ = prism::algorithms::run_slicing_inner
            as fn(
                &prism::cpg::CpgContext,
                &prism::diff::DiffInput,
                &prism::slice::SliceConfig,
            ) -> anyhow::Result<prism::slice::SliceResult>;
    }
}

#[cfg(test)]
mod exit_tests {
    use super::*;
    use prism::slice::{DiagramWarning, DiagramWarningKind};

    fn warn(kind: DiagramWarningKind) -> DiagramWarning {
        DiagramWarning {
            algorithm: "Taint".to_string(),
            graph_title: None,
            kind,
            detail: "x".to_string(),
        }
    }

    #[test]
    fn determine_exit_code_strict_with_bug_warning() {
        let warns = vec![warn(DiagramWarningKind::DanglingEdge)];
        assert_eq!(determine_exit_code(true, &warns), 2);
    }

    #[test]
    fn determine_exit_code_strict_with_only_informational() {
        let warns = vec![warn(DiagramWarningKind::NodeCapExceeded)];
        assert_eq!(determine_exit_code(true, &warns), 0);
    }

    #[test]
    fn determine_exit_code_strict_off() {
        let warns = vec![warn(DiagramWarningKind::DanglingEdge)];
        assert_eq!(determine_exit_code(false, &warns), 0);
    }

    #[test]
    fn determine_exit_code_strict_no_warnings() {
        let warns: Vec<DiagramWarning> = vec![];
        assert_eq!(determine_exit_code(true, &warns), 0);
    }

    #[test]
    fn targets_strict_exit_code_tracks_algorithm_errors() {
        let errors = vec![prism::slice::AlgorithmError {
            algorithm: "DeltaSlice".to_string(),
            error: "fixture error".to_string(),
        }];
        assert_eq!(targets_exit_code(true, &errors), 3);
        assert_eq!(targets_exit_code(false, &errors), 0);
        assert_eq!(targets_exit_code(true, &[]), 0);
    }
}
