//! Rust macro-argument call extraction (transparency-allowlisted, token-pattern).
//!
//! Ordinary tree-sitter-rust parses macro arguments as flat token soup:
//! `token_tree` children are only identifiers/literals/self/super/crate/
//! primitive types/metavariables/token_repetitions/nested token_trees, plus
//! anonymous punctuation — never `call_expression`, `scoped_identifier`, or
//! `field_expression`. A real call like `check(x)` written inside
//! `assert!(check(x))` therefore mints no `CallSite` under the ordinary
//! call-extraction path (see `is_call_node`/`call_function_name` in
//! `crate::languages`), and every assert-wrapped test call site is invisible
//! to `nav callers`.
//!
//! This module recovers those calls for a fixed allowlist of **transparent**
//! macros — macros whose arguments are ordinary evaluated Rust expressions,
//! not a DSL/pattern grammar (`matches!`'s 2nd arg is a PATTERN, `stringify!`'s
//! argument is raw tokens, `quote!`, `lazy_static!`, `tokio::select!`, ...).
//! Everything else is skipped (and counted — `MacroArgFacts::skipped_macros`)
//! — that is the precision gate: minting a wrong call from a macro whose
//! argument grammar we don't understand would be worse than missing it.
//!
//! Scope guard: this is NOT the macro-name-resolution increment (still
//! deferred — see
//! `docs/superpowers/specs/2026-06-17-prism-macro-resolution-deferred.md`).
//! We extract ordinary value calls appearing in macro ARGUMENTS only;
//! derive/proc-macro-generated bodies stay out (adjudicated `oracle_artifact`).

use crate::ast::{CallSiteMeta, ParsedFile};
use crate::call_graph::{CallKind, CallSiteOrigin};
use tree_sitter::Node;

/// Macros whose arguments are ordinary evaluated Rust expressions — matched
/// on the LAST path segment of the macro's name, so `std::assert!` ≡
/// `assert!`.
///
/// This is the SINGLE source of truth for macro transparency: both this
/// extractor and the scope-graph populator's wildcard-poison suppression
/// (`crate::name_resolution::rust_populator::walk::locals::walk_expr`) import
/// it, so the two can never drift — a macro that is transparent for call
/// extraction MUST also be transparent for name-introduction poisoning (the
/// P8 scope-graph BLOCKER: without this, `assert!(check(x))` would mint a
/// site whose name `check` is poisoned by the very macro that contains it).
pub const TRANSPARENT_ARG_MACROS: &[&str] = &[
    "assert",
    "assert_eq",
    "assert_ne",
    "debug_assert",
    "debug_assert_eq",
    "debug_assert_ne",
    "vec",
    "format",
    "format_args",
    "print",
    "println",
    "eprint",
    "eprintln",
    "write",
    "writeln",
    "panic",
    "todo",
    "unimplemented",
    "unreachable",
    "dbg",
];

/// Rust keywords (and contextual tokens) that must never be minted as a call
/// name, even when they parse as a plain `identifier` inside macro token soup
/// (`move`, `dyn`, `ref`, `in`, `else`, `Self` all lex as generic
/// `identifier` there — `mut`/`self`/`super`/`crate` do not, they get their
/// own tree-sitter node kinds, so they never reach the identifier branch in
/// the first place; this list is kept as the full defensive superset from
/// the spec regardless of which half is structurally redundant).
const KEYWORD_GUARD: &[&str] = &[
    "if", "match", "while", "for", "loop", "return", "move", "fn", "unsafe", "else", "let", "in",
    "as", "ref", "mut", "impl", "dyn", "break", "continue", "where", "struct", "enum", "union",
    "self", "super", "crate", "Self",
];

/// Returns the last `::`-separated path segment (`std::assert` -> `assert`).
pub fn last_path_segment(name: &str) -> &str {
    name.rsplit("::").next().unwrap_or(name)
}

/// Is `macro_name`'s last path segment in the transparent allowlist?
pub fn is_transparent_arg_macro(macro_name: &str) -> bool {
    TRANSPARENT_ARG_MACROS.contains(&last_path_segment(macro_name))
}

