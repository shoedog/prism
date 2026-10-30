//! Negative scope facts for explicit function-expression names (#450).
use super::Span;
use crate::call_graph::{CallSite, FunctionId};
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
            let bodies = self.0.entry(id.clone()).or_default();
            if let Some(body) = owner.child_by_field_name("body") {
                bodies.push((body.start_byte(), body.end_byte()));
            }
        }
    }

    pub(super) fn visible_at(&self, id: &FunctionId, site: &CallSite) -> bool {
        self.0.get(id).is_none_or(|bodies| {
            // FunctionIds carry lines, so multiple syntax claims can share an ID.
            // Every claim must cover the site before spelling can prove a target.
            id.file == site.caller.file
                && !bodies.is_empty()
                && bodies
                    .iter()
                    .all(|&(start, end)| start <= site.start_byte && site.end_byte <= end)
        })
    }
}
