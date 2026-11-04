//! JS/TS/TSX callables whose whole body is one CFG node (#599).
//!
//! The CFG is line-granular: edges join distinct statement lines. A callable
//! whose statement universe is a single line (one statement, several
//! statements sharing a line, or an expression-bodied arrow) therefore has no
//! edge to build, and the reaching-definitions pass used to refuse such an
//! owner (`RdUnavailable::NoCfgEdges`) and label every one of its DataFlow
//! edges `NameOnly(CfgIncomplete)`. A single node has no internal control flow
//! at line granularity, so that CFG is complete, not missing. This module
//! states when the pass may run on it. It adds no CFG edge and no DataFlow
//! edge: only the label of an existing edge can move.
use crate::ast::{ParsedFile, StatementSpan};
use crate::languages::Language;
use std::collections::BTreeSet;
use tree_sitter::Node;

/// Kind of the synthetic statement standing for an expression body.
pub(crate) const EXPRESSION_BODY_KIND: &str = "expression_body";

fn is_js_family(language: Language) -> bool {
    matches!(
        language,
        Language::JavaScript | Language::TypeScript | Language::Tsx
    )
}

/// The one statement an expression-bodied arrow function consists of, when
/// the parser located no more than one statement inside it.
///
/// `statements_in_function` walks the children of a block body, so an
/// expression body yields either nothing or an arbitrary nested node (a call
/// found inside a binary expression covers only part of the body). Both
/// universes are partial. The whole body expression is the statement, and
/// every line of the body maps to its first line. An arrow whose body holds
/// two or more located statements keeps its existing universe.
pub(crate) fn expression_body_statement(
    parsed: &ParsedFile,
    func_node: &Node<'_>,
    located_statements: usize,
) -> Option<StatementSpan> {
    if !is_js_family(parsed.language) || located_statements > 1 {
        return None;
    }
    if func_node.kind() != "arrow_function" {
        return None;
    }
    let body = func_node.child_by_field_name("body")?;
    if body.kind() == "statement_block" {
        return None;
    }
    Some(StatementSpan {
        line: body.start_position().row + 1,
        kind: EXPRESSION_BODY_KIND.to_string(),
        start_byte: body.start_byte(),
        end_byte: body.end_byte(),
    })
}

/// Whether an owner with no CFG edge has a complete, edge-free CFG.
///
/// True for a JS-family owner whose statement universe is exactly one node and
/// whose definitions all bind distinct names. The second condition is what
/// keeps the result sound: a node cannot order two definitions of one name
/// that it contains, so reaching-definitions over it could not see one kill
/// the other. With distinct names no definition kills another, and every
/// definition of a parameter reaches every read in the node. An owner with
/// zero statements (an empty body) has nothing to flow and keeps the refusal.
pub(crate) fn single_node_cfg_is_complete<'a>(
    language: Language,
    statement_lines: usize,
    def_bases: impl IntoIterator<Item = &'a str>,
) -> bool {
    if !is_js_family(language) || statement_lines != 1 {
        return false;
    }
    let mut seen = BTreeSet::new();
    def_bases.into_iter().all(|base| seen.insert(base))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_only_for_one_node_with_distinct_names_in_the_js_family() {
        for language in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
            assert!(single_node_cfg_is_complete(language, 1, ["a", "b"]));
            assert!(single_node_cfg_is_complete(language, 1, []));
            assert!(!single_node_cfg_is_complete(language, 0, ["a"]));
            assert!(!single_node_cfg_is_complete(language, 2, ["a"]));
            assert!(!single_node_cfg_is_complete(language, 1, ["a", "a"]));
        }
        assert!(!single_node_cfg_is_complete(Language::Python, 1, ["a"]));
        assert!(!single_node_cfg_is_complete(Language::Go, 1, ["a"]));
    }

    fn arrow_span(source: &str, located: usize) -> Option<StatementSpan> {
        let parsed = ParsedFile::parse("a.js", source, Language::JavaScript).unwrap();
        let func = parsed.all_functions().into_iter().next().unwrap();
        expression_body_statement(&parsed, &func, located)
    }

    #[test]
    fn expression_body_is_the_synthetic_statement_only_for_small_universes() {
        let span = arrow_span("const f = (c) =>\n  exec(c);\n", 0).unwrap();
        assert_eq!((span.line, span.kind.as_str()), (2, EXPRESSION_BODY_KIND));
        assert!(arrow_span("const f = (c) => exec(c);\n", 1).is_some());
        assert!(arrow_span("const f = (c) => exec(c);\n", 2).is_none());
        assert!(arrow_span("const f = (c) => { exec(c); };\n", 0).is_none());
        assert!(arrow_span("function f(c) { exec(c); }\n", 0).is_none());
    }
}
