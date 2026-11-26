use super::{Resolver, Slot};
use crate::targets::inventory::{
    py_receiver::{
        census::{Binding, Import, Kind},
        lookup::{args, direct, Alternative},
        slots::{Instance, Read},
    },
    python::{
        source::{children, name, ungroup},
        walk::ScopeId,
    },
};
use tree_sitter::Node;

#[derive(Clone, Default, PartialEq, Eq)]
pub(super) struct Objects {
    paths: Vec<String>,
    instances: Vec<Instance>,
    unknown: bool,
    untracked_parameter: bool,
    other: bool,
}

impl Objects {
    fn imported(path: String) -> Self {
        Self {
            paths: vec![path],
            ..Self::default()
        }
    }

    fn instance(origin: Instance) -> Self {
        Self {
            instances: vec![origin],
            ..Self::default()
        }
    }

    pub(super) fn unknown() -> Self {
        Self {
            unknown: true,
            ..Self::default()
        }
    }

    // A definitely unrelated object has no affected import slots.
    pub(super) fn other() -> Self {
        Self {
            other: true,
            ..Self::default()
        }
    }

    fn extend(&mut self, other: &Self) {
        self.unknown |= other.unknown;
        self.untracked_parameter |= other.untracked_parameter;
        self.other |= other.other;
        for path in &other.paths {
            if !self.paths.contains(path) {
                self.paths.push(path.clone());
            }
        }
        for origin in &other.instances {
            if !self.instances.contains(origin) {
                self.instances.push(origin.clone());
            }
        }
    }

    pub(super) fn slots(self, key: Option<String>) -> Vec<Slot> {
        let definite = self.paths.len() == 1
            && self.instances.is_empty()
            && !self.unknown
            && !self.untracked_parameter
            && !self.other
            && key.is_some();
        self.paths
            .into_iter()
            .map(Some)
            .chain(self.unknown.then_some(None))
            .map(|object| Slot::Member {
                object,
                key: key.clone(),
                definite,
            })
            .chain(
                (self.untracked_parameter && !self.unknown)
                    .then(|| Slot::ParameterMember { key: key.clone() }),
            )
            .chain(self.instances.into_iter().map(|origin| Slot::Instance {
                origin,
                key: key.clone(),
            }))
            .collect()
    }
}

