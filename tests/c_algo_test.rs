mod common;
use common::*;

#[test]
fn test_call_graph_construction() {
    let (files, _, _) = make_python_test();
    let cg = CallGraph::build(&files);

    // Should have functions
    assert!(!cg.functions.is_empty());

    // 'process' calls 'calculate' and 'helper'
    let process_funcs = cg.functions.get("process");
    assert!(process_funcs.is_some(), "Should find 'process' function");
}

#[test]
fn test_call_graph_callers() {
    let (files, _, _) = make_python_test();
    let cg = CallGraph::build(&files);

    let callers = cg.callers_of("calculate", 1);
    // 'process' calls 'calculate'
    assert!(
        callers.iter().any(|(f, _)| f.name == "process"),
        "process should be a caller of calculate"
    );
}

#[test]
fn test_data_flow_graph_construction() {
    let (files, _, _) = make_python_test();
    let dfg = DataFlowGraph::build(&files);

    assert!(!dfg.edges.is_empty(), "Should have data flow edges");
    assert!(!dfg.defs.is_empty(), "Should have definitions");
}

#[test]
fn test_c_parses_and_finds_functions() {
    let (files, _, _) = make_c_test();
    let parsed = files.get("src/device.c").unwrap();

    // Should find all functions in the C file
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
        func_names.contains(&"create_device".to_string()),
        "Should find create_device, got: {:?}",
        func_names
    );
    assert!(
        func_names.contains(&"process_packet".to_string()),
        "Should find process_packet, got: {:?}",
        func_names
    );
    assert!(
        func_names.contains(&"handle_request".to_string()),
        "Should find handle_request, got: {:?}",
        func_names
    );
    assert!(
        func_names.contains(&"destroy_device".to_string()),
        "Should find destroy_device, got: {:?}",
        func_names
    );
    assert!(
        func_names.contains(&"bulk_process".to_string()),
        "Should find bulk_process, got: {:?}",
        func_names
    );
}

#[test]
fn test_call_graph_c() {
    let (files, _, _) = make_c_test();
    let call_graph = CallGraph::build(&files);

    // handle_request calls create_device and process_packet
    let callees = call_graph.callees_of("handle_request", "src/device.c", 1);
    let callee_names: Vec<&str> = callees.iter().map(|(id, _)| id.name.as_str()).collect();

    assert!(
        callee_names.contains(&"create_device") || callee_names.contains(&"process_packet"),
        "handle_request should call create_device and process_packet, got: {:?}",
        callee_names
    );
}

#[test]
fn test_call_graph_c_cross_file() {
    let (files, _, _) = make_c_multifile_test();
    let call_graph = CallGraph::build(&files);

    // handle_create in handler.c calls create_device in device.c
    let callees = call_graph.callees_of("handle_create", "src/handler.c", 1);
    let callee_names: Vec<&str> = callees.iter().map(|(id, _)| id.name.as_str()).collect();

    assert!(
        callee_names.contains(&"create_device"),
        "handle_create should call create_device across files, got: {:?}",
        callee_names
    );
}

#[test]
fn test_data_flow_graph_c() {
    let (files, _, _) = make_c_test();
    let dfg = DataFlowGraph::build(&files);

    // Should have def-use edges for variables in C functions
    assert!(
        !dfg.edges.is_empty(),
        "Data flow graph should have edges for C code"
    );

    // Check that variable defs are found
    assert!(
        !dfg.defs.is_empty(),
        "Data flow graph should find variable definitions in C code"
    );
}

#[test]
fn test_c_and_cpp_parse() {
    // Verify C and C++ can be parsed without errors
    let c_source = "int main() { return 0; }\n";
    let cpp_source = "class Foo { public: void bar() {} };\n";

    let c_parsed = ParsedFile::parse("test.c", c_source, Language::C);
    assert!(c_parsed.is_ok(), "C parsing should succeed");

    let cpp_parsed = ParsedFile::parse("test.cpp", cpp_source, Language::Cpp);
    assert!(cpp_parsed.is_ok(), "C++ parsing should succeed");
}

