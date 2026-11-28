use super::{super::certificate::Clause, *};
use std::collections::{BTreeSet, VecDeque};

impl<'a> PythonBindings<'a> {
    pub(in crate::framework_entries::routes::receivers) fn refused_factory(
        &self,
        file: &'a ParsedFile,
        call: Node<'a>,
        context: &SetupContext<'a>,
    ) -> bool {
        if call.kind() != "call" || call.has_error() || !arguments(call).nodes().is_empty() {
            return false;
        }
        let Some(callee) = call.child_by_field_name("function") else {
            return false;
        };
        let Some(path) = self.trace_import_root(file, callee, context, 24, true) else {
            return false;
        };
        let proof = self.prove_import(file, &path, 24);
        let reason = proof.reason;
        if proof.selection().is_some() {
            return false;
        }
        // A refused import can retain a registration only if a candidate has
        // the supported factory shape. The candidate cannot prove its receiver.
        let recognized = self
            .possible_imported_values(file, callee, context, false)
            .into_iter()
            .any(|value| match value {
                PythonValue::Definition(member) => self
                    .factory_at_call(file, call, member, context, 23)
                    .is_some(),
                _ => false,
            });
        if let (true, Some(reason)) = (recognized, reason) {
            self.record_import_refusal(file, call, reason);
        }
        recognized
    }

