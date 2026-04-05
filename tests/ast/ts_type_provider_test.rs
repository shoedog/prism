// Tests for the TypeScript type provider (Phase 3 of E12).
//
// Covers: interface extraction, class extraction, type aliases, enums,
// interface satisfaction (nominal + structural), dispatch resolution,
// structural typing compatibility, Arc sharing, and registry integration.

use prism::ast::ParsedFile;
use prism::languages::Language;
use prism::type_provider::{
    Compatibility, DispatchProvider, ResolvedTypeKind, StructuralTypingProvider, TypeProvider,
};
use prism::type_providers::typescript::TypeScriptTypeProvider;
use std::collections::{BTreeMap, BTreeSet};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_ts(filename: &str, source: &str) -> BTreeMap<String, ParsedFile> {
    let parsed = ParsedFile::parse(filename, source, Language::TypeScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(filename.to_string(), parsed);
    files
}

fn parse_tsx(filename: &str, source: &str) -> BTreeMap<String, ParsedFile> {
    let parsed = ParsedFile::parse(filename, source, Language::Tsx).unwrap();
    let mut files = BTreeMap::new();
    files.insert(filename.to_string(), parsed);
    files
}

// ---------------------------------------------------------------------------
// Interface extraction
// ---------------------------------------------------------------------------

#[test]
fn test_ts_interface_extraction() {
    let source = r#"
interface Config {
    baseUrl: string;
    retries: number;
}
"#;
    let files = parse_ts("config.ts", source);
    let provider = TypeScriptTypeProvider::from_parsed_files(&files);

    let resolved = provider.resolve_type("config.ts", "Config", 0).unwrap();
    assert_eq!(resolved.kind, ResolvedTypeKind::Interface);
    assert_eq!(resolved.name, "Config");

    let fields = provider.field_layout("Config").unwrap();
    let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
    assert!(
        names.contains(&"baseUrl"),
        "Expected baseUrl in {:?}",
        names
    );
    assert!(
        names.contains(&"retries"),
        "Expected retries in {:?}",
        names
    );
}

#[test]
fn test_ts_interface_with_methods() {
    let source = r#"
interface Repository {
    find(id: string): Promise<User>;
    save(user: User): void;
}
"#;
    let files = parse_ts("repo.ts", source);
    let provider = TypeScriptTypeProvider::from_parsed_files(&files);

    let resolved = provider.resolve_type("repo.ts", "Repository", 0).unwrap();
    assert_eq!(resolved.kind, ResolvedTypeKind::Interface);

    let fields = provider.field_layout("Repository").unwrap();
    let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
    assert!(names.contains(&"find"), "Expected find in {:?}", names);
    assert!(names.contains(&"save"), "Expected save in {:?}", names);
}

#[test]
fn test_ts_interface_extends() {
    let source = r#"
interface Base {
    id: string;
}

interface Extended extends Base {
    name: string;
}

class MyEntity {
    id: string;
    name: string;

    greet(): string { return this.name; }
}
"#;
    let files = parse_ts("entity.ts", source);
    let provider = TypeScriptTypeProvider::from_parsed_files(&files);

    // Extended interface should flatten Base's properties for satisfaction.
    let subtypes = provider.subtypes_of("Extended");
    assert!(
        subtypes.contains(&"MyEntity".to_string()),
        "MyEntity should satisfy Extended (has id + name), got: {:?}",
        subtypes
    );

    // Also satisfies Base.
    let base_subtypes = provider.subtypes_of("Base");
    assert!(
        base_subtypes.contains(&"MyEntity".to_string()),
        "MyEntity should satisfy Base (has id), got: {:?}",
        base_subtypes
    );
}

#[test]
fn test_ts_field_layout_resolves_extends() {
    let source = r#"
interface Base {
    id: string;
}

interface Extended extends Base {
    name: string;
}
"#;
    let files = parse_ts("extends.ts", source);
    let provider = TypeScriptTypeProvider::from_parsed_files(&files);

    // field_layout("Extended") should include both own and inherited properties.
    let fields = provider.field_layout("Extended").unwrap();
    let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
    assert!(
        names.contains(&"id"),
        "Extended should inherit id from Base, got: {:?}",
        names
    );
    assert!(
        names.contains(&"name"),
        "Extended should have own name property, got: {:?}",
        names
    );
}

#[test]
fn test_ts_structural_compat_with_extends() {
    let source = r#"
interface Base {
    id: string;
}

interface Extended extends Base {
    name: string;
}

class Full {
    id: string;
    name: string;
}

class Partial {
    name: string;
}
"#;
    let files = parse_ts("compat.ts", source);
    let provider = TypeScriptTypeProvider::from_parsed_files(&files);

    let extended = provider.resolve_type("compat.ts", "Extended", 0).unwrap();
    let full = provider.resolve_type("compat.ts", "Full", 0).unwrap();
    let partial = provider.resolve_type("compat.ts", "Partial", 0).unwrap();

    // Full has both id and name → compatible with Extended.
    assert_eq!(
        provider.is_assignable_to(&full, &extended),
        Compatibility::Compatible
    );

    // Partial is missing id (inherited from Base) → incompatible.
    match provider.is_assignable_to(&partial, &extended) {
        Compatibility::Incompatible { reason } => {
            assert!(
                reason.contains("id"),
                "Should report missing 'id', got: {}",
                reason
            );
        }
        other => panic!("Expected Incompatible, got: {:?}", other),
    }
}

// ---------------------------------------------------------------------------
// Class extraction
// ---------------------------------------------------------------------------

#[test]
fn test_ts_class_extraction() {
    let source = r#"
class UserService {
    private db: Database;

    findUser(id: string): User {
        return this.db.find(id);
    }

    saveUser(user: User): void {
        this.db.save(user);
    }
}
"#;
    let files = parse_ts("service.ts", source);
    let provider = TypeScriptTypeProvider::from_parsed_files(&files);

    let resolved = provider
        .resolve_type("service.ts", "UserService", 0)
        .unwrap();
    assert_eq!(resolved.kind, ResolvedTypeKind::Concrete);
    assert_eq!(resolved.name, "UserService");

    let fields = provider.field_layout("UserService").unwrap();
    let names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
    assert!(
        names.contains(&"findUser"),
        "Expected findUser in {:?}",
        names
    );
    assert!(
        names.contains(&"saveUser"),
        "Expected saveUser in {:?}",
        names
    );
}

#[test]
fn test_ts_class_implements() {
    let source = r#"
interface Serializable {
    serialize(): string;
}

class Config implements Serializable {
    serialize(): string {
        return JSON.stringify(this);
    }
}
"#;
    let files = parse_ts("config.ts", source);
    let provider = TypeScriptTypeProvider::from_parsed_files(&files);

    let subtypes = provider.subtypes_of("Serializable");
    assert!(
        subtypes.contains(&"Config".to_string()),
        "Config implements Serializable, got: {:?}",
        subtypes
    );
}

// ---------------------------------------------------------------------------
// Type aliases
// ---------------------------------------------------------------------------

#[test]
fn test_ts_type_alias() {
    let source = r#"
type UserId = string;
type Handler = (req: Request, res: Response) => void;
"#;
    let files = parse_ts("types.ts", source);
    let provider = TypeScriptTypeProvider::from_parsed_files(&files);

    let resolved = provider.resolve_type("types.ts", "UserId", 0).unwrap();
    assert_eq!(resolved.kind, ResolvedTypeKind::Alias);

    assert_eq!(provider.resolve_alias("UserId"), "string");
}

// ---------------------------------------------------------------------------
// Enums
// ---------------------------------------------------------------------------

#[test]
fn test_ts_enum_extraction() {
    let source = r#"
enum Direction {
    Up,
    Down,
    Left,
    Right
}
"#;
    let files = parse_ts("direction.ts", source);
    let provider = TypeScriptTypeProvider::from_parsed_files(&files);

    let resolved = provider
        .resolve_type("direction.ts", "Direction", 0)
        .unwrap();
    assert_eq!(resolved.kind, ResolvedTypeKind::Enum);
    assert_eq!(resolved.name, "Direction");
}

// ---------------------------------------------------------------------------
// Structural satisfaction
// ---------------------------------------------------------------------------

#[test]
fn test_ts_structural_satisfaction() {
    let source = r#"
interface Printable {
    toString(): string;
}

class Document {
    title: string;

    toString(): string {
        return this.title;
    }
}
"#;
    let files = parse_ts("doc.ts", source);
    let provider = TypeScriptTypeProvider::from_parsed_files(&files);

    // Document structurally satisfies Printable (has toString method).
    let subtypes = provider.subtypes_of("Printable");
    assert!(
        subtypes.contains(&"Document".to_string()),
        "Document should structurally satisfy Printable, got: {:?}",
        subtypes
    );
}

#[test]
fn test_ts_structural_satisfaction_missing_method() {
    let source = r#"
interface Closable {
    close(): void;
    flush(): void;
}

class SimpleWriter {
    close(): void {}
}
"#;
    let files = parse_ts("writer.ts", source);
    let provider = TypeScriptTypeProvider::from_parsed_files(&files);

    // SimpleWriter is missing flush() — should NOT satisfy Closable.
    let subtypes = provider.subtypes_of("Closable");
    assert!(
        !subtypes.contains(&"SimpleWriter".to_string()),
        "SimpleWriter should NOT satisfy Closable (missing flush), got: {:?}",
        subtypes
    );
}

// ---------------------------------------------------------------------------
// Dispatch resolution
// ---------------------------------------------------------------------------

#[test]
fn test_ts_dispatch_concrete() {
    let source = r#"
class Calculator {
    add(a: number, b: number): number {
        return a + b;
    }

    multiply(a: number, b: number): number {
        return a * b;
    }
}
"#;
    let files = parse_ts("calc.ts", source);
    let provider = TypeScriptTypeProvider::from_parsed_files(&files);

    let targets = provider.resolve_dispatch("Calculator", "add", &BTreeSet::new());
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].name, "add");
    assert_eq!(targets[0].file, "calc.ts");
}

