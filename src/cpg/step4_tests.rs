//! Whole-graph identity and cost regressions for ID-based Step 4.
use super::*;
use crate::cpg::dfg_observation::{fieldwise_payload, lookup_indexes};
use crate::data_flow::{test_support, ExactFlowEdge, FlowEdge};
use crate::languages::Language;
use std::cell::Cell;

thread_local! { static ENDPOINT_PROBES: Cell<usize> = const { Cell::new(0) }; }
pub(super) fn endpoint_probe() {
    ENDPOINT_PROBES.with(|c| c.set(c.get() + 1));
}

fn parsed(ext: &str, language: Language, source: &str) -> BTreeMap<String, ParsedFile> {
    let path = format!("unit.{ext}");
    let file = ParsedFile::parse(&path, source, language).unwrap();
    assert!(!file.tree.root_node().has_error());
    BTreeMap::from([(path, file)])
}

fn compare(files: &BTreeMap<String, ParsedFile>, dfg: DataFlowGraph) -> CodePropertyGraph {
    let cg = CallGraph::build(files);
    let candidate = CodePropertyGraph::assemble_graph(cg.clone(), dfg.clone(), files, None);
    let reference =
        step4_reference::with_reference(|| CodePropertyGraph::assemble_graph(cg, dfg, files, None));
    assert_eq!(fieldwise_payload(&candidate), fieldwise_payload(&reference));
    assert_eq!(lookup_indexes(&candidate), lookup_indexes(&reference));
    candidate
}

fn synthetic(functions: usize, width: usize) -> BTreeMap<String, ParsedFile> {
    let mut files = BTreeMap::new();
    for file in 0..4 {
        let path = format!("dense{file}.go");
        let mut source = "package dense\n".to_string();
        for f in 0..functions {
            source.push_str(&format!("func flow{f}(x int) int {{\n"));
            for i in 0..width {
                source.push_str(&format!("y{i} := x\n"));
            }
            source.push_str("return x");
            for i in 0..width {
                source.push_str(&format!(" + y{i}"));
            }
            source.push_str("\n}\n");
        }
        files.insert(
            path.clone(),
            ParsedFile::parse(&path, &source, Language::Go).unwrap(),
        );
    }
    files
}

#[test]
fn step4_complete_graph_bytes_and_indexes() {
    crate::build_pool::install(|| {
        for (ext, lang, source) in [
            (
                "go",
                Language::Go,
                "package p\nfunc f(x int) int {\ny := x\nfor y < 4 {\ny = y + x\n}\nreturn y\n}\n",
            ),
            (
                "js",
                Language::JavaScript,
                "function f(x){let y=x; y=y+x; return y+y;} function g(z){return f(z);}",
            ),
            (
                "ts",
                Language::TypeScript,
                "function f(x:number){let y=x; y=y+x; return y+y;}",
            ),
            (
                "tsx",
                Language::Tsx,
                "function f(x:number){let y=x; return <div>{y}{y}</div>;}",
            ),
            (
                "py",
                Language::Python,
                "def f(x):\n    y = x\n    y = y + x\n    return y\n",
            ),
            (
                "rs",
                Language::Rust,
                "fn f(x:i32)->i32 { let mut y=x; y=y+x; y }",
            ),
            ("js", Language::JavaScript, ""),
        ] {
            let files = parsed(ext, lang, source);
            compare(&files, DataFlowGraph::build(&files));
        }
        let files = synthetic(12, 24);
        compare(&files, DataFlowGraph::build(&files));
    });
}

#[test]
fn step4_missing_labels_and_conflicting_exact_bindings() {
    crate::build_pool::install(|| {
        let files = parsed(
            "js",
            Language::JavaScript,
            "function f(x){let y=x; y=y+x; return y+y;}\n",
        );
        let original = DataFlowGraph::build(&files);
        assert!(!original.edges().is_empty());
        let mut missing = original.clone();
        test_support::edit_legacy_parts(&mut missing, |p| {
            p.labels.clear();
            p.exact_labels.clear();
        });
        let graph = compare(&files, missing);
        assert!(graph.dfg_label_stats.dfg_rd_functions_without_cfg > 0);
        let mut conflicting = original.clone();
        test_support::edit_legacy_parts(&mut conflicting, |p| {
            // Force an exact overlap with the opposite confidence. Additional
            // siblings from this binding must all be refused together.
            let ((from, to), label) = p.labels.iter_mut().next().unwrap();
            *label = FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete);
            p.exact_labels
                .insert(ExactFlowEdge::from_refs(from, to), FlowConfidence::Exact);
        });
        compare(&files, conflicting);
    });
}

