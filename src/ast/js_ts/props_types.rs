//! JS/TS parameter and props types: contextual parameter types, local type declarations.

use crate::ast::{is_plain_ident, JsTsReceiverBindingEvidence, ParsedFile};
use crate::languages::Language;
use std::collections::BTreeSet;
use tree_sitter::Node;

impl ParsedFile {
    pub(super) fn js_ts_parameter_receiver_binding(
        &self,
        func_node: Node<'_>,
        receiver_name: &str,
    ) -> Option<JsTsReceiverBindingEvidence> {
        let params = self.find_parameters_node(&func_node)?;
        let mut matches = Vec::new();
        let mut cursor = params.walk();
        for parameter in params.named_children(&mut cursor) {
            let mut names = BTreeSet::new();
            self.collect_js_ts_parameter_binding_names(parameter, &mut names);
            if !names.contains(receiver_name) {
                continue;
            }
            let binding = parameter
                .child_by_field_name("pattern")
                .or_else(|| parameter.child_by_field_name("name"))
                .or_else(|| parameter.child_by_field_name("left"))
                .or_else(|| (parameter.kind() == "identifier").then_some(parameter));
            let simple_binding = binding.is_some_and(|binding| {
                binding.kind() == "identifier" && self.node_text(&binding) == receiver_name
            });
            // Preserve the original contextual declaration/argument node. An
            // explicit annotation is terminal, including unsupported forms.
            let props_type = || match parameter.child_by_field_name("type") {
                Some(annotation) => annotation.named_child(0),
                None => self.js_ts_contextual_parameter_type(func_node),
            };
            let recovered_type = matches!(self.language, Language::TypeScript | Language::Tsx)
                .then(|| {
                    if simple_binding {
                        self.js_ts_simple_type_annotation(parameter.child_by_field_name("type")?)
                    } else {
                        let ty = props_type()?;
                        self.js_ts_inline_prop_receiver_type(parameter, binding?, ty, receiver_name)
                    }
                })
                .flatten();
            let imported = || {
                let ty = props_type()?;
                let name = self.node_text(&ty);
                if !matches!(self.language, Language::TypeScript | Language::Tsx)
                    || ty.kind() != "type_identifier"
                    || self.js_ts_type_name_shadowed(self.tree.root_node(), &ty, name)
                    || crate::call_graph::js_ts::props::named_import(self, name).is_none()
                {
                    return None;
                }
                Some(JsTsReceiverBindingEvidence::ImportedProps {
                    type_name: name.to_string(),
                    property: self.js_ts_prop_binding(parameter, binding?, receiver_name)?,
                    declaration_end_byte: parameter.end_byte(),
                })
            };
            matches.push(recovered_type.map_or_else(
                || imported().unwrap_or(JsTsReceiverBindingEvidence::Materialized),
                |static_type| JsTsReceiverBindingEvidence::Recovered {
                    static_type,
                    recovery: crate::resolution::ReceiverRecovery::TypedParam,
                    declaration_end_byte: Some(parameter.end_byte()),
                },
            ));
        }
        match matches.len() {
            0 => None,
            1 => matches.pop(),
            _ => Some(JsTsReceiverBindingEvidence::Materialized),
        }
    }

