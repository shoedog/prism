//! Call-site sanitizer matching used by the taint algorithms and the
//! reasoning layer: per-function cleansed categories, category names, and the
//! node-scoped `sanitizer_call_site` matcher.
//!
//! Moved verbatim out of `algorithms/taint.rs` (#702), resolving the old
//! "TEMPORARY layering inversion" between `reasoning` and the taint algorithm.

use crate::ast::tree_helpers::{call_path_matches, call_path_text, collect_calls};
use crate::ast::ParsedFile;
use crate::frameworks::{CallSite, SanitizerCategory};
use tree_sitter::Node;

/// Returns true if the function body containing `line` in `parsed` has at least one
/// active sanitizer recognizer call whose category equals `category` AND whose
/// `languages` admits `parsed.language` (item B / #7: the same predicate
/// `sanitizer_call_site` applies). Walks the enclosing function for the line, applies
/// each recognizer's `semantic_check` and textual `paired_check`, and returns on
/// first match.
///
/// Used both by `apply_cleansers` (per-FlowPath, all categories) and by the
/// source==sink fallback (single-category check when no FlowPath exists for the line);
/// `cleansed_categories_for_source` also delegates its per-category matching here, so
/// this is the single place both advisory entry points are gated by language.
///
/// The walk is intraprocedural (cleanser must live in same function as `line`);
/// cross-function cleansing is a Phase 1.5+ concern.
pub fn function_body_cleansed_for(
    parsed: &ParsedFile,
    line: usize,
    category: SanitizerCategory,
) -> bool {
    let func_node = match parsed.enclosing_function(line) {
        Some(n) => n,
        None => return false,
    };
    let func_text = func_node.utf8_text(parsed.source.as_bytes()).unwrap_or("");

    let mut calls = Vec::new();
    collect_calls(parsed, func_node, &mut calls);

    for recognizer in crate::sanitizers::active_recognizers() {
        if recognizer.category != category {
            continue;
        }
        if !recognizer.languages.contains(&parsed.language) {
            continue;
        }
        // Look for a call to the recognizer's call_path within the function.
        let mut matched = false;
        for call in &calls {
            let actual = match call_path_text(parsed, call) {
                Some(s) => s,
                None => continue,
            };
            if !call_path_matches(parsed, &actual, recognizer.call_path) {
                continue;
            }
            if let Some(check) = recognizer.semantic_check {
                let cs = CallSite {
                    call_node: *call,
                    source: parsed.source.as_str(),
                };
                if !check(&cs) {
                    continue;
                }
            }
            matched = true;
            break;
        }
        if !matched {
            continue;
        }
        // For paired-check recognizers, the second-half check must also appear
        // in the function body (textual co-occurrence per §3.4 / §3.8).
        if let Some(paired) = recognizer.paired_check {
            if !crate::sanitizers::paired_check_satisfied(func_text, paired) {
                continue;
            }
        }
        return true;
    }
    false
}

/// A4: the single reasoning-facing cleansing adapter. Returns the sanitizer categories present
/// in the SOURCE FUNCTION BODY (NOT path-proof) as lowercase strings. Gated to Go/Python/JS-TS
/// exactly like apply_cleansers; honest-empty otherwise. Per-recognizer language filtering
/// (item B / #7) happens one layer down in `function_body_cleansed_for`, which this function
/// delegates every category check to — there is no independent recognizer-matching logic here
/// to filter separately.
pub(crate) fn cleansed_categories_for_source(
    files: &std::collections::BTreeMap<String, ParsedFile>,
    source: &crate::data_flow::VarLocation,
) -> Vec<String> {
    let parsed = match files.get(&source.file) {
        Some(p) => p,
        None => return Vec::new(),
    };
    if !crate::sanitizers::sanitizer_supported(parsed.language) {
        return Vec::new();
    }

    let mut out = Vec::new();
    let cats: std::collections::BTreeSet<crate::frameworks::SanitizerCategory> =
        crate::sanitizers::active_recognizers()
            .map(|r| r.category)
            .collect();
    for category in cats {
        if function_body_cleansed_for(parsed, source.line, category) {
            out.push(sanitizer_category_str(category).to_string());
        }
    }
    out
}

