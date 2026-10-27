use super::{CaptureRow, Predicate, Timing};
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

static PYTHON: &[CaptureRow] = &[
    CaptureRow {
        language: Language::Python,
        kind: "lambda",
        variant: Some(Predicate::IsImmediatelyInvoked),
        timing: Timing::Immediate,
        regression: "py-lambda-called-in-place",
    },
    CaptureRow {
        language: Language::Python,
        kind: "function_definition",
        variant: None,
        timing: Timing::Deferred,
        regression: "e0a-x-capture-py-function_definition",
    },
    CaptureRow {
        language: Language::Python,
        kind: "decorated_definition",
        variant: None,
        timing: Timing::Deferred,
        regression: "e0a-x-capture-py-decorated_definition",
    },
    CaptureRow {
        language: Language::Python,
        kind: "lambda",
        variant: None,
        timing: Timing::Deferred,
        regression: "e0a-x-capture-py-lambda",
    },
];
macro_rules! js_capture_rows {
    ($name:ident, $language:expr, $short:literal, $arrow_regression:literal, $function_regression:literal, [$($kind:literal),+ $(,)?]) => {
        static $name: &[CaptureRow] = &[
            CaptureRow { language: $language, kind: "arrow_function", variant: Some(Predicate::IsImmediatelyInvoked), timing: Timing::Immediate, regression: $arrow_regression },
            CaptureRow { language: $language, kind: "function_expression", variant: Some(Predicate::IsImmediatelyInvoked), timing: Timing::Immediate, regression: $function_regression },
            $(CaptureRow { language: $language, kind: $kind, variant: None, timing: Timing::Deferred, regression: concat!("e0a-x-capture-", $short, "-", $kind) },)+
        ];
    };
}
js_capture_rows!(
    JAVASCRIPT,
    Language::JavaScript,
    "js",
    "js-iife-arrow-invoked",
    "js-iife-parenthesized-invoked",
    [
        "function_declaration",
        "method_definition",
        "arrow_function",
        "function_expression",
        "generator_function_declaration",
        "generator_function",
    ]
);
js_capture_rows!(
    TYPESCRIPT,
    Language::TypeScript,
    "ts",
    "ts-iife-arrow-invoked",
    "ts-iife-function-invoked",
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
static CPP: &[CaptureRow] = &[
    CaptureRow {
        language: Language::Cpp,
        kind: "lambda_expression",
        variant: Some(Predicate::IsImmediatelyInvoked),
        timing: Timing::Immediate,
        regression: "cpp-lambda-invoked-vs-stored",
    },
    CaptureRow {
        language: Language::Cpp,
        kind: "function_definition",
        variant: None,
        timing: Timing::Deferred,
        regression: "e0a-x-capture-cpp-function_definition",
    },
    CaptureRow {
        language: Language::Cpp,
        kind: "template_declaration",
        variant: None,
        timing: Timing::Deferred,
        regression: "e0a-x-capture-cpp-template_declaration",
    },
    CaptureRow {
        language: Language::Cpp,
        kind: "lambda_expression",
        variant: None,
        timing: Timing::Deferred,
        regression: "e0a-x-capture-cpp-lambda_expression",
    },
];
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
