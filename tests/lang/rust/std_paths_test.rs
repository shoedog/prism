use prism::ast::ParsedFile;
use prism::call_graph::{CallGraph, ScopeGraphBuildInputs};
use prism::languages::Language::Rust;
use prism::resolution::{DropReason, ResolutionConfidence, ResolutionKind};
use std::collections::BTreeMap;

const OWNERS: &str = r#"
mod fs {
    pub struct OpenOptions;
    impl OpenOptions {
        pub fn new() -> Self { Self }
        pub fn read(&mut self, _: bool) {}
    }
}
mod mine {
    pub struct Instant; impl Instant { pub fn now() {} }
    pub struct Error; impl Error { pub fn new() {} }
    pub struct Cell; impl Cell { pub fn new() {} }
    pub struct Vec; impl Vec { pub fn new() {} }
    pub struct AtomicUsize; impl AtomicUsize { pub fn new() {} }
}
mod sync {
    pub struct RwLock; impl RwLock { pub fn new() {} }
}
"#;

pub(super) fn build(body: &str, complete: bool) -> CallGraph {
    let source = format!("{OWNERS}\n{body}");
    let files = BTreeMap::from([(
        "src/lib.rs".into(),
        ParsedFile::parse("src/lib.rs", &source, Rust).unwrap(),
    )]);
    if complete {
        let mut inputs = ScopeGraphBuildInputs::from_files_convention(&files);
        inputs.cfg.edition = 2021;
        CallGraph::build_with_scope_graph_inputs(&files, Some(&inputs))
    } else {
        CallGraph::build_with_scope_graph_inputs(&files, None)
    }
}

// Keep separate named callers so every site gets its own target/kind/grade oracle.
#[test]
fn std_paths_refuse_owner_collisions() {
    let variants = [
        ("direct", "std::fs::OpenOptions::new()"),
        ("absolute", "::std::fs::OpenOptions::new()"),
        ("raw_std", "r#std::fs::OpenOptions::new()"),
        ("spaced_std", "std ::fs::OpenOptions::new()"),
        ("core", "core::cell::Cell::new()"),
        ("alloc", "alloc::vec::Vec::new()"),
        (
            "type_import",
            "{ use std::fs::OpenOptions; OpenOptions::new() }",
        ),
        ("module_import", "{ use std::io; io::Error::new() }"),
        ("spaced_import", "{ use std::io; io ::Error::new() }"),
        ("raw_use", "{ use r#std::io; io::Error::new() }"),
        (
            "group_module",
            "{ use std::{io, sync::atomic::AtomicUsize}; io::Error::new() }",
        ),
        (
            "group_type",
            "{ use std::{io, sync::atomic::AtomicUsize}; AtomicUsize::new() }",
        ),
        (
            "renamed",
            "{ use std::io::Error as IoError; IoError::new() }",
        ),
        ("module_self", "{ use std::io::{self}; io::Error::new() }"),
        ("sync_import", "{ use std::sync; sync::RwLock::new() }"),
        (
            "external",
            "{ extern crate outside; outside::fs::OpenOptions::new() }",
        ),
        (
            "external_import",
            "{ extern crate outside; use outside::fs::OpenOptions; OpenOptions::new() }",
        ),
    ];
    let mut failures = Vec::new();
    for complete in [false, true] {
        for (label, expression) in variants {
            // Import rules require the existing complete scope model.
            if !complete
                && !matches!(
                    label,
                    "direct" | "absolute" | "raw_std" | "spaced_std" | "core" | "alloc"
                )
            {
                continue;
            }
            let cg = build(&format!("fn invoke() {{ {expression}; }}"), complete);
            let site = cg
                .calls
                .iter()
                .find(|(f, _)| f.name == "invoke")
                .unwrap()
                .1
                .iter()
                .find(|s| s.callee_name.contains("::"))
                .unwrap();
            let outcome = cg.resolve_call_site_full(site);
            let targets: Vec<_> = outcome
                .resolved
                .iter()
                .map(|c| (c.target.name.as_str(), c.kind, c.confidence))
                .collect();
            let expected = if complete {
                vec![]
            } else {
                vec![(
                    "new",
                    ResolutionKind::QualifiedOwner,
                    ResolutionConfidence::Exact,
                )]
            };
            let drop = complete.then_some(DropReason::ImportExternal);
            if targets != expected || outcome.drop != drop {
                failures.push(format!(
                    "{label} complete={complete}: {targets:?}, drop={:?}",
                    outcome.drop
                ));
            }
        }
    }
    assert!(
        failures.is_empty(),
        "external paths bound in-repo: {failures:#?}"
    );
}

