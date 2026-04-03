#[path = "../common/mod.rs"]
mod common;
use common::*;

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