#[test]
fn step4_resolves_each_occurrence_once_without_legacy_label_searches() {
    crate::build_pool::install(|| {
        let files = synthetic(2, 32);
        let cg = CallGraph::build(&files);
        let dfg = DataFlowGraph::build(&files);
        let occurrences = dfg.assembly_view().len();
        let run = |reference| {
            DataFlowGraph::reset_assembly_label_probes();
            ENDPOINT_PROBES.with(|c| c.set(0));
            let build = || CodePropertyGraph::assemble_graph(cg.clone(), dfg.clone(), &files, None);
            let graph = if reference {
                step4_reference::with_reference(build)
            } else {
                build()
            };
            (
                graph,
                DataFlowGraph::assembly_label_probes(),
                ENDPOINT_PROBES.with(Cell::get),
            )
        };
        let (old, old_labels, old_endpoints) = run(true);
        // Switch only the measured route to the frozen pre-change algorithm
        // for a RED receipt of these same assertions.
        let (new, labels, endpoints) = run(std::env::var_os("PRISM_STEP4_REFERENCE").is_some());
        eprintln!("Step 4 cost: legacy labels {old_labels}->{labels}; endpoints {old_endpoints}->{endpoints}; occurrences {occurrences}");
        assert_eq!(fieldwise_payload(&old), fieldwise_payload(&new));
        assert!(old_labels > 0);
        assert!(old_endpoints > occurrences, "fixture must reuse endpoints");
        assert_eq!(
            labels, 0,
            "Step 4 must use the existing compact legacy ranks"
        );
        assert!(
            endpoints <= occurrences,
            "resolve at most once per occurrence"
        );
    });
}

#[test]
#[ignore = "synthetic timing; run explicitly in release mode"]
fn step4_synthetic_timing() {
    crate::build_pool::install(|| {
        let files = synthetic(80, 64);
        let dfg = DataFlowGraph::build(&files);
        let mut prefix = DiGraph::new();
        let mut vars = BTreeMap::new();
        let mut exact = ExactVarIndex::new();
        let mut locations = BTreeMap::new();
        for (access, rows) in [
            (VarAccess::Def, dfg.defs().collect::<Vec<_>>()),
            (VarAccess::Use, dfg.uses().collect::<Vec<_>>()),
        ] {
            for (_, locs) in rows {
                for loc in locs {
                    CodePropertyGraph::materialize_var_location(
                        loc,
                        access,
                        &files,
                        &mut prefix,
                        &mut vars,
                        &mut exact,
                        &mut locations,
                    );
                }
            }
        }
        let mode = std::env::var("PRISM_STEP4_TIMING_MODE").unwrap_or_default();
        let routes: &[bool] = match mode.as_str() {
            "reference" => &[true, true, true],
            "candidate" => &[false, false, false],
            _ => &[true, false, false, true],
        };
        let mut expected = None;
        for &reference in routes {
            let mut graph = prefix.clone();
            let mut flow = dfg.clone();
            let start = std::time::Instant::now();
            let stats = if reference {
                CodePropertyGraph::assemble_step4_reference(
                    &mut flow, &files, &vars, &exact, &mut graph,
                )
            } else {
                CodePropertyGraph::assemble_step4(&mut flow, &files, &vars, &exact, &mut graph)
            };
            let elapsed = start.elapsed();
            let bytes = bincode::serialize(&(
                graph.node_indices().map(|n| &graph[n]).collect::<Vec<_>>(),
                graph
                    .edge_references()
                    .map(|e| (e.source().index(), e.target().index(), e.weight()))
                    .collect::<Vec<_>>(),
                &stats,
                flow.rd_function_stats(),
            ))
            .unwrap();
            if let Some(ref expected) = expected {
                assert_eq!(&bytes, expected);
            } else {
                expected = Some(bytes);
            }
            eprintln!("step4 synthetic reference={reference} step4_us={} nodes={} edges={} occurrences={} endpoint_cache_bytes={}", elapsed.as_micros(), graph.node_count(), graph.edge_count(), dfg.assembly_view().len(), dfg.assembly_view().len() * std::mem::size_of::<Option<Option<NodeIndex>>>());
            std::hint::black_box(graph);
        }
    });
}

