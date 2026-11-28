use super::{ImportRefusalReason::*, *};
use crate::framework_entries::routes::python::literal_string;

impl<'a> PythonBindings<'a> {
    pub(super) fn member_refusal(
        &self,
        owner: Option<&'a ParsedFile>,
        module: &str,
        name: &str,
        child_exists: bool,
        sites: &[Node<'a>],
    ) -> Option<ImportRefusalReason> {
        if let Some(owner) = owner {
            let writes = &self.writes[owner.path.as_str()];
            let root = owner.tree.root_node();
            if has_getattr(writes) {
                return Some(GetattrModule);
            }
            if writes.wildcard(root).iter().any(|star| {
                self.star_could_supply(owner, *star, name, &mut vec![owner.path.as_str()])
            }) {
                return Some(StarImport);
            }
            if writes.global_write(name) {
                return Some(GlobalWrite);
            }
            if sites.len() > 1 {
                return Some(MultiWrite);
            }
            if let [site] = sites {
                if !direct(*site, root) {
                    return Some(NestedSingle);
                }
                if child_exists
                    && !self.same_child_import(owner, *site, name, &format!("{module}.{name}"))
                {
                    return Some(SubmoduleShadow);
                }
            }
        }
        matches!(name, "__dict__" | "__class__").then_some(ModuleDescriptorName)
    }

    pub(in crate::framework_entries::routes::receivers) fn star_could_supply(
        &self,
        origin: &'a ParsedFile,
        star: Node<'a>,
        name: &str,
        visited: &mut Vec<&'a str>,
    ) -> bool {
        let Some(source) = self.star_source(origin, star) else {
            return true;
        };
        if visited.len() >= 24 || visited.contains(&source.path.as_str()) {
            return true;
        }
        visited.push(&source.path);
        let result = self.exports_name(source, name, visited);
        visited.pop();
        result
    }

    pub(super) fn star_source(
        &self,
        origin: &ParsedFile,
        star: Node<'_>,
    ) -> Option<&'a ParsedFile> {
        let module = self.import_module(origin, star.child_by_field_name("module_name")?)?;
        self.module_file(&module)
    }

    fn exports_name(&self, source: &'a ParsedFile, name: &str, visited: &mut Vec<&'a str>) -> bool {
        let writes = &self.writes[source.path.as_str()];
        let root = source.tree.root_node();
        let stars = writes.wildcard(root);
        let all_sites = writes.module_sites("__all__");
        if stars
            .iter()
            .any(|star| self.star_could_supply(source, *star, "__all__", visited))
        {
            return true;
        }
        match all_sites {
            [] if !has_getattr(writes) => {
                !name.starts_with('_')
                    && (!writes.module_sites(name).is_empty()
                        || stars
                            .iter()
                            .any(|star| self.star_could_supply(source, *star, name, visited)))
            }
            [all] if direct(*all, root) => literal_exports(source, *all)
                .is_none_or(|exports| exports.iter().any(|export| export == name)),
            _ => true,
        }
    }
}

fn has_getattr(writes: &super::super::LexicalWrites<'_>) -> bool {
    writes
        .module_sites("__getattr__")
        .iter()
        .any(|node| !matches!(node.kind(), "delete_statement" | "for_in_clause"))
}

fn literal_exports(file: &ParsedFile, node: Node<'_>) -> Option<Vec<String>> {
    if node.kind() != "assignment" || node.child_by_field_name("left")?.kind() != "identifier" {
        return None;
    }
    let value = ungroup(node.child_by_field_name("right")?);
    if !matches!(value.kind(), "list" | "tuple") {
        return None;
    }
    children(value)
        .into_iter()
        .filter(|item| item.kind() != "comment")
        .map(|item| literal_string(file, item))
        .collect()
}
