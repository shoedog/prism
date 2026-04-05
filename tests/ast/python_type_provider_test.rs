// Tests for the Python type provider (Phase 6 of E12).
//
// Covers: class extraction, function extraction, type annotations, type aliases,
// inheritance, dataclass detection, decorated methods, field_layout with
// inheritance, subtypes_of, Arc sharing, multi-file resolution, CpgContext
// integration, and cycle protection.

use prism::ast::ParsedFile;
use prism::languages::Language;
use prism::type_provider::{ResolvedTypeKind, TypeProvider};
use prism::type_providers::python::PythonTypeProvider;
use std::collections::BTreeMap;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_python(path: &str, source: &str) -> BTreeMap<String, ParsedFile> {
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    files
}

fn parse_python_files(sources: &[(&str, &str)]) -> BTreeMap<String, ParsedFile> {
    let mut files = BTreeMap::new();
    for (path, source) in sources {
        let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
        files.insert(path.to_string(), parsed);
    }
    files
}

// ---------------------------------------------------------------------------
// Class extraction
// ---------------------------------------------------------------------------

#[test]
fn test_python_class_extraction() {
    let source = r#"
class User:
    name: str
    age: int

    def greet(self) -> str:
        return f"Hello, {self.name}"
"#;
    let files = parse_python("user.py", source);
    let provider = PythonTypeProvider::from_parsed_files(&files);

    let resolved = provider.resolve_type("user.py", "User", 0);
    assert!(resolved.is_some());
    let rt = resolved.unwrap();
    assert_eq!(rt.name, "User");
    assert_eq!(rt.kind, ResolvedTypeKind::Concrete);
}

#[test]
fn test_python_class_field_layout() {
    let source = r#"
class Config:
    host: str
    port: int
    debug: bool

    def validate(self) -> bool:
        return True
"#;
    let files = parse_python("config.py", source);
    let provider = PythonTypeProvider::from_parsed_files(&files);

    let fields = provider.field_layout("Config").unwrap();
    let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
    assert!(names.contains(&"host"), "Expected 'host' in {:?}", names);
    assert!(names.contains(&"port"), "Expected 'port' in {:?}", names);
    assert!(names.contains(&"debug"), "Expected 'debug' in {:?}", names);
    assert!(
        names.contains(&"validate"),
        "Expected 'validate' in {:?}",
        names
    );
}

#[test]
fn test_python_class_with_bases() {
    let source = r#"
class Animal:
    name: str

    def speak(self) -> str:
        return ""

class Dog(Animal):
    breed: str

    def speak(self) -> str:
        return "Woof"
"#;
    let files = parse_python("animals.py", source);
    let provider = PythonTypeProvider::from_parsed_files(&files);

    assert!(provider.resolve_type("animals.py", "Animal", 0).is_some());
    assert!(provider.resolve_type("animals.py", "Dog", 0).is_some());

    // Dog should be a subtype of Animal.
    let subtypes = provider.subtypes_of("Animal");
    assert!(
        subtypes.contains(&"Dog".to_string()),
        "Expected Dog in subtypes: {:?}",
        subtypes
    );
}

// ---------------------------------------------------------------------------
// Inheritance and field_layout
// ---------------------------------------------------------------------------

#[test]
fn test_python_inherited_fields() {
    let source = r#"
class Base:
    x: int

    def base_method(self) -> None:
        pass

class Child(Base):
    y: int

    def child_method(self) -> None:
        pass
"#;
    let files = parse_python("inherit.py", source);
    let provider = PythonTypeProvider::from_parsed_files(&files);

    let fields = provider.field_layout("Child").unwrap();
    let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
    assert!(
        names.contains(&"x"),
        "Expected inherited attr 'x' in {:?}",
        names
    );
    assert!(names.contains(&"y"), "Expected own attr 'y' in {:?}", names);
    assert!(
        names.contains(&"base_method"),
        "Expected inherited method in {:?}",
        names
    );
    assert!(
        names.contains(&"child_method"),
        "Expected own method in {:?}",
        names
    );
}

#[test]
fn test_python_method_override() {
    let source = r#"
class Base:
    def run(self) -> str:
        return "base"

class Child(Base):
    def run(self) -> str:
        return "child"
"#;
    let files = parse_python("override.py", source);
    let provider = PythonTypeProvider::from_parsed_files(&files);

    let fields = provider.field_layout("Child").unwrap();
    let run_methods: Vec<_> = fields.iter().filter(|f| f.name == "run").collect();
    assert_eq!(
        run_methods.len(),
        1,
        "Overridden method should appear once: {:?}",
        run_methods
    );
}

