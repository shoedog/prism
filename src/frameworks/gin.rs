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

// Empty in Commit 1; populated in Commit 2 with `c.File` etc.
const SINKS: &[SinkPattern] = &[];

const SANITIZERS: &[SanitizerRecognizer] = &[];
