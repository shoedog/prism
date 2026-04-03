mod common;
use common::*;

#[test]
fn test_paper_format_output() {
    let (files, _, diff) = make_python_test();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::OriginalDiff);
    let result = algorithms::run_slicing_compat(&files, &diff, &config, None).unwrap();

    let paper = output::to_paper_format(&result.blocks);
    assert!(paper.is_array());
    let arr = paper.as_array().unwrap();
    assert!(!arr.is_empty());
    assert!(arr[0].get("block_id").is_some());
    assert!(arr[0].get("diff_lines").is_some());
    assert!(arr[0].get("diff_list").is_some());
}

#[test]
fn test_text_format_output() {
    let (files, sources, diff) = make_python_test();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::ParentFunction);
    let result = algorithms::run_slicing_compat(&files, &diff, &config, None).unwrap();

    let text = output::format_slice_result(&result.blocks, &sources);
    // Should have diff markers
    assert!(text.contains('+'), "Should have + markers for diff lines");
    // Should have block header
    assert!(text.contains("Block"), "Should have block header");
}

#[test]
fn test_json_diff_input() {
    let json = r#"{
        "files": [
            {
                "file_path": "test.py",
                "modify_type": "Modified",
                "diff_lines": [1, 5, 10]
            }
        ]
    }"#;

    let input = DiffInput::from_json(json).unwrap();
    assert_eq!(input.files.len(), 1);
    assert_eq!(input.files[0].diff_lines.len(), 3);
}

#[test]
fn test_all_languages_parse() {
    let cases = vec![
        ("test.py", Language::Python, "def foo():\n    return 1\n"),
        (
            "test.js",
            Language::JavaScript,
            "function foo() { return 1; }\n",
        ),
        (
            "test.ts",
            Language::TypeScript,
            "function foo(): number { return 1; }\n",
        ),
        (
            "test.go",
            Language::Go,
            "package main\nfunc foo() int { return 1 }\n",
        ),
        (
            "test.java",
            Language::Java,
            "class T { int foo() { return 1; } }\n",
        ),
    ];

    for (path, lang, source) in cases {
        let parsed = ParsedFile::parse(path, source, lang);
        assert!(
            parsed.is_ok(),
            "Failed to parse {}: {:?}",
            path,
            parsed.err()
        );
    }
}

#[test]
fn test_all_algorithms_listed() {
    let all = SlicingAlgorithm::all();
    assert_eq!(all.len(), 26, "Should have 26 algorithms total");

    // Verify each can be round-tripped through from_str
    for algo in &all {
        let name = algo.name();
        let parsed = SlicingAlgorithm::from_str(name);
        assert!(parsed.is_some(), "Should parse algorithm name: {}", name);
    }
}

#[test]
fn test_left_flow_c() {
    let (files, sources, diff) = make_c_test();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow);
    let result = algorithms::run_slicing_compat(&files, &diff, &config, None).unwrap();

    assert!(
        !result.blocks.is_empty(),
        "LeftFlow should produce output for C code"
    );
    let formatted = output::format_slice_result(&result.blocks, &sources);
    // Should include the process_packet function context
    assert!(
        formatted.contains("local_buf")
            || formatted.contains("memcpy")
            || formatted.contains("buf"),
        "LeftFlow should trace buffer-related variables in C code"
    );
}

#[test]
fn test_left_flow_cpp() {
    let (files, sources, diff) = make_cpp_test();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow);
    let result = algorithms::run_slicing_compat(&files, &diff, &config, None).unwrap();

    assert!(
        !result.blocks.is_empty(),
        "LeftFlow should produce output for C++ code"
    );
    let formatted = output::format_slice_result(&result.blocks, &sources);
    assert!(
        formatted.contains("device")
            || formatted.contains("lock")
            || formatted.contains("add_device"),
        "LeftFlow should include C++ method context"
    );
}

