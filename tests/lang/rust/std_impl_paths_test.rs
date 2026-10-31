use super::std_paths_test::build;
use prism::resolution::{DropReason, ResolutionConfidence, ResolutionKind};

// The impl index also contains external self types and external traits. Those
// names alone are not declaration evidence, and their existing edges must stay.
#[test]
fn std_impl_only_keep_cases() {
    let cases = [
        ("use std::process::ExitCode; struct X; impl From<X> for std::process::ExitCode { fn from(_: X) -> Self { todo!() } }", "ExitCode::from(X)", "from", 1, ResolutionConfidence::Exact),
        ("use std::borrow::Cow; struct X; impl From<X> for Cow<'_, [u8]> { fn from(_: X) -> Self { todo!() } }", "Cow::from(X)", "from", 1, ResolutionConfidence::Exact),
        ("use std::collections::BTreeMap; struct X; impl From<X> for BTreeMap<u8,u8> { fn from(_: X) -> Self { todo!() } }", "BTreeMap::from(X)", "from", 1, ResolutionConfidence::Exact),
        ("use std::num::NonZeroU8; struct X; impl From<X> for NonZeroU8 { fn from(_: X) -> Self { todo!() } }", "NonZeroU8::from(X)", "from", 1, ResolutionConfidence::Exact),
        ("use std::num::NonZeroU16; struct X; impl From<X> for NonZeroU16 { fn from(_: X) -> Self { todo!() } }", "NonZeroU16::from(X)", "from", 1, ResolutionConfidence::Exact),
        ("use std::fmt; struct A; struct B; impl fmt::Display for A { fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result { todo!() } }\nimpl fmt::Display for B { fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result { todo!() } }", "std::fmt::Display::fmt(&A, f)", "fmt", 2, ResolutionConfidence::NameOnly),
        ("use std::fmt; struct A; struct B; impl fmt::Debug for A { fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result { todo!() } }\nimpl fmt::Debug for B { fn fmt(&self, _: &mut fmt::Formatter<'_>) -> fmt::Result { todo!() } }", "std::fmt::Debug::fmt(&A, f)", "fmt", 2, ResolutionConfidence::NameOnly),
        ("use std::io; struct W; impl std::io::Write for W { fn write_fmt(&mut self, _: std::fmt::Arguments<'_>) -> io::Result<()> { todo!() } }", "io::Write::write_fmt(&mut W, args)", "write_fmt", 1, ResolutionConfidence::Exact),
    ];
    let mut failures = Vec::new();
    for (items, call, name, count, confidence) in cases {
        for complete in [false, true] {
            let cg = build(&format!("{items}\nfn invoke() {{ {call}; }}"), complete);
            let site = cg
                .calls
                .iter()
                .find(|(f, _)| f.name == "invoke")
                .unwrap()
                .1
                .iter()
                .find(|s| s.callee_name.ends_with(name))
                .unwrap();
            let out = cg.resolve_call_site_full(site);
            let actual: Vec<_> = out
                .resolved
                .iter()
                .map(|r| {
                    (
                        r.target.file.as_str(),
                        r.target.name.as_str(),
                        r.target.start_line,
                        r.target.end_line,
                        r.kind,
                        r.confidence,
                    )
                })
                .collect();
            let (expected, drop) = if count == 2 && complete {
                (vec![], Some(DropReason::UnknownName))
            } else {
                let kind = if count == 2 {
                    ResolutionKind::TraitCha
                } else {
                    ResolutionKind::QualifiedOwner
                };
                (
                    (20..20 + count)
                        .map(|line| ("src/lib.rs", name, line, line, kind, confidence))
                        .collect(),
                    None,
                )
            };
            if actual != expected || out.drop != drop {
                failures.push(format!(
                    "{call}: actual={actual:?}, expected={expected:?}, drop={:?}",
                    out.drop
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{failures:#?}");
}

#[test]
fn std_both_declaration_and_external_impl_refuse() {
    let cg = build("mod mine2 { pub struct ExitCode; } use std::process::ExitCode; struct X; impl From<X> for std::process::ExitCode { fn from(_: X) -> Self { todo!() } } fn invoke() { ExitCode::from(X); }", true);
    let site = cg
        .calls
        .iter()
        .find(|(f, _)| f.name == "invoke")
        .unwrap()
        .1
        .iter()
        .next()
        .unwrap();
    let out = cg.resolve_call_site_full(site);
    assert!(out.resolved.is_empty());
    assert_eq!(out.drop, Some(DropReason::ImportExternal));
}

#[test]
fn std_missing_or_incomplete_scope_facts_keep_main() {
    for route in ["absent", "incomplete", "missing_file", "missing_extent"] {
        let mut cg = build("fn invoke() { std::fs::OpenOptions::new(); }", true);
        match route {
            "absent" => cg.scope_graph = None,
            "incomplete" => cg.scope_graph.as_mut().unwrap().complete = false,
            "missing_file" => {
                cg.scope_graph.as_mut().unwrap().file_paths.clear();
            }
            _ => {
                cg.scope_graph.as_mut().unwrap().scopes.clear();
            }
        }
        let site = cg
            .calls
            .iter()
            .find(|(f, _)| f.name == "invoke")
            .unwrap()
            .1
            .iter()
            .next()
            .unwrap();
        let out = cg.resolve_call_site_full(site);
        let actual: Vec<_> = out
            .resolved
            .iter()
            .map(|r| {
                (
                    r.target.name.as_str(),
                    r.target.start_line,
                    r.kind,
                    r.confidence,
                )
            })
            .collect();
        assert_eq!(
            actual,
            vec![(
                "new",
                5,
                ResolutionKind::QualifiedOwner,
                ResolutionConfidence::Exact
            )],
            "{route}"
        );
        assert_eq!(out.drop, None, "{route}");
    }
}

#[test]
fn std_declaration_kinds_refuse() {
    for declaration in [
        "struct ExitCode;",
        "enum ExitCode { A }",
        "union ExitCode { a: u8 }",
        "trait ExitCode {}",
        "type ExitCode = u8;",
        "mod ExitCode {}",
    ] {
        let cg = build(&format!("mod mine2 {{ {declaration} }} use std::process::ExitCode; struct X; impl From<X> for std::process::ExitCode {{ fn from(_: X) -> Self {{ todo!() }} }} fn invoke() {{ ExitCode::from(X); }}"), true);
        let site = cg
            .calls
            .iter()
            .find(|(f, _)| f.name == "invoke")
            .unwrap()
            .1
            .iter()
            .next()
            .unwrap();
        let out = cg.resolve_call_site_full(site);
        assert!(out.resolved.is_empty(), "{declaration}");
        assert_eq!(out.drop, Some(DropReason::ImportExternal), "{declaration}");
    }
}

#[test]
fn std_constructor_impl_only_keeps_main() {
    for complete in [false, true] {
        let cg = build("use std::process::ExitCode; impl std::process::ExitCode { fn new() -> Self { todo!() } fn inspect(&self) {} } fn invoke() { let code = ExitCode::new(); code.inspect(); }", complete);
        let site = cg
            .calls
            .iter()
            .find(|(f, _)| f.name == "invoke")
            .unwrap()
            .1
            .iter()
            .find(|s| s.callee_name == "inspect")
            .unwrap();
        let out = cg.resolve_call_site_full(site);
        let actual: Vec<_> = out
            .resolved
            .iter()
            .map(|r| (r.target.name.as_str(), r.kind, r.confidence))
            .collect();
        assert_eq!(
            actual,
            vec![(
                "inspect",
                if complete {
                    ResolutionKind::QualifiedOwner
                } else {
                    ResolutionKind::ConstructorLocal
                },
                ResolutionConfidence::Exact
            )],
            "complete={complete}"
        );
        assert_eq!(out.drop, None);
    }
}

#[test]
fn std_known_external_impl_only_keeps_main() {
    let cg = build("extern crate outside; struct X; impl From<X> for outside::Code { fn from(_: X) -> Self { todo!() } } fn invoke() { outside::Code::from(X); }", true);
    let site = cg
        .calls
        .iter()
        .find(|(f, _)| f.name == "invoke")
        .unwrap()
        .1
        .iter()
        .next()
        .unwrap();
    let out = cg.resolve_call_site_full(site);
    let targets: Vec<_> = out
        .resolved
        .iter()
        .map(|r| {
            (
                r.target.file.as_str(),
                r.target.name.as_str(),
                r.target.start_line,
                r.target.end_line,
                r.kind,
                r.confidence,
            )
        })
        .collect();
    assert_eq!(
        targets,
        vec![(
            "src/lib.rs",
            "from",
            20,
            20,
            ResolutionKind::QualifiedOwner,
            ResolutionConfidence::Exact
        )]
    );
    assert_eq!(out.drop, None);
}
