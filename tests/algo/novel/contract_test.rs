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

    // No precondition-related findings (contract summary or guard violation)
    let precondition_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("contract"))
        .collect();
    assert!(
        precondition_findings.is_empty(),
        "No precondition findings when function has no guard clauses"
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

// === Tier 2: Contract — Yoda conditions (item 10) ===

#[test]
fn test_contract_yoda_null_check_c() {
    // `if (NULL == ptr)` is a Yoda condition — should still be detected as non-null guard.
    let source = r#"
int process(const char* buf) {
    if (NULL == buf) {
        return -1;
    }
    int len = strlen(buf);
    return len;
}
"#;
    let result = run_contract(source, "process.c", Language::C, BTreeSet::from([6]));

    let contract_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("contract"))
        .collect();
    assert!(
        !contract_findings.is_empty(),
        "Should detect Yoda `NULL == buf` as a precondition guard"
    );

    let has_non_null = result
        .findings
        .iter()
        .any(|f| f.description.contains("non-null"));
    assert!(
        has_non_null,
        "Yoda NULL check should be classified as non-null constraint"
    );
}

#[test]
fn test_contract_yoda_nil_check_go() {
    // `if nil != err` is a Yoda nil-check.
    let source = r#"package main

func handle(err error) string {
    if nil != err {
        return ""
    }
    result := compute()
    return result
}
"#;
    let result = run_contract(source, "handle.go", Language::Go, BTreeSet::from([7]));

    let contract_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("contract"))
        .collect();
    assert!(
        !contract_findings.is_empty(),
        "Should detect Yoda `nil != err` as a precondition guard"
    );
}

// === Tier 2: Contract — Range/bounds checks (item 11) ===

#[test]
fn test_contract_range_check_python() {
    // `if x < 0` is a range check guard.
    let source = r#"
def withdraw(amount, balance):
    if amount < 0:
        raise ValueError("negative amount")
    if amount > balance:
        raise ValueError("insufficient funds")
    result = balance - amount
    return result
"#;
    let result = run_contract(source, "bank.py", Language::Python, BTreeSet::from([7]));

    let contract_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("contract"))
        .collect();
    assert!(
        !contract_findings.is_empty(),
        "Should detect range check guards (amount < 0, amount > balance)"
    );
}

// ==========================================================================
// Phase 2: Postcondition extraction tests
// ==========================================================================

#[test]
fn test_postcondition_always_non_null() {
    // All return paths return a non-null value → AlwaysNonNull
    let source = r#"
def find_user(user_id):
    if user_id is None:
        raise ValueError("missing id")
    user = db.get(user_id)
    return user
"#;
    let result = run_contract(source, "test.py", Language::Python, BTreeSet::from([5]));

    let post_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("contract_postcondition"))
        .collect();
    assert!(
        !post_findings.is_empty(),
        "Should emit postcondition summary"
    );
    assert!(
        post_findings[0].description.contains("non-null"),
        "Should detect non-null postcondition, got: {}",
        post_findings[0].description
    );
}

#[test]
fn test_postcondition_nullable() {
    // Mix of return None and return value → Nullable
    let source = r#"
def find_user(user_id):
    if user_id < 0:
        return None
    user = db.get(user_id)
    if not user:
        return None
    return user
"#;
    let result = run_contract(source, "test.py", Language::Python, BTreeSet::from([5]));

    let post_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("contract_postcondition"))
        .collect();
    assert!(
        !post_findings.is_empty(),
        "Should emit postcondition summary"
    );
    assert!(
        post_findings[0].description.contains("nullable"),
        "Should detect nullable postcondition, got: {}",
        post_findings[0].description
    );
}

#[test]
fn test_postcondition_void() {
    // No return value → Void
    let source = r#"void setup(int flags) {
    config = flags;
    init();
}
"#;
    let result = run_contract(source, "test.c", Language::C, BTreeSet::from([2]));

    let post_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("contract_postcondition"))
        .collect();
    assert!(
        !post_findings.is_empty(),
        "Should emit postcondition summary for void function"
    );
    assert!(
        post_findings[0].description.contains("void"),
        "Should detect void postcondition, got: {}",
        post_findings[0].description
    );
}

#[test]
fn test_postcondition_always_bool() {
    // All returns are true/false → AlwaysBool
    let source = r#"function isValid(x) {
    if (x < 0) {
        return false;
    }
    if (x > 100) {
        return false;
    }
    return true;
}
"#;
    let result = run_contract(source, "test.js", Language::JavaScript, BTreeSet::from([4]));

    let post_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("contract_postcondition"))
        .collect();
    assert!(
        !post_findings.is_empty(),
        "Should emit postcondition summary"
    );
    assert!(
        post_findings[0].description.contains("bool"),
        "Should detect bool postcondition, got: {}",
        post_findings[0].description
    );
}

#[test]
fn test_postcondition_go_result_pair() {
    // Go (value, error) return pattern → GoResultPair
    let source = r#"package main

func Parse(data string) (Result, error) {
    if data == "" {
        return nil, fmt.Errorf("empty data")
    }
    result := doParse(data)
    return result, nil
}
"#;
    let result = run_contract(source, "test.go", Language::Go, BTreeSet::from([7]));

    let post_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("contract_postcondition"))
        .collect();
    assert!(
        !post_findings.is_empty(),
        "Should emit postcondition summary"
    );
    assert!(
        post_findings[0].description.contains("error"),
        "Should detect Go (T, error) postcondition, got: {}",
        post_findings[0].description
    );
}

