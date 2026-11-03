//! Keep the pre-0.25 distinction between broad value walks (which included
//! type_identifier) and the narrower `(identifier)` reference query. In
//! particular, the old parser read m[i][j] as a type instantiation even for
//! real containers. This view preserves that historical query behavior without
//! promoting the expression to a generic call or rewriting the raw parse tree.
use super::ParsedFile;
use crate::languages::Language;
use tree_sitter::Node;

fn type_shape(p: &ParsedFile, n: Node<'_>) -> bool {
    match n.kind() {
        "identifier" => true,
        "selector_expression" => n
            .child_by_field_name("operand")
            .is_some_and(|b| b.kind() == "identifier"),
        "unary_expression" => {
            n.child_by_field_name("operator")
                .is_some_and(|o| p.node_text(&o) == "*")
                && n.child_by_field_name("operand")
                    .is_some_and(|b| type_shape(p, b))
        }
        "parenthesized_expression" => n.named_child(0).is_some_and(|b| type_shape(p, b)),
        _ => false,
    }
}

fn nested_instantiation(p: &ParsedFile, n: Node<'_>) -> bool {
    if n.kind() != "index_expression" {
        return false;
    }
    let Some(inner) = n
        .child_by_field_name("operand")
        .filter(|n| n.kind() == "index_expression")
    else {
        return false;
    };
    inner.child_by_field_name("operand").is_some_and(|b| {
        matches!(b.kind(), "identifier" | "selector_expression") && type_shape(p, b)
    }) && inner
        .child_by_field_name("index")
        .is_some_and(|b| type_shape(p, b))
        && n.child_by_field_name("index")
            .is_some_and(|b| type_shape(p, b))
}

// The old parser chose type_arguments syntactically even for handlers[i]().
// Preserve that query inventory independently of semantic callee admission.
fn call_type_argument(p: &ParsedFile, n: Node<'_>) -> bool {
    n.parent().is_some_and(|index| {
        index.kind() == "index_expression"
            && index.child_by_field_name("index") == Some(n)
            && type_shape(p, n)
            && index
                .child_by_field_name("operand")
                .is_some_and(|base| matches!(base.kind(), "identifier" | "selector_expression"))
            && index.parent().is_some_and(|call| {
                call.kind() == "call_expression"
                    && call.child_by_field_name("function") == Some(index)
            })
    })
}

impl ParsedFile {
    pub(crate) fn is_go_legacy_type_context(&self, n: Node<'_>) -> bool {
        if self.language != Language::Go {
            return false;
        }
        let mut current = Some(n);
        while let Some(node) = current {
            if call_type_argument(self, node) || nested_instantiation(self, node) {
                return true;
            }
            if matches!(node.kind(), "statement_list" | "block" | "source_file") {
                break;
            }
            current = node.parent();
        }
        false
    }

    /// Preserve the old callee's value-use span even when the semantic call
    /// stays an unresolved container index. This never feeds call resolution.
    pub(super) fn call_value_name<'t>(&self, call: &Node<'t>) -> Option<Node<'t>> {
        let name = self.call_function_name(call)?;
        if self.language == Language::Go
            && name.kind() == "index_expression"
            && name
                .child_by_field_name("index")
                .is_some_and(|n| call_type_argument(self, n))
        {
            let base = name.child_by_field_name("operand")?;
            return Some(if base.kind() == "selector_expression" {
                base.child_by_field_name("field").unwrap_or(base)
            } else {
                base
            });
        }
        Some(name)
    }

    /// The old qualified_type emitted its type name, whereas a new selector
    /// would emit pkg.Name plus pkg. Keep the old broad path/span inventory.
    pub(super) fn compat_value_node<'t>(&self, n: Node<'t>) -> Option<Node<'t>> {
        if self.language.is_erased_type_boundary(n) {
            return None;
        }
        if n.kind() == "selector_expression" && self.is_go_legacy_type_context(n) {
            return Some(n.child_by_field_name("field").unwrap_or(n));
        }
        Some(n)
    }
}
