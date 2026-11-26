use super::*;
use crate::ast::{JsBinding, JsFollow};
use crate::call_graph::ImportBindingKind;
use param_slot::Walk;

pub(super) enum Terminal<'a> {
    Callable(FunctionId, (usize, usize)),
    Parameter(&'a str, Parameter<'a>),
}

impl<'a> Context<'a> {
    pub(super) fn index_escapes(&mut self, candidates: &[Candidate<'a>]) {
        let mut names: BTreeMap<&str, BTreeSet<&str>> = BTreeMap::new();
        let holders: BTreeSet<_> = candidates
            .iter()
            .map(|candidate| &candidate.slot.holder)
            .collect();
        for holder in &holders {
            names.entry(&holder.file).or_default().insert(&holder.name);
        }
        for (file, names) in names {
            let parsed = &self.files[file];
            let mut cursor = parsed.tree.walk();
            loop {
                let node = cursor.node();
                if matches!(node.kind(), "identifier" | "shorthand_property_identifier")
                    && names.contains(parsed.node_text(&node))
                    && !parsed.js_ts_identifier_is_not_a_read(node)
                    && !invoked(node)
                {
                    if let Ok((owner, _)) = self.callable_at(file, node, false) {
                        if holders.contains(&owner) {
                            self.escapes.insert(owner);
                        }
                    }
                }
                if cursor.goto_first_child() {
                    continue;
                }
                while !cursor.goto_next_sibling() {
                    if !cursor.goto_parent() {
                        break;
                    }
                }
                if cursor.node() == parsed.tree.root_node() {
                    break;
                }
            }
        }
    }

    pub(super) fn index_owners(&mut self) {
        for file in self.bindings.keys().copied() {
            let parsed = &self.files[file];
            for node in crate::anon_owner::candidate_nodes(parsed, true) {
                if let Some(id) = crate::anon_owner::identity(parsed, &node, file) {
                    self.owners.entry(id).or_default().push(node);
                }
            }
        }
        for export in self
            .cg
            .js_ts_resolved_exports
            .values()
            .flat_map(|exports| exports.values())
        {
            if let Some((start_line, end_line)) = export.span {
                self.exports.insert(FunctionId {
                    file: export.file.clone(),
                    name: export.local_name.clone(),
                    start_line,
                    end_line,
                });
            }
        }
    }

    pub(super) fn index_callers(&mut self) {
        for call in self.cg.calls.values().flatten() {
            let file = call.caller.file.as_str();
            if !self.bindings.contains_key(file) {
                continue;
            }
            let hint = match &call.local_binding {
                JsLocalBinding::Callable(terminal) | JsLocalBinding::AliasCallable(terminal) => {
                    Some(terminal.local.as_str())
                }
                JsLocalBinding::AliasImport { local } => Some(local.as_str()),
                _ => None,
            };
            let name = hint.unwrap_or(&call.callee_name);
            let names = self.cg.functions.get(name);
            let imported = self
                .cg
                .import_bindings
                .get(file)
                .is_some_and(|bindings| bindings.iter().any(|binding| binding.local == name));
            #[cfg(test)]
            let supplied = self.supplied_call(call).is_some();
            #[cfg(not(test))]
            let supplied = false;
            if names.is_none() && !imported && !supplied {
                continue;
            }
            let parsed = &self.files[file];
            let lexical = call_node(parsed, call)
                .and_then(|node| self.holder_at(node, call).ok())
                .map(|(holder, _)| holder);
            let resolved = self.resolve_caller(call);
            let mut possible: BTreeSet<&FunctionId> = lexical.iter().collect();
            possible.extend(resolved.resolved.iter().map(|target| target.target));
            if possible.is_empty() {
                possible.extend(names.into_iter().flatten().filter(|id| id.file == file));
            }
            for holder in possible {
                if self.owners.contains_key(holder) {
                    self.callers.entry(holder.clone()).or_default().push(call);
                }
            }
        }
    }

    #[cfg(test)]
    pub(super) fn supplied_constructor(
        &self,
        file: &str,
        callee: Node<'a>,
    ) -> Option<(FunctionId, (usize, usize))> {
        let parsed = &self.files[file];
        let name = param_slot::name(parsed, callee)?;
        let mut cache = self.bindings[file].borrow_mut();
        let (scope, class) = parsed.js_ts_site_declaration(callee, &name, &mut cache)?;
        if class.kind() != "class_declaration"
            || class.end_byte() > callee.start_byte()
            || parsed.js_ts_scoped_written(scope, &name, &mut cache)
            || dynamic_for(parsed, scope, &name, &mut cache)
        {
            return None;
        }
        let body = class.child_by_field_name("body")?;
        let mut constructors = param_slot::children(body).filter(|method| {
            method.kind() == "method_definition"
                && method.child_by_field_name("name").is_some_and(|name| {
                    parsed.node_text(&name) == "constructor"
                        && !method
                            .children(&mut method.walk())
                            .any(|child| child.kind() == "static")
                })
        });
        let constructor = constructors
            .next()
            .filter(|_| constructors.next().is_none())?;
        self.witnessed(file, constructor).ok()
    }

    pub(super) fn unique_owner(&self, id: &FunctionId, node: Node<'_>) -> bool {
        matches!(self.owners.get(id).map(Vec::as_slice), Some([owner]) if *owner == node)
    }

    fn witnessed(
        &self,
        file: &str,
        node: Node<'a>,
    ) -> Result<(FunctionId, (usize, usize)), &'static str> {
        let parsed = &self.files[file];
        let id = crate::anon_owner::identity(parsed, &node, file).ok_or("stale_terminal")?;
        if !self.unique_owner(&id, node) {
            return Err("callable_collision");
        }
        Ok((id, bytes(node)))
    }

