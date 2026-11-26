//! The accept-list of callee shapes (S2-W fold 6): a callee shape that is not listed earns nothing.
use super::*;

type Row = (&'static str, &'static str, bool);

fn wrong_rows(language: Language, file: &str, source: &str, rows: &[Row]) -> Vec<String> {
    let parsed = ParsedFile::parse(file, source, language).unwrap();
    assert!(!parsed.tree.root_node().has_error(), "{file}");
    let def = node(&parsed, parsed.tree.root_node(), "x");
    let mut wrong = Vec::new();
    for &(operation, target, passes) in rows {
        let operation = node(&parsed, parsed.tree.root_node(), operation);
        let operand = node(&parsed, operation, "x");
        let bound = Bound::from_binder(
            operation,
            node(&parsed, operation, target),
            operand,
            Role::Argument(0),
            FLAT,
        );
        let line = operation.start_position().row + 1;
        let path = Box::leak(Box::new(path_to(&parsed, def, operand)));
        let witness = receipt(&parsed, line, bound, operand, FLAT, destination(path));
        let earned = collect_rules(
            &parsed,
            line,
            &admitted(&parsed, &[line]),
            [witness],
            select_eval,
        )
        .len();
        if earned != usize::from(passes) {
            wrong.push(format!(
                "{file}: {target} in {} want {passes}",
                parsed.node_text(&operation)
            ));
        }
    }
    wrong
}

#[test]
fn python_callee_shapes() {
    let source = "def f(x):\n    (lambda: f)(x)\n    (k if c else f)(x)\n    (g, f)(x)\n    f()(x)\n    (f)(x)\n    a.f(x)\n    a[\"f\"](x)\n    a[k](x)\n    (a.f)(x)\n";
    let rows: &[Row] = &[
        // control of the owner check (evidence.rs), not a table refusal
        ("(lambda: f)(x)", "f", false),
        ("(k if c else f)(x)", "f", false),
        ("(g, f)(x)", "f", false),
        ("f()(x)", "f", false),
        ("(f)(x)", "f", true),
        ("a.f(x)", "f", true),
        ("a[\"f\"](x)", "\"f\"", true),
        ("a[k](x)", "k", false),
        ("(a.f)(x)", "f", true),
    ];
    assert_eq!(
        wrong_rows(Language::Python, "a.py", source, rows),
        Vec::<String>::new()
    );
}

#[test]
fn javascript_callee_shapes() {
    let source = "async function f(x) {\n  (() => f)(x);\n  (function () { return f; })(x);\n  (c ? g : f)(x);\n  (g, f)(x);\n  f()(x);\n  (await f)(x);\n  (f)(x);\n  a.f(x);\n  a?.f(x);\n  a[\"f\"](x);\n  a[k](x);\n  new a.f(x);\n}\n";
    let rows: &[Row] = &[
        // control of the owner check (evidence.rs), not a table refusal
        ("(() => f)(x)", "f", false),
        ("(function () { return f; })(x)", "f", false),
        ("(c ? g : f)(x)", "f", false),
        ("(g, f)(x)", "f", false),
        ("f()(x)", "f", false),
        ("(await f)(x)", "f", false),
        ("(f)(x)", "f", true),
        ("a.f(x)", "f", true),
        // Optional chain: the property is still the callee's own last name, so it earns (pinned).
        ("a?.f(x)", "f", true),
        ("a[\"f\"](x)", "\"f\"", true),
        ("a[k](x)", "k", false),
        ("new a.f(x)", "f", true),
    ];
    assert_eq!(
        wrong_rows(Language::JavaScript, "a.js", source, rows),
        Vec::<String>::new()
    );
}

#[test]
fn typescript_callee_shapes() {
    let source =
        "function f(x) {\n  (f!)(x);\n  a!.f(x);\n  (f as G)(x);\n  (<G>f)(x);\n  f!(x);\n}\n";
    let rows: &[Row] = &[
        ("(f!)(x)", "f", true),
        ("a!.f(x)", "f", true),
        ("f!(x)", "f", true),
        // Over-refusal (pinned): a type assertion is not on the list, so it earns nothing.
        ("(f as G)(x)", "f", false),
        ("(<G>f)(x)", "f", false),
    ];
    assert_eq!(
        wrong_rows(Language::TypeScript, "a.ts", source, rows),
        Vec::<String>::new()
    );
}

#[test]
fn go_callee_shapes() {
    let source = "package p\nfunc f(x int) {\n\tfunc() T { return f }(x)\n\tf()(x)\n\t(f)(x)\n\ta.f(x)\n\t(*p).f(x)\n\tF[int](x)\n}\n";
    let rows: &[Row] = &[
        // control of the owner check (evidence.rs), not a table refusal
        ("func() T { return f }(x)", "f", false),
        ("f()(x)", "f", false),
        ("(f)(x)", "f", true),
        ("a.f(x)", "f", true),
        ("(*p).f(x)", "f", true),
        // Over-refusal (pinned): an instantiated generic function is an index expression.
        ("F[int](x)", "F", false),
    ];
    assert_eq!(
        wrong_rows(Language::Go, "a.go", source, rows),
        Vec::<String>::new()
    );
}

#[test]
fn java_callee_shapes() {
    let source = "class A {\n  void f(int x) {\n    new Foo(x);\n    new Foo<T>(x);\n    a.f(x);\n    a.g().f(x);\n  }\n}\n";
    let rows: &[Row] = &[
        ("new Foo(x)", "Foo", true),
        // Over-refusal (pinned): a parameterised constructor type is a generic_type node.
        ("new Foo<T>(x)", "Foo", false),
        ("a.f(x)", "f", true),
        ("a.g().f(x)", "g", false),
        ("a.g().f(x)", "f", true),
    ];
    assert_eq!(
        wrong_rows(Language::Java, "A.java", source, rows),
        Vec::<String>::new()
    );
}