#[test]
fn test_postcondition_consistent_type() {
    // All returns are string literals → ConsistentType
    let source = r#"function getStatus(code: number): string {
    if (code === 200) {
        return "ok";
    }
    if (code === 404) {
        return "not found";
    }
    return "unknown";
}
"#;
    let result = run_contract(source, "test.ts", Language::TypeScript, BTreeSet::from([4]));

    let post_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("contract_postcondition"))
        .collect();
    assert!(
        !post_findings.is_empty(),
        "Should emit postcondition summary"
    );
    assert!(
        post_findings[0].description.contains("consistent-type")
            || post_findings[0].description.contains("string"),
        "Should detect consistent-type (string) postcondition, got: {}",
        post_findings[0].description
    );
}

#[test]
fn test_postcondition_new_null_path() {
    // Diff adds `return None` to a function with other non-null returns → warning
    let source = r#"
def get_user(user_id):
    if user_id == 0:
        return None
    user = db.get(user_id)
    return user
"#;
    // Line 4 is `return None` and is a diff line
    let result = run_contract(source, "test.py", Language::Python, BTreeSet::from([4]));

    let new_null_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("contract_postcondition_new_null"))
        .collect();
    assert!(
        !new_null_findings.is_empty(),
        "Should detect new null return path when diff adds return None"
    );
    assert!(
        new_null_findings[0]
            .description
            .contains("return None/null"),
        "Description should mention new null return, got: {}",
        new_null_findings[0].description
    );
}

#[test]
fn test_postcondition_modified_return() {
    // Diff touches a return line → violation warning
    let source = r#"function compute(x) {
    if (x < 0) {
        return -1;
    }
    return x * 2;
}
"#;
    // Line 5 is `return x * 2` and is a diff line
    let result = run_contract(source, "test.js", Language::JavaScript, BTreeSet::from([5]));

    let violations: Vec<_> = result
        .findings
        .iter()
        .filter(|f| {
            f.category.as_deref() == Some("contract_violation")
                && f.description.contains("Return behavior modified")
        })
        .collect();
    assert!(
        !violations.is_empty(),
        "Should flag modified return statement as contract violation"
    );
}

#[test]
fn test_postcondition_rust_trailing() {
    // Rust trailing expression detected as return
    let source = r#"fn compute(x: i32) -> i32 {
    if x > 0 {
        x * 2
    } else {
        -1
    }
}
"#;
    let result = run_contract(source, "test.rs", Language::Rust, BTreeSet::from([3]));

    let post_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("contract_postcondition"))
        .collect();
    assert!(
        !post_findings.is_empty(),
        "Should detect postcondition from Rust trailing expressions"
    );
    // The trailing expressions return numeric values
    let desc = &post_findings[0].description;
    assert!(
        desc.contains("non-null") || desc.contains("numeric") || desc.contains("consistent"),
        "Should classify trailing expressions, got: {}",
        desc
    );
}

#[test]
fn test_postcondition_raises_on_error() {
    // Function has raise but no null return → NonNullOrThrows
    let source = r#"
def parse_config(path):
    if not path:
        raise FileNotFoundError("no path")
    data = read_file(path)
    if not data:
        raise ValueError("empty file")
    return parse(data)
"#;
    let result = run_contract(source, "test.py", Language::Python, BTreeSet::from([5]));

    let post_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("contract_postcondition"))
        .collect();
    assert!(
        !post_findings.is_empty(),
        "Should emit postcondition summary"
    );
    assert!(
        post_findings[0].description.contains("non-null-or-throws")
            || post_findings[0].description.contains("non-null"),
        "Should detect non-null-or-throws postcondition, got: {}",
        post_findings[0].description
    );
}

#[test]
fn test_postcondition_summary() {
    // Info finding includes both preconditions and postconditions
    let source = r#"
def get_user(user_id):
    if user_id is None:
        raise ValueError("missing id")
    user = db.get(user_id)
    return user
"#;
    let result = run_contract(source, "test.py", Language::Python, BTreeSet::from([5]));

    // Check for contract summary that includes both pre and post conditions
    let contract_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("contract"))
        .collect();
    assert!(
        !contract_findings.is_empty(),
        "Should emit contract summary"
    );
    let desc = &contract_findings[0].description;
    assert!(
        desc.contains("preconditions") && desc.contains("postconditions"),
        "Contract summary should mention both pre and postconditions, got: {}",
        desc
    );
}

#[test]
fn test_contract_length_check_js() {
    // `if (arr.length === 0)` is a non-empty guard.
    let source = r#"
function first(arr) {
    if (arr.length === 0) {
        throw new Error("empty array");
    }
    const val = arr[0];
    return val;
}
"#;
    let result = run_contract(source, "util.js", Language::JavaScript, BTreeSet::from([6]));

    let contract_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("contract"))
        .collect();
    assert!(
        !contract_findings.is_empty(),
        "Should detect `.length === 0` as a non-empty precondition guard"
    );
}
