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
        let primitive = self.fixed_key(value).is_some()
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
        let node = peel(node);
        if self.fixed_key(node).is_some() {
            return;
        }
        if let Some(read) = self.analysis.read(node) {
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
                    if seen.len() == SELECTOR_SOURCE_LIMIT {
                        break;
                    }
                    if !seen.insert(site.start_byte) {
                        continue;
                    }
                    if let Some(default) = self.analysis.defaults.get(&site.start_byte) {
                        self.key_source(*default, role, out, seen);
                    }
                    match self.origin(site) {
                        Origin::Value(source)
                        | Origin::Element(source)
                        | Origin::Child(source, _) => {
                            self.key_source(source, role, out, seen);
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
                let method = callee.and_then(|callee| field(peel(callee), "property"));
                if let Some(callee) = callee.map(peel).filter(|callee| !identifier(*callee)) {
                    inputs.extend(field(callee, "object").or(Some(callee)));
                }
                if let Some(args) = field(node, "arguments") {
                    let offset = method.is_some_and(|method| self.file.node_text(&method) == "at");
                    inputs.extend(children(args).skip(usize::from(offset)));
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
                if matches!(operator(node, self.file), "||=" | "??=" | "&&=") {
                    inputs.extend(field(node, "left"));
                }
                inputs.extend(field(node, "right"));
            }
            "unary_expression"
                if matches!(operator(node, self.file), "void" | "typeof" | "delete") => {}
            "pair" => inputs.extend(field(node, "value")),
            _ if !is_owner(node) => inputs.extend(value_children(self.file, node)),
            _ => {}
        }
        for input in inputs {
            self.key_reads(input, role, out, seen);
        }
    }

    fn key_source(
        &self,
        source: Node<'a>,
        role: KeyRole,
        out: &mut BTreeMap<AstSpan, KeyOperand>,
        seen: &mut BTreeSet<usize>,
    ) {
        let source = peel(source);
        match source.kind() {
            "identifier"
            | "member_expression"
            | "subscript_expression"
            | "ternary_expression"
            | "array"
            | "assignment_expression"
            | "augmented_assignment_expression" => {
                self.key_reads(source, role, out, seen);
            }
            "call_expression" => {
                if let Some(callee) = field(source, "function") {
                    if let Some(method) = field(callee, "property").filter(|method| {
                        matches!(
                            self.file.node_text(method),
                            "shift" | "pop" | "at" | "slice" | "split"
                        )
                    }) {
                        if matches!(self.file.node_text(&method), "shift" | "pop") {
                            self.key_reads(callee, role, out, seen);
                        }
                        if let Some(object) = field(callee, "object") {
                            self.key_reads(object, role, out, seen);
                        }
                    }
                }
            }
            _ => {}
        }
    }
}
