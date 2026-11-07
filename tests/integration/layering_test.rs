//! Layering rule (#702): core modules must not depend on the review layer.
//!
//! No non-test source file outside the review layer may mention
//! `crate::algorithms` or `algorithms::`. The review layer is
//! `src/algorithms/**`, `src/output/**`, `src/targets/**`, `src/api/**`,
//! `src/cli.rs`, `src/main.rs`, `src/lib.rs` and `src/bin/**`.
//!
//! Exempt test code: any `*_test.rs` / `*_tests.rs` file, any path containing
//! `/tests/`, and, heuristically, everything after the first line that
//! matches `^#[cfg(test)]` (a trailing `#[cfg(test)] mod tests` block).

use std::fs;
use std::path::{Path, PathBuf};

const REVIEW_LAYER: &[&str] = &[
    "src/algorithms/",
    "src/output/",
    "src/targets/",
    "src/api/",
    "src/bin/",
];
const REVIEW_LAYER_FILES: &[&str] = &["src/cli.rs", "src/main.rs", "src/lib.rs"];

fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) {
    let mut entries: Vec<_> = fs::read_dir(dir)
        .unwrap_or_else(|e| panic!("read_dir {}: {e}", dir.display()))
        .map(|e| e.unwrap().path())
        .collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            collect_rs(&p, out);
        } else if p.extension().is_some_and(|e| e == "rs") {
            out.push(p);
        }
    }
}

fn is_review_layer(rel: &str) -> bool {
    REVIEW_LAYER.iter().any(|p| rel.starts_with(p)) || REVIEW_LAYER_FILES.contains(&rel)
}

fn is_test_file(rel: &str) -> bool {
    rel.ends_with("_test.rs") || rel.ends_with("_tests.rs") || rel.contains("/tests/")
}

/// Offending `file:line` entries for one file's text. Text from the first
/// `#[cfg(test)]` line onward is ignored.
fn offending_lines(rel: &str, text: &str) -> Vec<String> {
    let mut hits = Vec::new();
    for (i, line) in text.lines().enumerate() {
        if line.starts_with("#[cfg(test)]") {
            break;
        }
        if line.contains("crate::algorithms") || line.contains("algorithms::") {
            hits.push(format!("{rel}:{}: {}", i + 1, line.trim()));
        }
    }
    hits
}

#[test]
fn core_modules_do_not_import_algorithms() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    collect_rs(&root.join("src"), &mut files);
    let mut offenders = Vec::new();
    for path in files {
        let rel = path
            .strip_prefix(root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/");
        if is_review_layer(&rel) || is_test_file(&rel) {
            continue;
        }
        let text = fs::read_to_string(&path).unwrap();
        offenders.extend(offending_lines(&rel, &text));
    }
    assert!(
        offenders.is_empty(),
        "core modules reference the review layer (crate::algorithms):\n{}",
        offenders.join("\n")
    );
}

#[test]
fn heuristic_ignores_trailing_cfg_test_block() {
    let text = "use a::b;\n#[cfg(test)]\nmod tests {\n    use crate::algorithms::x;\n}\n";
    assert!(offending_lines("src/x.rs", text).is_empty());
    let text = "use crate::algorithms::x;\n#[cfg(test)]\nmod tests {}\n";
    assert_eq!(offending_lines("src/x.rs", text).len(), 1);
}

#[test]
fn review_layer_and_test_paths_are_classified() {
    assert!(is_review_layer("src/algorithms/taint.rs"));
    assert!(is_review_layer("src/cli.rs"));
    assert!(!is_review_layer("src/frameworks/mod.rs"));
    assert!(is_test_file("src/foo/bar_tests.rs"));
    assert!(is_test_file("src/foo/tests/x.rs"));
    assert!(!is_test_file("src/foo/bar.rs"));
}