#[test]
fn test_full_flow_c() {
    let (files, sources, diff) = make_c_test();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::FullFlow);
    let result = algorithms::run_slicing_compat(&files, &diff, &config, None).unwrap();

    assert!(
        !result.blocks.is_empty(),
        "FullFlow should produce output for C code"
    );
    let formatted = output::format_slice_result(&result.blocks, &sources);
    assert!(
        formatted.contains("result") || formatted.contains("local_buf"),
        "FullFlow should trace forward from the buffer operations"
    );
}

#[test]
fn test_full_flow_cpp() {
    let (files, sources, diff) = make_cpp_test();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::FullFlow);
    let result = algorithms::run_slicing_compat(&files, &diff, &config, None).unwrap();

    assert!(
        !result.blocks.is_empty(),
        "FullFlow should produce output for C++ code"
    );
}

#[test]
fn test_review_output_format_single_algorithm() {
    let (files, sources, diff) = make_python_test();

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow),
        None,
    )
    .unwrap();

    let review = to_review_output(&result, &sources);

    // Verify schema fields
    assert_eq!(review.algorithm, "LeftFlow");
    assert!(
        review.slices.iter().all(|s| !s.file.is_empty()),
        "Each slice block should have a file"
    );
    assert!(
        review.slices.iter().all(|s| !s.modify_type.is_empty()),
        "Each slice block should have a modify_type"
    );

    // Verify serialization to JSON succeeds and is valid
    let json = serde_json::to_string_pretty(&review).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["algorithm"], "LeftFlow");
    assert!(parsed["slices"].is_array());
    assert!(parsed["findings"].is_array());
}

#[test]
fn test_review_output_json_schema_multi() {
    let (files, sources, diff) = make_python_test();

    let algorithms_to_run = vec![SlicingAlgorithm::LeftFlow, SlicingAlgorithm::ThinSlice];
    let mut results = vec![];
    for &algo in &algorithms_to_run {
        let r = algorithms::run_slicing_compat(
            &files,
            &diff,
            &SliceConfig::default().with_algorithm(algo),
            None,
        )
        .unwrap();
        results.push(r);
    }

    let algorithms_run: Vec<String> = algorithms_to_run
        .iter()
        .map(|a| a.name().to_string())
        .collect();
    let all_findings: Vec<SliceFinding> = results.iter().flat_map(|r| r.findings.clone()).collect();
    let review_results: Vec<_> = results
        .iter()
        .map(|r| to_review_output(r, &sources))
        .collect();

    let multi = MultiReviewOutput {
        version: "1.0".to_string(),
        algorithms_run: algorithms_run.clone(),
        results: review_results,
        all_findings,
        errors: vec![],
        warnings: vec![],
    };

    // Verify schema
    assert_eq!(multi.version, "1.0");
    assert_eq!(multi.algorithms_run.len(), 2);
    assert!(multi.algorithms_run.contains(&"LeftFlow".to_string()));
    assert!(multi.algorithms_run.contains(&"ThinSlice".to_string()));
    assert_eq!(multi.results.len(), 2);

    // Verify valid JSON
    let json = serde_json::to_string_pretty(&multi).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed["version"], "1.0");
    assert_eq!(parsed["algorithms_run"].as_array().unwrap().len(), 2);
    assert!(parsed["results"].is_array());
    assert!(parsed["all_findings"].is_array());
}

#[test]
fn test_review_suite_list() {
    let suite = SlicingAlgorithm::review_suite();
    // Review suite should be non-empty and contain core algorithms
    assert!(!suite.is_empty());
    assert!(suite.contains(&SlicingAlgorithm::LeftFlow));
    assert!(suite.contains(&SlicingAlgorithm::FullFlow));
    assert!(suite.contains(&SlicingAlgorithm::Taint));
    assert!(suite.contains(&SlicingAlgorithm::AbsenceSlice));
    assert!(suite.contains(&SlicingAlgorithm::EchoSlice));
    // Git-history-only algorithms should NOT be in the review suite
    assert!(!suite.contains(&SlicingAlgorithm::ResonanceSlice));
    assert!(!suite.contains(&SlicingAlgorithm::PhantomSlice));
}

