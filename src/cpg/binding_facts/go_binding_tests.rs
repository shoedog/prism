//! Binding facts are tested even when the producer emits no endpoint.
use super::*;

fn check(source: &str, name: &str, bound: bool) {
    let parsed = ParsedFile::parse("binding.go", source, Language::Go).unwrap();
    assert!(!parsed.tree.root_node().has_error());
    let owner = parsed.all_functions()[0];
    let facts = BindingFacts::new(&parsed, &owner, &[], std::iter::empty());
    let marker = source.find("// read").unwrap();
    let byte = source[..marker].rfind(name).unwrap();
    assert_eq!(
        facts.visible_declaration(name, byte).is_some(),
        bound,
        "{source}"
    );
}

#[test]
fn go_fold_r2_signatures_without_endpoints() {
    for source in [
        "package p\nfunc f(v int) { println(v) // read\n}\n",
        "package p\nfunc f(v ...int) { println(v) // read\n}\n",
        "package p\nfunc f() (v int) { println(v) // read\nreturn }\n",
        "package p\ntype T int\nfunc (v T) f() { println(v) // read\n}\n",
        "package p\nfunc f[V ~int]() { println(V(1)) // read\n}\n",
        "package p\ntype T[V ~int] struct{}\nfunc (p T[V]) f() { println(V(1)) // read\n}\n",
        "package p\ntype T[V ~int] struct{}\nfunc (p *T[V]) f() { println(V(1)) // read\n}\n",
        "package p\nfunc f() { g := func(v int) { println(v) // read\n}; println(g) }\n",
        "package p\nfunc f() { g := func(v ...int) { println(v) // read\n}; println(g) }\n",
        "package p\nfunc f() { g := func() (v int) { println(v) // read\nreturn }; println(g) }\n",
    ] {
        check(source, if source.contains("[V") { "V" } else { "v" }, true);
    }
    check("package p\nfunc f() { println(v) // read\n}\n", "v", false);
    check(
        "package p\nfunc f() { v: println(v) // read\ngoto v }\n",
        "v",
        false,
    );
    check(
        "package p\nfunc f() { _ := 1; println(_) // read\n}\n",
        "_",
        false,
    );
}

#[test]
fn go_fold_r2_visibility_and_literal_capture() {
    let source = "package p\nfunc f() {\nv := 1\ng := func(v int) (q int) {\nprintln(v) // inner\nreturn\n}\nprintln(v) // outer\nprintln(g)\n}\n";
    let parsed = ParsedFile::parse("capture.go", source, Language::Go).unwrap();
    let owner = parsed.all_functions()[0];
    let facts = BindingFacts::new(&parsed, &owner, &[], std::iter::empty());
    let at = |marker: &str| source[..source.find(marker).unwrap()].rfind('v').unwrap();
    let inner = facts
        .visible_declaration("v", at("// inner"))
        .expect("literal parameter");
    let outer = facts
        .visible_declaration("v", at("// outer"))
        .expect("outer declaration");
    assert_ne!(inner.id, outer.id);
    let source = source.replace("func(v int) (q int)", "func() (q int)");
    let parsed = ParsedFile::parse("capture.go", &source, Language::Go).unwrap();
    let facts = BindingFacts::new(&parsed, &parsed.all_functions()[0], &[], std::iter::empty());
    let bindings: Vec<_> = ["// inner", "// outer"]
        .iter()
        .map(|marker| {
            let byte = source[..source.find(marker).unwrap()].rfind('v').unwrap();
            facts.visible_declaration("v", byte).unwrap().id
        })
        .collect();
    assert_eq!(bindings[0], bindings[1]);
}

#[test]
fn go_fold_r2_full_population_bindings_without_endpoints() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/go_fold_r2");
    for row in include_str!("../../../tests/fixtures/go_fold_r2/population.jsonl").lines() {
        let row: serde_json::Value = serde_json::from_str(row).unwrap();
        let file = row["file"].as_str().unwrap();
        let source = std::fs::read_to_string(root.join(file)).unwrap();
        let parsed = ParsedFile::parse(file, &source, Language::Go).unwrap();
        let facts = BindingFacts::new(&parsed, &parsed.all_functions()[0], &[], std::iter::empty());
        let binding = |marker: &str| {
            let byte = source[..source.find(marker).unwrap()].rfind('v').unwrap();
            facts.visible_declaration("v", byte).unwrap().id
        };
        if row["check"] == "distinct" {
            assert_ne!(binding("// inner"), binding("// write"), "{file}");
        } else if row["check"] == "existing" || file == "literal_capture.go" {
            assert_eq!(binding("// inner"), binding("// write"), "{file}");
        }
    }
}

#[test]
fn go_fold_r2_initializers_read_outer_before_new_binding() {
    for statement in ["v := v + 1", "var v = v + 1", "var (v = v + 1)"] {
        let source = format!("package p\nfunc f(v int) {{\nif true {{\n{statement} // rhs\nprintln(v) // inner\n}}\nprintln(v) // outer\n}}\n");
        let parsed = ParsedFile::parse("rhs.go", &source, Language::Go).unwrap();
        let facts = BindingFacts::new(&parsed, &parsed.all_functions()[0], &[], std::iter::empty());
        let binding = |marker: &str| {
            let byte = source[..source.find(marker).unwrap()].rfind('v').unwrap();
            facts.visible_declaration("v", byte).unwrap().id
        };
        assert_eq!(binding("// rhs"), binding("// outer"), "{statement}");
        assert_ne!(binding("// inner"), binding("// outer"), "{statement}");
    }
    // Types in ordinary instantiations and function types introduce no locals.
    check(
        "package p\nfunc f() { var q Box[V]; println(V) // read\nprintln(q) }\n",
        "V",
        false,
    );
    check(
        "package p\nfunc f() { var q func(v int); println(v) // read\nprintln(q) }\n",
        "v",
        false,
    );
}
