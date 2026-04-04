//! Tests for multi-language type providers (E12).
//!
//! Tests the TypeProvider, DispatchProvider traits and TypeRegistry integration
//! for Go (struct/interface extraction, method sets, interface satisfaction,
//! dispatch resolution) and the CppTypeProvider Arc-based sharing.

use prism::ast::ParsedFile;
use prism::languages::Language;
use prism::type_provider::{LanguageVersion, TypeProvider};
use prism::type_providers::go::GoTypeProvider;
use std::collections::{BTreeMap, BTreeSet};

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

fn parse_go(path: &str, source: &str) -> BTreeMap<String, ParsedFile> {
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    files
}

// ---------------------------------------------------------------------------
// Go struct extraction
// ---------------------------------------------------------------------------

#[test]
fn test_go_struct_extraction() {
    let source = r#"
package main

type Config struct {
    Host string
    Port int
    Debug bool
}
"#;
    let files = parse_go("config.go", source);
    let provider = GoTypeProvider::from_parsed_files(&files);

    // Should find the Config struct
    let resolved = provider.resolve_type("config.go", "Config", 0);
    assert!(resolved.is_some());
    let rt = resolved.unwrap();
    assert_eq!(rt.name, "Config");
    assert_eq!(rt.kind, prism::type_provider::ResolvedTypeKind::Concrete);

    // Should have correct field layout
    let fields = provider.field_layout("Config").unwrap();
    assert_eq!(fields.len(), 3);
    assert_eq!(fields[0].name, "Host");
    assert_eq!(fields[0].type_str, "string");
    assert_eq!(fields[1].name, "Port");
    assert_eq!(fields[1].type_str, "int");
    assert_eq!(fields[2].name, "Debug");
    assert_eq!(fields[2].type_str, "bool");
}

#[test]
fn test_go_struct_with_pointer_fields() {
    let source = r#"
package main

type Node struct {
    Value int
    Next  *Node
    Prev  *Node
}
"#;
    let files = parse_go("node.go", source);
    let provider = GoTypeProvider::from_parsed_files(&files);

    let fields = provider.field_layout("Node").unwrap();
    assert_eq!(fields.len(), 3);
    assert_eq!(fields[0].name, "Value");
    assert_eq!(fields[1].name, "Next");
    assert_eq!(fields[1].type_str, "*Node");
    assert_eq!(fields[2].name, "Prev");
}

// ---------------------------------------------------------------------------
// Go interface extraction
// ---------------------------------------------------------------------------

#[test]
fn test_go_interface_extraction() {
    let source = r#"
package main

type Reader interface {
    Read(p []byte) (n int, err error)
}

type Writer interface {
    Write(p []byte) (n int, err error)
}
"#;
    let files = parse_go("io.go", source);
    let provider = GoTypeProvider::from_parsed_files(&files);

    let resolved = provider.resolve_type("io.go", "Reader", 0);
    assert!(resolved.is_some());
    assert_eq!(
        resolved.unwrap().kind,
        prism::type_provider::ResolvedTypeKind::Interface
    );

    let resolved_w = provider.resolve_type("io.go", "Writer", 0);
    assert!(resolved_w.is_some());
    assert_eq!(
        resolved_w.unwrap().kind,
        prism::type_provider::ResolvedTypeKind::Interface
    );

    // Unknown type returns None
    assert!(provider.resolve_type("io.go", "NonExistent", 0).is_none());
}

// ---------------------------------------------------------------------------
// Go embedded struct fields
// ---------------------------------------------------------------------------

#[test]
fn test_go_embedded_struct() {
    let source = r#"
package main

type Base struct {
    ID   int
    Name string
}

type Extended struct {
    Base
    Extra string
}
"#;
    let files = parse_go("embed.go", source);
    let provider = GoTypeProvider::from_parsed_files(&files);

    // Extended should have its own fields plus the embedded type
    let fields = provider.field_layout("Extended").unwrap();
    // Should contain Base (embedded) and Extra
    let field_names: Vec<&str> = fields.iter().map(|f| f.name.as_str()).collect();
    assert!(field_names.contains(&"Base")); // embedded appears as a field
    assert!(field_names.contains(&"Extra"));
}