#[test]
fn test_multi_algorithm_findings_merged() {
    let (files, sources, diff) = make_python_test();

    let algorithms_to_run = SlicingAlgorithm::review_suite();
    let mut all_results = vec![];
    let mut errors = vec![];

    for &algo in &algorithms_to_run {
        match algorithms::run_slicing_compat(
            &files,
            &diff,
            &SliceConfig::default().with_algorithm(algo),
            None,
        ) {
            Ok(r) => all_results.push(r),
            Err(e) => errors.push(e.to_string()),
        }
    }

    // Collect all findings across all algorithms
    let merged_findings: Vec<SliceFinding> = all_results
        .iter()
        .flat_map(|r| r.findings.clone())
        .collect();

    // All findings should have non-empty required fields
    for finding in &merged_findings {
        assert!(!finding.algorithm.is_empty());
        assert!(!finding.file.is_empty());
        assert!(!finding.description.is_empty());
        assert!(["info", "warning", "concern"].contains(&finding.severity.as_str()));
    }

    // Results count should match algorithms that succeeded (no panics)
    let review_results: Vec<_> = all_results
        .iter()
        .map(|r| to_review_output(r, &sources))
        .collect();
    let multi = MultiSliceResult {
        version: "1.0".to_string(),
        algorithms_run: algorithms_to_run
            .iter()
            .map(|a| a.name().to_string())
            .collect(),
        results: all_results,
        findings: merged_findings,
        errors: vec![],
        warnings: vec![],
    };

    assert_eq!(multi.version, "1.0");
    assert_eq!(multi.results.len(), review_results.len());
    assert!(multi.algorithms_run.contains(&"LeftFlow".to_string()));

    // JSON round-trip
    let json = serde_json::to_string_pretty(&multi).unwrap();
    let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
    assert!(parsed["results"].is_array());
    assert!(parsed["findings"].is_array());
}

#[test]
fn test_full_flow_python_no_returns() {
    let (files, _, diff) = make_python_test();
    let config = SliceConfig {
        algorithm: SlicingAlgorithm::FullFlow,
        include_returns: false,
        trace_callees: false,
        ..SliceConfig::default()
    };
    let result = algorithms::run_slicing_compat(&files, &diff, &config, None).unwrap();
    assert!(!result.blocks.is_empty());
}

#[test]
fn test_full_flow_go_trace_callees() {
    let (files, _, diff) = make_go_test();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::FullFlow);
    let result = algorithms::run_slicing_compat(&files, &diff, &config, None).unwrap();
    assert!(!result.blocks.is_empty());
}

#[test]
fn test_original_diff_go() {
    let (files, _, diff) = make_go_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::OriginalDiff),
        None,
    )
    .unwrap();
    assert!(!result.blocks.is_empty());
}

#[test]
fn test_left_flow_go() {
    let (files, _, diff) = make_go_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow),
        None,
    )
    .unwrap();
    assert!(!result.blocks.is_empty());
}

#[test]
fn test_left_flow_java() {
    let (files, _, diff) = make_java_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow),
        None,
    )
    .unwrap();
    assert!(!result.blocks.is_empty());
}

#[test]
fn test_parent_function_global_scope_python() {
    // Test parent function with diff lines outside any function
    let source = r#"
import os

CONFIG = os.getenv("CONFIG", "default")
DATA_DIR = "/tmp/data"

def process():
    return CONFIG
"#;
    let path = "config.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([4, 5]),
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
        "ParentFunction should include global diff lines"
    );
}

#[test]
fn test_parent_function_global_scope_go() {
    let source = r#"package main

var Config = "default"
var Port = 8080

func main() {
	println(Config)
}
"#;
    let path = "main.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3, 4]),
        }],
    };

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ParentFunction),
        None,
    )
    .unwrap();
    assert!(!result.blocks.is_empty());
}

