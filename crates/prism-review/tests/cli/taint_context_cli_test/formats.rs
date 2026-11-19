//! C06-C14. REAL captured inputs and native CLI, with public API output as the oracle.
#[path = "format_fixture.rs"]
mod fixture;
#[path = "format_oracle.rs"]
mod oracle;

use fixture::{Fixture, Policy, B, BOUND, DEGRADED, ON};
use oracle::Oracle;
use prism::finding_confidence::{
    EvidenceHop, FindingConfidence, FindingTier, MinConfidence, ParseQuality, ResolutionMode,
};
use prism::slice::SlicingAlgorithm;
use prism_review::api::TaintContextEdges;
use serde_json::Value;
use std::process::Output;

const FORMATS: [&str; 6] = ["json", "review", "sarif", "text", "paper", "mermaid"];
const DEPTH_NOTE: &str = "taint-context: depth bound 1 reached; 1 boundary calls not followed";

fn json(out: &Output) -> Value {
    serde_json::from_slice(&out.stdout).unwrap()
}

fn success(out: &Output) {
    assert_eq!(
        out.status.code(),
        Some(0),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
}

fn same_bytes(actual: &[u8], expected: &[u8], label: &str) {
    if actual != expected {
        panic!(
            "{label}\nexpected:\n{}\nactual:\n{}",
            String::from_utf8_lossy(expected),
            String::from_utf8_lossy(actual)
        );
    }
}

fn checked(f: &Fixture, api: &Oracle, selection: &str, format: &str, flags: &[&str]) -> Output {
    let out = f.invoke(format, selection, flags);
    success(&out);
    same_bytes(
        &out.stdout,
        &api.doc(format),
        &format!("{selection} {format} stdout"),
    );
    let stderr = std::str::from_utf8(&out.stderr).unwrap();
    let mut expected = Vec::new();
    if matches!(format, "text" | "paper" | "mermaid") {
        expected.extend(api.messages().iter().map(|m| format!("WARNING: {m}")));
    }
    let actual: Vec<_> = stderr
        .lines()
        .filter(|l| l.contains("taint-context"))
        .collect();
    assert_eq!(actual, expected, "{selection} {format} stderr diagnostics");
    out
}

fn sink_index(r: &prism::slice::SliceResult, file: &str) -> Option<usize> {
    r.findings
        .iter()
        .position(|f| f.category.as_deref() == Some("taint_sink") && f.file == file)
}

fn taint(api: &Oracle) -> &prism::slice::SliceResult {
    api.run
        .results
        .iter()
        .find(|r| r.algorithm == SlicingAlgorithm::Taint)
        .unwrap()
}

fn admitted(api: &Oracle, file: &str) {
    assert!(
        api.context
            .as_ref()
            .is_some_and(|c| c.context_files().contains(file)),
        "inadmissible precondition: library must independently admit {file}"
    );
    assert!(!api.inputs.files.contains_key(file));
}

fn js_witness(api: &Oracle) {
    admitted(api, "b.js");
    let r = taint(api);
    let index = sink_index(r, "b.js").expect("inadmissible precondition: native JS sink");
    let finding = &r.findings[index];
    let path = r.evidence[index]
        .as_ref()
        .expect("inadmissible precondition: native selected evidence");
    assert!(path
        .hops
        .iter()
        .any(|h| matches!(h, EvidenceHop::DataFlow { from, to, .. }
        if from.file == "a.js" && to.file == "b.js")));
    assert!(finding.description.contains("loaded as taint context"));
    assert!(finding.related_files.iter().any(|f| f == "main.js"));
    assert!(r.findings.iter().any(|f| f.file == "main.js"
        && f.line == 3
        && f.category.as_deref() == Some("taint_source")
        && f.related_files.iter().any(|file| file == "b.js")));
}

fn bounded(api: &Oracle) {
    admitted(api, "a.js");
    let d = api.diagnostics.as_ref().unwrap();
    assert_eq!(
        d.run_notes().len(),
        1,
        "inadmissible precondition: actual depth note"
    );
    assert_eq!(d.run_notes()[0].message(), DEPTH_NOTE);
    assert_eq!(d.located().len(), 1);
    assert_eq!((d.located()[0].file(), d.located()[0].line()), ("a.js", 2));
}

#[test]
fn c06_off_is_byte_identical_to_no_flags() {
    let f = Fixture::js(B);
    let control = Oracle::capture(&f, "taint", &Policy::off(), false);
    assert!(
        sink_index(taint(&control), "main.js").is_some(),
        "inadmissible precondition: original sink"
    );
    assert!(control.context.is_none());
    for (selection, format) in ["taint", "taint,absence"]
        .iter()
        .flat_map(|&a| {
            FORMATS
                .into_iter()
                .chain(["callers"])
                .map(move |fmt| (a, fmt))
        })
        .chain([("taint", "targets")])
    {
        let original = f.invoke(format, selection, &[]);
        success(&original);
        for flags in [
            &["--taint-context", "off"][..],
            &[
                "--taint-context",
                "off",
                "--taint-context-edges",
                "exact",
                "--taint-context-depth",
                "4",
                "--taint-context-max-files",
                "1",
            ],
        ] {
            let explicit = f.invoke(format, selection, flags);
            assert_eq!(
                explicit.status.code(),
                original.status.code(),
                "{selection} {format}"
            );
            same_bytes(
                &explicit.stdout,
                &original.stdout,
                &format!("{selection} {format} OFF stdout"),
            );
            same_bytes(
                &explicit.stderr,
                &original.stderr,
                &format!("{selection} {format} OFF stderr"),
            );
        }
    }
}

fn formats_and_notes(selection: &str) {
    let f = Fixture::js(DEGRADED);
    let api = Oracle::capture(&f, selection, &Policy::imports(), false);
    js_witness(&api);
    let partial = Oracle::capture(&f, selection, &Policy::bounded(), false);
    bounded(&partial);
    for fmt in FORMATS {
        checked(&f, &api, selection, fmt, ON);
        checked(&f, &partial, selection, fmt, BOUND);
    }
}

#[test]
fn c07_on_single_each_format() {
    formats_and_notes("taint");
}

// Pretty-printed nested entries have four-space object delimiters. Keep the literal
// bytes between those delimiters, including field order, spacing and escaped strings.
fn result_entries(stdout: &[u8]) -> Vec<&str> {
    let text = std::str::from_utf8(stdout).unwrap();
    let (_, entries) = text.split_once("\n  \"results\": [\n    {").unwrap();
    let (entries, _) = entries.split_once("\n    }\n  ]").unwrap();
    entries.split("\n    },\n    {").collect()
}

#[test]
fn c08_on_multi_each_format_and_review_suite() {
    formats_and_notes("taint,absence");
    let f = Fixture::js(DEGRADED);
    for selection in ["taint,absence", "review"] {
        let off = Oracle::capture(&f, selection, &Policy::off(), false);
        let on = Oracle::capture(&f, selection, &Policy::imports(), false);
        js_witness(&on);
        assert!(on
            .run
            .results
            .iter()
            .any(|r| r.algorithm != SlicingAlgorithm::Taint));
        assert_eq!(off.run.results.len(), on.run.results.len());
        for (a, b) in off.run.results.iter().zip(&on.run.results) {
            assert_eq!(a.algorithm, b.algorithm);
            if a.algorithm != SlicingAlgorithm::Taint {
                assert_eq!(
                    serde_json::to_vec(a).unwrap(),
                    serde_json::to_vec(b).unwrap()
                );
                assert_eq!(format!("{:?}", a.evidence), format!("{:?}", b.evidence));
            }
        }
        let formats: &[_] = if selection == "review" {
            &["json"]
        } else {
            &["json", "review"]
        };
        for &format in formats {
            let old = checked(&f, &off, selection, format, &["--resolution", "scoped"]);
            let new = checked(&f, &on, selection, format, ON);
            let (a, b) = (result_entries(&old.stdout), result_entries(&new.stdout));
            assert_eq!(a.len(), on.run.results.len());
            assert_eq!(b.len(), a.len());
            for (index, r) in on.run.results.iter().enumerate() {
                if r.algorithm != SlicingAlgorithm::Taint {
                    assert_eq!(a[index], b[index], "{format} {}", r.algorithm.name());
                }
            }
        }
    }
}

fn entry<'a>(doc: &'a Value, fmt: &str, file: &str) -> Option<&'a Value> {
    let rows = match fmt {
        "targets" => &doc["targets"],
        "sarif" => &doc["runs"][0]["results"],
        _ => &doc["findings"],
    };
    rows.as_array()?.iter().find(|item| match fmt {
        "targets" => item["site"]["file"] == file && item["category"] == "taint_sink",
        "sarif" => {
            item["locations"][0]["physicalLocation"]["artifactLocation"]["uri"] == file
                && item["properties"]["category"] == "taint_sink"
        }
        _ => item["file"] == file && item["category"] == "taint_sink",
    })
}

