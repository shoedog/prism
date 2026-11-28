//! The one place that decides "this name is one static binding" (DESIGN section 5.3).
//!
//! A route proof is a static-binding proof: the name it relies on resolves, by Python's
//! declaration, scope, import and `global`/`nonlocal` syntax as prism models it, to exactly
//! one binding. Run-time mutation (reflective writes, descriptors, metaclasses) and changes
//! made through another holder of the same object are outside the claim.
//!
//! Binding occurrences of a key are the closed syntactic list of `writes::written_names`:
//! import (plain, as, from, from-as), def, async def, class, parameter (lambda included),
//! assignment target (plain, augmented, annotated with a value, tuple/list/starred leaves),
//! walrus (the enclosing non-comprehension scope), for and comprehension targets, with-as,
//! except-as and except*-as, match captures, `del K` and `type K = ..`. PEP 695 type
//! parameters bind an implicit annotation scope of their own, so by scope awareness they are
//! another scope's binding of the same spelling and neither count nor refuse.
//! `global K` and `nonlocal K` are directives that redirect a write in their scope.
//!
//! For a module scope the clauses are, first failing names the refusal:
//! (e) the module has a parse error; (a) the module has exactly one binding occurrence
//! (`global` redirects from any nesting and module-system names count, and so does a star
//! import that the repository says can bind the key); (c) a star that can bind the key counts
//! (a star from an external or unresolved module is not seen); (b) the single binding is an
//! import, def, async def, class or `K = <expr>` with one bare target, directly under the
//! module body. A function scope keeps the flow-sensitive lookup of its own locals and is
//! certified only against redirects: a nested scope that declares `nonlocal K` and binds it.

use super::{direct, refusals::ImportRefusalReason, ParsedFile, PythonBindings};
use tree_sitter::Node;

/// The clause that refused a name; each maps to an existing refusal label.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::framework_entries::routes) enum Clause {
    /// The module has a parse error (clause e).
    ParseError,
    /// Zero or two-plus binding occurrences (clause a, and clause d for module-system names).
    Count,
    /// A `global K` or `nonlocal K` redirect from a nested scope (clause a).
    Redirect,
    /// A star import that can bind the key (clause c).
    Star,
    /// The single binding is not a direct statement of an admitted kind (clause b).
    Kind,
}

impl From<Clause> for ImportRefusalReason {
    fn from(clause: Clause) -> Self {
        match clause {
            Clause::ParseError | Clause::Count => Self::MultiWrite,
            Clause::Redirect => Self::GlobalWrite,
            Clause::Star => Self::StarImport,
            Clause::Kind => Self::NestedSingle,
        }
    }
}

/// Names the import system and the interpreter bind in every module.
const MODULE_NAMES: [&str; 11] = [
    "__doc__",
    "__annotations__",
    "__package__",
    "__name__",
    "__file__",
    "__spec__",
    "__loader__",
    "__builtins__",
    "__cached__",
    "__path__",
    "__warningregistry__",
];

impl<'a> PythonBindings<'a> {
    /// Decide whether `key` (the NFKC-normalised, class-mangled name) is one static binding
    /// in `scope`. Cached per (file, scope, key).
    pub(super) fn certify(
        &self,
        file: &ParsedFile,
        scope: Node<'a>,
        key: &str,
    ) -> Result<(), Clause> {
        let cache = (file.path.clone(), scope.id(), key.to_owned());
        if let Some(verdict) = self.certificates.borrow().get(&cache) {
            return *verdict;
        }
        let verdict = self.decide(file, scope, key);
        self.certificates.borrow_mut().insert(cache, verdict);
        verdict
    }

    /// Certify a name written as `name` in `file`'s module scope.
    pub(in crate::framework_entries::routes) fn certified_module_name(
        &self,
        file: &'a ParsedFile,
        name: &str,
    ) -> bool {
        self.certify_in(file, file.tree.root_node(), name).is_ok()
    }

    /// Certify a name written as `name` in `scope`, mangling it as that scope would.
    pub(in crate::framework_entries::routes) fn certify_in(
        &self,
        file: &'a ParsedFile,
        scope: Node<'a>,
        name: &str,
    ) -> Result<(), Clause> {
        let key = super::writes::effective_name(file, scope, name);
        self.certify(file, scope, &key)
    }

    fn decide(&self, file: &ParsedFile, scope: Node<'a>, key: &str) -> Result<(), Clause> {
        let writes = &self.writes[file.path.as_str()];
        let root = file.tree.root_node();
        if root.has_error() {
            return Err(Clause::ParseError);
        }
        if scope.kind() != "module" {
            return if writes.nonlocal_write(scope, key) {
                Err(Clause::Redirect)
            } else {
                Ok(())
            };
        }
        if writes.global_write(key) {
            return Err(Clause::Redirect);
        }
        let sites = writes.binding_sites(scope, key);
        let system = usize::from(MODULE_NAMES.contains(&key));
        let stars = writes
            .wildcard(root)
            .iter()
            .filter(|&&star| {
                self.star_source(file, star).is_some()
                    && self.star_could_supply(
                        self.files[file.path.as_str()],
                        star,
                        key,
                        &mut vec![file.path.as_str()],
                    )
            })
            .count();
        if sites.is_empty() {
            return Err(if stars > 0 {
                Clause::Star
            } else {
                Clause::Count
            });
        }
        if sites.len() + system > 1 {
            return Err(Clause::Count);
        }
        if stars > 0 {
            return Err(Clause::Star);
        }
        let site = sites[0];
        if !direct(site, scope) || !admitted_kind(site) {
            return Err(Clause::Kind);
        }
        Ok(())
    }
}

/// The kinds of single binding the certificate admits.
fn admitted_kind(site: Node<'_>) -> bool {
    match site.kind() {
        "import_statement"
        | "import_from_statement"
        | "function_definition"
        | "class_definition" => true,
        "assignment" => {
            site.child_by_field_name("right").is_some()
                && site
                    .child_by_field_name("left")
                    .is_some_and(|left| left.kind() == "identifier")
        }
        _ => false,
    }
}
