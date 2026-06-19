use prism::ast::ParsedFile;
use prism::cpg::{CodePropertyGraph, Trace};
use prism::languages::Language;
use prism::navigation::types::{ReasoningWarning, WarningKind};
use prism::reasoning::scope_honesty::{
    warnings_for_function, warnings_for_sink_location, warnings_for_trace,
};

fn parsed(path: &str, source: &str, language: Language) -> ParsedFile {
    ParsedFile::parse(path, source, language).unwrap()
}

fn constructs_from_warnings(warnings: Vec<prism::navigation::types::Warning>) -> Vec<String> {
    warnings
        .into_iter()
        .map(|warning| match warning.kind {
            WarningKind::Reasoning(ReasoningWarning::UnmodeledConstruct { construct, .. }) => {
                construct
            }
            other => panic!("unexpected warning kind: {other:?}"),
        })
        .collect()
}

fn warning_constructs(file: &ParsedFile) -> Vec<String> {
    let function = file.all_functions().into_iter().next().unwrap();
    constructs_from_warnings(warnings_for_function(file, function))
}

#[test]
fn rust_try_and_closure_emit_scope_honesty_warnings() {
    let file = parsed(
        "src/lib.rs",
        "fn f() -> Result<(), E> {\n    let cb = || source();\n    fallible()?;\n    Ok(())\n}\n",
        Language::Rust,
    );

    let constructs = warning_constructs(&file);
    assert!(constructs.contains(&"Rust ? operator".to_string()));
    assert!(constructs.contains(&"Rust closure".to_string()));
}

#[test]
fn go_concurrency_and_channel_ops_emit_scope_honesty_warnings() {
    let file = parsed(
        "main.go",
        "func f(ch chan string) {\n    go work()\n    ch <- source()\n    sink(<-ch)\n    select { case v := <-ch: sink(v) }\n}\n",
        Language::Go,
    );

    let constructs = warning_constructs(&file);
    assert!(constructs.contains(&"Go go statement".to_string()));
    assert!(constructs.contains(&"Go channel send".to_string()));
    assert!(constructs.contains(&"Go channel receive".to_string()));
    assert!(constructs.contains(&"Go select".to_string()));
}

#[test]
fn js_and_ts_async_constructs_emit_scope_honesty_warnings() {
    let js = parsed(
        "app.js",
        "async function f(p) {\n    await p;\n    return p.then(sink);\n}\n",
        Language::JavaScript,
    );
    let ts = parsed(
        "app.ts",
        "async function f(p: Promise<string>) {\n    await p;\n    return p.then(sink);\n}\n",
        Language::TypeScript,
    );

    for file in [&js, &ts] {
        let constructs = warning_constructs(file);
        assert!(constructs.contains(&"JS/TS await".to_string()));
        assert!(constructs.contains(&"JS/TS Promise .then()".to_string()));
    }
}

#[test]
fn python_with_and_comprehension_emit_scope_honesty_warnings() {
    let file = parsed(
        "app.py",
        "def f(xs):\n    with open('x') as fh:\n        vals = [x for x in xs]\n    return vals\n",
        Language::Python,
    );

    let constructs = warning_constructs(&file);
    assert!(constructs.contains(&"Python with statement".to_string()));
    assert!(constructs.contains(&"Python comprehension binding".to_string()));
}

#[test]
fn duplicate_constructs_are_deduped_per_function() {
    let file = parsed(
        "app.js",
        "async function f(a, b) {\n    await a;\n    await b;\n}\n",
        Language::JavaScript,
    );

    let constructs = warning_constructs(&file);
    assert_eq!(constructs, vec!["JS/TS await".to_string()]);
}

#[test]
fn js_nested_callback_is_reported_without_leaking_inner_await() {
    let file = parsed(
        "app.js",
        "function f(items) {\n    items.forEach(async item => { await sink(item); });\n}\n",
        Language::JavaScript,
    );

    let constructs = warning_constructs(&file);
    assert!(constructs.contains(&"JS/TS callback".to_string()));
    assert!(!constructs.contains(&"JS/TS await".to_string()));
}

#[test]
fn go_func_literal_body_constructs_are_not_attributed_to_outer_function() {
    let file = parsed(
        "main.go",
        "func f(ch chan string) {\n    cb := func() { select { case v := <-ch: sink(v) } }\n    _ = cb\n}\n",
        Language::Go,
    );

    let constructs = warning_constructs(&file);
    assert!(constructs.contains(&"Go nested callable".to_string()));
    assert!(!constructs.contains(&"Go select".to_string()));
    assert!(!constructs.contains(&"Go channel receive".to_string()));
}

