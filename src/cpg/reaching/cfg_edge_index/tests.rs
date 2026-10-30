use super::*;
use crate::ast::ParsedFile;
use crate::cfg::{self, ArmSpan, EdgeOrigin};
use crate::cpg::reaching::{RdOutcome, RdResult};
use crate::data_flow::DataFlowGraph;
use crate::languages::Language;
use std::cell::{Cell, RefCell};
use std::collections::BTreeSet;

#[derive(Default, Debug, PartialEq, Eq)]
struct Observation {
    admitted: Vec<Vec<SelectedEdge>>,
    outcomes: Vec<RdOutcome>,
}

thread_local! {
    static LEGACY: Cell<bool> = const { Cell::new(false) };
    static OBSERVATION: RefCell<Option<Observation>> = const { RefCell::new(None) };
}

pub(super) fn legacy_enabled() -> bool {
    LEGACY.with(Cell::get)
}

pub(super) fn record_edges(edges: &[SelectedEdge]) {
    OBSERVATION.with(|slot| {
        if let Some(observation) = slot.borrow_mut().as_mut() {
            observation.admitted.push(edges.to_vec());
        }
    });
}

pub(in crate::cpg::reaching) fn record_outcome(outcome: &RdOutcome) {
    OBSERVATION.with(|slot| {
        if let Some(observation) = slot.borrow_mut().as_mut() {
            observation.outcomes.push(outcome.clone());
        }
    });
}

struct Mode;
impl Mode {
    fn begin(legacy: bool) -> Self {
        LEGACY.with(|mode| assert!(!mode.replace(legacy)));
        OBSERVATION.with(|slot| assert!(slot.replace(Some(Observation::default())).is_none()));
        Self
    }
    fn take() -> Observation {
        OBSERVATION.with(|slot| slot.take().unwrap())
    }
}
impl Drop for Mode {
    fn drop(&mut self) {
        LEGACY.with(|mode| mode.set(false));
        OBSERVATION.with(|slot| slot.take());
    }
}

fn compare(parsed: &ParsedFile) -> Observation {
    rayon::ThreadPoolBuilder::new()
        .num_threads(1)
        .stack_size(64 * 1024 * 1024)
        .build()
        .unwrap()
        .install(|| {
            let files = BTreeMap::from([(parsed.path.clone(), parsed.clone())]);
            let old_mode = Mode::begin(true);
            let old = DataFlowGraph::build_on_current_pool(&files);
            let old_observation = Mode::take();
            drop(old_mode);
            let new_mode = Mode::begin(false);
            let new = DataFlowGraph::build_on_current_pool(&files);
            let new_observation = Mode::take();
            drop(new_mode);
            assert_eq!(
                old_observation, new_observation,
                "complete RD and ordered admitted edges: {}",
                parsed.path
            );
            assert_eq!(
                crate::cpg::dfg_observation::dfg_storage(&old),
                crate::cpg::dfg_observation::dfg_storage(&new),
                "DFG labels, exact spans, kill lines and RD statistics: {}",
                parsed.path
            );
            new_observation
        })
}

fn tuple(
    from_line: usize,
    to_line: usize,
    origin: EdgeOrigin,
    cross: bool,
) -> (CfgEdge, ArmProvenance) {
    let from_arm = ArmSpan {
        id: 1,
        subtree_last: 1,
    };
    let to_arm = if cross {
        ArmSpan {
            id: 2,
            subtree_last: 2,
        }
    } else {
        from_arm
    };
    (
        CfgEdge {
            file: "same.ts".into(),
            from_line,
            to_line,
        },
        ArmProvenance {
            origin,
            from_arm,
            to_arm,
        },
    )
}

#[test]
fn ordinal_order_duplicates_provenance_and_overlapping_lines() {
    use EdgeOrigin::{Sequential, Structured};
    // File order differs from source-line order; keep two identical tuples,
    // one duplicate endpoint with different provenance, a self and back-edge.
    let edges = vec![
        tuple(4, 2, Structured, false),
        tuple(2, 3, Sequential, true),
        tuple(2, 3, Structured, true),
        tuple(3, 9, Structured, false),
        tuple(9, 2, Sequential, false),
        tuple(3, 3, Sequential, false),
        tuple(4, 2, Structured, false),
        tuple(3, 2, Structured, false),
    ];
    let index = FileCfgEdgeIndex::new(&edges);
    for lines in [vec![], vec![999], vec![2, 3], vec![2, 3, 4], vec![3, 4]] {
        let lines: BTreeMap<_, _> = lines
            .into_iter()
            .enumerate()
            .map(|(i, line)| (line, 7 + i * 3))
            .collect();
        assert_eq!(index.for_owner(&lines), index.legacy_for_owner(&lines));
    }
    let lines = BTreeMap::from([(2, 7), (3, 9), (4, 2)]);
    let selected = index.for_owner(&lines);
    assert_eq!(selected.len(), 6);
    assert_eq!(
        selected[0], selected[4],
        "identical original tuples remain duplicated"
    );
    assert_eq!(selected[1].2, selected[2].2);
    assert_ne!(selected[1].3, selected[2].3);
    assert_eq!(selected[3].2.from_line, selected[3].2.to_line);
    assert!(selected.last().unwrap().2.from_line > selected.last().unwrap().2.to_line);
}

