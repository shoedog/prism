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
/// True for a JS-family owner whose statement universe is exactly one node,
/// whose definitions all bind distinct names, and that holds no hazard. The
/// second condition keeps the result sound for the definitions RD extracts: a
/// node cannot order two definitions of one name that it contains, so RD over
/// it could not see one kill the other. The hazards are the writes and deferred
/// reads the definition census does not record (see [`first_hazard`]); each one
/// keeps today's refusal. An owner with zero statements (an empty body) has
/// nothing to flow and keeps the refusal. Both the solver and the cap preflight
/// call this one function, so they cannot disagree.
pub(crate) fn single_node_cfg_is_complete(
    parsed: &ParsedFile,
    owner: &Node<'_>,
    statement_lines: usize,
    def_bases: &[&str],
) -> bool {
    if !is_js_family(parsed.language) || statement_lines != 1 {
        return false;
    }
    let mut seen = BTreeSet::new();
    def_bases.iter().all(|base| seen.insert(*base))
        && first_hazard(parsed, owner, def_bases).is_none()
}

/// Conservative hazards for the one-node path. The def census records no def
/// for these, so distinct bases would admit an owner whose formal is
/// overwritten or whose read is deferred. Refusing too much only keeps main's
/// label. Pre-existing multi-statement owners are not touched (#660).
///
/// - (a) an update expression (`x++`, `--x`), anywhere in the owner, its own
///   parameter defaults included;
/// - (b) a default inside an assignment pattern (`[a = 1] = ..`,
///   `({ a = 1 } = ..)`), anywhere except the owner's own formal
///   wrappers (their default values are walked);
/// - (c) a function declaration in the owner's own scope named like a def
///   (hoisted: it replaces the formal at entry);
/// - (d) `arguments` in the owner or in an arrow nested in it (sloppy-mode
///   mapped arguments alias the formals);
/// - (e) a direct `eval(..)` call (it can write any local), found by
///   `js_ts_direct_eval_anywhere`;
/// - (f) a nested callable with a parameter default (the default is a read
///   that runs when the callable runs: a deferred capture the B8 metadata
///   does not collect, which only walks nested bodies).
pub(crate) fn first_hazard(
    parsed: &ParsedFile,
    owner: &Node<'_>,
    def_bases: &[&str],
) -> Option<char> {
    // A direct eval anywhere in the owner (own defaults and nested callables
    // included) can write any local. The shared detector handles parentheses,
    // TS wrappers, comments and escaped identifiers; indirect and optional
    // calls are not direct.
    if parsed.js_ts_direct_eval_anywhere(owner) {
        return Some('e');
    }
    let boundaries = parsed.language.callable_boundary_node_types();
    let own_params = owner
        .child_by_field_name("parameters")
        .or_else(|| owner.child_by_field_name("parameter"))
        .map(|node| node.id());
    // (node, inside a nested callable, `arguments` is the owner's, inside the
    // owner's own parameter list)
    let mut stack = vec![(*owner, false, true, false)];
    while let Some((node, nested, own_args, in_params)) = stack.pop() {
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            let kind = child.kind();
            // The owner's own formal binding/default wrappers are not hazards
            // (`b = 1` is an assignment pattern); their default values are
            // walked like any expression. Defaults make the list non-simple,
            // so `arguments` there is unmapped.
            let wrapper = in_params && !nested;
            match kind {
                "update_expression" => return Some('a'),
                "assignment_pattern" | "object_assignment_pattern" if !wrapper => {
                    return Some(if nested { 'f' } else { 'b' })
                }
                "required_parameter" | "optional_parameter"
                    if !wrapper && child.child_by_field_name("value").is_some() =>
                {
                    return Some('f');
                }
                "identifier" if own_args && parsed.node_text(&child) == "arguments" => {
                    return Some('d');
                }
                "function_declaration" | "generator_function_declaration"
                    if !nested
                        && child
                            .child_by_field_name("name")
                            .is_some_and(|name| def_bases.contains(&parsed.node_text(&name))) =>
                {
                    return Some('c');
                }
                _ => {}
            }
            let boundary = boundaries.contains(&kind);
            let params_here = Some(child.id()) == own_params && node.id() == owner.id();
            // The default value of an own formal is an ordinary expression: leave
            // the wrapper state so a pattern default written inside it counts.
            let default_value = in_params
                && !nested
                && matches!(
                    node.kind(),
                    "assignment_pattern"
                        | "object_assignment_pattern"
                        | "required_parameter"
                        | "optional_parameter"
                )
                && ["right", "value"]
                    .iter()
                    .any(|field| node.child_by_field_name(field) == Some(child));
            stack.push((
                child,
                nested || boundary,
                own_args && !params_here && (!boundary || kind == "arrow_function"),
                (in_params || params_here) && !default_value,
            ));
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_only_for_one_node_with_distinct_names_in_the_js_family() {
        for language in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
            let parsed =
                ParsedFile::parse("a.js", "function f(a, b) { g(a, b); }", language).unwrap();
            let owner = parsed.all_functions().into_iter().next().unwrap();
            assert!(single_node_cfg_is_complete(&parsed, &owner, 1, &["a", "b"]));
            assert!(single_node_cfg_is_complete(&parsed, &owner, 1, &[]));
            assert!(!single_node_cfg_is_complete(&parsed, &owner, 0, &["a"]));
            assert!(!single_node_cfg_is_complete(&parsed, &owner, 2, &["a"]));
            assert!(!single_node_cfg_is_complete(
                &parsed,
                &owner,
                1,
                &["a", "a"]
            ));
        }
        let py = ParsedFile::parse("a.py", "def f(a):\n    g(a)\n", Language::Python).unwrap();
        let owner = py.all_functions().into_iter().next().unwrap();
        assert!(!single_node_cfg_is_complete(&py, &owner, 1, &["a"]));
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
