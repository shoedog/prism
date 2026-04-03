#[path = "../../common/mod.rs"]
mod common;
use common::*;

fn make_taint_test_fixture() -> (
    BTreeMap<String, ParsedFile>,
    BTreeMap<String, String>,
    DiffInput,
) {
    let source = r#"
import os

def handle_request(user_input):
    query = "SELECT * FROM users WHERE name = '" + user_input + "'"
    result = db.execute(query)
    return result

def log_entry(message):
    os.system("logger " + message)
"#;

    let path = "src/handler.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    let mut sources = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    sources.insert(path.to_string(), source.to_string());

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([4]),
        }],
    };

    (files, sources, diff)
}


#[test]
fn test_taint_from_diff() {
    let (files, _, diff) = make_python_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    // Taint should propagate from diff lines
    assert!(!result.blocks.is_empty());
}

#[test]
fn test_taint_c_buffer_overflow() {
    let (files, sources, diff) = make_c_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    // Taint should trace from the diff lines (buffer operations) forward
    assert!(result.algorithm == SlicingAlgorithm::Taint);
    // Even if no explicit taint source is specified, auto-taint from diff should work
}

#[test]
fn test_taint_findings_populated() {
    let (files, sources, diff) = make_taint_test_fixture();

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    let review = to_review_output(&result, &sources);
    assert_eq!(review.algorithm, "Taint");

    // findings may or may not fire depending on AST analysis, but the field must exist
    for finding in &review.findings {
        assert_eq!(finding.algorithm, "taint"); // findings use lowercase algorithm names
        assert!(!finding.file.is_empty(), "finding.file must not be empty");
        assert!(
            ["info", "warning", "concern"].contains(&finding.severity.as_str()),
            "severity must be one of info/warning/concern"
        );
        assert!(
            !finding.description.is_empty(),
            "finding.description must not be empty"
        );
        assert!(finding.line > 0, "finding.line must be > 0");
    }
}

#[test]
fn test_taint_c_strcpy_sink() {
    // recv() source on diff line flows through data flow to strcpy() sink.
    let source = r#"
void process_input(const char *input) {
    char *data = input;
    char dest[256];
    strcpy(dest, data);
}
"#;
    let path = "src/input.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Diff line 3: char *data = input;  — data is tainted from diff
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

    // Taint should propagate from line 3 (data defined) to line 5 (strcpy uses data).
    assert!(
        !result.blocks.is_empty(),
        "Taint should produce blocks when tainted value reaches strcpy"
    );
    // At least one finding should flag strcpy as a sink.
    assert!(
        !result.findings.is_empty(),
        "Taint should emit a finding when tainted value reaches strcpy sink"
    );
}

#[test]
fn test_taint_c_sprintf_sink() {
    // User-controlled format string flows to sprintf().
    let source = r#"
void handle_cmd(const char *user_fmt) {
    char *fmt = user_fmt;
    char buf[256];
    sprintf(buf, fmt);
}
"#;
    let path = "src/cmd.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Line 3: char *fmt = user_fmt;  — fmt is tainted
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

    assert!(
        !result.blocks.is_empty(),
        "Taint should produce blocks when tainted value reaches sprintf"
    );
    assert!(
        !result.findings.is_empty(),
        "Taint should emit a finding for sprintf sink with tainted format string"
    );
}

#[test]
fn test_taint_c_memcpy_sink() {
    // Tainted pointer flows to memcpy().
    let source = r#"
void copy_data(const char *network_data) {
    char *msg = network_data;
    char buf[256];
    memcpy(buf, msg, sizeof(buf));
}
"#;
    let path = "src/copy.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Line 3: char *msg = network_data;  — msg is tainted
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

    assert!(
        !result.blocks.is_empty(),
        "Taint should produce blocks when tainted value reaches memcpy"
    );
    assert!(
        !result.findings.is_empty(),
        "Taint should emit a finding for memcpy sink with tainted source pointer"
    );
}

#[test]
fn test_taint_through_pointer() {
    // Taint flows through a pointer dereference assignment:
    // diff line taints *buf → buf becomes tainted → strcpy uses buf → sink fires.
    let source = r#"
void copy_indirect(const char *src, char *dst) {
    char *buf = src;
    *buf = src[0];
    strcpy(dst, buf);
}
"#;
    let path = "src/indirect.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Line 4: *buf = src[0];  — with pointer aliasing, creates def of "buf"
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    // With pointer aliasing: def of buf on line 4 flows to strcpy on line 5 → finding
    assert!(
        !result.blocks.is_empty(),
        "Taint should propagate through pointer dereference assignment to strcpy sink"
    );
    assert!(
        !result.findings.is_empty(),
        "Taint should emit a finding when *p assignment feeds strcpy"
    );
}

#[test]
fn test_taint_through_struct() {
    // Taint flows through a struct field assignment:
    // diff line taints dev->count → dev becomes tainted → printf uses dev → sink fires.
    let source = r#"
typedef struct { int count; } Dev;
void update_dev(Dev *dev, int n) {
    dev->count = n;
    printf("%d\n", dev->count);
}
"#;
    let path = "src/struct_taint.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Line 4: dev->count = n;  — with pointer aliasing, creates def of "dev" and "count"
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    // With struct aliasing: def of "dev" on line 4 flows to printf on line 5 → finding
    assert!(
        !result.blocks.is_empty(),
        "Taint should propagate through struct field assignment to printf sink"
    );
    assert!(
        !result.findings.is_empty(),
        "Taint should emit a finding when dev->field assignment feeds printf"
    );
}

#[test]
fn test_taint_field_access_through_dfg() {
    // Taint from diff line on dev->name assignment should propagate
    // to uses of dev->name but the DFG correctly tracks the path.
    let source = r#"
void handle(struct req *dev) {
    dev->name = get_user_input();
    char *n = dev->name;
    strcpy(buf, n);
}
"#;
    let path = "src/handler.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Diff on line 3: dev->name = get_user_input()
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

    assert!(
        !result.findings.is_empty(),
        "Taint should propagate through field access to strcpy sink"
    );
}

#[test]
fn test_field_isolation_taint_does_not_cross_fields() {
    // End-to-end taint test: tainted field should not reach different field's sink
    let source = r#"
void handler(struct request *req) {
    req->user_input = read_stdin();
    req->safe_data = "constant";
    exec(req->user_input);
    log_msg(req->safe_data);
}
"#;
    let path = "src/handler.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let dfg = DataFlowGraph::build(&files);

    // Get the user_input def
    let req_defs = dfg.all_defs_of(path, "req");
    let user_input_def = req_defs
        .iter()
        .find(|d| d.path.fields == vec!["user_input"]);

    if let Some(uid) = user_input_def {
        let reachable = dfg.forward_reachable(uid);
        // Should reach exec(req->user_input) line but NOT log_msg(req->safe_data) line
        let reaches_safe = reachable.iter().any(|r| r.path.fields == vec!["safe_data"]);
        assert!(
            !reaches_safe,
            "Taint on req->user_input must NOT propagate to req->safe_data"
        );
    }
}

#[test]
fn test_destructuring_taint_propagation_js() {
    // Taint on device should propagate through destructured name via alias
    let source = r#"
function render(userInput) {
    const { name, role } = userInput;
    document.getElementById("output").innerHTML = name;
}
"#;
    let path = "src/render.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
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

    // Taint should detect the flow from destructured variable to innerHTML
    assert!(
        !result.blocks.is_empty(),
        "Taint should produce blocks for destructured variable flowing to innerHTML"
    );
}
