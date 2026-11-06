//! `VarIndex` against the old `BTreeMap` it replaced (#437 memory slice 3).
//! The programs are parsed, never executed.
use super::recall_hidden::HiddenUses;
use super::*;
use crate::languages::Language;

const GO: &str = "package pkg\n\ntype R struct{ f int }\n\nfunc (r *R) get() *R { return r }\n\nfunc (r *R) Value() int { return r.f }\n\nfunc f(r *R, x int) int {\n\tr.f = x\n\treturn r.get().Value()\n}\n";
// Same path twice on one line: several occurrences share one legacy key.
const TS: &str = "export function f(a: number) { const b = a + a; return b + a + a; }\nexport function g(a: number) { return f(a) + f(a); }\n";
const JS: &str =
    "function f(o) { o.x = o.x + o.x; return o.x; }\nfunction g(a) { return f({ x: a, y: a }); }\n";
const PY: &str = "def f(a, b):\n    c = a + b\n    return c + a\n\ndef g(x):\n    return f(x, x)\n";
const C: &str = "int f(int a, int b) {\n    int c = a + b;\n    return c + a;\n}\n\nint g(int x) { return f(x, x); }\n";

fn fixtures() -> Vec<(&'static str, BTreeMap<String, ParsedFile>)> {
    let one = |path: &str, src: &str, lang| {
        BTreeMap::from([(
            path.to_string(),
            ParsedFile::parse(path, src, lang).unwrap(),
        )])
    };
    let mut all = one("a.go", GO, Language::Go);
    all.extend(one("b.ts", TS, Language::TypeScript));
    all.extend(one("c.js", JS, Language::JavaScript));
    all.extend(one("d.py", PY, Language::Python));
    all.extend(one("e.c", C, Language::C));
    vec![
        ("go", one("a.go", GO, Language::Go)),
        ("ts", one("b.ts", TS, Language::TypeScript)),
        ("js", one("c.js", JS, Language::JavaScript)),
        ("py", one("d.py", PY, Language::Python)),
        ("c", one("e.c", C, Language::C)),
        ("all", all),
    ]
}

