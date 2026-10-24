use super::super::binding_table::{guard_span_row, java_statement_cannot_complete_normally};
use crate::ast::ParsedFile;
use crate::languages::Language;
use tree_sitter::Node;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::cpg::reaching) struct ByteSpan {
    pub(in crate::cpg::reaching) start: usize,
    pub(in crate::cpg::reaching) end: usize,
}

impl ByteSpan {
    pub(in crate::cpg::reaching) fn contains(self, byte: usize) -> bool {
        self.start <= byte && byte < self.end
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::cpg::reaching) enum SpanKind {
    Known(Vec<ByteSpan>),
    Empty,
    Unknown,
}

pub(in crate::cpg::reaching) fn visibility_span(
    parsed: &ParsedFile,
    binder_node: Node<'_>,
) -> SpanKind {
    if let Some(row) = guard_span_row(parsed.language, binder_node.kind()) {
        let Some(guard) = ancestor_of_kind(binder_node, row.guard_kind) else {
            return SpanKind::Unknown;
        };
        let Some(owner) = ancestor_of_kind(guard, row.owner_kind) else {
            return SpanKind::Unknown;
        };
        let Some(condition) = owner.child_by_field_name(row.condition_field) else {
            return SpanKind::Unknown;
        };
        if !(condition.start_byte() <= guard.start_byte()
            && guard.end_byte() <= condition.end_byte())
        {
            return SpanKind::Unknown;
        }
        let Some(negated) = guard_is_negated(parsed, guard, condition) else {
            return SpanKind::Unknown;
        };
        let mut spans = Vec::new();
        if !negated && row.positive_condition_tail && binder_node.end_byte() < condition.end_byte()
        {
            spans.push(ByteSpan {
                start: binder_node.end_byte(),
                end: condition.end_byte(),
            });
        }
        let fields = if negated {
            row.negative_fields
        } else {
            row.positive_fields
        };
        for field in fields {
            if let Some(node) = owner.child_by_field_name(field) {
                spans.push(ByteSpan {
                    start: node.start_byte(),
                    end: node.end_byte(),
                });
            }
        }
        if negated
            && java_statement_cannot_complete_normally(owner.child_by_field_name("consequence"))
        {
            if let Some(parent) = owner.parent() {
                if owner.end_byte() < parent.end_byte() {
                    spans.push(ByteSpan {
                        start: owner.end_byte(),
                        end: parent.end_byte(),
                    });
                }
            }
        }
        return if spans.is_empty() {
            SpanKind::Empty
        } else {
            SpanKind::Known(spans)
        };
    }
    if parsed.language == Language::Python && super::is_python_match_pattern(binder_node) {
        let Some(case_clause) = ancestor_of_kind(binder_node, "case_clause") else {
            return SpanKind::Unknown;
        };
        return if binder_node.end_byte() < case_clause.end_byte() {
            SpanKind::Known(vec![ByteSpan {
                start: binder_node.end_byte(),
                end: case_clause.end_byte(),
            }])
        } else {
            SpanKind::Empty
        };
    }
    let scoped = match parsed.language {
        Language::Rust => rust_visibility_node(binder_node),
        Language::Python => ancestor_field(binder_node, "if_statement", "consequence"),
        Language::Go | Language::JavaScript | Language::TypeScript | Language::Tsx => {
            nearest_field(binder_node, "body")
        }
        _ => None,
    };
    let Some(scoped) = scoped else {
        return SpanKind::Unknown;
    };
    if scoped.named_child_count() == 0 {
        SpanKind::Empty
    } else {
        SpanKind::Known(vec![ByteSpan {
            start: scoped.start_byte(),
            end: scoped.end_byte(),
        }])
    }
}

fn ancestor_of_kind<'a>(mut node: Node<'a>, kind: &str) -> Option<Node<'a>> {
    loop {
        if node.kind() == kind {
            return Some(node);
        }
        node = node.parent()?;
    }
}

fn ancestor_field<'a>(mut node: Node<'a>, kind: &str, field: &str) -> Option<Node<'a>> {
    while let Some(parent) = node.parent() {
        if parent.kind() == kind {
            return parent.child_by_field_name(field);
        }
        node = parent;
    }
    None
}

fn nearest_field<'a>(mut node: Node<'a>, field: &str) -> Option<Node<'a>> {
    while let Some(parent) = node.parent() {
        if let Some(scoped) = parent.child_by_field_name(field) {
            return Some(scoped);
        }
        node = parent;
    }
    None
}

fn rust_visibility_node(binder_node: Node<'_>) -> Option<Node<'_>> {
    let mut current = binder_node;
    while let Some(parent) = current.parent() {
        match parent.kind() {
            "match_arm" => return parent.child_by_field_name("value"),
            "if_expression" => return parent.child_by_field_name("consequence"),
            "while_expression" => return parent.child_by_field_name("body"),
            _ => current = parent,
        }
    }
    None
}

fn guard_is_negated(parsed: &ParsedFile, guard: Node<'_>, condition: Node<'_>) -> Option<bool> {
    let mut current = guard;
    let mut negated = false;
    let mut accepted_and = false;
    loop {
        if current.id() == condition.id() {
            return Some(negated);
        }
        let parent = current.parent()?;
        match parent.kind() {
            "parenthesized_expression" => {}
            "unary_expression" => {
                let operator = parent.child_by_field_name("operator")?;
                if parsed.node_text(&operator) != "!" || accepted_and {
                    return None;
                }
                negated = !negated;
            }
            "binary_expression" => {
                let operator = parent.child_by_field_name("operator")?;
                let left = parent.child_by_field_name("left")?;
                if negated || parsed.node_text(&operator) != "&&" || left.id() != current.id() {
                    return None;
                }
                accepted_and = true;
            }
            _ => return None,
        }
        current = parent;
    }
}