#[test]
fn std_constructor_locals_refuse_owner_collisions() {
    let variants = [
        "let mut o = std::fs::OpenOptions::new(); o.read(true);",
        "use std::fs::OpenOptions; let mut o = OpenOptions::new(); o.read(true);",
        "use std::fs; let mut o = fs::OpenOptions::new(); o.read(true);",
        "use std::fs::OpenOptions as Options; let mut o = Options::new(); o.read(true);",
        "let mut o = ::std::fs::OpenOptions::new(); o.read(true);",
        "{ use std::fs::OpenOptions; let mut o = OpenOptions::new(); { struct OpenOptions; o.read(true); } }",
    ];
    let mut failures = Vec::new();
    for body in variants {
        let cg = build(&format!("fn invoke() {{ {body} }}"), true);
        let site = cg
            .calls
            .iter()
            .find(|(f, _)| f.name == "invoke")
            .unwrap()
            .1
            .iter()
            .find(|s| s.callee_name == "read")
            .unwrap();
        let outcome = cg.resolve_call_site_full(site);
        let targets: Vec<_> = outcome
            .resolved
            .iter()
            .map(|c| (&c.target.name, c.kind, c.confidence))
            .collect();
        if !targets.is_empty() || outcome.drop != Some(DropReason::ExternalReceiver) {
            failures.push(format!("{body}: {targets:?}, drop={:?}", outcome.drop));
        }
    }
    assert!(
        failures.is_empty(),
        "external constructors typed in-repo: {failures:#?}"
    );
}

#[test]
fn std_direct_constructor_without_scope_graph() {
    let cg = build(
        "fn invoke() { let mut o = std::fs::OpenOptions::new(); o.read(true); }",
        false,
    );
    let site = cg
        .calls
        .iter()
        .find(|(f, _)| f.name == "invoke")
        .unwrap()
        .1
        .iter()
        .find(|s| s.callee_name == "read")
        .unwrap();
    let outcome = cg.resolve_call_site_full(site);
    let targets: Vec<_> = outcome
        .resolved
        .iter()
        .map(|c| {
            (
                c.target.file.as_str(),
                c.target.name.as_str(),
                c.target.start_line,
                c.kind,
                c.confidence,
            )
        })
        .collect();
    assert_eq!(
        targets,
        vec![(
            "src/lib.rs",
            "read",
            6,
            ResolutionKind::ConstructorLocal,
            ResolutionConfidence::Exact
        )]
    );
    assert_eq!(outcome.drop, None);
}

#[test]
fn std_path_keep_cases() {
    let variants = [
        "fn invoke() { crate::fs::OpenOptions::new(); }",
        "fn invoke() { use crate::fs::OpenOptions; OpenOptions::new(); }",
        "fn invoke() { self::fs::OpenOptions::new(); }",
        "mod child { fn invoke() { super::fs::OpenOptions::new(); } }",
        "mod io { pub use crate::mine::Error; } fn invoke() { io::Error::new(); }",
        "use std::io; fn invoke() { mod io { pub use crate::mine::Error; } io::Error::new(); }",
        "use std::time::Instant; fn invoke() { struct Instant; impl Instant { fn now() {} } Instant::now(); }",
    ];
    for body in variants {
        let cg = build(body, true);
        let site = cg
            .calls
            .iter()
            .find(|(f, _)| f.name == "invoke")
            .unwrap()
            .1
            .iter()
            .find(|s| s.callee_name.contains("::"))
            .unwrap();
        let outcome = cg.resolve_call_site_full(site);
        let targets: Vec<_> = outcome
            .resolved
            .iter()
            .map(|c| {
                (
                    c.target.file.as_str(),
                    c.target.name.as_str(),
                    c.kind,
                    c.confidence,
                )
            })
            .collect();
        let mut target_lines: Vec<_> = outcome
            .resolved
            .iter()
            .map(|c| c.target.start_line)
            .collect();
        target_lines.sort_unstable();
        let expected_lines = if body.contains("Instant") {
            vec![10, 20]
        } else if body.contains("Error") {
            vec![11]
        } else {
            vec![5]
        };
        assert_eq!(target_lines, expected_lines, "{body}");
        let expected = if body.contains("Instant") {
            vec![
                (
                    "src/lib.rs",
                    "now",
                    ResolutionKind::QualifiedOwner,
                    ResolutionConfidence::NameOnly
                );
                2
            ]
        } else {
            vec![(
                "src/lib.rs",
                "new",
                ResolutionKind::QualifiedOwner,
                ResolutionConfidence::Exact,
            )]
        };
        assert_eq!(targets, expected, "{body}");
    }
}

#[test]
fn std_type_trait_impl_main_outcome() {
    let cg = build(
        r#"
use std::process::ExitCode;
struct X;
impl From<X> for ExitCode { fn from(_: X) -> Self { todo!() } }
fn invoke() { ExitCode::from(X); }
"#,
        true,
    );
    let site = cg
        .calls
        .iter()
        .find(|(f, _)| f.name == "invoke")
        .unwrap()
        .1
        .iter()
        .next()
        .unwrap();
    let outcome = cg.resolve_call_site_full(site);
    let targets: Vec<_> = outcome
        .resolved
        .iter()
        .map(|c| {
            (
                c.target.file.as_str(),
                c.target.name.as_str(),
                c.kind,
                c.confidence,
            )
        })
        .collect();
    assert_eq!(
        targets,
        vec![(
            "src/lib.rs",
            "from",
            ResolutionKind::QualifiedOwner,
            ResolutionConfidence::Exact
        )]
    );
    assert_eq!(outcome.drop, None);
}

