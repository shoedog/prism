//! AST-recovered `dependency_hint` for `external_call` targets (roadmap
//! `03-tooling-plan-roadmap.md` §3 Phase 1; controller ruling 2026-09-04
//! after the astra review of the first cut).
//!
//! Two fields, two very different evidence bars.
//!
//! **`callee` is the deliverable.** The projection in `mod.rs` otherwise
//! carries only `finding.description`, which for `echo`/`missing_error_handling`
//! names the *resolved function identity* (`func_id.name` — often just the last
//! segment of a qualified call, e.g. `"post"` for `requests.post(...)`, since
//! the call graph tracks callee *definitions*, not call-site *syntax*). The
//! harness globs against the source chain, so this module recovers the callee
//! **exactly as written at the site**, in the site's own language: Java's
//! `this.client.get()` is `this.client.get`, never `get`.
//!
//! **`kind` is emitted only with verified dependency identity.** `kind`
//! RESTRICTS fault selection *before* any callee glob runs
//! (`~/code/tools/specs/2026-09-04-runtime-harness-v0-spec.md` §5.2 steps
//! 2–4), so a wrong `kind` is strictly worse than none: it excludes the whole
//! family the site actually belongs to. A `kind` therefore requires all of:
//!
//! 1. the chain's root **binds to an imported library** in this file — an
//!    unimported `requests` is a local name, not the library, and the import's
//!    module path (not the spelling of the local binding) is what is looked up;
//! 2. **no repo-local module** of that name shadows the import (a repository
//!    that ships its own `requests.py` is what `import requests` resolves to);
//! 3. **no local binding** — parameter, assignment or def — of that root in the
//!    enclosing function or at file scope;
//! 4. for a receiver chain (`self.client.get`), **exactly one** constructor
//!    assignment to that receiver inside the receiver's own lexical owner (the
//!    class for `self.`/`this.`, the function for a local); a second assignment
//!    or one in another owner leaves the type unproven;
//! 5. a **single-purpose** library. Multipurpose ones (`redis` — cache *and*
//!    queue; bare `os` — filesystem, process and environment; `urllib` —
//!    `request` is http, `parse` is pure) are not in the table at all, or are
//!    keyed at the submodule that *is* single-purpose (`urllib.request`).
//!
//! Bounded and textual, no `regex` dependency — the same posture as
//! `mapping::ABSENCE_PAIRS`. Every uncertainty resolves to "omit `kind`, keep
//! `callee`", and a site whose own finding cannot be matched to one of several
//! calls on its line keeps the hint it already had and reports the ambiguity.

use super::dependency_identity::{enclosing_function_node, kind_for_binding};
use crate::ast::ParsedFile;
use std::collections::BTreeMap;
use std::path::Path;
use tree_sitter::Node;

/// AST-recovered callee text plus the `kind` resolved from it, if any.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AstHint {
    pub callee: String,
    pub kind: Option<&'static str>,
}

/// What the site's own file and run know about the call site, beyond the AST:
/// the repo the file sits in (for the repo-local-module check that stops a
/// vendored `requests.py` from being read as the `requests` library) and the
/// finding's own recorded callee identity (for correlating one of several
/// same-line calls with the finding that anchored there).
pub struct SiteContext<'a> {
    /// Repo-relative path of the file containing the site.
    pub file: &'a str,
    /// Repo root, for the repo-local-module check.
    pub repo_root: &'a Path,
    /// Every file this run parsed, keyed by repo-relative path.
    pub known_files: &'a BTreeMap<String, ParsedFile>,
    /// The callee identity the producing algorithm resolved for this finding
    /// (`mapping::echo_callee`'s output), used only to pick between several
    /// candidate calls on the site line.
    pub resolved_name: Option<&'a str>,
}

/// The outcome of one site resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolution {
    /// A single call was attributed to the site.
    Hint(AstHint),
    /// `n` candidate calls span the site line and the finding's own evidence
    /// does not single one out. The caller keeps the hint it already had and
    /// records the ambiguity.
    Ambiguous(usize),
}

