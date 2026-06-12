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