/// What a cache load used to build: every Variable node's key, first wins.
fn reference_map(graph: &DiGraph<CpgNode, CpgEdge>) -> BTreeMap<LegacyVarKey, NodeIndex> {
    let mut reference = BTreeMap::new();
    for idx in graph.node_indices() {
        if let CpgNode::Variable {
            path,
            file,
            function,
            function_start_line,
            line,
            access,
            ..
        } = &graph[idx]
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
    reference
}

#[test]
fn every_key_resolves_to_the_same_node_as_the_old_map() {
    for (name, files) in fixtures() {
        let cpg = CodePropertyGraph::build(&files);
        let reference = reference_map(&cpg.graph);
        assert!(!reference.is_empty(), "{name}: fixture has variables");
        assert_eq!(cpg.var_index.len(), reference.len(), "{name}");
        assert_eq!(cpg.var_index.to_map(&cpg.graph), reference, "{name}");
        let rebuilt = VarIndex::from_graph(&cpg.graph);
        for (key, &idx) in &reference {
            assert_eq!(
                cpg.var_index.get(&cpg.graph, key),
                Some(idx),
                "{name} {key:?}"
            );
            assert_eq!(rebuilt.get(&cpg.graph, key), Some(idx), "{name} rebuilt");
            assert_eq!(
                cpg.var_index
                    .get_ref(&cpg.graph, VarKeyRef::from_legacy(key)),
                Some(idx),
                "{name} borrowed lookup"
            );
            // The lookup key is exactly the node's own fields.
            assert!(
                matches!(&cpg.graph[idx], CpgNode::Variable { file, line, .. }
                if *file == key.0 && *line == key.3)
            );
        }
        // Misses stay misses.
        let mut miss = reference.keys().next().unwrap().clone();
        miss.3 += 10_000;
        assert_eq!(cpg.var_index.get(&cpg.graph, &miss), None, "{name}");
        // Key order, as the old iteration produced it.
        let sorted: Vec<_> = cpg.var_index.sorted_nodes(&cpg.graph);
        assert_eq!(
            sorted,
            reference.values().copied().collect::<Vec<_>>(),
            "{name}"
        );
    }
}

#[test]
fn duplicate_keys_resolve_to_the_first_node() {
    // JS and TS give several same-key occurrences; assert the fixtures really
    // contain duplicates so first-wins is exercised.
    let mut saw_duplicate = false;
    for (name, files) in fixtures() {
        let cpg = CodePropertyGraph::build(&files);
        let total = cpg
            .graph
            .node_weights()
            .filter(|n| matches!(n, CpgNode::Variable { .. }))
            .count();
        saw_duplicate |= total > cpg.var_index.len();
        for (key, &first) in &reference_map(&cpg.graph) {
            let holder = cpg.var_index.get(&cpg.graph, key).unwrap();
            assert_eq!(holder, first, "{name}: not first-wins for {key:?}");
        }
    }
    assert!(saw_duplicate, "no fixture produced a duplicate key");
}

fn variable(file: &str, line: usize, start: usize) -> CpgNode {
    CpgNode::Variable {
        path: AccessPath::simple("v"),
        file: file.into(),
        function: "f".into(),
        function_start_line: 1,
        line,
        access: VarAccess::Use,
        start_byte: start,
        end_byte: start + 1,
    }
}

#[test]
fn insert_is_first_wins_in_insertion_order() {
    let mut graph = DiGraph::new();
    let a = graph.add_node(variable("x.rs", 2, 10));
    let b = graph.add_node(variable("x.rs", 2, 20)); // same key as `a`
    let c = graph.add_node(variable("x.rs", 3, 30));
    let mut index = VarIndex::new();
    assert_eq!(index.insert_first_wins(&graph, b), b);
    assert_eq!(
        index.insert_first_wins(&graph, a),
        b,
        "later duplicate loses"
    );
    assert_eq!(index.insert_first_wins(&graph, c), c);
    assert_eq!(index.len(), 2);
    let key = reference_map(&graph).keys().next().unwrap().clone();
    assert_eq!(index.get(&graph, &key), Some(b));
}

#[test]
fn growth_rehashes_from_the_graph() {
    let mut graph = DiGraph::new();
    let nodes: Vec<_> = (0..5_000)
        .map(|i| graph.add_node(variable("big.rs", i % 977, i)))
        .collect();
    let mut index = VarIndex::new();
    for &n in &nodes {
        index.insert_first_wins(&graph, n);
    }
    let reference = reference_map(&graph);
    assert_eq!(index.to_map(&graph), reference);
    assert_eq!(index.len(), reference.len());
}

#[test]
fn hidden_uses_detach_and_restore_round_trip() {
    for (name, files) in fixtures() {
        let mut cpg = CodePropertyGraph::build(&files);
        let before = cpg.var_index.to_map(&cpg.graph);
        let victims: Vec<NodeIndex> = before.values().copied().step_by(3).collect();
        let hidden = HiddenUses::for_test(
            victims
                .iter()
                .map(|&n| {
                    let CpgNode::Variable {
                        path,
                        file,
                        function,
                        function_start_line,
                        line,
                        access,
                        ..
                    } = &cpg.graph[n]
                    else {
                        unreachable!()
                    };
                    (
                        (
                            file.clone(),
                            function.clone(),
                            *function_start_line,
                            *line,
                            path.clone(),
                            *access,
                        ),
                        n,
                    )
                })
                .collect(),
        );
        let mut exact = CodePropertyGraph::exact_var_index_from_graph(&cpg.graph);
        {
            let view = hidden.detach(&mut cpg.var_index, &mut exact);
            for (key, idx) in &before {
                let seen = view.get(&cpg.graph, key);
                if victims.contains(idx) {
                    assert_eq!(seen, None, "{name}: hidden entry still visible");
                } else {
                    assert_eq!(seen, Some(*idx), "{name}: visible entry lost");
                }
            }
        }
        assert_eq!(cpg.var_index.to_map(&cpg.graph), before, "{name}: restored");
    }
}

#[test]
fn a_mutated_or_replaced_node_misses_without_panicking() {
    // Contract (#689): indexed nodes are not mutated after indexing. A caller
    // that does so through the public graph gets a miss, never a panic.
    let mut graph = DiGraph::new();
    let a = graph.add_node(variable("x.rs", 2, 10));
    let b = graph.add_node(variable("x.rs", 3, 20));
    let mut index = VarIndex::new();
    index.insert_first_wins(&graph, a);
    index.insert_first_wins(&graph, b);
    let keys: Vec<LegacyVarKey> = reference_map(&graph).into_keys().collect();
    if let CpgNode::Variable { line, .. } = &mut graph[a] {
        *line = 99;
    }
    graph[b] = CpgNode::Statement {
        file: "x.rs".into(),
        line: 3,
        kind: StmtKind::Other,
        start_byte: 20,
        end_byte: 21,
    };
    assert_eq!(index.get(&graph, &keys[0]), None, "mutated key misses");
    assert_eq!(index.get(&graph, &keys[1]), None, "replaced node misses");
}

#[test]
fn all_defs_of_returns_several_defs_in_the_old_key_order() {
    // Several defs of one name across functions and lines: the order must be
    // the old map's key order whatever the table's (seeded) iteration order.
    let src =
        "def f(a):\n    x = a\n    x = x + 1\n    return x\n\ndef e(b):\n    x = b\n    return x\n";
    let files = BTreeMap::from([(
        "m.py".to_string(),
        ParsedFile::parse("m.py", src, Language::Python).unwrap(),
    )]);
    let cpg = CodePropertyGraph::build(&files);
    let expected: Vec<(String, usize)> = reference_map(&cpg.graph)
        .into_keys()
        .filter(|k| k.0 == "m.py" && k.4.base == "x" && k.5 == VarAccess::Def)
        .map(|k| (k.1, k.3))
        .collect();
    assert!(
        expected.len() >= 3,
        "fixture has several defs: {expected:?}"
    );
    let got: Vec<(String, usize)> = cpg
        .all_defs_of("m.py", "x")
        .into_iter()
        .map(|loc| (loc.function, loc.line))
        .collect();
    assert_eq!(got, expected);
}
