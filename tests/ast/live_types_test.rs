// Tests for multi-language live type collection (Phase 7 of E12).
//
// Covers: Java, Go, TypeScript, JavaScript, Rust, Python, and C++ instantiation
// pattern detection, cross-language collection, CpgContext integration, and
// edge cases.

use prism::ast::ParsedFile;
use prism::languages::Language;
use prism::live_types::collect_live_types;
use std::collections::{BTreeMap, BTreeSet};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_file(path: &str, source: &str, lang: Language) -> BTreeMap<String, ParsedFile> {
    let parsed = ParsedFile::parse(path, source, lang).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    files
}

fn parse_files(sources: &[(&str, &str, Language)]) -> BTreeMap<String, ParsedFile> {
    let mut files = BTreeMap::new();
    for (path, source, lang) in sources {
        let parsed = ParsedFile::parse(path, source, *lang).unwrap();
        files.insert(path.to_string(), parsed);
    }
    files
}

// ---------------------------------------------------------------------------
// Java: new ClassName(...)
// ---------------------------------------------------------------------------

#[test]
fn test_java_new_expression() {
    let source = r#"
public class Main {
    public static void main(String[] args) {
        ArrayList<String> list = new ArrayList<String>();
        HashMap<String, Integer> map = new HashMap<>();
        Object obj = new Object();
    }
}
"#;
    let files = parse_file("Main.java", source, Language::Java);
    let live = collect_live_types(&files, &BTreeSet::new());

    assert!(
        live.contains("ArrayList"),
        "Expected ArrayList in {:?}",
        live
    );
    assert!(live.contains("HashMap"), "Expected HashMap in {:?}", live);
    assert!(live.contains("Object"), "Expected Object in {:?}", live);
}

#[test]
fn test_java_no_instantiation() {
    let source = r#"
public interface Repository {
    void save(Object item);
}
"#;
    let files = parse_file("Repo.java", source, Language::Java);
    let live = collect_live_types(&files, &BTreeSet::new());

    assert!(
        live.is_empty(),
        "Interface has no instantiations: {:?}",
        live
    );
}

// ---------------------------------------------------------------------------
// Go: StructName{...}
// ---------------------------------------------------------------------------

#[test]
fn test_go_composite_literal() {
    let source = r#"
package main

func main() {
    p := Point{X: 1, Y: 2}
    s := &Server{Port: 8080}
    c := Config{}
}
"#;
    let files = parse_file("main.go", source, Language::Go);
    let live = collect_live_types(&files, &BTreeSet::new());

    assert!(live.contains("Point"), "Expected Point in {:?}", live);
    assert!(live.contains("Server"), "Expected Server in {:?}", live);
    assert!(live.contains("Config"), "Expected Config in {:?}", live);
}

#[test]
fn test_go_ignores_lowercase() {
    // Go types start with uppercase; lowercase composite literals are
    // likely map/slice literals, not struct instantiations.
    let source = r#"
package main

func main() {
    m := map[string]int{"a": 1}
}
"#;
    let files = parse_file("main.go", source, Language::Go);
    let live = collect_live_types(&files, &BTreeSet::new());

    // map literal shouldn't produce a live type.
    assert!(
        !live.iter().any(|t| t == "map"),
        "map should not be a live type: {:?}",
        live
    );
}

// ---------------------------------------------------------------------------
// TypeScript: new ClassName(...)
// ---------------------------------------------------------------------------

#[test]
fn test_typescript_new_expression() {
    let source = r#"
const list = new Array<string>();
const map = new Map<string, number>();
const svc = new UserService();
"#;
    let files = parse_file("app.ts", source, Language::TypeScript);
    let live = collect_live_types(&files, &BTreeSet::new());

    assert!(live.contains("Array"), "Expected Array in {:?}", live);
    assert!(live.contains("Map"), "Expected Map in {:?}", live);
    assert!(
        live.contains("UserService"),
        "Expected UserService in {:?}",
        live
    );
}

#[test]
fn test_tsx_new_expression() {
    let source = r#"
const ref = new React.createRef<HTMLDivElement>();
const controller = new AbortController();
"#;
    let files = parse_file("app.tsx", source, Language::Tsx);
    let live = collect_live_types(&files, &BTreeSet::new());

    assert!(
        live.contains("AbortController"),
        "Expected AbortController in {:?}",
        live
    );
}

