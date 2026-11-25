mod cli_nav;
mod cli_output;
mod cli_targets;

use anyhow::{Context, Result};
use clap::{CommandFactory, FromArgMatches};
use cli_output::{determine_exit_code, emit_warnings_to_stderr};
use prism::slice::{MultiSliceResult, SliceConfig};
use prism_review::cli::{Cli, Command, ReviewArgs};
use prism_review::output;
use std::collections::BTreeMap;
use std::fs;

// #437: the `prism` CLI uses jemalloc on Linux; glibc arenas retain several GiB
// under the parallel CPG build. `prism-mcp` deliberately stays on the system
// allocator (its Linux memory sampler reads glibc `mallinfo2`).
#[cfg(target_os = "linux")]
#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

fn main() -> Result<()> {
    let _profile = prism::profile::Session::start();
    let min_confidence_explicit = std::env::args_os().any(|argument| {
        argument == "--min-confidence"
            || argument
                .to_str()
                .is_some_and(|argument| argument.starts_with("--min-confidence="))
    });
    let matches = Cli::command()
        .try_get_matches()
        .unwrap_or_else(|error| prism::profile::clap_exit(error));
    let cli =
        Cli::from_arg_matches(&matches).unwrap_or_else(|error| prism::profile::clap_exit(error));
    if let Err(error) = cli.validate_min_confidence(min_confidence_explicit) {
        prism::profile::clap_exit(error);
    }
    // Run the whole command on the large-stack pool. prism walks tree-sitter
    // ASTs recursively in parsing, CPG build, and the live-type scan; on deeply
    // nested files those overflow a default ~2 MiB thread/worker stack. install()
    // runs the command and every nested par_iter on big-stack workers, covering
    // all of run_nav/run_review (incl. direct changed-file / delta / contract
    // parses) in one place. See prism::build_pool.
    prism::build_pool::install(|| match &cli.command {
        Some(Command::Nav(nav)) => cli_nav::run_nav(nav),
        Some(Command::Targets(targets)) => {
            cli_targets::run_targets(targets, matches.subcommand_matches("targets").unwrap())
        }
        None => run_review(&cli.review),
    })
}

