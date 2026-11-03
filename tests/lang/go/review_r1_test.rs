//! Review regressions: each positive expectation is also run against main 72025b3e.
use prism::ast::ParsedFile;
use prism::languages::Language;
use prism::resolution::ResolutionConfidence;

fn restored(prefix: &str, body: &str) {
    let source = format!("package app\nimport lib \"example/lib\"\n{prefix}\nfunc F[T any]() {{}}\nfunc run() {{\n{body}\n}}\n");
    let p = ParsedFile::parse("app/run.go", &source, Language::Go).unwrap();
    assert!(
        !p.tree.root_node().has_error(),
        "{}",
        p.tree.root_node().to_sexp()
    );
    let run = p
        .all_functions()
        .into_iter()
        .find(|n| {
            n.child_by_field_name("name")
                .is_some_and(|name| p.node_text(&name) == "run")
        })
        .unwrap();
    let calls = p.function_calls_on_lines_with_qualifier(&run, &(1..=100).collect());
    assert_eq!(
        calls.last().map(|c| (&*c.0, c.2.as_deref())),
        Some(("F", None)),
        "{calls:?}"
    );
    let cg = super::test_support::build_go(&[("app/run.go", &source)]);
    let call = cg
        .calls
        .values()
        .flatten()
        .find(|c| c.caller.name == "run" && c.callee_name == "F")
        .unwrap();
    let resolution = cg.resolve_call_site_full(call);
    assert_eq!(resolution.resolved.len(), 1, "{resolution:?}");
    assert_eq!(
        resolution.resolved[0].confidence,
        ResolutionConfidence::Exact
    );
}

macro_rules! restore_case {
    ($name:ident, $prefix:expr, $body:expr) => {
        #[test]
        fn $name() {
            restored($prefix, $body);
        }
    };
}
restore_case!(
    default_arm_ends,
    "",
    "switch { default: F := []func(){}; _ = F }; F[int]()"
);
restore_case!(
    expression_switch_init_ends,
    "",
    "switch F := []func(){}; len(F) { default: }; F[int]()"
);
restore_case!(
    type_switch_init_ends,
    "",
    "switch F := []func(){}; any(F).(type) { default: }; F[int]()"
);
restore_case!(
    select_default_ends,
    "",
    "select { default: F := []func(){}; _ = F }; F[int]()"
);
restore_case!(
    function_type_parameter_is_not_value,
    "type Callback func(F int)",
    "F[int]()"
);
restore_case!(
    interface_parameter_is_not_value,
    "type I interface { M(F int) }",
    "F[int]()"
);
restore_case!(
    nested_callback_parameter_is_not_value,
    "",
    "var cb func(F int); _ = cb; F[int]()"
);
restore_case!(
    nested_result_parameter_is_not_value,
    "",
    "var cb func() func(F int); _ = cb; F[int]()"
);

fn type_shadow(source: &str) {
    let cg = super::test_support::build_go(&[
        (
            "lib/f.go",
            "package lib\nfunc Keys[K comparable](m map[K]int, unused int) {}\n",
        ),
        ("app/run.go", source),
    ]);
    let call = cg
        .calls
        .values()
        .flatten()
        .find(|c| c.caller.name == "run")
        .unwrap();
    assert_eq!(call.callee_name, "Keys");
    assert_eq!(call.qualifier.as_deref(), Some("lib"));
    let out = cg.resolve_call_site_full(call);
    assert_eq!(out.resolved.len(), 1, "{out:?}");
    assert_eq!(out.resolved[0].target.file, "lib/f.go");
    assert_eq!(out.resolved[0].confidence, ResolutionConfidence::Exact);
}

#[test]
fn function_type_parameter_shadows_package_value() {
    type_shadow("package app\nimport lib \"example/lib\"\nvar K = 1\nfunc run[K comparable](m map[K]int) { lib.Keys[K](m,0) }\n");
}
#[test]
fn local_type_shadows_package_value() {
    type_shadow("package app\nimport lib \"example/lib\"\nvar K = 1\nfunc run() { type K int; lib.Keys[K](nil,0) }\n");
}
#[test]
fn local_type_alias_shadows_package_value() {
    type_shadow("package app\nimport lib \"example/lib\"\nvar K = 1\nfunc run() { type K = int; lib.Keys[K](nil,0) }\n");
}

#[test]
fn real_values_still_shadow_and_ended_types_do_not_leak() {
    let source = "package p\nvar K = 1\nfunc run(F []func(), m map[int]func(), k int) {\n { type K int; pkg.Keys[K]() }\n pkg.Keys[K]()\n F[k]()\n m[k]()\n func(F []func()) { F[k]() }(F)\n}\n";
    let p = ParsedFile::parse("p.go", source, Language::Go).unwrap();
    let calls =
        p.function_calls_on_lines_with_qualifier(&p.all_functions()[0], &(1..=20).collect());
    for (line, name) in [(4, "Keys"), (5, "pkg.Keys[K]"), (6, "F[k]"), (7, "m[k]")] {
        assert!(
            calls.iter().any(|c| c.1 == line && c.0 == name),
            "{line} {name}: {calls:?}"
        );
    }
    let literal = p.tree.root_node();
    let calls = p.function_calls_on_lines_with_qualifier(&literal, &(8..=8).collect());
    assert!(
        calls.iter().any(|c| c.0 == "F[k]"),
        "ordinary literal parameter shadows: {calls:?}"
    );
}

#[test]
fn inner_value_overrides_type_parameter_only_in_its_scope() {
    let source =
        "package p\nfunc run[K comparable]() {\n { K := 0; pkg.F[K](0,0) }\n pkg.F[K](0,0)\n}\n";
    let p = ParsedFile::parse("p.go", source, Language::Go).unwrap();
    let calls = p.function_calls_on_lines_with_qualifier(&p.all_functions()[0], &(1..=5).collect());
    assert_eq!(
        calls,
        [
            ("pkg.F[K]".into(), 3, None),
            ("F".into(), 4, Some("pkg".into()))
        ]
    );
}
