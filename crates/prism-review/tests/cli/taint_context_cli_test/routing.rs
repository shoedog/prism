//! C15–C22 exercise the real binary over parsed repositories. Public API producer
//! checks run independently of CLI flag mapping and serialization.
#[path = "routing_fixture.rs"]
mod fixture;

use fixture::*;
use prism::{
    finding_confidence::EvidenceHop,
    slice::{DiagramWarningKind, SliceResult},
};
use prism_review::{
    api::{self, AlgorithmParams, ContextRunNoteKind},
    output,
};
use serde_json::Value;

const ON: &[&str] = &["--no-cache", "--taint-context", "imports"];
const OFF: &[&str] = &["--no-cache", "--taint-context", "off"];
const CHANNELS: &[&str] = &[
    "json", "review", "sarif", "text", "paper", "mermaid", "targets",
];
const IGNORED: &str = "taint-context ignored: taint not selected";
const CALLERS: &str = "taint-context ignored: --format callers runs no algorithm";
const DEPTH: &str = "taint-context: depth bound 1 reached; 1 boundary calls not followed";
const FILES: &str = "taint-context: max-files bound 1 reached; 1 context files not loaded";

fn js() -> Fixture {
    Fixture::js(&[("main.js", &[2, 3])])
}

fn assert_originals(sample: &Sample, expected: &[&str]) {
    assert_eq!(
        sample
            .inputs
            .files
            .keys()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        expected,
        "inadmissible precondition: original parsed file set",
    );
}

fn assert_context(sample: &Sample, expected: &[&str]) {
    let context = sample
        .context
        .as_ref()
        .expect("inadmissible precondition: eligible native context");
    assert_eq!(
        context
            .context_files()
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        expected,
        "inadmissible precondition: native admission population",
    );
}

fn assert_producer_note(sample: &Sample, kind: ContextRunNoteKind, message: &str) {
    let notes: Vec<_> = sample
        .outcome
        .diagnostics
        .iter()
        .flat_map(|diagnostics| diagnostics.run_notes())
        .map(|note| (note.kind(), note.message()))
        .collect();
    assert_eq!(
        notes,
        [(kind, message)],
        "inadmissible precondition: exact typed run note from separate public API run",
    );
}

fn assert_producer_sink(sample: &Sample, file: &str, line: usize, nonempty_hops: bool) {
    let result = &sample.outcome.result;
    let index = result
        .findings
        .iter()
        .position(|finding| {
            finding.file == file
                && finding.line == line
                && finding.category.as_deref() == Some("taint_sink")
        })
        .expect("inadmissible precondition: native context sink");
    let evidence = result.evidence[index]
        .as_ref()
        .expect("inadmissible precondition: aligned selected sink evidence");
    if nonempty_hops {
        assert!(
            !evidence.hops.is_empty(),
            "inadmissible precondition: selected JS witness has hops",
        );
    }
    let context = sample.context.as_ref().unwrap();
    let parsed = context.files();
    for hop in &evidence.hops {
        match hop {
            EvidenceHop::DataFlow { from, to, .. } => assert!(
                parsed.contains_key(&from.file) && parsed.contains_key(&to.file),
                "inadmissible precondition: selected dataflow coordinates in the analysed file set",
            ),
            EvidenceHop::Call { edge, .. } => assert!(
                parsed.contains_key(&edge.caller.file),
                "inadmissible precondition: selected call site in the analysed file set",
            ),
            _ => unreachable!("unexpected evidence hop: {hop:?}"),
        }
    }
    assert!(
        result
            .blocks
            .iter()
            .any(|block| block.file_line_map.contains_key(file)),
        "inadmissible precondition: producer has a block for the context sink",
    );
}

fn assert_same_result(left: &SliceResult, right: &SliceResult) {
    assert_eq!(
        serde_json::to_vec(left).unwrap(),
        serde_json::to_vec(right).unwrap()
    );
    assert_eq!(
        format!("{:?}", left.evidence),
        format!("{:?}", right.evidence)
    );
}

