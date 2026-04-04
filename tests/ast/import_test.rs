#[path = "../common/mod.rs"]
mod common;
use common::*;

// --- Python import extraction ---

#[test]
fn test_python_import_basic() {
    let source = r#"
import utils
import os.path

result = utils.process(data)
"#;
    let parsed = ParsedFile::parse("src/main.py", source, Language::Python).unwrap();
    let imports = parsed.extract_imports();

    assert_eq!(imports.get("utils"), Some(&"utils".to_string()));
    assert_eq!(imports.get("path"), Some(&"os.path".to_string()));
}

#[test]
fn test_python_import_alias() {
    let source = r#"
import utils as u
import numpy as np
"#;
    let parsed = ParsedFile::parse("src/main.py", source, Language::Python).unwrap();
    let imports = parsed.extract_imports();

    assert_eq!(imports.get("u"), Some(&"utils".to_string()));
    assert_eq!(imports.get("np"), Some(&"numpy".to_string()));
}

#[test]
fn test_python_from_import() {
    let source = r#"
from utils import process
from os.path import join
"#;
    let parsed = ParsedFile::parse("src/main.py", source, Language::Python).unwrap();
    let imports = parsed.extract_imports();

    assert_eq!(imports.get("process"), Some(&"utils".to_string()));
    assert_eq!(imports.get("join"), Some(&"os.path".to_string()));
}

#[test]
fn test_python_from_import_alias() {
    let source = r#"
from utils import process as proc
"#;
    let parsed = ParsedFile::parse("src/main.py", source, Language::Python).unwrap();
    let imports = parsed.extract_imports();

    assert_eq!(imports.get("proc"), Some(&"utils".to_string()));
}

// --- JavaScript import extraction ---

#[test]
fn test_js_es6_import_default() {
    let source = r#"
import utils from './utils';

utils.process(data);
"#;
    let parsed = ParsedFile::parse("src/app.js", source, Language::JavaScript).unwrap();
    let imports = parsed.extract_imports();

    assert_eq!(imports.get("utils"), Some(&"./utils".to_string()));
}

#[test]
fn test_js_es6_import_named() {
    let source = r#"
import { process, validate } from './utils';
"#;
    let parsed = ParsedFile::parse("src/app.js", source, Language::JavaScript).unwrap();
    let imports = parsed.extract_imports();

    assert_eq!(imports.get("process"), Some(&"./utils".to_string()));
    assert_eq!(imports.get("validate"), Some(&"./utils".to_string()));
}

#[test]
fn test_js_es6_import_namespace() {
    let source = r#"
import * as utils from './utils';
"#;
    let parsed = ParsedFile::parse("src/app.js", source, Language::JavaScript).unwrap();
    let imports = parsed.extract_imports();

    assert_eq!(imports.get("utils"), Some(&"./utils".to_string()));
}

#[test]
fn test_js_commonjs_require() {
    let source = r#"
const utils = require('./utils');
const { process } = require('./helpers');
"#;
    let parsed = ParsedFile::parse("src/app.js", source, Language::JavaScript).unwrap();
    let imports = parsed.extract_imports();

    assert_eq!(imports.get("utils"), Some(&"./utils".to_string()));
    assert_eq!(imports.get("process"), Some(&"./helpers".to_string()));
}

// --- Go import extraction ---

#[test]
fn test_go_import_single() {
    let source = r#"
package main

import "fmt"

func main() {
    fmt.Println("hello")
}
"#;
    let parsed = ParsedFile::parse("main.go", source, Language::Go).unwrap();
    let imports = parsed.extract_imports();

    assert_eq!(imports.get("fmt"), Some(&"fmt".to_string()));
}

#[test]
fn test_go_import_multi() {
    let source = r#"
package main

import (
    "fmt"
    "os/exec"
    u "mypackage/utils"
)
"#;
    let parsed = ParsedFile::parse("main.go", source, Language::Go).unwrap();
    let imports = parsed.extract_imports();

    assert_eq!(imports.get("fmt"), Some(&"fmt".to_string()));
    assert_eq!(imports.get("exec"), Some(&"os/exec".to_string()));
    assert_eq!(imports.get("u"), Some(&"mypackage/utils".to_string()));
}

// --- Qualifier extraction ---

#[test]
fn test_python_call_qualifier() {
    let source = r#"
import utils

def main():
    result = utils.process(data)
"#;
    let parsed = ParsedFile::parse("src/main.py", source, Language::Python).unwrap();
    let func = parsed.all_functions().into_iter().next().unwrap();
    let all_lines: BTreeSet<usize> = (1..=6).collect();
    let calls = parsed.function_calls_on_lines_with_qualifier(&func, &all_lines);

    assert_eq!(calls.len(), 1);
    let (name, _line, qualifier) = &calls[0];
    assert_eq!(name, "process");
    assert_eq!(qualifier.as_deref(), Some("utils"));
}

