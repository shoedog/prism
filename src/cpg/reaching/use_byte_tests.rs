//! Synthetic-only equivalence and operation assertions; no corpus or clocks.
use super::*;
use crate::access_path::AccessPath;
use crate::cpg::reaching::{DefId, RdOutcome};
use crate::data_flow::{DataFlowGraph, VarAccessKind, VarLocation};
use std::cell::{Cell, RefCell};

#[derive(Default, Debug, PartialEq, Eq)]
pub(crate) struct Counts {
    pub builds: usize,
    pub rvalue_batches: usize,
    pub identifier_batches: usize,
}

thread_local! {
    static LEGACY: Cell<bool> = const { Cell::new(false) };
    static OUTCOMES: RefCell<Option<Vec<RdOutcome>>> = const { RefCell::new(None) };
    static COUNTS: RefCell<Option<Counts>> = const { RefCell::new(None) };
}

pub(crate) fn legacy_enabled() -> bool {
    LEGACY.with(Cell::get)
}

pub(crate) fn bump(f: impl FnOnce(&mut Counts)) {
    COUNTS.with(|counts| {
        if let Some(counts) = counts.borrow_mut().as_mut() {
            f(counts);
        }
    });
}

pub(crate) fn record_outcome(outcome: &RdOutcome) {
    OUTCOMES.with(|outcomes| {
        if let Some(outcomes) = outcomes.borrow_mut().as_mut() {
            outcomes.push(outcome.clone());
        }
    });
}

struct Mode;
impl Mode {
    fn begin(legacy: bool) -> Self {
        LEGACY.with(|slot| assert!(!slot.replace(legacy)));
        OUTCOMES.with(|slot| assert!(slot.replace(Some(Vec::new())).is_none()));
        Self
    }
    fn take() -> Vec<RdOutcome> {
        OUTCOMES.with(|slot| slot.take().unwrap())
    }
}
impl Drop for Mode {
    fn drop(&mut self) {
        LEGACY.with(|slot| slot.set(false));
        OUTCOMES.with(|slot| slot.take());
        COUNTS.with(|slot| slot.take());
    }
}

fn parse(language: Language, source: &str) -> ParsedFile {
    ParsedFile::parse("synthetic", source, language).unwrap()
}

fn query(owner: Line, line: Line, path: &str) -> FlowEdge {
    let to = VarLocation {
        file: "synthetic".to_owned(),
        function: "synthetic".to_owned(),
        function_start_line: owner,
        line,
        path: AccessPath::from_expr(path),
        start_byte: 0,
        end_byte: 0,
        kind: VarAccessKind::Use,
    };
    FlowEdge {
        from: VarLocation {
            kind: VarAccessKind::Def,
            ..to.clone()
        },
        to,
    }
}

fn compare_queries(parsed: &ParsedFile, queries: &[FlowEdge]) -> Vec<Option<usize>> {
    let lookup = crate::cpg::reaching::use_bytes::UseBytes::new(queries.iter());
    queries
        .iter()
        .map(|edge| {
            let old = use_byte(parsed, edge);
            let new = lookup.get(parsed, edge);
            assert_eq!(old, new, "{edge:?}\n{}", parsed.source);
            new
        })
        .collect()
}

