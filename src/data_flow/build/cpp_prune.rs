//! C++ additions to rules U and D (#630 slice 6c), consumed by `c_prune.rs`.
//! Design of record: `docs/superpowers/plans/2026-11-11-630-s6-cpp-ud/DESIGN.md` §5.1.
//!
//! C++ owners (`function_definition`, and a `template_declaration` holding one) run the C rules
//! plus (a `.h` file is parsed as C, #529, and stays off: the C grammar parses C++ shapes such
//! as `T& r = (x);` without an error, so the escapes below cannot see them):
//! - the scalar gate (E2): the declared type is a `primitive_type`, a `sized_type_specifier`,
//!   an `enum_specifier`, any pointer declarator, or `auto` initialised by a literal;
//!   references, class and typedef types, `decltype` and `auto` from anything else fail it;
//! - the C++ escape set (E3), owner-wide by name: every lvalue root (`escape_roots`) of an
//!   address-of operand and of the range of a range-for, a bare argument of any call, a
//!   constructor or brace initializer, an operand of `>>`, anything inside the initializer of a
//!   reference or structured binding, every name inside a lambda with a non-empty capture list
//!   (captures are read when the closure is created), a block-scope using-declaration's names,
//!   and the bare initializer of a declaration whose type fails the gate;
//! - owners that are off: a block-scope `using namespace`, a coroutine (`co_await`, `co_yield`,
//!   `co_return` outside nested lambdas: promise construction and `await_transform` /
//!   `yield_value` can bind a parameter or an operand by reference);
//! - bindings from `catch` parameters, condition and init-statement declarations and range-for
//!   variables (never dominating writes).
//!
//! Out of model (X3, disclosed): an overloaded operator other than `>>` that binds a scalar
//! operand by non-const reference and keeps it.
use super::c_prune::COwner;
use crate::ast::ParsedFile;
use tree_sitter::Node;

/// The identifiers an expression can hand over as an lvalue: itself, through parentheses, both
/// branches of `?:`, a comma sequence, the target of an assignment, an increment, or a cast to
/// a reference type. A call such as `f(c ? x : y)` binds a reference to `x`.
pub(super) fn lvalue_roots<'a>(parsed: &ParsedFile, node: Node<'a>, out: &mut Vec<Node<'a>>) {
    match node.kind() {
        "identifier" => out.push(node),
        "parenthesized_expression" => {
            if let Some(c) = node.named_child(0) {
                lvalue_roots(parsed, c, out);
            }
        }
        "conditional_expression" => {
            // `c ?: y` (GNU) yields the condition itself when it is true.
            let fields: &[&str] = if node.child_by_field_name("consequence").is_some() {
                &["consequence", "alternative"]
            } else {
                &["condition", "alternative"]
            };
            for f in fields {
                if let Some(c) = node.child_by_field_name(f) {
                    lvalue_roots(parsed, c, out);
                }
            }
        }
        "comma_expression" => {
            for f in ["left", "right"] {
                if let Some(c) = node.child_by_field_name(f) {
                    lvalue_roots(parsed, c, out);
                }
            }
        }
        "assignment_expression" => {
            if let Some(c) = node.child_by_field_name("left") {
                lvalue_roots(parsed, c, out);
            }
        }
        "update_expression" => {
            if let Some(c) = node.child_by_field_name("argument") {
                lvalue_roots(parsed, c, out);
            }
        }
        "cast_expression" => {
            let to_ref = node
                .child_by_field_name("type")
                .is_some_and(|t| parsed.node_text(&t).contains('&'));
            if to_ref {
                if let Some(c) = node.child_by_field_name("value") {
                    lvalue_roots(parsed, c, out);
                }
            }
        }
        // A designated initializer's value.
        "initializer_pair" => {
            if let Some(c) = node.child_by_field_name("value") {
                lvalue_roots(parsed, c, out);
            }
        }
        _ => {}
    }
}

