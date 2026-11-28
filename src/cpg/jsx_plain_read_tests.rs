//! `jsx_plain_read`: the callee-side proof of one reached read (lane 3 S3, codecov patch). Each
//! refusal is paired with a control that differs only in the guarded fact and is accepted.
//! The endpoints are the shapes the graph hands the binder (an exact read span, a zero-width
//! line anchor, a span no node has), built over real parsed callees.

use super::jsx_props::{jsx_plain_read, FormalReadContext, PlainRead, ReachedRead, ReadRefusal};
use super::*;
use crate::languages::Language;
use petgraph::graph::NodeIndex;

const SOURCE: &str = "function P(p) {\n  return use(p.a, p[b], p);\n}\n";

fn parse(source: &str, language: Language) -> ParsedFile {
    let file = ParsedFile::parse("m.js", source, language).unwrap();
    assert_eq!(file.parse_error_count, 0, "{source}");
    file
}

fn read(
    file: &ParsedFile,
    fields: &[&str],
    line: usize,
    range: std::ops::Range<usize>,
) -> Result<PlainRead, ReadRefusal> {
    let path = AccessPath::with_fields("p", fields.iter().map(|f| (*f).to_owned()).collect());
    let owner = file.all_functions()[0];
    jsx_plain_read(
        file,
        &ReachedRead {
            node: NodeIndex::new(0),
            path: &path,
            line,
            range,
            label: FlowConfidence::Exact,
        },
        FormalReadContext { owner },
    )
}

fn span(source: &str, needle: &str) -> std::ops::Range<usize> {
    let start = source.find(needle).unwrap();
    start..start + needle.len()
}

fn dot(chain: &[&str]) -> Result<PlainRead, ReadRefusal> {
    Ok(PlainRead::Dot(
        chain.iter().map(|f| (*f).to_owned()).collect(),
    ))
}

fn same(left: Result<PlainRead, ReadRefusal>, right: Result<PlainRead, ReadRefusal>) -> bool {
    match (left, right) {
        (Ok(PlainRead::Whole), Ok(PlainRead::Whole)) => true,
        (Ok(PlainRead::Dot(a)), Ok(PlainRead::Dot(b))) => a == b,
        (Err(a), Err(b)) => std::mem::discriminant(&a) == std::mem::discriminant(&b),
        _ => false,
    }
}

#[test]
fn an_exact_read_span_proves_the_whole_formal_or_its_dot_chain() {
    let file = parse(SOURCE, Language::JavaScript);
    let bare = SOURCE.rfind("p)").unwrap();
    assert!(same(
        read(&file, &[], 2, bare..bare + 1),
        Ok(PlainRead::Whole)
    ));
    assert!(same(
        read(&file, &["a"], 2, span(SOURCE, "p.a")),
        dot(&["a"])
    ));
    // A bare path may name the root of a longer chain; the chain is what the read selects.
    assert!(same(read(&file, &[], 2, span(SOURCE, "p.a")), dot(&["a"])));
}

#[test]
fn a_zero_width_bare_anchor_is_refused_but_a_zero_width_projection_is_not() {
    let file = parse(SOURCE, Language::JavaScript);
    let at = SOURCE.find("p.a").unwrap();
    assert!(same(
        read(&file, &[], 2, at..at),
        Err(ReadRefusal::ZeroWidth)
    ));
    // Control: the same anchor for a projected path is judged by its line, not refused as bare.
    assert!(same(read(&file, &["a"], 2, at..at), dot(&["a"])));
}

#[test]
fn a_bracket_read_is_refused_and_a_dot_read_beside_it_is_not() {
    let file = parse(SOURCE, Language::JavaScript);
    assert!(same(
        read(&file, &[], 2, span(SOURCE, "p[b]")),
        Err(ReadRefusal::Bracket)
    ));
    assert!(same(read(&file, &[], 2, span(SOURCE, "p.a")), dot(&["a"])));
}

#[test]
fn a_chain_that_does_not_extend_the_recorded_path_is_unproved() {
    let file = parse(SOURCE, Language::JavaScript);
    // The graph says `p.b` but the syntax at that span reads `p.a`.
    assert!(same(
        read(&file, &["b"], 2, span(SOURCE, "p.a")),
        Err(ReadRefusal::Unproved)
    ));
    assert!(same(
        read(&file, &["a"], 2, span(SOURCE, "p.a")),
        dot(&["a"])
    ));
}

#[test]
fn a_span_that_is_not_a_read_of_the_formal_is_unproved() {
    let file = parse(SOURCE, Language::JavaScript);
    // The property name of `p.a` is not a value read of `p`.
    let property = SOURCE.find(".a").unwrap() + 1;
    assert!(same(
        read(&file, &[], 2, property..property + 1),
        Err(ReadRefusal::Unproved)
    ));
    let shadow = "function P(p) {\n  { let p = 1; use(p); }\n  return p;\n}\n";
    let file = parse(shadow, Language::JavaScript);
    let inner = shadow.find("use(p)").unwrap() + 4;
    assert!(same(
        read(&file, &[], 2, inner..inner + 1),
        Err(ReadRefusal::Unproved)
    ));
    let outer = shadow.rfind("p;").unwrap();
    assert!(same(
        read(&file, &[], 3, outer..outer + 1),
        Ok(PlainRead::Whole)
    ));
}

const LINES: &str = "function P(p) {\n  one(p.a);\n  two(p.a, p.a);\n  three(p.b);\n  four(p.a['x']);\n  five(p.a);\n}\n";

#[test]
fn a_projected_anchor_must_select_exactly_one_receiver_on_its_line() {
    let file = parse(LINES, Language::JavaScript);
    // A range no node has (the middle of `one`) sends the endpoint to its recorded line.
    let anchor = LINES.find("one").unwrap() + 1;
    let blind = anchor..anchor + 1;
    assert!(same(read(&file, &["a"], 2, blind.clone()), dot(&["a"])));
    assert!(same(
        read(&file, &["a"], 3, blind.clone()),
        Err(ReadRefusal::Unproved)
    ));
    assert!(same(
        read(&file, &["a"], 4, blind.clone()),
        Err(ReadRefusal::Unproved)
    ));
    assert!(same(
        read(&file, &["a"], 5, blind.clone()),
        Err(ReadRefusal::Bracket)
    ));
    assert!(same(read(&file, &["b"], 4, blind.clone()), dot(&["b"])));
    assert!(same(read(&file, &["a"], 6, blind.clone()), dot(&["a"])));
    // A line outside the callee selects nothing.
    assert!(same(
        read(&file, &["a"], 99, blind),
        Err(ReadRefusal::Unproved)
    ));
}

#[test]
fn a_bare_anchor_without_a_node_is_unproved_instead_of_searched() {
    let file = parse(LINES, Language::JavaScript);
    let anchor = LINES.find("one").unwrap() + 1;
    // The line search is for projected paths only: a bare path with no exact span is refused.
    assert!(same(
        read(&file, &[], 2, anchor..anchor + 1),
        Err(ReadRefusal::Unproved)
    ));
    assert!(same(
        read(&file, &["a"], 2, anchor..anchor + 1),
        dot(&["a"])
    ));
}

#[test]
fn a_read_that_the_grammar_wraps_is_proved_through_the_wrapper() {
    let source = "function P(p) {\n  return use((p as any).a, p!.a);\n}\n";
    let file = parse(source, Language::Tsx);
    for needle in ["(p as any).a", "p!.a"] {
        assert!(
            same(read(&file, &["a"], 2, span(source, needle)), dot(&["a"])),
            "{needle}"
        );
    }
}
