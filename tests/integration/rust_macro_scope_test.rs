//! #765 PR D, E2: an expression macro of the std that expands to an expression (`env!`,
//! `matches!`, `concat!`, ...) can introduce no item or binding, so it must not poison the
//! lookup of the names after it; macros that can introduce names (`thread_local!`, `include!`,
//! user macros, item position) still do. RED rows fail on the pre-fix route; KEEP rows pin the
//! refusals. Each row kills a mutant of `mutants/rust-macro-scope.json`.
use prism::{
    ast::ParsedFile,
    call_graph::{CallGraph, CallSite, ScopeGraphBuildInputs},
    languages::Language,
    name_resolution::rust_populator::RustCrateConfig,
    resolution::ResolutionConfidence,
};
use std::collections::BTreeMap;

fn build(sources: &[(&str, &str)]) -> CallGraph {
    let files = sources
        .iter()
        .map(|(p, s)| {
            (
                (*p).to_string(),
                ParsedFile::parse(p, s, Language::Rust).unwrap(),
            )
        })
        .collect::<BTreeMap<_, _>>();
    let mut inputs = ScopeGraphBuildInputs::from_files_convention(&files);
    let mut cfg = RustCrateConfig::from_convention(&files);
    cfg.edition = 2021;
    inputs.cfg = cfg;
    let cg = CallGraph::build_with_scope_graph_inputs(&files, Some(&inputs));
    assert!(cg.scope_graph.as_ref().unwrap().complete);
    cg
}

fn site<'a>(cg: &'a CallGraph, file: &str, caller: &str, callee: &str) -> &'a CallSite {
    let sites = cg
        .calls
        .values()
        .flatten()
        .filter(|s| s.caller.file == file && s.caller.name == caller && s.callee_name == callee)
        .collect::<Vec<_>>();
    assert_eq!(sites.len(), 1, "{file}:{caller} {callee}: {sites:?}");
    sites[0]
}

/// The call binds exactly one Exact target: function `name` at `line` of `target_file`.
fn binds(cg: &CallGraph, file: &str, caller: &str, callee: &str, target: (&str, &str, usize)) {
    let o = cg.resolve_call_site_full(site(cg, file, caller, callee));
    assert_eq!(o.resolved.len(), 1, "{file}:{caller} {callee}: {o:?}");
    let r = &o.resolved[0];
    assert_eq!(
        (
            r.target.file.as_str(),
            r.target.name.as_str(),
            r.target.start_line,
            r.confidence
        ),
        (target.0, target.1, target.2, ResolutionConfidence::Exact),
        "{o:?}"
    );
}

/// The call binds nothing (a local, or a refusal).
fn unbound(cg: &CallGraph, file: &str, caller: &str, callee: &str) {
    let o = cg.resolve_call_site_full(site(cg, file, caller, callee));
    assert!(o.resolved.is_empty(), "{file}:{caller} {callee}: {o:?}");
}

// ── E2: expression macros that cannot bind names do not poison ──────────────

fn macro_fn(name: &str, stmt: &str) -> String {
    format!("pub fn {name}() {{\n    {stmt}\n    helper(1);\n}}\n")
}

fn macro_graph(extra: &str) -> CallGraph {
    let mut m = String::from("fn helper(n: u8) -> u8 { n }\n");
    m.push_str(extra);
    build(&[
        ("src/lib.rs", "pub fn top(n: u8) -> u8 { n }\nmod m;\n"),
        ("src/m.rs", &m),
    ])
}

const NO_NAME_MACROS: &[(&str, &str)] = &[
    (
        "e_env",
        "let _p = std::path::Path::new(env!(\"CARGO_MANIFEST_DIR\"));",
    ),
    ("e_option_env", "let _ = option_env!(\"X\");"),
    ("e_concat", "let _ = concat!(\"a\", \"b\");"),
    ("e_include_str", "let _ = include_str!(\"m.rs\");"),
    ("e_include_bytes", "let _ = include_bytes!(\"m.rs\");"),
    ("e_cfg", "let _ = cfg!(test);"),
    ("e_line", "let _ = line!();"),
    ("e_column", "let _ = column!();"),
    ("e_file", "let _ = file!();"),
    ("e_module_path", "let _ = module_path!();"),
    ("e_stringify", "let _ = stringify!(x);"),
    ("e_matches", "let _ = matches!(1u8, 1);"),
    ("e_std_matches", "let _ = std::matches!(1u8, 1);"),
    ("e_compile_error", "compile_error!(\"x\");"),
];

