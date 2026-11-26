//! Lexical declarations, hoisting and the body environment for parameter defaults.
use super::*;
use std::borrow::Cow;

#[derive(Default)]
pub(super) struct Scope<'a> {
    pub(super) parent: Option<usize>,
    pub(super) names: BTreeMap<Cow<'a, str>, Option<AstSpan>>,
    pub(super) entry: Vec<AstSpan>,
    shared_vars: BTreeSet<Cow<'a, str>>,
    pub(super) parameter_copies: Vec<(usize, AstSpan)>,
    dynamic: bool,
    implicit_arguments: bool,
}

pub(super) struct Lexical<'a> {
    file: &'a ParsedFile,
    pub(super) module_syntax: bool,
    pub(super) scopes: BTreeMap<usize, Scope<'a>>,
    pub(super) binding_owners: BTreeMap<usize, AstSpan>,
    pub(super) binding_owner_ids: BTreeMap<usize, usize>,
    pub(super) hoisted_vars: BTreeSet<usize>,
    pub(super) escaped_names: BTreeSet<String>,
    decoded: BTreeMap<&'a str, String>,
    pub(super) binding_scopes: BTreeMap<usize, usize>,
    pub(super) eval_functions: BTreeSet<usize>,
    pub(super) namespaces: BTreeSet<usize>,
}

impl<'a> Lexical<'a> {
    pub(super) fn build(file: &'a ParsedFile) -> Self {
        let root = file.tree.root_node();
        let mut lexical = Self {
            file,
            module_syntax: children(root)
                .any(|child| matches!(child.kind(), "import_statement" | "export_statement")),
            scopes: BTreeMap::new(),
            binding_owners: BTreeMap::new(),
            binding_owner_ids: BTreeMap::new(),
            hoisted_vars: BTreeSet::new(),
            escaped_names: BTreeSet::new(),
            decoded: BTreeMap::new(),
            binding_scopes: BTreeMap::new(),
            eval_functions: BTreeSet::new(),
            namespaces: BTreeSet::new(),
        };
        let mut pending = vec![root];
        while let Some(node) = pending.pop() {
            if identifier(node) && file.source.as_bytes()[node.byte_range()].contains(&b'\\') {
                let raw = file.node_text(&node);
                if !lexical.decoded.contains_key(raw) {
                    let name = ParsedFile::js_ts_decode_identifier_text(raw)
                        .unwrap_or_else(|| raw.to_owned());
                    lexical.escaped_names.insert(name.clone());
                    lexical.decoded.insert(raw, name);
                }
            }
            pending.extend(children(node));
        }
        lexical.collect(
            root,
            Context {
                scope: root,
                owner: root,
            },
            root,
        );
        lexical
    }
    fn declare(
        &mut self,
        pattern: Node<'a>,
        scope: Node<'a>,
        owner: Node<'a>,
        entry: bool,
        shared_var: bool,
    ) {
        if identifier(pattern) || pattern.kind() == "type_identifier" {
            let span = AstSpan::from(pattern);
            let raw = self.file.node_text(&pattern);
            let name = self
                .decoded
                .get(raw)
                .map_or(Cow::Borrowed(raw), |name| Cow::Owned(name.clone()));
            let scope_id = scope.id();
            let scope = self.scopes.entry(scope.id()).or_default();
            if let Some(binding) = scope.names.get_mut(name.as_ref()) {
                if !shared_var || !scope.shared_vars.contains(name.as_ref()) {
                    *binding = None;
                }
                return;
            }
            scope.names.insert(name.clone(), Some(span.clone()));
            if shared_var {
                scope.shared_vars.insert(name);
            }
            self.binding_scopes.insert(span.start_byte, scope_id);
            self.binding_owners
                .insert(span.start_byte, AstSpan::from(owner));
            self.binding_owner_ids.insert(span.start_byte, owner.id());
            if entry {
                scope.entry.push(span);
            }
        } else {
            for part in pattern_parts(pattern) {
                self.declare(part, scope, owner, entry, shared_var);
            }
        }
    }