/// Per-file macro-argument extraction telemetry (P8 call-stats).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MacroArgFacts {
    /// Value calls minted from allowlisted macro arguments.
    pub calls_recorded: usize,
    /// Non-allowlisted macro invocations encountered that contained at least
    /// one call-shaped token pattern (counted once per macro invocation, not
    /// once per shape) — a cheap signal for future allowlist growth.
    pub skipped_macros: usize,
    /// Uppercase-constructor-shaped callees skipped (adjudicated: tuple
    /// struct / enum variant constructors are `callable: true` value
    /// bindings in the scope graph — walk/types.rs:66,:134 — and with an
    /// incomplete graph the legacy name-fallback could mint a wrong Exact
    /// edge to an unrelated `fn Foo`).
    pub ctor_skips: usize,
}

/// One token in the flattened direct-children sequence of a `token_tree`.
#[derive(Debug, Clone, Copy)]
enum Tok<'a> {
    /// A named `identifier` child — the only node kind that can start a call
    /// (free, qualified, or method) or a nested macro invocation.
    Ident(Node<'a>),
    /// A named `token_tree` child (nested parens/brackets/braces).
    TokenTree(Node<'a>),
    /// One punctuation atom (post-split) or any other named/opaque child
    /// (`self`/`super`/`crate`/`mutable_specifier`/literal/primitive_type/
    /// metavariable/token_repetition/anonymous keyword) — never a call/macro
    /// start, but relevant for qualifier derivation ("preceding token").
    Other(&'a str),
}

/// Longest-match punctuation-atom table, ordered longest-first so the greedy
/// splitter never mis-splits a real multi-char Rust operator (`..=`, `!=`,
/// `=>`, `::`, ...) into its constituent single-char atoms.
const PUNCT_ATOMS: &[&str] = &[
    "..=", "...", "::", "..", "=>", "->", "==", "!=", "<=", ">=", "&&", "||", "<<", ">>", "+=",
    "-=", "*=", "/=", "%=", "^=", "&=", "|=", "+", "-", "*", "/", "%", "^", "!", "&", "|", "=",
    ">", "<", "@", "_", ".", ",", ";", ":", "#", "?", "(", ")", "{", "}", "[", "]", "'",
];

/// Split one anonymous token's raw text into logical Rust punctuation atoms.
///
/// Tree-sitter-rust's `token_tree` grammar lexes a *run* of the
/// `_non_special_token` punctuation choice greedily (longest-match), so a
/// single anonymous node's text is usually already one atom for tree-sitter-
/// rust 0.24.2 — but nothing downstream may assume that (spec-review MAJOR).
/// Without this normalization, a merged run could be mis-read as e.g. a
/// standalone `!` (nested-macro marker) when the real text is `!=`, or a
/// standalone `.` (method-call marker) when the real text is a range
/// (`..`/`..=`).
pub(crate) fn split_punct_atoms(text: &str) -> Vec<&'static str> {
    let mut out = Vec::new();
    let mut rest = text;
    'outer: while !rest.is_empty() {
        for atom in PUNCT_ATOMS {
            if let Some(tail) = rest.strip_prefix(atom) {
                out.push(*atom);
                rest = tail;
                continue 'outer;
            }
        }
        // Unrecognized byte (shouldn't happen for real token_tree
        // punctuation) — skip one char so the splitter always terminates.
        let mut chars = rest.char_indices();
        chars.next();
        let next = chars.next().map(|(i, _)| i).unwrap_or(rest.len());
        rest = &rest[next..];
    }
    out
}

fn starts_uppercase(text: &str) -> bool {
    text.chars().next().is_some_and(|c| c.is_uppercase())
}

