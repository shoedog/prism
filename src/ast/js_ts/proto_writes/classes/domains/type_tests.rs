//! Dominating tests of the numeric typeof result.
use super::*;

impl<'a> Classifier<'a, '_> {
    pub(super) fn numeric_type_test_dominates(&self, key: Node<'a>) -> bool {
        if !identifier(peel(key)) {
            return false;
        }
        self.dominating_test(peel(key), |test, succeeds| {
            let op = operator(test, self.file);
            if !matches!((op, succeeds), ("===", true) | ("!==", false)) {
                return false;
            }
            let Some(value) = field(test, "left").map(peel) else {
                return false;
            };
            value.kind() == "unary_expression"
                && operator(value, self.file) == "typeof"
                && field(test, "right").and_then(|rhs| self.plain_string(rhs)) == Some("number")
                && field(value, "argument")
                    .is_some_and(|arg| self.same_tested_value(peel(arg), key))
        })
    }

    fn same_tested_value(&self, guard: Node<'a>, key: Node<'a>) -> bool {
        [guard, key].into_iter().all(|node| {
            self.analysis
                .read(node)
                .is_some_and(|read| read.resolution_complete)
        }) && self.same_value(guard, key)
    }

    fn dominating_test(
        &self,
        mut node: Node<'a>,
        establishes: impl Fn(Node<'a>, bool) -> bool,
    ) -> bool {
        for _ in 0..RECEIVER_DEPTH_LIMIT {
            if !self.proof_charge(1) {
                return false;
            }
            let Some(parent) = node.parent() else {
                return false;
            };
            if is_owner(parent) {
                return false;
            }
            if parent.kind() == "if_statement"
                && field(parent, "consequence") == Some(node)
                && field(parent, "condition")
                    .is_some_and(|test| self.test_branch(test, true, &establishes, 0))
            {
                return true;
            }
            if parent.kind() == "statement_block" {
                let mut previous = node.prev_named_sibling();
                while let Some(statement) = previous {
                    if !self.proof_charge(1) {
                        return false;
                    }
                    if statement.kind() == "if_statement"
                        && field(statement, "consequence").is_some_and(immediate_exit)
                        && field(statement, "condition")
                            .is_some_and(|test| self.test_branch(test, false, &establishes, 0))
                    {
                        return true;
                    }
                    previous = statement.prev_named_sibling();
                }
            }
            node = parent;
        }
        false
    }

    fn test_branch(
        &self,
        node: Node<'a>,
        succeeds: bool,
        establishes: &impl Fn(Node<'a>, bool) -> bool,
        depth: usize,
    ) -> bool {
        if depth == 32 || !self.proof_charge(1) {
            return false;
        }
        let node = peel(node);
        if node.kind() == "unary_expression" && operator(node, self.file) == "!" {
            return field(node, "argument")
                .is_some_and(|test| self.test_branch(test, !succeeds, establishes, depth + 1));
        }
        if node.kind() == "binary_expression"
            && matches!(
                (operator(node, self.file), succeeds),
                ("&&", true) | ("||", false)
            )
        {
            return ["left", "right"]
                .into_iter()
                .filter_map(|name| field(node, name))
                .any(|child| self.test_branch(child, succeeds, establishes, depth + 1));
        }
        establishes(node, succeeds)
    }
}

fn immediate_exit(node: Node<'_>) -> bool {
    let node = if node.kind() == "statement_block" {
        children(node).next().unwrap_or(node)
    } else {
        node
    };
    matches!(node.kind(), "return_statement" | "throw_statement")
        || matches!(node.kind(), "continue_statement" | "break_statement")
            && children(node).next().is_none()
}
