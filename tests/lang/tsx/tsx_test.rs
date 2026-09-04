use crate::common::*;

// ====== TSX Parsing & Algorithm Coverage ======

#[test]
fn test_tsx_parses_without_errors() {
    let (files, _, _) = make_tsx_test();
    let parsed = files.values().next().unwrap();
    assert_eq!(parsed.language, Language::Tsx);
    let error_rate = parsed.error_rate();
    assert!(
        error_rate == 0.0,
        "TSX file should parse without errors, got error_rate={error_rate}"
    );
}

#[test]
fn test_tsx_extension_routing() {
    assert_eq!(Language::from_extension("tsx"), Some(Language::Tsx));
    assert_eq!(Language::from_extension("ts"), Some(Language::TypeScript));
    assert_eq!(Language::from_extension("jsx"), Some(Language::JavaScript));
}

#[test]
fn test_tsx_from_path() {
    assert_eq!(Language::from_path("src/App.tsx"), Some(Language::Tsx));
    assert_eq!(
        Language::from_path("src/utils.ts"),
        Some(Language::TypeScript)
    );
}

#[test]
fn test_tsx_parameter_receiver_binding_suppresses_import_qualified() {
    use prism::call_graph::CallGraph;
    use prism::resolution::{ResolutionConfidence, ResolutionKind};
    use std::collections::BTreeMap;

    let files = BTreeMap::from([
        (
            "api.ts".to_string(),
            ParsedFile::parse(
                "api.ts",
                "export function m() {}\n",
                Language::TypeScript,
            )
            .unwrap(),
        ),
        (
            "view.tsx".to_string(),
            ParsedFile::parse(
                "view.tsx",
                "import api from './api';\ninterface Foo {}\nfunction run(api: Foo) { api.m(); return <div />; }\n",
                Language::Tsx,
            )
            .unwrap(),
        ),
    ]);
    let graph = CallGraph::build(&files);
    let call = graph
        .calls
        .iter()
        .find(|(caller, _)| caller.file == "view.tsx" && caller.name == "run")
        .and_then(|(_, calls)| calls.iter().find(|call| call.callee_name == "m"))
        .expect("run -> m");

    assert!(call.receiver_lexically_bound);
    assert!(graph.resolve_call_site(call).iter().all(|candidate| {
        candidate.kind != ResolutionKind::ImportQualified
            || candidate.confidence != ResolutionConfidence::Exact
            || candidate.target.file != "api.ts"
    }));
}

#[test]
fn test_tsx_typed_parameter_and_new_constructor_recover() {
    use prism::call_graph::CallGraph;
    use prism::resolution::{ReceiverRecovery, ResolutionConfidence, ResolutionKind};
    use std::collections::BTreeMap;

    let files = BTreeMap::from([(
        "view.tsx".to_string(),
        ParsedFile::parse(
            "view.tsx",
            "class Foo { m() {} }\nfunction typed(x: Foo) { x.m(); return <div />; }\nfunction made() { const x = new Foo(); x.m(); return <div />; }\n",
            Language::Tsx,
        )
        .unwrap(),
    )]);
    let graph = CallGraph::build(&files);

    for (caller_name, recovery, kind) in [
        (
            "typed",
            ReceiverRecovery::TypedParam,
            ResolutionKind::TypedParam,
        ),
        (
            "made",
            ReceiverRecovery::ConstructorLocal,
            ResolutionKind::ConstructorLocal,
        ),
    ] {
        let call = graph
            .calls
            .iter()
            .find(|(caller, _)| caller.file == "view.tsx" && caller.name == caller_name)
            .and_then(|(_, calls)| calls.iter().find(|call| call.callee_name == "m"))
            .expect("caller -> m");
        assert_eq!(call.receiver_type.as_deref(), Some("Foo"), "{caller_name}");
        assert_eq!(call.receiver_recovery, Some(recovery), "{caller_name}");
        assert!(call.receiver_materialized, "{caller_name}");
        let resolved = graph.resolve_call_site(call);
        assert_eq!(resolved.len(), 1, "{caller_name}: {resolved:?}");
        assert_eq!(resolved[0].kind, kind, "{caller_name}");
        assert_eq!(resolved[0].confidence, ResolutionConfidence::Exact);
        assert_eq!(resolved[0].target.file, "view.tsx");
    }
}

#[test]
fn test_original_diff_tsx() {
    let (files, _, diff) = make_tsx_test();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::OriginalDiff);
    let result = algorithms::run_slicing_compat(&files, &diff, &config, None).unwrap();
    assert!(
        !result.blocks.is_empty(),
        "OriginalDiff TSX should produce blocks"
    );
    assert_eq!(result.algorithm, SlicingAlgorithm::OriginalDiff);
}

