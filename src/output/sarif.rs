//! SARIF 2.1.0 serializer for slice findings (design §2.2). The mapping tables
//! it applies live in `sarif_rules.rs`.
//!
//! One `result` per `SliceFinding` — nothing is dropped. Data SARIF cannot
//! represent as a location (related lines whose owning file is ambiguous) is
//! preserved under `properties`, and everything the run could not do (a failed
//! algorithm, a parse warning, a file skipped at load, a non-fatal build
//! condition, a path escaping the repo root) becomes a
//! `toolExecutionNotifications` entry. `to_sarif` is total over its inputs: no
//! finding, error or path shape can make it fail.
//!
//! Determinism is by construction (§2.2.4): `results` sorted by
//! `(uri, startLine, ruleId, message.text)`, `rules` by `id`, `ruleIndex`
//! assigned after both sorts, `relatedLocations` sorted and deduplicated. The
//! typed structs fix the key SET and which keys are omitted; `serde_json::Map`
//! decides key ORDER (alphabetical — `preserve_order` is off), which is
//! deterministic and carries no meaning in JSON. Artifact URIs are
//! repo-relative with `uriBaseId: "%SRCROOT%"`; there is deliberately no
//! `originalUriBaseIds` (it would disclose the local checkout path).
//!
//! **`partialFingerprints` caveat.** `prism/finding/v1` is an *occurrence*
//! discriminator that survives line shifts, not a unique alert id: two findings
//! with the same masked description and line text share one. GitHub code
//! scanning ignores it and matches on its own `primaryLocationLineHash`; any
//! other consumer keying on it MUST combine it with the primary location.

pub use super::sarif_rules::{fingerprint, level_for_severity, path_escapes_repo, sarif_uri};

use super::sarif_model::{
    ArtifactLocation, Driver, Invocation, Location, LogicalLocation, Notification,
    PhysicalLocation, Region, RelatedLocation, ResultProperties, Rule, RuleProperties, Run,
    RunProperties, SarifLog, SarifResult, Text, Tool,
};
use super::sarif_rules::{attribution, line_text_of, rule_description, Attribution, UNCATEGORIZED};
use crate::ast::ParsedFile;
use crate::finding_confidence::{classify, parse_quality_for, RESOLUTION_MODE};
use crate::slice::{AlgorithmError, FileParseQuality, SliceFinding};
use std::collections::{BTreeMap, BTreeSet};

const SRCROOT: &str = "%SRCROOT%";
const SCHEMA: &str = "https://json.schemastore.org/sarif-2.1.0.json";
const INFORMATION_URI: &str = "https://github.com/shoedog/prism";
/// Versions prism's SARIF *mapping*; the SARIF `version` versions the standard.
const MAPPING_VERSION: &str = "1";
const FINGERPRINT_KEY: &str = "prism/finding/v1";

/// Everything the serializer reads. Borrowed, so the CLI passes its own locals
/// without cloning.
///
/// `#[non_exhaustive]` (§2.3.1): this type is re-exported from `prism::api`, so
/// a new input field must not break an embedder. Build one with
/// [`SarifInputs::new`] plus the chainable setters — every field a caller does
/// not set defaults to empty, and `findings` alone is a valid (if empty)
/// document's worth of input:
///
/// ```
/// use prism::api::{to_sarif, SarifInputs};
///
/// let doc = to_sarif(&SarifInputs::new(&[]).algorithms_run(&["AbsenceSlice".to_string()]));
/// assert_eq!(doc["runs"][0]["properties"]["algorithms_run"][0], "AbsenceSlice");
/// ```
#[non_exhaustive]
pub struct SarifInputs<'a> {
    pub findings: &'a [SliceFinding],
    pub errors: &'a [AlgorithmError],
    pub parse_warnings: &'a [String],
    /// Files skipped at load (design §2.3.2); one `warning` notification each.
    pub load_warnings: &'a [String],
    /// Non-fatal build conditions — `api::build_context`'s `BuiltContext
    /// .warnings` (a cache-save failure, a type-database load failure and its
    /// fallback). One `warning` notification each, text verbatim, so what the
    /// run printed to stderr is also in the document a consumer reads.
    pub build_warnings: &'a [String],
    pub algorithms_run: &'a [String],
    /// SPARSE authoritative per-file grade map (a file appears only when its
    /// error rate exceeds 1%). Absent from it + present in `files` = clean;
    /// absent from both = unknown.
    pub parse_quality: &'a BTreeMap<String, FileParseQuality>,
    pub files: &'a BTreeMap<String, ParsedFile>,
    pub sources: &'a BTreeMap<String, String>,
}

