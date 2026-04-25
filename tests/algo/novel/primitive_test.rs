#[path = "../../common/mod.rs"]
mod common;
use common::*;
use prism::slice::SliceResult;

fn run_primitive(
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
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::PrimitiveSlice);
    algorithms::run_slicing_compat(&files, &diff, &config, None).unwrap()
}

fn findings_for_rule(result: &SliceResult, rule_id: &str) -> Vec<SliceFinding> {
    result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some(rule_id))
        .cloned()
        .collect()
}

// --- Python: HASH_TRUNCATED_BELOW_128_BITS ---

#[test]
fn test_primitive_hash_trunc_direct_below_128_python() {
    let source = r#"
import hashlib

def make_id(s):
    h = hashlib.sha256(s.encode())
    return h.hexdigest()[:12]
"#;
    let result = run_primitive(source, "src/util.py", Language::Python, BTreeSet::from([6]));
    let findings = findings_for_rule(&result, "HASH_TRUNCATED_BELOW_128_BITS");
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].severity, "concern");
    assert!(
        findings[0].description.contains("48 bits")
            || findings[0].description.contains("hex chars"),
        "description should mention bit count or hex chars, got: {}",
        findings[0].description
    );
}

#[test]
fn test_primitive_hash_trunc_at_threshold_no_finding_python() {
    let source = r#"
import hashlib

def make_id(s):
    h = hashlib.sha256(s.encode())
    return h.hexdigest()[:32]
"#;
    let result = run_primitive(source, "src/util.py", Language::Python, BTreeSet::from([6]));
    let findings = findings_for_rule(&result, "HASH_TRUNCATED_BELOW_128_BITS");
    assert!(
        findings.is_empty(),
        "[:32] is exactly 128 bits — should not fire"
    );
}

#[test]
fn test_primitive_hash_trunc_raw_below_threshold_python() {
    let source = r#"
import hashlib

def make_raw_id(s):
    h = hashlib.sha256(s.encode())
    return h.digest()[:8]
"#;
    let result = run_primitive(source, "src/util.py", Language::Python, BTreeSet::from([6]));
    let findings = findings_for_rule(&result, "HASH_TRUNCATED_BELOW_128_BITS");
    assert_eq!(findings.len(), 1);
    assert!(
        findings[0].description.contains("bytes"),
        "description should use 'bytes' unit for raw digest, got: {}",
        findings[0].description
    );
}

// --- Python: HASH_TRUNCATION_VIA_CALL (2-pass) ---

#[test]
fn test_primitive_hash_trunc_via_call_python() {
    let source = r#"
import hashlib

def get_str_hash(s, length):
    h = hashlib.sha256(s.encode())
    return h.hexdigest()[:length]

def cache_key(name):
    return get_str_hash(name, 12)
"#;
    let result = run_primitive(
        source,
        "src/cache.py",
        Language::Python,
        BTreeSet::from([9]),
    );
    let findings = findings_for_rule(&result, "HASH_TRUNCATION_VIA_CALL");
    assert_eq!(
        findings.len(),
        1,
        "expected one HASH_TRUNCATION_VIA_CALL finding"
    );
    assert!(
        findings[0].description.contains("get_str_hash"),
        "description should reference callee, got: {}",
        findings[0].description
    );
    assert_eq!(
        findings[0].related_lines.len(),
        1,
        "related_lines should point to callee def"
    );
}

// --- Python: WEAK_HASH_FOR_IDENTITY ---

#[test]
fn test_primitive_weak_hash_for_identity_python() {
    let source = r#"
import hashlib

def make_cache_key(data):
    cache_key = hashlib.md5(data).hexdigest()
    return cache_key
"#;
    let result = run_primitive(
        source,
        "src/cache.py",
        Language::Python,
        BTreeSet::from([5]),
    );
    let findings = findings_for_rule(&result, "WEAK_HASH_FOR_IDENTITY");
    assert_eq!(findings.len(), 1);
    assert!(
        findings[0].description.contains("MD5"),
        "should name MD5, got: {}",
        findings[0].description
    );
}

#[test]
fn test_primitive_weak_hash_for_checksum_no_finding_python() {
    // 'checksum' is not an identity-shaped name.
    let source = r#"
import hashlib

def integrity(data):
    checksum = hashlib.md5(data).hexdigest()
    return checksum
"#;
    let result = run_primitive(source, "src/util.py", Language::Python, BTreeSet::from([5]));
    let findings = findings_for_rule(&result, "WEAK_HASH_FOR_IDENTITY");
    assert!(
        findings.is_empty(),
        "non-identity name should not fire WEAK_HASH_FOR_IDENTITY"
    );
}

// --- Python: SHELL_TRUE_WITH_INTERPOLATION ---

#[test]
fn test_primitive_shell_true_with_fstring_python() {
    let source = r#"
import subprocess

def copy(src, dst):
    subprocess.run(f"cp {src} {dst}", shell=True)
"#;
    let result = run_primitive(
        source,
        "src/runner.py",
        Language::Python,
        BTreeSet::from([5]),
    );
    let findings = findings_for_rule(&result, "SHELL_TRUE_WITH_INTERPOLATION");
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].severity, "concern");
}

