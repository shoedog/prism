#[path = "../../common/mod.rs"]
mod common;
use common::*;

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