#[test]
fn test_ts_dispatch_interface() {
    let source = r#"
interface Storage {
    get(key: string): string;
    set(key: string, value: string): void;
}

class MemoryStorage implements Storage {
    get(key: string): string { return ""; }
    set(key: string, value: string): void {}
}

class DiskStorage implements Storage {
    get(key: string): string { return ""; }
    set(key: string, value: string): void {}
}
"#;
    let files = parse_ts("storage.ts", source);
    let provider = TypeScriptTypeProvider::from_parsed_files(&files);

    let targets = provider.resolve_dispatch("Storage", "get", &BTreeSet::new());
    assert_eq!(
        targets.len(),
        2,
        "Expected 2 dispatch targets for Storage.get, got: {:?}",
        targets
    );
    let target_names: BTreeSet<String> = targets.iter().map(|t| t.file.clone()).collect();
    assert!(target_names.contains("storage.ts"));
}

#[test]
fn test_ts_dispatch_rta_filtering() {
    let source = r#"
interface Logger {
    log(msg: string): void;
}

class ConsoleLogger implements Logger {
    log(msg: string): void {}
}

class FileLogger implements Logger {
    log(msg: string): void {}
}

class NetworkLogger implements Logger {
    log(msg: string): void {}
}
"#;
    let files = parse_ts("logger.ts", source);
    let provider = TypeScriptTypeProvider::from_parsed_files(&files);

    // RTA: only ConsoleLogger is live.
    let live = BTreeSet::from(["ConsoleLogger".to_string()]);
    let targets = provider.resolve_dispatch("Logger", "log", &live);
    assert_eq!(targets.len(), 1, "RTA should filter to 1 target");
    assert_eq!(targets[0].name, "log");
}

