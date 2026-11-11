//! Repo-wide macro verdicts for the DFG's Rust rules U and D (#630 slice 5, fold 3).
//!
//! Rules U and D read unexpanded source and are sound only if every read of a local on a
//! line is witnessed by an identifier token on that line. A `macro_rules!` that is generated
//! by a macro invoked in the owner (or written in the token tree of a pass-through macro)
//! can read a local captured at its definition while its later invocation carries no token
//! of the name. Only a macro that can emit a `macro_rules!` can do that, so the census
//! answers one question per invocation spelling: [`Verdict::Allowed`] (its expansion
//! contains no `macro_rules!` and invokes nothing that does) or [`Verdict::Refused`].
//!
//! * A std spelling of the closed list ([`std_allowed`]) is Allowed, subject to the E2
//!   shadow rules: an external-rooted `use` bringing a macro under the spelling, a
//!   `macro_rules!` of the same name, or a `#[macro_use] extern crate` of a non-std crate
//!   (unqualified spellings only) withhold it.
//! * An in-repo `macro_rules!` is Allowed only when EVERY definition of the spelling is
//!   non-generating, a least fixpoint: a definition generates when its right-hand sides hold
//!   the identifier `macro_rules`, a metavariable in macro position (`$m!(..)`), a definition
//!   that did not parse, or an invocation of a Refused spelling.
//! * Everything else (a qualified external path, an unknown spelling, a proc macro) is
//!   Refused (fail closed).
//!
//! A `macro_rules ! NAME` spelled in any token tree (outside owners too) or in a definition's
//! right-hand side defines NAME as generating (fold 4, fail closed); comments in token soup
//! are ignored by the recognizers.
//!
//! Out of model: an external generator under a spelling by a glob import (`use ext::*;`) when
//! an unrelated clean in-repo definition of that spelling is out of textual scope, and the
//! root-collision form E2 also accepts (an external crate name that is
//! also an unrelated in-repo module spelling is not told apart, since in-repo qualifiers are
//! only `crate`/`self`/`super`/`$crate`; a `mod`-rooted macro path is Refused).
use super::{walk_use_tree, UseLeaf, NON_BINDING_EXPR_MACROS, STD_ROOTS, TRANSPARENT_ARG_MACROS};
use crate::ast::ParsedFile;
use crate::languages::Language;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

/// Whether an invocation spelling can emit a `macro_rules!`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    Allowed,
    Refused,
}

/// Std macros whose expansion contains no `macro_rules!` and invokes nothing that does:
/// the two call-extraction lists plus `pin` and `thread_local`. `include!`, `concat_idents!`
/// and `asm!` are deliberately absent.
pub fn std_allowed(name: &str) -> bool {
    TRANSPARENT_ARG_MACROS.contains(&name)
        || NON_BINDING_EXPR_MACROS.contains(&name)
        || matches!(name, "pin" | "thread_local")
}

fn strip_raw(name: &str) -> &str {
    name.strip_prefix("r#").unwrap_or(name)
}

/// One `ident ! token_tree` spelled in a token tree (or a `macro_invocation`'s own path).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Invocation {
    pub qualifier: Option<String>,
    pub spelling: String,
}

impl Invocation {
    /// Split a macro path (`std::println`, `$crate::m`, `::ext::m`) at its last segment.
    pub fn from_path(path: &str) -> Self {
        let path: String = path.split_whitespace().collect();
        match path.rsplit_once("::") {
            None => Self {
                qualifier: None,
                spelling: strip_raw(&path).to_owned(),
            },
            Some((q, last)) => Self {
                qualifier: Some(q.to_owned()),
                spelling: strip_raw(last).to_owned(),
            },
        }
    }
}

/// What a token tree holds, at any depth.
#[derive(Default)]
pub struct Soup<'a> {
    /// Every identifier token (`r#` stripped).
    pub idents: BTreeSet<String>,
    /// Every `ident ! token_tree` spelled in the soup.
    pub invocations: Vec<Invocation>,
    /// The identifier `macro_rules` occurs.
    pub macro_rules: bool,
    /// A metavariable stands in macro position (`$m!(..)`).
    pub meta_bang: bool,
    /// String literal tokens (format captures live in them).
    pub strings: Vec<Node<'a>>,
    /// Every `macro_rules ! NAME` spelled in the soup (`r#` stripped): the rules are not
    /// parsed, so the census treats NAME as defined and generating (fail closed).
    pub defined: BTreeSet<String>,
}

