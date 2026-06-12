use prism::ast::ParsedFile;
use prism::languages::Language;

fn parse(src: &str, lang: Language, path: &str) -> ParsedFile {
    ParsedFile::parse(path, src, lang).unwrap()
}

fn owners(pf: &ParsedFile) -> Vec<(Option<String>, Option<String>)> {
    pf.functions()
        .iter()
        .map(|f| (f.name.clone(), f.owner.clone()))
        .collect()
}

#[test]
fn rust_inherent_impl_method_has_owner() {
    let pf = parse(
        "struct Foo;\nimpl Foo {\n    fn m(&self) {}\n}\nfn free() {}\n",
        Language::Rust,
        "a.rs",
    );
    let o = owners(&pf);
    assert!(o.contains(&(Some("m".into()), Some("Foo".into()))));
    assert!(o.contains(&(Some("free".into()), None)));
}

#[test]
fn rust_generic_impl_owner_strips_generics() {
    let pf = parse(
        "impl<T> Wrapper<T> {\n    fn get(&self) {}\n}\n",
        Language::Rust,
        "a.rs",
    );
    assert!(owners(&pf).contains(&(Some("get".into()), Some("Wrapper".into()))));
}

#[test]
fn rust_trait_impl_owner_is_type_not_trait() {
    let pf = parse(
        "impl Display for Foo {\n    fn fmt(&self) {}\n}\n",
        Language::Rust,
        "a.rs",
    );
    assert!(owners(&pf).contains(&(Some("fmt".into()), Some("Foo".into()))));
}

#[test]
fn rust_trait_default_method_owner_is_trait() {
    let pf = parse(
        "trait Greet {\n    fn hello(&self) {}\n}\n",
        Language::Rust,
        "a.rs",
    );
    assert!(owners(&pf).contains(&(Some("hello".into()), Some("Greet".into()))));
}

#[test]
fn rust_nested_fn_inside_method_is_not_a_method() {
    let pf = parse(
        "impl Foo {\n    fn m(&self) {\n        fn helper() {}\n        helper();\n    }\n}\n",
        Language::Rust,
        "a.rs",
    );
    assert!(owners(&pf).contains(&(Some("helper".into()), None)));
}

#[test]
fn go_method_owner_and_receiver_var() {
    let pf = parse(
        "package p\n\ntype T struct{}\n\nfunc (t *T) M() {}\n\nfunc Free() {}\n",
        Language::Go,
        "a.go",
    );
    let f = pf
        .functions()
        .iter()
        .find(|f| f.name.as_deref() == Some("M"))
        .unwrap();
    assert_eq!(f.owner.as_deref(), Some("T")); // '*' stripped by owner_key
    assert_eq!(f.receiver_var.as_deref(), Some("t"));
    let free = pf
        .functions()
        .iter()
        .find(|f| f.name.as_deref() == Some("Free"))
        .unwrap();
    assert_eq!(free.owner, None);
}

#[test]
fn python_direct_member_only() {
    let pf = parse(
        "class C:\n    def m(self):\n        def nested():\n            pass\n\n@deco\ndef free():\n    pass\n",
        Language::Python,
        "a.py",
    );
    let o = owners(&pf);
    assert!(o.contains(&(Some("m".into()), Some("C".into()))));
    assert!(o.contains(&(Some("nested".into()), None)));
    assert!(o.contains(&(Some("free".into()), None)));
}

#[test]
fn js_class_method_owner() {
    let pf = parse(
        "class Widget {\n  render() {}\n  handler = () => {};\n}\nfunction free() {}\n",
        Language::JavaScript,
        "a.js",
    );
    let o = owners(&pf);
    assert!(o.contains(&(Some("render".into()), Some("Widget".into()))));
    // class-field arrow method (plan-review MINOR): owner via field_definition -> class_body
    assert!(o.contains(&(Some("handler".into()), Some("Widget".into()))));
    assert!(o.contains(&(Some("free".into()), None)));
}

#[test]
fn java_every_method_has_owner() {
    let pf = parse(
        "class App {\n    void run() {}\n    static void main(String[] a) {}\n}\n",
        Language::Java,
        "A.java",
    );
    for f in pf.functions() {
        assert!(f.owner.as_deref() == Some("App"), "{:?}", f.name);
    }
}

#[test]
fn cpp_in_class_and_out_of_line_owners() {
    let pf = parse(
        "class Foo {\n  void bar();\n  void inline_m() {}\n};\nvoid Foo::bar() {}\nvoid free_fn() {}\nnamespace ns { void nf(); }\nvoid ns::nf() {}\n",
        Language::Cpp,
        "a.cpp",
    );
    let o = owners(&pf);
    assert!(o.contains(&(Some("inline_m".into()), Some("Foo".into()))));
    assert!(o.contains(&(Some("bar".into()), Some("Foo".into())))); // out-of-line
    assert!(o.contains(&(Some("free_fn".into()), None)));
    // namespace prefix indexes uniformly as the prefix key (spec 2.1 C++ row)
    assert!(o.contains(&(Some("nf".into()), Some("ns".into()))));
}

#[test]
fn cpp_templated_out_of_line_method_has_owner() {
    let pf = parse(
        "template <typename T>\nclass Foo {\n  void bar();\n};\ntemplate <typename T>\nvoid Foo<T>::bar() {}\n",
        Language::Cpp,
        "a.cpp",
    );

    assert!(owners(&pf).contains(&(Some("bar".into()), Some("Foo".into()))));
}

#[test]
fn lua_table_methods_key_by_bare_name_with_owner() {
    let pf = parse(
        "local M = {}\nfunction M.f() end\nfunction M:g() end\nfunction free() end\n",
        Language::Lua,
        "a.lua",
    );
    let o = owners(&pf);
    // KEYING CHANGE (spec 2.1): name is now "f", not "M.f"
    assert!(o.contains(&(Some("f".into()), Some("M".into()))));
    assert!(o.contains(&(Some("g".into()), Some("M".into()))));
    assert!(o.contains(&(Some("free".into()), None)));
}
