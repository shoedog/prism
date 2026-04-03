mod common;
use common::*;

#[test]
fn test_extract_lvalue_paths_pointer_deref() {
    // *ptr = val should create a def for base "ptr" only.
    let source = r#"
void write(int *ptr, int val) {
    *ptr = val;
}
"#;
    let path = "src/write.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let ptr_defs = dfg.all_defs_of(path, "ptr");
    assert!(
        ptr_defs.iter().any(|d| d.line == 3 && d.path.is_simple()),
        "Dereference *ptr should create a simple path def for 'ptr'"
    );
}

#[test]
fn test_rvalue_field_expression_paths() {
    // R-value field expressions should create AccessPath entries.
    let source = r#"
void copy(struct dev *src, struct dev *dst) {
    dst->id = src->id;
}
"#;
    let path = "src/copy.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);

    // dst->id should have a def with fields
    let dst_defs = dfg.all_defs_of(path, "dst");
    assert!(
        dst_defs
            .iter()
            .any(|d| d.path.fields == vec!["id".to_string()]),
        "Should have AccessPath dst.id def"
    );

    // src->id should appear in uses (rvalue)
    let has_src_use = dfg.uses.values().any(|locs| {
        locs.iter()
            .any(|l| l.path.base == "src" && l.path.fields == vec!["id".to_string()])
    });
    assert!(
        has_src_use,
        "R-value src->id should create a field-qualified use in DFG"
    );
}

#[test]
fn test_field_isolation_python() {
    let source = r#"
class Handler:
    def process(self):
        self.secret = get_password()
        self.label = "public"
        send(self.secret)
        display(self.label)
"#;
    let path = "src/handler.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let self_defs = dfg.all_defs_of(path, "self");

    let base_only: Vec<_> = self_defs.iter().filter(|d| !d.path.has_fields()).collect();
    assert!(
        base_only.is_empty(),
        "Phase 2 Python: field assignments should NOT create base-only defs"
    );

    let secret_def = self_defs.iter().find(|d| d.path.fields == vec!["secret"]);
    if let Some(sd) = secret_def {
        let reachable = dfg.forward_reachable(sd);
        let reaches_label = reachable.iter().any(|r| r.path.fields == vec!["label"]);
        assert!(
            !reaches_label,
            "Phase 2 Python: taint on self.secret must NOT propagate to self.label"
        );
    }
}

#[test]
fn test_field_isolation_go() {
    let source = r#"
package main

func process(dev Device) {
    dev.Name = getInput()
    dev.ID = 42
    useName(dev.Name)
    useID(dev.ID)
}
"#;
    let path = "src/dev.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let dev_defs = dfg.all_defs_of(path, "dev");

    let base_only: Vec<_> = dev_defs.iter().filter(|d| !d.path.has_fields()).collect();
    assert!(
        base_only.is_empty(),
        "Phase 2 Go: field assignments should NOT create base-only defs"
    );
}

#[test]
fn test_field_isolation_rust() {
    let source = r#"
fn process(dev: &mut Device) {
    dev.name = get_input();
    dev.id = 42;
    use_name(dev.name);
    use_id(dev.id);
}
"#;
    let path = "src/dev.rs";
    let parsed = ParsedFile::parse(path, source, Language::Rust).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let dev_defs = dfg.all_defs_of(path, "dev");

    let base_only: Vec<_> = dev_defs.iter().filter(|d| !d.path.has_fields()).collect();
    assert!(
        base_only.is_empty(),
        "Phase 2 Rust: field assignments should NOT create base-only defs"
    );
}

#[test]
fn test_field_isolation_lua() {
    let source = r#"
function process(dev)
    dev.name = get_input()
    dev.id = 42
    use_name(dev.name)
    use_id(dev.id)
end
"#;
    let path = "src/dev.lua";
    let parsed = ParsedFile::parse(path, source, Language::Lua).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let dev_defs = dfg.all_defs_of(path, "dev");

    let base_only: Vec<_> = dev_defs.iter().filter(|d| !d.path.has_fields()).collect();
    assert!(
        base_only.is_empty(),
        "Phase 2 Lua: field assignments should NOT create base-only defs"
    );
}

