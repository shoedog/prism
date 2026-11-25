//! Ordinary-code receiver exclusions. Every admitted child is local data.
use super::*;

impl<'a> Classifier<'a, '_> {
    pub(super) fn excluded_step(
        &self,
        step: Selection<'a>,
        target: Node<'a>,
        store: Node<'a>,
    ) -> bool {
        if step.shared_link || !self.proof_charge(1) {
            return false;
        }
        (self.benign_key(step.key) && self.no_visible_shared_slot(step.object))
            || self.installed_slot(step, target)
            || self.private_table_child(step, target, store)
            || self.private_null_walk(step, target)
    }

    fn no_visible_shared_slot(&self, receiver: Node<'a>) -> bool {
        let receiver = peel(receiver);
        let Some(binding) = self
            .analysis
            .read(receiver)
            .and_then(|read| read.binding.as_ref())
        else {
            return self.no_prototype_value(receiver, 0);
        };
        if let Some(initial) = self.initializer(receiver) {
            if !self.no_prototype_value(initial, 0) {
                return false;
            }
        }
        let Some(references) = self.binding_references(binding) else {
            return false;
        };
        references.into_iter().all(|reference| {
            let Some(parent) = self.expression_use(reference) else {
                return false;
            };
            let container = if matches!(parent.kind(), "member_expression" | "subscript_expression")
            {
                self.expression_use(parent).unwrap_or(parent)
            } else if parent.kind() == "arguments" {
                parent.parent().unwrap_or(parent)
            } else {
                parent
            };
            match container.kind() {
                "variable_declarator" if field(container, "value").map(peel) == Some(reference) => {
                    false
                }
                "assignment_expression" | "augmented_assignment_expression" => {
                    field(container, "right").is_some_and(|right| self.no_prototype_value(right, 0))
                }
                "call_expression" if parent.kind() == "arguments" => {
                    children(parent).all(|arg| arg == reference || self.no_prototype_value(arg, 0))
                }
                _ => true,
            }
        })
    }

    fn no_prototype_value(&self, node: Node<'a>, depth: usize) -> bool {
        if depth == 24 || !self.proof_charge(1) {
            return false;
        }
        let node = peel(node);
        match node.kind() {
            "member_expression" | "subscript_expression" => {
                !self.shared_link(node)
                    && field(node, "object")
                        .is_some_and(|base| self.no_prototype_value(base, depth + 1))
            }
            "identifier" => self
                .initializer(node)
                .is_none_or(|value| self.no_prototype_value(value, depth + 1)),
            "object"
            | "array"
            | "pair"
            | "spread_element"
            | "ternary_expression"
            | "binary_expression"
            | "assignment_expression"
            | "sequence_expression" => {
                children(node).all(|child| self.no_prototype_value(child, depth + 1))
            }
            "call_expression" => {
                if self.static_call(node, "Object", "getPrototypeOf")
                    || self.static_call(node, "Reflect", "getPrototypeOf")
                {
                    return false;
                }
                field(node, "arguments").is_some_and(|args| {
                    children(args).all(|arg| self.no_prototype_value(arg, depth + 1))
                })
            }
            _ => true,
        }
    }

    fn installed_slot(&self, step: Selection<'a>, target: Node<'a>) -> bool {
        let base = peel(step.object);
        let key = self.primitive_identity(&AstSpan::from(step.key));
        let Some(key) = key else { return false };
        let Some(initial) = self.initializer(base) else {
            return false;
        };
        if initial.kind() != "object" && !self.null_root(initial) {
            return false;
        }
        let Some(lookup) = self.expression_use(base) else {
            return false;
        };
        if field(target, "object").map(peel) != Some(lookup)
            || field(lookup, "index").map(peel) != Some(step.key)
        {
            return false;
        }
        let Some(statement) = self.enclosing(target, |kind| {
            kind == "expression_statement" || kind == "statement_block"
        }) else {
            return false;
        };
        if statement.kind() != "expression_statement"
            || step.key.start_byte() < statement.start_byte()
        {
            return false;
        }
        let Some(previous) = statement
            .prev_named_sibling()
            .filter(|node| node.kind() == "expression_statement")
        else {
            return false;
        };
        let Some(installation) = children(previous).next().map(peel) else {
            return false;
        };
        if !self.static_call(installation, "Object", "defineProperty") {
            return false;
        }
        let Some([install_base, install_key, descriptor]) = self.args(installation) else {
            return false;
        };
        let Some(value) = self.descriptor_value(peel(descriptor)) else {
            return false;
        };
        self.same_value(peel(install_base), base)
            && self.primitive_identity(&AstSpan::from(peel(install_key))) == Some(key)
            && self.private_literal(value, 0)
    }

    fn descriptor_value(&self, descriptor: Node<'a>) -> Option<Node<'a>> {
        if descriptor.kind() != "object" {
            return None;
        }
        let mut value = None;
        for pair in children(descriptor) {
            if !self.proof_charge(1) || pair.kind() != "pair" {
                return None;
            }
            let rhs = field(pair, "value")?;
            match self.pair_name(pair)? {
                "value" if value.is_none() => value = Some(rhs),
                "enumerable" | "configurable" | "writable"
                    if matches!(peel(rhs).kind(), "true" | "false") => {}
                _ => return None,
            }
        }
        value
    }

    fn private_table_child(&self, step: Selection<'a>, target: Node<'a>, store: Node<'a>) -> bool {
        let base = peel(step.object);
        if !identifier(base) {
            return false;
        }
        let Some(initial) = self.initializer(base) else {
            return false;
        };
        let descriptors = self.static_call(initial, "Object", "getOwnPropertyDescriptors");
        let grouped = self.static_call(initial, "Object", "groupBy");
        let table = self.null_root(initial) || self.private_null_initializer(initial);
        let own_children = !table && !grouped && !descriptors && self.table_literal(initial, false);
        if !table && !grouped && !descriptors && !own_children {
            return false;
        }
        if (descriptors || own_children) && !self.own_enumerated_key(step.key, base) {
            return false;
        }
        let Some(binding) = self
            .analysis
            .read(base)
            .and_then(|read| read.binding.as_ref())
        else {
            return false;
        };
        let Some(references) = self.binding_references(binding) else {
            return false;
        };
        let Some((owner, limit)) = self.repeat_limit(store) else {
            return false;
        };
        references.into_iter().all(|reference| {
            self.enclosing(reference, |kind| {
                kind == "program" || crate::ast::is_js_ts_function_like(kind)
            }) == Some(owner)
                && (reference.start_byte() >= limit
                    || self.private_table_reference(reference, initial, target, table))
        })
    }

    fn repeat_limit(&self, target: Node<'a>) -> Option<(Node<'a>, usize)> {
        let mut node = target;
        let mut limit = target.end_byte();
        for _ in 0..RECEIVER_DEPTH_LIMIT {
            if !self.proof_charge(1) {
                return None;
            }
            node = node.parent()?;
            if super::super::is_loop(node) {
                limit = node.end_byte()
            }
            if is_owner(node) {
                return Some((node, limit));
            }
        }
        None
    }

    fn private_table_reference(
        &self,
        reference: Node<'a>,
        initial: Node<'a>,
        target: Node<'a>,
        permits_slots: bool,
    ) -> bool {
        let Some(usage) = self.expression_use(reference) else {
            return false;
        };
        if usage.kind() == "assignment_expression"
            && field(usage, "left").map(peel) == Some(reference)
        {
            return field(usage, "right").map(peel) == Some(initial);
        }
        if usage.kind() == "return_statement" {
            return true;
        }
        if usage.kind() == "binary_expression"
            && operator(usage, self.file) == "in"
            && field(usage, "right").map(peel) == Some(reference)
        {
            return true;
        }
        if usage.kind() == "arguments" {
            return usage.parent().is_some_and(|call| {
                self.static_call(call, "Object", "keys")
                    || self.static_call(call, "Object", "entries") && self.key_only_entries(call)
            });
        }
        if !matches!(usage.kind(), "member_expression" | "subscript_expression")
            || field(usage, "object").map(peel) != Some(reference)
        {
            return false;
        }
        let Some(next) = self.expression_use(usage) else {
            return false;
        };
        if next.kind() == "assignment_expression" && field(next, "left").map(peel) == Some(usage) {
            return permits_slots
                && field(next, "right").is_some_and(|rhs| self.private_literal(rhs, 0));
        }
        if matches!(next.kind(), "member_expression" | "subscript_expression")
            && field(next, "object").map(peel) == Some(usage)
        {
            return self.private_child_use(next, target, permits_slots);
        }
        match next.kind() {
            "unary_expression" => operator(next, self.file) == "!",
            "if_statement" => field(next, "condition").map(peel) == Some(usage),
            "binary_expression" => {
                matches!(operator(next, self.file), "==" | "!=" | "===" | "!==")
                    || operator(next, self.file) == "in"
                        && field(next, "right").map(peel) == Some(usage)
            }
            _ => false,
        }
    }

    fn private_child_use(
        &self,
        mut access: Node<'a>,
        target: Node<'a>,
        permits_push: bool,
    ) -> bool {
        for _ in 0..RECEIVER_DEPTH_LIMIT {
            if access == target {
                return true;
            }
            let Some(parent) = self.expression_use(access) else {
                return false;
            };
            match parent.kind() {
                "member_expression" | "subscript_expression"
                    if field(parent, "object").map(peel) == Some(access) =>
                {
                    access = parent
                }
                "assignment_expression" => {
                    return field(parent, "left").map(peel) == Some(access)
                        && field(parent, "right").is_some_and(|rhs| self.private_literal(rhs, 0));
                }
                "binary_expression" => {
                    return operator(parent, self.file) == "in"
                        && field(parent, "right").map(peel) == Some(access);
                }
                "call_expression" if permits_push => {
                    return field(parent, "function").map(peel) == Some(access)
                        && field(access, "property")
                            .is_some_and(|property| self.text(property) == "push");
                }
                _ => return false,
            }
        }
        false
    }

    fn own_enumerated_key(&self, key: Node<'a>, base: Node<'a>) -> bool {
        let Some(read) = self
            .analysis
            .read(key)
            .filter(|read| read.resolution_complete)
        else {
            return false;
        };
        let [site] = read.reaching_versions.as_slice() else {
            return false;
        };
        let Origin::Element(keys) = self.origin(site) else {
            return false;
        };
        let Some(binding) = self.node_at(site) else {
            return false;
        };
        let keys = peel(keys);
        let direct = self.static_call(keys, "Object", "keys")
            && binding.parent().is_some_and(|loop_node| {
                loop_node.kind() == "for_in_statement"
                    && field(loop_node, "left") == Some(binding)
                    && field(loop_node, "right").map(peel) == Some(keys)
                    && field(loop_node, "operator").is_some_and(|op| self.text(op) == "of")
            });
        let entries = self.static_call(keys, "Object", "entries")
            && self.key_only_entries(keys)
            && binding.parent().is_some_and(|pattern| {
                pattern.kind() == "array_pattern"
                    && super::super::array_positions(pattern).next() == Some((0, binding))
            });
        (direct || entries)
            && self
                .args::<1>(keys)
                .is_some_and(|[receiver]| self.same_value(peel(receiver), base))
    }

    fn key_only_entries(&self, call: Node<'a>) -> bool {
        let Some(loop_node) = self.expression_use(call) else {
            return false;
        };
        if loop_node.kind() != "for_in_statement"
            || field(loop_node, "right").map(peel) != Some(call)
            || !field(loop_node, "operator").is_some_and(|op| self.text(op) == "of")
        {
            return false;
        }
        let Some(pattern) = field(loop_node, "left").filter(|node| node.kind() == "array_pattern")
        else {
            return false;
        };
        let mut parts = children(pattern);
        let Some(key) = parts.next().filter(|node| identifier(*node)) else {
            return false;
        };
        parts.next().is_none() && super::super::array_positions(pattern).next() == Some((0, key))
    }

    fn null_root(&self, node: Node<'a>) -> bool {
        let node = peel(node);
        self.static_call(node, "Object", "create")
            && self
                .args::<1>(node)
                .is_some_and(|[arg]| peel(arg).kind() == "null")
    }

    fn private_null_initializer(&self, node: Node<'a>) -> bool {
        if self.static_call(node, "Object", "assign") {
            return self.args::<2>(node).is_some_and(|[base, init]| {
                self.null_root(base) && self.table_literal(peel(init), false)
            });
        }
        self.table_literal(node, true)
    }

    fn table_literal(&self, object: Node<'a>, null_literal: bool) -> bool {
        if object.kind() != "object" {
            return false;
        }
        let mut null_proto = false;
        for pair in children(object) {
            if !self.proof_charge(1) || pair.kind() != "pair" {
                return false;
            }
            let (Some(name), Some(value)) = (self.pair_name(pair), field(pair, "value")) else {
                return false;
            };
            if name == "__proto__" && null_literal && value.kind() == "null" && !null_proto {
                null_proto = true;
            } else if name == "__proto__" || !self.private_literal(value, 0) {
                return false;
            }
        }
        !null_literal || null_proto
    }

    fn private_literal(&self, value: Node<'a>, depth: usize) -> bool {
        if depth == 24 || !self.proof_charge(1) {
            return false;
        }
        let value = peel(value);
        if self.null_root(value) {
            return true;
        }
        let data = |node| {
            let node = peel(node);
            matches!(node.kind(), "null" | "string" | "number" | "true" | "false")
                || self.private_literal(node, depth + 1)
        };
        match value.kind() {
            "array" => children(value).all(data),
            "object" => children(value).all(|pair| {
                pair.kind() == "pair"
                    && field(pair, "value").is_some_and(|value| {
                        (if self.pair_name(pair) == Some("__proto__") {
                            peel(value).kind() == "null"
                        } else {
                            self.pair_name(pair).is_some()
                                || field(pair, "key")
                                    .is_some_and(|key| key.kind() == "computed_property_name")
                        }) && data(value)
                    })
            }),
            _ => false,
        }
    }

    fn private_null_walk(&self, step: Selection<'a>, target: Node<'a>) -> bool {
        self.null_walk(step, target).is_some()
    }

    fn null_walk(&self, step: Selection<'a>, target: Node<'a>) -> Option<()> {
        let cursor = peel(step.object);
        let binding = self.analysis.read(cursor)?.binding.as_ref()?;
        let lookup = self.expression_use(cursor)?;
        if lookup.kind() != "subscript_expression"
            || field(lookup, "index").map(peel) != Some(step.key)
        {
            return None;
        }
        let advance = self.expression_use(lookup)?;
        let next = field(advance, "left").map(peel)?;
        if advance.kind() != "assignment_expression"
            || field(advance, "right").map(peel) != Some(lookup)
            || self.binding_of(next).as_ref() != Some(binding)
        {
            return None;
        }
        let advance = self
            .expression_use(advance)?
            .parent()
            .filter(|node| node.kind() == "statement_block")?;
        let loop_node = advance
            .parent()
            .filter(|node| node.kind() == "for_in_statement")?;
        let final_receiver = field(target, "object").map(peel)?;
        if self.analysis.read(final_receiver)?.binding.as_ref() != Some(binding)
            || target.start_byte() < loop_node.end_byte()
        {
            return None;
        }
        let [guard, movement]: [Node<'a>; 2] = children(advance)
            .take(3)
            .collect::<Vec<_>>()
            .try_into()
            .ok()?;
        if movement.start_byte() > lookup.start_byte()
            || movement.end_byte() < lookup.end_byte()
            || guard.kind() != "if_statement"
            || field(guard, "alternative").is_some()
        {
            return None;
        }
        let condition = peel(field(guard, "condition")?);
        let checked = peel(field(condition, "left")?);
        if operator(condition, self.file) != "=="
            || peel(field(condition, "right")?).kind() != "null"
        {
            return None;
        }
        let install = peel(children(field(guard, "consequence")?).next()?);
        let slot = peel(field(install, "left")?);
        if install.kind() != "assignment_expression" || !self.null_root(field(install, "right")?) {
            return None;
        }
        for selection in [slot, checked] {
            let base = field(selection, "object").map(peel)?;
            let key = field(selection, "index").map(peel)?;
            if self.analysis.read(base)?.binding.as_ref() != Some(binding)
                || !self.same_value(key, step.key)
            {
                return None;
            }
        }
        let decl = self.node_at(binding)?;
        let root = field(decl.parent()?, "value").map(peel)?;
        if !self.null_root(self.initializer(root)?) {
            return None;
        }
        let root_binding = self.analysis.read(root)?.binding.as_ref()?;
        if !self
            .binding_references(root_binding)?
            .into_iter()
            .all(|reference| {
                reference == root
                    || reference.start_byte() >= target.end_byte()
                        && self
                            .expression_use(reference)
                            .is_some_and(|parent| parent.kind() == "return_statement")
            })
        {
            return None;
        }
        let allowed = [
            cursor,
            next,
            final_receiver,
            field(slot, "object").map(peel)?,
            field(checked, "object").map(peel)?,
        ];
        self.binding_references(binding)?
            .into_iter()
            .all(|reference| allowed.contains(&reference))
            .then_some(())
    }
}
