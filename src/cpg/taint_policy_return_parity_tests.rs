//! Return-flow walker crossing-policy tests (#602 S3b-C): RF05-RF08 (context-to-context and the
//! depth bound, degraded and no-CFG seeds, parity with no policy, the public entry). The fixtures
//! and helpers are those of `taint_policy_return_tests`: REAL parsed Python programs, CONTROLLED by
//! relabelling, adding or injecting edges where a test says so.

use std::collections::BTreeSet;

use petgraph::graph::NodeIndex;

use super::taint_policy_return_tests::{
    build, chain, dbg, entry_labels, pair, pair_with, parent, reached, reached_in_file, refused,
    relabel, sources, trace, var_at, without_cfg, without_crossing, LIB, LIB_SRC, MAIN,
    MAIN_ONCE_SRC, MAIN_STRAY_SRC, MAIN_TWICE_SRC, ROWS,
};
use super::taint_policy_tests::{files, loc, EXACT, WEAK};
use super::trace::MAX_TAINT_DESCENT_DEPTH;
use super::{
    CodePropertyGraph, CpgEdge, CrossingCeiling, Relation, ReturnFlowMode, TaintCrossingPolicy,
    VarAccess,
};

const MID: &str = "mid.py";
const LEAF: &str = "leaf.py";

/// main -> mid.fa -> {leaf.fb (another file), mid.inner (the same file)}.
const MAIN_TRI_SRC: &str =
    "from mid import fa\ndef run(user):\n    value = fa(user)\n    sink(value)\n";
const MID_SRC: &str = "from leaf import fb\ndef inner(w):\n    return w\ndef fa(p):\n    r = fb(p)\n    s = inner(p)\n    return r\n";
const LEAF_SRC: &str = "def fb(x):\n    y = x\n    return y\n";

/// Three files: nodes by role (all pristine: every entry is an Exact edge).
struct Tri {
    cpg: CodePropertyGraph,
    enter: NodeIndex,       // Use `user`, main.py:3
    fa_param: NodeIndex,    // Def `p`, mid.py:4
    to_leaf: NodeIndex,     // Use `p`, mid.py:5
    leaf_param: NodeIndex,  // Def `x`, leaf.py:1
    leaf_body: NodeIndex,   // Use `x`, leaf.py:2
    to_inner: NodeIndex,    // Use `p`, mid.py:6
    inner_param: NodeIndex, // Def `w`, mid.py:2
    inner_body: NodeIndex,  // Use `w`, mid.py:3
}

fn tri() -> Tri {
    let cpg = build(&[(MAIN, MAIN_TRI_SRC), (MID, MID_SRC), (LEAF, LEAF_SRC)]);
    let at = |file, line, name, access| var_at(&cpg, file, line, name, access);
    let found = Tri {
        enter: at(MAIN, 3, "user", VarAccess::Use),
        fa_param: at(MID, 4, "p", VarAccess::Def),
        to_leaf: at(MID, 5, "p", VarAccess::Use),
        leaf_param: at(LEAF, 1, "x", VarAccess::Def),
        leaf_body: at(LEAF, 2, "x", VarAccess::Use),
        to_inner: at(MID, 6, "p", VarAccess::Use),
        inner_param: at(MID, 2, "w", VarAccess::Def),
        inner_body: at(MID, 3, "w", VarAccess::Use),
        cpg,
    };
    let free = trace(&found.cpg, &sources(MAIN, 3), None);
    for (from, to) in [
        (found.enter, found.fa_param),
        (found.to_leaf, found.leaf_param),
        (found.to_inner, found.inner_param),
    ] {
        assert_eq!(
            entry_labels(&found.cpg, from, to),
            vec![EXACT],
            "inadmissible precondition: Exact entry {from:?} -> {to:?}"
        );
        assert_eq!(
            parent(&free, found.enter, to),
            Some((from, Relation::CallDescent)),
            "inadmissible precondition: the policy-free trace descends into {to:?}"
        );
    }
    assert!(reached(&free, found.enter, found.leaf_body));
    assert!(reached(&free, found.enter, found.inner_body));
    found
}

