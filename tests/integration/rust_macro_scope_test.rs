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