// ---------------------------------------------------------------------------
// Dataclass detection
// ---------------------------------------------------------------------------

#[test]
fn test_python_dataclass() {
    let source = r#"
from dataclasses import dataclass

@dataclass
class Point:
    x: float
    y: float
"#;
    let files = parse_python("point.py", source);
    let provider = PythonTypeProvider::from_parsed_files(&files);

    let resolved = provider.resolve_type("point.py", "Point", 0);
    assert!(resolved.is_some());
    assert_eq!(resolved.unwrap().kind, ResolvedTypeKind::Concrete);

    let fields = provider.field_layout("Point").unwrap();
    let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
    assert!(names.contains(&"x"), "Expected 'x' in {:?}", names);
    assert!(names.contains(&"y"), "Expected 'y' in {:?}", names);
}

#[test]
fn test_python_dataclass_with_args() {
    let source = r#"
from dataclasses import dataclass

@dataclass(frozen=True)
class Immutable:
    value: int
"#;
    let files = parse_python("frozen.py", source);
    let provider = PythonTypeProvider::from_parsed_files(&files);

    assert!(provider.resolve_type("frozen.py", "Immutable", 0).is_some());
}

// ---------------------------------------------------------------------------
// Function extraction
// ---------------------------------------------------------------------------

#[test]
fn test_python_function_extraction() {
    let source = r#"
def greet(name: str, age: int = 0) -> str:
    return f"Hello {name}, you are {age}"
"#;
    let files = parse_python("greet.py", source);
    let provider = PythonTypeProvider::from_parsed_files(&files);

    // Functions are not resolvable as types — they are stored internally
    // but resolve_type only handles classes and aliases.
    assert!(provider.resolve_type("greet.py", "greet", 0).is_none());
}

// ---------------------------------------------------------------------------
// Type aliases
// ---------------------------------------------------------------------------

#[test]
fn test_python_type_alias_explicit() {
    let source = r#"
from typing import TypeAlias

UserId: TypeAlias = int
"#;
    let files = parse_python("aliases.py", source);
    let provider = PythonTypeProvider::from_parsed_files(&files);

    let resolved = provider.resolve_type("aliases.py", "UserId", 0);
    assert!(resolved.is_some());
    assert_eq!(resolved.unwrap().kind, ResolvedTypeKind::Alias);

    assert_eq!(provider.resolve_alias("UserId"), "int");
}

#[test]
fn test_python_type_alias_heuristic() {
    let source = r#"
UserList = List
"#;
    let files = parse_python("aliases2.py", source);
    let provider = PythonTypeProvider::from_parsed_files(&files);

    let resolved = provider.resolve_type("aliases2.py", "UserList", 0);
    assert!(resolved.is_some());
    assert_eq!(resolved.unwrap().kind, ResolvedTypeKind::Alias);
    assert_eq!(provider.resolve_alias("UserList"), "List");
}

#[test]
fn test_python_alias_non_matching() {
    // lowercase = lowercase should not be treated as a type alias.
    let source = r#"
count = 0
"#;
    let files = parse_python("noalias.py", source);
    let provider = PythonTypeProvider::from_parsed_files(&files);

    assert!(provider.resolve_type("noalias.py", "count", 0).is_none());
}

// ---------------------------------------------------------------------------
// Decorated methods (staticmethod, classmethod, property)
// ---------------------------------------------------------------------------

#[test]
fn test_python_decorated_methods() {
    let source = r#"
class Service:
    _instance: object

    @staticmethod
    def create() -> object:
        return object()

    @classmethod
    def get_instance(cls) -> object:
        return cls._instance

    @property
    def name(self) -> str:
        return "service"

    def run(self) -> None:
        pass
"#;
    let files = parse_python("service.py", source);
    let provider = PythonTypeProvider::from_parsed_files(&files);

    let fields = provider.field_layout("Service").unwrap();
    let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
    assert!(
        names.contains(&"create"),
        "Expected @staticmethod 'create' in {:?}",
        names
    );
    assert!(
        names.contains(&"get_instance"),
        "Expected @classmethod 'get_instance' in {:?}",
        names
    );
    assert!(
        names.contains(&"name"),
        "Expected @property 'name' in {:?}",
        names
    );
    assert!(
        names.contains(&"run"),
        "Expected regular method 'run' in {:?}",
        names
    );
}

