// Tests for the Rust type provider (Phase 5 of E12).
//
// Covers: struct extraction, enum extraction, trait extraction, impl blocks,
// trait satisfaction, dispatch resolution with RTA filtering, type aliases,
// supertrait propagation, Arc sharing, multi-file resolution, CpgContext
// integration, and cycle protection.

use prism::ast::ParsedFile;
use prism::languages::Language;
use prism::type_provider::{DispatchProvider, ResolvedTypeKind, TypeProvider};
use prism::type_providers::rust_provider::RustTypeProvider;
use std::collections::{BTreeMap, BTreeSet};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_rust(path: &str, source: &str) -> BTreeMap<String, ParsedFile> {
    let parsed = ParsedFile::parse(path, source, Language::Rust).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    files
}

fn parse_rust_files(sources: &[(&str, &str)]) -> BTreeMap<String, ParsedFile> {
    let mut files = BTreeMap::new();
    for (path, source) in sources {
        let parsed = ParsedFile::parse(path, source, Language::Rust).unwrap();
        files.insert(path.to_string(), parsed);
    }
    files
}

// ---------------------------------------------------------------------------
// Struct extraction
// ---------------------------------------------------------------------------

#[test]
fn test_rust_struct_extraction() {
    let source = r#"
struct Config {
    host: String,
    port: u16,
    debug: bool,
}
"#;
    let files = parse_rust("config.rs", source);
    let provider = RustTypeProvider::from_parsed_files(&files);

    let resolved = provider.resolve_type("config.rs", "Config", 0);
    assert!(resolved.is_some());
    let rt = resolved.unwrap();
    assert_eq!(rt.name, "Config");
    assert_eq!(rt.kind, ResolvedTypeKind::Concrete);

    let fields = provider.field_layout("Config").unwrap();
    let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
    assert!(names.contains(&"host"), "Expected 'host' in {:?}", names);
    assert!(names.contains(&"port"), "Expected 'port' in {:?}", names);
    assert!(names.contains(&"debug"), "Expected 'debug' in {:?}", names);
}

#[test]
fn test_rust_struct_field_types() {
    let source = r#"
struct Node {
    value: i32,
    next: Option<Box<Node>>,
}
"#;
    let files = parse_rust("node.rs", source);
    let provider = RustTypeProvider::from_parsed_files(&files);

    let fields = provider.field_layout("Node").unwrap();
    let field_map: BTreeMap<&str, &str> = fields
        .iter()
        .map(|f| (f.name.as_str(), f.type_str.as_str()))
        .collect();
    assert_eq!(field_map.get("value"), Some(&"i32"));
    assert_eq!(field_map.get("next"), Some(&"Option<Box<Node>>"));
}

#[test]
fn test_rust_struct_with_methods() {
    let source = r#"
struct Counter {
    count: u64,
}

impl Counter {
    fn new() -> Self {
        Counter { count: 0 }
    }

    fn increment(&mut self) {
        self.count += 1;
    }

    fn value(&self) -> u64 {
        self.count
    }
}
"#;
    let files = parse_rust("counter.rs", source);
    let provider = RustTypeProvider::from_parsed_files(&files);

    let fields = provider.field_layout("Counter").unwrap();
    let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
    assert!(names.contains(&"count"), "Expected field 'count'");
    assert!(names.contains(&"new"), "Expected method 'new'");
    assert!(names.contains(&"increment"), "Expected method 'increment'");
    assert!(names.contains(&"value"), "Expected method 'value'");
}

// ---------------------------------------------------------------------------
// Enum extraction
// ---------------------------------------------------------------------------

#[test]
fn test_rust_enum_extraction() {
    let source = r#"
enum Color {
    Red,
    Green,
    Blue,
}
"#;
    let files = parse_rust("color.rs", source);
    let provider = RustTypeProvider::from_parsed_files(&files);

    let resolved = provider.resolve_type("color.rs", "Color", 0);
    assert!(resolved.is_some());
    assert_eq!(resolved.unwrap().kind, ResolvedTypeKind::Enum);
}