#[test]
fn test_field_isolation_java() {
    let source = r#"
class Handler {
    void process(Device dev) {
        dev.name = getInput();
        dev.id = 42;
        useName(dev.name);
        useId(dev.id);
    }
}
"#;
    let path = "src/Handler.java";
    let parsed = ParsedFile::parse(path, source, Language::Java).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let dev_defs = dfg.all_defs_of(path, "dev");

    let base_only: Vec<_> = dev_defs.iter().filter(|d| !d.path.has_fields()).collect();
    assert!(
        base_only.is_empty(),
        "Phase 2 Java: field assignments should NOT create base-only defs"
    );
}

#[test]
fn test_field_isolation_whole_struct_still_works() {
    // Whole-struct assignment (no field) should still create a base-only def
    let source = r#"
void init() {
    struct device *dev = malloc(sizeof(struct device));
    int x = 42;
    use(dev);
    use(x);
}
"#;
    let path = "src/init.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);

    // dev should still have a base-only def from the whole-struct assignment
    let dev_defs = dfg.all_defs_of(path, "dev");
    assert!(
        !dev_defs.is_empty(),
        "Whole-struct assignment should still create a def for dev"
    );
    assert!(
        dev_defs.iter().any(|d| !d.path.has_fields()),
        "Whole-struct assignment should create base-only def"
    );
}

#[test]
fn test_type_db_struct_fields_and_typedef() {
    use prism::type_db::{FieldInfo, RecordInfo, RecordKind, TypeDatabase, TypedefInfo};

    let mut db = TypeDatabase::default();
    db.records.insert(
        "device".to_string(),
        RecordInfo {
            name: "device".to_string(),
            kind: RecordKind::Struct,
            fields: vec![
                FieldInfo {
                    name: "name".to_string(),
                    type_str: "char *".to_string(),
                    offset: None,
                },
                FieldInfo {
                    name: "id".to_string(),
                    type_str: "int".to_string(),
                    offset: None,
                },
                FieldInfo {
                    name: "config".to_string(),
                    type_str: "struct config *".to_string(),
                    offset: None,
                },
            ],
            bases: vec![],
            virtual_methods: std::collections::BTreeMap::new(),
            size: Some(24),
            file: "device.h".to_string(),
        },
    );
    db.typedefs.insert(
        "dev_t".to_string(),
        TypedefInfo {
            name: "dev_t".to_string(),
            underlying: "struct device *".to_string(),
        },
    );

    // Typedef resolution
    assert_eq!(db.resolve_typedef("dev_t"), "struct device *");

    // Record lookup via typedef
    let record = db.resolve_record("dev_t").unwrap();
    assert_eq!(record.name, "device");
    assert_eq!(record.fields.len(), 3);

    // Field type query
    assert_eq!(db.field_type("device", "name"), Some("char *".to_string()));
    assert_eq!(
        db.field_type("device", "config"),
        Some("struct config *".to_string())
    );
    assert_eq!(db.field_type("device", "nonexistent"), None);

    // All fields
    let fields = db.all_fields("device");
    assert_eq!(fields.len(), 3);
}

#[test]
fn test_type_db_class_hierarchy_virtual_dispatch() {
    use prism::type_db::{RecordInfo, RecordKind, TypeDatabase};

    let mut db = TypeDatabase::default();

    // Base class: Shape with virtual draw()
    db.records.insert(
        "Shape".to_string(),
        RecordInfo {
            name: "Shape".to_string(),
            kind: RecordKind::Class,
            fields: vec![],
            bases: vec![],
            virtual_methods: std::collections::BTreeMap::from([
                ("draw".to_string(), "void ()".to_string()),
                ("area".to_string(), "double ()".to_string()),
            ]),
            size: None,
            file: "shape.h".to_string(),
        },
    );

    // Circle overrides draw() and area()
    db.records.insert(
        "Circle".to_string(),
        RecordInfo {
            name: "Circle".to_string(),
            kind: RecordKind::Class,
            fields: vec![],
            bases: vec!["Shape".to_string()],
            virtual_methods: std::collections::BTreeMap::from([
                ("draw".to_string(), "void ()".to_string()),
                ("area".to_string(), "double ()".to_string()),
            ]),
            size: None,
            file: "circle.h".to_string(),
        },
    );

    // Rect overrides draw() only
    db.records.insert(
        "Rect".to_string(),
        RecordInfo {
            name: "Rect".to_string(),
            kind: RecordKind::Class,
            fields: vec![],
            bases: vec!["Shape".to_string()],
            virtual_methods: std::collections::BTreeMap::from([(
                "draw".to_string(),
                "void ()".to_string(),
            )]),
            size: None,
            file: "rect.h".to_string(),
        },
    );

    db.class_hierarchy
        .insert("Circle".to_string(), vec!["Shape".to_string()]);
    db.class_hierarchy
        .insert("Rect".to_string(), vec!["Shape".to_string()]);

    // Virtual dispatch: draw() on Shape → Shape, Circle, Rect
    let mut draw_targets = db.virtual_dispatch_targets("Shape", "draw");
    draw_targets.sort();
    assert_eq!(draw_targets, vec!["Circle", "Rect", "Shape"]);

    // Virtual dispatch: area() on Shape → Shape, Circle (Rect doesn't override)
    let mut area_targets = db.virtual_dispatch_targets("Shape", "area");
    area_targets.sort();
    assert_eq!(area_targets, vec!["Circle", "Shape"]);

    // Hierarchy queries
    assert!(db.is_subclass_of("Circle", "Shape"));
    assert!(!db.is_subclass_of("Shape", "Circle"));
}