// ---------------------------------------------------------------------------
// JavaScript: new ClassName(...)
// ---------------------------------------------------------------------------

#[test]
fn test_javascript_new_expression() {
    let source = r#"
const date = new Date();
const err = new Error("oops");
const worker = new Worker("./worker.js");
"#;
    let files = parse_file("app.js", source, Language::JavaScript);
    let live = collect_live_types(&files, &BTreeSet::new());

    assert!(live.contains("Date"), "Expected Date in {:?}", live);
    assert!(live.contains("Error"), "Expected Error in {:?}", live);
    assert!(live.contains("Worker"), "Expected Worker in {:?}", live);
}

// ---------------------------------------------------------------------------
// Rust: StructName { ... }
// ---------------------------------------------------------------------------

#[test]
fn test_rust_struct_expression() {
    let source = r#"
struct Config {
    host: String,
    port: u16,
}

fn main() {
    let cfg = Config { host: "localhost".to_string(), port: 8080 };
}
"#;
    let files = parse_file("main.rs", source, Language::Rust);
    let live = collect_live_types(&files, &BTreeSet::new());

    assert!(live.contains("Config"), "Expected Config in {:?}", live);
}

#[test]
fn test_rust_path_qualified_struct() {
    let source = r#"
mod inner {
    pub struct Inner {
        pub value: i32,
    }
}

fn main() {
    let x = inner::Inner { value: 42 };
}
"#;
    let files = parse_file("qualified.rs", source, Language::Rust);
    let live = collect_live_types(&files, &BTreeSet::new());

    assert!(live.contains("Inner"), "Expected Inner in {:?}", live);
}

// ---------------------------------------------------------------------------
// Python: ClassName(...) with known_classes
// ---------------------------------------------------------------------------

#[test]
fn test_python_class_instantiation() {
    let source = r#"
class User:
    name: str

class Config:
    debug: bool

def main():
    u = User()
    c = Config()
    result = some_function()
"#;
    let files = parse_file("app.py", source, Language::Python);
    let known = BTreeSet::from(["User".to_string(), "Config".to_string()]);
    let live = collect_live_types(&files, &known);

    assert!(live.contains("User"), "Expected User in {:?}", live);
    assert!(live.contains("Config"), "Expected Config in {:?}", live);
    // some_function is not a known class, so it shouldn't be in live.
    assert!(
        !live.contains("some_function"),
        "Function call should not be a live type: {:?}",
        live
    );
}

#[test]
fn test_python_without_known_classes() {
    // Without known_classes, Python call expressions are not counted.
    let source = r#"
class Foo:
    pass

x = Foo()
"#;
    let files = parse_file("foo.py", source, Language::Python);
    let live = collect_live_types(&files, &BTreeSet::new());

    assert!(
        !live.contains("Foo"),
        "Without known_classes, Foo() should not be detected: {:?}",
        live
    );
}

// ---------------------------------------------------------------------------
// C++: new, make_unique, make_shared, stack allocation
// ---------------------------------------------------------------------------

#[test]
fn test_cpp_new_expression() {
    let source = r#"
class Shape {};
class Circle : public Shape {};

int main() {
    Shape* s = new Circle();
    return 0;
}
"#;
    let files = parse_file("main.cpp", source, Language::Cpp);
    let live = collect_live_types(&files, &BTreeSet::new());

    assert!(live.contains("Circle"), "Expected Circle in {:?}", live);
}

#[test]
fn test_cpp_stack_allocation() {
    let source = r#"
class Widget {};

int main() {
    Widget w;
    return 0;
}
"#;
    let files = parse_file("main.cpp", source, Language::Cpp);
    let live = collect_live_types(&files, &BTreeSet::new());

    assert!(live.contains("Widget"), "Expected Widget in {:?}", live);
}

// ---------------------------------------------------------------------------
// Cross-language collection
// ---------------------------------------------------------------------------

