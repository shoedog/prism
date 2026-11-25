//! The input of every test that sweeps the repository fixtures, and the guard on its size.
//!
//! Several differential and identity tests prove an optimisation or a refactor against "all the
//! fixtures": they build, parse or compare every Go, Rust, Python, C, C++, JS or TS file they
//! find. Their cost is linear (some quadratic) in the number of files they visit, so a walk of
//! the whole of `tests/fixtures` makes each of them slower every time an unrelated lane adds a
//! fixture matrix there: on 2026-11-25, 1,447 matrix files doubled the file count and pushed
//! `repository_fixtures_complete_index_and_serialization` to the 300 s limit (#856).
//!
//! Those sweeps are tied to a fixed corpus, the fixture directories that existed before the
//! lane matrices (commit 9d1a7d06f), so they walk the explicit [`LEGACY_FIXTURE_DIRS`] and never
//! the root. A directory added under `tests/fixtures` later is not a sweep input and costs them
//! nothing. A matrix has its own runner, which reads its own directory.
//!
//! This one file is compiled into the `prism` library's tests (`src/lib.rs`, `#[path]`) and into
//! the umbrella targets through `tests/common/mod.rs`, so there is one list. It uses only `std`.
#![allow(dead_code)]

use std::path::{Path, PathBuf};

/// The top-level directories of `tests/fixtures` the sweeps read. Do not add a new matrix here:
/// keep it out of this list, or the sweeps pay for it (and their size guards fail). Adding a
/// directory is a deliberate change of the corpus, to be argued in the PR, together with the
/// bound of every sweep that reads it.
pub const LEGACY_FIXTURE_DIRS: &[&str] = &[
    "alias_twin_c",
    "alias_twin_cpp",
    "alias_twin_go",
    "alias_twin_java",
    "alias_twin_js",
    "alias_twin_python",
    "alias_twin_receipt_diff",
    "alias_twin_rust",
    "bash",
    "binding_memo_rust",
    "c",
    "c_jumps",
    "dfg_ident_index_rust",
    "dfg_prune",
    "diagram_snapshot",
    "go_blank",
    "go_branches",
    "go_field_write",
    "go_fold_r1",
    "go_fold_r2",
    "go_join",
    "go_join_classes",
    "go_join_controller",
    "go_join_reverse",
    "go_jumps",
    "go_jumps_round2",
    "go_loop_cycles",
    "go_new_expr",
    "hapi-4552-source",
    "item2-selected-evidence",
    "js_alias_matrix",
    "js_cjs_anonymous",
    "js_cjs_named_fn_expr",
    "js_loops",
    "js_mixed_esm_cjs",
    "js_write_census",
    "mcp",
    "nav_callgraph",
    "nav_compat",
    "nav_scoped_rust",
    "p8-param-slots",
    "python",
    "review_no_diagrams",
    "rust-self-allowlist",
    "rust-self-calls",
    "rust-self-closed",
    "sanitizer-suite-go",
    "sanitizer-suite-js-ts",
    "sanitizer-suite-python",
    "targets",
    "targets_hint",
    "terraform",
];

/// A directory entry no sweep reads: private corpora and benchmark checkouts a developer may
/// keep next to the tracked fixtures.
fn is_private(name: &str) -> bool {
    name == "private-corpus"
        || name.starts_with("f-")
        || name.starts_with("F-private")
        || name.to_ascii_lowercase().contains("secbench")
}

fn collect(dir: &Path, keep: &dyn Fn(&Path) -> bool, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("fixture directory {}: {e}", dir.display()))
        .map(|e| e.unwrap())
        .collect();
    entries.sort_by_key(|e| e.file_name());
    for entry in entries {
        if is_private(&entry.file_name().to_string_lossy()) {
            continue;
        }
        let path = entry.path();
        let kind = entry.file_type().unwrap();
        assert!(!kind.is_symlink(), "fixture symlink: {}", path.display());
        if kind.is_dir() {
            collect(&path, keep, out);
        } else if keep(&path) {
            out.push(path);
        }
    }
}

/// The files of the legacy corpus under `fixtures` (the `tests/fixtures` directory, spelled as the
/// caller spells it, relative or absolute) for which `keep` holds, in path order.
pub fn legacy_fixture_files(fixtures: &Path, keep: impl Fn(&Path) -> bool) -> Vec<PathBuf> {
    files_in_dirs(fixtures, LEGACY_FIXTURE_DIRS, keep)
}

