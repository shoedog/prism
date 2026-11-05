//! S1 Go syntax certificates: genuine writes and maximal eager value reads.
use crate::access_path::{AccessPath, DEFAULT_K_LIMIT};
use crate::ast::ParsedFile;
use std::collections::BTreeMap;
use tree_sitter::Node;

#[derive(Clone, Debug)]
pub(super) struct Occurrence {
    pub path: AccessPath,
    pub line: usize,
    pub start: usize,
    pub end: usize,
}

#[derive(Default)]
pub(super) struct Facts {
    pub writes: BTreeMap<(AccessPath, usize), usize>,
    pub reads: Vec<Occurrence>,
}

/// Parse the entire path, refusing indices and depth truncation. Dereference
/// normalizes storage exactly as the owner's existing DefSites do.
pub(super) fn path(parsed: &ParsedFile, node: Node<'_>) -> Option<AccessPath> {
    match node.kind() {
        "identifier" => Some(AccessPath::simple(parsed.node_text(&node))),
        "selector_expression" => {
            let mut p = path(parsed, node.child_by_field_name("operand")?)?;
            let field = node.child_by_field_name("field")?;
            if field.kind() != "field_identifier" || p.fields.len() == DEFAULT_K_LIMIT {
                return None;
            }
            p.fields.push(parsed.node_text(&field).to_owned());
            Some(p)
        }
        "parenthesized_expression" => path(parsed, node.named_child(0)?),
        "unary_expression" if parsed.node_text(&node.child_by_field_name("operator")?) == "*" => {
            path(parsed, node.child_by_field_name("operand")?)
        }
        _ => None,
    }
}

impl Facts {
    pub fn new(parsed: &ParsedFile, owner: Node<'_>) -> Self {
        let mut facts = Self::default();
        if !owner.has_error() {
            if let Some(body) = owner.child_by_field_name("body") {
                facts.walk(parsed, body);
            }
        }
        facts
    }

    fn write(&mut self, parsed: &ParsedFile, node: Node<'_>) {
        if let Some(p) = path(parsed, node) {
            self.writes.insert((p, node.start_byte()), node.end_byte());
        }
        // An LHS map key is evaluated; the storage operand is not an aggregate read.
        if node.kind() == "index_expression" {
            if let Some(index) = node.child_by_field_name("index") {
                self.value(parsed, index);
            }
        }
    }

    fn walk(&mut self, parsed: &ParsedFile, node: Node<'_>) {
        match node.kind() {
            "func_literal" | "function_declaration" | "method_declaration" => return,
            "assignment_statement" | "short_var_declaration" => {
                if let Some(left) = node.child_by_field_name("left") {
                    let mut cursor = left.walk();
                    for child in left.named_children(&mut cursor) {
                        self.write(parsed, child);
                    }
                }
                if let Some(right) = node.child_by_field_name("right") {
                    self.value(parsed, right);
                }
                return;
            }
            "var_spec" => {
                let mut cursor = node.walk();
                for name in node.children_by_field_name("name", &mut cursor) {
                    self.write(parsed, name);
                }
                if let Some(value) = node.child_by_field_name("value") {
                    self.value(parsed, value);
                }
                return;
            }
            "inc_statement" | "dec_statement" => {
                if let Some(value) = node.named_child(0) {
                    self.write(parsed, value);
                }
                return;
            }
            "return_statement" => {
                if let Some(value) = node.named_child(0) {
                    self.value(parsed, value);
                }
                return;
            }
            "send_statement" => {
                if let Some(value) = node.child_by_field_name("value") {
                    self.value(parsed, value);
                }
                return;
            }
            "call_expression" => {
                self.value(parsed, node);
                return;
            }
            // Type-only identifiers are never consumers.
            "type_declaration" | "type_spec" => return,
            _ => {}
        }
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            self.walk(parsed, child);
        }
    }

    fn value(&mut self, parsed: &ParsedFile, node: Node<'_>) {
        if let Some(p) = path(parsed, node) {
            // Keep the actual identifier/selector span inside &/*/parentheses,
            // so an existing Use can be reused without changing its identity.
            let mut actual = node;
            while matches!(
                actual.kind(),
                "parenthesized_expression" | "unary_expression"
            ) {
                actual = actual
                    .child_by_field_name("operand")
                    .or_else(|| actual.named_child(0))
                    .unwrap();
            }
            self.reads.push(Occurrence {
                path: p,
                line: actual.start_position().row + 1,
                start: actual.start_byte(),
                end: actual.end_byte(),
            });
            return;
        }
        match node.kind() {
            "func_literal" => {}
            "call_expression" => {
                // S1 excludes the callable/receiver, but arguments are eager
                // even in defer/go statements and nested calls.
                if let Some(args) = node.child_by_field_name("arguments") {
                    self.value(parsed, args);
                }
            }
            "composite_literal" => {
                if let Some(body) = node.child_by_field_name("body") {
                    self.value(parsed, body);
                }
            }
            "unary_expression" => {
                if node
                    .child_by_field_name("operator")
                    .is_some_and(|op| matches!(parsed.node_text(&op), "&" | "*"))
                {
                    if let Some(operand) = node.child_by_field_name("operand") {
                        self.value(parsed, operand);
                    }
                } else {
                    self.walk_children(parsed, node);
                }
            }
            "expression_list"
            | "argument_list"
            | "literal_value"
            | "literal_element"
            | "parenthesized_expression" => {
                let mut cursor = node.walk();
                for child in node.named_children(&mut cursor) {
                    self.value(parsed, child);
                }
            }
            "keyed_element" => {
                if let Some(value) = node.child_by_field_name("value") {
                    self.value(parsed, value);
                }
                // Bare keys of struct literals are field names. Only a
                // syntactically proven map literal admits its key as a value.
                let mut parent = node.parent();
                while let Some(p) = parent {
                    // An elided literal (`{Name: "v"}` as a map element) has no
                    // type of its own here: its keys are never proven values.
                    if p.kind() == "literal_value"
                        && p.parent().is_some_and(|q| q.kind() == "literal_element")
                    {
                        break;
                    }
                    if p.kind() == "composite_literal" {
                        if p.child_by_field_name("type")
                            .is_some_and(|t| t.kind() == "map_type")
                        {
                            if let Some(key) = node.child_by_field_name("key") {
                                self.value(parsed, key);
                            }
                        }
                        break;
                    }
                    parent = p.parent();
                }
            }
            "index_expression" => {
                if let Some(index) = node.child_by_field_name("index") {
                    self.value(parsed, index);
                }
            }
            // Binary identity tests, selector prefixes and type operands do
            // not consume aggregates. Still discover calls nested within them.
            _ => self.walk_children(parsed, node),
        }
    }

    fn walk_children(&mut self, parsed: &ParsedFile, node: Node<'_>) {
        let mut cursor = node.walk();
        for child in node.named_children(&mut cursor) {
            self.walk(parsed, child);
        }
    }
}
