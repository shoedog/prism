use super::*;
use crate::access_path::AccessPath;
use crate::call_graph::FunctionId;
use crate::cpg::{FlowConfidence, FlowDoubt};
use crate::data_flow::{VarAccessKind, VarLocation};
use crate::resolution::{ResolutionConfidence, ResolutionKind, ResolvedCallEdge};
use crate::slice::{FileParseQuality, SliceFinding};
use std::collections::BTreeMap;

fn fpq(q: &str) -> FileParseQuality {
    FileParseQuality {
        error_count: 0,
        node_count: 1,
        error_rate: 0.0,
        quality: q.to_string(),
        error_lines: vec![],
    }
}
fn finding(algorithm: &str, file: &str, related: &[&str]) -> SliceFinding {
    SliceFinding {
        algorithm: algorithm.into(),
        file: file.into(),
        line: 1,
        severity: "warning".into(),
        description: String::new(),
        function_name: None,
        related_lines: vec![],
        related_files: related.iter().map(|s| s.to_string()).collect(),
        category: None,
        parse_quality: None,
        diagrams: vec![],
    }
}

fn location(line: usize, kind: VarAccessKind) -> VarLocation {
    VarLocation {
        file: "a.py".into(),
        function: "f".into(),
        function_start_line: 1,
        line,
        path: AccessPath::simple("x"),
        start_byte: line,
        end_byte: line + 1,
        kind,
    }
}

fn call_hop(confidence: ResolutionConfidence) -> EvidenceHop {
    let edge = ResolvedCallEdge {
        caller: FunctionId {
            file: "a.py".into(),
            name: "caller".into(),
            start_line: 1,
            end_line: 3,
        },
        call_site_line: 2,
        confidence,
        kind: ResolutionKind::FreeSingle,
    };
    EvidenceHop::Call { edge, confidence }
}

fn flow_hop(confidence: FlowConfidence) -> EvidenceHop {
    EvidenceHop::DataFlow {
        from: location(2, VarAccessKind::Def),
        to: location(3, VarAccessKind::Use),
        confidence,
    }
}