#[test]
fn uniqueness_paths_fallback_and_shortcut() {
    let parsed = parse(Language::TypeScript,
        "function f(x, obj) {\nconsume(x, x);\nconsume(obj.a, obj.b);\nlet fallback = 1;\nambiguous = ambiguous;\nreturn x;\n}\n");
    let mut queries = vec![
        query(1, 2, "x"),
        query(1, 3, "obj.a"),
        query(1, 3, "obj.b"),
        query(1, 3, "obj"),
        query(1, 4, "fallback"),
        query(1, 5, "ambiguous"),
        query(1, 4, "absent"),
        query(1, 6, "x"),
        query(999, 4, "fallback"),
    ];
    // The shortcut bypasses even a nonexistent owner and line.
    let mut real = query(999, 999, "absent");
    real.to.start_byte = 7;
    real.to.end_byte = 8;
    queries.push(real);
    // A reversed interval follows the zero-width branch too.
    let mut reversed = query(1, 4, "fallback");
    reversed.to.start_byte = 9;
    queries.push(reversed);
    let actual = compare_queries(&parsed, &queries);
    assert_eq!(actual[0], None, "multiple rvalue spans stay ambiguous");
    assert_eq!(actual[1], Some(parsed.source.find("obj.a").unwrap()));
    assert_eq!(actual[2], Some(parsed.source.find("obj.b").unwrap()));
    assert_eq!(actual[3], None, "base has two occurrences");
    assert_eq!(actual[4], Some(parsed.source.find("fallback").unwrap()));
    assert_eq!(
        actual[5],
        Some(parsed.source.rfind("ambiguous").unwrap()),
        "one RHS span wins even though fallback identifiers are ambiguous"
    );
    assert_eq!(actual[6], None);
    assert_eq!(actual[7], Some(parsed.source.rfind('x').unwrap()));
    assert_eq!(actual[8], None, "no fallback without inventoried owner");
    assert_eq!(actual[9], Some(7));
    assert_eq!(actual[10], actual[4]);

    let parsed = parse(Language::TypeScript, "function f() {\nlet a=1, a=2;\n}\n");
    assert!(parsed
        .rvalue_identifier_spans_on_lines(&parsed.all_functions()[0], &BTreeSet::from([2]))
        .iter()
        .all(|span| span.path.base != "a"));
    assert_eq!(
        compare_queries(&parsed, &[query(1, 2, "a")]),
        vec![None],
        "ambiguous declaration-only fallback must not choose first"
    );
}

#[test]
fn singleton_origin_and_order() {
    for (language, source) in fixtures() {
        let parsed = parse(language, source);
        let all_lines: BTreeSet<_> = (0..=source.lines().count() + 2).collect();
        for owner in parsed
            .all_functions()
            .into_iter()
            .chain([parsed.tree.root_node()])
        {
            let actual = parsed.singleton_rvalue_spans_on_lines(&owner, &all_lines);
            let mut expected: Vec<_> = all_lines
                .iter()
                .flat_map(|line| {
                    parsed
                        .rvalue_identifier_spans_on_lines(&owner, &BTreeSet::from([*line]))
                        .into_iter()
                        .filter(move |span| span.line == *line)
                })
                .collect();
            expected.sort_by_key(|span| (span.line, span.start_byte, span.end_byte));
            assert_eq!(
                actual, expected,
                "singleton multiset and order: {language:?}, {owner:?}"
            );
        }
        let batched = parsed.identifier_bytes_on_lines(&all_lines);
        for line in all_lines {
            let mut expected = BTreeMap::new();
            for node in parsed.identifiers_on_line(line) {
                expected
                    .entry((line.saturating_sub(1), parsed.node_text(&node).to_owned()))
                    .and_modify(|value| *value = None)
                    .or_insert(Some(node.start_byte()));
            }
            let actual: BTreeMap<_, _> = batched
                .iter()
                .filter(|((row, _), _)| *row == line.saturating_sub(1))
                .map(|(key, value)| (key.clone(), *value))
                .collect();
            assert_eq!(
                actual, expected,
                "identifier singleton: {language:?}, line {line}"
            );
        }
    }
    // The outer call starts on line 2; its line-3 emissions must not contaminate
    // the unique inner call selected by a singleton query on line 3.
    let parsed = parse(
        Language::TypeScript,
        "function f(x) {\nconsume(\nconsume(x));\n}\n",
    );
    let queries = [query(1, 2, "x"), query(1, 3, "x")];
    let actual = compare_queries(&parsed, &queries);
    assert_eq!(actual, vec![None, Some(parsed.source.rfind('x').unwrap())]);
    let union = parsed
        .rvalue_identifier_spans_on_lines(&parsed.all_functions()[0], &BTreeSet::from([2, 3]));
    assert_eq!(
        union
            .iter()
            .filter(|span| span.line == 3 && span.path.base == "x")
            .count(),
        2,
        "witness must distinguish naive union batching"
    );
}

