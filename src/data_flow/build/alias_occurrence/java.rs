//! Java occurrence rows `J-01..J-06` (alias-twin lane, slice 1).
//!
//! The classifier never reads a type name: a Java local of any type is a reference or value
//! slot, `u = x` rebinds the slot and never writes the object `x` names, and compound assignment
//! exists only for numeric, boxed and `String` operands, all of which rebind. The only evidence
//! is the syntactic form of the occurrence and whether a local/parameter declaration of the name
//! is visible at it (scope and position). Missing evidence (implicit field, outer scope, a
//! same-named local in another block, anything the table does not name) is `Unsupported`.
use super::{Classified, Disposition};
use crate::ast::{ParsedFile, PathSpan};
use tree_sitter::Node;

const fn done(disposition: Disposition, rule: &'static str) -> Classified {
    Classified { disposition, rule }
}

fn field_in_parent(node: Node<'_>) -> Option<&'static str> {
    let parent = node.parent()?;
    let mut cursor = parent.walk();
    if !cursor.goto_first_child() {
        return None;
    }
    loop {
        if cursor.node().id() == node.id() {
            return cursor.field_name();
        }
        if !cursor.goto_next_sibling() {
            return None;
        }
    }
}

fn is_plain_ident(s: &str) -> bool {
    s.chars()
        .next()
        .is_some_and(|c| c.is_alphabetic() || c == '_' || c == '$')
        && s.chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '$')
}

/// An occurrence in a position the grammar READS (call argument, operand, initialiser value,
/// receiver): the span extraction records it as an lvalue, which is an extraction defect repaired
/// in its own slice. Such an occurrence keeps today's behaviour (`X-READ`).
fn read_position(parent_kind: &str, field: &str) -> bool {
    matches!(
        parent_kind,
        "argument_list"
            | "binary_expression"
            | "cast_expression"
            | "ternary_expression"
            | "unary_expression"
            | "parenthesized_expression"
            | "method_invocation"
            | "array_access"
    ) || (parent_kind == "variable_declarator" && field == "value")
}

/// Classify the lvalue occurrence `span` of a Java file.
pub(super) fn classify(parsed: &ParsedFile, span: &PathSpan) -> Classified {
    let unrecognised = done(Disposition::Unsupported, "X-UNRECOGNISED");
    let Some(mut node) = parsed
        .tree
        .root_node()
        .descendant_for_byte_range(span.start_byte, span.end_byte)
    else {
        return unrecognised;
    };
    // The largest node with exactly the span's range (`S u;` spans the declarator, not its name).
    while let Some(parent) = node.parent() {
        if parent.start_byte() == span.start_byte && parent.end_byte() == span.end_byte {
            node = parent;
        } else {
            break;
        }
    }
    if node.start_byte() != span.start_byte || node.end_byte() != span.end_byte {
        return unrecognised;
    }
    let Some(parent) = node.parent() else {
        return unrecognised;
    };
    let field = field_in_parent(node).unwrap_or("");
    if read_position(parent.kind(), field) {
        return done(Disposition::Unsupported, "X-READ");
    }
    let kind = node.kind();
    let is_name = matches!(kind, "identifier" | "variable_declarator")
        && is_plain_ident(parsed.node_text(&node).trim());
    let assignment_left = parent.kind() == "assignment_expression" && field == "left";
    let base = span.path.base.as_str();
    let visible = || visible_declaration(parsed, node, base, span.start_byte);

    if is_name && assignment_left {
        // J-01: a plain or compound assignment to a name with a visible local or parameter.
        return done(
            if visible() {
                Disposition::BindingWrite
            } else {
                Disposition::Unsupported
            },
            "J-01",
        );
    }
    if is_name && kind == "identifier" && parent.kind() == "variable_declarator" && field == "name"
    {
        return match parent.parent().map(|g| g.kind()) {
            Some("local_variable_declaration") => done(Disposition::BindingWrite, "J-02"),
            Some("field_declaration") => done(Disposition::Unsupported, "J-04"),
            _ => unrecognised,
        };
    }
    if is_name && kind == "variable_declarator" {
        // A declaration without an initialiser: the declarator spans just the name.
        return match parent.kind() {
            "local_variable_declaration" => done(Disposition::BindingWrite, "J-03"),
            "field_declaration" => done(Disposition::Unsupported, "J-04"),
            _ => unrecognised,
        };
    }
    if assignment_left && kind == "field_access" {
        return done(
            if visible() {
                Disposition::StorageWrite
            } else {
                Disposition::Unsupported
            },
            "J-05",
        );
    }
    if assignment_left && kind == "array_access" {
        return done(
            if visible() {
                Disposition::StorageWrite
            } else {
                Disposition::Unsupported
            },
            "J-06",
        );
    }
    unrecognised
}

/// Is a local variable or parameter named `name` declared at a point whose scope holds the
/// occurrence? Walks the occurrence's ancestors up to the nearest callable; stops at a class
/// body or initialiser, whose members are fields (a field of an anonymous class shadows an
/// outer local, and assigning a captured local is not Java), and never reads a type.
fn visible_declaration(parsed: &ParsedFile, occurrence: Node<'_>, name: &str, byte: usize) -> bool {
    let mut current = occurrence.parent();
    while let Some(ancestor) = current {
        if declares_in_scope(parsed, ancestor, name, byte) {
            return true;
        }
        if matches!(
            ancestor.kind(),
            "method_declaration"
                | "constructor_declaration"
                | "compact_constructor_declaration"
                | "lambda_expression"
                | "class_body"
                | "interface_body"
                | "enum_body"
                | "annotation_type_body"
                | "static_initializer"
                | "program"
        ) {
            return false;
        }
        current = ancestor.parent();
    }
    false
}

