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
                self.star_could_supply(owner, *star, name, &mut vec![owner.path.as_str()], true)
            }) {
                return Some(StarImport);
            }
            // The certificate counts the binding occurrences of the name in the source module.
            // A name the module does not bind (a child module, a missing member) has no
            // binding to certify, but a module with a parse error certifies nothing.
            if !sites.is_empty() || root.has_error() {
                if let Err(clause) = self.certify(owner, root, name) {
                    return Some(clause.into());
                }
            }
            if let [site] = sites {
                if child_exists
                    && !self.same_child_import(owner, *site, name, &format!("{module}.{name}"))
                {
                    return Some(SubmoduleShadow);
                }
            }
        }
        matches!(name, "__dict__" | "__class__").then_some(ModuleDescriptorName)
    }

    /// True when `star` can bind `name`. A star from a module outside the repository counts
    /// only when `external` is set (the source side keeps it; the capture side, which sees
    /// only what the repository declares, never counts one, directly or nested).
    pub(in crate::framework_entries::routes::receivers) fn star_could_supply(
        &self,
        origin: &'a ParsedFile,
        star: Node<'a>,
        name: &str,
        visited: &mut Vec<&'a str>,
        external: bool,
    ) -> bool {
        let Some(source) = self.star_source(origin, star) else {
            return external;
        };
        if visited.len() >= 24 || visited.contains(&source.path.as_str()) {
            return true;
        }
        visited.push(&source.path);
        let result = self.exports_name(source, name, visited, external);
        visited.pop();
        result
    }

    pub(in crate::framework_entries::routes::receivers) fn star_source(
        &self,
        origin: &ParsedFile,
        star: Node<'_>,
    ) -> Option<&'a ParsedFile> {
        let module = self.import_module(origin, star.child_by_field_name("module_name")?)?;
        self.module_file(&module)
    }

    fn exports_name(
        &self,
        source: &'a ParsedFile,
        name: &str,
        visited: &mut Vec<&'a str>,
        external: bool,
    ) -> bool {
        let writes = &self.writes[source.path.as_str()];
        let root = source.tree.root_node();
        let stars = writes.wildcard(root);
        let all_sites = writes.module_sites("__all__");
        if stars
            .iter()
            .any(|star| self.star_could_supply(source, *star, "__all__", visited, external))
        {
            return true;
        }
        match all_sites {
            [] if !has_getattr(writes) => {
                !name.starts_with('_')
                    && (!writes.module_sites(name).is_empty()
                        || self.initializer_installs(source, name)
                        || stars.iter().any(|star| {
                            self.star_could_supply(source, *star, name, visited, external)
                        }))
            }
            [all] if direct(*all, root) => literal_exports(source, *all)
                .is_none_or(|exports| exports.iter().any(|export| export == name)),
            _ => true,
        }
    }
}

impl<'a> PythonBindings<'a> {
    /// True when a package initializer's own import syntax installs its child module `name`
    /// as a member: `from .name import x`, `from . import name`, `import pkg.name`. Python
    /// binds the submodule on the package, so a default star import copies it.
    fn initializer_installs(&self, source: &'a ParsedFile, name: &str) -> bool {
        let Some(package) = source.path.strip_suffix("/__init__.py") else {
            return false;
        };
        let package = package.replace('/', ".");
        let child = format!("{package}.{name}");
        let below = format!("{child}.");
        let root = source.tree.root_node();
        self.writes[source.path.as_str()]
            .imports(root)
            .iter()
            .any(|&import| {
                let from = import
                    .child_by_field_name("module_name")
                    .and_then(|from| self.import_module(source, from));
                match from {
                    Some(module) => {
                        module == child
                            || module.starts_with(&below)
                            || (module == package
                                && import_items(import)
                                    .any(|(path, _)| source_path(source, path) == name))
                    }
                    None => import_items(import).any(|(path, _)| {
                        let path = source_path(source, path);
                        path == child || path.starts_with(&below)
                    }),
                }
            })
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