#[test]
fn same_line_owners_and_module_absence() {
    let parsed = parse(Language::TypeScript,
        "function outer(x, obj) { const inner = () => {\nconsume(obj.a, obj.b);\n};\nreturn x;\n}\nlet moduleOnly=1;\nconsume(moduleOnly);\n");
    let owners = parsed.all_functions();
    assert_eq!(owners.len(), 2);
    assert_eq!(
        owners[0].start_position().row,
        owners[1].start_position().row
    );
    let first = owners[0];
    let other = owners[1];
    assert_ne!(first.id(), other.id());
    let mut queries = vec![
        query(1, 2, "obj.a"),
        query(1, 4, "x"),
        query(6, 7, "moduleOnly"),
    ];
    let actual = compare_queries(&parsed, &queries);
    assert_eq!(
        actual[0], None,
        "outer excludes nested body; base fallback is ambiguous"
    );
    assert_eq!(
        parsed
            .rvalue_identifier_spans_on_lines(&other, &BTreeSet::from([2]))
            .iter()
            .filter(|span| span.path == AccessPath::from_expr("obj.a"))
            .count(),
        1,
        "choosing the other same-line owner would return a byte"
    );
    assert_eq!(actual[2], None, "module owner is not in all_functions");
    // Key owner start as well as path/line: same source occurrence with a
    // nonexistent owner must still return None.
    queries.push(query(999, 4, "x"));
    assert_eq!(compare_queries(&parsed, &queries)[3], None);
    let empty = parse(
        Language::TypeScript,
        "let moduleOnly=1;\nconsume(moduleOnly);\n",
    );
    assert_eq!(
        compare_queries(&empty, &[query(1, 2, "moduleOnly")]),
        vec![None]
    );
}

#[test]
fn batches_are_per_invocation_not_per_line_or_path() {
    let _mode = Mode::begin(false);
    let parsed = parse(
        Language::TypeScript,
        "function f(x) {\nconsume(x);\nconsume(x);\nlet a=1;\nlet b=2;\nlet a=3, a=4;\n}\n",
    );
    let queries = [
        query(1, 2, "x"),
        query(1, 3, "x"),
        query(1, 4, "a"),
        query(1, 5, "b"),
        query(1, 6, "a"),
        query(999, 2, "x"),
    ];
    COUNTS.with(|slot| *slot.borrow_mut() = Some(Counts::default()));
    let lookup = crate::cpg::reaching::use_bytes::UseBytes::new(queries.iter());
    let before = crate::ast::call_membership_tests::inventory_count();
    for _ in 0..3 {
        for edge in &queries {
            let _ = lookup.get(&parsed, edge);
        }
    }
    assert_eq!(
        crate::ast::call_membership_tests::inventory_count() - before,
        2 // One inventory plus the unchanged JS/TS rvalue-scope validation.
    );
    COUNTS.with(|slot| {
        assert_eq!(
            slot.borrow().as_ref().unwrap(),
            &Counts {
                builds: 1,
                rvalue_batches: 1,
                identifier_batches: 1,
            }
        )
    });
    let real = FlowEdge {
        to: VarLocation {
            start_byte: 1,
            end_byte: 2,
            ..queries[0].to.clone()
        },
        ..queries[0].clone()
    };
    let lookup = crate::cpg::reaching::use_bytes::UseBytes::new(std::iter::once(&real));
    assert_eq!(lookup.get(&parsed, &real), Some(1));
    COUNTS.with(|slot| {
        assert_eq!(
            slot.borrow().as_ref().unwrap().builds,
            1,
            "shortcut does no batch work"
        )
    });
}

