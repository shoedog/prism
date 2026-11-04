//! Small-cap contracts; None runs the original single emission pass as oracle.
use super::{FileRefs, Producer};
use crate::ast::ParsedFile;
use crate::cpg::{CodePropertyGraph, FlowConfidence, RdFileStats};
use crate::cpg_cache::{self, CacheResult};
use crate::data_flow::{compact::CompactDfg, DataFlowGraph};
use crate::languages::Language;
use std::collections::BTreeMap;

const SOURCE: &str =
    "def capped(x, y, z): return x + y + z\n\ndef other(a):\n    b = a\n    return b\n";

fn refs(source: &str, file: &str, lang: Language, cap: Option<usize>) -> FileRefs {
    let parsed = ParsedFile::parse(file, source, lang).unwrap();
    Producer::file_refs_with_cap(&file.to_string(), &parsed, cap)
}

fn compact(refs: FileRefs) -> DataFlowGraph {
    DataFlowGraph {
        storage: CompactDfg::from_parts(refs.into_parts()).unwrap(),
    }
}

fn bytes(refs: FileRefs) -> Vec<u8> {
    bincode::serialize(&compact(refs)).unwrap()
}

#[test]
fn fallback_cap_unavailable_keeps_locations_and_other_owner() {
    let old = refs(SOURCE, "a.py", Language::Python, None);
    let count = old
        .edges
        .iter()
        .filter(|e| e.from.function == "capped")
        .count();
    assert_eq!(count, 3);
    assert!(old.rd_function_stats.functions_without_cfg > 0);
    let new = refs(SOURCE, "a.py", Language::Python, Some(count - 1));
    assert_eq!(new.defs, old.defs);
    assert_eq!(new.uses, old.uses);
    assert_eq!(
        new.edges,
        old.edges
            .iter()
            .filter(|e| e.from.function != "capped")
            .cloned()
            .collect::<Vec<_>>()
    );
    assert_eq!(
        new.labels,
        old.labels
            .into_iter()
            .filter(|((f, _), _)| f.function != "capped")
            .collect()
    );
    assert_eq!(
        new.exact_labels,
        old.exact_labels
            .into_iter()
            .filter(|(e, _)| e.from.function != "capped")
            .collect()
    );
    assert_eq!(new.rd_function_stats.fallback_functions_capped, 1);
    assert_eq!(new.rd_function_stats.fallback_edges_capped, count);
    assert_eq!(new.rd_function_stats.functions_over_cap, 0);
    assert_eq!(new.rd_function_stats.functions_without_cfg, 1);
}

#[test]
fn fallback_cap_boundary_is_strict_and_available_owner_is_exempt() {
    let old = refs(SOURCE, "a.py", Language::Python, None);
    let count = old
        .edges
        .iter()
        .filter(|e| e.from.function == "capped")
        .count();
    let expected = bytes(old);
    // Exactly cap, and one below cap, preserve the entire serialized graph.
    for cap in [count, count + 1] {
        assert_eq!(
            bytes(refs(SOURCE, "a.py", Language::Python, Some(cap))),
            expected
        );
    }
    let source = "def available(x):\n    a = x\n    b = a\n    return b\n";
    let old = refs(source, "a.py", Language::Python, None);
    assert_eq!(old.rd_function_stats.functions_without_cfg, 0);
    assert!(!old.edges.is_empty());
    assert!(old.labels.values().any(|c| *c == FlowConfidence::Exact));
    assert_eq!(
        bytes(refs(source, "a.py", Language::Python, Some(0))),
        bytes(old)
    );
}

#[test]
fn fallback_cap_defs_and_statement_limits_keep_locations() {
    let mut defs = String::from("package p\nfunc capped() {\n");
    for i in 0..2050 {
        defs += &format!("v{i} := {i}\nuse(v{i})\n");
    }
    defs += "}\n";
    let mut lines = String::from("package p\nfunc capped(x int) {\n");
    for _ in 0..4100 {
        lines += "use(x)\n";
    }
    lines += "}\n";
    for source in [defs, lines] {
        let old = refs(&source, "a.go", Language::Go, None);
        assert_eq!(old.rd_function_stats.functions_over_cap, 1);
        assert!(!old.edges.is_empty());
        let new = refs(&source, "a.go", Language::Go, Some(1));
        assert!(new.edges.is_empty());
        assert!(new.labels.is_empty());
        assert!(new.exact_labels.is_empty());
        assert_eq!(new.defs, old.defs);
        assert_eq!(new.uses, old.uses);
        assert_eq!(new.rd_function_stats.fallback_functions_capped, 1);
        assert_eq!(new.rd_function_stats.fallback_edges_capped, old.edges.len());
        assert_eq!(new.rd_function_stats.functions_over_cap, 1);
        assert_eq!(new.rd_function_stats.functions_without_cfg, 0);
    }
}

