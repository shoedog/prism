use super::*;
use crate::languages::Language;

fn parsed(path: &str, source: &str) -> ParsedFile {
    ParsedFile::parse(path, source, Language::from_path(path).unwrap()).unwrap()
}

#[test]
fn exact_source_facts_keep_call_owner_arguments_and_value_path() {
    let source = "function h(obj) { f(obj.field); g('safe'); }";
    let parsed = parsed("app.js", source);
    let facts = ReviewOccurrenceFacts::new(&parsed);
    let start = source.find("f(obj.field)").unwrap();
    let call = facts
        .call(start..start + "f(obj.field)".len())
        .expect("actual call");
    assert_eq!(call.callee, "f");
    assert_eq!(call.arguments, [start + 2..start + 11]);
    let owner = call.owner.as_ref().unwrap();
    assert!(owner.unique);
    assert_eq!(owner.function.as_ref().unwrap().name, "h");
    let values = facts.values(start + 2..start + 11);
    assert!(values.iter().any(|value| value.semantic_value
        && value.occurrence.path == crate::access_path::AccessPath::from_expr("obj.field")
        && value.owner.as_ref() == Some(owner)));
    assert!(facts.call(start + 1..start + 12).is_none());
    assert!(facts.values(start + 2..start + 10).is_empty());
}

#[test]
fn full_boundary_census_retains_unindexed_callables() {
    for (path, source, boundary) in [
        (
            "a.py",
            "def h(x):\n    outer(lambda: inner(x))\n",
            "lambda: inner(x)",
        ),
        (
            "a.go",
            "package p\nfunc h(x string) { outer(func() { inner(x) }) }",
            "func() { inner(x) }",
        ),
        (
            "a.js",
            "function h(x) { outer(function*(){ inner(x); }); }",
            "function*(){ inner(x); }",
        ),
    ] {
        let parsed = parsed(path, source);
        let facts = ReviewOccurrenceFacts::new(&parsed);
        let start = source.find(boundary).unwrap();
        let owner = facts
            .owners()
            .iter()
            .find(|owner| owner.span == (start..start + boundary.len()))
            .expect("full callable boundary retained");
        if path != "a.js" {
            assert!(owner.function.is_none(), "unindexed owner is unknown");
        }
        let start = source.find("inner(x)").unwrap();
        assert_eq!(
            facts.call(start..start + 8).unwrap().owner.as_ref(),
            Some(owner)
        );
    }
}

#[test]
fn repeated_named_and_inferred_arrow_identities_are_ambiguous() {
    for source in [
        "function h(x) { f(x); } function h(y) { g(y); }",
        "var h = x => f(x); var h = y => g(y);",
    ] {
        let parsed = parsed("app.js", source);
        let facts = ReviewOccurrenceFacts::new(&parsed);
        let owners: Vec<_> = facts
            .owners()
            .iter()
            .filter(|owner| owner.function.as_ref().is_some_and(|id| id.name == "h"))
            .collect();
        assert_eq!(owners.len(), 2);
        assert!(owners.iter().all(|owner| !owner.unique));
        assert_ne!(owners[0].span, owners[1].span);
    }
}

#[test]
fn decorated_bodies_are_owned_but_headers_and_class_keys_are_not() {
    for (path, source) in [
        ("a.py", "@deco\ndef h(x=header(y)):\n    f(x)\n"),
        ("a.js", "class C { [header(y)](x) { f(x); } }"),
    ] {
        let parsed = parsed(path, source);
        let facts = ReviewOccurrenceFacts::new(&parsed);
        for (text, owned) in [("header(y)", false), ("f(x)", true)] {
            let start = source.find(text).unwrap();
            assert_eq!(
                facts
                    .call(start..start + text.len())
                    .unwrap()
                    .owner
                    .is_some(),
                owned
            );
        }
        if path == "a.py" {
            assert_eq!(
                facts
                    .owners()
                    .iter()
                    .filter(|owner| owner.function.as_ref().is_some_and(|id| id.name == "h"))
                    .count(),
                1
            );
        }
    }
    let parsed = parsed("a.rs", "fn h(x: i32) { f(x); }");
    assert!(ReviewOccurrenceFacts::new(&parsed).owners().is_empty());
}
