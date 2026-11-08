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

/// Std macros that expand to a single EXPRESSION and can introduce no item and no binding
/// into the enclosing block (#765 PR D, E2). They are not argument-transparent for call
/// extraction (their arguments are not call arguments), so they are a SECOND list beside
/// [`TRANSPARENT_ARG_MACROS`], consulted only by the scope-graph populator to skip the
/// wildcard poison of an unexpanded name-introducing macro. `thread_local!`,
/// `macro_rules!`, `include!`, `concat_idents!` and every user macro can introduce names
/// and stay off this list; so does every item-position invocation (the populator never
/// consults this list there).
pub const NON_BINDING_EXPR_MACROS: &[&str] = &[
    "env",
    "option_env",
    "concat",
    "include_str",
    "include_bytes",
    "cfg",
    "line",
    "column",
    "file",
    "module_path",
    "stringify",
    "matches",
    "compile_error",
];

/// Does `macro_name` name a std expression macro of [`NON_BINDING_EXPR_MACROS`]? Same
/// qualifier rule as [`is_transparent_arg_macro`] (only an unqualified path or one qualified
/// by exactly `std`/`core`/`alloc` is the real macro) with the E2 shadow set
/// ([`collect_expr_macro_shadow_set`]: `macro_rules!` names plus the names a `use` item can
/// bring a macro under). `macro_use_extern` (a `#[macro_use] extern crate` of a non-std crate
/// exists, [`has_macro_use_extern_crate`]) withholds the exemption from an unqualified
/// spelling: such a crate's macros shadow the prelude silently, while a glob import of one is
/// rustc error E0659 (ambiguous with the prelude) and cannot occur in a compiling crate.
pub fn is_non_binding_expr_macro(
    macro_name: &str,
    shadow: &BTreeSet<String>,
    macro_use_extern: bool,
) -> bool {
    let last = last_path_segment(macro_name);
    if !NON_BINDING_EXPR_MACROS.contains(&last) || shadow.contains(last) {
        return false;
    }
    match macro_name.rsplit_once("::") {
        None => !macro_use_extern,
        Some((qualifier, _)) => TRANSPARENT_QUALIFIERS.contains(&qualifier),
    }
}

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
    let mut defs = BTreeSet::new();
    for parsed in files.values() {
        if parsed.language != crate::languages::Language::Rust {
            continue;
        }
        collect_macro_defs(parsed, parsed.tree.root_node(), &mut defs);
    }
    defs
}

/// The shadow set of the E2 list ([`NON_BINDING_EXPR_MACROS`]) only: [`collect_macro_shadow_set`]
/// plus every listed spelling that a `use` item can bring a macro under (#765 PR D, W1).
/// `TRANSPARENT_ARG_MACROS` keeps the `macro_rules!`-names-only set, so argument-call
/// extraction does not move when a repo imports a listed spelling from outside
/// (`use pretty_assertions::assert_eq;`). A `use` item can bring a macro under a spelling
/// only (1) from a path rooted outside the repo, except the std item of the same name,
/// (2) as an in-repo `as <spelling>` alias of an in-repo macro (chains followed), while (3)
/// an unaliased in-repo `use` can only import an in-repo macro of that name, already in the
/// `macro_rules!` census, and an in-repo alias of a fn or type imports no macro.
pub fn collect_expr_macro_shadow_set(files: &BTreeMap<String, ParsedFile>) -> BTreeSet<String> {
    let mut defs = BTreeSet::new();
    let mut mods = BTreeSet::new();
    let mut leaves = Vec::new();
    for parsed in files.values() {
        if parsed.language != crate::languages::Language::Rust {
            continue;
        }
        collect_macro_defs(parsed, parsed.tree.root_node(), &mut defs);
        collect_mod_names(parsed, parsed.tree.root_node(), &mut mods);
        collect_use_leaves(parsed, parsed.tree.root_node(), &mut leaves);
    }
    let crates = in_repo_crate_names(files);
    let in_repo = |u: &UseLeaf| {
        matches!(u.root.as_str(), "crate" | "self" | "super")
            || mods.contains(&u.root)
            || crates.contains(&u.root)
    };
    // Names that can denote an in-repo macro: every `macro_rules!` name, and every in-repo
    // `as` alias of one (`use crate::make as mk; use crate::mk as env;`), to a fixpoint.
    let mut macro_like = defs.clone();
    loop {
        let before = macro_like.len();
        for u in &leaves {
            if in_repo(u) && u.aliased && macro_like.contains(&u.original) {
                macro_like.insert(u.bound.clone());
            }
        }
        if macro_like.len() == before {
            break;
        }
    }
    let mut shadow = defs;
    for u in &leaves {
        if !is_listed_spelling(&u.bound) {
            continue;
        }
        if !in_repo(u) {
            // (1) rooted outside the repo (an unresolvable first segment counts): the macro
            // may be any macro of that crate; only the std item of the same name is trusted.
            if !(STD_ROOTS.contains(&u.root.as_str()) && !u.aliased && u.bound == u.original) {
                shadow.insert(u.bound.clone());
            }
        } else if u.aliased && macro_like.contains(&u.original) {
            // (2) an in-repo alias of an in-repo macro: `use crate::make as env;`.
            shadow.insert(u.bound.clone());
        }
        // (3) in-repo and unaliased: it can only import an in-repo macro of that very name,
        // whose `macro_rules!` is already in the set; an in-repo alias of a fn/type/mod
        // (`use crate::helper as env;`) imports no macro.
    }
    shadow
}

