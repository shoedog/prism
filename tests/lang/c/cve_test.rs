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

// --- Goto-based error path tracking tests ---

#[test]
fn test_close_only_on_error_path() {
    let source = r#"
int init(void) {
    char *buf = kmalloc(256);
    if (!buf) return -1;
    int ret = process(buf);
    if (ret < 0) goto err;
    return 0;
err:
    kfree(buf);
    return -1;
}
"#;

    let path = "src/init.c";
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
    )
    .unwrap();

    let error_path_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("close_only_on_error_path"))
        .collect();
    assert!(
        !error_path_findings.is_empty(),
        "Should detect close only on error path for kmalloc, got findings: {:?}",
        result.findings
    );
    assert_eq!(
        error_path_findings[0].severity, "info",
        "close_only_on_error_path should be info severity"
    );
}

#[test]
fn test_cascading_goto_fall_through() {
    let source = r#"
int init(void) {
    char *buf = kmalloc(256);
    if (!buf) return -1;
    char *dev = kmalloc(64);
    if (!dev) goto err_buf;
    int ret = register_dev(dev);
    if (ret < 0) goto err_dev;
    return 0;
err_dev:
    kfree(dev);
err_buf:
    kfree(buf);
    return -1;
}
"#;

    let path = "src/cascade.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3, 5]),
        }],
    };

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
    )
    .unwrap();

    // Both kmalloc lines should have close_only_on_error_path findings
    let error_path_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("close_only_on_error_path"))
        .collect();
    assert!(
        !error_path_findings.is_empty(),
        "Should detect close_only_on_error_path for cascading gotos, got: {:?}",
        result.findings
    );

    // No missing_close_on_error_path: goto err_dev reaches kfree(dev) directly
    // and kfree(buf) via fall-through; goto err_buf reaches kfree(buf) directly
    let missing_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("missing_close_on_error_path"))
        .collect();
    assert!(
        missing_findings.is_empty(),
        "Correct cascading cleanup should not flag missing_close_on_error_path, got: {:?}",
        missing_findings
    );
}

#[test]
fn test_missing_close_on_error_path() {
    // dev is allocated successfully, then a DIFFERENT error triggers goto err
    // which only frees buf but NOT dev — a real resource leak.
    let source = r#"
int init(void) {
    char *buf = kmalloc(256);
    if (!buf) return -1;
    char *dev = kmalloc(64);
    if (!dev) goto err;
    int ret = register_dev(dev);
    if (ret < 0) goto err;
    return 0;
err:
    kfree(buf);
    return -1;
}
"#;

    let path = "src/missing.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Diff touches line 5 (dev = kmalloc). The goto at line 8 (ret < 0)
    // goes to err which only frees buf, not dev.
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

    let missing_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("missing_close_on_error_path"))
        .collect();
    assert!(
        !missing_findings.is_empty(),
        "Should detect dev not freed on 'goto err' path (ret < 0), got: {:?}",
        result.findings
    );
    assert_eq!(
        missing_findings[0].severity, "warning",
        "missing_close_on_error_path should be warning severity"
    );
}

#[test]
fn test_correct_goto_no_close_only_finding() {
    let source = r#"
int init(void) {
    char *buf = kmalloc(256);
    if (!buf) return -1;
    char *dev = kmalloc(64);
    if (!dev) goto err_buf;
    use(buf, dev);
    kfree(dev);
    kfree(buf);
    return 0;
err_buf:
    kfree(buf);
    return -1;
}
"#;

    let path = "src/correct.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([3, 5]),
        }],
    };

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
    )
    .unwrap();

    let error_only_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("close_only_on_error_path"))
        .collect();
    assert!(
        error_only_findings.is_empty(),
        "Both buf and dev have close on normal path — no close_only_on_error_path expected, got: {:?}",
        error_only_findings
    );
}

