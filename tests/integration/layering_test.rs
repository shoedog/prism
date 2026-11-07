//! Layering rule (#702): core modules must not depend on the review layer.
//!
//! No non-test source file outside the review layer may mention
//! `crate::algorithms` or `algorithms::`. The review layer is
//! `src/algorithms/**`, `src/output/**`, `src/targets/**`, `src/api/**`,
//! `src/cli.rs`, `src/main.rs`, `src/lib.rs` and `src/bin/**`.
//!
//! Exempt test code: any `*_test.rs` / `*_tests.rs` / `tests.rs` file, any path
//! containing `/tests/`, comment lines (`//`, `///`, `//!`), and each
//! recognized `#[cfg(test)]`-gated ITEM inside a file. The rule fails CLOSED:
//! when the gated construct is not recognized, nothing is exempted.
//!
//! After a `#[cfg(test)]` line (any indentation), skip trivia (blank lines,
//! `//` lines) and further `#[..]` attribute lines to reach the item head. Then:
//! - head ends with `;` (`mod x;`, `use ..;`): that line only is exempt;
//! - head is a one-line braced item (contains `{`, ends with `}` or `};`):
//!   that line only;
//! - head ends with `{` and starts (after optional `pub`, `pub(crate)`,
//!   `pub(super)`, `pub(in ..)`) with `mod`, `fn`, `impl`, `struct`, `enum`,
//!   `union`, `trait`, `const`, `static`, `type`, `macro_rules!`,
//!   `thread_local!`, an `<ident>! {` macro call, or `unsafe`/`async`/`extern`
//!   modifiers before `fn`/`impl`/`trait`: exempt through the first later line
//!   that is exactly `}` or `};` at the attribute's indentation. With no such
//!   line before EOF nothing is exempt;
//! - anything else (enum variant, struct field, match arm, expression
//!   statement, unknown shape): nothing is exempt and scanning resumes right
//!   after the attribute line.
//!
//! Scanning resumes after an exempt item, so production code after a test
//! module is checked.

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
    rel.ends_with("_test.rs")
        || rel.ends_with("_tests.rs")
        || rel.rsplit('/').next() == Some("tests.rs")
        || rel.contains("/tests/")
}

