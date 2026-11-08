//! Layering rule (#702, enforced by the crate split of #701 S5): the core crate must not depend on
//! the review layer.
//!
//! The review layer is the `prism-review` crate (`crates/prism-review`), which depends on `prism`;
//! Cargo rejects `crate::algorithms` in core (the module no longer exists there), so the rule that
//! remains is that no file under `src/` names `prism_review`: the root's dev-dependency on it (test
//! placement T3) must not leak into core source, tests included, with no file-name exemption. Comment
//! lines (`//`, `///`, `//!`) are skipped, so docs may name the review facade.
//!
//! `crates/prism-mcp/src/**` is scanned for the review layer too (`crate::algorithms`,
//! `algorithms::`, `prism::algorithms`, `prism_review`): the crate boundary alone does not stop it
//! depending on `prism-review` if someone added the edge. Its files named `*_tests.rs`, `*_test.rs` or
//! `tests.rs`, and any path with a `/tests/` component, are exempt there.
//! There is deliberately NO `#[cfg(test)]` exemption, because no line-based heuristic can tell where a
//! gated item ends.

use std::fs;
use std::path::{Path, PathBuf};

/// What the review layer is called from outside: the crate and the modules it owns.
const MCP_REVIEW_PATTERNS: &[&str] = &[
    "crate::algorithms",
    "prism::algorithms",
    "algorithms::",
    "prism_review",
];
/// Core `src/` must not name the review crate at all.
const CORE_REVIEW_PATTERNS: &[&str] = &["prism_review"];

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

fn is_test_file(rel: &str) -> bool {
    rel.ends_with("_tests.rs")
        || rel.ends_with("_test.rs")
        || rel.rsplit('/').next() == Some("tests.rs")
        || rel.contains("/tests/")
}

/// Offending `file:line` entries for one file's text: every non-comment line
/// containing one of `patterns`, gated code included.
fn lines_matching(rel: &str, text: &str, patterns: &[&str]) -> Vec<String> {
    text.lines()
        .enumerate()
        .filter(|(_, line)| !line.trim_start().starts_with("//"))
        .filter(|(_, line)| patterns.iter().any(|p| line.contains(p)))
        .map(|(i, line)| format!("{rel}:{}: {}", i + 1, line.trim()))
        .collect()
}

/// A line of a `prism-mcp` source file that mentions the review layer.
fn offending_lines(rel: &str, text: &str) -> Vec<String> {
    lines_matching(rel, text, MCP_REVIEW_PATTERNS)
}

/// A line of a core `src/` file that names the review crate.
fn core_offending_lines(rel: &str, text: &str) -> Vec<String> {
    lines_matching(rel, text, CORE_REVIEW_PATTERNS)
}

#[test]
fn core_modules_do_not_import_algorithms() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let rel_of = |path: &Path| {
        path.strip_prefix(root)
            .unwrap()
            .to_string_lossy()
            .replace('\\', "/")
    };
    let mut offenders = Vec::new();
    // Core `src/`: no mention of the review crate in any file, tests included.
    let mut core = Vec::new();
    collect_rs(&root.join("src"), &mut core);
    assert!(!core.is_empty(), "no files found under src");
    for path in core {
        let text = fs::read_to_string(&path).unwrap();
        offenders.extend(core_offending_lines(&rel_of(&path), &text));
    }
    // `crates/prism-mcp/src`: no import of the review layer (test files exempt).
    let mut mcp = Vec::new();
    collect_rs(&root.join("crates/prism-mcp/src"), &mut mcp);
    assert!(!mcp.is_empty(), "no files found under crates/prism-mcp/src");
    for path in mcp {
        let rel = rel_of(&path);
        if is_test_file(&rel) {
            continue;
        }
        let text = fs::read_to_string(&path).unwrap();
        offenders.extend(offending_lines(&rel, &text));
    }
    assert!(
        offenders.is_empty(),
        "core or prism-mcp sources reference the review layer (prism_review, algorithms):\n{}",
        offenders.join("\n")
    );
}

#[test]
fn review_layer_and_test_paths_are_classified() {
    assert!(is_test_file("src/foo/bar_tests.rs"));
    assert!(is_test_file(
        "src/call_graph/js_relative_candidates_test.rs"
    ));
    assert!(is_test_file("src/foo/tests/x.rs"));
    assert!(!is_test_file("src/foo/bar.rs"));
}

#[test]
fn core_test_files_get_no_exemption_from_the_review_crate_rule() {
    let text = "#[cfg(test)]\nmod t {\n    use prism_review::api::review;\n}\n";
    for rel in [
        "src/x.rs",
        "src/foo/x_tests.rs",
        "src/foo/tests.rs",
        "src/foo/tests/a.rs",
    ] {
        let hits = core_offending_lines(rel, text);
        assert_eq!(hits.len(), 1, "{rel}: {hits:?}");
        assert!(hits[0].starts_with(&format!("{rel}:3:")), "{hits:?}");
    }
}

#[test]
fn core_rule_skips_comments_and_ignores_the_algorithms_spelling() {
    let text = "//! `prism_review::api` is the review facade\n/// prism_review\n// prism_review\nuse crate::algorithms::x;\nfn a() {}\n";
    assert!(core_offending_lines("src/api/mod.rs", text).is_empty());
    let code = "pub use prism_review::api::review; // why\n";
    assert_eq!(core_offending_lines("src/api/mod.rs", code).len(), 1);
}

#[test]
fn the_review_crate_spelling_is_reported_in_the_mcp_crate() {
    let text = "use prism_review::api::run_review;\n";
    let hits = offending_lines("crates/prism-mcp/src/x.rs", text);
    assert_eq!(hits.len(), 1, "{hits:?}");
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
    assert_eq!(
        offending_lines("crates/prism-mcp/src/x_tests.rs", text).len(),
        1
    );
    assert!(is_test_file("crates/prism-mcp/src/x_tests.rs"));
    assert!(is_test_file("crates/prism-mcp/src/tests.rs"));
}

#[test]
fn trailing_comment_on_code_line_is_still_reported() {
    let text = "// algorithms::x\nuse crate::algorithms::x; // why\n";
    let hits = offending_lines("src/x.rs", text);
    assert_eq!(hits.len(), 1, "{hits:?}");
    assert!(hits[0].starts_with("src/x.rs:2:"), "{hits:?}");
}