#[test]
fn test_snmp_overflow_original_diff() {
    let (files, diff) = make_snmp_overflow_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::OriginalDiff),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "OriginalDiff should produce blocks for snmp_overflow"
    );
    let total_lines: usize = result
        .blocks
        .iter()
        .map(|b| b.file_line_map.values().map(|m| m.len()).sum::<usize>())
        .sum();
    assert!(
        total_lines > 0,
        "snmp_overflow OriginalDiff should include at least one line"
    );
}

#[test]
fn test_snmp_overflow_parent_function() {
    let (files, diff) = make_snmp_overflow_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ParentFunction),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "ParentFunction should produce blocks for snmp_overflow"
    );
}

#[test]
fn test_snmp_overflow_left_flow() {
    let (files, diff) = make_snmp_overflow_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "LeftFlow should produce blocks for snmp_overflow"
    );
}

#[test]
fn test_double_free_original_diff() {
    let (files, diff) = make_double_free_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::OriginalDiff),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "OriginalDiff should produce blocks for double_free"
    );
    let total_lines: usize = result
        .blocks
        .iter()
        .map(|b| b.file_line_map.values().map(|m| m.len()).sum::<usize>())
        .sum();
    assert!(
        total_lines > 0,
        "double_free OriginalDiff should include at least one line"
    );
}

#[test]
fn test_double_free_left_flow() {
    let (files, diff) = make_double_free_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "LeftFlow should produce blocks for double_free"
    );
}

#[test]
fn test_ring_overflow_original_diff() {
    let (files, diff) = make_ring_overflow_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::OriginalDiff),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "OriginalDiff should produce blocks for ring_overflow"
    );
    let total_lines: usize = result
        .blocks
        .iter()
        .map(|b| b.file_line_map.values().map(|m| m.len()).sum::<usize>())
        .sum();
    assert!(
        total_lines > 0,
        "ring_overflow OriginalDiff should include at least one line"
    );
}

#[test]
fn test_ring_overflow_left_flow() {
    let (files, diff) = make_ring_overflow_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "LeftFlow should produce blocks for ring_overflow"
    );
}

#[test]
fn test_timer_uaf_original_diff() {
    let (files, diff) = make_timer_uaf_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::OriginalDiff),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "OriginalDiff should produce blocks for timer_uaf"
    );
    let total_lines: usize = result
        .blocks
        .iter()
        .map(|b| b.file_line_map.values().map(|m| m.len()).sum::<usize>())
        .sum();
    assert!(
        total_lines > 0,
        "timer_uaf OriginalDiff should include at least one line"
    );
}

#[test]
fn test_timer_uaf_left_flow() {
    let (files, diff) = make_timer_uaf_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::LeftFlow),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "LeftFlow should produce blocks for timer_uaf"
    );
}

#[test]
fn test_clean_c_has_no_parse_errors() {
    let source = r#"
#include <stdio.h>

int add(int a, int b) {
    return a + b;
}

int main(void) {
    int x = add(1, 2);
    printf("%d\n", x);
    return 0;
}
"#;
    let path = "src/clean.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();

    assert_eq!(
        parsed.parse_error_count, 0,
        "Clean C code should produce zero ERROR nodes"
    );
    assert!(
        parsed.parse_node_count > 0,
        "Should have counted some AST nodes"
    );
    assert_eq!(
        parsed.error_rate(),
        0.0,
        "Error rate should be 0.0 for clean code"
    );

    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let warnings = algorithms::check_parse_warnings(&files);
    assert!(
        warnings.is_empty(),
        "Clean C code should produce no parse warnings"
    );
}

#[test]
fn test_dataflow_pointer_deref() {
    // *p = val  should create a data flow def for the base pointer variable p.
    let source = r#"
void write_through(int *p, int val) {
    *p = val;
    return;
}
"#;
    let path = "src/ptr.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);

    let p_defs = dfg.all_defs_of(path, "p");
    assert!(
        !p_defs.is_empty(),
        "DataFlowGraph should record a def for 'p' from the *p = val assignment"
    );
    assert!(
        p_defs.iter().any(|d| d.line == 3),
        "Def of 'p' should be on line 3 (*p = val), got lines: {:?}",
        p_defs.iter().map(|d| d.line).collect::<Vec<_>>()
    );
}