fn is_comment(node: &Node<'_>) -> bool {
    matches!(node.kind(), "line_comment" | "block_comment")
}

/// An identifier or keyword-spelled anonymous token (a macro may carry a keyword's spelling).
fn name_token(parsed: &ParsedFile, node: &Node<'_>) -> Option<String> {
    match node.kind() {
        "identifier" => Some(strip_raw(parsed.node_text(node)).to_owned()),
        kind if !node.is_named()
            && kind
                .chars()
                .next()
                .is_some_and(|c| c.is_alphabetic() || c == '_')
            && kind.chars().all(|c| c.is_alphanumeric() || c == '_') =>
        {
            Some(kind.to_owned())
        }
        _ => None,
    }
}

/// Scan a `token_tree` (or `token_repetition`) recursively.
pub fn scan_soup<'a>(parsed: &'a ParsedFile, tt: Node<'a>, out: &mut Soup<'a>) {
    let mut cursor = tt.walk();
    // Comments are extras between any two tokens (`name /* c */ !(..)`): drop them before
    // invocation, path and metavariable recognition.
    let kids: Vec<Node<'a>> = tt
        .children(&mut cursor)
        .filter(|n| !is_comment(n))
        .collect();
    let bang_tree = |i: usize| {
        kids.get(i + 1).is_some_and(|n| n.kind() == "!")
            && kids.get(i + 2).is_some_and(|n| n.kind() == "token_tree")
    };
    for (i, kid) in kids.iter().enumerate() {
        match kid.kind() {
            "identifier" => {
                let text = strip_raw(parsed.node_text(kid)).to_owned();
                if text == "macro_rules" {
                    out.macro_rules = true;
                    if kids.get(i + 1).is_some_and(|n| n.kind() == "!") {
                        if let Some(name) = kids.get(i + 2).and_then(|n| name_token(parsed, n)) {
                            out.defined.insert(name);
                        }
                    }
                }
                if bang_tree(i) {
                    out.invocations.push(Invocation {
                        qualifier: path_before(parsed, &kids, i),
                        spelling: text.clone(),
                    });
                }
                out.idents.insert(text);
            }
            // `$m!(..)` and `$k! name { .. }`: a metavariable names the macro (it may become
            // `macro_rules`), whatever follows the `!`.
            "metavariable" if kids.get(i + 1).is_some_and(|n| n.kind() == "!") => {
                out.meta_bang = true
            }
            "token_tree" | "token_repetition" => scan_soup(parsed, *kid, out),
            // A keyword spelling (`gen`, `try`, `async`) is an anonymous token in soup: a
            // macro may still carry its name, so it counts as an invocation (fail closed).
            kind if !kid.is_named()
                && kind
                    .chars()
                    .next()
                    .is_some_and(|c| c.is_alphabetic() || c == '_')
                && kind.chars().all(|c| c.is_alphanumeric() || c == '_')
                && bang_tree(i) =>
            {
                out.invocations.push(Invocation {
                    qualifier: path_before(parsed, &kids, i),
                    spelling: kind.to_owned(),
                });
            }
            "string_literal" | "raw_string_literal" => out.strings.push(*kid),
            _ => {}
        }
    }
}