#[test]
fn test_rust_enum_with_methods() {
    let source = r#"
enum Shape {
    Circle(f64),
    Rectangle(f64, f64),
}

impl Shape {
    fn area(&self) -> f64 {
        match self {
            Shape::Circle(r) => 3.14 * r * r,
            Shape::Rectangle(w, h) => w * h,
        }
    }
}
"#;
    let files = parse_rust("shape.rs", source);
    let provider = RustTypeProvider::from_parsed_files(&files);

    let fields = provider.field_layout("Shape").unwrap();
    let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
    assert!(
        names.contains(&"area"),
        "Expected method 'area' in {:?}",
        names
    );
}

// ---------------------------------------------------------------------------
// Trait extraction
// ---------------------------------------------------------------------------

#[test]
fn test_rust_trait_extraction() {
    let source = r#"
trait Drawable {
    fn draw(&self);
    fn bounds(&self) -> (f64, f64, f64, f64);
}
"#;
    let files = parse_rust("draw.rs", source);
    let provider = RustTypeProvider::from_parsed_files(&files);

    let resolved = provider.resolve_type("draw.rs", "Drawable", 0);
    assert!(resolved.is_some());
    let rt = resolved.unwrap();
    assert_eq!(rt.name, "Drawable");
    assert_eq!(rt.kind, ResolvedTypeKind::Interface);

    let fields = provider.field_layout("Drawable").unwrap();
    let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
    assert!(names.contains(&"draw"), "Expected 'draw' in {:?}", names);
    assert!(
        names.contains(&"bounds"),
        "Expected 'bounds' in {:?}",
        names
    );
}

// ---------------------------------------------------------------------------
// Trait satisfaction (impl Trait for Type)
// ---------------------------------------------------------------------------

#[test]
fn test_rust_trait_impl() {
    let source = r#"
trait Reader {
    fn read(&mut self, buf: &mut [u8]) -> usize;
}

struct FileReader {
    path: String,
}

impl Reader for FileReader {
    fn read(&mut self, buf: &mut [u8]) -> usize {
        0
    }
}

struct StringReader {
    data: String,
}

impl Reader for StringReader {
    fn read(&mut self, buf: &mut [u8]) -> usize {
        0
    }
}
"#;
    let files = parse_rust("readers.rs", source);
    let provider = RustTypeProvider::from_parsed_files(&files);

    let subtypes = provider.subtypes_of("Reader");
    assert!(
        subtypes.contains(&"FileReader".to_string()),
        "Expected FileReader in subtypes: {:?}",
        subtypes
    );
    assert!(
        subtypes.contains(&"StringReader".to_string()),
        "Expected StringReader in subtypes: {:?}",
        subtypes
    );
}

#[test]
fn test_rust_multiple_trait_impls() {
    let source = r#"
trait Display {
    fn fmt(&self) -> String;
}

trait Debug {
    fn debug(&self) -> String;
}

struct Point {
    x: f64,
    y: f64,
}

impl Display for Point {
    fn fmt(&self) -> String {
        format!("({}, {})", self.x, self.y)
    }
}

impl Debug for Point {
    fn debug(&self) -> String {
        format!("Point {{ x: {}, y: {} }}", self.x, self.y)
    }
}
"#;
    let files = parse_rust("point.rs", source);
    let provider = RustTypeProvider::from_parsed_files(&files);

    assert!(provider
        .subtypes_of("Display")
        .contains(&"Point".to_string()));
    assert!(provider.subtypes_of("Debug").contains(&"Point".to_string()));
}

// ---------------------------------------------------------------------------
// Dispatch resolution
// ---------------------------------------------------------------------------

#[test]
fn test_rust_dispatch_concrete() {
    let source = r#"
struct Service {
    name: String,
}

impl Service {
    fn run(&self) {}
    fn stop(&self) {}
}
"#;
    let files = parse_rust("service.rs", source);
    let provider = RustTypeProvider::from_parsed_files(&files);

    let results = provider.resolve_dispatch("Service", "run", &BTreeSet::new());
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "run");
    assert_eq!(results[0].file, "service.rs");
}

