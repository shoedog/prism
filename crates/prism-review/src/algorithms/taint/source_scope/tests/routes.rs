//! RS01-RS08: source admission on every route that can start a root or a finding (#602 S3a.1s).
//!
//! Kind: REAL. Paired fixtures come from `cases::route_cases()`; each test first runs the
//! all-original control, whose `Needs` evidence must appear in BOTH copies.
use super::cases::*;
use super::*;
use crate::algorithms::taint::source_scope::admission::{
    ExcludedExplicit, ExclusionReason, ScopedTaintOutcome,
};
use prism::slice::SliceFinding;

fn framework_roots<'a>(outcome: &'a ScopedTaintOutcome, file: &str) -> Vec<&'a TaintRoot> {
    let roots = roots_of(outcome, file);
    roots
        .into_iter()
        .filter(|r| r.origin == Framework)
        .collect()
}

fn block_files(outcome: &ScopedTaintOutcome) -> BTreeSet<&str> {
    outcome
        .result
        .blocks
        .iter()
        .map(|b| b.file.as_str())
        .collect()
}

fn no_findings_in(outcome: &ScopedTaintOutcome, file: &str) -> bool {
    outcome.result.findings.iter().all(|f| f.file != file)
}

#[test]
fn source_scope_route_table_is_ordered_and_unique() {
    let names: Vec<_> = route_cases().iter().map(|case| case.name).collect();
    assert_eq!(names, ROUTE_NAMES, "the table is a contract for S3b-R");
    assert_eq!(names.iter().collect::<BTreeSet<_>>().len(), names.len());
    let ids: Vec<_> = ROUTE_EXCLUSIONS.iter().map(|(id, _)| *id).collect();
    assert_eq!(ids, ["RS15", "RS09 wiring", "RS10 wiring"]);
    for case in route_cases() {
        let loaded = case.loaded_set();
        let unloaded: Vec<_> = case
            .original
            .iter()
            .filter(|f| !loaded.contains(**f))
            .collect();
        let named = case.name == "explicit";
        assert_eq!(unloaded.is_empty(), !named, "{}: {unloaded:?}", case.name);
        assert!(case.original.contains(&case.original_path()));
    }
}

#[test]
fn source_scope_filters_diff_files_and_capture() {
    let case = route("diff_roots");
    let all = case.all_original();
    let diff_in = |o: &ScopedTaintOutcome, f: &str| {
        roots_of(o, f).iter().filter(|r| r.origin == Diff).count()
    };
    let msg = "inadmissible precondition: Diff roots in both copies";
    assert!(
        diff_in(&all, "app.js") > 0 && diff_in(&all, "ctx/app.js") > 0,
        "{msg}"
    );
    assert_eq!(
        diff_in(&all, "gone.js"),
        0,
        "an unloaded diff path never roots"
    );
    assert_eq!(
        block_files(&all),
        BTreeSet::from(["app.js", "ctx/app.js"]),
        "control keeps both"
    );

    let outcome = case.scoped();
    let kept = snaps(roots_of(&all, "app.js"));
    assert!(!kept.is_empty());
    assert_eq!(
        snaps(&outcome.trace.roots),
        kept,
        "original roots only, unchanged"
    );
    let isolated = |r: &&TaintRoot| r.location.path.base == "x" && r.reached.is_empty();
    assert!(outcome
        .trace
        .roots
        .iter()
        .filter(isolated)
        .any(|r| r.origin == Diff));
    assert!(roots_of(&outcome, "ctx/app.js").is_empty());
    assert_eq!(
        block_files(&outcome),
        BTreeSet::from(["app.js"]),
        "result seeds"
    );
    assert!(
        outcome.excluded_explicit.is_empty(),
        "diff paths are never exclusions"
    );
    let eligible = outcome
        .trace
        .roots
        .iter()
        .filter(|r| matches!(r.origin, Diff | Explicit));
    assert_eq!(snaps(&outcome.loading.roots), snaps(eligible));
}