#[test]
fn test_type_db_union_field_aliasing() {
    use prism::type_db::{FieldInfo, RecordInfo, RecordKind, TypeDatabase};

    let mut db = TypeDatabase::default();
    db.records.insert(
        "value".to_string(),
        RecordInfo {
            name: "value".to_string(),
            kind: RecordKind::Union,
            fields: vec![
                FieldInfo {
                    name: "i".to_string(),
                    type_str: "int".to_string(),
                    offset: None,
                },
                FieldInfo {
                    name: "f".to_string(),
                    type_str: "float".to_string(),
                    offset: None,
                },
                FieldInfo {
                    name: "p".to_string(),
                    type_str: "void *".to_string(),
                    offset: None,
                },
            ],
            bases: vec![],
            virtual_methods: std::collections::BTreeMap::new(),
            size: Some(8),
            file: "value.h".to_string(),
        },
    );

    assert!(db.is_union("value"));
    assert!(!db.is_union("nonexistent"));
    assert_eq!(db.all_fields("value").len(), 3);
}

#[test]
fn test_type_db_clang_json_parsing() {
    use prism::type_db::TypeDatabase;

    // Simulate a clang JSON AST with struct, typedef, and class hierarchy
    let json = r#"{
        "kind": "TranslationUnitDecl",
        "inner": [
            {
                "kind": "RecordDecl",
                "name": "device",
                "tagUsed": "struct",
                "completeDefinition": true,
                "inner": [
                    {
                        "kind": "FieldDecl",
                        "name": "name",
                        "type": { "qualType": "char *" }
                    },
                    {
                        "kind": "FieldDecl",
                        "name": "id",
                        "type": { "qualType": "int" }
                    }
                ]
            },
            {
                "kind": "TypedefDecl",
                "name": "device_t",
                "type": { "qualType": "struct device *", "desugaredQualType": "struct device *" }
            },
            {
                "kind": "CXXRecordDecl",
                "name": "Base",
                "tagUsed": "class",
                "completeDefinition": true,
                "inner": [
                    {
                        "kind": "CXXMethodDecl",
                        "name": "process",
                        "virtual": true,
                        "type": { "qualType": "void ()" }
                    }
                ]
            },
            {
                "kind": "CXXRecordDecl",
                "name": "Derived",
                "tagUsed": "class",
                "completeDefinition": true,
                "bases": [
                    { "type": { "qualType": "class Base" } }
                ],
                "inner": [
                    {
                        "kind": "FieldDecl",
                        "name": "data",
                        "type": { "qualType": "int" }
                    },
                    {
                        "kind": "CXXMethodDecl",
                        "name": "process",
                        "virtual": true,
                        "type": { "qualType": "void ()" }
                    }
                ]
            }
        ]
    }"#;

    let mut db = TypeDatabase::default();
    db.extract_from_ast(json, "test.cpp").unwrap();

    // Struct extraction
    let device = db.records.get("device").unwrap();
    assert_eq!(device.fields.len(), 2);
    assert_eq!(device.fields[0].name, "name");

    // Typedef extraction
    let td = db.typedefs.get("device_t").unwrap();
    assert_eq!(td.underlying, "struct device *");

    // Record via typedef
    assert!(db.resolve_record("device_t").is_some());

    // C++ class with virtual method
    let base = db.records.get("Base").unwrap();
    assert!(base.virtual_methods.contains_key("process"));

    // Derived class with base and override
    let derived = db.records.get("Derived").unwrap();
    assert_eq!(derived.bases, vec!["Base"]);
    assert!(derived.virtual_methods.contains_key("process"));
    assert_eq!(derived.fields.len(), 1);
    assert_eq!(derived.fields[0].name, "data");
}