// ---------------------------------------------------------------------------
// Structural typing (is_assignable_to)
// ---------------------------------------------------------------------------

#[test]
fn test_ts_structural_compatible() {
    let source = r#"
interface Point {
    x: number;
    y: number;
}

class Coordinate {
    x: number;
    y: number;
    z: number;
}
"#;
    let files = parse_ts("point.ts", source);
    let provider = TypeScriptTypeProvider::from_parsed_files(&files);

    let point = provider.resolve_type("point.ts", "Point", 0).unwrap();
    let coord = provider.resolve_type("point.ts", "Coordinate", 0).unwrap();

    // Coordinate has x and y → assignable to Point.
    let result = provider.is_assignable_to(&coord, &point);
    assert_eq!(result, Compatibility::Compatible);
}

#[test]
fn test_ts_structural_incompatible() {
    let source = r#"
interface Named {
    firstName: string;
    lastName: string;
}

class Identifier {
    id: number;
}
"#;
    let files = parse_ts("named.ts", source);
    let provider = TypeScriptTypeProvider::from_parsed_files(&files);

    let named = provider.resolve_type("named.ts", "Named", 0).unwrap();
    let ident = provider.resolve_type("named.ts", "Identifier", 0).unwrap();

    let result = provider.is_assignable_to(&ident, &named);
    match result {
        Compatibility::Incompatible { reason } => {
            assert!(
                reason.contains("firstName"),
                "Expected missing firstName in reason: {}",
                reason
            );
        }
        other => panic!("Expected Incompatible, got: {:?}", other),
    }
}

