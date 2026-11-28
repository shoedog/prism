use super::*;

impl<'a> PythonBindings<'a> {
    // A written root can select an earlier explicit import in the same lexical
    // scope. The file-wide census then refuses the competing write at that import.
    pub(super) fn import_binding(
        &self,
        file: &ParsedFile,
        use_node: Node<'a>,
        name: &str,
        context: &SetupContext<'a>,
        capture_root: bool,
    ) -> Option<Node<'a>> {
        let local = python_scope(use_node);
        let root = file.tree.root_node();
        let writes = &self.writes[file.path.as_str()];
        let mut scope = local;
        let mut directed_write = None;
        loop {
            let before = if scope == local {
                use_node.start_byte()
            } else if scope == root {
                self.module_before(file, context)
            } else {
                scope.end_byte()
            };
            if capture_root {
                if let Some(import) = writes.import_before(scope, name, before) {
                    return Some(import);
                }
            }
            let named = writes.named_before(scope, name, before);
            let directive = writes.directive(scope, name);
            if capture_root && directive.is_some() {
                // An explicit import in the directed scope can be hidden by this write.
                directed_write = directed_write.or(named);
            } else if let Some(write) = directed_write.or(named) {
                if scope == root
                    && scope != local
                    && directed_write.is_none()
                    && self.mutations.is_some()
                    && context.frames.is_empty()
                    && writes.named_before(root, name, local.start_byte()) != Some(write)
                {
                    return None;
                }
                return Some(write);
            }
            if scope.kind() != "class_definition"
                && directive.is_none()
                && writes.binds_local(scope, name)
            {
                return None;
            }
            let Some(outer) = writes.outer_binding_scope(scope, name) else {
                return directed_write;
            };
            scope = outer;
        }
    }

    pub(super) fn trace_import(
        &self,
        file: &'a ParsedFile,
        expression: Node<'a>,
        context: &SetupContext<'a>,
        depth: usize,
    ) -> Option<ImportPath<'a>> {
        self.trace_import_root(file, expression, context, depth, false)
    }

    pub(super) fn trace_import_root(
        &self,
        file: &'a ParsedFile,
        expression: Node<'a>,
        context: &SetupContext<'a>,
        depth: usize,
        capture_root: bool,
    ) -> Option<ImportPath<'a>> {
        if depth == 0 {
            return None;
        }
        let expression = ungroup(expression);
        if let Some((object, name)) = attribute(expression) {
            let mut path =
                self.trace_import_root(file, object, context, depth - 1, capture_root)?;
            let name = normalized(file.node_text(&name));
            path.private |= private_name(&name);
            path.steps.push(Step {
                name: name.into_owned(),
                read: Some(expression),
            });
            path.span = expression;
            // A module alias does not alias the callable selected by an attribute read.
            path.alias = false;
            return Some(path);
        }
        if expression.kind() != "identifier" {
            return None;
        }
        let spelling = file.node_text(&expression);
        let name = effective_name(file, python_scope(expression), spelling);
        let binding = self.import_binding(file, expression, &name, context, capture_root)?;
        if matches!(binding.kind(), "import_statement" | "import_from_statement") {
            let mut path = self.import_path(file, binding, &name)?;
            path.private |= private_name(&normalized(spelling));
            return Some(path);
        }
        let scope = python_scope(binding);
        let value = assignment_alias(binding)?;
        if !direct(binding, scope)
            || (self.mutations.is_some()
                && (self.writes[file.path.as_str()].named_before(scope, &name, scope.end_byte())
                    != Some(binding)
                    || !self.writes[file.path.as_str()]
                        .binding_scope_of_write(binding, &name)
                        .is_some_and(|scope| self.certify(file, scope, &name).is_ok())))
        {
            return None;
        }
        let mut path = self.trace_import(file, value, context, depth - 1)?;
        path.private |= private_name(&normalized(spelling));
        path.alias = true;
        Some(path)
    }
}