#[test]
fn test_parent_function_tsx() {
    let (files, _, diff) = make_tsx_test();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::ParentFunction);
    let result = algorithms::run_slicing_compat(&files, &diff, &config, None).unwrap();
    assert!(
        !result.blocks.is_empty(),
        "ParentFunction TSX should produce blocks"
    );
    assert_eq!(result.algorithm, SlicingAlgorithm::ParentFunction);
}

#[test]
fn test_left_flow_tsx() {
    let (files, _, diff) = make_tsx_test();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow);
    let result = algorithms::run_slicing_compat(&files, &diff, &config, None).unwrap();
    assert!(
        !result.blocks.is_empty(),
        "LeftFlow TSX should produce blocks"
    );
    assert_eq!(result.algorithm, SlicingAlgorithm::LeftFlow);
}

#[test]
fn test_full_flow_tsx() {
    let (files, _, diff) = make_tsx_test();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::FullFlow);
    let result = algorithms::run_slicing_compat(&files, &diff, &config, None).unwrap();
    assert!(
        !result.blocks.is_empty(),
        "FullFlow TSX should produce blocks"
    );
    assert_eq!(result.algorithm, SlicingAlgorithm::FullFlow);
}

#[test]
fn test_thin_slice_tsx() {
    let (files, _, diff) = make_tsx_test();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::ThinSlice);
    let result = algorithms::run_slicing_compat(&files, &diff, &config, None).unwrap();
    assert!(
        !result.blocks.is_empty(),
        "ThinSlice TSX should produce blocks"
    );
    assert_eq!(result.algorithm, SlicingAlgorithm::ThinSlice);
}

#[test]
fn test_barrier_slice_tsx() {
    let (files, _, diff) = make_tsx_test();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::BarrierSlice);
    let result = algorithms::run_slicing_compat(&files, &diff, &config, None).unwrap();
    // BarrierSlice may produce empty blocks for single-file fixtures
    assert_eq!(result.algorithm, SlicingAlgorithm::BarrierSlice);
}

#[test]
fn test_relevant_slice_tsx() {
    let (files, _, diff) = make_tsx_test();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::RelevantSlice);
    let result = algorithms::run_slicing_compat(&files, &diff, &config, None).unwrap();
    assert!(
        !result.blocks.is_empty(),
        "RelevantSlice TSX should produce blocks"
    );
    assert_eq!(result.algorithm, SlicingAlgorithm::RelevantSlice);
}

#[test]
fn test_gradient_slice_tsx() {
    let (files, _, diff) = make_tsx_test();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::GradientSlice);
    let result = algorithms::run_slicing_compat(&files, &diff, &config, None).unwrap();
    assert!(
        !result.blocks.is_empty(),
        "GradientSlice TSX should produce blocks"
    );
    assert_eq!(result.algorithm, SlicingAlgorithm::GradientSlice);
}

#[test]
fn test_absence_slice_tsx() {
    let (files, _, diff) = make_tsx_test();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice);
    let result = algorithms::run_slicing_compat(&files, &diff, &config, None).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::AbsenceSlice);
}

#[test]
fn test_symmetry_slice_tsx() {
    let (files, _, diff) = make_tsx_test();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::SymmetrySlice);
    let result = algorithms::run_slicing_compat(&files, &diff, &config, None).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::SymmetrySlice);
}

#[test]
fn test_horizontal_slice_tsx() {
    let (files, _, diff) = make_tsx_test();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::HorizontalSlice);
    let result = algorithms::run_slicing_compat(&files, &diff, &config, None).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::HorizontalSlice);
}

#[test]
fn test_quantum_slice_tsx() {
    let (files, _, diff) = make_tsx_test();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::QuantumSlice);
    let result = algorithms::run_slicing_compat(&files, &diff, &config, None).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::QuantumSlice);
}

#[test]
fn test_angle_slice_tsx() {
    let (files, _, diff) = make_tsx_test();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::AngleSlice);
    let result = algorithms::run_slicing_compat(&files, &diff, &config, None).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::AngleSlice);
}

#[test]
fn test_provenance_slice_tsx() {
    let (files, _, diff) = make_tsx_test();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice);
    let result = algorithms::run_slicing_compat(&files, &diff, &config, None).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::ProvenanceSlice);
}

#[test]
fn test_echo_slice_tsx() {
    let (files, _, diff) = make_tsx_test();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::EchoSlice);
    let result = algorithms::run_slicing_compat(&files, &diff, &config, None).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::EchoSlice);
}

#[test]
fn test_membrane_slice_tsx() {
    let (files, _, diff) = make_tsx_test();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::MembraneSlice);
    let result = algorithms::run_slicing_compat(&files, &diff, &config, None).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::MembraneSlice);
}
