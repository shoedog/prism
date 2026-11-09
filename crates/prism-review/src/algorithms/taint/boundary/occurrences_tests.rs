use super::*;
use crate::algorithms::taint::{self, TaintConfig};
use prism::access_path::AccessPath;
use prism::ast::ParsedFile;
use prism::call_graph::CallGraph;
use prism::cpg::CpgContext;
use prism::data_flow::VarAccessKind;
use prism::diff::{DiffInfo, DiffInput, ModifyType};
use prism::languages::Language;
use std::collections::BTreeMap;

fn parsed(path: &str, source: &str) -> ParsedFile {
    ParsedFile::parse(path, source, Language::from_path(path).unwrap()).unwrap()
}

fn calls(parsed: &ParsedFile) -> Vec<CallSite> {
    CallGraph::build(&BTreeMap::from([(parsed.path.clone(), parsed.clone())]))
        .calls
        .into_values()
        .flatten()
        .collect()
}

fn location(parsed: &ParsedFile, call: &CallSite, text: &str, start: usize) -> VarLocation {
    VarLocation {
        file: parsed.path.clone(),
        function: call.caller.name.clone(),
        function_start_line: call.caller.start_line,
        line: parsed.source[..start]
            .bytes()
            .filter(|b| *b == b'\n')
            .count()
            + 1,
        path: AccessPath::from_expr(text),
        start_byte: start,
        end_byte: start + text.len(),
        kind: VarAccessKind::Use,
    }
}

#[test]
fn exact_arguments_distinguish_adjacent_nested_multiline_and_field_uses() {
    let source = "function h(obj) { f('safe'); f(\n inner(obj.field)); g(\n obj.field); }";
    let parsed = parsed("a.js", source);
    let facts = ReviewOccurrenceFacts::new(&parsed);
    let sites = calls(&parsed);
    let mut positive = 0;
    for (start, _) in source.match_indices("obj.field") {
        for call in &sites {
            let value = location(&parsed, call, "obj.field", start);
            let expected = call.start_byte < start && start < call.end_byte;
            assert_eq!(
                matches_argument(&facts, call, &value),
                expected,
                "{}",
                call.callee_name
            );
            positive += usize::from(expected);
            let mut wrong = value.clone();
            wrong.path = AccessPath::from_expr("obj.other");
            assert!(!matches_argument(&facts, call, &wrong));
        }
    }
    assert_eq!(positive, 3);
}

#[test]
fn semantic_arguments_exclude_callees_types_and_keyword_labels() {
    let mut positives = Vec::new();
    for (path, source, wanted, rejected) in [
        ("a.js", "function h(x) { outer(x('safe')); }", "'safe'", "x"),
        ("a.ts", "function h(x: X) { outer(inner<X>(x)); }", "x", "X"),
        (
            "a.py",
            "def h(x):\n    outer(x, keyword=x, *x, **x)\n",
            "x",
            "keyword",
        ),
    ] {
        let parsed = parsed(path, source);
        let facts = ReviewOccurrenceFacts::new(&parsed);
        let call = calls(&parsed)
            .into_iter()
            .find(|call| call.callee_name == "outer")
            .unwrap();
        let start = source[call.start_byte..].find(rejected).unwrap() + call.start_byte;
        assert!(!matches_argument(
            &facts,
            &call,
            &location(&parsed, &call, rejected, start)
        ));
        if path != "a.js" {
            for (offset, _) in source[call.start_byte..call.end_byte].match_indices(wanted) {
                let start = call.start_byte + offset;
                positives.push(matches_argument(
                    &facts,
                    &call,
                    &location(&parsed, &call, wanted, start),
                ));
            }
        }
    }
    assert_eq!(positives, [true; 5]);
}

#[test]
fn callback_values_do_not_belong_to_outer_calls() {
    let mut recognized_callback = false;
    for (path, source) in [
        ("a.js", "function h(x) { outer(() => inner(x)); }"),
        ("a.py", "def h(x):\n    outer(lambda: inner(x))\n"),
        (
            "a.go",
            "package p\nfunc h(x string) { outer(func() { inner(x) }) }",
        ),
        ("a.js", "function h(x) { outer(function*(){ inner(x); }); }"),
    ] {
        let parsed = parsed(path, source);
        let facts = ReviewOccurrenceFacts::new(&parsed);
        let sites = calls(&parsed);
        let outer = sites
            .iter()
            .find(|call| call.callee_name == "outer")
            .unwrap();
        let start = source.rfind("x)").unwrap();
        assert!(!matches_argument(
            &facts,
            outer,
            &location(&parsed, outer, "x", start)
        ));
        if source.contains("() =>") {
            let inner = sites
                .iter()
                .find(|call| call.callee_name == "inner")
                .unwrap();
            recognized_callback =
                matches_argument(&facts, inner, &location(&parsed, inner, "x", start));
        }
    }
    assert!(recognized_callback);
}