#[test]
fn test_rust_dispatch_through_trait() {
    let source = r#"
trait Handler {
    fn handle(&self);
}

struct RequestHandler;
struct EventHandler;

impl Handler for RequestHandler {
    fn handle(&self) {}
}

impl Handler for EventHandler {
    fn handle(&self) {}
}
"#;
    let files = parse_rust("handlers.rs", source);
    let provider = RustTypeProvider::from_parsed_files(&files);

    let results = provider.resolve_dispatch("Handler", "handle", &BTreeSet::new());
    assert_eq!(results.len(), 2);
    let names: BTreeSet<String> = results.iter().map(|r| r.name.clone()).collect();
    assert!(names.contains("handle"));
}

#[test]
fn test_rust_dispatch_with_rta() {
    let source = r#"
trait Cache {
    fn get(&self, key: &str) -> Option<String>;
}

struct MemoryCache;
struct DiskCache;
struct RedisCache;

impl Cache for MemoryCache {
    fn get(&self, key: &str) -> Option<String> { None }
}

impl Cache for DiskCache {
    fn get(&self, key: &str) -> Option<String> { None }
}

impl Cache for RedisCache {
    fn get(&self, key: &str) -> Option<String> { None }
}
"#;
    let files = parse_rust("cache.rs", source);
    let provider = RustTypeProvider::from_parsed_files(&files);

    // Without RTA: all 3.
    let all = provider.resolve_dispatch("Cache", "get", &BTreeSet::new());
    assert_eq!(all.len(), 3);

    // With RTA: only MemoryCache live.
    let live = BTreeSet::from(["MemoryCache".to_string()]);
    let filtered = provider.resolve_dispatch("Cache", "get", &live);
    assert_eq!(filtered.len(), 1);
}

#[test]
fn test_rust_dispatch_rta_fallback() {
    let source = r#"
trait Processor {
    fn process(&self);
}

struct FastProcessor;

impl Processor for FastProcessor {
    fn process(&self) {}
}
"#;
    let files = parse_rust("proc.rs", source);
    let provider = RustTypeProvider::from_parsed_files(&files);

    // RTA with no matching live types should fall back to all.
    let live = BTreeSet::from(["Unrelated".to_string()]);
    let results = provider.resolve_dispatch("Processor", "process", &live);
    assert_eq!(results.len(), 1);
}

// ---------------------------------------------------------------------------
// Supertraits
// ---------------------------------------------------------------------------

#[test]
fn test_rust_supertrait_satisfaction() {
    let source = r#"
trait Base {
    fn base(&self);
}

trait Extended: Base {
    fn extended(&self);
}

struct Impl;

impl Base for Impl {
    fn base(&self) {}
}

impl Extended for Impl {
    fn extended(&self) {}
}
"#;
    let files = parse_rust("supertrait.rs", source);
    let provider = RustTypeProvider::from_parsed_files(&files);

    // Impl satisfies Extended.
    let ext_subtypes = provider.subtypes_of("Extended");
    assert!(ext_subtypes.contains(&"Impl".to_string()));

    // Impl also satisfies Base (both directly and through supertrait propagation).
    let base_subtypes = provider.subtypes_of("Base");
    assert!(
        base_subtypes.contains(&"Impl".to_string()),
        "Expected Impl in Base subtypes: {:?}",
        base_subtypes
    );
}

#[test]
fn test_rust_trait_method_inheritance() {
    let source = r#"
trait Read {
    fn read(&self) -> Vec<u8>;
}

trait BufRead: Read {
    fn read_line(&self) -> String;
}
"#;
    let files = parse_rust("bufread.rs", source);
    let provider = RustTypeProvider::from_parsed_files(&files);

    // BufRead's field layout should include Read's methods.
    let fields = provider.field_layout("BufRead").unwrap();
    let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
    assert!(
        names.contains(&"read_line"),
        "Expected 'read_line' in {:?}",
        names
    );
    assert!(
        names.contains(&"read"),
        "Expected inherited 'read' in {:?}",
        names
    );
}

