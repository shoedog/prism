//! S1b-2b (S1b SPEC §3.2, §6, §7 B-11, B-12): the export facts' new state (`VerifiedLocal`,
//! refusal and may-call counts, `ResolvedJsExport.wrapped`) survives serde and the CPG cache,
//! and full and incremental builds agree across source epochs.
use super::js_wrapped_export_test::app_sites;
use prism::ast::ParsedFile;
use prism::cpg::CodePropertyGraph;
use prism::cpg_cache::{self, CacheResult};
use prism::js_exports::{JsExportFacts, JsExportTarget, ResolvedJsExport};
use prism::languages::Language;
use std::collections::{BTreeMap, BTreeSet};

const APP: &str = "import { f } from './lib';\nexport function run() {\n  f();\n}\n";
const LIB: &str = "function outer() {\n  function f() { return 99; }\n  return f();\n}\nconst f = \
    () => {\n  return 1;\n};\nexport { f };\n";

fn parse(files: &[(String, String)]) -> BTreeMap<String, ParsedFile> {
    let parse = |p: &String, s: &String| ParsedFile::parse(p, s, Language::from_path(p).unwrap());
    files
        .iter()
        .map(|(p, s)| (p.clone(), parse(p, s).unwrap()))
        .collect()
}

#[test]
fn b11_facts_serde_and_cpg_cache_full_hit() {
    // One verified, one refused and one may-call export occurrence; the imported `k` keeps
    // base `Local`, poisoned, uncounted (SPEC §3.2 step 2).
    let src = "function f() {\n  return 1;\n}\nconst g = f ? 1 : 2;\nlet h = () => 1;\nh = f;\n\
        import { k } from 'pkg';\nexport { f, g, h, k };\n";
    let facts = ParsedFile::parse("lib.tsx", src, Language::Tsx)
        .unwrap()
        .extract_js_ts_export_facts();
    let verified = JsExportTarget::VerifiedLocal {
        local: "f".to_string(),
        start_line: 1,
        end_line: 3,
    };
    assert_eq!(facts.named["f"], verified);
    assert_eq!(
        facts.named["g"],
        JsExportTarget::UnprovenLocal("g".to_string())
    );
    assert_eq!(facts.named["h"], JsExportTarget::Local("h".to_string()));
    assert_eq!(facts.named["k"], JsExportTarget::Local("k".to_string()));
    assert!(facts.conflicted.contains("k"));
    let refusals = BTreeMap::from([("not_callable".to_string(), 1)]);
    assert_eq!(
        (&facts.local_export_refusals, facts.local_export_may_call),
        (&refusals, 1)
    );
    let json = serde_json::to_string(&facts).unwrap();
    assert_eq!(serde_json::from_str::<JsExportFacts>(&json).unwrap(), facts);
    // Facts and resolved exports persisted before S1b-2b read back with the new fields empty.
    let old: JsExportFacts = serde_json::from_str(
        r#"{"esm_named_imports":[],"type_only_imports":{},"named":{},"star_reexports":[],
        "skipped_expr_count":0,"conflicted":[]}"#,
    )
    .unwrap();
    assert!(old.local_export_refusals.is_empty() && old.local_export_may_call == 0);
    let old: ResolvedJsExport =
        serde_json::from_str(r#"{"file":"a.ts","local_name":"f","span":[1,3]}"#).unwrap();
    assert!(!old.wrapped);

    let sources: BTreeMap<String, String> = [("lib.tsx", LIB), ("app.tsx", APP)]
        .map(|(p, s)| (p.into(), s.into()))
        .into();
    let cold = CodePropertyGraph::build(&parse(&sources.clone().into_iter().collect::<Vec<_>>()));
    let dir = tempfile::tempdir().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&cold, &hashes, false, dir.path()).unwrap();
    let CacheResult::Hit(hit) = cpg_cache::load_cache(&hashes, false, dir.path()) else {
        panic!("expected a full CPG cache hit");
    };
    assert_eq!(
        app_sites(&hit.call_graph),
        ["L3 f: Exact import_member lib:f@5-7"]
    );
}

fn epochs(ext: &str) {
    let two = "L3 f: NameOnly import_member lib:f@2-2, NameOnly import_member lib:f@5-7";
    let states = [
        (LIB.to_string(), "L3 f: Exact import_member lib:f@5-7"),
        // Written (M2): base `Local(f)`, so both same-named functions.
        (format!("{LIB}f = null;\n"), two),
        // Refused (`not_callable`): `UnprovenLocal`, no edge.
        (
            LIB.replace("() => {\n  return 1;\n}", "1"),
            "L3 f: drop UnknownName",
        ),
        (LIB.to_string(), "L3 f: Exact import_member lib:f@5-7"),
        (
            LIB.replace("\nconst f", "\n\n\nconst f"),
            "L3 f: Exact import_member lib:f@7-9",
        ),
    ];
    let (lib, app) = (format!("lib.{ext}"), format!("app.{ext}"));
    let mut previous: Option<CodePropertyGraph> = None;
    for (epoch, (src, want)) in states.into_iter().enumerate() {
        let files = parse(&[(lib.clone(), src), (app.clone(), APP.to_string())]);
        let full = CodePropertyGraph::build(&files);
        assert_eq!(
            app_sites(&full.call_graph),
            [want],
            "{lib} full epoch{epoch}"
        );
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
                    [want],
                    "{lib} incremental epoch{epoch}"
                );
                inc
            }
            None => full,
        });
    }
}

#[test]
fn b12_source_epochs_full_and_incremental() {
    epochs("jsx");
    epochs("tsx");
}
