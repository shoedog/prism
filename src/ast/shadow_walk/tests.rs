use super::*;
use crate::languages::Language;

fn nodes(root: Node<'_>) -> Vec<Node<'_>> {
    let mut out = vec![root];
    let mut cursor = root.walk();
    for child in root.children(&mut cursor) {
        out.extend(nodes(child));
    }
    out
}

fn fixture(language: Language) -> &'static str {
    match language {
        Language::Go => "package p\nfunc outer() {\n x := 0\n {\n x := 1\n use(x)\n f := func() { use(x) }; use(f)\n }\n use(x)\n}\nfunc sibling() { x := 2; use(x) }\n",
        Language::Python => "def outer():\n    x = 0\n    if x:\n        x = 1\n        use(x)\n        def inner():\n            use(x)\n    use(x)\ndef sibling():\n    x = 2\n    use(x)\n",
        Language::JavaScript | Language::TypeScript => "function outer() {\n let x = 0;\n {\n let x = 1;\n use(x);\n function inner() { use(x); }\n }\n use(x);\n}\nfunction sibling() { let x = 2; use(x); }\n",
        Language::Rust => "fn outer() {\n let x = 0;\n {\n let x = 1;\n use_it(x);\n fn inner() { use_it(x); }\n }\n use_it(x);\n}\nfn sibling() { let x = 2; use_it(x); }\n",
        Language::C => "void outer() {\n int x = 0;\n {\n int x = 1;\n use(x);\n void inner() { use(x); }\n }\n use(x);\n}\nvoid sibling() { int x = 2; use(x); }\n",
        Language::Java => "class A { void outer() {\n int x = 0;\n {\n int x = 1;\n use(x);\n class Inner { void inner() { use(x); } }\n }\n use(x);\n}\nvoid sibling() { int x = 2; use(x); } }\n",
        _ => unreachable!(),
    }
}

const LANGUAGES: [Language; 7] = [
    Language::Go,
    Language::Python,
    Language::JavaScript,
    Language::TypeScript,
    Language::Rust,
    Language::C,
    Language::Java,
];

fn compare(parsed: &ParsedFile, references_only: bool, all_lines: bool) -> usize {
    let all = nodes(parsed.tree.root_node());
    let mut owners = parsed.all_functions();
    owners.push(parsed.tree.root_node());
    if !references_only {
        owners.extend(all.iter().copied().filter(|n| {
            parsed.language.is_scope_block(n.kind())
                || parsed
                    .language
                    .callable_boundary_node_types()
                    .contains(&n.kind())
        }));
    }
    let mut count = 0;
    for node in all.iter().copied().filter(|n| {
        !references_only
            || parsed.language.is_identifier_node(n.kind()) && parsed.node_text(n) == "x"
    }) {
        // Check the chain itself, not just its boolean projection. This pins
        // equal-range wrappers, extras, MISSING nodes, and bottom-up order.
        let actual: Vec<_> = ancestors(parsed.tree.root_node(), node)
            .unwrap()
            .into_iter()
            .rev()
            .map(|n| n.id())
            .collect();
        let expected: Vec<_> = std::iter::successors(node.parent(), |n| n.parent())
            .map(|n| n.id())
            .collect();
        assert_eq!(actual, expected, "chain for {}", node.to_sexp());
        for owner in &owners {
            let lines: Vec<_> = if all_lines {
                (0..=parsed.source.lines().count() + 2).collect()
            } else {
                vec![0, 1, node.start_position().row + 1, usize::MAX]
            };
            for line in lines {
                for name in ["x", "absent"] {
                    assert_eq!(
                        parsed.is_shadowed_at(&node, owner, name, line),
                        legacy(parsed, node, *owner, name, line),
                        "{:?}, node={}, owner={}, name={name}, def={line}",
                        parsed.language,
                        node.to_sexp(),
                        owner.to_sexp(),
                    );
                    count += 1;
                }
            }
        }
    }
    count
}

#[test]
fn every_node_and_reference_matches_parent_walk_in_seven_languages() {
    for language in LANGUAGES {
        let parsed = ParsedFile::parse("synthetic", fixture(language), language).unwrap();
        assert!(!parsed.tree.root_node().has_error(), "{language:?}");
        let callable_kinds = language.callable_boundary_node_types();
        assert!(
            nodes(parsed.tree.root_node())
                .iter()
                .filter(|n| callable_kinds.contains(&n.kind()))
                .count()
                >= 3,
            "nested callable required for {language:?}"
        );
        assert!(compare(&parsed, false, true) > 1_000);
    }
}

#[test]
fn recovery_missing_empty_equal_ranges_and_extras() {
    let mut missing = 0;
    let mut errors = 0;
    let mut equal_ranges = 0;
    let mut extras = 0;
    for language in LANGUAGES {
        let broken = match language {
            Language::Go => "package p\nfunc f() { x := ; use(x)\n",
            Language::Python => "def f():\n    x = (\n    use(x)\n",
            Language::JavaScript | Language::TypeScript => {
                "function f() { let x = ; /* comment */ use(x)\n"
            }
            Language::Rust => "fn f() { let x = ; use_it(x)\n",
            Language::C => "void f() { int x = ; /* comment */ use(x)\n",
            Language::Java => "class A { void f() { int x = ; /* comment */ use(x)\n",
            _ => unreachable!(),
        };
        for source in [broken, "", "/* comment */", "x", "function f() { use(x); }"] {
            let parsed = ParsedFile::parse("recovery", source, language).unwrap();
            for node in nodes(parsed.tree.root_node()) {
                missing += usize::from(node.is_missing());
                errors += usize::from(node.is_error());
                extras += usize::from(node.is_extra());
                equal_ranges += usize::from(
                    node.parent()
                        .is_some_and(|p| p.byte_range() == node.byte_range()),
                );
            }
            compare(&parsed, false, true);
        }
    }
    assert!(missing > 0, "must exercise zero-width MISSING nodes");
    assert!(errors > 0, "must exercise ERROR recovery");
    assert!(equal_ranges > 0, "must exercise equal-range chains");
    assert!(extras > 0, "must exercise comments/extras");
}

