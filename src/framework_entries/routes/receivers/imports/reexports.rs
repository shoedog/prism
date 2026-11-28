use super::*;

impl<'a> PythonBindings<'a> {
    /// Identify a terminal forwarding binding for the Flask receiver role.
    /// This does not make the forwarded value eligible for import proof.
    pub(crate) fn terminal_router_reexport(
        &self,
        file: &'a ParsedFile,
        expression: Node<'a>,
        context: &SetupContext<'a>,
    ) -> bool {
        let Some(path) = self.trace_import_root(file, expression, context, 24, true) else {
            return false;
        };
        let (owner, forwarded) = match self.prove_import(file, &path, 24).selection() {
            Some(Selection::Forwarded(member, name)) => (
                member.file,
                self.import_path(member.file, member.node, &name),
            ),
            Some(Selection::Member(member, _)) => (
                member.file,
                assignment_alias(member.node).and_then(|rhs| {
                    self.trace_import(member.file, rhs, &SetupContext::default(), 23)
                }),
            ),
            _ => return false,
        };
        forwarded.is_some_and(|path| {
            let selection = self.prove_import(owner, &path, 23).selection();
            matches!(self.imported_value(selection, 23),
                PythonValue::Router(router) if router.identity.file != owner.path)
        })
    }
}