// ---------------------------------------------------------------------------
// Go interface satisfaction
// ---------------------------------------------------------------------------

#[test]
fn test_go_interface_satisfaction() {
    let source = r#"
package main

type Reader interface {
    Read(p []byte) (n int, err error)
}

type Closer interface {
    Close() error
}

type ReadCloser interface {
    Read(p []byte) (n int, err error)
    Close() error
}

type File struct {
    path string
}

func (f *File) Read(p []byte) (n int, err error) {
    return 0, nil
}

func (f *File) Close() error {
    return nil
}

func (f *File) Name() string {
    return f.path
}

type Buffer struct {
    data []byte
}

func (b *Buffer) Read(p []byte) (n int, err error) {
    return 0, nil
}
"#;
    let files = parse_go("satisfy.go", source);
    let provider = GoTypeProvider::from_parsed_files(&files);

    // File satisfies Reader (has Read method)
    let reader_subtypes = provider.subtypes_of("Reader");
    assert!(
        reader_subtypes.contains(&"File".to_string()),
        "File should satisfy Reader, got: {:?}",
        reader_subtypes
    );
    // Buffer also satisfies Reader
    assert!(
        reader_subtypes.contains(&"Buffer".to_string()),
        "Buffer should satisfy Reader, got: {:?}",
        reader_subtypes
    );

    // File satisfies Closer (has Close method)
    let closer_subtypes = provider.subtypes_of("Closer");
    assert!(
        closer_subtypes.contains(&"File".to_string()),
        "File should satisfy Closer"
    );
    // Buffer does NOT satisfy Closer (no Close method)
    assert!(
        !closer_subtypes.contains(&"Buffer".to_string()),
        "Buffer should NOT satisfy Closer"
    );

    // File satisfies ReadCloser (has both Read and Close)
    let rc_subtypes = provider.subtypes_of("ReadCloser");
    assert!(
        rc_subtypes.contains(&"File".to_string()),
        "File should satisfy ReadCloser"
    );
    // Buffer does NOT satisfy ReadCloser (missing Close)
    assert!(
        !rc_subtypes.contains(&"Buffer".to_string()),
        "Buffer should NOT satisfy ReadCloser"
    );
}

// ---------------------------------------------------------------------------
// Go embedded interface
// ---------------------------------------------------------------------------

#[test]
fn test_go_embedded_interface() {
    let source = r#"
package main

type Reader interface {
    Read(p []byte) (n int, err error)
}

type Closer interface {
    Close() error
}

type ReadCloser interface {
    Reader
    Closer
}

type MyFile struct{}

func (f *MyFile) Read(p []byte) (n int, err error) {
    return 0, nil
}

func (f *MyFile) Close() error {
    return nil
}

type MyReader struct{}

func (r *MyReader) Read(p []byte) (n int, err error) {
    return 0, nil
}
"#;
    let files = parse_go("embedded_iface.go", source);
    let provider = GoTypeProvider::from_parsed_files(&files);

    // ReadCloser embeds Reader and Closer.
    // MyFile has both Read and Close → satisfies ReadCloser.
    let rc_subtypes = provider.subtypes_of("ReadCloser");
    assert!(
        rc_subtypes.contains(&"MyFile".to_string()),
        "MyFile should satisfy ReadCloser (embedded interface), got: {:?}",
        rc_subtypes
    );

    // MyReader only has Read → satisfies Reader but NOT ReadCloser.
    assert!(
        !rc_subtypes.contains(&"MyReader".to_string()),
        "MyReader should NOT satisfy ReadCloser"
    );
    let reader_subtypes = provider.subtypes_of("Reader");
    assert!(
        reader_subtypes.contains(&"MyReader".to_string()),
        "MyReader should satisfy Reader"
    );
}

// ---------------------------------------------------------------------------
// Go dispatch resolution
// ---------------------------------------------------------------------------

