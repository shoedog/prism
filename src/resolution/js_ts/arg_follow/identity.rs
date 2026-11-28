use super::*;
use crate::ast::{JsBinding, JsFollow};
use crate::call_graph::ImportBindingKind;
use param_slot::Walk;

pub(super) enum Terminal<'a> {
    Callable(FunctionId, (usize, usize)),
    Parameter(&'a str, Parameter<'a>),
}

enum CallableReference {
    Local(FunctionId, (usize, usize)),
    Import { module: String, member: String },
}

pub(super) type EvalSites<'a> = BTreeMap<usize, Vec<(Node<'a>, Node<'a>)>>;

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
        let files = self.files;
        let owners: Vec<_> = self
            .bindings
            .keys()
            .copied()
            .collect::<Vec<_>>()
            .into_par_iter()
            .map(|file| {
                let parsed = &files[file];
                crate::anon_owner::candidate_nodes(parsed, true)
                    .into_iter()
                    .filter_map(|node| {
                        crate::anon_owner::identity(parsed, &node, file).map(|id| (id, node))
                    })
                    .collect::<Vec<_>>()
            })
            .collect();
        for (id, node) in owners.into_iter().flatten() {
            self.owners.entry(id).or_default().push(node);
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
        let context: &Self = self;
        // Indexed collection preserves the graph's caller order for each holder.
        let calls: Vec<_> = self.cg.calls.iter().collect();
        let batches: Vec<_> = calls
            .chunk_by(|(left, _), (right, _)| left.file == right.file)
            .collect::<Vec<_>>()
            .into_par_iter()
            .map(|owners| {
                let mut batch = Vec::new();
                for (_, sites) in owners {
                    for call in *sites {
                        context.append_caller_holders(call, &mut batch);
                    }
                }
                batch
            })
            .collect();
        for (holder, call) in batches.into_iter().flatten() {
            self.callers.entry(holder).or_default().push(call);
        }
    }

    fn append_caller_holders(
        &self,
        call: &'a CallSite,
        batch: &mut Vec<(FunctionId, &'a CallSite)>,
    ) {
        let file = call.caller.file.as_str();
        if !self.bindings.contains_key(file) {
            return;
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
            return;
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
        batch.extend(
            possible
                .into_iter()
                .filter(|holder| self.owners.contains_key(*holder))
                .map(|holder| (holder.clone(), call)),
        );
    }

    #[cfg(test)]
    pub(super) fn supplied_constructor(
        &self,
        file: &str,
        callee: Node<'a>,
    ) -> Option<(FunctionId, (usize, usize))> {
        let parsed = &self.files[file];
        let name = param_slot::name(parsed, callee)?;
        let mut cache = self.bindings[file].cache();
        let (scope, class) = parsed.js_ts_site_declaration(callee, &name, &mut cache)?;
        if class.kind() != "class_declaration"
            || class.end_byte() > callee.start_byte()
            || parsed.js_ts_scoped_written(scope, &name, &mut cache)
            || self.dynamic_for(file, scope, &name, &mut cache)
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
                param_slot::parameter(parsed, node, &mut self.bindings[file].cache())
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
        let mut cache = self.bindings[file].cache();
        let reference = self.callable_reference(file, node, jsx, &mut cache)?;
        // An import may point to this file or be part of a cycle. Hold one file at a time.
        drop(cache);
        match reference {
            CallableReference::Local(id, span) => Ok((id, span)),
            CallableReference::Import { module, member } => {
                self.imported(file, &module, &member, jsx)
            }
        }
    }

    fn callable_reference(
        &self,
        file: &str,
        node: Node<'a>,
        jsx: bool,
        cache: &mut JsBindingCache<'a>,
    ) -> Result<CallableReference, &'static str> {
        let parsed = &self.files[file];
        if let Some(name) = param_slot::name(parsed, node) {
            return self.callable_identifier(file, node, name, jsx, cache);
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
                parsed.js_ts_namespace_binding(qualifier, cache)
            {
                return Ok(CallableReference::Import { module, member });
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
    ) -> Result<CallableReference, &'static str> {
        let parsed = &self.files[file];
        let mut followed = None;
        for _ in 0..=8 {
            let (binding, declaration) =
                parsed.js_ts_site_binding_and_declaration(node, &name, cache);
            let (scope, declaration) = match declaration {
                Some(declaration) => declaration,
                None if matches!(binding, JsBinding::Refused("unbound")) => {
                    return Err("opaque_terminal")
                }
                None => return Err("stale_terminal"),
            };
            if self.dynamic_for(file, scope, &name, cache) {
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
                    let (id, span) = self.witnessed(file, function)?;
                    return Ok(CallableReference::Local(id, span));
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
                    return Ok(CallableReference::Import {
                        module: binding.module_path.clone(),
                        member: binding.member.clone().ok_or("stale_terminal")?,
                    });
                }
                JsBinding::Alias => {
                    if !param_slot::is_const(declaration)
                        || !initialized_at(parsed, declaration, node)
                    {
                        return Err("stale_terminal");
                    }
                    // Keep the proof from the original occurrence for every alias link.
                    let Some(followed) = followed
                        .get_or_insert_with(|| parsed.js_ts_follow_alias(node, &name, cache))
                    else {
                        return Err("stale_terminal");
                    };
                    let initializer = declaration
                        .child_by_field_name("value")
                        .and_then(param_slot::peel)
                        .ok_or("stale_terminal")?;
                    if let Some(next) = param_slot::name(parsed, initializer) {
                        node = initializer;
                        name = next;
                        continue;
                    }
                    if let JsFollow::NamespaceMember {
                        module_path,
                        member,
                    } = followed
                    {
                        return Ok(CallableReference::Import {
                            module: module_path.clone(),
                            member: member.clone(),
                        });
                    }
                    return Err("opaque_terminal");
                }
                JsBinding::MayCall | JsBinding::Unchecked(_) => return Err("stale_terminal"),
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
            let mut cache = self.bindings[id.file.as_str()].cache();
            let scope = parsed.tree.root_node();
            if parsed.js_ts_scoped_written(scope, name, &mut cache)
                || self.dynamic_for(&id.file, scope, name, &mut cache)
            {
                return Err("stale_terminal");
            }
        }
        Ok((id, bytes(*node)))
    }

    pub(super) fn dynamic_for(
        &self,
        file: &str,
        scope: Node<'a>,
        name: &str,
        cache: &mut JsBindingCache<'a>,
    ) -> bool {
        let parsed = &self.files[file];
        let sites = self.bindings[file]
            .eval_sites
            .get_or_init(|| index_eval_sites(parsed));
        let bound_to_scope = |candidate: Node<'a>| {
            candidate == scope
                || candidate.kind() == "formal_parameters" && candidate.parent() == Some(scope)
                || scope.kind() == "formal_parameters" && scope.parent() == Some(candidate)
        };
        sites
            .get(&scope.id())
            .into_iter()
            .flatten()
            .any(|&(call, callee)| {
                matches!(
                    parsed.js_ts_binding_walk(callee, "eval", cache),
                    Walk::Unbound | Walk::Found(_, Some(_))
                ) && matches!(
                    parsed.js_ts_binding_walk(callee, name, cache),
                    Walk::Found(binding_scope, _)
                        if bound_to_scope(binding_scope) || binding_scope.kind() == "with_statement"
                ) && eval_may_write(parsed, call, callee, name, cache)
            })
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

pub(super) fn index_eval_sites(parsed: &ParsedFile) -> EvalSites<'_> {
    let mut sites = EvalSites::new();
    if !parsed.source.contains("eval") && !parsed.source.contains('\\') {
        return sites;
    }
    let mut stack = vec![(parsed.tree.root_node(), 0)];
    let mut ancestors = Vec::new();
    while let Some((node, depth)) = stack.pop() {
        ancestors.truncate(depth);
        ancestors.push(node.id());
        if let Some(callee) = direct_eval_callee(parsed, node) {
            // Match the original scope walk's node set and reverse child order.
            for &ancestor in &ancestors {
                sites.entry(ancestor).or_default().push((node, callee));
            }
        }
        stack.extend(
            node.named_children(&mut node.walk())
                .filter(|child| !child.is_extra())
                .map(|child| (child, depth + 1)),
        );
    }
    sites
}

fn direct_eval_callee<'a>(parsed: &ParsedFile, node: Node<'a>) -> Option<Node<'a>> {
    if node.kind() != "call_expression" {
        return None;
    }
    let callee = node
        .child_by_field_name("function")
        .and_then(param_slot::peel)?;
    if param_slot::name(parsed, callee).as_deref() != Some("eval")
        || node
            .children(&mut node.walk())
            .any(|part| matches!(part.kind(), "optional_chain" | "?."))
    {
        return None;
    }
    Some(callee)
}

pub(super) fn eval_may_write<'a>(
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::languages::Language;

    #[test]
    fn indexes_escaped_direct_eval_by_ancestry() {
        let source =
            r"function holder() { (ev\u0061l)('inside'); eval?.('optional'); } eval('outside');";
        let parsed = ParsedFile::parse("m.js", source, Language::JavaScript).unwrap();
        assert_eq!(parsed.parse_error_count, 0);
        let root = parsed.tree.root_node();
        let holder = root.named_child(0).unwrap();
        let sites = index_eval_sites(&parsed);
        let calls = |scope: Node<'_>| {
            sites[&scope.id()]
                .iter()
                .map(|(call, _)| parsed.node_text(call))
                .collect::<Vec<_>>()
        };
        assert_eq!(calls(root), ["eval('outside')", r"(ev\u0061l)('inside')"]);
        assert_eq!(calls(holder), [r"(ev\u0061l)('inside')"]);
    }
}