fn sink<'a>(document: &'a Value, file: &str, line: usize) -> &'a Value {
    document["findings"]
        .as_array()
        .unwrap()
        .iter()
        .find(|finding| {
            finding["file"] == file
                && finding["line"] == line
                && finding["category"] == "taint_sink"
        })
        .unwrap_or_else(|| panic!("binary JSON must report sink at {file}:{line}"))
}

fn slice_text<'a>(document: &'a Value, file: &str) -> &'a str {
    document["slices"]
        .as_array()
        .unwrap()
        .iter()
        .find(|slice| slice["file"] == file)
        .unwrap_or_else(|| panic!("binary JSON must render a block for {file}"))["slice_text"]
        .as_str()
        .unwrap()
}

#[test]
fn c15_callers_with_imports() {
    let f = js();
    let inputs = f.inputs();
    let built = api::build_context(&inputs, &f.options).unwrap();
    let expected = output::to_callers_output(&built.ctx, &inputs.diff, 5);
    assert!(
        expected
            .callers
            .iter()
            .any(|entry| entry.file == "main.js" && entry.function == "main"),
        "inadmissible precondition: callers has a real touched function",
    );
    let default = execute(&mut f.command("taint", "callers", &["--no-cache"]));
    let off = execute(&mut f.command("taint", "callers", OFF));
    let on = execute(&mut f.command("taint", "callers", ON));
    assert_bytes(
        &default.stdout,
        &pretty(&expected),
        "callers library oracle",
    );
    assert_bytes(&off.stdout, &default.stdout, "explicit off callers stdout");
    assert_bytes(&on.stdout, &default.stdout, "imports callers stdout");
    assert!(default.stderr.is_empty());
    assert!(off.stderr.is_empty());
    assert_eq!(stderr(&on), note_line(CALLERS));
}

#[test]
fn c16_no_taint_selected_with_imports() {
    let f = js();
    let baseline = f.single("absence", &AlgorithmParams::default(), false);
    let requesting = f.single("absence", &imports(), false);
    assert_originals(&baseline, &["main.js"]);
    assert!(
        requesting.context.is_none() && baseline.messages().is_empty(),
        "inadmissible precondition: no native expansion or baseline notes for absence",
    );
    assert_producer_note(&requesting, ContextRunNoteKind::Ignored, IGNORED);
    assert_same_result(&baseline.outcome.result, &requesting.outcome.result);
    for &format in CHANNELS {
        let off = execute(&mut f.command("absence", format, OFF));
        let on = execute(&mut f.command("absence", format, ON));
        if let Some(without_note) = strip_note(&on, format, IGNORED, "warning") {
            assert_eq!(
                without_note,
                json(&off),
                "{format} document differs beyond the note"
            );
            assert_eq!(on.stderr, off.stderr, "{format} stderr");
        } else {
            assert_eq!(
                stderr(&on).replacen(&note_line(IGNORED), "", 1),
                stderr(&off)
            );
            assert_bytes(&on.stdout, &off.stdout, format);
        }
        if format == "targets" {
            assert_bytes(
                &off.stdout,
                &f.targets("absence", &AlgorithmParams::default()),
                format,
            );
            assert_bytes(&on.stdout, &f.targets("absence", &imports()), format);
        } else {
            assert_document(&off, &baseline, format);
            assert_document(&on, &requesting, format);
        }
    }
}