    /// Keep declaration nodes for ordinary signatures and use-site argument nodes
    /// for bounded F<P> = (p: P) instantiation; never substitute name strings.
    fn js_ts_contextual_parameter_type<'a>(&'a self, func: Node<'a>) -> Option<Node<'a>> {
        fn single_child(params: Node<'_>) -> Option<Node<'_>> {
            let mut cursor = params.walk();
            let mut children = params
                .named_children(&mut cursor)
                .filter(|n| n.kind() != "comment");
            let child = children.next()?;
            children.next().is_none().then_some(child)
        }
        fn single_parameter(params: Node<'_>) -> Option<Node<'_>> {
            single_child(params).filter(|n| n.kind() == "required_parameter")
        }
        if !matches!(func.kind(), "arrow_function" | "function_expression")
            || func.child_by_field_name("type_parameters").is_some()
        {
            return None;
        }
        let implementation = single_parameter(func.child_by_field_name("parameters")?)?;
        if implementation.child_by_field_name("type").is_some() {
            return None;
        }
        let declaration = func.parent()?;
        if declaration.kind() != "variable_declarator"
            || declaration.has_error()
            || declaration.child_by_field_name("value")? != func
            || declaration.child_by_field_name("name")?.kind() != "identifier"
        {
            return None;
        }
        let reference = declaration.child_by_field_name("type")?.named_child(0)?;
        let (signature, substitution) = if reference.kind() == "generic_type" {
            let (declaration, shape) =
                self.js_ts_local_type_definition(reference.child_by_field_name("name")?)?;
            let binder = single_child(declaration.child_by_field_name("type_parameters")?)?;
            let name = binder.child_by_field_name("name")?;
            let mut cursor = binder.walk();
            if binder.kind() != "type_parameter"
                || name.kind() != "type_identifier"
                || !is_plain_ident(self.node_text(&name))
                || binder
                    .children(&mut cursor)
                    .any(|n| n != name && n.kind() != "comment")
            {
                return None;
            }
            let argument = single_child(reference.child_by_field_name("type_arguments")?)?;
            (shape, Some((name, argument)))
        } else if reference.kind() == "function_type" {
            (reference, None)
        } else {
            let (declaration, shape) = self.js_ts_local_type_definition(reference)?;
            if declaration.child_by_field_name("type_parameters").is_some() {
                return None;
            }
            (shape, None)
        };
        // Object/interface bodies require a proven local declaration.
        // Exactly one callable member: overloads and extra slots are not proof.
        let signature = match signature.kind() {
            "function_type" => signature,
            "object_type" | "interface_body" => {
                single_child(signature).filter(|n| n.kind() == "call_signature")?
            }
            _ => return None,
        };
        if signature.child_by_field_name("type_parameters").is_some() {
            return None;
        }
        let contextual = single_parameter(signature.child_by_field_name("parameters")?)?;
        if contextual.child_by_field_name("pattern")?.kind() != "identifier" {
            return None;
        }
        let mut cursor = contextual.walk();
        if contextual
            .children(&mut cursor)
            .any(|n| matches!(n.kind(), "=" | "?"))
        {
            return None;
        }
        let ty = contextual.child_by_field_name("type")?.named_child(0)?;
        if let Some((binder, argument)) = substitution {
            // Only the entire parameter type can be substituted, not a nested
            // member/union/alias. The signature has no nearer generic binder.
            (ty.kind() == "type_identifier" && self.node_text(&ty) == self.node_text(&binder))
                .then_some(argument)
        } else {
            Some(ty)
        }
    }

    /// Preserve the original shape node: its class names belong to the
    /// declaration scope, while receiver writes belong to the implementation.
    /// One non-generic declaration hop: an object alias or private interface.
    fn js_ts_local_props_shape<'a>(&'a self, reference: Node<'a>) -> Option<Node<'a>> {
        if reference.kind() == "object_type" {
            return Some(reference);
        }
        let (declaration, shape) = self.js_ts_local_type_definition(reference)?;
        if declaration.child_by_field_name("type_parameters").is_some() {
            return None;
        }
        matches!(shape.kind(), "object_type" | "interface_body").then_some(shape)
    }

    /// Keep original shape nodes for aliases or private, heritage-free interfaces.
    /// Callable and props consumers separately prove their binders and body shape.
    fn js_ts_local_type_definition<'a>(
        &'a self,
        reference: Node<'a>,
    ) -> Option<(Node<'a>, Node<'a>)> {
        let declaration = self.js_ts_local_type_declaration(
            reference,
            &["type_alias_declaration", "interface_declaration"],
        )?;
        let shape = if declaration.kind() == "interface_declaration" {
            // Scripts and exported interfaces can merge beyond this file. Until
            // cross-module augmentation has source-backed proof, require private
            // ownership in a syntactically explicit module (not config inference).
            let root = self.tree.root_node();
            let mut cursor = root.walk();
            if declaration.parent()? != root
                || !root
                    .named_children(&mut cursor)
                    .any(|n| matches!(n.kind(), "import_statement" | "export_statement"))
            {
                return None;
            }
            let name = self.node_text(&reference);
            let mut cursor = root.walk();
            for export in root.named_children(&mut cursor).filter(|n| {
                n.kind() == "export_statement" && n.child_by_field_name("source").is_none()
            }) {
                if export
                    .child_by_field_name("value")
                    .is_some_and(|n| self.node_text(&n) == name)
                {
                    return None;
                }
                let mut cursor = export.walk();
                for clause in export
                    .named_children(&mut cursor)
                    .filter(|n| n.kind() == "export_clause")
                {
                    let mut cursor = clause.walk();
                    if clause.named_children(&mut cursor).any(|n| {
                        n.child_by_field_name("name")
                            .is_some_and(|n| self.node_text(&n) == name)
                    }) {
                        return None;
                    }
                }
            }
            let mut cursor = declaration.walk();
            if declaration
                .named_children(&mut cursor)
                .any(|n| n.kind() == "extends_type_clause")
            {
                return None;
            }
            declaration.child_by_field_name("body")?
        } else {
            declaration.child_by_field_name("value")?
        };
        Some((declaration, shape))
    }

    /// Declaration identity only. Each consumer separately proves its supported
    /// binders and shape; no chains, merging, ambient or imported authority.
    fn js_ts_local_type_declaration<'a>(
        &'a self,
        reference: Node<'a>,
        allowed_kinds: &[&str],
    ) -> Option<Node<'a>> {
        let root = self.tree.root_node();
        let name = self.node_text(&reference);
        if reference.kind() != "type_identifier"
            || !is_plain_ident(name)
            || root.has_error()
            || self.js_ts_type_name_shadowed(root, &reference, name)
            || self.js_ts_type_only_imports().contains_key(name)
            || self
                .extract_import_bindings()
                .iter()
                .any(|b| b.local == name)
        {
            return None;
        }
        let mut alias = None;
        let mut cursor = root.walk();
        for mut declaration in root.named_children(&mut cursor) {
            let mut ambient = false;
            while matches!(
                declaration.kind(),
                "export_statement" | "ambient_declaration"
            ) {
                ambient |= declaration.kind() == "ambient_declaration";
                declaration = declaration
                    .child_by_field_name("declaration")
                    .or_else(|| declaration.named_child(0))?;
            }
            if !matches!(
                declaration.kind(),
                "type_alias_declaration"
                    | "interface_declaration"
                    | "class_declaration"
                    | "abstract_class_declaration"
                    | "enum_declaration"
                    | "internal_module"
                    | "module"
                    | "import_alias"
            ) {
                continue;
            }
            let declared = declaration.child_by_field_name("name").or_else(|| {
                (declaration.kind() == "import_alias")
                    .then(|| declaration.named_child(0))
                    .flatten()
            });
            if !declared.is_some_and(|n| self.node_text(&n).split('.').next() == Some(name)) {
                continue;
            }
            if ambient
                || !allowed_kinds.contains(&declaration.kind())
                || alias.replace(declaration).is_some()
            {
                return None;
            }
        }
        alias
    }

    /// Required own properties in a direct, alias or private-interface shape; declaration
    /// type names and implementation bindings retain distinct source positions.
    fn js_ts_inline_prop_receiver_type(
        &self,
        parameter: Node<'_>,
        pattern: Node<'_>,
        ty: Node<'_>,
        receiver: &str,
    ) -> Option<String> {
        let selected = self.js_ts_prop_binding(parameter, pattern, receiver)?;
        let shape = self.js_ts_local_props_shape(ty)?;
        let annotation = self.js_ts_props_property_annotation(shape, &selected)?;
        self.js_ts_simple_type_annotation(annotation)
    }

    fn js_ts_prop_binding(
        &self,
        parameter: Node<'_>,
        pattern: Node<'_>,
        receiver: &str,
    ) -> Option<String> {
        if pattern.kind() != "object_pattern" || parameter.has_error() {
            return None;
        }
        let mut cursor = parameter.walk();
        if parameter
            .children(&mut cursor)
            .any(|n| matches!(n.kind(), "=" | "?"))
        {
            return None;
        }
        let mut properties = BTreeSet::new();
        let mut locals = BTreeSet::new();
        let mut selected = None;
        let mut cursor = pattern.walk();
        for child in pattern.named_children(&mut cursor) {
            if child.kind() == "comment" {
                continue;
            }
            let (property, local) = match child.kind() {
                "shorthand_property_identifier_pattern" => (child, child),
                "pair_pattern" => {
                    let property = child.child_by_field_name("key")?;
                    let local = child.child_by_field_name("value")?;
                    if property.kind() != "property_identifier" || local.kind() != "identifier" {
                        return None;
                    }
                    (property, local)
                }
                _ => return None,
            };
            let property = self.node_text(&property);
            let local = self.node_text(&local);
            if !properties.insert(property) || !locals.insert(local) {
                return None;
            }
            if local == receiver {
                selected = Some(property);
            }
        }
        selected.map(str::to_string)
    }

    fn js_ts_props_property_annotation<'a>(
        &self,
        ty: Node<'a>,
        selected: &str,
    ) -> Option<Node<'a>> {
        let mut names = BTreeSet::new();
        let mut result = None;
        let mut cursor = ty.walk();
        for property in ty.named_children(&mut cursor) {
            if property.kind() == "comment" {
                continue;
            }
            if property.kind() != "property_signature" {
                return None;
            }
            let name = property.child_by_field_name("name")?;
            if name.kind() != "property_identifier" {
                return None;
            }
            let name = self.node_text(&name);
            if !names.insert(name) {
                return None;
            }
            if name == selected {
                let mut cursor = property.walk();
                if property.children(&mut cursor).any(|n| n.kind() == "?") {
                    return None;
                }
                result = property.child_by_field_name("type");
            }
        }
        result
    }

    /// Only an inline named export of one object alias is a foreign shape.
    pub(crate) fn js_ts_exported_props_property(
        &self,
        name: &str,
        property: &str,
    ) -> Option<(String, (usize, usize), (usize, usize))> {
        let root = self.tree.root_node();
        let mut selected = None;
        let mut cursor = root.walk();
        for export in root.named_children(&mut cursor) {
            if export.kind() != "export_statement" {
                continue;
            }
            // This slice has no barrel or export-clause authority, including
            // a second route that could collide with the direct alias name.
            if export.child_by_field_name("source").is_some() {
                return None;
            }
            let mut export_cursor = export.walk();
            if export
                .named_children(&mut export_cursor)
                .any(|n| n.kind() == "export_clause")
            {
                return None;
            }
            let Some(declaration) = export.child_by_field_name("declaration") else {
                continue;
            };
            if declaration.kind() != "type_alias_declaration" {
                continue;
            }
            let reference = declaration.child_by_field_name("name")?;
            if self.node_text(&reference) != name {
                continue;
            }
            // Also reject duplicates and competing type declarations/imports.
            if self.js_ts_local_type_declaration(reference, &["type_alias_declaration"])?
                != declaration
                || declaration.child_by_field_name("type_parameters").is_some()
            {
                return None;
            }
            let shape = declaration.child_by_field_name("value")?;
            if shape.kind() != "object_type" {
                return None;
            }
            let annotation = self.js_ts_props_property_annotation(shape, property)?;
            let class_name = self.js_ts_simple_type_annotation(annotation)?;
            if selected
                .replace((
                    class_name,
                    (declaration.start_byte(), declaration.end_byte()),
                    (annotation.start_byte(), annotation.end_byte()),
                ))
                .is_some()
            {
                return None;
            }
        }
        selected
    }

    fn js_ts_simple_type_annotation(&self, annotation: Node<'_>) -> Option<String> {
        let ty = annotation.named_child(0)?;
        if ty.kind() != "type_identifier" {
            return None;
        }
        let text = self.node_text(&ty);
        (is_plain_ident(text) && !self.js_ts_type_name_shadowed(self.tree.root_node(), &ty, text))
            .then(|| text.to_string())
    }

    /// Value-binding absence does not prove a TypeScript type name. Only the
    /// module class is supported; nearer type declarations and generics fence it.
    fn js_ts_type_name_shadowed(&self, node: Node<'_>, at: &Node<'_>, name: &str) -> bool {
        if node.kind() == "class"
            && node
                .child_by_field_name("name")
                .is_some_and(|n| self.node_text(&n) == name)
            && node.start_byte() <= at.start_byte()
            && at.end_byte() <= node.end_byte()
        {
            return true;
        }
        if matches!(node.kind(), "type_parameter")
            && node
                .child_by_field_name("name")
                .is_some_and(|n| self.node_text(&n) == name)
            && node.parent().and_then(|n| n.parent()).is_some_and(|scope| {
                scope.start_byte() <= at.start_byte() && at.end_byte() <= scope.end_byte()
            })
        {
            return true;
        }
        if matches!(
            node.kind(),
            "class_declaration"
                | "abstract_class_declaration"
                | "interface_declaration"
                | "type_alias_declaration"
                | "enum_declaration"
                | "internal_module"
                | "module"
                | "import_alias"
        ) && node
            .child_by_field_name("name")
            .is_some_and(|n| self.node_text(&n) == name)
        {
            if let Some(scope) =
                self.js_ts_nearest_lexical_scope_id(&node, self.tree.root_node().id())
            {
                // Module declarations are checked by clean_class_spans at
                // resolution; retain their existing unsupported-type metadata.
                if scope != self.tree.root_node().id()
                    && self.js_ts_scope_distance(at, scope).is_some()
                {
                    return true;
                }
            }
        }
        let mut cursor = node.walk();
        let found = node
            .named_children(&mut cursor)
            .any(|child| self.js_ts_type_name_shadowed(child, at, name));
        found
    }
}