#[test]
fn fallback_cap_codec_and_cold_cache_hit_agree() {
    let sources = BTreeMap::from([("a.py".to_string(), SOURCE.to_string())]);
    let files = BTreeMap::from([(
        "a.py".to_string(),
        ParsedFile::parse("a.py", SOURCE, Language::Python).unwrap(),
    )]);
    let mut cpg = CodePropertyGraph::build(&files);
    let old_variables = bincode::serialize(
        &cpg.graph
            .node_weights()
            .filter(|n| matches!(n, crate::cpg::CpgNode::Variable { .. }))
            .collect::<Vec<_>>(),
    )
    .unwrap();
    cpg.dfg = compact(refs(SOURCE, "a.py", Language::Python, Some(2)));
    // Rebuild derived CPG nodes, edges, and nested label counters from the capped cold DFG.
    let cpg = CodePropertyGraph::build_incremental(
        cpg.call_graph,
        cpg.dfg,
        &Default::default(),
        &files,
        None,
    );
    assert_eq!(
        old_variables,
        bincode::serialize(
            &cpg.graph
                .node_weights()
                .filter(|n| matches!(n, crate::cpg::CpgNode::Variable { .. }))
                .collect::<Vec<_>>()
        )
        .unwrap()
    );
    assert_eq!(cpg.dfg_label_stats.dfg_fallback_functions_capped, 1);
    assert_eq!(cpg.dfg_label_stats.dfg_fallback_edges_capped, 3);
    let graph = bincode::serialize(&cpg.dfg).unwrap();
    let decoded: DataFlowGraph = bincode::deserialize(&graph).unwrap();
    assert_eq!(graph, bincode::serialize(&decoded).unwrap());
    assert_eq!(decoded.rd_function_stats(), cpg.dfg.rd_function_stats());
    let dir = tempfile::tempdir().unwrap();
    let hashes = cpg_cache::compute_file_hashes(&sources);
    cpg_cache::save_cache(&cpg, &hashes, false, dir.path()).unwrap();
    let CacheResult::Hit(hit) = cpg_cache::load_cache(&hashes, false, dir.path()) else {
        panic!("cache miss")
    };
    assert_eq!(hit.dfg_label_stats, cpg.dfg_label_stats);
    assert_eq!(graph, bincode::serialize(&hit.dfg).unwrap());
}

#[test]
fn fallback_cap_stats_merge_is_idempotent() {
    let mut a = RdFileStats::default();
    a.record_fallback_capped("f".into(), 1, 0, 3);
    a.merge(a.clone());
    assert_eq!(
        (a.fallback_functions_capped, a.fallback_edges_capped),
        (1, 3)
    );
    let mut b = RdFileStats::default();
    b.record_fallback_capped("f".into(), 1, 10, 4);
    a.merge(b);
    assert_eq!(
        (a.fallback_functions_capped, a.fallback_edges_capped),
        (2, 7)
    );
    assert!(a.counts_match_keys());
    let json = serde_json::to_vec(&a).unwrap();
    assert_eq!(serde_json::from_slice::<RdFileStats>(&json).unwrap(), a);
}

#[test]
fn fallback_cap_counting_never_constructs_an_edge() {
    let mut edges = Vec::new();
    let mut counter = super::fallback_cap::Emission::new(&mut edges, true);
    for _ in 0..10 {
        counter.push(|| panic!("counting constructed a FlowEdge"));
    }
    assert_eq!(counter.count(), 10);
    drop(counter);
    assert!(edges.is_empty());
}

#[test]
fn fallback_cap_threads_one_and_four_agree() {
    use rayon::prelude::*;
    let mut result = Vec::new();
    for threads in [1, 4] {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .stack_size(32 * 1024 * 1024)
            .build()
            .unwrap();
        let graphs = pool.install(|| {
            (0..8)
                .into_par_iter()
                .map(|i| bytes(refs(SOURCE, &format!("f{i}.py"), Language::Python, Some(2))))
                .collect::<Vec<_>>()
        });
        result.push(graphs);
    }
    assert_eq!(result[0], result[1]);
}

