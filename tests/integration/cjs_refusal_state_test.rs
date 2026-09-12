//! Exact changed-source custody, actual cross-file parameter flow, and serde.
use super::{
    cjs_refusal_test::{CJS, LANGUAGES},
    esm_forwarding_state_test::endpoints,
};
use prism::{
    ast::ParsedFile,
    cpg::CodePropertyGraph,
    js_exports::{resolve_js_exports, JsExportFacts, JsExportTarget},
    languages::Language,
};
use std::collections::{BTreeMap, BTreeSet};

fn transitions(lang: Language, ext: &str, forwarding: bool) {
    let producer = format!("origin.{ext}");
    let app = format!("app.{ext}");
    let clean = "function origin(input) { return input; }\nexport { origin as item };";
    let states = if forwarding {
        vec![(clean.into(),Some("origin")),("function origin(input) { origin = other; return input; }\nexport { origin as item };".into(),None),(clean.into(),Some("origin")),(clean.replace("origin","replacement"),Some("replacement")),(clean.into(),Some("origin"))]
    } else {
        vec![
            ("".into(), Some("item")),
            (format!("{CJS} const alias = module.exports;"), None),
            ("".into(), Some("item")),
            (format!("{CJS} module.exports.item = () => 0;"), None),
            ("".into(), Some("item")),
        ]
    };
    let mut previous: Option<CodePropertyGraph> = None;
    let mut previous_sources: BTreeMap<String, String> = BTreeMap::new();
    for (epoch, (source, target)) in states.into_iter().enumerate() {
        let srcs = if forwarding {
            vec![(producer.clone(),source),(format!("bridge.{ext}"),"import { item as local } from './origin'; export { local as item };".into()),(app.clone(),"import { item as invoke } from './bridge'; function run(value) { return invoke(value); }".into())]
        } else {
            vec![(producer.clone(),source),(format!("good.{ext}"),"export function item(input) { return input; }".into()),(format!("bridge.{ext}"),"export * from './origin'; export * from './good';".into()),(app.clone(),"import { item as invoke } from './bridge'; function run(value) { return invoke(value); }".into())]
        };
        let files: BTreeMap<_, _> = srcs
            .into_iter()
            .map(|(p, s)| {
                let f = ParsedFile::parse(&p, &s, lang).unwrap();
                assert!(!f.tree.root_node().has_error());
                (p, f)
            })
            .collect();
        let target_file = if forwarding {
            producer.clone()
        } else {
            format!("good.{ext}")
        };
        let expected = target
            .map(|n| (target_file, n.to_owned()))
            .into_iter()
            .collect();
        let full = CodePropertyGraph::build(&files);
        assert_eq!(
            endpoints(&full, &files, &app),
            expected,
            "{ext} full {epoch}, forward={forwarding}"
        );
        let current = if let Some(old) = previous {
            let changed: BTreeSet<_> = files
                .iter()
                .filter(|(p, f)| previous_sources.get(*p) != Some(&f.source))
                .map(|(p, _)| p.clone())
                .collect();
            assert_eq!(changed, BTreeSet::from([producer.clone()]));
            let incremental = CodePropertyGraph::build_incremental(
                old.call_graph,
                old.dfg,
                &changed,
                &files,
                None,
            );
            assert_eq!(
                endpoints(&incremental, &files, &app),
                expected,
                "{ext} incremental {epoch}, forward={forwarding}"
            );
            incremental
        } else {
            full
        };
        previous = Some(current);
        previous_sources = files
            .iter()
            .map(|(p, f)| (p.clone(), f.source.clone()))
            .collect();
    }
}
#[test]
fn cjs_refusal_js_epochs() {
    for mode in [false, true] {
        transitions(Language::JavaScript, "js", mode);
    }
}
#[test]
fn cjs_refusal_ts_epochs() {
    for mode in [false, true] {
        transitions(Language::TypeScript, "ts", mode);
    }
}
#[test]
fn cjs_refusal_tsx_epochs() {
    for mode in [false, true] {
        transitions(Language::Tsx, "tsx", mode);
    }
}

#[test]
fn cjs_refusal_raw_serde_and_esm_custody() {
    for (lang, ext) in LANGUAGES {
        for esm in [false, true] {
            let source = format!(
                "{CJS} module.exports.other = origin; const alias = module.exports; {}",
                if esm {
                    "export { origin as item };"
                } else {
                    ""
                }
            );
            let path = format!("origin.{ext}");
            let parsed = ParsedFile::parse(&path, &source, lang).unwrap();
            let facts = parsed.extract_js_ts_export_facts();
            assert_eq!(facts.named.len(), 2);
            assert!(matches!(
                facts.named["other"],
                JsExportTarget::UnprovenLocal(_)
            ));
            assert_eq!(matches!(facts.named["item"], JsExportTarget::Local(_)), esm);
            let restored: JsExportFacts =
                serde_json::from_slice(&serde_json::to_vec(&facts).unwrap()).unwrap();
            assert_eq!(facts, restored);
            let resolved =
                resolve_js_exports(&BTreeMap::from([(path.clone(), restored)]), &|_, _| None);
            assert_eq!(
                resolved.resolved.get(&path).map_or(0, |m| m.len()),
                usize::from(esm)
            );
            if esm {
                assert_eq!(resolved.resolved[&path]["item"].local_name, "origin");
            }
        }
    }
}