/// Recover a dependency hint for the call site at `line` (1-indexed) in
/// `parsed`. `None` means no call node's span contains `line` (or no callee
/// expression can be extracted from one) — the caller keeps whatever hint it
/// already had.
pub fn resolve(parsed: &ParsedFile, line: usize, ctx: &SiteContext<'_>) -> Option<Resolution> {
    let scope = parsed
        .function_node_spanning(line)
        .unwrap_or_else(|| parsed.tree.root_node());
    let candidates = call_candidates(parsed, scope, line);
    if candidates.is_empty() {
        return None;
    }
    let Some((node, callee)) = select_candidate(&candidates, ctx.resolved_name) else {
        return Some(Resolution::Ambiguous(candidates.len()));
    };
    let kind = resolve_kind(parsed, ctx, node, callee);
    Some(Resolution::Hint(AstHint {
        callee: callee.clone(),
        kind,
    }))
}

/// Every call node in `scope` whose 1-indexed line range contains `line`,
/// paired with its source-verbatim callee text, in pre-order.
fn call_candidates<'a>(
    parsed: &ParsedFile,
    scope: Node<'a>,
    line: usize,
) -> Vec<(Node<'a>, String)> {
    let mut out = Vec::new();
    collect_call_candidates(parsed, scope, line, &mut out);
    out
}

fn collect_call_candidates<'a>(
    parsed: &ParsedFile,
    node: Node<'a>,
    line: usize,
    out: &mut Vec<(Node<'a>, String)>,
) {
    if parsed.language.is_call_node(node.kind()) {
        let (start, end) = parsed.node_line_range(&node);
        if start <= line && line <= end {
            if let Some(callee) = callee_text(parsed, &node) {
                out.push((node, callee));
            }
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_call_candidates(parsed, child, line, out);
    }
}

/// Pick the candidate this finding is actually about.
///
/// One candidate needs no evidence. Several (sibling statements on one line,
/// a call nested in another call's arguments, a multi-line call whose header
/// spans the site) are correlated with the finding's own resolved callee
/// identity: the algorithm recorded which function it flagged, and a candidate
/// whose last chain segment is that name is the finding's call. Anything that
/// does not single one out is ambiguous — the previous deterministic
/// "rightmost call wins" rule reliably attributed the *neighbouring* call.
fn select_candidate<'a, 'b>(
    candidates: &'b [(Node<'a>, String)],
    resolved_name: Option<&str>,
) -> Option<(Node<'a>, &'b String)> {
    if let [(node, callee)] = candidates {
        return Some((*node, callee));
    }
    let name = resolved_name?;
    let mut matches = candidates
        .iter()
        .filter(|(_, callee)| callee == name || last_segment(callee) == name);
    let first = matches.next()?;
    matches.next().is_none().then_some((first.0, &first.1))
}

fn last_segment(callee: &str) -> &str {
    callee.rsplit('.').next().unwrap_or(callee)
}

/// The callee exactly as written, from the receiver through the method name.
///
/// Most grammars expose the whole callee expression as the call's `function`
/// field, so its span is the answer. Java's `method_invocation` does not: it
/// carries `object` and `name` as siblings, and taking `name` alone (what
/// `Language::call_function_name` deliberately does, so calls bind to
/// definitions) drops the receiver and violates the schema's "callee text as
/// written". For that shape the span runs from the receiver's first byte to
/// the method name's last.
fn callee_text(parsed: &ParsedFile, node: &Node<'_>) -> Option<String> {
    let (start, end) = callee_span(node)?;
    parsed.source.get(start..end).map(str::to_string)
}

fn callee_span(node: &Node<'_>) -> Option<(usize, usize)> {
    if let Some(function) = node.child_by_field_name("function") {
        return Some((function.start_byte(), function.end_byte()));
    }
    let name = node.child_by_field_name("name");
    let object = node.child_by_field_name("object");
    if let (Some(object), Some(name)) = (object, name) {
        if object.start_byte() < name.end_byte() {
            return Some((object.start_byte(), name.end_byte()));
        }
    }
    if let Some(name) = name {
        return Some((name.start_byte(), name.end_byte()));
    }
    if node.kind() == "function_call" || node.kind() == "command" {
        let name = node.child_by_field_name("name").or_else(|| node.child(0))?;
        return Some((name.start_byte(), name.end_byte()));
    }
    object.map(|object| (object.start_byte(), object.end_byte()))
}

