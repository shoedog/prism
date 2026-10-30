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
fn nonascii_other_variant_refuses() {
    let cg = graph(&[(
        "src/lib.rs",
        "enum Other { K } struct T; impl T { fn K() {} fn caller() { Self::K(); } }",
    )]);
    check(&cg, "caller", "Self::K", None);
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

#[test]
fn identifier_nominal_nfc_refuses() {
    let cg = graph(&[(
        "src/lib.rs",
        include_str!("../fixtures/rust-self-closed/nominal_nfc/src/lib.rs"),
    )]);
    check(&cg, "a", "Self::b", None);
}
#[test]
fn identifier_reference_only_refuses() {
    let cg = graph(&[(
        "src/lib.rs",
        "struct K; fn reference() { let _ = K; } impl K { fn b() {} fn caller() { Self::b(); } }",
    )]);
    check(&cg, "caller", "Self::b", None);
}
#[test]
fn identifier_other_file_refuses() {
    let mut cg = graph(&[
        (
            "src/lib.rs",
            "struct T; impl T { fn b() {} fn caller() { Self::b(); } }",
        ),
        ("unreachable.rs", "const café: u8 = 1;"),
    ]);
    check(&cg, "caller", "Self::b", None);
    // Fold6 preserves main's legacy outcome when no scope proof is available.
    cg.scope_graph = None;
    check(&cg, "caller", "Self::b", Some(("src/lib.rs", 1, 1, "b")));
}
#[test]
fn identifier_macro_tokens_refuse() {
    let cg = graph(&[("src/lib.rs", "macro_rules! discard { ($($t:tt)*) => {}; } discard!(K); struct T; impl T { fn b() {} fn caller() { Self::b(); } }")]);
    check(&cg, "caller", "Self::b", None);
}
#[test]
fn identifier_token_kinds_refuse() {
    for text in [
        "struct É;",
        "struct Field { é: u8 }",
        "fn field(f: Field) { let _ = f.é; }",
        "fn shorthand() { let Field { é } = todo!(); }",
        "fn raw() { let r#é = 1; }",
    ] {
        let source =
            format!("struct T; impl T {{ fn b() {{}} fn caller() {{ Self::b(); }} }} {text}");
        let cg = graph(&[("src/lib.rs", &source)]);
        check(&cg, "caller", "Self::b", None);
    }
}
#[test]
fn identifier_unicode_literals_allow() {
    let cg = graph(&[("src/lib.rs", "const S: &str = \"Ké\"; const R: &str = r#\"é\"#; const C: char = 'é'; struct T; impl T { fn b() {} fn caller() { Self::b(); } }")]);
    check(&cg, "caller", "Self::b", Some(("src/lib.rs", 1, 1, "b")));
}
#[test]
fn identifier_unicode_comments_allow() {
    let cg = graph(&[("src/lib.rs", "// Ké\n/// café\n/** K */ struct T; /* é */ impl T { fn b() {} fn caller() { Self::b(); } }")]);
    check(&cg, "caller", "Self::b", Some(("src/lib.rs", 3, 3, "b")));
}

#[test]
fn workspace_dependency_visibility_controls() {
    for (with_dependency, public_target, expected) in [
        (true, false, None),
        (true, true, Some(("a/src/lib.rs", 2, 2, "b"))),
        (false, true, None),
    ] {
        let root = tempfile::tempdir().unwrap();
        for (relative, source) in [
            (
                "Cargo.toml",
                include_str!("../fixtures/rust-self-closed/p16_ws_pubcrate/Cargo.toml"),
            ),
            (
                "a/Cargo.toml",
                include_str!("../fixtures/rust-self-closed/p16_ws_pubcrate/a/Cargo.toml"),
            ),
            (
                "b/Cargo.toml",
                include_str!("../fixtures/rust-self-closed/p16_ws_pubcrate/b/Cargo.toml"),
            ),
            (
                "a/src/lib.rs",
                include_str!("../fixtures/rust-self-closed/p16_ws_pubcrate/a/src/lib.rs"),
            ),
            (
                "b/src/lib.rs",
                include_str!("../fixtures/rust-self-closed/p16_ws_pubcrate/b/src/lib.rs"),
            ),
        ] {
            let source = match relative {
                "a/src/lib.rs" if public_target => source.replace("pub(crate) fn b", "pub fn b"),
                "b/src/lib.rs" if public_target => source.replace("fn a() -> u8", "fn a() -> bool"),
                "b/Cargo.toml" if !with_dependency => {
                    source.split_once("[dependencies]").unwrap().0.to_string()
                }
                _ => source.to_string(),
            };
            let path = root.path().join(relative);
            std::fs::create_dir_all(path.parent().unwrap()).unwrap();
            std::fs::write(path, source).unwrap();
        }
        let repo = prism::repo_loader::load_repo(root.path()).unwrap();
        let cg =
            CallGraph::build_with_scope_graph_inputs(&repo.files, repo.scope_graph_inputs.as_ref());
        assert!(cg.scope_graph.as_ref().unwrap().complete);
        check(&cg, "a", "Self::b", expected);
    }
}

fn identifier_workspace(dependency: bool) -> CallGraph {
    let root = tempfile::tempdir().unwrap();
    let manifest = if dependency {
        "[package]\nname='caller'\nedition='2021'\n[dependencies]\ndep={path='../dep'}\n"
    } else {
        "[package]\nname='caller'\nedition='2021'\n"
    };
    for (path, source) in [
        ("Cargo.toml", "[workspace]\nmembers=['caller','dep']\n"),
        ("caller/Cargo.toml", manifest),
        ("dep/Cargo.toml", "[package]\nname='dep'\nedition='2021'\n"),
        (
            "caller/src/lib.rs",
            if dependency {
                "use dep::*;\nstruct T; impl T { fn b() {} fn caller() { Self::b(); } }"
            } else {
                "// Unicode in comments is inert: café\nstruct T; impl T { fn b() {} fn caller() { Self::b(); } }"
            },
        ),
        ("dep/src/lib.rs", "pub const café: u8 = 1;"),
    ] {
        let path = root.path().join(path);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, source).unwrap();
    }
    let repo = prism::repo_loader::load_repo(root.path()).unwrap();
    CallGraph::build_with_scope_graph_inputs(&repo.files, repo.scope_graph_inputs.as_ref())
}