/// Empty borrowed maps for the builder's defaults. `const` rather than
/// `static`: the values are empty, so materialising one per use site costs
/// nothing, and a `const` puts no `Sync` bound on the value types.
const NO_PARSE_QUALITY: &BTreeMap<String, FileParseQuality> = &BTreeMap::new();
const NO_FILES: &BTreeMap<String, ParsedFile> = &BTreeMap::new();
const NO_SOURCES: &BTreeMap<String, String> = &BTreeMap::new();

impl<'a> SarifInputs<'a> {
    /// The findings to serialize; every other input defaults to empty.
    pub fn new(findings: &'a [SliceFinding]) -> Self {
        Self {
            findings,
            errors: &[],
            parse_warnings: &[],
            load_warnings: &[],
            build_warnings: &[],
            algorithms_run: &[],
            parse_quality: NO_PARSE_QUALITY,
            files: NO_FILES,
            sources: NO_SOURCES,
        }
    }

    /// Algorithms that failed; one `error` notification each, and
    /// `executionSuccessful` is false when any is present.
    pub fn errors(mut self, errors: &'a [AlgorithmError]) -> Self {
        self.errors = errors;
        self
    }

    pub fn parse_warnings(mut self, parse_warnings: &'a [String]) -> Self {
        self.parse_warnings = parse_warnings;
        self
    }

    pub fn load_warnings(mut self, load_warnings: &'a [String]) -> Self {
        self.load_warnings = load_warnings;
        self
    }

    pub fn build_warnings(mut self, build_warnings: &'a [String]) -> Self {
        self.build_warnings = build_warnings;
        self
    }

    pub fn algorithms_run(mut self, algorithms_run: &'a [String]) -> Self {
        self.algorithms_run = algorithms_run;
        self
    }

    pub fn parse_quality(mut self, parse_quality: &'a BTreeMap<String, FileParseQuality>) -> Self {
        self.parse_quality = parse_quality;
        self
    }

    pub fn files(mut self, files: &'a BTreeMap<String, ParsedFile>) -> Self {
        self.files = files;
        self
    }

    pub fn sources(mut self, sources: &'a BTreeMap<String, String>) -> Self {
        self.sources = sources;
        self
    }
}

// --- Document construction ------------------------------------------------

/// Build the deterministic SARIF 2.1 document. Total over its inputs: no
/// finding, error or path shape makes it fail.
pub fn to_sarif(inputs: &SarifInputs) -> serde_json::Value {
    // Paths that escape the repo root, collected while building locations so
    // related files are covered too. Sorted + deduplicated by BTreeSet.
    let mut escaping: BTreeSet<String> = BTreeSet::new();
    let mut rule_meta: BTreeMap<String, (String, String)> = BTreeMap::new();

    let mut results: Vec<SarifResult> = inputs
        .findings
        .iter()
        .map(|finding| {
            let result = build_result(finding, inputs, &mut escaping);
            let category = finding.category.as_deref().unwrap_or(UNCATEGORIZED);
            rule_meta
                .entry(result.rule_id.clone())
                .or_insert_with(|| (finding.algorithm.clone(), category.to_string()));
            result
        })
        .collect();

    results.sort_by(|a, b| sort_key(a).cmp(&sort_key(b)));

    // `rule_meta` is a BTreeMap, so this is already sorted by id.
    let rule_index: BTreeMap<&str, usize> = rule_meta
        .keys()
        .enumerate()
        .map(|(i, id)| (id.as_str(), i))
        .collect();
    for result in results.iter_mut() {
        // Every result registered its own rule id in `rule_meta` above, so the
        // lookup cannot miss; a silent `unwrap_or(0)` would mislabel a result
        // with another rule's index instead of failing.
        result.rule_index = *rule_index
            .get(result.rule_id.as_str())
            .expect("every result's rule id was registered");
    }
    let rules: Vec<Rule> = rule_meta
        .iter()
        .map(|(id, (algorithm, category))| Rule {
            id: id.clone(),
            name: category.clone(),
            short_description: Text {
                text: format!("{algorithm}: {category}"),
            },
            full_description: Text {
                text: rule_description(algorithm, category),
            },
            properties: RuleProperties {
                algorithm: algorithm.clone(),
                category: category.clone(),
            },
        })
        .collect();

    let mut notifications: Vec<Notification> = Vec::new();
    for error in inputs.errors {
        notifications.push(notification(
            "error",
            format!("{}: {}", error.algorithm, error.error),
        ));
    }
    for warning in inputs
        .parse_warnings
        .iter()
        .chain(inputs.load_warnings)
        .chain(inputs.build_warnings)
    {
        notifications.push(notification("warning", warning.clone()));
    }
    for path in &escaping {
        notifications.push(notification(
            "warning",
            format!("path escapes repo root: {path}"),
        ));
    }

    let log = SarifLog {
        schema: SCHEMA,
        version: "2.1.0",
        runs: vec![Run {
            tool: Tool {
                driver: Driver {
                    name: "prism",
                    version: env!("CARGO_PKG_VERSION"),
                    semantic_version: env!("CARGO_PKG_VERSION"),
                    information_uri: INFORMATION_URI,
                    rules,
                },
            },
            invocations: vec![Invocation {
                execution_successful: inputs.errors.is_empty(),
                tool_execution_notifications: notifications,
            }],
            results,
            properties: RunProperties {
                mapping_version: MAPPING_VERSION,
                algorithms_run: inputs.algorithms_run.to_vec(),
                resolution_mode: RESOLUTION_MODE,
                errors: inputs.errors.to_vec(),
                prism_build_identity: crate::cpg_cache::current_cache_build_identity(),
                prism_git_sha: env!("GIT_SHA"),
                binary_input_dirty: crate::cpg_cache::binary_input_dirty(),
            },
        }],
    };
    // The model holds only strings, numbers, bools and Vecs of the same — it
    // has no failing `Serialize` path. Returning `Value::Null` on an
    // "impossible" error would print `null` and exit 0: a silent failure.
    serde_json::to_value(&log).expect("SARIF model is always serializable")
}