#[test]
fn fallback_cap_projection_and_synthetic_replay_keep_every_location() {
    let cases = [
        (
            "a.py",
            Language::Python,
            "def projected(obj): return obj.a + obj.b\n",
        ),
        // #599: a one-statement owner with distinct names now runs reaching
        // definitions, so only owners that define a name twice (here a
        // member write to the formal) are still RD-unavailable and cap-eligible.
        (
            "a.js",
            Language::JavaScript,
            "function projected(obj) { obj.c = 1; return obj.a + obj.b; }\n",
        ),
        (
            "a.js",
            Language::JavaScript,
            "const projected = (obj) => { obj.c = 1; return obj.a + obj.b; };\n",
        ),
        (
            "a.js",
            Language::JavaScript,
            "const read = (x) => { x = 1; };\n",
        ),
    ];
    for (file, lang, source) in cases {
        let old = refs(source, file, lang, None);
        assert!(old.rd_function_stats.functions_without_cfg > 0, "{source}");
        let old_bytes = bytes(refs(source, file, lang, None));
        assert_eq!(
            bytes(refs(source, file, lang, Some(usize::MAX))),
            old_bytes,
            "replay {source}"
        );
        let capped = refs(source, file, lang, Some(0));
        assert!(capped.edges.is_empty(), "{source}");
        assert!(capped.labels.is_empty(), "{source}");
        assert!(capped.exact_labels.is_empty(), "{source}");
        assert_eq!(capped.defs, old.defs, "{source}");
        assert_eq!(capped.uses, old.uses, "{source}");
        assert_eq!(
            (
                capped.rd_function_stats.functions_over_cap,
                capped.rd_function_stats.functions_without_cfg,
            ),
            (
                old.rd_function_stats.functions_over_cap,
                old.rd_function_stats.functions_without_cfg,
            ),
            "{source}"
        );
        assert_eq!(
            capped.rd_function_stats.fallback_functions_capped,
            usize::from(!old.edges.is_empty())
        );
        assert_eq!(
            capped.rd_function_stats.fallback_edges_capped,
            old.edges.len()
        );
    }
}

fn assert_anchor(refs: &FileRefs, parsed: &ParsedFile, line: usize) {
    use crate::data_flow::VarAccessKind;
    let uses: Vec<_> = refs
        .uses
        .values()
        .flatten()
        .filter(|loc| loc.function == "f" && loc.path.to_string() == "x" && loc.line == line)
        .collect();
    assert_eq!(uses.len(), 1, "exactly one assertion Use: {uses:?}");
    let loc = uses[0];
    assert_eq!(loc.kind, VarAccessKind::Use);
    assert_eq!(loc.function_start_line, 1);
    assert_eq!(loc.start_byte, parsed.line_start_byte(line));
    assert_eq!(loc.end_byte, loc.start_byte);
    assert!(refs.edges.iter().any(|edge| {
        edge.to.line == line && edge.to.path == loc.path && edge.to.kind == VarAccessKind::Use
    }));
}

#[test]
fn fallback_cap_available_owner_registers_assertion_use() {
    let source = "def f(x):\n    assert x\n    return 0\n";
    let parsed = ParsedFile::parse("a.py", source, Language::Python).unwrap();
    for cap in [None, Some(0)] {
        let result = Producer::file_refs_with_cap(&parsed.path, &parsed, cap);
        assert_eq!(result.rd_function_stats.functions_without_cfg, 0);
        assert_eq!(result.rd_function_stats.functions_over_cap, 0);
        assert_eq!(result.rd_function_stats.fallback_functions_capped, 0);
        assert_anchor(&result, &parsed, 2);
    }
}

#[test]
fn fallback_cap_unavailable_replay_registers_assertion_use_once() {
    let source = "def f(x):\n    assert x\n";
    let parsed = ParsedFile::parse("a.py", source, Language::Python).unwrap();
    let old = Producer::file_refs_with_cap(&parsed.path, &parsed, None);
    assert_anchor(&old, &parsed, 2);
    assert_eq!(old.edges.len(), 1);
    for cap in [1, usize::MAX] {
        let replay = Producer::file_refs_with_cap(&parsed.path, &parsed, Some(cap));
        assert_eq!(replay.rd_function_stats.functions_without_cfg, 1);
        assert_eq!(replay.rd_function_stats.fallback_functions_capped, 0);
        assert_anchor(&replay, &parsed, 2);
        assert_eq!(
            bytes(replay),
            bytes(refs(source, "a.py", Language::Python, None))
        );
    }
    let capped = Producer::file_refs_with_cap(&parsed.path, &parsed, Some(0));
    assert_eq!(capped.uses, old.uses);
    assert!(capped.edges.is_empty());
    assert_eq!(capped.rd_function_stats.functions_over_cap, 0);
    assert_eq!(capped.rd_function_stats.functions_without_cfg, 1);
}