#[test]
fn test_ts_structural_same_type() {
    let source = r#"
interface Config {
    host: string;
}
"#;
    let files = parse_ts("config.ts", source);
    let provider = TypeScriptTypeProvider::from_parsed_files(&files);

    let config = provider.resolve_type("config.ts", "Config", 0).unwrap();
    assert_eq!(
        provider.is_assignable_to(&config, &config),
        Compatibility::Compatible
    );
}

// ---------------------------------------------------------------------------
// Export statements
// ---------------------------------------------------------------------------

#[test]
fn test_ts_exported_interface() {
    let source = r#"
export interface ApiResponse {
    status: number;
    data: any;
}

export class HttpClient {
    fetch(url: string): ApiResponse {
        return { status: 200, data: null };
    }
}
"#;
    let files = parse_ts("api.ts", source);
    let provider = TypeScriptTypeProvider::from_parsed_files(&files);

    let resolved = provider.resolve_type("api.ts", "ApiResponse", 0).unwrap();
    assert_eq!(resolved.kind, ResolvedTypeKind::Interface);

    let resolved = provider.resolve_type("api.ts", "HttpClient", 0).unwrap();
    assert_eq!(resolved.kind, ResolvedTypeKind::Concrete);
}

// ---------------------------------------------------------------------------
// Multi-file
// ---------------------------------------------------------------------------

#[test]
fn test_ts_multi_file() {
    let iface_source = r#"
interface Handler {
    handle(req: Request): Response;
}
"#;
    let impl_source = r#"
class UserHandler {
    handle(req: Request): Response {
        return new Response();
    }
}
"#;

    let parsed1 = ParsedFile::parse("handler.ts", iface_source, Language::TypeScript).unwrap();
    let parsed2 = ParsedFile::parse("user_handler.ts", impl_source, Language::TypeScript).unwrap();

    let mut files = BTreeMap::new();
    files.insert("handler.ts".to_string(), parsed1);
    files.insert("user_handler.ts".to_string(), parsed2);

    let provider = TypeScriptTypeProvider::from_parsed_files(&files);

    // UserHandler structurally satisfies Handler (has handle method).
    let subtypes = provider.subtypes_of("Handler");
    assert!(
        subtypes.contains(&"UserHandler".to_string()),
        "UserHandler should satisfy Handler, got: {:?}",
        subtypes
    );

    // Dispatch through interface.
    let targets = provider.resolve_dispatch("Handler", "handle", &BTreeSet::new());
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].file, "user_handler.ts");
}

// ---------------------------------------------------------------------------
// TSX support
// ---------------------------------------------------------------------------

#[test]
fn test_tsx_file_support() {
    let source = r#"
interface Props {
    name: string;
    onClick: () => void;
}

class Button {
    name: string;
    onClick: () => void;

    render(): string { return ""; }
}
"#;
    let files = parse_tsx("button.tsx", source);
    let provider = TypeScriptTypeProvider::from_parsed_files(&files);

    let resolved = provider.resolve_type("button.tsx", "Props", 0).unwrap();
    assert_eq!(resolved.kind, ResolvedTypeKind::Interface);

    let subtypes = provider.subtypes_of("Props");
    assert!(
        subtypes.contains(&"Button".to_string()),
        "Button should satisfy Props, got: {:?}",
        subtypes
    );
}