// ---------------------------------------------------------------------------
// Dunder methods excluded from field_layout
// ---------------------------------------------------------------------------

#[test]
fn test_python_dunder_methods_excluded() {
    let source = r#"
class Item:
    value: int

    def __init__(self, value: int) -> None:
        self.value = value

    def __repr__(self) -> str:
        return f"Item({self.value})"

    def display(self) -> str:
        return str(self.value)
"#;
    let files = parse_python("item.py", source);
    let provider = PythonTypeProvider::from_parsed_files(&files);

    let fields = provider.field_layout("Item").unwrap();
    let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
    assert!(
        !names.contains(&"__init__"),
        "Dunder __init__ should be excluded"
    );
    assert!(
        !names.contains(&"__repr__"),
        "Dunder __repr__ should be excluded"
    );
    assert!(
        names.contains(&"display"),
        "Regular method should be included"
    );
    assert!(names.contains(&"value"), "Attribute should be included");
}

// ---------------------------------------------------------------------------
// Unknown types
// ---------------------------------------------------------------------------

#[test]
fn test_python_unknown_type() {
    let source = "x = 42";
    let files = parse_python("unknown.py", source);
    let provider = PythonTypeProvider::from_parsed_files(&files);

    assert!(provider
        .resolve_type("unknown.py", "NonExistent", 0)
        .is_none());
    assert!(provider.field_layout("NonExistent").is_none());
    assert!(provider.subtypes_of("NonExistent").is_empty());
    assert_eq!(provider.resolve_alias("NonExistent"), "NonExistent");
}

// ---------------------------------------------------------------------------
// Multi-file
// ---------------------------------------------------------------------------

#[test]
fn test_python_multi_file() {
    let sources = &[
        (
            "base.py",
            r#"
class Base:
    def run(self) -> None:
        pass
"#,
        ),
        (
            "impl_a.py",
            r#"
class ImplA(Base):
    def run(self) -> None:
        print("A")
"#,
        ),
        (
            "impl_b.py",
            r#"
class ImplB(Base):
    def run(self) -> None:
        print("B")
"#,
        ),
    ];
    let files = parse_python_files(sources);
    let provider = PythonTypeProvider::from_parsed_files(&files);

    let subtypes = provider.subtypes_of("Base");
    assert!(subtypes.contains(&"ImplA".to_string()));
    assert!(subtypes.contains(&"ImplB".to_string()));
    assert_eq!(subtypes.len(), 2);
}

// ---------------------------------------------------------------------------
// Arc sharing
// ---------------------------------------------------------------------------

#[test]
fn test_python_arc_sharing() {
    let source = "class Foo:\n    pass\n";
    let files = parse_python("foo.py", source);
    let provider = PythonTypeProvider::from_parsed_files(&files);
    let cloned = provider.clone();

    assert!(provider.resolve_type("foo.py", "Foo", 0).is_some());
    assert!(cloned.resolve_type("foo.py", "Foo", 0).is_some());
    assert!(std::sync::Arc::ptr_eq(&provider.data, &cloned.data));
}

// ---------------------------------------------------------------------------
// Languages method
// ---------------------------------------------------------------------------

#[test]
fn test_python_provider_languages() {
    let files = parse_python("empty.py", "x = 1\n");
    let provider = PythonTypeProvider::from_parsed_files(&files);
    assert_eq!(provider.languages(), vec![Language::Python]);
}

// ---------------------------------------------------------------------------
// Non-Python files ignored
// ---------------------------------------------------------------------------

#[test]
fn test_python_ignores_non_python_files() {
    let py_parsed =
        ParsedFile::parse("app.py", "class App:\n    pass\n", Language::Python).unwrap();
    let go_parsed =
        ParsedFile::parse("main.go", "package main\nfunc main() {}\n", Language::Go).unwrap();

    let mut files = BTreeMap::new();
    files.insert("app.py".to_string(), py_parsed);
    files.insert("main.go".to_string(), go_parsed);

    let provider = PythonTypeProvider::from_parsed_files(&files);
    assert!(provider.resolve_type("app.py", "App", 0).is_some());
    assert!(provider.resolve_type("main.go", "main", 0).is_none());
}

// ---------------------------------------------------------------------------
// Registry integration
// ---------------------------------------------------------------------------

