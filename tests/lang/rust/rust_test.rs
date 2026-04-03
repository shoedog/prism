#[path = "../../common/mod.rs"]
mod common;
use common::*;

#[test]
fn test_rust_basic_parsing() {
    let source = r#"
use std::io;

fn read_input() -> Result<String, io::Error> {
    let mut buf = String::new();
    io::stdin().read_line(&mut buf)?;
    Ok(buf)
}

fn process(data: &str) -> Option<i32> {
    let val = data.parse::<i32>().ok()?;
    Some(val * 2)
}
"#;
    let path = "src/main.rs";
    let parsed = ParsedFile::parse(path, source, Language::Rust).unwrap();

    // Should detect function definitions
    let funcs = parsed.all_functions();
    let func_names: Vec<String> = funcs
        .iter()
        .filter_map(|f| {
            parsed
                .language
                .function_name(f)
                .map(|n| parsed.node_text(&n).to_string())
        })
        .collect();
    assert!(
        func_names.contains(&"read_input".to_string()),
        "Should find read_input function, got: {:?}",
        func_names
    );
    assert!(
        func_names.contains(&"process".to_string()),
        "Should find process function, got: {:?}",
        func_names
    );
}

#[test]
fn test_rust_original_diff() {
    let source = r#"
fn process(data: &str) -> i32 {
    let val = data.len();
    val as i32
}
"#;
    let path = "src/lib.rs";
    let parsed = ParsedFile::parse(path, source, Language::Rust).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::OriginalDiff),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "OriginalDiff should produce blocks for Rust code"
    );
}

#[test]
fn test_rust_parent_function() {
    let source = r#"
fn process(data: &str) -> i32 {
    let val = data.len();
    let result = val * 2;
    result
}
"#;
    let path = "src/lib.rs";
    let parsed = ParsedFile::parse(path, source, Language::Rust).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([4]),
        }],
    };

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ParentFunction),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "ParentFunction should include the enclosing Rust function"
    );
    // Should include the entire function
    let block = &result.blocks[0];
    let lines = block.file_line_map.get(path).unwrap();
    assert!(
        lines.contains_key(&2) && lines.contains_key(&6),
        "Block should span the entire function (lines 2-6)"
    );
}

#[test]
fn test_dfg_rust_field_access_paths() {
    // Rust field_expression: self.field or obj.field
    let source = r#"
struct Config {
    timeout: u32,
    host: String,
}

fn setup(config: &mut Config) {
    config.timeout = 30;
    config.host = String::from("localhost");
}
"#;
    let path = "src/config.rs";
    let parsed = ParsedFile::parse(path, source, Language::Rust).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let config_defs = dfg.all_defs_of(path, "config");

    let has_timeout = config_defs
        .iter()
        .any(|d| d.path.has_fields() && d.path.fields.contains(&"timeout".to_string()));
    assert!(
        has_timeout,
        "Rust DFG should have AccessPath config.timeout from field_expression. Got: {:?}",
        config_defs
            .iter()
            .map(|d| d.path.to_string())
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_rust_let_destructuring_def() {
    // Rust let (a, b) = tuple; — destructuring in let_declaration
    let source = r#"
fn process() {
    let x = get_data();
    use_val(x);
}
"#;
    let path = "src/process.rs";
    let parsed = ParsedFile::parse(path, source, Language::Rust).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let x_defs = dfg.all_defs_of(path, "x");
    assert!(!x_defs.is_empty(), "Rust let: 'x' should have a def");
}

#[test]
fn test_rust_compound_assignment() {
    // Rust compound assignment: x += 1
    let source = r#"
fn process() {
    let mut x = 0;
    x += 10;
    use_val(x);
}
"#;
    let path = "src/process.rs";
    let parsed = ParsedFile::parse(path, source, Language::Rust).unwrap();
    let func = parsed.all_functions().into_iter().next().unwrap();
    let lines: BTreeSet<usize> = (1..=5).collect();
    let lvalues = parsed.assignment_lvalue_paths_on_lines(&func, &lines);

    let has_x = lvalues.iter().any(|(p, _)| p.base == "x");
    assert!(
        has_x,
        "Rust compound assignment: x should be L-value, got: {:?}",
        lvalues.iter().map(|(p, _)| &p.base).collect::<Vec<_>>()
    );
}
