//! #602: a review-taint sink in a second diff file must name its real
//! (path-derived) source, not a same-file diff-line fallback, and must not
//! emit a spurious `taint_source` finding at that fallback line.
use crate::common::*;

const NAMED_JS: &str = "const { runCmd } = require('./lib/cmd');
function viaNamed(req, res) {
  runCmd(req.body.c);
  res.end();
}
module.exports = { viaNamed };
";

const CMD_JS: &str = "const { exec } = require('child_process');
function runCmd(c) {
  const full = 'ls ' + c;
  exec(full);
}
module.exports = { runCmd };
";

// CMD_JS plus a local caller placed AFTER the sink line.
const CMD_WITH_LOCAL_CALLER_JS: &str = "const { exec } = require('child_process');
function runCmd(c) {
  const full = 'ls ' + c;
  exec(full);
}
module.exports = { runCmd };
function viaLocal(req, res) {
  runCmd(req.body.c);
}
";

const APP_PY: &str = "from flask import request
from lib import run_cmd

def handler():
    x = request.args.get(\"c\")
    run_cmd(x)
    return \"ok\"
";

const LIB_PY: &str = "import os

def run_cmd(c):
    full = \"ls \" + c
    os.system(full)
";

const SAME_FILE_JS: &str = "const { exec } = require('child_process');
function runCmd(c) {
  const full = 'ls ' + c;
  exec(full);
}
function viaNamed(req, res) {
  runCmd(req.body.c);
  res.end();
}
";

fn run_taint(files: &[(&str, &str, Language, &[usize])]) -> SliceResult {
    let mut parsed = BTreeMap::new();
    let mut infos = Vec::new();
    for (path, src, lang, lines) in files {
        parsed.insert(
            path.to_string(),
            ParsedFile::parse(path, src, *lang).unwrap(),
        );
        infos.push(DiffInfo {
            file_path: path.to_string(),
            modify_type: ModifyType::Modified,
            diff_lines: lines.iter().copied().collect(),
        });
    }
    algorithms::run_slicing_compat(
        &parsed,
        &DiffInput { files: infos },
        &SliceConfig::default().with_algorithm(SlicingAlgorithm::Taint),
        None,
    )
    .unwrap()
}

fn by_category<'a>(r: &'a SliceResult, cat: &str) -> Vec<&'a SliceFinding> {
    r.findings
        .iter()
        .filter(|f| f.category.as_deref() == Some(cat))
        .collect()
}

fn summary(r: &SliceResult) -> Vec<(String, usize, String, String)> {
    r.findings
        .iter()
        .map(|f| {
            (
                f.file.clone(),
                f.line,
                f.category.clone().unwrap_or_default(),
                f.description.clone(),
            )
        })
        .collect()
}

#[test]
fn js_cross_file_sink_names_the_other_file_source_and_no_spurious_source() {
    // The callee file is in the diff at line 5 (a `}`), which used to be
    // picked as the sink's "source" by the same-file fallback.
    let r = run_taint(&[
        ("named.js", NAMED_JS, Language::JavaScript, &[3]),
        ("lib/cmd.js", CMD_JS, Language::JavaScript, &[5]),
    ]);
    let sinks = by_category(&r, "taint_sink");
    assert_eq!(sinks.len(), 1, "findings: {:?}", summary(&r));
    let sink = sinks[0];
    assert_eq!(sink.file, "lib/cmd.js");
    assert_eq!(sink.line, 4);
    assert!(
        sink.description.contains("named.js:3"),
        "sink must name its cross-file source named.js:3, got: {:?}",
        sink.description
    );
    let sources = by_category(&r, "taint_source");
    assert!(
        sources.iter().all(|s| s.file != "lib/cmd.js"),
        "no spurious taint_source in the sink's file: {:?}",
        summary(&r)
    );
    assert!(
        sources.iter().any(|s| s.file == "named.js" && s.line == 3),
        "true source named.js:3 still reported: {:?}",
        summary(&r)
    );
}

