//! Go gorilla/mux framework spec.

use super::{FrameworkSpec, Origin, SanitizerRecognizer, SinkPattern, SourcePattern};
use crate::ast::ParsedFile;

pub const SPEC: FrameworkSpec = FrameworkSpec {
    name: "gorilla/mux",
    detect,
    sources: SOURCES,
    sinks: SINKS,
    sanitizers: SANITIZERS,
};

/// Detection: import path `"github.com/gorilla/mux"` plus a corroborating signal —
/// call to `mux.Vars(`.
fn detect(parsed: &ParsedFile) -> bool {
    let source = parsed.source.as_str();
    if !source.contains("\"github.com/gorilla/mux\"") {
        return false;
    }
    source.contains("mux.Vars(")
}

const SOURCES: &[SourcePattern] = &[SourcePattern {
    call_path: "mux.Vars",
    origin: Origin::UserInput,
    taints_arg: None,
}];

// Empty for Phase 1.
const SINKS: &[SinkPattern] = &[];
const SANITIZERS: &[SanitizerRecognizer] = &[];
