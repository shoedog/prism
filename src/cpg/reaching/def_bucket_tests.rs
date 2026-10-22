//! Definition-bucket cost and identity witnesses.
use super::*;

#[test]
fn distinct_paths_do_no_cross_path_kill_work() {
    for n in [8, 32, 128] {
        let source = format!(
            "function f() {{\n{}return 0;\n}}",
            (0..n)
                .map(|i| format!("let x{i}={i};\nconsume(x{i});\n"))
                .collect::<String>()
        );
        let parsed = ParsedFile::parse("cost.ts", &source, Language::TypeScript).unwrap();
        let (_, records) = work_tests::build(parsed);
        assert_eq!(records.len(), 1);
        let c = &records[0];
        assert!(c.definition_pair_comparisons > 0);
        // Independent of the projection counter: only each definition itself
        // shares its path, even though GEN then excludes it from the kill set.
        assert_eq!(c.definition_pair_comparisons, n, "{c:?}");
        assert_eq!(
            c.definition_pair_comparisons, c.projected_definition_pair_comparisons,
            "cross-path work: {c:?}"
        );
    }
}

#[test]
fn matching_examines_only_bucket_members() {
    for n in [8, 32, 128] {
        let source = format!(
            "function f() {{\n{}return 0;\n}}",
            (0..n)
                .map(|i| format!("let x{i}={i};\nconsume(x{i});\n"))
                .collect::<String>()
        );
        let parsed = ParsedFile::parse("cost.ts", &source, Language::TypeScript).unwrap();
        let (_, records) = work_tests::build(parsed);
        let c = &records[0];
        assert!(c.matching_defs_calls > 0);
        assert_eq!(
            c.matching_defs_calls, n,
            "one shared lookup per edge: {c:?}"
        );
        assert_eq!(c.matching_defs_items, n, "one member per bucket: {c:?}");
        assert_eq!(
            c.matching_defs_items, c.projected_matching_defs_items,
            "full scans: {c:?}"
        );
    }
}

use std::cell::{Cell, RefCell};
use std::path::Path;

#[derive(Default, Debug, PartialEq, Eq)]
struct Observation {
    outcomes: Vec<RdOutcome>,
    sets: Vec<(Vec<BitSet>, Vec<BitSet>)>,
    kills: Vec<(Vec<BitSet>, Vec<BitSet>)>,
}
thread_local! {
    static LEGACY: Cell<bool> = const { Cell::new(false) };
    static OBSERVED: RefCell<Option<Observation>> = const { RefCell::new(None) };
}
pub(super) fn legacy_enabled() -> bool {
    LEGACY.with(Cell::get)
}
pub(super) fn record_outcome(outcome: &RdOutcome) {
    OBSERVED.with(|slot| {
        if let Some(o) = slot.borrow_mut().as_mut() {
            o.outcomes.push(outcome.clone());
        }
    });
}
pub(super) fn record_sets(
    bound: &[BitSet],
    flat: &[BitSet],
    kill: &[BitSet],
    flat_kill: &[BitSet],
) {
    OBSERVED.with(|slot| {
        if let Some(o) = slot.borrow_mut().as_mut() {
            o.sets.push((bound.to_vec(), flat.to_vec()));
            o.kills.push((kill.to_vec(), flat_kill.to_vec()));
        }
    });
}
struct Mode;
impl Mode {
    fn begin(legacy: bool) -> Self {
        LEGACY.with(|m| assert!(!m.replace(legacy)));
        OBSERVED.with(|s| assert!(s.replace(Some(Observation::default())).is_none()));
        Self
    }
    fn take() -> Observation {
        OBSERVED.with(|s| s.take().unwrap())
    }
}
impl Drop for Mode {
    fn drop(&mut self) {
        LEGACY.with(|m| m.set(false));
        OBSERVED.with(|s| s.take());
    }
}

