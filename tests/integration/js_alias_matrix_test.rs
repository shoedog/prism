//! Table-driven pin of prism's JS/TS call resolution on the alias variant
//! matrix. See `tests/fixtures/js_alias_matrix/README.md`.
//!
//! Each fixture is a tiny repo with one `// SITE` call. The test materializes it,
//! builds a navigation session in-process (the library path behind
//! `prism nav call-stats --dump-sites`), and compares the whole derived row with
//! `expected.jsonl`. All differing rows are reported at once.
//! `PRISM_BLESS_JS_ALIAS_MATRIX=1` rewrites `expected.jsonl`, refusing unaccepted
//! regressions.

use super::js_alias_matrix_common::*;
use rayon::prelude::*;
use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

const BLESS_ENV: &str = "PRISM_BLESS_JS_ALIAS_MATRIX";

fn matrix_dir() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/js_alias_matrix")
}

fn read_lines(name: &str) -> Vec<String> {
    let path = matrix_dir().join(name);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("read {}: {e}", path.display()))
        .lines()
        .filter(|l| !l.trim().is_empty())
        .map(str::to_string)
        .collect()
}

fn parse_lines<T: serde::de::DeserializeOwned>(name: &str) -> Vec<T> {
    read_lines(name)
        .iter()
        .enumerate()
        .map(|(i, l)| {
            serde_json::from_str(l).unwrap_or_else(|e| panic!("{name} line {}: {e}", i + 1))
        })
        .collect()
}

fn fixtures() -> Vec<Fixture> {
    parse_lines("fixtures.jsonl")
}

fn expected() -> Vec<Row> {
    parse_lines("expected.jsonl")
}

fn accepted() -> BTreeSet<String> {
    #[derive(serde::Deserialize)]
    struct Accepted {
        id: String,
        #[allow(dead_code)]
        reason: String,
    }
    parse_lines::<Accepted>("accepted_changes.jsonl")
        .into_iter()
        .map(|a| a.id)
        .collect()
}

/// The `call_site` records `call-stats --dump-sites` emits for the fixture's marked line.
fn site_records(f: &Fixture) -> Vec<serde_json::Value> {
    let tmp = tempfile::tempdir().expect("tempdir");
    let repo = tmp.path().join("repo");
    for (rel, src) in &f.files {
        let p = repo.join(rel);
        std::fs::create_dir_all(p.parent().unwrap()).unwrap();
        std::fs::write(&p, src).unwrap();
    }
    let mut options = prism::api::NavOptions::default();
    options.no_cache = true;
    let session = prism::api::nav_session(&repo, &options)
        .unwrap_or_else(|e| panic!("{}: nav_session: {e:#}", f.id));
    let line = f.site_line.expect("site_line");
    prism::navigation::queries::call_site_dump_iter(session.index.call_graph())
        .filter(|r| {
            r["record_kind"] == "call_site"
                && r["source_span"]["file"] == f.site_file.as_str()
                && r["source_span"]["line"] == line
        })
        .collect()
}

fn actual_rows(fixtures: &[Fixture]) -> Vec<Row> {
    fixtures
        .par_iter()
        .map(|f| row_for(f, observe(&site_records(f))))
        .collect()
}

fn report(changes: &[Change], fixtures: &[Fixture]) -> String {
    let step_of: BTreeMap<&str, &str> = fixtures
        .iter()
        .map(|f| (f.id.as_str(), f.step.as_str()))
        .collect();
    let mut out = format!("{} of {} rows differ\n", changes.len(), fixtures.len());
    for c in changes {
        let was = c
            .expected
            .as_ref()
            .map_or("(no pinned row)".to_string(), |r| {
                serde_json::to_string(r).unwrap()
            });
        out.push_str(&format!(
            "- {}\n    expected: {was}\n    actual:   {}\n",
            c.id,
            serde_json::to_string(&c.actual).unwrap()
        ));
    }
    out.push_str("by (step, expected verdict -> actual verdict):\n");
    out.push_str(&summary(changes, &step_of));
    out
}

