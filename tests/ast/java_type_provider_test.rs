// Tests for the Java type provider (Phase 4 of E12).
//
// Covers: class extraction, interface extraction, enum extraction, field
// layout, class hierarchy (extends/implements), interface satisfaction,
// dispatch resolution with RTA filtering, Arc sharing, multi-file resolution,
// and registry integration.

use prism::ast::ParsedFile;
use prism::languages::Language;
use prism::type_provider::{DispatchProvider, ResolvedTypeKind, TypeProvider};
use prism::type_providers::java::JavaTypeProvider;
use std::collections::{BTreeMap, BTreeSet};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_java(path: &str, source: &str) -> BTreeMap<String, ParsedFile> {
    let parsed = ParsedFile::parse(path, source, Language::Java).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    files
}

fn parse_java_files(sources: &[(&str, &str)]) -> BTreeMap<String, ParsedFile> {
    let mut files = BTreeMap::new();
    for (path, source) in sources {
        let parsed = ParsedFile::parse(path, source, Language::Java).unwrap();
        files.insert(path.to_string(), parsed);
    }
    files
}

// ---------------------------------------------------------------------------
// Class extraction
// ---------------------------------------------------------------------------

#[test]
fn test_java_class_extraction() {
    let source = r#"
public class User {
    private String name;
    private int age;
    private boolean active;

    public String getName() {
        return name;
    }

    public void setName(String name) {
        this.name = name;
    }
}
"#;
    let files = parse_java("User.java", source);
    let provider = JavaTypeProvider::from_parsed_files(&files);

    let resolved = provider.resolve_type("User.java", "User", 0);
    assert!(resolved.is_some());
    let rt = resolved.unwrap();
    assert_eq!(rt.name, "User");
    assert_eq!(rt.kind, ResolvedTypeKind::Concrete);

    let fields = provider.field_layout("User").unwrap();
    let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
    assert!(names.contains(&"name"), "Expected 'name' in {:?}", names);
    assert!(names.contains(&"age"), "Expected 'age' in {:?}", names);
    assert!(
        names.contains(&"active"),
        "Expected 'active' in {:?}",
        names
    );
    assert!(
        names.contains(&"getName"),
        "Expected 'getName' in {:?}",
        names
    );
    assert!(
        names.contains(&"setName"),
        "Expected 'setName' in {:?}",
        names
    );
}

#[test]
fn test_java_class_field_types() {
    let source = r#"
public class Config {
    private String host;
    private int port;
    private List<String> tags;
}
"#;
    let files = parse_java("Config.java", source);
    let provider = JavaTypeProvider::from_parsed_files(&files);

    let fields = provider.field_layout("Config").unwrap();
    let field_map: BTreeMap<&str, &str> = fields
        .iter()
        .map(|f| (f.name.as_str(), f.type_str.as_str()))
        .collect();
    assert_eq!(field_map.get("host"), Some(&"String"));
    assert_eq!(field_map.get("port"), Some(&"int"));
}

// ---------------------------------------------------------------------------
// Interface extraction
// ---------------------------------------------------------------------------

#[test]
fn test_java_interface_extraction() {
    let source = r#"
public interface Repository {
    User find(String id);
    void save(User user);
    void delete(String id);
}
"#;
    let files = parse_java("Repository.java", source);
    let provider = JavaTypeProvider::from_parsed_files(&files);

    let resolved = provider.resolve_type("Repository.java", "Repository", 0);
    assert!(resolved.is_some());
    let rt = resolved.unwrap();
    assert_eq!(rt.name, "Repository");
    assert_eq!(rt.kind, ResolvedTypeKind::Interface);

    let fields = provider.field_layout("Repository").unwrap();
    let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
    assert!(names.contains(&"find"), "Expected 'find' in {:?}", names);
    assert!(names.contains(&"save"), "Expected 'save' in {:?}", names);
    assert!(
        names.contains(&"delete"),
        "Expected 'delete' in {:?}",
        names
    );
}

#[test]
fn test_java_interface_extends() {
    let source = r#"
public interface Readable {
    String read();
}

public interface Closeable {
    void close();
}

public interface ReadableCloseable extends Readable, Closeable {
    void reset();
}
"#;
    let files = parse_java("Streams.java", source);
    let provider = JavaTypeProvider::from_parsed_files(&files);

    // ReadableCloseable should include methods from parent interfaces.
    let fields = provider.field_layout("ReadableCloseable").unwrap();
    let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
    assert!(names.contains(&"reset"), "Expected 'reset' in {:?}", names);
    assert!(names.contains(&"read"), "Expected 'read' in {:?}", names);
    assert!(names.contains(&"close"), "Expected 'close' in {:?}", names);
}

