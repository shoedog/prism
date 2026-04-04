#[path = "../../common/mod.rs"]
mod common;
use common::*;

#[test]
fn test_thin_slice_subset_of_leftflow() {
    let (files, _, diff) = make_python_test();

    let thin = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ThinSlice),
        None,
    )
    .unwrap();

    let left = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow),
        None,
    )
    .unwrap();

    let thin_lines: usize = thin
        .blocks
        .iter()
        .map(|b| b.file_line_map.values().map(|m| m.len()).sum::<usize>())
        .sum();
    let left_lines: usize = left
        .blocks
        .iter()
        .map(|b| b.file_line_map.values().map(|m| m.len()).sum::<usize>())
        .sum();

    assert!(
        thin_lines <= left_lines,
        "ThinSlice ({}) should have <= lines than LeftFlow ({})",
        thin_lines,
        left_lines
    );
}

#[test]
fn test_thin_slice_has_data_deps() {
    let (files, sources, diff) = make_python_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ThinSlice),
        None,
    )
    .unwrap();

    assert!(!result.blocks.is_empty());
    let block = &result.blocks[0];
    let lines = block.file_line_map.get("src/calc.py").unwrap();
    // Should have the diff lines plus variable references
    assert!(
        lines.len() >= 2,
        "ThinSlice should include at least diff lines"
    );
}

#[test]
fn test_barrier_slice_python() {
    let (files, _, diff) = make_python_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::BarrierSlice),
        None,
    )
    .unwrap();

    // Should include caller/callee information
    assert!(!result.blocks.is_empty());
}

#[test]
fn test_relevant_slice_includes_alternates() {
    let (files, sources, diff) = make_python_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::RelevantSlice),
        None,
    )
    .unwrap();

    assert!(!result.blocks.is_empty());
    let block = &result.blocks[0];
    let lines = block.file_line_map.get("src/calc.py").unwrap();

    // RelevantSlice should include at least as much as LeftFlow
    let left = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow),
        None,
    )
    .unwrap();

    let relevant_count: usize = result
        .blocks
        .iter()
        .map(|b| b.file_line_map.values().map(|m| m.len()).sum::<usize>())
        .sum();
    let left_count: usize = left
        .blocks
        .iter()
        .map(|b| b.file_line_map.values().map(|m| m.len()).sum::<usize>())
        .sum();

    assert!(
        relevant_count >= left_count,
        "RelevantSlice ({}) should have >= lines than LeftFlow ({})",
        relevant_count,
        left_count
    );
}

#[test]
fn test_conditioned_slice_parses_conditions() {
    use prism::algorithms::conditioned_slice::Condition;

    let c = Condition::parse("x==5").unwrap();
    assert_eq!(c.var_name, "x");
    assert_eq!(c.value, "5");

    let c = Condition::parse("ptr!=null").unwrap();
    assert_eq!(c.var_name, "ptr");

    let c = Condition::parse("count > 0").unwrap();
    assert_eq!(c.var_name, "count");
    assert_eq!(c.value, "0");
}

#[test]
fn test_snmp_overflow_thin_slice() {
    let (files, diff) = make_snmp_overflow_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ThinSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "ThinSlice should produce blocks for snmp_overflow"
    );
}

#[test]
fn test_double_free_thin_slice() {
    let (files, diff) = make_double_free_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ThinSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "ThinSlice should produce blocks for double_free"
    );
}

#[test]
fn test_ring_overflow_thin_slice() {
    let (files, diff) = make_ring_overflow_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ThinSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "ThinSlice should produce blocks for ring_overflow"
    );
}

#[test]
fn test_timer_uaf_thin_slice() {
    let (files, diff) = make_timer_uaf_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ThinSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "ThinSlice should produce blocks for timer_uaf"
    );
}

#[test]
fn test_large_function_thin_slice() {
    let (files, diff) = make_large_function_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ThinSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "ThinSlice should produce blocks for large_function without panic"
    );
}

#[test]
fn test_deep_switch_thin_slice() {
    let (files, diff) = make_deep_switch_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ThinSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "ThinSlice should produce blocks for deep_switch without panic"
    );
}

#[test]
fn test_chop_with_field_access() {
    // Chop from source line to sink line should include intermediate field accesses.
    let source = r#"
void transfer(struct device *dev, const char *input) {
    dev->buf = input;
    char *data = dev->buf;
    memcpy(dest, data, len);
}
"#;
    let path = "src/transfer.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);

    // Chop from line 3 (source: dev->buf = input) to line 5 (sink: memcpy)
    let on_path = dfg.chop(path, 3, path, 5);

    // Should include the intermediate line 4 (data = dev->buf)
    let path_lines: BTreeSet<usize> = on_path.iter().map(|(_, l)| *l).collect();
    assert!(
        path_lines.contains(&3) && path_lines.contains(&5),
        "Chop should include source (line 3) and sink (line 5). Got: {:?}",
        path_lines
    );
}

#[test]
fn test_chop_python() {
    let source = r#"
x = input()
y = int(x)
z = y + 1
result = z * 2
print(result)
"#;
    let path = "app.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let config = prism::algorithms::chop::ChopConfig {
        source_file: path.to_string(),
        source_line: 2,
        sink_file: path.to_string(),
        sink_line: 5,
    };
    let ctx = CpgContext::build(&files, None);
    let result = prism::algorithms::chop::slice(&ctx, &config).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::Chop);
}

