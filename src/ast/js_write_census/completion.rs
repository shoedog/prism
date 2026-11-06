//! The census as reaching-definitions input (#660 S1b-1, DESIGN.md 5.2 A7, 5.3 E15/E16/E18, 8).
//!
//! `completion` runs the S0 walk once for an owner and keeps only what the solver needs: the owner's
//! refusal, the kill-only defs for sites no recorded def covers, and the bases whose site maps to no
//! statement node. The result is small (most owners have none of the three), so the two
//! deduplication seams share it through a per-file memo instead of walking twice.

use super::tables::Tables;
use super::walk::{census_with, Census, Form, Refusal, Site};
use crate::access_path::{AccessPath, ARRAY_FIELD};
use crate::ast::ParsedFile;
use crate::js_write_census::identity::{Comp, Path};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};
use tree_sitter::Node;

/// A write the producer did not record: it gens and kills at `line`'s node, never emits an edge.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct KillOnly {
    pub path: AccessPath,
    /// First line of the mapped statement span (E18).
    pub line: usize,
    pub start_byte: usize,
    /// The census form (trace mechanism `kill:<form>`).
    pub form: &'static str,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct Completion {
    pub fingerprint: u64,
    /// The owner is refused as a whole (escape, parse, a refused target kind).
    pub refusal: Option<Refusal>,
    pub kill_only: Vec<KillOnly>,
    /// Bases with an eager site that maps to no statement node (`site-unmapped`).
    pub refused_bases: BTreeSet<String>,
}

/// Per-file memo of completions, keyed by the owner's byte range.
#[derive(Default)]
pub(crate) struct CompletionMemo(Mutex<BTreeMap<(usize, usize), Arc<Completion>>>);

impl Clone for CompletionMemo {
    fn clone(&self) -> Self {
        Self(Mutex::new(
            self.0.lock().map(|m| m.clone()).unwrap_or_default(),
        ))
    }
}

impl CompletionMemo {
    /// The completion for `key` when its recorded-def fingerprint matches, else compute and keep it.
    pub(crate) fn get_or_compute(
        &self,
        key: (usize, usize),
        fingerprint: u64,
        compute: impl FnOnce() -> Completion,
    ) -> Arc<Completion> {
        if let Some(hit) = self.0.lock().ok().and_then(|m| m.get(&key).cloned()) {
            if hit.fingerprint == fingerprint {
                return hit;
            }
        }
        let fresh = Arc::new(compute());
        if let Ok(mut m) = self.0.lock() {
            m.insert(key, fresh.clone());
        }
        fresh
    }
}

/// Identity of the recorded defs the completion was computed against.
pub(crate) fn fingerprint<'a>(recorded: impl Iterator<Item = (&'a AccessPath, usize)>) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut h = std::collections::hash_map::DefaultHasher::new();
    for (path, byte) in recorded {
        path.hash(&mut h);
        byte.hash(&mut h);
    }
    h.finish()
}

/// `ident`'s path in the producer's representation (`[]` for a subscript).
pub(crate) fn access_path(p: &Path) -> AccessPath {
    AccessPath::with_fields(
        p.base.clone(),
        p.comps
            .iter()
            .map(|c| match c {
                Comp::Name(n) => n.clone(),
                Comp::Index => ARRAY_FIELD.to_string(),
            })
            .collect(),
    )
}

#[cfg(test)]
thread_local! {
    static WALKS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}

/// Census walks this thread has run (tests assert one per owner across both seams).
#[cfg(test)]
pub(crate) fn walks() -> usize {
    WALKS.with(std::cell::Cell::get)
}

