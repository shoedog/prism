//! Complete-assembly differential observations and an algorithmic work bound.
use super::*;
use crate::cpg::dfg_observation::{fieldwise_payload, lookup_indexes};
use crate::languages::Language;
use std::cell::Cell;
use std::path::Path;

thread_local! { static WORK: Cell<usize> = const { Cell::new(0) }; }
pub(crate) fn record_work(n: usize) {
    WORK.with(|c| c.set(c.get() + n));
}
fn measured<T>(f: impl FnOnce() -> T) -> (T, usize) {
    WORK.with(|c| c.set(0));
    let result = f();
    (result, WORK.with(Cell::get))
}

fn cli_bytes(cpg: &CodePropertyGraph) -> Vec<u8> {
    use crate::navigation::queries;
    let mut stats = queries::call_stats(&cpg.call_graph);
    stats["return_flow"] = serde_json::to_value(&cpg.return_flow_stats).unwrap();
    stats["dfg_labels"] = serde_json::to_value(&cpg.dfg_label_stats).unwrap();
    let mut bytes = serde_json::to_vec_pretty(&stats).unwrap();
    bytes.push(b'\n');
    for row in queries::call_site_dump_iter(&cpg.call_graph) {
        serde_json::to_writer(&mut bytes, &row).unwrap();
        bytes.push(b'\n');
    }
    for row in queries::dfg_edge_dump(cpg) {
        serde_json::to_writer(&mut bytes, &row).unwrap();
        bytes.push(b'\n');
    }
    bytes.extend(bincode::serialize(&cpg.dfg).unwrap());
    bytes
}

fn compare(files: &BTreeMap<String, ParsedFile>) -> CodePropertyGraph {
    let cg = CallGraph::build(files);
    let dfg = DataFlowGraph::build(files);
    let candidate = CodePropertyGraph::assemble_graph(cg.clone(), dfg.clone(), files, None);
    let reference = step5c_reference::with_reference(|| {
        CodePropertyGraph::assemble_graph(cg, dfg, files, None)
    });
    assert_eq!(fieldwise_payload(&candidate), fieldwise_payload(&reference));
    assert_eq!(lookup_indexes(&candidate), lookup_indexes(&reference));
    assert_eq!(candidate.return_flow_stats, reference.return_flow_stats);
    assert_eq!(candidate.dfg_label_stats, reference.dfg_label_stats);
    assert_eq!(cli_bytes(&candidate), cli_bytes(&reference));
    candidate
}

#[test]
fn generated_complete_identity() {
    crate::build_pool::install(|| {
        for (ext, language, source) in callee_cases::languages() {
            compare(&callee_cases::parsed(ext, language, source));
        }
        for sites in [1, 16, 64] {
            let mut source =
                "function helper(x) { return x + x; }\nfunction caller(x, obj) {\n".to_string();
            for i in 0..sites {
                source.push_str(&format!("let a{i} = helper(x);\n"));
            }
            source.push_str("obj.field = helper(x); return helper(x); }\n");
            let cpg = compare(&callee_cases::parsed("js", Language::JavaScript, &source));
            assert_eq!(cpg.return_flow_stats.return_flow_edges, sites);
            assert_eq!(cpg.return_flow_stats.return_input_edges, 2);
            assert_eq!(cpg.return_flow_stats.return_flow_skipped_non_simple_lhs, 1);
        }
        compare(&callee_cases::parsed("py", Language::Python,
            "def self_rec(x):\n    return self_rec(x)\ndef a(x):\n    return b(x)\ndef b(x):\n    return a(x)\ndef run(x):\n    y = self_rec(x)\n    z = a(x)\n    return y + z\n"));
        let multi = compare(&callee_cases::parsed("go", Language::Go,
            "package main\ntype I interface { F(string) string }\ntype A struct{}\ntype B struct{}\nfunc (A) F(x string) string { return x }\nfunc (B) F(x string) string { return x }\nfunc run(i I, user string) {\n value := i.F(user)\n sink(value)\n}\n"));
        assert_eq!(multi.return_flow_stats.return_flow_skipped_multi, 1);
        let nameonly = compare(&callee_cases::parsed("rs", Language::Rust,
            "trait Transform {\n fn f(&self, x: String) -> String { x }\n}\nfn run(t: &dyn Transform, user: String) {\n let value = t.f(user);\n sink(value);\n}\n"));
        assert_eq!(nameonly.return_flow_stats.return_flow_skipped_nameonly, 1);
    });
}

#[test]
fn anonymous_and_refusal_identity() {
    crate::build_pool::install(|| {
        let files = BTreeMap::from([
            ("callee.js".into(), ParsedFile::parse("callee.js",
                "export function free(x) { return x; }\n", Language::JavaScript).unwrap()),
            ("caller.js".into(), ParsedFile::parse("caller.js",
                "function local(x) { return x + 1; }\nfunction bridge(x) { let a = free(x); return a; }\nfunction run(x) {\n sink(x => { let a = bridge(x); return a; });\n sink(x => { let a = local(x); return a; });\n let a = local(x); return a;\n}\n", Language::JavaScript).unwrap()),
        ]);
        let cpg = compare(&files);
        let mut outgoing = BTreeMap::new();
        for n in cpg.graph.node_indices() {
            if let CpgNode::Variable {
                file,
                function,
                path,
                access: VarAccess::Def,
                ..
            } = &cpg.graph[n]
            {
                if file == "caller.js"
                    && path.base == "x"
                    && crate::anon_owner::is_anonymous_owner_name(function)
                {
                    outgoing.insert(
                        function.clone(),
                        cpg.graph
                            .edges(n)
                            .filter(|e| matches!(e.weight(), CpgEdge::DataFlow(_)))
                            .count(),
                    );
                }
            }
        }
        assert!(
            outgoing.values().any(|&n| n == 0),
            "reachable source must be refused: {outgoing:?}"
        );
        assert!(
            outgoing.values().any(|&n| n > 0),
            "non-reachable source must retain flow: {outgoing:?}"
        );
        assert!(cpg.return_flow_stats.return_flow_edges > 0);
    });
}

