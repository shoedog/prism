use super::pattern_formals_tests::{evidence, refs};
use super::{FileRefs, Producer};
use crate::ast::ParsedFile;
use crate::cpg::{FlowConfidence, FlowDoubt};
use crate::data_flow::{ExactFlowEdge, VarLocation};
use crate::languages::Language;

const INCOMPLETE: FlowConfidence = FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete);
const ALIAS: FlowConfidence = FlowConfidence::NameOnly(FlowDoubt::AliasUnstable);

fn spans(loc: &VarLocation) -> [usize; 2] {
    [loc.start_byte, loc.end_byte]
}

fn emitted_label(graph: &FileRefs, path: &str, from: [usize; 2], to: [usize; 2]) -> FlowConfidence {
    let edge = graph
        .edges
        .iter()
        .find(|edge| {
            edge.from.path.to_string() == path && spans(&edge.from) == from && spans(&edge.to) == to
        })
        .unwrap_or_else(|| panic!("missing {path} {from:?} -> {to:?}"));
    graph
        .exact_labels
        .get(&ExactFlowEdge::from_legacy(edge))
        .copied()
        .unwrap_or_else(|| graph.labels[&(edge.from.clone(), edge.to.clone())])
}

#[test]
fn raw_var_keeps_legacy_signature_endpoint_and_body_label() {
    let source = "function C({x}, f=()=> see(x)) {\n  var x = 0;\n  return x;\n}";
    assert_eq!(
        emitted_label(&refs(source, Language::JavaScript), "x", [39, 40], [0, 0]),
        INCOMPLETE
    );
    let source = "function C({x}, y = x) {\n  var x = 0;\n  return [x, y];\n}";
    for language in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        assert_eq!(
            emitted_label(&refs(source, language), "x", [31, 32], [48, 49]),
            FlowConfidence::Exact
        );
    }
}

#[test]
fn signature_and_wholly_body_candidates_match_legacy_outside_relocation() {
    let cases = [
        ("f=()=>\nsee(x)", "return x;", false),
        ("y = x", "return [x, y];", false),
        ("y = [x]", "return use(x);", false),
        ("{y = x}", "return x;", false),
        ("{x, y = x}", "return x;", false),
        ("y = () => x", "return x;", false),
        ("y = x + y", "return x;", false),
        ("f=()=>\nx", "return use(1);", false),
        ("f=()=>\nx", "return x.k;", true),
        ("f = () => x.k", "with ({}) { return x.k; }", true),
        ("f = () => x.k", "if (x.k) return 1;", true),
    ];
    for (signature, body, alias) in cases {
        let binding = if signature.starts_with("{x,") {
            ""
        } else {
            "{x}, "
        };
        let declaration = if alias {
            "var x;\n  const alias = x;\n  alias.k = 0;"
        } else {
            "var x = 0;"
        };
        let source = format!("function C({binding}{signature}) {{\n  {declaration}\n  {body}\n}}");
        let file = ParsedFile::parse("test", &source, Language::JavaScript).unwrap();
        let base = Producer::file_refs_frozen_with_cap(&file.path, &file, None);
        let live = refs(&source, Language::JavaScript);
        let body_var = source.find("var x").unwrap() + 4;
        let store = source.find("alias.k = 0");
        let selected = |graph: &FileRefs| {
            let mut edges: Vec<_> = graph
                .edges
                .iter()
                .filter(|edge| {
                    (edge.from.start_byte == body_var || Some(edge.from.start_byte) == store)
                        && edge.from.path.base == "x"
                })
                .map(|edge| {
                    (
                        ExactFlowEdge::from_legacy(edge),
                        graph.labels[&(edge.from.clone(), edge.to.clone())],
                    )
                })
                .collect();
            edges.sort();
            edges
        };
        let expected = selected(&base);
        assert!(!expected.is_empty(), "{source}");
        assert_eq!(selected(&live), expected, "{source}");
    }
}

#[test]
fn alias_twins_keep_mixed_line_with_and_non_read_legacy_candidates() {
    let source = "function C({x}, f=()=>\nx) {\n  var x;\n  const alias = x;\n  alias.k = 0;\n  return x.k;\n}\n";
    let graph = refs(source, Language::JavaScript);
    for def in [[34, 35], [45, 50]] {
        assert_eq!(emitted_label(&graph, "x", def, [23, 23]), INCOMPLETE);
    }

    let source = "function C({x = 0}, f = () => x.k) {\n  var x;\n  const alias = x;\n  alias.k = 0;\n  with ({}) { return x.k; }\n}\n";
    assert_eq!(
        emitted_label(
            &refs(source, Language::JavaScript),
            "x.k",
            [67, 74],
            [101, 104]
        ),
        ALIAS
    );

    let source = "function C({x = 0}, f = () => x.k) {\n  var x;\n  const alias = x;\n  alias.k = 0;\n  x.k = 1;\n}\n";
    assert_eq!(
        emitted_label(&refs(source, Language::JavaScript), "x", [54, 59], [80, 80]),
        ALIAS
    );

    let source = "function C({x = 0}, f = () => x) {\n  var x = 0;\n  x = 1;\n}\n";
    assert_eq!(
        emitted_label(&refs(source, Language::JavaScript), "x", [41, 42], [0, 0]),
        INCOMPLETE
    );
}

