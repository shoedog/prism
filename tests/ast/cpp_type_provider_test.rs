// Tests for the C/C++ type provider wrapper (CppTypeProvider).
//
// Covers every code path in src/type_providers/cpp.rs: constructors,
// resolve_type (struct, class, union, typedef, typedef→record, unknown),
// field_layout (with fields, unknown type), subtypes_of (with/without
// subclasses), resolve_alias, languages, resolve_dispatch (concrete,
// virtual with RTA, empty results, missing record file).

use prism::languages::Language;
use prism::type_db::{FieldInfo, RecordInfo, RecordKind, TypeDatabase, TypedefInfo};
use prism::type_provider::{DispatchProvider, ResolvedTypeKind, TypeProvider};
use prism::type_providers::cpp::CppTypeProvider;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

// ---------------------------------------------------------------------------
// Helper: build a TypeDatabase with some records, typedefs, and hierarchy
// ---------------------------------------------------------------------------

fn make_test_db() -> TypeDatabase {
    let mut db = TypeDatabase::default();

    // Struct: device_t with two fields
    db.records.insert(
        "device_t".to_string(),
        RecordInfo {
            name: "device_t".to_string(),
            kind: RecordKind::Struct,
            fields: vec![
                FieldInfo {
                    name: "name".to_string(),
                    type_str: "char *".to_string(),
                    offset: Some(0),
                },
                FieldInfo {
                    name: "id".to_string(),
                    type_str: "int".to_string(),
                    offset: Some(8),
                },
            ],
            bases: Vec::new(),
            virtual_methods: BTreeMap::new(),
            size: Some(16),
            file: "device.h".to_string(),
        },
    );

    // Class: Base with a virtual method
    db.records.insert(
        "Base".to_string(),
        RecordInfo {
            name: "Base".to_string(),
            kind: RecordKind::Class,
            fields: vec![FieldInfo {
                name: "value".to_string(),
                type_str: "int".to_string(),
                offset: Some(0),
            }],
            bases: Vec::new(),
            virtual_methods: BTreeMap::from([("process".to_string(), "void".to_string())]),
            size: Some(8),
            file: "base.h".to_string(),
        },
    );

    // Class: Derived extends Base, overrides process
    db.records.insert(
        "Derived".to_string(),
        RecordInfo {
            name: "Derived".to_string(),
            kind: RecordKind::Class,
            fields: vec![
                FieldInfo {
                    name: "value".to_string(),
                    type_str: "int".to_string(),
                    offset: Some(0),
                },
                FieldInfo {
                    name: "extra".to_string(),
                    type_str: "double".to_string(),
                    offset: Some(8),
                },
            ],
            bases: vec!["Base".to_string()],
            virtual_methods: BTreeMap::from([("process".to_string(), "void".to_string())]),
            size: Some(16),
            file: "derived.h".to_string(),
        },
    );

    // Class: AnotherDerived extends Base
    db.records.insert(
        "AnotherDerived".to_string(),
        RecordInfo {
            name: "AnotherDerived".to_string(),
            kind: RecordKind::Class,
            fields: vec![],
            bases: vec!["Base".to_string()],
            virtual_methods: BTreeMap::from([("process".to_string(), "void".to_string())]),
            size: Some(8),
            file: "another.h".to_string(),
        },
    );

    // Union: my_union
    db.records.insert(
        "my_union".to_string(),
        RecordInfo {
            name: "my_union".to_string(),
            kind: RecordKind::Union,
            fields: vec![
                FieldInfo {
                    name: "i".to_string(),
                    type_str: "int".to_string(),
                    offset: Some(0),
                },
                FieldInfo {
                    name: "f".to_string(),
                    type_str: "float".to_string(),
                    offset: Some(0),
                },
            ],
            bases: Vec::new(),
            virtual_methods: BTreeMap::new(),
            size: Some(4),
            file: "types.h".to_string(),
        },
    );

    // Typedef: dev_ptr → device_t (resolves to a known record)
    db.typedefs.insert(
        "dev_ptr".to_string(),
        TypedefInfo {
            name: "dev_ptr".to_string(),
            underlying: "device_t".to_string(),
        },
    );

    // Typedef: handle_t → opaque_handle (no matching record)
    db.typedefs.insert(
        "handle_t".to_string(),
        TypedefInfo {
            name: "handle_t".to_string(),
            underlying: "opaque_handle".to_string(),
        },
    );

    // Chained typedef: dev_alias → dev_ptr → device_t (two levels)
    db.typedefs.insert(
        "dev_alias".to_string(),
        TypedefInfo {
            name: "dev_alias".to_string(),
            underlying: "dev_ptr".to_string(),
        },
    );

    // Class hierarchy
    db.class_hierarchy
        .insert("Derived".to_string(), vec!["Base".to_string()]);
    db.class_hierarchy
        .insert("AnotherDerived".to_string(), vec!["Base".to_string()]);

    db
}