#[test]
fn test_primitive_shell_true_no_interp_no_finding_python() {
    let source = r#"
import subprocess

def list_dir():
    subprocess.run("ls -la", shell=True)
"#;
    let result = run_primitive(
        source,
        "src/runner.py",
        Language::Python,
        BTreeSet::from([5]),
    );
    let findings = findings_for_rule(&result, "SHELL_TRUE_WITH_INTERPOLATION");
    assert!(
        findings.is_empty(),
        "literal command without interp should not fire"
    );
}

// --- Python: CERT_VALIDATION_DISABLED ---

#[test]
fn test_primitive_cert_validation_disabled_python() {
    let source = r#"
import requests

def fetch(url):
    return requests.get(url, verify=False)
"#;
    let result = run_primitive(
        source,
        "src/client.py",
        Language::Python,
        BTreeSet::from([5]),
    );
    let findings = findings_for_rule(&result, "CERT_VALIDATION_DISABLED");
    assert_eq!(findings.len(), 1);
    assert!(findings[0].description.contains("verify=False"));
}

// --- Python: HARDCODED_SECRET (positive + inline negative) ---

#[test]
fn test_primitive_hardcoded_secret_python() {
    // Two-statement file: the positive case fires, the placeholder case does not.
    let source = r#"
API_KEY = "sk-real-looking-1234"
TOKEN = "changeme"
"#;
    let result = run_primitive(
        source,
        "src/config.py",
        Language::Python,
        BTreeSet::from([2]),
    );
    let findings = findings_for_rule(&result, "HARDCODED_SECRET");
    assert_eq!(findings.len(), 1, "should fire on real secret only");
    assert!(
        findings[0].description.contains("API_KEY"),
        "should name API_KEY, got: {}",
        findings[0].description
    );
}

// --- C: CERT_VALIDATION_DISABLED (5 tests, exercising marker list + severity ladder) ---

#[test]
fn test_primitive_cert_validation_verifypeer_zero_c() {
    let source = r#"
#include <curl/curl.h>

void fetch(CURL *curl) {
    curl_easy_setopt(curl, CURLOPT_SSL_VERIFYPEER, 0L);
}
"#;
    let result = run_primitive(source, "src/fetch.c", Language::C, BTreeSet::from([5]));
    let findings = findings_for_rule(&result, "CERT_VALIDATION_DISABLED");
    assert_eq!(findings.len(), 1);
    assert_eq!(findings[0].severity, "concern");
    assert!(findings[0].description.contains("CURLOPT_SSL_VERIFYPEER"));
}

#[test]
fn test_primitive_cert_validation_verifyhost_zero_c() {
    let source = r#"
#include <curl/curl.h>

void fetch(CURL *curl) {
    curl_easy_setopt(curl, CURLOPT_SSL_VERIFYHOST, 0);
}
"#;
    let result = run_primitive(source, "src/fetch.c", Language::C, BTreeSet::from([5]));
    let findings = findings_for_rule(&result, "CERT_VALIDATION_DISABLED");
    assert_eq!(findings.len(), 1);
    assert!(findings[0].description.contains("CURLOPT_SSL_VERIFYHOST"));
}

#[test]
fn test_primitive_cert_validation_in_dirty_function_concern_c() {
    // Marker is on a non-diff line of a function whose other line IS in the diff.
    let source = r#"
#include <curl/curl.h>

void fetch(CURL *curl) {
    int x = 1;
    curl_easy_setopt(curl, CURLOPT_SSL_VERIFYPEER, 0L);
}
"#;
    // diff_lines = {5} (touches `int x = 1`); marker is on line 6 inside same function
    let result = run_primitive(source, "src/fetch.c", Language::C, BTreeSet::from([5]));
    let findings = findings_for_rule(&result, "CERT_VALIDATION_DISABLED");
    assert_eq!(findings.len(), 1);
    assert_eq!(
        findings[0].severity, "concern",
        "marker in dirty function should be 'concern' even when not on diff line"
    );
}

#[test]
fn test_primitive_cert_validation_outside_dirty_function_suggestion_c() {
    // Marker is in `other_func`; diff touches `dirty_func` only.
    let source = r#"
#include <curl/curl.h>

void dirty_func(int x) {
    int y = x + 1;
}

void other_func(CURL *curl) {
    curl_easy_setopt(curl, CURLOPT_SSL_VERIFYPEER, 0L);
}
"#;
    let result = run_primitive(source, "src/fetch.c", Language::C, BTreeSet::from([5]));
    let findings = findings_for_rule(&result, "CERT_VALIDATION_DISABLED");
    assert_eq!(findings.len(), 1);
    assert_eq!(
        findings[0].severity, "suggestion",
        "marker outside dirty function should be 'suggestion'"
    );
}

#[test]
fn test_primitive_no_finding_proper_curl_validation_c() {
    let source = r#"
#include <curl/curl.h>

void fetch(CURL *curl) {
    curl_easy_setopt(curl, CURLOPT_SSL_VERIFYPEER, 1L);
}
"#;
    let result = run_primitive(source, "src/fetch.c", Language::C, BTreeSet::from([5]));
    let findings = findings_for_rule(&result, "CERT_VALIDATION_DISABLED");
    assert!(
        findings.is_empty(),
        "CURLOPT_SSL_VERIFYPEER, 1L (proper validation) should not fire"
    );
}
