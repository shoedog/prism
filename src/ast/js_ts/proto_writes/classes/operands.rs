//! Value-bearing operands and selector provenance.
use super::super::name_reads::value_children;
use super::super::KeyOperand;
use super::*;

const SELECTOR_SOURCE_LIMIT: usize = 32;

impl<'a> Classifier<'a, '_> {
    pub(super) fn promote_reused_primitives(
        &self,
        steps: &Selectors<'a>,
        operands: &mut BTreeMap<AstSpan, KeyOperand>,
    ) {
        // Candidate sources enrich earlier-site provenance. Only an actual
        // step can prove that the final property reused the selected value.
        let selectors: BTreeSet<_> = steps
            .values()
            .filter_map(|node| self.primitive_identity(&AstSpan::from(*node)))
            .collect();
        for (span, operand) in operands {
            if operand.role == KeyRole::Property
                && self
                    .primitive_identity(span)
                    .is_some_and(|key| selectors.contains(&key))
            {
                operand.role = KeyRole::Selector;
            }
        }
    }

    fn primitive_identity(&self, span: &AstSpan) -> Option<(AstSpan, AstSpan)> {
        let read = self.analysis.read_at(span)?;
        let [site] = read.reaching_versions.as_slice() else {
            return None;
        };
        if !read.resolution_complete || read.path != read.base {
            return None;
        }
        let Origin::Value(value) = self.origin(site) else {
            return None;
        };
        let value = peel(value);
        let primitive = value.kind() == "template_string"
            || self.fixed_key(value).is_some()
            || (value.kind() == "call_expression"
                && field(value, "function").is_some_and(|callee| {
                    matches!(
                        self.file.node_text(&callee),
                        "String" | "Number" | "Boolean" | "BigInt"
                    ) && !self.analysis.has_binding(callee)
                }));
        if !primitive || self.analysis.may_repeat(value) {
            return None;
        }
        Some((read.binding.clone()?, site.clone()))
    }

    pub(super) fn key_reads(
        &self,
        node: Node<'a>,
        role: KeyRole,
        out: &mut BTreeMap<AstSpan, KeyOperand>,
        seen: &mut BTreeSet<usize>,
    ) {
        self.key_value(node, role, out, seen, 0);
    }

    fn key_value(
        &self,
        node: Node<'a>,
        role: KeyRole,
        out: &mut BTreeMap<AstSpan, KeyOperand>,
        seen: &mut BTreeSet<usize>,
        depth: usize,
    ) {
        if depth == RECEIVER_DEPTH_LIMIT || !self.charge(1) {
            return;
        }
        let node = peel(node);
        if self.fixed_key(node).is_some() {
            return;
        }
        if let Some(read) = self.analysis.read(node) {
            if !self.charge(read.reaching_versions.len()) {
                return;
            }
            out.entry(read.span.clone()).or_insert_with(|| KeyOperand {
                name: read.path.clone(),
                base: read.base.clone(),
                role,
                owner: read.owner.clone(),
                binding: read.binding.clone(),
                reaching_versions: read.reaching_versions.clone(),
                line: node.start_position().row + 1,
                start_byte: read.span.start_byte,
                end_byte: read.span.end_byte,
            });
            if role == KeyRole::Selector && identifier(node) {
                for site in &read.reaching_versions {
                    if seen.len() == SELECTOR_SOURCE_LIMIT || !self.charge(1) {
                        break;
                    }
                    if !seen.insert(site.start_byte) {
                        continue;
                    }
                    if let Some(default) = self.analysis.defaults.get(&site.start_byte) {
                        self.key_source(*default, role, out, seen, depth + 1);
                    }
                    match self.origin(site) {
                        Origin::Value(source)
                        | Origin::Element(source)
                        | Origin::Child(source, _) => {
                            self.key_source(source, role, out, seen, depth + 1);
                        }
                        Origin::Unknown => {}
                    }
                }
            }
            return;
        }
        let mut inputs = Vec::new();
        match node.kind() {
            "call_expression" | "new_expression" => {
                let callee = field(node, "function").or_else(|| field(node, "constructor"));
                let method = callee
                    .and_then(|callee| field(peel(callee), "property"))
                    .map(|method| self.file.node_text(&method));
                let receiver = callee
                    .and_then(|callee| field(peel(callee), "object"))
                    .map(peel);
                if let Some(receiver) = receiver {
                    if receiver.kind() == "array" && matches!(method, Some("shift" | "pop" | "at"))
                    {
                        self.array_call_inputs(receiver, node, method.unwrap(), &mut inputs);
                    } else if !self.analysis.namespace_receiver(receiver) {
                        if let Some(array) = self.stable_array(receiver) {
                            if let Some(method @ ("pop" | "shift" | "at")) = method {
                                self.array_call_inputs(array, node, method, &mut inputs);
                            }
                        }
                        inputs.push(receiver);
                    }
                } else if let Some(callee) = callee.map(peel).filter(|callee| !identifier(*callee))
                {
                    inputs.push(callee);
                }
                if let Some(args) = field(node, "arguments") {
                    let offset = method == Some("at")
                        && receiver.is_some_and(|receiver| self.native_sequence(receiver));
                    inputs.extend(
                        children(args)
                            .take_while(|_| self.charge(1))
                            .skip(usize::from(offset)),
                    );
                }
            }
            "subscript_expression" | "member_expression" => {
                inputs.extend(field(node, "object"));
                let position = field(node, "object")
                    .map(peel)
                    .filter(|object| object.kind() == "call_expression")
                    .and_then(|call| field(call, "function"))
                    .and_then(|callee| field(callee, "property"))
                    .is_some_and(|method| {
                        matches!(self.file.node_text(&method), "split" | "slice")
                    });
                if !position {
                    inputs.extend(field(node, "index"));
                }
            }
            "ternary_expression" => {
                inputs.extend(
                    ["consequence", "alternative"]
                        .into_iter()
                        .filter_map(|arm| field(node, arm)),
                );
            }
            "sequence_expression" => inputs.extend(children(node).last()),
            "assignment_expression" | "augmented_assignment_expression" => {
                if matches!(operator(node, self.file), "||=" | "??=" | "&&=" | "+=") {
                    inputs.extend(field(node, "left"));
                }
                inputs.extend(field(node, "right"));
            }
            "unary_expression"
                if matches!(
                    operator(node, self.file),
                    "void" | "typeof" | "delete" | "!"
                ) => {}
            "binary_expression"
                if matches!(
                    operator(node, self.file),
                    "===" | "!==" | "==" | "!=" | "in" | "instanceof" | "<" | ">" | "<=" | ">="
                ) => {}
            "yield_expression" => {}
            "pair" => inputs.extend(field(node, "value")),
            _ if !is_owner(node) => {
                inputs.extend(value_children(self.file, node).take_while(|_| self.charge(1)))
            }
            _ => {}
        }
        for input in inputs {
            self.key_value(input, role, out, seen, depth + 1);
        }
    }

    fn key_source(
        &self,
        source: Node<'a>,
        role: KeyRole,
        out: &mut BTreeMap<AstSpan, KeyOperand>,
        seen: &mut BTreeSet<usize>,
        depth: usize,
    ) {
        if depth == RECEIVER_DEPTH_LIMIT || !self.charge(1) {
            return;
        }
        let source = peel(source);
        match source.kind() {
            "identifier"
            | "member_expression"
            | "subscript_expression"
            | "ternary_expression"
            | "assignment_expression"
            | "augmented_assignment_expression" => {
                self.key_value(source, role, out, seen, depth + 1);
            }
            "call_expression" => {
                if let Some(callee) = field(source, "function") {
                    if let Some(method) = field(callee, "property").filter(|method| {
                        matches!(
                            self.file.node_text(method),
                            "shift" | "pop" | "at" | "slice" | "split"
                        )
                    }) {
                        if let Some(object) = field(callee, "object").map(peel) {
                            let method = self.file.node_text(&method);
                            let array = (object.kind() == "array")
                                .then_some(object)
                                .or_else(|| self.stable_array(object));
                            if let Some(array) =
                                array.filter(|_| matches!(method, "shift" | "pop" | "at"))
                            {
                                let mut inputs = Vec::new();
                                self.array_call_inputs(array, source, method, &mut inputs);
                                for input in inputs {
                                    self.key_value(input, role, out, seen, depth + 1);
                                }
                            } else {
                                if matches!(method, "shift" | "pop") {
                                    self.key_value(callee, role, out, seen, depth + 1);
                                }
                                self.key_value(object, role, out, seen, depth + 1);
                            }
                        }
                    }
                }
            }
            _ => {}
        }
    }
    fn native_sequence(&self, receiver: Node<'a>) -> bool {
        matches!(receiver.kind(), "array" | "string" | "template_string")
            || self.stable_array(receiver).is_some()
    }

    fn stable_array(&self, receiver: Node<'a>) -> Option<Node<'a>> {
        if !identifier(receiver) || !self.analysis.unmutated_receiver(receiver) {
            return None;
        }
        let read = self.analysis.read(receiver)?;
        let [site] = read.reaching_versions.as_slice() else {
            return None;
        };
        if !read.resolution_complete || self.analysis.may_repeat(receiver) {
            return None;
        }
        match self.origin(site) {
            Origin::Value(value) if peel(value).kind() == "array" => Some(peel(value)),
            _ => None,
        }
    }

    fn array_call_inputs(
        &self,
        array: Node<'a>,
        call: Node<'a>,
        method: &str,
        out: &mut Vec<Node<'a>>,
    ) {
        let mut cursor = array.walk();
        let mut slot = None;
        let mut elements = Vec::new();
        let mut comma = false;
        let mut position = 0usize;
        for child in array
            .children(&mut cursor)
            .filter(|child| !child.is_extra())
        {
            if !self.charge(1) {
                return;
            }
            match child.kind() {
                "," if comma && method == "pop" => slot = None,
                "," => {
                    position += 1;
                    comma = true;
                    if method == "shift" {
                        return;
                    }
                }
                "spread_element" => {
                    slot = None;
                    comma = false;
                    if method != "pop" {
                        return;
                    }
                }
                _ if child.is_named() => {
                    if method == "shift" {
                        out.push(child);
                        return;
                    }
                    if method == "at" {
                        elements.push((position, child));
                    }
                    slot = Some(child);
                    comma = false;
                }
                _ => {}
            }
        }
        if method == "pop" {
            out.extend(slot);
        } else if method == "at" {
            let offset = field(call, "arguments")
                .and_then(|args| children(args).next())
                .and_then(|arg| self.file.node_text(&peel(arg)).parse::<isize>().ok());
            let length = position + usize::from(!comma && slot.is_some());
            if let Some(offset) = offset {
                let index = if offset.is_negative() {
                    length.checked_sub(offset.unsigned_abs())
                } else {
                    Some(offset as usize)
                };
                out.extend(
                    elements
                        .into_iter()
                        .filter_map(|(position, child)| (Some(position) == index).then_some(child)),
                );
            } else {
                out.extend(elements.into_iter().map(|(_, child)| child));
            }
        }
    }
}