#[test]
fn m1_no_name_expression_macros_do_not_poison_later_calls() {
    let body: String = NO_NAME_MACROS.iter().map(|(n, s)| macro_fn(n, s)).collect();
    let cg = macro_graph(&body);
    for (name, _) in NO_NAME_MACROS {
        binds(&cg, "src/m.rs", name, "helper", ("src/m.rs", "helper", 1));
    }
}

#[test]
fn m2_a_macro_before_the_call_in_an_enclosing_block_no_longer_poisons_a_glob_name() {
    // `top` is reached through `use super::*`; `env!` earlier in the same fn.
    let m = "use super::*;\npub fn g() {\n    let _ = env!(\"X\");\n    top(1);\n}\n";
    let cg = build(&[
        ("src/lib.rs", "pub fn top(n: u8) -> u8 { n }\nmod m;\n"),
        ("src/m.rs", m),
    ]);
    binds(&cg, "src/m.rs", "g", "top", ("src/lib.rs", "top", 1));
}

#[test]
fn k5_a_user_macro_still_poisons() {
    let cg = macro_graph(&format!(
        "macro_rules! mk {{ () => {{}}; }}\n{}",
        macro_fn("c_user", "mk!();")
    ));
    unbound(&cg, "src/m.rs", "c_user", "helper");
}

#[test]
fn k6_thread_local_still_poisons() {
    let cg = macro_graph(&macro_fn("c_tls", "thread_local! { static T: u8 = 0; }"));
    unbound(&cg, "src/m.rs", "c_tls", "helper");
}

#[test]
fn k7_include_still_poisons() {
    let cg = macro_graph(&macro_fn("c_include", "include!(\"gen.rs\");"));
    unbound(&cg, "src/m.rs", "c_include", "helper");
}

#[test]
fn k8_a_foreign_qualifier_is_not_the_std_macro() {
    let cg = macro_graph(&macro_fn("c_qual", "let _ = my::matches!(1u8, 1);"));
    unbound(&cg, "src/m.rs", "c_qual", "helper");
}

#[test]
fn k9_a_user_macro_named_like_a_std_macro_withholds_the_exemption() {
    let cg = macro_graph(&format!(
        "macro_rules! matches {{ ($($t:tt)*) => {{ false }}; }}\n{}",
        macro_fn("c_shadow", "let _ = matches!(1u8, 1);")
    ));
    unbound(&cg, "src/m.rs", "c_shadow", "helper");
}

#[test]
fn k10_item_position_macro_still_poisons_a_glob_name() {
    let m = "use super::*;\nmacro_rules! mk2 { () => {}; }\nmk2! {}\npub fn c_item() {\n    top(1);\n}\n";
    let cg = build(&[
        ("src/lib.rs", "pub fn top(n: u8) -> u8 { n }\nmod m;\n"),
        ("src/m.rs", m),
    ]);
    unbound(&cg, "src/m.rs", "c_item", "top");
}

// ── Review folds (#765 PR D, W1): the spelling must be the std macro ────────

const HELPER_FN: &str = "fn helper(n: u8) -> u8 { n }\n";

/// `src/m.rs` = `uses`, a module-level `helper`, and `g` calling `body` then `helper(1)`.
fn import_graph(lib: &str, uses: &str, body: &str) -> CallGraph {
    let m = format!("{uses}{HELPER_FN}pub fn g() {{\n    {body}\n    helper(1);\n}}\n");
    build(&[("src/lib.rs", lib), ("src/m.rs", &m)])
}

const PLAIN_LIB: &str = "mod m;\n";

#[test]
fn w1_a_macro_imported_under_a_std_spelling_still_poisons() {
    // sol W1: `make!` emits `fn helper()`; `use crate::make as env;` makes `env!()` that
    // macro, so the later `helper()` is the emitted one, not the module-level fn.
    let lib = "#[macro_export]\nmacro_rules! make { () => { fn helper() -> u8 { 2 } }; }\nmod m;\n";
    let cg = import_graph(lib, "use crate::make as env;\n", "env!();");
    unbound(&cg, "src/m.rs", "g", "helper");
}

#[test]
fn w2_an_external_import_of_a_std_spelling_still_poisons() {
    let cg = import_graph(PLAIN_LIB, "use other::env;\n", "let _ = env!(\"X\");");
    unbound(&cg, "src/m.rs", "g", "helper");
}

#[test]
fn w3_a_block_scope_import_still_poisons() {
    let cg = import_graph(
        PLAIN_LIB,
        "",
        "use other::{x as y, concat};\n    let _ = concat!(\"a\");",
    );
    unbound(&cg, "src/m.rs", "g", "helper");
}

