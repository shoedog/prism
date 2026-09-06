//! Task 4 evidence delivery and Task 6 DFG telemetry CLI cases.

use assert_cmd::Command;
use serde_json::{json, Value};
use std::fs;
use std::process::Output;
use tempfile::TempDir;

pub(super) fn command_output(
    files: &[(&str, &str)],
    diff_file: &str,
    diff_line: usize,
    algorithm: &str,
    format: Option<&str>,
    targets: bool,
    extra: &[&str],
) -> Output {
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
        ]);
        if let Some(format) = format {
            command.args(["--format", format]);
        }
    }
    command.args(extra).output().unwrap()
}

fn output(
    files: &[(&str, &str)],
    diff_file: &str,
    diff_line: usize,
    algorithm: &str,
    targets: bool,
) -> Value {
    let output = command_output(
        files,
        diff_file,
        diff_line,
        algorithm,
        (!targets).then_some("sarif"),
        targets,
        &["--resolution", "scoped"],
    );
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

pub(super) fn call_files(name_only: bool) -> Vec<(&'static str, &'static str)> {
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

fn dfg_stats_output(repo: &std::path::Path, edges: bool) -> Output {
    let mut command = Command::cargo_bin("prism").unwrap();
    command.args(["nav", "--no-cache", "dfg-stats", "--repo"]);
    command.arg(repo);
    if edges {
        command.arg("--edges");
    }
    command.output().unwrap()
}

fn dfg_fixture(language: &str, case: &str) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("eval/fixtures")
        .join(language)
        .join(case)
}

const DFG_CLASS_COUNTERS: [&str; 6] = [
    "dfg_label_exact",
    "dfg_label_nameonly_killed",
    "dfg_label_nameonly_sameline",
    "dfg_label_nameonly_cfg_incomplete",
    "dfg_label_nameonly_alias_unstable",
    "dfg_label_nameonly_call",
];

#[test]
fn dfg_stats_empty_repo_emits_all_zero_counters() {
    let repo = tempfile::tempdir().unwrap();
    let output = dfg_stats_output(repo.path(), false);
    assert!(
        output.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    let actual: Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(
        actual,
        json!({
            "dfg_label_exact": 0,
            "dfg_label_loop_carried": 0,
            "dfg_label_nameonly_killed": 0,
            "dfg_label_nameonly_sameline": 0,
            "dfg_label_nameonly_cfg_incomplete": 0,
            "dfg_label_nameonly_alias_unstable": 0,
            "dfg_label_nameonly_call": 0,
            "dfg_rd_functions_over_cap": 0,
            "dfg_rd_functions_without_cfg": 0,
        })
    );
}

#[test]
fn dfg_stats_each_label_shape_increments_its_counter_and_matches_call_stats() {
    let cases = [
        ("python", "dfg_reaching_killed_def", "dfg_label_exact"),
        (
            "python",
            "dfg_reaching_killed_def",
            "dfg_label_nameonly_killed",
        ),
        (
            "python",
            "dfg_reaching_loop_carried",
            "dfg_label_loop_carried",
        ),
        (
            "python",
            "dfg_reaching_cfg_gap",
            "dfg_label_nameonly_cfg_incomplete",
        ),
        (
            "python",
            "dfg_reaching_alias_unstable",
            "dfg_label_nameonly_alias_unstable",
        ),
        (
            "python",
            "dfg_reaching_same_line",
            "dfg_label_nameonly_sameline",
        ),
        (
            "python",
            "dfg_reaching_interproc_nameonly",
            "dfg_label_nameonly_call",
        ),
    ];

    for (language, case, counter) in cases {
        let repo = dfg_fixture(language, case);
        let stats_output = dfg_stats_output(&repo, false);
        assert!(
            stats_output.status.success(),
            "{case}: {}",
            String::from_utf8_lossy(&stats_output.stderr)
        );
        let stats: Value = serde_json::from_slice(&stats_output.stdout).unwrap();
        assert!(stats[counter].as_u64().unwrap() > 0, "{case}: {stats:#}");
        assert!(
            stats["dfg_label_loop_carried"].as_u64().unwrap()
                <= stats["dfg_label_exact"].as_u64().unwrap(),
            "{case}: {stats:#}"
        );

        let call_stats = Command::cargo_bin("prism")
            .unwrap()
            .args(["nav", "--no-cache", "call-stats", "--repo"])
            .arg(&repo)
            .output()
            .unwrap();
        assert!(call_stats.status.success(), "{case}: {call_stats:?}");
        let call_stats: Value = serde_json::from_slice(&call_stats.stdout).unwrap();
        assert_eq!(stats, call_stats["dfg_labels"], "{case}");

        let edges_output = dfg_stats_output(&repo, true);
        assert!(edges_output.status.success(), "{case}: {edges_output:?}");
        let labeled_edges = edges_output
            .stdout
            .split(|byte| *byte == b'\n')
            .filter(|line| !line.is_empty())
            .count() as u64;
        let counter_sum: u64 = DFG_CLASS_COUNTERS
            .iter()
            .map(|field| stats[*field].as_u64().unwrap())
            .sum();
        assert_eq!(counter_sum, labeled_edges, "{case}: {stats:#}");
    }
}

#[test]
fn dfg_stats_loop_carried_fixture_preserves_partition_identity() {
    let repo = dfg_fixture("python", "dfg_reaching_loop_carried");
    let stats_output = dfg_stats_output(&repo, false);
    assert!(stats_output.status.success(), "{stats_output:?}");
    let stats: Value = serde_json::from_slice(&stats_output.stdout).unwrap();
    let exact = stats["dfg_label_exact"].as_u64().unwrap();
    let loop_carried = stats["dfg_label_loop_carried"].as_u64().unwrap();
    assert!(loop_carried > 0, "{stats:#}");
    assert!(loop_carried <= exact, "{stats:#}");

    let edges_output = dfg_stats_output(&repo, true);
    assert!(edges_output.status.success(), "{edges_output:?}");
    let labeled_edges = edges_output
        .stdout
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .count() as u64;
    let partition_total: u64 = DFG_CLASS_COUNTERS
        .iter()
        .map(|field| stats[*field].as_u64().unwrap())
        .sum();
    assert_eq!(partition_total, labeled_edges, "{stats:#}");
}

#[test]
fn dfg_stats_edges_are_deterministic_sorted_and_use_the_exact_wire_vocabulary() {
    let repo = dfg_fixture("python", "dfg_reaching_killed_def");
    let first = dfg_stats_output(&repo, true);
    let second = dfg_stats_output(&repo, true);
    assert!(first.status.success(), "{first:?}");
    assert!(second.status.success(), "{second:?}");
    assert_eq!(first.stdout, second.stdout);

    let rows: Vec<Value> = first
        .stdout
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .map(|line| serde_json::from_slice(line).unwrap())
        .collect();
    assert!(!rows.is_empty());
    let sort_keys: Vec<_> = rows
        .iter()
        .map(|row| serde_json::to_string(&json!([row["from"].clone(), row["to"].clone()])).unwrap())
        .collect();
    assert!(sort_keys.windows(2).all(|pair| pair[0] <= pair[1]));

    for row in rows {
        assert_eq!(
            row["from"].as_object().unwrap().keys().collect::<Vec<_>>(),
            ["access", "file", "line", "path"]
        );
        assert_eq!(
            row["to"].as_object().unwrap().keys().collect::<Vec<_>>(),
            ["access", "file", "line", "path"]
        );
        assert!(matches!(
            row["from"]["access"].as_str(),
            Some("def" | "use")
        ));
        assert!(matches!(row["to"]["access"].as_str(), Some("def" | "use")));
        assert!(row["from"]["path"].is_object());
        assert!(row["to"]["path"].is_object());
        match row["doubt"].as_str() {
            Some("killed") => assert!(row.get("kill_line").is_some_and(Value::is_u64)),
            Some("sameline" | "cfg_incomplete" | "alias_unstable" | "call_nameonly") | None => {
                assert!(row.get("kill_line").is_none(), "{row:#}")
            }
            other => panic!("unexpected doubt spelling {other:?}: {row:#}"),
        }
    }
}
