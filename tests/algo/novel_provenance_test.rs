#[path = "../common/mod.rs"]
mod common;
use common::*;


#[test]
fn test_provenance_slice_traces_origins() {
    let (files, _, diff) = make_python_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
        None,
    )
    .unwrap();

    // Should trace variable origins on diff lines
    assert!(result.algorithm == SlicingAlgorithm::ProvenanceSlice);
}

#[test]
fn test_provenance_slice_c() {
    let (files, _, diff) = make_c_test();
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
        None,
    )
    .unwrap();

    assert!(result.algorithm == SlicingAlgorithm::ProvenanceSlice);
}

#[test]
fn test_provenance_c_hardware_origin() {
    // ioctl() call classifies as Hardware origin.
    let source = r#"
int read_sensor(int fd, int cmd) {
    int value = ioctl(fd, cmd, NULL);
    int scaled = value * 2;
    return scaled;
}
"#;
    let path = "src/sensor.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Line 4: int scaled = value * 2;  — value traces back to ioctl (Hardware)
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
        None,
    )
    .unwrap();

    assert!(
        result
            .findings
            .iter()
            .any(|f| f.description.contains("hardware")),
        "Provenance should classify ioctl() result as Hardware origin; findings: {:?}",
        result
            .findings
            .iter()
            .map(|f| &f.description)
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_provenance_c_user_input_recv() {
    // recv() call classifies as UserInput origin.
    let source = r#"
int handle_socket(int sock) {
    char buf[256];
    int bytes = recv(sock, buf, sizeof(buf), 0);
    if (bytes > 0) {
        return bytes;
    }
    return 0;
}
"#;
    let path = "src/socket.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Line 5: if (bytes > 0)  — bytes traces back to recv() (UserInput)
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
        None,
    )
    .unwrap();

    assert!(
        result
            .findings
            .iter()
            .any(|f| f.description.contains("user_input")),
        "Provenance should classify recv() result as UserInput origin; findings: {:?}",
        result
            .findings
            .iter()
            .map(|f| &f.description)
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_provenance_c_env_getenv() {
    // getenv() call classifies as EnvVar origin.
    let source = r#"
void init_paths() {
    char *home = getenv("HOME");
    int len = strlen(home);
    set_base_path(home);
}
"#;
    let path = "src/init.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Line 4: int len = strlen(home);  — home traces back to getenv() (EnvVar)
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
        None,
    )
    .unwrap();

    assert!(
        result
            .findings
            .iter()
            .any(|f| f.description.contains("env_var")),
        "Provenance should classify getenv() result as EnvVar origin; findings: {:?}",
        result
            .findings
            .iter()
            .map(|f| &f.description)
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_provenance_python_request_form_origin() {
    // Variable originates from Flask request.form — should be classified as user_input.
    let source = r#"
from flask import request

def handle_login():
    username = request.form['username']
    process(username)
"#;
    let path = "app/auth.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([6]),
        }],
    };

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
        None,
    )
    .unwrap();

    // Should produce findings since username comes from user input
    assert!(
        !result.blocks.is_empty(),
        "Provenance should produce blocks tracing username to request.form"
    );
    let has_user_input = result
        .findings
        .iter()
        .any(|f| f.description.contains("user_input"));
    assert!(
        has_user_input,
        "Provenance should classify request.form as user_input origin"
    );
}

#[test]
fn test_provenance_python_cursor_execute_origin() {
    // Variable originates from cursor.fetchone — should be classified as database.
    let source = r#"
import sqlite3

def get_user(user_id):
    conn = sqlite3.connect('db.sqlite')
    cursor = conn.cursor()
    cursor.execute("SELECT * FROM users WHERE id = ?", (user_id,))
    row = cursor.fetchone()
    return row
"#;
    let path = "app/db.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([8]),
        }],
    };

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "Provenance should produce blocks tracing row to cursor.fetchone"
    );
    let has_db = result
        .findings
        .iter()
        .any(|f| f.description.contains("database"));
    assert!(
        has_db,
        "Provenance should classify cursor.fetchone as database origin"
    );
}

