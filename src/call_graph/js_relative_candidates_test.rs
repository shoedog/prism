//! I1: candidate list for JS/TS relative module specifiers
//! (`js_ts_relative_module_candidates`). The rule is additive: a specifier
//! that already yields a candidate keeps its exact list; only specifiers that
//! yield none today may gain candidates (`.js` -> `.ts`/`.tsx`, bare `.`/`..`,
//! redundant `//`, `/./`, `/../` segments).
use super::js_ts_relative_module_candidates;
use std::collections::BTreeSet;

fn cands(spec: &str, caller: &str, indexed: &[&str]) -> Option<Vec<String>> {
    let indexed: BTreeSet<String> = indexed.iter().map(|s| s.to_string()).collect();
    js_ts_relative_module_candidates(spec, caller, &indexed)
}

/// Candidate list with `None` (not a relative specifier / escapes the root)
/// and `Some(vec![])` (relative, nothing indexed) both read as empty.
fn list(spec: &str, caller: &str, indexed: &[&str]) -> Vec<String> {
    cands(spec, caller, indexed).unwrap_or_default()
}

fn v(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| s.to_string()).collect()
}

const APP: &str = "pkg/app.ts";

#[test]
fn js_suffix_finds_same_stem_ts() {
    assert_eq!(list("./x.js", APP, &["pkg/x.ts"]), v(&["pkg/x.ts"]));
}

#[test]
fn js_suffix_finds_same_stem_tsx() {
    assert_eq!(list("./x.js", APP, &["pkg/x.tsx"]), v(&["pkg/x.tsx"]));
}

#[test]
fn js_suffix_orders_ts_before_tsx() {
    assert_eq!(
        list("./x.js", APP, &["pkg/x.tsx", "pkg/x.ts"]),
        v(&["pkg/x.ts", "pkg/x.tsx"])
    );
}

#[test]
fn js_suffix_literal_js_file_wins_and_is_alone() {
    assert_eq!(
        list("./x.js", APP, &["pkg/x.js", "pkg/x.ts", "pkg/x.tsx"]),
        v(&["pkg/x.js"])
    );
}

#[test]
fn js_suffix_does_not_substitute_declaration_files() {
    assert!(list("./x.js", APP, &["pkg/x.d.ts"]).is_empty());
}

#[test]
fn js_suffix_on_index_file() {
    assert_eq!(
        list("./dir/index.js", APP, &["pkg/dir/index.ts"]),
        v(&["pkg/dir/index.ts"])
    );
}

#[test]
fn js_suffix_under_parent_traversal() {
    assert_eq!(list("../lib/x.js", APP, &["lib/x.ts"]), v(&["lib/x.ts"]));
}

#[test]
fn js_suffix_keeps_inner_dots_in_stem() {
    assert_eq!(
        list("./x.test.js", APP, &["pkg/x.test.ts"]),
        v(&["pkg/x.test.ts"])
    );
}

#[test]
fn js_suffix_never_picks_an_unrelated_directory() {
    assert!(list("./x.js", APP, &["other/x.ts", "x.ts"]).is_empty());
}

#[test]
fn js_suffix_existing_dotted_ts_candidate_is_kept_alone() {
    // `x.js.ts` is already a candidate today (`rel + ".ts"`); the additive
    // rule must not add `x.ts` next to it.
    assert_eq!(
        list("./x.js", APP, &["pkg/x.js.ts", "pkg/x.ts"]),
        v(&["pkg/x.js.ts"])
    );
}

// Not modeled in this slice: pin that nothing is substituted for these.
#[test]
fn js_suffix_on_a_bare_dotfile_name_has_no_stem_to_substitute() {
    assert!(list("./.js", APP, &["pkg/.ts"]).is_empty());
    assert!(list("./.js", "app.ts", &[".ts"]).is_empty());
}

#[test]
fn jsx_suffix_is_not_substituted() {
    assert!(list("./x.jsx", APP, &["pkg/x.tsx"]).is_empty());
}

#[test]
fn mjs_and_cjs_suffixes_are_not_substituted() {
    assert!(list("./x.mjs", APP, &["pkg/x.ts"]).is_empty());
    assert!(list("./x.cjs", APP, &["pkg/x.ts"]).is_empty());
}

#[test]
fn bare_dot_resolves_to_directory_index() {
    assert_eq!(list(".", APP, &["pkg/index.ts"]), v(&["pkg/index.ts"]));
    assert_eq!(list("./", APP, &["pkg/index.ts"]), v(&["pkg/index.ts"]));
}

