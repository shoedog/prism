//! #598: the CommonJS module-call route through the whole graph. A producer edit that admits or
//! revokes its export (and a consumer edit that rebinds the name) must grant or withdraw the
//! argument-to-parameter flow in a full build, an incremental one, and after a serde round trip
//! of the raw facts; the producer-only changed set is the only one that moves.
use super::esm_forwarding_state_test::endpoints;
use prism::{ast::ParsedFile, cpg::CodePropertyGraph, languages::Language};
use std::collections::{BTreeMap, BTreeSet};

const SAFE: &str = "function origin(input) { return input; }\nmodule.exports = origin;";
const OBJECT: &str = "function origin(input) { return input; }\nmodule.exports = { item: origin };";
const WHOLE: &str = "const m = require('./origin');\nfunction run(value) { return m(value); }";
const MEMBER: &str =
    "const m = require('./origin');\nfunction run(value) { return m.item(value); }";
const INLINE: &str = "function run(value) { return require('./origin').item(value); }";
const ALIAS: &str =
    "const item = require('./origin').item;\nfunction run(value) { return item(value); }";

fn build(lang: Language, origin: &str, app: &str, ext: &str) -> BTreeMap<String, ParsedFile> {
    [
        (format!("origin.{ext}"), origin.to_owned()),
        (format!("app.{ext}"), app.to_owned()),
        (
            format!("decoy.{ext}"),
            "function item(input) { return input; }\nfunction m(input) { return input; }"
                .to_owned(),
        ),
    ]
    .into_iter()
    .map(|(p, s)| {
        let parsed = ParsedFile::parse(&p, &s, lang).unwrap();
        assert!(!parsed.tree.root_node().has_error(), "{p}");
        (p, parsed)
    })
    .collect()
}

fn transitions(lang: Language, ext: &str, app: &str, admitted: &str, refused: &[&str]) {
    let origin = format!("origin.{ext}");
    let app_path = format!("app.{ext}");
    let mut states: Vec<(String, bool)> = vec![(admitted.to_owned(), true)];
    for bad in refused {
        states.push((format!("{admitted}\n{bad}"), false));
        states.push((admitted.to_owned(), true));
    }
    let mut previous: Option<(CodePropertyGraph, BTreeMap<String, String>)> = None;
    for (epoch, (source, expect)) in states.into_iter().enumerate() {
        let files = build(lang, &source, app, ext);
        let expected = if expect {
            BTreeSet::from([(origin.clone(), "origin".to_owned())])
        } else {
            BTreeSet::new()
        };
        let mut full = CodePropertyGraph::build(&files);
        assert_eq!(
            endpoints(&full, &files, &app_path),
            expected,
            "{ext} full {epoch}"
        );
        let bytes = serde_json::to_string(&full.call_graph.js_ts_exports).unwrap();
        full.call_graph.js_ts_exports = serde_json::from_str(&bytes).unwrap();
        full.call_graph.apply_js_export_resolution();
        assert_eq!(
            endpoints(&full, &files, &app_path),
            expected,
            "{ext} serde {epoch}"
        );
        let sources: BTreeMap<String, String> = files
            .iter()
            .map(|(p, f)| (p.clone(), f.source.clone()))
            .collect();
        let current = if let Some((old, old_sources)) = previous {
            let changed: BTreeSet<String> = sources
                .iter()
                .filter(|(p, s)| old_sources.get(*p) != Some(s))
                .map(|(p, _)| p.clone())
                .collect();
            assert_eq!(
                changed,
                BTreeSet::from([origin.clone()]),
                "{ext} changed {epoch}"
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
                "{ext} incremental {epoch}"
            );
            incremental
        } else {
            full
        };
        previous = Some((current, sources));
    }
}

fn all_languages(app: &str, admitted: &str, refused: &[&str]) {
    for (lang, ext) in [
        (Language::JavaScript, "js"),
        (Language::TypeScript, "ts"),
        (Language::Tsx, "tsx"),
    ] {
        transitions(lang, ext, app, admitted, refused);
    }
}

#[test]
fn module_value_epochs() {
    all_languages(
        WHOLE,
        SAFE,
        &[
            "module.exports = function other() {};",
            "const alias = module; alias.exports = other;",
            "function change() { origin = other; }",
        ],
    );
}

#[test]
fn member_epochs() {
    all_languages(
        MEMBER,
        OBJECT,
        &[
            "module.exports.item = other;",
            "module.exports = {};",
            "const e = module.exports; e.item = other;",
        ],
    );
    all_languages(INLINE, OBJECT, &["module.exports.item = other;"]);
    all_languages(ALIAS, OBJECT, &["module.exports.item = other;"]);
}

#[test]
fn consumer_rebind_withdraws_the_route() {
    // Same producer, the consumer edit is the only change: rebound name, then restored.
    for (lang, ext) in [(Language::JavaScript, "js"), (Language::TypeScript, "ts")] {
        let app_path = format!("app.{ext}");
        let origin = format!("origin.{ext}");
        let rebound = format!("{WHOLE}\nm = other;");
        let mut seen = Vec::new();
        for app in [WHOLE, rebound.as_str(), WHOLE] {
            let files = build(lang, SAFE, app, ext);
            let cpg = CodePropertyGraph::build(&files);
            seen.push(endpoints(&cpg, &files, &app_path));
        }
        let yes = BTreeSet::from([(origin, "origin".to_owned())]);
        assert_eq!(seen, [yes.clone(), BTreeSet::new(), yes]);
    }
}