#[test]
fn malformed_identity_and_empty_spans_never_match() {
    let parsed = parsed("a.js", "function h(x) { f(x); }");
    let facts = ReviewOccurrenceFacts::new(&parsed);
    let call = calls(&parsed)
        .into_iter()
        .find(|call| call.callee_name == "f")
        .unwrap();
    let value = location(&parsed, &call, "x", parsed.source.rfind('x').unwrap());
    for field in 0..8 {
        let mut bad = value.clone();
        match field {
            0 => bad.end_byte = bad.start_byte,
            1 => bad.start_byte = bad.end_byte + 1,
            2 => bad.file = "other.js".into(),
            3 => bad.function = "other".into(),
            4 => bad.function_start_line += 1,
            5 => bad.start_byte -= 1,
            6 => bad.kind = VarAccessKind::Def,
            _ => bad.line += 1,
        }
        assert!(
            !matches_argument(&facts, &call, &bad),
            "value field {field}"
        );
    }
    for field in 0..7 {
        let mut bad = call.clone();
        match field {
            0 => bad.start_byte += 1,
            1 => bad.end_byte += 1,
            2 => bad.callee_name = "other".into(),
            3 => bad.caller.end_line += 1,
            4 => bad.caller.file = "other.js".into(),
            5 => bad.line += 1,
            _ => bad.jsx_element = true,
        }
        assert!(
            !matches_argument(&facts, &bad, &value),
            "call field {field}"
        );
    }
}

#[test]
fn exact_value_positive_and_effective_source_callee() {
    let parsed = parsed("a.js", "function h(x) { f(x); }");
    let facts = ReviewOccurrenceFacts::new(&parsed);
    let mut call = calls(&parsed)
        .into_iter()
        .find(|call| call.callee_name == "f")
        .unwrap();
    let value = location(&parsed, &call, "x", parsed.source.rfind('x').unwrap());
    assert!(matches_argument(&facts, &call, &value));
    call.source_callee_name = Some("f".into());
    call.callee_name = "derived".into();
    assert!(matches_argument(&facts, &call, &value));
    call.source_callee_name = Some("wrong".into());
    assert!(!matches_argument(&facts, &call, &value));
}

#[test]
fn ambiguous_named_and_arrow_owners_are_rejected() {
    // The graph already refuses some ambiguous owners. Supply an exact
    // detector-level site from a clean template; this is not a graph claim.
    let template = parsed("a.js", "function h(x) { f(x); }");
    let template = calls(&template)
        .into_iter()
        .find(|call| call.callee_name == "f")
        .unwrap();
    for source in [
        "function h(x) { f(x); } function h(y) { g(y); }",
        "var h = x => f(x); var h = y => g(y);",
    ] {
        let parsed = parsed("a.js", source);
        let facts = ReviewOccurrenceFacts::new(&parsed);
        let mut call = template.clone();
        call.start_byte = source.find("f(x)").unwrap();
        call.end_byte = call.start_byte + 4;
        let start = source.find("f(x)").unwrap() + 2;
        assert!(!matches_argument(
            &facts,
            &call,
            &location(&parsed, &call, "x", start)
        ));
    }
}

fn trace_control(path: &str, source: &str, line: usize) {
    let parsed = parsed(path, source);
    let files = BTreeMap::from([(path.into(), parsed)]);
    let ctx = CpgContext::build(&files, None);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.into(),
            modify_type: ModifyType::Modified,
            diff_lines: [line].into(),
        }],
    };
    let config = TaintConfig::default();
    let legacy = taint::slice(&ctx, &diff, &config).unwrap();
    let (result, trace) = taint::slice_with_trace(&ctx, &diff, &config).unwrap();
    let parsed = &files[path];
    let facts = ReviewOccurrenceFacts::new(parsed);
    let sites = calls(parsed);
    let unknown = sites
        .iter()
        .find(|call| call.callee_name == "unknown")
        .unwrap();
    let clean = sites
        .iter()
        .find(|call| call.callee_name == "clean")
        .unwrap();
    let values: Vec<_> = trace
        .roots
        .iter()
        .flat_map(|root| {
            std::iter::once(&root.location).chain(root.reached.iter().map(|r| &r.location))
        })
        .collect();
    assert!(
        values
            .iter()
            .all(|value| !matches_argument(&facts, clean, value)),
        "{path}"
    );
    assert!(
        !result.findings.is_empty(),
        "positive finding control {path}"
    );
    assert!(
        result.evidence.iter().flatten().any(|e| !e.hops.is_empty()),
        "positive evidence control {path}"
    );
    assert_eq!(
        serde_json::to_value(&legacy).unwrap(),
        serde_json::to_value(&result).unwrap()
    );
    assert_eq!(
        format!("{:?}", legacy.evidence),
        format!("{:?}", result.evidence)
    );
    assert!(
        values
            .iter()
            .any(|value| matches_argument(&facts, unknown, value)),
        "{path}"
    );
}

#[test]
fn real_js_trace_and_parity() {
    trace_control(
        "a.js",
        "function h(x) {\n let y = x;\n unknown(y); eval(y); clean('safe');\n}",
        2,
    );
}

#[test]
fn real_python_trace_and_parity() {
    trace_control(
        "a.py",
        "import os\ndef h(x):\n    y = x\n    unknown(y)\n    os.system(y)\n    clean('safe')\n",
        3,
    );
}

#[test]
fn real_go_trace_and_parity() {
    trace_control("a.go", "package p\nimport \"os/exec\"\nfunc h(x string) {\n y := x\n unknown(y); exec.Command(y); clean(\"safe\")\n}", 4);
}
