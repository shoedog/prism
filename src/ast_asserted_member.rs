//! Syntax identity only: assertions provide no binding or receiver authority.
use super::{AccessPath, Language, Node, ParsedFile};

pub(super) struct MemberPath<'a> {
    pub(super) path: AccessPath,
    pub(super) receiver: Node<'a>,
}

impl ParsedFile {
    /// A single dot member with an identifier receiver through <=8 supported
    /// wrappers. The returned path is logical, not a contiguous source slice.
    pub(super) fn bounded_member_path<'a>(&self, node: Node<'a>) -> Option<MemberPath<'a>> {
        self.member_path_with_budget(node, 8)
    }

    fn member_path_with_budget<'a>(&self, node: Node<'a>, budget: usize) -> Option<MemberPath<'a>> {
        if !matches!(self.language, Language::TypeScript | Language::Tsx)
            || node.kind() != "member_expression"
            || node.has_error()
            || self.language.is_in_erased_type_context(node)
            || node.child_by_field_name("optional_chain").is_some()
        {
            return None;
        }
        let mut cursor = node.walk();
        if node
            .children(&mut cursor)
            .any(|child| child.kind() == "optional_chain")
        {
            return None;
        }
        let field = node.child_by_field_name("property")?;
        if field.kind() != "property_identifier" || !literal_identifier(self.node_text(&field)) {
            return None;
        }
        let mut receiver = node.child_by_field_name("object")?;
        let mut wrappers = 0;
        while matches!(
            receiver.kind(),
            "parenthesized_expression" | "as_expression" | "satisfies_expression"
        ) {
            wrappers += 1;
            if wrappers > budget {
                return None;
            }
            let mut cursor = receiver.walk();
            let children: Vec<_> = receiver.children(&mut cursor).collect();
            let operator = if receiver.kind() == "parenthesized_expression" {
                receiver.end_byte()
            } else {
                children
                    .iter()
                    .find(|n| matches!(n.kind(), "as" | "satisfies"))?
                    .start_byte()
            };
            let mut values = children
                .into_iter()
                .filter(|n| n.is_named() && !n.is_extra() && n.end_byte() <= operator);
            let value = values.next()?;
            if values.next().is_some() {
                return None;
            }
            receiver = value;
        }
        if receiver.kind() != "identifier" || !literal_identifier(self.node_text(&receiver)) {
            return None;
        }
        Some(MemberPath {
            path: AccessPath::with_fields(
                self.node_text(&receiver),
                vec![self.node_text(&field).to_owned()],
            ),
            receiver,
        })
    }

    /// CPG consumers have original argument/return spans. Require the exact
    /// member node, not a descendant from an arbitrary enclosing expression.
    pub(crate) fn bounded_member_path_at(&self, start: usize, end: usize) -> Option<AccessPath> {
        if start >= end || end > self.source.len() {
            return None;
        }
        let node = self
            .tree
            .root_node()
            .descendant_for_byte_range(start, end)?;
        if node.start_byte() != start || node.end_byte() != end {
            return None;
        }
        self.bounded_member_path(node).map(|member| member.path)
    }

    /// Argument grouping is transparent, but arbitrary descendants are not.
    /// Share the eight-wrapper budget with the receiver and retain the original
    /// argument envelope for the caller's occurrence-containment check.
    pub(crate) fn bounded_argument_member_path_at(
        &self,
        start: usize,
        end: usize,
    ) -> Option<AccessPath> {
        if !matches!(self.language, Language::TypeScript | Language::Tsx)
            || start >= end
            || end > self.source.len()
        {
            return None;
        }
        let mut node = self
            .tree
            .root_node()
            .descendant_for_byte_range(start, end)?;
        if node.start_byte() != start || node.end_byte() != end || node.has_error() {
            return None;
        }
        let mut budget: usize = 8;
        while node.kind() == "parenthesized_expression" {
            budget = budget.checked_sub(1)?;
            let mut cursor = node.walk();
            let mut values = node.named_children(&mut cursor).filter(|n| !n.is_extra());
            let value = values.next()?;
            if values.next().is_some() {
                return None;
            }
            node = value;
        }
        self.member_path_with_budget(node, budget)
            .map(|member| member.path)
    }
}

fn literal_identifier(text: &str) -> bool {
    let mut bytes = text.bytes();
    bytes
        .next()
        .is_some_and(|b| b.is_ascii_alphabetic() || matches!(b, b'_' | b'$'))
        && bytes.all(|b| b.is_ascii_alphanumeric() || matches!(b, b'_' | b'$'))
}
