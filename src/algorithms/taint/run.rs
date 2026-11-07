//! The taint slice entry point.

use crate::algorithms::taint::cleansing::{
    apply_cleansers, cleansed_structured_sink_call_ranges, flow_path_cleansed_for_sink,
    node_in_ranges, source_line_cleansed_for_sink, structured_sink_line_cleansed_for_path,
};
use crate::algorithms::taint::flat::{
    detect_format_string_wrappers, detect_ipc_sources, detect_unquoted_expansions, SINK_PATTERNS,
};
use crate::algorithms::taint::go::sinks::{
    find_sink_with_inline_framework_source, go_sink_outcome,
};
use crate::algorithms::taint::js_ts::sinks::{
    js_ts_flat_sink_identifier_has_tainted_request_source,
    js_ts_inline_framework_source_yaml_sink_lines,
};
use crate::algorithms::taint::js_ts::source_access::{
    js_ts_framework_source_access_ranges_by_line, js_ts_request_source_identifier,
};
use crate::algorithms::taint::js_ts::source_ranges::js_ts_framework_source_target_ranges_by_line;
use crate::algorithms::taint::python::sinks::python_sink_with_inline_flask_source;
use crate::algorithms::taint::report::{
    build_taint_chain_diagram, keep_best_evidence, source_line_text,
};
use crate::algorithms::taint::seeds::{detect_framework_sources, synthesize_target_seed_paths};
use crate::algorithms::taint::sink_match::{structured_sink_outcome, SinkMatchOutcome};
use crate::algorithms::taint::{matches_sink, TaintConfig, TaintSeed};
use crate::algorithms::taint_sinks;
use crate::ast::tree_helpers::is_js_ts_language;
use crate::cpg::CpgContext;
use crate::data_flow::FlowPath;
use crate::diff::{DiffBlock, DiffInput, ModifyType};
use crate::finding_confidence::EvidencePath;
use crate::languages::Language;
use crate::slice::{SliceFinding, SliceResult, SlicingAlgorithm};
use anyhow::Result;
use std::collections::{BTreeMap, BTreeSet};

