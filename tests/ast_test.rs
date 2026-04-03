mod common;
use common::*;

#[test]
fn test_dfg_field_qualified_paths_created() {
    // Verify that the DFG creates AccessPath entries with field chains,
    // not just bare base names.
    let source = r#"
void init(struct device *dev) {
    dev->name = "eth0";
    dev->id = 42;
    dev->config->timeout = 100;
}
"#;
    let path = "src/dev.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let dev_defs = dfg.all_defs_of(path, "dev");

    // Should have qualified paths for each field
    let field_names: Vec<Vec<String>> = dev_defs
        .iter()
        .filter(|d| d.path.has_fields())
        .map(|d| d.path.fields.clone())
        .collect();
    assert!(
        field_names.iter().any(|f| f == &vec!["name".to_string()]),
        "DFG should have AccessPath dev.name, got: {:?}",
        field_names
    );
    assert!(
        field_names.iter().any(|f| f == &vec!["id".to_string()]),
        "DFG should have AccessPath dev.id, got: {:?}",
        field_names
    );
}


#[test]
fn test_dfg_dot_access_paths() {
    // Python-style dot access creates field-qualified paths.
    let source = r#"
class Config:
    def setup(self):
        self.timeout = 30
        self.host = "localhost"
"#;
    let path = "src/config.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let self_defs = dfg.all_defs_of(path, "self");

    // Should have field-qualified paths
    let has_timeout = self_defs
        .iter()
        .any(|d| d.path.has_fields() && d.path.fields.contains(&"timeout".to_string()));
    assert!(
        has_timeout,
        "DFG should record self.timeout AccessPath for Python dot access"
    );
}


#[test]
fn test_dfg_field_path_def_line_scoping() {
    // Verify that find_path_references_scoped only returns references AFTER
    // the definition line, preventing backward data flow edges.
    let source = r#"
void process(struct dev *d) {
    int old = d->status;
    d->status = 1;
    int new_val = d->status;
}
"#;
    let path = "src/proc.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let dev_defs = dfg.all_defs_of(path, "d");

    // The def of d->status on line 4 should only reach line 5 (new_val = d->status),
    // NOT line 3 (old = d->status) which is before the definition.
    let status_def = dev_defs
        .iter()
        .find(|d| d.path.fields == vec!["status".to_string()] && d.line == 4);
    assert!(
        status_def.is_some(),
        "Should have a def for d->status on line 4"
    );

    // Check forward edges from this def
    if let Some(def) = status_def {
        let reachable = dfg.forward_reachable(def);
        let reachable_lines: Vec<usize> = reachable.iter().map(|r| r.line).collect();
        assert!(
            !reachable_lines.contains(&3),
            "d->status def on line 4 should NOT reach line 3 (before def). Got: {:?}",
            reachable_lines
        );
    }
}


#[test]
fn test_dfg_var_name_backward_compat() {
    // Verify the var_name() accessor works for backward compatibility.
    let source = r#"
void f(int x) {
    int y = x;
}
"#;
    let path = "src/f.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let y_defs = dfg.all_defs_of(path, "y");
    assert!(!y_defs.is_empty());
    // var_name() returns the base name
    assert_eq!(y_defs[0].var_name(), "y");
}


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
fn test_dfg_go_field_access_paths() {
    // Go selector_expression: obj.Field
    let source = r#"
package main

func process(dev *Device) {
	dev.Name = "eth0"
	dev.ID = 42
	x := dev.Name
	_ = x
}
"#;
    let path = "src/dev.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let dev_defs = dfg.all_defs_of(path, "dev");

    let has_name = dev_defs
        .iter()
        .any(|d| d.path.has_fields() && d.path.fields.contains(&"Name".to_string()));
    assert!(
        has_name,
        "Go DFG should have AccessPath dev.Name from selector_expression. Got: {:?}",
        dev_defs
            .iter()
            .map(|d| d.path.to_string())
            .collect::<Vec<_>>()
    );
}


