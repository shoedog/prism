//! Binding evidence available to guard proofs.
use super::{identifier, peel, Analysis, AstSpan, Node, Origin, Proof};
use crate::ast::js_ts::proto_writes::NameRead;
use std::collections::BTreeSet;

pub(super) struct Bindings<'a, 'b> {
    analysis: &'b Analysis<'a>,
}

impl<'a, 'b> Bindings<'a, 'b> {
    pub(super) fn new(analysis: &'b Analysis<'a>) -> Self {
        Self { analysis }
    }

    /// A member or index read carries its base's binding and versions.
    /// Admit only the identifier itself, through transparent wrappers.
    pub(super) fn bare_read(&self, node: Node<'a>) -> Option<&'b NameRead> {
        let node = peel(node);
        if identifier(node) {
            self.analysis.read(node)
        } else {
            None
        }
    }

    /// Shadowing also includes ambiguous and implicit lexical bindings.
    pub(super) fn is_shadowed(&self, node: Node<'a>) -> bool {
        let node = peel(node);
        !identifier(node) || self.analysis.has_binding(node)
    }

    /// The file-wide census needs lexical entries for identifier writes as well as reads.
    pub(super) fn lexical_entry(
        &self,
        scope: Node<'a>,
        node: Node<'a>,
        spelling: &str,
    ) -> Option<&Option<AstSpan>> {
        if !identifier(peel(node)) {
            return Some(&None);
        }
        self.analysis.lexical.lookup(scope, spelling)
    }

    pub(super) fn canonical<'s>(&self, spelling: &'s str) -> &'s str
    where
        'b: 's,
    {
        self.analysis.lexical.canonical(spelling)
    }

    pub(super) fn in_module(&self) -> bool {
        self.analysis.lexical.module_syntax
    }

    pub(super) fn binding_owner(&self, binding: &AstSpan) -> Option<&AstSpan> {
        self.analysis
            .lexical
            .binding_owners
            .get(&binding.start_byte)
    }

    pub(super) fn proof_writes(&self, binding: &AstSpan) -> Option<&BTreeSet<AstSpan>> {
        self.analysis.proof_writes(binding)
    }

    pub(super) fn correlated_before_stores(
        &self,
        a: &NameRead,
        b: &NameRead,
        pending_stores: &[AstSpan],
    ) -> bool {
        self.analysis.correlated_before_stores(a, b, pending_stores)
    }

    pub(super) fn value_origin(&self, read: &NameRead, site: &AstSpan) -> Option<Node<'a>> {
        if self.analysis.deferred_definition(read, site)
            || self.analysis.defaults.contains_key(&site.start_byte)
        {
            return None;
        }
        match self.analysis.origins.get(&site.start_byte)? {
            Origin::Value(value) => Some(peel(*value)),
            _ => None,
        }
    }

    pub(super) fn element_origin(&self, site: &AstSpan) -> Option<Node<'a>> {
        match self.analysis.origins.get(&site.start_byte)? {
            Origin::Element(sequence) => Some(peel(*sequence)),
            _ => None,
        }
    }
}

impl<'a, F: FnMut(usize) -> bool> Proof<'a, '_, F> {
    pub(super) fn bare_declaration(&self, node: Node<'a>) -> Option<(AstSpan, Node<'a>)> {
        let binding = self.bindings.bare_read(node)?.binding.as_ref()?;
        let declaration = self
            .file
            .tree
            .root_node()
            .descendant_for_byte_range(binding.start_byte, binding.end_byte)?;
        (AstSpan::from(declaration) == *binding).then_some((binding.clone(), declaration))
    }
}
