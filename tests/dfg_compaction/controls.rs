use super::common::*;
use super::outputs::{
    assert_cache_matches, loaded, output_pair, params, refresh, roundtrip, write_sources,
};
use prism::api::{run_review, AlgorithmParams};
use prism::cpg::dfg_compaction_support::{assemble_with_dfg, legacy_dfg};
// These controls exercise direct DFG consumers without history queries or Git writes.
// They are additional local controls, not rows in the committed-repository matrix.
fn local_modes(
    files: &BTreeMap<String, ParsedFile>,
    diff: &DiffInput,
    old_source: Option<(&str, &str)>,
    algorithms: &[SlicingAlgorithm],
    tune: impl Fn(&mut AlgorithmParams),
    check: impl Fn(SlicingAlgorithm, &prism::api::ReviewRun),
) {
    let repo = TempDir::new().unwrap();
    write_sources(repo.path(), files);
    let inputs = loaded(repo.path(), files, diff);
    let prior = TempDir::new().unwrap();
    write_sources(prior.path(), files);
    if let Some((path, source)) = old_source {
        std::fs::write(prior.path().join(path), source).unwrap();
    }
    let mut p = params(files, prior.path());
    tune(&mut p);
    for mode in ["cold", "hit", "refresh-readd"] {
        let old = assemble_with_dfg(files, legacy_dfg(files));
        let new = CodePropertyGraph::build(files);
        let adapt = |g, legacy| match mode {
            "hit" => roundtrip(&g, &inputs),
            "refresh-readd" => refresh(g, files, legacy),
            _ => g,
        };
        let cold = CpgContext::build_with_cached_cpg(
            files,
            assemble_with_dfg(files, legacy_dfg(files)),
            None,
        );
        let old = adapt(old, true);
        let new = adapt(new, false);
        assert_cache_matches(&cold.cpg, &old, &new);
        let old = CpgContext::build_with_cached_cpg(files, old, None);
        let new = CpgContext::build_with_cached_cpg(files, new, None);
        for &algorithm in algorithms {
            let config = SliceConfig::default().with_algorithm(algorithm);
            let a = run_review(&old, &inputs, &[algorithm], &config, &p, repo.path());
            let b = run_review(&new, &inputs, &[algorithm], &config, &p, repo.path());
            let reference = run_review(&cold, &inputs, &[algorithm], &config, &p, repo.path());
            output_pair(&reference, &a, &inputs, repo.path());
            output_pair(&reference, &b, &inputs, repo.path());
            check(algorithm, &a);
            check(algorithm, &b);
            eprintln!("DFG_LOCAL_DIRECT {} {mode} success=true", algorithm.name());
        }
    }
}
fn local_direct_dfg_positive_and_negative_controls_inner() {
    let (files, _, diff) = make_python_test();
    local_modes(
        &files,
        &diff,
        None,
        &[SlicingAlgorithm::LeftFlow, SlicingAlgorithm::FullFlow],
        |_| {},
        |a, r| {
            let lines = r.results[0].blocks[0]
                .file_line_map
                .get("src/calc.py")
                .unwrap();
            let sources = files
                .iter()
                .map(|(path, parsed)| (path.clone(), parsed.source.clone()))
                .collect();
            assert!(output::format_slice_result(&r.results[0].blocks, &sources).contains("return"));
            assert!(
                lines.len()
                    > if a == SlicingAlgorithm::LeftFlow {
                        2
                    } else {
                        1
                    }
            );
        },
    );
    let source = "def f():\n    x = 1\n    y = x\n    return y\n";
    let files = super::corpus::explicit("a.py", source, Language::Python);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "a.py".into(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([2]),
        }],
    };
    local_modes(
        &files,
        &diff,
        Some(("a.py", "def f():\n    return 1\n")),
        &[SlicingAlgorithm::DeltaSlice],
        |_| {},
        |_, r| {
            assert!(r.results[0].blocks.iter().any(|b| b
                .file_line_map
                .get("a.py")
                .is_some_and(|l| l.contains_key(&4))));
        },
    );
    local_modes(
        &files,
        &diff,
        None,
        &[SlicingAlgorithm::DeltaSlice],
        |_| {},
        |_, r| assert!(r.results[0].blocks.is_empty()),
    );
    for (positive,source,line) in [(true,"from flask import request\n\ndef handle_login():\n    username = request.form['username']\n    process(username)\n",5),
        (false,"def transform_data(raw):\n    result = transform(raw)\n    return result\n",2)] {
        let files=super::corpus::explicit("auth.py",source,Language::Python);
        let diff=DiffInput{files:vec![DiffInfo{file_path:"auth.py".into(),modify_type:ModifyType::Modified,diff_lines:BTreeSet::from([line])}]};
        local_modes(&files,&diff,None,&[SlicingAlgorithm::ProvenanceSlice],|_|{},|_,r| {
            let descriptions=r.findings.iter().map(|f|f.description.as_str()).collect::<Vec<_>>().join("\n");
            if positive {assert!(descriptions.contains("user_input"),"{descriptions}");} else {assert!(!descriptions.contains("user_input"),"{descriptions}");}
        });
    }
}