#[test]
fn test_dfg_java_field_access_paths() {
    // Java field_access: obj.field
    let source = r#"
class Device {
    String name;
    int id;

    void setup(Device dev) {
        dev.name = "eth0";
        dev.id = 42;
    }
}
"#;
    let path = "src/Device.java";
    let parsed = ParsedFile::parse(path, source, Language::Java).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let dev_defs = dfg.all_defs_of(path, "dev");

    let has_name = dev_defs
        .iter()
        .any(|d| d.path.has_fields() && d.path.fields.contains(&"name".to_string()));
    assert!(
        has_name,
        "Java DFG should have AccessPath dev.name from field_access. Got: {:?}",
        dev_defs
            .iter()
            .map(|d| d.path.to_string())
            .collect::<Vec<_>>()
    );
}


#[test]
fn test_dfg_same_line_cross_field_assignment() {
    // dev->name = dev->id on a single line.
    // LHS creates def for dev->name (and dev base).
    // RHS creates use for dev->id (and dev base).
    // Assignment propagation should connect use of dev->id → def of dev->name.
    let source = r#"
void copy_field(struct device *dev) {
    dev->name = dev->id;
    char *n = dev->name;
}
"#;
    let path = "src/dev.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);

    // Should have both field-qualified defs
    let dev_defs = dfg.all_defs_of(path, "dev");
    let has_name_def = dev_defs
        .iter()
        .any(|d| d.path.fields == vec!["name".to_string()] && d.line == 3);
    assert!(has_name_def, "Should have dev->name def on line 3");

    // Verify field-qualified use exists for RHS
    let has_id_use = dfg.uses.values().any(|locs| {
        locs.iter()
            .any(|l| l.path.base == "dev" && l.path.fields == vec!["id".to_string()] && l.line == 3)
    });
    assert!(has_id_use, "Should have dev->id use on line 3 (RHS)");
}


#[test]
fn test_dfg_assignment_propagation_with_fields() {
    // Taint on dev->id (line 3) should propagate through assignment:
    // dev->id = tainted → x = dev->id → strcpy(buf, x)
    let source = r#"
void process(struct device *dev, const char *input) {
    dev->id = input;
    char *x = dev->id;
    strcpy(buf, x);
}
"#;
    let path = "src/proc.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]),
        }],
    };

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    // Taint should flow: line 3 (dev->id = input) → line 4 (x = dev->id) → line 5 (strcpy)
    assert!(
        !result.findings.is_empty(),
        "Taint should propagate through field assignment to strcpy sink"
    );
}


#[test]
fn test_dfg_forward_reachable_field_to_simple() {
    // Assignment propagation: dev->name = val on line 3, x = dev->name on line 4.
    // Forward reachable from dev->name def should reach x def via assignment propagation.
    let source = r#"
void f(struct dev *dev) {
    dev->name = "test";
    char *x = dev->name;
    printf("%s", x);
}
"#;
    let path = "src/f.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);

    // Find the dev->name def
    let dev_defs = dfg.all_defs_of(path, "dev");
    let name_def = dev_defs
        .iter()
        .find(|d| d.path.fields == vec!["name".to_string()] && d.line == 3);

    assert!(name_def.is_some(), "Should have dev->name def on line 3");

    if let Some(def) = name_def {
        let reachable = dfg.forward_reachable(def);
        let reachable_lines: BTreeSet<usize> = reachable.iter().map(|r| r.line).collect();
        // Should reach line 4 (x = dev->name) and line 5 (printf uses x)
        assert!(
            reachable_lines.contains(&4) || reachable_lines.contains(&5),
            "Forward reachable from dev->name should reach uses. Got lines: {:?}",
            reachable_lines
        );
    }
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
fn test_without_cpg_context_runs_ast_only() {
    // CpgContext::without_cpg should work for AST-only algorithms
    let source = r#"
def add(x, y):
    return x + y
"#;
    let path = "src/add.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]),
        }],
    };

    let ctx = CpgContext::without_cpg(&files, None);
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::OriginalDiff);
    let result = algorithms::run_slicing(&ctx, &diff, &config).unwrap();

    assert!(
        !result.blocks.is_empty(),
        "AST-only algorithm should work with empty CPG context"
    );
}


