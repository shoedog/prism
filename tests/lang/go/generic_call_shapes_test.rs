//! Grammar-0.25 indexed callees must retain the pre-bump call identity.
use prism::ast::ParsedFile;
use prism::call_graph::{CallGraph, CallSite};
use prism::languages::Language;
use prism::resolution::ResolutionConfidence;
use std::collections::{BTreeMap, BTreeSet};

fn site<'a>(cg: &'a CallGraph, caller: &str, callee: &str) -> &'a CallSite {
    cg.calls
        .values()
        .flatten()
        .find(|s| s.caller.name == caller && s.callee_name == callee)
        .unwrap_or_else(|| panic!("missing {caller}->{callee}: {:?}", cg.calls))
}

#[test]
fn generic_call_names_qualifiers_and_argument_identity() {
    let source = "package p\nfunc F[T any](a,b T) {}\nfunc run[T any](x T) {\n F[T]()\n F[T](x,x)\n pkg.F[T]()\n pkg.F[T](x,x)\n x.M[T]()\n}\n";
    let p = ParsedFile::parse("p.go", source, Language::Go).unwrap();
    let run = p
        .all_functions()
        .into_iter()
        .find(|n| p.node_text(&n.child_by_field_name("name").unwrap()) == "run")
        .unwrap();
    let calls = p.function_calls_on_lines_with_qualifier(&run, &(1..=9).collect());
    assert_eq!(
        calls,
        vec![
            ("F".into(), 4, None),
            ("F".into(), 5, None),
            ("F".into(), 6, Some("pkg".into())),
            ("F".into(), 7, Some("pkg".into())),
            ("M".into(), 8, Some("x".into())),
        ]
    );
    let args = p.call_argument_texts_at(source.find("F[T](x,x)").unwrap(), "F");
    assert_eq!(
        args.iter().map(|a| a.as_str()).collect::<Vec<_>>(),
        ["x", "x"]
    );
}

#[test]
fn generic_import_and_local_calls_resolve_exactly() {
    let cg = super::test_support::build_go(&[
        ("lib/f.go", "package lib\nfunc F[T any](a,b T) {}\n"),
        ("app/run.go", "package app\nimport lib \"example/lib\"\nfunc F[T any](a,b T) {}\nfunc run() {\n lib.F[int](1,2)\n F[int](1,2)\n}\n"),
    ]);
    let calls: Vec<_> = cg
        .calls
        .values()
        .flatten()
        .filter(|s| s.caller.name == "run")
        .collect();
    assert_eq!(calls.len(), 2);
    for s in calls {
        assert_eq!(s.callee_name, "F");
        let out = cg.resolve_call_site_full(s);
        assert_eq!(out.resolved.len(), 1, "{s:?}: {out:?}");
        assert_eq!(out.resolved[0].confidence, ResolutionConfidence::Exact);
        let expected = if s.qualifier.is_some() {
            "lib/f.go"
        } else {
            "app/run.go"
        };
        assert_eq!(out.resolved[0].target.file, expected);
    }
}

#[test]
fn real_slice_and_map_calls_and_values_keep_index_identity() {
    let source = "package p\nfunc run(handlers []func(), m map[string]func(), i int, k string) {\n handlers[i]()\n m[k]()\n _ = handlers[i]\n _ = m[k]\n}\n";
    let p = ParsedFile::parse("p.go", source, Language::Go).unwrap();
    let cg = CallGraph::build(&BTreeMap::from([("p.go".into(), p)]));
    let names: BTreeSet<_> = cg
        .calls
        .values()
        .flatten()
        .map(|s| s.callee_name.as_str())
        .collect();
    assert_eq!(names, BTreeSet::from(["handlers[i]", "m[k]"]));
    for s in cg.calls.values().flatten() {
        assert!(cg.resolve_call_site_full(s).resolved.is_empty());
    }
}

#[test]
fn generic_constructor_receiver_keeps_existing_new_name_heuristic() {
    let cg = super::test_support::build_go(&[("p/run.go", "package p\ntype C struct{}\nfunc (*C) M() {}\nfunc NewC[T any](a,b T) *C { return &C{} }\nfunc run() {\n x := NewC[int](1,2)\n x.M()\n}\n")]);
    let s = site(&cg, "run", "M");
    let out = cg.resolve_call_site_full(s);
    assert_eq!(s.receiver_type.as_deref(), Some("C"));
    assert_eq!(out.resolved.len(), 1, "{s:?}: {out:?}");
    assert_eq!(out.resolved[0].target.file, "p/run.go");
    assert_eq!(out.resolved[0].confidence, ResolutionConfidence::Exact);
}

#[test]
fn generic_type_arguments_preserve_broad_inventory_and_narrow_reference_query() {
    let source = "package p\nfunc F[T any](a,b T) {}\nfunc run[T any](x T) {\n y := F[T](x,x)\n sink(y)\n}\n";
    let p = ParsedFile::parse("p.go", source, Language::Go).unwrap();
    let run = p
        .all_functions()
        .into_iter()
        .find(|n| p.node_text(&n.child_by_field_name("name").unwrap()) == "run")
        .unwrap();
    let uses = p.rvalue_identifier_spans_on_lines(&run, &BTreeSet::from([4]));
    assert!(
        uses.iter().any(|s| s.path.base == "T"),
        "legacy broad inventory: {uses:?}"
    );
    assert!(!p.find_variable_references(&run, "T").contains(&4));
    assert!(uses.iter().any(|s| s.path.base == "x"));
    assert!(uses.iter().any(|s| s.path.base == "F"));
}

