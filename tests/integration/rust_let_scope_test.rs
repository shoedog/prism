//! #765 PR D, E1: a `let`/`for`/`if let`/`while let` pattern binds its names only AFTER the
//! initializer (rustc), so `let report = report(2);` calls the fn, not the new local, in an
//! ORDINARY module. RED rows fail on the pre-fix route (a refusal of a call rustc binds); KEEP
//! rows pin what stays refused or unchanged. Each row kills a mutant of
//! `mutants/rust-let-scope.json`.
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

// ── E1: a binding is in scope only after its statement ──────────────────────

const LET_FILE: &str = "\
fn report(n: u8) -> u8 { n }
fn pair(n: u8) -> (u8, u8) { (n, n) }
fn opt(n: u8) -> Option<u8> { Some(n) }
fn items() -> Vec<u8> { vec![1] }
fn add(n: u8) -> u8 { n }
pub fn l1() {
    let report = report(2);
    let _ = report;
}
pub fn l2() {
    let (pair, other) = pair(1);
    let _ = (pair, other);
}
pub fn l3a() {
    let Some(opt) = opt(1) else { return };
    let _ = opt;
}
pub fn l3b() {
    let Some(opt) = Some(1u8) else {
        opt(2);
        return;
    };
    let _ = opt;
}
pub fn l4() {
    for items in items() {
        let _ = items;
    }
}
pub fn l5() {
    if let Some(opt) = opt(1) {
        let _ = opt;
    }
}
pub fn l6() {
    while let Some(opt) = opt(1) {
        let _ = opt;
        break;
    }
}
pub fn k1() {
    let add = |n: u8| n;
    add(1);
}
pub fn k2() {
    let _ = match report(1) {
        report => report,
    };
}
pub fn k3() {
    let c = |report| report(1);
    let _ = c;
}
pub fn l7() {
    let x = 1u8;
    let report = report(x);
    let report = report + 1;
    let _ = report;
}
";

fn let_graph() -> CallGraph {
    build(&[("src/lib.rs", "mod m;\n"), ("src/m.rs", LET_FILE)])
}

#[test]
fn l1_let_initializer_calls_the_fn_not_the_new_local() {
    binds(
        &let_graph(),
        "src/m.rs",
        "l1",
        "report",
        ("src/m.rs", "report", 1),
    );
}

#[test]
fn l2_tuple_pattern_binds_after_the_initializer() {
    binds(
        &let_graph(),
        "src/m.rs",
        "l2",
        "pair",
        ("src/m.rs", "pair", 2),
    );
}

#[test]
fn l3a_let_else_initializer_calls_the_fn() {
    binds(
        &let_graph(),
        "src/m.rs",
        "l3a",
        "opt",
        ("src/m.rs", "opt", 3),
    );
}

#[test]
fn l3b_let_else_block_does_not_see_the_pattern_binding() {
    binds(
        &let_graph(),
        "src/m.rs",
        "l3b",
        "opt",
        ("src/m.rs", "opt", 3),
    );
}

#[test]
fn l4_for_iterator_expression_is_outside_the_loop_variable() {
    binds(
        &let_graph(),
        "src/m.rs",
        "l4",
        "items",
        ("src/m.rs", "items", 4),
    );
}

#[test]
fn l5_if_let_scrutinee_is_outside_the_pattern_binding() {
    binds(
        &let_graph(),
        "src/m.rs",
        "l5",
        "opt",
        ("src/m.rs", "opt", 3),
    );
}

#[test]
fn l6_while_let_scrutinee_is_outside_the_pattern_binding() {
    binds(
        &let_graph(),
        "src/m.rs",
        "l6",
        "opt",
        ("src/m.rs", "opt", 3),
    );
}

#[test]
fn k1_a_later_call_through_the_local_closure_is_not_the_fn() {
    unbound(&let_graph(), "src/m.rs", "k1", "add");
}

#[test]
fn k2_match_scrutinee_is_unaffected_by_its_arm_binding() {
    binds(
        &let_graph(),
        "src/m.rs",
        "k2",
        "report",
        ("src/m.rs", "report", 1),
    );
}

#[test]
fn k3_closure_parameter_shadows_the_fn_in_its_body() {
    unbound(&let_graph(), "src/m.rs", "k3", "report");
}

#[test]
fn l7_chained_shadowing_binds_the_first_initializer_to_the_fn() {
    binds(
        &let_graph(),
        "src/m.rs",
        "l7",
        "report",
        ("src/m.rs", "report", 1),
    );
}