fn location(line: usize, start: usize, kind: VarAccessKind) -> VarLocation {
    VarLocation {
        file: "unit.js".into(),
        function: "f".into(),
        function_start_line: 1,
        line,
        path: AccessPath::simple("x"),
        start_byte: start,
        end_byte: start + 1,
        kind,
    }
}

#[test]
fn step4_supplemental_and_missing_endpoint_matrix() {
    crate::build_pool::install(|| {
        let files = parsed("js", Language::JavaScript, "function f(x){return x+x;}\n");
        let def = location(1, 11, VarAccessKind::Def);
        let use1 = location(1, 21, VarAccessKind::Use);
        let use2 = location(1, 23, VarAccessKind::Use);
        let edge = FlowEdge {
            from: def.clone(),
            to: use1.clone(),
        };
        let sibling = FlowEdge {
            from: def.clone(),
            to: use2.clone(),
        };
        for legacy in [false, true] {
            for confidence in [
                FlowConfidence::Exact,
                FlowConfidence::NameOnly(FlowDoubt::SameLine),
                FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete),
            ] {
                let dfg = test_support::step4_graph(
                    if legacy {
                        vec![edge.clone(), edge.clone()]
                    } else {
                        vec![]
                    },
                    vec![(edge.clone(), confidence), (sibling.clone(), confidence)],
                    vec![def.clone(), use1.clone(), use2.clone()],
                );
                let cpg = compare(&files, dfg);
                assert!(cpg
                    .graph
                    .edge_references()
                    .any(|e| matches!(e.weight(), CpgEdge::DataFlow(_))));
            }
        }
        // Refusals: absent endpoint, zero-byte occurrence, wrong access,
        // mismatched owner/file/path, compound path and unsupported language.
        for case in 0..10 {
            let mut edge = sibling.clone();
            let mut locations = vec![def.clone(), use2.clone()];
            let mut case_files = files.clone();
            match case {
                0 => {
                    locations.pop();
                }
                1 => {
                    edge.to.end_byte = edge.to.start_byte;
                    locations[1] = edge.to.clone();
                }
                2 => {
                    edge.from.kind = VarAccessKind::Use;
                    locations[0] = edge.from.clone();
                }
                3 => {
                    edge.to.function = "other".into();
                    locations[1] = edge.to.clone();
                }
                4 => {
                    edge.to.file = "other.js".into();
                    locations[1] = edge.to.clone();
                }
                5 => {
                    edge.to.path = AccessPath::simple("other");
                    locations[1] = edge.to.clone();
                }
                6 => {
                    edge.from.path = AccessPath::from_expr("x.field");
                    edge.to.path = edge.from.path.clone();
                    locations = vec![edge.from.clone(), edge.to.clone()];
                }
                7 => {
                    case_files = parsed(
                        "go",
                        Language::Go,
                        "package p\nfunc f(x int) int { return x }\n",
                    );
                }
                8 => {
                    edge.to.function_start_line = 2;
                    locations[1] = edge.to.clone();
                }
                9 => {
                    edge.from.end_byte = edge.from.start_byte;
                    locations[0] = edge.from.clone();
                }
                _ => unreachable!(),
            }
            let cpg = compare(
                &case_files,
                test_support::step4_graph(vec![], vec![(edge, FlowConfidence::Exact)], locations),
            );
            assert!(
                !cpg.graph
                    .edge_references()
                    .any(|e| matches!(e.weight(), CpgEdge::DataFlow(_))),
                "refusal case {case}"
            );
        }
        // Backwards exact edge increments loop-carried count. Legacy missing
        // endpoints never inflate the without-CFG identity set.
        let back = FlowEdge {
            from: location(3, 11, VarAccessKind::Def),
            to: location(2, 21, VarAccessKind::Use),
        };
        let cpg = compare(
            &files,
            test_support::step4_graph(
                vec![],
                vec![(back.clone(), FlowConfidence::Exact)],
                vec![back.from, back.to],
            ),
        );
        assert_eq!(cpg.dfg_label_stats.dfg_label_loop_carried, 1);
        let cpg = compare(
            &files,
            test_support::step4_graph(vec![edge.clone(), edge], vec![], vec![def]),
        );
        assert_eq!(cpg.dfg_label_stats.dfg_rd_functions_without_cfg, 0);
    });
}

