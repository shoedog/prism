use super::*;
use crate::data_flow::DataFlowGraph;

#[test]
fn producer_shares_one_member_index_across_functions() {
    for functions in [0, 1, 4] {
        let source: String = (0..functions)
            .map(|i| {
                format!("function f{i}(obj) {{\n obj.x = 1;\n use(obj.x);\n return obj.x;\n}}\n")
            })
            .collect();
        let p = ParsedFile::parse("producer.js", &source, Language::JavaScript).unwrap();
        let mut nodes = vec![p.tree.root_node()];
        let mut members = 0;
        while let Some(n) = nodes.pop() {
            members += usize::from(ParsedFile::is_field_access_node(n.kind()));
            let mut cursor = n.walk();
            nodes.extend(n.children(&mut cursor));
        }
        let files = BTreeMap::from([("producer.js".into(), p)]);
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(1)
            .build()
            .unwrap();
        let (edges, normalizations) = pool.install(|| {
            NORMALIZATIONS.with(|c| c.set(0));
            let graph = DataFlowGraph::build(&files);
            let edges = graph.edges().len();
            (edges, NORMALIZATIONS.with(|c| c.get()))
        });
        assert_eq!(edges, functions * 2);
        assert_eq!(
            normalizations, members,
            "one normalization per file member across {functions} functions"
        );
    }
}

#[test]
fn complete_dfg_fixture_sweep_against_fully_legacy_path() {
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .stack_size(64 * 1024 * 1024)
        .build()
        .unwrap();
    pool.install(|| {
        let mut pending =
            vec![std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures")];
        let mut files_checked = 0;
        let mut owners_checked = 0;
        while let Some(path) = pending.pop() {
            if path.is_dir() {
                for entry in std::fs::read_dir(path).unwrap() {
                    let entry = entry.unwrap();
                    let name = entry.file_name();
                    let name = name.to_string_lossy();
                    if name == "frontend-portal"
                        || name.starts_with("f-")
                        || name.starts_with("F-private")
                        || name.contains("SecBench")
                    {
                        continue;
                    }
                    pending.push(entry.path());
                }
                continue;
            }
            let Some(lang) = Language::from_path(&path.to_string_lossy()) else {
                continue;
            };
            let Ok(source) = std::fs::read_to_string(&path) else {
                continue;
            };
            let parsed = ParsedFile::parse(&path.to_string_lossy(), &source, lang).unwrap();
            owners_checked += parsed.all_functions().len();
            let files = BTreeMap::from([(parsed.path.clone(), parsed)]);
            let build = || bincode::serialize(&DataFlowGraph::build(&files)).unwrap();
            let indexed = build();
            USE_ORACLE.with(|f| f.set(true));
            let legacy = crate::ast::binding_verdict_memo::direct(build);
            USE_ORACLE.with(|f| f.set(false));
            assert_eq!(
                indexed,
                legacy,
                "complete serialized DFG: {}",
                path.display()
            );
            files_checked += 1;
        }
        println!("complete DFG differential: files={files_checked}, owners={owners_checked}");
        assert!(files_checked > 100 && owners_checked > 100);
    });
}
