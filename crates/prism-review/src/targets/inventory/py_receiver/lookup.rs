use super::{
    census::{Binding, Kind},
    class_slots, Resolver,
};
use crate::targets::dependency_identity::{module_root, python_module_paths};
use crate::targets::inventory::python::{
    source::{chain, children},
    walk::{ScopeId, ScopeKind},
};
use tree_sitter::Node;

pub(super) struct Alternative<'a, 'tree> {
    pub owner: ScopeId,
    pub bindings: Vec<&'a Binding<'tree>>,
    pub implicit_cell: bool,
}

impl<'tree> Resolver<'_, 'tree> {
    pub(super) fn lookup(
        &self,
        mut scope: ScopeId,
        name: &str,
        at: Node<'tree>,
    ) -> Vec<Alternative<'_, 'tree>> {
        let immediate = scope;
        loop {
            let info = &self.walk.scopes[scope];
            if scope != immediate && info.kind == ScopeKind::Class {
                if name == "__class__" && self.walk.scopes[immediate].kind != ScopeKind::Class {
                    return vec![Alternative {
                        owner: scope,
                        bindings: Vec::new(),
                        implicit_cell: true,
                    }];
                }
                scope = info.parent;
                continue;
            }
            let owner = self.census.declared_owner(self.walk, scope, name);
            if owner != scope {
                return vec![self.lookup_slot(owner, immediate, name, at)];
            }

            let bindings = self.census.spaces[scope].bindings.get(name);
            if info.kind == ScopeKind::Class {
                if let Some(bindings) = bindings {
                    let own: Vec<_> = class_slots::states(info.body, bindings, at)
                        .into_iter()
                        .map(|state| {
                            state.map_or_else(
                                || self.lookup_slot(0, immediate, name, at),
                                |binding| Alternative {
                                    owner: scope,
                                    bindings: vec![binding],
                                    implicit_cell: false,
                                },
                            )
                        })
                        .collect();
                    return own;
                }
            } else if bindings.is_some() || scope == 0 {
                return vec![self.lookup_slot(scope, immediate, name, at)];
            }
            scope = info.parent;
        }
    }

    fn lookup_slot(
        &self,
        owner: ScopeId,
        immediate: ScopeId,
        name: &str,
        at: Node<'tree>,
    ) -> Alternative<'_, 'tree> {
        let info = &self.walk.scopes[owner];
        let mut bindings: Vec<_> = self.census.spaces[owner]
            .bindings
            .get(name)
            .into_iter()
            .flatten()
            .filter(|binding| {
                info.kind != ScopeKind::Module || !matches!(binding.kind, Kind::Annotation)
            })
            .collect();
        if info.kind.is_function() && owner == immediate {
            let fresh_suites: Vec<_> = bindings
                .iter()
                .filter_map(|binding| {
                    if binding.scope != owner
                        || binding.position > at.start_byte()
                        || !matches!(binding.kind, Kind::Import(_))
                    {
                        return None;
                    }
                    let suite = binding
                        .node
                        .parent()
                        .filter(|suite| matches!(suite.kind(), "block" | "module"))?;
                    (suite.start_byte() <= at.start_byte() && at.end_byte() <= suite.end_byte())
                        .then_some(suite)
                })
                .collect();
            bindings.retain(|binding| {
                !(binding.scope == owner
                    && binding.position > at.start_byte()
                    && plain_rebind(binding)
                    && fresh_suites
                        .iter()
                        .any(|suite| direct(*suite, binding.node)))
            });
        }
        Alternative {
            owner,
            bindings,
            implicit_cell: false,
        }
    }

    pub(super) fn coalesces(&self, alternative: &Alternative<'_, 'tree>) -> bool {
        let [first, rest @ ..] = alternative.bindings.as_slice() else {
            return false;
        };
        if rest.is_empty() {
            return true;
        }
        let info = &self.walk.scopes[alternative.owner];
        let Kind::Import(initial) = &first.kind else {
            return false;
        };
        matches!(info.kind, ScopeKind::Module | ScopeKind::Function)
            && alternative.bindings.iter().all(|binding| {
                binding.scope == alternative.owner && direct(info.body, binding.node)
                    && matches!(&binding.kind, Kind::Import(import) if
                        !import.alias && !import.from && !import.relative && import.path == initial.path)
            })
    }

    pub(super) fn repository_local(&self, path: &str) -> bool {
        let root = module_root(path);
        // Built-in/frozen and release-startup identities in the supported
        // Python 3.11 profile precede the default search roots.
        if matches!(root, "builtins" | "os" | "io") {
            return false;
        }
        self.roots.iter().any(|directory| {
            python_module_paths(directory, root)
                .iter()
                .any(|path| self.repo.join(path).is_file())
        })
    }

    pub(super) fn type_only(&self, binding: &Binding<'tree>) -> bool {
        let mut scope = binding.scope;
        let mut node = binding.node;
        while let Some(parent) = node.parent() {
            // A guard on the enclosing definition uses that guard's scope,
            // even when an import inside the definition redirects elsewhere.
            if node == self.walk.scopes[scope].body {
                scope = self.walk.scopes[scope].parent;
            }
            if matches!(parent.kind(), "if_statement" | "elif_clause")
                && parent.child_by_field_name("consequence") == Some(node)
            {
                if let Some(test) = parent.child_by_field_name("condition") {
                    if self.type_guard(test, scope) {
                        return true;
                    }
                }
            }
            node = parent;
        }
        false
    }

    fn type_guard(&self, test: Node<'tree>, scope: ScopeId) -> bool {
        let Some(path) = chain(self.parsed, test, self.walk.class(scope)) else {
            return false;
        };
        let [root, rest @ ..] = path.as_slice() else {
            return false;
        };
        let alternatives = self.lookup(scope, root, test);
        alternatives.iter().all(|alt| {
            let [binding] = alt.bindings.as_slice() else {
                return false;
            };
            if !self.available(binding, test, scope) {
                return false;
            }
            let is_false = root == "TYPE_CHECKING"
                && rest.is_empty()
                && matches!(&binding.kind, Kind::Assigned(Some(value)) if value.kind() == "false")
                && direct(self.walk.scopes[binding.scope].body, binding.node);
            if is_false {
                return !self.name_rebound(alt.owner, root, test, scope);
            }
            let Kind::Import(import) = &binding.kind else {
                return false;
            };
            let path = super::import_path(import, rest);
            !import.relative
                && !self.repository_local(&path)
                && matches!(
                    path.as_str(),
                    "typing.TYPE_CHECKING" | "typing_extensions.TYPE_CHECKING"
                )
                && direct(self.walk.scopes[binding.scope].body, binding.node)
                && !self.name_rebound(alt.owner, root, test, scope)
                && !self.member_rebound(import, binding, test, scope, rest)
        })
    }

    pub(super) fn builtin_unshadowed(&self, scope: ScopeId, name: &str, at: Node<'tree>) -> bool {
        self.lookup(scope, name, at).iter().all(|alt| {
            alt.bindings.is_empty()
                && !alt.implicit_cell
                && !self.name_rebound(alt.owner, name, at, scope)
        })
    }
}

pub(super) fn direct(suite: Node<'_>, mut node: Node<'_>) -> bool {
    if let Some(parent) = node.parent() {
        if matches!(
            parent.kind(),
            "expression_statement" | "decorated_definition"
        ) {
            node = parent;
        }
    }
    node.parent() == Some(suite)
}

fn plain_rebind(binding: &Binding<'_>) -> bool {
    matches!(binding.kind, Kind::Assigned(_) | Kind::Delete)
        || matches!(binding.node.kind(), "assignment" | "augmented_assignment")
}

pub(super) fn args(call: Node<'_>) -> impl Iterator<Item = Node<'_>> {
    call.child_by_field_name("arguments")
        .into_iter()
        .flat_map(children)
}