/// Does `scope` itself declare `name` for the occurrence at `byte`?
fn declares_in_scope(parsed: &ParsedFile, scope: Node<'_>, name: &str, byte: usize) -> bool {
    let named = |n: Option<Node<'_>>| n.is_some_and(|n| parsed.node_text(&n) == name);
    let inside =
        |n: Option<Node<'_>>| n.is_some_and(|n| n.start_byte() <= byte && byte < n.end_byte());
    match scope.kind() {
        "block" | "constructor_body" | "switch_block_statement_group" => {
            statements_declare(parsed, scope, name, byte)
        }
        "switch_block" => {
            let mut cursor = scope.walk();
            let declared = scope
                .children(&mut cursor)
                .filter(|group| group.kind() == "switch_block_statement_group")
                .any(|group| statements_declare(parsed, group, name, byte));
            declared
        }
        "for_statement" => {
            let mut cursor = scope.walk();
            let declared = scope
                .children_by_field_name("init", &mut cursor)
                .filter(|init| init.kind() == "local_variable_declaration")
                .any(|init| local_declaration_declares(parsed, init, name, byte));
            declared
        }
        // `for (T x : xs) body`: `x` is in scope in the body, not in the iterated expression.
        "enhanced_for_statement" => {
            named(scope.child_by_field_name("name")) && inside(scope.child_by_field_name("body"))
        }
        "catch_clause" => {
            let mut cursor = scope.walk();
            let declared = scope
                .children(&mut cursor)
                .filter(|child| child.kind() == "catch_formal_parameter")
                .any(|param| named(param.child_by_field_name("name")));
            declared && inside(scope.child_by_field_name("body"))
        }
        // Resources are in scope in later resources and the body, not in catch or finally.
        "try_with_resources_statement" => {
            let Some(body) = scope.child_by_field_name("body") else {
                return false;
            };
            let Some(resources) = scope.child_by_field_name("resources") else {
                return false;
            };
            let mut cursor = resources.walk();
            let declared = resources.children(&mut cursor).any(|resource| {
                resource.kind() == "resource"
                    && named(resource.child_by_field_name("name"))
                    && resource.start_byte() <= byte
            });
            declared && byte < body.end_byte()
        }
        "method_declaration"
        | "constructor_declaration"
        | "compact_constructor_declaration"
        | "lambda_expression" => parameters_declare(parsed, scope, name),
        _ => false,
    }
}

/// A `local_variable_declaration` child of `scope` that declares `name` before `byte`.
fn statements_declare(parsed: &ParsedFile, scope: Node<'_>, name: &str, byte: usize) -> bool {
    let mut cursor = scope.walk();
    let declared = scope
        .children(&mut cursor)
        .filter(|child| child.kind() == "local_variable_declaration")
        .any(|decl| local_declaration_declares(parsed, decl, name, byte));
    declared
}

fn local_declaration_declares(
    parsed: &ParsedFile,
    decl: Node<'_>,
    name: &str,
    byte: usize,
) -> bool {
    let mut cursor = decl.walk();
    let declared = decl
        .children_by_field_name("declarator", &mut cursor)
        .filter_map(|declarator| declarator.child_by_field_name("name"))
        .any(|id| parsed.node_text(&id) == name && id.start_byte() <= byte);
    declared
}

/// Formal parameters (explicitly typed for lambdas: inferred lambda parameters carry no
/// declaration evidence and stay unsupported).
fn parameters_declare(parsed: &ParsedFile, callable: Node<'_>, name: &str) -> bool {
    let Some(params) = callable.child_by_field_name("parameters") else {
        return false;
    };
    if params.kind() != "formal_parameters" {
        return false;
    }
    let mut cursor = params.walk();
    let declared = params
        .children(&mut cursor)
        .any(|param| match param.kind() {
            "formal_parameter" => param
                .child_by_field_name("name")
                .is_some_and(|id| parsed.node_text(&id) == name),
            "spread_parameter" => {
                let mut inner = param.walk();
                let found = param
                    .children(&mut inner)
                    .filter(|child| child.kind() == "variable_declarator")
                    .filter_map(|declarator| declarator.child_by_field_name("name"))
                    .any(|id| parsed.node_text(&id) == name);
                found
            }
            _ => false,
        });
    declared
}

/// Spellings the occurrence extraction can mistake for a variable inside `owner`: method names
/// (`x.size()`, `Foo::bar`) and the member names of another object (`node.key`; `this.key` names the
/// variable's own field and is not a token of this kind). Refusal R1 of the policy.
pub(super) fn member_name_tokens(
    parsed: &ParsedFile,
    owner: &Node<'_>,
) -> std::collections::BTreeSet<String> {
    let mut out = std::collections::BTreeSet::new();
    let mut stack = vec![*owner];
    while let Some(node) = stack.pop() {
        match node.kind() {
            "method_invocation" => {
                if let Some(name) = node.child_by_field_name("name") {
                    out.insert(parsed.node_text(&name).to_string());
                }
            }
            "field_access" => {
                let this_object = node
                    .child_by_field_name("object")
                    .is_some_and(|object| parsed.node_text(&object) == "this");
                if let (false, Some(field)) = (this_object, node.child_by_field_name("field")) {
                    out.insert(parsed.node_text(&field).to_string());
                }
            }
            "method_reference" => {
                let mut cursor = node.walk();
                for child in node.children(&mut cursor) {
                    if child.kind() == "identifier" {
                        out.insert(parsed.node_text(&child).to_string());
                    }
                }
            }
            _ => {}
        }
        let mut cursor = node.walk();
        stack.extend(node.children(&mut cursor));
    }
    out
}
