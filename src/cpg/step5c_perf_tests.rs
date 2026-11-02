//! Same-owner selection cost and multi-file Go emission-order regressions.
use super::*;

#[test]
fn same_owner_query_work_is_bounded_by_selected_spans() {
    let source = format!("function f(x) {{ return {}; }}", vec!["x"; 256].join(" + "));
    let parsed = ParsedFile::parse("a.js", &source, Language::JavaScript).unwrap();
    let owner = FunctionId {
        file: "a.js".into(),
        name: "f".into(),
        start_line: 1,
        end_line: 1,
    };
    let mut graph = DiGraph::new();
    let spans: Vec<_> = source
        .match_indices('x')
        .skip(1)
        .map(|(start, _)| {
            let idx = graph.add_node(CpgNode::Variable {
                file: owner.file.clone(),
                function: owner.name.clone(),
                function_start_line: 1,
                line: 1,
                path: AccessPath::simple("x"),
                access: VarAccess::Use,
                start_byte: start,
                end_byte: start + 1,
            });
            (start, idx)
        })
        .collect();
    let index = uses::UseIndex::new(&graph);
    let (_, work) = measured(|| {
        for &(start, idx) in &spans {
            assert_eq!(index.within(&owner, start, start + 1, &parsed), [idx]);
        }
    });
    assert!(
        work <= spans.len() * 2,
        "same-owner queries visited {work} candidates for {} spans",
        spans.len()
    );
}

#[test]
fn repeated_span_queries_walk_syntax_once_including_rejections() {
    let source = "function f(x) { return g(x); }";
    let parsed = ParsedFile::parse("a.js", source, Language::JavaScript).unwrap();
    let owner = FunctionId {
        file: "a.js".into(),
        name: "f".into(),
        start_line: 1,
        end_line: 1,
    };
    let mut graph = DiGraph::new();
    let mut add = |start, end| {
        graph.add_node(CpgNode::Variable {
            file: owner.file.clone(),
            function: owner.name.clone(),
            function_start_line: 1,
            line: 1,
            path: AccessPath::simple("x"),
            access: VarAccess::Use,
            start_byte: start,
            end_byte: end,
        })
    };
    let callee = source.find("g(").unwrap();
    let value = source.rfind('x').unwrap();
    add(callee, callee + 1);
    let expected = add(value, value + 1);
    let index = uses::UseIndex::new(&graph);
    SEMANTIC.with(|c| c.set(0));
    for _ in 0..16 {
        assert_eq!(index.within(&owner, callee, value + 1, &parsed), [expected]);
    }
    assert_eq!(
        SEMANTIC.with(Cell::get),
        2,
        "accepted and rejected semantics must both be cached"
    );
}

#[test]
fn range_index_matches_legacy_predicate_for_all_boundaries() {
    let source = "function f(x) { return g(x); }";
    let parsed = ParsedFile::parse("a.js", source, Language::JavaScript).unwrap();
    let owner = FunctionId {
        file: "a.js".into(),
        name: "f".into(),
        start_line: 1,
        end_line: 1,
    };
    let mut graph = DiGraph::new();
    let value = source.rfind('x').unwrap();
    // Deliberately non-byte insertion order, duplicate and reversed spans.
    let spans = [
        (value, value + 1),
        (value + 1, value),
        (value, value),
        (0, source.len()),
        (value, value + 1),
        (source.len() + 2, source.len() + 3),
    ];
    for (start_byte, end_byte) in spans {
        graph.add_node(CpgNode::Variable {
            file: owner.file.clone(),
            function: owner.name.clone(),
            function_start_line: 1,
            line: 1,
            path: AccessPath::simple("x"),
            access: VarAccess::Use,
            start_byte,
            end_byte,
        });
    }
    let index = uses::UseIndex::new(&graph);
    for start in 0..=source.len() + 3 {
        for end in 0..=source.len() + 3 {
            let expected: Vec<_> = graph
                .node_indices()
                .filter(|idx| {
                    let (a, b) = spans[idx.index()];
                    start <= a && b <= end && parsed.is_semantic_value_use(a, b)
                })
                .collect();
            assert_eq!(
                index.within(&owner, start, end, &parsed),
                expected,
                "range {start}..{end}"
            );
        }
    }
}

