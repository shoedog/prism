//! Explicit test-adapter decisions for the collector's prior regression inputs.

use super::*;

fn visits<'a>(
    parsed: &'a ParsedFile,
    path: &FlowPath,
    proved: &[(Site<'a>, Node<'a>)],
) -> Vec<(&'a str, Role<'a>)> {
    let line = path.edges[0].to.line;
    let witnesses = proved
        .iter()
        .map(|&(site, reached)| receipt(parsed, line, site, reached, FLAT, destination(path)));
    let mut seen = Vec::new();
    collect_rules(
        parsed,
        line,
        &admitted(parsed, &[line]),
        witnesses,
        |witness, _| {
            let site = witness.site();
            seen.push((parsed.node_text(&site.target), site.role));
        },
    );
    seen
}

fn call<'a>(
    parsed: &'a ParsedFile,
    text: &str,
    target: &str,
    operand: &str,
    slot: usize,
) -> Site<'a> {
    site(parsed, text, target, operand, Role::Argument(slot))
}

fn first<'a>(parsed: &'a ParsedFile, text: &str) -> Node<'a> {
    node(parsed, parsed.tree.root_node(), text)
}

#[test]
fn receipt_accepts_ri02_command_after_dom_assignment() {
    let parsed = ParsedFile::parse(
        "a.js",
        "function f(x) {\n  el.innerHTML = x; cp.exec(x);\n}",
        Language::JavaScript,
    )
    .unwrap();
    let dom = site(
        &parsed,
        "el.innerHTML = x",
        "innerHTML",
        "x",
        Role::AssignmentValue,
    );
    let exec = call(&parsed, "cp.exec(x)", "exec", "x", 0);
    for representative in [dom.operand, exec.operand] {
        let path = path_to(&parsed, first(&parsed, "x"), representative);
        assert!(visits(&parsed, &path, &[]).is_empty());
        assert_eq!(
            visits(&parsed, &path, &[(dom, dom.operand), (exec, exec.operand)]),
            [
                ("innerHTML", Role::AssignmentValue),
                ("exec", Role::Argument(0))
            ]
        );
    }
}

#[test]
fn binding_and_write_cases_consume_only_supplied_reads() {
    let cases = [
        ("function f(x) {\n  { const x = 'safe'; res.send(x); } observe(x);\n  confirm(x);\n}".into(), "x"),
        ("function f() {\n  let x = source();\n  const reset = () => { x = 'safe'; };\n  observe(x); reset(); res.send(x);\n}".into(), "x"),
    ].into_iter().chain([
        "({x} = o);",
        "({x = d} = o);",
        "var {x} = o;",
        "for ({x} of xs);",
        "for (var {x} of xs);",
    ].map(|write| (format!("function f(x) {{\n  observe(x); {write} res.send(x);\n}}"), "x")));
    for (source, def) in cases {
        let parsed = ParsedFile::parse("a.js", &source, Language::JavaScript).unwrap();
        assert!(!parsed.tree.root_node().has_error(), "{source}");
        let observe = call(&parsed, "observe(x)", "observe", "x", 0);
        let send = call(&parsed, "res.send(x)", "send", "x", 0);
        for representative in [observe.operand, send.operand] {
            let path = path_to(&parsed, first(&parsed, def), representative);
            assert!(visits(&parsed, &path, &[]).is_empty());
            assert_eq!(
                visits(&parsed, &path, &[(observe, observe.operand)]),
                [("observe", Role::Argument(0))]
            );
        }
    }

    let parsed = ParsedFile::parse(
        "a.js",
        "function f(x, y) { one(x); two(x, x); { let x = 'clean'; shadow(x); } three(y); y = 'clean'; rebound(y); }",
        Language::JavaScript,
    ).unwrap();
    let one = call(&parsed, "one(x)", "one", "x", 0);
    let two = call(&parsed, "two(x, x)", "two", "x", 0);
    let three = call(&parsed, "three(y)", "three", "y", 0);
    let y = path_to(&parsed, first(&parsed, "y"), three.operand);
    assert_eq!(
        visits(&parsed, &y, &[(three, three.operand)]),
        [("three", Role::Argument(0))]
    );
    let two_second = Site {
        operand: prism::ast::tree_helpers::call_arg_node(&two.operation, 1).unwrap(),
        role: Role::Argument(1),
        ..two
    };
    let path = path_to(&parsed, first(&parsed, "x"), one.operand);
    assert_eq!(
        visits(
            &parsed,
            &path,
            &[
                (one, one.operand),
                (two, two.operand),
                (two_second, two_second.operand)
            ]
        ),
        [
            ("one", Role::Argument(0)),
            ("two", Role::Argument(0)),
            ("two", Role::Argument(1))
        ]
    );
}

