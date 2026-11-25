use serde::Deserialize;
use serde_json::Value;
use std::collections::HashSet;
use std::path::{Component, Path};

pub(super) const FIXTURE_REPO: &str = "tests/targets_inventory/py";
const ROWS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/targets_inventory/rows.jsonl"
));
const GROUPS: &[&str] = &["http.root", "http.receiver", "stdlib"];
const ENABLED_GROUPS: &[&str] = &[];
const FAMILIES: &[&str] = &["http", "filesystem", "parse", "config"];
const REASONS: &[&str] = &[
    "parse_error_in_function",
    "module_level",
    "class_body_frame",
    "lambda_frame",
    "multiline_callee",
    "not_line_anchor",
];
const CAUSES: &[&str] = &[
    "star_import",
    "shadowed_root",
    "not_imported",
    "repository_local_module",
    "binding_unavailable",
    "member_rebound",
    "receiver_not_unique",
    "annotation_conflict",
    "helper_unannotated",
    "helper_not_admissible",
    "receiver_value_unproven",
    "receiver_is_none",
];

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Row {
    id: String,
    group: String,
    fixture: String,
    line: u64,
    callee: String,
    #[serde(rename = "class")]
    census_class: String,
    want: String,
    truth: String,
    consumer: String,
}

enum Want<'a> {
    Site {
        family: &'a str,
        callee: &'a str,
        grade: &'a str,
        symbol: Option<&'a str>,
        bounds: Option<(u64, u64)>,
    },
    Unsupported {
        reason: &'a str,
        count: u64,
    },
    None(Option<&'a str>),
}

impl Row {
    fn expectation(&self) -> Want<'_> {
        parse_want(&self.want)
            .unwrap_or_else(|| panic!("{} has invalid want {:?}", self.id, self.want))
    }
}

fn read_rows() -> Vec<Row> {
    ROWS.lines()
        .enumerate()
        .map(|(index, line)| {
            serde_json::from_str(line)
                .unwrap_or_else(|err| panic!("rows.jsonl line {}: {err}", index + 1))
        })
        .collect()
}

fn identifier(value: &str) -> bool {
    let mut chars = value.chars();
    chars
        .next()
        .is_some_and(|ch| ch == '_' || ch.is_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_alphanumeric())
}

fn dotted_callee(value: &str) -> bool {
    value.contains('.') && value.split('.').all(identifier)
}

fn positive_integer(value: &str) -> Option<u64> {
    if !value.bytes().all(|byte| byte.is_ascii_digit()) {
        return None;
    }
    value.parse().ok().filter(|&number| number > 0)
}

fn parse_bounds(value: &str) -> Option<(u64, u64)> {
    let (start, end) = value.split_once('-')?;
    let start = positive_integer(start)?;
    let end = positive_integer(end)?;
    (start <= end).then_some((start, end))
}

fn parse_want(value: &str) -> Option<Want<'_>> {
    if value == "none" {
        return Some(Want::None(None));
    }
    let (kind, arguments) = value.strip_suffix(')')?.split_once('(')?;
    let mut fields = arguments.split(',').peekable();
    let want = match kind {
        "site" => {
            let family = fields.next().filter(|value| FAMILIES.contains(value))?;
            let callee = fields.next().filter(|value| dotted_callee(value))?;
            let grade = fields
                .next()
                .filter(|value| matches!(*value, "exact" | "nameonly"))?;
            let symbol = fields
                .next_if(|value| value.starts_with("symbol="))
                .and_then(|value| value.strip_prefix("symbol="));
            if symbol.is_some_and(|name| !identifier(name)) {
                return None;
            }
            let bounds = match fields.next() {
                Some(value) => Some(parse_bounds(value.strip_prefix("bounds=")?)?),
                None => None,
            };
            Want::Site {
                family,
                callee,
                grade,
                symbol,
                bounds,
            }
        }
        "unsupported" => {
            let reason = fields.next().filter(|value| REASONS.contains(value))?;
            let count = positive_integer(fields.next()?)?;
            Want::Unsupported { reason, count }
        }
        "none" => {
            let cause = fields.next()?.strip_prefix("unproven:")?;
            if !CAUSES.contains(&cause) {
                return None;
            }
            Want::None(Some(cause))
        }
        _ => return None,
    };
    fields.next().is_none().then_some(want)
}

