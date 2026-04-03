#[path = "../common/mod.rs"]
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