// ---------------------------------------------------------------------------
// Type aliases
// ---------------------------------------------------------------------------

#[test]
fn test_rust_type_alias() {
    let source = r#"
type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;
"#;
    let files = parse_rust("types.rs", source);
    let provider = RustTypeProvider::from_parsed_files(&files);

    let resolved = provider.resolve_type("types.rs", "Result", 0);
    assert!(resolved.is_some());
    assert_eq!(resolved.unwrap().kind, ResolvedTypeKind::Alias);

    assert_ne!(provider.resolve_alias("Result"), "Result");
}

// ---------------------------------------------------------------------------
// Unknown types
// ---------------------------------------------------------------------------

#[test]
fn test_rust_unknown_type() {
    let source = "struct Foo;";
    let files = parse_rust("foo.rs", source);
    let provider = RustTypeProvider::from_parsed_files(&files);

    assert!(provider.resolve_type("foo.rs", "NonExistent", 0).is_none());
    assert!(provider.field_layout("NonExistent").is_none());
    assert!(provider.subtypes_of("NonExistent").is_empty());
    assert_eq!(provider.resolve_alias("NonExistent"), "NonExistent");
}

#[test]
fn test_rust_dispatch_nonexistent() {
    let source = "struct Foo;";
    let files = parse_rust("foo.rs", source);
    let provider = RustTypeProvider::from_parsed_files(&files);

    assert!(provider
        .resolve_dispatch("Foo", "bar", &BTreeSet::new())
        .is_empty());
    assert!(provider
        .resolve_dispatch("Unknown", "bar", &BTreeSet::new())
        .is_empty());
}

// ---------------------------------------------------------------------------
// Multi-file
// ---------------------------------------------------------------------------

#[test]
fn test_rust_multi_file() {
    let sources = &[
        (
            "trait.rs",
            r#"
trait Animal {
    fn speak(&self) -> &str;
}
"#,
        ),
        (
            "dog.rs",
            r#"
struct Dog;

impl Animal for Dog {
    fn speak(&self) -> &str { "Woof" }
}
"#,
        ),
        (
            "cat.rs",
            r#"
struct Cat;

impl Animal for Cat {
    fn speak(&self) -> &str { "Meow" }
}
"#,
        ),
    ];
    let files = parse_rust_files(sources);
    let provider = RustTypeProvider::from_parsed_files(&files);

    let subtypes = provider.subtypes_of("Animal");
    assert!(subtypes.contains(&"Dog".to_string()));
    assert!(subtypes.contains(&"Cat".to_string()));

    let results = provider.resolve_dispatch("Animal", "speak", &BTreeSet::new());
    assert_eq!(results.len(), 2);
    let dispatch_files: BTreeSet<String> = results.iter().map(|r| r.file.clone()).collect();
    assert!(dispatch_files.contains("dog.rs"));
    assert!(dispatch_files.contains("cat.rs"));
}

// ---------------------------------------------------------------------------
// Arc sharing
// ---------------------------------------------------------------------------

#[test]
fn test_rust_arc_sharing() {
    let source = "struct Foo;";
    let files = parse_rust("foo.rs", source);
    let provider = RustTypeProvider::from_parsed_files(&files);
    let cloned = provider.clone();

    assert!(provider.resolve_type("foo.rs", "Foo", 0).is_some());
    assert!(cloned.resolve_type("foo.rs", "Foo", 0).is_some());
    assert!(std::sync::Arc::ptr_eq(&provider.data, &cloned.data));
}

// ---------------------------------------------------------------------------
// Registry integration
// ---------------------------------------------------------------------------

