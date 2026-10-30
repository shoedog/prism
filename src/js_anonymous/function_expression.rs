//! Negative scope facts for explicit function-expression names (#450).
use super::Span;
use crate::call_graph::{CallGraph, CallSite, FunctionId};
use crate::js_exports::ResolvedJsExport;
use crate::resolution::{
    DropReason, ResolutionConfidence, ResolutionKind, ResolutionOutcome, ResolvedCallee,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tree_sitter::Node;

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct ExpressionScopes(BTreeMap<FunctionId, Vec<Span>>);

impl ExpressionScopes {
    pub(super) fn record(&mut self, owner: Node<'_>, id: &FunctionId) {
        if matches!(owner.kind(), "function_expression" | "generator_function")
            && owner.child_by_field_name("name").is_some()
        {
            let regions = self.0.entry(id.clone()).or_default();
            if let (Some(parameters), Some(body)) = (
                owner.child_by_field_name("parameters"),
                owner.child_by_field_name("body"),
            ) {
                regions.push((parameters.start_byte(), body.end_byte()));
            }
        }
    }

    pub(super) fn visible_at(&self, id: &FunctionId, site: &CallSite) -> bool {
        self.0.get(id).is_none_or(|regions| {
            // FunctionIds carry lines, so multiple syntax claims can share an ID.
            // Every claim must cover the site before spelling can prove a target.
            id.file == site.caller.file
                && !regions.is_empty()
                && regions
                    .iter()
                    .all(|&(start, end)| start <= site.start_byte && site.end_byte <= end)
        })
    }

    fn contains(&self, id: &FunctionId) -> bool {
        self.0.contains_key(id)
    }
}

impl CallGraph {
    fn expression_private_name(&self, id: &FunctionId) -> bool {
        self.js_anonymous
            .0
            .get(&id.file)
            .is_some_and(|proof| proof.expression_scopes.contains(id))
    }

    /// Spanless exports only supply spelling, never the expression's outer binding.
    /// Span-verified exports may expose a named expression under any exported name.
    pub(crate) fn js_ts_export_candidates(&self, export: &ResolvedJsExport) -> Vec<&FunctionId> {
        self.functions
            .get(&export.local_name)
            .into_iter()
            .flatten()
            .filter(|id| id.file == export.file && !self.method_owners.contains_key(*id))
            .filter(|id| match export.span {
                Some((start, end)) => id.start_line == start && id.end_line == end,
                None => !self.expression_private_name(id),
            })
            .collect()
    }

    /// The namespace stem fallback has no callable-span authority. Keep its
    /// existing candidates except for expression-local names, even within the file.
    pub(crate) fn js_ts_import_spelling_exact<'a>(
        &self,
        candidates: Vec<&'a FunctionId>,
        kind: ResolutionKind,
    ) -> ResolutionOutcome<'a> {
        let resolved: Vec<_> = candidates
            .into_iter()
            .filter(|id| !self.expression_private_name(id))
            .map(|target| ResolvedCallee {
                target,
                confidence: ResolutionConfidence::Exact,
                kind,
            })
            .collect();
        if resolved.is_empty() {
            ResolutionOutcome::dropped(DropReason::ImportExternal)
        } else {
            ResolutionOutcome::hit(resolved)
        }
    }
}