/// Every identifier an address-taking site (`&E`) or a range-for range can bind a pointer or
/// a reference to: the lvalue roots of `lvalue_roots` (any cast included), plus `.`/`->` and
/// subscript chains down to their root; a shape not listed escapes every identifier in it.
pub(super) fn escape_roots<'a>(node: Node<'a>, out: &mut Vec<Node<'a>>) {
    match node.kind() {
        "identifier" => out.push(node),
        "parenthesized_expression" => {
            if let Some(c) = node.named_child(0) {
                escape_roots(c, out);
            }
        }
        "conditional_expression" => {
            let fields: &[&str] = if node.child_by_field_name("consequence").is_some() {
                &["consequence", "alternative"]
            } else {
                &["condition", "alternative"]
            };
            for f in fields {
                if let Some(c) = node.child_by_field_name(f) {
                    escape_roots(c, out);
                }
            }
        }
        "comma_expression" => {
            for f in ["left", "right"] {
                if let Some(c) = node.child_by_field_name(f) {
                    escape_roots(c, out);
                }
            }
        }
        "assignment_expression" => {
            if let Some(c) = node.child_by_field_name("left") {
                escape_roots(c, out);
            }
        }
        "update_expression" | "field_expression" | "subscript_expression" => {
            if let Some(c) = node.child_by_field_name("argument") {
                escape_roots(c, out);
            }
        }
        "cast_expression" => {
            if let Some(c) = node.child_by_field_name("value") {
                escape_roots(c, out);
            }
        }
        _ => identifiers_in(node, out),
    }
}

fn is_reference_like(declarator: Node<'_>) -> bool {
    let mut node = declarator;
    loop {
        match node.kind() {
            "reference_declarator" | "structured_binding_declarator" => return true,
            "pointer_declarator"
            | "parenthesized_declarator"
            | "array_declarator"
            | "attributed_declarator" => match node
                .child_by_field_name("declarator")
                .or_else(|| node.named_child(0))
            {
                Some(n) => node = n,
                None => return false,
            },
            _ => return false,
        }
    }
}

fn literal_kind(kind: &str) -> bool {
    matches!(
        kind,
        "number_literal"
            | "char_literal"
            | "true"
            | "false"
            | "string_literal"
            | "raw_string_literal"
    )
}

/// E2, C++ half: the type of the declaration passes the scalar gate. `decl` is the
/// `declaration` or `parameter_declaration`, `top` the declarator chain under it (or under its
/// `init_declarator`).
pub(super) fn scalar_gate(
    parsed: &ParsedFile,
    decl: Node<'_>,
    top: Node<'_>,
    pointer: bool,
) -> bool {
    if pointer {
        return true;
    }
    let Some(ty) = decl.child_by_field_name("type") else {
        return false;
    };
    match ty.kind() {
        "primitive_type" | "sized_type_specifier" | "enum_specifier" => true,
        "placeholder_type_specifier" => {
            if parsed.node_text(&ty).trim() != "auto" {
                return false;
            }
            let init = match top.parent() {
                Some(p) if p.kind() == "init_declarator" => p.child_by_field_name("value"),
                Some(p) if p.kind() == "declaration" => p.child_by_field_name("value"),
                _ => None,
            };
            init.is_some_and(|v| literal_kind(v.kind()))
        }
        _ => false,
    }
}

fn identifiers_in<'a>(node: Node<'a>, out: &mut Vec<Node<'a>>) {
    if node.kind() == "identifier" {
        out.push(node);
    }
    let mut cursor = node.walk();
    for c in node.named_children(&mut cursor) {
        identifiers_in(c, out);
    }
}

/// The initializer expression of one declarator of a declaration.
fn initializer_of<'a>(decl: Node<'a>, declarator: Node<'a>) -> Option<Node<'a>> {
    if declarator.kind() == "init_declarator" {
        declarator.child_by_field_name("value")
    } else {
        decl.child_by_field_name("value")
    }
}