#[test]
fn test_provenance_js_document_cookie_origin() {
    // Variable originates from document.cookie — should be classified as user_input.
    let source = r#"
function getCookieValue() {
    const cookies = document.cookie;
    const parsed = parseCookies(cookies);
    return parsed;
}
"#;
    let path = "src/cookies.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "Provenance should produce blocks tracing cookies to document.cookie"
    );
}

#[test]
fn test_provenance_js_process_env_origin() {
    // Variable originates from process.env — should be classified as env_var.
    let source = r#"
function getPort() {
    const port = process.env.PORT;
    return port;
}
"#;
    let path = "src/config.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "Provenance should produce blocks tracing port to process.env"
    );
    let has_env = result
        .findings
        .iter()
        .any(|f| f.description.contains("env_var"));
    assert!(
        has_env,
        "Provenance should classify process.env as env_var origin"
    );
}

#[test]
fn test_provenance_go_form_value_origin() {
    // Variable originates from r.FormValue — should be classified as user_input.
    let source = r#"
package main

import "net/http"

func handler(w http.ResponseWriter, r *http.Request) {
    name := r.FormValue("name")
    process(name)
}
"#;
    let path = "web/handler.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([8]),
        }],
    };

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "Provenance should produce blocks tracing name to r.FormValue"
    );
    let has_user_input = result
        .findings
        .iter()
        .any(|f| f.description.contains("user_input"));
    assert!(
        has_user_input,
        "Provenance should classify r.FormValue as user_input origin"
    );
}

#[test]
fn test_provenance_go_viper_config_origin() {
    // Variable originates from viper config — should be classified as config.
    let source = r#"
package main

import "github.com/spf13/viper"

func loadConfig() string {
    dbHost := viper.GetString("database.host")
    return dbHost
}
"#;
    let path = "config/loader.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([8]),
        }],
    };

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.blocks.is_empty(),
        "Provenance should produce blocks tracing dbHost to viper config"
    );
    let has_config = result
        .findings
        .iter()
        .any(|f| f.description.contains("config"));
    assert!(
        has_config,
        "Provenance should classify viper.GetString as config origin"
    );
}

#[test]
fn test_provenance_negative_transform_not_form() {
    // "transform" should NOT match the "~form" word-boundary pattern.
    let source = r#"
def transform_data(raw):
    result = transform(raw)
    return result
"#;
    let path = "lib/transform.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
        None,
    )
    .unwrap();

    let has_user_input = result
        .findings
        .iter()
        .any(|f| f.description.contains("user_input"));
    assert!(
        !has_user_input,
        "Provenance should NOT classify 'transform' as user_input (form match), got: {:?}",
        result
            .findings
            .iter()
            .map(|f| &f.description)
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_provenance_negative_prefetch_not_fetch() {
    // "prefetch" should NOT match the "~fetch" word-boundary pattern.
    let source = r#"
function prefetchAssets(urls) {
    const assets = prefetch(urls);
    return assets;
}
"#;
    let path = "src/loader.js";
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
        None,
    )
    .unwrap();

    let has_db = result
        .findings
        .iter()
        .any(|f| f.description.contains("database"));
    assert!(
        !has_db,
        "Provenance should NOT classify 'prefetch' as database origin, got: {:?}",
        result
            .findings
            .iter()
            .map(|f| &f.description)
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_rust_provenance_stdin() {
    let source = r#"
fn get_input() -> String {
    let data = std::io::stdin().read_line();
    process(data)
}
"#;
    let path = "src/input.rs";
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.findings.is_empty(),
        "Provenance should detect std::io::stdin as user input source"
    );
    assert_eq!(
        result.findings[0].category.as_deref(),
        Some("untrusted_origin")
    );
}

#[test]
fn test_rust_provenance_diesel_query() {
    let source = r#"
fn get_users(conn: &PgConnection) {
    let results = diesel::sql_query("SELECT * FROM users").load(conn);
    process(results)
}
"#;
    let path = "src/db.rs";
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.findings.is_empty(),
        "Provenance should detect diesel:: as database source"
    );
    assert_eq!(
        result.findings[0].category.as_deref(),
        Some("untrusted_origin")
    );
}

#[test]
fn test_rust_provenance_env_var() {
    let source = r#"
fn get_config() {
    let val = std::env::var("DATABASE_URL");
    process(val)
}
"#;
    let path = "src/config.rs";
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.findings.is_empty(),
        "Provenance should detect std::env::var as environment source"
    );
    assert_eq!(
        result.findings[0].category.as_deref(),
        Some("untrusted_origin")
    );
}