    pub(super) fn terminal(&self, value: value::Value<'a>) -> Result<Terminal<'a>, &'static str> {
        let value::Value::Node(file, node) = value else {
            return Err("opaque_terminal");
        };
        let parsed = &self.files[file];
        let node = param_slot::peel(node).ok_or("opaque_terminal")?;
        if param_slot::name(parsed, node).is_some() {
            if let Some(parameter) =
                param_slot::parameter(parsed, node, &mut self.bindings[file].borrow_mut())
            {
                return Ok(Terminal::Parameter(file, parameter));
            }
        }
        let (target, bytes) = self.callable_at(file, node, false)?;
        Ok(Terminal::Callable(target, bytes))
    }

    pub(super) fn callable_at(
        &self,
        file: &str,
        node: Node<'a>,
        jsx: bool,
    ) -> Result<(FunctionId, (usize, usize)), &'static str> {
        let node = param_slot::peel(node).ok_or("opaque_terminal")?;
        if matches!(
            node.kind(),
            "arrow_function" | "function_expression" | "generator_function" | "method_definition"
        ) {
            return self.witnessed(file, node);
        }
        let parsed = &self.files[file];
        let mut cache = self.bindings[file].borrow_mut();
        if let Some(name) = param_slot::name(parsed, node) {
            return self.callable_identifier(file, node, name, jsx, &mut cache);
        }
        if node.kind() == "member_expression" {
            let qualifier = node
                .child_by_field_name("object")
                .and_then(param_slot::peel)
                .ok_or("opaque_terminal")?;
            let member = node
                .child_by_field_name("property")
                .and_then(|n| param_slot::property_key(parsed, n))
                .ok_or("opaque_terminal")?;
            if let Some((JsBinding::Import, Some(module))) =
                parsed.js_ts_namespace_binding(qualifier, &mut cache)
            {
                return self.imported(file, &module, &member, jsx);
            }
        }
        Err("opaque_terminal")
    }

    fn callable_identifier(
        &self,
        file: &str,
        mut node: Node<'a>,
        mut name: String,
        jsx: bool,
        cache: &mut JsBindingCache<'a>,
    ) -> Result<(FunctionId, (usize, usize)), &'static str> {
        let parsed = &self.files[file];
        let followed = parsed.js_ts_follow_alias(node, &name, cache);
        for _ in 0..=8 {
            let binding = parsed.js_ts_site_binding(node, &name, cache);
            let (scope, declaration) = match parsed.js_ts_site_declaration(node, &name, cache) {
                Some(declaration) => declaration,
                None if matches!(binding, JsBinding::Refused("unbound")) => {
                    return Err("opaque_terminal")
                }
                None => return Err("stale_terminal"),
            };
            if dynamic_for(parsed, scope, &name, cache) {
                return Err("stale_terminal");
            }
            match binding {
                JsBinding::Callable(terminal) => {
                    let function = match declaration.kind() {
                        "function_declaration" | "generator_function_declaration" => declaration,
                        "variable_declarator"
                            if param_slot::is_const(declaration)
                                && initialized_at(parsed, declaration, node) =>
                        {
                            let initializer = declaration
                                .child_by_field_name("value")
                                .and_then(param_slot::peel)
                                .ok_or("stale_terminal")?;
                            if initializer.kind() == "call_expression" {
                                // The Callable classification already proved React import
                                // provenance. Its wrapped flag distinguishes memo and forwardRef
                                // render functions from the useCallback passthrough.
                                if terminal.wrapped && !jsx {
                                    return Err("opaque_terminal");
                                }
                                initializer
                                    .child_by_field_name("arguments")
                                    .and_then(|arguments| param_slot::children(arguments).next())
                                    .and_then(param_slot::peel)
                                    .ok_or("stale_terminal")?
                            } else {
                                initializer
                            }
                        }
                        _ => return Err("stale_terminal"),
                    };
                    if !matches!(
                        function.kind(),
                        "function_declaration"
                            | "generator_function_declaration"
                            | "arrow_function"
                            | "function_expression"
                            | "generator_function"
                    ) || parsed.node_line_range(&function)
                        != (terminal.start_line, terminal.end_line)
                    {
                        return Err("stale_terminal");
                    }
                    return self.witnessed(file, function);
                }
                JsBinding::Import if scope.kind() == "program" => {
                    let mut imports = self
                        .cg
                        .import_bindings
                        .get(file)
                        .into_iter()
                        .flatten()
                        .filter(|binding| {
                            binding.local == name
                                && binding.eligible
                                && binding.kind == ImportBindingKind::MemberImport
                        });
                    let binding = imports
                        .next()
                        .filter(|_| imports.next().is_none())
                        .ok_or("stale_terminal")?;
                    return self.imported(
                        file,
                        &binding.module_path,
                        binding.member.as_deref().ok_or("stale_terminal")?,
                        jsx,
                    );
                }
                JsBinding::Alias
                    if followed.is_some()
                        && param_slot::is_const(declaration)
                        && initialized_at(parsed, declaration, node) =>
                {
                    let initializer = declaration
                        .child_by_field_name("value")
                        .and_then(param_slot::peel)
                        .ok_or("stale_terminal")?;
                    if let Some(next) = param_slot::name(parsed, initializer) {
                        node = initializer;
                        name = next;
                        continue;
                    }
                    if let Some(JsFollow::NamespaceMember {
                        module_path,
                        member,
                    }) = followed.as_ref()
                    {
                        return self.imported(file, module_path, member, jsx);
                    }
                    return Err("opaque_terminal");
                }
                JsBinding::Alias | JsBinding::MayCall | JsBinding::Unchecked(_) => {
                    return Err("stale_terminal")
                }
                _ => return Err("opaque_terminal"),
            }
        }
        Err("stale_terminal")
    }

    fn imported(
        &self,
        file: &str,
        module: &str,
        member: &str,
        jsx: bool,
    ) -> Result<(FunctionId, (usize, usize)), &'static str> {
        let module_file = self
            .cg
            .resolve_js_ts_member_module(module, file)
            .ok_or("stale_terminal")?;
        let exports = if module.trim().starts_with('.') {
            &self.cg.js_ts_resolved_exports
        } else {
            let (_, project) = self
                .cg
                .js_ts_path_modules
                .get(&(file.into(), module.into()))
                .ok_or("stale_terminal")?;
            self.cg
                .js_ts_path_exports
                .get(project)
                .ok_or("stale_terminal")?
        };
        let export = exports
            .get(&module_file)
            .and_then(|exports| exports.get(member))
            .ok_or("stale_terminal")?;
        if (export.wrapped && !jsx) || export.via_unresolved_star {
            return Err("stale_terminal");
        }
        let (start_line, end_line) = export.span.ok_or("stale_terminal")?;
        let id = FunctionId {
            file: export.file.clone(),
            name: export.local_name.clone(),
            start_line,
            end_line,
        };
        let [node] = self
            .owners
            .get(&id)
            .map(Vec::as_slice)
            .ok_or("stale_terminal")?
        else {
            return Err("callable_collision");
        };
        let parsed = &self.files[&id.file];
        if let Some(binding) = node.child_by_field_name("name") {
            let name = parsed.node_text(&binding);
            let mut cache = JsBindingCache::for_sites();
            let scope = parsed.tree.root_node();
            if parsed.js_ts_scoped_written(scope, name, &mut cache)
                || dynamic_for(parsed, scope, name, &mut cache)
            {
                return Err("stale_terminal");
            }
        }
        Ok((id, bytes(*node)))
    }
}