#[test]
fn c09_on_targets_stdout_out_and_strict() {
    let f = Fixture::js(DEGRADED);
    let api = Oracle::capture(&f, "taint", &Policy::imports(), true);
    js_witness(&api);
    let out = checked(&f, &api, "taint", "targets", ON);
    let doc = json(&out);
    let t = entry(&doc, "targets", "b.js").unwrap();
    assert_eq!(t["site"]["symbol"], "g");
    assert_eq!(t["site"]["function_start_line"], 1);
    assert_eq!(t["site"]["function_end_line"], 1);
    assert_eq!(doc["diff"]["files"], serde_json::json!(["main.js"]));
    assert_eq!(doc["diff"]["sha256"], api.inputs.diff_text_sha256);
    let file = f.dir.path().join("targets.json");
    let mut flags = ON.to_vec();
    flags.extend(["--out", file.to_str().unwrap(), "--strict"]);
    let written = f.invoke("targets", "taint", &flags);
    assert_eq!(written.status.code(), out.status.code());
    assert_eq!(written.stderr, out.stderr);
    assert!(written.stdout.is_empty());
    same_bytes(&std::fs::read(file).unwrap(), &out.stdout, "targets --out");
    let partial = Oracle::capture(&f, "taint", &Policy::bounded(), true);
    bounded(&partial);
    for flags in [BOUND.to_vec(), [BOUND, &["--strict"]].concat()] {
        checked(&f, &partial, "taint", "targets", &flags);
    }
}

