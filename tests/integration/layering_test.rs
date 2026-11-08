//! Layering rule (#702): core modules must not depend on the review layer.
//!
//! No source file outside the review layer may mention `crate::algorithms` or
//! `algorithms::` on a non-comment line. The review layer is
//! `src/algorithms/**`, `src/output/**`, `src/targets/**`, `src/api/**`,
//! `src/cli.rs`, `src/main.rs` and `src/lib.rs`.
//! `crates/prism-mcp/src/**` is scanned too, with no review-layer allowlist.
//!
//! Exempt: files named `*_tests.rs`, `*_test.rs` or `tests.rs`, any path with a `/tests/`
//! component, and comment lines (`//`, `///`, `//!`). There is deliberately NO
//! `#[cfg(test)]` exemption, because no line-based heuristic can tell where a
//! gated item ends. Consequence: a core file's inline `#[cfg(test)]` module
//! may not import `crate::algorithms`; a test that needs review-layer items
//! lives in a `*_tests.rs`, `*_test.rs` or `tests.rs` sibling (which the crate split in
//! #701 will make the compiler enforce).

use std::fs;
use std::path::{Path, PathBuf};

const REVIEW_LAYER: &[&str] = &[
    "src/algorithms/",
    "src/output/",
    "src/targets/",
    "src/api/",
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
    rel.ends_with("_tests.rs")
        || rel.ends_with("_test.rs")
        || rel.rsplit('/').next() == Some("tests.rs")
        || rel.contains("/tests/")
}

/// Offending `file:line` entries for one file's text: every non-comment line
/// mentioning the review layer, gated code included.
fn offending_lines(rel: &str, text: &str) -> Vec<String> {
    text.lines()
        .enumerate()
        .filter(|(_, line)| !line.trim_start().starts_with("//"))
        .filter(|(_, line)| {
            line.contains("crate::algorithms")
                || line.contains("prism::algorithms")
                || line.contains("algorithms::")
        })
        .map(|(i, line)| format!("{rel}:{}: {}", i + 1, line.trim()))
        .collect()
}

#[test]
fn core_modules_do_not_import_algorithms() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut offenders = Vec::new();
    // `src` is the core crate; `crates/prism-mcp/src` is scanned with no review-layer
    // allowlist (the crate boundary does not stop it importing the `pub` algorithms
    // module, #731 S4).
    for scanned in ["src", "crates/prism-mcp/src"] {
        let mut files = Vec::new();
        collect_rs(&root.join(scanned), &mut files);
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
    }
    assert!(
        offenders.is_empty(),
        "core modules reference the review layer (crate::algorithms):\n{}",
        offenders.join("\n")
    );
}

#[test]
fn review_layer_and_test_paths_are_classified() {
    assert!(is_review_layer("src/algorithms/taint.rs"));
    assert!(is_review_layer("src/cli.rs"));
    assert!(!is_review_layer("src/frameworks/mod.rs"));
    assert!(is_test_file("src/foo/bar_tests.rs"));
    assert!(is_test_file(
        "src/call_graph/js_relative_candidates_test.rs"
    ));
    assert!(is_test_file("src/foo/tests/x.rs"));
    assert!(!is_test_file("src/foo/bar.rs"));
}

#[test]
fn comment_lines_are_not_scanned() {
    let text = "/// see `algorithms::check_parse_quality`\n// crate::algorithms::taint\n//! algorithms::x\nfn a() {}\n";
    assert!(offending_lines("src/x.rs", text).is_empty());
}

#[test]
fn tests_rs_basename_is_a_test_file() {
    assert!(is_test_file("src/cpg/tests.rs"));
    assert!(is_test_file("src/finding_confidence/tests.rs"));
    assert!(is_test_file("src/tests.rs"));
    assert!(!is_test_file("src/cpg/attests.rs"));
    assert!(!is_test_file("src/cpg/tests_helper.rs"));
}

#[test]
fn gated_test_module_import_is_reported() {
    let text = "fn a() {}\n#[cfg(test)]\nmod tests {\n    use crate::algorithms::x;\n}\n";
    let hits = offending_lines("src/x.rs", text);
    assert_eq!(hits.len(), 1, "{hits:?}");
    assert!(hits[0].starts_with("src/x.rs:4:"), "{hits:?}");
}

#[test]
fn the_crates_own_spelling_prism_algorithms_is_reported_even_when_aliased() {
    let text = "use prism::algorithms as review;\nfn _x() { let _ = review::taint::slice; }\n";
    let hits = offending_lines("crates/prism-mcp/src/x.rs", text);
    assert_eq!(hits.len(), 1, "{hits:?}");
    assert!(
        hits[0].starts_with("crates/prism-mcp/src/x.rs:1:"),
        "{hits:?}"
    );
}

#[test]
fn gated_one_liner_does_not_exempt_the_next_line() {
    let text = "#[cfg(test)]\nmod t;\nuse crate::algorithms::x;\n";
    let hits = offending_lines("src/x.rs", text);
    assert_eq!(hits.len(), 1, "{hits:?}");
    assert!(hits[0].starts_with("src/x.rs:3:"), "{hits:?}");
}

#[test]
fn tests_files_are_exempt_from_the_walk() {
    let text = "use crate::algorithms::x;\n";
    assert_eq!(offending_lines("src/foo/x_tests.rs", text).len(), 1);
    assert!(is_test_file("src/foo/x_tests.rs"));
    assert!(is_test_file("src/foo/tests.rs"));
}

#[test]
fn trailing_comment_on_code_line_is_still_reported() {
    let text = "// algorithms::x\nuse crate::algorithms::x; // why\n";
    let hits = offending_lines("src/x.rs", text);
    assert_eq!(hits.len(), 1, "{hits:?}");
    assert!(hits[0].starts_with("src/x.rs:2:"), "{hits:?}");
}