#[test]
fn rows_file_is_well_formed() {
    let rows = read_rows();
    assert!(!rows.is_empty(), "rows.jsonl is empty");
    let repo = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(FIXTURE_REPO);
    let mut ids = HashSet::new();
    for row in &rows {
        let id = &row.id;
        assert!(!id.trim().is_empty(), "empty row id");
        assert!(ids.insert(id), "duplicate row id {id}");
        assert!(
            GROUPS.contains(&row.group.as_str()),
            "{id}: unknown group {:?}",
            row.group
        );
        let fixture = Path::new(&row.fixture);
        assert!(
            !row.fixture.is_empty()
                && fixture
                    .components()
                    .all(|part| matches!(part, Component::Normal(_)))
                && repo.join(fixture).is_file(),
            "{id}: invalid fixture {:?}",
            row.fixture
        );
        assert!(row.line > 0, "{id}: line must be positive");
        assert!(!row.census_class.trim().is_empty(), "{id}: missing class");
        assert!(
            matches!(
                row.truth.as_str(),
                "python-semantics" | "faultpath-guard" | "design"
            ),
            "{id}: invalid truth {:?}",
            row.truth
        );
        let want = row.expectation();
        assert!(
            dotted_callee(&row.callee) || (row.callee == "*" && matches!(want, Want::None(_))),
            "{id}: invalid callee {:?}",
            row.callee
        );
        if let Want::Site { callee, bounds, .. } = want {
            assert_eq!(row.callee, callee, "{id}: conflicting callees");
            if let Some((start, end)) = bounds {
                assert!(
                    (start..=end).contains(&row.line),
                    "{id}: callee line is outside the function bounds"
                );
            }
            assert!(!row.consumer.trim().is_empty(), "{id}: missing consumer");
        } else {
            assert!(row.consumer.is_empty(), "{id}: unexpected consumer");
        }
    }
}

#[test]
fn matrix_rows_match() {
    if ENABLED_GROUPS.is_empty() {
        return;
    }

    let rows = read_rows();
    for group in ENABLED_GROUPS {
        assert!(
            rows.iter().any(|row| row.group == *group),
            "no matrix rows for enabled group {group:?}"
        );
    }
    let output = crate::prism_bin()
        .unwrap()
        .args([
            "targets",
            "--repo",
            FIXTURE_REPO,
            "--inventory",
            "python-deps",
        ])
        .assert()
        .success();
    let document: Value = serde_json::from_slice(&output.get_output().stdout)
        .expect("inventory document must be JSON");
    let targets = document["targets"]
        .as_array()
        .expect("inventory document must include a targets array");

    for row in rows
        .iter()
        .filter(|row| ENABLED_GROUPS.contains(&row.group.as_str()))
    {
        check_row(row, targets, &document["warnings"]);
    }
}

fn check_row(row: &Row, targets: &[Value], warnings: &Value) {
    let context = format!("{} {}:{} {}", row.id, row.fixture, row.line, row.want);
    let candidates: Vec<_> = targets
        .iter()
        .filter(|target| {
            let site = &target["site"];
            site["file"] == row.fixture
                && site["line"].as_u64() == Some(row.line)
                && (row.callee == "*" || target["dependency_hint"]["callee"] == row.callee)
        })
        .collect();

    match row.expectation() {
        Want::Site {
            family,
            grade,
            symbol,
            bounds,
            ..
        } => {
            let sites: Vec<_> = candidates
                .iter()
                .filter(|target| target["kind"] == "external_call")
                .collect();
            let [target] = sites.as_slice() else {
                panic!("{context}: expected one external_call, got {candidates:#?}");
            };
            assert_eq!(target["dependency_hint"]["kind"], family, "{context}");
            assert_eq!(target["confidence"], grade, "{context}");
            if let Some(symbol) = symbol {
                assert_eq!(target["site"]["symbol"], symbol, "{context}");
            }
            if let Some((start, end)) = bounds {
                assert_eq!(
                    target["site"]["function_start_line"].as_u64(),
                    Some(start),
                    "{context}"
                );
                assert_eq!(
                    target["site"]["function_end_line"].as_u64(),
                    Some(end),
                    "{context}"
                );
            }
        }
        Want::Unsupported { reason, count } => {
            let detail = if count == 1 {
                reason.to_owned()
            } else {
                format!("{reason}; {count} calls")
            };
            let matching = candidates
                .iter()
                .filter(|target| {
                    target["kind"] == "other" && target["expected"]["detail"] == detail
                })
                .count();
            assert_eq!(
                matching, 1,
                "{context}: expected one unsupported row with detail {detail:?}, got {candidates:#?}"
            );
        }
        Want::None(cause) => {
            assert!(
                candidates.is_empty(),
                "{context}: unexpected inventory rows {candidates:#?}"
            );
            if let Some(cause) = cause {
                assert!(
                    has_unproven_cause(warnings, &row.fixture, cause),
                    "{context}: missing cause {cause:?} in warnings {warnings:#}"
                );
            }
        }
    }
}

fn has_unproven_cause(warnings: &Value, fixture: &str, cause: &str) -> bool {
    let prefix = format!("inventory: identity unproven: {fixture}: ");
    warnings
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .filter_map(|warning| warning.strip_prefix(&prefix))
        .filter_map(|detail| detail.strip_suffix(')')?.split_once(" ("))
        .filter(|(total, _)| positive_integer(total).is_some())
        .flat_map(|(_, causes)| causes.split(", "))
        .filter_map(|entry| entry.split_once(": "))
        .any(|(name, count)| name == cause && positive_integer(count).is_some())
}