// ---------------------------------------------------------------------------
// Constructors and accessors
// ---------------------------------------------------------------------------

#[test]
fn test_cpp_new_and_type_db() {
    let db = make_test_db();
    let provider = CppTypeProvider::new(db);

    // type_db() should give access to the underlying database.
    assert!(provider.type_db().records.contains_key("device_t"));
    assert!(provider.type_db().records.contains_key("Base"));
}

#[test]
fn test_cpp_from_arc() {
    let db = make_test_db();
    let arc = Arc::new(db);
    let provider = CppTypeProvider::from_arc(arc.clone());

    // Should share the same Arc.
    assert!(Arc::ptr_eq(&provider.db, &arc));
    assert!(provider.type_db().records.contains_key("device_t"));
}

#[test]
fn test_cpp_clone_shares_arc() {
    let db = make_test_db();
    let provider = CppTypeProvider::new(db);
    let cloned = provider.clone();

    assert!(Arc::ptr_eq(&provider.db, &cloned.db));
}

// ---------------------------------------------------------------------------
// resolve_type
// ---------------------------------------------------------------------------

#[test]
fn test_cpp_resolve_type_struct() {
    let provider = CppTypeProvider::new(make_test_db());

    let resolved = provider.resolve_type("device.h", "device_t", 0).unwrap();
    assert_eq!(resolved.name, "device_t");
    assert_eq!(resolved.kind, ResolvedTypeKind::Concrete);
    assert!(resolved.type_params.is_empty());
}

#[test]
fn test_cpp_resolve_type_class() {
    let provider = CppTypeProvider::new(make_test_db());

    let resolved = provider.resolve_type("base.h", "Base", 0).unwrap();
    assert_eq!(resolved.name, "Base");
    assert_eq!(resolved.kind, ResolvedTypeKind::Concrete);
}

#[test]
fn test_cpp_resolve_type_union() {
    let provider = CppTypeProvider::new(make_test_db());

    let resolved = provider.resolve_type("types.h", "my_union", 0).unwrap();
    assert_eq!(resolved.name, "my_union");
    assert_eq!(resolved.kind, ResolvedTypeKind::Concrete);
}

#[test]
fn test_cpp_resolve_type_typedef_to_record() {
    let provider = CppTypeProvider::new(make_test_db());

    // dev_ptr is a typedef for device_t (a known record).
    let resolved = provider.resolve_type("device.h", "dev_ptr", 0).unwrap();
    assert_eq!(resolved.name, "device_t");
    assert_eq!(resolved.kind, ResolvedTypeKind::Concrete);
}

#[test]
fn test_cpp_resolve_type_chained_typedef() {
    let provider = CppTypeProvider::new(make_test_db());

    // dev_alias → dev_ptr → device_t: TypeDatabase::resolve_typedef chains
    // up to 10 levels, so a two-level chain should resolve to device_t.
    let resolved = provider.resolve_type("device.h", "dev_alias", 0).unwrap();
    assert_eq!(resolved.name, "device_t");
    assert_eq!(resolved.kind, ResolvedTypeKind::Concrete);
}

#[test]
fn test_cpp_resolve_alias_chained_typedef() {
    let provider = CppTypeProvider::new(make_test_db());

    // resolve_alias should follow the full chain.
    assert_eq!(provider.resolve_alias("dev_alias"), "device_t");
}

#[test]
fn test_cpp_resolve_type_typedef_to_unknown() {
    let provider = CppTypeProvider::new(make_test_db());

    // handle_t is a typedef for opaque_handle (not a known record).
    let resolved = provider.resolve_type("types.h", "handle_t", 0).unwrap();
    assert_eq!(resolved.name, "opaque_handle");
    assert_eq!(resolved.kind, ResolvedTypeKind::Alias);
}

#[test]
fn test_cpp_resolve_type_unknown() {
    let provider = CppTypeProvider::new(make_test_db());

    // Completely unknown type.
    assert!(provider.resolve_type("foo.c", "NonExistent", 0).is_none());
}

// ---------------------------------------------------------------------------
// field_layout
// ---------------------------------------------------------------------------

#[test]
fn test_cpp_field_layout_struct() {
    let provider = CppTypeProvider::new(make_test_db());

    let fields = provider.field_layout("device_t").unwrap();
    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0].name, "name");
    assert_eq!(fields[0].type_str, "char *");
    assert_eq!(fields[1].name, "id");
    assert_eq!(fields[1].type_str, "int");
}

#[test]
fn test_cpp_field_layout_class() {
    let provider = CppTypeProvider::new(make_test_db());

    let fields = provider.field_layout("Derived").unwrap();
    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0].name, "value");
    assert_eq!(fields[1].name, "extra");
}

