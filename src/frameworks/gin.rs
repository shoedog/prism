//! Go gin framework spec.

use super::{FrameworkSpec, Origin, SanitizerRecognizer, SinkPattern, SourcePattern};
use crate::ast::ParsedFile;

pub const SPEC: FrameworkSpec = FrameworkSpec {
    name: "gin",
    detect,
    sources: SOURCES,
    sinks: SINKS,
    sanitizers: SANITIZERS,
};

/// Detection: import path `"github.com/gin-gonic/gin"` plus a corroborating signal —
/// function with parameter typed `*gin.Context`.
fn detect(parsed: &ParsedFile) -> bool {
    let source = parsed.source.as_str();
    if !source.contains("\"github.com/gin-gonic/gin\"") {
        return false;
    }
    source.contains("*gin.Context")
}

const SOURCES: &[SourcePattern] = &[
    SourcePattern {
        call_path: "c.Param",
        origin: Origin::UserInput,
        taints_arg: None,
    },
    SourcePattern {
        call_path: "c.Query",
        origin: Origin::UserInput,
        taints_arg: None,
    },
    SourcePattern {
        call_path: "c.PostForm",
        origin: Origin::UserInput,
        taints_arg: None,
    },
    SourcePattern {
        call_path: "c.GetHeader",
        origin: Origin::UserInput,
        taints_arg: None,
    },
    SourcePattern {
        call_path: "c.Request.URL.Path",
        origin: Origin::UserInput,
        taints_arg: None,
    },
];

/// Framework-gated CWE-22 sinks (spec §3.3 / §2.7). Cross-cutting Go path
/// sinks (`os.Open`, etc.) live in `taint.rs::GO_CWE22_SINKS`; this list is
/// for sinks that are only meaningful in a gin context.
const SINKS: &[SinkPattern] = &[SinkPattern {
    call_path: "c.File",
    category: super::SanitizerCategory::PathTraversal,
    tainted_arg_indices: &[0],
    semantic_check: None,
}];

const SANITIZERS: &[SanitizerRecognizer] = &[];
