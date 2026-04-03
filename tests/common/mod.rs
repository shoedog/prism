// Shared test helpers and fixture generators used across test files.

#![allow(dead_code)]

pub use prism::access_path::AccessPath;
pub use prism::algorithms;
pub use prism::ast::ParsedFile;
pub use prism::call_graph::CallGraph;
pub use prism::cpg::CpgContext;
pub use prism::data_flow::DataFlowGraph;
pub use prism::diff::{DiffInfo, DiffInput, ModifyType};
pub use prism::languages::Language;
pub use prism::output;
pub use prism::slice::{SliceConfig, SlicingAlgorithm};
pub use std::collections::{BTreeMap, BTreeSet};
pub use std::path::Path;
pub use tempfile::TempDir;

pub fn make_python_test() -> (
    BTreeMap<String, ParsedFile>,
    BTreeMap<String, String>,
    DiffInput,
) {
    let source = r#"
import os

GLOBAL_VAR = 42

def calculate(x, y):
    total = x + y
    if total > 10:
        result = total * 2
        print(result)
    else:
        result = total
    return result

def helper(val):
    return val + GLOBAL_VAR

def process(data):
    filtered = [d for d in data if d > 0]
    total = calculate(filtered[0], filtered[1])
    extra = helper(total)
    return extra
"#;

    let path = "src/calc.py";
    let parsed = ParsedFile::parse(path, source, Language::Python).unwrap();
    let mut files = BTreeMap::new();
    let mut sources = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    sources.insert(path.to_string(), source.to_string());

    // Diff: lines 7 (total = x + y) and 9 (result = total * 2) were changed
    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([7, 9]),
        }],
    };

    (files, sources, diff)
}

pub fn make_javascript_test() -> (
    BTreeMap<String, ParsedFile>,
    BTreeMap<String, String>,
    DiffInput,
) {
    let source = r#"
function fetchData(url, options) {
    const headers = options.headers || {};
    const timeout = options.timeout || 5000;

    if (timeout > 10000) {
        throw new Error("Timeout too long");
    }

    const response = fetch(url, { headers, timeout });
    const data = response.json();

    if (data.error) {
        console.error(data.error);
        return null;
    }

    return data.result;
}

function processItems(items) {
    const results = [];
    for (const item of items) {
        const processed = fetchData(item.url, item.options);
        if (processed) {
            results.push(processed);
        }
    }
    return results;
}
"#;

    let path = "src/api.js";
    let parsed = ParsedFile::parse(path, source, Language::JavaScript).unwrap();
    let mut files = BTreeMap::new();
    let mut sources = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    sources.insert(path.to_string(), source.to_string());

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([10, 11]),
        }],
    };

    (files, sources, diff)
}

pub fn make_go_test() -> (
    BTreeMap<String, ParsedFile>,
    BTreeMap<String, String>,
    DiffInput,
) {
    let source = r#"package main

import "fmt"

func sum(numbers []int) int {
	total := 0
	for _, n := range numbers {
		if n > 0 {
			total += n
		}
	}
	return total
}

func main() {
	data := []int{1, -2, 3, -4, 5}
	result := sum(data)
	fmt.Println(result)
}
"#;

    let path = "main.go";
    let parsed = ParsedFile::parse(path, source, Language::Go).unwrap();
    let mut files = BTreeMap::new();
    let mut sources = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    sources.insert(path.to_string(), source.to_string());

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([9]),
        }],
    };

    (files, sources, diff)
}

pub fn make_java_test() -> (
    BTreeMap<String, ParsedFile>,
    BTreeMap<String, String>,
    DiffInput,
) {
    let source = r#"public class Calculator {
    private int accumulator = 0;

    public int add(int a, int b) {
        int sum = a + b;
        accumulator += sum;
        return sum;
    }

    public int getAccumulator() {
        return accumulator;
    }

    public void reset() {
        accumulator = 0;
    }
}
"#;

    let path = "Calculator.java";
    let parsed = ParsedFile::parse(path, source, Language::Java).unwrap();
    let mut files = BTreeMap::new();
    let mut sources = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    sources.insert(path.to_string(), source.to_string());

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([5, 6]),
        }],
    };

    (files, sources, diff)
}

pub fn make_typescript_test() -> (
    BTreeMap<String, ParsedFile>,
    BTreeMap<String, String>,
    DiffInput,
) {
    let source = r#"
interface Config {
    baseUrl: string;
    retries: number;
}

function createClient(config: Config) {
    const url = config.baseUrl;
    const maxRetries = config.retries;

    async function request(path: string): Promise<any> {
        let attempts = 0;
        while (attempts < maxRetries) {
            attempts += 1;
            try {
                const response = await fetch(url + path);
                return response.json();
            } catch (e) {
                if (attempts >= maxRetries) throw e;
            }
        }
    }

    return { request };
}
"#;

    let path = "src/client.ts";
    let parsed = ParsedFile::parse(path, source, Language::TypeScript).unwrap();
    let mut files = BTreeMap::new();
    let mut sources = BTreeMap::new();
    files.insert(path.to_string(), parsed);
    sources.insert(path.to_string(), source.to_string());

    let diff = DiffInput {
        files: vec![DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: BTreeSet::from([14, 16, 17]),
        }],
    };

    (files, sources, diff)
}