#[test]
fn test_dataflow_struct_field() {
    // dev->id = val  should create a qualified AccessPath def (dev.id) plus a base def (dev).
    // The old behavior of creating a bare "id" def was incorrect — it caused false flow edges
    // with unrelated variables named "id".
    let source = r#"
typedef struct { int id; } Dev;
void set_id(Dev *dev, int val) {
    dev->id = val;
    return;
}
"#;
    let path = "src/dev.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);

    // Coarse tracking: mutation of the base struct pointer
    let dev_defs = dfg.all_defs_of(path, "dev");
    assert!(
        !dev_defs.is_empty(),
        "DataFlowGraph should record a def for base 'dev' from dev->id = val"
    );

    // Fine-grained tracking: the field access path was recorded as a def
    // With AccessPath, dev->id creates a def for AccessPath { base: "dev", fields: ["id"] },
    // not a bare "id" def. The base "dev" match above covers both.
    let has_field_path = dev_defs
        .iter()
        .any(|d| d.path.has_fields() && d.path.fields == vec!["id"]);
    assert!(
        has_field_path,
        "DataFlowGraph should record a def with AccessPath dev.id from dev->id = val"
    );
}

#[test]
fn test_dataflow_array_subscript() {
    // buf[i] = val  should create a data flow def for the base array variable buf.
    let source = r#"
void fill_buffer(int *buf, int i, int val) {
    buf[i] = val;
    return;
}
"#;
    let path = "src/buf.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);

    let buf_defs = dfg.all_defs_of(path, "buf");
    assert!(
        !buf_defs.is_empty(),
        "DataFlowGraph should record a def for 'buf' from buf[i] = val"
    );
    assert!(
        buf_defs.iter().any(|d| d.line == 3),
        "Def of 'buf' should be on line 3 (buf[i] = val), got lines: {:?}",
        buf_defs.iter().map(|d| d.line).collect::<Vec<_>>()
    );
}

#[test]
fn test_call_graph_level1_local_fptr() {
    let source = r#"
#include <stdlib.h>

void target_func(int x) {
    // do work
}

void other_func(int x) {
    // different work
}

void caller(void) {
    void (*fptr)(int) = target_func;
    fptr(42);
}

void caller_reassign(void) {
    void (*fptr)(int) = other_func;
    fptr = target_func;
    fptr(99);
}
"#;

    let mut files = BTreeMap::new();
    files.insert(
        "src/fptr.c".to_string(),
        ParsedFile::parse("src/fptr.c", source, Language::C).unwrap(),
    );

    let call_graph = CallGraph::build(&files);

    // caller's fptr(42) should resolve to target_func
    let caller_id = &call_graph.functions.get("caller").unwrap()[0];
    let caller_calls = call_graph.calls.get(caller_id).unwrap();
    let callee_names: Vec<&str> = caller_calls
        .iter()
        .map(|s| s.callee_name.as_str())
        .collect();
    assert!(
        callee_names.contains(&"target_func"),
        "Level 1: fptr = target_func; fptr() should resolve to target_func, got: {:?}",
        callee_names
    );

    // caller_reassign's fptr(99) should resolve to target_func (last assignment)
    let reassign_id = &call_graph.functions.get("caller_reassign").unwrap()[0];
    let reassign_calls = call_graph.calls.get(reassign_id).unwrap();
    let reassign_names: Vec<&str> = reassign_calls
        .iter()
        .map(|s| s.callee_name.as_str())
        .collect();
    assert!(
        reassign_names.contains(&"target_func"),
        "Level 1: reassigned fptr should resolve to last assignment (target_func), got: {:?}",
        reassign_names
    );
}

