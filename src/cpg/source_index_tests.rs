use super::*;
use crate::anon_owner::{candidate_nodes, identity, source_bytes, S4_CANDIDATES};
use crate::languages::Language;

#[test]
fn step1_enumerates_once_per_file_with_exact_bytes_and_missing_ids() {
    let empty = BTreeMap::new();
    let mut cg = CallGraph::build(&empty);
    let mut files = BTreeMap::new();
    let mut expected = Vec::new();
    for (file, source, lang) in [
        (
            "alias.js",
            "consume(() => 1, () => nested(() => 2));",
            Language::JavaScript,
        ),
        (
            "alias.ts",
            "consume(() => longerName, function*(){ yield 3; });",
            Language::TypeScript,
        ),
        (
            "alias.tsx",
            "consume(() => <X/>, () => 4444);",
            Language::Tsx,
        ),
    ] {
        let parsed = ParsedFile::parse("different-original-name", source, lang).unwrap();
        for node in candidate_nodes(&parsed, true) {
            let Some(id) = identity(&parsed, &node, file) else {
                continue;
            };
            expected.push((id.clone(), source_bytes(&parsed, &id).unwrap()));
            cg.functions.entry(id.name.clone()).or_default().push(id);
        }
        files.insert(file.to_owned(), parsed);
    }
    let mut missing = expected[0].0.clone();
    missing.end_line += 1;
    cg.functions
        .entry(missing.name.clone())
        .or_default()
        .push(missing.clone());
    expected.push((missing, (0, 0)));
    let mut absent_file = expected[0].0.clone();
    absent_file.file = "not-in-files".into();
    cg.functions
        .entry(absent_file.name.clone())
        .or_default()
        .push(absent_file.clone());
    expected.push((absent_file, (0, 0)));
    let dfg = DataFlowGraph::build(&empty);
    S4_CANDIDATES.with(|c| c.set(0));
    let cpg = CodePropertyGraph::assemble_graph(cg, dfg, &files, None);
    for (id, span) in expected {
        assert!(cpg.graph.node_weights().any(|n| matches!(n, CpgNode::Function {file, name, start_line, end_line, start_byte, end_byte}
            if (file, name, *start_line, *end_line, *start_byte, *end_byte) == (&id.file, &id.name, id.start_line, id.end_line, span.0, span.1))), "{id:?}");
    }
    assert_eq!(
        S4_CANDIDATES.with(|c| c.get()),
        3,
        "one candidate enumeration per file in Step 1"
    );
}

#[test]
fn step1_bytes_and_contains_identity_across_languages() {
    for (file, source, language) in [
        ("x.js", "function outer(){ consume((p) => nested((q) => p + q), function*(g){ yield g; }, function* named(n){ yield n; }); } function dup(){return 1;} function dup(){return 22;}", Language::JavaScript),
        ("x.ts", "function outer(){ consume((p: number) => nested((q: number) => p + q), function*(g: number){ yield g; }, function* named(n: number){ yield n; }); } function dup(){return 1;} function dup(){return 22;}", Language::TypeScript),
        ("x.tsx", "function outer(){ consume((p: number) => nested((q: number) => <X value={p+q}/>), function*(g: number){ yield g; }, function* named(n: number){ yield n; }); } function dup(){return 1;} function dup(){return 22;}", Language::Tsx),
    ] {
        let files = BTreeMap::from([(file.into(), ParsedFile::parse("original", source, language).unwrap())]);
        let parsed = &files[file];
        let cg = CallGraph::build(&files);
        let expected: Vec<_> = cg.functions.values().flatten().map(|id| (id.clone(), source_bytes(parsed, id).unwrap())).collect();
        assert!(expected.len() >= 6);
        let cpg = CodePropertyGraph::assemble_graph(cg, DataFlowGraph::build(&files), &files, None);
        for (id, span) in expected {
            let idx = cpg.graph.node_indices().find(|&i| matches!(&cpg.graph[i], CpgNode::Function {file, name, start_line, end_line, ..}
                if (file, name, *start_line, *end_line) == (&id.file, &id.name, id.start_line, id.end_line))).unwrap();
            assert!(matches!(&cpg.graph[idx], CpgNode::Function {start_byte, end_byte, ..} if (*start_byte, *end_byte) == span));
            assert_eq!(&source[span.0..span.1], &parsed.source[span.0..span.1]);
            let contained: Vec<_> = cpg.graph.edges(idx).filter(|e| *e.weight() == CpgEdge::Contains).collect();
            assert!(contained.iter().all(|e| matches!(&cpg.graph[e.target()], CpgNode::Variable {file, function, function_start_line, ..}
                if (file, function, *function_start_line) == (&id.file, &id.name, id.start_line))));
            if crate::anon_owner::is_anonymous_owner_name(&id.name) && !crate::anon_owner::is_module_owner_name(&id.name) { assert!(!contained.is_empty(), "{id:?}"); }
        }
    }
}
