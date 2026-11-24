use super::*;

const APP: &str = "function h(x) {\n  unknown(x);\n}\n";
const SOURCE: &str = "q := c.Query(\"q\")";

/// Graph integration control: the NameOnly D -> X edge is injected, so this proves
/// traversal and projection behavior, not parser reach. `framework_first` puts the
/// framework source on line 4 and the diff line on 5 (trace order F, D; the sink
/// finding then names D, the nearest source line). Otherwise D is line 4 and F line 5
/// (trace order D, F; the finding's selected witness is F's Exact path).
fn gin(framework_first: bool, from_diff: bool) -> (SliceResult, TaintTrace) {
    let (d_line, text) = if framework_first {
        (5, [SOURCE, "p := d"])
    } else {
        (4, ["p := d", SOURCE])
    };
    let source = format!(
        "package main\nimport \"github.com/gin-gonic/gin\"\nfunc h(c *gin.Context, d string) {{\n\t{}\n\t{}\n\tc.File(q)\n}}\n",
        text[0], text[1]
    );
    let files = parse(&[("main.go", source.as_str())]);
    let config = TaintConfig {
        taint_from_diff: from_diff,
        ..TaintConfig::default()
    };
    traced(&files, &diff("main.go", &[d_line]), &config, |ctx| {
        let from = unique_var_node(ctx, "main.go", d_line, "p", VarAccessKind::Def);
        let to = unique_var_node(ctx, "main.go", 6, "q", VarAccessKind::Use);
        add_flow(ctx, from, to, FlowConfidence::NameOnly(FlowDoubt::SameLine));
    })
}

fn reaches_x(root: &TaintRoot) -> bool {
    let weak = |e: &EvidencePath| !e.hops.is_empty() && e.hops.iter().all(|h| !h.is_exact());
    (root.reached.iter())
        .any(|r| r.location.line == 6 && r.location.path.base == "q" && weak(&r.evidence))
}

fn assert_gin_preconditions(result: &SliceResult, trace: &TaintTrace, f_line: usize) {
    let framework = trace
        .roots
        .iter()
        .any(|r| r.origin == Framework && r.seed.line == f_line);
    assert!(
        framework,
        "inadmissible precondition: no targetless Framework root on {f_line}"
    );
    assert!(trace
        .roots
        .iter()
        .filter(|r| r.origin == Framework)
        .all(|r| r.seed.target.is_none()));
    let sink = (result.findings.iter().enumerate())
        .find(|(_, f)| f.category.as_deref() == Some("taint_sink") && f.line == 6);
    assert!(
        sink.is_some(),
        "inadmissible precondition: no taint_sink finding on line 6"
    );
}

fn assert_witness_is_framework(result: &SliceResult, f_line: usize) {
    let (index, _) = (result.findings.iter().enumerate())
        .find(|(_, f)| f.category.as_deref() == Some("taint_sink") && f.line == 6)
        .unwrap();
    let hops = &result.evidence[index]
        .as_ref()
        .expect("aligned evidence")
        .hops;
    let starts =
        matches!(hops.first(), Some(EvidenceHop::DataFlow { from, .. }) if from.line == f_line);
    let msg = "inadmissible precondition: witness is not the framework root's Exact path";
    assert!(
        starts && hops.iter().all(EvidenceHop::is_exact),
        "{msg}: {hops:?}"
    );
}