fn indent_of(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// Strip a leading `pub`, `pub(crate)`, `pub(super)` or `pub(in ..)`.
fn strip_visibility(head: &str) -> &str {
    if let Some(rest) = head.strip_prefix("pub(") {
        return rest.split_once(')').map_or(head, |(_, r)| r.trim_start());
    }
    head.strip_prefix("pub ").map_or(head, str::trim_start)
}

/// Is `head` (ending with `{`) the head of a block item we recognize?
fn is_block_item_head(head: &str) -> bool {
    let head = strip_visibility(head.trim());
    let words: Vec<&str> = head
        .split(|c: char| !(c.is_alphanumeric() || c == '_' || c == '!'))
        .filter(|w| !w.is_empty())
        .collect();
    let Some(first) = words.first() else {
        return false;
    };
    match *first {
        "mod" | "fn" | "impl" | "struct" | "enum" | "union" | "trait" | "const" | "static"
        | "type" | "macro_rules!" | "thread_local!" => true,
        "unsafe" | "async" | "extern" => {
            words.iter().any(|w| matches!(*w, "fn" | "impl" | "trait"))
        }
        w => w.ends_with('!') && w.len() > 1 && head.contains("! {"),
    }
}

/// Index at which scanning resumes after the `#[cfg(test)]` at `attr`.
/// Fails closed: an unrecognized construct exempts nothing.
fn skip_cfg_test_item(lines: &[&str], attr: usize) -> usize {
    let fail_closed = attr + 1;
    let indent = indent_of(lines[attr]);
    let rest = lines[attr].trim_start()["#[cfg(test)]".len()..].trim();
    let mut item = attr;
    if rest.is_empty() {
        item += 1;
        while item < lines.len() {
            let t = lines[item].trim_start();
            if t.is_empty() || t.starts_with("//") || t.starts_with("#[") {
                item += 1;
            } else {
                break;
            }
        }
        if item >= lines.len() {
            return fail_closed;
        }
    }
    let head = if rest.is_empty() {
        lines[item].trim()
    } else {
        rest
    };
    if head.ends_with(';') || (head.contains('{') && (head.ends_with('}') || head.ends_with("};")))
    {
        return item + 1;
    }
    if !(head.ends_with('{') && is_block_item_head(head)) {
        return fail_closed;
    }
    let pad = " ".repeat(indent);
    let (close, close_semi) = (format!("{pad}}}"), format!("{pad}}};"));
    for (k, line) in lines.iter().enumerate().skip(item + 1) {
        if *line == close || *line == close_semi {
            return k + 1;
        }
    }
    fail_closed
}

/// Offending `file:line` entries for one file's text. Each
/// `#[cfg(test)]`-gated item is skipped; scanning resumes after it.
fn offending_lines(rel: &str, text: &str) -> Vec<String> {
    let lines: Vec<&str> = text.lines().collect();
    let mut hits = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        if line.trim_start().starts_with("#[cfg(test)]") {
            i = skip_cfg_test_item(&lines, i);
            continue;
        }
        if line.trim_start().starts_with("//") {
            i += 1;
            continue;
        }
        if line.contains("crate::algorithms") || line.contains("algorithms::") {
            hits.push(format!("{rel}:{}: {}", i + 1, line.trim()));
        }
        i += 1;
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

const PROD: &str = "use crate::algorithms::taint::TaintConfig;\n";

#[test]
fn production_import_after_cfg_test_module_is_reported() {
    let text = format!("#[cfg(test)]\nmod tests {{}}\n{PROD}");
    assert_eq!(offending_lines("src/x.rs", &text).len(), 1);
    let text = format!("#[cfg(test)]\nmod tests {{\n    fn t() {{}}\n}}\n\n{PROD}");
    assert_eq!(offending_lines("src/x.rs", &text).len(), 1);
}

#[test]
fn production_import_after_cfg_test_fn_is_reported() {
    let text = format!("#[cfg(test)]\nfn helper() {{\n    let _ = 1;\n}}\n{PROD}");
    assert_eq!(offending_lines("src/x.rs", &text).len(), 1);
}

#[test]
fn imports_inside_cfg_test_module_stay_exempt() {
    let text = "#[cfg(test)]\nmod tests {\n    use crate::algorithms::taint::TaintConfig;\n    fn t() {\n        algorithms::x();\n    }\n}\n";
    assert!(offending_lines("src/x.rs", text).is_empty());
    let nested = "impl A {\n    #[cfg(test)]\n    fn t() {\n        algorithms::x();\n    }\n}\n";
    assert!(offending_lines("src/x.rs", nested).is_empty());
}

#[test]
fn cfg_test_one_liner_exempts_only_that_line() {
    let text = format!("#[cfg(test)]\nmod x;\n{PROD}");
    let hits = offending_lines("src/x.rs", &text);
    assert_eq!(hits.len(), 1);
    assert!(hits[0].starts_with("src/x.rs:3:"), "{hits:?}");
    let exempt = "#[cfg(test)]\nuse crate::algorithms::taint::TaintConfig;\nfn a() {}\n";
    assert!(offending_lines("src/x.rs", exempt).is_empty());
}

#[test]
fn comment_lines_are_not_scanned() {
    let text = "/// see `algorithms::check_parse_quality`\n// crate::algorithms::taint\n//! algorithms::x\nfn a() {}\n";
    assert!(offending_lines("src/x.rs", text).is_empty());
    let code = "    use crate::algorithms::x; // trailing comment\n";
    assert_eq!(offending_lines("src/x.rs", code).len(), 1);
}

#[test]
fn tests_rs_basename_is_a_test_file() {
    assert!(is_test_file("src/cpg/tests.rs"));
    assert!(is_test_file("src/finding_confidence/tests.rs"));
    assert!(is_test_file("src/tests.rs"));
    assert!(!is_test_file("src/cpg/attests.rs"));
    assert!(!is_test_file("src/cpg/tests_helper.rs"));
}

fn count(text: &str) -> usize {
    offending_lines("src/x.rs", text).len()
}

#[test]
fn gated_enum_variant_does_not_exempt_the_rest_of_the_file() {
    let text = format!("enum E {{\n    #[cfg(test)]\n    Test,\n}}\n{PROD}");
    assert_eq!(count(&text), 1);
}

#[test]
fn gated_struct_field_does_not_exempt_the_rest_of_the_file() {
    let text = format!("struct S {{\n    #[cfg(test)]\n    seam: u8,\n}}\n{PROD}");
    assert_eq!(count(&text), 1);
}

fn assert_only_line(text: &str, line: usize) {
    let hits = offending_lines("src/x.rs", text);
    assert_eq!(hits.len(), 1, "{hits:?}");
    assert!(
        hits[0].starts_with(&format!("src/x.rs:{line}:")),
        "{hits:?}"
    );
}

#[test]
fn comment_between_attribute_and_item_is_trivia() {
    let one_line =
        format!("#[cfg(test)]\n// Test support.\nmod t {{ use crate::algorithms::x; }}\n{PROD}");
    assert_only_line(&one_line, 4);
    let block = format!(
        "#[cfg(test)]\n// Test support.\nmod t {{\n    use crate::algorithms::x;\n}}\n{PROD}"
    );
    assert_only_line(&block, 6);
}

#[test]
fn blank_line_between_attribute_and_item_is_trivia() {
    let one_line = format!("#[cfg(test)]\n\nmod t {{ use crate::algorithms::x; }}\n{PROD}");
    assert_only_line(&one_line, 4);
    let block = format!("#[cfg(test)]\n\nmod t {{\n    use crate::algorithms::x;\n}}\n{PROD}");
    assert_only_line(&block, 6);
}

#[test]
fn unclosed_gated_module_fails_closed() {
    let text = format!("#[cfg(test)]\nmod t {{\n    fn a() {{}}\n{PROD}");
    assert_eq!(count(&text), 1);
}

#[test]
fn gated_impl_block_is_exempt_and_scanning_resumes() {
    let text = format!(
        "#[cfg(test)]\nimpl A {{\n    fn t() {{\n        algorithms::x();\n    }}\n}}\n{PROD}"
    );
    let hits = offending_lines("src/x.rs", &text);
    assert_eq!(hits.len(), 1, "{hits:?}");
    assert!(hits[0].starts_with("src/x.rs:7:"), "{hits:?}");
}
