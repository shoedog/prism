#[path = "../common/mod.rs"]
mod common;
use common::*;

#[test]
fn test_line_has_code_text_python_comment() {
    let source = "def foo():\n    # open(path, 'r')\n    x = 1\n";
    let parsed = ParsedFile::parse("test.py", source, Language::Python).unwrap();
    // Line 2 is the comment — "open" should NOT be found in code.
    assert!(!parsed.line_has_code_text(2, "open"));
    // Line 3 has actual code.
    assert!(parsed.line_has_code_text(3, "x"));
}

#[test]
fn test_line_has_code_text_python_string() {
    let source = "def foo():\n    msg = \"call open here\"\n    open('file.txt')\n";
    let parsed = ParsedFile::parse("test.py", source, Language::Python).unwrap();
    // Line 2: "open" is inside a string literal, not code.
    assert!(!parsed.line_has_code_text(2, "open"));
    // Line 3: "open" is an actual function call.
    assert!(parsed.line_has_code_text(3, "open"));
}

#[test]
fn test_line_has_code_text_c_comment() {
    let source = "void foo() {\n    // free(ptr)\n    int x = 1;\n}\n";
    let parsed = ParsedFile::parse("test.c", source, Language::C).unwrap();
    assert!(!parsed.line_has_code_text(2, "free"));
    assert!(parsed.line_has_code_text(3, "int"));
}

#[test]
fn test_line_has_code_text_c_string_literal() {
    let source = "void foo() {\n    char *s = \"free the memory\";\n    free(ptr);\n}\n";
    let parsed = ParsedFile::parse("test.c", source, Language::C).unwrap();
    // "free" on line 2 is in a string.
    assert!(!parsed.line_has_code_text(2, "free"));
    // "free" on line 3 is actual code.
    assert!(parsed.line_has_code_text(3, "free"));
}

#[test]
fn test_line_has_code_text_go_comment() {
    let source = "package main\n\nfunc foo() {\n    // defer f.Close()\n    x := 1\n}\n";
    let parsed = ParsedFile::parse("test.go", source, Language::Go).unwrap();
    assert!(!parsed.line_has_code_text(4, "defer "));
    assert!(parsed.line_has_code_text(5, "x"));
}

#[test]
fn test_line_has_code_text_js_template_string() {
    let source = "function foo() {\n    let msg = `call open(path) here`;\n    open('file');\n}\n";
    let parsed = ParsedFile::parse("test.js", source, Language::JavaScript).unwrap();
    assert!(!parsed.line_has_code_text(2, "open"));
    assert!(parsed.line_has_code_text(3, "open"));
}

#[test]
fn test_line_has_code_text_cpp_block_comment() {
    let source = "void foo() {\n    /* std::unique_ptr<int> p; */\n    int *x = new int(1);\n}\n";
    let parsed = ParsedFile::parse("test.cpp", source, Language::Cpp).unwrap();
    assert!(!parsed.line_has_code_text(2, "std::unique_ptr"));
    assert!(parsed.line_has_code_text(3, "new "));
}

#[test]
fn test_line_has_code_text_mixed_code_and_comment() {
    // Line has both code and a trailing comment with the pattern.
    let source = "void foo() {\n    int x = 1; // use free() here later\n}\n";
    let parsed = ParsedFile::parse("test.c", source, Language::C).unwrap();
    // "free" appears only in the trailing comment.
    assert!(!parsed.line_has_code_text(2, "free"));
    // "int" appears in actual code on this line.
    assert!(parsed.line_has_code_text(2, "int"));
}
