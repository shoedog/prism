//! Preserve the pre-0.25 Go statement view without inventing another scope.
use crate::languages::Language;
use tree_sitter::{Node, TreeCursor};

/// All direct children, with Go's `statement_list` replaced by its children.
/// Keep case headers, comments and anonymous tokens in their original order.
/// Other languages keep their native tree shape.
pub(crate) fn children(node: Node<'_>, language: Language) -> Children<'_> {
    let mut cursor = node.walk();
    let ready = cursor.goto_first_child();
    Children {
        cursor,
        ready,
        flatten: language == Language::Go,
        inside_list: false,
    }
}

pub(crate) struct Children<'tree> {
    cursor: TreeCursor<'tree>,
    ready: bool,
    flatten: bool,
    inside_list: bool,
}

impl Children<'_> {
    fn advance(&mut self) {
        if self.cursor.goto_next_sibling() {
            return;
        }
        if self.inside_list {
            self.inside_list = false;
            self.cursor.goto_parent();
            if self.cursor.goto_next_sibling() {
                return;
            }
        }
        self.ready = false;
    }
}

impl<'tree> Iterator for Children<'tree> {
    type Item = Node<'tree>;

    fn next(&mut self) -> Option<Self::Item> {
        while self.ready {
            let node = self.cursor.node();
            if self.flatten && node.kind() == "statement_list" {
                if self.cursor.goto_first_child() {
                    self.inside_list = true;
                } else {
                    self.advance();
                }
                continue;
            }
            self.advance();
            return Some(node);
        }
        None
    }
}

/// Go-only semantic parent: the wrapper is neither a region nor a statement.
pub(crate) fn parent(node: Node<'_>) -> Option<Node<'_>> {
    let parent = node.parent()?;
    if parent.kind() == "statement_list" {
        parent.parent()
    } else {
        Some(parent)
    }
}
