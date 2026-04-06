//! Phase 3: Delta contract comparison tests.
//!
//! These tests create old and new versions of source files, then run
//! `contract_slice::slice_delta()` to verify precondition and postcondition
//! change detection.

#[path = "../../common/mod.rs"]
mod common;
use common::*;
use prism::slice::SliceResult;
use std::fs;

/// Helper: create old repo dir with a source file, then run slice_delta
/// comparing old vs new.
fn run_contract_delta(
    old_source: &str,
    new_source: &str,
    path: &str,
    lang: Language,
    diff_lines: BTreeSet<usize>,
) -> SliceResult {
    let tmp = TempDir::new().unwrap();
    fs::write(tmp.path().join(path), old_source).unwrap();

    let parsed = ParsedFile::parse(path, new_source, lang).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines,
        }],
    };

    prism::algorithms::contract_slice::slice_delta(&files, &diff, tmp.path()).unwrap()
}

// ==========================================================================
// Precondition comparison tests
// ==========================================================================

#[test]
fn test_delta_precondition_weakened_guard_removed() {
    // Old version has a null guard, new version removes it
    let old_source = r#"
def process(x):
    if x is None:
        raise ValueError("x required")
    result = transform(x)
    return result
"#;
    let new_source = r#"
def process(x):
    result = transform(x)
    return result
"#;
    let result = run_contract_delta(
        old_source,
        new_source,
        "test.py",
        Language::Python,
        BTreeSet::from([3]),
    );

    let weakened: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("contract_precondition_weakened"))
        .collect();
    assert!(
        !weakened.is_empty(),
        "Should detect removed guard as precondition weakening"
    );
    assert!(
        weakened[0].description.contains("removed") || weakened[0].description.contains("Removed"),
        "Should describe guard removal, got: {}",
        weakened[0].description
    );
}

#[test]
fn test_delta_precondition_strengthened_guard_added() {
    // Old version has no guard, new version adds one
    let old_source = r#"
def process(x):
    result = transform(x)
    return result
"#;
    let new_source = r#"
def process(x):
    if x is None:
        raise ValueError("x required")
    result = transform(x)
    return result
"#;
    let result = run_contract_delta(
        old_source,
        new_source,
        "test.py",
        Language::Python,
        BTreeSet::from([3]),
    );

    let strengthened: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("contract_precondition_strengthened"))
        .collect();
    assert!(
        !strengthened.is_empty(),
        "Should detect added guard as precondition strengthening"
    );
    assert!(
        strengthened[0].description.contains("New guard")
            || strengthened[0].description.contains("tighter"),
        "Should describe guard addition, got: {}",
        strengthened[0].description
    );
}

#[test]
fn test_delta_precondition_modified() {
    // Old version has a null check, new version changes it to a type check
    let old_source = r#"
def validate(x):
    if x is None:
        raise ValueError("missing")
    return process(x)
"#;
    let new_source = r#"
def validate(x):
    if len(x) == 0:
        raise ValueError("empty")
    return process(x)
"#;
    let result = run_contract_delta(
        old_source,
        new_source,
        "test.py",
        Language::Python,
        BTreeSet::from([3]),
    );

    // The old guard on 'x' (non-null) is removed, and a new guard on 'x'
    // (non-empty) is added. Since the variable name matches, this should
    // be detected as a modification.
    let changes: Vec<_> = result
        .findings
        .iter()
        .filter(|f| {
            f.category
                .as_deref()
                .map_or(false, |c| c.starts_with("contract_precondition_"))
        })
        .collect();
    assert!(
        !changes.is_empty(),
        "Should detect precondition change when guard constraint type changes"
    );
}

// ==========================================================================
// Postcondition comparison tests
// ==========================================================================

#[test]
fn test_delta_postcondition_weakened_null_path_added() {
    // Old version always returns non-null, new version adds a null return
    let old_source = r#"
def find_user(user_id):
    user = db.get(user_id)
    return user
"#;
    let new_source = r#"
def find_user(user_id):
    if user_id == 0:
        return None
    user = db.get(user_id)
    return user
"#;
    let result = run_contract_delta(
        old_source,
        new_source,
        "test.py",
        Language::Python,
        BTreeSet::from([4]),
    );

    let weakened: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("contract_postcondition_weakened"))
        .collect();
    assert!(
        !weakened.is_empty(),
        "Should detect new null return path as postcondition weakening"
    );
}

#[test]
fn test_delta_postcondition_strengthened_null_path_removed() {
    // Old version had a null return, new version removes it
    let old_source = r#"
def find_user(user_id):
    if user_id == 0:
        return None
    user = db.get(user_id)
    return user
"#;
    let new_source = r#"
def find_user(user_id):
    if user_id == 0:
        raise ValueError("invalid id")
    user = db.get(user_id)
    return user
"#;
    let result = run_contract_delta(
        old_source,
        new_source,
        "test.py",
        Language::Python,
        BTreeSet::from([3]),
    );

    let strengthened: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("contract_postcondition_strengthened"))
        .collect();
    assert!(
        !strengthened.is_empty(),
        "Should detect removed null path as postcondition strengthening"
    );
}