#[test]
fn test_go_dispatch_concrete() {
    use prism::type_provider::DispatchProvider;

    let source = r#"
package main

type Server struct{}

func (s *Server) Start() error {
    return nil
}

func (s *Server) Stop() error {
    return nil
}
"#;
    let files = parse_go("server.go", source);
    let provider = GoTypeProvider::from_parsed_files(&files);

    // Direct dispatch on concrete type
    let targets = provider.resolve_dispatch("Server", "Start", &BTreeSet::new());
    assert_eq!(targets.len(), 1);
    assert_eq!(targets[0].name, "Start");
    assert_eq!(targets[0].file, "server.go");

    // Method not found
    let none = provider.resolve_dispatch("Server", "NonExistent", &BTreeSet::new());
    assert!(none.is_empty());
}

#[test]
fn test_go_dispatch_interface() {
    use prism::type_provider::DispatchProvider;

    let source = r#"
package main

type Handler interface {
    Handle(req string) string
}

type UserHandler struct{}

func (h *UserHandler) Handle(req string) string {
    return "user"
}

type AdminHandler struct{}

func (h *AdminHandler) Handle(req string) string {
    return "admin"
}
"#;
    let files = parse_go("handler.go", source);
    let provider = GoTypeProvider::from_parsed_files(&files);

    // Dispatch on interface — should resolve to both concrete types
    let targets = provider.resolve_dispatch("Handler", "Handle", &BTreeSet::new());
    assert_eq!(targets.len(), 2);
    let target_names: BTreeSet<String> = targets.iter().map(|t| t.file.clone()).collect();
    assert!(target_names.contains("handler.go"));
}

#[test]
fn test_go_dispatch_rta_filtering() {
    use prism::type_provider::DispatchProvider;

    let source = r#"
package main

type Shape interface {
    Area() float64
}

type Circle struct{}

func (c *Circle) Area() float64 { return 0 }

type Square struct{}

func (s *Square) Area() float64 { return 0 }

type Triangle struct{}

func (t *Triangle) Area() float64 { return 0 }
"#;
    let files = parse_go("shapes.go", source);
    let provider = GoTypeProvider::from_parsed_files(&files);

    // Without RTA: all 3 types
    let all = provider.resolve_dispatch("Shape", "Area", &BTreeSet::new());
    assert_eq!(all.len(), 3);

    // With RTA: only Circle and Square are live
    let mut live = BTreeSet::new();
    live.insert("Circle".to_string());
    live.insert("Square".to_string());
    let filtered = provider.resolve_dispatch("Shape", "Area", &live);
    assert_eq!(filtered.len(), 2);
}

// ---------------------------------------------------------------------------
// Go type aliases
// ---------------------------------------------------------------------------

#[test]
fn test_go_type_alias() {
    let source = r#"
package main

type MyString = string
type Handler func(string) error
"#;
    let files = parse_go("alias.go", source);
    let provider = GoTypeProvider::from_parsed_files(&files);

    assert_eq!(provider.resolve_alias("MyString"), "string");
    // Non-alias types return themselves
    assert_eq!(provider.resolve_alias("int"), "int");

    let resolved = provider.resolve_type("alias.go", "MyString", 0);
    assert!(resolved.is_some());
    assert_eq!(
        resolved.unwrap().kind,
        prism::type_provider::ResolvedTypeKind::Alias
    );
}

// ---------------------------------------------------------------------------
// Go promoted methods from embedded structs
// ---------------------------------------------------------------------------

#[test]
fn test_go_promoted_methods() {
    let source = r#"
package main

type Logger interface {
    Log(msg string)
}

type BaseService struct{}

func (b *BaseService) Log(msg string) {}

type UserService struct {
    BaseService
}
"#;
    let files = parse_go("promote.go", source);
    let provider = GoTypeProvider::from_parsed_files(&files);

    // UserService embeds BaseService which has Log().
    // UserService should satisfy Logger via promoted method.
    let subtypes = provider.subtypes_of("Logger");
    assert!(
        subtypes.contains(&"UserService".to_string()),
        "UserService should satisfy Logger via promoted Log(), got: {:?}",
        subtypes
    );
    assert!(subtypes.contains(&"BaseService".to_string()));
}

// ---------------------------------------------------------------------------
// Multi-file Go type resolution
// ---------------------------------------------------------------------------

