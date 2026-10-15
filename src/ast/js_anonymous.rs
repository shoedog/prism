//! Conservative anonymous proof environments, using the binder's evaluation table.
use super::js_binding::{classified, is_class, JsBindingCache};
use super::js_binding_walk::{e_rule, field_of, Pos, Walk};
use super::ParsedFile;
use crate::call_graph::ImportBinding;
use crate::js_anonymous::{Environment, ImportEvidence};
use tree_sitter::Node;

impl ParsedFile {
    pub(crate) fn anonymous_file_environment(&self) -> Environment {
        let mut result = Environment::default();
        let mut stack = vec![self.tree.root_node()];
        while let Some(n) = stack.pop() {
            let k = n.kind();
            result.escaped |= k.contains("identifier") && self.node_text(&n).contains('\\');
            result.with |= k == "with_statement";
            // Includes direct, indirect, shadowed and property eval. Narrowing needs proof.
            result.eval |= k.contains("identifier") && self.node_text(&n) == "eval";
            result.eval |= k == "subscript_expression"
                && n.child_by_field_name("index")
                    .is_some_and(|i| self.anonymous_eval_index(i));
            result.nested_import |= matches!(k, "import_statement" | "import_alias")
                && n.parent().is_none_or(|p| p.kind() != "program");
            result.uncertain |= n.is_error() || n.is_missing() || (n.is_named() && !classified(k));
            let mut cursor = n.walk();
            stack.extend(n.children(&mut cursor));
        }
        result
    }

    /// Literal member names beneath wrappers; uncertain template names may name eval.
    fn anonymous_eval_index(&self, index: Node<'_>) -> bool {
        let mut stack = vec![index];
        while let Some(node) = stack.pop() {
            match node.kind() {
                "string" => {
                    if self
                        .js_ts_module_export_name(node)
                        .as_deref()
                        .is_none_or(|name| name == "eval")
                    {
                        return true;
                    }
                    continue;
                }
                "template_string" => {
                    let mut cursor = node.walk();
                    if self.node_text(&node) == "`eval`"
                        || node
                            .named_children(&mut cursor)
                            .any(|part| part.kind() != "string_fragment")
                    {
                        return true;
                    }
                    continue;
                }
                _ => {}
            }
            let mut cursor = node.walk();
            stack.extend(node.named_children(&mut cursor));
        }
        false
    }

    /// No lexical lookup short-circuit: every evaluation environment must be supported.
    pub(crate) fn anonymous_evaluation_supported(&self, site: Node<'_>) -> bool {
        let mut child = site;
        let mut current = site.parent();
        while let Some(node) = current {
            let mut next = node;
            match e_rule(node.kind(), field_of(node, child)) {
                Some(Pos::Unlisted | Pos::WithBody | Pos::Leave) => return false,
                Some(Pos::Inside) if node.kind() == "class_static_block" => return false,
                Some(Pos::Decorator) => {
                    let Some(holder) = node.parent() else {
                        return false;
                    };
                    if holder.kind() == "class" && holder.child_by_field_name("name").is_some() {
                        return false;
                    }
                    if is_class(holder.kind()) {
                        next = holder;
                    } else {
                        let mut up = Some(holder);
                        while let Some(n) = up.filter(|n| n.kind() != "class_body") {
                            up = n.parent();
                        }
                        let Some(body) = up else {
                            return false;
                        };
                        child = body;
                        current = body.parent();
                        continue;
                    }
                }
                _ => {}
            }
            child = next;
            current = next.parent();
        }
        true
    }

    /// Actual nearest declaration and its bytes, restricted to value ESM syntax.
    pub(crate) fn anonymous_program_import<'a>(
        &'a self,
        at: Node<'a>,
        cache: &mut JsBindingCache<'a>,
    ) -> Option<ImportEvidence> {
        let name = self.node_text(&at);
        let Walk::Found(scope, None) = self.js_ts_binding_walk(at, name, cache) else {
            return None;
        };
        if scope.kind() != "program" {
            return None;
        }
        let (decls, annex) = self.js_ts_scope_lookup(scope, name, cache);
        let decls = decls?;
        let [decl] = decls.as_slice() else {
            return None;
        };
        if annex
            || decl.kind() != "import_statement"
            || decl.has_error()
            || decl.child_by_field_name("source").is_none()
            || self.js_ts_import_statement_is_type_only(*decl)
        {
            return None;
        }
        let mut imports: Vec<ImportBinding> = Vec::new();
        self.collect_js_import_bindings_node(*decl, &mut imports);
        let mut matched = imports.into_iter().filter(|b| b.local == name);
        let binding = matched.next()?;
        if matched.next().is_some() {
            return None;
        }
        Some(ImportEvidence {
            declaration: (decl.start_byte(), decl.end_byte()),
            binding,
        })
    }
}
