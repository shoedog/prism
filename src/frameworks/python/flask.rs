//! Python Flask framework spec.

use crate::ast::ParsedFile;
use crate::languages::Language;

use super::super::FrameworkSpec;

pub const SPEC: FrameworkSpec = FrameworkSpec {
    name: "flask",
    detect,
    sources: &[],
    sinks: &[],
    sanitizers: &[],
};

fn detect(parsed: &ParsedFile) -> bool {
    if parsed.language != Language::Python {
        return false;
    }
    let source = parsed.source.as_str();
    (source.contains("from flask import") || source.contains("import flask"))
        && source.contains("Flask(")
}