fn diff_at(path: &str, lines: &[usize]) -> DiffInput {
    DiffInput {
        files: vec![DiffInfo {
            file_path: path.into(),
            modify_type: ModifyType::Modified,
            diff_lines: lines.iter().copied().collect(),
        }],
    }
}
fn paired_left_full_disconnected_and_empty_dfg_controls_inner() {
    for (empty, source) in [
        (
            true,
            "def f():\n    return 1\n\ndef disconnected():\n    return 9\n",
        ),
        (
            false,
            "def f(x):\n    y = x\n    return y\n\ndef disconnected(z):\n    return z\n",
        ),
    ] {
        let files = super::corpus::explicit("a.py", source, Language::Python);
        assert_eq!(DataFlowGraph::build(&files).edges().is_empty(), empty);
        local_modes(
            &files,
            &diff_at("a.py", &[2]),
            None,
            &[SlicingAlgorithm::LeftFlow, SlicingAlgorithm::FullFlow],
            |_| {},
            |_, r| {
                let lines = &r.results[0].blocks[0].file_line_map["a.py"];
                assert!(lines.contains_key(&2));
                assert!(
                    lines.keys().all(|l| *l < 4),
                    "disconnected function entered the slice"
                );
            },
        );
    }
}
fn paired_chop_and_conditioned_controls_inner() {
    // Preserve misc_lang_test::test_chop_python_verifies_path_lines verbatim input/endpoints.
    let files = super::corpus::explicit(
        "app.py",
        "x = input()\ny = int(x)\nz = y + 1\nresult = z * 2\nprint(result)\n",
        Language::Python,
    );
    local_modes(
        &files,
        &diff_at("app.py", &[1, 4]),
        None,
        &[SlicingAlgorithm::Chop],
        |p| {
            p.chop_source = Some("app.py:1".into());
            p.chop_sink = Some("app.py:4".into());
        },
        |_, r| {
            let block = r.results[0]
                .blocks
                .first()
                .expect("connected chop must produce a block");
            let lines = &block.file_line_map["app.py"];
            assert!(lines.contains_key(&1) || lines.contains_key(&4));
        },
    );
    let source = "def f(x):\n    y = x\n    z = y\n    return z\n\ndef other():\n    return 0\n";
    let files = super::corpus::explicit("app.py", source, Language::Python);
    for (sink, connected) in [(3, true), (7, false)] {
        local_modes(
            &files,
            &diff_at("app.py", &[1]),
            None,
            &[SlicingAlgorithm::Chop],
            |p| {
                p.chop_source = Some("app.py:2".into());
                p.chop_sink = Some(format!("app.py:{sink}"));
            },
            |_, r| {
                let lines: BTreeSet<_> = r.results[0]
                    .blocks
                    .iter()
                    .flat_map(|b| b.file_line_map.values().flat_map(|ls| ls.keys().copied()))
                    .collect();
                if connected {
                    assert_eq!(lines, BTreeSet::from([2, 3]), "direct def-use chop path");
                } else {
                    assert!(
                        lines.is_empty(),
                        "disconnected CFG has no connecting path: {lines:?}"
                    );
                }
            },
        );
    }
    // Preserve misc_lang_test::test_conditioned_slice_prunes_unreachable_python.
    let source="\ndef process(x):\n    if x != 5:\n        result = x * 2\n    else:\n        result = 0\n    return result\n";
    let files = super::corpus::explicit("cond.py", source, Language::Python);
    let diff = diff_at("cond.py", &[4, 6]);
    let ctx = CpgContext::build(&files, None);
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow);
    let left = algorithms::run_slicing(&ctx, &diff, &config).unwrap();
    let left_count: usize = left
        .blocks
        .iter()
        .map(|b| b.file_line_map.values().map(|ls| ls.len()).sum::<usize>())
        .sum();
    for (condition, kept, removed) in [("x==5", 6, 4), ("x!=5", 4, 6)] {
        local_modes(
            &files,
            &diff,
            None,
            &[SlicingAlgorithm::ConditionedSlice],
            |p| p.condition = Some(condition.into()),
            |_, r| {
                let lines: BTreeSet<_> = r.results[0]
                    .blocks
                    .iter()
                    .flat_map(|b| b.file_line_map.values().flat_map(|ls| ls.keys().copied()))
                    .collect();
                assert!(lines.len() <= left_count);
                assert!(lines.contains(&kept));
                assert!(!lines.contains(&removed));
            },
        );
    }
}

#[test]
fn local_direct_dfg_positive_and_negative_controls() {
    prism::build_pool::install(local_direct_dfg_positive_and_negative_controls_inner);
}

#[test]
fn paired_left_full_disconnected_and_empty_dfg_controls() {
    prism::build_pool::install(paired_left_full_disconnected_and_empty_dfg_controls_inner);
}

#[test]
fn paired_chop_and_conditioned_controls() {
    prism::build_pool::install(paired_chop_and_conditioned_controls_inner);
}