#[test]
fn c17_partial_bounds() {
    let f = js();
    let baseline = f.single("taint", &AlgorithmParams::default(), false);
    let complete = f.single("taint", &imports(), false);
    assert_originals(&baseline, &["main.js"]);
    assert_context(&complete, &["a.js", "b.js"]);
    assert_producer_sink(&complete, "b.js", 1, true);
    let baseline_text = baseline.document("text").0;
    assert!(
        !baseline_text.is_empty(),
        "inadmissible precondition: native baseline has a rendered block",
    );
    let binary_baseline = execute(&mut f.command("taint", "text", OFF));
    assert_bytes(
        &binary_baseline.stdout,
        &baseline_text,
        "baseline rendered prefix",
    );
    for (flag, note, kind) in [
        (
            "--taint-context-depth",
            DEPTH,
            ContextRunNoteKind::DepthBound,
        ),
        (
            "--taint-context-max-files",
            FILES,
            ContextRunNoteKind::FilesBound,
        ),
    ] {
        let mut params = imports();
        if kind == ContextRunNoteKind::DepthBound {
            params.taint_context.depth = 1;
        } else {
            params.taint_context.max_files = 1;
        }
        let bounded = f.single("taint", &params, false);
        assert_context(&bounded, &["a.js"]);
        assert_producer_note(&bounded, kind, note);
        assert_eq!(
            bounded.context.as_ref().unwrap().sources()["a.js"],
            A,
            "inadmissible precondition: round-one source was admitted",
        );
        assert!(
            bounded
                .outcome
                .result
                .findings
                .iter()
                .all(|finding| finding.file != "b.js"),
            "inadmissible precondition: second-hop sink unavailable within the bound",
        );
        // G10/G11's round-one file has no sink. Its completed result
        // retains baseline rendering, with the bound/reconciled notes.
        let profile = tempfile::tempdir().unwrap();
        let profile_path = profile.path().join("round-one.jsonl");
        for &format in CHANNELS {
            let mut command = f.command("taint", format, ON);
            command.args([flag, "1"]);
            if format == "json" {
                command.env("PRISM_PROFILE", &profile_path);
            }
            let output = execute(&mut command);
            let _ = strip_note(&output, format, note, "note");
            if format == "targets" {
                assert_bytes(&output.stdout, &f.targets("taint", &params), format);
                assert!(output.stderr.is_empty());
            } else {
                assert_document(&output, &bounded, format);
            }
            if format == "text" {
                assert!(output.stdout.starts_with(&binary_baseline.stdout));
            }
            if format == "json" {
                let counters = profile_counters(&profile_path);
                assert_eq!(counters.get("rounds_completed"), Some(&1));
                assert_eq!(counters.get("admitted_files"), Some(&1));
                assert_eq!(counters.get("admitted_bytes"), Some(&(A.len() as u64)));
                assert!(json(&output)["findings"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|finding| finding["file"] != "b.js"));
            }
        }
    }
}

#[test]
fn c18_scoped_cpg_and_cache() {
    let mut f = js();
    let plain = f.single("taint", &imports(), false);
    assert_originals(&plain, &["main.js"]);
    assert_context(&plain, &["a.js", "b.js"]);
    assert_producer_sink(&plain, "b.js", 1, true);
    f.options.scoped_cpg = true;
    let scoped = f.single("taint", &imports(), false);
    assert_context(&scoped, &["a.js", "b.js"]);
    assert_producer_sink(&scoped, "b.js", 1, true);
    assert_same_result(&plain.outcome.result, &scoped.outcome.result);

    let unscoped_output = execute(&mut f.command("taint", "json", ON));
    let scoped_output = execute(f.command("taint", "json", ON).arg("--scoped-cpg"));
    sink(&json(&scoped_output), "b.js", 1);
    assert_document(&unscoped_output, &plain, "json");
    assert_document(&scoped_output, &scoped, "json");
    assert_bytes(&scoped_output.stdout, &unscoped_output.stdout, "scoped CPG");
    f.options.scoped_cpg = false;

    let cache = tempfile::tempdir().unwrap();
    let mut cached = f.command("taint", "json", &["--taint-context", "imports"]);
    cached.arg("--cache-dir").arg(cache.path());
    let first = execute(&mut cached);
    let first_json = json(&first);
    sink(&first_json, "b.js", 1);
    assert!(slice_text(&first_json, "b.js").contains(B.trim_end()));
    assert_bytes(&first.stdout, &unscoped_output.stdout, "cold CPG cache");

    let changed = "export function g(x) {\n  eval(x);\n}\n";
    f.write("b.js", changed);
    let fresh = f.single("taint", &imports(), false);
    assert_context(&fresh, &["a.js", "b.js"]);
    assert_producer_sink(&fresh, "b.js", 2, true);
    assert_eq!(
        fresh.context.as_ref().unwrap().sources()["b.js"],
        changed,
        "inadmissible precondition: producer sees the new context source",
    );
    let second = execute(&mut cached);
    assert!(
        stderr(&second)
            .lines()
            .any(|line| line == "CPG loaded from cache (1 files)"),
        "binary must hit the original-only CPG cache: {}",
        stderr(&second),
    );
    let second_json = json(&second);
    sink(&second_json, "b.js", 2);
    let fresh_text = slice_text(&second_json, "b.js");
    assert!(fresh_text.contains("   2|  eval(x);"));
    assert!(!fresh_text.contains(B.trim_end()));
    assert_bytes(
        &second.stdout,
        &fresh.document("json").0,
        "cache hit with fresh context bytes",
    );
}