fn compare(path: &str, source: &str) -> (usize, BTreeSet<FlowDoubt>) {
    // The observer must own its worker: a shared Rayon worker can run unrelated
    // builds while a parallel consumer yields, polluting thread-local records.
    rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .stack_size(256 * 1024 * 1024)
        .build()
        .unwrap()
        .install(|| {
            let parsed =
                ParsedFile::parse(path, source, Language::from_path(path).unwrap()).unwrap();
            let files = BTreeMap::from([(path.to_owned(), parsed)]);
            let mode = Mode::begin(true);
            let old_dfg = crate::data_flow::DataFlowGraph::build(&files);
            let old_observation = Mode::take();
            drop(mode);
            let mode = Mode::begin(false);
            let new_dfg = crate::data_flow::DataFlowGraph::build(&files);
            let new_observation = Mode::take();
            drop(mode);
            assert_eq!(
                old_observation, new_observation,
                "outcomes, both kill sets and in-sets: {path}"
            );
            assert_eq!(
                crate::cpg::dfg_observation::dfg_storage(&old_dfg),
                crate::cpg::dfg_observation::dfg_storage(&new_dfg),
                "producer DFG serialization: {path}"
            );
            // The public call-graph builder uses the same default scope inputs as
            // CPG::build. Assemble both producer results after disabling the recorder.
            let cg = crate::call_graph::CallGraph::build(&files);
            let old =
                crate::cpg::CodePropertyGraph::assemble_graph(cg.clone(), old_dfg, &files, None);
            let new = crate::cpg::CodePropertyGraph::assemble_graph(cg, new_dfg, &files, None);
            assert_eq!(
                crate::cpg::dfg_observation::dfg_storage(&old.dfg),
                crate::cpg::dfg_observation::dfg_storage(&new.dfg),
                "full DFG serialization: {path}"
            );
            assert_eq!(
                crate::cpg::dfg_observation::fieldwise_payload(&old),
                crate::cpg::dfg_observation::fieldwise_payload(&new),
                "CPG labels and all doubt reasons: {path}"
            );
            let doubts = new
                .graph
                .edge_weights()
                .filter_map(|e| match e {
                    crate::cpg::CpgEdge::DataFlow(FlowConfidence::NameOnly(doubt)) => Some(*doubt),
                    _ => None,
                })
                .collect();
            (new_observation.outcomes.len(), doubts)
        })
}

#[test]
fn differential_all_fixtures() {
    let mut files = Vec::new();
    for root in ["tests/fixtures", "eval/fixtures"] {
        work_tests::fixture_files(
            &Path::new(env!("CARGO_MANIFEST_DIR")).join(root),
            &mut files,
        );
    }
    files.sort();
    assert!(files.len() >= 100);
    let mut owners = 0;
    let mut doubts = BTreeSet::new();
    for file in &files {
        let (n, d) = compare(
            file.to_str().unwrap(),
            &std::fs::read_to_string(file).unwrap(),
        );
        owners += n;
        doubts.extend(d);
    }
    assert!(owners > 100);
    for doubt in [
        FlowDoubt::SameLine,
        FlowDoubt::CfgIncomplete,
        FlowDoubt::AliasUnstable,
        FlowDoubt::CallNameOnly,
    ] {
        assert!(
            doubts.contains(&doubt),
            "missing coverage: {doubt:?}; got {doubts:?}"
        );
    }
    assert!(doubts.iter().any(|d| matches!(d, FlowDoubt::Killed { .. })));
    eprintln!(
        "definition differential: {} files, {owners} RD invocations; doubts {doubts:?}",
        files.len()
    );
}

#[test]
#[ignore = "explicit external definition-bucket differential; PRISM_STATEMENT_FIXTURES"]
fn differential_external_fixtures() {
    let root = std::env::var("PRISM_STATEMENT_FIXTURES").unwrap();
    let mut directories: Vec<_> = std::fs::read_dir(root)
        .unwrap()
        .map(Result::unwrap)
        .collect();
    directories.sort_by_key(|d| d.path());
    let mut owners = 0;
    for directory in directories {
        let files: Vec<_> = std::fs::read_dir(directory.path())
            .unwrap()
            .map(|e| e.unwrap().path())
            .collect();
        assert_eq!(files.len(), 1);
        owners += compare(
            files[0].to_str().unwrap(),
            &std::fs::read_to_string(&files[0]).unwrap(),
        )
        .0;
    }
    assert!(owners > 1000);
    eprintln!("external definition differential: {owners} RD invocations per mode");
}

