#[path = "../common/mod.rs"]
mod common;
use common::*;

#[test]
fn test_cpp_parses_and_finds_methods() {
    let (files, _, _) = make_cpp_test();
    let parsed = files.get("src/device_manager.cpp").unwrap();

    let functions = parsed.all_functions();
    let func_names: Vec<String> = functions
        .iter()
        .filter_map(|f| {
            parsed
                .language
                .function_name(f)
                .map(|n| parsed.node_text(&n).to_string())
        })
        .collect();

    assert!(
        func_names.contains(&"process_devices".to_string()),
        "Should find process_devices, got: {:?}",
        func_names
    );
    // C++ methods inside classes should also be found
    assert!(
        func_names.len() >= 2,
        "Should find at least free function + some class methods, got {} functions: {:?}",
        func_names.len(),
        func_names
    );
}


#[test]
fn test_ts_fallback_extracts_cpp_class() {
    use prism::type_db::TypeDatabase;

    let source = r#"
class Shape {
public:
    virtual void draw() = 0;
    int x;
    int y;
};
"#;
    let path = "src/shape.cpp";
    let parsed = ParsedFile::parse(path, source, Language::Cpp).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let db = TypeDatabase::from_parsed_files(&files);

    let record = db.records.get("Shape").expect("should extract Shape class");
    assert_eq!(record.kind, prism::type_db::RecordKind::Class);
    assert!(
        record.virtual_methods.contains_key("draw"),
        "should detect virtual draw method"
    );
    let field_names: Vec<&str> = record.fields.iter().map(|f| f.name.as_str()).collect();
    assert!(field_names.contains(&"x"));
    assert!(field_names.contains(&"y"));
}


#[test]
fn test_ts_fallback_skips_forward_decl() {
    use prism::type_db::TypeDatabase;

    let source = r#"
struct device;
void use_device(struct device *d);
"#;
    let path = "src/forward.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let db = TypeDatabase::from_parsed_files(&files);

    assert!(
        db.records.is_empty(),
        "Forward declaration should not be extracted as a record"
    );
}


#[test]
fn test_ts_fallback_union_detection() {
    use prism::type_db::TypeDatabase;

    let source = r#"
union data {
    int i;
    float f;
    char bytes[4];
};
"#;
    let path = "src/data.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let db = TypeDatabase::from_parsed_files(&files);

    let record = db.records.get("data").expect("should extract data union");
    assert_eq!(record.kind, prism::type_db::RecordKind::Union);
}


#[test]
fn test_ts_fallback_nested_struct() {
    use prism::type_db::TypeDatabase;

    let source = r#"
struct config {
    int timeout;
    int retries;
};

struct device {
    char *name;
    struct config *cfg;
};
"#;
    let path = "src/device.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let db = TypeDatabase::from_parsed_files(&files);

    assert!(
        db.records.contains_key("config"),
        "should extract config struct"
    );
    assert!(
        db.records.contains_key("device"),
        "should extract device struct"
    );
    let device = db.records.get("device").unwrap();
    let field_names: Vec<&str> = device.fields.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(field_names, vec!["name", "cfg"]);
}


#[test]
fn test_ts_fallback_no_false_extraction() {
    use prism::type_db::TypeDatabase;

    // Python and JS files should produce no records
    let py_source = r#"
class Device:
    def __init__(self, name):
        self.name = name
"#;
    let js_source = r#"
class Device {
    constructor(name) {
        this.name = name;
    }
}
"#;
    let mut files = BTreeMap::new();
    let py_parsed = ParsedFile::parse("src/device.py", py_source, Language::Python).unwrap();
    let js_parsed = ParsedFile::parse("src/device.js", js_source, Language::JavaScript).unwrap();
    files.insert("src/device.py".to_string(), py_parsed);
    files.insert("src/device.js".to_string(), js_parsed);

    let db = TypeDatabase::from_parsed_files(&files);

    assert!(
        db.records.is_empty(),
        "Non-C/C++ files should produce no records"
    );
}


#[test]
fn test_ts_fallback_cpp_inheritance() {
    use prism::type_db::TypeDatabase;

    let source = r#"
class Shape {
public:
    virtual void draw() = 0;
    int x;
};

class Circle : public Shape {
    float radius;
public:
    virtual void draw();
};
"#;
    let path = "src/shapes.cpp";
    let parsed = ParsedFile::parse(path, source, Language::Cpp).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let db = TypeDatabase::from_parsed_files(&files);

    let circle = db.records.get("Circle").expect("should extract Circle");
    assert!(
        circle.bases.contains(&"Shape".to_string()),
        "Circle should have Shape as base class, got: {:?}",
        circle.bases
    );
    assert!(
        db.class_hierarchy.contains_key("Circle"),
        "Class hierarchy should include Circle"
    );
    assert!(db.is_subclass_of("Circle", "Shape"));
}


#[test]
fn test_ts_fallback_typedef() {
    use prism::type_db::TypeDatabase;

    let source = r#"
struct device {
    char *name;
    int id;
};

typedef struct device dev_t;
typedef int handle_t;
"#;
    let path = "src/types.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let db = TypeDatabase::from_parsed_files(&files);

    assert!(db.records.contains_key("device"));
    assert!(
        db.typedefs.contains_key("dev_t"),
        "should extract dev_t typedef"
    );
    assert!(
        db.typedefs.contains_key("handle_t"),
        "should extract handle_t typedef"
    );
}


#[test]
fn test_cpp_update_expression_def() {
    // C/C++ update_expression (++/--) is treated as assignment
    let source = r#"
void process() {
    int count = 0;
    count++;
    ++count;
    count--;
    use(count);
}
"#;
    let path = "src/process.cpp";
    let parsed = ParsedFile::parse(path, source, Language::Cpp).unwrap();
    let func = parsed.all_functions().into_iter().next().unwrap();
    let lines: BTreeSet<usize> = (1..=7).collect();
    let lvalues = parsed.assignment_lvalue_paths_on_lines(&func, &lines);

    // count should have a def from the initial declaration
    let has_count = lvalues.iter().any(|(p, _)| p.base == "count");
    assert!(
        has_count,
        "C++ count should have L-value, got: {:?}",
        lvalues.iter().map(|(p, _)| &p.base).collect::<Vec<_>>()
    );
}