impl<'tree> Resolver<'_, 'tree> {
    pub(in crate::targets::inventory::py_receiver) fn names_instance(
        &self,
        node: Node<'tree>,
        scope: ScopeId,
        origin: &Instance,
    ) -> bool {
        self.objects(node, scope, true).instances.contains(origin)
    }

    pub(super) fn objects(&self, node: Node<'tree>, scope: ScopeId, alias: bool) -> Objects {
        let node = ungroup(node);
        if self.implicit_builtins(node, scope) {
            return Objects::imported("builtins".to_owned());
        }
        match node.kind() {
            "identifier" => {
                let key = name(self.parsed.node_text(&node), self.walk.class(scope));
                self.lookup_objects(self.lookup(scope, &key, node), &key, node, scope, alias)
            }
            "attribute" => {
                let (Some(base), Some(member)) = (
                    node.child_by_field_name("object"),
                    node.child_by_field_name("attribute"),
                ) else {
                    return Objects::unknown();
                };
                let key = name(self.parsed.node_text(&member), self.walk.class(scope));
                self.member_objects(
                    self.objects(base, scope, alias),
                    &key,
                    self.read(node, scope),
                )
            }
            "subscript" => {
                let Some(value) = node.child_by_field_name("value").map(ungroup) else {
                    return Objects::unknown();
                };
                let Some(key) = node
                    .child_by_field_name("subscript")
                    .and_then(|index| super::strings::literal(self.parsed, index))
                else {
                    return Objects::unknown();
                };
                if self.reflection_helper(value, scope, "globals") && args(value).next().is_none() {
                    return self.lookup_objects(
                        self.lookup(0, &key, node),
                        &key,
                        node,
                        scope,
                        alias,
                    );
                }
                if self.implicit_builtins(value, scope) {
                    return self.member_objects(
                        Objects::imported("builtins".to_owned()),
                        &key,
                        self.read(node, scope),
                    );
                }
                Objects::unknown()
            }
            "assignment" => node
                .child_by_field_name("right")
                .map_or_else(Objects::unknown, |value| self.objects(value, scope, alias)),
            "conditional_expression" => {
                let mut exprs = children(node);
                let (Some(yes), Some(no)) = (exprs.next(), exprs.nth(1)) else {
                    return Objects::unknown();
                };
                let mut values = self.objects(yes, scope, alias);
                values.extend(&self.objects(no, scope, alias));
                values
            }
            "none"
            | "true"
            | "false"
            | "integer"
            | "float"
            | "string"
            | "concatenated_string"
            | "list"
            | "dictionary"
            | "tuple"
            | "set" => Objects::other(),
            "call" if self.fresh_object(node, scope) => Objects::other(),
            _ => Objects::unknown(),
        }
    }

    fn lookup_objects(
        &self,
        alternatives: Vec<Alternative<'_, 'tree>>,
        key: &str,
        at: Node<'tree>,
        scope: ScopeId,
        alias: bool,
    ) -> Objects {
        let read = self.read(at, scope);
        let mut objects = Objects::default();
        for alt in alternatives {
            let latest = alt
                .bindings
                .iter()
                .filter(|binding| {
                    binding.scope == scope
                        && binding.position <= read.byte
                        && dominates(binding.node, at)
                })
                .map(|binding| binding.position)
                .max();
            let (effects, reflected) =
                self.object_effects(read, latest, |slot| slot.matches_name(alt.owner, key));
            objects.extend(&effects);
            let latest = latest.max(reflected);
            let mut resolved = Objects {
                unknown: alt.bindings.is_empty() && reflected.is_none(),
                ..Objects::default()
            };
            let mut unavailable = false;
            for binding in alt.bindings {
                if binding.scope == scope
                    && latest.is_some_and(|latest| {
                        binding.position < latest
                            || (binding.position > read.byte && !shares_loop(binding.node, at))
                    })
                {
                    continue;
                }
                unavailable |= !self.available(binding, at, scope);
                let origin = self
                    .owning_self(binding)
                    .map(Instance::OwningSelf)
                    .or_else(|| {
                        self.construction_origin(binding, alt.owner)
                            .map(Instance::Constructed)
                    })
                    .or_else(|| {
                        self.declared_origin(binding, alt.owner)
                            .map(Instance::Constructed)
                    });
                let value = if let Some(origin) = origin {
                    Objects::instance(origin)
                } else {
                    match &binding.kind {
                        Kind::Import(import) => self.import_objects(import, binding),
                        Kind::Assigned(Some(value)) if alias => {
                            let mut objects = self.objects(*value, binding.scope, false);
                            if !matches!(ungroup(*value).kind(), "identifier" | "attribute") {
                                objects.instances.clear();
                            }
                            objects
                        }
                        Kind::Parameter => Objects {
                            untracked_parameter: true,
                            ..Objects::default()
                        },
                        Kind::Delete => Objects::other(),
                        _ => Objects::unknown(),
                    }
                };
                resolved.extend(&value);
            }
            if unavailable && resolved.paths.is_empty() && resolved.instances.is_empty() {
                resolved.unknown = true;
            }
            objects.extend(&resolved);
        }
        objects
    }

    fn member_objects(&self, base: Objects, key: &str, read: Read) -> Objects {
        let mut objects = Objects {
            unknown: base.unknown,
            untracked_parameter: base.untracked_parameter,
            other: base.other,
            ..Objects::default()
        };
        for path in base.paths {
            let (effects, replacement) =
                self.object_effects(read, None, |slot| slot.matches_member(&path, key));
            objects.extend(&effects);
            if replacement.is_none() {
                objects.extend(&Objects::imported(format!("{path}.{key}")));
            }
        }
        for origin in base.instances {
            if let Instance::OwningSelf(owner) = origin {
                objects.extend(&Objects::instance(Instance::Field {
                    owner,
                    name: key.to_owned(),
                }));
            }
        }
        objects
    }

    fn import_objects(&self, import: &Import, binding: &Binding<'tree>) -> Objects {
        if import.relative || self.repository_local(&import.path) {
            return Objects::other();
        }
        if !import.from && !import.alias {
            return Objects::imported(import.path.clone());
        }
        let mut parts = import.path.split('.');
        let root = parts.next().expect("nonempty import path");
        let read = Read {
            byte: binding.position,
            ..self.read(binding.node, binding.scope)
        };
        parts.fold(Objects::imported(root.to_owned()), |object, key| {
            self.member_objects(object, key, read)
        })
    }

    fn object_effects(
        &self,
        read: Read,
        after: Option<usize>,
        matches: impl Fn(&Slot) -> bool,
    ) -> (Objects, Option<usize>) {
        let writes = || {
            self.writes.iter().filter(|write| {
                write.reaches(read)
                    && matches(&write.slot)
                    && (write.read_point.scope != read.scope
                        || after.is_none_or(|after| write.read_point.byte >= after))
            })
        };
        let last = writes()
            .filter(|write| write.slot.definite() && write.replaces(read))
            .map(|write| write.read_point.byte)
            .max();
        let mut values = Objects::default();
        for write in writes() {
            if write.replaces(read) && last.is_some_and(|last| write.read_point.byte < last) {
                continue;
            }
            values.extend(&write.effect);
            // A parameter is not a tracked write target. An explicit
            // replacement with that parameter can still obscure provenance.
            values.unknown |= write.effect.untracked_parameter;
        }
        (values, last)
    }

    fn fresh_object(&self, call: Node<'tree>, scope: ScopeId) -> bool {
        let Some(callee) = self.parsed.call_function_node(&call).map(ungroup) else {
            return false;
        };
        if callee.kind() != "identifier" || args(call).next().is_some() {
            return false;
        }
        let key = name(self.parsed.node_text(&callee), self.walk.class(scope));
        if key == "object" && self.builtin_unshadowed(scope, &key, callee) {
            return !self.builtin_rebound(&key, callee, scope);
        }
        let alternatives = self.lookup(scope, &key, callee);
        let [alt] = alternatives.as_slice() else {
            return false;
        };
        let [binding] = alt.bindings.as_slice() else {
            return false;
        };
        let class = binding.node;
        let read = self.read(call, scope);
        let construction_changed = self.writes.iter().any(|write| {
            write.reaches(read)
                && ["__new__", "__bases__", "__class__"].iter().any(|key| {
                    // An unresolved object write can include this local class.
                    write.slot.matches_member("", key)
                })
        });
        !self.name_rebound(alt.owner, &key, callee, scope)
            && !construction_changed
            && self.available(binding, callee, scope)
            && class.kind() == "class_definition"
            && class
                .parent()
                .is_none_or(|parent| parent.kind() != "decorated_definition")
            && class.child_by_field_name("superclasses").is_none()
            && class.child_by_field_name("body").is_some_and(|body| {
                children(body).all(|statement| statement.kind() == "pass_statement")
            })
    }
}

fn dominates(binding: Node<'_>, at: Node<'_>) -> bool {
    let suite = std::iter::successors(binding.parent(), |node| node.parent())
        .find(|node| matches!(node.kind(), "module" | "block"));
    suite.is_some_and(|suite| {
        direct(suite, binding)
            && suite.start_byte() <= at.start_byte()
            && at.end_byte() <= suite.end_byte()
    })
}

fn shares_loop(binding: Node<'_>, at: Node<'_>) -> bool {
    std::iter::successors(at.parent(), |node| node.parent()).any(|loop_node| {
        matches!(loop_node.kind(), "for_statement" | "while_statement")
            && loop_node.start_byte() <= binding.start_byte()
            && binding.end_byte() <= loop_node.end_byte()
    })
}