#[test]
fn explicit_nested_and_assignment_wrapped_reads_keep_their_operations() {
    let parsed = ParsedFile::parse(
        "a.js",
        "function f() {\n  const x = source();\n  observe(x); inspect('x'); /* x */ res.send(eval(x));\n}",
        Language::JavaScript,
    ).unwrap();
    let observe = call(&parsed, "observe(x)", "observe", "x", 0);
    let eval = call(&parsed, "eval(x)", "eval", "x", 0);
    let send = call(&parsed, "res.send(eval(x))", "send", "eval(x)", 0);
    let path = path_to(&parsed, first(&parsed, "x"), observe.operand);
    assert_eq!(
        visits(
            &parsed,
            &path,
            &[
                (observe, observe.operand),
                (eval, eval.operand),
                (send, eval.operand)
            ]
        ),
        [
            ("observe", Role::Argument(0)),
            ("eval", Role::Argument(0)),
            ("send", Role::Argument(0))
        ]
    );

    let parsed = ParsedFile::parse(
        "a.go",
        r#"package p
func f(name string) {
    cleaned := filepath.Clean(name)
    if !strings.HasPrefix(cleaned, "/safe/") {
        return
    }
    _ = os.WriteFile(cleaned, []byte("data"), 0644); _ = fmt.Sprintf(cleaned)
}
"#,
        Language::Go,
    )
    .unwrap();
    let write = call(
        &parsed,
        "os.WriteFile(cleaned, []byte(\"data\"), 0644)",
        "WriteFile",
        "cleaned",
        0,
    );
    let format = call(&parsed, "fmt.Sprintf(cleaned)", "Sprintf", "cleaned", 0);
    let path = path_to(&parsed, first(&parsed, "cleaned"), write.operand);
    assert_eq!(
        visits(
            &parsed,
            &path,
            &[(write, write.operand), (format, format.operand)]
        ),
        [
            ("WriteFile", Role::Argument(0)),
            ("Sprintf", Role::Argument(0))
        ]
    );
}

#[test]
fn supplied_field_and_argument_spans_preserve_adapter_roles() {
    for language in [Language::TypeScript, Language::Tsx] {
        for expr in [
            "runtime.X",
            "(runtime as any).X",
            "(runtime satisfies any).X",
            "((runtime as any) satisfies any).X",
            "(/* before */ runtime /* value */ as any).X",
        ] {
            let operation = format!("fetch({expr})");
            let source = format!("function f(runtime: any) {{\n  {operation};\n}}");
            let parsed = ParsedFile::parse("a.ts", &source, language).unwrap();
            assert!(!parsed.tree.root_node().has_error(), "{language:?}: {expr}");
            let fetch = call(&parsed, &operation, "fetch", expr, 0);
            let mut path = path_to(&parsed, first(&parsed, "runtime"), fetch.operand);
            path.edges[0].to.path = AccessPath::from_expr("runtime.X");
            assert_eq!(
                visits(&parsed, &path, &[(fetch, fetch.operand)]),
                [("fetch", Role::Argument(0))]
            );
        }
    }

    let first_call = "el.insertAdjacentHTML(/* position */ x, 'safe')";
    let second_call = "el.insertAdjacentHTML('beforeend', /* html */ x)";
    let source = format!("function f(x) {{\n  {first_call}; {second_call};\n}}");
    let parsed = ParsedFile::parse("a.js", &source, Language::JavaScript).unwrap();
    let position = call(&parsed, first_call, "insertAdjacentHTML", "x", 0);
    let html = call(&parsed, second_call, "insertAdjacentHTML", "x", 1);
    let path = path_to(&parsed, first(&parsed, "x"), position.operand);
    assert_eq!(
        visits(
            &parsed,
            &path,
            &[(position, position.operand), (html, html.operand)]
        ),
        [
            ("insertAdjacentHTML", Role::Argument(0)),
            ("insertAdjacentHTML", Role::Argument(1))
        ]
    );
}