pub fn slice(
    ctx: &CpgContext,
    diff: &DiffInput,
    taint_config: &TaintConfig,
) -> Result<SliceResult> {
    let mut result = SliceResult::new(SlicingAlgorithm::Taint);

    // Collect taint sources. Line-scoped seeds preserve existing behavior;
    // target-scoped seeds are used by framework handler parameters.
    let mut taint_seeds: Vec<TaintSeed> = taint_config
        .sources
        .iter()
        .map(|(file, line)| TaintSeed::line(file.clone(), *line))
        .collect();

    if taint_config.taint_from_diff {
        for diff_info in &diff.files {
            // Files prism cannot parse (e.g. Cargo.toml, .md) are never keys
            // of `ctx.files`, so they can never be traced; seeding them only
            // produces an untraceable `taint_source` finding (P1 Change 1).
            if !ctx.files.contains_key(&diff_info.file_path) {
                continue;
            }
            for &line in &diff_info.diff_lines {
                taint_seeds.push(TaintSeed::line(diff_info.file_path.clone(), line));
            }
        }
    }

    // Add GLib/D-Bus IPC accessor lines as explicit taint sources.
    // This enables tracing confused-deputy paths where user-controlled IPC data
    // (e.g. from `g_hash_table_lookup(settings->data, "usercert")`) flows into
    // a privileged sink (e.g. `BUILD_FROM_FILE`) that runs as root.
    // These are additional sources that extend (not replace) diff-line sources.
    let ipc_sources: Vec<(String, usize)> = detect_ipc_sources(ctx, diff);
    for ipc_src in &ipc_sources {
        taint_seeds.push(TaintSeed::line(ipc_src.0.clone(), ipc_src.1));
    }
    let ipc_source_set: BTreeSet<(String, usize)> = ipc_sources.into_iter().collect();

    // Add framework-aware taint sources (Phase 1 Go: net/http, gin, gorilla/mux).
    // For each Go file with a detected framework, every call to a framework
    // SourcePattern (`c.Query`, `r.URL.Query`, `mux.Vars`, …) is a taint source.
    // These extend (not replace) diff-line and IPC sources.
    let framework_sources: Vec<TaintSeed> = detect_framework_sources(ctx);
    for fw_src in &framework_sources {
        taint_seeds.push(fw_src.clone());
    }
    taint_seeds.sort();
    taint_seeds.dedup();
    let taint_sources: Vec<(String, usize)> = taint_seeds
        .iter()
        .filter(|s| s.target.is_none())
        .map(|s| (s.file.clone(), s.line))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    // Lines whose identifiers are recognized framework SOURCE calls or JS/TS
    // request-data accesses (e.g. `r.URL.Query()`, `c.Query()`,
    // `req.query.term`). These overlap textually with the cross-language flat
    // sink registry — `Query` is in SINK_PATTERNS as a generic `sql.Query`
    // substring matcher — so without this set, a tainted source line would
    // double-fire as a sink. Used during sink evaluation to suppress flat
    // substring matches on lines positively identified as sources; structured
    // sink checks still run for true source==sink lines.
    let framework_source_set: BTreeSet<(String, usize)> = framework_sources
        .iter()
        .filter(|s| {
            s.target.is_none()
                || ctx
                    .files
                    .get(&s.file)
                    .is_some_and(|parsed| is_js_ts_language(parsed.language))
        })
        .map(|s| (s.file.clone(), s.line))
        .collect();
    let framework_source_target_ranges =
        js_ts_framework_source_target_ranges_by_line(ctx.files, &framework_sources);
    let framework_source_access_ranges =
        js_ts_framework_source_access_ranges_by_line(ctx.files, &framework_source_set);

    // Forward propagation from each source (CFG-constrained when available)
    let (mut paths, flow_evidence) = if taint_config.return_flow {
        ctx.cpg
            .taint_forward_cfg_with_return_flow_labeled(&taint_sources)
    } else {
        ctx.cpg.taint_forward_cfg_labeled(&taint_sources)
    };
    synthesize_target_seed_paths(&framework_sources, ctx, &mut paths);

    let mut evidence_by_lines: BTreeMap<(String, usize, String, usize), EvidencePath> =
        BTreeMap::new();
    for path in &paths {
        for edge in &path.edges {
            let evidence = flow_evidence
                .get(&(edge.from.clone(), edge.to.clone()))
                .cloned()
                .unwrap_or_else(|| EvidencePath::unlabeled_for("taint"));
            keep_best_evidence(
                &mut evidence_by_lines,
                (
                    edge.from.file.clone(),
                    edge.from.line,
                    edge.to.file.clone(),
                    edge.to.line,
                ),
                evidence,
            );
        }
    }

    // Sanitizer propagation hook (spec §3.6): for each path, walk the function
    // body containing its source and mark `cleansed_for` for any cleanser whose
    // call_path occurs there (with paired_check satisfied if required). This
    // happens after path construction but before sink evaluation so the
    // suppression check below can consult the cleansed-for set.
    for path in &mut paths {
        apply_cleansers(path, ctx.files);
    }

    // Detect variadic wrapper functions and add them as dynamic sinks
    let wrapper_sinks = detect_format_string_wrappers(ctx.files);

    // Collect all tainted lines and identify sinks
    let mut all_tainted: BTreeMap<String, BTreeSet<usize>> = BTreeMap::new();
    let mut sink_lines: BTreeSet<(String, usize)> = BTreeSet::new();
    // For each sink location, the set of source locations that reach it via a FlowPath.
    // Empty set means "added by non-path code" — caller falls back to file-scan heuristic.
    let mut sink_to_path_sources: BTreeMap<(String, usize), BTreeSet<(String, usize)>> =
        BTreeMap::new();
    // Selected witness retained at the moment a FlowPath target is accepted as
    // a sink. This is independent of the same-file source chosen for legacy
    // display text.
    let mut sink_to_path_evidence: BTreeMap<(String, usize), EvidencePath> = BTreeMap::new();

    // Flat built-in sinks are decided by `taint_sinks` (call position and
    // receiver, #622 S1); caller-supplied and wrapper sinks keep name matching.
    let sink_classifier = taint_sinks::SinkClassifier::default();
    let extra_sinks: Vec<&str> = taint_config
        .extra_sinks
        .iter()
        .map(|s| s.as_str())
        .chain(wrapper_sinks.iter().map(|s| s.as_str()))
        .collect();

    for path in &paths {
        for edge in &path.edges {
            all_tainted
                .entry(edge.from.file.clone())
                .or_default()
                .insert(edge.from.line);
            all_tainted
                .entry(edge.to.file.clone())
                .or_default()
                .insert(edge.to.line);

            // Check if the target location involves a sink
            if let Some(parsed) = ctx.files.get(&edge.to.file) {
                // Consult the structured Go sink registry. Three outcomes:
                // - `Match(p)`: structured sink fires (modulo cleanser
                //   suppression by category).
                // - `SemanticallyExcluded`: a structured pattern's call_path
                //   matched but did not fire on this path. This outcome is NOT
                //   used to suppress every flat-pattern catch-all on the line —
                //   see PR #73 review feedback (P1: unmodeled shells; P2:
                //   unrelated same-line sinks). Cleanser suppression below is
                //   scoped to identifiers inside the cleansed structured call.
                // - `NoMatch`: no structured opinion; flat-pattern catch-all
                //   runs normally.
                //
                // Path-aware cleanser suppression (spec §3.7): if `Match(p)` and
                // the path is cleansed for `p.category`, suppress the structured
                // finding and flat identifiers inside that structured call. Do
                // not suppress unrelated flat sinks that share the same line.
                let outcome = structured_sink_outcome(parsed, edge.to.line, Some(path));
                let cleansed_structured_ranges =
                    cleansed_structured_sink_call_ranges(parsed, &ctx.cpg, edge.to.line, path);
                let structured_suppressed_by_cleanser = match outcome {
                    SinkMatchOutcome::Match(p) => structured_sink_line_cleansed_for_path(
                        parsed,
                        &ctx.cpg,
                        path,
                        edge.to.line,
                        p,
                    ),
                    _ => false,
                };

                // Suppress flat substring matches on lines that are recognized
                // framework SOURCE calls — e.g., `Query` would otherwise fire
                // as a flat sink on `r.URL.Query()` even though that's the
                // source, not a sink. Structured Go sinks (which we know are
                // sinks, not sources) are not affected by this filter.
                let is_framework_source_line =
                    framework_source_set.contains(&(edge.to.file.clone(), edge.to.line));
                let framework_source_access_ranges =
                    framework_source_access_ranges.get(&(edge.to.file.clone(), edge.to.line));
                let framework_source_target_ranges =
                    framework_source_target_ranges.get(&(edge.to.file.clone(), edge.to.line));
                let is_js_ts_framework_source_line =
                    is_framework_source_line && is_js_ts_language(parsed.language);
                let ids = parsed.identifiers_on_line(edge.to.line);
                for id in &ids {
                    if node_in_ranges(id, &cleansed_structured_ranges) {
                        continue;
                    }
                    let text = parsed.node_text(id);
                    if is_framework_source_line
                        && (!is_js_ts_language(parsed.language)
                            || js_ts_request_source_identifier(text)
                            || framework_source_target_ranges
                                .is_some_and(|ranges| node_in_ranges(id, ranges))
                            || framework_source_access_ranges
                                .is_some_and(|ranges| node_in_ranges(id, ranges)))
                    {
                        continue;
                    }
                    let flat_sink = sink_classifier
                        .classify(&edge.to.file, parsed, *id, text, SINK_PATTERNS)
                        .is_some();
                    if !flat_sink && !extra_sinks.iter().any(|s| matches_sink(text, s)) {
                        continue;
                    }
                    if is_js_ts_framework_source_line
                        && !js_ts_flat_sink_identifier_has_tainted_request_source(
                            parsed,
                            id,
                            path,
                            framework_source_access_ranges
                                .map(|ranges| ranges.as_slice())
                                .unwrap_or(&[]),
                        )
                    {
                        continue;
                    }
                    {
                        let sink = (edge.to.file.clone(), edge.to.line);
                        sink_lines.insert(sink.clone());
                        if let Some(first_edge) = path.edges.first() {
                            sink_to_path_sources
                                .entry(sink.clone())
                                .or_default()
                                .insert((first_edge.from.file.clone(), first_edge.from.line));
                        }
                        if let Some(evidence) = evidence_by_lines
                            .get(&(
                                edge.from.file.clone(),
                                edge.from.line,
                                edge.to.file.clone(),
                                edge.to.line,
                            ))
                            .cloned()
                        {
                            keep_best_evidence(&mut sink_to_path_evidence, sink, evidence);
                        }
                    }
                }

                // Structured sinks (Go Phase 1 plus Python Phase 2).
                if matches!(outcome, SinkMatchOutcome::Match(_))
                    && !structured_suppressed_by_cleanser
                {
                    let sink = (edge.to.file.clone(), edge.to.line);
                    sink_lines.insert(sink.clone());
                    if let Some(first_edge) = path.edges.first() {
                        sink_to_path_sources
                            .entry(sink.clone())
                            .or_default()
                            .insert((first_edge.from.file.clone(), first_edge.from.line));
                    }
                    if let Some(evidence) = evidence_by_lines
                        .get(&(
                            edge.from.file.clone(),
                            edge.from.line,
                            edge.to.file.clone(),
                            edge.to.line,
                        ))
                        .cloned()
                    {
                        keep_best_evidence(&mut sink_to_path_evidence, sink, evidence);
                    }
                }
            }
        }
    }

    for (file, parsed) in ctx.files {
        if is_js_ts_language(parsed.language) {
            for line in js_ts_inline_framework_source_yaml_sink_lines(parsed) {
                sink_lines.insert((file.clone(), line));
            }
        }
    }

    // Source lines themselves are taint-bearing — a structured sink on the
    // exact source line still fires. (E.g., `c.File(c.Param("f"))` — the
    // c.Param source and the c.File sink share a line.)
    //
    // Three branches (see design note §2.3):
    //   1. originating.is_empty() → primary Option::None fallback for pure
    //      source==sink shapes (no FlowEdge connects source to sink because
    //      they share a line and the source result is consumed inline).
    //   2. originating non-empty → per-path Match-and-cleansing combined loop.
    //      Fire iff at least one matching path is uncleansed. Non-matching
    //      paths are skipped — their cleansing state is irrelevant because
    //      per-arg DFG already says they don't fire this sink.
    //   3. originating non-empty + branch 2 found no fire → secondary inline-
    //      source fallback for mixed same-line shapes (the line hosts both a
    //      non-inline source driving the FlowPath AND an inline source==sink
    //      shape that conservative recursion can't recognize).
    for (file, line) in &taint_sources {
        if let Some(parsed) = ctx.files.get(file) {
            if parsed.language == Language::Python {
                if let Some(pat) = python_sink_with_inline_flask_source(parsed, *line) {
                    let cleansed = source_line_cleansed_for_sink(parsed, &ctx.cpg, *line, pat);
                    if !cleansed {
                        sink_lines.insert((file.clone(), *line));
                    }
                }
                continue;
            }
            if parsed.language != Language::Go {
                continue;
            }
            // Find every path whose source is this (file, line).
            let originating: Vec<&FlowPath> = paths
                .iter()
                .filter(|p| {
                    p.edges
                        .first()
                        .map(|e| e.from.file == *file && e.from.line == *line)
                        .unwrap_or(false)
                })
                .collect();

            if originating.is_empty() {
                // No FlowPath originates — pure source==sink shape (e.g.
                // c.File(c.Param("f"))). Pass None to skip per-arg DFG; the
                // engine falls back to call_path + semantic_check matching.
                // Preserves today's source==sink behavior.
                let sink_pat = match go_sink_outcome(parsed, *line, None) {
                    SinkMatchOutcome::Match(p) => p,
                    SinkMatchOutcome::SemanticallyExcluded | SinkMatchOutcome::NoMatch => continue,
                };
                // No FlowPath cleansing applies. Fall back to function-body scan.
                let cleansed = source_line_cleansed_for_sink(parsed, &ctx.cpg, *line, sink_pat);
                if !cleansed {
                    sink_lines.insert((file.clone(), *line));
                }
            } else {
                // Per-arg DFG applies. Walk originating paths; for each that
                // Matches, check whether its FlowPath is cleansed for the
                // matched category. Fire iff AT LEAST ONE matching path is
                // not cleansed.
                //
                // Crucially, we ONLY consult cleansing for paths that actually
                // Match (per-arg DFG: relevant args are tainted on this path).
                // A non-matching path's cleansing state is irrelevant — per-arg
                // DFG already says it doesn't fire this sink, so its cleansing-
                // or-not can't move the decision.
                let mut any_matching_uncleansed = false;
                for p in &originating {
                    if let SinkMatchOutcome::Match(pat) = go_sink_outcome(parsed, *line, Some(p)) {
                        if !flow_path_cleansed_for_sink(parsed, &ctx.cpg, p, *line, pat) {
                            any_matching_uncleansed = true;
                            break;
                        }
                    }
                }

                // Secondary inline-source fallback for mixed same-line shapes.
                // The line may host both a non-inline source (driving an
                // originating FlowPath) AND an inline source==sink shape (e.g.
                // `c.File(c.Param("f"))`) that the primary per-arg DFG can't
                // recognize because its conservative recursion only checks
                // identifiers against FlowPath edges, and the inline c.Param
                // call generates no FlowEdge.
                //
                // The helper scans ALL (call, sink_pat) combinations on the
                // line and returns the first sink pattern whose tainted_arg
                // subtree contains an inline framework-source call. This is
                // intentionally NOT routed through go_sink_outcome's first-
                // match-wins aggregation — that would only consider the first
                // matching sink and miss inline shapes when an unrelated
                // structured sink earlier on the line shadows them.
                if !any_matching_uncleansed {
                    if let Some(pat) = find_sink_with_inline_framework_source(parsed, *line) {
                        let cleansed = source_line_cleansed_for_sink(parsed, &ctx.cpg, *line, pat);
                        if !cleansed {
                            any_matching_uncleansed = true;
                        }
                    }
                }

                if any_matching_uncleansed {
                    sink_lines.insert((file.clone(), *line));
                }
                // else: every matching path is cleansed (or no path matches
                // AND no inline source==sink shape was detected). Suppress.
            }
        }
    }

    // Also check source lines for sinks (taint at source)
    for (file, line) in &taint_sources {
        all_tainted.entry(file.clone()).or_default().insert(*line);
    }

    // Sources whose location was actually selected as the reported origin of
    // an emitted sink finding (P1 Change 2). `taint_source` findings are only
    // emitted for members of this set — built up below as each sink finding's
    // `source_location` is resolved — instead of unconditionally for every
    // seed (most diff-line seeds never reach a sink and are noise).
    let mut sources_with_emitted_sinks: BTreeSet<(String, usize)> = BTreeSet::new();
    let mut evidence_for_sources: BTreeMap<(String, usize), EvidencePath> = BTreeMap::new();

    // Emit findings for each taint sink reached
    for (file, line) in &sink_lines {
        // Find a source that reaches this sink.
        // Prefer a source that the actual FlowPath identified as flowing into this sink.
        // Pick the one nearest to the sink (largest line number that's still <= sink line)
        // to remain consistent with the existing "nearest-in-file" semantics for the
        // reviewer-friendly source description.
        let path_derived_source: Option<(&str, usize)> = sink_to_path_sources
            .get(&(file.clone(), *line))
            .and_then(|set| {
                set.iter()
                    .filter(|(sf, sl)| sf == file && *sl <= *line)
                    .max_by_key(|(_, sl)| *sl)
                    // #602: no same-file path source -> name a cross-file one.
                    .or_else(|| set.iter().find(|(sf, _)| sf != file))
                    .map(|(sf, sl)| (sf.as_str(), *sl))
            });

        // Fall back to existing heuristic for sinks added without path linkage
        // (e.g., YAML inline-framework-source sinks, IPC sources).
        // Prefer the nearest IPC source before the sink (user-controlled IPC reads
        // are the semantically interesting starting point for confused-deputy analysis).
        // Then fall back to the nearest diff-line source, then any source in the same file.
        let source_location: Option<(&str, usize)> = path_derived_source.or_else(|| {
            ipc_source_set
                .iter()
                .filter(|(sf, sl)| sf == file && *sl < *line)
                .max_by_key(|(_, sl)| *sl)
                .map(|(sf, sl)| (sf.as_str(), *sl))
                .or_else(|| {
                    taint_sources
                        .iter()
                        .filter(|(sf, sl)| sf == file && *sl < *line)
                        .max_by_key(|(_, sl)| *sl)
                        .map(|(sf, sl)| (sf.as_str(), *sl))
                })
                .or_else(|| {
                    taint_sources
                        .iter()
                        .find(|(sf, _)| sf == file)
                        .map(|(sf, sl)| (sf.as_str(), *sl))
                })
        });
        // Every path-derived source recorded for this sink genuinely reached
        // it via a real FlowPath — including cross-file ones. `path_derived_source`
        // above is filtered to the same file as the sink (a reviewer-friendly
        // "nearest in this file" pick for `source_desc`); it must not gate
        // which sources earn a `taint_source` finding, or a source that
        // reaches a sink in a *different* file (e.g. a cross-file call chain)
        // is silently dropped (P1 review-fix F1). Union in the whole set.
        if let Some(set) = sink_to_path_sources.get(&(file.clone(), *line)) {
            for (sf, sl) in set {
                sources_with_emitted_sinks.insert((sf.clone(), *sl));
                let evidence = evidence_by_lines
                    .get(&(sf.clone(), *sl, file.clone(), *line))
                    .cloned()
                    .unwrap_or_else(|| EvidencePath::unlabeled_for("taint"));
                keep_best_evidence(&mut evidence_for_sources, (sf.clone(), *sl), evidence);
            }
        }

        // Record the exact source location chosen for this sink finding:
        // the path-derived source unconditionally if available, otherwise
        // the fallback source only if it is itself a member of
        // `taint_sources` (the ipc/nearest-in-file fallbacks all resolve to
        // members of `taint_sources`, but guard explicitly per spec).
        match path_derived_source {
            Some((sf, sl)) => {
                sources_with_emitted_sinks.insert((sf.to_string(), sl));
            }
            None => {
                if let Some((sf, sl)) = source_location {
                    if taint_sources.iter().any(|(f, l)| f == sf && *l == sl) {
                        sources_with_emitted_sinks.insert((sf.to_string(), sl));
                        keep_best_evidence(
                            &mut evidence_for_sources,
                            (sf.to_string(), sl),
                            EvidencePath::unlabeled_for("taint"),
                        );
                    }
                }
            }
        }

        let source_desc = source_location
            .map(|(sf, sl)| {
                if sf == file.as_str() {
                    format!("line {}", sl)
                } else {
                    format!("{}:{}", sf, sl)
                }
            })
            .unwrap_or_else(|| "diff lines".to_string());

        // Build a Chain diagram for this sink finding.
        // If we have an identified source location, use it as the Source node;
        // otherwise fall back to a sink-only diagram (Source == Sink as the taint origin).
        let sink_text = ctx
            .files
            .get(file.as_str())
            .map(|p| source_line_text(&p.source, *line))
            .unwrap_or_default();
        let diagram = if let Some((src_file, src_line)) = source_location {
            let src_text = ctx
                .files
                .get(src_file)
                .map(|p| source_line_text(&p.source, src_line))
                .unwrap_or_default();
            build_taint_chain_diagram(src_file, src_line, &src_text, file, *line, &sink_text, &[])
        } else {
            // No upstream source resolved; emit a single-step chain with sink only.
            // Use the sink as both source and sink nodes to satisfy the Chain contract.
            build_taint_chain_diagram(file, *line, &sink_text, file, *line, &sink_text, &[])
        };

        // #636: a sink whose argument shape defuses it is reported at `suggestion`.
        let severity = ctx.files.get(file.as_str()).map_or("warning", |parsed| {
            let is_extra = |t: &str| extra_sinks.iter().any(|s| matches_sink(t, s));
            sink_classifier
                .line_strength(file, parsed, *line, &is_extra)
                .severity()
        });
        let mut finding = SliceFinding {
            algorithm: "taint".to_string(),
            file: file.clone(),
            line: *line,
            severity: severity.to_string(),
            description: format!(
                "tainted value from {} reaches sink at line {}",
                source_desc, line
            ),
            function_name: None,
            related_lines: taint_sources
                .iter()
                .filter(|(sf, _)| sf == file)
                .map(|(_, sl)| *sl)
                .collect(),
            related_files: vec![],
            category: Some("taint_sink".to_string()),
            parse_quality: None,
            diagrams: vec![],
        };
        finding.diagrams.push(diagram);
        let finding_evidence = source_location
            .and_then(|(source_file, source_line)| {
                evidence_by_lines.get(&(source_file.to_string(), source_line, file.clone(), *line))
            })
            .or_else(|| sink_to_path_evidence.get(&(file.clone(), *line)))
            .cloned()
            .unwrap_or_else(|| EvidencePath::unlabeled_for("taint"));
        result.findings.push(finding);
        result.evidence.push(Some(finding_evidence));
    }

    // Bash-specific: detect unquoted variable expansions on tainted lines.
    // In shell, unquoted $VAR in command arguments is a word-splitting /
    // injection vector. Runs BEFORE the gated source-emission loop below
    // (P1 review-fix F2): an unquoted-expansion finding is itself a
    // sink-style finding, so — like a sink finding — it must license its
    // reaching source into `sources_with_emitted_sinks` before that loop
    // decides which `taint_source` findings to emit. Without this, a bash
    // flow whose only sink-style finding is an unquoted expansion would
    // license no source and silently drop that source's `taint_source`
    // finding (a regression vs. the pre-Change-2 behavior of emitting every
    // source unconditionally).
    let unquoted = detect_unquoted_expansions(ctx.files, &all_tainted);
    for (file, line, var_name) in &unquoted {
        // Avoid duplicate findings if line is already flagged as a sink
        if !sink_lines.contains(&(file.clone(), *line)) {
            sink_lines.insert((file.clone(), *line));

            // License the nearest preceding same-file taint source (the
            // member of `taint_sources` with the same file and the largest
            // line <= this finding's line, if any) as having reached an
            // emitted sink-style finding.
            let unquoted_source = taint_sources
                .iter()
                .filter(|(sf, sl)| sf == file && *sl <= *line)
                .max_by_key(|(_, sl)| *sl)
                .cloned();
            if let Some((sf, sl)) = &unquoted_source {
                sources_with_emitted_sinks.insert((sf.clone(), *sl));
                let evidence = evidence_by_lines
                    .get(&(sf.clone(), *sl, file.clone(), *line))
                    .cloned()
                    .unwrap_or_else(|| EvidencePath::unlabeled_for("taint"));
                keep_best_evidence(&mut evidence_for_sources, (sf.clone(), *sl), evidence);
            }

            let finding_evidence = unquoted_source
                .as_ref()
                .and_then(|(sf, sl)| evidence_by_lines.get(&(sf.clone(), *sl, file.clone(), *line)))
                .cloned()
                .unwrap_or_else(|| EvidencePath::unlabeled_for("taint"));
            result.findings.push(SliceFinding {
                algorithm: "taint".to_string(),
                file: file.clone(),
                line: *line,
                severity: "warning".to_string(),
                description: format!(
                    "unquoted expansion {} in command argument — word splitting / injection risk",
                    var_name,
                ),
                function_name: None,
                related_lines: taint_sources
                    .iter()
                    .filter(|(sf, _)| sf == file)
                    .map(|(_, sl)| *sl)
                    .collect(),
                related_files: vec![],
                category: Some("unquoted_expansion".to_string()),
                parse_quality: None,
                diagrams: vec![],
            });
            result.evidence.push(Some(finding_evidence));
        }
    }

    // Emit findings for each taint source that reaches an emitted sink-style
    // finding (P1 Change 2). Runs AFTER sink emission AND the
    // unquoted-expansion block above so `sources_with_emitted_sinks` is
    // fully populated; a source that reaches nothing contributes no finding.
    // Applies uniformly to diff-seeded, explicit `--taint-source`, IPC, and
    // framework sources alike, since all of them flow through the shared
    // `taint_sources` collection.
    for (file, line) in &taint_sources {
        if !sources_with_emitted_sinks.contains(&(file.clone(), *line)) {
            continue;
        }
        result.findings.push(SliceFinding {
            algorithm: "taint".to_string(),
            file: file.clone(),
            line: *line,
            severity: "info".to_string(),
            description: format!("taint source: origin of tainted data at line {}", line),
            function_name: None,
            related_lines: vec![],
            related_files: vec![],
            category: Some("taint_source".to_string()),
            parse_quality: None,
            diagrams: vec![],
        });
        result.evidence.push(Some(
            evidence_for_sources
                .get(&(file.clone(), *line))
                .cloned()
                .unwrap_or_else(|| EvidencePath::unlabeled_for("taint")),
        ));
    }

    // Build output blocks
    let mut block_id = 0;
    for (file, lines) in &all_tainted {
        let mut block = DiffBlock::new(block_id, file.clone(), ModifyType::Modified);

        for &line in lines {
            let is_source = taint_sources.iter().any(|(f, l)| f == file && *l == line);
            let is_sink = sink_lines.contains(&(file.clone(), line));
            // Mark sources and sinks as diff lines for highlighting
            block.add_line(file, line, is_source || is_sink);
        }

        if !lines.is_empty() {
            result.blocks.push(block);
            block_id += 1;
        }
    }

    Ok(result)
}