#[test]
fn std_free_function_paths_refuse_stem_collisions() {
    for expression in [
        "std::ptr::eq()",
        "::core::ptr::eq()",
        "{ use std::ptr; ptr::eq() }",
    ] {
        for complete in [false, true] {
            if !complete && expression.starts_with('{') {
                continue;
            }
            let mut files = BTreeMap::from([
                (
                    "src/lib.rs".into(),
                    ParsedFile::parse(
                        "src/lib.rs",
                        &format!("fn invoke() {{ {expression}; }}"),
                        Rust,
                    )
                    .unwrap(),
                ),
                (
                    "src/ptr.rs".into(),
                    ParsedFile::parse("src/ptr.rs", "pub fn eq() {}", Rust).unwrap(),
                ),
            ]);
            // Complete scope construction needs an actual module declaration.
            if complete {
                files.insert(
                    "src/lib.rs".into(),
                    ParsedFile::parse(
                        "src/lib.rs",
                        &format!("mod ptr; fn invoke() {{ {expression}; }}"),
                        Rust,
                    )
                    .unwrap(),
                );
            }
            let inputs = ScopeGraphBuildInputs::from_files_convention(&files);
            let cg = CallGraph::build_with_scope_graph_inputs(&files, complete.then_some(&inputs));
            let site = cg
                .calls
                .iter()
                .find(|(f, _)| f.name == "invoke")
                .unwrap()
                .1
                .iter()
                .next()
                .unwrap();
            let outcome = cg.resolve_call_site_full(site);
            let targets: Vec<_> = outcome
                .resolved
                .iter()
                .map(|c| {
                    (
                        c.target.file.as_str(),
                        c.target.name.as_str(),
                        c.target.start_line,
                        c.kind,
                        c.confidence,
                    )
                })
                .collect();
            let expected = if complete {
                vec![]
            } else {
                vec![(
                    "src/ptr.rs",
                    "eq",
                    1,
                    ResolutionKind::StemSingle,
                    ResolutionConfidence::Exact,
                )]
            };
            assert_eq!(targets, expected, "{expression}");
            assert_eq!(outcome.drop, complete.then_some(DropReason::ImportExternal));
        }
    }
}

#[test]
fn std_names_do_not_veto_self_or_in_repo_crates() {
    let cg = build(
        "struct Local; impl Local { fn new() -> Self { Self } fn invoke() { Self::new(); } }",
        true,
    );
    let site = cg
        .calls
        .iter()
        .find(|(f, _)| f.name == "invoke")
        .unwrap()
        .1
        .iter()
        .next()
        .unwrap();
    let resolved = cg.resolve_call_site(site);
    assert_eq!(resolved.len(), 1);
    assert_eq!(
        (
            resolved[0].target.file.as_str(),
            resolved[0].target.name.as_str(),
            resolved[0].kind,
            resolved[0].confidence
        ),
        (
            "src/lib.rs",
            "new",
            ResolutionKind::SelfReceiver,
            ResolutionConfidence::Exact
        )
    );

    let files = BTreeMap::from([
        (
            "app/src/lib.rs".into(),
            ParsedFile::parse("app/src/lib.rs", "fn invoke() { io::Error::new(); }", Rust).unwrap(),
        ),
        (
            "io/src/lib.rs".into(),
            ParsedFile::parse(
                "io/src/lib.rs",
                "pub struct Error; impl Error { pub fn new() {} }",
                Rust,
            )
            .unwrap(),
        ),
    ]);
    let mut inputs = ScopeGraphBuildInputs::from_files_convention(&files);
    inputs.cfg.edition = 2021;
    inputs.cfg.workspace_members = vec!["app".into(), "io".into()];
    inputs.cfg.crate_roots = files.keys().cloned().collect();
    inputs.cfg.member_in_repo_deps =
        BTreeMap::from([("app".into(), BTreeMap::from([("io".into(), "io".into())]))]);
    let cg = CallGraph::build_with_scope_graph_inputs(&files, Some(&inputs));
    let site = cg
        .calls
        .iter()
        .find(|(f, _)| f.name == "invoke")
        .unwrap()
        .1
        .iter()
        .next()
        .unwrap();
    let resolved = cg.resolve_call_site(site);
    assert_eq!(resolved.len(), 1);
    assert_eq!(
        (
            resolved[0].target.file.as_str(),
            resolved[0].target.name.as_str(),
            resolved[0].kind,
            resolved[0].confidence
        ),
        (
            "io/src/lib.rs",
            "new",
            ResolutionKind::QualifiedOwner,
            ResolutionConfidence::Exact
        )
    );
}
