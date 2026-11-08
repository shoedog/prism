//! Lexically correct sole-binding lookup for module identity (#637 A1).
//!
//! `js_scope::resolve` models only parameters, blocks and the program, and is shared with the S1
//! rules, so it is left alone. Module identity asks a stricter question: is the innermost scope that
//! binds NAME holding exactly one binding of it, and is that binding never written? Scopes are
//! functions (parameters, patterns with defaults, `var` and function declarations hoisted out of
//! nested blocks), blocks, `switch` bodies, `catch` clauses, `for` heads and the program. A
//! binding in an unrelated sibling scope does not count; a binding anywhere on the path does.

use super::callee::Src;
use super::js_scope::{in_declarator, in_import, is_function_kind, Binding};
use tree_sitter::Node;

pub(super) enum Lookup<'t> {
    /// No scope on the path binds the name (a global).
    Unbound,
    /// The innermost binding scope holds exactly one binding.
    Sole {
        binding: Binding<'t>,
        scope: Node<'t>,
    },
    /// The innermost binding scope holds several bindings of the name.
    Ambiguous,
}

pub(super) fn lookup<'t>(p: Src<'_>, from: Node<'t>, name: &str) -> Lookup<'t> {
    let mut cur = from.parent();
    while let Some(scope) = cur {
        // Inside `with (o) { .. }` any name may resolve to a property of `o`: nothing is provable.
        if scope.kind() == "with_statement"
            && scope.child_by_field_name("body").is_some_and(|b| {
                b.start_byte() <= from.start_byte() && from.end_byte() <= b.end_byte()
            })
        {
            return Lookup::Ambiguous;
        }
        let mut found = Vec::new();
        collect(p, scope, name, &mut found);
        match found.len() {
            0 => {}
            1 => {
                return Lookup::Sole {
                    binding: found.pop().unwrap_or(Binding::Other),
                    scope,
                }
            }
            _ => return Lookup::Ambiguous,
        }
        cur = scope.parent();
    }
    Lookup::Unbound
}

/// The sole binding of `name` as seen from `from`, when nothing writes to it.
pub(super) fn sole<'t>(p: Src<'_>, from: Node<'t>, name: &str) -> Option<Binding<'t>> {
    match lookup(p, from, name) {
        Lookup::Sole { binding, scope } => match binding {
            Binding::Import { .. } => Some(binding),
            Binding::Var { .. } | Binding::Destructured { .. } if !is_written(p, scope, name) => {
                Some(binding)
            }
            _ => None,
        },
        _ => None,
    }
}

fn is_func(kind: &str) -> bool {
    is_function_kind(kind)
        || matches!(
            kind,
            "function_declaration"
                | "generator_function_declaration"
                | "method_definition"
                | "arrow_function"
        )
}

/// True when binding pattern (or assignment target) `n` binds/targets `name`.
fn pattern_binds(p: Src<'_>, n: Node<'_>, name: &str) -> bool {
    match n.kind() {
        "identifier"
        | "shorthand_property_identifier_pattern"
        | "shorthand_property_identifier" => p.node_text(&n) == name,
        "pair_pattern" => n
            .child_by_field_name("value")
            .is_some_and(|v| pattern_binds(p, v, name)),
        "assignment_pattern" | "object_assignment_pattern" => n
            .child_by_field_name("left")
            .is_some_and(|l| pattern_binds(p, l, name)),
        "required_parameter" | "optional_parameter" => n
            .child_by_field_name("pattern")
            .is_some_and(|l| pattern_binds(p, l, name)),
        "rest_pattern" | "object_pattern" | "array_pattern" | "parenthesized_expression" => {
            let mut c = n.walk();
            let hit = n
                .named_children(&mut c)
                .any(|ch| pattern_binds(p, ch, name));
            hit
        }
        _ => false,
    }
}

fn params_bind(p: Src<'_>, func: Node<'_>, name: &str) -> bool {
    if func
        .child_by_field_name("parameter")
        .is_some_and(|s| pattern_binds(p, s, name))
    {
        return true;
    }
    let Some(params) = func.child_by_field_name("parameters") else {
        return false;
    };
    let mut c = params.walk();
    let hit = params
        .named_children(&mut c)
        .any(|ch| pattern_binds(p, ch, name));
    hit
}

fn declarators<'t>(p: Src<'_>, decl: Node<'t>, name: &str, out: &mut Vec<Binding<'t>>) {
    let mut c = decl.walk();
    for d in decl.named_children(&mut c) {
        if d.kind() == "variable_declarator"
            && d.child_by_field_name("name")
                .is_some_and(|t| pattern_binds(p, t, name))
        {
            out.push(in_declarator(p, d, name).unwrap_or(Binding::Other));
        }
    }
}

/// `let`/`const`/class declarations and imports directly in a block, switch case or program.
fn lexical<'t>(p: Src<'_>, scope: Node<'t>, name: &str, out: &mut Vec<Binding<'t>>) {
    let mut c = scope.walk();
    for stmt in scope.named_children(&mut c) {
        let stmt = if stmt.kind() == "export_statement" {
            stmt.child_by_field_name("declaration").unwrap_or(stmt)
        } else {
            stmt
        };
        match stmt.kind() {
            "lexical_declaration" => declarators(p, stmt, name, out),
            "class_declaration" | "abstract_class_declaration" | "enum_declaration" => {
                if stmt
                    .child_by_field_name("name")
                    .is_some_and(|n| p.node_text(&n) == name)
                {
                    out.push(Binding::Other);
                }
            }
            "import_statement" if scope.kind() == "program" => {
                if let Some(b) = in_import(p, stmt, name) {
                    out.push(b);
                }
            }
            _ => {}
        }
    }
}