/// Record the C++ escapes one node causes (E3). Called for every node of the owner.
pub(super) fn note_node(parsed: &ParsedFile, node: Node<'_>, out: &mut COwner) {
    let mut roots = Vec::new();
    let esc = |out: &mut COwner, kind: &'static str, nodes: &[Node<'_>]| {
        for n in nodes {
            out.escape(kind, parsed.node_text(n));
        }
    };
    match node.kind() {
        // A bare argument binds a reference parameter or a constructor parameter.
        "argument_list" | "initializer_list" => {
            let mut cursor = node.walk();
            for c in node.named_children(&mut cursor) {
                lvalue_roots(parsed, c, &mut roots);
            }
            let kind = match node.parent().map(|p| p.kind()) {
                Some("field_initializer") => "mem_init",
                Some("new_expression") => "new_arg",
                _ if node.kind() == "initializer_list" => "brace_init",
                _ => "call_arg",
            };
            esc(out, kind, &roots);
        }
        // `cin >> x`: operator>> takes its target by reference. `<<` is not an escape (X3).
        "binary_expression" => {
            if node
                .child_by_field_name("operator")
                .is_some_and(|op| parsed.node_text(&op) == ">>")
            {
                for f in ["left", "right"] {
                    if let Some(c) = node.child_by_field_name(f) {
                        lvalue_roots(parsed, c, &mut roots);
                    }
                }
                esc(out, "shift", &roots);
            }
        }
        "declaration" => {
            let mut cursor = node.walk();
            let declarators: Vec<Node<'_>> = node
                .children_by_field_name("declarator", &mut cursor)
                .collect();
            let mut ids = Vec::new();
            for d in &declarators {
                let top = if d.kind() == "init_declarator" {
                    match d.child_by_field_name("declarator") {
                        Some(t) => t,
                        None => continue,
                    }
                } else {
                    *d
                };
                let init = initializer_of(node, *d);
                if is_reference_like(top) {
                    // Anything in the initializer of a reference or structured binding.
                    if let Some(i) = init {
                        identifiers_in(i, &mut ids);
                    }
                    esc(out, "ref_init", &ids);
                    ids.clear();
                    continue;
                }
                // `Foo f(a);` parses as a function declarator whose parameter type is the
                // name `a`: a direct-initialisation argument.
                if top.kind() == "function_declarator" {
                    let mut ids = Vec::new();
                    if let Some(params) = top.child_by_field_name("parameters") {
                        identifiers_in_types(params, &mut ids);
                    }
                    for n in ids {
                        out.escape("direct_init", parsed.node_text(&n));
                    }
                    continue;
                }
                let pointer = {
                    let mut n = top;
                    let mut p = false;
                    loop {
                        match n.kind() {
                            "pointer_declarator" => {
                                p = true;
                                match n.child_by_field_name("declarator") {
                                    Some(x) => n = x,
                                    None => break,
                                }
                            }
                            "parenthesized_declarator" => match n.named_child(0) {
                                Some(x) => n = x,
                                None => break,
                            },
                            _ => break,
                        }
                    }
                    p
                };
                // The bare initializer of a declaration whose type fails the gate may be
                // taken by a converting constructor by reference.
                if !scalar_gate(parsed, node, top, pointer) {
                    if let Some(i) = init {
                        lvalue_roots(parsed, i, &mut roots);
                        esc(out, "ctor_conv", &roots);
                        roots.clear();
                    }
                    out.scalar_fail += 1;
                }
            }
        }
        // The range is bound through an implicit reference, and ADL `begin(E&)` / `end(E&)` can
        // keep it, whatever the loop variable's own declarator.
        "for_range_loop" => {
            if let Some(r) = node.child_by_field_name("right") {
                escape_roots(r, &mut roots);
                esc(out, "range_for", &roots);
            }
        }
        // `&E`: every lvalue root of E (the C rule's `chain_root` sees identifiers, `.`,
        // subscripts and parentheses only).
        "pointer_expression" => {
            let amp = node
                .child_by_field_name("operator")
                .is_some_and(|op| parsed.node_text(&op) == "&");
            if amp {
                if let Some(arg) = node.child_by_field_name("argument") {
                    escape_roots(arg, &mut roots);
                    esc(out, "addr_of", &roots);
                }
            }
        }
        // Captures are initialised when the closure is created, on the creating statement's
        // line, and a by-reference capture or a stored closure keeps the variable: every name
        // in a lambda with any capture escapes. `[]` reads no local.
        "lambda_expression" => {
            let non_empty = node.child_by_field_name("captures").is_some_and(|caps| {
                let mut cursor = caps.walk();
                let any = caps
                    .named_children(&mut cursor)
                    .any(|c| c.kind() != "comment");
                any
            });
            if non_empty {
                identifiers_in(node, &mut roots);
                esc(out, "lambda_capture", &roots);
            }
        }
        // A block-scope using-declaration names a variable of another scope (`using N::x;`
        // hides an enclosing local `x` and writes N::x).
        "using_declaration" if !is_using_directive(node) => {
            collect_names(node, &mut roots);
            esc(out, "using_decl", &roots);
        }
        _ => {}
    }
}

