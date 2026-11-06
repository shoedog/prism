//! #662: lexical binding of uses in a Go `if`/`switch`/`for` header whose
//! initializer declares the name. The use reads the header's own variable, so a
//! def from outside that statement cannot reach it. Removal-only: any shape not
//! proved here keeps the pair.
use crate::ast::ParsedFile;
use tree_sitter::Node;

/// What one identifier occurrence on a line is, relative to header declarations.
pub(super) enum Occurrence {
    /// The declaring name in a header initializer: neither a read nor an outer use.
    Declaration,
    /// A use bound to the declaration of the header statement with this byte span.
    Bound(usize, usize),
    /// Anything else: the line cannot be proven header-bound.
    Other,
}

pub(super) fn classify(parsed: &ParsedFile, occ: Node<'_>) -> Occurrence {
    let name = parsed.node_text(&occ);
    if name == "_" || is_composite_key(occ) || is_storage_access(occ) {
        return Occurrence::Other;
    }
    // Closures between the occurrence and the claiming header; scanned only once a
    // header claims the use.
    let mut literals: Vec<Node<'_>> = Vec::new();
    let mut child = occ;
    while let Some(parent) = child.parent() {
        match parent.kind() {
            "func_literal" => literals.push(parent),
            "if_statement" | "expression_switch_statement" | "for_statement" => {
                if let Some(init) = initializer(parent) {
                    if init_declares(parsed, init, name) {
                        if init.start_byte() <= occ.start_byte()
                            && occ.end_byte() <= init.end_byte()
                        {
                            return if in_left(init, occ) {
                                Occurrence::Declaration
                            } else {
                                Occurrence::Other
                            };
                        }
                        let span = Occurrence::Bound(parent.start_byte(), parent.end_byte());
                        let claimed = match region(parent, child, occ, init) {
                            Region::Header => true,
                            // The name is in scope in the body, unless something in it
                            // declares the same spelling again.
                            Region::Body => !declares_name(parsed, child, name),
                            Region::Outside => false,
                        };
                        return if claimed
                            && !literals.iter().any(|l| declares_name(parsed, *l, name))
                        {
                            span
                        } else {
                            Occurrence::Other
                        };
                    }
                }
            }
            _ => {}
        }
        child = parent;
    }
    Occurrence::Other
}

/// `S{x: 1}`: the key names a field (or is a map key read), never the plain use.
fn is_composite_key(occ: Node<'_>) -> bool {
    let Some(element) = occ.parent().filter(|p| p.kind() == "literal_element") else {
        return false;
    };
    element.parent().is_some_and(|k| {
        k.kind() == "keyed_element"
            && k.child_by_field_name("key")
                .is_some_and(|key| key.id() == element.id())
    })
}

/// A dereference, index/slice operand or field operand (outside a call's callee) reads
/// storage reachable through the name, which a shadowing copy can alias: not provable.
fn is_storage_access(occ: Node<'_>) -> bool {
    let mut node = occ;
    while let Some(p) = node.parent() {
        match p.kind() {
            "parenthesized_expression" => {}
            "unary_expression" => {
                return p
                    .child_by_field_name("operator")
                    .is_some_and(|op| op.kind() == "*")
                    && p.child_by_field_name("operand")
                        .is_some_and(|o| o.id() == node.id())
            }
            "index_expression" | "slice_expression" => {
                return p
                    .child_by_field_name("operand")
                    .is_some_and(|o| o.id() == node.id())
            }
            "selector_expression" => {
                return p
                    .child_by_field_name("operand")
                    .is_some_and(|o| o.id() == node.id())
                    && !p.parent().is_some_and(|c| {
                        c.kind() == "call_expression"
                            && c.child_by_field_name("function")
                                .is_some_and(|f| f.id() == p.id())
                    })
            }
            _ => return false,
        }
        node = p;
    }
    false
}

fn initializer(statement: Node<'_>) -> Option<Node<'_>> {
    let holder = if statement.kind() == "for_statement" {
        let mut cursor = statement.walk();
        let clause = statement
            .named_children(&mut cursor)
            .find(|n| n.kind() == "for_clause");
        clause?
    } else {
        statement
    };
    holder
        .child_by_field_name("initializer")
        .filter(|n| n.kind() == "short_var_declaration")
}

fn init_declares(parsed: &ParsedFile, init: Node<'_>, name: &str) -> bool {
    init.child_by_field_name("left").is_some_and(|left| {
        let mut cursor = left.walk();
        let found = left
            .named_children(&mut cursor)
            .any(|n| n.kind() == "identifier" && parsed.node_text(&n) == name);
        found
    })
}

fn in_left(init: Node<'_>, occ: Node<'_>) -> bool {
    init.child_by_field_name("left")
        .is_some_and(|l| l.start_byte() <= occ.start_byte() && occ.end_byte() <= l.end_byte())
}

enum Region {
    Header,
    Body,
    Outside,
}

/// `child` is the direct child of `statement` that holds the occurrence.
fn region(statement: Node<'_>, child: Node<'_>, occ: Node<'_>, init: Node<'_>) -> Region {
    let is = |field: &str| {
        statement
            .child_by_field_name(field)
            .is_some_and(|n| n.id() == child.id())
    };
    match statement.kind() {
        "if_statement" if is("condition") => Region::Header,
        "if_statement" if is("consequence") || is("alternative") => Region::Body,
        "expression_switch_statement" if is("value") => Region::Header,
        "expression_switch_statement"
            if matches!(child.kind(), "expression_case" | "default_case") =>
        {
            Region::Body
        }
        // The clause's condition and post, never its initializer.
        "for_statement" if child.kind() == "for_clause" && occ.start_byte() >= init.end_byte() => {
            Region::Header
        }
        "for_statement" if child.kind() == "block" => Region::Body,
        _ => Region::Outside,
    }
}

/// Parameter, `:=`, `var`, `const` or range declarations of `name` inside a literal.
fn declares_name(parsed: &ParsedFile, literal: Node<'_>, name: &str) -> bool {
    fn walk(parsed: &ParsedFile, node: Node<'_>, name: &str) -> bool {
        if node.kind() == "identifier" && parsed.node_text(&node) == name {
            if let Some(p) = node.parent() {
                let declared = match p.kind() {
                    "parameter_declaration"
                    | "variadic_parameter_declaration"
                    | "var_spec"
                    | "const_spec" => true,
                    "expression_list" => p.parent().is_some_and(|g| match g.kind() {
                        "short_var_declaration" | "range_clause" => g
                            .child_by_field_name("left")
                            .is_some_and(|l| l.id() == p.id()),
                        // `case v := <-ch` declares; `case v = <-ch` assigns.
                        "receive_statement" => {
                            g.child_by_field_name("left")
                                .is_some_and(|l| l.id() == p.id())
                                && has_token(g, ":=")
                        }
                        // `switch v := x.(type)`.
                        "type_switch_statement" => g
                            .child_by_field_name("alias")
                            .is_some_and(|a| a.id() == p.id()),
                        _ => false,
                    }),
                    "short_var_declaration" | "range_clause" => true,
                    _ => false,
                };
                if declared {
                    return true;
                }
            }
        }
        let mut cursor = node.walk();
        let found = node
            .named_children(&mut cursor)
            .any(|c| walk(parsed, c, name));
        found
    }
    walk(parsed, literal, name)
}

fn has_token(node: Node<'_>, kind: &str) -> bool {
    let mut cursor = node.walk();
    let found = node.children(&mut cursor).any(|c| c.kind() == kind);
    found
}
