//! Stable coordinate contract of the cache, independent of graph numbering.
use super::*;
use crate::languages::Language;

#[test]
fn parameter_targets_resolve_coordinates_after_graph_relocation() {
    let files = BTreeMap::from([("coords.js".to_string(), ParsedFile::parse("coords.js",
        "function helper(x) { return x; }\nfunction caller(x) { let y = helper(x); return y; }\n", Language::JavaScript).unwrap())]);
    let cpg = CodePropertyGraph::build(&files);
    let callee = &cpg.call_graph.functions["helper"][0];
    let cache = ParamCache::new(&files);
    let cell = cache.get(callee, &files["coords.js"], &cpg.var_index, &cpg.graph);
    let target = cell.get().unwrap().as_ref().unwrap()[0].as_ref().unwrap();
    let original = target.resolve(&cpg.var_index, &cpg.graph).unwrap();
    let mut relocated = DiGraph::new();
    relocated.add_node(CpgNode::Statement {
        file: "padding".into(),
        line: 1,
        kind: StmtKind::Other,
        start_byte: 0,
        end_byte: 0,
    });
    for node in cpg.graph.node_weights() {
        relocated.add_node(node.clone());
    }
    let index = cpg
        .var_index
        .iter()
        .map(|(key, idx)| (key.clone(), NodeIndex::new(idx.index() + 1)))
        .collect();
    let expected = NodeIndex::new(original.index() + 1);
    assert_eq!(target.resolve(&index, &relocated), Some(expected));
    let CpgNode::Variable { end_byte, .. } = &mut relocated[expected] else {
        unreachable!()
    };
    *end_byte += 1;
    assert_eq!(
        target.resolve(&index, &relocated),
        None,
        "wrong Def bytes must refuse"
    );
    assert_eq!(
        target.resolve(&BTreeMap::new(), &relocated),
        None,
        "missing coordinate must refuse"
    );
}
