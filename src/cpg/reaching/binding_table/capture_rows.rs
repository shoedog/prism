use super::{CaptureRow, Timing};
use crate::languages::Language;

macro_rules! capture_rows {
    ($name:ident, $language:expr, $short:literal, [$($kind:literal),+ $(,)?]) => {
        static $name: &[CaptureRow] = &[
            $(CaptureRow {
                language: $language,
                kind: $kind,
                variant: None,
                timing: Timing::Deferred,
                regression: concat!("e0a-x-capture-", $short, "-", $kind),
            },)+
        ];
    };
}

capture_rows!(
    PYTHON,
    Language::Python,
    "py",
    ["function_definition", "decorated_definition", "lambda"]
);
capture_rows!(
    JAVASCRIPT,
    Language::JavaScript,
    "js",
    [
        "function_declaration",
        "method_definition",
        "arrow_function",
        "function_expression",
        "generator_function_declaration",
        "generator_function",
    ]
);
capture_rows!(
    TYPESCRIPT,
    Language::TypeScript,
    "ts",
    [
        "function_declaration",
        "method_definition",
        "arrow_function",
        "function_expression",
        "generator_function_declaration",
        "generator_function",
    ]
);
capture_rows!(
    TSX,
    Language::Tsx,
    "tsx",
    [
        "function_declaration",
        "method_definition",
        "arrow_function",
        "function_expression",
        "generator_function_declaration",
        "generator_function",
    ]
);
capture_rows!(
    GO,
    Language::Go,
    "go",
    ["function_declaration", "method_declaration", "func_literal"]
);
capture_rows!(
    JAVA,
    Language::Java,
    "java",
    [
        "method_declaration",
        "constructor_declaration",
        "lambda_expression",
    ]
);
capture_rows!(C, Language::C, "c", ["function_definition"]);
capture_rows!(
    CPP,
    Language::Cpp,
    "cpp",
    [
        "function_definition",
        "template_declaration",
        "lambda_expression",
    ]
);
capture_rows!(
    RUST,
    Language::Rust,
    "rs",
    [
        "function_item",
        "closure_expression",
        "async_block",
        "gen_block",
    ]
);
capture_rows!(
    LUA,
    Language::Lua,
    "lua",
    ["function_declaration", "function_definition"]
);
capture_rows!(TERRAFORM, Language::Terraform, "tf", ["block"]);
capture_rows!(BASH, Language::Bash, "bash", ["function_definition"]);

pub(super) fn rows(language: Language) -> &'static [CaptureRow] {
    match language {
        Language::Python => PYTHON,
        Language::JavaScript => JAVASCRIPT,
        Language::TypeScript => TYPESCRIPT,
        Language::Tsx => TSX,
        Language::Go => GO,
        Language::Java => JAVA,
        Language::C => C,
        Language::Cpp => CPP,
        Language::Rust => RUST,
        Language::Lua => LUA,
        Language::Terraform => TERRAFORM,
        Language::Bash => BASH,
    }
}
