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

use std::collections::{BTreeMap, BTreeSet};

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

/// Qualifiers under which a transparent macro name may be path-qualified and
/// still count as the REAL std/core/alloc macro (`std::assert!`). Any other
/// qualifier (`my::assert!`, `crate::assert!`) is a different macro entirely
/// (name resolution — deliberately not attempted here) and is NOT transparent
/// (F1 codex BLOCKER).
const TRANSPARENT_QUALIFIERS: &[&str] = &["std", "core", "alloc"];

/// Returns the last `::`-separated path segment (`std::assert` -> `assert`).
pub fn last_path_segment(name: &str) -> &str {
    name.rsplit("::").next().unwrap_or(name)
}

/// Is `macro_name` transparent for macro-argument call extraction AND for the
/// scope-graph populator's wildcard-poison exemption? THE single shared
/// decision function for both consumers (F1 BLOCKER: two independently
/// maintained copies could drift).
///
/// ALL of the following must hold:
/// 1. the last path segment is in [`TRANSPARENT_ARG_MACROS`];
/// 2. that last segment is NOT in `shadow` — the repo-wide set of names
///    introduced by a `macro_rules!` definition anywhere in the indexed files
///    ([`collect_macro_shadow_set`]). A user `macro_rules! assert`/`vec`
///    shares its name with a real std macro but is not known to be
///    argument-transparent, so its name is withheld from transparency
///    EVERYWHERE in the repo — fail-closed over-suppression, adjudicated
///    acceptable (custom std-named macros are rare);
/// 3. the macro path is either unqualified (`assert!`) or qualified by
///    EXACTLY `std`/`core`/`alloc` (`std::assert!`) — any other qualifier
///    (`my::assert!`, `crate::assert!`) is a different macro and is not
///    transparent, regardless of its last segment or the shadow set.
pub fn is_transparent_arg_macro(macro_name: &str, shadow: &BTreeSet<String>) -> bool {
    let last = last_path_segment(macro_name);
    if !TRANSPARENT_ARG_MACROS.contains(&last) {
        return false;
    }
    if shadow.contains(last) {
        return false;
    }
    match macro_name.rsplit_once("::") {
        None => true,
        Some((qualifier, _)) => TRANSPARENT_QUALIFIERS.contains(&qualifier),
    }
}

/// Collect the repo-wide set of names introduced by `macro_rules! NAME { .. }`
/// across every Rust file in `files` — the "shadow set" consumed by
/// [`is_transparent_arg_macro`] (P8 F1 BLOCKER).
///
/// Scanned over ALL indexed files regardless of module reachability or
/// `only_files` scoping — a `macro_rules!` definition need not be
/// `mod`-visible from a given call site for this repo-wide gate to apply
/// (fail-closed over-suppression is the adjudicated posture, not precision
/// tuning). Callers compute this once per build from the same
/// `files: &BTreeMap<String, ParsedFile>` already in scope at each of this
/// extractor's/the populator's whole-program entry points (mirrors the
/// existing per-build whole-program-fact pattern, e.g.
/// `CallGraph::extract_js_ts_resolution_facts`).
pub fn collect_macro_shadow_set(files: &BTreeMap<String, ParsedFile>) -> BTreeSet<String> {
    let mut shadow = BTreeSet::new();
    for parsed in files.values() {
        if parsed.language != crate::languages::Language::Rust {
            continue;
        }
        collect_macro_defs(parsed, parsed.tree.root_node(), &mut shadow);
    }
    shadow
}

fn collect_macro_defs(parsed: &ParsedFile, node: Node, out: &mut BTreeSet<String>) {
    if node.kind() == "macro_definition" {
        if let Some(name_node) = node.child_by_field_name("name") {
            out.insert(parsed.node_text(&name_node).to_string());
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_macro_defs(parsed, child, out);
    }
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
/// Returns minted call sites (empty if `macro_name` isn't transparent per
/// [`is_transparent_arg_macro`]) plus per-invocation telemetry. `macro_node`
/// must be a `macro_invocation` node (the Rust `Calls` tree-sitter query
/// already matches `[(call_expression) (macro_invocation)] @call`, so the
/// caller has one in hand for every macro call in scope). `shadow` is the
/// repo-wide macro-name shadow set from [`collect_macro_shadow_set`].
pub fn extract_calls<'a>(
    parsed: &'a ParsedFile,
    macro_node: Node<'a>,
    shadow: &BTreeSet<String>,
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
    if is_transparent_arg_macro(macro_name, shadow) {
        scan_token_tree(parsed, args, shadow, &mut out, &mut facts);
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
/// minting `CallSiteMeta` entries into `out` and updating `facts`. `shadow`
/// is the repo-wide macro-name shadow set (threaded through every recursive
/// call and every nested-macro transparency check).
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
    shadow: &BTreeSet<String>,
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
                        scan_token_tree(parsed, *args, shadow, out, facts);
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
                        if is_transparent_arg_macro(macro_name, shadow) {
                            scan_token_tree(parsed, *margs, shadow, out, facts);
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
                        scan_token_tree(parsed, *args, shadow, out, facts);
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
                scan_token_tree(parsed, tt2, shadow, out, facts);
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
mod tests;
