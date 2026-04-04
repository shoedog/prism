#[path = "../../common/mod.rs"]
mod common;
use common::*;

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

// === Tier 3: Provenance — C++ (item 14) ===

#[test]
fn test_cpp_provenance_cin_user_input() {
    let source = r#"
#include <iostream>
#include <string>

std::string get_name() {
    std::string name;
    std::cin >> name;
    return name;
}
"#;
    let path = "src/input.cpp";
    let parsed = ParsedFile::parse(path, source, Language::Cpp).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([7]),
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
        !result.findings.is_empty() || !result.blocks.is_empty(),
        "Provenance should detect C++ std::cin as user input source"
    );
}

#[test]
fn test_cpp_provenance_getenv() {
    let source = r#"
#include <cstdlib>

std::string get_config_path() {
    const char* path = getenv("CONFIG_PATH");
    return std::string(path);
}
"#;
    let path = "src/config.cpp";
    let parsed = ParsedFile::parse(path, source, Language::Cpp).unwrap();
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
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::ProvenanceSlice),
        None,
    )
    .unwrap();

    assert!(
        !result.findings.is_empty() || !result.blocks.is_empty(),
        "Provenance should detect C++ getenv() as environment source"
    );
}

#[test]
fn test_cpp_provenance_recv_network() {
    let source = r#"
#include <sys/socket.h>

int handle_client(int sockfd) {
    char buffer[1024];
    int n = recv(sockfd, buffer, sizeof(buffer), 0);
    process(buffer, n);
    return 0;
}
"#;
    let path = "src/server.cpp";
    let parsed = ParsedFile::parse(path, source, Language::Cpp).unwrap();
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

    assert!(
        !result.findings.is_empty() || !result.blocks.is_empty(),
        "Provenance should detect C++ recv() as network input source"
    );
}