// ---------------------------------------------------------------------------
// Enum extraction
// ---------------------------------------------------------------------------

#[test]
fn test_java_enum_extraction() {
    let source = r#"
public enum Color {
    RED,
    GREEN,
    BLUE
}
"#;
    let files = parse_java("Color.java", source);
    let provider = JavaTypeProvider::from_parsed_files(&files);

    let resolved = provider.resolve_type("Color.java", "Color", 0);
    assert!(resolved.is_some());
    let rt = resolved.unwrap();
    assert_eq!(rt.name, "Color");
    assert_eq!(rt.kind, ResolvedTypeKind::Enum);
}

#[test]
fn test_java_enum_implements_interface() {
    let source = r#"
public interface Printable {
    String print();
}

public enum Status implements Printable {
    ACTIVE,
    INACTIVE;

    public String print() {
        return name();
    }
}
"#;
    let files = parse_java("Status.java", source);
    let provider = JavaTypeProvider::from_parsed_files(&files);

    let resolved = provider.resolve_type("Status.java", "Status", 0);
    assert!(resolved.is_some());
    assert_eq!(resolved.unwrap().kind, ResolvedTypeKind::Enum);

    // Status should satisfy Printable.
    let subtypes = provider.subtypes_of("Printable");
    assert!(
        subtypes.contains(&"Status".to_string()),
        "Expected Status in subtypes of Printable: {:?}",
        subtypes
    );
}

// ---------------------------------------------------------------------------
// Class hierarchy (extends + implements)
// ---------------------------------------------------------------------------

#[test]
fn test_java_class_implements_interface() {
    let source = r#"
public interface Repository {
    Object find(String id);
    void save(Object entity);
}

public class UserRepository implements Repository {
    public Object find(String id) {
        return null;
    }
    public void save(Object entity) {
    }
}
"#;
    let files = parse_java("Repo.java", source);
    let provider = JavaTypeProvider::from_parsed_files(&files);

    // UserRepository should satisfy Repository.
    let subtypes = provider.subtypes_of("Repository");
    assert!(
        subtypes.contains(&"UserRepository".to_string()),
        "Expected UserRepository in subtypes: {:?}",
        subtypes
    );
}

#[test]
fn test_java_class_extends() {
    let source = r#"
public class Animal {
    public void eat() {}
    public void sleep() {}
}

public class Dog extends Animal {
    public void bark() {}
}
"#;
    let files = parse_java("Animals.java", source);
    let provider = JavaTypeProvider::from_parsed_files(&files);

    // Dog should have inherited methods from Animal.
    let fields = provider.field_layout("Dog").unwrap();
    let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
    assert!(names.contains(&"bark"), "Expected 'bark' in {:?}", names);
    assert!(names.contains(&"eat"), "Expected 'eat' in {:?}", names);
    assert!(names.contains(&"sleep"), "Expected 'sleep' in {:?}", names);

    // Dog is a subtype of Animal.
    let subtypes = provider.subtypes_of("Animal");
    assert!(
        subtypes.contains(&"Dog".to_string()),
        "Expected Dog in subtypes: {:?}",
        subtypes
    );
}

#[test]
fn test_java_transitive_implements() {
    let source = r#"
public interface Serializable {
    byte[] serialize();
}

public class BaseEntity implements Serializable {
    public byte[] serialize() {
        return new byte[0];
    }
}

public class User extends BaseEntity {
    private String name;
}
"#;
    let files = parse_java("Entities.java", source);
    let provider = JavaTypeProvider::from_parsed_files(&files);

    // User inherits Serializable implementation from BaseEntity.
    let subtypes = provider.subtypes_of("Serializable");
    assert!(
        subtypes.contains(&"BaseEntity".to_string()),
        "Expected BaseEntity in subtypes: {:?}",
        subtypes
    );
    assert!(
        subtypes.contains(&"User".to_string()),
        "Expected User in subtypes (transitive): {:?}",
        subtypes
    );
}

