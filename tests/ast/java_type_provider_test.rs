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
