use super::{
    census::{Binding, Import},
    lookup::direct,
    Resolver,
};
use crate::targets::inventory::{py_tables, python::walk::ScopeId};
use tree_sitter::Node;

#[derive(Debug, PartialEq, Eq)]
pub(super) enum Slot {
    Name { owner: ScopeId, name: String },
    Member { object: String, key: String },
}

#[derive(Clone, Copy)]
pub(super) struct Read {
    pub scope: ScopeId,
    pub byte: usize,
    pub direct: bool,
}

impl<'tree> Resolver<'_, 'tree> {
    pub(super) fn name_rebound(
        &self,
        owner: ScopeId,
        name: &str,
        at: Node<'tree>,
        scope: ScopeId,
    ) -> bool {
        let read = self.read(at, scope);
        self.writes.iter().any(|write| {
            matches!(&write.slot, Slot::Name { owner: written_owner, name: key } if *written_owner == owner && key == name)
                && write.reaches(read)
        })
    }

    pub(super) fn builtin_rebound(&self, name: &str, at: Node<'tree>, scope: ScopeId) -> bool {
        self.reads_member("builtins", name, self.read(at, scope))
    }

    pub(super) fn member_rebound(
        &self,
        import: &Import,
        binding: &Binding<'tree>,
        at: Node<'tree>,
        scope: ScopeId,
        rest: &[String],
    ) -> bool {
        let mut parts = import.path.split('.');
        let Some(root) = parts.next() else {
            return false;
        };
        let mut owner = root.to_owned();
        if import.from || import.alias {
            let captured = Read {
                scope: binding.scope,
                byte: binding.position,
                direct: direct(self.walk.scopes[binding.scope].body, binding.node),
            };
            for key in parts {
                if self.reads_member(&owner, key, captured) {
                    return true;
                }
                owner.push('.');
                owner.push_str(key);
            }
        } else {
            owner.clone_from(&import.path);
        }

        let read = self.read(at, scope);
        for key in rest {
            if self.reads_member(&owner, key, read) {
                return true;
            }
            owner.push('.');
            owner.push_str(key);
        }
        false
    }

    fn reads_member(&self, object: &str, key: &str, read: Read) -> bool {
        let owners = py_tables::method_owners(object, key);
        self.writes.iter().any(|write| match &write.slot {
            Slot::Member {
                object: written,
                key: member,
            } if write.reaches(read) => {
                let written = py_tables::class_object(written);
                if owners.is_empty() {
                    written == object && (member == key || member == "__class__")
                } else {
                    (member == key && owners.contains(&written))
                        || (member == "__bases__" && owners[..owners.len() - 1].contains(&written))
                }
            }
            _ => false,
        })
    }

    pub(super) fn read(&self, at: Node<'tree>, scope: ScopeId) -> Read {
        let suite = self.walk.scopes[scope].body;
        let statement = std::iter::successors(Some(at), |node| node.parent())
            .take_while(|node| *node != suite)
            .last();
        let direct = statement.is_some_and(|stmt| {
            !matches!(
                stmt.kind(),
                "if_statement"
                    | "for_statement"
                    | "while_statement"
                    | "try_statement"
                    | "match_statement"
                    | "with_statement"
            )
        });
        Read {
            scope,
            byte: at.start_byte(),
            direct,
        }
    }
}