#[test]
fn test_cpp_field_layout_union() {
    let provider = CppTypeProvider::new(make_test_db());

    let fields = provider.field_layout("my_union").unwrap();
    assert_eq!(fields.len(), 2);
    assert_eq!(fields[0].name, "i");
    assert_eq!(fields[1].name, "f");
}

#[test]
fn test_cpp_field_layout_empty_fields() {
    let provider = CppTypeProvider::new(make_test_db());

    let fields = provider.field_layout("AnotherDerived").unwrap();
    assert!(fields.is_empty());
}

#[test]
fn test_cpp_field_layout_unknown() {
    let provider = CppTypeProvider::new(make_test_db());
    assert!(provider.field_layout("NonExistent").is_none());
}

// ---------------------------------------------------------------------------
// subtypes_of
// ---------------------------------------------------------------------------

#[test]
fn test_cpp_subtypes_of_base() {
    let provider = CppTypeProvider::new(make_test_db());

    let subtypes = provider.subtypes_of("Base");
    assert!(
        subtypes.contains(&"Derived".to_string()),
        "Derived should be a subtype of Base, got: {:?}",
        subtypes
    );
    assert!(
        subtypes.contains(&"AnotherDerived".to_string()),
        "AnotherDerived should be a subtype of Base, got: {:?}",
        subtypes
    );
}

#[test]
fn test_cpp_subtypes_of_leaf() {
    let provider = CppTypeProvider::new(make_test_db());

    // Derived has no subclasses.
    let subtypes = provider.subtypes_of("Derived");
    assert!(subtypes.is_empty());
}

#[test]
fn test_cpp_subtypes_of_unknown() {
    let provider = CppTypeProvider::new(make_test_db());
    assert!(provider.subtypes_of("NonExistent").is_empty());
}

// ---------------------------------------------------------------------------
// resolve_alias
// ---------------------------------------------------------------------------

#[test]
fn test_cpp_resolve_alias_typedef() {
    let provider = CppTypeProvider::new(make_test_db());

    assert_eq!(provider.resolve_alias("dev_ptr"), "device_t");
    assert_eq!(provider.resolve_alias("handle_t"), "opaque_handle");
}

#[test]
fn test_cpp_resolve_alias_passthrough() {
    let provider = CppTypeProvider::new(make_test_db());

    // Not a typedef — should return the same name.
    assert_eq!(provider.resolve_alias("device_t"), "device_t");
    assert_eq!(provider.resolve_alias("NonExistent"), "NonExistent");
}

// ---------------------------------------------------------------------------
// languages
// ---------------------------------------------------------------------------

#[test]
fn test_cpp_languages() {
    let provider = CppTypeProvider::new(TypeDatabase::default());
    let langs = provider.languages();
    assert!(langs.contains(&Language::C));
    assert!(langs.contains(&Language::Cpp));
    assert_eq!(langs.len(), 2);
}

// ---------------------------------------------------------------------------
// resolve_dispatch
// ---------------------------------------------------------------------------

#[test]
fn test_cpp_dispatch_virtual_no_rta() {
    let provider = CppTypeProvider::new(make_test_db());

    // Dispatch on Base.process with no RTA filter — should find all overrides.
    let targets = provider.resolve_dispatch("Base", "process", &BTreeSet::new());
    let target_files: BTreeSet<String> = targets.iter().map(|t| t.file.clone()).collect();

    // Should include Derived and AnotherDerived (and possibly Base itself).
    assert!(
        targets.len() >= 2,
        "Expected at least Derived + AnotherDerived, got: {:?}",
        targets
    );
    assert!(target_files.contains("derived.h"));
    assert!(target_files.contains("another.h"));

    // All targets should have the method name "process".
    assert!(targets.iter().all(|t| t.name == "process"));
}

#[test]
fn test_cpp_dispatch_virtual_with_rta() {
    let provider = CppTypeProvider::new(make_test_db());

    // Only Derived is live.
    let live = BTreeSet::from(["Derived".to_string()]);
    let targets = provider.resolve_dispatch("Base", "process", &live);

    let target_files: BTreeSet<String> = targets.iter().map(|t| t.file.clone()).collect();
    assert!(
        target_files.contains("derived.h"),
        "RTA should include Derived, got: {:?}",
        targets
    );
    // AnotherDerived should be filtered out.
    assert!(
        !target_files.contains("another.h"),
        "RTA should exclude AnotherDerived, got: {:?}",
        targets
    );
}

#[test]
fn test_cpp_dispatch_nonexistent_method() {
    let provider = CppTypeProvider::new(make_test_db());

    let targets = provider.resolve_dispatch("Base", "nonExistent", &BTreeSet::new());
    assert!(targets.is_empty());
}