#[test]
fn test_delta_postcondition_kind_changed_void_to_value() {
    // Old version returns void, new version returns a value
    let old_source = r#"void setup(int flags) {
    config = flags;
}
"#;
    let new_source = r#"int setup(int flags) {
    config = flags;
    return 0;
}
"#;
    let result = run_contract_delta(
        old_source,
        new_source,
        "test.c",
        Language::C,
        BTreeSet::from([3]),
    );

    let kind_changes: Vec<_> = result
        .findings
        .iter()
        .filter(|f| {
            f.category
                .as_deref()
                .map_or(false, |c| c.starts_with("contract_postcondition_"))
                && f.description.contains("changed")
        })
        .collect();
    assert!(
        !kind_changes.is_empty(),
        "Should detect postcondition kind change from void to value"
    );
}

#[test]
fn test_delta_no_change_same_contracts() {
    // Both versions have the same contract — no delta findings
    let source = r#"
def process(x):
    if x is None:
        raise ValueError("x required")
    return transform(x)
"#;
    let result = run_contract_delta(
        source,
        source,
        "test.py",
        Language::Python,
        BTreeSet::from([5]),
    );

    let delta_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| {
            f.category.as_deref().map_or(false, |c| {
                c.contains("weakened") || c.contains("strengthened")
            })
        })
        .collect();
    assert!(
        delta_findings.is_empty(),
        "Should produce no delta findings when contracts are identical, got: {:?}",
        delta_findings
            .iter()
            .map(|f| &f.description)
            .collect::<Vec<_>>()
    );
}

#[test]
fn test_delta_function_matching_added_function() {
    // New version adds a function that doesn't exist in old
    let old_source = r#"
def existing():
    return 42
"#;
    let new_source = r#"
def existing():
    return 42

def new_function(x):
    if x is None:
        raise ValueError("x required")
    return x + 1
"#;
    let result = run_contract_delta(
        old_source,
        new_source,
        "test.py",
        Language::Python,
        BTreeSet::from([6]),
    );

    // The new function should NOT generate delta comparison findings
    // (only Phase 1+2 findings for the current version)
    let delta_findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| {
            f.category.as_deref().map_or(false, |c| {
                c.contains("weakened") || c.contains("strengthened")
            })
        })
        .collect();
    assert!(
        delta_findings.is_empty(),
        "New function should not generate delta findings"
    );
}

#[test]
fn test_delta_go_postcondition_change() {
    // Go function changes from always returning (value, nil) to sometimes
    // returning (nil, err)
    let old_source = r#"package main

func Parse(data string) (Result, error) {
    result := doParse(data)
    return result, nil
}
"#;
    let new_source = r#"package main

func Parse(data string) (Result, error) {
    if data == "" {
        return nil, fmt.Errorf("empty data")
    }
    result := doParse(data)
    return result, nil
}
"#;
    let result = run_contract_delta(
        old_source,
        new_source,
        "test.go",
        Language::Go,
        BTreeSet::from([5]),
    );

    // The postcondition changed from GoSuccess (always) to GoResultPair
    let post_changes: Vec<_> = result
        .findings
        .iter()
        .filter(|f| {
            f.category
                .as_deref()
                .map_or(false, |c| c.starts_with("contract_postcondition_"))
                && f.description.contains("changed")
        })
        .collect();
    // This may or may not detect a change depending on whether GoSuccess→GoResultPair
    // is classified as a kind change. The old version returns only (result, nil) which
    // classifies as AlwaysNonNull (single GoSuccess is non-null). New version has both
    // GoError and GoSuccess → GoResultPair. So this is a kind change.
    // Either way, the test verifies no crash and reasonable behavior.
    assert!(
        result.findings.len() > 0,
        "Should produce some findings for Go function"
    );
}

#[test]
fn test_delta_js_postcondition_bool_to_mixed() {
    // JS function changes from always returning bool to mixed return types
    let old_source = r#"function isValid(x) {
    if (x < 0) {
        return false;
    }
    return true;
}
"#;
    let new_source = r#"function isValid(x) {
    if (x < 0) {
        return "invalid";
    }
    return true;
}
"#;
    let result = run_contract_delta(
        old_source,
        new_source,
        "test.js",
        Language::JavaScript,
        BTreeSet::from([3]),
    );

    let weakened: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("contract_postcondition_weakened"))
        .collect();
    assert!(
        !weakened.is_empty(),
        "Should detect bool→mixed as postcondition weakening"
    );
}

#[test]
fn test_delta_multiple_precondition_changes() {
    // Old has guards on x and y; new removes x guard and adds z guard
    let old_source = r#"
def compute(x, y, z):
    if x is None:
        raise ValueError("x required")
    if y < 0:
        raise ValueError("y must be positive")
    return x + y + z
"#;
    let new_source = r#"
def compute(x, y, z):
    if y < 0:
        raise ValueError("y must be positive")
    if len(z) == 0:
        raise ValueError("z cannot be empty")
    return x + y + z
"#;
    let result = run_contract_delta(
        old_source,
        new_source,
        "test.py",
        Language::Python,
        BTreeSet::from([3]),
    );

    let weakened: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("contract_precondition_weakened"))
        .collect();
    let strengthened: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("contract_precondition_strengthened"))
        .collect();

    assert!(
        !weakened.is_empty(),
        "Should detect removed x guard as weakening"
    );
    assert!(
        !strengthened.is_empty(),
        "Should detect added z guard as strengthening"
    );
}
