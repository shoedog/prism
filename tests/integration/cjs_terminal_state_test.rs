//! Producer-only edits must revoke/restore the actual cross-file parameter flow.
use super::esm_forwarding_state_test::endpoints;
use prism::{ast::ParsedFile, cpg::CodePropertyGraph, languages::Language};
use std::collections::{BTreeMap, BTreeSet};

fn transitions(lang: Language, ext: &str) {
    let origin_path = format!("origin.{ext}");
    let app_path = format!("app.{ext}");
    let declaration = "function origin(input) { return input; }";
    let clean = format!("{declaration}\nexports.item = origin;");
    let sources = [
        (clean.clone(), Some("origin")),
        (
            format!("{declaration}\norigin = other; exports.item = origin;"),
            None,
        ),
        (clean.clone(), Some("origin")),
        (format!("{clean}\norigin = other;"), Some("origin")),
        (
            format!("function holder() {{ {declaration} }}\nexports.item = origin;"),
            None,
        ),
        (clean.clone(), Some("origin")),
        (
            "exports.item = origin; const origin = (input) => input;".into(),
            None,
        ),
        (
            "const origin = (input) => input; exports.item = origin;".into(),
            Some("origin"),
        ),
        (clean.replace("origin", "replacement"), Some("replacement")),
        (clean, Some("origin")),
    ];
    let mut previous: Option<CodePropertyGraph> = None;
    for (epoch, (source, target)) in sources.into_iter().enumerate() {
        let files: BTreeMap<_, _> = [
            (origin_path.clone(), source),
            (app_path.clone(), "const { item: invoke } = require('./origin'); function run(value) { return invoke(value); }".into()),
            (format!("decoy.{ext}"), "function invoke(input) { return input; }".into()),
        ].into_iter().map(|(p,s)| { let parsed = ParsedFile::parse(&p,&s,lang).unwrap(); assert!(!parsed.tree.root_node().has_error()); (p,parsed) }).collect();
        let full = CodePropertyGraph::build(&files);
        let expected = target
            .map(|n| (origin_path.clone(), n.to_owned()))
            .into_iter()
            .collect();
        assert_eq!(
            endpoints(&full, &files, &app_path),
            expected,
            "{ext} full epoch{epoch}"
        );
        let current = if let Some(old) = previous {
            let changed = BTreeSet::from([origin_path.clone()]);
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
    }
}

#[test]
fn cjs_terminal_js_source_epochs() {
    transitions(Language::JavaScript, "js");
}
#[test]
fn cjs_terminal_ts_source_epochs() {
    transitions(Language::TypeScript, "ts");
}
#[test]
fn cjs_terminal_tsx_source_epochs() {
    transitions(Language::Tsx, "tsx");
}

#[test]
fn cjs_terminal_unproven_claim_serde() {
    use prism::js_exports::{resolve_js_exports, JsExportFacts, JsExportTarget};
    let source = "function origin(input) { return input; } const version = 1; module.exports = { item: origin, version };";
    let parsed = ParsedFile::parse("origin.js", source, Language::JavaScript).unwrap();
    let facts = parsed.extract_js_ts_export_facts();
    assert_eq!(facts.named["item"], JsExportTarget::Local("origin".into()));
    assert_eq!(
        serde_json::to_value(&facts.named["version"]).unwrap(),
        serde_json::json!({"UnprovenLocal":"version"})
    );
    let restored: JsExportFacts =
        serde_json::from_slice(&serde_json::to_vec(&facts).unwrap()).unwrap();
    assert_eq!(facts, restored);
    let result = resolve_js_exports(
        &BTreeMap::from([("origin.js".into(), restored.clone())]),
        &|_, _| None,
    );
    assert_eq!(result.resolved["origin.js"].len(), 1);
    assert_eq!(result.resolved["origin.js"]["item"].local_name, "origin");
    for first in [true, false] {
        let mut duplicate = JsExportFacts::default();
        let local = facts.named["item"].clone();
        let unproven = restored.named["version"].clone();
        let pair = if first {
            [local, unproven]
        } else {
            [unproven, local]
        };
        for target in pair {
            duplicate.insert_named("item".into(), target);
        }
        assert!(duplicate.conflicted.contains("item"));
        assert!(!duplicate.named.contains_key("item"));
    }
}
