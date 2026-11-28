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
    let index = VarIndex::from_map(
        cpg.var_index
            .to_map(&cpg.graph)
            .into_iter()
            .map(|(key, idx)| (key, NodeIndex::new(idx.index() + 1)))
            .collect(),
    );
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
        target.resolve(&VarIndex::new(), &relocated),
        None,
        "missing coordinate must refuse"
    );
}

fn variable(owner: &str, access: VarAccess, bytes: (usize, usize)) -> CpgNode {
    CpgNode::Variable {
        file: "coords.js".into(),
        path: AccessPath::simple("local"),
        function: owner.into(),
        function_start_line: 1,
        line: 1,
        access,
        start_byte: bytes.0,
        end_byte: bytes.1,
    }
}

#[test]
fn pattern_parameter_cache_selects_second_same_name_def_after_reindex() {
    let first = variable("D", VarAccess::Def, (12, 17));
    let second = variable("D", VarAccess::Def, (22, 27));
    let target = PatternTarget::from_node(&second);
    let mut original = DiGraph::new();
    original.add_node(first.clone());
    let original_second = original.add_node(second.clone());
    let mut relocated = DiGraph::new();
    relocated.add_node(variable("wrong_owner", VarAccess::Def, (22, 27)));
    relocated.add_node(first);
    let expected = relocated.add_node(second);
    let relocated_index = CodePropertyGraph::exact_var_index_from_graph(&relocated);
    assert_eq!(
        relocated_index.values().map(BTreeMap::len).sum::<usize>(),
        3
    );
    assert_ne!(original_second, expected);
    assert_eq!(target.resolve(&relocated_index, &relocated), Some(expected));
    assert_eq!(
        target.resolve(
            &CodePropertyGraph::exact_var_index_from_graph(&original),
            &original
        ),
        Some(original_second),
    );
}

#[test]
fn pattern_parameter_cache_refuses_wrong_owner_access_bytes_and_duplicates() {
    let sought = variable("D", VarAccess::Def, (22, 27));
    let target = PatternTarget::from_node(&sought);
    for decoys in [
        vec![variable("other", VarAccess::Def, (22, 27))],
        vec![variable("D", VarAccess::Use, (22, 27))],
        vec![variable("D", VarAccess::Def, (22, 28))],
        vec![sought.clone(), sought.clone()],
    ] {
        let mut graph = DiGraph::new();
        for node in decoys {
            graph.add_node(node);
        }
        let index = CodePropertyGraph::exact_var_index_from_graph(&graph);
        assert_eq!(target.resolve(&index, &graph), None, "{index:?}");
    }
}

#[test]
fn pattern_parameter_cache_keeps_empty_and_alias_skipped_slots() {
    let path = "coords.js";
    let src = "function D(defaults, {}, {a} = defaults, tail) {\n  use(a, tail);\n}\n";
    let parsed = ParsedFile::parse(path, src, Language::JavaScript).unwrap();
    let callable = parsed.all_functions()[0];
    let syntax = parsed.js_ts_pattern_params(&callable);
    assert_eq!(
        syntax
            .iter()
            .map(|at| (at.position, at.alias_skipped, at.elements.len()))
            .collect::<Vec<_>>(),
        [(1, false, 0), (2, true, 0)]
    );
    let files = BTreeMap::from([(path.into(), parsed)]);
    let cpg = CodePropertyGraph::build(&files);
    let callee = &cpg.call_graph.functions["D"][0];
    let cache = ParamCache::new(&files);
    let exact = CodePropertyGraph::exact_var_index_from_graph(&cpg.graph);
    for _ in 0..2 {
        let got = cache.pattern_positions(callee, &files[path], &exact, &cpg.graph);
        assert_eq!(
            got.iter()
                .map(|(position, alias, elements)| (*position, *alias, elements.len()))
                .collect::<Vec<_>>(),
            [(1, false, 0), (2, true, 0)]
        );
    }
}

#[test]
fn anonymous_fixed_slots_open_only_for_argument_follow() {
    use crate::resolution::ResolutionKind::{ArgumentFollow, FreeSingle, ParameterCallback};
    let anonymous = FunctionId {
        file: "m.js".into(),
        name: crate::anon_owner::anonymous_owner_name(5, 7),
        start_line: 5,
        end_line: 5,
    };
    let named = FunctionId {
        name: "handler".into(),
        ..anonymous.clone()
    };
    let slots = [Some(1u8), Some(2)];
    let bare = |slot: &u8| *slot == 9;
    // A route other than argument_follow keeps the earlier reach: nothing for a fixed slot.
    for kind in [FreeSingle, ParameterCallback] {
        assert!(admit_anonymous_slots(&anonymous, kind, &slots, bare).is_empty());
    }
    assert_eq!(
        admit_anonymous_slots(&anonymous, ArgumentFollow, &slots, bare),
        slots
    );
    // A bare first slot (slice C) and every named callee keep their slots on any route.
    let bare_slots = [Some(9u8)];
    assert_eq!(
        admit_anonymous_slots(&anonymous, FreeSingle, &bare_slots, bare),
        bare_slots
    );
    assert_eq!(
        admit_anonymous_slots(&named, FreeSingle, &slots, bare),
        slots
    );
}