#[test]
fn test_rust_registry_integration() {
    use prism::type_provider::TypeRegistry;

    let source = r#"
trait Runnable {
    fn run(&self);
}

struct Task;

impl Runnable for Task {
    fn run(&self) {}
}
"#;
    let files = parse_rust("task.rs", source);
    let provider = RustTypeProvider::from_parsed_files(&files);

    let mut registry = TypeRegistry::empty();
    let dispatch = provider.clone();
    registry.register_provider(Box::new(provider));
    registry.register_dispatch_provider(Box::new(dispatch));

    let tp = registry.provider_for(Language::Rust);
    assert!(tp.is_some());
    assert_eq!(
        tp.unwrap().resolve_type("task.rs", "Task", 0).unwrap().kind,
        ResolvedTypeKind::Concrete
    );

    let dp = registry.dispatch_for(Language::Rust);
    assert!(dp.is_some());
    let results = dp
        .unwrap()
        .resolve_dispatch("Runnable", "run", &BTreeSet::new());
    assert_eq!(results.len(), 1);

    // No structural typing for Rust.
    assert!(registry.structural_for(Language::Rust).is_none());
}

// ---------------------------------------------------------------------------
// CpgContext integration
// ---------------------------------------------------------------------------

#[test]
fn test_rust_cpg_context_integration() {
    use prism::cpg::CpgContext;

    let source = r#"
trait Logger {
    fn log(&self, msg: &str);
}

struct ConsoleLogger;

impl Logger for ConsoleLogger {
    fn log(&self, msg: &str) {
        println!("{}", msg);
    }
}
"#;
    let parsed = ParsedFile::parse("logger.rs", source, Language::Rust).unwrap();
    let mut files = BTreeMap::new();
    files.insert("logger.rs".to_string(), parsed);

    let ctx = CpgContext::build(&files, None);

    let tp = ctx.types.provider_for(Language::Rust);
    assert!(tp.is_some(), "Rust provider should be auto-registered");

    let tp = tp.unwrap();
    assert_eq!(
        tp.resolve_type("logger.rs", "Logger", 0).unwrap().kind,
        ResolvedTypeKind::Interface
    );
    assert_eq!(
        tp.resolve_type("logger.rs", "ConsoleLogger", 0)
            .unwrap()
            .kind,
        ResolvedTypeKind::Concrete
    );

    let dp = ctx.types.dispatch_for(Language::Rust);
    assert!(dp.is_some());
    let targets = dp
        .unwrap()
        .resolve_dispatch("Logger", "log", &BTreeSet::new());
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].name, "log");
}

// ---------------------------------------------------------------------------
// Languages method
// ---------------------------------------------------------------------------

#[test]
fn test_rust_provider_languages() {
    let files = parse_rust("empty.rs", "struct Empty;");
    let provider = RustTypeProvider::from_parsed_files(&files);
    assert_eq!(provider.languages(), vec![Language::Rust]);
}

// ---------------------------------------------------------------------------
// Non-Rust files ignored
// ---------------------------------------------------------------------------

#[test]
fn test_rust_ignores_non_rust_files() {
    let rust_parsed = ParsedFile::parse("lib.rs", "struct Foo;", Language::Rust).unwrap();
    let go_parsed =
        ParsedFile::parse("main.go", "package main\nfunc main() {}\n", Language::Go).unwrap();

    let mut files = BTreeMap::new();
    files.insert("lib.rs".to_string(), rust_parsed);
    files.insert("main.go".to_string(), go_parsed);

    let provider = RustTypeProvider::from_parsed_files(&files);
    assert!(provider.resolve_type("lib.rs", "Foo", 0).is_some());
    assert!(provider.resolve_type("main.go", "main", 0).is_none());
}

// ---------------------------------------------------------------------------
// Inherent vs trait methods in field_layout
// ---------------------------------------------------------------------------