fn invoked(mut node: Node<'_>) -> bool {
    while let Some(parent) = node.parent() {
        if param_slot::peel(parent) == Some(node) {
            node = parent;
            continue;
        }
        return match parent.kind() {
            "call_expression" => parent.child_by_field_name("function") == Some(node),
            "new_expression" => parent.child_by_field_name("constructor") == Some(node),
            "jsx_opening_element" | "jsx_self_closing_element" | "jsx_closing_element" => {
                parent.child_by_field_name("name") == Some(node)
            }
            _ => false,
        };
    }
    false
}

pub(super) fn initialized_at(
    parsed: &ParsedFile,
    declaration: Node<'_>,
    mut read: Node<'_>,
) -> bool {
    if declaration.end_byte() > read.start_byte() {
        return false;
    }
    let Some(mut parent) = declaration.parent().and_then(|list| list.parent()) else {
        return false;
    };
    if parent.kind() == "export_statement" {
        let Some(program) = parent.parent() else {
            return false;
        };
        parent = program;
    }
    if !matches!(parent.kind(), "program" | "statement_block") {
        return false;
    }
    let boundaries = parsed.language.callable_boundary_node_types();
    while let Some(enclosing) = read.parent() {
        if enclosing == parent {
            return read.start_byte() >= declaration.end_byte();
        }
        if boundaries.contains(&enclosing.kind()) {
            return false;
        }
        read = enclosing;
    }
    false
}