#[test]
fn test_rta_filters_uninstantiated_class() {
    use prism::type_db::TypeDatabase;

    let source = r#"
class Shape {
public:
    virtual void draw() = 0;
};

class Circle : public Shape {
public:
    float radius;
    virtual void draw();
};

class Square : public Shape {
public:
    float side;
    virtual void draw();
};

void render() {
    Circle c;
    c.draw();
}
"#;
    let path = "src/shapes.cpp";
    let parsed = ParsedFile::parse(path, source, Language::Cpp).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let db = TypeDatabase::from_parsed_files(&files);
    let live = TypeDatabase::collect_live_classes(&files);

    // Circle is instantiated (stack allocation), Square is not
    assert!(
        live.contains("Circle"),
        "Circle should be live, got: {:?}",
        live
    );

    // RTA should include Circle but not Square
    let rta_targets = db.virtual_dispatch_targets_rta("Shape", "draw", &live);
    assert!(
        rta_targets.contains(&"Circle".to_string()),
        "RTA should include Circle"
    );
    assert!(
        !rta_targets.contains(&"Square".to_string()),
        "RTA should exclude uninstantiated Square"
    );

    // CHA should include both
    let cha_targets = db.virtual_dispatch_targets("Shape", "draw");
    assert!(
        cha_targets.contains(&"Circle".to_string()),
        "CHA should include Circle"
    );
    assert!(
        cha_targets.contains(&"Square".to_string()),
        "CHA should include Square"
    );
}


#[test]
fn test_rta_preserves_instantiated_class() {
    use prism::type_db::TypeDatabase;

    let source = r#"
class Base {
public:
    virtual void process();
};

class Derived : public Base {
public:
    virtual void process();
};

void run() {
    Derived d;
    d.process();
}
"#;
    let path = "src/derived.cpp";
    let parsed = ParsedFile::parse(path, source, Language::Cpp).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let db = TypeDatabase::from_parsed_files(&files);
    let live = TypeDatabase::collect_live_classes(&files);

    assert!(live.contains("Derived"));

    let targets = db.virtual_dispatch_targets_rta("Base", "process", &live);
    assert!(
        targets.contains(&"Derived".to_string()),
        "RTA should preserve instantiated Derived"
    );
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
fn test_rta_stack_allocation() {
    use prism::type_db::TypeDatabase;

    let source = r#"
class Processor {
public:
    virtual void run();
};

void main() {
    Processor p;
    p.run();
}
"#;
    let path = "src/proc.cpp";
    let parsed = ParsedFile::parse(path, source, Language::Cpp).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let live = TypeDatabase::collect_live_classes(&files);
    assert!(
        live.contains("Processor"),
        "Stack allocation should count as instantiation"
    );
}


#[test]
fn test_go_multi_return_individual_defs() {
    let source = r#"
package main

func process() {
    val, err := getData()
    use(val)
    check(err)
}
"#;
    let path = "src/process.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);

    let val_defs = dfg.all_defs_of(path, "val");
    let err_defs = dfg.all_defs_of(path, "err");
    assert!(
        !val_defs.is_empty(),
        "Go multi-return: 'val' should have its own def"
    );
    assert!(
        !err_defs.is_empty(),
        "Go multi-return: 'err' should have its own def"
    );

    let composite_defs = dfg.all_defs_of(path, "val, err");
    assert!(
        composite_defs.is_empty(),
        "Go multi-return: should not have composite 'val, err' def, got {:?}",
        composite_defs
    );
}


#[test]
fn test_go_type_assertion_individual_defs() {
    let source = r#"
package main

func check(x interface{}) {
    str, ok := x.(string)
    if ok {
        use(str)
    }
}
"#;
    let path = "src/check.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    assert!(
        !dfg.all_defs_of(path, "str").is_empty(),
        "Go type assertion: 'str' should have def"
    );
    assert!(
        !dfg.all_defs_of(path, "ok").is_empty(),
        "Go type assertion: 'ok' should have def"
    );
}


#[test]
fn test_python_tuple_unpack_individual_defs() {
    let source = r#"
def process():
    name, age = get_user()
    use(name)
    use(age)
"#;
    let path = "src/process.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    assert!(
        !dfg.all_defs_of(path, "name").is_empty(),
        "Python tuple unpack: 'name' should have def"
    );
    assert!(
        !dfg.all_defs_of(path, "age").is_empty(),
        "Python tuple unpack: 'age' should have def"
    );
}