/// The `a::b::` prefix of the identifier at `i` in `kids`, `None` when there is none. A
/// leading `::` is kept as an empty first segment.
fn path_before(parsed: &ParsedFile, kids: &[Node<'_>], i: usize) -> Option<String> {
    let mut parts: Vec<String> = Vec::new();
    let mut at = i;
    while at >= 2
        && kids[at - 1].kind() == "::"
        && matches!(
            kids[at - 2].kind(),
            "identifier" | "crate" | "self" | "super" | "metavariable"
        )
    {
        parts.push(parsed.node_text(&kids[at - 2]).to_owned());
        at -= 2;
    }
    if at >= 1 && kids[at - 1].kind() == "::" {
        parts.push(String::new());
    }
    if parts.is_empty() {
        return None;
    }
    parts.reverse();
    Some(parts.join("::"))
}

/// Per-definition facts the fixpoint needs.
#[derive(Default)]
struct DefInfo {
    /// `macro_rules` token, metavariable in macro position, or a definition that did not parse.
    generates: bool,
    invocations: Vec<Invocation>,
}

/// The repo-wide macro census. Cheap to clone; compare with [`RustMacroCensus::digest`].
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RustMacroCensus {
    /// Spellings with at least one generating definition.
    generating: BTreeSet<String>,
    /// Every `macro_rules!` name.
    defined: BTreeSet<String>,
    /// Defined or std-listed spellings that a `use` can bring an other macro under.
    ext_bound: BTreeSet<String>,
    /// A `#[macro_use] extern crate` of a non-std crate exists.
    macro_use_extern: bool,
}

#[derive(Default)]
struct Acc {
    defs: BTreeMap<String, Vec<DefInfo>>,
    leaves: Vec<UseLeaf>,
    macro_use_extern: bool,
    /// `macro_rules! NAME` written inside token trees or definition bodies.
    soup_defined: BTreeSet<String>,
}

fn walk(parsed: &ParsedFile, node: Node<'_>, acc: &mut Acc) {
    match node.kind() {
        "macro_definition" => {
            if let Some(name) = node.child_by_field_name("name") {
                let mut info = DefInfo {
                    generates: node.has_error(),
                    ..DefInfo::default()
                };
                let mut cursor = node.walk();
                for rule in node.children(&mut cursor) {
                    let Some(right) = (rule.kind() == "macro_rule")
                        .then(|| rule.child_by_field_name("right"))
                        .flatten()
                    else {
                        continue;
                    };
                    let mut soup = Soup::default();
                    scan_soup(parsed, right, &mut soup);
                    info.generates |= soup.macro_rules || soup.meta_bang;
                    acc.soup_defined.extend(soup.defined);
                    info.invocations.extend(soup.invocations);
                }
                acc.defs
                    .entry(strip_raw(parsed.node_text(&name)).to_owned())
                    .or_default()
                    .push(info);
            }
            return;
        }
        // A token tree anywhere (module level, `cfg_if!` bodies, inside owners): a
        // `macro_rules! NAME` written in it defines NAME, but the rules are not parsed.
        "token_tree" => {
            let mut soup = Soup::default();
            scan_soup(parsed, node, &mut soup);
            acc.soup_defined.extend(soup.defined);
            return;
        }
        "use_declaration" => {
            if let Some(arg) = node.child_by_field_name("argument") {
                walk_use_tree(parsed, arg, None, &mut acc.leaves);
            }
            return;
        }
        "extern_crate_declaration" => {
            let name = node
                .child_by_field_name("name")
                .map(|n| parsed.node_text(&n).to_owned())
                .unwrap_or_default();
            let mut prev = node.prev_sibling();
            while let Some(sib) = prev {
                if is_comment(&sib) {
                    prev = sib.prev_sibling();
                    continue;
                }
                if sib.kind() != "attribute_item" {
                    break;
                }
                if parsed.node_text(&sib).contains("macro_use")
                    && !STD_ROOTS.contains(&name.as_str())
                {
                    acc.macro_use_extern = true;
                }
                prev = sib.prev_sibling();
            }
        }
        _ => {}
    }
    let mut cursor = node.walk();
    for child in node.named_children(&mut cursor) {
        walk(parsed, child, acc);
    }
}

impl RustMacroCensus {
    /// The census of every Rust file in `files`.
    pub fn from_files(files: &BTreeMap<String, ParsedFile>) -> Self {
        Self::from_parsed(files.values())
    }

    /// The census of a single file: the same-file verdicts, used by test seams that have
    /// no repository.
    pub fn of_file(parsed: &ParsedFile) -> Self {
        Self::from_parsed(std::iter::once(parsed))
    }

    pub fn from_parsed<'a>(files: impl Iterator<Item = &'a ParsedFile>) -> Self {
        let mut acc = Acc::default();
        for parsed in files.filter(|p| p.language == Language::Rust) {
            walk(parsed, parsed.tree.root_node(), &mut acc);
        }
        let mut census = RustMacroCensus {
            defined: acc.defs.keys().chain(&acc.soup_defined).cloned().collect(),
            macro_use_extern: acc.macro_use_extern,
            ..Self::default()
        };
        census.ext_bound = ext_bound(&census.defined, &acc.leaves);
        census.generating = acc
            .defs
            .iter()
            .filter(|(_, defs)| defs.iter().any(|d| d.generates))
            .map(|(name, _)| name.clone())
            .chain(acc.soup_defined.iter().cloned())
            .collect();
        // Least fixpoint: a definition invoking a Refused spelling generates.
        loop {
            let before = census.generating.len();
            for (name, defs) in &acc.defs {
                if census.generating.contains(name) {
                    continue;
                }
                let refused = defs.iter().any(|d| {
                    d.invocations.iter().any(|inv| {
                        census.verdict(inv.qualifier.as_deref(), &inv.spelling) == Verdict::Refused
                    })
                });
                if refused {
                    census.generating.insert(name.clone());
                }
            }
            if census.generating.len() == before {
                break;
            }
        }
        census
    }

    /// Can an invocation of `spelling`, written with `qualifier` (a path prefix, without the
    /// final `::`), emit a `macro_rules!`?
    pub fn verdict(&self, qualifier: Option<&str>, spelling: &str) -> Verdict {
        let name = strip_raw(spelling);
        let in_repo = |this: &Self| {
            this.defined.contains(name)
                && !this.generating.contains(name)
                && !this.ext_bound.contains(name)
        };
        let allowed = match qualifier {
            Some("std" | "core" | "alloc" | "::std" | "::core" | "::alloc") => std_allowed(name),
            Some(q) if is_in_repo_qualifier(q) => in_repo(self),
            Some(_) => false,
            None if self.macro_use_extern => false,
            None if self.defined.contains(name) => in_repo(self),
            None => std_allowed(name) && !self.ext_bound.contains(name),
        };
        if allowed {
            Verdict::Allowed
        } else {
            Verdict::Refused
        }
    }

    /// A stable digest of every fact a verdict depends on; empty for an empty census. The
    /// DFG storage persists it, and a partial cache hit that finds a different digest
    /// rebuilds in full.
    pub fn digest(&self) -> String {
        if *self == Self::default() {
            return String::new();
        }
        let mut h = Sha256::new();
        let mut put = |tag: &str, set: &BTreeSet<String>| {
            h.update(tag.as_bytes());
            for name in set {
                h.update(name.as_bytes());
                h.update([0]);
            }
            h.update([1]);
        };
        put("generating", &self.generating);
        put("defined", &self.defined);
        put("ext_bound", &self.ext_bound);
        h.update([u8::from(self.macro_use_extern)]);
        h.finalize().iter().map(|b| format!("{b:02x}")).collect()
    }
}

