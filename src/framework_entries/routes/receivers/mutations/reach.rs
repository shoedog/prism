use super::*;
use crate::framework_entries::routes::receivers::{
    direct,
    imports::{import_items, source_path},
};
use std::collections::BTreeMap;

#[derive(Default)]
pub(super) struct FileReach<'a> {
    direct_from: BTreeMap<(&'a str, &'a str), usize>,
    components: BTreeMap<&'a str, &'a str>,
}

impl<'a> FileReach<'a> {
    pub(super) fn index(bindings: &PythonBindings<'a>) -> Self {
        let mut reach = Self::default();
        let mut neighbors = BTreeMap::<_, BTreeSet<_>>::new();
        for (&path, &file) in &bindings.files {
            for &node in bindings.nodes(path) {
                if !matches!(node.kind(), "import_statement" | "import_from_statement") {
                    continue;
                }
                let from = node.child_by_field_name("module_name");
                let module = from.and_then(|from| bindings.import_module(file, from));
                if direct(node, file.tree.root_node()) {
                    if let Some(source) = module
                        .as_deref()
                        .and_then(|name| bindings.module_file(name))
                    {
                        reach
                            .direct_from
                            .entry((path, source.path.as_str()))
                            .or_insert(node.end_byte());
                    }
                }
                let members = import_items(node).map(|(name, _)| {
                    let name = source_path(file, name);
                    match &module {
                        Some(module) => format!("{module}.{name}"),
                        None => name,
                    }
                });
                for name in module.iter().cloned().chain(members) {
                    let prefixes = name.match_indices('.').map(|(end, _)| &name[..end]);
                    for imported in prefixes.chain([name.as_str()]).filter_map(|module| {
                        bindings.module_file(module).map(|file| file.path.as_str())
                    }) {
                        neighbors.entry(path).or_default().insert(imported);
                        neighbors.entry(imported).or_default().insert(path);
                    }
                }
            }
        }
        for &path in bindings.files.keys() {
            let mut pending = vec![path];
            while let Some(member) = pending.pop() {
                if let std::collections::btree_map::Entry::Vacant(entry) =
                    reach.components.entry(member)
                {
                    entry.insert(path);
                    pending.extend(neighbors.get(member).into_iter().flatten().copied());
                }
            }
        }
        reach
    }

    pub(super) fn component(&self, path: &'a str) -> &'a str {
        self.components.get(path).copied().unwrap_or(path)
    }

    pub(super) fn direct_import_precedes(
        &self,
        source: &'a str,
        registration: &'a str,
        before: usize,
    ) -> bool {
        self.direct_from
            .get(&(registration, source))
            .is_some_and(|&end| end <= before)
    }
}
