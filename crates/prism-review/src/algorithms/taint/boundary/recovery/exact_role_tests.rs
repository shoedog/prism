use super::*;
use crate::algorithms::taint::{self, TaintConfig, TaintReached, TaintRootOrigin};
use prism::{
    access_path::AccessPath,
    api::{load_review_inputs, ReviewBoundaryHints, ReviewImportHint, ReviewInputs, ReviewOptions},
    call_graph::CallGraph,
    cpg::CpgContext,
    finding_confidence::EvidencePath,
};
use std::ops::Range;

fn fixture(
    argument: &str,
    needle: &str,
) -> (tempfile::TempDir, ReviewInputs, CallSite, Range<usize>) {
    let dir = tempfile::tempdir().unwrap();
    let source = format!(
        "import {{ send }} from './transport.js';\nfunction h(req) {{\n  send({argument});\n}}\n"
    );
    std::fs::write(dir.path().join("a.js"), &source).unwrap();
    std::fs::write(
        dir.path().join("transport.js"),
        "export function send(x) {}\n",
    )
    .unwrap();
    let diff = serde_json::json!({"files": [{
        "file_path": "a.js", "modify_type": "Modified", "diff_lines": [3]
    }]});
    let inputs = load_review_inputs(&ReviewOptions::new(dir.path()), &diff.to_string()).unwrap();
    assert_eq!(
        inputs.files.len(),
        1,
        "transport stays outside the loaded diff"
    );
    assert_eq!(inputs.files["a.js"].parse_error_count, 0);
    let call = CallGraph::build(&inputs.files)
        .calls
        .into_values()
        .flatten()
        .find(|call| call.callee_name == "send")
        .unwrap();
    assert_eq!(
        (
            &*call.caller.file,
            &*call.caller.name,
            call.caller.start_line,
            call.line
        ),
        ("a.js", "h", 2, 3)
    );
    assert_eq!(
        &source[call.start_byte..call.end_byte],
        format!("send({argument})")
    );
    assert!(
        matches!(ReviewBoundaryHints::new(&inputs).unwrap().hint_for_call(&call),
        Some(ReviewImportHint::InRepo { files, .. }) if files == ["transport.js"])
    );
    let start = source.rfind(needle).unwrap();
    (dir, inputs, call, start..start + needle.len())
}

#[test]
fn real_diff_and_explicit_root_roles_are_qualified() {
    let mut observed = Vec::new();
    let mut expected = Vec::new();
    for (argument, needle, wanted) in [
        ("{req: 'safe'}", "req", false),
        ("req", "req", true),
        ("{req}", "req", true),
        // Evaluated identifier syntax, but inherited computed-key ownership is unknown.
        ("{[req]: 'safe'}", "req", false),
        ("req.body", "req.body", true),
    ] {
        let (_dir, inputs, call, span) = fixture(argument, needle);
        let graph = CpgContext::build(&inputs.files, None);
        let recovery = RecoveryContext::new(&inputs.files["a.js"]);
        for explicit in [false, true] {
            let config = TaintConfig {
                sources: if explicit {
                    vec![("a.js".into(), 3)]
                } else {
                    vec![]
                },
                taint_from_diff: !explicit,
                ..Default::default()
            };
            let legacy = taint::slice(&graph, &inputs.diff, &config).unwrap();
            let (result, trace) = taint::slice_with_trace(&graph, &inputs.diff, &config).unwrap();
            let before = (
                serde_json::to_value(&result).unwrap(),
                format!("{:?}", result.evidence),
                format!("{trace:?}"),
            );
            let roots: Vec<_> = trace
                .roots
                .iter()
                .filter(|root| {
                    let value = &root.location;
                    root.origin
                        == if explicit {
                            TaintRootOrigin::Explicit
                        } else {
                            TaintRootOrigin::Diff
                        }
                        && value.kind == VarAccessKind::Use
                        && value.file == "a.js"
                        && value.function == "h"
                        && value.function_start_line == 2
                        && value.line == 3
                        && value.path == AccessPath::from_expr(needle)
                        && (value.start_byte..value.end_byte) == span
                })
                .collect();
            if argument == "{[req]: 'safe'}" {
                let values = recovery.facts.values(span.clone());
                assert!(!values.is_empty());
                assert!(values.iter().all(|v| v.owner.is_none() && v.semantic_value));
                assert_eq!(
                    recovery
                        .parsed
                        .tree
                        .root_node()
                        .descendant_for_byte_range(span.start, span.end)
                        .unwrap()
                        .kind(),
                    "identifier"
                );
            }
            // All actual roots/frontiers are printed before the table admission assertion.
            eprintln!(
                "ROLE argument={argument:?} explicit={explicit} matching_roots={} trace={trace:?}",
                roots.len()
            );
            observed.push((!roots.is_empty()).then(|| {
                roots.iter().all(|root| {
                    recovery.matches_argument(&call, recovery.prepare(root).root_value())
                })
            }));
            expected.push(Some(wanted));
            assert_eq!(
                before,
                (
                    serde_json::to_value(&result).unwrap(),
                    format!("{:?}", result.evidence),
                    format!("{trace:?}")
                )
            );
            assert_eq!(
                serde_json::to_value(&legacy).unwrap(),
                serde_json::to_value(&result).unwrap()
            );
            assert_eq!(
                format!("{:?}", legacy.evidence),
                format!("{:?}", result.evidence)
            );
        }
    }
    assert!(
        observed.iter().all(Option::is_some),
        "inadmissible missing actual root: {observed:?}"
    );
    assert_eq!(observed, expected);
}

#[test]
fn synthetic_identity_controls_keep_destination_exact_and_original_root_values() {
    let (_dir, inputs, call, span) = fixture("req", "req");
    let graph = CpgContext::build(&inputs.files, None);
    let (_, trace) =
        taint::slice_with_trace(&graph, &inputs.diff, &TaintConfig::default()).unwrap();
    let root = trace
        .roots
        .iter()
        .find(|root| {
            root.location.kind == VarAccessKind::Use
                && (root.location.start_byte..root.location.end_byte) == span
        })
        .expect("actual plain argument root");
    let recovery = RecoveryContext::new(&inputs.files["a.js"]);
    assert!(recovery.matches_argument(&call, recovery.prepare(root).root_value()));
    let mut wrong = root.location.clone();
    wrong.file = "other.js".into();
    assert!(!recovery.matches_argument(&call, &wrong));
    wrong = root.location.clone();
    wrong.end_byte -= 1;
    assert!(!recovery.matches_argument(&call, &wrong));
    wrong.start_byte = wrong.end_byte + 1;
    assert!(!recovery.matches_argument(&call, &wrong));
    let mut foreign = root.clone();
    foreign.seed.file = "source.js".into();
    foreign.location.file = "source.js".into();
    foreign.reached = vec![TaintReached {
        location: root.location.clone(),
        evidence: EvidencePath::default(),
    }];
    let prepared = recovery.prepare(&foreign);
    assert!(
        recovery.matches_argument(&call, prepared.value(0).unwrap()),
        "destination context keeps original exact Use despite failed source proof"
    );
    assert!(!recovery.matches_argument(&call, prepared.root_value()));
}