#[test]
fn test_python_star_unpack_individual_defs() {
    let source = r#"
def process():
    first, *rest = get_items()
    use(first)
    use(rest)
"#;
    let path = "src/process.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    assert!(
        !dfg.all_defs_of(path, "first").is_empty(),
        "Python star unpack: 'first' should have def"
    );
    assert!(
        !dfg.all_defs_of(path, "rest").is_empty(),
        "Python star unpack: 'rest' should have def"
    );
}


#[test]
fn test_python_walrus_operator_def() {
    let source = r#"
def process(items):
    if (n := len(items)) > 10:
        use(n)
"#;
    let path = "src/process.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    assert!(
        !dfg.all_defs_of(path, "n").is_empty(),
        "Python walrus operator: 'n' should have def from := expression"
    );
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
fn test_go_assignment_multi_target() {
    let source = r#"
package main

func process() {
    var x, y int
    x, y = getCoords()
    use(x)
}
"#;
    let path = "src/go_assign.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let dfg = DataFlowGraph::build(&files);

    assert!(
        !dfg.all_defs_of(path, "x").is_empty(),
        "Go multi-assign: 'x' should have def"
    );
    assert!(
        !dfg.all_defs_of(path, "y").is_empty(),
        "Go multi-assign: 'y' should have def"
    );
}


#[test]
fn test_optional_chaining_single_level() {
    let ap = AccessPath::from_expr("obj?.name");
    assert_eq!(ap.base, "obj");
    assert_eq!(ap.fields, vec!["name"]);
}


#[test]
fn test_optional_chaining_element_access() {
    let ap = AccessPath::from_expr("arr?.[0]");
    assert_eq!(ap.base, "arr");
    assert!(
        !ap.fields.is_empty(),
        "arr?.[0] should produce fields, got {:?}",
        ap
    );
}


#[test]
fn test_optional_chaining_deep() {
    let ap = AccessPath::from_expr("a?.b?.c?.d");
    assert_eq!(ap.base, "a");
    assert_eq!(ap.fields, vec!["b", "c", "d"]);
}


#[test]
fn test_optional_chaining_does_not_break_arrow() {
    let ap = AccessPath::from_expr("dev->config->host");
    assert_eq!(ap.base, "dev");
    assert_eq!(ap.fields, vec!["config", "host"]);
}


#[test]
fn test_walrus_does_not_affect_regular_assignment() {
    let source = r#"
def process():
    x = 42
    y = x + 1
"#;
    let path = "src/regular.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let func = parsed.all_functions().into_iter().next().unwrap();
    let lines: BTreeSet<usize> = (1..=5).collect();
    let lvalues = parsed.assignment_lvalue_paths_on_lines(&func, &lines);

    assert!(
        lvalues.iter().any(|(p, _)| p.base == "x"),
        "Regular assignment: 'x' should have L-value"
    );
    assert!(
        lvalues.iter().any(|(p, _)| p.base == "y"),
        "Regular assignment: 'y' should have L-value"
    );
}


#[test]
fn test_walrus_does_not_affect_augmented_assignment() {
    let source = r#"
def process():
    x = 0
    x += 1
"#;
    let path = "src/augmented.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let func = parsed.all_functions().into_iter().next().unwrap();
    let lines: BTreeSet<usize> = (1..=5).collect();
    let lvalues = parsed.assignment_lvalue_paths_on_lines(&func, &lines);

    let x_count = lvalues.iter().filter(|(p, _)| p.base == "x").count();
    assert!(
        x_count >= 2,
        "Augmented assignment: 'x' should have defs from both = and +=, got {}",
        x_count
    );
}


#[test]
fn test_walrus_in_while_loop() {
    let source = r#"
def process(stream):
    while (chunk := stream.read(1024)):
        use(chunk)
"#;
    let path = "src/walrus_while.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let dfg = DataFlowGraph::build(&files);

    assert!(
        !dfg.all_defs_of(path, "chunk").is_empty(),
        "Walrus in while: 'chunk' should have def from := expression"
    );
}