fn go_files(count: usize, callers: usize) -> BTreeMap<String, ParsedFile> {
    let mut files = BTreeMap::new();
    for file in 0..count {
        let path = format!("p/file{file:03}.go");
        let mut source = format!(
            "package p\ntype T{file} struct {{}}\ntype I{file} interface {{ M{file}(int) int }}\n\
             func (T{file}) M{file}(x int) int {{ return x + 1 }}\n\
             func f{file}(x int) int {{ return x }}\n\
             func pair{file}(x int) (int, error) {{ return x, nil }}\n"
        );
        for caller in 0..callers {
            source.push_str(&format!(
                "func run{file}_{caller}(x int, t T{file}, i I{file}) int {{\n\
                 a := f{file}(x)\n b, err := pair{file}(x)\n c := t.M{file}(a)\n\
                 d := i.M{file}(b)\n nested := f{file}(f{file}(d))\n\
                 missing := unknown(x)\n return a+b+c+d+nested+missing\n}}\n"
            ));
        }
        files.insert(
            path.clone(),
            ParsedFile::parse(&path, &source, Language::Go).unwrap(),
        );
    }
    files
}

#[test]
fn multifile_go_complete_identity() {
    crate::build_pool::install(|| {
        let files = go_files(8, 8);
        let cpg = compare(&files);
        assert!(cpg.return_flow_stats.return_flow_edges >= 8 * 8 * 5);
        assert!(cpg.return_flow_stats.return_flow_skipped_nameonly >= 8 * 8);
    });
}

#[test]
#[ignore = "subprocess helper for whole_graph_is_identical_at_one_and_four_threads"]
fn thread_graph_child() {
    let path = std::env::var_os("PRISM_STEP5C_GRAPH_BYTES").expect("child output path");
    crate::build_pool::install(|| {
        let graph = compare(&go_files(8, 8));
        let bytes = bincode::serialize(&(
            fieldwise_payload(&graph),
            lookup_indexes(&graph),
            cli_bytes(&graph),
        ))
        .unwrap();
        std::fs::write(path, bytes).unwrap();
    });
}

#[test]
fn whole_graph_is_identical_at_one_and_four_threads() {
    let dir = tempfile::tempdir().unwrap();
    let mut outputs = Vec::new();
    for threads in [1, 4] {
        let path = dir.path().join(format!("threads-{threads}.bin"));
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "cpg::build::step5c::tests::perf::thread_graph_child",
                "--ignored",
                "--nocapture",
            ])
            .env("RAYON_NUM_THREADS", threads.to_string())
            .env("PRISM_STEP5C_GRAPH_BYTES", &path)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "child: {} {}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        // Reading the artifact also guards against an incorrect test filter.
        outputs.push(std::fs::read(path).unwrap());
    }
    assert_eq!(outputs[0], outputs[1]);
}

#[test]
fn retained_site_layout_excludes_unused_resolution_evidence() {
    let bytes = std::mem::size_of::<facts::SiteFacts<'_>>();
    let compact = std::mem::size_of::<Vec<crate::ast::PathSpan>>()
        + std::mem::size_of::<Vec<crate::resolution::ResolvedCallee<'_>>>()
        + std::mem::size_of::<Vec<NodeIndex>>();
    eprintln!(
        "retained site: {bytes} B; compact budget: {compact} B; outcome: {} B; callee: {} B; LHS: {} B",
        std::mem::size_of::<ResolutionOutcome<'_>>(),
        std::mem::size_of::<crate::resolution::ResolvedCallee<'_>>(),
        std::mem::size_of::<crate::ast::PathSpan>(),
    );
    assert!(bytes <= compact, "unused resolution evidence is retained");
}

#[test]
fn retained_sites_exclude_assignment_rejections() {
    let files = callee_cases::parsed(
        "go",
        Language::Go,
        "package p\nfunc f(x int) int { return x }\nfunc empty(x int) { f(x) }\n\
         func run(x int) int {\n f(x)\n a := f(f(x))\n b := unknown(x)\n return f(a+b)\n}\n",
    );
    let cg = CallGraph::build(&files);
    let resolver = crate::resolution::go_satisfiers::GoSatisfierMemo::new(&cg);
    let index = uses::UseIndex::new(&DiGraph::new());
    let mut rejected = 0;
    for (caller, facts) in facts::collect(&resolver, &files, &index) {
        let parsed = &files[&caller.file];
        let sites = &cg.calls[caller];
        let expected = sites
            .iter()
            .filter(|site| {
                parsed
                    .assignment_for_exact_call(site.start_byte, site.end_byte)
                    .is_some()
            })
            .count();
        rejected += sites.len() - expected;
        assert_eq!(
            facts.len(),
            expected,
            "rejected sites retained for {caller:?}"
        );
    }
    assert!(rejected >= 4, "fixture must exercise rejected call sites");
    // Missing parsed files reject every site without resolution or replay work.
    assert!(facts::collect(&resolver, &BTreeMap::new(), &index)
        .iter()
        .all(|(_, facts)| facts.is_empty()));
}