#[test]
fn test_go_multi_file() {
    let source1 = r#"
package main

type Repository interface {
    Find(id int) string
    Save(data string) error
}
"#;
    let source2 = r#"
package main

type UserRepo struct{}

func (r *UserRepo) Find(id int) string {
    return ""
}

func (r *UserRepo) Save(data string) error {
    return nil
}
"#;
    let mut files = BTreeMap::new();
    files.insert(
        "repo.go".to_string(),
        ParsedFile::parse("repo.go", source1, Language::Go).unwrap(),
    );
    files.insert(
        "user_repo.go".to_string(),
        ParsedFile::parse("user_repo.go", source2, Language::Go).unwrap(),
    );

    let provider = GoTypeProvider::from_parsed_files(&files);

    // UserRepo (defined in file 2) should satisfy Repository (defined in file 1)
    let subtypes = provider.subtypes_of("Repository");
    assert!(
        subtypes.contains(&"UserRepo".to_string()),
        "UserRepo should satisfy Repository across files, got: {:?}",
        subtypes
    );
}

// ---------------------------------------------------------------------------
// GoTypeProvider languages()
// ---------------------------------------------------------------------------

#[test]
fn test_go_provider_languages() {
    let files = BTreeMap::new();
    let provider = GoTypeProvider::from_parsed_files(&files);
    assert_eq!(provider.languages(), vec![Language::Go]);
}

// ---------------------------------------------------------------------------
// TypeRegistry integration
// ---------------------------------------------------------------------------

#[test]
fn test_registry_go_integration() {
    use prism::cpg::CpgContext;

    let source = r#"
package main

type Handler interface {
    Handle() error
}

type MyHandler struct{}

func (h *MyHandler) Handle() error {
    return nil
}
"#;
    let mut files = BTreeMap::new();
    files.insert(
        "main.go".to_string(),
        ParsedFile::parse("main.go", source, Language::Go).unwrap(),
    );

    // Build CpgContext — should auto-register GoTypeProvider.
    let ctx = CpgContext::build(&files, None);

    // The registry should have a Go provider.
    let go_provider = ctx.types.provider_for(Language::Go);
    assert!(
        go_provider.is_some(),
        "GoTypeProvider should be auto-registered for Go files"
    );

    // Should resolve Go types through the registry.
    let provider = go_provider.unwrap();
    let resolved = provider.resolve_type("main.go", "Handler", 0);
    assert!(resolved.is_some());
    assert_eq!(
        resolved.unwrap().kind,
        prism::type_provider::ResolvedTypeKind::Interface
    );
}

// ---------------------------------------------------------------------------
// LanguageVersion parsing
// ---------------------------------------------------------------------------

#[test]
fn test_language_version_parsing() {
    let v = LanguageVersion::parse("3.8").unwrap();
    assert_eq!(v.major, 3);
    assert_eq!(v.minor, 8);
    assert_eq!(v.version, "3.8");

    let v = LanguageVersion::parse("18").unwrap();
    assert_eq!(v.major, 18);
    assert_eq!(v.minor, 0);

    let v = LanguageVersion::parse("1.21").unwrap();
    assert_eq!(v.major, 1);
    assert_eq!(v.minor, 21);

    assert!(LanguageVersion::parse("").is_none());
    assert!(LanguageVersion::parse("abc").is_none());
}

// ---------------------------------------------------------------------------
// CppTypeProvider Arc sharing
// ---------------------------------------------------------------------------

#[test]
fn test_cpp_provider_arc_sharing() {
    use prism::type_db::TypeDatabase;
    use prism::type_providers::cpp::CppTypeProvider;

    let db = TypeDatabase::default();
    let provider = CppTypeProvider::new(db);

    // Clone should share the same Arc
    let clone = provider.clone();
    assert!(std::sync::Arc::ptr_eq(&provider.db, &clone.db));
}

// ---------------------------------------------------------------------------
// Empty interface (any)
// ---------------------------------------------------------------------------

#[test]
fn test_go_empty_interface() {
    let source = r#"
package main

type Any interface{}

type Concrete struct{}

func (c *Concrete) Method() {}
"#;
    let files = parse_go("any.go", source);
    let provider = GoTypeProvider::from_parsed_files(&files);

    // Empty interface should be satisfied by all concrete types
    let subtypes = provider.subtypes_of("Any");
    assert!(
        subtypes.contains(&"Concrete".to_string()),
        "Every type satisfies empty interface, got: {:?}",
        subtypes
    );
}