/// Flatten a `token_tree`'s DIRECT children (including anonymous ones, via
/// `node.child(i)`/`.children()` rather than `named_children`) into the `Tok`
/// sequence used for pattern matching.
fn flatten<'a>(parsed: &'a ParsedFile, tt: Node<'a>) -> Vec<Tok<'a>> {
    let mut seq = Vec::new();
    let mut cursor = tt.walk();
    for child in tt.children(&mut cursor) {
        if child.is_named() {
            match child.kind() {
                "identifier" => seq.push(Tok::Ident(child)),
                "token_tree" => seq.push(Tok::TokenTree(child)),
                _ => seq.push(Tok::Other(parsed.node_text(&child))),
            }
        } else {
            for atom in split_punct_atoms(parsed.node_text(&child)) {
                seq.push(Tok::Other(atom));
            }
        }
    }
    seq
}

/// Extract macro-argument calls from a Rust `macro_invocation` node.
///
/// Returns minted call sites (empty if the macro's last path segment isn't
/// in [`TRANSPARENT_ARG_MACROS`]) plus per-invocation telemetry. `macro_node`
/// must be a `macro_invocation` node (the Rust `Calls` tree-sitter query
/// already matches `[(call_expression) (macro_invocation)] @call`, so the
/// caller has one in hand for every macro call in scope).
pub fn extract_calls<'a>(
    parsed: &'a ParsedFile,
    macro_node: Node<'a>,
) -> (Vec<CallSiteMeta<'a>>, MacroArgFacts) {
    let mut out = Vec::new();
    let mut facts = MacroArgFacts::default();
    let Some(name_node) = macro_node.child_by_field_name("macro") else {
        return (out, facts);
    };
    let macro_name = parsed.node_text(&name_node);
    let Some(args) = find_token_tree_child(macro_node) else {
        return (out, facts);
    };
    if is_transparent_arg_macro(macro_name) {
        scan_token_tree(parsed, args, &mut out, &mut facts);
    } else if contains_call_shape(parsed, args) {
        facts.skipped_macros += 1;
    }
    (out, facts)
}

fn find_token_tree_child<'a>(node: Node<'a>) -> Option<Node<'a>> {
    let mut cursor = node.walk();
    let found = node
        .children(&mut cursor)
        .find(|c| c.kind() == "token_tree");
    found
}

/// Scan one `token_tree`'s direct children for call-shaped token patterns,
/// minting `CallSiteMeta` entries into `out` and updating `facts`.
///
/// Recurses into every nested `token_tree` (call args, method args, bare
/// grouping parens, struct-literal braces, array brackets, ...) so a call
/// nested arbitrarily deep in the arguments is still found — EXCEPT the
/// argument tree of a nested macro invocation, which is walked only if that
/// nested macro is itself allowlisted. Transparency is not viral:
/// `stringify!` INSIDE `assert!` stays a token-soup boundary even though
/// `assert!` itself is transparent.
fn scan_token_tree<'a>(
    parsed: &'a ParsedFile,
    tt: Node<'a>,
    out: &mut Vec<CallSiteMeta<'a>>,
    facts: &mut MacroArgFacts,
) {
    let seq = flatten(parsed, tt);
    let mut i = 0;
    while i < seq.len() {
        match seq[i] {
            Tok::Ident(id) => {
                // Try to extend a maximal `Ident (:: Ident)*` chain (mirrors
                // how ordinary extraction represents `mod::f`/`T::m` in
                // `callee_name`: the full "::"-joined text, qualifier=None).
                let mut chain = vec![id];
                let mut j = i + 1;
                loop {
                    let Some(Tok::Other("::")) = seq.get(j) else {
                        break;
                    };
                    let Some(Tok::Ident(n)) = seq.get(j + 1) else {
                        break;
                    };
                    chain.push(*n);
                    j += 2;
                }
                if let Some(Tok::TokenTree(args)) = seq.get(j) {
                    if parsed.node_text(args).starts_with('(') {
                        handle_call_candidate(parsed, &chain, *args, out, facts);
                        scan_token_tree(parsed, *args, out, facts);
                        i = j + 1;
                        continue;
                    }
                }
                // Not a call chain. Single-identifier nested-macro check:
                // Ident + standalone "!" atom + token_tree.
                if chain.len() == 1 {
                    if let (Some(Tok::Other("!")), Some(Tok::TokenTree(margs))) =
                        (seq.get(i + 1), seq.get(i + 2))
                    {
                        let macro_name = parsed.node_text(&id);
                        if is_transparent_arg_macro(macro_name) {
                            scan_token_tree(parsed, *margs, out, facts);
                        } else if contains_call_shape(parsed, *margs) {
                            facts.skipped_macros += 1;
                        }
                        i += 3;
                        continue;
                    }
                }
                i += 1;
            }
            Tok::Other(".") => {
                if let (Some(Tok::Ident(m)), Some(Tok::TokenTree(args))) =
                    (seq.get(i + 1), seq.get(i + 2))
                {
                    if parsed.node_text(args).starts_with('(') {
                        let qualifier = match i.checked_sub(1).and_then(|p| seq.get(p)) {
                            Some(Tok::Ident(q)) => Some(parsed.node_text(q).to_string()),
                            _ => None,
                        };
                        handle_method_call_candidate(parsed, qualifier, *m, *args, out, facts);
                        scan_token_tree(parsed, *args, out, facts);
                        i += 3;
                        continue;
                    }
                }
                i += 1;
            }
            Tok::TokenTree(tt2) => {
                // A token_tree not otherwise consumed above (struct-literal
                // braces, bare grouping parens, array brackets, the args of
                // an uppercase-ctor-guarded/keyword-guarded callee, ...):
                // still recurse — it may itself contain further calls.
                scan_token_tree(parsed, tt2, out, facts);
                i += 1;
            }
            Tok::Other(_) => {
                i += 1;
            }
        }
    }
}

