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
use super::js_alias_matrix_guard::{bless_problems, ledger_problems, Accepted};
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

fn accepted() -> Vec<Accepted> {
    parse_lines("accepted_changes.jsonl")
}

fn ids(fixtures: &[Fixture]) -> Vec<&str> {
    fixtures.iter().map(|f| f.id.as_str()).collect()
}

const EXPECTED_AT_HEAD: &str = "HEAD:./tests/fixtures/js_alias_matrix/expected.jsonl";

/// The committed `expected.jsonl`: the old state a bless compares against. Only `id` and
/// `verdict` are read from it, so a change that adds or removes a row field can still bless
/// against a baseline written in the previous shape.
fn baseline() -> Result<Vec<Row>, String> {
    #[derive(serde::Deserialize)]
    struct Pinned {
        id: String,
        verdict: String,
    }
    let out = std::process::Command::new("git")
        .args(["show", EXPECTED_AT_HEAD])
        .current_dir(env!("CARGO_MANIFEST_DIR"))
        .output()
        .map_err(|e| format!("cannot run git: {e}"))?;
    if !out.status.success() {
        return Err(format!(
            "`git show {EXPECTED_AT_HEAD}` failed: {}",
            String::from_utf8_lossy(&out.stderr).trim()
        ));
    }
    String::from_utf8(out.stdout)
        .map_err(|e| e.to_string())?
        .lines()
        .filter(|l| !l.trim().is_empty())
        .enumerate()
        .map(|(i, l)| {
            serde_json::from_str::<Pinned>(l)
                .map(|p| Row {
                    id: p.id,
                    verdict: p.verdict,
                    record: true,
                    exact: Vec::new(),
                    nameonly: Vec::new(),
                    drop: None,
                    origins: Vec::new(),
                })
                .map_err(|e| format!("baseline line {}: {e}", i + 1))
        })
        .collect()
}

/// Write via a temp file in the same directory, then rename over the target.
fn write_atomic(path: &Path, text: &str) {
    let tmp = path.with_extension(format!("jsonl.tmp{}", std::process::id()));
    std::fs::write(&tmp, text).expect("write temp file");
    std::fs::rename(&tmp, path).expect("rename over expected.jsonl");
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

/// Fixtures run on prism's large-stack pool: from the global rayon pool (2 MiB
/// workers) each `nav_session` crosses into the build pool, and the waiting
/// worker steals another fixture onto the same stack, so depth grows with the
/// fixture count and overflows on small thread counts.
fn actual_rows(fixtures: &[Fixture]) -> Vec<Row> {
    prism::build_pool::install(|| {
        fixtures
            .par_iter()
            .map(|f| row_for(f, observe(&site_records(f))))
            .collect()
    })
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

fn bless_requested() -> bool {
    std::env::var(BLESS_ENV).as_deref() == Ok("1")
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
    let pinned = expected();
    let accepted = accepted();
    let ids = ids(&fixtures);

    if bless_requested() {
        let baseline = baseline().unwrap_or_else(|e| {
            panic!("refusing to bless: the committed baseline is unavailable ({e}); bless needs git and expected.jsonl committed at HEAD")
        });
        let problems = bless_problems(&ids, &baseline, &pinned, &actual, &accepted);
        assert!(
            problems.is_empty(),
            "refusing to bless; nothing was written:\n  {}\n{}",
            problems.join("\n  "),
            report(&changes(&pinned, &actual), &fixtures)
        );
        write_atomic(&matrix_dir().join("expected.jsonl"), &render(&actual));
        eprintln!(
            "js_alias_matrix: blessed ({} row(s) differed from the working tree)",
            changes(&pinned, &actual).len()
        );
        return;
    }

    let changes = changes(&pinned, &actual);
    let orphans = orphan_expected(&pinned, &actual);
    assert!(
        changes.is_empty() && orphans.is_empty(),
        "{}\npinned rows with no fixture: {orphans:?}\n\
         If the change is intended, rerun with {BLESS_ENV}=1 and review the expected.jsonl diff.",
        report(&changes, &fixtures)
    );
}

#[test]
fn accepted_changes_ledger_has_no_stale_entries() {
    if bless_requested() {
        return; // a bless checks the ledger against the rows it is about to write
    }
    let problems = ledger_problems(&accepted(), &ids(&fixtures()), &expected());
    assert!(problems.is_empty(), "{}", problems.join("\n"));
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

/// `line` (1-indexed) of `file` in the fixture, if both exist.
fn source_line<'a>(f: &'a Fixture, file: &str, line: u64) -> Option<&'a str> {
    let n = usize::try_from(line).ok()?.checked_sub(1)?;
    f.files.get(file)?.lines().nth(n)
}

#[test]
fn every_truth_points_at_real_source() {
    for f in fixtures() {
        match &f.truth {
            Truth::One { targets } | Truth::Set { targets } | Truth::May { targets } => {
                for t in targets {
                    assert!(
                        f.files.contains_key(&t.file),
                        "{}: truth target file {} is not in files",
                        f.id,
                        t.file
                    );
                    if let Some(line) = t.line {
                        let text = source_line(&f, &t.file, line).unwrap_or_else(|| {
                            panic!("{}: target line {line} not in {}", f.id, t.file)
                        });
                        assert!(
                            text.contains(&t.name),
                            "{}: {}:{line} does not contain `{}`: {text:?}",
                            f.id,
                            t.file,
                            t.name
                        );
                    }
                }
            }
            Truth::None => {}
            Truth::Anon { file, line } => {
                assert!(
                    f.files.contains_key(file),
                    "{}: anon file {file} missing",
                    f.id
                );
                let line = line.unwrap_or_else(|| panic!("{}: anon truth has no line", f.id));
                let text = source_line(&f, file, line)
                    .unwrap_or_else(|| panic!("{}: anon line {line} not in {file}", f.id));
                assert!(
                    text.contains("// ANON"),
                    "{}: anon line {file}:{line} lacks `// ANON`: {text:?}",
                    f.id
                );
            }
        }
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
fn pinned_file_is_in_the_format_bless_writes() {
    let on_disk = std::fs::read_to_string(matrix_dir().join("expected.jsonl")).unwrap();
    assert_eq!(
        render(&expected()),
        on_disk,
        "expected.jsonl is not in canonical form (compact JSON, fixed key order, trailing newline)"
    );
}