#[test]
fn test_rust_field_layout_inherent_only() {
    let source = r#"
trait Printable {
    fn print(&self);
}

struct Doc {
    title: String,
}

impl Doc {
    fn new(title: String) -> Self {
        Doc { title }
    }
}

impl Printable for Doc {
    fn print(&self) {}
}
"#;
    let files = parse_rust("doc.rs", source);
    let provider = RustTypeProvider::from_parsed_files(&files);

    // field_layout for a struct should include struct fields + inherent methods
    // but NOT trait impl methods.
    let fields = provider.field_layout("Doc").unwrap();
    let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
    assert!(names.contains(&"title"), "Expected field 'title'");
    assert!(names.contains(&"new"), "Expected inherent method 'new'");
    assert!(
        !names.contains(&"print"),
        "Trait impl methods should not be in field_layout"
    );
}

// ---------------------------------------------------------------------------
// Cyclic trait bounds (safety)
// ---------------------------------------------------------------------------

#[test]
fn test_rust_cyclic_supertrait_no_overflow() {
    // Invalid Rust, but tree-sitter parses it.
    let source = r#"
trait A: B {
    fn a(&self);
}

trait B: A {
    fn b(&self);
}
"#;
    let files = parse_rust("cycle.rs", source);
    let provider = RustTypeProvider::from_parsed_files(&files);

    // Should not stack-overflow.
    let _ = provider.field_layout("A");
    let _ = provider.field_layout("B");
    let _ = provider.subtypes_of("A");
}

// ---------------------------------------------------------------------------
// Inline module items
// ---------------------------------------------------------------------------

#[test]
fn test_rust_inline_module() {
    let source = r#"
mod inner {
    pub struct InnerType {
        pub value: i32,
    }

    pub trait InnerTrait {
        fn inner_method(&self);
    }

    impl InnerTrait for InnerType {
        fn inner_method(&self) {}
    }
}
"#;
    let files = parse_rust("module.rs", source);
    let provider = RustTypeProvider::from_parsed_files(&files);

    assert!(provider.resolve_type("module.rs", "InnerType", 0).is_some());
    assert!(provider
        .resolve_type("module.rs", "InnerTrait", 0)
        .is_some());
    assert!(provider
        .subtypes_of("InnerTrait")
        .contains(&"InnerType".to_string()));
}

// ---------------------------------------------------------------------------
// Enum implementing trait
// ---------------------------------------------------------------------------

#[test]
fn test_rust_enum_impl_trait() {
    let source = r#"
trait Describable {
    fn describe(&self) -> &str;
}

enum Color {
    Red,
    Green,
    Blue,
}

impl Describable for Color {
    fn describe(&self) -> &str {
        match self {
            Color::Red => "red",
            Color::Green => "green",
            Color::Blue => "blue",
        }
    }
}
"#;
    let files = parse_rust("color.rs", source);
    let provider = RustTypeProvider::from_parsed_files(&files);

    let subtypes = provider.subtypes_of("Describable");
    assert!(
        subtypes.contains(&"Color".to_string()),
        "Color should satisfy Describable: {:?}",
        subtypes
    );

    // Dispatch through trait should find Color's implementation.
    let results = provider.resolve_dispatch("Describable", "describe", &BTreeSet::new());
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "describe");
}

// ---------------------------------------------------------------------------
// Generic struct and trait
// ---------------------------------------------------------------------------

#[test]
fn test_rust_generic_struct() {
    let source = r#"
struct Container<T> {
    value: T,
    count: usize,
}

impl<T> Container<T> {
    fn new(value: T) -> Self {
        Container { value, count: 0 }
    }

    fn get(&self) -> &T {
        &self.value
    }
}
"#;
    let files = parse_rust("container.rs", source);
    let provider = RustTypeProvider::from_parsed_files(&files);

    // Generic params are stripped: Container<T> → Container.
    let resolved = provider.resolve_type("container.rs", "Container", 0);
    assert!(resolved.is_some());
    assert_eq!(resolved.unwrap().kind, ResolvedTypeKind::Concrete);

    let fields = provider.field_layout("Container").unwrap();
    let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
    assert!(names.contains(&"value"));
    assert!(names.contains(&"count"));
    assert!(names.contains(&"new"));
    assert!(names.contains(&"get"));
}

