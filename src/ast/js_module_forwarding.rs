//! Narrow ESM imported-local forwarding; no CJS/alias or receiver authority.
use super::{is_js_ts_function_like, ParsedFile};
use crate::call_graph::ImportBindingKind;
use crate::js_exports::JsExportTarget;
use std::collections::BTreeSet;
use tree_sitter::Node;

impl ParsedFile {
    /// Only called at a plain-identifier, top-level ESM export occurrence.
    pub(super) fn js_ts_forwarded_import(&self, local: &str) -> Option<JsExportTarget> {
        let root = self.tree.root_node();
        if root.has_error()
            || self.js_ts_forwarding_hazard(root)
            || self.js_ts_forwarding_competitor(root, local, None)
            || self.js_ts_module_value_written(local)
            || self.js_ts_type_only_imports().contains_key(local)
            || !self.js_ts_esm_named_imports(Some(local)).contains(local)
        {
            return None;
        }
        let bindings = self.extract_import_bindings();
        let mut matches = bindings.iter().filter(|b| b.local == local);
        let binding = matches.next()?;
        if matches.next().is_some()
            || binding.kind != ImportBindingKind::MemberImport
            || !(binding.module_path.starts_with("./") || binding.module_path.starts_with("../"))
        {
            return None;
        }
        Some(JsExportTarget::ImportForward {
            module_path: binding.module_path.clone(),
            imported: binding.member.clone()?,
        })
    }

    /// Source terminal proof consumed ONLY by ImportForward. Existing direct
    /// export/import and class lanes keep their independent contracts.
    pub(super) fn js_ts_forwardable_functions(&self) -> BTreeSet<String> {
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

    /// Whole-file refusal of dynamic scope and CJS spellings. Deliberately
    /// conservative: unrelated/shadowed spellings can reduce recall, never
    /// authorize an origin through a dynamic or mutable export object.
    fn js_ts_forwarding_hazard(&self, node: Node<'_>) -> bool {
        if node.kind() == "with_statement"
            || (node.kind() == "identifier"
                && matches!(self.node_text(&node), "eval" | "module" | "exports"))
        {
            return true;
        }
        let mut cursor = node.walk();
        let found = node
            .named_children(&mut cursor)
            .any(|child| self.js_ts_forwarding_hazard(child));
        found
    }

    /// Refuse duplicate/competing value or type declarations, even in nested
    /// scopes. This also fences the downstream file/name callable lookup from
    /// nested same-name function candidates. Import duplicates are counted
    /// separately before extraction loses occurrence information.
    fn js_ts_forwarding_competitor(
        &self,
        node: Node<'_>,
        name: &str,
        allowed_declaration: Option<usize>,
    ) -> bool {
        self.js_ts_forwarding_competitor_except(
            node,
            name,
            &allowed_declaration.into_iter().collect::<Vec<_>>(),
        )
    }

    pub(super) fn js_ts_forwarding_competitor_except(
        &self,
        node: Node<'_>,
        name: &str,
        allowed: &[usize],
    ) -> bool {
        if !allowed.contains(&node.id()) {
            if node.kind() == "variable_declarator" {
                if let Some(pattern) = node.child_by_field_name("name") {
                    let mut names = BTreeSet::new();
                    self.collect_js_ts_binding_pattern_names(pattern, &mut names);
                    if names.contains(name) {
                        return true;
                    }
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
                if node
                    .child_by_field_name("name")
                    .is_some_and(|n| self.node_text(&n).split('.').next() == Some(name))
                {
                    return true;
                }
            }
        }
        let mut cursor = node.walk();
        let found = node
            .named_children(&mut cursor)
            .any(|child| self.js_ts_forwarding_competitor_except(child, name, allowed));
        found
    }
}

#[cfg(test)]
mod tests {
    use super::ParsedFile;
    use crate::languages::Language;

    #[test]
    fn cjs_refusal_module_write_source_bindings() {
        let mut failures = Vec::new();
        for language in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
            for (source, written) in [
                ("function origin() { origin = other; }", true),
                ("function* origin() { origin = other; }", true),
                ("async function* origin() { origin = other; }", true),
                ("function origin(input = (origin = other)) {}", true),
                ("let origin = function() { origin = other; };", true),
                ("let origin = async function() { origin = other; };", true),
                ("let origin = () => { origin = other; };", true),
                (
                    "let origin = function different() { origin = other; };",
                    true,
                ),
                ("function origin() { [origin] = values; }", true),
                ("function origin() { origin++; }", true),
                ("function origin() { for (origin of values) {} }", true),
                ("let origin = function origin() { origin = other; };", false),
                (
                    "let origin = function* origin() { origin = other; };",
                    false,
                ),
                ("function origin(origin) { origin = other; }", false),
                ("function origin() { let origin; origin = other; }", false),
                (
                    "function origin() { try {} catch(origin) { origin = other; } }",
                    false,
                ),
                (
                    "function outer() { function origin() { origin = other; } }",
                    false,
                ),
            ] {
                let parsed = ParsedFile::parse("source.ts", source, language).unwrap();
                assert!(!parsed.tree.root_node().has_error());
                let actual = parsed.js_ts_module_value_written("origin");
                println!("WRITE_AUDIT {language:?} {written} {actual} {source}");
                if actual != written {
                    failures.push(format!("{language:?}: {source}"));
                }
            }
        }
        assert!(failures.is_empty(), "{}", failures.join("; "));
    }
}