fn is_using_directive(node: Node<'_>) -> bool {
    let mut cursor = node.walk();
    let found = node.children(&mut cursor).any(|c| c.kind() == "namespace");
    found
}

fn collect_names<'a>(node: Node<'a>, out: &mut Vec<Node<'a>>) {
    if matches!(
        node.kind(),
        "identifier" | "namespace_identifier" | "type_identifier" | "field_identifier"
    ) {
        out.push(node);
    }
    let mut cursor = node.walk();
    for c in node.named_children(&mut cursor) {
        collect_names(c, out);
    }
}

/// Why a C++ owner is off, when it is (`None` otherwise): a block-scope using-directive makes
/// any name rebind; a coroutine (outside nested lambdas, which are their own callables) has
/// implicit promise and awaiter calls that can bind parameters and operands by reference.
pub(super) fn off_reason(node: Node<'_>, owner: Node<'_>) -> Option<super::c_prune::Off> {
    use super::c_prune::Off;
    match node.kind() {
        "using_declaration" if is_using_directive(node) => Some(Off::UsingDirective),
        "co_await_expression" | "co_yield_statement" | "co_return_statement" => {
            let mut p = node.parent();
            while let Some(n) = p {
                if n.id() == owner.id() {
                    break;
                }
                if n.kind() == "lambda_expression" {
                    return None;
                }
                p = n.parent();
            }
            Some(Off::Coroutine)
        }
        _ => None,
    }
}

/// Names parsed as types inside a function-declarator parameter list (`Foo f(a);`).
fn identifiers_in_types<'a>(params: Node<'a>, out: &mut Vec<Node<'a>>) {
    let mut cursor = params.walk();
    for p in params.named_children(&mut cursor) {
        if p.kind() == "parameter_declaration" && p.child_by_field_name("declarator").is_none() {
            if let Some(t) = p.child_by_field_name("type") {
                if t.kind() == "type_identifier" {
                    out.push(t);
                }
            }
        }
    }
}

/// Declarations a statement header makes visible (`if (int v = f(); v)`, `while (T* p = ..)`,
/// `switch (int s = ..; s)`): the `condition_clause` of a `condition` field.
pub(super) fn condition_decls<'a>(
    stmt: Node<'a>,
    found: &mut Vec<Node<'a>>,
    decl_idents: &dyn Fn(Node<'a>, &mut Vec<Node<'a>>),
) {
    let Some(cond) = stmt.child_by_field_name("condition") else {
        return;
    };
    if cond.kind() != "condition_clause" {
        return;
    }
    if let Some(init) = cond.child_by_field_name("initializer") {
        let mut cursor = init.walk();
        for d in init.named_children(&mut cursor) {
            if d.kind() == "declaration" {
                decl_idents(d, found);
            }
        }
    }
    if let Some(v) = cond.child_by_field_name("value") {
        if v.kind() == "declaration" {
            decl_idents(v, found);
        }
    }
}
