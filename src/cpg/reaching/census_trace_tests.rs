//! The S1a trace names the mechanism of exactly the edges its rules decide.
use super::super::capture::{capture_facts_with, is_capture_to};
use super::*;
use crate::ast::ParsedFile;
use crate::cpg::CodePropertyGraph;
use crate::data_flow::test_support::label_snapshot;
use crate::languages::Language;
use std::collections::BTreeMap;

/// The trace lines `is_capture_to` writes for every edge of the source's one top-level function.
fn trace_lines(language: Language, ext: &str, src: &str, trace: bool) -> Vec<String> {
    let name = format!("a.{ext}");
    let file = ParsedFile::parse(&name, src, language).unwrap();
    assert_eq!(file.parse_error_count, 0, "{src}");
    let owner = file.tree.root_node().child(0).unwrap();
    let facts = capture_facts_with(&file, owner, trace);
    let graph_file = ParsedFile::parse(&name, src, language).unwrap();
    let cpg = CodePropertyGraph::build(&BTreeMap::from([(name, graph_file)]));
    let mut out = Vec::new();
    for (from, to) in label_snapshot(&cpg.dfg).into_keys() {
        is_capture_to(&FlowEdge { from, to }, &facts, &mut out);
    }
    String::from_utf8(out)
        .unwrap()
        .lines()
        .map(String::from)
        .collect()
}

const PARAM: &str = "function f(y) {\n  log();\n  g(function (z = y) { return z; });\n}\n";
const FIELD: &str =
    "function f(a) {\n  const C = class {\n    x = a;\n  };\n  return new C();\n}\n";
const BELT: &str =
    "function f(a) {\n  log();\n  g(() => { a = 2; });\n  h(() => { sink(a); });\n}\n";
const MAIN: &str = "function f(y) {\n  log();\n  g(function (z) { return y; });\n}\n";

fn fields(line: &str) -> Vec<&str> {
    line.split('\t').collect()
}

#[test]
fn each_mechanism_is_named_with_its_positions() {
    for (src, mechanism, from, to) in [
        (PARAM, CAPTURE_PARAM, "y@1:11", "y@3:"),
        (FIELD, CAPTURE_FIELD, "a@1:11", "a@3:"),
        (BELT, CAPTURE_REGION, "a@3:", "a@4:"),
    ] {
        for (language, ext) in [(Language::JavaScript, "js"), (Language::TypeScript, "ts")] {
            let lines = trace_lines(language, ext, src, true);
            assert_eq!(lines.len(), 1, "{mechanism}: {lines:?}");
            let f = fields(&lines[0]);
            assert_eq!(f.len(), 6, "{lines:?}");
            assert_eq!(&f[..4], ["rd-census", mechanism, &format!("a.{ext}"), "f"]);
            assert!(f[4].starts_with(from), "{from}: {lines:?}");
            assert!(f[5].starts_with(to), "{to}: {lines:?}");
            let byte = f[5].rsplit(':').next().unwrap();
            assert!(byte.parse::<usize>().is_ok(), "{lines:?}");
        }
    }
}

#[test]
fn a_main_capture_is_not_traced() {
    for (language, ext) in [(Language::JavaScript, "js"), (Language::Tsx, "tsx")] {
        assert!(trace_lines(language, ext, MAIN, true).is_empty());
    }
}

#[test]
fn unset_writes_nothing() {
    for src in [PARAM, FIELD, BELT, MAIN] {
        assert!(trace_lines(Language::JavaScript, "js", src, false).is_empty());
    }
}

#[test]
fn other_languages_are_never_traced() {
    let py = "def f(y):\n    log()\n    g(lambda z=y: z)\n    h(lambda: y)\n";
    assert!(trace_lines(Language::Python, "py", py, true).is_empty());
}

#[test]
fn a_nested_body_read_main_did_not_place_is_capture_body() {
    // Main's collector stops at the member expression and never sees the receiver `o`.
    let src = "function f(o) {\n  log();\n  g(() => (o).x);\n}\n";
    for (language, ext) in [(Language::JavaScript, "js"), (Language::TypeScript, "ts")] {
        let lines = trace_lines(language, ext, src, true);
        assert_eq!(lines.len(), 1, "{lines:?}");
        assert_eq!(fields(&lines[0])[1], CAPTURE_BODY);
    }
    // A wrapped receiver in a parameter default is still a parameter capture.
    let param = "function f(y) {\n  log();\n  g(function (z = box(y).x) { return z; });\n}\n";
    let lines = trace_lines(Language::JavaScript, "js", param, true);
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert_eq!(fields(&lines[0])[1], CAPTURE_PARAM);
}