#[test]
fn matrix_matches_pinned_expectations() {
    let fixtures = fixtures();
    let started = std::time::Instant::now();
    let actual = actual_rows(&fixtures);
    eprintln!(
        "js_alias_matrix: {} fixtures in {:.1}s",
        fixtures.len(),
        started.elapsed().as_secs_f64()
    );
    let expected = expected();
    let accepted = accepted();

    let unknown = unknown_accepted(&accepted, &fixtures);
    assert!(
        unknown.is_empty(),
        "accepted_changes.jsonl lists ids that are not fixtures: {unknown:?}"
    );

    let changes = changes(&expected, &actual);
    let orphans = orphan_expected(&expected, &actual);

    if std::env::var(BLESS_ENV).as_deref() == Ok("1") {
        let refused = unaccepted_regressions(&changes, &accepted);
        assert!(
            refused.is_empty(),
            "refusing to bless: {} regression(s) not listed in accepted_changes.jsonl \
             (a row moving into WRONG/OVERCLAIM or out of CORRECT): {refused:?}\n{}",
            refused.len(),
            report(&changes, &fixtures)
        );
        let path = matrix_dir().join("expected.jsonl");
        std::fs::write(&path, render(&actual)).expect("write expected.jsonl");
        eprintln!(
            "js_alias_matrix: blessed {} changed row(s), dropped {} orphan row(s)",
            changes.len(),
            orphans.len()
        );
        return;
    }

    assert!(
        changes.is_empty() && orphans.is_empty(),
        "{}\npinned rows with no fixture: {orphans:?}\n\
         If the change is intended, rerun with {BLESS_ENV}=1 and review the expected.jsonl diff.",
        report(&changes, &fixtures)
    );
}

#[test]
fn fixture_ids_are_unique() {
    let fixtures = fixtures();
    let mut seen = BTreeSet::new();
    let dups: Vec<_> = fixtures
        .iter()
        .filter(|f| !seen.insert(f.id.as_str()))
        .map(|f| f.id.as_str())
        .collect();
    assert!(dups.is_empty(), "duplicate fixture ids: {dups:?}");
}

#[test]
fn every_site_file_is_present_and_its_site_line_is_marked() {
    for f in fixtures() {
        let src = f
            .files
            .get(&f.site_file)
            .unwrap_or_else(|| panic!("{}: site_file {} not in files", f.id, f.site_file));
        let line = f
            .site_line
            .unwrap_or_else(|| panic!("{}: no site_line", f.id)) as usize;
        let text = src
            .lines()
            .nth(line - 1)
            .unwrap_or_else(|| panic!("{}: site_line {line} past end of {}", f.id, f.site_file));
        assert!(
            text.contains("// SITE"),
            "{}: line {line} of {} lacks `// SITE`: {text:?}",
            f.id,
            f.site_file
        );
    }
}

#[test]
fn every_fixture_has_an_expected_row_and_vice_versa() {
    let fixtures: Vec<_> = fixtures().into_iter().map(|f| f.id).collect();
    let expected: Vec<_> = expected().into_iter().map(|r| r.id).collect();
    assert_eq!(
        fixtures, expected,
        "fixtures.jsonl and expected.jsonl must list the same ids in the same order"
    );
}

#[test]
fn accepted_changes_name_real_fixtures() {
    let unknown = unknown_accepted(&accepted(), &fixtures());
    assert!(
        unknown.is_empty(),
        "accepted_changes.jsonl lists ids that are not fixtures: {unknown:?}"
    );
}

#[test]
fn pinned_file_is_in_the_format_bless_writes() {
    let on_disk = std::fs::read_to_string(matrix_dir().join("expected.jsonl")).unwrap();
    assert_eq!(
        render(&expected()),
        on_disk,
        "expected.jsonl is not in canonical form (compact JSON, fixed key order, trailing newline)"
    );
}
