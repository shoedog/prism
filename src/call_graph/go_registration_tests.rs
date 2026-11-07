use super::*;
use crate::languages::Language::{Go, C};
use std::collections::BTreeMap;

fn build_go(files: &[(&str, &str)]) -> CallGraph {
    let mut map = BTreeMap::new();
    for (path, src) in files {
        map.insert(path.to_string(), ParsedFile::parse(path, src, Go).unwrap());
    }
    CallGraph::build(&map)
}

fn fid<'a>(cg: &'a CallGraph, name: &str) -> &'a FunctionId {
    cg.functions
        .get(name)
        .unwrap_or_else(|| panic!("no function named {name}"))
        .first()
        .unwrap()
}

#[test]
fn form_a_composite_literal_field_recorded_with_field_key() {
    let cg = build_go(&[(
        "main.go",
        "package main\n\
type Command struct {\n\tRun func()\n}\n\
func helper() {}\n\
func main() {\n\tc := Command{Run: helper}\n\t_ = c\n}\n",
    )]);
    let target = fid(&cg, "helper").clone();
    let enclosing = fid(&cg, "main").clone();
    let reg = cg
        .go_registrations
        .iter()
        .find(|r| r.target == target && r.enclosing == enclosing)
        .expect("form (a) registration recorded");
    assert!(reg.field_key.is_some(), "known struct+field -> field-keyed");
    assert_eq!(reg.field_key.as_ref().unwrap().1, "Run");
    assert_eq!(reg.site.line, 7); // `c := Command{Run: helper}` line
    assert_eq!(cg.go_registration_unknown_owner_recorded, 0);
}

#[test]
fn form_b_field_assignment_recorded_with_field_key() {
    let cg = build_go(&[(
        "main.go",
        "package main\n\
type Command struct {\n\tRun func()\n}\n\
func helper() {}\n\
func register() {\n\tvar cmd Command\n\tcmd.Run = helper\n}\n",
    )]);
    let target = fid(&cg, "helper").clone();
    let enclosing = fid(&cg, "register").clone();
    let reg = cg
        .go_registrations
        .iter()
        .find(|r| r.target == target && r.enclosing == enclosing)
        .expect("form (b) registration recorded");
    assert!(reg.field_key.is_some());
    assert_eq!(reg.field_key.as_ref().unwrap().1, "Run");
    assert_eq!(cg.go_registration_ambiguous_owner_skips, 0);
}

#[test]
fn form_b_unrecoverable_owner_type_is_skipped_and_counted() {
    // `cmd := getCommand()` — a short var decl whose RHS is a plain call
    // (not the `New*` constructor-name heuristic Go recovery uses), so
    // the owner type is NOT recoverable at all. Form (b) must skip
    // (never fall back the way form (a) does), and count it.
    let cg = build_go(&[(
        "main.go",
        "package main\n\
type Command struct {\n\tRun func()\n}\n\
func helper() {}\n\
func getCommand() Command { return Command{} }\n\
func register() {\n\tcmd := getCommand()\n\tcmd.Run = helper\n}\n",
    )]);
    let target = fid(&cg, "helper").clone();
    assert!(
        !cg.go_registrations.iter().any(|r| r.target == target),
        "unrecoverable owner type must not be recorded"
    );
    assert_eq!(cg.go_registration_ambiguous_owner_skips, 1);
}

#[test]
fn form_b_multiple_bindings_before_assignment_is_ambiguous() {
    // Two `var cmd Command` bindings of the SAME name before the
    // assignment (tree-sitter doesn't enforce Go's real "no redeclare"
    // rule) forces `receiver_type_in_fn`'s existing shadow-bail
    // (`bindings > 1 -> None`) — the owner type is not UNIQUELY
    // recovered, so form (b) must skip and count, not record.
    let cg = build_go(&[(
        "main.go",
        "package main\n\
type Command struct {\n\tRun func()\n}\n\
func helper() {}\n\
func register() {\n\tvar cmd Command\n\tvar cmd Command\n\tcmd.Run = helper\n}\n",
    )]);
    let target = fid(&cg, "helper").clone();
    assert!(
        !cg.go_registrations.iter().any(|r| r.target == target),
        "ambiguous (multiply-bound) owner must not be recorded"
    );
    assert_eq!(cg.go_registration_ambiguous_owner_skips, 1);
}

#[test]
fn form_c_call_argument_recorded_without_field_key() {
    let cg = build_go(&[(
        "main.go",
        "package main\n\
func helper() {}\n\
func Register(f func()) {}\n\
func main() {\n\tRegister(helper)\n}\n",
    )]);
    let target = fid(&cg, "helper").clone();
    let enclosing = fid(&cg, "main").clone();
    let reg = cg
        .go_registrations
        .iter()
        .find(|r| r.target == target && r.enclosing == enclosing)
        .expect("form (c) registration recorded");
    assert!(
        reg.field_key.is_none(),
        "form (c) never carries a field key"
    );
}

#[test]
fn shadowed_identifier_is_skipped_and_counted() {
    // `helper` is locally re-bound inside `main`, so the reference at the
    // call-argument site names the LOCAL, not the free function -> must
    // be skipped, not recorded.
    let cg = build_go(&[(
        "main.go",
        "package main\n\
func helper() {}\n\
func Register(f func()) {}\n\
func main() {\n\thelper := func() {}\n\t_ = helper\n\tRegister(helper)\n}\n",
    )]);
    let target = fid(&cg, "helper").clone();
    assert!(
        !cg.go_registrations.iter().any(|r| r.target == target),
        "shadowed identifier must not be recorded as a registration"
    );
    assert_eq!(cg.go_registration_shadowed_skips, 1);
}

#[test]
fn cross_package_bare_name_is_never_matched() {
    // `helper` exists only in package `other` (a different directory);
    // package `main`'s `Register(helper)` must NOT resolve cross-package
    // by bare name (spec-review MAJOR: never bare cross-package matching).
    // Since `helper` isn't in-repo from `main`'s package/file perspective,
    // no registration should be recorded at all (no false cross-package
    // hit, and no false-negative panic either).
    let cg = build_go(&[
        ("other/helper.go", "package other\nfunc helper() {}\n"),
        (
            "main/main.go",
            "package main\nfunc Register(f func()) {}\nfunc main() {\n\tRegister(helper)\n}\n",
        ),
    ]);
    assert!(
        cg.go_registrations.is_empty(),
        "cross-package bare name must never resolve into a registration"
    );
}

#[test]
fn non_go_files_leave_registration_state_empty() {
    let mut files = BTreeMap::new();
    files.insert(
            "main.c".to_string(),
            ParsedFile::parse(
                "main.c",
                "struct Command { void (*run)(); };\nvoid helper() {}\nvoid main() { struct Command c = { .run = helper }; }\n",
                C,
            )
            .unwrap(),
        );
    let cg = CallGraph::build(&files);
    assert!(cg.go_registrations.is_empty());
    assert_eq!(cg.go_registration_shadowed_skips, 0);
    assert_eq!(cg.go_registration_ambiguous_owner_skips, 0);
    assert_eq!(cg.go_registration_unknown_owner_recorded, 0);
}
