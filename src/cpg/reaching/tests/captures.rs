use super::*;

fn assert_capture(source: &str, language: Language, def_line: usize, use_line: usize) {
    let parsed = parsed(source, language);
    let func = function(&parsed);
    let outer = def(&parsed, 0, "x", def_line, 0, false);
    let capture = edge(&parsed, &func, &outer, use_line);
    let outcome = run(&parsed, &[outer], std::slice::from_ref(&capture)).0;
    assert_label(
        &outcome,
        &capture,
        FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete),
    );
}

#[test]
fn python_lambda_capture_timing_is_unknown() {
    assert_capture(
        "def f():\n    x = 1\n    delayed = lambda: use(x)\n    x = 2\n    return delayed\n",
        Language::Python,
        2,
        3,
    );
}

#[test]
fn python_yield_lambda_line_anchor_is_a_capture() {
    let parsed = parsed(
        "def f():\n    x = 1\n    yield lambda: x\n    x = 2\n",
        Language::Python,
    );
    let func = function(&parsed);
    let original = def(&parsed, 0, "x", 2, 0, false);
    let later = def(&parsed, 1, "x", 4, 0, false);
    let capture = edge(&parsed, &func, &original, 3);
    assert_eq!(capture.to.start_byte, parsed.line_start_byte(3));
    assert_eq!(capture.to.end_byte, capture.to.start_byte);
    let outcome = run(&parsed, &[original, later], std::slice::from_ref(&capture)).0;
    assert_label(
        &outcome,
        &capture,
        FlowConfidence::NameOnly(FlowDoubt::CfgIncomplete),
    );
}

#[test]
fn python_nested_def_capture_timing_is_unknown() {
    assert_capture(
        "def f():\n    x = 1\n    def inner():\n        use(x)\n    x = 2\n    return inner\n",
        Language::Python,
        2,
        4,
    );
}

#[test]
fn go_defer_function_literal_capture_timing_is_unknown() {
    assert_capture(
        "package p\nfunc f() {\n    x := 1\n    defer func() { use(x) }()\n    x = 2\n}\n",
        Language::Go,
        3,
        4,
    );
}

#[test]
fn go_statement_function_literal_capture_timing_is_unknown() {
    assert_capture(
        "package p\nfunc f() {\n    x := 1\n    go func() { use(x) }()\n    x = 2\n}\n",
        Language::Go,
        3,
        4,
    );
}

#[test]
fn javascript_arrow_capture_timing_is_unknown() {
    assert_capture(
        "function f() {\n  let x = 1;\n  const delayed = () => use(x);\n  x = 2;\n  return delayed;\n}\n",
        Language::JavaScript,
        2,
        3,
    );
}

#[test]
fn javascript_function_expression_capture_timing_is_unknown() {
    assert_capture(
        "function f() {\n  let x = 1;\n  const delayed = function () { use(x); };\n  x = 2;\n  return delayed;\n}\n",
        Language::JavaScript,
        2,
        3,
    );
}

#[test]
fn typescript_arrow_capture_timing_is_unknown() {
    assert_capture(
        "function f() {\n  let x: number = 1;\n  const delayed = () => use(x);\n  x = 2;\n  return delayed;\n}\n",
        Language::TypeScript,
        2,
        3,
    );
}

#[test]
fn typescript_function_expression_capture_timing_is_unknown() {
    assert_capture(
        "function f() {\n  let x: number = 1;\n  const delayed = function () { use(x); };\n  x = 2;\n  return delayed;\n}\n",
        Language::TypeScript,
        2,
        3,
    );
}

#[test]
fn rust_closure_capture_timing_is_unknown() {
    assert_capture(
        "fn f() {\n    let mut x = 1;\n    let delayed = || use_it(x);\n    x = 2;\n    drop(delayed);\n}\n",
        Language::Rust,
        2,
        3,
    );
}

#[test]
fn rust_async_block_capture_timing_is_unknown() {
    assert_capture(
        "fn f() {\n    let x = source();\n    let delayed = async {\n        use_it(x);\n    };\n    drop(delayed);\n}\n",
        Language::Rust,
        2,
        4,
    );
}

#[test]
fn rust_gen_block_capture_timing_is_unknown() {
    assert_capture(
        "fn f() {\n    let x = source();\n    let delayed = gen {\n        yield x;\n    };\n    drop(delayed);\n}\n",
        Language::Rust,
        2,
        4,
    );
}

#[test]
fn rust_immediate_block_read_stays_exact() {
    let parsed = parsed(
        "fn f() {\n    let x = source();\n    {\n        use_it(x);\n    }\n}\n",
        Language::Rust,
    );
    let func = function(&parsed);
    let outer = def(&parsed, 0, "x", 2, 0, false);
    let immediate = edge(&parsed, &func, &outer, 4);
    let outcome = run(&parsed, &[outer], std::slice::from_ref(&immediate)).0;
    assert_label(&outcome, &immediate, FlowConfidence::Exact);
}

#[test]
fn go_defer_argument_is_evaluated_now() {
    let parsed = parsed(
        "package p\nfunc f() {\n    x := 1\n    defer use(x)\n    x = 2\n    return\n}\n",
        Language::Go,
    );
    let func = function(&parsed);
    let original = def(&parsed, 0, "x", 3, 0, false);
    let later = def(&parsed, 1, "x", 5, 0, false);
    let immediate = edge(&parsed, &func, &original, 4);
    let outcome = run(
        &parsed,
        &[original, later],
        std::slice::from_ref(&immediate),
    )
    .0;
    assert_label(&outcome, &immediate, FlowConfidence::Exact);
}
