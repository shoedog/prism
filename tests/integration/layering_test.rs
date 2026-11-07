//! Layering rule (#702): core modules must not depend on the review layer.
//!
//! No non-test source file outside the review layer may mention
//! `crate::algorithms` or `algorithms::`. The review layer is
//! `src/algorithms/**`, `src/output/**`, `src/targets/**`, `src/api/**`,
//! `src/cli.rs`, `src/main.rs`, `src/lib.rs` and `src/bin/**`.
//!
//! Exempt test code: any `*_test.rs` / `*_tests.rs` file, any path containing
//! `/tests/`, and each `#[cfg(test)]`-gated ITEM inside a file. Scanning
//! resumes after the item, so production code after a test module is checked.
//! The item is found at any indentation: skip the attribute line(s), then the
//! item line. If the item line ends with `;` (`mod x;`, `use ..;`) or is a
//! one-line `{ .. }` item, only that line is exempt. Otherwise the item runs
//! through the first following line that is exactly `}` or `};` at the SAME
//! indentation as the attribute (rustfmt guarantees this for items).

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

fn indent_of(line: &str) -> usize {
    line.len() - line.trim_start().len()
}

/// Index just past the `#[cfg(test)]`-gated item whose attribute is at `attr`.
fn skip_cfg_test_item(lines: &[&str], attr: usize) -> usize {
    let indent = indent_of(lines[attr]);
    let rest = lines[attr].trim_start()["#[cfg(test)]".len()..].trim();
    let mut item = attr;
    if rest.is_empty() {
        item += 1;
        while item < lines.len() && lines[item].trim_start().starts_with("#[") {
            item += 1;
        }
    }
    if item >= lines.len() {
        return lines.len();
    }
    let head = lines[item].trim_end();
    if head.ends_with(';') || (head.ends_with('}') && head.contains('{')) {
        return item + 1;
    }
    let pad = " ".repeat(indent);
    let (close, close_semi) = (format!("{pad}}}"), format!("{pad}}};"));
    for (k, line) in lines.iter().enumerate().skip(item + 1) {
        if *line == close || *line == close_semi {
            return k + 1;
        }
    }
    lines.len()
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