fn site(id: u32, path: AccessPath, line: usize, start_byte: usize, alias_derived: bool) -> DefSite {
    DefSite {
        id: DefId(id),
        path,
        line,
        start_byte,
        alias_derived,
    }
}
fn query(path: AccessPath, line: usize, start_byte: usize) -> FlowEdge {
    let from = VarLocation {
        file: "abstract.ts".into(),
        function: "f".into(),
        function_start_line: 1,
        line,
        path,
        start_byte,
        end_byte: start_byte + 1,
        kind: crate::data_flow::VarAccessKind::Def,
    };
    FlowEdge {
        to: VarLocation {
            line: line + 1,
            kind: crate::data_flow::VarAccessKind::Use,
            ..from.clone()
        },
        from,
    }
}

#[test]
fn exact_fallback_and_index_order() {
    let field = AccessPath::with_fields("a", vec!["b".into()]);
    let base = AccessPath::simple("a");
    let dotted_base = AccessPath::simple("a.b");
    let defs = vec![
        site(0, field.clone(), 3, 30, false),
        site(1, base.clone(), 3, 30, false),
        site(2, field.clone(), 3, 31, true),
        site(3, field.clone(), 3, 30, true),
        site(4, field.clone(), 4, 30, false),
        site(5, dotted_base.clone(), 3, 30, false),
    ];
    let buckets = def_buckets::DefinitionBuckets::new(&defs);
    for (path, line, byte, expected) in [
        (field.clone(), 3, 30, vec![0, 3]), // Multiple exact matches stay ascending, including aliases.
        (field.clone(), 3, 31, vec![2]),    // Exact must exclude fallback-only members.
        (field.clone(), 3, 999, vec![0, 2, 3]), // Exact miss uses the whole line bucket.
        (field.clone(), 4, 30, vec![4]),
        (field.clone(), 99, 30, vec![]),
        (base, 3, 30, vec![1]),
        (dotted_base, 3, 30, vec![5]), // Identical Display spelling is unequal AccessPath.
        (AccessPath::simple("absent"), 3, 30, vec![]),
    ] {
        let edge = query(path, line, byte);
        assert_eq!(matching_defs(&edge, &defs), expected, "frozen scan");
        assert_eq!(buckets.matching_defs(&edge), expected, "bucket");
    }
    let counts = work_tests::cfg_selection_counts(|| {
        buckets.matching_defs(&query(field.clone(), 3, 30));
        buckets.matching_defs(&query(field.clone(), 3, 999));
        buckets.matching_defs(&query(field, 99, 30));
    });
    assert_eq!(counts.matching_defs_calls, 3);
    assert_eq!(counts.matching_defs_fallbacks, 2);
    assert_eq!(counts.matching_defs_items, 5);
}

#[test]
fn kill_alias_gen_binding_and_fields() {
    let source = "function f() {\nlet x=0;\nconsume(x);\n{\nlet x=1; x=2;\nconsume(x);\n}\nx=3;\nconsume(x);\na.b=1;\na=2;\nconsume(a.b);\n}\n";
    let parsed = ParsedFile::parse("kills.ts", source, Language::TypeScript).unwrap();
    let owner = parsed.all_functions()[0];
    let spans = parsed.assignment_lvalue_spans_on_lines(&owner, &(1..=14).collect());
    let originals: Vec<_> = spans
        .into_iter()
        .enumerate()
        .map(|(i, s)| site(i as u32, s.path, s.line, s.start_byte, false))
        .collect();
    assert!(originals.len() >= 6);
    let mut divergent_bindings = false;
    // Every definition is tested as alias-derived on either side of every pair.
    for alias in 0..=originals.len() {
        let mut defs = originals.clone();
        if alias < defs.len() {
            defs[alias].alias_derived = true;
        }
        let facts = scope::BindingFacts::new(&parsed, &owner, &defs);
        for together in [false, true] {
            let mut gen = vec![BitSet::new(defs.len()); 15];
            for (index, def) in defs.iter().enumerate() {
                gen[if together { 1 } else { def.line }].insert(index);
            }
            let buckets = def_buckets::DefinitionBuckets::new(&defs);
            let actual = buckets.kill_sets(&defs, &gen, &facts);
            let expected = def_buckets::legacy_kill_sets(&defs, &gen, &facts);
            assert_eq!(actual, expected, "alias {alias}, together {together}");
            divergent_bindings |= actual.0 != actual.1;
            if together {
                assert!(actual
                    .0
                    .iter()
                    .chain(&actual.1)
                    .all(|s| s.members().next().is_none()));
            }
        }
    }
    assert!(
        divergent_bindings,
        "shadowed equal paths must separate bound from flat kills"
    );
}