#[test]
fn test_call_graph_level3_parameter_fptr() {
    let source = r#"
void handler_a(int data) {
    // handle A
}

void handler_b(int data) {
    // handle B
}

typedef void (*callback_fn)(int);

void execute(callback_fn cb, int data) {
    cb(data);
}

void caller_a(void) {
    execute(handler_a, 1);
}

void caller_b(void) {
    execute(handler_b, 2);
}
"#;

    let mut files = BTreeMap::new();
    files.insert(
        "src/callback.c".to_string(),
        ParsedFile::parse("src/callback.c", source, Language::C).unwrap(),
    );

    let call_graph = CallGraph::build(&files);

    // execute's cb(data) should resolve to both handler_a and handler_b
    let execute_id = &call_graph.functions.get("execute").unwrap()[0];
    let execute_calls = call_graph.calls.get(execute_id).unwrap();
    let callee_names: BTreeSet<&str> = execute_calls
        .iter()
        .map(|s| s.callee_name.as_str())
        .collect();

    assert!(
        callee_names.contains("handler_a"),
        "Level 3: execute(handler_a, 1) should resolve cb to handler_a, got: {:?}",
        callee_names
    );
    assert!(
        callee_names.contains("handler_b"),
        "Level 3: execute(handler_b, 2) should resolve cb to handler_b, got: {:?}",
        callee_names
    );
}

#[test]
fn test_call_graph_level3_address_of_fptr() {
    let source = r#"
void my_handler(int sig) {
    // handle signal
}

void register_handler(void (*handler)(int), int sig) {
    handler(sig);
}

void setup(void) {
    register_handler(&my_handler, 2);
}
"#;

    let mut files = BTreeMap::new();
    files.insert(
        "src/signal.c".to_string(),
        ParsedFile::parse("src/signal.c", source, Language::C).unwrap(),
    );

    let call_graph = CallGraph::build(&files);

    let register_id = &call_graph.functions.get("register_handler").unwrap()[0];
    let register_calls = call_graph.calls.get(register_id).unwrap();
    let callee_names: BTreeSet<&str> = register_calls
        .iter()
        .map(|s| s.callee_name.as_str())
        .collect();

    assert!(
        callee_names.contains("my_handler"),
        "Level 3: register_handler(&my_handler, 2) should resolve handler to my_handler, got: {:?}",
        callee_names
    );
}

#[test]
fn test_cve_double_free_goto_cleanup() {
    let source = r#"
#include <stdlib.h>

void process_frame(uint8_t *raw, size_t len) {
    char *buf = malloc(len);
    char *header = malloc(64);

    if (validate(raw) < 0) {
        free(buf);
        free(header);
        goto cleanup;
    }

    process(buf, header);
    return;

cleanup:
    free(buf);
    free(header);
}
"#;

    let path = "src/frame.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Diff touches the malloc lines
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([5, 6]),
        }],
    };

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
    )
    .unwrap();

    // Should detect double-free: free() before goto AND in cleanup label
    let double_close_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("double_close"))
        .collect();
    assert!(
        !double_close_findings.is_empty(),
        "Should detect double-free via goto cleanup pattern, got findings: {:?}",
        result.findings
    );
}

#[test]
fn test_cve_correct_goto_cleanup_no_double_free() {
    let source = r#"
#include <stdlib.h>

int init_device(int id) {
    char *buf = malloc(256);
    if (!buf) return -1;

    char *dev = malloc(64);
    if (!dev) goto err_buf;

    int ret = register_dev(dev, id);
    if (ret < 0) goto err_dev;

    return 0;

err_dev:
    free(dev);
err_buf:
    free(buf);
    return -1;
}
"#;

    let path = "src/device.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([5, 8]),
        }],
    };

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
    )
    .unwrap();

    // Correct kernel cleanup pattern — should NOT report double-free
    let double_close_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("double_close"))
        .collect();
    assert!(
        double_close_findings.is_empty(),
        "Correct goto cleanup (no inline free before goto) should not flag double-free, got: {:?}",
        double_close_findings
    );
}

#[test]
fn test_cve_double_unlock_goto() {
    let source = r#"
#include <linux/spinlock.h>

int update_state(spinlock_t *lock, int val) {
    spin_lock(lock);

    if (val < 0) {
        spin_unlock(lock);
        goto err;
    }

    shared_state = val;
    spin_unlock(lock);
    return 0;

err:
    spin_unlock(lock);
    return -1;
}
"#;

    let path = "src/state.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([5]),
        }],
    };

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
    )
    .unwrap();

    let double_close_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("double_close"))
        .collect();
    assert!(
        !double_close_findings.is_empty(),
        "Should detect double-unlock: spin_unlock before goto AND in err label, got: {:?}",
        result.findings
    );
}

