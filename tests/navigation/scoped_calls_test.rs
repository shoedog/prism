use prism::call_graph::{CallKind, CallSite, CallSiteOrigin};
use prism::navigation::call_resolve::resolve_site_nav;
use prism::navigation::module_graph::{module_deps, repo_map};
use prism::navigation::queries;
use prism::navigation::types::SymbolRef;
use prism::navigation::{NavigationIndex, NavigationSession};
use prism::repo_loader::load_repo;
use prism::resolution::ResolutionKind;
use std::sync::Arc;

fn session(files: &[(&str, &str)]) -> NavigationSession {
    let dir = tempfile::tempdir().unwrap();
    for (name, src) in files {
        let path = dir.path().join(name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, src).unwrap();
    }
    let repo = Arc::new(load_repo(dir.path()).unwrap());
    let index = Arc::new(NavigationIndex::build(&repo));
    NavigationSession { repo, index }
}

fn resolved_targets(
    s: &NavigationSession,
    callee_name: &str,
    caller_file: &str,
    qualifier: Option<&str>,
) -> Vec<(String, String)> {
    let cg = s.index.call_graph();
    let site = cg
        .calls
        .values()
        .flatten()
        .find(|site| {
            site.caller.file == caller_file
                && site.callee_name == callee_name
                && site.qualifier.as_deref() == qualifier
        })
        .cloned()
        .unwrap_or_else(|| {
            let caller = cg
                .functions
                .values()
                .flatten()
                .find(|fid| fid.file == caller_file)
                .unwrap()
                .clone();
            CallSite {
                caller,
                callee_name: callee_name.to_string(),
                line: 1,
                kind: CallKind::Call,
                start_byte: 0,
                end_byte: 0,
                qualifier: qualifier.map(str::to_string),
                receiver_type: None,
                receiver_recovery: None,
                receiver_materialized: false,
                arg_count: None,
                arg_spread: false,
                receiver_outcome: None,
                origin: CallSiteOrigin::Source,
                pre_resolved_target: None,
            }
        });
    resolve_site_nav(cg, &site)
        .into_iter()
        .map(|edge| (edge.target.file.clone(), edge.target.name.clone()))
        .collect()
}

fn resolves_to(
    s: &NavigationSession,
    callee_name: &str,
    caller_file: &str,
    target_file: &str,
    target_name: &str,
) -> bool {
    resolved_targets(s, callee_name, caller_file, None)
        .iter()
        .any(|(file, name)| file == target_file && name == target_name)
}

fn resolves_empty(s: &NavigationSession, callee_name: &str, caller_file: &str) -> bool {
    resolved_targets(s, callee_name, caller_file, None).is_empty()
}

#[test]
fn scoped_mod_fn_resolves_cross_file_rust() {
    let s = session(&[
        ("algo.rs", "pub fn run() -> i32 { 1 }\n"),
        (
            "main.rs",
            "mod algo;\nfn dispatch() -> i32 { algo::run() }\n",
        ),
    ]);
    assert!(
        resolves_to(&s, "algo::run", "main.rs", "algo.rs", "run"),
        "algo::run should resolve to algo.rs::run, got {:?}",
        resolved_targets(&s, "algo::run", "main.rs", None)
    );
}

#[test]
fn scoped_ns_fn_resolves_cross_file_cpp() {
    // C++ namespace-qualified call: util::helper() -> util.cpp::helper (same `::` gap as Rust).
    let s = session(&[
        (
            "util.cpp",
            "namespace util { int helper() { return 1; } }\n",
        ),
        (
            "main.cpp",
            "namespace util { int helper(); }\nint dispatch() { return util::helper(); }\n",
        ),
    ]);
    assert!(
        resolves_to(&s, "util::helper", "main.cpp", "util.cpp", "helper"),
        "util::helper should resolve to util.cpp::helper, got {:?}",
        resolved_targets(&s, "util::helper", "main.cpp", None)
    );
}

#[test]
fn scoped_call_to_wrong_stem_does_not_resolve() {
    let s = session(&[
        ("algo.rs", "pub fn run() -> i32 { 1 }\n"),
        ("main.rs", "fn dispatch() -> i32 { nope::run() }\n"),
    ]);
    assert!(resolves_empty(&s, "nope::run", "main.rs"));
}

#[test]
fn reserved_keyword_hints_do_not_resolve() {
    // A decoy file named `crate.rs`/`self.rs` must NOT satisfy crate::run / self::run.
    let s = session(&[
        ("crate.rs", "pub fn run() -> i32 { 1 }\n"),
        ("self.rs", "pub fn go() -> i32 { 1 }\n"),
        ("main.rs", "fn d() -> i32 { crate::run() + self::go() }\n"),
    ]);
    assert!(resolves_empty(&s, "crate::run", "main.rs"));
    assert!(resolves_empty(&s, "self::go", "main.rs"));
}

#[test]
fn external_crate_path_does_not_resolve_without_stem_match() {
    // bincode::serialize with NO bincode-stem file -> empty (external crate, not in repo).
    let s = session(&[
        ("main.rs", "fn d() { bincode::serialize(); }\n"),
        ("other.rs", "pub fn serialize() -> i32 { 1 }\n"), // decoy: wrong stem, must not match
    ]);
    assert!(resolves_empty(&s, "bincode::serialize", "main.rs"));
}

#[test]
fn multi_segment_scoped_path_uses_last_module_segment() {
    let s = session(&[
        ("algo.rs", "pub fn run() -> i32 { 1 }\n"),
        (
            "main.rs",
            "mod algo;\nfn d() -> i32 { crate::algo::run() }\n",
        ),
    ]);
    assert!(resolves_to(
        &s,
        "crate::algo::run",
        "main.rs",
        "algo.rs",
        "run"
    ));
}