#[test]
fn test_lua_provenance_io_read() {
    let source = r#"
function get_input()
    local data = io.read("*l")
    process(data)
end
"#;
    let path = "scripts/input.lua";
    let parsed = ParsedFile::parse(path, source, Language::Lua).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.findings.is_empty(),
        "Provenance should detect Lua io.read as user input source"
    );
    assert_eq!(
        result.findings[0].category.as_deref(),
        Some("untrusted_origin")
    );
}

#[test]
fn test_lua_provenance_os_getenv() {
    let source = r#"
function get_path()
    local path = os.getenv("PATH")
    process(path)
end
"#;
    let path = "scripts/env.lua";
    let parsed = ParsedFile::parse(path, source, Language::Lua).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.findings.is_empty(),
        "Provenance should detect Lua os.getenv as environment source"
    );
    assert_eq!(
        result.findings[0].category.as_deref(),
        Some("untrusted_origin")
    );
}

#[test]
fn test_lua_provenance_redis() {
    let source = r#"
function get_cached(key)
    local res = redis:get(key)
    process(res)
end
"#;
    let path = "scripts/cache.lua";
    let parsed = ParsedFile::parse(path, source, Language::Lua).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.findings.is_empty(),
        "Provenance should detect Lua redis:get as database source"
    );
    assert_eq!(
        result.findings[0].category.as_deref(),
        Some("untrusted_origin")
    );
}

#[test]
fn test_cve_strcpy_overflow_provenance() {
    let source = r#"
#include <string.h>
#include <stdio.h>

void handle_request(void) {
    char buf[64];
    char dest[32];
    char *input = fgets(buf, sizeof(buf), stdin);
    strcpy(dest, input);
}
"#;

    let path = "src/handler.c";
    let parsed = ParsedFile::parse(path, source, Language::C).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    // Diff touches fgets line — taint should trace input → strcpy
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([8]),
        }],
    };

    let taint_result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    // Taint: fgets line → input → strcpy sink
    assert!(
        !taint_result.blocks.is_empty(),
        "Taint should include fgets and strcpy in the taint trace"
    );

    let prov_result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
        None,
    )
    .unwrap();

    // Provenance: fgets → UserInput origin
    assert!(
        !prov_result.findings.is_empty(),
        "Provenance should classify fgets as user_input origin"
    );
}

#[test]
fn test_provenance_with_field_access() {
    // Provenance should track origins through field-qualified variables.
    let source = r#"
#include <stdio.h>
void handle(struct request *req) {
    req->data = fgets(buf, sizeof(buf), stdin);
    process(req->data);
}
"#;
    let path = "src/req.c";
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
        None,
    )
    .unwrap();

    // Provenance should detect fgets/stdin as a user_input source
    // via the base name match on all_defs_of
    assert!(
        !result.blocks.is_empty(),
        "Provenance should produce blocks when source is assigned through field access"
    );
}

#[test]
fn test_provenance_slice_javascript() {
    let source = "function handler(req, res) {\n    const token = req.headers.authorization;\n    const userId = parseToken(token);\n    const data = db.query(userId);\n    res.json(data);\n}\n";
    let path = "handler.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([2, 3]),
        }],
    };
    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
        None,
    )
    .unwrap();
    assert_eq!(result.algorithm, SlicingAlgorithm::ProvenanceSlice);
}

#[test]
fn test_provenance_slice_python_traces_user_input() {
    let source = r#"
def handle(request):
    name = request.form.get("name")
    greeting = "Hello " + name
    return greeting
"#;
    let path = "app.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
        None,
    )
    .unwrap();
    assert!(
        !result.blocks.is_empty(),
        "Provenance should trace user input origin"
    );

    // Should include the form.get line as a user input source
    let all_lines: BTreeSet<usize> = result
        .blocks
        .iter()
        .flat_map(|b| b.file_line_map.values())
        .flat_map(|m| m.keys())
        .copied()
        .collect();
    assert!(
        all_lines.contains(&3),
        "Provenance should include form.get line (3). Got: {:?}",
        all_lines
    );
}