#[test]
fn c19_files_filter_wording() {
    let mut f = Fixture::js(&[("main.js", &[2, 3]), ("b.js", &[1])]);
    let unfiltered = f.inputs();
    assert!(
        unfiltered
            .diff
            .files
            .iter()
            .any(|entry| entry.file_path == "b.js")
            && unfiltered.files.contains_key("b.js"),
        "inadmissible precondition: target is a real changed file in the raw diff",
    );
    f.options.files_filter = Some(["main.js".to_owned()].into_iter().collect());
    let filtered = f.single("taint", &imports(), false);
    assert_originals(&filtered, &["main.js"]);
    assert_context(&filtered, &["a.js", "b.js"]);
    assert_producer_sink(&filtered, "b.js", 1, true);
    let output = execute(f.command("taint", "json", ON).args(["--files", "main.js"]));
    let document = json(&output);
    let description = sink(&document, "b.js", 1)["description"].as_str().unwrap();
    assert!(
        description.contains("(sink file outside the analysed file set; loaded as taint context)")
    );
    assert!(!description.contains("outside the diff"));
    assert_document(&output, &filtered, "json");
}

#[test]
fn c20_strict_diagrams_exit_is_unaffected_by_notes() {
    let f = js();
    let mut params = imports();
    params.taint_context.depth = 1;
    let off = f.single("taint", &AlgorithmParams::default(), true);
    let on = f.single("taint", &params, true);
    assert_originals(&off, &["main.js"]);
    assert_context(&on, &["a.js"]);
    assert_producer_note(&on, ContextRunNoteKind::DepthBound, DEPTH);
    let warnings = &off.outcome.result.diagram_warnings;
    assert!(
        warnings
            .iter()
            .any(|warning| warning.kind == DiagramWarningKind::LabelTruncated)
            && warnings.iter().all(|warning| !warning.kind.is_bug()),
        "inadmissible precondition: real informational diagnostic and no diagram bugs",
    );
    assert_eq!(
        &on.outcome.result.diagram_warnings, warnings,
        "inadmissible precondition: same diagram diagnostics with the bound note",
    );

    for (mode, args, sample) in [("off", OFF, &off), ("imports", ON, &on)] {
        let mut command = f.command("taint", "mermaid", args);
        command.args(["--taint-context-depth", "1"]);
        let lax = execute(&mut command);
        let strict = execute(command.arg("--strict-diagrams"));
        if mode == "imports" {
            let _ = strip_note(&strict, "mermaid", DEPTH, "note");
        } else {
            assert!(!stderr(&strict).contains(DEPTH));
        }
        assert!(stderr(&strict).contains("LabelTruncated"));
        assert_bytes(&strict.stdout, &lax.stdout, mode);
        assert_bytes(&strict.stderr, &lax.stderr, mode);
        assert_document(&strict, sample, "mermaid");
    }
}