fn handle_call_candidate<'a>(
    parsed: &'a ParsedFile,
    chain: &[Node<'a>],
    args: Node<'a>,
    out: &mut Vec<CallSiteMeta<'a>>,
    facts: &mut MacroArgFacts,
) {
    let final_ident = *chain.last().expect("chain is never empty");
    let final_text = parsed.node_text(&final_ident);
    if KEYWORD_GUARD.contains(&final_text) {
        return;
    }
    if starts_uppercase(final_text) {
        facts.ctor_skips += 1;
        return;
    }
    let callee_name = chain
        .iter()
        .map(|n| parsed.node_text(n))
        .collect::<Vec<_>>()
        .join("::");
    let start = chain[0];
    out.push(CallSiteMeta {
        callee_name,
        line: start.start_position().row + 1,
        qualifier: None,
        start_byte: start.start_byte(),
        end_byte: args.end_byte(),
        receiver_node: None,
        arg_count: None,
        arg_spread: false,
        kind_override: Some(CallKind::Call),
        origin_override: Some(CallSiteOrigin::MacroArg),
    });
    facts.calls_recorded += 1;
}

fn handle_method_call_candidate<'a>(
    parsed: &'a ParsedFile,
    qualifier: Option<String>,
    method: Node<'a>,
    args: Node<'a>,
    out: &mut Vec<CallSiteMeta<'a>>,
    facts: &mut MacroArgFacts,
) {
    let method_text = parsed.node_text(&method);
    if KEYWORD_GUARD.contains(&method_text) {
        return;
    }
    if starts_uppercase(method_text) {
        facts.ctor_skips += 1;
        return;
    }
    out.push(CallSiteMeta {
        callee_name: method_text.to_string(),
        line: method.start_position().row + 1,
        qualifier,
        start_byte: method.start_byte(),
        end_byte: args.end_byte(),
        // No AST receiver expression to surface from token soup — unknown-
        // receiver sites flow through the existing Rust floor unchanged.
        receiver_node: None,
        arg_count: None,
        arg_spread: false,
        kind_override: Some(CallKind::Call),
        origin_override: Some(CallSiteOrigin::MacroArg),
    });
    facts.calls_recorded += 1;
}

