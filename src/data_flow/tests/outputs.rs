use super::super::*;
use crate::cpg::CodePropertyGraph;
use crate::languages::Language;

#[test]
fn decoded_corruption_is_rejected_against_cold_reference() {
    use super::super::test_support::edit_legacy_parts;
    use crate::cpg::dfg_observation::{fieldwise_payload, lookup_indexes};
    use crate::cpg_cache::{compute_file_hashes, load_cache, save_cache, CacheResult, SaveOutcome};
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
        let cold = CodePropertyGraph::build(&files);
        let cache = tempfile::tempdir().unwrap();
        let hashes = compute_file_hashes(&sources);
        assert_eq!(
            save_cache(&cold, &hashes, false, cache.path()).unwrap(),
            SaveOutcome::Saved
        );
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
