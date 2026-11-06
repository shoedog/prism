//! Memory slice 1: assembly copies. The programs are parsed, never executed.
use super::*;
use crate::languages::Language;

const GO_CHAIN: &str = "package pkg\n\ntype R struct{ f int }\n\nfunc (r *R) get() *R { return r }\n\nfunc (r *R) Value() int { return r.f }\n\nfunc f(r *R, x int) int {\n\tr.f = x\n\treturn r.get().Value()\n}\n";
const TS_SRC: &str = "export function f(a: number) { const b = a; return b + a; }\n";

fn parse(path: &str, src: &str, lang: Language) -> BTreeMap<String, ParsedFile> {
    let parsed = ParsedFile::parse(path, src, lang).unwrap();
    BTreeMap::from([(path.to_string(), parsed)])
}

fn loc(file: &str) -> VarLocation {
    VarLocation {
        file: file.to_string(),
        function: "f".into(),
        function_start_line: 1,
        line: 1,
        path: AccessPath::simple("a"),
        start_byte: 3,
        end_byte: 4,
        kind: crate::data_flow::VarAccessKind::Use,
    }
}

fn materialize(file: &str, src: &str, lang: Language) -> (usize, usize) {
    let files = parse(file, src, lang);
    let mut graph = DiGraph::new();
    let (mut var, mut exact, mut location) = Default::default();
    CodePropertyGraph::materialize_var_location(
        &loc(file),
        VarAccess::Use,
        &files,
        &mut graph,
        &mut var,
        &mut exact,
        &mut location,
    );
    (var.len(), exact.len())
}

#[test]
fn go_contributes_no_exact_entries_and_ts_still_does() {
    assert_eq!(materialize("a.go", GO_CHAIN, Language::Go), (1, 0));
    assert_eq!(materialize("a.ts", TS_SRC, Language::TypeScript), (1, 1));
    assert_eq!(materialize("a.js", TS_SRC, Language::JavaScript), (1, 1));
}

/// The stored `var_index` equals the first-wins index of its own nodes (what a
/// cache load rebuilds), so the in-place detach left nothing out, including the
/// recall-only receiver Use the lookups hide from Step 5b/5c.
fn assert_var_index_complete(files: &BTreeMap<String, ParsedFile>) -> usize {
    let cpg = CodePropertyGraph::build(files);
    let mut reference: BTreeMap<LegacyVarKey, NodeIndex> = BTreeMap::new();
    for idx in cpg.graph.node_indices() {
        if let CpgNode::Variable {
            path,
            file,
            function,
            function_start_line,
            line,
            access,
            ..
        } = &cpg.graph[idx]
        {
            reference
                .entry((
                    file.clone(),
                    function.clone(),
                    *function_start_line,
                    *line,
                    path.clone(),
                    *access,
                ))
                .or_insert(idx);
        }
    }
    assert_eq!(cpg.var_index, reference);
    reference.len()
}

#[test]
fn assembled_var_index_is_complete_with_recall_hidden_uses() {
    let files = parse("pkg/r.go", GO_CHAIN, Language::Go);
    let dfg = DataFlowGraph::build(&files);
    assert!(
        !dfg.recall_only_use_spans().is_empty(),
        "fixture has a recall-only receiver Use"
    );
    assert!(assert_var_index_complete(&files) > 0);
    assert!(assert_var_index_complete(&parse("a.ts", TS_SRC, Language::TypeScript)) > 0);
}
