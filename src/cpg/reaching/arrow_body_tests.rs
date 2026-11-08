//! #759: an expression-bodied arrow whose body is a callable keeps one node. Rows are
//! `tests/fixtures/js_write_census/arrow-body-matrix.tsv`; `expected` is the target label of every
//! owner's edge, sorted and joined by `+` (the nested edge has one label per owner); `main` is the same
//! on 1ee1cbb8. Label-only: every row keeps its edge, and no row moves toward `exact`.
use super::super::census_defs_tests::{availability, parse, recorded, trace_lines};
use crate::cpg::census_label_tests::labels_of;
use crate::languages::Language;
use std::collections::BTreeSet;

const MATRIX: &str = include_str!("../../../tests/fixtures/js_write_census/arrow-body-matrix.tsv");

struct Row {
    fixture: String,
    lang: Language,
    edge: String,
    main: String,
    expected: String,
    source: String,
}

fn rows() -> Vec<Row> {
    MATRIX
        .lines()
        .skip(1)
        .map(|line| {
            let c: Vec<&str> = line.split('\t').collect();
            assert_eq!(c.len(), 7, "matrix row: {line}");
            Row {
                fixture: c[0].to_string(),
                lang: match c[1] {
                    "ts" => Language::TypeScript,
                    "tsx" => Language::Tsx,
                    "js" => Language::JavaScript,
                    other => panic!("language {other}"),
                },
                edge: c[2].to_string(),
                main: c[3].to_string(),
                expected: c[4].to_string(),
                source: c[6].replace("\\n", "\n"),
            }
        })
        .collect()
}

/// Rows whose new one-node universe trips the distinct-bases belt (two recorded defs of one name).
const BELT_ROWS: &[&str] = &[
    "e01_two_defs_in_nested_body_belt",
    "f01_ternary_arms",
    "f02_ternary_formal_arm_def",
    "f03_ternary_arm_def_other_arm_use",
    "f04_ternary_each_arm_defines_and_reads",
    "f05_nested_ternary",
    "f11_ts_ternary",
    "f12_tsx_ternary",
];

fn sorted_labels(r: &Row) -> String {
    let mut got = labels_of(r.lang, &r.source, &r.edge);
    got.sort();
    got.join("+")
}

