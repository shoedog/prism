use crate::common::*;

fn slots(source: &str, language: Language) -> Option<Vec<String>> {
    let parsed = ParsedFile::parse("test", source, language).unwrap();
    let function = parsed.all_functions().into_iter().next().unwrap();
    parsed.function_parameter_slots(&function)
}

fn first_node_of_kind<'a>(
    node: tree_sitter::Node<'a>,
    kind: &str,
) -> Option<tree_sitter::Node<'a>> {
    if node.kind() == kind {
        return Some(node);
    }
    let mut cursor = node.walk();
    let found = node
        .children(&mut cursor)
        .find_map(|child| first_node_of_kind(child, kind));
    found
}

fn nodes_of_kind<'a>(node: tree_sitter::Node<'a>, kind: &str) -> Vec<tree_sitter::Node<'a>> {
    let mut out = Vec::new();
    if node.kind() == kind {
        out.push(node);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        out.extend(nodes_of_kind(child, kind));
    }
    out
}

#[test]
fn javascript_slots_keep_defaults_but_stop_at_rest_or_destructuring() {
    assert_eq!(
        slots(
            "function f(a, b = ignored, cb, ...rest) {}",
            Language::JavaScript,
        ),
        Some(vec!["a".into(), "b".into(), "cb".into()]),
    );
    assert_eq!(
        slots("function f({value}, cb) {}", Language::JavaScript),
        Some(vec![]),
    );
    assert_eq!(
        slots("function f([value], cb) {}", Language::JavaScript),
        Some(vec![]),
    );
    assert_eq!(
        slots("const f = cb => cb();", Language::JavaScript),
        Some(vec!["cb".into()]),
    );
}

#[test]
fn javascript_duplicate_or_recovered_parameter_lists_are_unknown() {
    assert_eq!(
        slots("function f(cb, cb) {}", Language::JavaScript),
        None,
        "duplicate bindings cannot identify a callback slot exactly",
    );
    assert_eq!(
        slots("function f({cb}, cb) {}", Language::JavaScript),
        None,
        "a duplicate after a truncated prefix still makes the list ambiguous",
    );
    let duplicate = ParsedFile::parse(
        "duplicate.js",
        "function f(cb, cb) {}",
        Language::JavaScript,
    )
    .unwrap();
    assert_eq!(
        duplicate.functions()[0].param_names,
        None,
        "the eager FunctionInfo fallback must retain the fail-closed state",
    );
    assert_eq!(
        slots("function f(a, {) {}", Language::JavaScript),
        None,
        "any parse-recovery node in parameters fails closed",
    );
    assert_eq!(
        slots("function f({x = 0}, x) {}", Language::JavaScript),
        None,
        "object_assignment_pattern bindings participate in duplicate detection",
    );
}

#[test]
fn typescript_slots_skip_this_and_stop_at_typed_rest_or_destructuring() {
    assert_eq!(
        slots(
            "function f(this: void, a: string, b?: number, ...rest: string[]) {}",
            Language::TypeScript,
        ),
        Some(vec!["a".into(), "b".into()]),
    );
    assert_eq!(
        slots(
            "function f({value}: {value: string}, cb: () => void) {}",
            Language::TypeScript,
        ),
        Some(vec![]),
    );
    assert_eq!(
        slots(
            "function f({x = 0}: {x?: number}, x: number) {}",
            Language::TypeScript,
        ),
        None,
        "TypeScript object_assignment_pattern bindings participate in duplicate detection",
    );
}

