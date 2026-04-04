#[path = "../../common/mod.rs"]
mod common;
use common::*;
use prism::slice::SliceResult;

fn run_contract(
    source: &str,
    path: &str,
    lang: Language,
    diff_lines: BTreeSet<usize>,
) -> SliceResult {
    let parsed = ParsedFile::parse(path, source, lang).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines,
        }],
    };

    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::ContractSlice);
    algorithms::run_slicing_compat(&files, &diff, &config, None).unwrap()
}

#[test]
fn test_contract_detects_null_guard_python() {
    let source = r#"
def process(x):
    if x is None:
        raise ValueError("x required")
    result = transform(x)
    return result
"#;
    let result = run_contract(source, "test.py", Language::Python, BTreeSet::from([5]));

    // Should find a contract summary (info finding)
    let contract_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("contract"))
        .collect();
    assert!(
        !contract_findings.is_empty(),
        "Should detect contract summary with null guard"
    );
    assert!(
        contract_findings[0].description.contains("non-null"),
        "Should identify non-null constraint, got: {}",
        contract_findings[0].description
    );
}

#[test]
fn test_contract_flags_modified_guard_python() {
    let source = r#"
def process(x):
    if x is None:
        raise ValueError("x required")
    result = transform(x)
    return result
"#;
    // Diff touches the guard clause itself (line 3)
    let result = run_contract(source, "test.py", Language::Python, BTreeSet::from([3]));

    let violations: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("contract_violation"))
        .collect();
    assert!(
        !violations.is_empty(),
        "Should flag modified guard clause as contract violation"
    );
    assert!(
        violations[0].description.contains("Guard clause modified"),
        "Description should mention guard modification"
    );
}

#[test]
fn test_contract_no_guard_no_findings() {
    let source = r#"
def simple(x):
    return x + 1
"#;
    let result = run_contract(source, "test.py", Language::Python, BTreeSet::from([3]));

    let contract_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| {
            f.category.as_deref() == Some("contract")
                || f.category.as_deref() == Some("contract_violation")
        })
        .collect();
    assert!(
        contract_findings.is_empty(),
        "No contract findings when function has no guard clauses"
    );
}

#[test]
fn test_contract_go_error_guard() {
    let source = r#"package main

func getData(path string) ([]byte, error) {
    if path == "" {
        return nil, fmt.Errorf("empty path")
    }
    data := readFile(path)
    return data, nil
}
"#;
    let result = run_contract(source, "test.go", Language::Go, BTreeSet::from([7]));

    let contract_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("contract"))
        .collect();
    assert!(
        !contract_findings.is_empty(),
        "Should detect Go guard clause"
    );
}

#[test]
fn test_contract_js_type_guard() {
    let source = r#"function validate(x) {
    if (typeof x !== 'string') {
        throw new TypeError('expected string');
    }
    return x.trim();
}
"#;
    let result = run_contract(source, "test.js", Language::JavaScript, BTreeSet::from([5]));

    let contract_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("contract"))
        .collect();
    assert!(
        !contract_findings.is_empty(),
        "Should detect JS typeof guard"
    );
    assert!(
        contract_findings[0].description.contains("type-check"),
        "Should identify type-check constraint, got: {}",
        contract_findings[0].description
    );
}

#[test]
fn test_contract_c_null_check() {
    let source = r#"int process(int *ptr) {
    if (!ptr) {
        return -1;
    }
    return *ptr + 1;
}
"#;
    let result = run_contract(source, "test.c", Language::C, BTreeSet::from([5]));

    let contract_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("contract"))
        .collect();
    assert!(
        !contract_findings.is_empty(),
        "Should detect C null check guard"
    );
    assert!(
        contract_findings[0].description.contains("non-null"),
        "Should identify non-null constraint, got: {}",
        contract_findings[0].description
    );
}

#[test]
fn test_contract_multiple_guards() {
    let source = r#"
def fetch_data(url, timeout):
    if url is None:
        raise ValueError("url required")
    if timeout < 0:
        raise ValueError("timeout must be positive")
    if len(url) == 0:
        raise ValueError("url cannot be empty")
    return http_get(url, timeout)
"#;
    let result = run_contract(source, "test.py", Language::Python, BTreeSet::from([9]));

    let contract_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("contract"))
        .collect();
    assert!(
        !contract_findings.is_empty(),
        "Should detect multiple guards"
    );
    // The summary should mention multiple preconditions
    let desc = &contract_findings[0].description;
    assert!(
        desc.contains("non-null") || desc.contains("url"),
        "Should mention url constraint, got: {}",
        desc
    );
}

