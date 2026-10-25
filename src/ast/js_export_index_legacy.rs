//! Frozen pre-index export helpers, used only by differential tests.
use super::super::ParsedFile;
use std::cell::Cell;
use std::collections::BTreeSet;
thread_local! { static LEGACY: Cell<bool> = const { Cell::new(false) }; }
pub(super) fn enabled() -> bool {
    LEGACY.with(Cell::get)
}
impl ParsedFile {
    pub(crate) fn with_legacy_export_facts<T>(op: impl FnOnce() -> T) -> T {
        struct Restore(bool);
        impl Drop for Restore {
            fn drop(&mut self) {
                LEGACY.with(|v| v.set(self.0));
            }
        }
        let _restore = Restore(LEGACY.with(|v| v.replace(true)));
        op()
    }
    pub(in crate::ast) fn js_ts_esm_named_imports_legacy(
        &self,
        only: Option<&str>,
    ) -> BTreeSet<String> {
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
        names.retain(|name| !self.js_ts_module_value_written(name));
        names
    }

    pub(in crate::ast) fn js_ts_forwardable_functions_legacy(&self) -> BTreeSet<String> {
        let root = self.tree.root_node();
        if root.has_error() || self.js_ts_forwarding_hazard(root) {
            return BTreeSet::new();
        }
        let imports = self.extract_import_bindings();
        let types = self.js_ts_type_only_imports();
        let mut out = BTreeSet::new();
        let mut cursor = root.walk();
        for wrapper in root.named_children(&mut cursor) {
            let declaration = if wrapper.kind() == "export_statement" {
                let Some(declaration) = wrapper.child_by_field_name("declaration") else {
                    continue;
                };
                declaration
            } else {
                wrapper
            };
            if declaration.kind() != "function_declaration"
                || declaration.child_by_field_name("body").is_none()
            {
                continue;
            }
            let Some(name) = declaration.child_by_field_name("name") else {
                continue;
            };
            let name = self.node_text(&name);
            if !imports.iter().any(|b| b.local == name)
                && !types.contains_key(name)
                && !self.js_ts_forwarding_competitor(root, name, Some(declaration.id()))
                && !self.js_ts_module_value_written(name)
            {
                out.insert(name.to_owned());
            }
        }
        out
    }
}