#[test]
fn test_python_nested_tuple_unpack() {
    let source = r#"
def process():
    (a, b), c = get_nested()
    use(a)
    use(b)
    use(c)
"#;
    let path = "src/nested.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let func = parsed.all_functions().into_iter().next().unwrap();
    let lines: BTreeSet<usize> = (1..=7).collect();
    let lvalues = parsed.assignment_lvalue_paths_on_lines(&func, &lines);

    let has_a = lvalues.iter().any(|(p, _)| p.base == "a");
    let has_b = lvalues.iter().any(|(p, _)| p.base == "b");
    let has_c = lvalues.iter().any(|(p, _)| p.base == "c");
    assert!(
        has_a,
        "Nested tuple: 'a' should have L-value, got {:?}",
        lvalues.iter().map(|(p, _)| &p.base).collect::<Vec<_>>()
    );
    assert!(has_b, "Nested tuple: 'b' should have L-value");
    assert!(has_c, "Nested tuple: 'c' should have L-value");
}


#[test]
fn test_python_walrus_rhs_collected() {
    // Walrus RHS should be collected as a use (assignment_value must work)
    let source = r#"
def process(items):
    if (n := len(items)) > 10:
        use(n)
"#;
    let path = "src/walrus.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let func = parsed.all_functions().into_iter().next().unwrap();
    let lines: BTreeSet<usize> = (1..=4).collect();

    let aliases = parsed.collect_alias_assignments(&func, &lines);
    // Walrus RHS is a call, not plain ident — no alias expected, but should not panic.
    // The key test is that assignment_value returns Some for named_expression.
    let _ = aliases;

    // Verify def exists for n
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let dfg = DataFlowGraph::build(&files);
    let n_defs = dfg.all_defs_of(path, "n");
    assert!(!n_defs.is_empty(), "Walrus: 'n' should have a def");
}


#[test]
fn test_python_with_as_binding_def() {
    // Gap 4: with...as should produce a def for the bound variable
    let source = r#"
def process():
    with open("file.txt") as f:
        data = f.read()
        send(data)
"#;
    let path = "src/process.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let f_defs = dfg.all_defs_of(path, "f");
    assert!(!f_defs.is_empty(), "with...as: 'f' should have def");
}


#[test]
fn test_python_except_as_binding_def() {
    // except...as uses as_pattern too
    let source = r#"
def process():
    try:
        risky()
    except Exception as e:
        handle(e)
"#;
    let path = "src/process.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let e_defs = dfg.all_defs_of(path, "e");
    assert!(!e_defs.is_empty(), "except...as: 'e' should have def");
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


#[test]
fn test_walrus_assignment_value_flow() {
    // Walrus assignment_value should extract RHS identifiers for DFG use tracking.
    // If items is on a diff line, its use on the walrus RHS should be collected.
    let source = r#"
def process(items):
    if (n := len(items)) > 10:
        use(n)
"#;
    let path = "src/walrus.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let func = parsed.all_functions().into_iter().next().unwrap();
    let lines: BTreeSet<usize> = (1..=4).collect();

    // The walrus RHS is len(items) — not a plain ident, so no alias.
    // But the assignment_value path should be Some (not None).
    let aliases = parsed.collect_alias_assignments(&func, &lines);
    // No alias expected (RHS is a call), but shouldn't panic
    let _ = aliases;

    // Verify DFG has defs
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let dfg = DataFlowGraph::build(&files);
    let n_defs = dfg.all_defs_of(path, "n");
    assert!(!n_defs.is_empty(), "Walrus: 'n' def should exist");
}


#[test]
fn test_go_short_var_declaration_value() {
    // Go short_var_declaration uses "right" field for declaration_value
    let source = r#"
package main

func process() {
    x := getData()
    use(x)
}
"#;
    let path = "src/process.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let func = parsed.all_functions().into_iter().next().unwrap();
    let lines: BTreeSet<usize> = (1..=6).collect();

    // short_var_declaration is both declaration and assignment in Go
    let aliases = parsed.collect_alias_assignments(&func, &lines);
    // getData() is not a plain ident, so no alias — but the path should not crash
    let _ = aliases;

    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let dfg = DataFlowGraph::build(&files);
    let x_defs = dfg.all_defs_of(path, "x");
    assert!(
        !x_defs.is_empty(),
        "Go short_var_declaration: 'x' should have def"
    );
}


#[test]
fn test_go_var_declaration_with_value() {
    // Go var_declaration with explicit value
    let source = r#"
package main

func process() {
    var x int = 42
    use(x)
}
"#;
    let path = "src/process.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let x_defs = dfg.all_defs_of(path, "x");
    assert!(
        !x_defs.is_empty(),
        "Go var declaration: 'x' should have def"
    );
}

