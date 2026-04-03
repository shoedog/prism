#[path = "../../common/mod.rs"]
mod common;
use common::*;

// ====== Arrow Function & Anonymous Function Naming Tests ======

#[test]
fn test_arrow_const_assignment_named() {
    // Pattern 1: const X = () => {}
    let source = r#"
const fetchData = () => {
    const result = getData();
    return result;
};

function getData() {
    return 42;
}
"#;
    let path = "arrow.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let call_graph = CallGraph::build(&files);
    let func_names: Vec<&str> = call_graph.functions.keys().map(|s| s.as_str()).collect();

    assert!(
        func_names.contains(&"fetchData"),
        "Arrow function assigned to const should be named 'fetchData', got: {func_names:?}"
    );

    // Verify calls inside the arrow function are detected
    let callee_names: Vec<String> = call_graph
        .calls
        .values()
        .flat_map(|sites| sites.iter().map(|s| s.callee_name.clone()))
        .collect();

    assert!(
        callee_names.contains(&"getData".to_string()),
        "Calls inside arrow function should be detected, got: {callee_names:?}"
    );
}

#[test]
fn test_arrow_const_assignment_typescript() {
    // Pattern 1 in TypeScript with type annotation
    let source = r#"
const processData: (x: number) => number = (x) => {
    return compute(x);
};

function compute(x: number): number {
    return x * 2;
}
"#;
    let path = "arrow.ts";
    let parsed = ParsedFile::parse(path, source, Language::TypeScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let call_graph = CallGraph::build(&files);
    let func_names: Vec<&str> = call_graph.functions.keys().map(|s| s.as_str()).collect();

    assert!(
        func_names.contains(&"processData"),
        "TS arrow function with type annotation should be named, got: {func_names:?}"
    );
}

#[test]
fn test_anonymous_function_expression_named() {
    // Pattern 1 with function expression
    let source = r#"
const handler = function() {
    return process();
};

function process() {
    return 1;
}
"#;
    let path = "anon.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let call_graph = CallGraph::build(&files);
    let func_names: Vec<&str> = call_graph.functions.keys().map(|s| s.as_str()).collect();

    assert!(
        func_names.contains(&"handler"),
        "Anonymous function expression should be named 'handler', got: {func_names:?}"
    );
}

#[test]
fn test_named_function_expression_preserves_own_name() {
    // Named function expressions should use their own name, not the variable name
    let source = r#"
const wrapper = function innerName() {
    return innerName();
};
"#;
    let path = "named_expr.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let call_graph = CallGraph::build(&files);
    let func_names: Vec<&str> = call_graph.functions.keys().map(|s| s.as_str()).collect();

    assert!(
        func_names.contains(&"innerName"),
        "Named function expression should keep its own name, got: {func_names:?}"
    );
}

#[test]
fn test_object_property_arrow_named() {
    // Pattern 2: { key: () => {} }
    let source = r#"
const routes = {
    getUser: () => {
        return fetchUser();
    },
    deleteUser: () => {
        return removeUser();
    }
};

function fetchUser() { return null; }
function removeUser() { return null; }
"#;
    let path = "obj.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let call_graph = CallGraph::build(&files);
    let func_names: Vec<&str> = call_graph.functions.keys().map(|s| s.as_str()).collect();

    assert!(
        func_names.contains(&"getUser"),
        "Object property arrow should be named 'getUser', got: {func_names:?}"
    );
    assert!(
        func_names.contains(&"deleteUser"),
        "Object property arrow should be named 'deleteUser', got: {func_names:?}"
    );
}

#[test]
fn test_react_memo_wrapped_arrow_named() {
    // Pattern 3: const X = React.memo(() => {})
    let source = r#"
const MemoizedComponent = React.memo(() => {
    return render();
});

function render() { return null; }
"#;
    let path = "memo.tsx";
    let parsed = ParsedFile::parse(path, source, Language::Tsx).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let call_graph = CallGraph::build(&files);
    let func_names: Vec<&str> = call_graph.functions.keys().map(|s| s.as_str()).collect();

    assert!(
        func_names.contains(&"MemoizedComponent"),
        "React.memo wrapped arrow should be named 'MemoizedComponent', got: {func_names:?}"
    );
}

#[test]
fn test_class_field_arrow_named() {
    // Pattern 4: class Foo { handler = () => {} }
    let source = r#"
class EventManager {
    handleClick = () => {
        this.process();
    };

    process() {
        return true;
    }
}
"#;
    let path = "class.ts";
    let parsed = ParsedFile::parse(path, source, Language::TypeScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let call_graph = CallGraph::build(&files);
    let func_names: Vec<&str> = call_graph.functions.keys().map(|s| s.as_str()).collect();

    assert!(
        func_names.contains(&"handleClick"),
        "Class field arrow should be named 'handleClick', got: {func_names:?}"
    );
}

#[test]
fn test_exports_assignment_arrow_named() {
    // Pattern 5: exports.handler = () => {}
    let source = r#"
exports.handler = () => {
    return processRequest();
};

function processRequest() { return null; }
"#;
    let path = "exports.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let call_graph = CallGraph::build(&files);
    let func_names: Vec<&str> = call_graph.functions.keys().map(|s| s.as_str()).collect();

    assert!(
        func_names.contains(&"handler"),
        "exports.handler arrow should be named 'handler', got: {func_names:?}"
    );
}

#[test]
fn test_truly_anonymous_arrow_skipped() {
    // Truly anonymous: export default () => {}, callbacks in arguments
    let source = r#"
function main() {
    setTimeout(() => {
        console.log("timeout");
    }, 100);

    [1, 2, 3].forEach((x) => {
        console.log(x);
    });
}
"#;
    let path = "anon_callbacks.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let call_graph = CallGraph::build(&files);
    // main should be the only named function
    // The arrow callbacks should not appear as named functions
    assert!(
        call_graph.functions.contains_key("main"),
        "main should be in call graph"
    );
    // setTimeout and forEach callbacks are anonymous — they have parent "arguments"
    // pointing to setTimeout/forEach calls, not variable_declarators
    let func_names: Vec<&str> = call_graph.functions.keys().map(|s| s.as_str()).collect();
    assert!(
        !func_names
            .iter()
            .any(|n| n.starts_with("setTimeout") || n.starts_with("forEach")),
        "Callback-position arrows should not get caller's name, got: {func_names:?}"
    );
}

#[test]
fn test_nested_arrows_only_named_ones_appear() {
    // Nested arrow functions: only the ones with variable bindings get names
    let source = r#"
const App = () => {
    const handleClick = () => {
        submit();
    };
    const items = data.map(item => transform(item));
    return null;
};

function submit() {}
function transform(x) { return x; }
"#;
    let path = "nested.tsx";
    let parsed = ParsedFile::parse(path, source, Language::Tsx).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let call_graph = CallGraph::build(&files);
    let func_names: Vec<&str> = call_graph.functions.keys().map(|s| s.as_str()).collect();

    assert!(
        func_names.contains(&"App"),
        "Outer arrow should be named 'App', got: {func_names:?}"
    );
    assert!(
        func_names.contains(&"handleClick"),
        "Inner named arrow should be named 'handleClick', got: {func_names:?}"
    );
}

#[test]
fn test_arrow_in_tsx_component_call_graph() {
    // Verify arrow-function components in TSX have proper call graph edges
    let source = r#"
const UserCard = ({ name, avatar }) => {
    return <div><Avatar src={avatar} /><span>{name}</span></div>;
};

function Avatar({ src }) {
    return <img src={src} />;
}
"#;
    let path = "card.tsx";
    let parsed = ParsedFile::parse(path, source, Language::Tsx).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let call_graph = CallGraph::build(&files);

    // UserCard should be named and its JSX calls detected
    assert!(
        call_graph.functions.contains_key("UserCard"),
        "Arrow component should be named 'UserCard'"
    );

    let callee_names: Vec<String> = call_graph
        .calls
        .values()
        .flat_map(|sites| sites.iter().map(|s| s.callee_name.clone()))
        .collect();

    assert!(
        callee_names.contains(&"Avatar".to_string()),
        "JSX calls inside arrow component should be detected, got: {callee_names:?}"
    );
}