#[test]
fn c10_text_block_is_not_header_only() {
    let f = Fixture::js(B);
    for selection in ["taint", "taint,absence"] {
        let api = Oracle::capture(&f, selection, &Policy::imports(), false);
        js_witness(&api);
        let block = taint(&api)
            .blocks
            .iter()
            .find(|b| b.file == "b.js")
            .unwrap();
        let out = checked(&f, &api, selection, "text", ON);
        let text = String::from_utf8(out.stdout).unwrap();
        let body = text
            .split(&format!(
                "# Block {} [{}] b.js\n",
                block.block_id,
                block.modify_type.code()
            ))
            .nth(1)
            .unwrap();
        let numbered_source = format!("   1|{}", B.trim_end());
        assert!(body
            .lines()
            .take_while(|l| !l.starts_with("# Block") && !l.starts_with("==="))
            .any(|line| line.get(1..) == Some(numbered_source.as_str())));
    }
}

#[test]
fn c11_sarif_grade_and_fingerprint() {
    for (body, grade) in [(B, "clean"), (DEGRADED, "degraded")] {
        let f = Fixture::js(body);
        for selection in ["taint", "taint,absence"] {
            let api = Oracle::capture(&f, selection, &Policy::imports(), false);
            js_witness(&api);
            let r = taint(&api);
            let index = sink_index(r, "b.js").unwrap();
            let finding = &r.findings[index];
            let native = prism::finding_confidence::classify_with_evidence(
                "taint",
                ParseQuality::from_quality_str(grade),
                r.evidence[index].as_ref().unwrap(),
            );
            assert_eq!(
                native,
                (FindingConfidence::NameOnly, FindingTier::Candidate)
            );
            let doc = json(&checked(&f, &api, selection, "sarif", ON));
            let item = entry(&doc, "sarif", "b.js").unwrap();
            assert_eq!(item["properties"]["parse_quality"], grade);
            assert_eq!(item["properties"]["confidence"], "nameonly");
            assert_eq!(item["properties"]["tier"], "candidate");
            let line = body.lines().nth(finding.line - 1).unwrap();
            let fingerprint = prism_review::output::sarif::fingerprint(finding, line);
            assert_ne!(
                fingerprint,
                prism_review::output::sarif::fingerprint(finding, "")
            );
            assert_eq!(item["partialFingerprints"]["prism/finding/v1"], fingerprint);
        }
    }
}

