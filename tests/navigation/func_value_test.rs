//! P5: Go function-value callback nav surfacing (S2 registration edges + S3
//! gated invocation resolution), exercised end-to-end through `nav_callers`/
//! `nav_callees`.

use prism::navigation::types::SymbolRef;
use prism::navigation::{queries, NavigationIndex, NavigationSession};
use prism::repo_loader::load_repo;
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

// NOTE: an incremental-rebuild parity test (confirming
// `build_incremental_with_scope_graph_inputs` explicitly clears + re-applies
// the whole-program S1/S2 state via `apply_go_func_value_fields` /
// `apply_go_registrations`, the way it already does for Go embedding/
// interface dispatch) lives in `src/navigation/mod.rs`'s own `#[cfg(test)]`
// module, alongside the existing `incremental_from_previous_*` tests —
// `NavigationIndex::build_incremental_from_previous` is `pub(crate)`, so it
// is not reachable from this external integration-test crate.

#[test]
fn callback_registration_surfaces_as_nameonly_caller() {
    let s = session(&[(
        "main.go",
        "package main\n\
type Command struct {\n\tRun func()\n}\n\
func helper() {}\n\
func main() {\n\tc := Command{Run: helper}\n\t_ = c\n}\n",
    )]);
    let ev = queries::callers(&s, Some("helper"), None, None, 1).unwrap();
    let hit = ev
        .items
        .iter()
        .find(|i| matches!(&i.symbol, Some(SymbolRef::Function { name, .. }) if name == "main"))
        .expect("main() surfaces as a caller of helper via its registration site");
    assert!(
        (hit.score - 0.6).abs() < 1e-6,
        "callback_registration is NameOnly -> score 0.6, got {}",
        hit.score
    );
    assert!(hit.why.iter().any(|r| matches!(r,
        prism::navigation::types::Reason::Resolution { kind } if kind == "callback_registration"
    )));
}

#[test]
fn callback_registration_surfaces_symmetrically_in_callees() {
    let s = session(&[(
        "main.go",
        "package main\n\
type Command struct {\n\tRun func()\n}\n\
func helper() {}\n\
func main() {\n\tc := Command{Run: helper}\n\t_ = c\n}\n",
    )]);
    let ev = queries::callees(&s, Some("main"), None, None, 1).unwrap();
    assert!(
        ev.items.iter().any(
            |i| matches!(&i.symbol, Some(SymbolRef::Function { name, .. }) if name == "helper")
        ),
        "callees(main) should include helper via the registration site"
    );
}

// NOTE: these two use composite-literal (form a) registrations, not field
// ASSIGNMENT (form b), deliberately. Form (b)'s `x.Field = target` syntax is
// ALSO matched by the pre-existing, language-general "Level-4 struct-field
// callback" indirect-call heuristic (`CallGraph::compute_indirect_call_sites`,
// call_graph.rs) — a text scan for `.field = value` that predates P5 and is
// not Go-gated. That heuristic creates its own SEPARATE, name-based (not
// package-scoped) synthetic CallSite at the SAME call-site line with its own
// (typically Exact) resolution, independent of S3's new gated logic. Both
// edges are real and coexist; a nav-level assertion that expects the
// func_value_field edge to be the ONLY or top-scored one is fragile against
// that pre-existing interaction. Composite-literal registrations (form a —
// the flagship adjudicated `Command{Run: emptyRun}` shape) have no such
// collision (Level-4 requires `=`, never `:`), so they exercise S3 cleanly
// end-to-end. Form (b) itself IS covered directly (unaffected by Level-4,
// since it inspects `CallGraph` state / a specific `CallSite` rather than
// aggregated nav edges): `call_graph::go_registration_tests` (S2) and
// `resolution::go_func_value_field_resolution_tests` (S3).

#[test]
fn func_value_field_invocation_resolves_via_registration() {
    let s = session(&[(
        "main.go",
        "package main\n\
type Command struct {\n\tRun func()\n}\n\
func helper() {}\n\
func register() *Command {\n\treturn &Command{Run: helper}\n}\n\
func invoke(cmd *Command) {\n\tcmd.Run()\n}\n",
    )]);
    let ev = queries::callers(&s, Some("helper"), None, None, 1).unwrap();
    let hit = ev
        .items
        .iter()
        .find(|i| matches!(&i.symbol, Some(SymbolRef::Function { name, .. }) if name == "invoke"))
        .expect("invoke() surfaces as a caller of helper via func_value_field resolution");
    assert!(
        (hit.score - 0.6).abs() < 1e-6,
        "func_value_field is NameOnly -> score 0.6, got {}",
        hit.score
    );
    assert!(hit.why.iter().any(|r| matches!(r,
        prism::navigation::types::Reason::Resolution { kind } if kind == "func_value_field"
    )));
}

#[test]
fn func_value_fanout_is_not_attributed_to_any_single_registrant() {
    let s = session(&[(
        "main.go",
        "package main\n\
type Command struct {\n\tRun func()\n}\n\
func h1() {}\n\
func h2() {}\n\
func h3() {}\n\
func h4() {}\n\
func register_a() *Command { return &Command{Run: h1} }\n\
func register_b() *Command { return &Command{Run: h2} }\n\
func register_c() *Command { return &Command{Run: h3} }\n\
func register_d() *Command { return &Command{Run: h4} }\n\
func invoke(cmd *Command) {\n\tcmd.Run()\n}\n",
    )]);
    let ev = queries::callers(&s, Some("h1"), None, None, 1).unwrap();
    assert!(
        !ev.items.iter().any(
            |i| matches!(&i.symbol, Some(SymbolRef::Function { name, .. }) if name == "invoke")
        ),
        ">3 distinct registration targets must not attribute the invocation to any one of them"
    );
    // The registration site itself is still a (callback_registration) caller.
    assert!(ev.items.iter().any(
        |i| matches!(&i.symbol, Some(SymbolRef::Function { name, .. }) if name == "register_a")
    ));
}
