//! Bounded syntax queries used by the exclusion proofs.
use super::*;

pub(super) const SHARED_LINKS: [&str; 3] = ["__proto__", "constructor", "prototype"];

impl<'a> Classifier<'a, '_> {
    pub(super) fn proof_charge(&self, work: usize) -> bool {
        // Leave room for the candidate walk and operand collection. Refusing
        // an optional proof must not exhaust the candidate's work budget.
        let proof = self.proof_work.get().saturating_add(work);
        let file = self.file_work.get().saturating_add(work);
        if proof > TARGET_WORK_LIMIT / 2 || file > FILE_WORK_LIMIT / 4 {
            return false;
        }
        self.proof_work.set(proof);
        self.file_work.set(file);
        true
    }

    pub(super) fn text(&self, node: Node<'a>) -> &'a str {
        self.file.node_text(&node)
    }

    pub(super) fn node_at(&self, span: &AstSpan) -> Option<Node<'a>> {
        let node = self
            .file
            .tree
            .root_node()
            .descendant_for_byte_range(span.start_byte, span.end_byte)?;
        (node.start_byte() == span.start_byte && node.end_byte() == span.end_byte).then_some(node)
    }

    // Recognize the spelling and lexical binding of a modeled operation.
    pub(super) fn unshadowed_name(&self, node: Node<'a>, name: &str) -> bool {
        let node = peel(node);
        identifier(node) && self.text(node) == name && !self.analysis.has_binding(node)
    }

    pub(super) fn method(&self, call: Node<'a>, name: &str) -> Option<Node<'a>> {
        let callee = field(call, "function").map(peel)?;
        (call.kind() == "call_expression"
            && field(call, "optional_chain").is_none()
            && field(callee, "optional_chain").is_none()
            && self.text(field(callee, "property")?) == name)
            .then(|| field(callee, "object").map(peel))
            .flatten()
    }

    pub(super) fn static_call(&self, call: Node<'a>, namespace: &str, method: &str) -> bool {
        self.method(call, method)
            .is_some_and(|base| self.unshadowed_name(base, namespace))
    }

    pub(super) fn args<const N: usize>(&self, call: Node<'a>) -> Option<[Node<'a>; N]> {
        let args: Vec<_> = children(field(call, "arguments")?).take(N + 1).collect();
        if args.iter().any(|arg| arg.kind() == "spread_element") {
            return None;
        }
        args.try_into().ok()
    }

    pub(super) fn initializer(&self, read: Node<'a>) -> Option<Node<'a>> {
        let read = self.analysis.read(peel(read))?;
        let [site] = read.reaching_versions.as_slice() else {
            return None;
        };
        if !read.resolution_complete
            || self.script_global_binding(read.binding.as_ref()?)
            || self.analysis.defaults.contains_key(&site.start_byte)
        {
            return None;
        }
        match self.origin(site) {
            Origin::Value(value) => Some(peel(value)),
            _ => None,
        }
    }

    pub(super) fn script_global_binding(&self, binding: &AstSpan) -> bool {
        let lexical = &self.analysis.lexical;
        let id = binding.start_byte;
        !lexical.module_syntax
            && lexical.binding_scopes.get(&id) == Some(&self.file.tree.root_node().id())
            && (lexical.hoisted_vars.contains(&id)
                || self
                    .node_at(binding)
                    .and_then(|name| name.parent())
                    .is_none_or(|decl| {
                        matches!(
                            decl.kind(),
                            "function_declaration" | "generator_function_declaration"
                        )
                    }))
    }

    pub(super) fn same_value(&self, first: Node<'a>, second: Node<'a>) -> bool {
        let (Some(a), Some(b)) = (self.analysis.read(first), self.analysis.read(second)) else {
            return false;
        };
        self.proof_charge(a.reaching_versions.len() + b.reaching_versions.len() + 1)
            && a.binding.is_some()
            && a.binding == b.binding
            && a.owner == b.owner
            && a.path == b.path
            && !a.reaching_versions.is_empty()
            && a.reaching_versions == b.reaching_versions
            && (a.resolution_complete && b.resolution_complete
                || identifier(first)
                    && identifier(second)
                    && self.analysis.correlated_identifiers(a, b))
    }

    pub(super) fn enclosing(
        &self,
        mut node: Node<'a>,
        kind: impl Fn(&str) -> bool,
    ) -> Option<Node<'a>> {
        for _ in 0..RECEIVER_DEPTH_LIMIT {
            if !self.proof_charge(1) {
                return None;
            }
            if kind(node.kind()) {
                return Some(node);
            }
            node = node.parent()?;
        }
        None
    }

    pub(super) fn expression_use(&self, mut node: Node<'a>) -> Option<Node<'a>> {
        for _ in 0..RECEIVER_DEPTH_LIMIT {
            if !self.proof_charge(1) {
                return None;
            }
            let parent = node.parent()?;
            if peel(parent) != peel(node) {
                return Some(parent);
            }
            node = parent;
        }
        None
    }

    pub(super) fn binding_of(&self, node: Node<'a>) -> Option<AstSpan> {
        let mut scope = node;
        for _ in 0..RECEIVER_DEPTH_LIMIT {
            if !self.proof_charge(1) {
                return None;
            }
            if super::super::is_scope(scope) {
                return self.analysis.lexical.binding(scope, self.text(node));
            }
            scope = scope.parent()?;
        }
        None
    }

    pub(super) fn binding_references(&self, binding: &AstSpan) -> Option<Vec<Node<'a>>> {
        let decl = self.node_at(binding)?;
        let owner = self.node_at(
            self.analysis
                .lexical
                .binding_owners
                .get(&binding.start_byte)?,
        )?;
        let name = self.text(decl);
        let mut pending = vec![owner];
        let mut refs = Vec::new();
        while let Some(node) = pending.pop() {
            if !self.proof_charge(1) {
                return None;
            }
            if identifier(node) && self.text(node) == name && node != decl {
                if &self.binding_of(node)? == binding {
                    refs.push(node);
                }
            }
            pending.extend(children(node).take(TARGET_WORK_LIMIT / 2 + 1));
            if pending.len() > TARGET_WORK_LIMIT / 2 {
                return None;
            }
        }
        Some(refs)
    }

    pub(super) fn plain_string(&self, node: Node<'a>) -> Option<&'a str> {
        let node = peel(node);
        if node.kind() != "string" {
            return None;
        }
        let raw = self.text(node);
        if !self.proof_charge(raw.len()) {
            return None;
        }
        raw.get(1..raw.len().checked_sub(1)?)
            .filter(|text| !text.contains('\\'))
    }

    pub(super) fn pair_name(&self, node: Node<'a>) -> Option<&'a str> {
        let key = field(node, "key")?;
        match key.kind() {
            "property_identifier" | "number" => Some(self.text(key)),
            "string" => self.plain_string(key),
            _ => None,
        }
    }
}
