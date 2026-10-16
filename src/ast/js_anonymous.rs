//! Conservative anonymous proof environments, using the binder's evaluation table.
use super::js_binding::{classified, is_class, JsBindingCache};
use super::js_binding_walk::{e_rule, field_of, Pos, Walk};
use super::ParsedFile;
use crate::call_graph::ImportBinding;
use crate::js_anonymous::{Environment, ImportEvidence};
use tree_sitter::Node;

impl ParsedFile {
    /// Eager/manual callable inventory adds anonymous generators without changing
    /// the legacy kind list used by default enclosing-function seeds.
    pub(super) fn is_inventoried_function(&self, node: &Node<'_>) -> bool {
        self.language.function_node_types().contains(&node.kind())
            || self.js_ts_synthetic_callable_name(node).is_some()
    }

    pub(crate) fn anonymous_file_environment(&self) -> Environment {
        let mut result = Environment::default();
        let mut cache = JsBindingCache::for_sites();
        let mut stack = vec![self.tree.root_node()];
        while let Some(n) = stack.pop() {
            let k = n.kind();
            result.escaped |= k.contains("identifier") && self.node_text(&n).contains('\\');
            result.with |= k == "with_statement";
            // Includes direct, indirect, shadowed and property eval. Narrowing needs proof.
            result.eval |= k.contains("identifier") && self.node_text(&n) == "eval";
            result.eval |= k == "subscript_expression"
                && n.child_by_field_name("index")
                    .is_some_and(|i| self.anonymous_eval_index(n, i, &mut cache));
            result.eval |= k == "pair_pattern"
                && n.child_by_field_name("key")
                    .filter(|key| key.kind() == "computed_property_name")
                    .and_then(|key| key.named_child(0))
                    .is_some_and(|i| self.anonymous_eval_index(n, i, &mut cache));
            result.nested_import |= matches!(k, "import_statement" | "import_alias")
                && n.parent().is_none_or(|p| p.kind() != "program");
            result.uncertain |= n.is_error() || n.is_missing() || (n.is_named() && !classified(k));
            let mut cursor = n.walk();
            stack.extend(n.children(&mut cursor));
        }
        result
    }