#[test]
fn differential_generated() {
    for n in [1, 4, 16] {
        for body in [
            "let x=p; x=2; consume(x);\n{ let x=p; consume(x); }\nx=3;\nconsume(x);\n",
            "let a=p;\nlet q=a;\na.b=1;\nq.b=2;\nconsume(q.b);\nconsume(a);\n",
            "let x=p;\ntry {\nx=2;\nconsume(x);\n} catch(e) {\nconsume(x);\n} finally {\nconsume(x);\n}\n",
            "let x=p;\nfunction inner() {\nconsume(x);\n}\nconsume(x);\n",
        ] {
            compare("generated.ts", &format!("function f(p) {{\n{}return p;\n}}",body.repeat(n)));
        }
    }
}

#[test]
fn differential_direct_rd_alias_twins_and_fallback() {
    let source = "function f() {\nlet x=0;\nconsume(x);\n{\nlet x=1; x=2;\nconsume(x);\n}\nx=3;\nconsume(x);\na.b=1;\na=2;\nconsume(x); consume(a.b); consume(a);\n}\n";
    let parsed = ParsedFile::parse("direct.ts", source, Language::TypeScript).unwrap();
    let owner = parsed.all_functions()[0];
    let original: Vec<_> = parsed
        .assignment_lvalue_spans_on_lines(&owner, &(1..=14).collect())
        .into_iter()
        .enumerate()
        .map(|(i, s)| site(i as u32, s.path, s.line, s.start_byte, false))
        .collect();
    let uses = parsed.rvalue_identifier_spans_on_lines(&owner, &BTreeSet::from([12]));
    let cfg_edges = cfg::build_cfg_edges_with_arms(&parsed);
    let cfg = FileCfgEdgeIndex::new(&cfg_edges);
    assert!(original.len() >= 6);
    for alias in 0..=original.len() {
        let mut defs = original.clone();
        if alias < defs.len() {
            defs[alias].alias_derived = true;
        }
        // A duplicate observation exercises deduplication's alias-OR semantics
        // before building either index; equal-key matches are tested above too.
        defs.push(defs[0].clone());
        for exact_miss in [false, true] {
            let edges: Vec<_> = defs
                .iter()
                .filter_map(|def| {
                    let span = uses.iter().find(|span| span.path == def.path)?;
                    let mut edge = query(def.path.clone(), def.line, def.start_byte);
                    edge.from.file = parsed.path.clone();
                    if exact_miss {
                        edge.from.start_byte += 100_000;
                    }
                    edge.to = VarLocation {
                        file: parsed.path.clone(),
                        line: span.line,
                        start_byte: span.start_byte,
                        end_byte: span.end_byte,
                        ..edge.to
                    };
                    Some(edge)
                })
                .collect();
            assert!(edges.len() >= 4);
            let mode = Mode::begin(true);
            let old = reaching_definitions_with_exact(&parsed, &owner, &defs, &edges, &edges, &cfg);
            let old_observation = Mode::take();
            drop(mode);
            let mode = Mode::begin(false);
            let new = reaching_definitions_with_exact(&parsed, &owner, &defs, &edges, &edges, &cfg);
            let new_observation = Mode::take();
            drop(mode);
            assert!(matches!(new, RdOutcome::Available(_)));
            assert_eq!(new_observation.kills.len(), 1);
            assert_eq!(old, new, "alias {alias}, exact miss {exact_miss}");
            assert_eq!(
                old_observation, new_observation,
                "both kill/in-set families: alias {alias}, exact miss {exact_miss}"
            );
        }
    }
}
