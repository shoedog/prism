use super::*;
use Language::*;
use Reason::*;
use Role::*;

mod refuses;

type Syntax<'a> = (&'a str, &'a str);
type Row<'a> = (
    Language,
    &'a str,
    Syntax<'a>,
    Syntax<'a>,
    Syntax<'a>,
    Role<'a>,
);

fn node<'tree>(
    parsed: &'tree ParsedFile,
    root: Node<'tree>,
    (kind, source): Syntax<'_>,
) -> Node<'tree> {
    let mut cursor = root.walk();
    loop {
        let node = cursor.node();
        if node.kind() == kind && parsed.node_text(&node) == source {
            return node;
        }
        if cursor.goto_first_child() {
            continue;
        }
        while !cursor.goto_next_sibling() {
            assert!(
                cursor.goto_parent(),
                "missing {kind} {source:?} below {}",
                root.to_sexp()
            );
        }
    }
}

fn check(
    (language, source, operation, target, operand, role): Row<'_>,
    expected: Result<(), (Table, Reason)>,
) {
    let parsed = ParsedFile::parse("certificate", source, language).unwrap();
    let root = parsed.tree.root_node();
    if expected != Err((Table::Operation, ParseRecovery)) {
        assert!(
            !root.has_error(),
            "{language:?} {source:?} {}",
            root.to_sexp()
        );
    }
    let op = node(&parsed, root, operation);
    let target = node(&parsed, op, target);
    let operand = node(&parsed, op, operand);
    assert_eq!(
        certify(&parsed, op, target, operand, role),
        expected,
        "{language:?} {source:?}, {}",
        op.to_sexp()
    );
}

fn check_all(rows: impl IntoIterator<Item = Row<'static>>, expected: Result<(), (Table, Reason)>) {
    let failures = rows
        .into_iter()
        .map(|row| std::panic::catch_unwind(|| check(row, expected)))
        .filter(Result::is_err)
        .count();
    assert_eq!(failures, 0, "certificate rows failed");
}

#[test]
fn call_and_constructor_forms() {
    let rows: &[Row<'_>] = &[
        (
            Python,
            "obj.eval(x)",
            ("call", "obj.eval(x)"),
            ("identifier", "eval"),
            ("identifier", "x"),
            Argument(0),
        ),
        (
            Go,
            "package a\nfunc g() { pkg.Eval(x) }",
            ("call_expression", "pkg.Eval(x)"),
            ("field_identifier", "Eval"),
            ("identifier", "x"),
            Argument(0),
        ),
        (
            Java,
            "class A { void m() { obj.eval(x); } }",
            ("method_invocation", "obj.eval(x)"),
            ("identifier", "eval"),
            ("identifier", "x"),
            Argument(0),
        ),
        (
            Java,
            "class A { Object g() { return obj.<T>eval(x); } }",
            ("method_invocation", "obj.<T>eval(x)"),
            ("identifier", "eval"),
            ("identifier", "x"),
            Argument(0),
        ),
        (
            Java,
            "class A { Object o = new a.b.Eval(x); }",
            ("object_creation_expression", "new a.b.Eval(x)"),
            ("type_identifier", "Eval"),
            ("identifier", "x"),
            Argument(0),
        ),
        (
            C,
            "void g() { obj->eval(x); }",
            ("call_expression", "obj->eval(x)"),
            ("field_identifier", "eval"),
            ("identifier", "x"),
            Argument(0),
        ),
        (
            Cpp,
            "void g() { a::b::eval(x); }",
            ("call_expression", "a::b::eval(x)"),
            ("identifier", "eval"),
            ("identifier", "x"),
            Argument(0),
        ),
        (
            Cpp,
            "void g() { new a::b::Eval(x); }",
            ("new_expression", "new a::b::Eval(x)"),
            ("type_identifier", "Eval"),
            ("identifier", "x"),
            Argument(0),
        ),
        (
            Rust,
            "fn g() { crate::m::eval(x); }",
            ("call_expression", "crate::m::eval(x)"),
            ("identifier", "eval"),
            ("identifier", "x"),
            Argument(0),
        ),
        (
            Lua,
            "obj:eval(x)",
            ("function_call", "obj:eval(x)"),
            ("identifier", "eval"),
            ("identifier", "x"),
            Argument(0),
        ),
        (
            Bash,
            "MODE=1 eval \"$x\" > out",
            ("command", "MODE=1 eval \"$x\""),
            ("word", "eval"),
            ("string", "\"$x\""),
            Argument(0),
        ),
        (
            Terraform,
            "a = render(var.x)",
            ("function_call", "render(var.x)"),
            ("identifier", "render"),
            ("expression", "var.x"),
            Argument(0),
        ),
    ];
    check_all(rows.iter().copied(), Ok(()));
    for language in [JavaScript, TypeScript, Tsx] {
        check(
            (
                language,
                "eval?.(x)",
                ("call_expression", "eval?.(x)"),
                ("identifier", "eval"),
                ("identifier", "x"),
                Argument(0),
            ),
            Ok(()),
        );
        check(
            (
                language,
                "obj?.eval(x)",
                ("call_expression", "obj?.eval(x)"),
                ("property_identifier", "eval"),
                ("identifier", "x"),
                Argument(0),
            ),
            Ok(()),
        );
        check(
            (
                language,
                "new Eval(x)",
                ("new_expression", "new Eval(x)"),
                ("identifier", "Eval"),
                ("identifier", "x"),
                Argument(0),
            ),
            Ok(()),
        );
    }
}

