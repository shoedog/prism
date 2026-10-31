use prism::ast::ParsedFile;
use prism::call_graph::{CallGraph, ScopeGraphBuildInputs};
use prism::languages::Language::Rust;
use prism::resolution::{DropReason, ResolutionConfidence as Confidence, ResolutionKind as Kind};
use std::collections::BTreeMap;

fn graph(sources: &[(&str, &str)]) -> CallGraph {
    graph_edition(sources, 2021)
}

fn graph_edition(sources: &[(&str, &str)], edition: u16) -> CallGraph {
    graph_with(sources, edition, true, &[])
}

// A workspace mixing anchoring classes records edition_uniform == false and
// keeps one graph-wide edition; leading `::` cannot be read with it.
fn graph_with(
    sources: &[(&str, &str)],
    edition: u16,
    uniform: bool,
    extra_roots: &[&str],
) -> CallGraph {
    let files = sources
        .iter()
        .map(|(path, text)| {
            (
                path.to_string(),
                ParsedFile::parse(path, text, Rust).unwrap(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut inputs = ScopeGraphBuildInputs::from_files_convention(&files);
    inputs.cfg.edition = edition;
    inputs.cfg.edition_uniform = uniform;
    inputs.cfg.crate_roots = files
        .keys()
        .filter(|p| p.ends_with("src/lib.rs") || extra_roots.contains(&p.as_str()))
        .cloned()
        .collect();
    inputs.cfg.workspace_members = inputs
        .cfg
        .crate_roots
        .iter()
        .filter_map(|p| p.strip_suffix("/src/lib.rs").map(str::to_string))
        .collect();
    CallGraph::build_with_scope_graph_inputs(&files, Some(&inputs))
}

fn check(
    cg: &CallGraph,
    call: &str,
    expected: &[(&str, Kind, Confidence)],
    drop: Option<DropReason>,
) {
    let site = cg
        .calls
        .iter()
        .filter(|(f, _)| f.name == "invoke")
        .flat_map(|(_, sites)| sites)
        .find(|s| s.callee_name == call)
        .unwrap();
    let out = cg.resolve_call_site_full(site);
    let actual = out
        .resolved
        .iter()
        .map(|r| (r.target.file.as_str(), r.kind, r.confidence))
        .collect::<Vec<_>>();
    assert_eq!(actual, expected, "{call}");
    for r in &out.resolved {
        assert_eq!(r.target.name, call.rsplit("::").next().unwrap());
        assert_eq!((r.target.start_line, r.target.end_line), (1, 1));
    }
    assert_eq!(out.drop, drop, "{call}");
}

#[test]
fn std_review_unrelated_crate_keeps_std_impl() {
    let cg = graph(&[
        ("app/src/lib.rs", "use std::io; trait SinkError { fn error_message(); } impl SinkError for io::Error { fn error_message() {} } fn invoke() { io::Error::error_message(); }"),
        ("other/src/lib.rs", "pub struct Error;"),
    ]);
    check(
        &cg,
        "io::Error::error_message",
        &[("app/src/lib.rs", Kind::QualifiedOwner, Confidence::Exact)],
        None,
    );
}

#[test]
fn std_review_per_target_subset_never_promotes() {
    let cg = graph(&[
        (
            "app/src/lib.rs",
            "use std::io; fn invoke() { io::Error::from(1); }",
        ),
        (
            "good/src/lib.rs",
            "use std::io; impl From<u8> for io::Error { fn from(_: u8) -> Self { todo!() } }",
        ),
        (
            "bad/src/lib.rs",
            "pub struct Error; impl Error { pub fn from(_: u8) -> Self { Self } }",
        ),
    ]);
    check(
        &cg,
        "io::Error::from",
        &[(
            "good/src/lib.rs",
            Kind::QualifiedOwner,
            Confidence::NameOnly,
        )],
        None,
    );
}

#[test]
fn std_review_local_roots_keep_literal_import_and_constructor() {
    for root in ["std", "core", "alloc"] {
        for body in [
            format!("{root}::time::Instant::now();"),
            format!("use {root}::time::Instant; Instant::now();"),
            format!("let x = {root}::time::Instant::new(); x.read();"),
        ] {
            let source = format!("mod {root} {{ pub mod time {{ pub struct Instant; impl Instant {{ pub fn now() {{}} pub fn new() -> Self {{ Self }} pub fn read(&self) {{}} }} }} }} fn invoke() {{ {body} }}");
            let cg = graph(&[("src/lib.rs", &source)]);
            let call = if body.contains("read") {
                "read".to_string()
            } else if body.starts_with("use") {
                "Instant::now".to_string()
            } else {
                format!("{root}::time::Instant::now")
            };
            check(
                &cg,
                &call,
                &[(
                    "src/lib.rs",
                    if body.contains("read") {
                        Kind::ConstructorLocal
                    } else {
                        Kind::QualifiedOwner
                    },
                    Confidence::Exact,
                )],
                None,
            );
            if body.contains("read") {
                check(
                    &cg,
                    &format!("{root}::time::Instant::new"),
                    &[("src/lib.rs", Kind::QualifiedOwner, Confidence::Exact)],
                    None,
                );
            }
        }
    }
}

#[test]
fn std_review_renamed_alias_collisions_refuse() {
    for (path, alias, ctor) in [
        ("std::fs::OpenOptions", "Options", "new"),
        ("std::io::Error", "IoError", "new"),
    ] {
        let source = format!("mod mine {{ pub struct {alias}; impl {alias} {{ pub fn new() -> Self {{ Self }} pub fn read(&self) {{}} }} }} use {path} as {alias}; fn invoke() {{ let x = {alias}::{ctor}(); x.read(); }}");
        let cg = graph(&[("src/lib.rs", &source)]);
        check(
            &cg,
            &format!("{alias}::{ctor}"),
            &[],
            Some(DropReason::ImportExternal),
        );
        check(&cg, "read", &[], Some(DropReason::ExternalReceiver));
    }
}

#[test]
fn std_review_aliased_impl_only_keeps() {
    let cg = graph(&[("src/lib.rs", "use std::process::ExitCode as Code; struct X; impl From<X> for Code { fn from(_: X) -> Self { todo!() } } fn invoke() { Code::from(X); }")]);
    check(
        &cg,
        "Code::from",
        &[("src/lib.rs", Kind::QualifiedOwner, Confidence::Exact)],
        None,
    );
}

#[test]
fn std_review_annotation_precedes_constructor_refusal() {
    let cg = graph(&[("src/lib.rs", "mod other { pub trait Default {} } struct Local; impl Local { fn read(&self) {} } impl std::default::Default for Local { fn default() -> Self { Local } } fn invoke() { let x: Local = std::default::Default::default(); x.read(); }")]);
    let site = cg
        .calls
        .iter()
        .filter(|(f, _)| f.name == "invoke")
        .flat_map(|(_, s)| s)
        .find(|s| s.callee_name == "read")
        .unwrap();
    assert_eq!(
        site.receiver_outcome.as_ref().unwrap().recovery,
        prism::resolution::ReceiverRecovery::TypedLet
    );
    check(
        &cg,
        "read",
        &[("src/lib.rs", Kind::TypedParam, Confidence::Exact)],
        None,
    );
}

#[test]
fn std_review_generic_names_are_not_declarations() {
    for (owner, path, generic) in [
        (
            "Instant",
            "std::time::Instant",
            "struct Wrapper<T>(T); impl<Instant> Wrapper<Instant> { fn noise() {} }",
        ),
        ("Error", "std::io::Error", "fn f<Error>() {}"),
    ] {
        let source = format!("{generic} trait Tag {{ fn stamp(); }} impl Tag for {path} {{ fn stamp() {{}} }} fn invoke() {{ {path}::stamp(); }}");
        let cg = graph(&[("src/lib.rs", &source)]);
        check(
            &cg,
            &format!("{path}::stamp"),
            &[("src/lib.rs", Kind::QualifiedOwner, Confidence::Exact)],
            None,
        );
        assert!(source.contains(owner));
    }
}

#[test]
fn std_review_facade_gap_pins_main() {
    // Known gap F4: the probe stops at the imported local facade module.
    let cg = graph(&[("src/lib.rs", "mod mine { pub struct Instant; impl Instant { pub fn now() {} } } mod api { pub use std::time::Instant; } use api as facade; fn invoke() { facade::Instant::now(); }")]);
    check(
        &cg,
        "facade::Instant::now",
        &[("src/lib.rs", Kind::QualifiedOwner, Confidence::Exact)],
        None,
    );
}

#[test]
fn std_review_chained_import_gap_pins_main() {
    // Known gap W3b: a pending import selected by another import is not expanded.
    let cg = graph(&[("src/lib.rs", "mod mine { pub struct Error; impl Error { pub fn new() {} } } use std::io; use io::Error; fn invoke() { Error::new(); }")]);
    check(
        &cg,
        "Error::new",
        &[("src/lib.rs", Kind::QualifiedOwner, Confidence::Exact)],
        None,
    );
}

#[test]
fn std_review_external_glob_gap_pins_main() {
    // Known gap W3c: unexpanded external globs supply no declaration binding.
    let cg = graph(&[("src/lib.rs", "mod mine { pub struct Error; impl Error { pub fn new() {} } } use std::io::*; fn invoke() { Error::new(); }")]);
    check(
        &cg,
        "Error::new",
        &[("src/lib.rs", Kind::QualifiedOwner, Confidence::Exact)],
        None,
    );
}

#[test]
fn std_review_incomplete_keeps_main() {
    let mut cg = graph(&[("src/lib.rs", "mod mine { pub struct Instant; impl Instant { pub fn now() {} } } fn invoke() { std::time::Instant::now(); }")]);
    cg.scope_graph.as_mut().unwrap().complete = false;
    check(
        &cg,
        "std::time::Instant::now",
        &[("src/lib.rs", Kind::QualifiedOwner, Confidence::Exact)],
        None,
    );
}

#[test]
fn std_review_absolute_std_bypasses_local_root() {
    let cg = graph(&[("src/lib.rs", "mod std { pub mod time { pub struct Instant; impl Instant { pub fn now() {} } } } fn invoke() { ::std::time::Instant::now(); }")]);
    check(
        &cg,
        "::std::time::Instant::now",
        &[],
        Some(DropReason::ImportExternal),
    );
}

#[test]
fn std_review_all_external_roots_refuse() {
    for path in [
        "std::time::Instant",
        "core::cell::Instant",
        "alloc::vec::Instant",
    ] {
        let source = format!("mod mine {{ pub struct Instant; impl Instant {{ pub fn now() {{}} }} }} fn invoke() {{ {path}::now(); }}");
        let cg = graph(&[("src/lib.rs", &source)]);
        check(
            &cg,
            &format!("{path}::now"),
            &[],
            Some(DropReason::ImportExternal),
        );
    }
}

#[test]
fn std_review_2015_absolute_local_root_keeps_main() {
    // In 2015, leading :: and use paths start at the crate root.
    for body in [
        "::std::time::Instant::now();",
        "use std::time::Instant; Instant::now();",
    ] {
        let source = format!("mod std {{ pub mod time {{ pub struct Instant; impl Instant {{ pub fn now() {{}} }} }} }} fn invoke() {{ {body} }}");
        let cg = graph_edition(&[("src/lib.rs", &source)], 2015);
        let call = if body.starts_with("use") {
            "Instant::now"
        } else {
            "::std::time::Instant::now"
        };
        check(
            &cg,
            call,
            &[("src/lib.rs", Kind::QualifiedOwner, Confidence::Exact)],
            None,
        );
    }
}

const LOCAL_ENGINE: &str = "pub struct Engine; impl Engine { pub fn new() {} }";

#[test]
fn std_review_mixed_edition_keeps_absolute_local_root() {
    // Edition-2015 crate-root `mod core`/`mod std` is selected by `::core`; the
    // graph's single edition (2021 here) is not authoritative when mixed.
    for root in ["core", "std", "alloc"] {
        let source =
            format!("mod {root} {{ {LOCAL_ENGINE} }} fn invoke() {{ ::{root}::Engine::new(); }}");
        let cg = graph_with(&[("a/src/lib.rs", &source)], 2021, false, &[]);
        check(
            &cg,
            &format!("::{root}::Engine::new"),
            &[("a/src/lib.rs", Kind::QualifiedOwner, Confidence::Exact)],
            None,
        );
    }
}

#[test]
fn std_review_uniform_2021_absolute_roots_refuse() {
    for root in ["core", "std", "alloc"] {
        let source =
            format!("mod {root} {{ {LOCAL_ENGINE} }} fn invoke() {{ ::{root}::Engine::new(); }}");
        let cg = graph_with(&[("a/src/lib.rs", &source)], 2021, true, &[]);
        check(
            &cg,
            &format!("::{root}::Engine::new"),
            &[],
            Some(DropReason::ImportExternal),
        );
    }
}

#[test]
fn std_review_extern_crate_self_is_not_external() {
    for body in [
        "selfcrate::m::Thing::new();",
        "use selfcrate::m::Thing; Thing::new();",
    ] {
        let source = format!("extern crate self as selfcrate; pub mod m {{ pub struct Thing; impl Thing {{ pub fn new() {{}} }} }} fn invoke() {{ {body} }}");
        let cg = graph(&[("a/src/lib.rs", &source)]);
        let call = if body.starts_with("use") {
            "Thing::new"
        } else {
            "selfcrate::m::Thing::new"
        };
        check(
            &cg,
            call,
            &[("a/src/lib.rs", Kind::QualifiedOwner, Confidence::Exact)],
            None,
        );
    }
}

#[test]
fn std_review_poisoned_std_root_keeps_main() {
    // An unexpanded external glob could supply `std`, so the lookup is
    // Poisoned and gives no external proof. Without the glob the call refuses.
    let look_alike = "mod mine { pub struct Instant; impl Instant { pub fn now() {} } }";
    let call = "std::time::Instant::now";
    let cg = graph(&[(
        "a/src/lib.rs",
        &format!("{look_alike} use ext::*; fn invoke() {{ {call}(); }}"),
    )]);
    check(
        &cg,
        call,
        &[("a/src/lib.rs", Kind::QualifiedOwner, Confidence::Exact)],
        None,
    );
    let cg = graph(&[(
        "a/src/lib.rs",
        &format!("{look_alike} fn invoke() {{ {call}(); }}"),
    )]);
    check(&cg, call, &[], Some(DropReason::ImportExternal));
}

#[test]
fn std_review_multi_root_target_file_keeps_main() {
    // A file attributed to two crate roots has no single owning crate, so the
    // target is kept; attributed to one root it is refused.
    let shared = "pub struct Instant; impl Instant { pub fn now() {} }";
    let app = (
        "app/src/lib.rs",
        "fn invoke() { std::time::Instant::now(); }",
    );
    let call = "std::time::Instant::now";
    let one = graph_with(
        &[app, ("a/src/lib.rs", "mod x;"), ("a/src/x.rs", shared)],
        2021,
        true,
        &[],
    );
    check(&one, call, &[], Some(DropReason::ImportExternal));
    let two = graph_with(
        &[
            app,
            ("a/src/lib.rs", "mod x;"),
            ("a/src/other.rs", "mod x;"),
            ("a/src/x.rs", shared),
        ],
        2021,
        true,
        &["a/src/other.rs"],
    );
    check(
        &two,
        call,
        &[("a/src/x.rs", Kind::QualifiedOwner, Confidence::Exact)],
        None,
    );
}