#[test]
fn test_rta_fallback_no_type_db() {
    use prism::type_db::TypeDatabase;

    let source = r#"
class Animal {
public:
    virtual void speak();
};

class Dog : public Animal {
public:
    virtual void speak();
};
"#;
    let path = "src/animals.cpp";
    let parsed = ParsedFile::parse(path, source, Language::Cpp).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let db = TypeDatabase::from_parsed_files(&files);

    // Empty live set → falls back to CHA
    let empty_live = std::collections::BTreeSet::new();
    let targets = db.virtual_dispatch_targets_rta("Animal", "speak", &empty_live);
    let cha = db.virtual_dispatch_targets("Animal", "speak");

    assert_eq!(targets, cha, "Empty live set should fall back to CHA");
}

#[test]
fn test_go_multi_return_individual_defs_lvalue() {
    let source = r#"
package main

func process() {
    host, port := getAddr()
    connect(host, port)
}
"#;
    let path = "src/go_multi.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let func = parsed.all_functions().into_iter().next().unwrap();
    let lines: BTreeSet<usize> = (1..=7).collect();
    let lvalues = parsed.assignment_lvalue_paths_on_lines(&func, &lines);

    let has_host = lvalues.iter().any(|(p, _)| p.base == "host");
    let has_port = lvalues.iter().any(|(p, _)| p.base == "port");
    assert!(
        has_host,
        "Go multi-return L-value: 'host' should exist, got {:?}",
        lvalues.iter().map(|(p, _)| &p.base).collect::<Vec<_>>()
    );
    assert!(has_port, "Go multi-return L-value: 'port' should exist");
    let has_composite = lvalues.iter().any(|(p, _)| p.base.contains(','));
    assert!(
        !has_composite,
        "Go multi-return: should not have composite L-value"
    );
}

#[test]
fn test_python_tuple_unpack_lvalue() {
    let source = r#"
def process():
    name, age = get_user()
    first, *rest = get_items()
    use(name)
"#;
    let path = "src/py_tuple.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let func = parsed.all_functions().into_iter().next().unwrap();
    let lines: BTreeSet<usize> = (1..=6).collect();
    let lvalues = parsed.assignment_lvalue_paths_on_lines(&func, &lines);

    assert!(
        lvalues.iter().any(|(p, _)| p.base == "name"),
        "Python: 'name' L-value"
    );
    assert!(
        lvalues.iter().any(|(p, _)| p.base == "age"),
        "Python: 'age' L-value"
    );
    assert!(
        lvalues.iter().any(|(p, _)| p.base == "first"),
        "Python: 'first' L-value"
    );
    assert!(
        lvalues.iter().any(|(p, _)| p.base == "rest"),
        "Python: 'rest' L-value"
    );
}

#[test]
fn test_python_multi_target_attribute_lvalue() {
    // Multi-target with attribute access: obj.x, obj.y = func()
    let source = r#"
def process(obj):
    obj.x, obj.y = get_coords()
    use(obj)
"#;
    let path = "src/process.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let func = parsed.all_functions().into_iter().next().unwrap();
    let lines: BTreeSet<usize> = (1..=4).collect();
    let lvalues = parsed.assignment_lvalue_paths_on_lines(&func, &lines);

    let has_obj_x = lvalues
        .iter()
        .any(|(p, _)| p.base == "obj" && p.fields.contains(&"x".to_string()));
    let has_obj_y = lvalues
        .iter()
        .any(|(p, _)| p.base == "obj" && p.fields.contains(&"y".to_string()));
    assert!(
        has_obj_x,
        "Python multi-target: obj.x should be L-value, got: {:?}",
        lvalues
    );
    assert!(
        has_obj_y,
        "Python multi-target: obj.y should be L-value, got: {:?}",
        lvalues
    );
}