fn fixtures(root: &Path, files: &mut BTreeMap<String, ParsedFile>, base: &Path) {
    let mut entries: Vec<_> = std::fs::read_dir(root)
        .unwrap()
        .map(Result::unwrap)
        .collect();
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        let kind = entry.file_type().unwrap();
        if kind.is_dir() {
            fixtures(&entry.path(), files, base);
        } else if kind.is_file() {
            let path = entry.path();
            let relative = path.strip_prefix(base).unwrap().to_str().unwrap();
            if let Some(language) = Language::from_path(relative) {
                files.insert(
                    relative.into(),
                    ParsedFile::parse(relative, &std::fs::read_to_string(&path).unwrap(), language)
                        .unwrap(),
                );
            }
        }
    }
}

#[test]
fn repository_fixtures_complete_identity() {
    crate::build_pool::install(|| {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
        let mut files = BTreeMap::new();
        fixtures(&root, &mut files, &root);
        assert!(files.len() >= 100, "empty/truncated fixture population");
        for (file, parsed) in &files {
            let one = BTreeMap::from([(
                file.clone(),
                ParsedFile::parse(file, &parsed.source, parsed.language).unwrap(),
            )]);
            compare(&one);
        }
        compare(&files);
        eprintln!(
            "step5c identity: {} fixtures individually and together",
            files.len()
        );
    });
}

#[test]
fn owner_partition_keeps_order_duplicates_and_span_boundaries() {
    let source = "function f(x) { return x + x; } f(x);";
    let parsed = ParsedFile::parse("a.js", source, Language::JavaScript).unwrap();
    let positions: Vec<_> = source.match_indices('x').map(|(i, _)| i).collect();
    let (left, right) = (positions[1], positions[2]);
    let owner = FunctionId {
        file: "a.js".into(),
        name: "f".into(),
        start_line: 1,
        end_line: 1,
    };
    let mut graph = DiGraph::new();
    let mut add = |file: &str, line, access, start_byte, end_byte| {
        graph.add_node(CpgNode::Variable {
            file: file.into(),
            function: "f".into(),
            function_start_line: line,
            line: 1,
            path: AccessPath::simple("x"),
            access,
            start_byte,
            end_byte,
        })
    };
    let first = add("a.js", 1, VarAccess::Use, right, right + 1);
    let second = add("a.js", 1, VarAccess::Use, left, left + 1);
    let duplicate = add("a.js", 1, VarAccess::Use, left, left + 1);
    add("b.js", 1, VarAccess::Use, left, left + 1);
    add("a.js", 2, VarAccess::Use, left, left + 1);
    add("a.js", 1, VarAccess::Def, left, left + 1);
    add("a.js", 1, VarAccess::Use, left - 1, right + 2);
    let zero = add("a.js", 1, VarAccess::Use, left, left);
    let callee = source.rfind("f(").unwrap();
    add("a.js", 1, VarAccess::Use, callee, callee + 1);
    let index = uses::UseIndex::new(&graph);
    assert_eq!(
        index.within(&owner, left, right + 1, &parsed),
        [first, second, duplicate, zero]
    );
    assert_eq!(
        index.within(&owner, left, left + 1, &parsed),
        [second, duplicate, zero]
    );
    assert!(index
        .within(&owner, right + 1, right + 1, &parsed)
        .is_empty());
    assert!(index.within(&owner, callee, callee + 1, &parsed).is_empty());
    assert!(index
        .within(
            &FunctionId {
                name: "missing".into(),
                ..owner
            },
            left,
            right + 1,
            &parsed
        )
        .is_empty());
}

#[test]
fn query_work_excludes_unrelated_owners() {
    crate::build_pool::install(|| {
        let mut source = "function helper(x) { return x + 1; }\nfunction caller(x) {\n".to_string();
        for i in 0..32 {
            source.push_str(&format!("let a{i} = helper(x);\n"));
        }
        source.push_str("return x; }\n");
        for i in 0..128 {
            source.push_str(&format!(
                "function unrelated{i}(z) {{ return z + z + z; }}\n"
            ));
        }
        let files = callee_cases::parsed("js", Language::JavaScript, &source);
        let cg = CallGraph::build(&files);
        let dfg = DataFlowGraph::build(&files);
        let (candidate, work) =
            measured(|| CodePropertyGraph::assemble_graph(cg.clone(), dfg.clone(), &files, None));
        let (reference, old_work) = measured(|| {
            step5c_reference::with_reference(|| {
                CodePropertyGraph::assemble_graph(cg, dfg, &files, None)
            })
        });
        assert_eq!(fieldwise_payload(&candidate), fieldwise_payload(&reference));
        assert_eq!(candidate.return_flow_stats.return_flow_edges, 32);
        assert!(
            work < candidate.graph.node_count() * 4,
            "indexed selection visited {work} nodes"
        );
        assert!(
            old_work > work * 8,
            "main {old_work} vs indexed {work}: cost witness too weak"
        );
        eprintln!("step5c node visits: main={old_work}, indexed={work}");
    });
}

#[test]
fn shared_reads_are_sync() {
    fn assert_sync<T: Sync>() {}
    assert_sync::<CallGraph>();
    assert_sync::<ParsedFile>();
    assert_sync::<callee_returns::ReturnCache<'_>>();
}
