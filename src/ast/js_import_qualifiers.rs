//! S2 extraction. Reuse scope identity, cleanliness, write walks and Callable terminals.
use super::js_binding::{JsBinding, JsBindingCache, JsTerminal};
use super::js_binding_walk::Walk;
use super::ParsedFile;
use crate::js_exports::{JsExportFacts, JsExportTarget, ResolvedJsExport};
use crate::js_import_qualifiers::{Members, QualifierExport, QualifierFacts};
use std::collections::{BTreeMap, BTreeSet};
use tree_sitter::Node;

impl ParsedFile {
    pub(super) fn js_ts_qualifier_facts(&self, facts: &JsExportFacts) -> QualifierFacts {
        let root = self.tree.root_node();
        let mut cache = JsBindingCache::for_sites();
        let mut out = QualifierFacts {
            syntax_incomplete: root.has_error(),
            ..Default::default()
        };
        out.written
            .extend(self.js_ts_qualifier_refused_uses(&mut out.called_members));
        if root.has_error()
            || self.js_ts_scope_clean(root, &mut cache).is_err()
            || !self.js_ts_is_module()
        {
            return out;
        }
        out.complete = true;
        out.written
            .extend(self.js_ts_qualifier_member_writes(&mut cache));
        let mut stack = vec![root];
        while let Some(n) = stack.pop() {
            if n.kind() == "call_expression" {
                if let Some(q) = n
                    .child_by_field_name("function")
                    .filter(|n| n.kind() == "member_expression")
                    .and_then(|n| n.child_by_field_name("object"))
                    .filter(|n| n.kind() == "identifier")
                {
                    if self.js_ts_site_binding(q, self.node_text(&q), &mut cache)
                        == JsBinding::Import
                    {
                        out.import_sites.insert((n.start_byte(), n.end_byte()));
                    }
                }
            }
            let mut c = n.walk();
            stack.extend(n.named_children(&mut c));
        }
        // Landed facts provide re-export/import-forward identity, including names
        // whose local values are unproven. The latter are absence barriers only.
        for (name, target) in &facts.named {
            let target = match target {
                JsExportTarget::ReExport {
                    module_path,
                    imported,
                }
                | JsExportTarget::ImportForward {
                    module_path,
                    imported,
                } => QualifierExport::Forward {
                    module: module_path.clone(),
                    imported: imported.clone(),
                },
                _ => QualifierExport::Other,
            };
            out.named.insert(name.clone(), target);
        }
        out.conflicted = facts.conflicted.clone();
        let mut cursor = root.walk();
        for stmt in root.named_children(&mut cursor) {
            let decl = if stmt.kind() == "export_statement" {
                stmt.child_by_field_name("declaration")
            } else {
                Some(stmt)
            };
            if let Some(decl) = decl {
                let mut declarations = vec![decl];
                if matches!(decl.kind(), "lexical_declaration" | "variable_declaration") {
                    let mut dc = decl.walk();
                    declarations = decl
                        .named_children(&mut dc)
                        .filter(|n| n.kind() == "variable_declarator")
                        .collect();
                }
                for d in declarations {
                    let Some(id) = d
                        .child_by_field_name("name")
                        .filter(|n| matches!(n.kind(), "identifier" | "type_identifier"))
                    else {
                        if stmt.kind() == "export_statement" && d.kind() == "variable_declarator" {
                            out.complete = false;
                        }
                        continue;
                    };
                    let name = self.node_text(&id).to_string();
                    let mut decorated = stmt.walk();
                    if stmt
                        .named_children(&mut decorated)
                        .any(|n| n.kind() == "decorator")
                    {
                        continue;
                    }
                    let mut decorated = d.walk();
                    if d.named_children(&mut decorated)
                        .any(|n| n.kind() == "decorator")
                    {
                        continue;
                    }
                    let (ds, annex) = self.js_ts_scope_lookup(root, &name, &mut cache);
                    let mut heritage = d.walk();
                    if d.kind() == "class_declaration"
                        && d.named_children(&mut heritage)
                            .any(|child| child.kind() == "class_heritage")
                    {
                        continue;
                    }
                    if annex
                        || !ds.is_some_and(|ds| ds.len() == 1 && ds[0].id() == d.id())
                        || self.js_ts_scoped_written(root, &name, &mut cache)
                        || out.written.contains(&name)
                    {
                        continue;
                    }
                    let members = match d.kind() {
                        "class_declaration" => self.js_ts_literal_members(
                            d.child_by_field_name("body"),
                            true,
                            &mut cache,
                        ),
                        "variable_declarator"
                            if d.parent().is_some_and(|p| {
                                p.kind() == "lexical_declaration"
                                    && self.node_text(&p).starts_with("const ")
                            }) =>
                        {
                            self.js_ts_literal_members(
                                d.child_by_field_name("value")
                                    .filter(|v| v.kind() == "object"),
                                false,
                                &mut cache,
                            )
                        }
                        "internal_module" | "module" => {
                            self.js_ts_declared_namespace_members(d, &mut cache)
                        }
                        _ => None,
                    };
                    let members = members.filter(|members| {
                        out.called_members
                            .get(&name)
                            .is_none_or(|calls| calls.iter().all(|m| members.contains_key(m)))
                    });
                    if let Some(members) = members {
                        out.locals.insert(name.clone(), members);
                        if stmt.kind() == "export_statement" {
                            let exported = if self.js_ts_export_statement_is_default(stmt) {
                                "default".into()
                            } else {
                                name.clone()
                            };
                            // Replace one existing syntactic claim, but never clear its conflicts.
                            out.named.insert(exported, QualifierExport::Local(name));
                        }
                    } else if stmt.kind() == "export_statement" {
                        out.named.entry(name).or_insert(QualifierExport::Other);
                    }
                }
            }
            if stmt.kind() != "export_statement" {
                continue;
            }
            if self.js_ts_import_statement_is_type_only(stmt) {
                continue;
            }
            let source = stmt
                .child_by_field_name("source")
                .and_then(|n| self.js_ts_module_export_name(n));
            let mut sc = stmt.walk();
            for child in stmt.named_children(&mut sc) {
                if child.kind() == "namespace_export" {
                    if let (Some(module), Some(id)) = (source.clone(), child.named_child(0)) {
                        out.insert(
                            self.node_text(&id).into(),
                            QualifierExport::Namespace(module),
                        );
                    } else {
                        out.complete = false;
                    }
                }
                if child.kind() == "export_clause" && source.is_none() {
                    let mut cc = child.walk();
                    for spec in child
                        .named_children(&mut cc)
                        .filter(|n| n.kind() == "export_specifier")
                    {
                        if self.js_ts_import_specifier_is_type_only(spec) {
                            continue;
                        }
                        let Some(local) = spec
                            .child_by_field_name("name")
                            .and_then(|n| self.js_ts_module_export_name(n))
                        else {
                            out.complete = false;
                            continue;
                        };
                        let exported = spec
                            .child_by_field_name("alias")
                            .and_then(|n| self.js_ts_module_export_name(n))
                            .unwrap_or_else(|| local.clone());
                        if out.written.contains(&local) {
                            out.named.insert(exported, QualifierExport::Other);
                        } else if out.locals.contains_key(&local) {
                            out.named.insert(exported, QualifierExport::Local(local));
                        }
                    }
                }
            }
            if let Some(value) = stmt.child_by_field_name("value") {
                if value.kind() == "identifier" && out.locals.contains_key(self.node_text(&value)) {
                    out.named.insert(
                        "default".into(),
                        QualifierExport::Local(self.node_text(&value).into()),
                    );
                } else {
                    out.named
                        .entry("default".into())
                        .or_insert(QualifierExport::Other);
                }
            }
        }
        out
    }

