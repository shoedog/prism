use super::output::McpToolResult;
use crate::navigation::types::{Warning, WarningKind};
use crate::repo_loader::LoadedRepo;
use serde_json::{Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::SystemTime;

pub(crate) const FRESHNESS_RESERVE_BYTES: usize = 4096;
pub(crate) const MAX_STALE_PATHS: usize = 5;
pub(crate) const MAX_STALE_PATH_BYTES: usize = 360;

const FRESHNESS_STATUS_KEY: &str = "prism/index_freshness";
const STALE_TOTAL_KEY: &str = "prism/stale_index_total";
const STALE_PATHS_KEY: &str = "prism/stale_index_paths";
const CONTENT_FORMAT_KEY: &str = "prism/content_text_format";
const VIEW_CLIPPED_KEY: &str = "prism/view_clipped";
const MAX_RESULT_KEY: &str = "anthropic/maxResultSizeChars";
const AGENT_JSON_FORMAT: &str = "agent_json";
const AGENT_MARKDOWN_FORMAT: &str = "agent_markdown";

#[derive(Debug)]
pub struct FreshnessProbe {
    root: PathBuf,
    tracked: BTreeMap<String, PathStamp>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FreshnessReport {
    pub stale: bool,
    pub total_changed: usize,
    pub changed_paths: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct PathStamp {
    kind: PathKind,
    len: Option<u64>,
    modified: Option<SystemTime>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PathKind {
    File,
    Directory,
    Missing,
    Other,
    Unreadable,
}

impl FreshnessProbe {
    pub fn from_loaded_repo(repo: &LoadedRepo) -> Self {
        let mut paths = BTreeSet::new();
        paths.insert(".".to_string());

        for rel in repo.files.keys().chain(repo.manifest_hashes.keys()) {
            paths.insert(rel.clone());
            add_ancestors(rel, &mut paths);
        }

        let tracked = paths
            .into_iter()
            .map(|rel| {
                let stamp = PathStamp::read(&repo.root, &rel);
                (rel, stamp)
            })
            .collect();

        Self {
            root: repo.root.clone(),
            tracked,
        }
    }

    pub fn check(&self) -> FreshnessReport {
        let mut total_changed = 0usize;
        let mut changed_paths = Vec::new();
        let mut path_bytes = 0usize;

        for (rel, before) in &self.tracked {
            let after = PathStamp::read(&self.root, rel);
            if before == &after {
                continue;
            }

            total_changed += 1;
            if changed_paths.len() >= MAX_STALE_PATHS || path_bytes >= MAX_STALE_PATH_BYTES {
                continue;
            }
            if let Some(display) = bounded_display_path(rel, MAX_STALE_PATH_BYTES - path_bytes) {
                path_bytes += display.len();
                changed_paths.push(display);
            }
        }

        FreshnessReport {
            stale: total_changed > 0,
            total_changed,
            changed_paths,
        }
    }

    pub(crate) fn tracked_len(&self) -> usize {
        self.tracked.len()
    }
}

impl FreshnessReport {
    #[cfg(test)]
    fn stale_for_tests(total_changed: usize, changed_paths: Vec<&str>) -> Self {
        Self {
            stale: total_changed > 0,
            total_changed,
            changed_paths: changed_paths.into_iter().map(str::to_owned).collect(),
        }
    }
}

impl PathStamp {
    fn read(root: &Path, rel: &str) -> Self {
        let path = if rel == "." {
            root.to_path_buf()
        } else {
            root.join(rel)
        };
        match fs::metadata(&path) {
            Ok(metadata) => {
                let file_type = metadata.file_type();
                let kind = if file_type.is_file() {
                    PathKind::File
                } else if file_type.is_dir() {
                    PathKind::Directory
                } else {
                    PathKind::Other
                };
                Self {
                    kind,
                    len: Some(metadata.len()),
                    modified: metadata.modified().ok(),
                }
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Self {
                kind: PathKind::Missing,
                len: None,
                modified: None,
            },
            Err(_) => Self {
                kind: PathKind::Unreadable,
                len: None,
                modified: None,
            },
        }
    }
}

pub(crate) fn apply_freshness_report(
    result: &mut McpToolResult,
    report: &FreshnessReport,
    original_cap: usize,
) {
    if !report.stale {
        return;
    }

    let warning = stale_warning(report);
    let warning_value = serde_json::to_value(&warning).expect("warning serializes");

    result
        .meta
        .insert(FRESHNESS_STATUS_KEY.into(), Value::String("stale".into()));
    result.meta.insert(
        STALE_TOTAL_KEY.into(),
        Value::Number(serde_json::Number::from(report.total_changed)),
    );
    result.meta.insert(
        STALE_PATHS_KEY.into(),
        Value::Array(
            report
                .changed_paths
                .iter()
                .cloned()
                .map(Value::String)
                .collect(),
        ),
    );
    result.meta.insert(
        MAX_RESULT_KEY.into(),
        Value::Number(serde_json::Number::from(original_cap)),
    );

    if let Some(structured) = &mut result.structured {
        upsert_stale_warning(structured, &warning_value);
    }

    let format = result.meta.get(CONTENT_FORMAT_KEY).and_then(Value::as_str);
    let clipped = result
        .meta
        .get(VIEW_CLIPPED_KEY)
        .and_then(Value::as_bool)
        .unwrap_or(false);

    match format {
        Some(AGENT_JSON_FORMAT) => {
            if !clipped {
                apply_agent_json_text(result, &warning_value);
            }
        }
        Some(AGENT_MARKDOWN_FORMAT) => {
            if !clipped {
                result.content_text =
                    apply_agent_markdown_text(&result.content_text, &warning.message);
            }
        }
        _ => {
            if let Some(structured) = &result.structured {
                if let Ok(text) = serde_json::to_string_pretty(structured) {
                    result.content_text = text;
                }
            }
        }
    }
}

pub(crate) fn stale_warning(report: &FreshnessReport) -> Warning {
    Warning {
        kind: WarningKind::StaleIndex,
        message: stale_message(report),
        location: None,
    }
}

fn stale_message(report: &FreshnessReport) -> String {
    let mut message = format!(
        "MCP index may be stale; {} tracked {} changed since the current MCP snapshot was loaded",
        report.total_changed,
        if report.total_changed == 1 {
            "path"
        } else {
            "paths"
        }
    );
    if !report.changed_paths.is_empty() {
        message.push_str(": ");
        message.push_str(&report.changed_paths.join(", "));
    }
    let omitted = report
        .total_changed
        .saturating_sub(report.changed_paths.len());
    if omitted > 0 {
        message.push_str(&format!(" ({omitted} more omitted)"));
    }
    message.push_str(
        ". If this server exposes refresh_index, call it; otherwise restart/re-add the MCP server or use CLI nav for a fresh snapshot.",
    );
    message
}

fn apply_agent_json_text(result: &mut McpToolResult, warning: &Value) {
    let Ok(mut value) = serde_json::from_str::<Value>(&result.content_text) else {
        return;
    };
    let count = upsert_stale_warning(&mut value, warning);
    if let Some(summary) = value
        .as_object_mut()
        .and_then(|obj| obj.get_mut("summary"))
        .and_then(Value::as_object_mut)
    {
        summary.insert(
            "warnings".into(),
            Value::Number(serde_json::Number::from(count)),
        );
    }
    if let Ok(text) = serde_json::to_string_pretty(&value) {
        result.content_text = text;
    }
}

fn apply_agent_markdown_text(content: &str, message: &str) -> String {
    let section = format!("## Freshness\n\n{message}");
    let Some(start) = content.find("## Freshness") else {
        let separator = if content.ends_with('\n') {
            "\n"
        } else {
            "\n\n"
        };
        return format!("{content}{separator}{section}\n");
    };
    let after_header = start + "## Freshness".len();
    let end = content[after_header..]
        .find("\n## ")
        .map(|offset| after_header + offset + 1)
        .unwrap_or(content.len());
    let mut updated = String::new();
    updated.push_str(&content[..start]);
    updated.push_str(&section);
    updated.push('\n');
    updated.push_str(&content[end..]);
    updated
}

fn upsert_stale_warning(value: &mut Value, warning: &Value) -> usize {
    let Some(obj) = value.as_object_mut() else {
        return 0;
    };
    let warnings = warnings_array(obj);
    if let Some(existing) = warnings
        .iter_mut()
        .find(|candidate| is_stale_warning(candidate))
    {
        *existing = warning.clone();
    } else {
        warnings.push(warning.clone());
    }
    warnings.len()
}

fn warnings_array(obj: &mut Map<String, Value>) -> &mut Vec<Value> {
    let entry = obj
        .entry("warnings")
        .or_insert_with(|| Value::Array(Vec::new()));
    if !entry.is_array() {
        *entry = Value::Array(Vec::new());
    }
    entry.as_array_mut().expect("warnings is array")
}

fn is_stale_warning(value: &Value) -> bool {
    value
        .as_object()
        .and_then(|obj| obj.get("kind"))
        .and_then(Value::as_str)
        == Some("StaleIndex")
}

fn add_ancestors(rel: &str, paths: &mut BTreeSet<String>) {
    let mut path = Path::new(rel);
    while let Some(parent) = path.parent() {
        if parent.as_os_str().is_empty() {
            paths.insert(".".to_string());
            break;
        }
        paths.insert(path_to_repo_string(parent));
        path = parent;
    }
}

fn path_to_repo_string(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/")
}

fn bounded_display_path(path: &str, remaining: usize) -> Option<String> {
    if remaining == 0 {
        return None;
    }
    let sanitized = path
        .chars()
        .map(|ch| if ch.is_control() { '?' } else { ch })
        .collect::<String>();
    if sanitized.len() <= remaining {
        return Some(sanitized);
    }
    if remaining <= 3 {
        return None;
    }
    let limit = remaining - 3;
    let mut end = 0usize;
    for (idx, ch) in sanitized.char_indices() {
        let next = idx + ch.len_utf8();
        if next > limit {
            break;
        }
        end = next;
    }
    Some(format!("{}...", &sanitized[..end]))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mcp::output::SCHEMA_VERSION;
    use crate::repo_loader::load_repo;
    use serde_json::json;

    fn loaded_probe(dir: &Path) -> FreshnessProbe {
        let repo = load_repo(dir).expect("load repo");
        FreshnessProbe::from_loaded_repo(&repo)
    }

    fn base_result() -> McpToolResult {
        let structured = json!({
            "query": "nodes-at:a.py:1",
            "items": [],
            "truncated": false,
            "warnings": []
        });
        McpToolResult {
            content_text: serde_json::to_string_pretty(&structured).unwrap(),
            structured: Some(structured),
            is_error: false,
            meta: [
                (
                    "prism/schema_version".into(),
                    Value::String(SCHEMA_VERSION.into()),
                ),
                (
                    MAX_RESULT_KEY.into(),
                    Value::Number(serde_json::Number::from(80_000usize)),
                ),
            ]
            .into_iter()
            .collect(),
        }
    }

    #[test]
    fn probe_is_fresh_immediately() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("a.py"), "def f():\n    return 1\n").unwrap();
        let probe = loaded_probe(dir.path());
        assert_eq!(
            probe.check(),
            FreshnessReport {
                stale: false,
                total_changed: 0,
                changed_paths: vec![]
            }
        );
    }

    #[test]
    fn editing_indexed_file_marks_stale() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.py");
        fs::write(&path, "def f():\n    return 1\n").unwrap();
        let probe = loaded_probe(dir.path());
        fs::write(&path, "def f():\n    return 12345\n").unwrap();
        let report = probe.check();
        assert!(report.stale);
        assert_eq!(report.total_changed, 1);
        assert_eq!(report.changed_paths, ["a.py"]);
    }

    #[test]
    fn deleting_indexed_file_marks_stale() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("a.py");
        fs::write(&path, "def f():\n    return 1\n").unwrap();
        let probe = loaded_probe(dir.path());
        fs::remove_file(path).unwrap();
        let report = probe.check();
        assert!(report.stale);
        assert!(report.changed_paths.contains(&"a.py".to_string()));
    }

    #[test]
    fn editing_manifest_marks_stale() {
        let dir = tempfile::tempdir().unwrap();
        fs::write(dir.path().join("Cargo.toml"), "[package]\nname=\"a\"\n").unwrap();
        fs::write(dir.path().join("lib.rs"), "fn f() {}\n").unwrap();
        let probe = loaded_probe(dir.path());
        fs::write(
            dir.path().join("Cargo.toml"),
            "[package]\nname=\"changed\"\nversion=\"0.1.0\"\n",
        )
        .unwrap();
        let report = probe.check();
        assert!(report.stale);
        assert!(report.changed_paths.contains(&"Cargo.toml".to_string()));
    }

    #[test]
    fn adding_file_in_tracked_directory_marks_stale() {
        let dir = tempfile::tempdir().unwrap();
        fs::create_dir_all(dir.path().join("pkg")).unwrap();
        fs::write(dir.path().join("pkg/a.py"), "def f():\n    return 1\n").unwrap();
        let probe = loaded_probe(dir.path());
        fs::write(dir.path().join("pkg/b.py"), "def g():\n    return 2\n").unwrap();
        let report = probe.check();
        assert!(report.stale);
        assert!(report.changed_paths.contains(&"pkg".to_string()));
    }

    #[test]
    fn changed_paths_are_sorted_and_capped() {
        let dir = tempfile::tempdir().unwrap();
        let mut tracked = BTreeMap::new();
        for idx in 0..10 {
            tracked.insert(
                format!("path-{idx}.py"),
                PathStamp {
                    kind: PathKind::Missing,
                    len: None,
                    modified: None,
                },
            );
        }
        let probe = FreshnessProbe {
            root: dir.path().to_path_buf(),
            tracked,
        };
        for idx in 0..10 {
            fs::write(dir.path().join(format!("path-{idx}.py")), "x=1\n").unwrap();
        }
        let report = probe.check();
        assert_eq!(report.total_changed, 10);
        assert_eq!(report.changed_paths.len(), MAX_STALE_PATHS);
        assert_eq!(
            report.changed_paths,
            [
                "path-0.py",
                "path-1.py",
                "path-2.py",
                "path-3.py",
                "path-4.py"
            ]
        );
        assert!(
            report.changed_paths.iter().map(String::len).sum::<usize>() <= MAX_STALE_PATH_BYTES
        );
    }

    #[test]
    fn apply_report_updates_meta_structured_and_canonical_text() {
        let mut result = base_result();
        let report = FreshnessReport::stale_for_tests(1, vec!["a.py"]);
        apply_freshness_report(&mut result, &report, 80_000);
        assert_eq!(result.meta[FRESHNESS_STATUS_KEY], "stale");
        assert_eq!(result.meta[STALE_TOTAL_KEY], 1);
        assert_eq!(result.meta[STALE_PATHS_KEY], json!(["a.py"]));
        let structured = result.structured.as_ref().unwrap();
        assert_eq!(structured["warnings"][0]["kind"], "StaleIndex");
        let text: Value = serde_json::from_str(&result.content_text).unwrap();
        assert_eq!(text["warnings"][0]["kind"], "StaleIndex");
    }

    #[test]
    fn apply_report_is_idempotent() {
        let mut result = base_result();
        let report = FreshnessReport::stale_for_tests(1, vec!["a.py"]);
        apply_freshness_report(&mut result, &report, 80_000);
        apply_freshness_report(&mut result, &report, 80_000);
        let warnings = result.structured.as_ref().unwrap()["warnings"]
            .as_array()
            .unwrap();
        assert_eq!(warnings.len(), 1);
    }

    #[test]
    fn apply_report_updates_agent_json_warning_count() {
        let mut result = base_result();
        result.meta.insert(
            CONTENT_FORMAT_KEY.into(),
            Value::String(AGENT_JSON_FORMAT.into()),
        );
        result.content_text = serde_json::to_string(&json!({
            "summary": { "warnings": 0 },
            "warnings": []
        }))
        .unwrap();
        let report = FreshnessReport::stale_for_tests(1, vec!["a.py"]);
        apply_freshness_report(&mut result, &report, 80_000);
        let text: Value = serde_json::from_str(&result.content_text).unwrap();
        assert_eq!(text["warnings"][0]["kind"], "StaleIndex");
        assert_eq!(text["summary"]["warnings"], 1);
    }

    #[test]
    fn clipped_agent_json_text_is_left_unchanged() {
        let mut result = base_result();
        result.meta.insert(
            CONTENT_FORMAT_KEY.into(),
            Value::String(AGENT_JSON_FORMAT.into()),
        );
        result
            .meta
            .insert(VIEW_CLIPPED_KEY.into(), Value::Bool(true));
        result.content_text = "Evidence view clipped".into();
        let report = FreshnessReport::stale_for_tests(1, vec!["a.py"]);
        apply_freshness_report(&mut result, &report, 80_000);
        assert_eq!(result.content_text, "Evidence view clipped");
        assert_eq!(
            result.structured.as_ref().unwrap()["warnings"][0]["kind"],
            "StaleIndex"
        );
    }

    #[test]
    fn apply_report_appends_agent_markdown_section() {
        let mut result = base_result();
        result.meta.insert(
            CONTENT_FORMAT_KEY.into(),
            Value::String(AGENT_MARKDOWN_FORMAT.into()),
        );
        result.content_text = "# Evidence\n\ncontent\n".into();
        let report = FreshnessReport::stale_for_tests(1, vec!["a.py"]);
        apply_freshness_report(&mut result, &report, 80_000);
        assert!(result.content_text.contains("## Freshness"));
        assert!(result.content_text.contains("MCP index may be stale"));
    }

    #[test]
    fn maximum_stale_payload_growth_fits_reserve() {
        for format in [None, Some(AGENT_JSON_FORMAT), Some(AGENT_MARKDOWN_FORMAT)] {
            let mut result = base_result();
            if let Some(format) = format {
                result
                    .meta
                    .insert(CONTENT_FORMAT_KEY.into(), Value::String(format.into()));
                if format == AGENT_JSON_FORMAT {
                    result.content_text = serde_json::to_string(&json!({
                        "summary": { "warnings": 0 },
                        "warnings": []
                    }))
                    .unwrap();
                } else {
                    result.content_text = "# Evidence\n\ncontent\n".into();
                }
            }
            let before = result.serialized_len();
            let paths = vec![
                "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa.py",
                "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb.py",
                "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc.py",
                "dddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddddd.py",
                "eeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee.py",
            ];
            let report = FreshnessReport::stale_for_tests(20, paths);
            apply_freshness_report(&mut result, &report, 80_000);
            let growth = result.serialized_len().saturating_sub(before);
            assert!(
                growth <= FRESHNESS_RESERVE_BYTES,
                "growth {growth} must fit reserve {FRESHNESS_RESERVE_BYTES}"
            );
        }
    }
}
