//! Alias-twin occurrence policy (alias-twin lane, slice 1: Java).
//!
//! A DFG alias twin is a fabricated Def of an alias's target, minted at an lvalue occurrence of
//! the alias name (`u = o` makes `u.f = 1` also a write of `o.f`). The invariant this module
//! enforces, one language at a time:
//!
//! > A synthetic write to the target's storage requires an actual storage-write occurrence AND an
//! > admitted storage relation. Establishing or replacing an ordinary variable binding does not
//! > itself write the copied value's source storage.
//!
//! This module decides only the first half, per OCCURRENCE (never per normalised path): it reads
//! the original AST node the lvalue span covers plus scoped declaration evidence. The relation half
//! (is `u` really a copy of `o`) is untouched: alias discovery, the alias map and every rejected or
//! unsupported candidate are exactly as before, so no relation is resurrected (the stale-relation
//! hazard of rejecting candidates does not arise here, because no candidate is rejected).
//!
//! Dispositions: `BindingWrite` (declares, initialises or reassigns the variable named by the
//! occurrence: mint NO target twin, keep the alias binding's own def and the right-hand reads),
//! `StorageWrite` (writes the storage the name refers to: twin unchanged) and `Unsupported`
//! (missing or contradictory evidence: twin unchanged, never repaired opportunistically).
//!
//! One decision per occurrence, keyed by the span's start byte inside its owner, computed once per
//! owner ([`TwinPolicy`]) and read at all three producer integration points (twin Def registration,
//! the fallback anchor registration and twin edge emission in `file.rs`). A language that is not
//! enabled here returns an empty policy and runs no classifier, so its output is byte-identical.
//!
//! Refusal R1 (member-name tokens, an extraction defect owned by its own slice): the occurrence
//! extraction records a method name (`x.size()`) or another object's member name (`node.key`) as an
//! occurrence of the variable of that spelling. A twin def of the target currently, by accident,
//! kills edges that run through such a token; removing it would promote them to `Exact` although the
//! token does not name the variable (the gate found 18 such edges on guava). So a binding write keeps
//! its twin when the owner holds a method-name or non-`this` member-name token spelled like the alias's
//! resolved target root. Purely subtractive: the refused occurrence behaves exactly as before.
//!
//! Specification: the Java rows `J-01..J-06` of the slice-0 classifier table (`CLASSIFIER.md`);
//! `java_tests.rs` pins them, including parity with the slice-0 oracle's pinned cases.
use crate::access_path::AccessPath;
use crate::ast::{ParsedFile, PathSpan};
use crate::languages::Language;
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

mod java;
#[cfg(test)]
mod java_tests;

/// What an lvalue occurrence does to storage (closed set).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Disposition {
    BindingWrite,
    StorageWrite,
    Unsupported,
}

/// A classified occurrence with the classifier-table row that decided it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Classified {
    pub disposition: Disposition,
    pub rule: &'static str,
}

/// The per-owner twin decisions, computed once and shared by registration, the fallback anchors
/// and edge emission.
#[derive(Default)]
pub(super) struct TwinPolicy {
    binding_writes: BTreeSet<usize>,
}

impl TwinPolicy {
    /// Classify every lvalue span of the owner whose base is an alias. Only Java is enabled;
    /// any other language yields the empty policy without inspecting a single span.
    pub(super) fn for_owner(
        parsed: &ParsedFile,
        owner: &Node<'_>,
        lvalue_spans: &[PathSpan],
        alias_map: &BTreeMap<String, String>,
    ) -> Self {
        let mut policy = Self::default();
        if parsed.language != Language::Java || alias_map.is_empty() {
            return policy;
        }
        let mut member_tokens: Option<BTreeSet<String>> = None;
        for span in lvalue_spans {
            let Some(target) = alias_map.get(&span.path.base) else {
                continue;
            };
            if !span.path.is_simple()
                || java::classify(parsed, span).disposition != Disposition::BindingWrite
            {
                continue;
            }
            // R1: refuse when a member-name token of the target's spelling exists in the owner.
            let tokens =
                member_tokens.get_or_insert_with(|| java::member_name_tokens(parsed, owner));
            if !tokens.contains(&AccessPath::from_expr(target).base) {
                policy.binding_writes.insert(span.start_byte);
            }
        }
        policy
    }

    /// True when the occurrence at `start_byte` only (re)binds the alias variable: it mints no
    /// target twin def and no twin edge.
    pub(super) fn suppresses_span(&self, start_byte: usize) -> bool {
        self.binding_writes.contains(&start_byte)
    }

    /// Fallback anchor decision, keyed by the alias line. Anchors exist only for destructuring
    /// aliases (JS/TS), which no enabled language produces, so no anchor is suppressed.
    pub(super) fn suppresses_anchor(&self, _alias: &str, _line: usize) -> bool {
        false
    }
}
