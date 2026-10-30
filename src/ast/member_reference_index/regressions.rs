use super::*;

#[test]
fn member_index_query_bound_and_ambiguous_ranges() {
    let source = (0..128)
        .map(|i| format!("function f{i}(obj) {{\n return obj.x;\n}}\n"))
        .collect::<String>();
    let parsed = ParsedFile::parse("bounds.js", &source, Language::JavaScript).unwrap();
    let owner = parsed.all_functions()[64];
    let search = MemberReferenceSearch::new(&parsed);
    let path = AccessPath::from_expr("obj.x");
    EXAMINED.with(|n| n.set(0));
    EXACT_CHECKS.with(|n| n.set(0));
    let matches: Vec<_> = search.matching(owner, &path, 0).collect();
    assert_eq!(matches.len(), 1);
    assert_eq!(
        EXAMINED.with(|n| n.get()),
        1,
        "examine only the scope's bucket interval"
    );
    assert_eq!(
        EXACT_CHECKS.with(|n| n.get()),
        0,
        "strict interior uses constant-time containment"
    );
    // A member used as the scope root has an equal range and must include itself.
    assert_eq!(
        search.matching(matches[0], &path, 0).collect::<Vec<_>>(),
        matches
    );
    assert_eq!(EXACT_CHECKS.with(|n| n.get()), 1);

    // Exercise exact identity for *all* roots/candidates, including missing nodes.
    // This tests the ambiguous fallback directly even if a grammar emits no
    // zero-width member-expression candidates for this malformed source.
    let malformed = ParsedFile::parse(
        "missing.js",
        "function f(obj) { return obj.x",
        Language::JavaScript,
    )
    .unwrap();
    let index = Index::build(&malformed);
    assert!(!index.by_path.is_empty());
    for candidate in index.by_path.values().flatten() {
        let mut cursor = index.root.walk();
        cursor.goto_descendant(candidate.preorder);
        assert_eq!(cursor.node(), candidate.node);
    }
    let mut nodes = Vec::new();
    let mut cursor = malformed.tree.root_node().walk();
    loop {
        nodes.push(cursor.node());
        if cursor.goto_first_child() {
            continue;
        }
        loop {
            if cursor.goto_next_sibling() {
                break;
            }
            if !cursor.goto_parent() {
                break;
            }
        }
        if cursor.node() == malformed.tree.root_node() {
            break;
        }
    }
    assert!(nodes.iter().any(|n| n.start_byte() == n.end_byte()));
    for (preorder, node) in nodes.iter().copied().enumerate() {
        let candidate = Candidate {
            node,
            preorder,
            subtree_end: preorder + 1,
        };
        for root in &nodes {
            let mut ancestor = Some(node);
            let mut expected = false;
            while let Some(n) = ancestor {
                if n == *root {
                    expected = true;
                    break;
                }
                ancestor = n.parent();
            }
            assert_eq!(index.exact_descendant(*root, &candidate), expected);
        }
    }
}

#[test]
fn member_index_deep_syntax_outside_owner() {
    const CHILD: &str = "PRISM_MEMBER_INDEX_STACK_CHILD";
    if std::env::var_os(CHILD).is_none() {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "ast::member_reference_index::regressions::member_index_deep_syntax_outside_owner",
                "--nocapture",
            ])
            .env(CHILD, "1")
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "child output:\n{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(String::from_utf8_lossy(&output.stdout)
            .contains("deep query and complete DFG verified"));
        return;
    }
    let source = format!(
        "function f(obj) {{\n obj.x = 1;\n return obj.x;\n}}\nconst unrelated = {}0{};\n",
        "[".repeat(4000),
        "]".repeat(4000)
    );
    let parsed = std::thread::Builder::new()
        .stack_size(64 * 1024 * 1024)
        .spawn(move || ParsedFile::parse("deep.js", &source, Language::JavaScript).unwrap())
        .unwrap()
        .join()
        .unwrap();
    assert_eq!(parsed.parse_error_count, 0);
    std::thread::Builder::new()
        .stack_size(2 * 1024 * 1024)
        .spawn(move || {
            let owner = parsed.all_functions()[0];
            assert_eq!(
                parsed.find_path_references_scoped(&owner, &AccessPath::from_expr("obj.x"), 2),
                BTreeSet::from([3])
            );
            assert!(parsed
                .find_path_references_scoped(&owner, &AccessPath::from_expr("absent.member"), 2)
                .is_empty());
            let files = BTreeMap::from([("deep.js".into(), parsed)]);
            let build = |legacy| {
                let pool = rayon::ThreadPoolBuilder::new()
                    .num_threads(1)
                    .stack_size(2 * 1024 * 1024)
                    .start_handler(move |_| USE_ORACLE.with(|flag| flag.set(legacy)))
                    .build()
                    .unwrap();
                let dfg =
                    pool.install(|| crate::data_flow::DataFlowGraph::build_on_current_pool(&files));
                assert_oracle_ran(&pool, legacy);
                dfg
            };
            let expected = build(true);
            let actual = build(false);
            assert_eq!(
                actual
                    .edges()
                    .iter()
                    .map(|e| (e.from.line, e.to.line))
                    .collect::<Vec<_>>(),
                [(2, 3)]
            );
            assert_eq!(
                bincode::serialize(&actual).unwrap(),
                bincode::serialize(&expected).unwrap(),
                "complete expected DFG, including spans, labels, adjacency and stats"
            );
            println!("deep query and complete DFG verified");
        })
        .unwrap()
        .join()
        .unwrap();
}