fn notification(level: &'static str, text: String) -> Notification {
    Notification {
        level,
        message: Text { text },
    }
}

/// `(uri, startLine, ruleId, message.text)` — §2.2.4.
fn sort_key(result: &SarifResult) -> (&str, usize, &str, &str) {
    let physical = &result.locations[0].physical_location;
    (
        physical.artifact_location.uri.as_str(),
        physical.region.as_ref().map_or(0, |r| r.start_line),
        result.rule_id.as_str(),
        result.message.text.as_str(),
    )
}

fn physical_location(
    path: &str,
    line: Option<usize>,
    escaping: &mut BTreeSet<String>,
) -> PhysicalLocation {
    let (uri, escapes) = sarif_uri(path);
    if escapes {
        escaping.insert(path.to_string());
    }
    PhysicalLocation {
        artifact_location: ArtifactLocation {
            uri,
            uri_base_id: SRCROOT,
        },
        region: line.map(|start_line| Region { start_line }),
    }
}

fn build_result(
    finding: &SliceFinding,
    inputs: &SarifInputs,
    escaping: &mut BTreeSet<String>,
) -> SarifResult {
    let category = finding.category.as_deref().unwrap_or(UNCATEGORIZED);
    let quality = parse_quality_for(finding, inputs.parse_quality, inputs.files);
    let (confidence, tier) = classify(&finding.algorithm, quality);
    let line_text = line_text_of(inputs.sources, &finding.file, finding.line);

    let region = (finding.line >= 1).then_some(finding.line);
    let primary = physical_location(&finding.file, region, escaping);
    let logical_locations = finding
        .function_name
        .iter()
        .map(|name| LogicalLocation {
            name: name.clone(),
            kind: "function",
        })
        .collect();

    let (related_locations, unattributed_lines) = related(finding, escaping);

    let mut partial_fingerprints = BTreeMap::new();
    partial_fingerprints.insert(FINGERPRINT_KEY, fingerprint(finding, &line_text));

    SarifResult {
        rule_id: format!("prism/{}/{}", finding.algorithm, category),
        // Placeholder: assigned in `to_sarif` once results and rules are sorted.
        rule_index: 0,
        level: level_for_severity(&finding.severity),
        message: Text {
            text: finding.description.clone(),
        },
        locations: vec![Location {
            physical_location: primary,
            logical_locations,
        }],
        related_locations,
        partial_fingerprints,
        properties: ResultProperties {
            algorithm: finding.algorithm.clone(),
            category: category.to_string(),
            severity: finding.severity.clone(),
            confidence,
            tier,
            resolution_mode: RESOLUTION_MODE,
            parse_quality: quality.as_str(),
            function_name: finding.function_name.clone(),
            related_files: finding.related_files.clone(),
            related_lines: unattributed_lines,
        },
    }
}