#[test]
fn nested_callable_boundaries_emit_warnings_even_without_language_constructs() {
    let js = parsed(
        "app.js",
        "function outer(x) {\n    function cb() { sink(x); }\n    function* gen() { yield x; }\n}\n",
        Language::JavaScript,
    );
    let py = parsed(
        "app.py",
        "def outer(x):\n    cb = lambda y: sink(y)\n    return cb(x)\n",
        Language::Python,
    );

    assert!(warning_constructs(&js).contains(&"JS/TS nested callable".to_string()));
    assert!(warning_constructs(&py).contains(&"Python nested callable".to_string()));
}

#[test]
fn anonymous_callable_scopes_do_not_leak_inner_constructs_to_outer_function() {
    let js = parsed(
        "app.js",
        "function outer(x) {\n    const gen = function* () { yield x; };\n}\n",
        Language::JavaScript,
    );
    let cpp = parsed(
        "app.cpp",
        "void outer() {\n    auto cb = [](){ return 1; };\n}\n",
        Language::Cpp,
    );
    let rust = parsed(
        "src/lib.rs",
        "fn outer() {\n    let fut = async { work(); };\n}\n",
        Language::Rust,
    );

    let js_constructs = warning_constructs(&js);
    assert!(js_constructs.contains(&"JS/TS nested callable".to_string()));
    assert!(!js_constructs.contains(&"JS/TS yield".to_string()));
    assert!(warning_constructs(&cpp).contains(&"C++ nested callable".to_string()));
    assert!(warning_constructs(&rust).contains(&"Rust nested callable".to_string()));
}

#[test]
fn python_decorated_function_warns_for_decorator_and_body_constructs() {
    let file = parsed(
        "app.py",
        "@route('/x')\ndef f(xs):\n    with open('x') as fh:\n        return [x for x in xs]\n",
        Language::Python,
    );
    let function = file
        .all_functions()
        .into_iter()
        .find(|node| node.kind() == "decorated_definition")
        .unwrap();

    let constructs = constructs_from_warnings(warnings_for_function(&file, function));
    assert!(constructs.contains(&"Python decorator".to_string()));
    assert!(constructs.contains(&"Python with statement".to_string()));
    assert!(constructs.contains(&"Python comprehension binding".to_string()));
}

#[test]
fn python_decorated_function_entry_points_use_the_wrapper_consistently() {
    let file = parsed(
        "app.py",
        "@route('/x')\ndef f():\n    user = input()\n    with open(user) as fh:\n        sink(user)\n",
        Language::Python,
    );
    let mut files = std::collections::BTreeMap::new();
    files.insert("app.py".to_string(), file);
    let sink_constructs = constructs_from_warnings(warnings_for_sink_location(&files, "app.py", 5));
    assert!(sink_constructs.contains(&"Python decorator".to_string()));
    assert!(sink_constructs.contains(&"Python with statement".to_string()));

    let cpg = CodePropertyGraph::build(&files);
    let trace = cpg.taint_trace(&[("app.py".to_string(), 3)]);
    let trace_constructs = constructs_from_warnings(warnings_for_trace(&files, &cpg, &trace));
    assert_eq!(
        trace_constructs
            .iter()
            .filter(|construct| construct.as_str() == "Python decorator")
            .count(),
        1
    );
    assert_eq!(
        trace_constructs
            .iter()
            .filter(|construct| construct.as_str() == "Python with statement")
            .count(),
        1
    );
}

#[test]
fn js_promise_handler_detection_uses_direct_callee_property() {
    let overmatch = parsed(
        "app.js",
        "function f(obj) {\n    return obj.then.foo();\n}\n",
        Language::JavaScript,
    );
    let direct = parsed(
        "app.js",
        "function f(obj) {\n    obj.pipeline.then(sink);\n    obj?.then(sink);\n    obj.catch(log);\n    obj.finally(cleanup);\n}\n",
        Language::JavaScript,
    );

    assert!(!warning_constructs(&overmatch).contains(&"JS/TS Promise .then()".to_string()));
    let constructs = warning_constructs(&direct);
    assert!(constructs.contains(&"JS/TS Promise .then()".to_string()));
    assert!(constructs.contains(&"JS/TS Promise .catch()".to_string()));
    assert!(constructs.contains(&"JS/TS Promise .finally()".to_string()));
}