#[test]
fn test_rust_generic_trait_impl() {
    let source = r#"
trait From<T> {
    fn from(value: T) -> Self;
}

struct Wrapper {
    inner: String,
}

impl From<String> for Wrapper {
    fn from(value: String) -> Self {
        Wrapper { inner: value }
    }
}
"#;
    let files = parse_rust("from.rs", source);
    let provider = RustTypeProvider::from_parsed_files(&files);

    // Wrapper should satisfy From (generics stripped).
    let subtypes = provider.subtypes_of("From");
    assert!(
        subtypes.contains(&"Wrapper".to_string()),
        "Wrapper should satisfy From: {:?}",
        subtypes
    );
}

// ---------------------------------------------------------------------------
// Diamond supertrait pattern
// ---------------------------------------------------------------------------

#[test]
fn test_rust_diamond_supertraits() {
    let source = r#"
trait A {
    fn a(&self);
}

trait B: A {
    fn b(&self);
}

trait C: A {
    fn c(&self);
}

trait D: B + C {
    fn d(&self);
}

struct Impl;

impl A for Impl { fn a(&self) {} }
impl B for Impl { fn b(&self) {} }
impl C for Impl { fn c(&self) {} }
impl D for Impl { fn d(&self) {} }
"#;
    let files = parse_rust("diamond.rs", source);
    let provider = RustTypeProvider::from_parsed_files(&files);

    // Impl should satisfy all four traits.
    assert!(provider.subtypes_of("A").contains(&"Impl".to_string()));
    assert!(provider.subtypes_of("B").contains(&"Impl".to_string()));
    assert!(provider.subtypes_of("C").contains(&"Impl".to_string()));
    assert!(provider.subtypes_of("D").contains(&"Impl".to_string()));

    // D's field_layout should include methods from A, B, C, D.
    let fields = provider.field_layout("D").unwrap();
    let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
    assert!(names.contains(&"a"), "Expected 'a' from supertrait A");
    assert!(names.contains(&"b"), "Expected 'b' from supertrait B");
    assert!(names.contains(&"c"), "Expected 'c' from supertrait C");
    assert!(names.contains(&"d"), "Expected 'd' from D itself");
}

// ---------------------------------------------------------------------------
// Cross-file dispatch with RTA
// ---------------------------------------------------------------------------

#[test]
fn test_rust_cross_file_dispatch_rta() {
    let sources = &[
        (
            "trait.rs",
            r#"
trait Cache {
    fn get(&self, key: &str) -> Option<String>;
}
"#,
        ),
        (
            "memory.rs",
            r#"
struct MemoryCache;
impl Cache for MemoryCache {
    fn get(&self, key: &str) -> Option<String> { None }
}
"#,
        ),
        (
            "redis.rs",
            r#"
struct RedisCache;
impl Cache for RedisCache {
    fn get(&self, key: &str) -> Option<String> { None }
}
"#,
        ),
        (
            "disk.rs",
            r#"
struct DiskCache;
impl Cache for DiskCache {
    fn get(&self, key: &str) -> Option<String> { None }
}
"#,
        ),
    ];
    let files = parse_rust_files(sources);
    let provider = RustTypeProvider::from_parsed_files(&files);

    // Without RTA: all 3.
    let all = provider.resolve_dispatch("Cache", "get", &BTreeSet::new());
    assert_eq!(all.len(), 3);

    // With RTA: only MemoryCache and RedisCache live.
    let live = BTreeSet::from(["MemoryCache".to_string(), "RedisCache".to_string()]);
    let filtered = provider.resolve_dispatch("Cache", "get", &live);
    assert_eq!(filtered.len(), 2);
    let ffiles: BTreeSet<String> = filtered.iter().map(|f| f.file.clone()).collect();
    assert!(ffiles.contains("memory.rs"));
    assert!(ffiles.contains("redis.rs"));
    assert!(!ffiles.contains("disk.rs"));
}