#[test]
fn test_cross_language_collection() {
    let sources = &[
        (
            "Main.java",
            "class Main { void f() { new ArrayList(); } }",
            Language::Java,
        ),
        (
            "main.go",
            "package main\nfunc f() { p := Point{} }",
            Language::Go,
        ),
        ("app.ts", "const s = new Service();", Language::TypeScript),
    ];
    let files = parse_files(sources);
    let live = collect_live_types(&files, &BTreeSet::new());

    assert!(live.contains("ArrayList"), "Java: {:?}", live);
    assert!(live.contains("Point"), "Go: {:?}", live);
    assert!(live.contains("Service"), "TS: {:?}", live);
}

// ---------------------------------------------------------------------------
// CpgContext integration
// ---------------------------------------------------------------------------

#[test]
fn test_cpg_context_live_types() {
    use prism::cpg::CpgContext;

    let java_source = r#"
public class App {
    public static void main(String[] args) {
        UserService svc = new UserService();
    }
}
"#;
    let go_source = r#"
package main

func main() {
    cfg := Config{Port: 8080}
}
"#;
    let java_parsed = ParsedFile::parse("App.java", java_source, Language::Java).unwrap();
    let go_parsed = ParsedFile::parse("main.go", go_source, Language::Go).unwrap();

    let mut files = BTreeMap::new();
    files.insert("App.java".to_string(), java_parsed);
    files.insert("main.go".to_string(), go_parsed);

    let ctx = CpgContext::build(&files, None);

    assert!(
        ctx.live_types.contains("UserService"),
        "Java live type: {:?}",
        ctx.live_types
    );
    assert!(
        ctx.live_types.contains("Config"),
        "Go live type: {:?}",
        ctx.live_types
    );
}

#[test]
fn test_cpg_context_python_live_types() {
    use prism::cpg::CpgContext;

    let source = r#"
class Handler:
    def handle(self) -> None:
        pass

def main():
    h = Handler()
"#;
    let parsed = ParsedFile::parse("app.py", source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert("app.py".to_string(), parsed);

    let ctx = CpgContext::build(&files, None);

    // CpgContext.collect_live_types should automatically detect Python class
    // names and pass them as known_classes for Python scanning.
    assert!(
        ctx.live_types.contains("Handler"),
        "Python live type: {:?}",
        ctx.live_types
    );
}

// ---------------------------------------------------------------------------
// TypeRegistry.collect_live_types integration
// ---------------------------------------------------------------------------

#[test]
fn test_registry_collect_live_types() {
    use prism::type_provider::TypeRegistry;
    use prism::type_providers::python::PythonTypeProvider;

    let source = r#"
class Worker:
    def run(self) -> None:
        pass

def main():
    w = Worker()
"#;
    let files = parse_file("worker.py", source, Language::Python);
    let provider = PythonTypeProvider::from_parsed_files(&files);

    let mut registry = TypeRegistry::empty();
    registry.register_provider(Box::new(provider));

    let live = registry.collect_live_types(&files);
    assert!(
        live.contains("Worker"),
        "Registry should collect Python live types: {:?}",
        live
    );
}

// ---------------------------------------------------------------------------
// Edge cases
// ---------------------------------------------------------------------------

#[test]
fn test_no_files() {
    let files = BTreeMap::new();
    let live = collect_live_types(&files, &BTreeSet::new());
    assert!(live.is_empty());
}

#[test]
fn test_languages_without_types() {
    let lua_source = "print('hello')\n";
    let files = parse_file("main.lua", lua_source, Language::Lua);
    let live = collect_live_types(&files, &BTreeSet::new());
    assert!(
        live.is_empty(),
        "Lua has no type instantiations: {:?}",
        live
    );
}

#[test]
fn test_bash_ignored() {
    let source = "#!/bin/bash\necho hello\n";
    let files = parse_file("script.sh", source, Language::Bash);
    let live = collect_live_types(&files, &BTreeSet::new());
    assert!(live.is_empty());
}

#[test]
fn test_python_dotted_class_call() {
    let source = r#"
import models

def create():
    u = models.User()
"#;
    let files = parse_file("create.py", source, Language::Python);
    let known = BTreeSet::from(["User".to_string()]);
    let live = collect_live_types(&files, &known);

    // models.User() — the base name "User" matches known_classes.
    assert!(
        live.contains("User"),
        "Dotted call should resolve: {:?}",
        live
    );
}