/// `crate`, `self`, `super` chains and `$crate` name the repository's own macros.
fn is_in_repo_qualifier(q: &str) -> bool {
    q == "$crate"
        || q == "crate"
        || (!q.is_empty() && q.split("::").all(|s| matches!(s, "self" | "super")))
}

/// Defined or std-listed spellings that a `use` item can bring another macro under: a path
/// rooted outside `crate`/`self`/`super` (the unaliased std item of the same name aside), or
/// an in-repo `as` alias of an in-repo macro (chains followed).
fn ext_bound(defined: &BTreeSet<String>, leaves: &[UseLeaf]) -> BTreeSet<String> {
    let relevant = |n: &str| defined.contains(n) || std_allowed(n);
    // `use ext::m as r#assert;` binds `assert`: compare the spellings without `r#`.
    let leaves: Vec<UseLeaf> = leaves
        .iter()
        .map(|u| UseLeaf {
            bound: strip_raw(&u.bound).to_owned(),
            original: strip_raw(&u.original).to_owned(),
            root: strip_raw(&u.root).to_owned(),
            aliased: u.aliased,
        })
        .collect();
    let leaves = &leaves[..];
    let in_repo = |u: &UseLeaf| matches!(u.root.as_str(), "crate" | "self" | "super");
    let mut macro_like = defined.clone();
    loop {
        let before = macro_like.len();
        for u in leaves {
            if in_repo(u) && u.aliased && macro_like.contains(&u.original) {
                macro_like.insert(u.bound.clone());
            }
        }
        if macro_like.len() == before {
            break;
        }
    }
    let mut out = BTreeSet::new();
    for u in leaves.iter().filter(|u| relevant(&u.bound)) {
        let bound = if in_repo(u) {
            u.aliased && macro_like.contains(&u.original)
        } else {
            !(STD_ROOTS.contains(&u.root.as_str())
                && !u.aliased
                && u.bound == u.original
                && std_allowed(&u.bound))
        };
        if bound {
            out.insert(u.bound.clone());
        }
    }
    out
}