#[test]
fn source_scope_accounts_for_explicit_requests() {
    let case = route("explicit");
    let all = case.all_original();
    let explicit_in = |f: &str| roots_of(&all, f).iter().any(|r| r.origin == Explicit);
    assert!(
        explicit_in("app.js") && explicit_in("ctx/app.js"),
        "inadmissible precondition"
    );

    let outcome = case.scoped();
    let refused = |file: &str, line, reason| ExcludedExplicit {
        file: file.into(),
        line,
        reason,
    };
    let (outside, not_loaded) = (
        ExclusionReason::OutsideOriginalFiles,
        ExclusionReason::NotLoaded,
    );
    let expected = vec![
        refused("ctx/app.js", 2, outside),
        refused("missing.js", 7, not_loaded),
        refused("missing2.js", 9, not_loaded),
        refused("ctx/app.js", 2, outside),
    ];
    assert_eq!(
        outcome.excluded_explicit, expected,
        "input order, repeats kept"
    );
    let roots = &outcome.trace.roots;
    assert!(roots
        .iter()
        .all(|r| r.origin == Explicit && r.seed.file == "app.js"));
    assert!(roots
        .iter()
        .any(|r| r.seed.line == 2 && r.location.path.base == "x"));
    assert!(
        roots.iter().all(|r| r.seed.line != 3),
        "no variable: no root, no exclusion"
    );
    assert_eq!(
        block_files(&outcome),
        BTreeSet::from(["app.js"]),
        "no file loaded for a request"
    );
    assert_eq!(snaps(&outcome.loading.roots), snaps(roots));
}

/// Seed metadata of each root, in order: target, start byte and both optional scopes.
fn seed_meta<'a>(roots: impl IntoIterator<Item = &'a TaintRoot>) -> Vec<String> {
    roots
        .into_iter()
        .map(|r| {
            let seed = &r.seed;
            let meta = (&seed.target, seed.start_byte, seed.scope, seed.byte_scope);
            format!("{meta:?}")
        })
        .collect()
}

#[test]
fn source_scope_retains_original_targeted_framework_seeds() {
    // Contract: a targeted Framework root of an original file keeps its metadata exactly
    // (target, optional scope, start bytes). `scope` is optional: the all-original Express
    // roots carry one, the targeted Django roots do not (`None` on the literal base too), so
    // `Some` is required only for Express, and only after the all-original run shows it.
    for name in ["express_js", "express_ts", "django"] {
        let case = route(name);
        let all = case.all_original();
        for (path, _) in &case.files {
            let target = |r: &&TaintRoot| r.origin == Framework && r.seed.target.is_some();
            let msg =
                format!("inadmissible precondition: {name}: no targeted Framework root in {path}");
            assert!(roots_of(&all, path).iter().any(target), "{msg}");
        }
        let scope_some = name != "django";
        if scope_some {
            let original = roots_of(&all, case.original_path());
            let msg = format!("inadmissible precondition: {name}: no scoped targeted root");
            assert!(
                original
                    .iter()
                    .any(|r| r.seed.target.is_some() && r.seed.scope.is_some()),
                "{msg}"
            );
        }
        let outcome = case.scoped();
        let kept = snaps(roots_of(&all, case.original_path()));
        assert_eq!(
            snaps(&outcome.trace.roots),
            kept,
            "{name}: target, scope and bytes equal"
        );
        assert_eq!(
            seed_meta(&outcome.trace.roots),
            seed_meta(roots_of(&all, case.original_path())),
            "{name}: seed metadata equals the all-original roots"
        );
        let roots = &outcome.trace.roots;
        assert!(roots
            .iter()
            .all(|r| r.origin == Framework && r.seed.file == case.original_path()));
        assert!(roots.iter().any(|r| r.seed.target.is_some()), "{name}");
        if scope_some {
            let msg = format!("{name}: the original scope is kept");
            assert!(
                roots
                    .iter()
                    .any(|r| r.seed.target.is_some() && r.seed.scope.is_some()),
                "{msg}"
            );
        }
        assert!(roots_of(&outcome, case.context_path()).is_empty(), "{name}");
    }
}

#[test]
fn source_scope_filters_targetless_framework_seeds() {
    for name in ["gin_query", "flask_inline"] {
        let case = route(name);
        let all = case.all_original();
        for (path, _) in &case.files {
            let records = framework_roots(&all, path);
            let msg = format!("inadmissible precondition: {name}: targetless Framework in {path}");
            assert!(
                !records.is_empty() && records.iter().all(|r| r.seed.target.is_none()),
                "{msg}"
            );
        }
        let outcome = case.scoped();
        assert_eq!(
            snaps(&outcome.trace.roots),
            snaps(framework_roots(&all, case.original_path()))
        );
        assert!(!outcome.trace.roots.is_empty());
        assert!(no_findings_in(&outcome, case.context_path()), "{name}");
        assert!(
            outcome.loading.roots.is_empty(),
            "framework roots never cause loading"
        );
    }
}