#[test]
fn test_full_flow_trace_callees_python() {
    // Test with trace_callees enabled to cover cross-file R-value resolution
    let source_main = r#"
def process(data):
    result = transform(data)
    return result
"#;
    let source_transform = r#"
def transform(x):
    return x * 2
"#;
    let mut files = BTreeMap::new();
    files.insert(
        "main.py".to_string(),
        ParsedFile::parse("main.py", source_main, Language::Python).unwrap(),
    );
    files.insert(
        "transform.py".to_string(),
        ParsedFile::parse("transform.py", source_transform, Language::Python).unwrap(),
    );

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "main.py".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3]),
        }],
    };

    let config = SliceConfig {
        algorithm: SlicingAlgorithm::FullFlow,
        include_returns: true,
        trace_callees: true,
        ..SliceConfig::default()
    };
    let result = algorithms::run_slicing_compat(&files, &diff, &config, None).unwrap();
    assert!(!result.blocks.is_empty());
}

#[test]
fn test_must_alias_python() {
    // ref = self; ref.secret = x → should create def for self.secret too
    let source = r#"
class Handler:
    def process(self):
        ref = self
        ref.secret = get_password()
        send(self.secret)
"#;
    let path = "src/alias.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let self_defs = dfg.all_defs_of(path, "self");

    let has_self_secret = self_defs.iter().any(|d| d.path.fields == vec!["secret"]);
    assert!(
        has_self_secret,
        "Phase 3 Python: ref = self alias should resolve ref.secret to self.secret"
    );
}

#[test]
fn test_must_alias_javascript() {
    let source = r#"
function process(config) {
    const ref = config;
    ref.timeout = getUserInput();
    use(config.timeout);
}
"#;
    let path = "src/alias.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let config_defs = dfg.all_defs_of(path, "config");

    let has_config_timeout = config_defs.iter().any(|d| d.path.fields == vec!["timeout"]);
    assert!(
        has_config_timeout,
        "Phase 3 JS: ref = config alias should resolve ref.timeout to config.timeout"
    );
}

#[test]
fn test_must_alias_go() {
    let source = r#"
package main

func process(dev Device) {
    ref := dev
    ref.Name = getInput()
    useName(dev.Name)
}
"#;
    let path = "src/alias.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let dev_defs = dfg.all_defs_of(path, "dev");

    let has_dev_name = dev_defs.iter().any(|d| d.path.fields == vec!["Name"]);
    assert!(
        has_dev_name,
        "Phase 3 Go: ref := dev alias should resolve ref.Name to dev.Name"
    );
}

#[test]
fn test_must_alias_rust() {
    let source = r#"
fn process(dev: &mut Device) {
    let ptr = dev;
    ptr.name = get_input();
    use_name(dev.name);
}
"#;
    let path = "src/alias.rs";
    let parsed = ParsedFile::parse(path, source, Language::Rust).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let dev_defs = dfg.all_defs_of(path, "dev");

    let has_dev_name = dev_defs.iter().any(|d| d.path.fields == vec!["name"]);
    assert!(
        has_dev_name,
        "Phase 3 Rust: ptr = dev alias should resolve ptr.name to dev.name"
    );
}

#[test]
fn test_must_alias_chain() {
    // Chain: a = dev; b = a; b->field → should resolve to dev.field
    let source = r#"
void chain(struct device *dev) {
    struct device *a = dev;
    struct device *b = a;
    b->name = "test";
    use_name(dev->name);
}
"#;
    let path = "src/chain.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let dev_defs = dfg.all_defs_of(path, "dev");

    let has_dev_name = dev_defs.iter().any(|d| d.path.fields == vec!["name"]);
    assert!(
        has_dev_name,
        "Phase 3: chained aliases (b = a = dev) should resolve b->name to dev.name"
    );
}