// ---------------------------------------------------------------------------
// Arc sharing
// ---------------------------------------------------------------------------

#[test]
fn test_ts_provider_arc_sharing() {
    let files = parse_ts("empty.ts", "const x = 1;\n");
    let provider = TypeScriptTypeProvider::from_parsed_files(&files);

    let clone = provider.clone();
    assert!(std::sync::Arc::ptr_eq(&provider.data, &clone.data));
}

// ---------------------------------------------------------------------------
// Registry integration
// ---------------------------------------------------------------------------

#[test]
fn test_registry_ts_integration() {
    let source = r#"
interface Service {
    start(): void;
}

class MyService implements Service {
    start(): void {}
}
"#;
    let files = parse_ts("service.ts", source);
    let provider = TypeScriptTypeProvider::from_parsed_files(&files);

    let mut registry = prism::type_provider::TypeRegistry::empty();
    let dispatch = provider.clone();
    let structural = provider.clone();
    registry.register_provider(Box::new(provider));
    registry.register_dispatch_provider(Box::new(dispatch));
    registry.register_structural_provider(Box::new(structural));

    // TypeProvider lookup.
    let tp = registry.provider_for(Language::TypeScript).unwrap();
    let resolved = tp.resolve_type("service.ts", "Service", 0).unwrap();
    assert_eq!(resolved.kind, ResolvedTypeKind::Interface);

    // DispatchProvider lookup.
    let dp = registry.dispatch_for(Language::TypeScript).unwrap();
    let targets = dp.resolve_dispatch("Service", "start", &BTreeSet::new());
    assert_eq!(targets.len(), 1);

    // StructuralTypingProvider lookup.
    let sp = registry.structural_for(Language::TypeScript).unwrap();
    let result = sp.resolve_generic("Array", &["string".to_string()]);
    assert!(result.is_none(), "Phase 3: resolve_generic returns None");

    // TSX should also route to the same provider.
    assert!(registry.provider_for(Language::Tsx).is_some());
    assert!(registry.dispatch_for(Language::Tsx).is_some());
    assert!(registry.structural_for(Language::Tsx).is_some());
}

// ---------------------------------------------------------------------------
// resolve_generic returns None (Phase 3)
// ---------------------------------------------------------------------------

#[test]
fn test_ts_resolve_generic_returns_none() {
    let files = parse_ts("empty.ts", "const x = 1;\n");
    let provider = TypeScriptTypeProvider::from_parsed_files(&files);
    assert!(provider
        .resolve_generic("Promise", &["string".to_string()])
        .is_none());
}

// ---------------------------------------------------------------------------
// Unknown type returns None
// ---------------------------------------------------------------------------

#[test]
fn test_ts_unknown_type() {
    let files = parse_ts("empty.ts", "const x = 1;\n");
    let provider = TypeScriptTypeProvider::from_parsed_files(&files);
    assert!(provider
        .resolve_type("empty.ts", "NonExistent", 0)
        .is_none());
    assert!(provider.field_layout("NonExistent").is_none());
    assert!(provider.subtypes_of("NonExistent").is_empty());
    assert_eq!(provider.resolve_alias("NonExistent"), "NonExistent");
}

// ---------------------------------------------------------------------------
// Languages returned
// ---------------------------------------------------------------------------

#[test]
fn test_ts_provider_languages() {
    let files = parse_ts("a.ts", "const x = 1;\n");
    let provider = TypeScriptTypeProvider::from_parsed_files(&files);
    let langs = provider.languages();
    assert!(langs.contains(&Language::TypeScript));
    assert!(langs.contains(&Language::Tsx));
}

// ---------------------------------------------------------------------------
// Empty interface satisfaction
// ---------------------------------------------------------------------------

#[test]
fn test_ts_empty_interface() {
    let source = r#"
interface Any {}

class Foo {
    bar(): void {}
}
"#;
    let files = parse_ts("any.ts", source);
    let provider = TypeScriptTypeProvider::from_parsed_files(&files);

    // Empty interface should be satisfied by all classes.
    let subtypes = provider.subtypes_of("Any");
    assert!(
        subtypes.contains(&"Foo".to_string()),
        "Every class satisfies empty interface, got: {:?}",
        subtypes
    );
}