#[test]
fn source_scope_filters_ipc_roots_and_fallback() {
    // Source-scope control, not a C/C++ expansion claim.
    let case = route("ipc_c");
    let all = case.all_original();
    let accessor = line_of(case.source(), "g_hash_table_lookup");
    let sink = line_of(case.source(), "system(x)");
    for (path, _) in &case.files {
        let ipc = roots_of(&all, path)
            .iter()
            .any(|r| r.origin == Ipc && r.seed.line == accessor);
        let msg = format!("inadmissible precondition: Ipc root and sink in {path}");
        assert!(ipc && sink_lines(&all.result, path) == [sink], "{msg}");
    }
    let outcome = case.scoped();
    let roots = &outcome.trace.roots;
    assert!(!roots.is_empty());
    assert!(roots
        .iter()
        .all(|r| r.origin == Ipc && r.seed.file == "app.c" && r.seed.line == accessor));
    assert_eq!(sink_lines(&outcome.result, "app.c"), [sink]);
    let named = |f: &SliceFinding| f.description.contains(&format!("line {accessor}"));
    let findings = outcome.result.findings.iter().filter(|f| f.file == "app.c");
    assert!(findings
        .clone()
        .filter(|f| f.category.as_deref() == Some("taint_sink"))
        .all(named));
    assert!(findings
        .clone()
        .any(|f| f.category.as_deref() == Some("taint_source") && f.line == accessor));
    assert!(
        no_findings_in(&outcome, "ctx/app.c"),
        "no context finding names the accessor"
    );
    // Control (inherited): with an empty diff there is no Ipc root at all.
    let empty = RouteCase {
        diff: vec![],
        ..case
    };
    assert!(empty.scoped().trace.roots.is_empty());
}

#[test]
fn source_scope_guards_independent_inline_yaml() {
    for name in ["yaml_js", "yaml_ts"] {
        let case = route(name);
        let all = case.all_original();
        let line = line_of(case.source(), "return load(req.body.payload)");
        let msg = format!("inadmissible precondition: {name}: both copies fire");
        assert_eq!(
            sink_lines(&all.result, case.context_path()),
            [line],
            "{msg}"
        );
        let outcome = case.scoped();
        assert_eq!(
            sink_lines(&outcome.result, case.original_path()),
            [line],
            "{name}"
        );
        assert!(
            no_findings_in(&outcome, case.context_path()),
            "{name}: context copy"
        );
        let literal = route(&format!("{name}_literal"));
        let msg = "inadmissible precondition: a literal payload is inert";
        assert!(literal.all_original().result.findings.is_empty(), "{msg}");
        assert!(
            literal.scoped().result.findings.is_empty(),
            "{name}: literal"
        );
    }
}

#[test]
fn source_scope_filters_python_inline_source_sink() {
    let case = route("flask_inline");
    let all = case.all_original();
    let line = line_of(case.source(), "return render_template_string");
    for (path, _) in &case.files {
        let on_sink: Vec<_> = framework_roots(&all, path)
            .into_iter()
            .filter(|r| r.seed.line == line)
            .collect();
        let msg = "inadmissible precondition: targetless request seed on the sink line";
        assert!(
            !on_sink.is_empty() && on_sink.iter().all(|r| r.seed.target.is_none()),
            "{msg}"
        );
        assert_eq!(sink_lines(&all.result, path), [line]);
    }
    let outcome = case.scoped();
    assert_eq!(
        sink_lines(&outcome.result, "app.py"),
        [line],
        "original positive"
    );
    assert!(no_findings_in(&outcome, "ctx/app.py"), "context negative");
    // Original autoescaped negative; its Framework seed exists (`needs`), so it is not vacuous.
    let escaped = route("flask_escaped");
    assert!(sink_lines(&escaped.all_original().result, "app.py").is_empty());
    assert!(sink_lines(&escaped.scoped().result, "app.py").is_empty());
}

#[test]
fn source_scope_filters_go_inline_source_sink_routes() {
    let originating = |o: &ScopedTaintOutcome, f: &str, line: usize| {
        let at = roots_of(o, f).into_iter().filter(|r| r.seed.line == line);
        at.filter(|r| r.origin == Framework)
            .any(|r| !r.reached.is_empty())
    };
    for (name, has_path) in [("go_param", false), ("go_mixed", true)] {
        let case = route(name);
        let all = case.all_original();
        let line = line_of(case.source(), "c.File(");
        let msg = format!("inadmissible precondition: {name}: originating paths");
        assert_eq!(originating(&all, "a.go", line), has_path, "{msg}");
        assert_eq!(
            sink_lines(&all.result, "ctx/a.go"),
            [line],
            "{name}: context copy fires"
        );
        let outcome = case.scoped();
        assert_eq!(
            sink_lines(&outcome.result, "a.go"),
            [line],
            "{name}: original positive"
        );
        assert!(
            no_findings_in(&outcome, "ctx/a.go"),
            "{name}: context negative"
        );
    }
    let literal = route("go_literal");
    assert!(sink_lines(&literal.all_original().result, "a.go").is_empty());
    assert!(
        sink_lines(&literal.scoped().result, "a.go").is_empty(),
        "constant path never fires"
    );
}