fn js_fixture() -> ParsedFile {
    ParsedFile::parse(
        "synthetic.js",
        fixture(Language::JavaScript),
        Language::JavaScript,
    )
    .unwrap()
}

#[test]
fn inclusive_rows_stop_boundary_and_node_itself() {
    let parsed = js_fixture();
    let all = nodes(parsed.tree.root_node());
    let inner = all
        .iter()
        .copied()
        .find(|n| n.kind() == "statement_block" && n.start_position().row == 2)
        .unwrap();
    let reference = all
        .iter()
        .copied()
        .find(|n| parsed.node_text(n) == "x" && n.start_position().row == 4)
        .unwrap();
    let owner = parsed.all_functions()[0];
    for line in [inner.start_position().row + 1, inner.end_position().row + 1] {
        assert!(!parsed.is_shadowed_at(&reference, &owner, "x", line));
    }
    assert!(parsed.is_shadowed_at(&reference, &owner, "x", 2));
    assert!(parsed.is_shadowed_at(&reference, &owner, "x", usize::MAX));
    assert!(!parsed.is_shadowed_at(&reference, &owner, "absent", usize::MAX));
    // A scope-block boundary with its own declaration MUST NOT be tested.
    assert!(!parsed.is_shadowed_at(&reference, &inner, "x", usize::MAX));
    // When node == boundary the original starts above it, and sees outer x.
    assert!(parsed.is_shadowed_at(&inner, &inner, "x", usize::MAX));
    // A declaration on node itself is not an ancestor declaration.
    assert!(!parsed.is_shadowed_at(&inner, &owner, "x", 2));
    assert!(!parsed.is_shadowed_at(&parsed.tree.root_node(), &owner, "x", 0));
}

#[test]
fn outside_boundary_and_foreign_tree_preserve_legacy() {
    let parsed = js_fixture();
    let owners = parsed.all_functions();
    let sibling = *owners.last().unwrap();
    let reference = nodes(sibling)
        .into_iter()
        .filter(|n| parsed.node_text(n) == "x")
        .last()
        .unwrap();
    assert!(ancestors(owners[0], reference).is_none());
    assert!(parsed.is_shadowed_at(&reference, &owners[0], "x", 0));
    assert!(!parsed.is_shadowed_at(&reference, &owners[0], "x", 10));
    DESCENTS.with(|count| count.set(0));
    assert!(ancestors(owners[0], reference).is_none());
    let failed_attempt = DESCENTS.with(|count| count.get());
    let from_root = ancestors(parsed.tree.root_node(), reference).unwrap().len();
    DESCENTS.with(|count| count.set(0));
    assert!(!parsed.is_shadowed_at(&reference, &owners[0], "absent", 0));
    assert_eq!(
        DESCENTS.with(|count| count.get()),
        failed_attempt + from_root
    );
    let clone = parsed.tree.clone();
    let own_ids: std::collections::HashSet<_> = nodes(parsed.tree.root_node())
        .into_iter()
        .map(|n| n.id())
        .collect();
    let shared = nodes(clone.root_node())
        .into_iter()
        .find(|n| own_ids.contains(&n.id()))
        .expect("cloned trees share descendant IDs");
    assert!(ancestors(parsed.tree.root_node(), shared).is_none());
    for node in nodes(clone.root_node()) {
        for line in [0, 4, usize::MAX] {
            assert_eq!(
                parsed.is_shadowed_at(&node, &owners[0], "x", line),
                legacy(&parsed, node, owners[0], "x", line),
            );
        }
    }
    let foreign = js_fixture();
    let foreign_ref = nodes(foreign.tree.root_node())
        .into_iter()
        .find(|n| foreign.node_text(n) == "x" && n.start_position().row == 4)
        .unwrap();
    assert!(ancestors(parsed.tree.root_node(), foreign_ref).is_none());
    for line in [0, 4, usize::MAX] {
        for name in ["x", "absent"] {
            assert_eq!(
                parsed.is_shadowed_at(&foreign_ref, &owners[0], name, line),
                legacy(&parsed, foreign_ref, owners[0], name, line),
            );
        }
    }
}

#[test]
fn wide_switch_every_reference_and_one_descent_per_ancestor() {
    let cases: String = (0..320)
        .map(|i| format!("case {i}: {{ x := {i}; use(x) }}\n"))
        .collect();
    let source =
        format!("package p\nfunc wide() {{ switch n {{\n{cases}}}\n}}\nfunc other() {{}}\n");
    let parsed = ParsedFile::parse("wide.go", &source, Language::Go).unwrap();
    assert!(!parsed.tree.root_node().has_error());
    let all = nodes(parsed.tree.root_node());
    assert_eq!(
        all.iter().filter(|n| n.kind() == "expression_case").count(),
        320
    );
    assert!(compare(&parsed, true, false) > 10_000);

    let owner = parsed.all_functions()[0];
    for reference in all
        .into_iter()
        .filter(|n| parsed.language.is_identifier_node(n.kind()) && parsed.node_text(n) == "x")
    {
        let expected = ancestors(owner, reference).unwrap().len();
        DESCENTS.with(|count| count.set(0));
        assert!(!parsed.is_shadowed_at(&reference, &owner, "absent", 0));
        assert_eq!(DESCENTS.with(|count| count.get()), expected);
        assert!(expected > 2 && expected < 20);
    }
}