#[test]
fn assignment_and_attribute_forms() {
    let rows: &[Row<'_>] = &[
        (
            Python,
            "obj.eval = x",
            ("assignment", "obj.eval = x"),
            ("identifier", "eval"),
            ("identifier", "x"),
            AssignmentValue,
        ),
        (
            Go,
            "package a\nfunc g() { obj.eval = x }",
            ("assignment_statement", "obj.eval = x"),
            ("field_identifier", "eval"),
            ("identifier", "x"),
            AssignmentValue,
        ),
        (
            Go,
            "package a\nfunc g() { eval := x }",
            ("short_var_declaration", "eval := x"),
            ("identifier", "eval"),
            ("identifier", "x"),
            AssignmentValue,
        ),
        (
            Java,
            "class A { void m() { obj.eval = x; } }",
            ("assignment_expression", "obj.eval = x"),
            ("identifier", "eval"),
            ("identifier", "x"),
            AssignmentValue,
        ),
        (
            C,
            "void g() { obj.eval = x; }",
            ("assignment_expression", "obj.eval = x"),
            ("field_identifier", "eval"),
            ("identifier", "x"),
            AssignmentValue,
        ),
        (
            Cpp,
            "void g() { obj.eval = x; }",
            ("assignment_expression", "obj.eval = x"),
            ("field_identifier", "eval"),
            ("identifier", "x"),
            AssignmentValue,
        ),
        (
            Rust,
            "fn g() { obj.eval = x; }",
            ("assignment_expression", "obj.eval = x"),
            ("field_identifier", "eval"),
            ("identifier", "x"),
            AssignmentValue,
        ),
        (
            Lua,
            "obj.eval = x",
            ("assignment_statement", "obj.eval = x"),
            ("identifier", "eval"),
            ("identifier", "x"),
            AssignmentValue,
        ),
        (
            Bash,
            "eval='x'",
            ("variable_assignment", "eval='x'"),
            ("variable_name", "eval"),
            ("raw_string", "'x'"),
            AssignmentValue,
        ),
        (
            Terraform,
            "eval = \"x\"",
            ("attribute", "eval = \"x\""),
            ("identifier", "eval"),
            ("expression", "\"x\""),
            AssignmentValue,
        ),
    ];
    check_all(rows.iter().copied(), Ok(()));
    for language in [JavaScript, TypeScript, Tsx] {
        check(
            (
                language,
                "obj.eval = x",
                ("assignment_expression", "obj.eval = x"),
                ("property_identifier", "eval"),
                ("identifier", "x"),
                AssignmentValue,
            ),
            Ok(()),
        );
    }
    for language in [JavaScript, Tsx] {
        let prop = "dangerouslySetInnerHTML={{__html:x}}";
        check(
            (
                language,
                "<X {...props} dangerouslySetInnerHTML={{__html:x}} />",
                ("jsx_attribute", prop),
                ("property_identifier", "dangerouslySetInnerHTML"),
                ("identifier", "x"),
                AttributeValue,
            ),
            Ok(()),
        );
        check(
            (
                language,
                "<X eval=\"x\"></X>",
                ("jsx_attribute", "eval=\"x\""),
                ("property_identifier", "eval"),
                ("string", "\"x\""),
                AttributeValue,
            ),
            Ok(()),
        );
    }
}