#[test]
fn rows_have_the_expected_labels() {
    let all = rows();
    assert_eq!(all.len(), 51, "matrix rows");
    let failures: Vec<String> = all
        .iter()
        .filter_map(|r| {
            let got = sorted_labels(r);
            (got != r.expected)
                .then(|| format!("{} {}: want {}, got {got}", r.fixture, r.edge, r.expected))
        })
        .collect();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// Label-only: each row's edge exists, and a moved row never gains an `exact`.
#[test]
fn rows_keep_their_edges_and_none_moves_toward_exact() {
    for r in rows() {
        assert!(
            !sorted_labels(&r).is_empty(),
            "{}: edge disappeared",
            r.fixture
        );
        let count = |s: &str| s.split('+').filter(|l| *l == "exact").count();
        assert!(
            count(&r.expected) <= count(&r.main),
            "{}: {} -> {}",
            r.fixture,
            r.main,
            r.expected
        );
    }
}

/// Owners whose universe the rule changed: named expression-bodied arrows whose body is a callable
/// holding two or more located statements.
fn universe_moved(r: &Row) -> bool {
    let parsed = parse(r.lang, &r.source);
    parsed
        .all_functions()
        .iter()
        .any(|f| super::arrow_control::universe_moved(&parsed, f))
}

/// The rule's population (callable and ternary bodies with 2+ located statements). Every row whose label
/// moved is in it; eight more change universe but their edge was already refused (S1a / D2-c: `a02`, `a08`,
/// `a13`, `c06`; already non-exact or the belt: `f05`, `f06`'s edge is unchanged) or belongs to an owner that
/// carries none of the nested callable's edges (`a07`'s anonymous middle arrow is an RD owner, `d04`'s
/// anonymous default export has no labelled edge).
#[test]
fn moved_rows_are_one_node_body_owners() {
    let (mut moved, mut universe) = (0, 0);
    for r in rows() {
        let flagged = universe_moved(&r);
        assert!(flagged || r.main == r.expected, "{}", r.fixture);
        moved += usize::from(r.main != r.expected);
        universe += usize::from(flagged);
    }
    assert_eq!((moved, universe), (20, 28));
}

/// The solver and the cap preflight use one universe, so they agree on every row's owners.
#[test]
fn preflight_and_solver_agree_on_every_row() {
    let mut owners = 0;
    for r in rows() {
        let parsed = parse(r.lang, &r.source);
        for owner in parsed.all_functions() {
            let (defs, _) = recorded(&parsed, &owner);
            let (pre, solver) = availability(&parsed, &owner, &defs);
            assert_eq!(pre, solver, "{}", r.fixture);
            owners += 1;
        }
    }
    assert!(owners >= 39, "{owners}");
}

/// Trace lines of one edge (`path@line->path@line`): `(mechanism, owner)` per `rd-census` line.
fn edge_lines(r: &Row) -> Vec<(String, String)> {
    let (from, to) = r.edge.split_once("->").unwrap();
    let (from, to) = (format!("{from}:"), format!("{to}:"));
    trace_lines(r.lang, &r.source)
        .iter()
        .map(|l| l.split('\t').collect::<Vec<_>>())
        .filter(|f| f.len() == 6 && f[0] == "rd-census")
        .filter(|f| f[4].starts_with(&from) && f[5].starts_with(&to))
        .map(|f| (f[1].to_string(), f[3].to_string()))
        .collect()
}

/// A moved row's edge is named `arrow-body-node`, once, and no other mechanism names it for that
/// owner (the nested callable's own owner may name its own edge); an unmoved row's edge is never named.
#[test]
fn the_trace_names_each_moved_edge_once() {
    for r in rows() {
        let lines = edge_lines(&r);
        let arrow: Vec<_> = lines
            .iter()
            .filter(|(m, _)| m == "arrow-body-node")
            .collect();
        if r.main != r.expected {
            assert_eq!(arrow.len(), 1, "{} {lines:?}", r.fixture);
            let owner = &arrow[0].1;
            let by_owner = lines.iter().filter(|(_, o)| o == owner).count();
            assert_eq!(by_owner, 1, "{} {lines:?}", r.fixture);
        } else {
            assert!(arrow.is_empty(), "{} {lines:?}", r.fixture);
        }
    }
}

/// One owner line per owner whose universe moved, none for an owner the rule left alone; the old guard's trace never
/// claims a moved owner.
#[test]
fn owner_lines_are_one_per_moved_owner() {
    for r in rows() {
        let lines = trace_lines(r.lang, &r.source);
        let owners: BTreeSet<&String> = lines
            .iter()
            .filter(|l| l.starts_with("rd-census-owner\t"))
            .collect();
        let arrow: Vec<_> = owners
            .iter()
            .filter(|l| l.contains("\tarrow-body-node\t"))
            .collect();
        let flagged = universe_moved(&r);
        assert_eq!(
            arrow.len(),
            usize::from(flagged),
            "{} {owners:?}",
            r.fixture
        );
        // Availability flips: only the belt rows turn an available owner unavailable.
        for a in &arrow {
            let flip = a.split('\t').nth(5).unwrap();
            let want = if BELT_ROWS.contains(&r.fixture.as_str()) {
                "ok>no-cfg"
            } else {
                "ok>ok"
            };
            assert_eq!(flip, want, "{} {owners:?}", r.fixture);
        }
        // The old guard's line never names an owner this rule moved (`a07`'s outer arrow keeps its own).
        for a in &arrow {
            let owner: Vec<&str> = a.split('\t').collect();
            let guard = owners.iter().any(|g| {
                let f: Vec<&str> = g.split('\t').collect();
                f[1].starts_with("guard-deleted") && f[3] == owner[3] && f[4] == owner[4]
            });
            assert!(!guard, "{} {owners:?}", r.fixture);
        }
    }
}

/// SOL-781-01: an edge supplied twice (`dfg_edges = [e, e]`; the producer emits per lvalue and the legacy
/// key has no bytes) is named once. The input is doubled directly, not rebuilt from a label map.
#[test]
fn a_duplicated_input_edge_is_named_once() {
    for name in [
        "a01_issue_arrow_in_arrow",
        "e01_two_defs_in_nested_body_belt",
        "f01_ternary_arms",
    ] {
        let r = rows().into_iter().find(|r| r.fixture == name).unwrap();
        let parsed = parse(r.lang, &r.source);
        let mut named = 0;
        for owner in parsed.all_functions() {
            let (defs, edges) = recorded(&parsed, &owner);
            let doubled: Vec<crate::data_flow::FlowEdge> =
                edges.iter().chain(edges.iter()).cloned().collect();
            let cfg = crate::cfg::build_cfg_edges_with_arms(&parsed);
            let index = super::super::FileCfgEdgeIndex::new(&cfg);
            let mut out = Vec::new();
            super::traced(&parsed, &owner, &defs, &doubled, &[], &index, &mut out);
            let text = String::from_utf8(out).unwrap();
            let lines: Vec<&str> = text
                .lines()
                .filter(|l| l.starts_with("rd-census\tarrow-body-node\t"))
                .collect();
            let distinct: BTreeSet<&&str> = lines.iter().collect();
            assert_eq!(lines.len(), distinct.len(), "{name}: {lines:?}");
            named += lines.len();
        }
        assert!(named > 0, "{name}: nothing was named");
    }
}

/// Opus S2: the control writes no capture lines of its own (the production pass and the `#674` control
/// already do). The call site reads the trace switch from the environment, which tests do not set, so the
/// predicate is pinned directly.
#[test]
fn only_the_arrow_control_and_the_census_free_control_skip_capture_lines() {
    use super::Pass;
    assert!(!Pass::ArrowControl.traces_capture());
    assert!(!Pass::NoCensus.traces_capture());
    for pass in [Pass::Census, Pass::CensusLogged, Pass::CensusEqualKills] {
        assert!(pass.traces_capture());
    }
}