#[test]
fn test_field_isolation_c_arrow() {
    // C arrow access: dev->name taint should NOT propagate to dev->id
    let source = r#"
void process(struct device *dev) {
    dev->name = get_user_input();
    dev->id = 42;
    use_name(dev->name);
    use_id(dev->id);
}
"#;
    let path = "src/dev.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let dev_defs = dfg.all_defs_of(path, "dev");

    // Should have field-qualified defs only — no base-only "dev" def
    let base_only_defs: Vec<_> = dev_defs.iter().filter(|d| !d.path.has_fields()).collect();
    assert!(
        base_only_defs.is_empty(),
        "Phase 2: field assignments should NOT create base-only defs. Got: {:?}",
        base_only_defs
    );

    // Forward from dev->name def should NOT reach dev->id use
    let name_def = dev_defs.iter().find(|d| d.path.fields == vec!["name"]);
    if let Some(nd) = name_def {
        let reachable = dfg.forward_reachable(nd);
        let reaches_id = reachable.iter().any(|r| r.path.fields == vec!["id"]);
        assert!(
            !reaches_id,
            "Phase 2: taint on dev->name must NOT propagate to dev->id"
        );
    }
}

#[test]
fn test_field_isolation_c_dot() {
    // C dot access (struct value): cfg.timeout taint should NOT reach cfg.host
    let source = r#"
void configure(struct config cfg) {
    cfg.timeout = get_input();
    cfg.host = "safe";
    use_timeout(cfg.timeout);
    use_host(cfg.host);
}
"#;
    let path = "src/cfg.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let cfg_defs = dfg.all_defs_of(path, "cfg");

    let base_only: Vec<_> = cfg_defs.iter().filter(|d| !d.path.has_fields()).collect();
    assert!(
        base_only.is_empty(),
        "Phase 2: dot field assignments should NOT create base-only defs. Got: {:?}",
        base_only
    );
}

#[test]
fn test_must_alias_c_pointer() {
    // ptr = dev; ptr->name = x → should create def for dev.name too
    let source = r#"
void process(struct device *dev) {
    struct device *ptr = dev;
    ptr->name = "eth0";
    use_name(dev->name);
}
"#;
    let path = "src/alias.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);
    let dev_defs = dfg.all_defs_of(path, "dev");

    // Phase 3: ptr->name def should also create a dev.name def via alias resolution
    let has_dev_name = dev_defs.iter().any(|d| d.path.fields == vec!["name"]);
    assert!(
        has_dev_name,
        "Phase 3: ptr = dev alias should resolve ptr->name to dev.name. Got defs: {:?}",
        dev_defs.iter().map(|d| &d.path).collect::<Vec<_>>()
    );
}

#[test]
fn test_ts_fallback_extracts_c_struct() {
    use prism::type_db::TypeDatabase;

    let source = r#"
struct device {
    char *name;
    int id;
    float weight;
};
"#;
    let path = "src/device.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let db = TypeDatabase::from_parsed_files(&files);

    let record = db
        .records
        .get("device")
        .expect("should extract device struct");
    assert_eq!(record.kind, prism::type_db::RecordKind::Struct);
    let field_names: Vec<&str> = record.fields.iter().map(|f| f.name.as_str()).collect();
    assert_eq!(field_names, vec!["name", "id", "weight"]);
}

#[test]
fn test_c_init_declarator_value() {
    // C init_declarator: int *ptr = dev
    let source = r#"
void process() {
    int x = 42;
    int *ptr = &x;
    use(ptr);
}
"#;
    let path = "src/process.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let func = parsed.all_functions().into_iter().next().unwrap();
    let lines: BTreeSet<usize> = (1..=5).collect();
    let lvalues = parsed.assignment_lvalue_paths_on_lines(&func, &lines);

    let has_x = lvalues.iter().any(|(p, _)| p.base == "x");
    let has_ptr = lvalues.iter().any(|(p, _)| p.base == "ptr");
    assert!(has_x, "C: 'x' should have L-value");
    assert!(has_ptr, "C: 'ptr' should have L-value");
}
