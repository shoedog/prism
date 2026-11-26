//! Closed syntactic tables for adapter-proved evidence.

use super::evidence::Role;
use prism::{ast::ParsedFile, languages::Language};
use std::{collections::BTreeMap, iter::successors};
use tree_sitter::Node;

mod bash;
mod c_family;
mod callee;
mod go;
mod hcl;
mod java;
mod js;
mod lua;
mod operand;
mod operation;
mod python;
mod rust;

#[cfg(test)]
mod tests;

pub(super) use callee::certify_callee;
#[cfg(test)]
pub(super) use callee::target_for_hit;
#[cfg(test)]
pub(super) use operand::candidates;
pub(super) use operand::certify_operand;
#[cfg(test)]
pub(super) use operation::operation_head;
pub(super) use operation::{certify_operation, OperationShape};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Table {
    Operation,
    Callee,
    Operand,
}

macro_rules! reasons {
    ($($table:ident { $($variant:ident => $code:literal),+ $(,)? })+) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq)]
        pub(super) enum Reason { $($($variant,)+)+ }

        impl Reason {
            pub(super) fn as_str(self) -> &'static str {
                match self { $($(Self::$variant => $code,)+)+ }
            }

            pub(super) fn table(self) -> Table {
                match self { $($(Self::$variant => Table::$table,)+)+ }
            }

            #[cfg(test)]
            const ALL: &'static [Self] = &[$($(Self::$variant,)+)+];
        }
    };
}

reasons! {
    Operation {
        OpKindUnlisted => "op_kind_unlisted",
        OpContextNonRuntime => "op_context_non_runtime",
        OpSpecialForm => "op_special_form",
        OpArgumentsUnlisted => "op_arguments_unlisted",
        OpTypeArguments => "op_type_arguments",
        AttributeSpread => "attribute_spread",
        ParseRecovery => "parse_recovery",
    }
    Callee {
        CalleeKindUnlisted => "callee_kind_unlisted",
        CalleeTargetUnlisted => "callee_target_unlisted",
        CalleeTypeUnlisted => "callee_type_unlisted",
        SubscriptKeyNotPlainLiteral => "subscript_key_not_plain_literal",
        SubscriptCFamily => "subscript_c_family",
        MemberPointerOperator => "member_pointer_operator",
    }
    Operand {
        OperandKindUnlisted => "operand_kind_unlisted",
        OperandNotDirectChild => "operand_not_direct_child",
        OperandSpread => "operand_spread",
        OperandLabel => "operand_label",
        OperandComment => "operand_comment",
        OperandAttribute => "operand_attribute",
        OperandMacroBlock => "operand_macro_block",
        RoleNotBound => "role_not_bound",
        SlotMismatch => "slot_mismatch",
        ParallelAssignment => "parallel_assignment",
        AttributeOverwritten => "attribute_overwritten",
    }
}

#[derive(Default)]
pub(super) struct Refusals(BTreeMap<&'static str, usize>);

impl Refusals {
    pub(super) fn record(&mut self, reason: Reason) {
        *self.0.entry(reason.as_str()).or_default() += 1;
    }

    pub(super) fn iter(&self) -> impl Iterator<Item = (&'static str, usize)> + '_ {
        self.0.iter().map(|(&code, &count)| (code, count))
    }
}

pub(super) fn certify(
    parsed: &ParsedFile,
    operation: Node<'_>,
    target: Node<'_>,
    operand: Node<'_>,
    role: Role<'_>,
) -> Result<(), (Table, Reason)> {
    let result = (|| {
        let shape = certify_operation(parsed, operation)?;
        certify_callee(parsed, operation, &shape, target)?;
        certify_operand(parsed, operation, &shape, target, operand, role)
    })();
    result.map_err(|reason| (reason.table(), reason))
}

fn comment(node: Node<'_>) -> bool {
    matches!(
        node.kind(),
        "comment" | "html_comment" | "line_comment" | "block_comment"
    )
}

fn named(node: Node<'_>) -> Vec<Node<'_>> {
    let mut cursor = node.walk();
    node.named_children(&mut cursor)
        .filter(|&child| !comment(child))
        .collect()
}

fn fields<'tree>(node: Node<'tree>, field: &str) -> Vec<Node<'tree>> {
    let mut cursor = node.walk();
    node.children_by_field_name(field, &mut cursor).collect()
}

fn tokens(node: Node<'_>) -> Vec<Node<'_>> {
    let mut cursor = node.walk();
    node.children(&mut cursor)
        .filter(|&child| !comment(child))
        .collect()
}

fn has_token(node: Node<'_>, kind: &str) -> bool {
    let mut cursor = node.walk();
    let mut children = node.children(&mut cursor);
    children.any(|child| !child.is_named() && child.kind() == kind)
}

fn field<'tree>(node: Node<'tree>, name: &str, kind: &str) -> Option<Node<'tree>> {
    node.child_by_field_name(name)
        .filter(|child| child.kind() == kind)
}

fn same_node(left: Node<'_>, right: Node<'_>) -> bool {
    left.into_raw().tree == right.into_raw().tree && left == right && left.range() == right.range()
}

fn ancestors(node: Node<'_>) -> impl Iterator<Item = Node<'_>> {
    successors(Some(node), Node::parent)
}

fn within(node: Node<'_>, root: Node<'_>) -> bool {
    ancestors(node).any(|parent| same_node(parent, root))
}

fn unwrapped<'tree>(node: Node<'tree>, open: &str, close: &str) -> Option<Node<'tree>> {
    match tokens(node).as_slice() {
        [first, value, last]
            if first.kind() == open && value.is_named() && last.kind() == close =>
        {
            Some(*value)
        }
        _ => None,
    }
}

#[derive(Clone, Copy)]
struct Path<'tree> {
    terminal: Node<'tree>,
    whole: Node<'tree>,
    receiver: Option<Node<'tree>>,
}

impl<'tree> Path<'tree> {
    fn plain(node: Node<'tree>) -> Self {
        Self {
            terminal: node,
            whole: node,
            receiver: None,
        }
    }

    fn member(whole: Node<'tree>, terminal: Node<'tree>, receiver: Option<Node<'tree>>) -> Self {
        Self {
            terminal,
            whole,
            receiver,
        }
    }

    fn matches(self, target: Node<'_>) -> bool {
        same_node(target, self.terminal) || same_node(target, self.whole)
    }

    fn literal_key(key: Node<'tree>, receiver: Option<Node<'tree>>) -> Self {
        Self::member(key, key, receiver)
    }
}

/// The value and assignment rows require member chains rooted in an identifier.
fn rooted_member<'tree>(
    mut node: Node<'tree>,
    member: &str,
    descend: impl Fn(Node<'tree>) -> Option<Node<'tree>>,
) -> bool {
    while node.kind() == member {
        let Some(receiver) = descend(node) else {
            return false;
        };
        node = receiver;
    }
    node.kind() == "identifier"
}