fn accepted_at_zero_cap(parsed: &ParsedFile) {
    let old = Producer::file_refs_with_cap(&parsed.path, parsed, None);
    assert!(
        !old.edges.is_empty(),
        "cap mutation must have something to omit"
    );
    assert_eq!(old.rd_function_stats.functions_over_cap, 0);
    assert_eq!(old.rd_function_stats.functions_without_cfg, 0);
    let new = Producer::file_refs_with_cap(&parsed.path, parsed, Some(0));
    assert_eq!(new.rd_function_stats.fallback_functions_capped, 0);
    assert_eq!(new.rd_function_stats.functions_over_cap, 0);
    assert_eq!(new.rd_function_stats.functions_without_cfg, 0);
    assert_eq!(bytes(new), bytes(old));
}

#[test]
fn fallback_cap_accepts_exact_rd_definition_limit() {
    use crate::cpg::RD_MAX_DEFS;
    let mut source = String::from("def f():\n");
    for i in 0..RD_MAX_DEFS {
        source += &format!("    v{i} = {i}\n");
    }
    source += "    return v0\n";
    let parsed = ParsedFile::parse("a.py", &source, Language::Python).unwrap();
    let old = Producer::file_refs_with_cap(&parsed.path, &parsed, None);
    assert_eq!(old.defs.values().map(Vec::len).sum::<usize>(), RD_MAX_DEFS);
    accepted_at_zero_cap(&parsed);
}

#[test]
fn fallback_cap_accepts_exact_rd_statement_limit() {
    use crate::cpg::RD_MAX_LINES;
    let source = format!(
        "def f(x):\n    consume(x)\n{}",
        "    g()\n".repeat(RD_MAX_LINES - 1)
    );
    let parsed = ParsedFile::parse("a.py", &source, Language::Python).unwrap();
    assert_eq!(
        parsed
            .statements_in_function(&parsed.all_functions()[0])
            .len(),
        RD_MAX_LINES
    );
    accepted_at_zero_cap(&parsed);
}

#[test]
fn fallback_cap_deduplicates_raw_definitions_before_limit() {
    use crate::cpg::RD_MAX_DEFS;
    use std::collections::BTreeSet;
    let mut source = String::from("package p\nfunc f() {\n");
    for i in 0..(RD_MAX_DEFS / 2 + 1) {
        source += &format!("v{i} := {i}\n");
    }
    source += "consume(v0)\n}\n";
    let parsed = ParsedFile::parse("a.go", &source, Language::Go).unwrap();
    let old = Producer::file_refs_with_cap(&parsed.path, &parsed, None);
    let raw: Vec<_> = old.defs.values().flatten().collect();
    let unique: BTreeSet<_> = raw
        .iter()
        .map(|loc| (&loc.path, loc.line, loc.start_byte))
        .collect();
    // Go short declarations are observed by overlapping assignment captures.
    assert!(raw.len() > RD_MAX_DEFS, "raw definitions: {}", raw.len());
    assert_eq!(unique.len(), RD_MAX_DEFS / 2 + 1);
    accepted_at_zero_cap(&parsed);
}

#[test]
fn fallback_cap_never_applies_to_one_node_owners_that_run_reaching_definitions() {
    // #599: these owners were RD-unavailable (and so cap-eligible) before.
    for source in [
        "function projected(obj) { return obj.a + obj.b; }\n",
        "const projected = (obj) => obj.a + obj.b;\n",
    ] {
        let old = refs(source, "a.js", Language::JavaScript, None);
        assert_eq!(old.rd_function_stats.functions_without_cfg, 0, "{source}");
        assert!(!old.edges.is_empty(), "{source}");
        let capped = refs(source, "a.js", Language::JavaScript, Some(0));
        assert_eq!(bytes(capped), bytes(old), "{source}");
    }
}
