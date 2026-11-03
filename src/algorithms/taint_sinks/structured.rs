//! Structured-registry veto for sink strength (#636).
//!
//! The flat rules grade a weak SQL call, but a sink only the structured registry knows (SSRF,
//! file paths, command execution, ...) may share the line or sit inside the weak call's
//! arguments. Every structured pattern is consulted, not the first match, and without a flow
//! path (call path and semantic check only): any non-SQL match keeps the line Strong.

use crate::algorithms::taint::{
    line_matches_structured_sink, SinkMatchOutcome, GO_CWE22_SINKS, GO_CWE78_SINKS, JS_CWE22_SINKS,
    JS_CWE502_SINKS, JS_CWE78_SINKS, JS_CWE79_SINKS, JS_CWE918_SINKS, PY_CWE502_SINKS,
    PY_CWE79_SINKS, PY_CWE918_SINKS,
};
use crate::ast::ParsedFile;
use crate::frameworks::{SanitizerCategory, SinkPattern};
use crate::languages::Language;

pub(super) fn non_sql_match_on_line(parsed: &ParsedFile, line: usize) -> bool {
    let lists: Vec<&'static [SinkPattern]> = match parsed.language {
        Language::Go => {
            let mut v: Vec<&'static [SinkPattern]> = vec![GO_CWE78_SINKS, GO_CWE22_SINKS];
            v.extend(parsed.framework().map(|f| f.sinks));
            v
        }
        Language::Python => vec![PY_CWE79_SINKS, PY_CWE918_SINKS, PY_CWE502_SINKS],
        l if l.is_js_family() => vec![
            JS_CWE79_SINKS,
            JS_CWE918_SINKS,
            JS_CWE502_SINKS,
            JS_CWE78_SINKS,
            JS_CWE22_SINKS,
        ],
        _ => return false,
    };
    lists.into_iter().flatten().any(|pat| {
        pat.category != SanitizerCategory::Sqli
            && matches!(
                line_matches_structured_sink(parsed, line, pat, None),
                SinkMatchOutcome::Match(_)
            )
    })
}
