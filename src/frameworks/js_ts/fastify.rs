use crate::ast::ParsedFile;
use crate::frameworks::FrameworkSpec;
use crate::languages::Language;

pub const SPEC: FrameworkSpec = FrameworkSpec {
    name: "fastify",
    detect,
    sources: &[],
    sinks: &[],
    sanitizers: &[],
};

fn detect(parsed: &ParsedFile) -> bool {
    if !matches!(
        parsed.language,
        Language::JavaScript | Language::TypeScript | Language::Tsx
    ) {
        return false;
    }
    parsed
        .extract_imports()
        .values()
        .any(|module| module == "fastify")
}