/// Lightweight shape-only scan (no guards, no minting): does this token_tree
/// contain at least one call-shaped token pattern (free/qualified call or
/// method call, uppercase/keyword included)? Used only for the
/// `macro_arg_skipped_macros` telemetry signal on a macro we've already
/// decided NOT to walk for real.
fn contains_call_shape(parsed: &ParsedFile, tt: Node) -> bool {
    let seq = flatten(parsed, tt);
    for (idx, tok) in seq.iter().enumerate() {
        match tok {
            Tok::Ident(_) => {
                if let Some(Tok::TokenTree(args)) = seq.get(idx + 1) {
                    if parsed.node_text(args).starts_with('(') {
                        return true;
                    }
                }
            }
            Tok::Other(".") => {
                if let (Some(Tok::Ident(_)), Some(Tok::TokenTree(args))) =
                    (seq.get(idx + 1), seq.get(idx + 2))
                {
                    if parsed.node_text(args).starts_with('(') {
                        return true;
                    }
                }
            }
            _ => {}
        }
    }
    seq.iter().any(|tok| match tok {
        Tok::TokenTree(inner) => contains_call_shape(parsed, *inner),
        _ => false,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::languages::Language;

    /// Parse `src` (must contain exactly one top-level `macro_invocation` in
    /// a function body) and return the parsed file plus that node.
    fn parse_one_macro(src: &str) -> ParsedFile {
        ParsedFile::parse("scratch.rs", src, Language::Rust).expect("parse")
    }

    fn find_macro_invocation<'a>(node: Node<'a>) -> Option<Node<'a>> {
        if node.kind() == "macro_invocation" {
            return Some(node);
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            if let Some(found) = find_macro_invocation(child) {
                return Some(found);
            }
        }
        None
    }

    fn extract(src: &str) -> (ParsedFile, Vec<(String, Option<String>)>, MacroArgFacts) {
        let pf = parse_one_macro(src);
        // Leak-free indirection: re-run extract_calls with a fresh borrow
        // scope so callers can inspect owned (name, qualifier) pairs without
        // fighting the `CallSiteMeta<'a>` lifetime.
        let root = pf.tree.root_node();
        let macro_node = find_macro_invocation(root).expect("must contain a macro_invocation");
        let (sites, facts) = extract_calls(&pf, macro_node);
        let simplified: Vec<(String, Option<String>)> = sites
            .iter()
            .map(|s| (s.callee_name.clone(), s.qualifier.clone()))
            .collect();
        (pf, simplified, facts)
    }

    // ---- allowlist / last-segment ----

    #[test]
    fn last_segment_strips_leading_path() {
        assert_eq!(last_path_segment("std::assert"), "assert");
        assert_eq!(last_path_segment("assert"), "assert");
    }

    #[test]
    fn allowlist_matches_on_last_segment() {
        assert!(is_transparent_arg_macro("assert"));
        assert!(is_transparent_arg_macro("std::assert"));
        assert!(!is_transparent_arg_macro("matches"));
        assert!(!is_transparent_arg_macro("stringify"));
        assert!(!is_transparent_arg_macro("quote"));
        assert!(!is_transparent_arg_macro("lazy_static"));
        assert!(!is_transparent_arg_macro("select")); // tokio::select!
    }

    // ---- punctuation-atom splitter (pure function; spec-review MAJOR) ----

    #[test]
    fn splitter_keeps_ne_as_one_atom() {
        assert_eq!(split_punct_atoms("!="), vec!["!="]);
    }

    #[test]
    fn splitter_keeps_range_as_one_atom() {
        assert_eq!(split_punct_atoms(".."), vec![".."]);
    }

    #[test]
    fn splitter_keeps_range_inclusive_as_one_atom() {
        assert_eq!(split_punct_atoms("..="), vec!["..="]);
    }

    #[test]
    fn splitter_splits_question_dot_into_two_atoms() {
        // `?.` is not itself a Rust token; a merged run must split into `?`, `.`.
        assert_eq!(split_punct_atoms("?."), vec!["?", "."]);
    }

    #[test]
    fn splitter_keeps_fat_arrow_as_one_atom() {
        assert_eq!(split_punct_atoms("=>"), vec!["=>"]);
    }

    #[test]
    fn splitter_splits_turbofish_colons_from_angle() {
        // `::<` is not itself a Rust token; must split into `::`, `<`.
        assert_eq!(split_punct_atoms("::<"), vec!["::", "<"]);
    }

    #[test]
    fn splitter_handles_bang_alone() {
        assert_eq!(split_punct_atoms("!"), vec!["!"]);
    }

    #[test]
    fn splitter_handles_dot_alone() {
        assert_eq!(split_punct_atoms("."), vec!["."]);
    }

    // ---- extractor: free / qualified calls ----

    #[test]
    fn free_call_is_minted() {
        let (_pf, sites, facts) = extract("fn f() { assert!(check(x)); }");
        assert_eq!(sites, vec![("check".to_string(), None)]);
        assert_eq!(facts.calls_recorded, 1);
    }

    #[test]
    fn qualified_path_call_is_minted_with_joined_name() {
        let (_pf, sites, facts) = extract("fn f() { assert_eq!(util::compute(1), 2); }");
        assert_eq!(sites, vec![("util::compute".to_string(), None)]);
        assert_eq!(facts.calls_recorded, 1);
    }

    #[test]
    fn lowercase_final_segment_qualified_ctor_style_call_still_mints() {
        // T::new(1): only the FINAL segment is tested for the ctor guard.
        let (_pf, sites, facts) = extract("fn f() { assert!(T::new(1)); }");
        assert_eq!(sites, vec![("T::new".to_string(), None)]);
        assert_eq!(facts.calls_recorded, 1);
        assert_eq!(facts.ctor_skips, 0);
    }

    #[test]
    fn method_call_after_paren_group_mints_both_unknown_receiver() {
        // f(x).is_none() — method on a call result: unknown receiver (None).
        let (_pf, sites, facts) = extract("fn f() { assert!(f(x).is_none()); }");
        assert_eq!(
            sites,
            vec![("f".to_string(), None), ("is_none".to_string(), None),]
        );
        assert_eq!(facts.calls_recorded, 2);
    }

    #[test]
    fn method_call_with_identifier_receiver_derives_qualifier() {
        let (_pf, sites, _facts) = extract("fn f() { assert!(v.contains(x)); }");
        assert_eq!(sites, vec![("contains".to_string(), Some("v".to_string()))]);
    }

    #[test]
    fn vec_macro_mints_two_free_calls() {
        let (_pf, sites, facts) = extract("fn f() { vec![compute(1), compute(2)]; }");
        assert_eq!(
            sites,
            vec![("compute".to_string(), None), ("compute".to_string(), None),]
        );
        assert_eq!(facts.calls_recorded, 2);
    }

    #[test]
    fn nested_transparent_macro_recurses_and_mints_inner_call() {
        let (_pf, sites, _facts) =
            extract(r#"fn f() { assert!(v.contains(&format!("{}", f(1)))); }"#);
        // v.contains(...) [method, qualifier "v"] and f(1) [free, nested 2
        // levels inside the transparent format! macro]. `format` itself is
        // never minted — it's the nested macro's own name, not a value call.
        assert!(sites.contains(&("contains".to_string(), Some("v".to_string()))));
        assert!(sites.contains(&("f".to_string(), None)));
        assert!(!sites.iter().any(|(name, _)| name == "format"));
    }

    #[test]
    fn non_allowlisted_nested_macro_is_not_walked_but_outer_method_still_mints() {
        // assert!(stringify!(g()).len() > 0): g must NEVER mint (nested
        // stringify! is not allowlisted); len() must still mint (method call
        // on the stringify! result, outside its token-soup boundary).
        let (_pf, sites, facts) = extract("fn f() { assert!(stringify!(g()).len() > 0); }");
        assert!(!sites.iter().any(|(name, _)| name == "g"));
        assert!(sites.contains(&("len".to_string(), None)));
        assert_eq!(facts.skipped_macros, 1);
    }

    #[test]
    fn matches_macro_is_fully_skipped() {
        let (_pf, sites, facts) = extract("fn f() { matches!(x, Some(y)); }");
        assert!(sites.is_empty());
        // Some(y) is a call-shaped token pattern -> counts toward the
        // allowlist-growth signal even though nothing is minted.
        assert_eq!(facts.skipped_macros, 1);
    }

    #[test]
    fn nontransparent_undefined_macro_is_skipped_and_counted() {
        let (_pf, sites, facts) = extract("fn f() { my_macro!(check(x)); }");
        assert!(sites.is_empty());
        assert_eq!(facts.skipped_macros, 1);
    }

    #[test]
    fn stringify_never_mints_the_call_inside_it() {
        let (_pf, sites, facts) = extract("fn f() { stringify!(check(x)); }");
        assert!(sites.is_empty());
        assert_eq!(facts.skipped_macros, 1);
    }

    #[test]
    fn struct_literal_lookalike_is_not_a_call() {
        let (_pf, sites, _facts) = extract("fn f() { assert!(Ident { a: 1 }.a == 1); }");
        assert!(sites.is_empty());
    }

    #[test]
    fn keyword_guard_skips_move_as_callee() {
        let (_pf, sites, _facts) = extract("fn f() { assert!(move(x)); }");
        assert!(sites.is_empty());
    }

    #[test]
    fn keyword_guard_skips_dyn_as_callee() {
        let (_pf, sites, _facts) = extract("fn f() { assert!(dyn(x)); }");
        assert!(sites.is_empty());
    }

    #[test]
    fn uppercase_constructor_is_skipped_and_counted() {
        let (_pf, sites, facts) = extract("fn f() { assert!(Foo(1)); }");
        assert!(sites.is_empty());
        assert_eq!(facts.ctor_skips, 1);
    }

    #[test]
    fn uppercase_constructor_inside_call_args_does_not_block_finding_nested_calls() {
        let (_pf, sites, facts) = extract("fn f() { assert!(Foo(check(1))); }");
        assert!(!sites.iter().any(|(name, _)| name == "Foo"));
        assert!(sites.contains(&("check".to_string(), None)));
        assert_eq!(facts.ctor_skips, 1);
        assert_eq!(facts.calls_recorded, 1);
    }

    #[test]
    fn turbofish_call_is_deliberately_not_minted() {
        let (_pf, sites, _facts) = extract("fn f() { assert!(f::<T>(x)); }");
        assert!(sites.is_empty());
    }

    #[test]
    fn punctuation_run_ne_does_not_trigger_nested_macro() {
        // `a != b`: no identifier is followed by "!" + token_tree, so no
        // nested-macro (mis)detection and no mint (a, b are bare values).
        let (_pf, sites, _facts) = extract("fn f() { assert!(a != b); }");
        assert!(sites.is_empty());
    }

    #[test]
    fn punctuation_run_range_does_not_trigger_method_call() {
        let (_pf, sites, _facts) = extract("fn f() { assert!(a..b); }");
        assert!(sites.is_empty());
    }

    #[test]
    fn punctuation_run_range_inclusive_does_not_trigger_method_call() {
        let (_pf, sites, _facts) = extract("fn f() { assert!(a..=b); }");
        assert!(sites.is_empty());
    }

    #[test]
    fn minted_sites_carry_call_kind_and_macro_arg_origin() {
        let pf = parse_one_macro("fn f() { assert!(check(x)); }");
        let root = pf.tree.root_node();
        let macro_node = find_macro_invocation(root).unwrap();
        let (sites, _facts) = extract_calls(&pf, macro_node);
        assert_eq!(sites.len(), 1);
        assert_eq!(sites[0].kind_override, Some(CallKind::Call));
        assert_eq!(sites[0].origin_override, Some(CallSiteOrigin::MacroArg));
        assert_eq!(sites[0].arg_count, None);
        assert!(!sites[0].arg_spread);
    }
}