#[test]
fn test_adapter_can_supply_or_withhold_targets_and_operands() {
    for (language, source, operation, target, operand, read, slot) in [
        (
            Language::Java,
            "class A { void f(String x) { new p.Builder(\"ok\", /* value */ x); } }",
            "new p.Builder(\"ok\", /* value */ x)",
            "Builder",
            "x",
            "x",
            1,
        ),
        (
            Language::Cpp,
            "void f(int x) { new ns::Builder(0, /* value */ x); }",
            "new ns::Builder(0, /* value */ x)",
            "Builder",
            "x",
            "x",
            1,
        ),
        (
            Language::Bash,
            "audit safe \"$x\"",
            "audit safe \"$x\"",
            "audit",
            "\"$x\"",
            "x",
            1,
        ),
        (
            Language::Terraform,
            "value = audit(\"ok\", /* value */ local.x)\n",
            "audit(\"ok\", /* value */ local.x)",
            "audit",
            "local.x",
            "local.x",
            1,
        ),
    ] {
        let parsed = ParsedFile::parse("fixture", source, language).unwrap();
        assert!(
            !parsed.tree.root_node().has_error(),
            "{language:?}: {source}"
        );
        let site = call(&parsed, operation, target, operand, slot);
        let reached = node(&parsed, site.operand, read);
        let path = path_to(&parsed, reached, reached);
        assert_eq!(
            visits(&parsed, &path, &[(site, reached)]),
            [(target, Role::Argument(slot))]
        );
    }
    let parsed = ParsedFile::parse(
        "a.js",
        "function f(x, table, audit) {\n  table[audit](x); audit(x);\n  const fn = new Function('return ' + x);\n}",
        Language::JavaScript,
    ).unwrap();
    let unresolved = first(&parsed, "table[audit](x)");
    let direct = call(&parsed, "audit(x)", "audit", "x", 0);
    let ctor = call(
        &parsed,
        "new Function('return ' + x)",
        "Function",
        "'return ' + x",
        0,
    );
    let def = first(&parsed, "x");
    let path = path_to(&parsed, def, node(&parsed, unresolved, "x"));
    assert_eq!(
        visits(&parsed, &path, &[(direct, direct.operand)]),
        [("audit", Role::Argument(0))]
    );
    let ctor_read = node(&parsed, ctor.operand, "x");
    let path = path_to(&parsed, def, ctor_read);
    assert_eq!(
        visits(&parsed, &path, &[(ctor, ctor_read)]),
        [("Function", Role::Argument(0))]
    );

    let parsed = ParsedFile::parse(
        "a.js",
        "function f(x) {\n  res.send(x = 'safe');\n}",
        Language::JavaScript,
    )
    .unwrap();
    let assignment = first(&parsed, "x = 'safe'");
    let path = path_to(&parsed, first(&parsed, "x"), node(&parsed, assignment, "x"));
    assert!(visits(&parsed, &path, &[]).is_empty());
}