#[test]
fn py_cross_file_sink_names_the_other_file_source_and_no_spurious_source() {
    let r = run_taint(&[
        ("app.py", APP_PY, Language::Python, &[5]),
        ("lib.py", LIB_PY, Language::Python, &[1]),
    ]);
    let sinks = by_category(&r, "taint_sink");
    assert_eq!(sinks.len(), 1, "findings: {:?}", summary(&r));
    assert_eq!(sinks[0].file, "lib.py");
    assert!(
        sinks[0].description.contains("app.py:5"),
        "sink must name app.py:5, got: {:?}",
        sinks[0].description
    );
    assert!(
        by_category(&r, "taint_source")
            .iter()
            .all(|s| s.file != "lib.py"),
        "no spurious taint_source in lib.py: {:?}",
        summary(&r)
    );
}

#[test]
fn same_file_sink_keeps_nearest_preceding_line_description() {
    // Control: source and sink in one file. Description keeps the
    // "line N" same-file wording with no file prefix.
    let r = run_taint(&[("app.js", SAME_FILE_JS, Language::JavaScript, &[7])]);
    let sinks = by_category(&r, "taint_sink");
    assert_eq!(sinks.len(), 1, "findings: {:?}", summary(&r));
    assert_eq!(
        sinks[0].description,
        "tainted value from line 7 reaches sink at line 4"
    );
    let sources = by_category(&r, "taint_source");
    assert!(
        sources.iter().any(|s| s.file == "app.js" && s.line == 7),
        "same-file source still reported: {:?}",
        summary(&r)
    );
}

#[test]
fn same_file_path_source_wins_over_cross_file_source() {
    // lib/cmd.js:3 (same file, before the sink) and named.js:3 (cross file)
    // both reach the sink at lib/cmd.js:4. The same-file source must win, in
    // the plain `line N` form, even though a cross-file source exists.
    let r = run_taint(&[
        ("named.js", NAMED_JS, Language::JavaScript, &[3]),
        ("lib/cmd.js", CMD_JS, Language::JavaScript, &[3]),
    ]);
    let sinks = by_category(&r, "taint_sink");
    assert_eq!(sinks.len(), 1, "findings: {:?}", summary(&r));
    assert_eq!(sinks[0].file, "lib/cmd.js");
    assert_eq!(sinks[0].line, 4);
    assert_eq!(
        sinks[0].description,
        "tainted value from line 3 reaches sink at line 4",
        "findings: {:?}",
        summary(&r)
    );
}

#[test]
fn cross_file_source_is_named_when_same_file_source_is_after_the_sink() {
    // Declared behaviour change (#602): the only same-file path source
    // (`runCmd(req.body.c)`, line 8) is AFTER the sink at line 4, so it
    // is not eligible; the sink names the cross-file source named.js:3.
    // "lib/cmd.js" sorts before "named.js", so a pick that ignored the
    // same-file/cross-file distinction would print `line N` instead.
    let r = run_taint(&[
        ("named.js", NAMED_JS, Language::JavaScript, &[3]),
        (
            "lib/cmd.js",
            CMD_WITH_LOCAL_CALLER_JS,
            Language::JavaScript,
            &[1, 8],
        ),
    ]);
    let sinks = by_category(&r, "taint_sink");
    assert_eq!(sinks.len(), 1, "findings: {:?}", summary(&r));
    assert_eq!(sinks[0].file, "lib/cmd.js");
    assert_eq!(sinks[0].line, 4);
    assert!(
        sinks[0].description.contains("named.js:3"),
        "sink must name cross-file source named.js:3, got: {:?}",
        sinks[0].description
    );
    let sources = by_category(&r, "taint_source");
    assert!(
        !sources
            .iter()
            .any(|s| s.file == "lib/cmd.js" && s.line == 1),
        "no taint_source at the unrelated diffed line 1: {:?}",
        summary(&r)
    );
    assert!(
        sources
            .iter()
            .any(|s| s.file == "lib/cmd.js" && s.line == 8),
        "local caller line 8 still emitted as taint_source: {:?}",
        summary(&r)
    );
}