#[test]
fn test_no_goto_functions_unchanged() {
    let source = r#"
void process(void) {
    char *buf = malloc(256);
    use(buf);
}
"#;

    let path = "src/nogoto.c";
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
    )
    .unwrap();

    let missing_counterpart: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("missing_counterpart"))
        .collect();
    assert!(
        !missing_counterpart.is_empty(),
        "Non-goto function missing free should still fire missing_counterpart, got: {:?}",
        result.findings
    );
}

#[test]
fn test_cascading_double_free_fall_through() {
    let source = r#"
int init(void) {
    char *buf = kmalloc(256);
    if (error) {
        kfree(buf);
        goto err_dev;
    }
    return 0;
err_dev:
    kfree(dev);
err_buf:
    kfree(buf);
    return -1;
}
"#;

    let path = "src/double_cascade.c";
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
    )
    .unwrap();

    let double_close: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("double_close"))
        .collect();
    assert!(
        !double_close.is_empty(),
        "Should detect double-free: kfree(buf) inline + kfree(buf) reachable via fall-through, got: {:?}",
        result.findings
    );
}

#[test]
fn test_variable_identity_different_resources() {
    let source = r#"
int init(void) {
    char *buf = kmalloc(256);
    char *dev = kmalloc(64);
    if (!dev) goto err;
    kfree(dev);
    return 0;
err:
    kfree(buf);
    return -1;
}
"#;

    let path = "src/varident.c";
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
    )
    .unwrap();

    // kfree(dev) on normal path should NOT satisfy close for buf
    let error_only: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("close_only_on_error_path") && f.line == 3)
        .collect();
    assert!(
        !error_only.is_empty(),
        "kfree(dev) should NOT satisfy close for buf — should get close_only_on_error_path, got: {:?}",
        result.findings
    );
}

#[test]
fn test_error_path_wrong_variable_not_suppressed() {
    // kfree(dev) on the error path must NOT suppress missing_counterpart for buf.
    // Before the fix, has_close_error matched on function name only (kfree),
    // so kfree(dev) satisfied the close check for buf, hiding the leak.
    let source = r#"
int init(void) {
    char *buf = kmalloc(256);
    char *dev = kmalloc(64);
    if (!dev) goto err;
    use(buf, dev);
    kfree(dev);
    return 0;
err:
    kfree(dev);
    return -1;
}
"#;

    let path = "src/wrongvar.c";
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
    )
    .unwrap();

    // kfree(dev) on error path should NOT count as close for buf.
    // buf has no close anywhere → missing_counterpart should fire.
    let missing: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("missing_counterpart") && f.line == 3)
        .collect();
    assert!(
        !missing.is_empty(),
        "kfree(dev) on error path must not suppress missing_counterpart for buf, got: {:?}",
        result.findings
    );
}

#[test]
fn test_mid_function_label_fall_through() {
    let source = r#"
void f(void) {
    char *buf = malloc(256);
retry:
    int r = process(buf);
    if (r == RETRY) goto retry;
    if (r < 0) goto err;
    free(buf);
    return;
err:
    free(buf);
    return;
}
"#;

    let path = "src/midlabel.c";
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
    )
    .unwrap();

    // retry: is fall-through (no preceding flow terminator), so free(buf) after
    // retry is on the normal path. Should NOT get close_only_on_error_path.
    let error_only: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("close_only_on_error_path"))
        .collect();
    assert!(
        error_only.is_empty(),
        "retry: is fall-through — free(buf) is on normal path, no close_only_on_error_path expected, got: {:?}",
        error_only
    );
}

#[test]
fn test_backward_goto_not_analyzed() {
    let source = r#"
void f(void) {
retry:
    char *buf = malloc(256);
    if (check(buf) < 0) {
        free(buf);
        goto retry;
    }
    use(buf);
    free(buf);
}
"#;

    let path = "src/backward.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::AbsenceSlice),
        None,
    )
    .unwrap();

    let missing_on_error: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("missing_close_on_error_path"))
        .collect();
    assert!(
        missing_on_error.is_empty(),
        "Backward goto retry should not trigger missing_close_on_error_path, got: {:?}",
        missing_on_error
    );
}
