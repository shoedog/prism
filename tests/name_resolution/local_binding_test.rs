use std::collections::BTreeMap;

use prism::ast::ParsedFile;
use prism::languages::Language;
use prism::name_resolution::binding_lookup::{lookup_visible_binding, BindingKind};
use prism::name_resolution::rust_policy::NS_VALUE;
use prism::name_resolution::rust_populator::{populate_rust, RustCrateConfig};
use prism::name_resolution::types::{BindTarget, Target};

fn rs(path: &str, src: &str) -> (String, ParsedFile) {
    (
        path.to_string(),
        ParsedFile::parse(path, src, Language::Rust).unwrap(),
    )
}

fn files(pairs: &[(&str, &str)]) -> BTreeMap<String, ParsedFile> {
    pairs.iter().map(|(p, s)| rs(p, s)).collect()
}

fn convention(fs: &BTreeMap<String, ParsedFile>) -> RustCrateConfig {
    RustCrateConfig::from_convention(fs)
}

#[test]
fn function_param_binding_refs_preserve_source_order_ordinals() {
    let src = "fn f(a: i32, b: i32) { let _ = (a, b); }\n";
    let fs = files(&[("src/lib.rs", src)]);
    let graph = populate_rust(&fs, &convention(&fs), None);
    let file = graph.file_paths["src/lib.rs"];
    let a_def = src.find("a: i32").unwrap();
    let b_def = src.find("b: i32").unwrap();

    let param_locals: BTreeMap<_, _> = graph
        .bindings
        .iter()
        .filter(|binding| binding.ns == NS_VALUE && matches!(binding.name.as_str(), "a" | "b"))
        .filter_map(|binding| match &binding.target {
            BindTarget::Resolved(Target::Local(binding_ref)) => {
                Some((binding.name.as_str(), binding_ref.clone()))
            }
            _ => None,
        })
        .collect();

    assert_eq!(
        param_locals.len(),
        2,
        "expected one local binding per param"
    );
    assert_eq!(param_locals["a"].scope, param_locals["b"].scope);
    assert_eq!(param_locals["a"].ordinal, 0);
    assert_eq!(param_locals["b"].ordinal, 1);
    assert_ne!(param_locals["a"], param_locals["b"]);
    assert_eq!(
        graph.local_facts.get(&(file, a_def)).map(|fact| &fact.kind),
        Some(&BindingKind::Param)
    );
    assert_eq!(
        graph.local_facts.get(&(file, b_def)).map(|fact| &fact.kind),
        Some(&BindingKind::Param)
    );
}

#[test]
fn lookup_visible_binding_returns_none_for_duplicate_same_rib_local() {
    let src = "fn f() { let x = 1; x; }\n";
    let fs = files(&[("src/lib.rs", src)]);
    let mut graph = populate_rust(&fs, &convention(&fs), None);
    let file = graph.file_paths["src/lib.rs"];
    let use_byte = src.rfind("x;").unwrap();
    let duplicate = graph
        .bindings
        .iter()
        .find(|binding| {
            binding.ns == NS_VALUE
                && binding.name == "x"
                && matches!(binding.target, BindTarget::Resolved(Target::Local(_)))
        })
        .expect("local x binding")
        .clone();
    graph.bindings.push(duplicate);

    assert_eq!(lookup_visible_binding(&graph, file, use_byte, "x"), None);
}
