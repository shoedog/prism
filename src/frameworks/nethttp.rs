//! Go net/http framework spec.

use super::{FrameworkSpec, Origin, SanitizerRecognizer, SinkPattern, SourcePattern};
use crate::ast::ParsedFile;

pub const SPEC: FrameworkSpec = FrameworkSpec {
    name: "net/http",
    detect,
    sources: SOURCES,
    sinks: SINKS,
    sanitizers: SANITIZERS,
};

/// Detection: import path `"net/http"` plus a corroborating signal — function with
/// parameter typed `*http.Request` OR call to `http.HandleFunc` / `http.Handle`.
fn detect(parsed: &ParsedFile) -> bool {
    let source = parsed.source.as_str();
    if !source.contains("\"net/http\"") {
        return false;
    }
    // Corroborating signal: at least one of these three patterns.
    source.contains("*http.Request")
        || source.contains("http.HandleFunc")
        || source.contains("http.Handle(")
}

const SOURCES: &[SourcePattern] = &[
    SourcePattern {
        call_path: "r.URL.Query",
        origin: Origin::UserInput,
        taints_arg: None,
    },
    SourcePattern {
        call_path: "r.FormValue",
        origin: Origin::UserInput,
        taints_arg: None,
    },
    SourcePattern {
        call_path: "r.PostFormValue",
        origin: Origin::UserInput,
        taints_arg: None,
    },
    SourcePattern {
        call_path: "r.Header.Get",
        origin: Origin::UserInput,
        taints_arg: None,
    },
    SourcePattern {
        call_path: "r.Cookie",
        origin: Origin::UserInput,
        taints_arg: None,
    },
    SourcePattern {
        call_path: "r.URL.Path",
        origin: Origin::UserInput,
        taints_arg: None,
    },
    SourcePattern {
        call_path: "r.URL.RawQuery",
        origin: Origin::UserInput,
        taints_arg: None,
    },
    SourcePattern {
        call_path: "r.PathValue",
        origin: Origin::UserInput,
        taints_arg: None,
    },
];

// Empty in Commit 1; populated in Commit 2 with `http.ServeFile` etc.
const SINKS: &[SinkPattern] = &[];

// Empty for Phase 1; reserved for Phase 2/3.
const SANITIZERS: &[SanitizerRecognizer] = &[];