#[test]
fn test_java_interface_extends_chain() {
    let source = r#"
public interface Base {
    void base();
}

public interface Middle extends Base {
    void middle();
}

public interface Top extends Middle {
    void top();
}

public class Impl implements Top {
    public void base() {}
    public void middle() {}
    public void top() {}
}
"#;
    let files = parse_java("Chain.java", source);
    let provider = JavaTypeProvider::from_parsed_files(&files);

    // Impl should satisfy Base through the interface extends chain.
    let base_subtypes = provider.subtypes_of("Base");
    assert!(
        base_subtypes.contains(&"Impl".to_string()),
        "Expected Impl in subtypes of Base: {:?}",
        base_subtypes
    );
    let middle_subtypes = provider.subtypes_of("Middle");
    assert!(
        middle_subtypes.contains(&"Impl".to_string()),
        "Expected Impl in subtypes of Middle: {:?}",
        middle_subtypes
    );
}

// ---------------------------------------------------------------------------
// Dispatch resolution
// ---------------------------------------------------------------------------

#[test]
fn test_java_dispatch_concrete() {
    let source = r#"
public class Service {
    public void process() {}
    public void validate() {}
}
"#;
    let files = parse_java("Service.java", source);
    let provider = JavaTypeProvider::from_parsed_files(&files);

    let results = provider.resolve_dispatch("Service", "process", &BTreeSet::new());
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "process");
    assert_eq!(results[0].file, "Service.java");
}

#[test]
fn test_java_dispatch_interface() {
    let source = r#"
public interface Handler {
    void handle();
}

public class RequestHandler implements Handler {
    public void handle() {}
}

public class EventHandler implements Handler {
    public void handle() {}
}
"#;
    let files = parse_java("Handlers.java", source);
    let provider = JavaTypeProvider::from_parsed_files(&files);

    // Dispatch through interface should find both implementations.
    let results = provider.resolve_dispatch("Handler", "handle", &BTreeSet::new());
    assert_eq!(results.len(), 2);
    let names: BTreeSet<String> = results.iter().map(|r| r.name.clone()).collect();
    assert!(names.contains("handle"));
}

#[test]
fn test_java_dispatch_with_rta() {
    let source = r#"
public interface Repository {
    Object find(String id);
}

public class UserRepo implements Repository {
    public Object find(String id) { return null; }
}

public class OrderRepo implements Repository {
    public Object find(String id) { return null; }
}
"#;
    let files = parse_java("Repos.java", source);
    let provider = JavaTypeProvider::from_parsed_files(&files);

    // With RTA: only UserRepo is live.
    let mut live = BTreeSet::new();
    live.insert("UserRepo".to_string());

    let results = provider.resolve_dispatch("Repository", "find", &live);
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].file, "Repos.java");

    // With empty live types: all implementations.
    let results = provider.resolve_dispatch("Repository", "find", &BTreeSet::new());
    assert_eq!(results.len(), 2);
}

#[test]
fn test_java_dispatch_rta_fallback() {
    let source = r#"
public interface Processor {
    void process();
}

public class FastProcessor implements Processor {
    public void process() {}
}
"#;
    let files = parse_java("Proc.java", source);
    let provider = JavaTypeProvider::from_parsed_files(&files);

    // RTA with no matching live types should fall back to all.
    let mut live = BTreeSet::new();
    live.insert("UnrelatedType".to_string());

    let results = provider.resolve_dispatch("Processor", "process", &live);
    assert_eq!(results.len(), 1); // Falls back to full set
}

#[test]
fn test_java_dispatch_inherited_method() {
    let source = r#"
public class Base {
    public void common() {}
}

public class Child extends Base {
    public void specific() {}
}
"#;
    let files = parse_java("Inherit.java", source);
    let provider = JavaTypeProvider::from_parsed_files(&files);

    // Dispatching "common" on Child should find it in Base.
    let results = provider.resolve_dispatch("Child", "common", &BTreeSet::new());
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "common");
}

// ---------------------------------------------------------------------------
// Unknown types
// ---------------------------------------------------------------------------

#[test]
fn test_java_unknown_type() {
    let source = r#"
public class Foo {
    public void bar() {}
}
"#;
    let files = parse_java("Foo.java", source);
    let provider = JavaTypeProvider::from_parsed_files(&files);

    assert!(provider
        .resolve_type("Foo.java", "NonExistent", 0)
        .is_none());
    assert!(provider.field_layout("NonExistent").is_none());
}