    /// Fold the whole value. Unknown names veto on global-capable receivers only:
    /// ordinary array/object indexing must not fence every anonymous site in a file.
    fn anonymous_eval_index<'a>(
        &'a self,
        member: Node<'a>,
        index: Node<'a>,
        cache: &mut JsBindingCache<'a>,
    ) -> bool {
        // Primitive literal property keys cannot spell eval, including numeric
        // array indexes. Keep this separate from string-only concatenation.
        if self
            .js_ts_unwrap_value(index)
            .is_some_and(|n| matches!(n.kind(), "number" | "true" | "false" | "null"))
        {
            return false;
        }
        match self.anonymous_constant_name(index, 64) {
            Some(name) => name == "eval",
            None => member
                .child_by_field_name("object")
                .or_else(|| {
                    let pattern = member.parent().filter(|p| p.kind() == "object_pattern")?;
                    let owner = pattern.parent()?;
                    // Nested property values, parameters (even with defaults)
                    // and loop elements/keys are not the outer receiver.
                    match owner.kind() {
                        "variable_declarator" => owner.child_by_field_name("value"),
                        "assignment_expression" => owner.child_by_field_name("right"),
                        _ => None,
                    }
                })
                .is_some_and(|object| self.anonymous_global_receiver(object, cache)),
        }
    }

    /// Follow single lexical const declarations by binding identity, never spelling.
    /// Only runtime shadows suppress host roots; const aliases have their own proof.
    fn anonymous_global_receiver<'a>(
        &'a self,
        mut node: Node<'a>,
        cache: &mut JsBindingCache<'a>,
    ) -> bool {
        let mut seen = std::collections::BTreeSet::new();
        loop {
            let Some(value) = self.js_ts_unwrap_value(node) else {
                return false;
            };
            node = value;
            if node.kind() == "this" {
                let mut child = node;
                while let Some(parent) = child.parent() {
                    // Decorator expressions run outside the decorated member's
                    // this binding, including parameter and field decorators.
                    if parent.kind() == "decorator" {
                        let mut holder = parent.parent();
                        while let Some(n) = holder.filter(|n| !is_class(n.kind())) {
                            holder = n.parent();
                        }
                        if let Some(class) = holder {
                            child = class;
                            continue;
                        }
                    }
                    let outside =
                        matches!(
                            e_rule(parent.kind(), field_of(parent, child)),
                            Some(Pos::Outside | Pos::Decorator)
                        ) || (matches!(
                            parent.kind(),
                            "field_definition" | "public_field_definition"
                        ) && matches!(field_of(parent, child), "property" | "name"))
                            || (is_class(parent.kind()) && field_of(parent, child) != "body");
                    // A computed member key is evaluated outside the class's
                    // instance/static this binding, just like an object key.
                    if outside {
                        if let Some(body) = parent.parent().filter(|p| p.kind() == "class_body") {
                            child = body.parent().expect("class body has a class");
                            continue;
                        }
                    }
                    if !outside
                        && ((super::is_js_ts_function_like(parent.kind())
                            && parent.kind() != "arrow_function")
                            || is_class(parent.kind())
                            || parent.kind() == "class_static_block")
                    {
                        return false;
                    }
                    child = parent;
                }
                return true;
            }
            if node.kind() != "identifier" {
                return false;
            }
            let name = self.node_text(&node);
            if matches!(name, "globalThis" | "window" | "self" | "global")
                && !self.anonymous_host_is_shadowed(node, name, cache)
            {
                return true;
            }
            let Some((_, decl)) = self.js_ts_alias_declarator(node, name, cache) else {
                return false;
            };
            if !seen.insert(decl.id())
                || decl
                    .child_by_field_name("name")
                    .is_none_or(|n| n.kind() != "identifier")
            {
                return false;
            }
            let Some(value) = decl.child_by_field_name("value") else {
                return false;
            };
            node = value;
        }
    }

    /// Program var/merged declarations may preserve an existing host property;
    /// ambient declarations create no runtime shadow at all.
    fn anonymous_host_is_shadowed<'a>(
        &'a self,
        at: Node<'a>,
        name: &str,
        cache: &mut JsBindingCache<'a>,
    ) -> bool {
        let Walk::Found(scope, _) = self.js_ts_binding_walk(at, name, cache) else {
            return false;
        };
        let (decls, annex) = self.js_ts_scope_lookup(scope, name, cache);
        if decls.as_ref().is_some_and(|decls| {
            decls.iter().any(|decl| {
                let mut current = Some(*decl);
                while let Some(node) = current {
                    if node.kind() == "ambient_declaration" {
                        return true;
                    }
                    if node.id() == scope.id() {
                        break;
                    }
                    current = node.parent();
                }
                false
            })
        }) {
            return false;
        }
        if scope.kind() != "program" {
            return true;
        }
        !annex
            && decls.is_some_and(|decls| {
                !decls.is_empty()
                    && decls.iter().all(|decl| {
                        matches!(
                            decl.kind(),
                            "function_declaration"
                                | "generator_function_declaration"
                                | "class_declaration"
                                | "abstract_class_declaration"
                        ) || (decl.kind() == "variable_declarator"
                            && decl
                                .parent()
                                .is_some_and(|p| p.kind() == "lexical_declaration"))
                    })
            })
    }

    /// Bounded folding of string values; unsupported syntax fails closed.
    fn anonymous_constant_name(&self, node: Node<'_>, fuel: usize) -> Option<String> {
        let fuel = fuel.checked_sub(1)?;
        let node = self.js_ts_unwrap_value(node)?;
        match node.kind() {
            "string" => self.js_ts_module_export_name(node),
            "binary_expression"
                if node
                    .child_by_field_name("operator")
                    .is_some_and(|op| self.node_text(&op) == "+") =>
            {
                let mut left =
                    self.anonymous_constant_name(node.child_by_field_name("left")?, fuel)?;
                left.push_str(
                    &self.anonymous_constant_name(node.child_by_field_name("right")?, fuel)?,
                );
                Some(left)
            }
            "template_string" => {
                let mut value = String::new();
                let mut cursor = node.walk();
                for part in node.named_children(&mut cursor) {
                    match part.kind() {
                        "string_fragment" => value.push_str(self.node_text(&part)),
                        "template_substitution" => {
                            let mut c = part.walk();
                            let expr = part
                                .named_children(&mut c)
                                .find(|n| n.kind() != "comment")?;
                            value.push_str(&self.anonymous_constant_name(expr, fuel)?);
                        }
                        // Escapes and non-string substitutions remain potentially eval.
                        _ => return None,
                    }
                }
                Some(value)
            }
            _ => None,
        }
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