#[test]
fn test_python_registry_integration() {
    use prism::type_provider::TypeRegistry;

    let source = r#"
class Handler:
    def handle(self) -> None:
        pass

class EventHandler(Handler):
    def handle(self) -> None:
        print("event")
"#;
    let files = parse_python("handler.py", source);
    let provider = PythonTypeProvider::from_parsed_files(&files);

    let mut registry = TypeRegistry::empty();
    registry.register_provider(Box::new(provider));

    let tp = registry.provider_for(Language::Python);
    assert!(tp.is_some());
    assert_eq!(
        tp.unwrap()
            .resolve_type("handler.py", "Handler", 0)
            .unwrap()
            .kind,
        ResolvedTypeKind::Concrete
    );

    // No dispatch provider for Python (duck typing).
    assert!(registry.dispatch_for(Language::Python).is_none());
    // No structural provider for Python.
    assert!(registry.structural_for(Language::Python).is_none());
}

// ---------------------------------------------------------------------------
// CpgContext integration
// ---------------------------------------------------------------------------

#[test]
fn test_python_cpg_context_integration() {
    use prism::cpg::CpgContext;

    let source = r#"
class Logger:
    level: str

    def log(self, msg: str) -> None:
        print(msg)
"#;
    let parsed = ParsedFile::parse("logger.py", source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert("logger.py".to_string(), parsed);

    let ctx = CpgContext::build(&files, None);

    let tp = ctx.types.provider_for(Language::Python);
    assert!(tp.is_some(), "Python provider should be auto-registered");

    let tp = tp.unwrap();
    assert_eq!(
        tp.resolve_type("logger.py", "Logger", 0).unwrap().kind,
        ResolvedTypeKind::Concrete
    );

    // No dispatch for Python.
    assert!(ctx.types.dispatch_for(Language::Python).is_none());
}

// ---------------------------------------------------------------------------
// Cycle protection in inheritance
// ---------------------------------------------------------------------------

#[test]
fn test_python_cyclic_inheritance_no_overflow() {
    // Invalid Python, but tree-sitter parses it.
    let source = r#"
class A(B):
    x: int

class B(A):
    y: int
"#;
    let files = parse_python("cycle.py", source);
    let provider = PythonTypeProvider::from_parsed_files(&files);

    // Should not stack-overflow.
    let _ = provider.field_layout("A");
    let _ = provider.field_layout("B");
    let _ = provider.subtypes_of("A");
}

// ---------------------------------------------------------------------------
// Multiple inheritance
// ---------------------------------------------------------------------------

#[test]
fn test_python_multiple_inheritance() {
    let source = r#"
class Readable:
    def read(self) -> str:
        return ""

class Writable:
    def write(self, data: str) -> None:
        pass

class ReadWrite(Readable, Writable):
    pass
"#;
    let files = parse_python("multi_inherit.py", source);
    let provider = PythonTypeProvider::from_parsed_files(&files);

    let fields = provider.field_layout("ReadWrite").unwrap();
    let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
    assert!(
        names.contains(&"read"),
        "Expected inherited 'read' in {:?}",
        names
    );
    assert!(
        names.contains(&"write"),
        "Expected inherited 'write' in {:?}",
        names
    );

    // ReadWrite is a subtype of both.
    assert!(provider
        .subtypes_of("Readable")
        .contains(&"ReadWrite".to_string()));
    assert!(provider
        .subtypes_of("Writable")
        .contains(&"ReadWrite".to_string()));
}

// ---------------------------------------------------------------------------
// Empty class
// ---------------------------------------------------------------------------

#[test]
fn test_python_empty_class() {
    let source = r#"
class Empty:
    pass
"#;
    let files = parse_python("empty.py", source);
    let provider = PythonTypeProvider::from_parsed_files(&files);

    assert!(provider.resolve_type("empty.py", "Empty", 0).is_some());
    let fields = provider.field_layout("Empty").unwrap();
    assert!(fields.is_empty());
}

// ---------------------------------------------------------------------------
// Class with Protocol base (ABC-like)
// ---------------------------------------------------------------------------

#[test]
fn test_python_protocol_class() {
    let source = r#"
from typing import Protocol

class Renderable(Protocol):
    def render(self) -> str: ...

class HtmlRenderer:
    def render(self) -> str:
        return "<html/>"
"#;
    let files = parse_python("protocol.py", source);
    let provider = PythonTypeProvider::from_parsed_files(&files);

    // Protocol is extracted as a class with Protocol base.
    assert!(provider
        .resolve_type("protocol.py", "Renderable", 0)
        .is_some());
    assert!(provider
        .resolve_type("protocol.py", "HtmlRenderer", 0)
        .is_some());
}

// ---------------------------------------------------------------------------
// Generic base class (subscript in superclasses)
// ---------------------------------------------------------------------------

#[test]
fn test_python_generic_base() {
    let source = r#"
from typing import Generic, TypeVar

T = TypeVar("T")

class Container(Generic):
    def get(self) -> object:
        pass

class StringContainer(Container):
    def get(self) -> str:
        return ""
"#;
    let files = parse_python("generic.py", source);
    let provider = PythonTypeProvider::from_parsed_files(&files);

    // Container inherits from Generic.
    assert!(provider
        .resolve_type("generic.py", "Container", 0)
        .is_some());
    let subtypes = provider.subtypes_of("Container");
    assert!(subtypes.contains(&"StringContainer".to_string()));
}

// ---------------------------------------------------------------------------
// Attribute type strings preserved
// ---------------------------------------------------------------------------

#[test]
fn test_python_attribute_types() {
    let source = r#"
class Node:
    value: int
    children: list
    parent: object
"#;
    let files = parse_python("node.py", source);
    let provider = PythonTypeProvider::from_parsed_files(&files);

    let fields = provider.field_layout("Node").unwrap();
    let field_map: BTreeMap<&str, &str> = fields
        .iter()
        .map(|f| (f.name.as_str(), f.type_str.as_str()))
        .collect();
    assert_eq!(field_map.get("value"), Some(&"int"));
    assert_eq!(field_map.get("children"), Some(&"list"));
    assert_eq!(field_map.get("parent"), Some(&"object"));
}

// ---------------------------------------------------------------------------
// Decorated class (not dataclass)
// ---------------------------------------------------------------------------

#[test]
fn test_python_decorated_class_non_dataclass() {
    let source = r#"
def my_decorator(cls):
    return cls

@my_decorator
class Decorated:
    value: str
"#;
    let files = parse_python("decorated.py", source);
    let provider = PythonTypeProvider::from_parsed_files(&files);

    assert!(provider
        .resolve_type("decorated.py", "Decorated", 0)
        .is_some());
}

// ---------------------------------------------------------------------------
// Bare annotation type strings (reviewer item #1)
// ---------------------------------------------------------------------------

#[test]
fn test_python_bare_annotation_type_strings() {
    // Verify that bare annotations (no default) extract the type string correctly.
    let source = r#"
class Config:
    host: str
    port: int
    debug: bool
    timeout: float = 30.0
"#;
    let files = parse_python("config.py", source);
    let provider = PythonTypeProvider::from_parsed_files(&files);

    let fields = provider.field_layout("Config").unwrap();
    let field_map: BTreeMap<&str, &str> = fields
        .iter()
        .map(|f| (f.name.as_str(), f.type_str.as_str()))
        .collect();
    // Bare annotations — no default value.
    assert_eq!(field_map.get("host"), Some(&"str"));
    assert_eq!(field_map.get("port"), Some(&"int"));
    assert_eq!(field_map.get("debug"), Some(&"bool"));
    // Annotated assignment — with default value.
    assert_eq!(field_map.get("timeout"), Some(&"float"));
}

// ---------------------------------------------------------------------------
// Dotted base class (e.g., abc.ABC)
// ---------------------------------------------------------------------------

#[test]
fn test_python_dotted_base_class() {
    let source = r#"
import abc

class MyABC(abc.ABC):
    def abstract_method(self) -> None:
        pass
"#;
    let files = parse_python("myabc.py", source);
    let provider = PythonTypeProvider::from_parsed_files(&files);

    assert!(provider.resolve_type("myabc.py", "MyABC", 0).is_some());
}

// ---------------------------------------------------------------------------
// Generic base with subscript (e.g., Generic[T])
// ---------------------------------------------------------------------------

#[test]
fn test_python_generic_subscript_base() {
    let source = r#"
from typing import Generic, TypeVar

T = TypeVar("T")

class Stack(Generic[T]):
    items: list

    def push(self, item: object) -> None:
        pass

    def pop(self) -> object:
        pass
"#;
    let files = parse_python("stack.py", source);
    let provider = PythonTypeProvider::from_parsed_files(&files);

    assert!(provider.resolve_type("stack.py", "Stack", 0).is_some());
    let fields = provider.field_layout("Stack").unwrap();
    let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
    assert!(names.contains(&"items"));
    assert!(names.contains(&"push"));
    assert!(names.contains(&"pop"));
}

// ---------------------------------------------------------------------------
// Dotted decorator path (dataclasses.dataclass)
// ---------------------------------------------------------------------------

#[test]
fn test_python_dotted_dataclass_decorator() {
    let source = r#"
import dataclasses

@dataclasses.dataclass
class Coords:
    lat: float
    lng: float
"#;
    let files = parse_python("coords.py", source);
    let provider = PythonTypeProvider::from_parsed_files(&files);

    assert!(provider.resolve_type("coords.py", "Coords", 0).is_some());
    let fields = provider.field_layout("Coords").unwrap();
    let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
    assert!(names.contains(&"lat"));
    assert!(names.contains(&"lng"));
}

// ---------------------------------------------------------------------------
// Decorated top-level function
// ---------------------------------------------------------------------------

#[test]
fn test_python_decorated_top_level_function() {
    let source = r#"
def decorator(f):
    return f

@decorator
def process(data: str) -> bool:
    return True
"#;
    let files = parse_python("decorated_fn.py", source);
    let provider = PythonTypeProvider::from_parsed_files(&files);

    // Decorated functions are extracted.
    // resolve_type doesn't return functions, but the function is stored internally.
    // Verify the provider doesn't crash on decorated top-level functions.
    assert!(provider
        .resolve_type("decorated_fn.py", "process", 0)
        .is_none());
}

// ---------------------------------------------------------------------------
// Function with no return type
// ---------------------------------------------------------------------------

#[test]
fn test_python_function_no_return_type() {
    let source = r#"
def fire_and_forget(msg: str):
    print(msg)
"#;
    let files = parse_python("noret.py", source);
    let provider = PythonTypeProvider::from_parsed_files(&files);

    // Should not crash. Functions aren't resolvable as types.
    assert!(provider
        .resolve_type("noret.py", "fire_and_forget", 0)
        .is_none());
}

// ---------------------------------------------------------------------------
// Method signature includes params and return type
// ---------------------------------------------------------------------------

#[test]
fn test_python_method_signature() {
    let source = r#"
class Calculator:
    def add(self, a: int, b: int) -> int:
        return a + b

    def reset(self) -> None:
        pass
"#;
    let files = parse_python("calc.py", source);
    let provider = PythonTypeProvider::from_parsed_files(&files);

    let fields = provider.field_layout("Calculator").unwrap();
    let add = fields.iter().find(|f| f.name == "add");
    assert!(add.is_some());
    let sig = &add.unwrap().type_str;
    assert!(sig.contains("->"), "Expected return type in sig: {:?}", sig);
    assert!(sig.contains("int"), "Expected 'int' in sig: {:?}", sig);

    let reset = fields.iter().find(|f| f.name == "reset");
    assert!(reset.is_some());
    assert!(reset.unwrap().type_str.contains("None"));
}

// ---------------------------------------------------------------------------
// Class with no superclasses
// ---------------------------------------------------------------------------

#[test]
fn test_python_class_no_superclasses() {
    let source = r#"
class Standalone:
    value: int

    def get(self) -> int:
        return self.value
"#;
    let files = parse_python("standalone.py", source);
    let provider = PythonTypeProvider::from_parsed_files(&files);

    assert!(provider.subtypes_of("Standalone").is_empty());
    let fields = provider.field_layout("Standalone").unwrap();
    assert!(!fields.is_empty());
}

// ---------------------------------------------------------------------------
// Typed default parameter in function
// ---------------------------------------------------------------------------

#[test]
fn test_python_typed_default_parameter() {
    let source = r#"
class Paginator:
    def paginate(self, page: int = 1, size: int = 20) -> list:
        return []
"#;
    let files = parse_python("paginator.py", source);
    let provider = PythonTypeProvider::from_parsed_files(&files);

    let fields = provider.field_layout("Paginator").unwrap();
    let paginate = fields.iter().find(|f| f.name == "paginate");
    assert!(paginate.is_some());
    let sig = &paginate.unwrap().type_str;
    // Signature should include params and return type.
    assert!(sig.contains("->"), "Expected return type in sig: {:?}", sig);
}
