//! Task 4 evidence delivery through the CLI SARIF projection. Task 5 owns the
//! `--min-confidence` and `--resolution` flag cases.

use assert_cmd::Command;
use serde_json::{json, Value};
use std::fs;
use tempfile::TempDir;

fn output(
    files: &[(&str, &str)],
    diff_file: &str,
    diff_line: usize,
    algorithm: &str,
    targets: bool,
) -> Value {
    let temp = TempDir::new().unwrap();
    for (path, source) in files {
        let full = temp.path().join(path);
        fs::create_dir_all(full.parent().unwrap()).unwrap();
        fs::write(full, source).unwrap();
    }
    let diff = temp.path().join("diff.json");
    let diff_files: Vec<_> = files
        .iter()
        .map(|(path, _)| {
            json!({
                "file_path": path,
                "modify_type": "Modified",
                "diff_lines": if *path == diff_file { vec![diff_line] } else { vec![] }
            })
        })
        .collect();
    fs::write(
        &diff,
        serde_json::to_vec(&json!({ "files": diff_files })).unwrap(),
    )
    .unwrap();
    let mut command = Command::cargo_bin("prism").unwrap();
    if targets {
        command.args([
            "targets",
            "--repo",
            temp.path().to_str().unwrap(),
            "--diff",
            diff.to_str().unwrap(),
            "--algorithm",
            algorithm,
        ]);
    } else {
        command.args([
            "--repo",
            temp.path().to_str().unwrap(),
            "--diff",
            diff.to_str().unwrap(),
            "--algorithm",
            algorithm,
            "--format",
            "sarif",
        ]);
    }
    let output = command.output().unwrap();
    assert!(
        output.status.success(),
        "status={} stderr={}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );
    serde_json::from_slice(&output.stdout).unwrap()
}

fn sarif(files: &[(&str, &str)], diff_file: &str, diff_line: usize, algorithm: &str) -> Value {
    output(files, diff_file, diff_line, algorithm, false)
}

fn targets(files: &[(&str, &str)], diff_file: &str, diff_line: usize, algorithm: &str) -> Value {
    output(files, diff_file, diff_line, algorithm, true)
}

fn assert_grade(
    doc: &Value,
    algorithm: &str,
    category: &str,
    result_line: Option<usize>,
    message_needle: Option<&str>,
    confidence: &str,
) {
    let result = doc["runs"][0]["results"]
        .as_array()
        .unwrap()
        .iter()
        .find(|result| {
            result["properties"]["algorithm"] == algorithm
                && result["properties"]["category"] == category
                && result_line.is_none_or(|line| {
                    result["locations"][0]["physicalLocation"]["region"]["startLine"] == line
                })
                && message_needle.is_none_or(|needle| {
                    result["message"]["text"]
                        .as_str()
                        .is_some_and(|message| message.contains(needle))
                })
        })
        .unwrap_or_else(|| panic!("missing {algorithm}/{category}: {doc:#}"));
    assert_eq!(result["properties"]["confidence"], confidence, "{doc:#}");
    assert_eq!(
        result["properties"]["tier"],
        if confidence == "exact" {
            "asserted"
        } else {
            "candidate"
        },
        "{doc:#}"
    );
}

fn assert_target_grade(
    doc: &Value,
    algorithm: &str,
    category: &str,
    result_line: Option<usize>,
    message_needle: Option<&str>,
    confidence: &str,
) {
    let target = doc["targets"]
        .as_array()
        .unwrap()
        .iter()
        .find(|target| {
            target["source_algorithm"] == algorithm
                && target["category"] == category
                && result_line.is_none_or(|line| target["site"]["line"] == line)
                && message_needle.is_none_or(|needle| {
                    target["description"]
                        .as_str()
                        .is_some_and(|message| message.contains(needle))
                })
        })
        .unwrap_or_else(|| panic!("missing {algorithm}/{category}: {doc:#}"));
    assert_eq!(target["confidence"], confidence, "{doc:#}");
    assert_eq!(
        target["tier"],
        if confidence == "exact" {
            "asserted"
        } else {
            "candidate"
        },
        "{doc:#}"
    );
}

fn assert_delivery(
    files: &[(&str, &str)],
    diff_file: &str,
    diff_line: usize,
    algorithm: &str,
    category: &str,
    selector: (Option<usize>, Option<&str>),
    confidence: &str,
) {
    let (result_line, message_needle) = selector;
    assert_grade(
        &sarif(files, diff_file, diff_line, algorithm),
        algorithm,
        category,
        result_line,
        message_needle,
        confidence,
    );
    assert_target_grade(
        &targets(files, diff_file, diff_line, algorithm),
        algorithm,
        category,
        result_line,
        message_needle,
        confidence,
    );
}

fn flow_source(killed: bool) -> (&'static str, usize) {
    if killed {
        (
            "def f(request):\n    query = request.GET['q']\n    query = clean(query)\n    cursor.execute(query)\n",
            4,
        )
    } else {
        (
            "def f(request):\n    query = request.GET['q']\n    cursor.execute(query)\n",
            3,
        )
    }
}

fn call_files(name_only: bool) -> Vec<(&'static str, &'static str)> {
    if name_only {
        vec![
            (
                "src/api.c",
                "\n#include <stdlib.h>\n\nint process(int *data, int len) {\n    for (int i = 0; i < len; i++) {\n        data[i] *= 2;\n    }\n    return 0;\n}\n",
            ),
            (
                "src/driver.c",
                "\n#include \"api.h\"\n\nstruct operations {\n    int (*process)(int *data, int len);\n};\n\nint run_pipeline(struct operations *ops, int *data, int len) {\n    int ret = ops->process(data, len);\n    consume(ret);\n    return ret;\n}\n",
            ),
        ]
    } else {
        vec![
            (
                "src/device.c",
                "\nint open_device(const char *path) {\n    int fd = open(path, 0);\n    return fd;\n}\n",
            ),
            (
                "src/init.c",
                "\nvoid init_system(void) {\n    int fd = open_device(\"/dev/eth0\");\n    use_fd(fd);\n}\n",
            ),
        ]
    }
}

#[test]
fn evidence_delivery_provenance_reaches_sarif_with_all_three_grades() {
    for (killed, expected) in [(false, "exact"), (true, "nameonly")] {
        let (source, line) = flow_source(killed);
        assert_delivery(
            &[("p.py", source)],
            "p.py",
            line,
            "provenance",
            "untrusted_origin",
            (Some(line), Some("variable 'query'")),
            expected,
        );
    }
    let bridge = "def source():\n    query = request.GET['q']\n\ndef consume():\n    cursor.execute(query)\n";
    assert_delivery(
        &[("p.py", bridge)],
        "p.py",
        5,
        "provenance",
        "untrusted_origin",
        (Some(5), Some("variable 'query'")),
        "unlabeled",
    );
}

#[test]
fn evidence_delivery_taint_reaches_sarif_with_exact_and_nameonly() {
    for (killed, expected) in [(false, "exact"), (true, "nameonly")] {
        let (source, line) = flow_source(killed);
        assert_delivery(
            &[("t.py", source)],
            "t.py",
            2,
            "taint",
            "taint_sink",
            (Some(line), None),
            expected,
        );
    }
}

#[test]
fn evidence_delivery_echo_reaches_sarif_with_exact_and_nameonly() {
    for (name_only, expected) in [(false, "exact"), (true, "nameonly")] {
        let (api, line) = if name_only {
            ("src/api.c", 8)
        } else {
            ("src/device.c", 4)
        };
        assert_delivery(
            &call_files(name_only),
            api,
            line,
            "echo",
            "missing_error_handling",
            (None, None),
            expected,
        );
    }
}

#[test]
fn evidence_delivery_membrane_reaches_sarif_with_exact_and_nameonly() {
    for (name_only, expected) in [(false, "exact"), (true, "nameonly")] {
        let (api, line) = if name_only {
            ("src/api.c", 8)
        } else {
            ("src/device.c", 4)
        };
        assert_delivery(
            &call_files(name_only),
            api,
            line,
            "membrane",
            "unprotected_caller",
            (None, None),
            expected,
        );
    }
}
