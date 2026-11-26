//! Registry writes affect view metadata without invalidating router bindings.
use super::{allocation, values::mutation_candidates, Allocation};
use crate::ast::ParsedFile;
use crate::framework_entries::routes::{
    python::{attribute, literal_string, ungroup},
    receivers::{PythonBindings, PythonValue},
    Invocation, RouterId,
};
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

#[derive(Default)]
pub(super) struct ViewMapWrites<'a>(BTreeMap<Allocation<'a>, InvocationWrites>);

type InvocationWrites = BTreeMap<Vec<Invocation>, ViewMapKeys>;

#[derive(Default)]
struct ViewMapKeys {
    literal: BTreeSet<String>,
    dynamic: bool,
}

impl<'a> ViewMapWrites<'a> {
    pub(super) fn record(
        &mut self,
        bindings: &PythonBindings<'a>,
        file: &'a ParsedFile,
        target: Node<'a>,
    ) {
        let Some((owner, field)) = target
            .child_by_field_name("value")
            .map(ungroup)
            .and_then(attribute)
        else {
            return;
        };
        if file.node_text(&field) != "view_functions" {
            return;
        }
        let key = target
            .child_by_field_name("subscript")
            .and_then(|key| literal_string(file, key));
        // Untraced owners have no attributed runtime write. Keep every lexical
        // candidate allocation, including those behind a refused import.
        for candidate in mutation_candidates(bindings, file, owner, true) {
            if let PythonValue::Router(router) = candidate {
                let keys = self
                    .0
                    .entry(allocation(&router))
                    .or_default()
                    .entry(router.identity.invocations)
                    .or_default();
                if let Some(key) = &key {
                    keys.literal.insert(key.clone());
                } else {
                    keys.dynamic = true;
                }
            }
        }
    }
}

impl PythonBindings<'_> {
    pub(crate) fn view_map_written(&self, router: &RouterId, endpoint: Option<&str>) -> bool {
        self.mutations
            .as_ref()
            .and_then(|writes| {
                writes
                    .view_map
                    .0
                    .get(&(router.file.as_str(), router.construction_byte))
            })
            .is_some_and(|invocations| {
                invocations.iter().any(|(frames, keys)| {
                    // Indexing may know the inner call but lack an active outer frame.
                    router.invocations.ends_with(frames)
                        && (keys.dynamic || endpoint.is_none_or(|key| keys.literal.contains(key)))
                })
            })
    }
}