    fn js_ts_literal_members<'a>(
        &'a self,
        body: Option<Node<'a>>,
        class: bool,
        cache: &mut JsBindingCache<'a>,
    ) -> Option<Members> {
        let body = body?;
        let mut counts = BTreeMap::<String, usize>::new();
        let mut members = Members::new();
        let mut cursor = body.walk();
        for m in body.named_children(&mut cursor) {
            if m.kind() == "comment" {
                continue;
            }
            let key = m
                .child_by_field_name("name")
                .or_else(|| m.child_by_field_name("property"))
                .or_else(|| m.child_by_field_name("key"));
            let key = key
                .filter(|k| matches!(k.kind(), "property_identifier" | "identifier" | "string"))?;
            let name = self.js_ts_module_export_name(key)?;
            *counts.entry(name.clone()).or_default() += 1;
            let mut mc = m.walk();
            let tokens: Vec<_> = m.children(&mut mc).map(|n| n.kind()).collect();
            if class && !tokens.contains(&"static") {
                continue;
            }
            if tokens
                .iter()
                .any(|k| matches!(*k, "get" | "set" | "decorator"))
            {
                continue;
            }
            let value = if m.kind() == "method_definition" {
                Some(m)
            } else {
                m.child_by_field_name("value")
            };
            if let Some(value) = value.filter(|v| {
                matches!(
                    v.kind(),
                    "method_definition" | "arrow_function" | "function_expression"
                )
            }) {
                if let JsBinding::Callable(t) = self.js_ts_qualifier_terminal(value, cache) {
                    members.insert(name, self.js_ts_qualifier_identity(t));
                }
            }
        }
        members.retain(|name, _| counts.get(name) == Some(&1));
        Some(members)
    }

    fn js_ts_declared_namespace_members<'a>(
        &'a self,
        decl: Node<'a>,
        cache: &mut JsBindingCache<'a>,
    ) -> Option<Members> {
        let body = decl.child_by_field_name("body")?;
        let mut out = Members::new();
        let mut cursor = body.walk();
        for stmt in body
            .named_children(&mut cursor)
            .filter(|n| n.kind() == "export_statement")
        {
            let d = stmt.child_by_field_name("declaration")?;
            if d.kind() != "function_declaration" {
                continue;
            }
            let id = d.child_by_field_name("name")?;
            let name = self.node_text(&id);
            if let JsBinding::Callable(t) =
                self.js_ts_scope_binding(body, None, name, d, cache, false)
            {
                out.insert(name.into(), self.js_ts_qualifier_identity(t));
            }
        }
        Some(out)
    }

    // Member syntax has no identifier binding of its own. Extend the binding
    // core's terminal capture only after the qualifier and member guards pass.
    fn js_ts_qualifier_terminal<'a>(
        &'a self,
        node: Node<'a>,
        cache: &mut JsBindingCache<'a>,
    ) -> JsBinding {
        if self.js_ts_scope_clean(node, cache).is_err()
            || node.child_by_field_name("body").is_none()
        {
            return JsBinding::Refused("not_callable");
        }
        let Some(id) = self.language.function_name(&node) else {
            return JsBinding::Refused("unindexed");
        };
        let (start_line, end_line) = self.node_line_range(&node);
        JsBinding::Callable(JsTerminal {
            local: self.node_text(&id).into(),
            start_line,
            end_line,
            wrapped: false,
        })
    }

    fn js_ts_qualifier_identity(&self, t: JsTerminal) -> ResolvedJsExport {
        ResolvedJsExport {
            file: self.path.clone(),
            local_name: t.local,
            span: Some((t.start_line, t.end_line)),
            wrapped: t.wrapped,
            via_unresolved_star: false,
        }
    }

    // Initializers and arrows do not establish receiver ownership. Conservatively
    // retain every enclosing declaration as a possible receiver carrier.
    fn js_ts_qualifier_receiver_carriers(&self, n: Node<'_>) -> BTreeSet<String> {
        let mut carriers = BTreeSet::new();
        let mut up = n.parent();
        while let Some(p) = up {
            let decl = if matches!(p.kind(), "class_declaration" | "internal_module" | "module") {
                Some(p)
            } else if p.kind() == "object" {
                p.parent().filter(|d| d.kind() == "variable_declarator")
            } else {
                None
            };
            if let Some(id) = decl.and_then(|d| d.child_by_field_name("name")) {
                carriers.insert(self.node_text(&id).into());
            }
            up = p.parent();
        }
        if carriers.is_empty() {
            carriers.insert("*namespace*".into());
        }
        carriers
    }

    // Closed lexical whitelist: a new syntax form refuses by default. Shadowed
    // uses deliberately over-approximate the program binding's value uses.
    fn js_ts_qualifier_refused_uses(
        &self,
        called_members: &mut BTreeMap<String, BTreeSet<String>>,
    ) -> BTreeSet<String> {
        let mut refused = BTreeSet::new();
        let mut default_values = BTreeSet::new();
        let mut active = BTreeSet::new();
        let mut stack = vec![self.tree.root_node()];
        while let Some(n) = stack.pop() {
            if n.kind() == "super"
                || n.kind() == "meta_property" && self.node_text(&n) == "new.target"
            {
                refused.extend(self.js_ts_qualifier_receiver_carriers(n));
            }
            if matches!(
                n.kind(),
                "identifier"
                    | "property_identifier"
                    | "shorthand_property_identifier"
                    | "shorthand_property_identifier_pattern"
                    | "this"
            ) {
                let names = if n.kind() == "this" {
                    self.js_ts_qualifier_receiver_carriers(n)
                } else {
                    BTreeSet::from([self.node_text(&n).to_string()])
                };
                for name in &names {
                    if !self.js_ts_qualifier_use_allowed(n) {
                        refused.insert(name.to_string());
                    }
                    if n.parent().is_some_and(|p| {
                        p.kind() == "export_statement" && p.child_by_field_name("value") == Some(n)
                    }) {
                        default_values.insert(name.to_string());
                    } else if self.js_ts_qualifier_direct_use(n) {
                        active.insert(name.to_string());
                        // The syntactic call arm is provisional until the joined
                        // identity proves this is a captured own Callable member.
                        if let Some(member) = n
                            .parent()
                            .filter(|p| p.kind() == "member_expression")
                            .and_then(|p| p.child_by_field_name("property"))
                        {
                            called_members
                                .entry(name.to_string())
                                .or_default()
                                .insert(self.node_text(&member).into());
                        }
                    }
                }
            }
            let mut cursor = n.walk();
            stack.extend(n.named_children(&mut cursor));
        }
        refused.extend(default_values.intersection(&active).cloned());
        refused
    }

    fn js_ts_qualifier_direct_use(&self, n: Node<'_>) -> bool {
        let Some(p) = n.parent() else { return false };
        if p.kind() == "new_expression" && p.child_by_field_name("constructor") == Some(n) {
            return false;
        }
        p.kind() == "member_expression"
            && p.child_by_field_name("object") == Some(n)
            && p.child_by_field_name("property")
                .is_some_and(|k| k.kind() == "property_identifier")
            && {
                let mut cursor = p.walk();
                let optional = p
                    .children(&mut cursor)
                    .any(|c| c.kind() == "optional_chain");
                !optional
            }
            && p.parent().is_some_and(|call| {
                call.kind() == "call_expression" && call.child_by_field_name("function") == Some(p)
            })
    }

    fn js_ts_qualifier_use_allowed(&self, n: Node<'_>) -> bool {
        if self.js_ts_qualifier_direct_use(n) {
            return true;
        }
        let mut up = n.parent();
        while let Some(p) = up {
            if matches!(
                p.kind(),
                "type_annotation"
                    | "type_arguments"
                    | "type_parameters"
                    | "type_alias_declaration"
                    | "interface_declaration"
            ) {
                return true;
            }
            up = p.parent();
        }
        let Some(p) = n.parent() else { return false };
        match p.kind() {
            "class_declaration"
            | "variable_declarator"
            | "function_declaration"
            | "function_expression"
            | "internal_module"
            | "module" => p.child_by_field_name("name") == Some(n),
            "required_parameter" | "optional_parameter" => {
                p.child_by_field_name("pattern") == Some(n)
            }
            "formal_parameters" | "import_clause" | "namespace_import" => true,
            "import_specifier" => {
                p.child_by_field_name("name") == Some(n)
                    || p.child_by_field_name("alias") == Some(n)
            }
            "export_specifier" => {
                p.child_by_field_name("name") == Some(n)
                    && p.child_by_field_name("alias")
                        .is_none_or(|alias| self.node_text(&alias) == self.node_text(&n))
            }
            "export_statement" => p.child_by_field_name("value") == Some(n),
            _ => false,
        }
    }

    fn js_ts_qualifier_member_writes<'a>(
        &'a self,
        cache: &mut JsBindingCache<'a>,
    ) -> BTreeSet<String> {
        let mut out = BTreeSet::new();
        let mut stack = vec![self.tree.root_node()];
        while let Some(n) = stack.pop() {
            if let Some(target) = self.js_ts_write_target(n) {
                // Include destructuring member targets. Visiting index/key
                // subexpressions is deliberately conservative under Option K.
                let mut targets = vec![target];
                while let Some(mut t) = targets.pop() {
                    let mut cursor = t.walk();
                    targets.extend(t.named_children(&mut cursor));
                    if !matches!(t.kind(), "member_expression" | "subscript_expression") {
                        continue;
                    }
                    while matches!(t.kind(), "member_expression" | "subscript_expression") {
                        let Some(object) = t.child_by_field_name("object") else {
                            break;
                        };
                        t = object;
                    }
                    if t.kind() == "identifier" {
                        let name = self.node_text(&t);
                        if matches!(self.js_ts_binding_walk(t, name, cache), Walk::Found(s, _) if s.kind() == "program")
                        {
                            out.insert(name.into());
                        }
                    } else if t.kind() == "this" {
                        out.extend(self.js_ts_qualifier_receiver_carriers(t));
                    }
                }
            }
            let mut cursor = n.walk();
            stack.extend(n.named_children(&mut cursor));
        }
        out
    }
}