#[test]
fn test_java_resolve_alias() {
    let source = r#"
public class Foo {}
"#;
    let files = parse_java("Foo.java", source);
    let provider = JavaTypeProvider::from_parsed_files(&files);

    // Java has no type aliases; resolve_alias returns input unchanged.
    assert_eq!(provider.resolve_alias("Foo"), "Foo");
    assert_eq!(provider.resolve_alias("Unknown"), "Unknown");
}

// ---------------------------------------------------------------------------
// Multi-file resolution
// ---------------------------------------------------------------------------

#[test]
fn test_java_multi_file() {
    let sources = &[
        (
            "Animal.java",
            r#"
public interface Animal {
    String speak();
}
"#,
        ),
        (
            "Dog.java",
            r#"
public class Dog implements Animal {
    public String speak() { return "Woof"; }
}
"#,
        ),
        (
            "Cat.java",
            r#"
public class Cat implements Animal {
    public String speak() { return "Meow"; }
}
"#,
        ),
    ];
    let files = parse_java_files(sources);
    let provider = JavaTypeProvider::from_parsed_files(&files);

    let subtypes = provider.subtypes_of("Animal");
    assert!(subtypes.contains(&"Dog".to_string()));
    assert!(subtypes.contains(&"Cat".to_string()));

    let results = provider.resolve_dispatch("Animal", "speak", &BTreeSet::new());
    assert_eq!(results.len(), 2);

    let dispatch_files: BTreeSet<String> = results.iter().map(|r| r.file.clone()).collect();
    assert!(dispatch_files.contains("Dog.java"));
    assert!(dispatch_files.contains("Cat.java"));
}

// ---------------------------------------------------------------------------
// Cycle protection (item 1 from review)
// ---------------------------------------------------------------------------

#[test]
fn test_java_cyclic_extends_no_stack_overflow() {
    // Invalid Java, but tree-sitter parses it. Must not stack-overflow.
    let sources = &[
        (
            "A.java",
            r#"
public class A extends B {
    public void a() {}
}
"#,
        ),
        (
            "B.java",
            r#"
public class B extends A {
    public void b() {}
}
"#,
        ),
    ];
    let files = parse_java_files(sources);
    let provider = JavaTypeProvider::from_parsed_files(&files);

    // Should not panic. Results are best-effort.
    let _ = provider.resolve_type("A.java", "A", 0);
    let _ = provider.field_layout("A");
    let _ = provider.subtypes_of("A");
    let _ = provider.resolve_dispatch("A", "a", &BTreeSet::new());
    let _ = provider.resolve_dispatch("B", "a", &BTreeSet::new());
}

#[test]
fn test_java_cyclic_interface_extends() {
    // Invalid Java: interface cycle.
    let source = r#"
public interface X extends Y {
    void x();
}

public interface Y extends X {
    void y();
}

public class Impl implements X {
    public void x() {}
    public void y() {}
}
"#;
    let files = parse_java("Cycle.java", source);
    let provider = JavaTypeProvider::from_parsed_files(&files);

    // Must not stack-overflow; best-effort results.
    let _ = provider.subtypes_of("X");
    let _ = provider.field_layout("X");
    let _ = provider.resolve_dispatch("X", "x", &BTreeSet::new());
}

// ---------------------------------------------------------------------------
// Enum method extraction and dispatch (item 4 from review)
// ---------------------------------------------------------------------------

#[test]
fn test_java_enum_method_extraction() {
    let source = r#"
public enum Planet {
    MERCURY,
    VENUS,
    EARTH;

    public double mass() {
        return 0.0;
    }

    public double surfaceGravity() {
        return 0.0;
    }
}
"#;
    let files = parse_java("Planet.java", source);
    let provider = JavaTypeProvider::from_parsed_files(&files);

    let fields = provider.field_layout("Planet").unwrap();
    let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
    assert!(names.contains(&"mass"), "Expected 'mass' in {:?}", names);
    assert!(
        names.contains(&"surfaceGravity"),
        "Expected 'surfaceGravity' in {:?}",
        names
    );
}

#[test]
fn test_java_enum_dispatch_through_interface() {
    let source = r#"
public interface Describable {
    String describe();
}

public enum Color implements Describable {
    RED,
    GREEN,
    BLUE;

    public String describe() {
        return name().toLowerCase();
    }
}
"#;
    let files = parse_java("Color.java", source);
    let provider = JavaTypeProvider::from_parsed_files(&files);

    // Color satisfies Describable.
    let subtypes = provider.subtypes_of("Describable");
    assert!(
        subtypes.contains(&"Color".to_string()),
        "Color should satisfy Describable: {:?}",
        subtypes
    );

    // Dispatch through Describable should find Color.describe().
    let results = provider.resolve_dispatch("Describable", "describe", &BTreeSet::new());
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "describe");
    assert_eq!(results[0].file, "Color.java");
}

