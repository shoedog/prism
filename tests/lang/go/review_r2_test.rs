//! Final review fold: receiver-introduced type names differ from the type's U.
use prism::ast::ParsedFile;
use prism::languages::Language;
use prism::resolution::ResolutionConfidence;

fn receiver_restores_exact(receiver: &str) {
    let source = format!(
        "package app\nimport lib \"example/lib\"\nvar K = 1\ntype Set[U comparable] struct{{}}\nfunc (s {receiver}) Run(m map[K]int) {{ lib.Keys[K](m, 0) }}\n"
    );
    let p = ParsedFile::parse("app/run.go", &source, Language::Go).unwrap();
    assert_eq!(p.parse_error_count, 0);
    let calls =
        p.function_calls_on_lines_with_qualifier(&p.all_functions()[0], &(1..=10).collect());
    assert_eq!(calls, [("Keys".into(), 5, Some("lib".into()))]);
    let cg = super::test_support::build_go(&[
        ("app/run.go", &source),
        (
            "lib/keys.go",
            "package lib\nfunc Keys[K comparable](m map[K]int, unused int) {}\n",
        ),
    ]);
    let call = cg
        .calls
        .values()
        .flatten()
        .find(|c| c.callee_name == "Keys")
        .unwrap();
    assert_eq!(call.qualifier.as_deref(), Some("lib"));
    let out = cg.resolve_call_site_full(call);
    assert_eq!(out.resolved.len(), 1, "{out:?}");
    assert_eq!(out.resolved[0].target.file, "lib/keys.go");
    assert_eq!(out.resolved[0].confidence, ResolutionConfidence::Exact);
}

#[test]
fn value_receiver_type_parameter_restores_generic_call() {
    receiver_restores_exact("Set[K]");
}

#[test]
fn pointer_receiver_type_parameter_restores_generic_call() {
    receiver_restores_exact("*Set[K]");
}

#[test]
fn receiver_type_binding_yields_to_inner_value_and_ends_with_method() {
    let source = "package app\nvar K = 1\ntype Set[U comparable] struct{}\nfunc (s *Set[K]) Run() {\n { K := 0; lib.Keys[K](nil,0) }\n lib.Keys[K](nil,0)\n}\nfunc after() { lib.Keys[K](nil,0) }\n";
    let p = ParsedFile::parse("app/run.go", source, Language::Go).unwrap();
    assert_eq!(p.parse_error_count, 0);
    let calls = p.function_calls_on_lines_with_qualifier(&p.tree.root_node(), &(1..=10).collect());
    assert_eq!(
        calls,
        [
            ("lib.Keys[K]".into(), 5, None),
            ("Keys".into(), 6, Some("lib".into())),
            ("lib.Keys[K]".into(), 8, None),
        ]
    );
}
