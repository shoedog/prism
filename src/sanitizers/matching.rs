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
pub(crate) fn function_body_cleansed_for(
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
/// `go_path_traversal_cleansed_for_sink` near :5986) to rule out inverted or guard-after-sink
/// shapes, which this single-call-node check cannot replicate. They stay advisory-only via the
/// unchanged `Cleansed` warning / `sanitizers_present_in_source_fn` path.
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
