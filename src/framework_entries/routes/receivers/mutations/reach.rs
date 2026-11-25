use super::*;
use crate::framework_entries::routes::receivers::{direct, import_parts};
use std::collections::BTreeMap;

#[derive(Default)]
pub(super) struct FileReach<'a> {
    imports: BTreeMap<&'a str, BTreeSet<&'a str>>,
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
                let module =
                    from.and_then(|from| bindings.absolute_module(path, file.node_text(&from)));
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
                let members = children(node)
                    .into_iter()
                    .filter(|child| Some(*child) != from)
                    .filter_map(|child| import_parts(child).map(|(name, _)| name))
                    .map(|name| match &module {
                        Some(module) => format!("{module}.{}", file.node_text(&name)),
                        None => file.node_text(&name).into(),
                    });
                for name in module.iter().cloned().chain(members) {
                    let prefixes = name.match_indices('.').map(|(end, _)| &name[..end]);
                    for imported in prefixes.chain([name.as_str()]).filter_map(|module| {
                        bindings.module_file(module).map(|file| file.path.as_str())
                    }) {
                        neighbors.entry(path).or_default().insert(imported);
                        neighbors.entry(imported).or_default().insert(path);
                        reach.imports.entry(path).or_default().insert(imported);
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

    // A directly imported member module has finished its module body before use
    // unless a static import path can bring execution back to the registrant.
    pub(super) fn member_precedes(
        &self,
        source: &'a str,
        registration: &'a str,
        before: usize,
    ) -> bool {
        if self
            .direct_from
            .get(&(registration, source))
            .is_none_or(|&end| end > before)
        {
            return false;
        }
        let mut visited = BTreeSet::new();
        let mut pending = vec![source];
        while let Some(path) = pending.pop() {
            if path == registration {
                return false;
            }
            if visited.insert(path) {
                pending.extend(self.imports.get(path).into_iter().flatten().copied());
            }
        }
        true
    }
}
