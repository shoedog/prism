//! Go callback registration candidates.

use crate::ast::{GoRegistrationCandidate, GoRegistrationForm, ParsedFile};
use std::collections::BTreeSet;
use tree_sitter::Node;

/// A `keyed_element`'s `key`/`value` field is a `literal_element`, which wraps
/// a single `_expression` (or `literal_value`) child. Returns that child when
/// it is a plain `identifier` — the shape form (a) needs for both the field
/// name and the registered value (tree-sitter-go does not use
/// `field_identifier` for composite-literal keys, since the parser can't
/// distinguish a struct field key from a map key syntactically).
fn literal_element_identifier<'a>(node: &Node<'a>) -> Option<Node<'a>> {
    if node.kind() != "literal_element" {
        return None;
    }
    let inner = node.named_child(0)?;
    (inner.kind() == "identifier").then_some(inner)
}

impl ParsedFile {
    /// P5 S2 (Go func-value callbacks): find registration candidates within
    /// `func_node`, restricted to `lines` (the same `_on_lines` gating idiom
    /// `function_calls_with_qualifier_and_spans_on_lines` uses). Raw
    /// extraction only — target resolution, the shadow check, and per-form
    /// owner recovery/field-typing gates all live in `call_graph.rs`
    /// (`CallGraph::apply_go_registration_candidate`), since they need
    /// whole-program CallGraph state (`functions`, `imports`,
    /// `go_func_typed_fields`) this AST layer doesn't have.
    ///
    /// Go has no anonymous-function node in `function_node_types()`
    /// (`languages/mod.rs`: only `function_declaration`/`method_declaration`),
    /// so `all_functions()` never visits a nested closure separately — a
    /// registration inside one is naturally attributed to the enclosing named
    /// function by this recursive walk, with no double-counting to guard
    /// against.
    pub(crate) fn go_registration_candidates(
        &self,
        func_node: &Node<'_>,
        lines: &BTreeSet<usize>,
    ) -> Vec<GoRegistrationCandidate> {
        let mut out = Vec::new();
        self.collect_go_registration_candidates(*func_node, lines, &mut out);
        out
    }

    fn collect_go_registration_candidates(
        &self,
        node: Node<'_>,
        lines: &BTreeSet<usize>,
        out: &mut Vec<GoRegistrationCandidate>,
    ) {
        match node.kind() {
            // Form (a): `Command{Run: helper}` — a keyed field in a composite
            // literal whose value is a bare identifier.
            "composite_literal" => {
                if let (Some(type_node), Some(body)) = (
                    node.child_by_field_name("type"),
                    node.child_by_field_name("body"),
                ) {
                    let struct_type_text = self.node_text(&type_node).trim().to_string();
                    let mut cursor = body.walk();
                    for child in body.children(&mut cursor) {
                        if child.kind() != "keyed_element" {
                            continue;
                        }
                        let (Some(key), Some(value)) = (
                            child.child_by_field_name("key"),
                            child.child_by_field_name("value"),
                        ) else {
                            continue;
                        };
                        let (Some(key_ident), Some(value_ident)) = (
                            literal_element_identifier(&key),
                            literal_element_identifier(&value),
                        ) else {
                            continue;
                        };
                        let line = value_ident.start_position().row + 1;
                        if !lines.contains(&line) {
                            continue;
                        }
                        out.push(GoRegistrationCandidate {
                            form: GoRegistrationForm::CompositeLiteralField {
                                struct_type_text: struct_type_text.clone(),
                                field_name: self.node_text(&key_ident).trim().to_string(),
                            },
                            value_name: self.node_text(&value_ident).trim().to_string(),
                            line,
                            start_byte: value_ident.start_byte(),
                            end_byte: value_ident.end_byte(),
                        });
                    }
                }
            }
            // Form (b): `x.Run = helper` — assignment to a selector whose
            // value is a bare identifier. Only the single-target,
            // single-value shape (`expression_list` of length 1 on both
            // sides) is recognized; multi-assignment (`a, b = x, y`) is out
            // of scope.
            "assignment_statement" => {
                if let (Some(left), Some(right)) = (
                    node.child_by_field_name("left"),
                    node.child_by_field_name("right"),
                ) {
                    let mut lcursor = left.walk();
                    let left_items: Vec<_> = left.named_children(&mut lcursor).collect();
                    let mut rcursor = right.walk();
                    let right_items: Vec<_> = right.named_children(&mut rcursor).collect();
                    if left_items.len() == 1 && right_items.len() == 1 {
                        let sel = left_items[0];
                        let value_ident = right_items[0];
                        if sel.kind() == "selector_expression" && value_ident.kind() == "identifier"
                        {
                            if let (Some(operand), Some(field)) = (
                                sel.child_by_field_name("operand"),
                                sel.child_by_field_name("field"),
                            ) {
                                if operand.kind() == "identifier" {
                                    let line = value_ident.start_position().row + 1;
                                    if lines.contains(&line) {
                                        out.push(GoRegistrationCandidate {
                                            form: GoRegistrationForm::FieldAssignment {
                                                operand_name: self
                                                    .node_text(&operand)
                                                    .trim()
                                                    .to_string(),
                                                field_name: self
                                                    .node_text(&field)
                                                    .trim()
                                                    .to_string(),
                                                assign_line: node.start_position().row + 1,
                                                assign_start_byte: node.start_byte(),
                                            },
                                            value_name: self
                                                .node_text(&value_ident)
                                                .trim()
                                                .to_string(),
                                            line,
                                            start_byte: value_ident.start_byte(),
                                            end_byte: value_ident.end_byte(),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
            }
            // Form (c): `Register(helper)` — a bare identifier passed
            // directly as a call argument (nested expressions, e.g.
            // `Register(wrap(helper))`'s outer argument, are excluded here;
            // the recursive walk below still visits `wrap(helper)` as its own
            // call_expression, so `helper` is still found as an argument of
            // THAT call).
            "call_expression" => {
                if let Some(args) = node.child_by_field_name("arguments") {
                    let mut cursor = args.walk();
                    for arg in args.named_children(&mut cursor) {
                        if arg.kind() != "identifier" {
                            continue;
                        }
                        let line = arg.start_position().row + 1;
                        if !lines.contains(&line) {
                            continue;
                        }
                        out.push(GoRegistrationCandidate {
                            form: GoRegistrationForm::CallArgument,
                            value_name: self.node_text(&arg).trim().to_string(),
                            line,
                            start_byte: arg.start_byte(),
                            end_byte: arg.end_byte(),
                        });
                    }
                }
            }
            _ => {}
        }
        let mut cursor = node.walk();
        for child in node.children(&mut cursor) {
            self.collect_go_registration_candidates(child, lines, out);
        }
    }
}