    fn collect(&mut self, node: Node<'a>, parent: Context<'a>, mut hoist: Node<'a>) {
        let parent_hoist = hoist;
        let mut context = parent;
        if is_scope(node) {
            self.scopes.insert(
                node.id(),
                Scope {
                    parent: (node != parent.scope).then_some(parent.scope.id()),
                    dynamic: node.kind() == "with_statement",
                    implicit_arguments: is_owner(node)
                        && !matches!(node.kind(), "program" | "arrow_function"),
                    ..Scope::default()
                },
            );
            context.scope = node;
        }
        if is_owner(node) {
            context.owner = node;
            hoist = node;
        }
        let has_body_environment = node.kind() == "statement_block"
            && field(context.owner, "body") == Some(node)
            && parameter_expressions(context.owner);
        if node.kind() == "class_static_block" || has_body_environment {
            hoist = node;
        }
        match node.kind() {
            "variable_declarator" => {
                if let Some(name) = field(node, "name") {
                    let hoisted = node
                        .parent()
                        .is_some_and(|n| n.kind() == "variable_declaration");
                    self.declare(
                        name,
                        if hoisted { hoist } else { context.scope },
                        context.owner,
                        false,
                        hoisted,
                    );
                    if hoisted {
                        self.mark_hoisted(name, hoist);
                    }
                }
            }
            "function_declaration"
            | "generator_function_declaration"
            | "class_declaration"
            | "abstract_class_declaration" => {
                if let Some(name) = field(node, "name") {
                    self.declare(name, parent.scope, parent.owner, is_owner(node), false);
                    if node.kind() == "function_declaration"
                        && self.canonical(self.file.node_text(&name)) == "eval"
                    {
                        self.eval_functions.insert(name.start_byte());
                    }
                    if !is_owner(node) {
                        self.declare(name, node, context.owner, false, false);
                    }
                }
            }
            "function_expression" | "generator_function" | "class" => {
                if let Some(name) = field(node, "name") {
                    self.declare(
                        name,
                        context.scope,
                        context.owner,
                        node.kind() != "class",
                        false,
                    );
                }
            }
            "for_in_statement" if field(node, "kind").is_some() => {
                if let Some(name) = field(node, "left") {
                    let hoisted = field(node, "kind").is_some_and(|n| n.kind() == "var");
                    self.declare(
                        name,
                        if hoisted { hoist } else { context.scope },
                        context.owner,
                        false,
                        hoisted,
                    );
                    if hoisted {
                        self.mark_hoisted(name, hoist);
                    }
                }
            }
            "catch_clause" => {
                if let Some(name) = field(node, "parameter") {
                    self.declare(name, context.scope, context.owner, false, false);
                }
            }
            "import_clause" | "namespace_import" | "import_specifier" => {
                let name = if node.kind() == "import_specifier" {
                    field(node, "alias").or_else(|| field(node, "name"))
                } else {
                    children(node).find(|&n| identifier(n))
                };
                if let Some(name) = name {
                    self.declare(name, context.scope, context.owner, true, false);
                    if node.kind() == "namespace_import" {
                        self.namespaces.insert(name.start_byte());
                    }
                }
            }
            _ => {}
        }
        if is_owner(node) {
            if let Some(params) = field(node, "parameters").or_else(|| field(node, "parameter")) {
                self.declare(params, node, node, false, true);
            }
        }
        if is_owner(node) {
            for child in eager_owner_inputs(node) {
                self.collect(child, parent, parent_hoist);
            }
        }
        for child in children(node) {
            if parameter_decorator(child, node)
                || (is_owner(node)
                    && matches!(child.kind(), "decorator" | "computed_property_name"))
            {
                continue;
            }
            let outer = node.kind() == "with_statement" && field(node, "object") == Some(child);
            self.collect(child, if outer { parent } else { context }, hoist);
        }
        if has_body_environment {
            self.body_parameter_copies(node, context.owner);
        }
    }

    fn mark_hoisted(&mut self, pattern: Node<'a>, scope: Node<'a>) {
        let mut pending = vec![pattern];
        while let Some(part) = pending.pop() {
            if identifier(part) {
                if let Some(site) = self.scopes[&scope.id()]
                    .names
                    .get(self.canonical(self.file.node_text(&part)))
                    .and_then(Option::as_ref)
                {
                    self.hoisted_vars.insert(site.start_byte);
                }
            } else {
                pending.extend(pattern_parts(part));
            }
        }
    }

    fn body_parameter_copies(&mut self, body: Node<'a>, owner: Node<'a>) {
        let owner = &self.scopes[&owner.id()];
        let scope = &self.scopes[&body.id()];
        let copies = scope
            .shared_vars
            .iter()
            .filter_map(|name| {
                let formal = owner.names.get(name)?.clone()?;
                let body_var = scope.names.get(name)?.as_ref()?;
                Some((body_var.start_byte, formal))
            })
            .collect();
        self.scopes
            .get_mut(&body.id())
            .expect("function body scope")
            .parameter_copies = copies;
    }

    pub(super) fn canonical<'s>(&'s self, name: &'s str) -> &'s str {
        self.decoded.get(name).map_or(name, String::as_str)
    }

    pub(super) fn implicit_arguments(&self, node: Node<'a>, name: &str) -> bool {
        if self.canonical(name) != "arguments" {
            return false;
        }
        let mut next = Some(node.id());
        while let Some(id) = next {
            // A scope the collector never registered (error recovery) proves nothing.
            let Some(scope) = self.scopes.get(&id) else {
                return true;
            };
            if scope.names.contains_key("arguments") {
                return false;
            }
            if scope.implicit_arguments {
                return true;
            }
            next = scope.parent;
        }
        false
    }

    pub(super) fn lookup(&self, node: Node<'a>, name: &str) -> Option<&Option<AstSpan>> {
        self.lookup_inner(node, name, false)
    }

    pub(super) fn lookup_inner(
        &self,
        node: Node<'a>,
        name: &str,
        through_with: bool,
    ) -> Option<&Option<AstSpan>> {
        let name = self.canonical(name);
        let mut next = Some(node.id());
        while let Some(id) = next {
            // Fail closed: an unregistered scope (error recovery) is an unproven identity.
            let Some(scope) = self.scopes.get(&id) else {
                return Some(&None);
            };
            if let Some(binding) = scope.names.get(name) {
                return Some(binding);
            }
            if scope.dynamic && !through_with {
                return Some(&None);
            }
            next = scope.parent;
        }
        None
    }

    pub(super) fn binding(&self, node: Node<'a>, name: &str) -> Option<AstSpan> {
        self.lookup(node, name).cloned().flatten()
    }
}