// ---------------------------------------------------------------------------
// Dispatch RTA fallback
// ---------------------------------------------------------------------------

#[test]
fn test_ts_dispatch_rta_fallback() {
    let source = r#"
interface Processor {
    process(): void;
}

class FastProcessor implements Processor {
    process(): void {}
}
"#;
    let files = parse_ts("proc.ts", source);
    let provider = TypeScriptTypeProvider::from_parsed_files(&files);

    // RTA with no matching live types should fall back to full set.
    let live = BTreeSet::from(["UnrelatedType".to_string()]);
    let results = provider.resolve_dispatch("Processor", "process", &live);
    assert_eq!(results.len(), 1, "Should fall back to full set");
}

// ---------------------------------------------------------------------------
// Dispatch nonexistent method/type
// ---------------------------------------------------------------------------

#[test]
fn test_ts_dispatch_nonexistent_method() {
    let source = r#"
class Service {
    run(): void {}
}
"#;
    let files = parse_ts("svc.ts", source);
    let provider = TypeScriptTypeProvider::from_parsed_files(&files);

    assert!(provider
        .resolve_dispatch("Service", "nonExistent", &BTreeSet::new())
        .is_empty());
    assert!(provider
        .resolve_dispatch("Unknown", "run", &BTreeSet::new())
        .is_empty());
}

// ---------------------------------------------------------------------------
// CpgContext integration
// ---------------------------------------------------------------------------

#[test]
fn test_ts_cpg_context_integration() {
    let source = r#"
interface Logger {
    log(msg: string): void;
}

class ConsoleLogger implements Logger {
    log(msg: string): void {}
}
"#;
    let parsed = ParsedFile::parse("logger.ts", source, Language::TypeScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert("logger.ts".to_string(), parsed);

    let ctx = prism::cpg::CpgContext::build(&files, None);

    let tp = ctx.types.provider_for(Language::TypeScript);
    assert!(tp.is_some(), "TS provider should be auto-registered");
    let tp = tp.unwrap();
    assert_eq!(
        tp.resolve_type("logger.ts", "Logger", 0).unwrap().kind,
        ResolvedTypeKind::Interface
    );

    let dp = ctx.types.dispatch_for(Language::TypeScript);
    assert!(dp.is_some());
    let targets = dp
        .unwrap()
        .resolve_dispatch("Logger", "log", &BTreeSet::new());
    assert_eq!(targets.len(), 1);

    let sp = ctx.types.structural_for(Language::TypeScript);
    assert!(
        sp.is_some(),
        "TS structural provider should be auto-registered"
    );
}

// ---------------------------------------------------------------------------
// Non-TS files ignored
// ---------------------------------------------------------------------------

#[test]
fn test_ts_ignores_non_ts_files() {
    let ts_parsed = ParsedFile::parse(
        "svc.ts",
        "interface Svc { run(): void; }",
        Language::TypeScript,
    )
    .unwrap();
    let go_parsed =
        ParsedFile::parse("main.go", "package main\nfunc main() {}\n", Language::Go).unwrap();

    let mut files = BTreeMap::new();
    files.insert("svc.ts".to_string(), ts_parsed);
    files.insert("main.go".to_string(), go_parsed);

    let provider = TypeScriptTypeProvider::from_parsed_files(&files);
    assert!(provider.resolve_type("svc.ts", "Svc", 0).is_some());
    assert!(provider.resolve_type("main.go", "main", 0).is_none());
}

// ---------------------------------------------------------------------------
// Multiple interface implementation
// ---------------------------------------------------------------------------

#[test]
fn test_ts_multiple_implements() {
    let source = r#"
interface Readable {
    read(): string;
}

interface Writable {
    write(data: string): void;
}

class FileStream implements Readable, Writable {
    read(): string { return ""; }
    write(data: string): void {}
}
"#;
    let files = parse_ts("stream.ts", source);
    let provider = TypeScriptTypeProvider::from_parsed_files(&files);

    assert!(provider
        .subtypes_of("Readable")
        .contains(&"FileStream".to_string()));
    assert!(provider
        .subtypes_of("Writable")
        .contains(&"FileStream".to_string()));
}
