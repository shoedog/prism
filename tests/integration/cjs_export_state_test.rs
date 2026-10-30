//! Exact producer-only changed sets, including persisted raw fact round trips.
use super::esm_forwarding_state_test::endpoints;
use prism::{ast::ParsedFile, cpg::CodePropertyGraph, languages::Language};
use std::collections::{BTreeMap, BTreeSet};

fn transitions(lang: Language, ext: &str) {
    let origin_path = format!("origin.{ext}");
    let app_path = format!("app.{ext}");
    let safe = "function origin(input) { return input; }\nexports.item = origin;";
    let object = "function origin(input) { return input; }\nmodule.exports = { item: origin };";
    let states = [
        (safe.to_owned(), true),
        (format!("{safe} const alias = exports; alias.item = () => 0;"), false),
        (safe.to_owned(), true),
        (format!("{safe} exports.item = () => 0;"), false),
        (object.to_owned(), true),
        (format!("{object} module.exports = {{}};"), false),
        (object.to_owned(), true),
        ("function origin(input) { return input; }\nmodule.exports = { item: origin, get item() { return () => 0; } };".to_owned(), false),
        (safe.to_owned(), true),
    ];
    let mut previous: Option<CodePropertyGraph> = None;
    let mut previous_sources = BTreeMap::new();
    for (epoch, (source, admitted)) in states.into_iter().enumerate() {
        let files: BTreeMap<_, _> = [
            (origin_path.clone(), source),
            (app_path.clone(), "const { item: invoke } = require('./origin');\nfunction run(value) { return invoke(value); }".to_owned()),
            (format!("decoy.{ext}"), "function invoke(input) { return input; }".to_owned()),
        ].into_iter().map(|(p,s)| {
            let parsed = ParsedFile::parse(&p, &s, lang).unwrap();
            assert!(!parsed.tree.root_node().has_error());
            (p, parsed)
        }).collect();
        let mut full = CodePropertyGraph::build(&files);
        let expected = if admitted {
            BTreeSet::from([(origin_path.clone(), "origin".to_owned())])
        } else {
            BTreeSet::new()
        };
        assert_eq!(
            endpoints(&full, &files, &app_path),
            expected,
            "{ext} full epoch{epoch}"
        );
        // Exercise serde custody of both admitted and refused producer facts.
        let bytes = serde_json::to_string(&full.call_graph.js_ts_exports).unwrap();
        full.call_graph.js_ts_exports = serde_json::from_str(&bytes).unwrap();
        full.call_graph.apply_js_export_resolution();
        assert_eq!(
            full.call_graph
                .js_ts_resolved_exports
                .get(&origin_path)
                .is_some_and(|m| m.contains_key("item")),
            admitted
        );
        let current = if let Some(old) = previous {
            let changed: BTreeSet<_> = files
                .iter()
                .filter(|(p, f)| previous_sources.get(*p) != Some(&f.source))
                .map(|(p, _)| p.clone())
                .collect();
            assert_eq!(
                changed,
                BTreeSet::from([origin_path.clone()]),
                "exact epoch{epoch} changes"
            );
            let incremental = CodePropertyGraph::build_incremental(
                old.call_graph,
                old.dfg,
                &changed,
                &files,
                None,
            );
            assert_eq!(
                endpoints(&incremental, &files, &app_path),
                expected,
                "{ext} incremental epoch{epoch}"
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
fn cjs_js_epochs() {
    transitions(Language::JavaScript, "js");
}
#[test]
fn cjs_ts_epochs() {
    transitions(Language::TypeScript, "ts");
}
#[test]
fn cjs_tsx_epochs() {
    transitions(Language::Tsx, "tsx");
}
