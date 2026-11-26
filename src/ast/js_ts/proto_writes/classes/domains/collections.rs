//! Finite data tables and fields of closed local record collections.
use super::*;

impl<'a> Classifier<'a, '_> {
    pub(super) fn member_domain(&self, member: Node<'a>, depth: usize) -> Option<Domain> {
        let receiver = field(member, "object").map(peel)?;
        if let Some(property) = field(member, "property") {
            if self.numeric_record_field(receiver, self.text(property), depth) {
                return Some(Domain::Numeric);
            }
        }
        self.finite_member(member, receiver, depth)
    }

    fn finite_member(&self, member: Node<'a>, receiver: Node<'a>, depth: usize) -> Option<Domain> {
        let table = if identifier(receiver) {
            let binding = self.analysis.read(receiver)?.binding.as_ref()?;
            let statement = self.node_at(binding)?.parent()?.parent()?;
            if statement.kind() != "lexical_declaration"
                || !super::super::super::has_token(statement, "const")
                || statement
                    .parent()
                    .is_some_and(|parent| parent.kind() == "export_statement")
                || !self.analysis.unmutated_receiver(receiver)
                || self.binding_references(binding)? != [receiver]
            {
                return None;
            }
            self.initializer(receiver)?
        } else {
            receiver
        };
        if table.kind() != "object" {
            return None;
        }
        if let Some(property) = field(member, "property") {
            let mut value = None;
            for pair in children(table) {
                if !self.proof_charge(1) || pair.kind() != "pair" {
                    return None;
                }
                let name = self.pair_name(pair)?;
                if name == "__proto__" {
                    return None;
                }
                if name == self.text(property) {
                    value = field(pair, "value")
                }
            }
            return self.key_domain(value?, depth);
        }
        let index = field(member, "index")?;
        if self.key_domain(index, depth) != Some(Domain::Numeric) {
            return None;
        }
        for pair in children(table) {
            if !self.proof_charge(1)
                || pair.kind() != "pair"
                || !self
                    .pair_name(pair)?
                    .bytes()
                    .all(|byte| byte.is_ascii_digit())
                || self.key_domain(field(pair, "value")?, depth) != Some(Domain::String)
                || self.plain_string(field(pair, "value")?).is_none()
            {
                return None;
            }
        }
        Some(Domain::Benign)
    }

    fn numeric_record_field(&self, record: Node<'a>, name: &str, depth: usize) -> bool {
        if !identifier(record) || SHARED_LINKS.contains(&name) {
            return false;
        }
        let Some(read) = self
            .analysis
            .read(record)
            .filter(|read| read.resolution_complete)
        else {
            return false;
        };
        let [site] = read.reaching_versions.as_slice() else {
            return false;
        };
        let Origin::Element(collection) = self.origin(site) else {
            return false;
        };
        let Some(binding) = &read.binding else {
            return false;
        };
        if self.script_global_binding(binding) {
            return false;
        }
        let Some(decl) = self.node_at(site) else {
            return false;
        };
        let Some(loop_node) = decl
            .parent()
            .filter(|node| node.kind() == "for_in_statement")
        else {
            return false;
        };
        if field(loop_node, "left") != Some(decl)
            || field(loop_node, "right") != Some(collection)
            || !field(loop_node, "operator").is_some_and(|op| self.text(op) == "of")
        {
            return false;
        }
        let collection = peel(collection);
        let array = self.initializer(collection).unwrap_or(collection);
        if array.kind() != "array" || !self.proof_charge(array.child_count()) || !dense_array(array)
        {
            return false;
        }
        if identifier(collection) {
            let Some(binding) = self
                .analysis
                .read(collection)
                .and_then(|read| read.binding.as_ref())
            else {
                return false;
            };
            if self
                .node_at(binding)
                .and_then(|name| name.parent()?.parent()?.parent())
                .is_none_or(|parent| parent.kind() == "export_statement")
            {
                return false;
            }
            let Some(refs) = self.binding_references(binding) else {
                return false;
            };
            if !refs
                .into_iter()
                .all(|reference| self.closed_collection_use(reference, name, depth))
            {
                return false;
            }
        }
        children(array).all(|element| self.initialized_record(element, name, depth))
            && self.record_references(binding, name, depth, None)
    }

    fn closed_collection_use(&self, reference: Node<'a>, name: &str, depth: usize) -> bool {
        let Some(use_node) = self.expression_use(reference) else {
            return false;
        };
        if use_node.kind() == "for_in_statement" {
            let Some(item) = field(use_node, "left") else {
                return false;
            };
            return field(use_node, "right").map(peel) == Some(reference)
                && identifier(item)
                && field(use_node, "kind").is_some()
                && field(use_node, "operator").is_some_and(|op| self.text(op) == "of")
                && self
                    .binding_of(item)
                    .is_some_and(|binding| self.record_references(&binding, name, depth, None));
        }
        if use_node.kind() != "member_expression"
            || field(use_node, "object").map(peel) != Some(reference)
        {
            return false;
        }
        let Some(property) = field(use_node, "property") else {
            return false;
        };
        match self.text(property) {
            "length" => !self.is_modified(use_node),
            "push" => {
                let Some(call) = self.expression_use(use_node) else {
                    return false;
                };
                if self.method(call, "push") != Some(reference) {
                    return false;
                }
                field(call, "arguments").is_some_and(|args| {
                    children(args).all(|arg| {
                        let arg = peel(arg);
                        if !identifier(arg) {
                            return self.initialized_record(arg, name, depth);
                        }
                        self.initializer(arg)
                            .is_some_and(|value| self.initialized_record(value, name, depth))
                            && self
                                .analysis
                                .read(arg)
                                .and_then(|read| read.binding.as_ref())
                                .is_some_and(|binding| {
                                    self.record_references(binding, name, depth, Some(arg))
                                })
                    })
                })
            }
            _ => false,
        }
    }

    fn initialized_record(&self, element: Node<'a>, name: &str, depth: usize) -> bool {
        let element = peel(element);
        if !self.proof_charge(1) || element.kind() != "object" {
            return false;
        }
        let mut initialized = false;
        for pair in children(element) {
            if !self.proof_charge(1) || pair.kind() != "pair" {
                return false;
            }
            let Some(key) = self.pair_name(pair) else {
                return false;
            };
            if key == "__proto__" {
                return false;
            }
            if key == name {
                initialized = field(pair, "value")
                    .is_some_and(|rhs| self.key_domain(rhs, depth) == Some(Domain::Numeric));
            }
        }
        initialized
    }

    fn is_modified(&self, node: Node<'a>) -> bool {
        let Some(parent) = self.expression_use(node) else {
            return true;
        };
        let in_field = |name| field(parent, name).map(peel) == Some(peel(node));
        match parent.kind() {
            "assignment_expression" | "augmented_assignment_expression" | "for_in_statement" => {
                in_field("left")
            }
            "update_expression" => true,
            "unary_expression" => operator(parent, self.file) == "delete",
            "array_pattern" | "object_pattern" | "assignment_pattern" | "pair_pattern"
            | "rest_pattern" => true,
            _ => false,
        }
    }

    fn record_references(
        &self,
        binding: &AstSpan,
        name: &str,
        depth: usize,
        insertion: Option<Node<'a>>,
    ) -> bool {
        // A script-level var is a global-object property: writes through `window.x` are
        // invisible to the references the lexical scope can see.
        if self.script_global_binding(binding) {
            return false;
        }
        let Some(refs) = self.binding_references(binding) else {
            return false;
        };
        refs.into_iter().all(|reference| {
            if Some(reference) == insertion {
                return true;
            }
            let Some(access) = self.expression_use(reference) else {
                return false;
            };
            if access.kind() != "member_expression"
                || field(access, "object").map(peel) != Some(reference)
            {
                return false;
            }
            let Some(property) = field(access, "property") else {
                return false;
            };
            let property = self.text(property);
            if SHARED_LINKS.contains(&property) {
                return false;
            }
            let Some(use_node) = self.expression_use(access) else {
                return false;
            };
            if use_node.kind() == "call_expression"
                && field(use_node, "function").map(peel) == Some(access)
            {
                return false;
            }
            if !self.is_modified(access) || property != name {
                return true;
            }
            use_node.kind() == "update_expression"
                || (use_node.kind() == "assignment_expression"
                    || operator(use_node, self.file) == "+=")
                    && field(use_node, "right")
                        .is_some_and(|rhs| self.key_domain(rhs, depth) == Some(Domain::Numeric))
        })
    }
}

fn dense_array(node: Node<'_>) -> bool {
    let mut after_value = false;
    let mut cursor = node.walk();
    for child in node.children(&mut cursor).filter(|child| !child.is_extra()) {
        match child.kind() {
            "spread_element" => return false,
            "," if !after_value => return false,
            "," => after_value = false,
            _ if child.is_named() => after_value = true,
            _ => {}
        }
    }
    true
}