#[test]
fn generic_higher_order_call_keeps_existing_level3_refusal() {
    let cg = super::test_support::build_go(&[("p/run.go", "package p\nfunc Invoke[T any](cb func(), unused int) { cb() }\nfunc Callback() {}\nfunc run() { Invoke[int](Callback,0) }\n")]);
    let direct = cg.resolve_call_site_full(site(&cg, "run", "Invoke"));
    assert_eq!(direct.resolved.len(), 1, "{direct:?}");
    assert_eq!(direct.resolved[0].confidence, ResolutionConfidence::Exact);
    // Level 3 has always excluded generic declarations from callback facts.
    let s = site(&cg, "Invoke", "cb");
    let out = cg.resolve_call_site_full(s);
    assert!(out.resolved.is_empty(), "{out:?}");
}

#[test]
fn pointer_type_calls_restore_but_standalone_values_and_conversions_do_not_widen() {
    let source = "package p\ntype T struct{}\nfunc run(x T) {\n pkg.F[*T](x,x)\n _ = pkg.F[T]\n _ = pkg.F[*T]\n _ = pkg.F[T](x)\n}\n";
    let p = ParsedFile::parse("p.go", source, Language::Go).unwrap();
    let run = p.all_functions()[0];
    let calls = p.function_calls_on_lines_with_qualifier(&run, &(1..=8).collect());
    assert_eq!(calls, [("F".into(), 4, Some("pkg".into()))]);
    for needle in ["pkg.F[T]\n", "pkg.F[*T]\n"] {
        let start = source.find(needle).unwrap();
        let n = p
            .tree
            .root_node()
            .descendant_for_byte_range(start, start + needle.len() - 1)
            .unwrap();
        assert_eq!(
            n.kind(),
            "index_expression",
            "standalone values retain main's raw index"
        );
    }
}

#[test]
fn locally_bound_index_operands_do_not_become_generic_receivers() {
    let source = "package p\nfunc run(obj S, int int) {\n handlers := []func(){}\n m := map[int]func(){}\n handlers[int]()\n m[int]()\n obj.handlers[int]()\n}\n";
    let p = ParsedFile::parse("p.go", source, Language::Go).unwrap();
    let run = p.all_functions()[0];
    let calls = p.function_calls_on_lines_with_qualifier(&run, &(1..=8).collect());
    assert_eq!(
        calls,
        [
            ("handlers[int]".into(), 5, None),
            ("m[int]".into(), 6, None),
            ("obj.handlers[int]".into(), 7, None)
        ]
    );
}

#[test]
fn builtin_and_declared_type_arguments_do_not_capture_value_shadows() {
    let source = "package p\ntype T struct{}\nfunc run(int int, k string, handlers []func()) {\n pkg.F[int]()\n handlers[0]()\n handlers[*(&int)]()\n _ = pkg.F[T]\n}\n";
    let p = ParsedFile::parse("p.go", source, Language::Go).unwrap();
    let calls = p.function_calls_on_lines_with_qualifier(&p.all_functions()[0], &(1..=8).collect());
    assert_eq!(
        calls.iter().map(|c| c.0.as_str()).collect::<Vec<_>>(),
        ["pkg.F[int]", "handlers[0]", "handlers[*(&int)]"]
    );
}

#[test]
fn nested_indices_keep_legacy_reference_queries_without_changing_calls() {
    let source = "package p\nfunc run(matrix [][]int, i,j int, table []int) {\n matrix[i][j] = 1\n sink(matrix[i][j])\n sink(table[i])\n sink(matrix[0][j])\n}\n";
    let p = ParsedFile::parse("p.go", source, Language::Go).unwrap();
    let run = p.all_functions()[0];
    for name in ["matrix", "i", "j"] {
        let refs = p.find_variable_references(&run, name);
        assert!(!refs.contains(&3) && !refs.contains(&4), "{name}: {refs:?}");
    }
    assert!(p.find_variable_references(&run, "i").contains(&5));
    assert!(p.find_variable_references(&run, "matrix").contains(&6));
}

#[test]
fn qualified_type_arguments_keep_legacy_access_paths() {
    let source = "package p\nfunc run(x int) {\n y := pkg.F[types.Item](x,x)\n sink(y)\n}\n";
    let p = ParsedFile::parse("p.go", source, Language::Go).unwrap();
    let run = p.all_functions()[0];
    let paths = p.rvalue_identifier_spans_on_lines(&run, &BTreeSet::from([3]));
    assert!(!paths.iter().any(|s| s.path.base == "types"), "{paths:?}");
    assert!(paths.iter().any(|s| s.path.base == "Item"), "{paths:?}");
    assert!(!p.find_variable_references(&run, "types").contains(&3));
}

#[test]
fn container_call_indices_keep_legacy_references_but_standalone_indices_remain() {
    let source = "package p\nfunc run(handlers []func(), table map[string]func(), i int, k string) {\n handlers[i]()\n table[k]()\n _ = handlers[i]\n sink(table[k])\n}\n";
    let p = ParsedFile::parse("p.go", source, Language::Go).unwrap();
    let run = p.all_functions()[0];
    for (name, call_line, index_line) in [("i", 3, 5), ("k", 4, 6)] {
        let refs = p.find_variable_references(&run, name);
        assert!(
            !refs.contains(&call_line),
            "legacy type-token query: {name}: {refs:?}"
        );
        assert!(
            refs.contains(&index_line),
            "ordinary index: {name}: {refs:?}"
        );
    }
    let spans = p.rvalue_identifier_spans_on_lines(&run, &BTreeSet::from([3, 4]));
    for name in ["handlers", "table"] {
        let start = source.find(&format!(" {name}[")).unwrap() + 1;
        assert!(
            spans.iter().any(|s| s.path.base == name
                && s.start_byte == start
                && s.end_byte == start + name.len()),
            "legacy callee use span {name}: {spans:?}"
        );
    }
}