/// `(relatedLocations, lines that could not be attributed)` per §2.2.2.
fn related(
    finding: &SliceFinding,
    escaping: &mut BTreeSet<String>,
) -> (Vec<RelatedLocation>, Vec<usize>) {
    let mut lines: Vec<usize> = finding.related_lines.clone();
    lines.retain(|line| *line > 0);
    lines.sort_unstable();
    lines.dedup();

    let host: Option<&str> = match attribution(&finding.algorithm) {
        Attribution::SameFile => Some(finding.file.as_str()),
        Attribution::CounterpartFile => finding.related_files.first().map(String::as_str),
        Attribution::Ambiguous if finding.related_files.is_empty() => Some(finding.file.as_str()),
        Attribution::Ambiguous => None,
    };

    let mut locations: Vec<RelatedLocation> = Vec::new();
    if let Some(host) = host {
        for line in &lines {
            locations.push(RelatedLocation {
                id: locations.len(),
                physical_location: physical_location(host, Some(*line), escaping),
            });
        }
    }
    let unattributed = if host.is_some() { Vec::new() } else { lines };

    // A file that already carries line locations is not repeated region-less.
    let line_host = (!locations.is_empty()).then_some(host).flatten();
    let mut files: Vec<&str> = finding
        .related_files
        .iter()
        .map(String::as_str)
        .filter(|path| Some(*path) != line_host)
        .collect();
    files.sort_unstable();
    files.dedup();
    for path in files {
        locations.push(RelatedLocation {
            id: locations.len(),
            physical_location: physical_location(path, None, escaping),
        });
    }

    (locations, unattributed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn finding(algorithm: &str, file: &str, line: usize) -> SliceFinding {
        SliceFinding {
            algorithm: algorithm.to_string(),
            file: file.to_string(),
            line,
            severity: "warning".to_string(),
            description: "d".to_string(),
            function_name: Some("f".to_string()),
            related_lines: vec![],
            related_files: vec![],
            category: Some("c".to_string()),
            parse_quality: None,
            diagrams: vec![],
        }
    }

    fn doc_of(f: SliceFinding) -> Value {
        to_sarif(&SarifInputs::new(&[f]))
    }

    /// `"<uri>:<startLine> id=<n>"` of a `location` / `relatedLocation`.
    /// A `-` line means the `region` KEY is absent, not a null `startLine`.
    fn at(location: &Value, id: u64) -> String {
        let physical = &location["physicalLocation"];
        let line = physical.get("region").map_or("-".to_string(), |r| {
            r["startLine"]
                .as_u64()
                .expect("region without a startLine")
                .to_string()
        });
        let uri = physical["artifactLocation"]["uri"].as_str().unwrap();
        format!("{uri}:{line} id={id}")
    }

    /// `(relatedLocations rendered by `at`, properties)` of the one result.
    fn related_and_properties(f: SliceFinding) -> (Vec<String>, Value) {
        let doc = doc_of(f);
        let result = &doc["runs"][0]["results"][0];
        let related = result["relatedLocations"]
            .as_array()
            .map(|ls| {
                ls.iter()
                    .map(|l| at(l, l["id"].as_u64().unwrap()))
                    .collect()
            })
            .unwrap_or_default();
        (related, result["properties"].clone())
    }

    /// §7.2.6 (a): SARIF requires `startLine >= 1`, so line 0 omits the whole
    /// `region` KEY — not `"region": {"startLine": null}`, which would fail
    /// schema validation while looking the same to a `.as_u64()` probe.
    #[test]
    fn line_zero_omits_the_region() {
        let zero = doc_of(finding("symmetry", "a.py", 0));
        let physical = &zero["runs"][0]["results"][0]["locations"][0]["physicalLocation"];
        assert_eq!(physical["artifactLocation"]["uri"], "a.py");
        assert!(
            physical.get("region").is_none(),
            "the region key itself must be absent: {physical:#?}"
        );

        let one = doc_of(finding("symmetry", "a.py", 1));
        let location = &one["runs"][0]["results"][0]["locations"][0];
        assert_eq!(at(location, 0), "a.py:1 id=0");
    }

    /// §7.2.6 (b): SameFile attribution — lines land in `finding.file`,
    /// sorted, deduplicated, zeros dropped, ids `0..n`; the related FILE is a
    /// separate region-less location.
    #[test]
    fn same_file_attribution_locates_lines_in_the_finding_file() {
        let mut f = finding("absence", "a.py", 1);
        f.related_lines = vec![5, 0, 3, 5];
        f.related_files = vec!["b.py".to_string()];
        let (related, properties) = related_and_properties(f);
        assert_eq!(related, ["a.py:3 id=0", "a.py:5 id=1", "b.py:- id=2"]);
        assert!(
            properties.get("related_lines").is_none(),
            "every line was attributed, so nothing spills into properties"
        );
        assert_eq!(properties["related_files"], serde_json::json!(["b.py"]));
    }

    /// §7.2.6 (c): CounterpartFile attribution — symmetry's lines belong to
    /// the counterpart file, never to the anchor file.
    #[test]
    fn counterpart_attribution_locates_lines_in_the_related_file() {
        let mut f = finding("symmetry", "a.py", 1);
        f.related_lines = vec![10, 20];
        f.related_files = vec!["b.py".to_string()];
        let (related, properties) = related_and_properties(f);
        assert_eq!(related, ["b.py:10 id=0", "b.py:20 id=1"]);
        assert!(properties.get("related_lines").is_none());
    }

    /// §7.2.6 (d): Ambiguous attribution — with candidate files the lines are
    /// NOT guessed onto one, they are preserved in `properties`; with no
    /// candidate file the finding's own file is the only possibility.
    #[test]
    fn ambiguous_attribution_preserves_lines_it_cannot_place() {
        let mut f = finding("callback_dispatcher", "a.py", 1);
        f.related_lines = vec![7, 4, 4];
        f.related_files = vec!["c.py".to_string(), "b.py".to_string()];
        let (related, properties) = related_and_properties(f);
        assert_eq!(
            related,
            ["b.py:- id=0", "c.py:- id=1"],
            "files only, sorted"
        );
        assert_eq!(
            properties["related_lines"],
            serde_json::json!([4, 7]),
            "unattributable lines are sorted, deduplicated and kept"
        );

        let mut unknown = finding("brand_new_algo", "a.py", 1);
        unknown.related_lines = vec![9];
        let (related, properties) = related_and_properties(unknown);
        assert_eq!(related, ["a.py:9 id=0"]);
        assert!(properties.get("related_lines").is_none());
    }

    /// Repo-root escape detection must be HOST-INDEPENDENT (final review,
    /// terra #2). A Windows drive-rooted or UNC path is absolute everywhere,
    /// but `Path::is_absolute()` answers `false` for both on a Unix host — so
    /// the old predicate emitted `C:/outside/a.py` under `%SRCROOT%` as if it
    /// were repo-relative, with no warning, whenever prism ran on Unix.
    #[test]
    fn escape_detection_is_host_independent() {
        // The four escaping shapes, at the shared predicate `targets` uses too.
        for escaping in [
            "C:/outside/a.py",
            "//srv/share/x.py",
            "/abs/x.py",
            "a/../b.py",
        ] {
            assert!(
                path_escapes_repo(escaping),
                "{escaping} escapes the repo root on every host"
            );
            assert!(
                sarif_uri(escaping).1,
                "{escaping} must be flagged by sarif_uri too"
            );
        }
        assert!(
            sarif_uri("C:\\outside\\a.py").1,
            "…including after backslash normalisation"
        );

        for inside in ["a/b.py", "a..b/x.py", "c:x.py", "C:", "src/mod.rs", ""] {
            assert!(
                !path_escapes_repo(inside),
                "{inside:?} stays inside the repo"
            );
        }
    }

    /// The build warnings `api::build_context` returns are notifications too
    /// (final review, terra #1): a non-fatal cache or type-database condition
    /// must be visible to a consumer reading only the document.
    #[test]
    fn build_warnings_become_warning_notifications() {
        let build = ["Warning: failed to write CPG cache: read-only".to_string()];
        let doc = to_sarif(&SarifInputs::new(&[]).build_warnings(&build));
        let notes = doc["runs"][0]["invocations"][0]["toolExecutionNotifications"]
            .as_array()
            .expect("a build warning produces a notification")
            .clone();
        assert_eq!(notes.len(), 1, "one notification per warning: {notes:#?}");
        assert_eq!(notes[0]["level"], "warning");
        assert_eq!(notes[0]["message"]["text"], build[0].as_str());
        assert_eq!(
            doc["runs"][0]["invocations"][0]["executionSuccessful"], true,
            "a non-fatal build condition is not an execution failure"
        );
    }
}
