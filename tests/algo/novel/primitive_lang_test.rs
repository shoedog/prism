#[path = "../../common/mod.rs"]
mod common;
use common::*;

#[test]
fn test_primitive_cert_validation_disabled_reject_unauthorized_js() {
    let source = r#"
const https = require('https');

function fetch(url) {
    return https.request(url, { rejectUnauthorized: false });
}
"#;
    let path = "src/client.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([5]),
        }],
    };
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::PrimitiveSlice);
    let result = algorithms::run_slicing_compat(&files, &diff, &config, None).unwrap();
    let findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("CERT_VALIDATION_DISABLED"))
        .collect();
    assert_eq!(findings.len(), 1);
    assert!(findings[0]
        .description
        .contains("rejectUnauthorized: false"));
}

#[test]
fn test_primitive_hardcoded_secret_object_field_js() {
    // The HARDCODED_SECRET rule's LHS check accepts `obj.field = "..."` form
    // (it rsplits on '.' and validates the rightmost segment as an identifier).
    let source = r#"
class Config {
    constructor() {
        this.apiKey = "sk-real-looking-1234";
    }
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
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::PrimitiveSlice);
    let result = algorithms::run_slicing_compat(&files, &diff, &config, None).unwrap();
    let findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("HARDCODED_SECRET"))
        .collect();
    assert_eq!(findings.len(), 1);
    assert!(
        findings[0].description.contains("apiKey"),
        "should name apiKey, got: {}",
        findings[0].description
    );
}

#[test]
fn test_primitive_cert_validation_disabled_insecure_skip_verify_go() {
    let source = r#"
package main

import "crypto/tls"

func tlsConfig() *tls.Config {
    return &tls.Config{InsecureSkipVerify: true}
}
"#;
    let path = "src/tls.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([7]),
        }],
    };
    let config = SliceConfig::default().with_algorithm(SlicingAlgorithm::PrimitiveSlice);
    let result = algorithms::run_slicing_compat(&files, &diff, &config, None).unwrap();
    let findings: Vec<_> = result
        .findings
        .iter()
        .filter(|f| f.category.as_deref() == Some("CERT_VALIDATION_DISABLED"))
        .collect();
    assert_eq!(findings.len(), 1);
    assert!(findings[0].description.contains("InsecureSkipVerify: true"));
}
