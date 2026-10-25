use super::*;

#[test]
fn root_parameters_match_main_inventory_and_classification() {
    for (language, source) in [
        (Language::TypeScript, "function outer(\n p: number\n) {\n function inner(\n q: string\n) {\n let x=q;\n return x;\n }\n return p;\n}\n"),
        (Language::Python, "def outer(p):\n    def inner(q):\n        x = q\n        return x\n    return p\n"),
        (Language::C, "int outer(int p) {\n int x=p;\n use(x);\n return x;\n}\nint other(int q) { return q; }\n"),
        (Language::Cpp, "int *outer(int p) {\n int *x=source(p);\n use(x);\n return x;\n}\nint other(int q) { return q; }\n"),
    ] {
        let parsed = ParsedFile::parse("parameters", source, language).unwrap();
        let owners = parsed.all_functions();
        assert!(owners.len() >= 2);
        for owner in owners {
            let root = RootFunction::new(&parsed, owner);
            assert_eq!(root.function.id(), owner.id());
            let main_owner = parsed.all_functions().into_iter().find(|f| f.id() == owner.id()).unwrap();
            let expected = parsed.find_parameters_node(&main_owner).unwrap();
            assert!(root.parameters.get().is_none(), "discovery is demand-driven");
            // A parameter-region introduction is classified before walking its
            // ancestors; a body region must not be mistaken for a parameter.
            assert!(introduction_is_classified(&parsed, expected, expected, &root));
            assert_eq!(root.parameters.get(), Some(&Some(expected)));
            let body = owner.child_by_field_name("body").unwrap();
            assert!(!introduction_is_classified(&parsed, body, body, &root));
        }
    }
}

#[test]
fn bare_arrow_keeps_the_original_absent_parameter_region() {
    for language in [Language::JavaScript, Language::TypeScript, Language::Tsx] {
        let parsed = ParsedFile::parse(
            "bare",
            "consume(p => {\n let x=p;\n return x;\n});",
            language,
        )
        .unwrap();
        let owner = parsed.all_functions()[0];
        let root = RootFunction::new(&parsed, owner);
        assert!(parsed.parameter_binding_region(&owner).is_some());
        assert!(
            root.parameters(&parsed).is_none(),
            "retain find_parameters_node semantics"
        );
        let parameter = owner.child_by_field_name("parameter").unwrap();
        assert!(!introduction_is_classified(
            &parsed, parameter, parameter, &root
        ));
    }
}

#[test]
fn rust_closure_parameters_use_the_closure_body_scope() {
    fn find_kind<'tree>(node: Node<'tree>, kind: &str) -> Option<Node<'tree>> {
        if node.kind() == kind {
            return Some(node);
        }
        let mut cursor = node.walk();
        let found = node
            .named_children(&mut cursor)
            .find_map(|child| find_kind(child, kind));
        found
    }

    let parsed = ParsedFile::parse(
        "closure.rs",
        "fn f() { let closure = |parameter: i32| { consume(parameter); }; }",
        Language::Rust,
    )
    .unwrap();
    let owner = parsed.all_functions()[0];
    let parameters =
        find_kind(parsed.tree.root_node(), "closure_parameters").expect("closure parameters");
    let body = parameters
        .parent()
        .and_then(|closure| closure.child_by_field_name("body"))
        .expect("closure body");
    let scope = declaration_scope(
        &parsed,
        &owner,
        parameters,
        DeclarationKind::Other,
        function_scope(&parsed, &owner),
    );

    assert_eq!(scope, scope_span(body));
}

#[test]
#[cfg(debug_assertions)]
#[should_panic(expected = "root function must remain indexed")]
fn unindexed_root_trips_debug_invariant() {
    let parsed = ParsedFile::parse("root.ts", "function f() {}", Language::TypeScript).unwrap();
    let owner = parsed.all_functions()[0];
    RootFunction::new(&parsed, owner.child_by_field_name("body").unwrap());
}