#[test]
fn python_and_js_yield_emit_scope_honesty_warnings() {
    let py = parsed(
        "app.py",
        "def f(xs):\n    for x in xs:\n        yield x\n",
        Language::Python,
    );
    let js = parsed(
        "app.js",
        "function* f(xs) {\n    yield xs.next();\n}\n",
        Language::JavaScript,
    );

    assert!(warning_constructs(&py).contains(&"Python yield".to_string()));
    assert!(warning_constructs(&js).contains(&"JS/TS yield".to_string()));
}

#[test]
fn sink_location_warnings_are_scoped_to_the_sink_function() {
    let file = parsed(
        "app.js",
        "async function f() {\n    await work();\n    sink();\n}\n",
        Language::JavaScript,
    );
    let mut files = std::collections::BTreeMap::new();
    files.insert("app.js".to_string(), file);

    let constructs = constructs_from_warnings(warnings_for_sink_location(&files, "app.js", 3));
    assert_eq!(constructs, vec!["JS/TS await".to_string()]);
}

#[test]
fn sink_location_on_same_line_unions_all_candidate_function_warnings() {
    let file = parsed(
        "app.js",
        "async function f(user) { await user; const cb = () => {}; sink(user); }\n",
        Language::JavaScript,
    );
    let mut files = std::collections::BTreeMap::new();
    files.insert("app.js".to_string(), file);

    let constructs = constructs_from_warnings(warnings_for_sink_location(&files, "app.js", 1));
    assert!(constructs.contains(&"JS/TS await".to_string()));
    assert!(constructs.contains(&"JS/TS callback".to_string()));
}

#[test]
fn trace_scope_honesty_uses_traversed_functions() {
    let file = parsed(
        "app.js",
        "async function f() {\n    const user = input();\n    await user;\n    sink(user);\n}\n",
        Language::JavaScript,
    );
    let mut files = std::collections::BTreeMap::new();
    files.insert("app.js".to_string(), file);
    let cpg = CodePropertyGraph::build(&files);
    let trace = cpg.taint_trace(&[("app.js".to_string(), 2)]);

    let constructs = constructs_from_warnings(warnings_for_trace(&files, &cpg, &trace));
    assert_eq!(constructs, vec!["JS/TS await".to_string()]);
}

#[test]
fn trace_scope_honesty_uses_cpg_function_identity_on_same_line_code() {
    let file = parsed(
        "app.js",
        "async function outer() { const cb = () => { const nested = 1; }; const user = input(); await user; sink(user); }\n",
        Language::JavaScript,
    );
    let mut files = std::collections::BTreeMap::new();
    files.insert("app.js".to_string(), file);
    let cpg = CodePropertyGraph::build(&files);
    let outer_user_nodes: std::collections::BTreeSet<_> = cpg
        .nodes_at("app.js", 1)
        .into_iter()
        .filter(|&idx| {
            cpg.to_var_location(idx).is_some_and(|loc| {
                loc.function == "outer" && loc.function_start_line == 1 && loc.var_name() == "user"
            })
        })
        .collect();
    let root = *outer_user_nodes.iter().next().unwrap();
    let mut trace = Trace::default();
    trace.frontier_by_root.insert(root, outer_user_nodes);

    let constructs = constructs_from_warnings(warnings_for_trace(&files, &cpg, &trace));
    assert!(constructs.contains(&"JS/TS await".to_string()));
    assert!(constructs.contains(&"JS/TS callback".to_string()));
}

#[test]
fn trace_scope_honesty_disambiguates_same_name_same_line_functions_by_var_byte_span() {
    let file = parsed(
        "app.js",
        "function f() { async function f() { const x = input(); await x; sink(x); } }\n",
        Language::JavaScript,
    );
    let mut files = std::collections::BTreeMap::new();
    files.insert("app.js".to_string(), file);
    let cpg = CodePropertyGraph::build(&files);
    let inner_x_nodes: std::collections::BTreeSet<_> = cpg
        .nodes_at("app.js", 1)
        .into_iter()
        .filter(|&idx| {
            cpg.to_var_location(idx).is_some_and(|loc| {
                loc.function == "f" && loc.function_start_line == 1 && loc.var_name() == "x"
            })
        })
        .collect();
    let root = *inner_x_nodes.iter().next().unwrap();
    let mut trace = Trace::default();
    trace.frontier_by_root.insert(root, inner_x_nodes);

    let constructs = constructs_from_warnings(warnings_for_trace(&files, &cpg, &trace));
    assert!(constructs.contains(&"JS/TS await".to_string()));
}