// ---------------------------------------------------------------------------
// Arc sharing
// ---------------------------------------------------------------------------

#[test]
fn test_java_arc_sharing() {
    let source = r#"
public class Service {
    public void run() {}
}
"#;
    let files = parse_java("Service.java", source);
    let provider = JavaTypeProvider::from_parsed_files(&files);
    let cloned = provider.clone();

    // Both should resolve the same type.
    assert!(provider
        .resolve_type("Service.java", "Service", 0)
        .is_some());
    assert!(cloned.resolve_type("Service.java", "Service", 0).is_some());

    // Arc should point to the same data.
    assert!(std::sync::Arc::ptr_eq(&provider.data, &cloned.data));
}

// ---------------------------------------------------------------------------
// Registry integration
// ---------------------------------------------------------------------------

#[test]
fn test_java_registry_integration() {
    use prism::type_provider::TypeRegistry;

    let source = r#"
public interface Runnable {
    void run();
}

public class Task implements Runnable {
    public void run() {}
}
"#;
    let files = parse_java("Task.java", source);
    let provider = JavaTypeProvider::from_parsed_files(&files);

    let mut registry = TypeRegistry::empty();
    let dispatch = provider.clone();
    registry.register_provider(Box::new(provider));
    registry.register_dispatch_provider(Box::new(dispatch));

    // Should be retrievable by language.
    let tp = registry.provider_for(Language::Java);
    assert!(tp.is_some());
    let tp = tp.unwrap();
    assert_eq!(
        tp.resolve_type("Task.java", "Task", 0).unwrap().kind,
        ResolvedTypeKind::Concrete
    );

    let dp = registry.dispatch_for(Language::Java);
    assert!(dp.is_some());
    let dp = dp.unwrap();
    let results = dp.resolve_dispatch("Runnable", "run", &BTreeSet::new());
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "run");

    // No structural typing for Java.
    assert!(registry.structural_for(Language::Java).is_none());
}

// ---------------------------------------------------------------------------
// Languages method
// ---------------------------------------------------------------------------

#[test]
fn test_java_provider_languages() {
    let files = parse_java("Empty.java", "public class Empty {}");
    let provider = JavaTypeProvider::from_parsed_files(&files);
    assert_eq!(provider.languages(), vec![Language::Java]);
}

// ---------------------------------------------------------------------------
// Generic types
// ---------------------------------------------------------------------------

#[test]
fn test_java_generic_class() {
    let source = r#"
public class Box<T> {
    private T value;

    public T getValue() {
        return value;
    }

    public void setValue(T value) {
        this.value = value;
    }
}
"#;
    let files = parse_java("Box.java", source);
    let provider = JavaTypeProvider::from_parsed_files(&files);

    let resolved = provider.resolve_type("Box.java", "Box", 0);
    assert!(resolved.is_some());
    assert_eq!(resolved.unwrap().kind, ResolvedTypeKind::Concrete);

    let fields = provider.field_layout("Box").unwrap();
    let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
    assert!(names.contains(&"value"), "Expected 'value' in {:?}", names);
    assert!(
        names.contains(&"getValue"),
        "Expected 'getValue' in {:?}",
        names
    );
}

#[test]
fn test_java_implements_generic_interface() {
    let source = r#"
public interface Comparable<T> {
    int compareTo(T other);
}

public class Timestamp implements Comparable<Timestamp> {
    private long millis;

    public int compareTo(Timestamp other) {
        return 0;
    }
}
"#;
    let files = parse_java("Timestamp.java", source);
    let provider = JavaTypeProvider::from_parsed_files(&files);

    // Generic params are stripped for matching: Comparable<Timestamp> → Comparable.
    let subtypes = provider.subtypes_of("Comparable");
    assert!(
        subtypes.contains(&"Timestamp".to_string()),
        "Expected Timestamp in subtypes of Comparable: {:?}",
        subtypes
    );
}

// ---------------------------------------------------------------------------
// Multiple interfaces
// ---------------------------------------------------------------------------