#[test]
fn receivers_and_positional_exceptions() {
    check_all(
        [
            (
                Python,
                "(obj.eval)(x)",
                ("call", "(obj.eval)(x)"),
                ("attribute", "obj.eval"),
                ("identifier", "obj"),
                ReceiverValue,
            ),
            (
                Python,
                "f(k=x, *[y])",
                ("call", "f(k=x, *[y])"),
                ("identifier", "f"),
                ("identifier", "x"),
                KeywordArgument("k"),
            ),
            (
                Python,
                "f(K=x)",
                ("call", "f(K=x)"),
                ("identifier", "f"),
                ("identifier", "x"),
                KeywordArgument("K"),
            ),
            (
                Python,
                "f(x, # note\n  y)",
                ("call", "f(x, # note\n  y)"),
                ("identifier", "f"),
                ("identifier", "y"),
                Argument(1),
            ),
            (
                Python,
                "obj[r\"eval\"](x)",
                ("call", "obj[r\"eval\"](x)"),
                ("string", "r\"eval\""),
                ("identifier", "x"),
                Argument(0),
            ),
            (
                JavaScript,
                "obj[`eval`](x)",
                ("call_expression", "obj[`eval`](x)"),
                ("template_string", "`eval`"),
                ("identifier", "x"),
                Argument(0),
            ),
            (
                TypeScript,
                "((obj.eval)!)(x)",
                ("call_expression", "((obj.eval)!)(x)"),
                ("member_expression", "obj.eval"),
                ("identifier", "obj"),
                ReceiverValue,
            ),
            (
                Go,
                "package a\nfunc f() { (obj.Eval)(x) }",
                ("call_expression", "(obj.Eval)(x)"),
                ("selector_expression", "obj.Eval"),
                ("identifier", "obj"),
                ReceiverValue,
            ),
            (
                Java,
                "class A { void m() { obj.eval(x); } }",
                ("method_invocation", "obj.eval(x)"),
                ("identifier", "eval"),
                ("identifier", "obj"),
                ReceiverValue,
            ),
            (
                C,
                "void g() { (/* ok */ obj.eval)(x); }",
                ("call_expression", "(/* ok */ obj.eval)(x)"),
                ("field_identifier", "eval"),
                ("identifier", "obj"),
                ReceiverValue,
            ),
            (
                Rust,
                "fn g() { obj.eval(x); }",
                ("call_expression", "obj.eval(x)"),
                ("field_expression", "obj.eval"),
                ("identifier", "obj"),
                ReceiverValue,
            ),
            (
                Lua,
                "obj:eval(x)",
                ("function_call", "obj:eval(x)"),
                ("method_index_expression", "obj:eval"),
                ("identifier", "obj"),
                ReceiverValue,
            ),
            (
                Bash,
                "eval plain-word",
                ("command", "eval plain-word"),
                ("word", "eval"),
                ("word", "plain-word"),
                Argument(0),
            ),
            (
                Bash,
                "eval \"plain word\"",
                ("command", "eval \"plain word\""),
                ("word", "eval"),
                ("string", "\"plain word\""),
                Argument(0),
            ),
            (
                Terraform,
                "a = render(42)",
                ("function_call", "render(42)"),
                ("identifier", "render"),
                ("expression", "42"),
                Argument(0),
            ),
        ],
        Ok(()),
    );
}

#[test]
fn error_outside_operation_does_not_refuse() {
    let parsed = ParsedFile::parse("certificate", "eval(x); const = ;", JavaScript).unwrap();
    let root = parsed.tree.root_node();
    assert!(root.has_error());
    let op = node(&parsed, root, ("call_expression", "eval(x)"));
    let callee = node(&parsed, op, ("identifier", "eval"));
    let operand = node(&parsed, op, ("identifier", "x"));
    assert_eq!(certify(&parsed, op, callee, operand, Argument(0)), Ok(()));
}
