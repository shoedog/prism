//! #660 S1b-1: every `variant-matrix.tsv` row whose slice names S1b-1 as a label row. A write the
//! producer never records (update, pattern default, member target, `delete`, wrapped or member `for`
//! head, eager class position) is a kill-only def for JS/TS/TSX owners; an escaped spelling or an
//! unmappable eager site refuses its owner or base. Label-only: no edge moves.
//! FIX rows are `exact` on base 72a23975 and move to the expected label; controls hold on both.
use super::*;
use crate::ast::ParsedFile;
use crate::data_flow::test_support::label_snapshot;
use crate::js_write_census::matrix_tests::lang_of;
use crate::languages::Language;
use std::collections::BTreeMap;

const MATRIX: &str = include_str!("../../tests/fixtures/js_write_census/variant-matrix.tsv");

struct Row {
    fixture: String,
    edge: String,
    main: String,
    expected: String,
    source: String,
}

fn rows() -> Vec<Row> {
    MATRIX
        .lines()
        .skip(1)
        .filter_map(|line| {
            let c: Vec<&str> = line.split('\t').collect();
            assert_eq!(c.len(), 8, "matrix row: {line}");
            c[5].contains("S1b-1").then(|| Row {
                fixture: c[0].to_string(),
                edge: c[1].to_string(),
                main: c[2].to_string(),
                expected: c[4].to_string(),
                source: c[7].replace("\\n", "\n"),
            })
        })
        .collect()
}

fn ext_of(language: Language) -> &'static str {
    match language {
        Language::TypeScript => "ts",
        Language::Tsx => "tsx",
        _ => "js",
    }
}

/// Labels of every DataFlow row of `source` whose endpoints are `edge` (`path@line->path@line`).
pub(super) fn labels_of(language: Language, source: &str, edge: &str) -> Vec<String> {
    let name = format!("a.{}", ext_of(language));
    let file = ParsedFile::parse(&name, source, language).unwrap();
    assert_eq!(file.parse_error_count, 0, "{language:?}: {source}");
    let cpg = CodePropertyGraph::build(&BTreeMap::from([(name, file)]));
    let (from, to) = edge.split_once("->").unwrap();
    let split = |s: &str| {
        let (p, l) = s.rsplit_once('@').unwrap();
        (p.to_string(), l.parse::<usize>().unwrap())
    };
    let (from, to) = (split(from), split(to));
    label_snapshot(&cpg.dfg)
        .into_iter()
        .filter(|((f, t), _)| {
            (f.path.to_string(), f.line) == from && (t.path.to_string(), t.line) == to
        })
        .map(|(_, label)| match label {
            FlowConfidence::Exact => "exact".to_string(),
            FlowConfidence::NameOnly(FlowDoubt::Killed { .. }) => "nameonly/killed".to_string(),
            FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete) => {
                "nameonly/cfg_incomplete".to_string()
            }
            FlowConfidence::NameOnly(FlowDoubt::SameLine) => "nameonly/sameline".to_string(),
            other => format!("{other:?}"),
        })
        .collect()
}

#[test]
fn matrix_rows_move_to_the_expected_label() {
    let all = rows();
    assert_eq!(all.len(), 54, "S1b-1 matrix rows");
    let mut failures = Vec::new();
    for r in &all {
        let language = lang_of(&r.fixture);
        let want = &r.expected;
        if want == "-" {
            continue; // no edge on main (w11)
        }
        let got = labels_of(language, &r.source, &r.edge);
        if got.is_empty() || got.iter().any(|g| g != want) {
            failures.push(format!(
                "{} {} (main {}): want {want}, got {got:?}",
                r.fixture, r.edge, r.main
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

/// The edge set never moves: each row's edge exists with some label on head.
#[test]
fn matrix_rows_keep_their_edges() {
    for r in rows() {
        if r.expected == "-" {
            continue;
        }
        let got = labels_of(lang_of(&r.fixture), &r.source, &r.edge);
        assert!(
            !got.is_empty(),
            "{} {}: edge disappeared",
            r.fixture,
            r.edge
        );
    }
}