pub(super) fn dynamic_for<'a>(
    parsed: &'a ParsedFile,
    scope: Node<'a>,
    name: &str,
    cache: &mut JsBindingCache<'a>,
) -> bool {
    if !parsed.source[scope.byte_range()].contains("eval")
        && !parsed.source[scope.byte_range()].contains('\\')
    {
        return false;
    }
    let bound_to_scope = |candidate: Node<'a>| {
        candidate == scope
            || candidate.kind() == "formal_parameters" && candidate.parent() == Some(scope)
            || scope.kind() == "formal_parameters" && scope.parent() == Some(candidate)
    };
    let mut stack = vec![scope];
    while let Some(node) = stack.pop() {
        if node.kind() == "call_expression" {
            let mut cursor = node.walk();
            let optional = node
                .children(&mut cursor)
                .any(|part| matches!(part.kind(), "optional_chain" | "?."));
            if !optional {
                if let Some(callee) = node
                    .child_by_field_name("function")
                    .and_then(param_slot::peel)
                {
                    if param_slot::name(parsed, callee).as_deref() == Some("eval")
                        && matches!(
                            parsed.js_ts_binding_walk(callee, "eval", cache),
                            Walk::Unbound | Walk::Found(_, Some(_))
                        )
                        && matches!(parsed.js_ts_binding_walk(callee, name, cache), Walk::Found(binding_scope, _) if bound_to_scope(binding_scope) || binding_scope.kind() == "with_statement")
                        && eval_may_write(parsed, node, callee, name, cache)
                    {
                        return true;
                    }
                }
            }
        }
        stack.extend(param_slot::children(node));
    }
    false
}

fn eval_may_write<'a>(
    parsed: &'a ParsedFile,
    call: Node<'a>,
    callee: Node<'a>,
    name: &str,
    cache: &mut JsBindingCache<'a>,
) -> bool {
    let Some(argument) = call
        .child_by_field_name("arguments")
        .and_then(|args| param_slot::children(args).next())
    else {
        return false;
    };
    if argument.kind() != "string" {
        return !matches!(
            argument.kind(),
            "null" | "number" | "true" | "false" | "object" | "array"
        );
    }
    let source = parsed.node_text(&argument);
    let Some((left, right)) = source
        .get(1..source.len() - 1)
        .and_then(|source| source.trim().trim_end_matches(';').split_once('='))
    else {
        return true;
    };
    // For a single unescaped identifier assignment we can distinguish a
    // write to another lexical binding from arbitrary executed source.
    let left = left.trim();
    let right = right.trim();
    if left == name
        || ![left, right].iter().all(|text| {
            !text.contains('\\') && ParsedFile::js_ts_decoded_identifier(text).is_some()
        })
    {
        return true;
    }
    !matches!(parsed.js_ts_binding_walk(callee, left, cache), Walk::Found(scope, None) if scope.kind() != "program")
        || !matches!(
            parsed.js_ts_site_binding(callee, right, cache),
            JsBinding::Callable(_)
        )
}