#[test]
fn entry_precedes_a_signature_write() {
    let source = "function C({ a }, b = (a = 1)) {\n  return use(a);\n}";
    assert_eq!(
        emitted_label(&refs(source, Language::JavaScript), "a", [23, 24], [46, 47]),
        FlowConfidence::Exact
    );
}

#[test]
fn entry_precedes_a_computed_key_write() {
    let source = "function C({o}, {[(o = 5)]: y}) {\n  return o;\n}";
    assert_eq!(
        emitted_label(&refs(source, Language::JavaScript), "o", [19, 20], [43, 44]),
        FlowConfidence::Exact
    );
}

#[test]
fn entry_cannot_overwrite_the_write_in_a_signature_closure() {
    let source = "function C({o}, p = () => { o.k = 1; }) {\n  p();\n  return o.k;\n}";
    assert_eq!(
        emitted_label(
            &refs(source, Language::JavaScript),
            "o.k",
            [28, 31],
            [58, 61]
        ),
        INCOMPLETE
    );
}

#[test]
fn entry_precedes_the_signature_alias_twin() {
    let source = "function C({o}, p = o) {\n  return use(o.k);\n}";
    assert_eq!(
        emitted_label(&refs(source, Language::JavaScript), "o", [16, 17], [38, 39]),
        ALIAS
    );
}

fn check_logical_key(operator: &str, first: &str, second: &str) {
    for body in [" return o; ", "\n  observe(0);\n  return o;\n"] {
        let source = format!(
            "function C({{o}}, {{[(o {operator} 5)]: y}}) {{{body}}}\nC({{o:{first}}}, {{}});\nC({{o:{second}}}, {{}});"
        );
        let receipt = evidence(&source, Language::JavaScript);
        let at = source.find("return o").unwrap() + "return ".len();
        let read = receipt.files["test"]
            .attempts
            .iter()
            .find(|attempt| attempt.key.operand == [at, at + 1])
            .unwrap();
        assert_eq!(read.disposition.as_str(), "emitted", "{source}");
        let endpoint = read.endpoint.as_ref().unwrap();
        assert_eq!(endpoint.bytes, [at, at + 1]);
        assert!(
            matches!(
                endpoint.label,
                Some(FlowConfidence::NameOnly(
                    FlowDoubt::CfgIncomplete | FlowDoubt::SameLine
                ))
            ),
            "{source}: {:?}",
            endpoint.label
        );
    }
}

#[test]
fn arithmetic_and_bitwise_compound_key_writes_replace_the_entry() {
    for operator in [
        "+=", "-=", "*=", "/=", "%=", "**=", "<<=", ">>=", ">>>=", "&=", "^=", "|=",
    ] {
        let source = format!(
            "function C({{o}}, {{[(o {operator} 5)]: y}}) {{\n  return o;\n}}\nC({{o:7}}, {{}});"
        );
        let receipt = evidence(&source, Language::JavaScript);
        let at = source.find("return o").unwrap() + "return ".len();
        let read = receipt.files["test"]
            .attempts
            .iter()
            .find(|attempt| attempt.key.operand == [at, at + 1])
            .unwrap();
        assert_eq!(read.disposition.as_str(), "refused_s1_replaced", "{source}");
    }
}

#[test]
fn logical_or_key_keeps_the_entry_when_truthy() {
    check_logical_key("||=", "7", "0");
}

#[test]
fn logical_and_key_keeps_the_entry_when_falsy() {
    check_logical_key("&&=", "0", "7");
}

#[test]
fn logical_nullish_key_keeps_the_entry_when_not_nullish() {
    check_logical_key("??=", "7", "null");
}

#[test]
fn delete_unqualified_identifier_does_not_read_its_value() {
    for operand in ["o", "(o)", "((o))", "(0, o)", "o.k"] {
        let source = format!("function C({{o}}) {{\n  return delete {operand};\n}}");
        let receipt = evidence(&source, Language::JavaScript);
        let expected = u64::from(operand == "(0, o)");
        assert_eq!(receipt.counters()["candidates"], expected, "{source}");
        assert_eq!(
            refs(&source, Language::JavaScript)
                .edges
                .iter()
                .filter(|edge| spans(&edge.from) == [12, 13])
                .count() as u64,
            expected,
            "{source}"
        );
    }
    let source = "function C({o}) {\n  delete o.k;\n  return o.j;\n}";
    let receipt = evidence(source, Language::JavaScript);
    let attempts = &receipt.files["test"].attempts;
    assert_eq!(attempts.len(), 1);
    assert_eq!(
        attempts[0].disposition.as_str(),
        "refused_param_list_write_effect"
    );
}