fn fixtures() -> Vec<(Language, &'static str)> {
    vec![
        (Language::Go, "package p\nfunc f(x int, obj T) int {\ny := x\nconsume(x, x)\nconsume(obj.a, obj.b)\nconsume(\nconsume(x))\nx += x\nfallback := 1\nreturn y\n}\n"),
        (Language::Python, "moduleOnly = 1\nconsume(moduleOnly)\ndef f(x, obj):\n    y = x\n    consume(x, x)\n    consume(obj.a, obj.b)\n    consume(\n        consume(x))\n    x += x\n    fallback = 1\n    return y\n"),
        (Language::JavaScript, "let moduleOnly=1;\nconsume(moduleOnly);\nfunction f(x, obj) { const inner = () => {\nconsume(x);\n};\nlet y=x;\nconsume(x,x);\nconsume(obj.a,obj.b);\nconsume(\nconsume(x));\nlet fallback=1;\nreturn y;\n}\n"),
        (Language::TypeScript, "let moduleOnly=1;\nconsume(moduleOnly);\nfunction f(x: number, obj: T) { const inner = () => {\nconsume(x);\n};\nlet y=x;\nconsume(x,x);\nconsume(obj.a,obj.b);\nconsume(\nconsume(x));\nx += x;\nlet fallback=1;\nreturn y;\n}\n"),
        (Language::Rust, "fn f(x: i32, obj: T) -> i32 {\nlet y=x;\nconsume(x,x);\nconsume(obj.a,obj.b);\nconsume(\nconsume(x));\nlet fallback=1;\nreturn y;\n}\n"),
        // Exercise the language with no Returns query too.
        (Language::Bash, "f() {\nx=$y\necho $x $x\n}\n"),
    ]
}

#[test]
fn every_edge_and_final_labels_match_reference() {
    rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .stack_size(64 * 1024 * 1024)
        .build()
        .unwrap()
        .install(|| {
            for (language, source) in fixtures() {
                let parsed = parse(language, source);
                let files = BTreeMap::from([(parsed.path.clone(), parsed)]);
                let mode = Mode::begin(true);
                let old = DataFlowGraph::build_on_current_pool(&files);
                let old_outcomes = Mode::take();
                drop(mode);
                let mode = Mode::begin(false);
                let new = DataFlowGraph::build_on_current_pool(&files);
                let new_outcomes = Mode::take();
                drop(mode);
                assert!(!new_outcomes.is_empty(), "{language:?} must exercise RD");
                assert_eq!(
                    new_outcomes, old_outcomes,
                    "all RD labels/doubts: {language:?}"
                );
                assert_eq!(
                    crate::cpg::dfg_observation::dfg_storage(&old),
                    crate::cpg::dfg_observation::dfg_storage(&new),
                    "full DFG: {language:?}"
                );
                let parsed = files.values().next().unwrap();
                let mut edges: Vec<_> = old.edges().iter().map(|edge| edge.to_owned()).collect();
                assert!(!edges.is_empty(), "{language:?} must produce edges");
                // Test every producer edge in both its real-span form and forced
                // legacy anchor form, including edges normally taking the shortcut.
                let anchors: Vec<_> = edges
                    .iter()
                    .cloned()
                    .map(|mut edge| {
                        edge.to.end_byte = edge.to.start_byte;
                        edge
                    })
                    .collect();
                edges.extend(anchors);
                compare_queries(parsed, &edges);
                for owner in parsed.all_functions() {
                    let start = parsed.node_line_range(&owner).0;
                    let defs: Vec<_> = parsed
                        .assignment_lvalue_spans_on_lines(
                            &owner,
                            &(1..=source.lines().count()).collect(),
                        )
                        .into_iter()
                        .enumerate()
                        .map(|(i, span)| DefSite {
                            id: DefId(i as u32),
                            path: span.path,
                            line: span.line,
                            start_byte: span.start_byte,
                            alias_derived: false,
                        })
                        .collect();
                    let owned: Vec<_> = edges
                        .iter()
                        .filter(|edge| edge.to.function_start_line == start)
                        .cloned()
                        .collect();
                    if defs.is_empty() || owned.is_empty() {
                        continue;
                    }
                    let cfg_edges = crate::cfg::build_cfg_edges_with_arms(parsed);
                    let cfg = crate::cpg::reaching::FileCfgEdgeIndex::new(&cfg_edges);
                    let mode = Mode::begin(true);
                    let old = crate::cpg::reaching::reaching_definitions_with_exact(
                        parsed, &owner, &defs, &owned, &owned, &cfg,
                    );
                    drop(mode);
                    let new = crate::cpg::reaching::reaching_definitions_with_exact(
                        parsed, &owner, &defs, &owned, &owned, &cfg,
                    );
                    assert_eq!(new, old, "zero-width RD labels/doubts: {language:?}");
                }
            }
        });
}
