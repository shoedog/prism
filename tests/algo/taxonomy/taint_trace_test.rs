use crate::common::*;
use algorithms::taint::{self, TaintConfig, TaintRootOrigin};

fn fixture(source: &str, path: &str, lines: &[usize]) -> (BTreeMap<String, ParsedFile>, DiffInput) {
    let parsed = ParsedFile::parse(path, source, Language::from_path(path).unwrap()).unwrap();
    (
        BTreeMap::from([(path.into(), parsed)]),
        DiffInput {
            files: vec![DiffInfo {
                file_path: path.into(),
                modify_type: ModifyType::Modified,
                diff_lines: lines.iter().copied().collect(),
            }],
        },
    )
}

fn assert_result_identity(legacy: &SliceResult, traced: &SliceResult) {
    assert_eq!(
        serde_json::to_vec(legacy).unwrap(),
        serde_json::to_vec(traced).unwrap()
    );
    assert_eq!(legacy.findings.len(), legacy.evidence.len());
    assert_eq!(traced.findings.len(), traced.evidence.len());
    // SliceResult serialization skips evidence. Compare the actual aligned
    // optional witnesses, including every hop, byte span and confidence label.
    for (index, (left, right)) in legacy.evidence.iter().zip(&traced.evidence).enumerate() {
        assert_eq!(
            format!("{left:?}"),
            format!("{right:?}"),
            "evidence for finding {index}"
        );
    }
}

fn sink_evidence(result: &SliceResult) -> Vec<&prism::finding_confidence::EvidencePath> {
    result
        .findings
        .iter()
        .enumerate()
        .filter(|(_, finding)| finding.category.as_deref() == Some("taint_sink"))
        .map(|(index, _)| {
            result.evidence[index]
                .as_ref()
                .expect("sink has aligned evidence")
        })
        .collect()
}

#[test]
fn taint_trace_keeps_explicit_and_diff_origins_and_isolated_use_roots() {
    let source = "function h(x) {\n  unknown(x);\n}\n";
    let (files, diff) = fixture(source, "app.js", &[2]);
    let ctx = CpgContext::build(&files, None);
    let mut config = TaintConfig::default();
    config.sources = vec![("app.js".into(), 2)];
    let (_, trace) = taint::slice_with_trace(&ctx, &diff, &config).unwrap();
    for origin in [TaintRootOrigin::Explicit, TaintRootOrigin::Diff] {
        let root = trace
            .roots
            .iter()
            .find(|r| r.origin == origin && r.location.path.base == "x")
            .expect("isolated argument root retained");
        assert_eq!(
            &source[root.location.start_byte..root.location.end_byte],
            "x"
        );
        assert_eq!(root.seed.line, 2);
        assert!(root.seed.target.is_none());
    }
    config.taint_from_diff = false;
    let (_, explicit) = taint::slice_with_trace(&ctx, &diff, &config).unwrap();
    assert!(!explicit.roots.is_empty());
    assert!(explicit
        .roots
        .iter()
        .all(|r| r.origin == TaintRootOrigin::Explicit));
    config.sources.clear();
    assert!(taint::slice_with_trace(&ctx, &diff, &config)
        .unwrap()
        .1
        .roots
        .is_empty());
}

#[test]
fn taint_trace_retains_reached_value_identity_and_graph_evidence() {
    let (files, diff) = fixture("def h(x):\n    y = x\n    unknown(y)\n", "app.py", &[2]);
    let ctx = CpgContext::build(&files, None);
    for return_flow in [false, true] {
        let config = TaintConfig {
            return_flow,
            ..Default::default()
        };
        let (_, trace) = taint::slice_with_trace(&ctx, &diff, &config).unwrap();
        let (paths, evidence) = if return_flow {
            ctx.cpg
                .taint_forward_cfg_with_return_flow_labeled(&[("app.py".into(), 2)])
        } else {
            ctx.cpg.taint_forward_cfg_labeled(&[("app.py".into(), 2)])
        };
        assert!(!paths.is_empty());
        let mut matched = 0;
        for root in &trace.roots {
            for reached in &root.reached {
                if let Some(expected) =
                    evidence.get(&(root.location.clone(), reached.location.clone()))
                {
                    assert_eq!(format!("{:?}", reached.evidence), format!("{expected:?}"));
                    assert_eq!(reached.location.function, "h");
                    matched += 1;
                }
            }
        }
        assert!(matched > 0);
    }
}

