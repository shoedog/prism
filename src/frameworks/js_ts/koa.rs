use crate::ast::ParsedFile;
use crate::frameworks::FrameworkSpec;
use crate::languages::Language;

pub const SPEC: FrameworkSpec = FrameworkSpec {
    name: "koa",
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
    parsed.extract_imports().values().any(|module| {
        matches!(
            module.as_str(),
            "koa" | "@koa/router" | "koa-router" | "@koa/cors"
        )
    })
}
