//! Keep the pre-bump query/DFG inventory for ambiguous non-call indices.
use crate::common::*;

fn parsed(expression: &str) -> ParsedFile {
    ParsedFile::parse(
        "p.go",
        &format!("package p\nfunc run() {{\n sink({expression})\n}}\n"),
        Language::Go,
    )
    .unwrap()
}

#[test]
fn nested_index_values_keep_legacy_reference_queries() {
    // These are the twelve normalized tree shapes observed in the static audit.
    for expression in [
        "a[b[obj.Field]]",
        "a[b[i]]",
        "a[obj.Indices[i]]",
        "obj.Values[b[i]]",
        "obj.Values[i][j]",
        "obj.Values[obj.Indices[i]]",
        "matrix[i][j]",
        "obj.Values[i][other.Field]",
        "matrix[i][other.Field]",
        "obj.Values[other.Field][j]",
        "obj.Values[b[*ptr]]",
        "*obj.Values[i][j]",
        // Same type syntax under parentheses and nested type arguments.
        "a[(b[i])]",
        "a[b[i]][j]",
    ] {
        let p = parsed(expression);
        assert_eq!(p.parse_error_count, 0, "{expression}");
        let function = p.all_functions()[0];
        for name in ["a", "b", "obj", "i", "j", "matrix", "other", "ptr"] {
            assert!(
                !p.find_variable_references(&function, name).contains(&3),
                "legacy type tokens must stay out of narrow queries: {expression}: {name}"
            );
        }
    }
}

#[test]
fn nested_qualified_indices_keep_legacy_broad_paths_and_spans() {
    let p = parsed("obj.Values[obj.Indices[i]]");
    let spans = p.rvalue_identifier_spans_on_lines(&p.all_functions()[0], &BTreeSet::from([3]));
    let paths: BTreeSet<_> = spans.iter().map(|s| s.path.to_string()).collect();
    assert_eq!(
        paths,
        BTreeSet::from(["sink".into(), "Values".into(), "Indices".into(), "i".into()])
    );
    for name in ["Values", "Indices", "i"] {
        let start = if name == "i" {
            p.source.find("[i]").unwrap() + 1
        } else {
            p.source.find(name).unwrap()
        };
        assert!(
            spans.iter().any(|s| s.path.to_string() == name
                && s.start_byte == start
                && s.end_byte == start + name.len()),
            "{name}: {spans:?}"
        );
    }
}

#[test]
fn ordinary_index_expressions_and_non_type_subscripts_remain_references() {
    for (expression, names) in [
        ("a[i]", vec!["a", "i"]),
        ("a[b[0]]", vec!["a", "b"]),
        ("a[b[i+1]]", vec!["a", "b", "i"]),
        ("obj.Values[obj.Other.Indices[i]]", vec!["obj", "i"]),
        // An extra bracket after a complete instantiation is an ordinary index.
        ("matrix[i][j][k]", vec!["k"]),
    ] {
        let p = parsed(expression);
        for name in names {
            assert!(
                p.find_variable_references(&p.all_functions()[0], name)
                    .contains(&3),
                "{expression}: {name}"
            );
        }
    }
}

#[test]
fn nested_index_dfg_does_not_add_legacy_suppressed_def_use_edges() {
    let source = "package p\nfunc run(a,b []int) {\n i := 1\n sink(a[b[i]])\n sink(a[b[i+1]])\n}\n";
    let p = ParsedFile::parse("p.go", source, Language::Go).unwrap();
    let dfg = DataFlowGraph::build(&BTreeMap::from([("p.go".into(), p)]));
    let reads: BTreeSet<_> = dfg
        .edges()
        .iter()
        .filter(|e| e.from.function == "run" && e.from.path.to_string() == "i" && e.from.line == 3)
        .map(|e| e.to.line)
        .collect();
    assert!(!reads.contains(&4), "legacy type-token query: {reads:?}");
    assert!(
        reads.contains(&5),
        "non-type subscript stays a read: {reads:?}"
    );
}

#[test]
fn nested_index_callees_keep_value_prefix_and_type_argument_boundary() {
    let source = "package p\nfunc run(handlers []func(), keys []int, matrix [][]func(), i,j int) {\n handlers[keys[i]]()\n matrix[i][j]()\n}\n";
    let p = ParsedFile::parse("p.go", source, Language::Go).unwrap();
    let function = p.all_functions()[0];
    for (name, line, expected) in [
        ("handlers", 3, true),
        ("keys", 3, false),
        ("i", 3, false),
        ("matrix", 4, true),
        ("i", 4, true),
        ("j", 4, false),
    ] {
        assert_eq!(
            p.find_variable_references(&function, name).contains(&line),
            expected,
            "{name}:{line}"
        );
    }
    // 0.25 itself retains the legacy type_arguments branch for the first
    // call. The semantic helper is unchanged by this query compatibility fix.
    let calls = p.function_calls_on_lines_with_qualifier(&function, &BTreeSet::from([3, 4]));
    assert_eq!(
        calls.iter().map(|c| c.0.as_str()).collect::<Vec<_>>(),
        ["handlers", "matrix[i][j]"]
    );
}