#[test]
fn taint_trace_framework_roots_keep_target_and_scope_without_diff_roots() {
    let source = "const express = require('express');\nconst app = express();\napp.get('/', (req, res) => {\n  const x = req.query.q;\n  eval(x);\n});\n";
    let (files, diff) = fixture(source, "app.js", &[5]);
    let ctx = CpgContext::build(&files, None);
    let config = TaintConfig {
        taint_from_diff: false,
        ..Default::default()
    };
    let (result, trace) = taint::slice_with_trace(&ctx, &diff, &config).unwrap();
    assert!(!sink_evidence(&result).is_empty());
    assert_result_identity(&taint::slice(&ctx, &diff, &config).unwrap(), &result);
    assert!(!trace.roots.is_empty());
    assert!(trace
        .roots
        .iter()
        .all(|r| r.origin == TaintRootOrigin::Framework));
    let target = trace
        .roots
        .iter()
        .find(|r| r.seed.target.is_some() && r.seed.scope.is_some())
        .expect("scoped target framework source");
    assert!(target.seed.scope.is_some());
    assert!(target.seed.byte_scope.is_some());
    // Legacy synthetic roots remain explicitly ambiguous, not guessed from a line.
    assert_eq!(
        (target.location.start_byte, target.location.end_byte),
        (0, 0)
    );
    assert!(!target.reached.is_empty());
    let (files, diff) = fixture(&source.replace("eval(x)", "unknown(x)"), "app.js", &[5]);
    let ctx = CpgContext::build(&files, None);
    let (negative, _) = taint::slice_with_trace(&ctx, &diff, &config).unwrap();
    assert!(sink_evidence(&negative).is_empty());
    assert_result_identity(&taint::slice(&ctx, &diff, &config).unwrap(), &negative);
}

#[test]
fn taint_trace_ipc_origin_is_distinct_from_diff_and_explicit() {
    let (files, diff) = fixture(
        "void h(void *data) {\n  char *x = g_hash_table_lookup(data, \"key\");\n  unknown(x);\n}\n",
        "app.c",
        &[3],
    );
    let ctx = CpgContext::build(&files, None);
    let config = TaintConfig {
        taint_from_diff: false,
        ..Default::default()
    };
    let (_, trace) = taint::slice_with_trace(&ctx, &diff, &config).unwrap();
    assert!(!trace.roots.is_empty());
    assert!(trace
        .roots
        .iter()
        .all(|r| r.origin == TaintRootOrigin::Ipc && r.seed.line == 2));
    let empty_diff = DiffInput { files: vec![] };
    assert!(taint::slice_with_trace(&ctx, &empty_diff, &config)
        .unwrap()
        .1
        .roots
        .is_empty());
}

#[test]
fn taint_trace_preserves_legacy_result_bytes_for_positive_and_negative_controls() {
    for (source, positive) in [
        (
            "import os\ndef h(user):\n    command = user\n    os.system(command)\n",
            true,
        ),
        (
            "import os\ndef h(user):\n    command = user\n    return 42\n",
            false,
        ),
    ] {
        let (files, diff) = fixture(source, "app.py", &[3]);
        let ctx = CpgContext::build(&files, None);
        for return_flow in [false, true] {
            let config = TaintConfig {
                return_flow,
                ..Default::default()
            };
            let legacy = taint::slice(&ctx, &diff, &config).unwrap();
            let (traced, _) = taint::slice_with_trace(&ctx, &diff, &config).unwrap();
            let evidence = sink_evidence(&legacy);
            assert_eq!(!evidence.is_empty(), positive);
            if positive {
                assert!(evidence.iter().any(|path| !path.hops.is_empty()));
            }
            assert_result_identity(&legacy, &traced);
        }
    }
}