#[test]
fn go_slots_and_binding_occurrences_expand_grouped_declarations() {
    let source = "package p\nfunc f(a, b string, c int, rest ...string) {}\n";
    let parsed = ParsedFile::parse("test.go", source, Language::Go).unwrap();
    let function = parsed.all_functions().into_iter().next().unwrap();

    assert_eq!(
        parsed.function_parameter_slots(&function),
        Some(vec!["a".into(), "b".into(), "c".into()]),
    );
    assert_eq!(
        parsed.function_parameter_names(&function),
        vec!["a", "b", "c", "rest"],
        "binding consumers retain every real Go parameter name",
    );
    let occurrences = parsed
        .function_parameter_slot_occurrences(&function)
        .unwrap();
    assert_eq!(
        occurrences
            .iter()
            .map(|(name, _, _)| name.as_str())
            .collect::<Vec<_>>(),
        vec!["a", "b", "c"],
    );

    assert_eq!(
        slots(
            "package p\nfunc f(a int, _ string, c int) {}\n",
            Language::Go,
        ),
        Some(vec!["a".into()]),
        "a blank declaration is a positional boundary",
    );
    assert_eq!(
        slots("package p\nfunc f(a int, string) {}\n", Language::Go,),
        Some(vec!["a".into()]),
        "an unnamed Go declaration is a positional boundary",
    );

    let grouped_blank = ParsedFile::parse(
        "grouped_blank.go",
        "package p\nfunc f(a, _ string, c int) {}\n",
        Language::Go,
    )
    .unwrap();
    let function = grouped_blank.all_functions().into_iter().next().unwrap();
    assert_eq!(
        grouped_blank.function_parameter_slots(&function),
        Some(vec!["a".into()]),
        "a grouped blank keeps the deterministic prefix before the blank",
    );
    assert_eq!(
        grouped_blank.function_parameter_names(&function),
        vec!["a", "c"],
        "Go blanks are not real binding occurrences",
    );
    assert_eq!(
        grouped_blank
            .function_parameter_occurrences(&function)
            .into_iter()
            .map(|(name, _, _)| name)
            .collect::<Vec<_>>(),
        vec!["a", "c"],
        "DFG and reasoning must not receive a blank binding occurrence",
    );
}

#[test]
fn rust_and_python_slots_preserve_prefixes_and_pseudo_parameter_rules() {
    assert_eq!(
        slots(
            "struct S; impl S { fn f(&self, a: u32, _: u32, c: u32) {} }",
            Language::Rust,
        ),
        Some(vec!["a".into()]),
    );
    assert_eq!(
        slots("def f(a=0, b=0, *args, c=0): pass", Language::Python),
        Some(vec!["a".into(), "b".into()]),
    );
    assert_eq!(
        slots("def f(a, *, b): pass", Language::Python),
        Some(vec!["a".into()]),
        "keyword-only parameters are not positional slots",
    );

    let parsed = ParsedFile::parse(
        "test.rs",
        "fn f() { let _closure = |a: u32, _: u32| a; }",
        Language::Rust,
    )
    .unwrap();
    let closure = first_node_of_kind(parsed.tree.root_node(), "closure_expression")
        .expect("Rust closure expression");
    assert_eq!(
        parsed.function_parameter_slots(&closure),
        Some(vec!["a".into()]),
        "Rust closure parameters share the same prefix contract",
    );

    let untyped = ParsedFile::parse(
        "untyped.rs",
        "fn f() { let one = |cb| cb(); let two = |a, (b, c)| a; }",
        Language::Rust,
    )
    .unwrap();
    let closures = nodes_of_kind(untyped.tree.root_node(), "closure_expression");
    assert_eq!(
        untyped.function_parameter_slots(&closures[0]),
        Some(vec!["cb".into()]),
        "untyped Rust closure identifiers are runtime slots",
    );
    assert_eq!(
        untyped.function_parameter_slots(&closures[1]),
        Some(vec!["a".into()]),
        "an untyped non-simple closure pattern truncates after its prefix",
    );

    let mutable = ParsedFile::parse("mutable.rs", "fn f(mut x: i32) {}", Language::Rust).unwrap();
    let function = mutable.all_functions().into_iter().next().unwrap();
    assert_eq!(
        mutable.function_parameter_names(&function),
        vec!["x"],
        "the non-positional Rust name API preserves main's name-field behavior",
    );
}

#[test]
fn java_parameters_are_not_currently_positional_slots() {
    assert_eq!(
        slots("class C { void f(int value) {} }", Language::Java),
        None,
    );
}
