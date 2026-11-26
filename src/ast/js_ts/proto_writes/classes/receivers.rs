//! Receiver-step exclusions from a bounded key or a completed own-slot installation.
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
        self.installed_slot(step, target, store)
            || self.benign_key(step.key) && self.bounded_key_receiver(step.object)
    }

    fn installed_slot(&self, step: Selection<'a>, target: Node<'a>, store: Node<'a>) -> bool {
        if store.kind() != "assignment_expression" || field(store, "left").map(peel) != Some(target)
        {
            return false;
        }
        let base = peel(step.object);
        let key = self.primitive_identity(&AstSpan::from(step.key), false);
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
        let Some(statement) = self
            .expression_use(store)
            .filter(|node| node.kind() == "expression_statement")
        else {
            return false;
        };
        let Some(previous) = statement
            .prev_named_sibling()
            .filter(|node| node.kind() == "expression_statement")
        else {
            return false;
        };
        let Some(installation) = children(previous).next().map(peel) else {
            return false;
        };
        if !self.static_call(installation, "Object", "defineProperty")
            || self.analysis.explicit_define_property_write()
        {
            return false;
        }
        let Some([install_base, install_key, descriptor]) = self.args(installation) else {
            return false;
        };
        let Some(value) = self.descriptor_value(peel(descriptor)) else {
            return false;
        };
        self.same_value(peel(install_base), base)
            && self.primitive_identity(&AstSpan::from(peel(install_key)), false) == Some(key)
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

    fn null_root(&self, node: Node<'a>) -> bool {
        let node = peel(node);
        self.static_call(node, "Object", "create")
            && self
                .args::<1>(node)
                .is_some_and(|[arg]| peel(arg).kind() == "null")
    }

    pub(super) fn private_literal(&self, value: Node<'a>, depth: usize) -> bool {
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
                            self.pair_name(pair)
                                .is_some_and(|name| !super::proofs::SHARED_LINKS.contains(&name))
                        }) && data(value)
                    })
            }),
            _ => false,
        }
    }
}