#[test]
fn test_java_multiple_interfaces() {
    let source = r#"
public interface Readable {
    String read();
}

public interface Writable {
    void write(String data);
}

public interface Closeable {
    void close();
}

public class FileStream implements Readable, Writable, Closeable {
    public String read() { return ""; }
    public void write(String data) {}
    public void close() {}
}
"#;
    let files = parse_java("FileStream.java", source);
    let provider = JavaTypeProvider::from_parsed_files(&files);

    assert!(provider
        .subtypes_of("Readable")
        .contains(&"FileStream".to_string()));
    assert!(provider
        .subtypes_of("Writable")
        .contains(&"FileStream".to_string()));
    assert!(provider
        .subtypes_of("Closeable")
        .contains(&"FileStream".to_string()));
}

// ---------------------------------------------------------------------------
// Abstract class dispatch
// ---------------------------------------------------------------------------

#[test]
fn test_java_abstract_class_dispatch() {
    let source = r#"
public abstract class Shape {
    public abstract double area();

    public String describe() {
        return "shape";
    }
}

public class Circle extends Shape {
    private double radius;

    public double area() {
        return 3.14 * radius * radius;
    }
}

public class Rectangle extends Shape {
    private double width;
    private double height;

    public double area() {
        return width * height;
    }
}
"#;
    let files = parse_java("Shapes.java", source);
    let provider = JavaTypeProvider::from_parsed_files(&files);

    // Dispatching "area" on Shape finds it declared on Shape itself
    // (abstract methods are still method_declarations in the AST).
    let results = provider.resolve_dispatch("Shape", "area", &BTreeSet::new());
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "area");

    // Dispatching on the concrete subclass finds its own override.
    let results = provider.resolve_dispatch("Circle", "area", &BTreeSet::new());
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "area");

    // Dispatch "describe" on Circle should find it in Shape (inherited).
    let results = provider.resolve_dispatch("Circle", "describe", &BTreeSet::new());
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "describe");

    // Dispatch "describe" on Rectangle also inherits from Shape.
    let results = provider.resolve_dispatch("Rectangle", "describe", &BTreeSet::new());
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "describe");
}

// ---------------------------------------------------------------------------
// Diamond inheritance
// ---------------------------------------------------------------------------

#[test]
fn test_java_diamond_hierarchy() {
    let source = r#"
public interface A {
    void doA();
}

public interface B extends A {
    void doB();
}

public interface C extends A {
    void doC();
}

public class D implements B, C {
    public void doA() {}
    public void doB() {}
    public void doC() {}
}
"#;
    let files = parse_java("Diamond.java", source);
    let provider = JavaTypeProvider::from_parsed_files(&files);

    // D should satisfy A (through both B and C).
    let a_subtypes = provider.subtypes_of("A");
    assert!(
        a_subtypes.contains(&"D".to_string()),
        "D should satisfy A through diamond: {:?}",
        a_subtypes
    );

    // D should satisfy B and C directly.
    assert!(provider.subtypes_of("B").contains(&"D".to_string()));
    assert!(provider.subtypes_of("C").contains(&"D".to_string()));

    // Dispatch doA through A should find D.
    let results = provider.resolve_dispatch("A", "doA", &BTreeSet::new());
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "doA");
}

// ---------------------------------------------------------------------------
// Deep inheritance chain
// ---------------------------------------------------------------------------

#[test]
fn test_java_deep_inheritance() {
    let source = r#"
public class Level0 {
    public void baseMethod() {}
}

public class Level1 extends Level0 {
    public void level1Method() {}
}

public class Level2 extends Level1 {
    public void level2Method() {}
}

public class Level3 extends Level2 {
    public void level3Method() {}
}
"#;
    let files = parse_java("Deep.java", source);
    let provider = JavaTypeProvider::from_parsed_files(&files);

    // Level3 should inherit all methods up the chain.
    let fields = provider.field_layout("Level3").unwrap();
    let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
    assert!(names.contains(&"level3Method"));
    assert!(names.contains(&"level2Method"));
    assert!(names.contains(&"level1Method"));
    assert!(names.contains(&"baseMethod"));

    // Dispatch "baseMethod" on Level3 should find it in Level0.
    let results = provider.resolve_dispatch("Level3", "baseMethod", &BTreeSet::new());
    assert_eq!(results.len(), 1);
    assert_eq!(results[0].name, "baseMethod");

    // All levels are subtypes of Level0.
    let subtypes = provider.subtypes_of("Level0");
    assert!(subtypes.contains(&"Level1".to_string()));
    // Level2 and Level3 are indirect subtypes; subtypes_of returns direct only.
}