/// A candidate call node recognized as an active, non-`paired_check` sanitizer.
pub(crate) struct SanitizerCallSite {
    pub category: crate::frameworks::SanitizerCategory,
    pub callee_text: String,
    pub file: String,
    pub line: usize,
    /// P10 F3: the matched recognizer's `data_param` (see `SanitizerRecognizer::data_param`),
    /// threaded through so `src/reasoning/sanitizer_walk.rs` can accept a Python
    /// `keyword_argument`'s VALUE span (name == this) as the sanitizer's data argument, distinct
    /// from the keyword LABEL identifier or an unrelated tainted kwarg.
    pub data_param: Option<&'static str>,
}

pub(crate) fn sanitizer_category_str(c: crate::frameworks::SanitizerCategory) -> &'static str {
    use crate::frameworks::SanitizerCategory::*;
    match c {
        Xss => "xss",
        Sqli => "sqli",
        Ssrf => "ssrf",
        Deserialization => "deserialization",
        OsCommand => "os_command",
        PathTraversal => "path_traversal",
    }
}

/// P10 node-scoped matcher: does `call_node` match an active sanitizer recognizer? Checked across
/// ALL categories (first match wins by `active_recognizers()` order) — a deliberate divergence from
/// the CWE engine's PER-CATEGORY sink cut (`path.cleansed_for.contains(&sink_pat.category)` near
/// :5996): `taint_reaches`'s honesty tier proves "a recognized sanitizer transform sits on this
/// path" without needing to know the specific sink category, so any-category is the right cut here.
///
/// Excludes `paired_check` recognizers (the Go path-traversal family, `src/sanitizers/path.rs`)
/// entirely: those need the CWE engine's sink-time AST+CFG guard validation (see
/// `go_path_traversal_cleansed_for_sink` in `taint/go/sanitizers.rs`) to rule out inverted or
/// guard-after-sink shapes, which this single-call-node check cannot replicate. They stay
/// advisory-only via the unchanged `Cleansed` warning / `sanitizers_present_in_source_fn` path.
///
/// F2 BLOCKER (also filters by `recognizer.languages`): `call_path_matches`'s exact-match branch is
/// NOT gated by language, and `sanitizer_supported` (the file-language gate one layer up) admits
/// Go — so an unqualified name shared across tables (bare `escape` is registered by BOTH
/// `JS_TS_RECOGNIZERS` and `PYTHON_RECOGNIZERS`) would otherwise match a same-named Go call and
/// produce a false path-proven `Sanitized` verdict.
///
/// Item B (#7): `function_body_cleansed_for` (and `cleansed_categories_for_source`, which
/// delegates to it) — the advisory tier feeding `sanitizers_present_in_source_fn` / the
/// `Cleansed` warning, and the CWE sink-suppression engine's `FlowPath.cleansed_for` marks —
/// now applies the identical `recognizer.languages` predicate. Pre-#7 those two entry points
/// iterated `active_recognizers()` unfiltered by language (the P10 BLOCKER above intentionally
/// scoped its fix to this verdict-path matcher only, byte-for-byte preserving the advisory
/// tier's pre-P10 behavior); a same-name cross-table match there could fire a false `Cleansed`
/// advisory and, worse, a false CWE sink suppression (a hidden vulnerability) — the unsafe
/// direction. Both entry points now share this exact filter.
///
/// Returns the matched category plus the discriminating fact (rendered callee text, call's own
/// file/line) — NOT proof that this specific call sits on any particular taint path. Chain-window
/// proof (is this call reached from a specific source and does its result feed a specific def) is
/// the reasoning layer's job: see `src/reasoning/sanitizer_walk.rs`.
pub(crate) fn sanitizer_call_site(
    parsed: &ParsedFile,
    call_node: &Node<'_>,
) -> Option<SanitizerCallSite> {
    let actual = call_path_text(parsed, call_node)?;
    for recognizer in crate::sanitizers::active_recognizers() {
        if recognizer.paired_check.is_some() {
            continue;
        }
        if !recognizer.languages.contains(&parsed.language) {
            continue;
        }
        if !call_path_matches(parsed, &actual, recognizer.call_path) {
            continue;
        }
        if let Some(check) = recognizer.semantic_check {
            let cs = CallSite {
                call_node: *call_node,
                source: parsed.source.as_str(),
            };
            if !check(&cs) {
                continue;
            }
        }
        return Some(SanitizerCallSite {
            category: recognizer.category,
            callee_text: actual,
            file: parsed.path.clone(),
            line: call_node.start_position().row + 1,
            data_param: recognizer.data_param,
        });
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::access_path::AccessPath;
    use crate::data_flow::{VarAccessKind, VarLocation};
    use crate::languages::Language;

    #[test]
    fn test_cleansed_categories_for_source_python_xss() {
        let src = "def f(u):\n    safe = html.escape(u)\n    return safe\n";
        let mut files = std::collections::BTreeMap::new();
        files.insert(
            "t.py".to_string(),
            ParsedFile::parse("t.py", src, Language::Python).unwrap(),
        );
        let source = VarLocation {
            file: "t.py".into(),
            function: "f".into(),
            function_start_line: 1,
            line: 2,
            path: AccessPath {
                base: "u".into(),
                fields: vec![],
            },
            start_byte: 0,
            end_byte: 0,
            kind: VarAccessKind::Use,
        };
        let cats = cleansed_categories_for_source(&files, &source);
        assert!(
            cats.iter().any(|c| c == "xss"),
            "html.escape => xss: {cats:?}"
        );
    }

    #[test]
    fn test_cleansed_categories_for_source_rust_empty() {
        let mut files = std::collections::BTreeMap::new();
        files.insert(
            "t.rs".to_string(),
            ParsedFile::parse(
                "t.rs",
                "fn f(u: &str) -> String { u.to_string() }",
                Language::Rust,
            )
            .unwrap(),
        );
        let s = VarLocation {
            file: "t.rs".into(),
            function: "f".into(),
            function_start_line: 1,
            line: 1,
            path: AccessPath {
                base: "u".into(),
                fields: vec![],
            },
            start_byte: 0,
            end_byte: 0,
            kind: VarAccessKind::Use,
        };
        assert!(cleansed_categories_for_source(&files, &s).is_empty());
    }

    #[test]
    fn test_sanitizer_call_site_python_xss_match() {
        let src = "def f(u):\n    safe = html.escape(u)\n    return safe\n";
        let parsed = ParsedFile::parse("t.py", src, Language::Python).unwrap();
        let mut calls = Vec::new();
        collect_calls(&parsed, parsed.tree.root_node(), &mut calls);
        let call = calls
            .iter()
            .find(|c| c.start_position().row + 1 == 2)
            .expect("html.escape(u) call on line 2");
        let site =
            sanitizer_call_site(&parsed, call).expect("html.escape recognized as a sanitizer");
        assert_eq!(site.category, crate::frameworks::SanitizerCategory::Xss);
        assert_eq!(site.callee_text, "html.escape");
        assert_eq!(site.file, "t.py");
        assert_eq!(site.line, 2);
        // P10 F3: `data_param` must be threaded through from the matched recognizer so
        // `src/reasoning/sanitizer_walk.rs` can distinguish `html.escape`'s data kwarg (`s`) from
        // its non-data kwarg (`quote`) and from an unrelated keyword's label identifier.
        assert_eq!(site.data_param, Some("s"));
    }

    #[test]
    fn test_sanitizer_call_site_excludes_paired_check_recognizer() {
        // BLOCKER ruling: Go's path-traversal family (`filepath.Clean` / `filepath.Rel`) carries a
        // `paired_check` and must be excluded from the node-scoped matcher entirely — the CWE engine
        // validates those with sink-time AST+CFG guard analysis this single-call check cannot
        // replicate.
        let src = "package m\nfunc f(p string) string {\n\treturn filepath.Clean(p)\n}\n";
        let parsed = ParsedFile::parse("t.go", src, Language::Go).unwrap();
        let mut calls = Vec::new();
        collect_calls(&parsed, parsed.tree.root_node(), &mut calls);
        let call = calls
            .iter()
            .find(|c| c.start_position().row + 1 == 3)
            .expect("filepath.Clean(p) call on line 3");
        assert!(
            sanitizer_call_site(&parsed, call).is_none(),
            "paired_check recognizers must be excluded from the node-scoped matcher"
        );
    }

    #[test]
    fn test_sanitizer_call_site_no_match_returns_none() {
        let src = "def f(u):\n    return str(u)\n";
        let parsed = ParsedFile::parse("t.py", src, Language::Python).unwrap();
        let mut calls = Vec::new();
        collect_calls(&parsed, parsed.tree.root_node(), &mut calls);
        let call = calls
            .iter()
            .find(|c| c.start_position().row + 1 == 2)
            .expect("str(u) call on line 2");
        assert!(sanitizer_call_site(&parsed, call).is_none());
    }

    #[test]
    fn test_sanitizer_call_site_rejects_wrong_language_recognizer() {
        // F2 BLOCKER: `escape` is registered by BOTH `JS_TS_RECOGNIZERS` and `PYTHON_RECOGNIZERS`,
        // and `call_path_matches`'s exact-match branch is not gated by language — a Go `escape(u)`
        // call must not match either table's entry in the verdict-path matcher, even though Go is
        // `sanitizer_supported` (via the paired-check path family).
        let src = "package m\nfunc f(u string) string {\n\treturn escape(u)\n}\n";
        let parsed = ParsedFile::parse("t.go", src, Language::Go).unwrap();
        let mut calls = Vec::new();
        collect_calls(&parsed, parsed.tree.root_node(), &mut calls);
        let call = calls
            .iter()
            .find(|c| c.start_position().row + 1 == 3)
            .expect("escape(u) call on line 3");
        assert!(
            sanitizer_call_site(&parsed, call).is_none(),
            "a same-name recognizer from another language's table must not match Go"
        );
    }

    // --- Item B (#7): advisory tier (`function_body_cleansed_for` /
    // `cleansed_categories_for_source`) must apply the SAME `recognizer.languages` predicate as
    // `sanitizer_call_site`. Pre-fix, these two functions iterate `active_recognizers()`
    // unfiltered by language, so a same-named recognizer from a different language's table
    // (bare `escape` is registered by BOTH `JS_TS_RECOGNIZERS` and `PYTHON_RECOGNIZERS`) can
    // produce a false advisory `Cleansed` mark and a false CWE sink suppression.

    #[test]
    fn test_function_body_cleansed_for_rejects_wrong_language_go_escape() {
        // Fail-open pole: a Go function whose only "sanitizer-shaped" call is a user-defined
        // `escape(u)` must NOT be reported as XSS-cleansed — `escape` in the active registry is
        // only meaningful for JS/TS and Python, never Go.
        let src = "package m\nfunc f(u string) string {\n\treturn escape(u)\n}\n";
        let parsed = ParsedFile::parse("t.go", src, Language::Go).unwrap();
        assert!(
            !function_body_cleansed_for(&parsed, 3, SanitizerCategory::Xss),
            "a same-name recognizer from another language's table must not cleanse a Go function"
        );
    }

    #[test]
    fn test_cleansed_categories_for_source_go_escape_not_xss() {
        let src = "package m\nfunc f(u string) string {\n\treturn escape(u)\n}\n";
        let mut files = std::collections::BTreeMap::new();
        files.insert(
            "t.go".to_string(),
            ParsedFile::parse("t.go", src, Language::Go).unwrap(),
        );
        let source = VarLocation {
            file: "t.go".into(),
            function: "f".into(),
            function_start_line: 2,
            line: 3,
            path: AccessPath {
                base: "u".into(),
                fields: vec![],
            },
            start_byte: 0,
            end_byte: 0,
            kind: VarAccessKind::Use,
        };
        let cats = cleansed_categories_for_source(&files, &source);
        assert!(
            !cats.iter().any(|c| c == "xss"),
            "Go escape(u) must not surface an xss advisory category: {cats:?}"
        );
    }

    #[test]
    fn test_cleansed_categories_for_source_js_rejects_python_bleach_clean() {
        // A JS/TS file that happens to call something whose text matches the Python `bleach.clean`
        // recognizer's `call_path` (nonsensical in real JS, but this is exactly the
        // wrong-language-table collision this predicate must close) must not surface an xss
        // advisory category. `bleach.clean` (unlike `markupsafe.escape` / `escape`) has no tail
        // that collides with any `JS_TS_RECOGNIZERS` bare-name entry via `call_path_matches`'s
        // language-scoped tail-match branch, so this isolates the cross-table exact-match bug
        // from that unrelated, intentional same-language tail-matching behavior.
        let src = "function f(u) {\n  return bleach.clean(u);\n}\n";
        let mut files = std::collections::BTreeMap::new();
        files.insert(
            "t.js".to_string(),
            ParsedFile::parse("t.js", src, Language::JavaScript).unwrap(),
        );
        let source = VarLocation {
            file: "t.js".into(),
            function: "f".into(),
            function_start_line: 1,
            line: 2,
            path: AccessPath {
                base: "u".into(),
                fields: vec![],
            },
            start_byte: 0,
            end_byte: 0,
            kind: VarAccessKind::Use,
        };
        let cats = cleansed_categories_for_source(&files, &source);
        assert!(
            !cats.iter().any(|c| c == "xss"),
            "JS bleach.clean(u) must not surface an xss advisory category: {cats:?}"
        );
    }

    #[test]
    fn test_cleansed_categories_for_source_python_html_escape_still_positive() {
        // Same-language positive pole: this must remain unaffected by the language filter.
        let src = "def f(u):\n    safe = html.escape(u)\n    return safe\n";
        let mut files = std::collections::BTreeMap::new();
        files.insert(
            "t.py".to_string(),
            ParsedFile::parse("t.py", src, Language::Python).unwrap(),
        );
        let source = VarLocation {
            file: "t.py".into(),
            function: "f".into(),
            function_start_line: 1,
            line: 2,
            path: AccessPath {
                base: "u".into(),
                fields: vec![],
            },
            start_byte: 0,
            end_byte: 0,
            kind: VarAccessKind::Use,
        };
        let cats = cleansed_categories_for_source(&files, &source);
        assert!(
            cats.iter().any(|c| c == "xss"),
            "html.escape must still cleanse for xss in its own language: {cats:?}"
        );
    }

    #[test]
    fn test_cleansed_categories_for_source_js_dompurify_sanitize_still_positive() {
        // Same-language positive pole for the JS/TS table itself — guards against an inverted
        // predicate silently disabling the whole `JS_TS_RECOGNIZERS` table.
        let src = "function f(u) {\n  return DOMPurify.sanitize(u);\n}\n";
        let mut files = std::collections::BTreeMap::new();
        files.insert(
            "t.js".to_string(),
            ParsedFile::parse("t.js", src, Language::JavaScript).unwrap(),
        );
        let source = VarLocation {
            file: "t.js".into(),
            function: "f".into(),
            function_start_line: 1,
            line: 2,
            path: AccessPath {
                base: "u".into(),
                fields: vec![],
            },
            start_byte: 0,
            end_byte: 0,
            kind: VarAccessKind::Use,
        };
        let cats = cleansed_categories_for_source(&files, &source);
        assert!(
            cats.iter().any(|c| c == "xss"),
            "DOMPurify.sanitize must still cleanse for xss in its own language: {cats:?}"
        );
    }

    #[test]
    fn test_function_body_cleansed_for_go_path_traversal_paired_check_still_positive() {
        // Go's paired-check family (`filepath.Clean` / `strings.HasPrefix`) is Go-only in
        // `languages`, so it must be completely unaffected by this fix.
        let src = "package m\nfunc f(base string, name string) string {\n\tcleaned := filepath.Clean(name)\n\tif strings.HasPrefix(cleaned, base) {\n\t\treturn cleaned\n\t}\n\treturn \"\"\n}\n";
        let parsed = ParsedFile::parse("t.go", src, Language::Go).unwrap();
        assert!(
            function_body_cleansed_for(&parsed, 5, SanitizerCategory::PathTraversal),
            "Go filepath.Clean + strings.HasPrefix paired check must remain an advisory positive"
        );
    }
}