#[test]
fn return_policy_gates_context_to_context_and_keeps_depth_bound() {
    let ctx = files(&[MID, LEAF]);
    let strict = TaintCrossingPolicy::new(&ctx, CrossingCeiling::Exact);
    let lenient = TaintCrossingPolicy::new(&ctx, CrossingCeiling::NameOnly);
    let s = sources(MAIN, 3);
    // With every entry Exact the gate is invisible, the interprocedural depth bound included.
    let t = tri();
    let free = trace(&t.cpg, &s, None);
    for policy in [&strict, &lenient] {
        assert_eq!(dbg(&trace(&t.cpg, &s, Some(policy))), dbg(&free));
    }
    let hops = chain(&free, t.enter, t.leaf_param);
    let descents = hops
        .iter()
        .filter(|hop| hop.2 == Relation::CallDescent)
        .count();
    assert_eq!(descents, MAX_TAINT_DESCENT_DEPTH);
    // CONTROLLED: the context -> context entry and the same-file entry are NameOnly.
    let mut t = tri();
    relabel(&mut t.cpg, t.to_leaf, t.leaf_param, WEAK);
    relabel(&mut t.cpg, t.to_inner, t.inner_param, WEAK);
    let free = trace(&t.cpg, &s, None);
    assert!(
        reached(&free, t.enter, t.leaf_param),
        "inadmissible precondition"
    );
    let gated = trace(&t.cpg, &s, Some(&strict));
    // Context -> other context is constrained: the leaf file is out of reach, the refusal kept.
    assert!(!reached_in_file(&t.cpg, &gated, t.enter, LEAF));
    assert!(!reached(&gated, t.enter, t.leaf_body));
    assert!(refused(&gated, t.enter, t.to_leaf, t.leaf_param));
    // The original -> context entry is Exact, and a hop inside one context file is never refused.
    assert!(reached(&gated, t.enter, t.fa_param));
    assert!(reached(&gated, t.enter, t.inner_param));
    assert!(reached(&gated, t.enter, t.inner_body));
    assert!(!refused(&gated, t.enter, t.to_inner, t.inner_param));
    // The NameOnly ceiling refuses nothing.
    assert_eq!(dbg(&trace(&t.cpg, &s, Some(&lenient))), dbg(&free));
}

#[test]
fn return_policy_applies_on_degraded_and_no_cfg_seeds() {
    let ctx = files(&[LIB]);
    let s = sources(MAIN, 2);
    for stripped in [false, true] {
        for (entry, ceiling, admitted) in ROWS {
            let mut p = pair();
            if stripped {
                without_cfg(&mut p.cpg);
                assert!(
                    !p.cpg.has_cfg_edges(),
                    "inadmissible precondition: every ControlFlow edge is removed"
                );
            }
            // The signature line seeds the parameter `user` itself.
            let root = var_at(&p.cpg, MAIN, 2, "user", VarAccess::Def);
            p.check(&s, root);
            let row = format!("stripped={stripped} {entry:?} {ceiling:?}");
            relabel(&mut p.cpg, p.arg, p.param, entry); // CONTROLLED
            let policy = TaintCrossingPolicy::new(&ctx, ceiling);
            let free = trace(&p.cpg, &s, None);
            let gated = trace(&p.cpg, &s, Some(&policy));
            assert_eq!(free.degraded, !stripped, "inadmissible precondition: {row}");
            assert_eq!(reached(&gated, root, p.param), admitted, "{row}");
            if admitted {
                assert_eq!(dbg(&gated), dbg(&free), "{row}");
            } else {
                assert!(!reached_in_file(&p.cpg, &gated, root, LIB), "{row}");
                assert!(refused(&gated, root, p.arg, p.param), "{row}");
                assert_eq!(gated.frontier_by_root[&root], without_crossing(&free, root));
            }
        }
    }
}

/// Every fixture with the seeds that exercise it (parity inputs of RF07): pristine and
/// CONTROLLED pairs, the two-call route, the three-file chain, the injected edge, no CFG.
fn parity_inputs() -> Vec<(CodePropertyGraph, Vec<(String, usize)>)> {
    let both = vec![(MAIN.to_string(), 2), (MAIN.to_string(), 3)];
    let mut all = vec![(pair().cpg, both.clone())];
    let mut weak = pair();
    relabel(&mut weak.cpg, weak.arg, weak.param, WEAK);
    all.push((weak.cpg, both.clone()));
    let mut stripped = pair();
    without_cfg(&mut stripped.cpg);
    all.push((stripped.cpg, both.clone()));
    all.push((pair().cpg, sources(LIB, 2)));
    for source in [MAIN_TWICE_SRC, MAIN_ONCE_SRC] {
        let mut cpg = build(&[(MAIN, source), (LIB, LIB_SRC)]);
        let first = var_at(&cpg, MAIN, 3, "user", VarAccess::Use);
        let param = var_at(&cpg, LIB, 1, "p", VarAccess::Def);
        relabel(&mut cpg, first, param, WEAK);
        all.push((cpg, sources(MAIN, 2)));
    }
    let mut stray = pair_with(MAIN_STRAY_SRC);
    let w = var_at(&stray.cpg, MAIN, 6, "w", VarAccess::Def);
    let edge = CpgEdge::ReturnFlow {
        suppress_shortcut: false,
    };
    stray.cpg.graph.add_edge(stray.ret, w, edge);
    all.push((stray.cpg, vec![(LIB.to_string(), 2), (MAIN.to_string(), 3)]));
    all.push((tri().cpg, sources(MAIN, 3)));
    let mut t = tri();
    relabel(&mut t.cpg, t.to_leaf, t.leaf_param, WEAK);
    all.push((t.cpg, sources(MAIN, 3)));
    all
}