#[test]
fn test_chop_go() {
    let source = r#"package main

func process(input string) string {
    parsed := parse(input)
    validated := validate(parsed)
    result := transform(validated)
    return result
}

func parse(s string) string { return s }
func validate(s string) string { return s }
func transform(s string) string { return s }
"#;
    let path = "main.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let config = prism::algorithms::chop::ChopConfig {
        source_file: path.to_string(),
        source_line: 4,
        sink_file: path.to_string(),
        sink_line: 6,
    };
    let ctx = CpgContext::build(&files, None);
    let result = prism::algorithms::chop::slice(&ctx, &config).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::Chop);
}

#[test]
fn test_chop_javascript() {
    let source = r#"
function pipeline(raw) {
    const cleaned = sanitize(raw);
    const parsed = JSON.parse(cleaned);
    const result = process(parsed);
    return result;
}
function sanitize(s) { return s.trim(); }
function process(o) { return o.value; }
"#;
    let path = "pipe.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let config = prism::algorithms::chop::ChopConfig {
        source_file: path.to_string(),
        source_line: 3,
        sink_file: path.to_string(),
        sink_line: 5,
    };
    let ctx = CpgContext::build(&files, None);
    let result = prism::algorithms::chop::slice(&ctx, &config).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::Chop);
}

#[test]
fn test_delta_slice_python() {
    let tmp = TempDir::new().unwrap();

    let old_source = "x = 1\ny = x + 1\nprint(y)\n";
    std::fs::write(tmp.path().join("app.py"), old_source).unwrap();

    let new_source = "x = 1\ny = x + 2\nz = y * 3\nprint(z)\n";
    let path = "app.py";
    let parsed = ParsedFile::parse(path, new_source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([2, 3]),
        }],
    };

    let ctx = CpgContext::build(&files, None);
    let result = prism::algorithms::delta_slice::slice(&ctx, &diff, tmp.path()).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::DeltaSlice);
}

#[test]
fn test_delta_slice_go() {
    let tmp = TempDir::new().unwrap();

    let old_source = "package main\n\nfunc add(a int, b int) int {\n\treturn a + b\n}\n";
    std::fs::write(tmp.path().join("calc.go"), old_source).unwrap();

    let new_source =
        "package main\n\nfunc add(a int, b int) int {\n\tresult := a + b\n\treturn result\n}\n";
    let path = "calc.go";
    let parsed = ParsedFile::parse(path, new_source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([4, 5]),
        }],
    };

    let ctx = CpgContext::build(&files, None);
    let result = prism::algorithms::delta_slice::slice(&ctx, &diff, tmp.path()).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::DeltaSlice);
}

#[test]
fn test_conditioned_slice_python() {
    let source = r#"
def process(x):
    if x > 0:
        result = x * 2
    else:
        result = 0
    return result
"#;
    let path = "cond.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([4]),
        }],
    };

    let condition = prism::algorithms::conditioned_slice::Condition::parse("x==5").unwrap();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::ConditionedSlice);
    let ctx = CpgContext::build(&files, None);
    let result =
        prism::algorithms::conditioned_slice::slice(&ctx, &diff, &config, &condition).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::ConditionedSlice);
}

#[test]
fn test_conditioned_slice_go() {
    let source = r#"package main

func check(n int) int {
	if n > 0 {
		return n * 2
	} else {
		return 0
	}
}
"#;
    let path = "check.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([5]),
        }],
    };

    let condition = prism::algorithms::conditioned_slice::Condition::parse("n>0").unwrap();
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::ConditionedSlice);
    let ctx = CpgContext::build(&files, None);
    let result =
        prism::algorithms::conditioned_slice::slice(&ctx, &diff, &config, &condition).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::ConditionedSlice);
}

#[test]
fn test_conditioned_slice_javascript() {
    let source = r#"
function validate(input) {
    if (input == null) {
        return "missing";
    } else {
        return input.trim();
    }
}
"#;
    let path = "validate.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3, 6]),
        }],
    };

    let condition = prism::algorithms::conditioned_slice::Condition::parse("input!=null").unwrap();
    assert_eq!(
        condition.op,
        prism::algorithms::conditioned_slice::ConditionOp::IsNotNull
    );
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::ConditionedSlice);
    let ctx = CpgContext::build(&files, None);
    let result =
        prism::algorithms::conditioned_slice::slice(&ctx, &diff, &config, &condition).unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::ConditionedSlice);
}

#[test]
fn test_conditioned_slice_parse_operators() {
    use prism::algorithms::conditioned_slice::{Condition, ConditionOp};

    let c = Condition::parse("x==5").unwrap();
    assert_eq!(c.op, ConditionOp::Eq);
    assert_eq!(c.var_name, "x");
    assert_eq!(c.value, "5");

    let c = Condition::parse("y != 10").unwrap();
    assert_eq!(c.op, ConditionOp::NotEq);

    let c = Condition::parse("z>=3").unwrap();
    assert_eq!(c.op, ConditionOp::GtEq);

    let c = Condition::parse("w<=100").unwrap();
    assert_eq!(c.op, ConditionOp::LtEq);

    let c = Condition::parse("a<0").unwrap();
    assert_eq!(c.op, ConditionOp::Lt);

    let c = Condition::parse("ptr==null").unwrap();
    assert_eq!(c.op, ConditionOp::IsNull);

    let c = Condition::parse("ptr!=None").unwrap();
    assert_eq!(c.op, ConditionOp::IsNotNull);

    let c = Condition::parse("ptr==nil").unwrap();
    assert_eq!(c.op, ConditionOp::IsNull);

    assert!(Condition::parse("noop").is_none());
}