#[test]
fn rejected_destinations_are_examined_but_never_cloned() {
    let edges = vec![
        tuple(2, 3, EdgeOrigin::Sequential, false),
        tuple(2, 99, EdgeOrigin::Structured, false),
        tuple(99, 2, EdgeOrigin::Structured, false),
    ];
    let index = FileCfgEdgeIndex::new(&edges);
    let lines = BTreeMap::from([(2, 1), (3, 2)]);
    let counts = crate::cpg::reaching::work_tests::cfg_selection_counts(|| {
        assert_eq!(index.for_owner(&lines).len(), 1);
    });
    assert_eq!(counts.cfg_candidates, 2);
    assert_eq!(counts.cfg_cloned, 1);
    assert_eq!(counts.cfg_admitted, 1);
    let empty = FileCfgEdgeIndex::new(&[]);
    assert!(empty.for_owner(&lines).is_empty());
}

fn language_cases() -> Vec<(&'static str, &'static str)> {
    vec![
        ("a.c", "int f(int p) {\n int x=p;\n while(p) {\n x=p;\n use(x);\n }\n x=2;\n use(x);\n return x;\n}\n"),
        ("a.cpp", "int f(int p) {\n int x=p;\n while(p) {\n x=p;\n use(x);\n }\n x=2;\n use(x);\n return x;\n}\n"),
        ("a.go", "package p\nfunc f(p int) int {\n x := p\n for p > 0 {\n x = p\n use(x)\n }\n x = 2\n use(x)\n return x\n}\n"),
        ("a.rs", "fn f(p: i32) -> i32 {\n let mut x=p;\n while x > 0 {\n x=p;\n use_x(x);\n }\n x=2;\n use_x(x);\n x\n}\n"),
        ("a.py", "def f(p):\n    x = p\n    while p:\n        x = p\n        use(x)\n    x = 2\n    use(x)\n    return x\n"),
        ("a.js", "function f(p) {\n let x=p;\n while(p) {\n x=p;\n use(x);\n }\n x=2;\n use(x);\n return x;\n}\n"),
        ("a.ts", "function f(p: number) {\n let x=p;\n while(p) {\n x=p;\n use(x);\n }\n x=2;\n use(x);\n return x;\n}\n"),
        ("a.tsx", "function F(p: number) {\n let x=p;\n while(p) {\n x=p;\n use(x);\n }\n x=2;\n use(x);\n return <X x={x}/>;\n}\n"),
        ("a.java", "class C { int f(int p) {\n int x=p;\n while(p>0) {\n x=p;\n use(x);\n }\n x=2;\n use(x);\n return x;\n} }\n"),
        ("a.lua", "function f(p)\n local x=p\n while p do\n x=p\n use(x)\n end\n x=2\n use(x)\n return x\nend\n"),
        ("a.sh", "f() {\n local x=1\n echo \"$x\"\n x=2\n echo \"$x\"\n}\n"),
        ("a.tf", "locals {\n x = 1\n y = local.x\n z = local.y\n}\n"),
    ]
}

#[test]
fn complete_rd_differential_all_file_cfg_languages() {
    let mut covered = BTreeSet::new();
    let mut available = 0;
    let mut killed = false;
    for (path, source) in language_cases() {
        let language = Language::from_path(path).unwrap();
        let parsed = ParsedFile::parse(path, source, language).unwrap();
        let cfg = cfg::build_cfg_edges_with_arms(&parsed);
        let observation = compare(&parsed);
        if !cfg.is_empty() {
            covered.insert(language);
            assert!(!observation.admitted.is_empty(), "{path}");
            assert!(
                observation.admitted.iter().any(|edges| !edges.is_empty()),
                "{path}"
            );
        } else {
            assert_eq!(
                language,
                Language::Terraform,
                "unexpected missing CFG: {path}"
            );
        }
        for outcome in observation.outcomes {
            if let RdOutcome::Available(RdResult { labels, .. }) = outcome {
                available += 1;
                killed |= labels.values().any(|label| {
                    matches!(
                        label,
                        crate::cpg::FlowConfidence::NameOnly(crate::cpg::FlowDoubt::Killed { .. })
                    )
                });
            }
        }
    }
    let expected: BTreeSet<_> = Language::all()
        .into_iter()
        .filter(|l| *l != Language::Terraform)
        .collect();
    assert_eq!(covered, expected);
    assert!(available >= 11);
    assert!(killed, "kill-line comparison must not be vacuous");
    eprintln!(
        "file CFG languages: {covered:?}; Terraform emitted none; {available} available RD outputs"
    );
}

#[test]
fn complete_rd_differential_nested_disjoint_and_same_line() {
    let cases = [
        "function outer(p) {\n let x=p;\n function inner(q) {\n let z=q;\n use(z);\n return z;\n }\n use(x);\n return x;\n}\n",
        "function f(p) {\n let x=p; x=2; use(x);\n while (p) { x=p; use(x); }\n return x;\n}\nfunction g(p) {\n let y=p;\n use(y);\n return y;\n}\n",
        "function f(c) {\n let x=0;\n if(c) {\n x=1;\n } else {\n x=2;\n }\n use(x);\n return x;\n}\n",
        "function empty() {}\nfunction noedges(p) { return p; }\n",
    ];
    for source in cases {
        let parsed = ParsedFile::parse("owners.ts", source, Language::TypeScript).unwrap();
        let observation = compare(&parsed);
        assert!(!observation.outcomes.is_empty());
    }
}