#[test]
fn unscoped_resolution_is_unchanged() {
    let s = session(&[
        ("util.rs", "pub fn helper() -> i32 { 1 }\n"),
        (
            "main.rs",
            "mod util;\nuse util::helper;\nfn run() -> i32 { helper() }\n",
        ),
    ]);
    assert!(resolves_to(&s, "helper", "main.rs", "util.rs", "helper"));
}

#[test]
fn callees_resolves_scoped_dispatch() {
    let s = session(&[
        ("algo.rs", "pub fn run() -> i32 { 1 }\n"),
        (
            "main.rs",
            "mod algo;\nfn dispatch() -> i32 { algo::run() }\n",
        ),
    ]);
    let ev = queries::callees(&s, Some("dispatch"), None, None, 1).unwrap();
    assert!(
        ev.items.iter().any(|it| it
            .symbol
            .as_ref()
            .map(|s| matches!(
                s, SymbolRef::Function { file, name, .. } if file == "algo.rs" && name == "run"
            ))
            .unwrap_or(false)),
        "callees(dispatch) should include scoped callee algo.rs::run"
    );
}

#[test]
fn module_deps_and_repo_map_include_scoped_edge() {
    let s = session(&[
        ("algo.rs", "pub fn run() -> i32 { 1 }\n"),
        (
            "main.rs",
            "mod algo;\nfn dispatch() -> i32 { algo::run() }\n",
        ),
    ]);
    let md = module_deps(&s, "main.rs");
    assert!(
        md.items.iter().any(|it| it.location.file == "algo.rs"),
        "module-deps(main.rs) should include a scoped edge to algo.rs"
    );
    // repo-map shares collect_module_edges -> the edge must appear there too.
    let rm = repo_map(&s);
    let g = rm.graph.as_ref().unwrap();
    let main_i = g
        .nodes
        .iter()
        .position(|n| n.location.file == "main.rs")
        .unwrap();
    let algo_i = g
        .nodes
        .iter()
        .position(|n| n.location.file == "algo.rs")
        .unwrap();
    assert!(g
        .edges
        .iter()
        .any(|e| e.from == main_i && e.to == algo_i && e.kind == "ModuleDep"));
}

#[test]
fn callers_finds_scoped_dispatcher() {
    let s = session(&[
        ("algo.rs", "pub fn run() -> i32 { 1 }\n"),
        (
            "main.rs",
            "mod algo;\nfn dispatch() -> i32 { algo::run() }\n",
        ),
    ]);
    let ev = queries::callers(&s, Some("run"), Some("algo.rs"), None, 1).unwrap();
    assert!(
        ev.items.iter().any(|it| it
            .symbol
            .as_ref()
            .map(|s| matches!(
                s, SymbolRef::Function { name, .. } if name == "dispatch"
            ))
            .unwrap_or(false)),
        "callers(run@algo.rs) should include the scoped dispatcher"
    );
}

#[test]
fn callers_excludes_other_stem_scoped_call() {
    // Two files define `run`; dispatcher calls other::run. callers(run@algo.rs)
    // must NOT include it — scoped_caller_sites returns a superset the identity
    // filter prunes (the Rust/C++ stem-collision guard).
    let s = session(&[
        ("algo.rs", "pub fn run() -> i32 { 1 }\n"),
        ("other.rs", "pub fn run() -> i32 { 2 }\n"),
        (
            "main.rs",
            "mod algo;\nmod other;\nfn dispatch() -> i32 { other::run() }\n",
        ),
    ]);
    let ev = queries::callers(&s, Some("run"), Some("algo.rs"), None, 1).unwrap();
    assert!(
        !ev.items.iter().any(|it| it
            .symbol
            .as_ref()
            .map(|s| matches!(
                s, SymbolRef::Function { name, .. } if name == "dispatch"
            ))
            .unwrap_or(false)),
        "callers(run@algo.rs) must exclude a dispatcher that calls other::run"
    );
}

/// Review-fix wave regression: `resolve_site_nav` must call
/// `resolve_call_site_full`, not the non-nav `resolve_call_site` wrapper --
/// the latter gates `FuncValueField` to singleton targets only (see
/// `resolution::go_func_value_field_resolution_tests::
/// two_target_func_value_field_is_filtered_from_resolve_call_site_but_not_from_full`).
/// A 2-target func-typed-field registration exercised directly through
/// `resolve_site_nav` must keep BOTH targets.
#[test]
fn resolve_site_nav_keeps_two_target_func_value_field_fanout() {
    let s = session(&[(
        "main.go",
        "package main\n\
type Command struct {\n\tRun func()\n}\n\
func h1() {}\n\
func h2() {}\n\
func register_a() *Command { return &Command{Run: h1} }\n\
func register_b() *Command { return &Command{Run: h2} }\n\
func invoke(cmd *Command) {\n\tcmd.Run()\n}\n",
    )]);
    let cg = s.index.call_graph();
    let caller_id = cg.functions.get("invoke").unwrap().first().unwrap();
    let site = cg
        .calls
        .get(caller_id)
        .unwrap()
        .iter()
        .find(|site| site.callee_name == "Run" && site.qualifier.is_some())
        .expect("qualified Run() call site present");

    let edges = resolve_site_nav(cg, site);
    let names: Vec<&str> = edges.iter().map(|e| e.target.name.as_str()).collect();
    assert!(
        names.contains(&"h1") && names.contains(&"h2"),
        "resolve_site_nav must keep both func_value_field targets, got {names:?}"
    );
    assert!(
        edges
            .iter()
            .all(|e| e.kind == ResolutionKind::FuncValueField),
        "expected both edges to be FuncValueField, got {edges:?}"
    );
}