#[test]
fn test_contract_deep_guard_excluded() {
    // Guard clause at line 20 in a 25-line function (80%) → not in guard zone
    let source = r#"
def long_function(x):
    step1 = prepare(x)
    step2 = process(step1)
    step3 = validate(step2)
    step4 = transform(step3)
    step5 = finalize(step4)
    step6 = cleanup(step5)
    step7 = wrap(step6)
    step8 = deliver(step7)
    step9 = log(step8)
    step10 = archive(step9)
    step11 = notify(step10)
    step12 = complete(step11)
    step13 = verify(step12)
    step14 = persist(step13)
    step15 = checkpoint(step14)
    step16 = summarize(step15)
    step17 = report(step16)
    if x is None:
        raise ValueError("too late for guard")
    return step17
"#;
    let result = run_contract(source, "test.py", Language::Python, BTreeSet::from([3]));

    // The guard at line 20 is past the 30% zone, so it should NOT be a precondition
    let contract_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("contract"))
        .collect();
    assert!(
        contract_findings.is_empty(),
        "Guard at 80% of function body should be excluded from preconditions"
    );
}

#[test]
fn test_contract_assert_statement_python() {
    let source = r#"
def process(items):
    assert len(items) > 0
    return items[0]
"#;
    let result = run_contract(source, "test.py", Language::Python, BTreeSet::from([4]));

    let contract_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("contract"))
        .collect();
    assert!(
        !contract_findings.is_empty(),
        "Should detect assert statement as precondition"
    );
    assert!(
        contract_findings[0].description.contains("assertion"),
        "Should identify assertion constraint, got: {}",
        contract_findings[0].description
    );
}

#[test]
fn test_contract_rust_guard() {
    let source = r#"fn process(data: &[u8]) -> Result<(), Error> {
    if data.is_empty() {
        return Err(Error::new("empty data"));
    }
    let result = parse(data);
    Ok(result)
}
"#;
    let result = run_contract(source, "test.rs", Language::Rust, BTreeSet::from([5]));

    let contract_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("contract"))
        .collect();
    assert!(
        !contract_findings.is_empty(),
        "Should detect Rust is_empty() guard"
    );
}

// === 2.7 Behavioral test: Contract — Go err != nil guard ===

#[test]
fn test_contract_go_err_nil_guard() {
    // `if err != nil { return err }` is a standard Go precondition.
    // Contract should detect it as a nil-check constraint.
    let source = r#"package main

import "os"

func readConfig(path string) ([]byte, error) {
    data, err := os.ReadFile(path)
    if err != nil {
        return nil, err
    }
    return data, nil
}
"#;
    let result = run_contract(source, "config.go", Language::Go, BTreeSet::from([7]));

    let contract_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("contract"))
        .collect();
    assert!(
        !contract_findings.is_empty(),
        "Should detect Go `if err != nil` as a precondition guard"
    );

    // Verify it's classified as a nil-check, not just a generic guard
    let has_nil_check = result
        .findings
        .iter()
        .any(|f| f.description.contains("nil-check") || f.description.contains("nil"));
    assert!(
        has_nil_check,
        "Go err != nil guard should be classified as nil-check"
    );
}

// === Tier 1: Contract tests for Java / C++ / Lua ===

#[test]
fn test_contract_java_null_guard() {
    let source = r#"
public class UserService {
    public String getDisplayName(String userId) {
        if (userId == null) {
            throw new IllegalArgumentException("userId must not be null");
        }
        String name = lookupName(userId);
        return name.toUpperCase();
    }
}
"#;
    let result = run_contract(
        source,
        "UserService.java",
        Language::Java,
        BTreeSet::from([7]),
    );

    let contract_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("contract"))
        .collect();
    assert!(
        !contract_findings.is_empty(),
        "Should detect Java `if (userId == null)` as a precondition guard"
    );

    let has_non_null = result
        .findings
        .iter()
        .any(|f| f.description.contains("non-null"));
    assert!(
        has_non_null,
        "Java null guard should be classified as non-null constraint"
    );
}

#[test]
fn test_contract_cpp_nullptr_guard() {
    let source = r#"
#include <stdexcept>

std::string format_entry(const Entry* entry) {
    if (!entry) {
        return "";
    }
    std::string result = entry->name;
    return result;
}
"#;
    let result = run_contract(source, "format.cpp", Language::Cpp, BTreeSet::from([7]));

    let contract_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("contract"))
        .collect();
    assert!(
        !contract_findings.is_empty(),
        "Should detect C++ `if (!entry)` as a precondition guard"
    );

    let has_non_null = result
        .findings
        .iter()
        .any(|f| f.description.contains("non-null"));
    assert!(
        has_non_null,
        "C++ nullptr guard should be classified as non-null constraint"
    );
}

#[test]
fn test_contract_lua_nil_guard() {
    let source = r#"
function process_request(req)
    if req == nil then
        error("req must not be nil")
    end
    local body = req.body
    return body
end
"#;
    let result = run_contract(source, "handler.lua", Language::Lua, BTreeSet::from([6]));

    let contract_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("contract"))
        .collect();
    assert!(
        !contract_findings.is_empty(),
        "Should detect Lua `if req == nil then` as a precondition guard"
    );
}