#[test]
fn ast_only_clean_is_exact_asserted() {
    assert_eq!(
        classify("absence", ParseQuality::Clean),
        (FindingConfidence::Exact, FindingTier::Asserted)
    );
}
#[test]
fn cpg_algorithm_is_unlabeled_candidate() {
    assert_eq!(
        classify("echo", ParseQuality::Clean),
        (FindingConfidence::Unlabeled, FindingTier::Candidate)
    );
}
#[test]
fn degraded_or_unknown_parse_is_candidate() {
    assert_eq!(
        classify("absence", ParseQuality::Degraded),
        (FindingConfidence::Exact, FindingTier::Candidate)
    );
    assert_eq!(
        classify("absence", ParseQuality::Unknown),
        (FindingConfidence::Exact, FindingTier::Candidate)
    );
}
#[test]
fn every_production_algorithm_string_maps_to_its_variant() {
    use crate::slice::SlicingAlgorithm;
    for (s, expected) in [
        ("absence", SlicingAlgorithm::AbsenceSlice),
        (
            "callback_dispatcher",
            SlicingAlgorithm::CallbackDispatcherSlice,
        ),
        ("contract", SlicingAlgorithm::ContractSlice),
        ("echo", SlicingAlgorithm::EchoSlice),
        ("membrane", SlicingAlgorithm::MembraneSlice),
        ("peer_consistency", SlicingAlgorithm::PeerConsistencySlice),
        ("primitive", SlicingAlgorithm::PrimitiveSlice),
        ("provenance", SlicingAlgorithm::ProvenanceSlice),
        ("symmetry", SlicingAlgorithm::SymmetrySlice),
        ("taint", SlicingAlgorithm::Taint),
    ] {
        assert_eq!(
            SlicingAlgorithm::from_str(s),
            Some(expected),
            "{s} must round-trip to {expected:?}"
        );
    }
}
#[test]
fn confidence_table_for_production_algorithms() {
    for (s, expected) in [
        ("absence", FindingConfidence::Exact),
        ("contract", FindingConfidence::Exact),
        ("symmetry", FindingConfidence::Exact),
        ("primitive", FindingConfidence::Exact),
        ("peer_consistency", FindingConfidence::Exact),
        ("callback_dispatcher", FindingConfidence::Exact),
        ("echo", FindingConfidence::Unlabeled),
        ("membrane", FindingConfidence::Unlabeled),
        ("provenance", FindingConfidence::Unlabeled),
        ("taint", FindingConfidence::Unlabeled),
    ] {
        assert_eq!(classify(s, ParseQuality::Clean).0, expected, "{s}");
    }
}
#[test]
fn unknown_algorithm_is_unlabeled_candidate() {
    assert_eq!(
        classify("not_an_algorithm", ParseQuality::Clean),
        (FindingConfidence::Unlabeled, FindingTier::Candidate)
    );
}
#[test]
fn crossed_unlabeled_is_unlabeled_even_with_exact_hops() {
    let evidence = EvidencePath {
        hops: vec![flow_hop(FlowConfidence::Exact)],
        crossed_unlabeled: true,
    };
    assert_eq!(
        classify_with_evidence("taint", ParseQuality::Clean, &evidence),
        (FindingConfidence::Unlabeled, FindingTier::Candidate)
    );
}
#[test]
fn exact_hops_plus_clean_parse_are_exact_asserted() {
    let evidence = EvidencePath {
        hops: vec![
            flow_hop(FlowConfidence::Exact),
            call_hop(ResolutionConfidence::Exact),
        ],
        crossed_unlabeled: false,
    };
    assert_eq!(
        classify_with_evidence("taint", ParseQuality::Clean, &evidence),
        (FindingConfidence::Exact, FindingTier::Asserted)
    );
}
#[test]
fn any_nameonly_flow_or_call_hop_is_nameonly_candidate() {
    let nameonly_flow = EvidencePath {
        hops: vec![flow_hop(FlowConfidence::NameOnly(FlowDoubt::SameLine))],
        crossed_unlabeled: false,
    };
    let nameonly_call = EvidencePath {
        hops: vec![call_hop(ResolutionConfidence::NameOnly)],
        crossed_unlabeled: false,
    };
    for evidence in [nameonly_flow, nameonly_call] {
        assert_eq!(
            classify_with_evidence("taint", ParseQuality::Clean, &evidence),
            (FindingConfidence::NameOnly, FindingTier::Candidate)
        );
    }
}
#[test]
fn a_non_clean_parse_prevents_asserted() {
    let evidence = EvidencePath {
        hops: vec![flow_hop(FlowConfidence::Exact)],
        crossed_unlabeled: false,
    };
    assert_eq!(
        classify_with_evidence("taint", ParseQuality::Degraded, &evidence),
        (FindingConfidence::Exact, FindingTier::Candidate)
    );
}
#[test]
fn ast_only_algorithm_with_empty_evidence_stays_exact() {
    assert_eq!(
        classify_with_evidence("absence", ParseQuality::Clean, &EvidencePath::default()),
        (FindingConfidence::Exact, FindingTier::Asserted)
    );
}
#[test]
fn a_missing_artifact_none_fails_to_unlabeled_not_empty_exact() {
    let evidence: Option<EvidencePath> = None;
    let classified = evidence.as_ref().map_or(
        (FindingConfidence::Unlabeled, FindingTier::Candidate),
        |evidence| classify_with_evidence("absence", ParseQuality::Clean, evidence),
    );
    assert_eq!(
        classified,
        (FindingConfidence::Unlabeled, FindingTier::Candidate)
    );
}
#[test]
fn trace_dataflow_relation_recovers_its_concrete_label() {
    let root = NodeIndex::new(1);
    let sink = NodeIndex::new(2);
    let mut trace = Trace::default();
    trace
        .parents_by_root
        .insert((root, sink), (root, Relation::DataFlow));
    trace
        .data_flow_hops
        .insert((root, sink), flow_hop(FlowConfidence::Exact));
    let evidence = EvidencePath::from_trace(&trace, root, sink);
    assert!(!evidence.crossed_unlabeled);
    assert!(matches!(
        evidence.hops.as_slice(),
        [EvidenceHop::DataFlow {
            confidence: FlowConfidence::Exact,
            ..
        }]
    ));
}
#[test]
fn trace_dataflow_relation_without_a_concrete_hop_crosses_unlabeled() {
    let root = NodeIndex::new(1);
    let sink = NodeIndex::new(2);
    let mut trace = Trace::default();
    trace
        .parents_by_root
        .insert((root, sink), (root, Relation::DataFlow));

    let evidence = EvidencePath::from_trace(&trace, root, sink);

    assert!(evidence.crossed_unlabeled);
    assert!(evidence.hops.is_empty());
}
#[test]
fn every_non_dataflow_trace_relation_crosses_unlabeled() {
    for relation in [
        Relation::AssignmentPropagation,
        Relation::RecoveredDefUse,
        Relation::CallDescent,
        Relation::ReturnInput,
        Relation::ReturnFlow,
    ] {
        let root = NodeIndex::new(1);
        let sink = NodeIndex::new(2);
        let mut trace = Trace::default();
        trace.parents_by_root.insert((root, sink), (root, relation));
        let evidence = EvidencePath::from_trace(&trace, root, sink);
        assert!(evidence.crossed_unlabeled, "{relation:?}");
        assert!(evidence.hops.is_empty(), "{relation:?}");
    }
}
#[test]
fn classify_equals_classify_with_evidence_for_every_production_algorithm() {
    for algorithm in SlicingAlgorithm::all() {
        let name = algorithm.name();
        assert_eq!(
            classify(name, ParseQuality::Clean),
            classify_with_evidence(
                name,
                ParseQuality::Clean,
                &EvidencePath::unlabeled_for(name)
            ),
            "{name}"
        );
    }
}
#[test]
fn finding_confidence_nameonly_serializes_as_nameonly() {
    assert_eq!(
        serde_json::to_string(&FindingConfidence::NameOnly).unwrap(),
        "\"nameonly\""
    );
}
#[test]
fn serde_spellings() {
    assert_eq!(
        serde_json::to_string(&FindingConfidence::Exact).unwrap(),
        "\"exact\""
    );
    assert_eq!(
        serde_json::to_string(&FindingConfidence::Unlabeled).unwrap(),
        "\"unlabeled\""
    );
    assert_eq!(
        serde_json::to_string(&FindingTier::Candidate).unwrap(),
        "\"candidate\""
    );
    assert_eq!(ParseQuality::Poor.as_str(), "poor");
}
#[test]
fn parse_quality_serde_matches_as_str() {
    for q in [
        ParseQuality::Clean,
        ParseQuality::Degraded,
        ParseQuality::Poor,
        ParseQuality::Unparseable,
        ParseQuality::Unknown,
    ] {
        assert_eq!(
            serde_json::to_string(&q).unwrap().trim_matches('"'),
            q.as_str()
        );
    }
}
#[test]
fn parse_quality_ordering_is_best_to_worst() {
    assert!(
        ParseQuality::Clean < ParseQuality::Degraded
            && ParseQuality::Degraded < ParseQuality::Poor
            && ParseQuality::Poor < ParseQuality::Unparseable
            && ParseQuality::Unparseable < ParseQuality::Unknown
    );
}
fn parsed_with(path: &str) -> BTreeMap<String, crate::ast::ParsedFile> {
    let mut parsed = BTreeMap::new();
    parsed.insert(
        path.to_string(),
        crate::ast::ParsedFile::parse(path, "x = 1\n", crate::languages::Language::Python).unwrap(),
    );
    parsed
}
#[test]
fn min_over_treats_sparse_map_absence_as_clean_when_parsed() {
    let parsed = parsed_with("a.py");
    let map: BTreeMap<String, FileParseQuality> = BTreeMap::new();
    // a.py is absent from the sparse map but was parsed -> Clean, not Unknown.
    assert_eq!(
        ParseQuality::min_over(&["a.py"], &map, &parsed),
        ParseQuality::Clean
    );
}
#[test]
fn min_over_takes_the_worst_over_files() {
    let parsed = parsed_with("a.py");
    let mut map = BTreeMap::new();
    map.insert("b.py".to_string(), fpq("degraded"));
    assert_eq!(
        ParseQuality::min_over(&["a.py", "b.py"], &map, &parsed),
        ParseQuality::Degraded
    );
}
#[test]
fn min_over_is_unknown_for_files_in_neither_map_nor_parsed() {
    let parsed = parsed_with("a.py");
    let map: BTreeMap<String, FileParseQuality> = BTreeMap::new();
    assert_eq!(
        ParseQuality::min_over(&["a.py", "missing.py"], &map, &parsed),
        ParseQuality::Unknown
    );
}
#[test]
fn min_over_empty_files_is_unknown() {
    let parsed = parsed_with("a.py");
    let map: BTreeMap<String, FileParseQuality> = BTreeMap::new();
    assert_eq!(
        ParseQuality::min_over(&[], &map, &parsed),
        ParseQuality::Unknown
    );
}
#[test]
fn min_over_unrecognized_map_quality_string_is_unknown() {
    let parsed: BTreeMap<String, crate::ast::ParsedFile> = BTreeMap::new();
    let mut map = BTreeMap::new();
    map.insert("c.py".to_string(), fpq("weird"));
    assert_eq!(
        ParseQuality::min_over(&["c.py"], &map, &parsed),
        ParseQuality::Unknown
    );
}
#[test]
fn parse_quality_for_treats_contract_delta_categories_as_unknown() {
    let parsed = parsed_with("a.py");
    let map: BTreeMap<String, FileParseQuality> = BTreeMap::new();
    let mut weakened = finding("contract", "a.py", &[]);
    weakened.category = Some("contract_precondition_weakened".to_string());
    assert_eq!(
        parse_quality_for(&weakened, &map, &parsed),
        ParseQuality::Unknown
    );
}
#[test]
fn parse_quality_for_non_delta_contract_category_uses_min_over() {
    let parsed = parsed_with("a.py");
    let map: BTreeMap<String, FileParseQuality> = BTreeMap::new();
    let mut violation = finding("contract", "a.py", &[]);
    violation.category = Some("contract_violation".to_string());
    assert_eq!(
        parse_quality_for(&violation, &map, &parsed),
        ParseQuality::Clean
    );
}
#[test]
fn parse_quality_for_symmetry_related_file_dominates() {
    let parsed = parsed_with("a.py");
    let mut map = BTreeMap::new();
    map.insert("b.py".to_string(), fpq("degraded"));
    let f = finding("symmetry", "a.py", &["b.py"]);
    assert_eq!(parse_quality_for(&f, &map, &parsed), ParseQuality::Degraded);
}
#[test]
fn evidence_files_is_anchor_then_related() {
    assert_eq!(
        evidence_files(&finding("symmetry", "a.py", &["b.py"])),
        vec!["a.py", "b.py"]
    );
    assert_eq!(
        evidence_files(&finding("absence", "a.py", &[])),
        vec!["a.py"]
    );
    assert_eq!(
        evidence_files(&finding("echo", "a.py", &["a.py", "b.py", "b.py"])),
        vec!["a.py", "b.py"]
    );
}
