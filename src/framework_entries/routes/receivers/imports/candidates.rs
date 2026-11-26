use super::*;
use std::collections::{BTreeSet, VecDeque};

impl<'a> PythonBindings<'a> {
    pub(in crate::framework_entries::routes::receivers) fn possible_imported_views(
        &self,
        file: &'a ParsedFile,
        expression: Node<'a>,
        context: &SetupContext<'a>,
    ) -> Vec<PythonValue<'a>> {
        let Some(path) = self.trace_import(file, expression, context, 24) else {
            return Vec::new();
        };
        let mut pending = VecDeque::from(self.possible_members(&path));
        let mut seen = BTreeSet::new();
        let mut values = Vec::new();
        while let Some((member, name)) = pending.pop_front() {
            if !seen.insert((member.file.path.as_str(), member.node.id(), name.clone())) {
                continue;
            }
            match member.node.kind() {
                "function_definition" => values.push(PythonValue::Definition(member)),
                "import_statement" | "import_from_statement" => {
                    if let Some(path) = self.import_path(member.file, member.node, &name) {
                        pending.extend(self.possible_members(&path));
                    }
                }
                "assignment" => {
                    if let Some(path) = assignment_alias(member.node)
                        .and_then(|rhs| self.trace_import(member.file, rhs, context, 24))
                    {
                        pending.extend(self.possible_members(&path));
                    }
                    if let value @ PythonValue::Definition(_) =
                        self.bound(member, &name, context, 23)
                    {
                        values.push(value);
                    }
                }
                _ => {}
            }
        }
        values
    }

    // Keep candidate view definitions for metadata checks even through a
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
