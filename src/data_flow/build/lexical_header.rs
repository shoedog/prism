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
    if name == "_" {
        return Occurrence::Other;
    }
    let mut child = occ;
    while let Some(parent) = child.parent() {
        match parent.kind() {
            "func_literal" if declares_name(parsed, parent, name) => return Occurrence::Other,
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
                        return match region(parent, child, occ, init) {
                            Region::Header => span,
                            // The name is in scope in the body, unless something in it
                            // declares the same spelling again.
                            Region::Body if !declares_name(parsed, child, name) => span,
                            _ => Occurrence::Other,
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
                    | "const_spec"
                    | "type_switch_statement" => true,
                    "expression_list" => p.parent().is_some_and(|g| {
                        matches!(g.kind(), "short_var_declaration" | "range_clause")
                            && g.child_by_field_name("left")
                                .is_some_and(|l| l.id() == p.id())
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