    pub(in crate::framework_entries::routes::receivers) fn possible_imported_values(
        &self,
        file: &'a ParsedFile,
        expression: Node<'a>,
        context: &SetupContext<'a>,
        keep_routers: bool,
    ) -> Vec<PythonValue<'a>> {
        let mut pending: VecDeque<_> = self
            .trace_import_root(file, expression, context, 24, true)
            .into_iter()
            .flat_map(|path| self.possible_members(&path))
            .collect();
        for path in self.import_candidates(file, expression, context) {
            pending.extend(self.possible_members(&path));
        }
        let mut seen = BTreeSet::new();
        let mut values = Vec::new();
        while let Some((member, name)) = pending.pop_front() {
            if !seen.insert((member.file.path.as_str(), member.node.id(), name.clone())) {
                continue;
            }
            match member.node.kind() {
                "function_definition" => values.push(PythonValue::Definition(member)),
                "import_statement" | "import_from_statement" => {
                    for path in self.import_paths(member.file, member.node, &name) {
                        pending.extend(self.possible_members(&path));
                    }
                }
                "assignment" => {
                    if let Some(path) = assignment_alias(member.node)
                        .and_then(|rhs| self.trace_import(member.file, rhs, context, 24))
                    {
                        pending.extend(self.possible_members(&path));
                    }
                    match self.bound(member, &name, context, 23) {
                        value @ PythonValue::Definition(_) => values.push(value),
                        value @ PythonValue::Router(_) if keep_routers => values.push(value),
                        _ => {}
                    }
                }
                _ => {}
            }
        }
        values
    }

    /// The clause that refuses the root name of `expression`, if the certificate does.
    pub(in crate::framework_entries::routes::receivers) fn root_clause(
        &self,
        file: &'a ParsedFile,
        expression: Node<'a>,
    ) -> Option<Clause> {
        let (root, key) = self.read_root(file, expression)?;
        let local = python_scope(root);
        let scope =
            self.writes[file.path.as_str()].read_binding_scope(local, &key, root.start_byte())?;
        self.certify(file, scope, &key).err()
    }

    /// True when the root name of `expression` is not one static binding (the certificate
    /// refuses it): its other binding occurrences can supply other targets.
    pub(in crate::framework_entries::routes::receivers) fn uncertified_root(
        &self,
        file: &'a ParsedFile,
        expression: Node<'a>,
    ) -> bool {
        self.root_clause(file, expression).is_some()
    }

    /// True when the root name has a competing binding occurrence (a second binding, a
    /// redirect or a star that can bind it), so other candidates are known to exist.
    pub(in crate::framework_entries::routes::receivers) fn competing_root(
        &self,
        file: &'a ParsedFile,
        expression: Node<'a>,
    ) -> bool {
        matches!(
            self.root_clause(file, expression),
            Some(Clause::Count | Clause::Redirect | Clause::Star)
        )
    }

    fn read_root(&self, file: &ParsedFile, expression: Node<'a>) -> Option<(Node<'a>, String)> {
        let mut root = ungroup(expression);
        while let Some((object, _)) = attribute(root) {
            root = ungroup(object);
        }
        (root.kind() == "identifier").then(|| {
            let key = effective_name(file, python_scope(root), file.node_text(&root));
            (root, key.into_owned())
        })
    }

    // Every import or alias occurrence of the name read, in the scope it resolves to,
    // wherever it sits or runs: a loop back edge or a second call runs a later write before
    // the read, so position is no filter. Used when the certificate refuses the name.
    fn import_candidates(
        &self,
        file: &'a ParsedFile,
        expression: Node<'a>,
        context: &SetupContext<'a>,
    ) -> Vec<ImportPath<'a>> {
        let mut root = ungroup(expression);
        let mut suffix = Vec::new();
        while let Some((object, field)) = attribute(root) {
            suffix.push(Step {
                name: normalized(file.node_text(&field)).into_owned(),
                read: Some(root),
            });
            root = ungroup(object);
        }
        let Some((root, name)) = self.read_root(file, root) else {
            return Vec::new();
        };
        let writes = &self.writes[file.path.as_str()];
        let mut paths = Vec::new();
        let mut seen = BTreeSet::new();
        for write in writes.occurrences(python_scope(root), &name, root.start_byte()) {
            if !seen.insert(write.id()) {
                continue;
            }
            let aliases = assignment_alias(write)
                .and_then(|rhs| self.trace_import_root(file, rhs, context, 24, true));
            for mut path in self.import_paths(file, write, &name).chain(aliases) {
                path.steps.extend(suffix.iter().rev().cloned());
                paths.push(path);
            }
        }
        paths
    }

    // Keep candidate definitions and allocations for metadata checks even through a
    // refused hop or a re-export.
    fn possible_members(&self, path: &ImportPath<'a>) -> Vec<(PythonMember<'a>, String)> {
        let mut modules = BTreeSet::from([path.module.clone()]);
        let mut members = Vec::new();
        for (index, step) in path.steps.iter().enumerate() {
            let terminal = index + 1 == path.steps.len();
            let mut children = BTreeSet::new();
            for module in modules {
                let child = format!("{module}.{}", step.name);
                if !terminal && self.source_module(&child) {
                    children.insert(child.clone());
                }
                if let Some(owner) = self.module_file(&module) {
                    for member in self.possible_writes(owner, &step.name) {
                        if terminal {
                            members.push((member, step.name.clone()));
                        } else if let Some(proof) = self.initializer_module(
                            member.file,
                            member.node,
                            &step.name,
                            &child,
                            24,
                        ) {
                            if let Some(Selection::Module(module)) = proof.target {
                                children.insert(module);
                            }
                        }
                    }
                }
            }
            modules = children;
        }
        members
    }

    fn possible_writes(&self, owner: &'a ParsedFile, name: &str) -> Vec<PythonMember<'a>> {
        let mut pending = vec![owner];
        let mut seen = BTreeSet::new();
        let mut members = Vec::new();
        while let Some(file) = pending.pop() {
            if !seen.insert(file.path.as_str()) {
                continue;
            }
            let writes = &self.writes[file.path.as_str()];
            members.extend(
                writes
                    .module_sites(name)
                    .iter()
                    .map(|&node| PythonMember { file, node }),
            );
            for &star in writes.wildcard(file.tree.root_node()) {
                if let Some(source) = self.star_source(file, star) {
                    if self.star_could_supply(file, star, name, &mut vec![file.path.as_str()]) {
                        pending.push(source);
                    }
                }
            }
        }
        members
    }
}
