//! Python Django REST Framework spec.

use crate::ast::ParsedFile;
use crate::languages::Language;

use super::super::FrameworkSpec;

pub const SPEC: FrameworkSpec = FrameworkSpec {
    name: "drf",
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
    source.contains("rest_framework")
        && (source.contains("@api_view")
            || source.contains("APIView")
            || source.contains("ViewSet"))
}