fn run_review(cli: &ReviewArgs) -> Result<()> {
    use prism_review::api::{ResultView, ReviewAuthority, TaintContextMode};

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

    let algorithms_to_run = prism_review::api::parse_algorithms(&cli.algorithm)?;

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
    review_options.min_confidence = cli.min_confidence;
    review_options.resolution = cli.resolution;
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

    let mut algorithm_params = prism_review::api::AlgorithmParams::default();
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
    algorithm_params.taint_context = cli.taint_context.options();

    let inputs = prism::api::load_review_inputs(&review_options, &diff_text)?;
    let built = prism::api::build_context(&inputs, &review_options)?;

    // --format callers: emit raw call graph without running any algorithm.
    if cli.format == "callers" {
        if cli.taint_context.mode == TaintContextMode::Imports {
            cli_output::write_messages(
                &mut std::io::stderr().lock(),
                &["taint-context ignored: --format callers runs no algorithm".into()],
            )?;
        }
        let callers_output = output::to_callers_output(&built.ctx, &inputs.diff, cli.caller_depth);
        println!("{}", serde_json::to_string_pretty(&callers_output)?);
        return Ok(());
    }

    if multi_run {
        // --- Multi-algorithm run ---
        let mut run = prism_review::api::run_review_authorized(
            &built.ctx,
            ReviewAuthority::capture(&inputs),
            &algorithms_to_run,
            &config,
            &algorithm_params,
            repo,
        );
        let context = run.take_context();
        let diagnostics = run.diagnostics;
        let warnings = run.warnings;
        let results = run.results;
        let all_errors = run.errors;
        let algorithms_run = run.algorithms_run;
        let all_findings = run.findings;
        let all_evidence = run.evidence;
        let min_confidence = run.min_confidence;
        let resolution = run.resolution;

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
                        output::to_compact_review_output_for(
                            r,
                            ResultView::for_result(context.as_ref(), r, &inputs),
                            min_rank,
                            cli.review_full_slices,
                            cli.review_no_diagrams,
                        )
                    })
                    .collect();
                let findings_below_floor = output::count_below_floor(&all_findings, min_rank);
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
                    findings_below_floor,
                    errors: all_errors,
                    warnings,
                    parse_quality: inputs.parse_quality.clone(),
                    diagram_warnings: all_diagram_warnings.clone(),
                };
                println!("{}", serde_json::to_string_pretty(&out)?);
                emit_warnings_to_stderr(&all_diagram_warnings);
                let exit_code = determine_exit_code(cli.strict_diagrams, &all_diagram_warnings);
                if exit_code != 0 {
                    prism::profile::exit(exit_code);
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
                    .map(|r| {
                        output::to_review_output_for(
                            r,
                            ResultView::for_result(context.as_ref(), r, &inputs),
                        )
                    })
                    .collect();
                let out = output::MultiReviewOutput {
                    version: "1.0".to_string(),
                    algorithms_run,
                    results: review_results,
                    all_findings,
                    errors: all_errors,
                    warnings,
                    parse_quality: inputs.parse_quality.clone(),
                    diagram_warnings: all_diagram_warnings.clone(),
                };
                println!("{}", serde_json::to_string_pretty(&out)?);
                emit_warnings_to_stderr(&all_diagram_warnings);
                let exit_code = determine_exit_code(cli.strict_diagrams, &all_diagram_warnings);
                if exit_code != 0 {
                    prism::profile::exit(exit_code);
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
                let document = {
                    let mut sarif = output::SarifInputs::new(&all_findings)
                        .evidence(&all_evidence)
                        .errors(&all_errors)
                        .parse_warnings(&inputs.parse_warnings)
                        .load_warnings(&inputs.load_warnings)
                        .build_warnings(&built.warnings)
                        .algorithms_run(&algorithms_run)
                        .parse_quality(&inputs.parse_quality)
                        .files(&inputs.files)
                        .sources(&inputs.sources)
                        .min_confidence(min_confidence)
                        .resolution(resolution);
                    if let Some(diagnostics) = &diagnostics {
                        sarif = sarif.boundary_diagnostics(diagnostics);
                    }
                    if let Some(context) = &context {
                        sarif = sarif.context(context);
                    }
                    output::to_sarif(&sarif)
                };
                println!("{}", serde_json::to_string_pretty(&document)?);
                emit_warnings_to_stderr(&all_diagram_warnings);
                let exit_code = determine_exit_code(cli.strict_diagrams, &all_diagram_warnings);
                if exit_code != 0 {
                    prism::profile::exit(exit_code);
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
                if let Some(diagnostics) = &diagnostics {
                    cli_output::write_messages(
                        &mut std::io::stderr().lock(),
                        &diagnostics.messages(),
                    )?;
                }
                let warnings = multi_result.aggregate_diagram_warnings();
                emit_warnings_to_stderr(&warnings);
                let exit_code = determine_exit_code(cli.strict_diagrams, &warnings);
                if exit_code != 0 {
                    prism::profile::exit(exit_code);
                }
            }
            _ => {
                for w in &warnings {
                    eprintln!("WARNING: {}", w);
                }
                for result in &results {
                    println!("=== {} ===", result.algorithm.name());
                    let view = ResultView::for_result(context.as_ref(), result, &inputs);
                    print!(
                        "{}",
                        output::format_slice_result(&result.blocks, view.sources())
                    );
                }
                let all_diagram_warnings: Vec<_> = results
                    .iter()
                    .flat_map(|r| r.diagram_warnings.iter().cloned())
                    .collect();
                emit_warnings_to_stderr(&all_diagram_warnings);
                let exit_code = determine_exit_code(cli.strict_diagrams, &all_diagram_warnings);
                if exit_code != 0 {
                    prism::profile::exit(exit_code);
                }
            }
        }
    } else {
        // --- Single-algorithm run ---
        let algorithm = algorithms_to_run[0];
        let mut run = prism_review::api::run_algorithm_authorized(
            algorithm,
            &built.ctx,
            ReviewAuthority::capture(&inputs),
            &config,
            &algorithm_params,
            repo,
        )?;
        run.filter_findings(&inputs, cli.min_confidence, cli.resolution);
        let (outcome, context) = run.into_parts();
        let diagnostics = outcome.diagnostics;
        let mut result = outcome.result;
        let ordinary_warnings = std::mem::take(&mut result.warnings);
        result.warnings = inputs.parse_warnings.clone();
        if !matches!(cli.format.as_str(), "paper" | "mermaid" | "sarif") {
            result.warnings = cli_output::compose_warnings(
                &inputs.parse_warnings,
                &ordinary_warnings,
                diagnostics.iter().flat_map(|d| d.messages()),
            );
        }

        match cli.format.as_str() {
            "review" => {
                // Compact review-only path (P1 Change 3): severity floor +
                // block retention + dropped slice_lines/diff_lines.
                let min_rank = output::severity_rank(&cli.review_min_severity);
                let review = output::to_compact_review_output_for(
                    &result,
                    ResultView::for_result(context.as_ref(), &result, &inputs),
                    min_rank,
                    cli.review_full_slices,
                    cli.review_no_diagrams,
                );
                println!("{}", serde_json::to_string_pretty(&review)?);
                emit_warnings_to_stderr(&result.diagram_warnings);
                let exit_code = determine_exit_code(cli.strict_diagrams, &result.diagram_warnings);
                if exit_code != 0 {
                    prism::profile::exit(exit_code);
                }
            }
            "json" => {
                // json retains the old ReviewOutput structure byte-for-byte
                // (compatibility tests pin this shape — see nav_compat_test.rs).
                let review = output::to_review_output_for(
                    &result,
                    ResultView::for_result(context.as_ref(), &result, &inputs),
                );
                println!("{}", serde_json::to_string_pretty(&review)?);
                emit_warnings_to_stderr(&result.diagram_warnings);
                let exit_code = determine_exit_code(cli.strict_diagrams, &result.diagram_warnings);
                if exit_code != 0 {
                    prism::profile::exit(exit_code);
                }
            }
            "paper" => {
                let paper_output = output::to_paper_format(&result.blocks);
                println!("{}", serde_json::to_string_pretty(&paper_output)?);
                if let Some(diagnostics) = &diagnostics {
                    cli_output::write_messages(
                        &mut std::io::stderr().lock(),
                        &diagnostics.messages(),
                    )?;
                }
                emit_warnings_to_stderr(&result.diagram_warnings);
                let exit_code = determine_exit_code(cli.strict_diagrams, &result.diagram_warnings);
                if exit_code != 0 {
                    prism::profile::exit(exit_code);
                }
            }
            "sarif" => {
                // Single-run: no AlgorithmError can exist (a failing algorithm
                // is a hard `?` above), so `errors` keeps the builder's empty
                // default. Keep original warning channels separate from notes.
                let algorithms_run = vec![algorithm.name().to_string()];
                let document = {
                    let mut sarif = output::SarifInputs::new(&result.findings)
                        .evidence(&result.evidence)
                        .parse_warnings(&inputs.parse_warnings)
                        .load_warnings(&inputs.load_warnings)
                        .build_warnings(&built.warnings)
                        .algorithms_run(&algorithms_run)
                        .parse_quality(&inputs.parse_quality)
                        .files(&inputs.files)
                        .sources(&inputs.sources)
                        .min_confidence(cli.min_confidence)
                        .resolution(cli.resolution);
                    if let Some(diagnostics) = &diagnostics {
                        sarif = sarif.boundary_diagnostics(diagnostics);
                    }
                    if let Some(context) = &context {
                        sarif = sarif.context(context);
                    }
                    output::to_sarif(&sarif)
                };
                println!("{}", serde_json::to_string_pretty(&document)?);
                emit_warnings_to_stderr(&result.diagram_warnings);
                let exit_code = determine_exit_code(cli.strict_diagrams, &result.diagram_warnings);
                if exit_code != 0 {
                    prism::profile::exit(exit_code);
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
                if let Some(diagnostics) = &diagnostics {
                    cli_output::write_messages(
                        &mut std::io::stderr().lock(),
                        &diagnostics.messages(),
                    )?;
                }
                let warnings = multi_result.aggregate_diagram_warnings();
                emit_warnings_to_stderr(&warnings);
                let exit_code = determine_exit_code(cli.strict_diagrams, &warnings);
                if exit_code != 0 {
                    prism::profile::exit(exit_code);
                }
            }
            _ => {
                for w in &result.warnings {
                    eprintln!("WARNING: {}", w);
                }
                let view = ResultView::for_result(context.as_ref(), &result, &inputs);
                print!(
                    "{}",
                    output::format_slice_result(&result.blocks, view.sources())
                );
                emit_warnings_to_stderr(&result.diagram_warnings);
                let exit_code = determine_exit_code(cli.strict_diagrams, &result.diagram_warnings);
                if exit_code != 0 {
                    prism::profile::exit(exit_code);
                }
            }
        }
    }

    Ok(())
}

#[cfg(test)]
mod cli_parse_tests {
    // `parse_diagram_cap`'s own unit tests moved with it to `src/cli.rs`
    // (spec §2.5.8 / §6 pure move): they now live in `prism_review::cli::tests`.

    /// Architecture pin: `run_slicing_inner` must be a public symbol in
    /// `prism_review::algorithms`.  The `run_algorithm` fallback path calls it so that
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
        let _ = prism_review::algorithms::run_slicing_inner
            as fn(
                &prism::cpg::CpgContext,
                &prism::diff::DiffInput,
                &prism::slice::SliceConfig,
            ) -> anyhow::Result<prism::slice::SliceResult>;
    }
}