#[test]
fn step4_exact_endpoint_ambiguity_and_stale_index() {
    let files = parsed("js", Language::JavaScript, "function f(x){return x;}\n");
    let def = location(1, 11, VarAccessKind::Def);
    let read = location(1, 21, VarAccessKind::Use);
    for ambiguous in [false, true] {
        let mut graph = DiGraph::new();
        let mut vars = BTreeMap::new();
        let mut exact = ExactVarIndex::new();
        let mut locations = BTreeMap::new();
        for loc in [&def, &read] {
            let access = if loc.kind == VarAccessKind::Def {
                VarAccess::Def
            } else {
                VarAccess::Use
            };
            CodePropertyGraph::materialize_var_location(
                loc,
                access,
                &files,
                &mut graph,
                &mut vars,
                &mut exact,
                &mut locations,
            );
        }
        let key = CodePropertyGraph::legacy_var_key(&read, VarAccess::Use);
        let read_node = vars[&key];
        if ambiguous {
            let duplicate = graph.add_node(graph[read_node].clone());
            exact
                .get_mut(&key)
                .unwrap()
                .get_mut(&(read.start_byte, read.end_byte))
                .unwrap()
                .push(duplicate);
        } else {
            // A bucket pointing at a different occurrence must be ignored.
            if let CpgNode::Variable { start_byte, .. } = &mut graph[read_node] {
                *start_byte += 1;
            }
        }
        let edge = FlowEdge {
            from: def.clone(),
            to: read.clone(),
        };
        let dfg = test_support::step4_graph(
            vec![edge.clone(), edge.clone()],
            vec![(edge, FlowConfidence::Exact)],
            vec![],
        );
        let mut candidate = graph.clone();
        let mut reference = graph;
        let mut new_dfg = dfg.clone();
        let mut old_dfg = dfg;
        ENDPOINT_PROBES.with(|c| c.set(0));
        let new =
            CodePropertyGraph::assemble_step4(&mut new_dfg, &files, &vars, &exact, &mut candidate);
        assert_eq!(
            ENDPOINT_PROBES.with(Cell::get),
            2,
            "refused endpoints must also be cached"
        );
        let old = CodePropertyGraph::assemble_step4_reference(
            &mut old_dfg,
            &files,
            &vars,
            &exact,
            &mut reference,
        );
        assert_eq!(new, old);
        assert_eq!(candidate.edge_count(), 0);
        assert_eq!(reference.edge_count(), 0);
    }
}

#[test]
fn step4_conflict_refuses_entire_binding_across_occurrence_ids() {
    crate::build_pool::install(|| {
        let files = parsed("js", Language::JavaScript, "function f(x){return x+x;}\n");
        let def = location(1, 11, VarAccessKind::Def);
        let other_id = location(2, 11, VarAccessKind::Def);
        let use1 = location(1, 21, VarAccessKind::Use);
        let use2 = location(2, 23, VarAccessKind::Use);
        let legacy = FlowEdge {
            from: def.clone(),
            to: use1.clone(),
        };
        let sibling = FlowEdge {
            from: other_id.clone(),
            to: use2.clone(),
        };
        let dfg = test_support::step4_graph(
            vec![legacy.clone()],
            vec![
                (legacy, FlowConfidence::Exact),
                (sibling, FlowConfidence::Exact),
            ],
            vec![def, other_id, use1, use2],
        );
        let cpg = compare(&files, dfg);
        assert_eq!(
            cpg.graph
                .edge_references()
                .filter(|e| matches!(e.weight(), CpgEdge::DataFlow(_)))
                .count(),
            1
        );
        assert_eq!(cpg.dfg_label_stats.dfg_label_exact, 0);
        assert_eq!(cpg.dfg_label_stats.dfg_label_nameonly_cfg_incomplete, 1);
    });
}