#[test]
fn return_policy_absent_preserves_trace_and_adapter_outputs() {
    let empty = BTreeSet::new();
    for (cpg, s) in parity_inputs() {
        for mode in [ReturnFlowMode::On, ReturnFlowMode::Off] {
            let legacy = dbg(&cpg.taint_trace_with_mode(&s, mode));
            assert_eq!(
                dbg(&cpg.taint_trace_with_mode_policy(&s, mode, None)),
                legacy
            );
            for ceiling in [CrossingCeiling::Exact, CrossingCeiling::NameOnly] {
                let policy = TaintCrossingPolicy::new(&empty, ceiling);
                let walked = cpg.taint_trace_with_mode_policy(&s, mode, Some(&policy));
                assert_eq!(dbg(&walked), legacy, "{mode:?} {ceiling:?}");
            }
        }
        assert_eq!(
            dbg(&cpg.taint_trace(&s)),
            dbg(&cpg.taint_trace_with_mode(&s, ReturnFlowMode::Off))
        );
        // The labeled adapter: paths and evidence are the policy-free form's.
        let labeled = cpg.taint_forward_cfg_with_return_flow_labeled(&s);
        let internal = cpg.taint_forward_cfg_with_return_flow_labeled_policy(&s, None);
        assert_eq!(dbg(&internal), dbg(&labeled));
        assert_eq!(
            dbg(&cpg.taint_forward_cfg_with_return_flow(&s)),
            dbg(&labeled.0)
        );
        for ceiling in [CrossingCeiling::Exact, CrossingCeiling::NameOnly] {
            let policy = TaintCrossingPolicy::new(&empty, ceiling);
            let public = cpg.taint_forward_cfg_with_return_flow_labeled_with_policy(&s, &policy);
            assert_eq!(dbg(&public), dbg(&labeled), "{ceiling:?}");
        }
    }
    // An unknown seed line: the same warning and an empty result, with and without a policy.
    let p = pair();
    let missing = sources("missing.py", 99);
    let ctx = files(&[LIB]);
    let strict = TaintCrossingPolicy::new(&ctx, CrossingCeiling::Exact);
    let plain = trace(&p.cpg, &missing, None);
    assert_eq!(
        plain.warnings,
        ["Seed missing.py:99 resolved to no variable nodes"]
    );
    assert!(plain.frontier_by_root.is_empty() && plain.boundary.is_empty());
    assert_eq!(dbg(&trace(&p.cpg, &missing, Some(&strict))), dbg(&plain));
    let nothing = p
        .cpg
        .taint_forward_cfg_with_return_flow_labeled_with_policy(&missing, &strict);
    assert!(nothing.0.is_empty() && nothing.1.is_empty());
}

#[test]
fn return_policy_public_entry_uses_same_gate() {
    let ctx = files(&[LIB]);
    let s = sources(MAIN, 3);
    for (entry, ceiling, admitted) in ROWS {
        let mut p = pair();
        relabel(&mut p.cpg, p.arg, p.param, entry); // CONTROLLED
        let policy = TaintCrossingPolicy::new(&ctx, ceiling);
        let row = format!("{entry:?} {ceiling:?}");
        let public = p
            .cpg
            .taint_forward_cfg_with_return_flow_labeled_with_policy(&s, &policy);
        let internal = p
            .cpg
            .taint_forward_cfg_with_return_flow_labeled_policy(&s, Some(&policy));
        assert_eq!(dbg(&public), dbg(&internal), "{row}");
        let (paths, evidence) = &public;
        let into_lib = paths
            .iter()
            .flat_map(|path| &path.edges)
            .any(|edge| edge.to.file == LIB);
        assert_eq!(into_lib, admitted, "{row}");
        assert_eq!(
            evidence.keys().any(|(_, target)| target.file == LIB),
            admitted,
            "{row}"
        );
        if admitted {
            let key = (loc(&p.cpg, p.arg), loc(&p.cpg, p.body));
            assert!(evidence[&key].crossed_unlabeled, "{row}");
        }
        // The policy-free adapter reaches the callee whatever the label.
        let legacy = p.cpg.taint_forward_cfg_with_return_flow_labeled(&s);
        assert_eq!(dbg(&legacy) == dbg(&public), admitted, "{row}");
    }
}