#[test]
fn test_must_alias_no_false_positive() {
    // x = unrelated_var should NOT alias to dev
    let source = r#"
void no_alias(struct device *dev, struct device *other) {
    struct device *ptr = other;
    ptr->name = "test";
    use_name(dev->name);
}
"#;
    let path = "src/no_alias.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let dev_defs = dfg.all_defs_of(path, "dev");

    // dev should NOT have a name def from ptr->name (ptr aliases other, not dev)
    let has_dev_name = dev_defs.iter().any(|d| d.path.fields == vec!["name"]);
    assert!(
        !has_dev_name,
        "Phase 3: ptr = other should NOT create alias to dev. Got defs: {:?}",
        dev_defs.iter().map(|d| &d.path).collect::<Vec<_>>()
    );
}

#[test]
fn test_cpg_with_type_enrichment() {
    use prism::cpg::CodePropertyGraph;
    use prism::type_db::{FieldInfo, RecordInfo, RecordKind, TypeDatabase, TypedefInfo};

    // Build a TypeDatabase manually
    let mut type_db = TypeDatabase::default();
    type_db.records.insert(
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
            ],
            bases: vec![],
            virtual_methods: std::collections::BTreeMap::new(),
            size: None,
            file: "device.h".to_string(),
        },
    );
    type_db.typedefs.insert(
        "dev_t".to_string(),
        TypedefInfo {
            name: "dev_t".to_string(),
            underlying: "struct device *".to_string(),
        },
    );

    // Build a CPG with type enrichment
    let source = r#"
struct device {
    char *name;
    int id;
};

void init(struct device *dev) {
    dev->name = "test";
    dev->id = 42;
}
"#;
    let path = "test.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let cpg = CodePropertyGraph::build_with_types(&files, type_db);

    // CPG should have type info
    assert!(cpg.has_type_info());
    assert_eq!(
        cpg.all_fields_of("device"),
        Some(vec!["name".to_string(), "id".to_string()])
    );
    assert_eq!(cpg.resolve_type("dev_t"), "struct device *");
    assert_eq!(cpg.field_type("device", "name"), Some("char *".to_string()));
}

#[test]
fn test_needs_cpg_classification() {
    // Verify that AST-only algorithms don't need CPG
    let ast_only = vec![
        SlicingAlgorithm::OriginalDiff,
        SlicingAlgorithm::ParentFunction,
        SlicingAlgorithm::LeftFlow,
        SlicingAlgorithm::FullFlow,
        SlicingAlgorithm::ThinSlice,
        SlicingAlgorithm::RelevantSlice,
        SlicingAlgorithm::ConditionedSlice,
        SlicingAlgorithm::QuantumSlice,
        SlicingAlgorithm::HorizontalSlice,
        SlicingAlgorithm::AngleSlice,
        SlicingAlgorithm::AbsenceSlice,
        SlicingAlgorithm::ResonanceSlice,
        SlicingAlgorithm::SymmetrySlice,
        SlicingAlgorithm::PhantomSlice,
    ];
    for algo in &ast_only {
        assert!(!algo.needs_cpg(), "{:?} should not need CPG", algo);
    }

    // Verify that CPG-consuming algorithms do need CPG
    let cpg_algos = vec![
        SlicingAlgorithm::BarrierSlice,
        SlicingAlgorithm::Chop,
        SlicingAlgorithm::Taint,
        SlicingAlgorithm::DeltaSlice,
        SlicingAlgorithm::SpiralSlice,
        SlicingAlgorithm::CircularSlice,
        SlicingAlgorithm::VerticalSlice,
        SlicingAlgorithm::ThreeDSlice,
        SlicingAlgorithm::GradientSlice,
        SlicingAlgorithm::ProvenanceSlice,
        SlicingAlgorithm::MembraneSlice,
        SlicingAlgorithm::EchoSlice,
    ];
    for algo in &cpg_algos {
        assert!(algo.needs_cpg(), "{:?} should need CPG", algo);
    }
}
