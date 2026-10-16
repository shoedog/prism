use super::super::*;
use crate::cpg::{
    dfg_compaction_support::{assemble_with_dfg, legacy_dfg},
    CpgContext,
};
use crate::diff::{DiffInfo, DiffInput, ModifyType};
use crate::languages::Language;

#[test]
fn delta_candidate_edges_contribute() {
    let old_repo = tempfile::tempdir().unwrap();
    std::fs::write(old_repo.path().join("a.py"), "def f():\n    return 1\n").unwrap();
    let files = BTreeMap::from([(
        "a.py".into(),
        ParsedFile::parse(
            "a.py",
            "def f():\n    x = 1\n    y = x\n    return y\n",
            Language::Python,
        )
        .unwrap(),
    )]);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "a.py".into(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([2]),
        }],
    };
    let old_graph = assemble_with_dfg(&files, legacy_dfg(&files));
    let new_graph = crate::cpg::CodePropertyGraph::build(&files);
    for graph in [old_graph, new_graph] {
        let ctx = CpgContext::build_with_cached_cpg(&files, graph, None);
        let run = crate::algorithms::delta_slice::slice(&ctx, &diff, old_repo.path()).unwrap();
        assert!(
            run.blocks.iter().any(|b| b
                .file_line_map
                .get("a.py")
                .is_some_and(|lines| lines.contains_key(&4))),
            "new-flow endpoint at return line 4 must contribute"
        );
        std::fs::write(old_repo.path().join("a.py"), &files["a.py"].source).unwrap();
        let same = crate::algorithms::delta_slice::slice(&ctx, &diff, old_repo.path()).unwrap();
        assert!(same.blocks.is_empty());
        std::fs::write(old_repo.path().join("a.py"), "def f():\n    return 1\n").unwrap();
    }
}

#[test]
fn decoded_corruption_is_rejected_against_cold_reference() {
    use super::super::test_support::edit_legacy_parts;
    use crate::cpg::dfg_compaction_support::{fieldwise_payload, lookup_indexes};
    use crate::cpg_cache::{compute_file_hashes, load_cache, save_cache, CacheResult};
    crate::build_pool::install(|| {
        let sources =
            BTreeMap::from([("a.py".into(), "def f(x):\n y = x + x\n return y\n".into())]);
        let files = sources
            .iter()
            .map(|(p, s): (&String, &String)| {
                (
                    p.clone(),
                    ParsedFile::parse(p, s, Language::Python).unwrap(),
                )
            })
            .collect();
        let cold = assemble_with_dfg(&files, legacy_dfg(&files));
        let cache = tempfile::tempdir().unwrap();
        let hashes = compute_file_hashes(&sources);
        save_cache(&cold, &hashes, false, cache.path()).unwrap();
        let decode = || match load_cache(&hashes, false, cache.path()) {
            CacheResult::Hit(graph) => graph,
            _ => panic!("expected full cache hit"),
        };
        let assert_matches = |a: &crate::cpg::CodePropertyGraph,
                              b: &crate::cpg::CodePropertyGraph| {
            for graph in [a, b] {
                assert_eq!(fieldwise_payload(&cold), fieldwise_payload(graph));
                assert_eq!(lookup_indexes(&cold), lookup_indexes(graph));
            }
        };
        let decoded = decode();
        assert_matches(&decoded, &decoded);
        for change_label in [false, true] {
            let mut corrupt = decode();
            if change_label {
                edit_legacy_parts(&mut corrupt.dfg, |parts| {
                    let label = parts.labels.values_mut().next().expect("nonempty labels");
                    *label = match *label {
                        crate::cpg::FlowConfidence::Exact => {
                            crate::cpg::FlowConfidence::NameOnly(crate::cpg::FlowDoubt::Killed {
                                kill_line: 7,
                            })
                        }
                        _ => crate::cpg::FlowConfidence::Exact,
                    };
                });
            } else {
                corrupt.dfg_label_stats.dfg_label_exact += 1;
            }
            assert!(
                std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| assert_matches(
                    &corrupt, &corrupt
                )))
                .is_err(),
                "identical decoded corruption escaped the cold reference"
            );
        }
    });
}