#[test]
fn c21_profile_counters() {
    let f = js();
    let on = f.single("taint", &imports(), false);
    assert_originals(&on, &["main.js"]);
    assert_context(&on, &["a.js", "b.js"]);
    assert_producer_sink(&on, "b.js", 1, true);
    let profile = tempfile::tempdir().unwrap();
    for (mode, args) in [("imports", ON), ("off", OFF)] {
        let path = profile.path().join(format!("{mode}.jsonl"));
        let unprofiled = execute(&mut f.command("taint", "json", args));
        let profiled = execute(f.command("taint", "json", args).env("PRISM_PROFILE", &path));
        assert_bytes(&profiled.stdout, &unprofiled.stdout, mode);
        assert_bytes(&profiled.stderr, &unprofiled.stderr, mode);
        let counters = profile_counters(&path);
        if mode == "imports" {
            // The exporter adds repeated writes into one counter row. Exact
            // nonzero observations also distinguish duplicate exports.
            for (name, expected) in [
                ("rounds_completed", 2),
                ("admitted_depth", 2),
                ("admitted_files", 2),
                ("admitted_bytes", (A.len() + B.len()) as u64),
                ("driver_stats_available", 1),
                ("union_stats_available", 1),
            ] {
                assert_eq!(counters.get(name), Some(&expected), "{name}");
            }
            assert_context_counter_names(&counters);
            assert_document(&profiled, &on, "json");
        } else {
            assert!(counters.is_empty(), "off must publish no context counter");
        }
    }
}

#[test]
fn c22_python_json() {
    let f = Fixture::new(
        &[
            ("main.py", "from a import helper\ndef lookup_view(request):\n    q = request.GET.get('q')\n    helper(q)\n"),
            ("a.py", "import os\ndef helper(p):\n    os.system(p)\n"),
        ],
        &[("main.py", &[4])],
    );
    let on = f.single("taint", &imports(), false);
    assert_originals(&on, &["main.py"]);
    assert_context(&on, &["a.py"]);
    // F20's native Python witness permits an empty selected hop list.
    assert_producer_sink(&on, "a.py", 3, false);
    let off = execute(&mut f.command("taint", "json", OFF));
    assert!(json(&off)["findings"]
        .as_array()
        .unwrap()
        .iter()
        .all(|finding| finding["file"] != "a.py"));
    let output = execute(&mut f.command("taint", "json", ON));
    let document = json(&output);
    sink(&document, "a.py", 3);
    assert!(slice_text(&document, "a.py").contains("   3|    os.system(p)"));
    assert_document(&output, &on, "json");
}

#[test]
fn c22_go_json() {
    // REAL declaring-file admission only. F20 did not establish a native Go context sink.
    const DECLARATION: &str = "package svc\nfunc H(x string) {}\n";
    let f = Fixture::new(
        &[
            ("go.mod", "module example.com/app\n"),
            (
                "main.go",
                "package main\nimport \"example.com/app/svc\"\nfunc main1(x string) { svc.H(x) }\n",
            ),
            ("svc/a.go", DECLARATION),
            ("svc/b.go", "package svc\nfunc B() {}\n"),
        ],
        &[("main.go", &[3])],
    );
    let baseline = f.single("taint", &AlgorithmParams::default(), false);
    let on = f.single("taint", &imports(), false);
    assert_originals(&on, &["main.go"]);
    assert_context(&on, &["svc/a.go"]);
    assert!(
        !on.context
            .as_ref()
            .unwrap()
            .files()
            .contains_key("svc/b.go")
            && on.outcome.result.findings.is_empty()
            && baseline.outcome.result.findings.is_empty(),
        "inadmissible precondition: accepted F20 Go admission and empty finding population",
    );
    let off = execute(&mut f.command("taint", "json", OFF));
    let profile = tempfile::tempdir().unwrap();
    let path = profile.path().join("go.jsonl");
    let output = execute(f.command("taint", "json", ON).env("PRISM_PROFILE", &path));
    let counters = profile_counters(&path);
    assert_eq!(counters.get("admitted_files"), Some(&1));
    assert_eq!(
        counters.get("admitted_bytes"),
        Some(&(DECLARATION.len() as u64))
    );
    assert_eq!(counters.get("admitted_depth"), Some(&1));
    assert_eq!(json(&output)["findings"], json(&off)["findings"]);
    assert_document(&off, &baseline, "json");
    assert_document(&output, &on, "json");
}
