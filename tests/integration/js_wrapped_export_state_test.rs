//! S1 (wrapped-export SPEC §5–§7): declarator counters, source epochs and serde state.
use super::js_wrapped_export_test::{app_sites, graph, APP, WRAP};
use prism::ast::ParsedFile;
use prism::cpg::CodePropertyGraph;
use prism::cpg_cache::{self, CacheResult};
use prism::js_exports::{JsExportFacts, JsExportTarget};
use prism::languages::Language;
use std::collections::{BTreeMap, BTreeSet};

fn facts(src: &str) -> JsExportFacts {
    ParsedFile::parse("lib.tsx", src, Language::Tsx)
        .unwrap()
        .extract_js_ts_export_facts()
}

fn parse(files: &[(String, String)]) -> BTreeMap<String, ParsedFile> {
    let parse = |p: &String, s: &String| ParsedFile::parse(p, s, Language::from_path(p).unwrap());
    files
        .iter()
        .map(|(p, s)| (p.clone(), parse(p, s).unwrap()))
        .collect()
}

#[test]
fn t_o1_admitted_declarator_is_counted_not_skipped() {
    let f = facts(WRAP);
    assert_eq!(
        (
            f.spanned_admitted,
            f.skipped_expr_count,
            f.skipped_decl_reasons.len()
        ),
        (1, 0, 0)
    );
}

const MIXED: &str = "import { memo } from 'react';\nexport const A = memo((p) => null);\nexport \
    const B = other((p) => null);\nexport const C = 1;\nexport let D = memo((p) => null);\nexport \
    const E = memo((p) => null, (a, b) => true);\nexport default someCall();\n";

#[test]
fn t_o3_skipped_expr_count_is_the_reasons_plus_other_skips() {
    let f = facts(MIXED);
    let want = [
        ("callee_not_admitted", 1),
        ("comparator_line_collision", 1),
        ("non_call_initializer", 1),
        ("not_const", 1),
    ];
    let want: BTreeMap<String, usize> = want.iter().map(|(r, n)| (r.to_string(), *n)).collect();
    assert_eq!(f.skipped_decl_reasons, want);
    assert_eq!(f.spanned_admitted, 1);
    // The `export default someCall()` skip is the one non-declarator skip.
    assert_eq!(f.skipped_expr_count, want.values().sum::<usize>() + 1);
}

fn epochs(lib_ext: &str, app_ext: &str) {
    let base = "import { forwardRef } from 'react';\nexport const Island = forwardRef((props, \
        ref) => {\n  return null;\n});\n";
    let states = [
        (base.to_string(), Some("2-4")),
        (base.replace("'react'", "'./shim'"), None),
        (base.to_string(), Some("2-4")),
        (base.replace("\nexport", "\n\n\nexport"), Some("4-6")),
        (base.to_string(), Some("2-4")),
    ];
    let (lib, app) = (format!("lib.{lib_ext}"), format!("app.{app_ext}"));
    let mut previous: Option<CodePropertyGraph> = None;
    for (epoch, (src, span)) in states.into_iter().enumerate() {
        let files = parse(&[(lib.clone(), src), (app.clone(), APP.to_string())]);
        let want = [match span {
            Some(s) => format!("L3 Island: Exact import_member lib:Island@{s}"),
            None => "L3 Island: drop UnknownName".to_string(),
        }];
        let full = CodePropertyGraph::build(&files);
        assert_eq!(app_sites(&full.call_graph), want, "{lib} full epoch{epoch}");
        previous = Some(match previous {
            Some(old) => {
                let changed = BTreeSet::from([lib.clone()]);
                let inc = CodePropertyGraph::build_incremental(
                    old.call_graph,
                    old.dfg,
                    &changed,
                    &files,
                    None,
                );
                assert_eq!(
                    app_sites(&inc.call_graph),
                    want,
                    "{lib} incremental epoch{epoch}"
                );
                inc
            }
            None => full,
        });
    }
}

#[test]
fn t_s1_source_epochs_full_and_incremental() {
    epochs("js", "js");
    epochs("ts", "tsx");
    epochs("tsx", "tsx");
}

#[test]
fn t_c1_serde_round_trip_and_cpg_cache_full_hit() {
    let f = facts(MIXED);
    assert!(matches!(f.named["A"], JsExportTarget::SpannedLocal { .. }));
    let back: JsExportFacts = serde_json::from_str(&serde_json::to_string(&f).unwrap()).unwrap();
    assert_eq!(back, f);
    let cg = graph(&[("lib.tsx", WRAP), ("app.tsx", APP)]);
    let jsx = cg
        .calls
        .values()
        .flatten()
        .find(|s| s.jsx_element)
        .unwrap()
        .clone();
    let json = serde_json::to_string(&jsx).unwrap();
    assert_eq!(
        serde_json::from_str::<prism::call_graph::CallSite>(&json).unwrap(),
        jsx
    );

    let sources: BTreeMap<String, String> = [("lib.tsx", WRAP), ("app.tsx", APP)]
        .map(|(p, s)| (p.into(), s.into()))
        .into();
    let files = parse(&sources.clone().into_iter().collect::<Vec<_>>());
    let cold = CodePropertyGraph::build(&files);
    let dir = tempfile::tempdir().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&cold, &hashes, false, dir.path()).unwrap();
    let CacheResult::Hit(hit) = cpg_cache::load_cache(&hashes, false, dir.path()) else {
        panic!("expected a full CPG cache hit");
    };
    let want = ["L3 Island: Exact import_member lib:Island@2-4"];
    assert_eq!(app_sites(&hit.call_graph), want);
}
