//! Pure helpers shared by the slice-level member-only parameter tests (copied from core's
//! `cpg::member_only_param_tests`, which keeps its own copies for the DFG-level tests).

use crate::languages::Language;

pub(super) const JS_TS: [Language; 3] = [Language::JavaScript, Language::TypeScript, Language::Tsx];

pub(super) fn file_name(language: Language) -> &'static str {
    match language {
        Language::JavaScript => "m.js",
        Language::TypeScript => "m.ts",
        Language::Tsx => "m.tsx",
        Language::Python => "m.py",
        Language::Go => "m.go",
        Language::C => "m.c",
        _ => unreachable!(),
    }
}

/// `{P}` in a template is the formal; TS/TSX get a type annotation.
pub(super) fn render(language: Language, template: &str, formal: &str) -> String {
    let formal = match language {
        Language::JavaScript => formal.to_string(),
        _ => format!("{formal}: any"),
    };
    template.replace("{P}", &formal)
}