#[test]
fn test_cpp_dispatch_nonexistent_type() {
    let provider = CppTypeProvider::new(make_test_db());

    let targets = provider.resolve_dispatch("Unknown", "process", &BTreeSet::new());
    assert!(targets.is_empty());
}

#[test]
fn test_cpp_dispatch_struct_no_virtual() {
    let provider = CppTypeProvider::new(make_test_db());

    // device_t has no virtual methods — dispatch should return empty.
    let targets = provider.resolve_dispatch("device_t", "name", &BTreeSet::new());
    assert!(targets.is_empty());
}

#[test]
fn test_cpp_dispatch_result_fields() {
    let provider = CppTypeProvider::new(make_test_db());

    let targets = provider.resolve_dispatch("Base", "process", &BTreeSet::new());
    for target in &targets {
        assert_eq!(target.name, "process");
        // Known limitation: start_line/end_line are always 0 because
        // TypeDatabase stores record-level info, not per-method locations.
        // Go/Java/Rust/TS providers produce real line numbers from tree-sitter.
        // Downstream code relying on line numbers (e.g., finding deduplication)
        // should be aware that the C++ path produces zeroes.
        assert_eq!(target.start_line, 0);
        assert_eq!(target.end_line, 0);
        // File should be non-empty (from the record's file).
        assert!(!target.file.is_empty());
    }
}

// ---------------------------------------------------------------------------
// CpgContext integration
// ---------------------------------------------------------------------------

#[test]
fn test_cpp_cpg_context_with_type_db() {
    use prism::ast::ParsedFile;
    use prism::cpg::CpgContext;

    let source = r#"
#include <stdio.h>
struct device {
    int id;
    char *name;
};
void init_device(struct device *dev) {
    dev->id = 0;
    dev->name = "unknown";
}
"#;
    let parsed = ParsedFile::parse("device.c", source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert("device.c".to_string(), parsed);

    // Build with a TypeDatabase to auto-register CppTypeProvider.
    let db = make_test_db();
    let ctx = CpgContext::build(&files, Some(&db));

    let tp = ctx.types.provider_for(Language::C);
    assert!(tp.is_some(), "C provider should be auto-registered");
    let tp = tp.unwrap();
    let resolved = tp.resolve_type("device.h", "device_t", 0).unwrap();
    assert_eq!(resolved.kind, ResolvedTypeKind::Concrete);

    // Also available for C++.
    assert!(ctx.types.provider_for(Language::Cpp).is_some());

    // Dispatch should also be registered.
    let dp = ctx.types.dispatch_for(Language::C);
    assert!(dp.is_some());
}

#[test]
fn test_cpp_cpg_context_without_type_db() {
    use prism::ast::ParsedFile;
    use prism::cpg::CpgContext;

    let source = "void foo() {}\n";
    let parsed = ParsedFile::parse("test.c", source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert("test.c".to_string(), parsed);

    // Build without TypeDatabase — no C/C++ provider registered.
    let ctx = CpgContext::build(&files, None);
    assert!(ctx.types.provider_for(Language::C).is_none());
    assert!(ctx.types.dispatch_for(Language::C).is_none());
}

// ---------------------------------------------------------------------------
// Registry integration
// ---------------------------------------------------------------------------

#[test]
fn test_cpp_registry_integration() {
    use prism::type_provider::TypeRegistry;

    let db = make_test_db();
    let provider = CppTypeProvider::new(db);
    let dispatch = provider.clone();

    let mut registry = TypeRegistry::empty();
    registry.register_provider(Box::new(provider));
    registry.register_dispatch_provider(Box::new(dispatch));

    // Provider lookup by language.
    let tp = registry.provider_for(Language::C).unwrap();
    assert!(tp.resolve_type("", "Base", 0).is_some());

    let tp_cpp = registry.provider_for(Language::Cpp).unwrap();
    assert!(tp_cpp.resolve_type("", "device_t", 0).is_some());

    // Dispatch lookup.
    let dp = registry.dispatch_for(Language::C).unwrap();
    let targets = dp.resolve_dispatch("Base", "process", &BTreeSet::new());
    assert!(!targets.is_empty());
}

// ---------------------------------------------------------------------------
// Empty database
// ---------------------------------------------------------------------------

#[test]
fn test_cpp_empty_database() {
    let provider = CppTypeProvider::new(TypeDatabase::default());

    assert!(provider.resolve_type("", "anything", 0).is_none());
    assert!(provider.field_layout("anything").is_none());
    assert!(provider.subtypes_of("anything").is_empty());
    assert_eq!(provider.resolve_alias("anything"), "anything");
    assert!(provider
        .resolve_dispatch("anything", "method", &BTreeSet::new())
        .is_empty());
}