fn ts_entry(api: &Oracle, strong: bool) {
    admitted(api, "x.ts");
    let r = taint(api);
    let i = sink_index(r, "x.ts").expect("inadmissible precondition: native TS sink");
    let path = r.evidence[i].as_ref().unwrap();
    let (function, line) = if strong { ("strong", 4) } else { ("weak", 3) };
    assert!(
        path.hops
            .iter()
            .any(|h| matches!(h, EvidenceHop::DataFlow { from, to, .. }
        if from.file == "app.ts" && from.line == line && from.function == function
            && to.file == "x.ts" && to.function == "f" && to.line == 1 && h.is_exact() == strong)),
        "inadmissible precondition: selected physical TS entry {function}"
    );
    let confidence = if strong {
        FindingConfidence::Exact
    } else {
        FindingConfidence::NameOnly
    };
    assert_eq!(
        prism::finding_confidence::classify_with_evidence("taint", ParseQuality::Clean, path).0,
        confidence
    );
}

#[test]
fn c12_resolution_and_floor() {
    // One diff seed per execution allows the targets subcommand to use precisely the
    // strong or weak source without an explicit source flag. Check its physical hop first.
    for strong in [true, false] {
        let f = Fixture::ts(if strong { &[4] } else { &[3] });
        let open = Oracle::capture(&f, "taint", &Policy::imports(), true);
        ts_entry(&open, strong);
        for (mode, resolution) in [
            ("scoped", ResolutionMode::Scoped),
            ("nominal", ResolutionMode::Nominal),
        ] {
            let mut p = Policy::imports();
            p.resolution = resolution;
            p.floor = MinConfidence::Exact;
            let flags = [
                "--taint-context",
                "imports",
                "--resolution",
                mode,
                "--min-confidence",
                "exact",
            ];
            for fmt in ["json", "review", "sarif", "targets"] {
                let api = Oracle::capture(&f, "taint", &p, fmt == "targets");
                let doc = json(&checked(&f, &api, "taint", fmt, &flags));
                let item = entry(&doc, fmt, "x.ts");
                assert_eq!(item.is_some(), strong, "{fmt} {mode} strong={strong}");
                if strong && matches!(fmt, "targets" | "sarif") {
                    let item = item.unwrap();
                    let props = if fmt == "sarif" {
                        &item["properties"]
                    } else {
                        item
                    };
                    assert_eq!(
                        props["confidence"],
                        if mode == "scoped" {
                            "exact"
                        } else {
                            "unlabeled"
                        }
                    );
                    assert_eq!(
                        props["tier"],
                        if mode == "scoped" {
                            "asserted"
                        } else {
                            "candidate"
                        }
                    );
                }
            }
        }
    }
    let f = Fixture::js(B);
    let mut p = Policy::bounded();
    p.floor = MinConfidence::Exact;
    let flags = [BOUND, &["--min-confidence", "exact"]].concat();
    for fmt in ["json", "review", "sarif", "targets"] {
        let api = Oracle::capture(&f, "taint", &p, fmt == "targets");
        bounded(&api);
        let out = checked(&f, &api, "taint", fmt, &flags);
        assert_eq!(
            String::from_utf8(out.stdout)
                .unwrap()
                .matches(DEPTH_NOTE)
                .count(),
            1
        );
    }
    for fmt in ["text", "paper", "mermaid", "callers"] {
        let before = f.invoke(fmt, "taint", &["--min-confidence", "exact"]);
        let after = f.invoke(
            fmt,
            "taint",
            &["--taint-context", "imports", "--min-confidence", "exact"],
        );
        assert_eq!(before.status.code(), Some(2));
        assert_eq!(after.status.code(), before.status.code());
        assert!(after.stdout.is_empty());
        assert!(String::from_utf8_lossy(&after.stderr).contains(&format!(
            "--min-confidence cannot be used with --format {fmt}"
        )));
        same_bytes(&after.stderr, &before.stderr, "rejection stderr");
    }
}

