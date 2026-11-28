//! Finite accepting domains for keys and paths.
use super::flow::{inside, Edge};
use super::names::RESERVED;
use super::*;

mod functions;

impl<'a, F: FnMut(usize) -> bool> Proof<'a, '_, F> {
    pub(super) fn finite_guard(
        &mut self,
        input: &GuardInput<'a>,
        name_steps: &[bool],
    ) -> (GuardState, Vec<GuardForm>) {
        let mut forms = BTreeSet::new();
        let mut all = !input.steps.is_empty();
        let mut partial = false;
        for (step, &name_proved) in input.steps.iter().zip(name_steps) {
            if name_proved {
                forms.insert(GuardForm::DenyNames);
                continue;
            }
            let edges = self.route(step.effect);
            let (mut relevant, mut form) = self.finite_step(*step, &edges);
            if form.is_none() && self.assertion(*step) {
                form = Some(GuardForm::AllowAssertion);
            }
            if form.is_none() && input.steps.len() == 1 && self.literal_after_lookup(input, *step) {
                form = Some(GuardForm::AllowLiteralAfterLookup);
            }
            relevant |= form.is_some();
            let shared = relevant && (step.shared_link || self.shared_receiver(step.object));
            if let Some(form) = form.filter(|_| !shared) {
                forms.insert(form);
            } else {
                all = false;
                if relevant && shared {
                    self.refuse(step.effect, "unproved_child");
                }
            }
            partial |= relevant;
        }
        if all && !self.shared_receiver(input.receiver) {
            (GuardState::Sufficient, forms.into_iter().collect())
        } else if partial {
            (GuardState::Partial, Vec::new())
        } else {
            (GuardState::None, Vec::new())
        }
    }

    fn finite_step(
        &mut self,
        step: Selection<'a>,
        edges: &[Edge<'a>],
    ) -> (bool, Option<GuardForm>) {
        let mut candidates = Vec::new();
        for edge in edges.iter() {
            self.true_atoms(*edge, &mut candidates);
        }
        for call in candidates {
            let Some((receiver, "has")) = self.method(call) else {
                if call.kind() == "call_expression"
                    && field(call, "function").is_some_and(|callee| identifier(peel(callee)))
                    && self
                        .args::<1>(call)
                        .is_some_and(|[value]| self.same_key(value, step.key))
                {
                    self.refuse(call, "unsupported_predicate");
                }
                continue;
            };
            let Some([value]) = self.args(call) else {
                continue;
            };
            if !self.same_key(value, step.key) {
                continue;
            }
            let Some(allocation) = self.initializer(receiver).filter(|allocation| {
                allocation.kind() == "new_expression"
                    && field(*allocation, "constructor")
                        .map(peel)
                        .is_some_and(|constructor| self.text(constructor) == "Set")
            }) else {
                continue;
            };
            if self.closed_set(receiver, call, allocation) {
                return (true, Some(GuardForm::AllowSet));
            }
            self.refuse(call, "unproved_native_identity");
            return (true, None);
        }
        if let Some(proved) = self.exact_path(step) {
            return (true, proved.then_some(GuardForm::AllowExactPath));
        }
        if self.normalised(step, edges) {
            return (true, Some(GuardForm::NormaliseAllowList));
        }
        (false, None)
    }

    fn closed_set(&mut self, receiver: Node<'a>, use_site: Node<'a>, value: Node<'a>) -> bool {
        let Some(constructor) = field(value, "constructor")
            .map(peel)
            .filter(|node| identifier(*node))
        else {
            return false;
        };
        let Some([items]) = self.args(value) else {
            return false;
        };
        items.kind() == "array"
            && items.named_child_count() > 0
            && children(items).all(|entry| {
                self.spend(1)
                    && self
                        .string(entry)
                        .is_some_and(|entry| !RESERVED.contains(&entry))
            })
            && self.dense(items)
            && self.native(use_site, constructor, &["Set", "prototype", "has"])
            && self.native(use_site, constructor, &["Set", "prototype", "add"])
            && self.unwritten(use_site, &["Array", "prototype", "@@iterator"])
            && self.only_references(receiver, |this, read| {
                let Some(member) = read.parent().filter(|member| {
                    member.kind() == "member_expression" && field(*member, "object") == Some(read)
                }) else {
                    return false;
                };
                member.parent().is_some_and(|call| {
                    field(call, "function") == Some(member)
                        && this.method(call).is_some_and(|(_, method)| method == "has")
                        && this.args::<1>(call).is_some()
                })
            })
    }

    pub(super) fn dense(&mut self, array: Node<'a>) -> bool {
        let mut cursor = array.walk();
        let mut element = false;
        for child in array.children(&mut cursor) {
            if !self.spend(1) {
                return false;
            }
            match child.kind() {
                "," if !element => return false,
                "," => element = false,
                _ if child.is_named() && !child.is_extra() => element = true,
                _ => {}
            }
        }
        true
    }

    fn exact_path(&mut self, step: Selection<'a>) -> Option<bool> {
        let [source] = self.sequence_sources(step.key).try_into().ok()?;
        let (path, "split") = self.method(source)? else {
            return None;
        };
        let [separator] = self.args(source)?;
        let delimiter = self.string(separator)?;
        if delimiter.is_empty() || !identifier(path) {
            return None;
        }
        let routes = self.route(source);
        for route in routes {
            let Some(allowed) =
                self.finite_strings(route, |proof, candidate| proof.same_key(candidate, path))
            else {
                if self
                    .finite_strings(route, |proof, candidate| {
                        proof.same_binding(candidate, path)
                    })
                    .is_some()
                {
                    self.refuse(route.guard, "wrong_version");
                }
                continue;
            };
            if allowed
                .iter()
                .any(|value| value.split(delimiter).any(|key| RESERVED.contains(&key)))
            {
                continue;
            }
            return Some(
                self.unwritten(source, &["String", "prototype", "split"])
                    && self.unwritten(source, &["String", "prototype", "@@split"])
                    && self.unwritten(source, &["Array", "prototype", "@@iterator"]),
            );
        }
        None
    }

    fn literal_after_lookup(&mut self, input: &GuardInput<'a>, step: Selection<'a>) -> bool {
        if step.effect.kind() != "subscript_expression"
            || self.initializer(input.receiver) != Some(step.effect)
            || self.owner(step.effect) != Some(input.owner)
        {
            return false;
        }
        for edge in self.route(input.store) {
            if edge.guard.start_byte() < step.effect.end_byte() {
                continue;
            }
            let Some(values) =
                self.finite_strings(edge, |proof, value| proof.same_key(value, step.key))
            else {
                continue;
            };
            let Some(table) = self
                .initializer(step.object)
                .filter(|table| table.kind() == "object")
            else {
                self.refuse(edge.guard, "unproved_child");
                return false;
            };
            if values.iter().all(|key| self.fresh_literal_slot(table, key))
                && self.only_references(step.object, |_, read| {
                    inside(step.effect, read)
                        || read.parent().is_some_and(|member| {
                            member.kind() == "member_expression"
                                && field(member, "object") == Some(read)
                                && member.parent().is_some_and(|exit| {
                                    exit.kind() == "return_statement"
                                        && (read.start_byte() > input.store.end_byte()
                                            || field(
                                                edge.guard,
                                                if edge.accepts {
                                                    "alternative"
                                                } else {
                                                    "consequence"
                                                },
                                            )
                                            .is_some_and(|rejected| inside(rejected, exit)))
                                })
                        })
                })
            {
                return true;
            }
            self.refuse(edge.guard, "unproved_child");
        }
        false
    }

    fn fresh_literal_slot(&mut self, object: Node<'a>, key: &str) -> bool {
        let mut found = false;
        for part in children(object) {
            if !self.spend(1) {
                return false;
            }
            let Some(name) = self.pair_name(part) else {
                return false;
            };
            if name == key {
                if found
                    || !field(part, "value")
                        .is_some_and(|value| matches!(peel(value).kind(), "object" | "array"))
                {
                    return false;
                }
                found = true;
            }
        }
        found
    }

    pub(super) fn pair_name(&self, pair: Node<'a>) -> Option<&'a str> {
        if pair.kind() == "shorthand_property_identifier" {
            return Some(self.text(pair));
        }
        if pair.kind() != "pair" {
            return None;
        }
        let key = field(pair, "key")?;
        if matches!(key.kind(), "property_identifier" | "number") {
            return Some(self.text(key));
        }
        self.string(key)
    }

    pub(super) fn only_references(
        &mut self,
        reference: Node<'a>,
        mut allow: impl FnMut(&mut Self, Node<'a>) -> bool,
    ) -> bool {
        let Some((binding, declaration)) = self.bare_declaration(reference) else {
            return false;
        };
        let Some(owner_span) = self.bindings.binding_owner(&binding) else {
            return false;
        };
        let Some(owner) = self
            .file
            .tree
            .root_node()
            .descendant_for_byte_range(owner_span.start_byte, owner_span.end_byte)
        else {
            return false;
        };
        if self
            .bindings
            .proof_writes(&binding)
            .is_none_or(|writes| writes.len() != 1)
        {
            return false;
        }
        let name = self.bindings.canonical(self.text(declaration));
        let mut nodes = vec![owner];
        while let Some(node) = nodes.pop() {
            if !self.spend(node.named_child_count() + 1) {
                return false;
            }
            if identifier(node)
                && node != declaration
                && self.bindings.canonical(self.text(node)) == name
            {
                let Some(scope) = std::iter::successors(Some(node), Node::parent)
                    .take(128)
                    .find(|scope| is_scope(*scope))
                else {
                    return false;
                };
                if self
                    .bindings
                    .lexical_entry(scope, node, name)
                    .and_then(Option::as_ref)
                    == Some(&binding)
                    && !allow(self, node)
                {
                    return false;
                }
            }
            nodes.extend(children(node));
        }
        true
    }

    pub(super) fn true_atoms(&mut self, edge: Edge<'a>, out: &mut Vec<Node<'a>>) {
        let mut pending = vec![(edge.condition, edge.accepts)];
        while let Some((node, accepts)) = pending.pop() {
            if !self.spend(1) {
                return;
            }
            let node = peel(node);
            let op = operator(node, self.file);
            if node.kind() == "unary_expression" && op == "!" {
                pending.extend(field(node, "argument").map(|arg| (arg, !accepts)));
            } else if matches!((op, accepts), ("&&", true) | ("||", false)) {
                pending.extend(
                    self.binary(node)
                        .into_iter()
                        .flatten()
                        .map(|child| (child, accepts)),
                );
            } else if accepts {
                out.push(node);
            }
        }
    }

    pub(super) fn finite_strings(
        &mut self,
        edge: Edge<'a>,
        mut allowed_key: impl FnMut(&mut Self, Node<'a>) -> bool,
    ) -> Option<BTreeSet<&'a str>> {
        let mut allowed = BTreeSet::new();
        let mut pending = vec![(edge.condition, edge.accepts)];
        while let Some((node, accepts)) = pending.pop() {
            if !self.spend(1) {
                return None;
            }
            let node = peel(node);
            let op = operator(node, self.file);
            if node.kind() == "unary_expression" && op == "!" {
                pending.push((field(node, "argument")?, !accepts));
                continue;
            }
            let [left, right] = self.binary(node)?;
            if matches!((op, accepts), ("||", true) | ("&&", false)) {
                pending.extend([(left, accepts), (right, accepts)]);
                continue;
            }
            if !matches!((op, accepts), ("===", true) | ("!==", false)) {
                return None;
            }
            let (value, key) = self
                .string(right)
                .map(|name| (left, name))
                .or_else(|| self.string(left).map(|name| (right, name)))?;
            if RESERVED.contains(&key) || !allowed_key(self, value) {
                return None;
            }
            allowed.insert(key);
        }
        (!allowed.is_empty()).then_some(allowed)
    }
}
