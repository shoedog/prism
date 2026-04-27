//! Python FastAPI framework spec.

use crate::ast::ParsedFile;
use crate::languages::Language;

use super::super::FrameworkSpec;

pub const SPEC: FrameworkSpec = FrameworkSpec {
    name: "fastapi",
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
    if !source.contains("fastapi") {
        return false;
    }
    (source.contains("FastAPI(") || source.contains("APIRouter("))
        && (source.contains("@app.")
            || source.contains("@router.")
            || source.contains(".api_route("))
}
