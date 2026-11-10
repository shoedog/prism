use prism::{data_flow::VarAccessKind, slice::SliceConfig};
use prism_review::{algorithms::taint, api::*, output};
use serde_json::{json, Value};
use std::{collections::BTreeSet, fs, panic::AssertUnwindSafe};
use tempfile::TempDir;

pub(super) struct Fixture {
    pub temp: TempDir,
    pub options: ReviewOptions,
    pub inputs: ReviewInputs,
}

impl Fixture {
    pub fn new(positive: bool, warnings: bool, loaded: bool) -> Self {
        let temp = TempDir::new().unwrap();
        let source = if positive {
            "import { send } from './transport';\nfunction h(req) {\n const copy = req;\n eval(copy);\n send(copy);\n}\n".into()
        } else {
            format!(
                "import {{ relay }} from './transport';\nfunction h(req) {{\n{}\n}}\n",
                [" relay(req);"; 12].join("\n")
            )
        };
        let lines: Vec<usize> = if positive {
            vec![3, 5]
        } else {
            (3..=14).collect()
        };
        // Disk inventory and loaded diff entries are deliberately independent.
        fs::write(temp.path().join("a.js"), source).unwrap();
        fs::write(
            temp.path().join("transport.js"),
            "export function relay(x) {}\nexport function send(x) {}\n",
        )
        .unwrap();
        let mut files =
            vec![json!({"file_path":"a.js", "modify_type":"Modified", "diff_lines":lines})];
        if loaded {
            files.push(
                json!({"file_path":"transport.js", "modify_type":"Modified", "diff_lines":[]}),
            );
        }
        let mut options = ReviewOptions::new(temp.path());
        options.no_cache = true;
        if warnings {
            for (path, source) in [
                (
                    "bad.c",
                    r#"
@@@ MACRO_CHAOS @@@
#define FOO( bar baz qux
int x = ))) + [[[;
typedef struct { int a; int b; } Foo
void broken( int a, { return a +;
@@@ MORE_GARBAGE @@@
int = = = ;
"#,
                ),
                ("notes.txt", "unsupported\n"),
            ] {
                fs::write(temp.path().join(path), source).unwrap();
                let lines: &[usize] = if path == "bad.c" { &[] } else { &[3] };
                files.push(json!({"file_path":path, "modify_type":"Modified", "diff_lines":lines}));
            }
            options.compile_commands = Some(temp.path().join("missing-compile-commands.json"));
        }
        let diff = json!({"files":files}).to_string();
        fs::write(temp.path().join("diff.json"), &diff).unwrap();
        let inputs = load_review_inputs(&options, &diff).unwrap();
        assert_eq!(
            inputs.files.contains_key("transport.js"),
            loaded,
            "loaded/disk precondition"
        );
        assert_eq!(
            inputs.files["a.js"].parse_error_count, 0,
            "clean caller precondition"
        );
        Self {
            temp,
            options,
            inputs,
        }
    }

    pub fn admit(&self, positive: bool, loaded: bool) -> ReviewBoundaryDiagnostics {
        let built = build_context(&self.inputs, &self.options).unwrap();
        let (_, trace) = taint::slice_with_trace(
            &built.ctx,
            &self.inputs.diff,
            &taint::TaintConfig::default(),
        )
        .unwrap();
        let name = if positive { "send" } else { "relay" };
        let source = &self.inputs.sources["a.js"];
        let sites: BTreeSet<_> = built
            .ctx
            .cpg
            .call_graph
            .calls
            .values()
            .flatten()
            .filter(|s| s.callee_name == name)
            .map(|s| (s.line, s.start_byte, s.end_byte))
            .collect();
        let rows: Vec<_> = sites
            .iter()
            .map(|&(line, start, end)| {
                let argument = self.inputs.files["a.js"]
                    .tree
                    .root_node()
                    .descendant_for_byte_range(start, end)
                    .filter(|n| n.kind() == "call_expression" && n.byte_range() == (start..end))
                    .and_then(|n| n.child_by_field_name("arguments"))
                    .filter(|n| n.named_child_count() == 1)
                    .and_then(|n| n.named_child(0));
                let actual = argument.is_some_and(|n| {
                    &source[n.byte_range()] == if positive { "copy" } else { "req" }
                });
                let retained = argument.is_some_and(|n| {
                    trace.roots.iter().any(|r| {
                        r.origin == taint::TaintRootOrigin::Diff
                            && r.location.file == "a.js"
                            && r.location.line == line
                            && r.location.kind == VarAccessKind::Use
                            && r.location.start_byte == n.start_byte()
                            && r.location.end_byte == n.end_byte()
                    })
                });
                (line, actual, retained)
            })
            .collect();
        eprintln!("PRODUCER positive={positive} loaded={loaded} arguments={rows:?}");
        assert_eq!(
            rows.len(),
            if positive { 1 } else { 12 },
            "physical site precondition"
        );
        assert!(
            rows.iter().all(|r| r.1 && r.2),
            "exact parsed argument/Diff Use precondition"
        );
        let config = SliceConfig::default();
        let params = AlgorithmParams::default();
        let typed = run_algorithm_with_diagnostics(
            prism::slice::SlicingAlgorithm::Taint,
            &built.ctx,
            &self.inputs,
            &config,
            &params,
            self.temp.path(),
        )
        .unwrap();
        let legacy = run_algorithm(
            prism::slice::SlicingAlgorithm::Taint,
            &built.ctx,
            &self.inputs,
            &config,
            &params,
            self.temp.path(),
        )
        .unwrap();
        if positive {
            assert!(
                typed
                    .result
                    .findings
                    .iter()
                    .any(|f| f.category.as_deref() == Some("taint_sink")),
                "real sink precondition"
            );
            assert!(
                typed
                    .result
                    .evidence
                    .iter()
                    .flatten()
                    .any(|e| !e.hops.is_empty()),
                "real evidence precondition"
            );
        } else {
            assert!(
                typed.result.findings.is_empty(),
                "note-only producer precondition"
            );
        }
        let d = typed.diagnostics.unwrap();
        assert_eq!(d.availability(), &ReviewBoundaryAvailability::Available);
        assert_eq!(
            (d.located().len(), d.omitted_count()),
            if loaded {
                (0, 0)
            } else if positive {
                (1, 0)
            } else {
                (10, 2)
            }
        );
        assert!(d.located().iter().all(|n| n.file() == "a.js"
            && &source[n.byte_range()] == if positive { "send(copy)" } else { "relay(req)" }));
        let mut legacy_expected = typed.result.clone();
        legacy_expected.warnings.extend(d.messages());
        assert_eq!(
            serde_json::to_value(&legacy_expected).unwrap(),
            serde_json::to_value(&legacy).unwrap(),
            "producer legacy payload parity after exact diagnostic append"
        );
        assert_eq!(
            format!("{:?}", typed.result.evidence),
            format!("{:?}", legacy.evidence),
            "serde-skipped aligned evidence parity"
        );
        d
    }

    pub fn invoke(&self, algorithm: &str, format: &str) -> (Value, String) {
        let mut command = crate::prism_std();
        command.args([
            "--repo",
            self.temp.path().to_str().unwrap(),
            "--diff",
            self.temp.path().join("diff.json").to_str().unwrap(),
            "--algorithm",
            algorithm,
            "--format",
            format,
            "--no-cache",
        ]);
        if let Some(path) = &self.options.compile_commands {
            command.arg("--compile-commands").arg(path);
        }
        let out = command.output().unwrap();
        let stderr = String::from_utf8(out.stderr).unwrap();
        assert!(
            out.status.success(),
            "CLI status={} stderr={stderr}",
            out.status
        );
        (serde_json::from_slice(&out.stdout).unwrap(), stderr)
    }

    pub fn expected(&self, spec: &str, format: &str) -> (Value, Option<ReviewBoundaryDiagnostics>) {
        let built = build_context(&self.inputs, &self.options).unwrap();
        if format == "callers" {
            return (
                serde_json::to_value(output::to_callers_output(&built.ctx, &self.inputs.diff, 5))
                    .unwrap(),
                None,
            );
        }
        let algorithms = parse_algorithms(spec).unwrap();
        let config = SliceConfig {
            algorithm: algorithms[0],
            ..SliceConfig::default()
        };
        let params = AlgorithmParams::default();
        let (mut results, findings, evidence, errors, warnings, diagnostics) =
            if algorithms.len() > 1 {
                let run = run_review(
                    &built.ctx,
                    &self.inputs,
                    &algorithms,
                    &config,
                    &params,
                    self.temp.path(),
                );
                (
                    run.results,
                    run.findings,
                    run.evidence,
                    run.errors,
                    run.warnings,
                    run.diagnostics,
                )
            } else {
                let outcome = run_algorithm_with_diagnostics(
                    algorithms[0],
                    &built.ctx,
                    &self.inputs,
                    &config,
                    &params,
                    self.temp.path(),
                )
                .unwrap();
                let mut result = outcome.result;
                filter_result_findings(
                    &mut result,
                    &self.inputs,
                    self.inputs.min_confidence,
                    self.inputs.resolution,
                );
                let mut warnings = self.inputs.parse_warnings.clone();
                warnings.extend(result.warnings.clone());
                if let Some(d) = &outcome.diagnostics {
                    warnings.extend(d.messages());
                }
                (
                    vec![result.clone()],
                    result.findings,
                    result.evidence,
                    vec![],
                    warnings,
                    outcome.diagnostics,
                )
            };
        let names: Vec<_> = algorithms.iter().map(|a| a.name().to_string()).collect();
        let document = if format == "sarif" {
            let mut inputs = output::SarifInputs::new(&findings)
                .evidence(&evidence)
                .errors(&errors)
                .parse_warnings(&self.inputs.parse_warnings)
                .load_warnings(&self.inputs.load_warnings)
                .build_warnings(&built.warnings)
                .algorithms_run(&names)
                .parse_quality(&self.inputs.parse_quality)
                .files(&self.inputs.files)
                .sources(&self.inputs.sources)
                .min_confidence(self.inputs.min_confidence)
                .resolution(self.inputs.resolution);
            if let Some(d) = &diagnostics {
                inputs = inputs.boundary_diagnostics(d);
            }
            output::to_sarif(&inputs)
        } else if results.len() == 1 {
            results[0].warnings = warnings;
            if format == "json" {
                serde_json::to_value(output::to_review_output(&results[0], &self.inputs.sources))
                    .unwrap()
            } else {
                serde_json::to_value(output::to_compact_review_output(
                    &results[0],
                    &self.inputs.sources,
                    output::severity_rank("warning"),
                    false,
                    false,
                ))
                .unwrap()
            }
        } else {
            let diagram_warnings = results
                .iter()
                .flat_map(|r| r.diagram_warnings.clone())
                .collect();
            if format == "json" {
                serde_json::to_value(output::MultiReviewOutput {
                    version: "1.0".into(),
                    algorithms_run: names,
                    results: results
                        .iter()
                        .map(|r| output::to_review_output(r, &self.inputs.sources))
                        .collect(),
                    all_findings: findings,
                    errors,
                    warnings,
                    parse_quality: self.inputs.parse_quality.clone(),
                    diagram_warnings,
                })
                .unwrap()
            } else {
                let min_rank = output::severity_rank("warning");
                serde_json::to_value(output::CompactMultiReviewOutput {
                    version: "1.0".into(),
                    algorithms_run: names,
                    results: results
                        .iter()
                        .map(|r| {
                            output::to_compact_review_output(
                                r,
                                &self.inputs.sources,
                                min_rank,
                                false,
                                false,
                            )
                        })
                        .collect(),
                    findings_below_floor: output::count_below_floor(&findings, min_rank),
                    all_findings: findings
                        .into_iter()
                        .filter(|f| output::severity_rank(&f.severity) >= min_rank)
                        .collect(),
                    errors,
                    warnings,
                    parse_quality: self.inputs.parse_quality.clone(),
                    diagram_warnings,
                })
                .unwrap()
            }
        };
        (document, diagnostics)
    }
}

pub(super) fn row(rows: &mut Vec<(String, bool)>, label: String, check: impl FnOnce()) {
    let passed = std::panic::catch_unwind(AssertUnwindSafe(check)).is_ok();
    eprintln!("ROW {label}: {passed}");
    rows.push((label, passed));
}

pub(super) fn notifications(doc: &Value) -> Vec<Value> {
    doc["runs"][0]["invocations"][0]["toolExecutionNotifications"]
        .as_array()
        .cloned()
        .unwrap_or_default()
}
