use super::*;

fn descendant<'a>(node: Node<'a>, kind: &str) -> Node<'a> {
    fn find<'a>(node: Node<'a>, kind: &str) -> Option<Node<'a>> {
        if node.kind() == kind {
            return Some(node);
        }
        let mut cursor = node.walk();
        let found = node
            .children(&mut cursor)
            .find_map(|child| find(child, kind));
        found
    }
    find(node, kind).unwrap()
}

#[test]
fn erased_roots_and_interior_arguments_are_refused_by_each_private_collector() {
    for language in [Language::TypeScript, Language::Tsx] {
        let source = "function owner() { type T = typeof import(\"./m\", { with: { \"resolution-mode\": \"import\" } }); }";
        let parsed = ParsedFile::parse("s.ts", source, language).unwrap();
        assert_eq!(parsed.parse_error_count, 0);
        for kind in [
            "type_alias_declaration",
            "type_query",
            "arguments",
            "object",
            "property_identifier",
        ] {
            let node = descendant(parsed.tree.root_node(), kind);
            let mut names = vec![];
            let mut paths = vec![];
            let mut spans = vec![];
            parsed.collect_all_identifiers(node, &mut names);
            parsed.collect_identifier_paths(node, &mut paths);
            parsed.collect_identifier_path_spans(node, &mut spans);
            assert!(
                names.is_empty() && paths.is_empty() && spans.is_empty(),
                "{kind}: {names:?} {paths:?} {spans:?}"
            );
        }
    }
}

#[test]
fn manual_and_query_rvalue_routes_preserve_runtime_values_and_raw_trees() {
    for language in [Language::TypeScript, Language::Tsx] {
        let source = "function owner() { result = runtime as typeof import(\"./m\", { with: { \"resolution-mode\": \"import\" } }); sink(runtime); }";
        let parsed = ParsedFile::parse("s.ts", source, language).unwrap();
        assert_eq!(parsed.parse_error_count, 0);
        let root = parsed.tree.root_node();
        let syntax = root.to_sexp();
        let lines = BTreeSet::from([1]);
        let mut names = vec![];
        let mut paths = vec![];
        let mut spans = vec![];
        parsed.collect_rvalues_manual(root, &lines, &mut names);
        parsed.collect_rvalue_paths_manual(root, &lines, &mut paths);
        parsed.collect_rvalue_spans_manual(root, &lines, &mut spans);
        assert!(
            names
                .iter()
                .all(|(name, _)| ["runtime", "sink"].contains(&name.as_str())),
            "{names:?}"
        );
        assert!(names.iter().any(|(name, _)| name == "runtime"));
        let mut query_names = parsed.rvalue_identifiers_on_lines(&root, &lines);
        names.sort();
        query_names.sort();
        assert_eq!(names, query_names);
        let mut query_paths = parsed.rvalue_identifier_paths_on_lines(&root, &lines);
        paths.sort();
        query_paths.sort();
        assert_eq!(paths, query_paths);
        let mut query_spans = parsed.rvalue_identifier_spans_on_lines(&root, &lines);
        spans.sort_by_key(|s| (s.start_byte, s.end_byte));
        query_spans.sort_by_key(|s| (s.start_byte, s.end_byte));
        assert_eq!(spans, query_spans);
        assert_eq!(root.to_sexp(), syntax);
        assert_eq!(parsed.source, source);
    }
}