/// Roots whose `use` of one of the listed macro spellings is the real std macro.
const STD_ROOTS: &[&str] = &["std", "core", "alloc"];

/// Is `name` one of the spellings the two allowlists trust?
fn is_listed_spelling(name: &str) -> bool {
    TRANSPARENT_ARG_MACROS.contains(&name) || NON_BINDING_EXPR_MACROS.contains(&name)
}

/// One `use` leaf: `bound` is the leaf or its `as` alias, `original` the last path segment,
/// `root` the first segment of the whole path.
struct UseLeaf {
    bound: String,
    original: String,
    root: String,
    aliased: bool,
}

/// Names of in-repo workspace crates, from `crates/<name>/` directories (hyphens become
/// underscores). The root package's own name is not known from the files, so a
/// `use <root crate>::..` counts as rooted outside the repo (fail-closed).
fn in_repo_crate_names(files: &BTreeMap<String, ParsedFile>) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for path in files.keys() {
        let mut parts = path.split('/');
        while let Some(seg) = parts.next() {
            if seg == "crates" {
                if let Some(name) = parts.next() {
                    out.insert(name.replace('-', "_"));
                }
                break;
            }
        }
    }
    out
}

fn collect_mod_names(parsed: &ParsedFile, node: Node, out: &mut BTreeSet<String>) {
    if node.kind() == "mod_item" {
        if let Some(name) = node.child_by_field_name("name") {
            out.insert(parsed.node_text(&name).to_string());
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_mod_names(parsed, child, out);
    }
}

fn first_segment(path: &str) -> &str {
    path.split("::").next().unwrap_or(path).trim()
}

fn last_segment(path: &str) -> &str {
    path.rsplit("::").next().unwrap_or(path).trim()
}

/// Every `use` leaf anywhere in `node` (block scope, nested lists, `pub use`, `as` aliases).
fn collect_use_leaves(parsed: &ParsedFile, node: Node, out: &mut Vec<UseLeaf>) {
    if node.kind() == "use_declaration" {
        if let Some(arg) = node.child_by_field_name("argument") {
            walk_use_tree(parsed, arg, None, out);
        }
        return;
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_use_leaves(parsed, child, out);
    }
}

/// Walk one use tree. `root` is the first segment of the path above this node (None at the
/// top). `external_glob` is set when a `*` import has a root other than std/core/alloc and
/// `crate`/`self`/`super` (which can only re-export what the census already sees).
fn walk_use_tree(parsed: &ParsedFile, node: Node, root: Option<&str>, out: &mut Vec<UseLeaf>) {
    let text = |n: &Node| parsed.node_text(n).to_string();
    match node.kind() {
        "identifier" => {
            let name = text(&node);
            out.push(UseLeaf {
                bound: name.clone(),
                original: name.clone(),
                root: root.unwrap_or(&name).to_string(),
                aliased: false,
            });
        }
        "scoped_identifier" => {
            let path = node
                .child_by_field_name("path")
                .map(|p| text(&p))
                .unwrap_or_default();
            if let Some(name) = node.child_by_field_name("name") {
                let leaf = text(&name);
                let r = root
                    .map(str::to_string)
                    .unwrap_or_else(|| first_segment(&path).to_string());
                out.push(UseLeaf {
                    bound: leaf.clone(),
                    original: leaf,
                    root: r,
                    aliased: false,
                });
            }
        }
        "use_as_clause" => {
            let path = node
                .child_by_field_name("path")
                .map(|p| text(&p))
                .unwrap_or_default();
            if let Some(alias) = node.child_by_field_name("alias") {
                let r = root
                    .map(str::to_string)
                    .unwrap_or_else(|| first_segment(&path).to_string());
                out.push(UseLeaf {
                    bound: text(&alias),
                    original: last_segment(&path).to_string(),
                    root: r,
                    aliased: true,
                });
            }
        }
        "use_list" => {
            let mut cursor = node.walk();
            for child in node.named_children(&mut cursor) {
                walk_use_tree(parsed, child, root, out);
            }
        }
        "scoped_use_list" => {
            let path = node
                .child_by_field_name("path")
                .map(|p| text(&p))
                .unwrap_or_default();
            let r = root
                .map(str::to_string)
                .unwrap_or_else(|| first_segment(&path).to_string());
            if let Some(list) = node.child_by_field_name("list") {
                walk_use_tree(parsed, list, Some(&r), out);
            }
        }
        _ => {}
    }
}

/// Does a `#[macro_use] extern crate` of a non-std crate exist in `files`? Its macros are in
/// scope crate-wide and shadow the prelude silently (#765 PR D, W1).
pub fn has_macro_use_extern_crate(files: &BTreeMap<String, ParsedFile>) -> bool {
    files
        .values()
        .filter(|p| p.language == crate::languages::Language::Rust)
        .any(|p| macro_use_extern_in(p, p.tree.root_node()))
}

fn macro_use_extern_in(parsed: &ParsedFile, node: Node) -> bool {
    if node.kind() == "extern_crate_declaration" {
        let name = node
            .child_by_field_name("name")
            .map(|n| parsed.node_text(&n).to_string())
            .unwrap_or_default();
        let mut prev = node.prev_sibling();
        while let Some(sib) = prev {
            if sib.kind() != "attribute_item" {
                break;
            }
            if parsed.node_text(&sib).contains("macro_use") && !STD_ROOTS.contains(&name.as_str()) {
                return true;
            }
            prev = sib.prev_sibling();
        }
    }
    let mut cursor = node.walk();
    let mut children = Vec::new();
    for child in node.children(&mut cursor) {
        children.push(child);
    }
    children.into_iter().any(|c| macro_use_extern_in(parsed, c))
}

/// Narrow a shadow set (as returned by [`collect_macro_shadow_set`]) down to
/// the subset that can actually change [`is_transparent_arg_macro`]'s
/// verdict for ANY call site — the intersection with [`TRANSPARENT_ARG_MACROS`].
/// A `macro_rules!` definition for some other name (e.g. `my_helper`) is
/// harmless noise for transparency purposes; only a shadowed name that also
/// appears in the allowlist can flip a call site from minted to suppressed
/// (or vice versa).
///
/// This is the value persisted on `CallGraph::macro_shadow_intersection`
/// (P8 F1 fix, codex re-review BLOCKER): a REPO-WIDE fact, so an incremental
/// rebuild that only re-extracts `changed_files` must detect when this
/// narrowed set drifts from its last-persisted value and fall back to a full
/// rebuild rather than silently leaving an unchanged file's macro-arg call
/// sites stale. See `CodePropertyGraph::build_incremental_with_scope_graph_inputs`.
pub fn transparent_shadow_intersection(shadow: &BTreeSet<String>) -> BTreeSet<String> {
    shadow
        .iter()
        .filter(|name| TRANSPARENT_ARG_MACROS.contains(&name.as_str()))
        .cloned()
        .collect()
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

// clippy's `manual_find` suggestion (`node.children(&mut cursor).find(...)`
// returned directly) does NOT compile here: the temporary iterator's
// lifetime-extension in tail position collides with `cursor`'s scope
// (rustc E0597). The explicit loop is required, not a style choice.
#[allow(clippy::manual_find)]
fn find_token_tree_child(node: Node<'_>) -> Option<Node<'_>> {
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if child.kind() == "token_tree" {
            return Some(child);
        }
    }
    None
}

/// Scan one `token_tree`'s direct children for call-shaped token patterns,
/// minting `CallSiteMeta` entries into `out` and updating `facts`. `shadow`
/// is the repo-wide macro-name shadow set (threaded through every recursive
/// call and every nested-macro transparency check).
///
/// Recurses into every nested `token_tree` (call args, method args, bare
/// grouping parens, struct-literal braces, array brackets, ...) so a call
/// nested arbitrarily deep in the arguments is still found — EXCEPT:
/// - the argument tree of a nested macro invocation, which is walked only if
///   that nested macro is itself allowlisted. Transparency is not viral:
///   `stringify!` INSIDE `assert!` stays a token-soup boundary even though
///   `assert!` itself is transparent;
/// - an attribute token tree (`#[...]` / `#![...]`), which is NEVER scanned
///   (F3 codex MAJOR) — `#[cfg_attr(feature = "x", path(foo()))]`-shaped
///   tokens inside a transparent macro's arguments must not mint
///   `cfg_attr`/`path`/`foo` as if they were real calls.
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
            Tok::Other("#") => {
                // `#` `[...]` (outer attribute) or `#` `!` `[...]` (inner
                // attribute): skip the bracket token_tree entirely, no scan,
                // no mint.
                let bracket_idx = match seq.get(i + 1) {
                    Some(Tok::Other("!")) => i + 2,
                    _ => i + 1,
                };
                if let Some(Tok::TokenTree(attr)) = seq.get(bracket_idx) {
                    if parsed.node_text(attr).starts_with('[') {
                        i = bracket_idx + 1;
                        continue;
                    }
                }
                i += 1;
            }
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
                        let anchor = leading_path_anchor(&seq, i);
                        handle_call_candidate(parsed, &chain, anchor, *args, out, facts);
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
                        // The qualifier is `Some(recv)` ONLY when the
                        // receiver token is a single bare identifier NOT
                        // itself preceded by a `.` atom (F2 codex MAJOR) —
                        // i.e. exactly `recv.method(...)` with nothing
                        // before `recv` except a non-`.` token or the start
                        // of the stream. `a.b.c(x)` must NOT derive `b` as
                        // c's qualifier: `b` is itself a chained receiver, so
                        // a local `b: B` could otherwise falsely resolve
                        // `.c()` via receiver recovery.
                        let qualifier = match i.checked_sub(1).and_then(|p| seq.get(p)) {
                            Some(Tok::Ident(q)) => {
                                let chained = i
                                    .checked_sub(2)
                                    .and_then(|p| seq.get(p))
                                    .is_some_and(|t| matches!(t, Tok::Other(".")));
                                if chained {
                                    None
                                } else {
                                    Some(parsed.node_text(q).to_string())
                                }
                            }
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

/// The `crate` / `self` / `super(::super)*` prefix written directly before the
/// chain that starts at `seq[i]` (`super::super::f(..)` gives `"super::super"`).
///
/// The extractor keeps `callee_name` and the span as the chain from its first
/// identifier (the keywords are not `identifier` nodes); the prefix is stored
/// aside in `CallSiteMeta::path_anchor` and read only by the authoritative Rust
/// route (`crate::resolution::rust::path_anchor`). A keyword must be followed by
/// `::` to count, `crate`/`self` must be the leftmost token of the path (not
/// preceded by `::`), `super` repeats only through `super ::`, and `self` may lead
/// `super` hops (`self::super::g`).
fn leading_path_anchor(seq: &[Tok<'_>], i: usize) -> Option<String> {
    let mut parts: Vec<&str> = Vec::new();
    let mut at = i;
    loop {
        let Some(Tok::Other("::")) = at.checked_sub(1).and_then(|p| seq.get(p)) else {
            break;
        };
        let Some(Tok::Other(kw @ ("crate" | "self" | "super"))) =
            at.checked_sub(2).and_then(|p| seq.get(p))
        else {
            break;
        };
        parts.push(kw);
        at -= 2;
        if *kw != "super" {
            break;
        }
    }
    // A path continued from the left (`a::super::f`, `::crate::f`) is not a
    // plain keyword-anchored one.
    if matches!(
        at.checked_sub(1).and_then(|p| seq.get(p)),
        Some(Tok::Other("::"))
    ) {
        return None;
    }
    if parts.is_empty() {
        return None;
    }
    parts.reverse();
    // `crate` stands alone; `self` may lead `super` hops (`self::super::g`); a
    // `super` hop never follows `crate` (the loop only continues past `super`, so
    // everything after the leftmost keyword is `super`).
    if parts[0] == "crate" && parts.len() > 1 {
        return None;
    }
    Some(parts.join("::"))
}

fn handle_call_candidate<'a>(
    parsed: &'a ParsedFile,
    chain: &[Node<'a>],
    path_anchor: Option<String>,
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
        path_anchor,
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
        path_anchor: None,
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