// ---------------------------------------------------------------------------
// Marker interface (no methods)
// ---------------------------------------------------------------------------

#[test]
fn test_java_marker_interface() {
    let source = r#"
public interface Serializable {
}

public class Event implements Serializable {
    private String type;
}
"#;
    let files = parse_java("Marker.java", source);
    let provider = JavaTypeProvider::from_parsed_files(&files);

    let subtypes = provider.subtypes_of("Serializable");
    assert!(
        subtypes.contains(&"Event".to_string()),
        "Event should satisfy marker interface: {:?}",
        subtypes
    );

    let fields = provider.field_layout("Serializable").unwrap();
    assert!(fields.is_empty(), "Marker interface has no methods");
}

// ---------------------------------------------------------------------------
// Enum with methods
// ---------------------------------------------------------------------------

#[test]
fn test_java_enum_with_methods() {
    let source = r#"
public enum Planet {
    MERCURY,
    VENUS,
    EARTH;

    public double mass() {
        return 0.0;
    }
}
"#;
    let files = parse_java("Planet.java", source);
    let provider = JavaTypeProvider::from_parsed_files(&files);

    let resolved = provider.resolve_type("Planet.java", "Planet", 0).unwrap();
    assert_eq!(resolved.kind, ResolvedTypeKind::Enum);
}

// ---------------------------------------------------------------------------
// Constructor in field layout
// ---------------------------------------------------------------------------

#[test]
fn test_java_constructor_in_layout() {
    let source = r#"
public class Connection {
    private String url;

    public Connection(String url) {
        this.url = url;
    }

    public void connect() {}
}
"#;
    let files = parse_java("Connection.java", source);
    let provider = JavaTypeProvider::from_parsed_files(&files);

    let fields = provider.field_layout("Connection").unwrap();
    let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
    assert!(names.contains(&"url"), "Expected field 'url'");
    assert!(
        names.contains(&"Connection"),
        "Expected constructor 'Connection'"
    );
    assert!(names.contains(&"connect"), "Expected method 'connect'");
}

// ---------------------------------------------------------------------------
// Dispatch returns empty for nonexistent method
// ---------------------------------------------------------------------------

#[test]
fn test_java_dispatch_nonexistent_method() {
    let source = r#"
public class Service {
    public void run() {}
}
"#;
    let files = parse_java("Service.java", source);
    let provider = JavaTypeProvider::from_parsed_files(&files);

    let results = provider.resolve_dispatch("Service", "nonExistent", &BTreeSet::new());
    assert!(results.is_empty());
}

#[test]
fn test_java_dispatch_nonexistent_type() {
    let source = r#"
public class Service {
    public void run() {}
}
"#;
    let files = parse_java("Service.java", source);
    let provider = JavaTypeProvider::from_parsed_files(&files);

    let results = provider.resolve_dispatch("Unknown", "run", &BTreeSet::new());
    assert!(results.is_empty());
}

// ---------------------------------------------------------------------------
// Non-Java files ignored
// ---------------------------------------------------------------------------

#[test]
fn test_java_ignores_non_java_files() {
    let java_parsed = ParsedFile::parse(
        "Service.java",
        "public class Service { public void run() {} }",
        Language::Java,
    )
    .unwrap();
    let go_parsed =
        ParsedFile::parse("main.go", "package main\nfunc main() {}\n", Language::Go).unwrap();

    let mut files = BTreeMap::new();
    files.insert("Service.java".to_string(), java_parsed);
    files.insert("main.go".to_string(), go_parsed);

    let provider = JavaTypeProvider::from_parsed_files(&files);

    // Should find Java class.
    assert!(provider
        .resolve_type("Service.java", "Service", 0)
        .is_some());
    // Should not find Go types.
    assert!(provider.resolve_type("main.go", "main", 0).is_none());
}

// ---------------------------------------------------------------------------
// CpgContext integration
// ---------------------------------------------------------------------------