#[test]
fn identifier_workspace_dependency_refuses() {
    check(&identifier_workspace(true), "caller", "Self::b", None);
}

#[test]
fn identifier_unrelated_workspace_exact() {
    check(
        &identifier_workspace(false),
        "caller",
        "Self::b",
        Some(("caller/src/lib.rs", 2, 2, "b")),
    );
}

#[test]
fn identifier_manifest_free_fixture_is_separate() {
    let cg = graph(&[
        (
            "src/lib.rs",
            "struct T; impl T { fn b() {} fn caller() { Self::b(); } }",
        ),
        (
            "tests/fixtures/unicode/src/lib.rs",
            "struct U; const café:u8=1; impl U { fn f() {} fn unicode_caller() { Self::f(); } }",
        ),
    ]);
    let sg = cg.scope_graph.as_ref().unwrap();
    for path in ["src/lib.rs", "tests/fixtures/unicode/src/lib.rs"] {
        assert!(sg
            .scopes
            .values()
            .any(|s| s.kind == prism::name_resolution::types::ScopeKind::Root
                && s.extents.iter().any(|e| e.file == sg.file_paths[path])));
    }
    check(&cg, "caller", "Self::b", Some(("src/lib.rs", 1, 1, "b")));
    check(&cg, "unicode_caller", "Self::f", None);
}

#[test]
fn identifier_main_legacy_parity() {
    // Minimized ast_callback_identity.rs shape: an unmodeled method file,
    // with a recursive Self call inside a closure. Main's owner lookup is exact.
    let cg = graph(&[
        ("src/lib.rs", "const café:u8=1;"),
        (
            "src/ast_callback_identity.rs",
            "struct T; impl T {\nfn f() { let _ = || Self::f(); }\n}",
        ),
    ]);
    let sg = cg.scope_graph.as_ref().unwrap();
    let site = cg
        .calls
        .values()
        .flatten()
        .find(|s| s.callee_name == "Self::f")
        .unwrap();
    assert!(prism::name_resolution::rust_populator::enclosing_scope(
        sg,
        sg.file_paths["src/ast_callback_identity.rs"],
        site.start_byte
    )
    .is_none());
    check(
        &cg,
        "f",
        "Self::f",
        Some(("src/ast_callback_identity.rs", 2, 2, "f")),
    );
}

#[test]
fn identifier_resolved_cross_crate_use_refuses() {
    for import in ["extern crate dep; use dep::*;", "extern crate dep;"] {
        let source =
            format!("{import} struct T; impl T {{ fn b() {{}} fn caller() {{ Self::b(); }} }}");
        let cg = graph(&[
            ("caller/src/lib.rs", &source),
            ("dep/src/lib.rs", "pub const café:u8=1;"),
        ]);
        assert!(cg
            .scope_graph
            .as_ref()
            .unwrap()
            .crate_deps_by_root
            .is_empty());
        check(&cg, "caller", "Self::b", None);
    }
}

#[test]
fn identifier_crate_guard_roundtrip() {
    for dependency in [false, true] {
        let cg = identifier_workspace(dependency);
        let expected = (!dependency).then_some(("caller/src/lib.rs", 2, 2, "b"));
        check(&cg, "caller", "Self::b", expected);
        let bytes = bincode::serialize(&cg).unwrap();
        let restored: CallGraph = bincode::deserialize(&bytes).unwrap();
        check(&restored, "caller", "Self::b", expected);
    }
}

#[test]
fn identifier_transitive_import_refuses() {
    let cg = graph(&[
        ("caller/src/lib.rs", "extern crate bridge; use bridge::*; struct T; impl T { fn b() {} fn caller() { Self::b(); } }"),
        ("bridge/src/lib.rs", "extern crate dep; pub use dep::*;"),
        ("dep/src/lib.rs", "pub const café:u8=1;"),
    ]);
    check(&cg, "caller", "Self::b", None);
}

#[test]
fn identifier_uncertain_inrepo_import_refuses() {
    let cg = graph(&[
        (
            "caller/src/lib.rs",
            "use dep::*; struct T; impl T { fn b() {} fn caller() { Self::b(); } }",
        ),
        ("dep/src/lib.rs", "pub const café:u8=1;"),
    ]);
    // No extern crate in edition2015: import resolution cannot establish the
    // edge, but the in-repo crate name is a possible dependency, so refuse.
    check(&cg, "caller", "Self::b", None);
}
