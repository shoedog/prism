//! Guard predicates on the value that will receive the store.
use super::flow::inside;
use super::*;

mod header;

impl<'a, F: FnMut(usize) -> bool> Proof<'a, '_, F> {
    pub(super) fn receiver_guard(
        &mut self,
        input: &GuardInput<'a>,
    ) -> (GuardState, Vec<GuardForm>) {
        let edges = self.route(input.store);
        let mut atoms = Vec::new();
        for edge in edges {
            self.true_atoms(edge, &mut atoms);
        }
        let mut partial = false;
        for atom in atoms {
            if let Some(state) = self.identity_set(input, atom) {
                if state == GuardState::Sufficient {
                    return (state, vec![GuardForm::ReceiverIdentitySet]);
                }
                partial |= state == GuardState::Partial;
            }
            if let Some(state) = self.private_brand(input, atom) {
                if state == GuardState::Sufficient {
                    return (state, vec![GuardForm::ReceiverPrivateBrand]);
                }
                partial |= state == GuardState::Partial;
            }
        }
        if self.header_redirect(input) {
            return (GuardState::Sufficient, vec![GuardForm::HeaderScratch]);
        }
        if input
            .steps
            .iter()
            .any(|step| self.one_identity(input, *step))
        {
            partial = true;
        }
        let state = if partial {
            GuardState::Partial
        } else {
            GuardState::None
        };
        (state, Vec::new())
    }

    fn identity_set(&mut self, input: &GuardInput<'a>, atom: Node<'a>) -> Option<GuardState> {
        let (collection, "has") = self.method(atom)? else {
            return None;
        };
        let [checked] = self.args(atom)?;
        let constructor = self.initializer(collection)?;
        let native = field(constructor, "constructor").map(peel)?;
        if constructor.kind() != "new_expression" || self.text(native) != "WeakSet" {
            self.refuse(atom, "unsupported_predicate");
            return None;
        }
        if !self.same_key(checked, input.receiver) {
            self.refuse(atom, "wrong_binding");
            return None;
        }
        let selected = self.initializer(input.receiver)?;
        let [step] = input.steps.as_slice() else {
            return Some(GuardState::Partial);
        };
        if selected != step.effect || self.owner(selected) != Some(input.owner) {
            self.refuse(atom, "wrong_version");
            return Some(GuardState::Partial);
        }
        let [entries] = self.args(constructor)?;
        let table = self.initializer(step.object)?;
        let native_collection = self.native(atom, native, &["WeakSet", "prototype", "has"])
            && self.native(atom, native, &["WeakSet", "prototype", "add"])
            && self.unwritten(atom, &["Array", "prototype", "@@iterator"]);
        let qualifies = native_collection
            && entries.kind() == "array"
            && entries.named_child_count() > 0
            && self.dense(entries)
            && table.kind() == "object"
            && self.only_references(collection, |proof, read| {
                read.parent().is_some_and(|member| {
                    field(member, "object") == Some(read)
                        && member.parent().is_some_and(|call| {
                            field(call, "function") == Some(member)
                                && proof
                                    .method(call)
                                    .is_some_and(|(_, method)| method == "has")
                                && proof.args::<1>(call).is_some()
                        })
                })
            })
            && self.only_references(step.object, |proof, read| {
                inside(step.effect, read) || proof.terminal_value(read, input.owner)
            })
            && children(entries).all(|entry| {
                self.initializer(entry)
                    .is_some_and(|value| self.fresh_empty(value))
                    && self.only_references(entry, |proof, read| {
                        inside(table, read)
                            || inside(entries, read)
                            || proof.terminal_value(read, input.owner)
                    })
            });
        if qualifies {
            Some(GuardState::Sufficient)
        } else {
            self.refuse(atom, "unproved_child");
            Some(GuardState::Partial)
        }
    }

    fn private_brand(&mut self, input: &GuardInput<'a>, atom: Node<'a>) -> Option<GuardState> {
        if atom.kind() != "binary_expression" || operator(atom, self.file) != "in" {
            return None;
        }
        let [brand, checked] = self.binary(atom)?;
        if brand.kind() != "private_property_identifier" {
            return None;
        }
        if !self.same_key(checked, input.receiver) {
            self.refuse(atom, "wrong_binding");
            return None;
        }
        let [step] = input.steps.as_slice() else {
            return Some(GuardState::Partial);
        };
        if self.initializer(input.receiver) != Some(step.effect) {
            self.refuse(atom, "wrong_version");
            return Some(GuardState::Partial);
        }
        let owner = input.owner;
        let body = owner.parent().filter(|body| body.kind() == "class_body")?;
        let class = body
            .parent()
            .filter(|class| class.kind() == "class_declaration")?;
        let name = field(class, "name")?;
        let method = field(owner, "name")?;
        let brand_name = self.text(brand);
        let class_is_closed = owner.kind() == "method_definition"
            && super::super::has_token(owner, "static")
            && children(class).all(|part| part == name || part == body)
            && children(body).any(|part| {
                field(part, "name")
                    .or_else(|| field(part, "property"))
                    .is_some_and(|field| self.text(field) == brand_name)
                    && field(part, "value").is_none()
                    && !super::super::has_token(part, "static")
            })
            && children(body).all(|part| {
                part == owner
                    || matches!(part.kind(), "field_definition" | "public_field_definition")
                        && field(part, "value").is_none()
            });
        if class_is_closed && self.class_uses(class, name, method) {
            Some(GuardState::Sufficient)
        } else {
            self.refuse(atom, "unproved_child");
            Some(GuardState::Partial)
        }
    }

    fn class_uses(&mut self, class: Node<'a>, name: Node<'a>, method: Node<'a>) -> bool {
        let Some(owner) = class.parent().and_then(|parent| self.owner(parent)) else {
            return false;
        };
        let mut nodes = vec![owner];
        let mut construction = false;
        let mut invocation = false;
        let id = AstSpan::from(name);
        let mut reference = None;
        while let Some(node) = nodes.pop() {
            if !self.spend(node.named_child_count() + 1) {
                return false;
            }
            if identifier(node)
                && node != name
                && self.text(node) == self.text(name)
                && self
                    .analysis
                    .read(node)
                    .and_then(|read| read.binding.as_ref())
                    == Some(&id)
            {
                reference = Some(node);
                let Some(parent) = node.parent() else {
                    return false;
                };
                if parent.kind() == "new_expression"
                    && field(parent, "constructor") == Some(node)
                    && self.args::<0>(parent).is_some()
                {
                    construction = true;
                } else if parent.kind() == "member_expression"
                    && field(parent, "object") == Some(node)
                    && field(parent, "property")
                        .is_some_and(|name| self.text(name) == self.text(method))
                    && parent.parent().is_some_and(|call| {
                        field(call, "function") == Some(parent) && self.args::<2>(call).is_some()
                    })
                {
                    invocation = true;
                } else {
                    return false;
                }
            }
            nodes.extend(children(node));
        }
        construction
            && invocation
            && reference.is_some_and(|node| {
                self.declaration(node).is_some_and(|(binding, _)| {
                    self.analysis
                        .proof_writes(&binding)
                        .is_some_and(|writes| writes.len() == 1)
                })
            })
    }

    fn one_identity(&mut self, input: &GuardInput<'a>, step: Selection<'a>) -> bool {
        let Some((binding, _)) = self.declaration(input.receiver) else {
            return false;
        };
        let mut node = step.effect;
        let mut cursor = None;
        while let Some(parent) = node.parent() {
            if !self.spend(1) {
                return false;
            }
            if parent.kind() == "assignment_expression" && field(parent, "right") == Some(node) {
                let left = peel(field(parent, "left").unwrap_or(node));
                if identifier(left) {
                    cursor = Some(left);
                }
            }
            if parent.kind() == "expression_statement" || is_owner(parent) {
                node = parent;
                break;
            }
            node = parent;
        }
        let Some(cursor) = cursor else { return false };
        let Some(check) = node
            .next_named_sibling()
            .filter(|node| node.kind() == "if_statement")
        else {
            return false;
        };
        let Some(condition) = field(check, "condition").map(peel) else {
            return false;
        };
        let Some([left, right]) = self.binary(condition) else {
            return false;
        };
        if operator(condition, self.file) != "==="
            || !self.is_object_prototype(right)
            || !field(check, "consequence").is_some_and(|arm| arm.kind() == "return_statement")
        {
            return false;
        }
        let Some(read) = self.analysis.read(left) else {
            return false;
        };
        if read.binding.as_ref() == Some(&binding)
            && read.reaching_versions.as_slice() == [AstSpan::from(cursor)]
        {
            self.refuse(check, "incomplete_coverage");
            return true;
        }
        false
    }
}