/// `var` and function declarations anywhere below `node`, not inside a nested function.
fn hoisted<'t>(p: Src<'_>, node: Node<'t>, name: &str, out: &mut Vec<Binding<'t>>) {
    let mut c = node.walk();
    for ch in node.named_children(&mut c) {
        match ch.kind() {
            "function_declaration" | "generator_function_declaration" => {
                if ch
                    .child_by_field_name("name")
                    .is_some_and(|n| p.node_text(&n) == name)
                {
                    out.push(Binding::Function);
                }
            }
            "variable_declaration" => declarators(p, ch, name, out),
            "for_in_statement" => {
                let var = ch
                    .child_by_field_name("kind")
                    .is_some_and(|k| p.node_text(&k) == "var");
                if var
                    && ch
                        .child_by_field_name("left")
                        .is_some_and(|l| pattern_binds(p, l, name))
                {
                    out.push(Binding::Other);
                }
                hoisted(p, ch, name, out);
            }
            k if is_func(k) => {}
            _ => hoisted(p, ch, name, out),
        }
    }
}

fn collect<'t>(p: Src<'_>, scope: Node<'t>, name: &str, out: &mut Vec<Binding<'t>>) {
    let kind = scope.kind();
    if is_func(kind) {
        if params_bind(p, scope, name) {
            out.push(Binding::Param { annotation: None });
        }
        let own_name = matches!(
            kind,
            "function_expression" | "function" | "generator_function"
        ) && scope
            .child_by_field_name("name")
            .is_some_and(|n| p.node_text(&n) == name);
        if own_name {
            out.push(Binding::Function);
        }
        if let Some(body) = scope.child_by_field_name("body") {
            if body.kind() == "statement_block" {
                hoisted(p, body, name, out);
            }
        }
        return;
    }
    match kind {
        "program" => {
            lexical(p, scope, name, out);
            hoisted(p, scope, name, out);
        }
        "statement_block" => lexical(p, scope, name, out),
        // A class expression's own name is bound inside its body.
        "class" => {
            if scope
                .child_by_field_name("name")
                .is_some_and(|n| p.node_text(&n) == name)
            {
                out.push(Binding::Other);
            }
        }
        "switch_body" => {
            let mut c = scope.walk();
            for case in scope.named_children(&mut c) {
                lexical(p, case, name, out);
            }
        }
        "catch_clause" => {
            if scope
                .child_by_field_name("parameter")
                .is_some_and(|q| pattern_binds(p, q, name))
            {
                out.push(Binding::Other);
            }
        }
        "for_statement" => {
            if let Some(init) = scope.child_by_field_name("initializer") {
                if init.kind() == "lexical_declaration" {
                    declarators(p, init, name, out);
                }
            }
        }
        "for_in_statement" => {
            let lex = scope
                .child_by_field_name("kind")
                .is_some_and(|k| matches!(p.node_text(&k), "let" | "const"));
            if lex
                && scope
                    .child_by_field_name("left")
                    .is_some_and(|l| pattern_binds(p, l, name))
            {
                out.push(Binding::Other);
            }
        }
        _ => {}
    }
}

/// True when a write to `name` below `scope` resolves to the binding that `scope` holds:
/// assignment, compound assignment, `++`/`--`, destructuring-assignment target, `for (x of ..)`.
fn is_written(p: Src<'_>, scope: Node<'_>, name: &str) -> bool {
    let mut stack = vec![scope];
    while let Some(n) = stack.pop() {
        let target = match n.kind() {
            "assignment_expression" | "augmented_assignment_expression" => {
                n.child_by_field_name("left")
            }
            "update_expression" => n.child_by_field_name("argument"),
            "for_in_statement" if n.child_by_field_name("kind").is_none() => {
                n.child_by_field_name("left")
            }
            _ => None,
        };
        if let Some(t) = target {
            if pattern_binds(p, t, name) {
                let ours = match lookup(p, t, name) {
                    Lookup::Sole { scope: s, .. } => s.id() == scope.id(),
                    Lookup::Ambiguous => true,
                    Lookup::Unbound => false,
                };
                if ours {
                    return true;
                }
            }
        }
        let mut c = n.walk();
        stack.extend(n.children(&mut c));
    }
    false
}

/// An import that only brings a type: `import type ..` or `import { type x }`.
pub(super) fn is_type_only(module: Node<'_>, imported: Option<Node<'_>>) -> bool {
    let has_type = |n: Node<'_>| {
        let mut c = n.walk();
        let hit = n.children(&mut c).any(|ch| ch.kind() == "type");
        hit
    };
    if imported
        .and_then(|i| i.parent())
        .is_some_and(|s| s.kind() == "import_specifier" && has_type(s))
    {
        return true;
    }
    let mut cur = module.parent();
    while let Some(n) = cur {
        if n.kind() == "import_statement" {
            return has_type(n);
        }
        cur = n.parent();
    }
    false
}