/// Run the walk for `owner` against the recorded defs `(path, start_byte)`.
pub(crate) fn completion(
    parsed: &ParsedFile,
    owner: Node<'_>,
    recorded: &[(&AccessPath, usize)],
    fingerprint: u64,
) -> Completion {
    #[cfg(test)]
    WALKS.with(|n| n.set(n.get() + 1));
    let mut out = Completion {
        fingerprint,
        ..Completion::default()
    };
    let c = census_with(parsed, Tables::shared(), owner, false);
    if let Some(r) = c.refusals.first() {
        out.refusal = Some(r.clone());
        return out;
    }
    let bases: BTreeSet<&str> = recorded.iter().map(|(p, _)| p.base.as_str()).collect();
    let mut by_start: BTreeMap<usize, Vec<&AccessPath>> = BTreeMap::new();
    for (path, byte) in recorded {
        by_start.entry(*byte).or_default().push(path);
    }
    for site in &c.sites {
        // Declarations are the refusal classes of S1d (hoisting, Annex B, dead formals), not writes
        // the producer fails to record. Writes in a deferred region or the owner's own parameter list
        // need the scope rules of S1c (inner bindings: a shadowing parameter is not the owner's
        // binding) and S1d (dead formals); the producer's own nested-write handling stands until then.
        let r = site.region;
        if site.form == Form::Declare
            || r.nested
            || r.instance_field
            || r.own_params
            || !bases.contains(site.path.base.as_str())
        {
            continue;
        }
        // `this` inside a class body (static field, static block) or a nested non-arrow callable is
        // another object: the producer's rule for the same question.
        if site.path.base == "this" && this_is_rebound(parsed, owner, site) {
            continue;
        }
        let path = access_path(&site.path);
        let covered = by_start
            .get(&site.target_start)
            .is_some_and(|stored| stored.iter().any(|p| **p == path));
        if covered {
            continue;
        }
        match node_line(parsed, &c, site) {
            Some(line) => out.kill_only.push(KillOnly {
                path,
                line,
                start_byte: site.target_start,
                form: form_of(parsed, site),
            }),
            None => {
                out.refused_bases.insert(site.path.base.clone());
            }
        }
    }
    out
}

fn this_is_rebound(parsed: &ParsedFile, owner: Node<'_>, site: &Site) -> bool {
    parsed
        .tree
        .root_node()
        .descendant_for_byte_range(site.target_start, site.target_start)
        .is_some_and(|node| parsed.js_ts_this_rebound_between(&node, &owner))
}

/// E18: the CFG node (its line) that executes the site's write. The innermost statement span
/// containing the write construct (or target) by bytes names it. The statement universe keeps one
/// statement per line, so a write after an earlier statement on its own line (`{ var a; a++; }`) is in
/// no span; the line is still one CFG node when a span starts on it, and that node executes the write.
/// Anything else (a class or enum declaration line, a continuation line outside every statement) is
/// unmapped and the caller refuses the base.
fn node_line(parsed: &ParsedFile, c: &Census, site: &Site) -> Option<usize> {
    let start = site.construct_start.unwrap_or(site.target_start);
    let end = site.moment;
    let contained = c
        .spans
        .iter()
        .filter(|(a, b)| *a <= start && end <= *b)
        .min_by_key(|(a, b)| (b - a, std::cmp::Reverse(*a)));
    if let Some((a, _)) = contained {
        return Some(parsed.line_for_byte(*a));
    }
    c.spans
        .iter()
        .any(|(a, _)| parsed.line_for_byte(*a) == site.line)
        .then_some(site.line)
}

/// The census form of a site, as the trace names it.
fn form_of(parsed: &ParsedFile, site: &Site) -> &'static str {
    let Some(node) = parsed
        .tree
        .root_node()
        .descendant_for_byte_range(site.target_start, site.target_start)
    else {
        return "other";
    };
    if in_for_increment(node) {
        return "for-increment";
    }
    match site.form {
        Form::Delete => "delete",
        Form::ReadWrite if site.construct_kind == Some("update_expression") => "update",
        Form::ReadWrite => "augmented",
        _ if site.for_head => "for-head",
        _ if in_pattern(node) => "pattern",
        _ => "assign",
    }
}

fn in_for_increment(node: Node<'_>) -> bool {
    let mut cur = node.parent();
    while let Some(n) = cur {
        if n.kind() == "for_statement" {
            return n.child_by_field_name("increment").is_some_and(|i| {
                i.start_byte() <= node.start_byte() && node.end_byte() <= i.end_byte()
            });
        }
        if n.kind().ends_with("statement") || n.kind().ends_with("declaration") {
            return false;
        }
        cur = n.parent();
    }
    false
}

fn in_pattern(node: Node<'_>) -> bool {
    let mut cur = node.parent();
    while let Some(n) = cur {
        if n.kind().ends_with("_pattern") {
            return true;
        }
        if n.kind().ends_with("statement")
            || n.kind().ends_with("expression") && n.kind() != "parenthesized_expression"
        {
            return false;
        }
        cur = n.parent();
    }
    false
}
