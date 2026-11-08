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

std::thread_local! {
    /// The trace's control only (#759): main's universe for an arrow whose body is a callable or a ternary.
    static MAIN_ARROW_UNIVERSE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// Runs `f` with an expression-bodied arrow whose body is a callable or a ternary keeping main's
/// statement universe (the located statements). Used only by the `PRISM_RD_CENSUS_TRACE` control pass
/// that names the owners the #759 rule moved; production never sets it.
pub(crate) fn with_main_arrow_universe<T>(f: impl FnOnce() -> T) -> T {
    struct Reset(bool);
    impl Drop for Reset {
        fn drop(&mut self) {
            MAIN_ARROW_UNIVERSE.with(|c| c.set(self.0));
        }
    }
    let _reset = Reset(MAIN_ARROW_UNIVERSE.with(|c| c.replace(true)));
    f()
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum BodyShape {
    /// An arrow, a function expression or a generator function expression: the located statements are
    /// the nested callable's own block statements (#759).
    Callable,
    /// A ternary: the located statements are calls of mutually exclusive arms (#759).
    Ternary,
}

fn body_shape(parsed: &ParsedFile, func_node: &Node<'_>) -> Option<BodyShape> {
    if !is_js_family(parsed.language) || func_node.kind() != "arrow_function" {
        return None;
    }
    match func_node.child_by_field_name("body")?.kind() {
        "arrow_function" | "function_expression" | "generator_function" => {
            Some(BodyShape::Callable)
        }
        "ternary_expression" => Some(BodyShape::Ternary),
        _ => None,
    }
}

/// Whether `func_node` is an expression-bodied arrow whose body expression is itself a callable.
/// `statements_in_function` walks the children of that body, so it reports the callable's own block
/// statements as the arrow's: a universe that belongs to the nested owner (#759).
#[cfg(test)]
pub(crate) fn arrow_body_is_callable(parsed: &ParsedFile, func_node: &Node<'_>) -> bool {
    body_shape(parsed, func_node) == Some(BodyShape::Callable)
}

/// Whether `func_node` is an expression-bodied arrow whose body is a ternary: its arms are direct
/// children of the body, so `statements_in_function` lists the calls of mutually exclusive arms as
/// sequential statements (#759).
#[cfg(test)]
pub(crate) fn arrow_body_is_ternary(parsed: &ParsedFile, func_node: &Node<'_>) -> bool {
    body_shape(parsed, func_node) == Some(BodyShape::Ternary)
}

/// Whether the arrow's located statements are not a sound universe: its body is a callable or a ternary.
pub(crate) fn arrow_body_takes_one_node(parsed: &ParsedFile, func_node: &Node<'_>) -> bool {
    body_shape(parsed, func_node).is_some()
}

/// The one statement an expression-bodied arrow function consists of, when
/// the parser located no more than one statement inside it, or when its body
/// is a callable or a ternary.
///
/// `statements_in_function` walks the children of a block body, so an
/// expression body yields either nothing or an arbitrary nested node (a call
/// found inside a binary expression covers only part of the body). Both
/// universes are partial. The whole body expression is the statement, and
/// every line of the body maps to its first line. Two body shapes take the
/// one node whatever the located count (#759). A callable body
/// (`k => () => { a(); b(); }`): the statements found there are the nested
/// callable's, which the census, S1a capture and D2-c see as a deferred
/// region of this owner. A ternary body (`c(k) ? f(k = 1) : sink(k)`): its
/// arms are direct children, so they were listed as sequential statements
/// although only one arm runs, and a def of one arm reached a use of the
/// other as `exact`. No other direct child puts located statements in
/// mutually exclusive positions: `||`, `&&` and `??` evaluate left to right,
/// so source order is evaluation order and a conditionally skipped def can
/// only over-kill (`killed`, never a wrong `exact`); every other wrapper
/// (parentheses, `as`/`satisfies`/`!`, a call, an array, an object, a
/// sequence) is not descended. An arrow body holding two or more located
/// statements of another shape (`c(k) ||\n d(k)`) keeps its existing universe.
pub(crate) fn expression_body_statement(
    parsed: &ParsedFile,
    func_node: &Node<'_>,
    located_statements: usize,
) -> Option<StatementSpan> {
    if !is_js_family(parsed.language) || func_node.kind() != "arrow_function" {
        return None;
    }
    let one_node_body = arrow_body_takes_one_node(parsed, func_node)
        && !MAIN_ARROW_UNIVERSE.with(std::cell::Cell::get);
    if located_statements > 1 && !one_node_body {
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
/// whose recorded definitions all bind distinct names. The second condition
/// keeps the result sound for the definitions RD extracts: a node cannot order
/// two definitions of one name that it contains, so RD over it could not see
/// one kill the other. The writes and deferred reads the producer records no
/// def for are handled by the census (kill-only defs, D3, the refusal classes
/// and capture regions), not by refusing the owner (#660 S1d-2 deleted the
/// enumerated guard; the old guard survives only as the trace's control, in
/// `cpg/reaching/guard_control.rs`). An owner with zero statements (an empty
/// body) has nothing to flow and keeps the refusal. Both the solver and the
/// cap preflight call this one function, so they cannot disagree.
pub(crate) fn single_node_cfg_is_complete(
    parsed: &ParsedFile,
    statement_lines: usize,
    def_bases: &[&str],
) -> bool {
    if !is_js_family(parsed.language) || statement_lines != 1 {
        return false;
    }
    let mut seen = BTreeSet::new();
    def_bases.iter().all(|base| seen.insert(*base))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complete_only_for_one_node_with_distinct_names_in_the_js_family() {
        for language in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
            let parsed =
                ParsedFile::parse("a.js", "function f(a, b) { g(a, b); }", language).unwrap();
            assert!(single_node_cfg_is_complete(&parsed, 1, &["a", "b"]));
            assert!(single_node_cfg_is_complete(&parsed, 1, &[]));
            assert!(!single_node_cfg_is_complete(&parsed, 0, &["a"]));
            assert!(!single_node_cfg_is_complete(&parsed, 2, &["a"]));
            assert!(!single_node_cfg_is_complete(&parsed, 1, &["a", "a"]));
        }
        let py = ParsedFile::parse("a.py", "def f(a):\n    g(a)\n", Language::Python).unwrap();
        assert!(!single_node_cfg_is_complete(&py, 1, &["a"]));
    }

    #[test]
    fn hazards_no_longer_refuse_admission() {
        // #660 S1d-2: update, pattern default, `arguments` and eval are the census's to label.
        for source in [
            "function f(a) { return (a++, g(a)); }",
            "function f(a) { return ([a = 1] = g(), a); }",
            "function f(a) { return (arguments[0] = 1, g(a)); }",
            "function f(a) { return (eval('a = 1'), g(a)); }",
            "function f(a) { return g(function (z = a) { return z; }); }",
        ] {
            let parsed = ParsedFile::parse("a.js", source, Language::JavaScript).unwrap();
            assert!(single_node_cfg_is_complete(&parsed, 1, &["a"]), "{source}");
            assert!(
                !single_node_cfg_is_complete(&parsed, 1, &["a", "a"]),
                "{source}"
            );
        }
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

    #[test]
    fn a_callable_body_takes_the_node_whatever_the_located_count() {
        // #759: the located statements of `k => () => { .. }` are the nested callable's.
        for source in [
            "const f = (k) =>\n  () => {\n    a();\n    b();\n  };\n",
            "const f = (k) =>\n  async () => {\n    a();\n    b();\n  };\n",
            "const f = (k) =>\n  function () {\n    a();\n    b();\n  };\n",
            "const f = (k) =>\n  function* () {\n    a();\n    b();\n  };\n",
        ] {
            let span = arrow_span(source, 2).expect(source);
            assert_eq!((span.line, span.kind.as_str()), (2, EXPRESSION_BODY_KIND));
            // The control keeps main's universe, only inside the switch, and restores it.
            assert!(with_main_arrow_universe(|| arrow_span(source, 2)).is_none());
            assert!(arrow_span(source, 2).is_some());
        }
        // Two located statements that are not a callable's keep the existing universe.
        assert!(arrow_span("const f = (k) =>\n  a(k) ||\n  b(k);\n", 2).is_none());
        assert!(arrow_span(
            "const f = (k) =>\n  wrap(() => {\n    a();\n    b();\n  });\n",
            2
        )
        .is_none());
        assert!(arrow_span(
            "const f = (k) => {\n  return () => {\n    a();\n    b();\n  };\n};\n",
            2
        )
        .is_none());
        assert!(arrow_span(
            "function f(k) {\n  return () => {\n    a();\n    b();\n  };\n}\n",
            2
        )
        .is_none());
    }

    #[test]
    fn callable_body_is_judged_on_the_arrow_alone() {
        let check = |source: &str, language: Language| {
            let parsed = ParsedFile::parse("a.ts", source, language).unwrap();
            let func = parsed.all_functions().into_iter().next().unwrap();
            arrow_body_is_callable(&parsed, &func)
        };
        assert!(check(
            "const f = (k: number) => (): void => { a(); };",
            Language::TypeScript
        ));
        assert!(!check(
            "const f = (k: number) => (() => { a(); }) as F;",
            Language::TypeScript
        ));
        assert!(!check(
            "const f = (k: number) => ({ m() { a(); } });",
            Language::TypeScript
        ));
        assert!(!check(
            "function f(k) { return () => 1; }",
            Language::TypeScript
        ));
        let py = ParsedFile::parse("a.py", "def f(a):\n    return a\n", Language::Python).unwrap();
        let func = py.all_functions().into_iter().next().unwrap();
        assert!(!arrow_body_is_callable(&py, &func));
    }

    #[test]
    fn a_ternary_body_takes_the_node_and_other_operators_do_not() {
        // #759: the arms of a ternary body are exclusive; `||`, `&&` and `??` run left to right.
        let ternary = "const f = (k) =>\n  c(k)\n    ? g(k = 1)\n    : sink(k);\n";
        let span = arrow_span(ternary, 3).expect(ternary);
        assert_eq!((span.line, span.kind.as_str()), (2, EXPRESSION_BODY_KIND));
        assert!(with_main_arrow_universe(|| arrow_span(ternary, 3)).is_none());
        for source in [
            "const f = (k) =>\n  c(k) ||\n  sink(k);\n",
            "const f = (k) =>\n  c(k) &&\n  sink(k);\n",
            "const f = (k) =>\n  c(k) ??\n  sink(k);\n",
        ] {
            assert!(arrow_span(source, 2).is_none(), "{source}");
        }
        let check = |source: &str| {
            let parsed = ParsedFile::parse("a.js", source, Language::JavaScript).unwrap();
            let func = parsed.all_functions().into_iter().next().unwrap();
            (
                arrow_body_is_ternary(&parsed, &func),
                arrow_body_is_callable(&parsed, &func),
                arrow_body_takes_one_node(&parsed, &func),
            )
        };
        assert_eq!(check(ternary), (true, false, true));
        assert_eq!(check("const f = (k) => () => k;"), (false, true, true));
        assert_eq!(
            check("const f = (k) => (k ? a : b);"),
            (false, false, false)
        );
        assert_eq!(
            check("const f = (k) => { return k ? a : b; };"),
            (false, false, false)
        );
    }
}