#[test]
fn c13_edges_exact_versus_nameonly() {
    let f = Fixture::ts(&[3, 4]);
    for (line, source) in [(3, "app.ts:3"), (4, "app.ts:4")] {
        for (name, edges) in [
            ("nameonly", TaintContextEdges::NameOnly),
            ("exact", TaintContextEdges::Exact),
        ] {
            let p = Policy::rooted(line, edges);
            let api = Oracle::capture(&f, "taint", &p, false);
            admitted(&api, "x.ts");
            assert!(!api
                .context
                .as_ref()
                .unwrap()
                .files()
                .contains_key("other/x.ts"));
            let expect_sink = line == 4 || edges == TaintContextEdges::NameOnly;
            if expect_sink {
                ts_entry(&api, line == 4);
            }
            let flags = [
                ON,
                &["--taint-source", source, "--taint-context-edges", name],
            ]
            .concat();
            let doc = json(&checked(&f, &api, "taint", "sarif", &flags));
            assert_eq!(
                entry(&doc, "sarif", "x.ts").is_some(),
                expect_sink,
                "{line} {name}"
            );
        }
    }
}

#[test]
fn c14_duplicate_taint_selection() {
    let f = Fixture::js(DEGRADED);
    let duplicate = "taint,taint";
    let on = Oracle::capture(&f, duplicate, &Policy::imports(), false);
    js_witness(&on);
    assert_eq!(on.run.results.len(), 2);
    let (first, second) = (&on.run.results[0], &on.run.results[1]);
    assert_eq!(
        serde_json::to_vec(first).unwrap(),
        serde_json::to_vec(second).unwrap()
    );
    assert_eq!(
        format!("{:?}", first.evidence),
        format!("{:?}", second.evidence)
    );
    let doc = json(&checked(&f, &on, duplicate, "json", ON));
    assert_eq!(doc["algorithms_run"], serde_json::json!(["Taint", "Taint"]));
    assert_eq!(doc["results"][0], doc["results"][1]);
    let n = doc["all_findings"].as_array().unwrap();
    assert_eq!(n.len(), 2 * first.findings.len());
    assert_eq!(n[..first.findings.len()], n[first.findings.len()..]);
    let partial = Oracle::capture(&f, duplicate, &Policy::bounded(), false);
    bounded(&partial);
    let bound = json(&checked(&f, &partial, duplicate, "json", BOUND));
    for message in partial.messages() {
        assert_eq!(
            bound["warnings"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|w| w.as_str() == Some(message.as_str()))
                .count(),
            1
        );
    }
}