fn assert_diff_frontier_survives(framework_first: bool) {
    let (result, trace) = gin(framework_first, true);
    let (d_line, f_line) = if framework_first { (5, 4) } else { (4, 5) };
    assert_gin_preconditions(&result, &trace, f_line);
    if !framework_first {
        assert_witness_is_framework(&result, f_line);
    }
    let diff_roots: Vec<_> = trace.roots.iter().filter(|r| r.origin == Diff).collect();
    let has_x = diff_roots
        .iter()
        .any(|r| r.seed.line == d_line && reaches_x(r));
    assert!(
        has_x,
        "inadmissible precondition: no Diff root on {d_line} reaching X"
    );
    let projected = project_same(&trace, &["main.go"]);
    let expected: Vec<_> = diff_roots.iter().map(|r| snapshot(r)).collect();
    assert_eq!(
        projected.roots.iter().map(snapshot).collect::<Vec<_>>(),
        expected
    );
    assert!(projected
        .roots
        .iter()
        .any(|r| r.seed.line == d_line && reaches_x(r)));
    assert!(projected.roots.iter().all(|r| r.origin != Framework));
}

#[test]
fn loading_projection_keeps_converging_eligible_frontier() {
    assert_diff_frontier_survives(false);
}

#[test]
fn loading_projection_keeps_converging_frontier_with_reversed_order() {
    assert_diff_frontier_survives(true);
}

#[test]
fn loading_projection_of_only_framework_roots_is_empty() {
    let (result, trace) = gin(false, false);
    assert_gin_preconditions(&result, &trace, 5);
    assert_witness_is_framework(&result, 5);
    assert!(trace.roots.iter().all(|r| r.origin == Framework));
    assert!(project_same(&trace, &["main.go"]).roots.is_empty());
}

fn app_trace() -> TaintTrace {
    let files = parse(&[("app.js", APP)]);
    let config = TaintConfig {
        sources: vec![("app.js".into(), 2)],
        ..TaintConfig::default()
    };
    traced(&files, &diff("app.js", &[2]), &config, |_| {}).1
}

fn x_root(trace: &TaintTrace, origin: TaintRootOrigin) -> &TaintRoot {
    let found = (trace.roots.iter()).find(|r| r.origin == origin && r.location.path.base == "x");
    found.unwrap_or_else(|| panic!("inadmissible precondition: no {origin:?} root for x"))
}

#[test]
fn loading_projection_keeps_distinct_origins_and_occurrences() {
    let trace = app_trace();
    let (explicit, diff_root) = (x_root(&trace, Explicit), x_root(&trace, Diff));
    let bytes = |r: &TaintRoot| (r.location.start_byte, r.location.end_byte);
    assert_eq!(
        bytes(explicit),
        bytes(diff_root),
        "inadmissible precondition: one occurrence"
    );
    let projected = project_same(&trace, &["app.js"]);
    for root in [explicit, diff_root] {
        assert!(
            projected
                .roots
                .iter()
                .any(|p| snapshot(p) == snapshot(root)),
            "{root:?}"
        );
    }
}

#[test]
fn loading_projection_retains_isolated_use() {
    let trace = app_trace();
    let root = x_root(&trace, Diff);
    assert_eq!(
        root.location.kind,
        VarAccessKind::Use,
        "inadmissible precondition"
    );
    assert_eq!(&APP[root.location.start_byte..root.location.end_byte], "x");
    assert!(
        root.reached.is_empty(),
        "inadmissible precondition: root is not isolated"
    );
    let projected = project_same(&trace, &["app.js"]);
    let kept = projected
        .roots
        .iter()
        .find(|p| snapshot(p) == snapshot(root));
    assert!(
        kept.is_some_and(|p| p.reached.is_empty()),
        "isolated Use dropped"
    );
}

#[test]
fn loading_projection_excludes_framework_and_ipc_roots() {
    let mut trace = app_trace();
    let template = x_root(&trace, Diff).clone();
    let eligible = trace
        .roots
        .iter()
        .filter(|r| matches!(r.origin, Diff | Explicit))
        .count();
    for origin in [Framework, Ipc] {
        trace.roots.push(TaintRoot {
            origin,
            ..template.clone()
        });
    }
    let projected = project_same(&trace, &["app.js"]);
    assert!(projected
        .roots
        .iter()
        .all(|r| matches!(r.origin, Diff | Explicit)));
    assert_eq!(projected.roots.len(), eligible);
}