#[test]
fn w4_an_alias_of_a_transparent_macro_withholds_its_transparency() {
    // The pre-existing hole of `is_transparent_arg_macro`: `use crate::make as assert;`.
    let lib = "#[macro_export]\nmacro_rules! make { () => { fn helper() -> u8 { 2 } }; }\nmod m;\n";
    let cg = import_graph(lib, "use crate::make as assert;\n", "assert!();");
    unbound(&cg, "src/m.rs", "g", "helper");
}

#[test]
fn w5_an_external_glob_poisons_the_unqualified_spelling() {
    let cg = import_graph(PLAIN_LIB, "use ext::*;\n", "let _ = env!(\"X\");");
    unbound(&cg, "src/m.rs", "g", "helper");
}

#[test]
fn w6_macro_use_extern_crate_poisons_every_file() {
    let cg = import_graph(
        "#[macro_use]\nextern crate ext;\nmod m;\n",
        "",
        "let _ = env!(\"X\");",
    );
    unbound(&cg, "src/m.rs", "g", "helper");
}

#[test]
fn k11_a_std_qualified_spelling_stays_trusted_under_an_external_glob() {
    let cg = import_graph(PLAIN_LIB, "use ext::*;\n", "let _ = std::env!(\"X\");");
    binds(&cg, "src/m.rs", "g", "helper", ("src/m.rs", "helper", 2));
}

#[test]
fn k12_super_and_crate_globs_and_std_imports_do_not_poison() {
    for uses in [
        "use super::*;\n",
        "use crate::*;\n",
        "use std::env;\n",
        "use std::collections::*;\n",
        "use core::matches;\n",
    ] {
        let body = if uses.contains("matches") {
            "let _ = matches!(1u8, 1);"
        } else {
            "let _ = env!(\"X\");"
        };
        let cg = import_graph(PLAIN_LIB, uses, body);
        let lines = uses.matches('\n').count();
        binds(
            &cg,
            "src/m.rs",
            "g",
            "helper",
            ("src/m.rs", "helper", lines + 1),
        );
    }
}

// ── Refined census (#765 PR D, round-2 prep): only what can import a macro taints ──

const MAKE_LIB: &str =
    "#[macro_export]\nmacro_rules! make { () => { fn helper() -> u8 { 2 } }; }\nmod m;\n";

#[test]
fn w7_an_alias_of_an_external_item_taints() {
    let cg = import_graph(PLAIN_LIB, "use ext::anything as env;\n", "env!();");
    unbound(&cg, "src/m.rs", "g", "helper");
}

#[test]
fn w8_a_chain_of_in_repo_aliases_of_a_macro_taints() {
    let cg = import_graph(
        MAKE_LIB,
        "use crate::make as mk;\nuse crate::mk as env;\n",
        "env!();",
    );
    unbound(&cg, "src/m.rs", "g", "helper");
}

#[test]
fn w9_an_external_import_of_write_withholds_the_transparent_macro() {
    let cg = import_graph(PLAIN_LIB, "use ext::write;\n", "write!(s, \"{}\", 1);");
    unbound(&cg, "src/m.rs", "g", "helper");
}

#[test]
fn k13_an_in_repo_alias_of_a_fn_imports_no_macro() {
    // `use crate::not_a_macro as env;` names a fn: `env!(..)` is still the std macro.
    let lib = "pub fn not_a_macro() {}\nmod m;\n";
    let cg = import_graph(
        lib,
        "use crate::not_a_macro as env;\n",
        "let _ = env!(\"X\");",
    );
    binds(&cg, "src/m.rs", "g", "helper", ("src/m.rs", "helper", 2));
}

#[test]
fn k14_an_unaliased_in_repo_import_of_a_fn_named_like_a_macro_does_not_taint() {
    // `use super::write;` imports a fn: no `macro_rules! write` exists, so `write!` is std.
    let lib = "pub fn write() {}\nmod m;\n";
    let cg = import_graph(lib, "use super::write;\n", "write!(s, \"{}\", 1);");
    binds(&cg, "src/m.rs", "g", "helper", ("src/m.rs", "helper", 2));
}

#[test]
fn k15_write_arguments_are_still_extracted_when_write_is_a_fn_import() {
    let m = "use super::write;\nfn helper(n: u8) -> u8 { n }\npub fn g() {\n    write!(s, \"{}\", helper(2)).unwrap();\n}\n";
    let cg = build(&[
        ("src/lib.rs", "pub fn write() {}\nmod m;\n"),
        ("src/m.rs", m),
    ]);
    binds(&cg, "src/m.rs", "g", "helper", ("src/m.rs", "helper", 2));
}
