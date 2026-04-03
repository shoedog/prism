#[path = "../../common/mod.rs"]
mod common;
use common::*;

fn all_callee_names(call_graph: &CallGraph) -> Vec<String> {
    call_graph
        .calls
        .values()
        .flat_map(|sites| sites.iter().map(|s| s.callee_name.clone()))
        .collect()
}

// ====== JSX Call Graph Integration Tests ======

#[test]
fn test_jsx_component_in_call_graph() {
    let (files, _, _) = make_tsx_test();
    let call_graph = CallGraph::build(&files);
    let names = all_callee_names(&call_graph);

    assert!(
        names.iter().any(|n| n == "Spinner"),
        "Call graph should contain Spinner component usage, got: {names:?}"
    );
    assert!(
        names.iter().any(|n| n == "Avatar"),
        "Call graph should contain Avatar component usage, got: {names:?}"
    );
    assert!(
        names.iter().any(|n| n == "ContactList"),
        "Call graph should contain ContactList component usage, got: {names:?}"
    );
}

#[test]
fn test_jsx_regular_calls_still_work() {
    let (files, _, _) = make_tsx_test();
    let call_graph = CallGraph::build(&files);
    let names = all_callee_names(&call_graph);

    assert!(
        names.iter().any(|n| n == "fetchUser"),
        "Call graph should contain regular function calls, got: {names:?}"
    );
}

#[test]
fn test_jsx_self_closing_element_is_call() {
    let source = r#"
function App() {
    return <Button onClick={handler} />;
}
"#;
    let path = "App.tsx";
    let parsed = ParsedFile::parse(path, source, Language::Tsx).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let call_graph = CallGraph::build(&files);
    let names = all_callee_names(&call_graph);

    assert!(
        names.iter().any(|n| n == "Button"),
        "Self-closing JSX element should be a call, got: {names:?}"
    );
}

#[test]
fn test_jsx_opening_element_is_call() {
    let source = r#"
function App() {
    return <Container><Child /></Container>;
}
"#;
    let path = "App.tsx";
    let parsed = ParsedFile::parse(path, source, Language::Tsx).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let call_graph = CallGraph::build(&files);
    let names = all_callee_names(&call_graph);

    assert!(
        names.iter().any(|n| n == "Container"),
        "Opening JSX element should be a call, got: {names:?}"
    );
    assert!(
        names.iter().any(|n| n == "Child"),
        "Nested JSX self-closing element should be a call, got: {names:?}"
    );
}

#[test]
fn test_jsx_member_expression_component() {
    let source = r#"
function App() {
    return <Icons.Star size={24} />;
}
"#;
    let path = "App.tsx";
    let parsed = ParsedFile::parse(path, source, Language::Tsx).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let call_graph = CallGraph::build(&files);
    let names = all_callee_names(&call_graph);

    assert!(
        names.iter().any(|n| n == "Star"),
        "Member expression JSX component should extract property name, got: {names:?}"
    );
}

#[test]
fn test_jsx_html_intrinsics_appear_as_calls() {
    let source = r#"
function App() {
    return <div><span>hello</span></div>;
}
"#;
    let path = "App.tsx";
    let parsed = ParsedFile::parse(path, source, Language::Tsx).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let call_graph = CallGraph::build(&files);
    let names = all_callee_names(&call_graph);

    assert!(
        names.iter().any(|n| n == "div"),
        "HTML intrinsic div should appear in call graph, got: {names:?}"
    );
    assert!(
        names.iter().any(|n| n == "span"),
        "HTML intrinsic span should appear in call graph, got: {names:?}"
    );
}

#[test]
fn test_jsx_also_works_in_javascript() {
    let source = r#"
function App() {
    return <Header title="Hello" />;
}
"#;
    let path = "App.jsx";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let call_graph = CallGraph::build(&files);
    let names = all_callee_names(&call_graph);

    assert!(
        names.iter().any(|n| n == "Header"),
        "JSX in .jsx should produce call graph edges, got: {names:?}"
    );
}
