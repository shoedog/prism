use crate::common::*;

#[test]
fn var_location_ord_eq_hash_agree_excluding_byte() {
    use prism::access_path::AccessPath;
    use prism::data_flow::{VarAccessKind, VarLocation};
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let base = |sb, eb| VarLocation {
        file: "a.rs".into(),
        function: "f".into(),
        function_start_line: 1,
        line: 5,
        path: AccessPath::simple("x"),
        start_byte: sb,
        end_byte: eb,
        kind: VarAccessKind::Use,
    };
    let (a, b) = (base(10, 11), base(99, 100));
    assert_eq!(a, b, "byte excluded from Eq");
    assert_eq!(
        a.cmp(&b),
        std::cmp::Ordering::Equal,
        "byte excluded from Ord"
    );
    let h = |v: &VarLocation| {
        let mut s = DefaultHasher::new();
        v.hash(&mut s);
        s.finish()
    };
    assert_eq!(h(&a), h(&b), "byte excluded from Hash");
    let mut c = base(10, 11);
    c.function_start_line = 7;
    assert_ne!(a, c);
    assert_ne!(a.cmp(&c), std::cmp::Ordering::Equal);
}

#[test]
fn two_calls_one_line_bind_their_own_args() {
    let cpg = build_rust_cpg("fn f(p: i32) { let _ = p; }\nfn c(a: i32, b: i32) { f(a); f(b); }\n");

    assert!(arg_binds(&cpg, "a", "p"), "a -> p");
    assert!(arg_binds(&cpg, "b", "p"), "b -> p");
}

#[test]
fn nested_augmented_base_peels_to_leftmost() {
    let source = concat!(
        "struct C { config: Cfg }\n",
        "struct Cfg { timeout: i32 }\n",
        "fn f(mut o: C) { o.config.timeout += 1; }\n",
    );
    let parsed = ParsedFile::parse("src/lib.rs", source, Language::Rust).unwrap();
    let mut files = BTreeMap::new();
    files.insert("src/lib.rs".to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let has_leftmost_o_use = dfg.uses.values().any(|locs| {
        locs.iter().any(|loc| {
            loc.path == AccessPath::simple("o") && loc.kind == prism::data_flow::VarAccessKind::Use
        })
    });

    assert!(
        has_leftmost_o_use,
        "`o.config.timeout += 1` should emit a Use of leftmost base `o`"
    );
}

#[test]
fn line_collapsed_reference_start_eq_end() {
    let source = "fn f(x: i32) {\n    if x > 0 {}\n}\n";
    let parsed = ParsedFile::parse("src/lib.rs", source, Language::Rust).unwrap();
    let mut files = BTreeMap::new();
    files.insert("src/lib.rs".to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let parsed = files.get("src/lib.rs").unwrap();
    let collapsed_refs: Vec<_> = dfg
        .uses
        .values()
        .flat_map(|locs| locs.iter())
        .filter(|loc| loc.start_byte == parsed.line_start_byte(loc.line))
        .collect();

    assert!(
        !collapsed_refs.is_empty(),
        "expected at least one line-collapsed reference"
    );
    assert!(
        collapsed_refs
            .iter()
            .all(|loc| loc.start_byte == loc.end_byte),
        "line-collapsed references must be zero-width anchors: {collapsed_refs:?}"
    );
}

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

// S2 full-branch review BLOCKER 2: dfg_forward_reachable's same-line Use->Def propagation
// must stay within the Use's function. Two functions on ONE minified line — a Use in `a`
// must not leak to a Def in `b` via the (file,line) bucket.
#[test]
fn dfg_forward_reachable_does_not_leak_across_same_line_functions() {
    use prism::access_path::AccessPath;
    use prism::data_flow::{VarAccessKind, VarLocation};

    let src = "fn a() { let x = src(); sink(x); } fn b() { let y = src(); sink(y); }\n";
    let cpg = build_rust_cpg(src);
    let seed = VarLocation {
        file: "test.rs".into(),
        function: "a".into(),
        function_start_line: 1,
        line: 1,
        path: AccessPath::simple("x"),
        start_byte: 0,
        end_byte: 0,
        kind: VarAccessKind::Use,
    };
    let reached = cpg.dfg_forward_reachable(&seed);
    assert!(
        reached.iter().all(|l| l.function == "a"),
        "forward reachability leaked out of a() into another same-line function: {reached:?}"
    );
}
