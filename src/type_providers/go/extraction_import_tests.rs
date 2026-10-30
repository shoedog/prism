use super::extraction_equivalence_tests::{check, files};
use super::*;

#[test]
fn version_aliases_explicit_aliases_dot_imports_and_collisions_are_identical() {
    check(
        &[
            (
                "p/a.go",
                r#"package p
        import ("example/widget/v2"; "gopkg.in/yaml.v3")
        type I interface { M(widget.T, yaml.T) }
        type T struct{}
        func (T) M(x widget.T, y yaml.T) {}
        "#,
            ),
            (
                "p/b.go",
                r#"package p
        import (widget "different/widget"; "example/widget/v2")
        func (T) Explicit(x widget.T) {}
        "#,
            ),
            (
                "q/a.go",
                r#"package q
        import ("first/widget/v2"; "second/widget/v3")
        type T struct{}
        func (T) Ambiguous(x widget.T) {}
        "#,
            ),
            (
                "q/b.go",
                r#"package q
        import . "example/dot"
        func (T) Dot(x Unbound) {}
        "#,
            ),
        ],
        &BTreeMap::new(),
    );
}

#[test]
fn missing_qualifier_and_no_signature_files_are_identical() {
    check(
        &[
            (
                "p/missing.go",
                "package p\ntype T struct{}\nfunc (T) M(x missing.T) {}\n",
            ),
            (
                "p/no_methods.go",
                "package p\ntype Fields struct{ X int }\n",
            ),
            ("p/empty.go", "package p\n"),
            ("p/broken.go", "type T struct{}\nfunc (T) M(x int) {}\n"),
        ],
        &BTreeMap::new(),
    );
}

#[test]
fn plain_and_import_aware_constructors_must_remain_distinct() {
    let rows = [
        ("p/t.go", "package p\ntype T struct{}\nfunc (T) M(x T) {}\n"),
        (
            "q/i.go",
            "package q\nimport p \"example/p\"\ntype I interface {M(p.T)}\n",
        ),
    ];
    let paths = BTreeMap::from([
        ("p/t.go".into(), "example/p".into()),
        ("q/i.go".into(), "example/q".into()),
    ]);
    let aware = check(&rows, &paths);
    let plain = GoTypeProvider::from_parsed_files(&files(&rows));
    assert_ne!(
        plain.data.methods["T"][0].signature,
        aware.data.methods["T"][0].signature
    );
    assert!(!plain.data.satisfaction["I"].contains("T"));
    assert!(aware.data.satisfaction["I"].contains("T"));
}

#[test]
fn changing_source_at_the_same_path_cannot_reuse_old_imports() {
    let make = |import: &str| {
        format!("package p\nimport q \"{import}\"\ntype T struct{{}}\nfunc (T) M(x q.T) {{}}\n")
    };
    let a = make("example/a");
    let b = make("example/b");
    let first = check(&[("p/t.go", &a)], &BTreeMap::new());
    let second = check(&[("p/t.go", &b)], &BTreeMap::new());
    assert_ne!(
        first.data.methods["T"][0].signature,
        second.data.methods["T"][0].signature
    );
}
