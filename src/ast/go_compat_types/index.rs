//! Two cursor passes, no root-descending Node::parent calls. Bottom-up shape
//! facts make nested brackets linear too; top-down context resets at exactly
//! the old walk's statement/block boundaries, including on recovery trees.
use super::ParsedFile;
use std::collections::{HashMap, HashSet};
use tree_sitter::Node;

#[derive(Clone, Copy, Default)]
struct Shape {
    ty: bool,
    generic: bool,
}

fn shape(n: Node<'_>, p: &ParsedFile, shapes: &HashMap<usize, Shape>) -> Shape {
    let get = |n: Option<Node<'_>>| {
        n.and_then(|n| shapes.get(&n.id()).copied())
            .unwrap_or_default()
    };
    let ty = match n.kind() {
        "identifier" => true,
        "selector_expression" => n
            .child_by_field_name("operand")
            .is_some_and(|b| b.kind() == "identifier"),
        "unary_expression" => {
            n.child_by_field_name("operator")
                .is_some_and(|o| p.node_text(&o) == "*")
                && get(n.child_by_field_name("operand")).ty
        }
        "parenthesized_expression" => get(n.named_child(0)).ty,
        "index_expression" => {
            n.child_by_field_name("operand").is_some_and(|b| {
                matches!(b.kind(), "identifier" | "selector_expression") && get(Some(b)).ty
            }) && get(n.child_by_field_name("index")).ty
        }
        _ => false,
    };
    let generic = match n.kind() {
        "index_expression" => true,
        "unary_expression" | "parenthesized_expression" => get(n.named_child(0)).generic,
        _ => false,
    };
    Shape { ty, generic }
}

#[derive(Clone, Default)]
pub(super) struct TypeContexts {
    nodes: HashSet<usize>,
    #[cfg(test)]
    pub(super) visits: usize,
}

impl TypeContexts {
    pub(super) fn build(p: &ParsedFile) -> Self {
        let mut out = Self::default();
        let mut shapes = HashMap::new();
        let mut cursor = p.tree.walk();
        // Postorder: every consulted child shape is already available.
        loop {
            if cursor.goto_first_child() {
                continue;
            }
            loop {
                let n = cursor.node();
                #[cfg(test)]
                {
                    out.visits += 1;
                }
                if matches!(
                    n.kind(),
                    "identifier"
                        | "selector_expression"
                        | "unary_expression"
                        | "parenthesized_expression"
                        | "index_expression"
                ) {
                    shapes.insert(n.id(), shape(n, p, &shapes));
                }
                if cursor.goto_next_sibling() {
                    break;
                }
                if !cursor.goto_parent() {
                    break;
                }
            }
            if cursor.node() == p.tree.root_node() {
                break;
            }
        }

        #[derive(Default)]
        struct Frame {
            active: bool,
            call: bool,
            call_index: bool,
        }
        let mut parents: Vec<Frame> = Vec::new();
        let mut cursor = p.tree.walk();
        // Preorder: carry parent facts, rather than rediscovering ancestors.
        loop {
            let n = cursor.node();
            #[cfg(test)]
            {
                out.visits += 1;
            }
            let parent = parents.last();
            let call_index = n.kind() == "index_expression"
                && parent.is_some_and(|f| f.call)
                && cursor.field_name() == Some("function");
            let get = |n: Option<Node<'_>>| {
                n.and_then(|n| shapes.get(&n.id()).copied())
                    .unwrap_or_default()
            };
            let call_arg = parent.is_some_and(|f| f.call_index)
                && cursor.field_name() == Some("index")
                && get(Some(n)).ty;
            let base = get(n.child_by_field_name("operand"));
            let index = get(n.child_by_field_name("index"));
            let nested = n.kind() == "index_expression"
                && !call_index
                && base.ty
                && index.ty
                && (base.generic || index.generic);
            let boundary = matches!(n.kind(), "statement_list" | "block" | "source_file");
            let active = call_arg || nested || (!boundary && parent.is_some_and(|f| f.active));
            if active {
                out.nodes.insert(n.id());
            }
            parents.push(Frame {
                active,
                call: n.kind() == "call_expression",
                call_index,
            });
            if cursor.goto_first_child() {
                continue;
            }
            loop {
                parents.pop();
                if cursor.goto_next_sibling() {
                    break;
                }
                if !cursor.goto_parent() {
                    return out;
                }
            }
        }
    }

    pub(super) fn contains(&self, node: Node<'_>) -> bool {
        self.nodes.contains(&node.id())
    }
}
