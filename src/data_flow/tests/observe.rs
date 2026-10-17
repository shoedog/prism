use super::super::test_support::*;
use super::super::*;
use crate::cpg::FlowDoubt;
use crate::finding_confidence::{EvidenceHop, EvidencePath};
use crate::frameworks::SanitizerCategory;
use crate::resolution::{ResolutionConfidence, ResolutionKind, ResolvedCallEdge};

fn loc(start: usize, kind: VarAccessKind) -> VarLocation {
    VarLocation {
        file: "a.py".into(),
        function: "f".into(),
        function_start_line: 1,
        line: 2,
        path: AccessPath::simple("x"),
        start_byte: start,
        end_byte: start + 1,
        kind,
    }
}
fn hop(start: usize) -> EvidenceHop {
    EvidenceHop::DataFlow {
        from: loc(start, VarAccessKind::Def),
        to: loc(start + 2, VarAccessKind::Use),
        confidence: FlowConfidence::Exact,
    }
}
fn obs(hops: Vec<EvidenceHop>, crossed_unlabeled: bool) -> Vec<u8> {
    evidence_obs(&[Some(EvidencePath {
        hops,
        crossed_unlabeled,
    })])
}
#[test]
fn observations_distinguish_spans_order_confidence_and_empty_evidence() {
    let baseline = obs(vec![hop(10), hop(20)], false);
    assert_ne!(baseline, obs(vec![hop(11), hop(20)], false));
    let mut changed = hop(10);
    if let EvidenceHop::DataFlow { to, .. } = &mut changed {
        to.end_byte += 1;
    }
    assert_ne!(baseline, obs(vec![changed, hop(20)], false));
    assert_ne!(baseline, obs(vec![hop(20), hop(10)], false));
    let mut changed = hop(10);
    if let EvidenceHop::DataFlow { confidence, .. } = &mut changed {
        *confidence = FlowConfidence::NameOnly(FlowDoubt::Killed { kill_line: 3 });
    }
    assert_ne!(baseline, obs(vec![changed, hop(20)], false));
    assert_ne!(baseline, obs(vec![hop(10), hop(20)], true));
    assert_ne!(evidence_obs(&[None]), obs(vec![], false));
    assert_ne!(obs(vec![], false), obs(vec![], true));
    assert_ne!(evidence_obs(&[]), evidence_obs(&[None]));
}
#[test]
fn observations_include_every_call_field_and_sanitizer_category() {
    let edge = ResolvedCallEdge {
        caller: crate::call_graph::FunctionId {
            file: "a.py".into(),
            name: "f".into(),
            start_line: 1,
            end_line: 4,
        },
        call_site_line: 2,
        confidence: ResolutionConfidence::Exact,
        kind: ResolutionKind::FreeSingle,
    };
    let call = |edge| EvidenceHop::Call {
        edge,
        confidence: ResolutionConfidence::Exact,
    };
    let baseline = obs(vec![call(edge.clone())], false);
    let mut variants = Vec::new();
    let mut e = edge.clone();
    e.caller.file = "b.py".into();
    variants.push(e);
    let mut e = edge.clone();
    e.caller.name = "g".into();
    variants.push(e);
    let mut e = edge.clone();
    e.caller.start_line += 1;
    variants.push(e);
    let mut e = edge.clone();
    e.caller.end_line += 1;
    variants.push(e);
    let mut e = edge.clone();
    e.call_site_line += 1;
    variants.push(e);
    let mut e = edge.clone();
    e.confidence = ResolutionConfidence::NameOnly;
    variants.push(e);
    let mut e = edge.clone();
    e.kind = ResolutionKind::FreeMulti;
    variants.push(e);
    for e in variants {
        assert_ne!(baseline, obs(vec![call(e)], false));
    }
    assert_ne!(
        baseline,
        obs(
            vec![EvidenceHop::Call {
                edge,
                confidence: ResolutionConfidence::NameOnly
            }],
            false
        )
    );
    let categories = [
        SanitizerCategory::Xss,
        SanitizerCategory::Sqli,
        SanitizerCategory::Ssrf,
        SanitizerCategory::Deserialization,
        SanitizerCategory::OsCommand,
        SanitizerCategory::PathTraversal,
    ];
    let mut seen = BTreeSet::new();
    for category in categories {
        let p = FlowPath {
            edges: vec![FlowEdge {
                from: loc(10, VarAccessKind::Def),
                to: loc(20, VarAccessKind::Use),
            }],
            cleansed_for: BTreeSet::from([category]),
        };
        assert!(seen.insert(path_bytes(&[p])));
    }
    let empty = FlowPath {
        edges: vec![],
        cleansed_for: BTreeSet::new(),
    };
    assert_ne!(path_bytes(&[]), path_bytes(&[empty]));
}
#[test]
fn ordered_storage_observation_includes_legacy_equal_byte_spans() {
    let a = loc(10, VarAccessKind::Def);
    let b = loc(20, VarAccessKind::Def);
    assert_eq!(a, b);
    assert_ne!(bytes(&a), bytes(&b));
    let mut g = oracle::LegacyDataFlowGraph::empty();
    g.edges.push(FlowEdge {
        from: a,
        to: loc(30, VarAccessKind::Use),
    });
    let before = current_storage(&from_legacy(g.clone()));
    g.edges[0].from = b;
    assert_ne!(before, current_storage(&from_legacy(g.clone())));
}
