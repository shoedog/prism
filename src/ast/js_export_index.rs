//! File-local inventories for export queries that formerly rescanned the whole tree per name.
//! Targets and declaration identities preserve the legacy predicates; lexical write refusal
//! still uses the same source-self-binding policy at the original target node.
use super::super::{is_js_ts_function_like, ParsedFile};
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

#[cfg(test)]
#[path = "js_export_index_legacy.rs"]
mod legacy;
#[cfg(test)]
pub(super) fn legacy_enabled() -> bool {
    legacy::enabled()
}
#[cfg(test)]
#[path = "js_export_index_tests.rs"]
mod tests;

pub(super) struct ExportIndex<'a> {
    declarations: BTreeMap<String, BTreeSet<usize>>,
    writes: BTreeMap<String, Vec<Node<'a>>>,
}

impl<'a> ExportIndex<'a> {
    pub(super) fn new(parsed: &'a ParsedFile) -> Self {
        #[cfg(test)]
        tests::record_build();
        let mut out = Self {
            declarations: BTreeMap::new(),
            writes: BTreeMap::new(),
        };
        let mut stack = vec![parsed.tree.root_node()];
        while let Some(node) = stack.pop() {
            #[cfg(test)]
            super::tests::visit();
            let mut names = BTreeSet::new();
            if node.kind() == "variable_declarator" {
                if let Some(pattern) = node.child_by_field_name("name") {
                    parsed.collect_js_ts_binding_pattern_names(pattern, &mut names);
                }
            } else if is_js_ts_function_like(node.kind())
                || matches!(
                    node.kind(),
                    "class_declaration"
                        | "class"
                        | "abstract_class_declaration"
                        | "interface_declaration"
                        | "type_alias_declaration"
                        | "enum_declaration"
                        | "function_signature"
                        | "internal_module"
                        | "module"
                        | "import_alias"
                )
            {
                if let Some(name) = node.child_by_field_name("name") {
                    names.insert(
                        parsed
                            .node_text(&name)
                            .split('.')
                            .next()
                            .unwrap_or_default()
                            .to_owned(),
                    );
                }
            }
            for name in names {
                out.declarations.entry(name).or_default().insert(node.id());
            }
            // This is exactly js_ts_module_written(name, false)'s target selection.
            // In particular, a declaring loop head and TS `using` retain legacy treatment.
            if matches!(
                node.kind(),
                "assignment_expression"
                    | "augmented_assignment_expression"
                    | "update_expression"
                    | "for_in_statement"
                    | "for_of_statement"
                    | "for_await_statement"
            ) {
                if let Some(target) = node
                    .child_by_field_name("left")
                    .or_else(|| node.child_by_field_name("argument"))
                    .or_else(|| node.named_child(0))
                {
                    let mut names = BTreeSet::new();
                    parsed.collect_js_ts_binding_pattern_names(target, &mut names);
                    for name in names {
                        out.writes.entry(name).or_default().push(target);
                    }
                }
            }
            let mut cursor = node.walk();
            stack.extend(node.named_children(&mut cursor));
        }
        out
    }

    pub(super) fn competes(&self, name: &str, allowed: usize) -> bool {
        self.declarations
            .get(name)
            .is_some_and(|ids| ids.iter().any(|id| *id != allowed))
    }

    pub(super) fn written(&self, parsed: &ParsedFile, name: &str) -> bool {
        if !matches!(
            parsed.language,
            crate::languages::Language::JavaScript
                | crate::languages::Language::TypeScript
                | crate::languages::Language::Tsx
        ) {
            return false;
        }
        self.writes.get(name).is_some_and(|targets| {
            targets.iter().any(|target| {
                #[cfg(test)]
                super::tests::visit();
                !parsed.js_ts_has_closer_binding(target, name, parsed.tree.root_node().id(), true)
            })
        })
    }
}

impl ParsedFile {
    pub(in crate::ast) fn js_ts_export_imports_and_functions(
        &self,
        facts: &mut crate::js_exports::JsExportFacts,
    ) {
        // Both all-name queries share one lazy inventory. Empty/refused queries
        // still avoid building it, and single-name queries keep their raw walk.
        let mut index = None;
        facts.esm_named_imports = self.js_ts_esm_named_imports_with_index(None, &mut index);
        facts.forwardable_function_locals = self.js_ts_forwardable_functions_with_index(&mut index);
    }

    pub(in crate::ast) fn js_ts_esm_named_imports(&self, only: Option<&str>) -> BTreeSet<String> {
        self.js_ts_esm_named_imports_with_index(only, &mut None)
    }

    fn js_ts_esm_named_imports_with_index<'a>(
        &'a self,
        only: Option<&str>,
        index: &mut Option<ExportIndex<'a>>,
    ) -> BTreeSet<String> {
        #[cfg(test)]
        if legacy_enabled() {
            return self.js_ts_esm_named_imports_legacy(only);
        }
        let mut names = BTreeSet::new();
        let root = self.tree.root_node();
        if root.has_error() {
            return names;
        }
        let mut cursor = root.walk();
        for node in root.named_children(&mut cursor) {
            if node.kind() != "import_statement" || self.js_ts_import_statement_is_type_only(node) {
                continue;
            }
            let mut imports = Vec::new();
            self.collect_js_import_statement_bindings(node, &mut imports);
            names.extend(
                imports
                    .into_iter()
                    .filter(|b| {
                        b.kind == crate::call_graph::ImportBindingKind::MemberImport
                            && only.is_none_or(|name| b.local == name)
                    })
                    .map(|b| b.local),
            );
        }
        if !names.is_empty() {
            if only.is_some() {
                names.retain(|name| !self.js_ts_module_value_written(name));
            } else {
                let index = index.get_or_insert_with(|| ExportIndex::new(self));
                names.retain(|name| !index.written(self, name));
            }
        }
        names
    }
}