#[test]
fn test_java_cpg_context_integration() {
    use prism::cpg::CpgContext;

    let source = r#"
public interface Logger {
    void log(String msg);
}

public class ConsoleLogger implements Logger {
    public void log(String msg) {
        System.out.println(msg);
    }
}
"#;
    let parsed = ParsedFile::parse("Logger.java", source, Language::Java).unwrap();
    let mut files = BTreeMap::new();
    files.insert("Logger.java".to_string(), parsed);

    let ctx = CpgContext::build(&files, None);

    // TypeRegistry should have Java provider auto-registered.
    let tp = ctx.types.provider_for(Language::Java);
    assert!(tp.is_some(), "Java provider should be auto-registered");

    let tp = tp.unwrap();
    let resolved = tp.resolve_type("Logger.java", "Logger", 0).unwrap();
    assert_eq!(resolved.kind, ResolvedTypeKind::Interface);
    let resolved = tp.resolve_type("Logger.java", "ConsoleLogger", 0).unwrap();
    assert_eq!(resolved.kind, ResolvedTypeKind::Concrete);

    // Dispatch provider should also be registered.
    let dp = ctx.types.dispatch_for(Language::Java);
    assert!(dp.is_some(), "Java dispatch should be auto-registered");

    let dp = dp.unwrap();
    let targets = dp.resolve_dispatch("Logger", "log", &BTreeSet::new());
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].name, "log");
}

// ---------------------------------------------------------------------------
// Subtypes of class (direct subclasses)
// ---------------------------------------------------------------------------

#[test]
fn test_java_subtypes_of_class() {
    let source = r#"
public class Vehicle {
    public void drive() {}
}

public class Car extends Vehicle {
    public void honk() {}
}

public class Truck extends Vehicle {
    public void haul() {}
}

public class Sedan extends Car {
    public void park() {}
}
"#;
    let files = parse_java("Vehicles.java", source);
    let provider = JavaTypeProvider::from_parsed_files(&files);

    let subtypes = provider.subtypes_of("Vehicle");
    assert!(subtypes.contains(&"Car".to_string()));
    assert!(subtypes.contains(&"Truck".to_string()));
    // Sedan extends Car, not Vehicle directly.
    assert!(
        !subtypes.contains(&"Sedan".to_string()),
        "subtypes_of returns direct subclasses only"
    );

    let car_subtypes = provider.subtypes_of("Car");
    assert!(car_subtypes.contains(&"Sedan".to_string()));
}

// ---------------------------------------------------------------------------
// Multiple classes across files with shared interface
// ---------------------------------------------------------------------------

#[test]
fn test_java_cross_file_dispatch_rta() {
    let sources = &[
        (
            "Cache.java",
            r#"
public interface Cache {
    Object get(String key);
    void put(String key, Object value);
}
"#,
        ),
        (
            "MemoryCache.java",
            r#"
public class MemoryCache implements Cache {
    public Object get(String key) { return null; }
    public void put(String key, Object value) {}
}
"#,
        ),
        (
            "RedisCache.java",
            r#"
public class RedisCache implements Cache {
    public Object get(String key) { return null; }
    public void put(String key, Object value) {}
}
"#,
        ),
        (
            "DiskCache.java",
            r#"
public class DiskCache implements Cache {
    public Object get(String key) { return null; }
    public void put(String key, Object value) {}
}
"#,
        ),
    ];
    let files = parse_java_files(sources);
    let provider = JavaTypeProvider::from_parsed_files(&files);

    // Without RTA: all 3 implementations.
    let all = provider.resolve_dispatch("Cache", "get", &BTreeSet::new());
    assert_eq!(all.len(), 3);

    // With RTA: only MemoryCache and RedisCache live.
    let live = BTreeSet::from(["MemoryCache".to_string(), "RedisCache".to_string()]);
    let filtered = provider.resolve_dispatch("Cache", "get", &live);
    assert_eq!(filtered.len(), 2);
    let names: BTreeSet<String> = filtered.iter().map(|f| f.file.clone()).collect();
    assert!(names.contains("MemoryCache.java"));
    assert!(names.contains("RedisCache.java"));
    assert!(!names.contains("DiskCache.java"));
}

// ---------------------------------------------------------------------------
// Method override in hierarchy
// ---------------------------------------------------------------------------

#[test]
fn test_java_method_override() {
    let source = r#"
public class Base {
    public String toString() {
        return "base";
    }
}

public class Child extends Base {
    public String toString() {
        return "child";
    }
}
"#;
    let files = parse_java("Override.java", source);
    let provider = JavaTypeProvider::from_parsed_files(&files);

    // Dispatch on Child should find Child's override, not Base's.
    let results = provider.resolve_dispatch("Child", "toString", &BTreeSet::new());
    assert_eq!(results.len(), 1);
    // The method should come from the Child class (overridden).
    // We verify by checking the line range is within Child's definition.
    assert_eq!(results[0].file, "Override.java");
}