#[test]
fn test_js_call_qualifier() {
    let source = r#"
import utils from './utils';

function main() {
    let result = utils.process(data);
}
"#;
    let parsed = ParsedFile::parse("src/app.js", source, Language::JavaScript).unwrap();
    let func = parsed.all_functions().into_iter().next().unwrap();
    let all_lines: BTreeSet<usize> = (1..=7).collect();
    let calls = parsed.function_calls_on_lines_with_qualifier(&func, &all_lines);

    assert_eq!(calls.len(), 1);
    let (name, _line, qualifier) = &calls[0];
    assert_eq!(name, "process");
    assert_eq!(qualifier.as_deref(), Some("utils"));
}

#[test]
fn test_go_call_qualifier() {
    let source = r#"
package main

import "fmt"

func main() {
    fmt.Println("hello")
}
"#;
    let parsed = ParsedFile::parse("main.go", source, Language::Go).unwrap();
    let func = parsed.all_functions().into_iter().next().unwrap();
    let all_lines: BTreeSet<usize> = (1..=8).collect();
    let calls = parsed.function_calls_on_lines_with_qualifier(&func, &all_lines);

    assert_eq!(calls.len(), 1);
    let (name, _line, qualifier) = &calls[0];
    assert_eq!(name, "Println");
    assert_eq!(qualifier.as_deref(), Some("fmt"));
}

#[test]
fn test_unqualified_call_no_qualifier() {
    let source = r#"
def main():
    result = process(data)
"#;
    let parsed = ParsedFile::parse("src/main.py", source, Language::Python).unwrap();
    let func = parsed.all_functions().into_iter().next().unwrap();
    let all_lines: BTreeSet<usize> = (1..=4).collect();
    let calls = parsed.function_calls_on_lines_with_qualifier(&func, &all_lines);

    assert_eq!(calls.len(), 1);
    let (name, _line, qualifier) = &calls[0];
    assert_eq!(name, "process");
    assert!(qualifier.is_none());
}

// --- End-to-end: import-aware call graph resolution ---

#[test]
fn test_call_graph_python_import_resolution() {
    // caller.py imports utils and calls utils.process()
    // utils.py defines process()
    let caller_source = r#"
import utils

def handler(request):
    user_data = request.input
    result = utils.process(user_data)
    return result
"#;

    let callee_source = r#"
def process(data):
    query = "SELECT * FROM users WHERE name = " + data
    return query
"#;

    let mut files = BTreeMap::new();
    let caller = ParsedFile::parse("src/caller.py", caller_source, Language::Python).unwrap();
    let callee = ParsedFile::parse("src/utils.py", callee_source, Language::Python).unwrap();
    files.insert("src/caller.py".to_string(), caller);
    files.insert("src/utils.py".to_string(), callee);

    let cg = CallGraph::build(&files);

    // The call graph should resolve utils.process() → process in utils.py
    let callees = cg.resolve_callees_qualified("process", "src/caller.py", Some("utils"));
    assert_eq!(callees.len(), 1);
    assert_eq!(callees[0].file, "src/utils.py");
    assert_eq!(callees[0].name, "process");
}

#[test]
fn test_call_graph_js_import_resolution() {
    let caller_source = r#"
import helpers from './helpers';

function main() {
    let result = helpers.validate(input);
}
"#;

    let callee_source = r#"
function validate(data) {
    return data.length > 0;
}
"#;

    let mut files = BTreeMap::new();
    let caller = ParsedFile::parse("src/app.js", caller_source, Language::JavaScript).unwrap();
    let callee = ParsedFile::parse("src/helpers.js", callee_source, Language::JavaScript).unwrap();
    files.insert("src/app.js".to_string(), caller);
    files.insert("src/helpers.js".to_string(), callee);

    let cg = CallGraph::build(&files);

    let callees = cg.resolve_callees_qualified("validate", "src/app.js", Some("helpers"));
    assert_eq!(callees.len(), 1);
    assert_eq!(callees[0].file, "src/helpers.js");
    assert_eq!(callees[0].name, "validate");
}

#[test]
fn test_taint_interprocedural_python_import() {
    // Taint should flow through import-qualified calls: utils.process()
    let caller_source = r#"
import utils

def handler(request):
    user_data = request.input
    result = utils.process(user_data)
    return result
"#;

    let callee_source = r#"
def process(data):
    query = "SELECT * FROM users WHERE name = " + data
    cursor.execute(query)
    return query
"#;

    let mut files = BTreeMap::new();
    let caller = ParsedFile::parse("src/caller.py", caller_source, Language::Python).unwrap();
    let callee = ParsedFile::parse("src/utils.py", callee_source, Language::Python).unwrap();
    files.insert("src/caller.py".to_string(), caller);
    files.insert("src/utils.py".to_string(), callee);

    // Diff touches caller line 5: user_data = request.input
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: "src/caller.py".to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([5]),
        }],
    };

    let result = algorithms::run_slicing_compat(
        &files,
        &diff,
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap();

    // Taint should flow: request.input → user_data → utils.process(user_data) →
    // data parameter → query → cursor.execute(query)
    assert!(
        !result.findings.is_empty(),
        "Import-qualified interprocedural taint should detect SQL injection"
    );
}