#[test]
fn bare_dot_uses_existing_index_order() {
    assert_eq!(
        list(".", APP, &["pkg/index.ts", "pkg/index.js", "pkg/index.tsx"]),
        v(&["pkg/index.js", "pkg/index.ts", "pkg/index.tsx"])
    );
}

#[test]
fn bare_dot_ignores_a_same_stem_file_and_other_directories() {
    assert!(list(".", APP, &["pkg.ts", "index.ts", "pkg/x.ts"]).is_empty());
}

#[test]
fn bare_dot_dot_resolves_to_parent_index() {
    let caller = "pkg/sub/app.ts";
    assert_eq!(list("..", caller, &["pkg/index.ts"]), v(&["pkg/index.ts"]));
    assert_eq!(list("../", caller, &["pkg/index.ts"]), v(&["pkg/index.ts"]));
}

#[test]
fn bare_dot_at_repo_root_resolves_root_index() {
    assert_eq!(list(".", "app.ts", &["index.ts"]), v(&["index.ts"]));
}

#[test]
fn bare_dot_dot_never_escapes_the_repo_root() {
    assert!(list("..", "app.ts", &["index.ts", "pkg/index.ts"]).is_empty());
    assert!(list("../", "app.ts", &["index.ts"]).is_empty());
    assert!(list("../..", "pkg/app.ts", &["index.ts", "pkg/index.ts"]).is_empty());
}

#[test]
fn bare_dot_dot_from_one_level_deep_reaches_the_root_index() {
    assert_eq!(list("..", "pkg/app.ts", &["index.ts"]), v(&["index.ts"]));
}

#[test]
fn redundant_slash_and_dot_segments_are_normalized() {
    let indexed = ["pkg/a/b.ts"];
    assert_eq!(list("./a//b", APP, &indexed), v(&["pkg/a/b.ts"]));
    assert_eq!(list("./a/./b", APP, &indexed), v(&["pkg/a/b.ts"]));
}

#[test]
fn inner_parent_segment_is_normalized() {
    assert_eq!(list("./a/../b", APP, &["pkg/b.ts"]), v(&["pkg/b.ts"]));
    assert_eq!(list("./../b", APP, &["b.ts"]), v(&["b.ts"]));
}

#[test]
fn normalization_composes_with_js_suffix_and_index() {
    assert_eq!(list("./a/../x.js", APP, &["pkg/x.ts"]), v(&["pkg/x.ts"]));
    assert_eq!(
        list("./a//b", APP, &["pkg/a/b/index.ts"]),
        v(&["pkg/a/b/index.ts"])
    );
}

#[test]
fn trailing_slash_means_directory_only() {
    assert_eq!(
        list("./sub/", APP, &["pkg/sub/index.ts"]),
        v(&["pkg/sub/index.ts"])
    );
    assert!(list("./sub/", APP, &["pkg/sub.ts"]).is_empty());
}

#[test]
fn normalized_specifier_escaping_the_root_has_no_candidates() {
    assert!(list("./a/../../../b", APP, &["b.ts", "pkg/b.ts"]).is_empty());
    assert!(list("./../b", "app.ts", &["b.ts"]).is_empty());
}

#[test]
fn non_relative_specifiers_are_unchanged() {
    for spec in ["x", "@scope/x", "/abs/x", "...", ".x", "~/x", ""] {
        assert_eq!(
            cands(spec, APP, &["pkg/x.ts", "x.ts", "pkg/index.ts", "index.ts"]),
            None,
            "{spec:?}"
        );
    }
}

// Additivity pins: specifiers with a candidate today keep their exact list.
#[test]
fn existing_candidate_lists_are_unchanged() {
    let indexed = [
        "pkg/x.js",
        "pkg/x.jsx",
        "pkg/x.mjs",
        "pkg/x.cjs",
        "pkg/x.ts",
        "pkg/x.tsx",
        "pkg/x/index.js",
        "pkg/x/index.ts",
    ];
    assert_eq!(
        list("./x", APP, &indexed),
        v(&[
            "pkg/x.js",
            "pkg/x.jsx",
            "pkg/x.mjs",
            "pkg/x.cjs",
            "pkg/x.ts",
            "pkg/x.tsx",
            "pkg/x/index.js",
            "pkg/x/index.ts"
        ])
    );
    assert_eq!(list("./x.ts", APP, &indexed), v(&["pkg/x.ts"]));
    assert_eq!(list("../pkg/x", "other/app.ts", &indexed).len(), 8);
    assert_eq!(cands("./nothing", APP, &indexed), Some(vec![]));
}
