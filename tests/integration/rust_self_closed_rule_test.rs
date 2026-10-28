use prism::{
    ast::ParsedFile,
    call_graph::{CallGraph, FunctionId},
    languages::Language,
    resolution::{DropReason, ResolutionConfidence, ResolutionKind},
};
use std::collections::BTreeMap;
fn graph(files: &[(&str, &str)]) -> CallGraph {
    let files = files
        .iter()
        .map(|(p, s)| {
            (
                (*p).to_string(),
                ParsedFile::parse(p, s, Language::Rust).unwrap(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let cg = CallGraph::build(&files);
    assert!(cg.scope_graph.as_ref().unwrap().complete);
    cg
}
fn check(cg: &CallGraph, caller: &str, callee: &str, target: Option<(&str, usize, usize, &str)>) {
    let sites = cg
        .calls
        .values()
        .flatten()
        .filter(|s| s.caller.name == caller && s.callee_name == callee)
        .collect::<Vec<_>>();
    assert_eq!(sites.len(), 1);
    let out = cg.resolve_call_site_full(sites[0]);
    let expected = target
        .map(|(file, start_line, end_line, name)| FunctionId {
            file: file.into(),
            start_line,
            end_line,
            name: name.into(),
        })
        .into_iter()
        .collect::<Vec<_>>();
    assert_eq!(
        out.resolved
            .iter()
            .map(|r| r.target.clone())
            .collect::<Vec<_>>(),
        expected,
        "{out:?}"
    );
    if target.is_some() {
        assert_eq!(out.drop, None);
        assert_eq!(
            (out.resolved[0].confidence, out.resolved[0].kind),
            (ResolutionConfidence::Exact, ResolutionKind::SelfReceiver)
        );
    } else {
        assert_eq!(out.drop, Some(DropReason::UnknownName));
    }
}

#[test]
fn alias_hidden_inherent() {
    let cg = graph(&[(
        "src/lib.rs",
        include_str!("../fixtures/rust-self-closed/alias_hidden_inherent/src/lib.rs"),
    )]);
    check(&cg, "caller", "Self::b", None);
}

#[test]
fn alias_ambiguity() {
    let cg = graph(&[(
        "src/lib.rs",
        include_str!("../fixtures/rust-self-closed/alias_ambiguity/src/lib.rs"),
    )]);
    check(&cg, "caller", "Self::b", None);
}

#[test]
fn inrepo_blanket() {
    let cg = graph(&[(
        "src/lib.rs",
        include_str!("../fixtures/rust-self-closed/inrepo_blanket/src/lib.rs"),
    )]);
    check(&cg, "convert", "Self::convert", None);
}

#[test]
fn same_impl_duplicate() {
    let cg = graph(&[(
        "src/lib.rs",
        include_str!("../fixtures/rust-self-closed/same_impl_duplicate/src/lib.rs"),
    )]);
    check(&cg, "caller", "Self::b", None);
}

#[test]
fn q2_alias_inherent() {
    let cg = graph(&[(
        "src/lib.rs",
        include_str!("../fixtures/rust-self-closed/q2_alias_inherent/src/lib.rs"),
    )]);
    check(&cg, "a", "Self::b", None);
}

#[test]
fn q2b_alias_mod() {
    let cg = graph(&[(
        "src/lib.rs",
        include_str!("../fixtures/rust-self-closed/q2b_alias_mod/src/lib.rs"),
    )]);
    check(&cg, "a", "Self::b", None);
}

#[test]
fn q5_generic_alias_inherent() {
    let cg = graph(&[(
        "src/lib.rs",
        include_str!("../fixtures/rust-self-closed/q5_generic_alias_inherent/src/lib.rs"),
    )]);
    check(&cg, "a", "Self::b", None);
}

#[test]
fn q7_private_inherent_blanket() {
    let cg = graph(&[(
        "src/lib.rs",
        include_str!("../fixtures/rust-self-closed/q7_private_inherent_blanket/src/lib.rs"),
    )]);
    check(&cg, "caller", "Self::b", None);
}

#[test]
fn q7c_private_inherent_trait_caller() {
    let cg = graph(&[(
        "src/lib.rs",
        include_str!("../fixtures/rust-self-closed/q7c_private_inherent_trait_caller/src/lib.rs"),
    )]);
    check(&cg, "a", "Self::b", None);
}

#[test]
fn q7b_private_inherent_derive() {
    let cg = graph(&[(
        "src/lib.rs",
        include_str!("../fixtures/rust-self-closed/q7b_private_inherent_derive/src/lib.rs"),
    )]);
    check(&cg, "caller", "Self::default", None);
}

#[test]
fn q9_variant_method() {
    let cg = graph(&[(
        "src/lib.rs",
        include_str!("../fixtures/rust-self-closed/q9_variant_method/src/lib.rs"),
    )]);
    check(&cg, "caller", "Self::V", None);
}

#[test]
fn trait_caller_to_inherent() {
    let cg = graph(&[(
        "src/lib.rs",
        "struct T; trait Tr { fn caller(); }
impl T { fn b() {} }
impl Tr for T { fn caller() { Self::b(); } }",
    )]);
    check(&cg, "caller", "Self::b", Some(("src/lib.rs", 2, 2, "b")));
}
#[test]
fn private_ancestor_inline() {
    let cg = graph(&[(
        "src/lib.rs",
        "struct T; impl T { fn b() {} }
mod child { use super::T; impl T { fn caller() { Self::b(); } } }",
    )]);
    check(&cg, "caller", "Self::b", Some(("src/lib.rs", 1, 1, "b")));
}
#[test]
fn private_ancestor_file() {
    let cg = graph(&[
        (
            "src/lib.rs",
            "struct T; impl T { fn b() {} }
mod child;",
        ),
        (
            "src/child.rs",
            "use super::T; impl T { fn caller() { Self::b(); } }",
        ),
    ]);
    check(&cg, "caller", "Self::b", Some(("src/lib.rs", 1, 1, "b")));
}
#[test]
fn crate_visible_sibling() {
    let cg = graph(&[(
        "src/lib.rs",
        "struct T;
mod a { use super::T; impl T { pub(crate) fn b() {} } }
mod b { use super::T; impl T { fn caller() { Self::b(); } } }",
    )]);
    check(&cg, "caller", "Self::b", Some(("src/lib.rs", 2, 2, "b")));
}
#[test]
fn public_sibling_file() {
    let cg = graph(&[
        ("src/lib.rs", "struct T; mod a; mod b;"),
        ("src/a.rs", "use super::T; impl T { pub fn b() {} }"),
        (
            "src/b.rs",
            "use super::T; impl T { fn caller() { Self::b(); } }",
        ),
    ]);
    check(&cg, "caller", "Self::b", Some(("src/a.rs", 1, 1, "b")));
}
#[test]
fn other_enum_variant_allows_struct() {
    let cg = graph(&[(
        "src/lib.rs",
        "enum Other { V(u8) } struct T;
impl T { fn V(_: u8) {} fn caller() { Self::V(1); } }",
    )]);
    check(&cg, "caller", "Self::V", Some(("src/lib.rs", 2, 2, "V")));
}
#[test]
fn same_named_enum_veto_is_conservative() {
    let cg = graph(&[(
        "src/lib.rs",
        "mod a { pub enum T { V(u8) } } struct T;
impl T { fn V(_: u8) {} fn caller() { Self::V(1); } }",
    )]);
    check(&cg, "caller", "Self::V", None);
}
#[test]
fn restricted_sibling_refuses_conservatively() {
    let cg = graph(&[(
        "src/lib.rs",
        "struct T;
mod a { use super::T; impl T { pub(super) fn b() {} } }
mod b { use super::T; impl T { fn caller() { Self::b(); } } }",
    )]);
    check(&cg, "caller", "Self::b", None);
}
#[test]
fn restricted_descendant_allows() {
    let cg = graph(&[(
        "src/lib.rs",
        "struct T; mod a { use super::T; impl T { pub(in crate) fn b() {} }
mod child { use super::T; impl T { fn caller() { Self::b(); } } } }",
    )]);
    check(&cg, "caller", "Self::b", Some(("src/lib.rs", 1, 1, "b")));
}

#[test]
fn raw_variant_name_refuses() {
    let cg = graph(&[(
        "src/lib.rs",
        "pub enum E { r#V(u8) }
impl E {
 pub fn V(_: u8) -> bool { true }
 pub fn caller() -> E { Self::V(1) }
}",
    )]);
    check(&cg, "caller", "Self::V", None);
}
#[test]
fn raw_enum_name_veto_is_conservative() {
    let cg = graph(&[(
        "src/lib.rs",
        "mod other { enum r#T { V(u8) } } struct T;
impl T { fn V(_: u8) {} fn caller() { Self::V(1); } }",
    )]);
    check(&cg, "caller", "Self::V", None);
}

#[test]
fn unicode_variant() {
    let cg = graph(&[(
        "src/lib.rs",
        include_str!("../fixtures/rust-self-closed/unicode_variant/src/lib.rs"),
    )]);
    check(&cg, "caller", "Self::É", None);
}

#[test]
fn unicode_reverse() {
    let cg = graph(&[(
        "src/lib.rs",
        include_str!("../fixtures/rust-self-closed/unicode_reverse/src/lib.rs"),
    )]);
    check(&cg, "caller", "Self::É", None);
}

#[test]
fn unicode_raw() {
    let cg = graph(&[(
        "src/lib.rs",
        include_str!("../fixtures/rust-self-closed/unicode_raw/src/lib.rs"),
    )]);
    check(&cg, "caller", "Self::r#É", None);
}

#[test]
fn unicode_kelvin() {
    let cg = graph(&[(
        "src/lib.rs",
        include_str!("../fixtures/rust-self-closed/unicode_kelvin/src/lib.rs"),
    )]);
    check(&cg, "caller", "Self::K", None);
}

#[test]
fn unicode_same() {
    let cg = graph(&[(
        "src/lib.rs",
        include_str!("../fixtures/rust-self-closed/unicode_same/src/lib.rs"),
    )]);
    check(&cg, "caller", "Self::É", None);
}

#[test]
fn p13_bare_dyn_2018() {
    let cg = graph(&[(
        "src/lib.rs",
        include_str!("../fixtures/rust-self-closed/p13_bare_dyn_2018/src/lib.rs"),
    )]);
    check(&cg, "a", "Self::b", None);
}

#[test]
fn p14_io_error_2018() {
    let cg = graph(&[(
        "src/lib.rs",
        include_str!("../fixtures/rust-self-closed/p14_io_error_2018/src/lib.rs"),
    )]);
    check(&cg, "code", "Self::raw_os_error", None);
}

#[test]
fn p13f_dyn_same_impl_2021() {
    let cg = graph(&[(
        "src/lib.rs",
        include_str!("../fixtures/rust-self-closed/p13f_dyn_same_impl_2021/src/lib.rs"),
    )]);
    check(&cg, "caller", "Self::b", Some(("src/lib.rs", 2, 2, "b")));
}

#[test]
fn p16_ws_pubcrate() {
    let cg = graph(&[
        (
            "a/src/lib.rs",
            include_str!("../fixtures/rust-self-closed/p16_ws_pubcrate/a/src/lib.rs"),
        ),
        (
            "b/src/lib.rs",
            include_str!("../fixtures/rust-self-closed/p16_ws_pubcrate/b/src/lib.rs"),
        ),
    ]);
    check(&cg, "a", "Self::b", None);
}

#[test]
fn nonascii_enum_owner_refuses() {
    let cg = graph(&[(
        "src/lib.rs",
        "enum É { V } impl É { fn b() {} fn caller() { Self::b(); } }",
    )]);
    check(&cg, "caller", "Self::b", None);
}
#[test]
fn nonascii_other_variant_allows() {
    let cg = graph(&[(
        "src/lib.rs",
        "enum Other { K } struct T; impl T { fn K() {} fn caller() { Self::K(); } }",
    )]);
    check(&cg, "caller", "Self::K", Some(("src/lib.rs", 1, 1, "K")));
}

#[test]
fn nonascii_enum_declaration_refuses() {
    let cg = graph(&[(
        "src/lib.rs",
        "mod other { pub enum K { V } }
struct K; impl K { fn b() {} fn caller() { Self::b(); } }",
    )]);
    check(&cg, "caller", "Self::b", None);
}
#[test]
fn unicode_kelvin_reverse() {
    let cg = graph(&[(
        "src/lib.rs",
        "pub enum E { K(u8) }
impl E {
 pub fn K(_: u8) -> bool { true }
 pub fn caller() -> E { Self::K(1) }
}",
    )]);
    check(&cg, "caller", "Self::K", None);
}