/// Split a callee chain into its dotted segments. `client?.get` (JS optional
/// chaining) segments as `["client", "get"]` — the `?` belongs to the source
/// text, not to the binding's name.
fn chain_segments(callee: &str) -> Vec<&str> {
    callee
        .split('.')
        .map(|segment| segment.trim_end_matches('?'))
        .collect()
}

/// Resolve the site's `kind`, or `None` when identity is not verified.
fn resolve_kind(
    parsed: &ParsedFile,
    ctx: &SiteContext<'_>,
    call_node: Node<'_>,
    callee: &str,
) -> Option<&'static str> {
    let segments = chain_segments(callee);
    let root = *segments.first()?;
    if let Some(kind) = kind_for_binding(parsed, ctx, call_node, root, &segments[1..]) {
        return Some(kind);
    }
    if segments.len() < 2 {
        return None;
    }
    // Receiver-construction hop: `self.client.get` takes its kind from the one
    // `self.client = requests.Session()` inside the receiver's own owner.
    let receiver = segments[..segments.len() - 1].join(".");
    let (ctor_node, ctor_callee) = sole_receiver_constructor(parsed, call_node, &receiver)?;
    let ctor_segments = chain_segments(&ctor_callee);
    let ctor_root = *ctor_segments.first()?;
    kind_for_binding(parsed, ctx, ctor_node, ctor_root, &ctor_segments[1..])
}

/// The single constructor assignment for `receiver` inside the receiver's own
/// lexical owner: the enclosing class for `self.`/`this.` receivers, the
/// enclosing function for a local. Two assignments, or none, leave the
/// receiver's type unproven — a whole-file first-match scan is exactly how a
/// hint crossed from one class to another.
fn sole_receiver_constructor<'a>(
    parsed: &'a ParsedFile,
    call_node: Node<'a>,
    receiver: &str,
) -> Option<(Node<'a>, String)> {
    let owner = receiver_owner_scope(parsed, call_node, receiver);
    let mut found = Vec::new();
    collect_receiver_assignments(parsed, owner, receiver, &mut found);
    match found.as_slice() {
        [(node, callee)] => Some((*node, callee.clone())),
        _ => None,
    }
}

fn receiver_owner_scope<'a>(
    parsed: &'a ParsedFile,
    call_node: Node<'a>,
    receiver: &str,
) -> Node<'a> {
    let function = enclosing_function_node(parsed, call_node);
    let root = receiver.split('.').next().unwrap_or(receiver);
    if matches!(root, "self" | "this") {
        if let Some(class) = function
            .as_ref()
            .and_then(|function| parsed.language.method_owner_class_node(function))
        {
            return class;
        }
    }
    function.unwrap_or_else(|| parsed.tree.root_node())
}

/// Every assignment/declaration in `scope` whose target text is exactly
/// `receiver`, paired with its value's callee text when the value is a call.
/// Non-call values still count as assignments (they are what makes a second
/// assignment disqualifying) and carry an empty callee.
fn collect_receiver_assignments<'a>(
    parsed: &ParsedFile,
    node: Node<'a>,
    receiver: &str,
    out: &mut Vec<(Node<'a>, String)>,
) {
    let language = parsed.language;
    if language.is_assignment_node(node.kind()) || language.is_declaration_node(node.kind()) {
        let target = language
            .assignment_target(&node)
            .or_else(|| language.declaration_name(&node));
        let value = language
            .assignment_value(&node)
            .or_else(|| language.declaration_value(&node));
        if let Some(target) = target {
            if parsed.node_text(&target) == receiver {
                let callee = value
                    .filter(|value| language.is_call_node(value.kind()))
                    .and_then(|value| callee_text(parsed, &value))
                    .unwrap_or_default();
                out.push((node, callee));
            }
        }
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        collect_receiver_assignments(parsed, child, receiver, out);
    }
}

#[cfg(test)]
#[path = "dependency_hint_tests.rs"]
mod tests;
