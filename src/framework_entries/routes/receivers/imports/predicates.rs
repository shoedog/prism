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
    /// True when running the package initializer can install its child module `name` as a
    /// member: an import it executes, directly or through in-repo modules it imports, names
    /// `pkg.name` (`from .name import x`, `from . import name`, `import pkg.name`). Python
    /// binds the submodule on the package, so a default star import copies it. The export set
    /// of such a package is not determined by its own bindings, so the star may bind `name`.
    fn initializer_installs(&self, source: &'a ParsedFile, name: &str) -> bool {
        let Some(package) = source.path.strip_suffix("/__init__.py") else {
            return false;
        };
        let child = format!("{}.{name}", package.replace('/', "."));
        self.executed_imports_name(source, &child, &mut vec![source.path.as_str()], 0)
    }

    /// True when `file`'s module-level imports, or those of the in-repo modules they reach
    /// (a visited set bounds cycles, `INSTALL_DEPTH` the chain), name `child` or a module
    /// below it. Past the depth cap the answer is yes for an existing child module: the
    /// chain is not followed to its end, so the export set stays open.
    fn executed_imports_name(
        &self,
        file: &'a ParsedFile,
        child: &str,
        visited: &mut Vec<&'a str>,
        depth: usize,
    ) -> bool {
        if depth >= INSTALL_DEPTH {
            return self.source_module(child);
        }
        let below = format!("{child}.");
        let root = file.tree.root_node();
        self.writes[file.path.as_str()]
            .imports(root)
            .iter()
            .any(|&import| {
                self.executed_modules(file, import)
                    .into_iter()
                    .any(|module| {
                        if module == child || module.starts_with(&below) {
                            return true;
                        }
                        match self.module_file(&module) {
                            Some(next) if !visited.contains(&next.path.as_str()) => {
                                visited.push(&next.path);
                                self.executed_imports_name(next, child, visited, depth + 1)
                            }
                            _ => false,
                        }
                    })
            })
    }

    /// The modules an import statement can execute: the imported module and its parents, and
    /// each imported member as a possible submodule.
    fn executed_modules(&self, file: &ParsedFile, import: Node<'_>) -> Vec<String> {
        let mut written: Vec<String> = Vec::new();
        match import.child_by_field_name("module_name") {
            Some(from) => {
                if let Some(module) = self.import_module(file, from) {
                    written.extend(
                        import_items(import)
                            .map(|(path, _)| format!("{module}.{}", source_path(file, path))),
                    );
                    written.push(module);
                }
            }
            None => written
                .extend(import_items(import).map(|(path, _)| source_path(file, path).to_owned())),
        }
        let mut modules = Vec::new();
        for name in written {
            modules.extend(
                name.match_indices('.')
                    .map(|(end, _)| name[..end].to_owned()),
            );
            modules.push(name);
        }
        modules
    }
}

/// The longest chain of in-repo initializer imports followed to find an installed child.
const INSTALL_DEPTH: usize = 12;

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