fn files_in_dirs(fixtures: &Path, dirs: &[&str], keep: impl Fn(&Path) -> bool) -> Vec<PathBuf> {
    let mut out = Vec::new();
    for dir in dirs {
        let root = fixtures.join(dir);
        assert!(
            root.is_dir(),
            "{} is in LEGACY_FIXTURE_DIRS (tests/common/fixture_corpus.rs) but is not a directory: \
             a renamed or removed corpus directory must be updated there",
            root.display()
        );
        collect(&root, &keep, &mut out);
    }
    out.sort();
    out
}

/// Every file under `root` for which `keep` holds (a tree no lane adds matrices to, such as
/// `eval/fixtures`), in path order.
pub fn tree_files(root: &Path, keep: impl Fn(&Path) -> bool) -> Vec<PathBuf> {
    let mut out = Vec::new();
    collect(root, &keep, &mut out);
    out.sort();
    out
}

/// The size guard of a sweep: it fails, naming the sweep, when `visited` exceeds `max`.
pub fn assert_visited_within(sweep: &str, visited: usize, max: usize) {
    assert!(
        visited <= max,
        "fixture sweep `{sweep}` visited {visited} files, over its bound of {max}. Its cost grows \
         with every file it reads. If a new fixture matrix reached it, keep the matrix out of \
         its input: put it in its own top-level tests/fixtures directory, read it only from the \
         matrix's own runner and leave that directory out of LEGACY_FIXTURE_DIRS \
         (tests/common/fixture_corpus.rs). If the sweep should now cover \
         more, raise its bound deliberately in the same change and say why."
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(root: &Path, relative: &str) {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, "x = 1\n").unwrap();
    }

    #[test]
    fn a_directory_outside_the_list_is_not_walked() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "listed/a.py");
        write(dir.path(), "listed/deep/b.py");
        write(dir.path(), "listed/skip.txt");
        write(dir.path(), "new_matrix/c.py");
        write(dir.path(), "new_matrix/deep/d.py");
        write(dir.path(), "root_level.py");
        let found = files_in_dirs(dir.path(), &["listed"], |p| {
            p.extension().is_some_and(|e| e == "py")
        });
        let names: Vec<_> = found
            .iter()
            .map(|p| {
                p.strip_prefix(dir.path())
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_owned()
            })
            .collect();
        assert_eq!(names, ["listed/a.py", "listed/deep/b.py"]);
    }

    #[test]
    fn private_corpora_are_skipped_inside_a_listed_directory() {
        let dir = tempfile::tempdir().unwrap();
        write(dir.path(), "listed/a.py");
        write(dir.path(), "listed/private-corpus/b.py");
        write(dir.path(), "listed/f-secret/c.py");
        write(dir.path(), "listed/F-private-x/d.py");
        write(dir.path(), "listed/SecBench/e.py");
        let found = files_in_dirs(dir.path(), &["listed"], |_| true);
        assert_eq!(found.len(), 1, "{found:?}");
    }

    #[test]
    #[should_panic(expected = "is in LEGACY_FIXTURE_DIRS")]
    fn a_listed_directory_that_is_gone_fails_loudly() {
        let dir = tempfile::tempdir().unwrap();
        files_in_dirs(dir.path(), &["renamed_away"], |_| true);
    }

    #[test]
    fn every_listed_directory_exists_and_the_list_is_sorted_and_unique() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
        for dir in LEGACY_FIXTURE_DIRS {
            assert!(root.join(dir).is_dir(), "{dir}");
        }
        assert!(LEGACY_FIXTURE_DIRS.windows(2).all(|w| w[0] < w[1]));
    }

    #[test]
    fn the_guard_passes_at_the_bound_and_names_the_sweep_above_it() {
        assert_visited_within("some_sweep", 10, 10);
        let caught = std::panic::catch_unwind(|| assert_visited_within("some_sweep", 11, 10))
            .expect_err("over the bound must fail");
        let message = caught.downcast_ref::<String>().unwrap();
        assert!(message.contains("`some_sweep`"), "{message}");
        assert!(message.contains("visited 11 files"), "{message}");
        assert!(message.contains("bound of 10"), "{message}");
        assert!(message.contains("keep the matrix out"), "{message}");
    }
}
