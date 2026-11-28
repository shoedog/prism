use serde::Deserialize;
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};
use std::path::{Component, Path};

pub(super) const FIXTURE_REPO: &str = "tests/targets_inventory/py";
const ROWS: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/targets_inventory/rows.jsonl"
));
const GROUPS: &[&str] = &["http.root", "http.receiver", "stdlib"];
const FIELD_FIXTURES: &str = "stdlib/pfield/";
const ENABLED_GROUPS: &[&str] = &["http.root", "http.receiver", "stdlib"];
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
    "receiver_is_none",
    "annotation_conflict",
    "helper_unannotated",
    "helper_not_admissible",
    "receiver_value_unproven",
];
const IDENTITY_WARNING: &str = "inventory: identity unproven: ";

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
        .env_remove("PRISM_INVENTORY_FIELD_ROWS")
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
    let warnings = document["warnings"].as_array().expect("inventory warnings");
    let unique: HashSet<_> = warnings
        .iter()
        .map(|warning| warning.as_str().unwrap())
        .collect();
    assert_eq!(warnings.len(), unique.len(), "duplicate inventory warning");

    // Each unsupported target answers at most one row (DESIGN 6.8: rows consume actual targets).
    let mut used = vec![false; targets.len()];
    let enabled: Vec<_> = rows
        .iter()
        .filter(|row| {
            ENABLED_GROUPS.contains(&row.group.as_str()) && !row.fixture.starts_with(FIELD_FIXTURES)
        })
        .collect();
    assert_eq!(enabled.len(), 784);
    assert_eq!(
        identity_warning_lines(warnings)
            .into_iter()
            .filter(|line| !line.starts_with(&format!("{IDENTITY_WARNING}{FIELD_FIXTURES}")))
            .collect::<Vec<_>>(),
        expected_identity_warnings(&enabled),
        "matrix warning totals and causes"
    );
    for row in enabled {
        check_row(row, targets, &document["warnings"], &mut used);
    }
}

fn expected_identity_warnings(rows: &[&Row]) -> Vec<String> {
    let mut counts = BTreeMap::<&str, BTreeMap<&str, usize>>::new();
    for row in rows {
        if let Want::None(Some(cause)) = row.expectation() {
            *counts
                .entry(&row.fixture)
                .or_default()
                .entry(cause)
                .or_default() += 1;
        }
    }
    // DESIGN §6.5 also counts a parameter call alongside each constructor row.
    for file in ["http_receiver/as1-19.py", "http_receiver/as2-27.py"] {
        if rows.iter().any(|row| row.fixture == file) {
            *counts
                .entry(file)
                .or_default()
                .entry("receiver_value_unproven")
                .or_default() += 1;
        }
    }
    counts
        .into_iter()
        .map(|(file, causes)| {
            let total: usize = causes.values().sum();
            let detail = CAUSES
                .iter()
                .filter_map(|&cause| causes.get(cause).map(|count| format!("{cause}: {count}")))
                .collect::<Vec<_>>()
                .join(", ");
            format!("{IDENTITY_WARNING}{file}: {total} ({detail})")
        })
        .collect()
}

fn identity_warning_lines(warnings: &[Value]) -> Vec<&str> {
    let mut lines: Vec<_> = warnings
        .iter()
        .map(|warning| warning.as_str().unwrap())
        .filter(|warning| warning.starts_with(IDENTITY_WARNING))
        .collect();
    lines.sort_unstable();
    lines
}

#[test]
fn inline_constructor_pins() {
    check_receiver_pins(&[
        "h10", "sl1-28", "sl2-16", "as2-27", "wf29", "esc06", "as5-08",
    ]);
}

#[test]
fn named_constructor_pins() {
    check_receiver_pins(&[
        "h05", "h06", "h11", "o05", "o07", "n06", "e01", "e05a", "e05b", "e05c", "e05d", "e05e",
        "tr3-19", "tr4-10", "tr5-03", "sl1-29", "sl1-30", "sl1-31", "sl2-21", "sl4-08", "sl5-11",
        "as1-18", "as2-13", "as2-14", "as2-16", "as3-23", "as4-11", "as5-04", "wf30", "wf31",
        "wf35", "wf36", "wf37", "wf38", "wf39", "wf52", "wf92", "wf94", "esc07", "esc08", "esc10",
        "esc20", "esc28", "esc29", "esc30",
    ]);
}

#[test]
fn field_receiver_pins() {
    check_receiver_pins(&[
        "h07", "e06", "e07", "tr1-10", "tr1-11", "tr2-48", "tr2-49", "tr2-50", "tr2-51", "tr3-20",
        "tr3-21", "as2-18", "as4-15", "wf40", "wf41", "wf42", "wf43", "sl4-09", "sl5-13", "sl2-17",
        "sl2-18", "as1-21", "as5-09", "as2-25", "as3-21", "as1-20", "esc27", "wf32",
    ]);
}

#[test]
fn helper_value_direct_pins() {
    check_receiver_pins(&[
        "h08", "h09", "n07", "e09a", "e09b", "e09c", "e09d", "o26", "tr1-13", "tr2-57", "tr2-58",
        "tr2-59", "tr2-60", "tr3-22", "tr3-23", "tr4-01", "tr4-02", "tr4-03", "tr5-04", "sl1-35",
        "sl1-36", "sl1-37", "sl1-38", "sl1-39", "sl1-40", "sl1-41", "sl2-22", "sl3-16", "sl3-17",
        "sl4-13", "as1-14", "as1-16", "as1-17", "as2-19", "as2-22", "as2-23", "as2-24", "as3-15",
        "as3-17", "as3-18", "as3-19", "as3-22", "as4-24", "as5-06", "wf33", "wf34", "wf44", "wf45",
        "wf46", "wf47", "wf54", "wf55", "wf56", "wf57", "wf58", "wf59", "wf61", "wf63", "wf64",
        "wf66", "wf67", "wf69", "wf70", "wf71", "wf73", "esc01", "esc02", "esc03", "esc04",
        "esc05", "esc09", "esc24",
    ]);
}

#[test]
fn helper_returned_local_pins() {
    check_receiver_pins(&[
        "esc23", "esc31", "esc32", "esc33", "esc34", "pin01", "pin09", "pin02",
    ]);
}

pub(super) fn check_receiver_pins(ids: &[&str]) {
    let rows: Vec<_> = read_rows()
        .into_iter()
        .filter(|row| ids.contains(&row.id.as_str()))
        .collect();
    assert_eq!(rows.len(), ids.len());
    let files = rows
        .iter()
        .map(|row| row.fixture.as_str())
        .collect::<std::collections::BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>()
        .join(",");
    let document = super::targets_inventory_test::document(&super::targets_inventory_test::run(
        Path::new(FIXTURE_REPO),
        &["--files", &files],
    ));
    let targets = document["targets"].as_array().unwrap();
    let mut used = vec![false; targets.len()];
    for row in &rows {
        check_row(row, targets, &document["warnings"], &mut used);
    }
}

fn check_row(row: &Row, targets: &[Value], warnings: &Value, used: &mut [bool]) {
    let context = format!("{} {}:{} {}", row.id, row.fixture, row.line, row.want);
    let indexes: Vec<usize> = targets
        .iter()
        .enumerate()
        .filter(|(_, target)| {
            let site = &target["site"];
            site["file"] == row.fixture
                && site["line"].as_u64() == Some(row.line)
                && (row.callee == "*" || target["dependency_hint"]["callee"] == row.callee)
        })
        .map(|(i, _)| i)
        .collect();
    let candidates: Vec<&Value> = indexes.iter().map(|&i| &targets[i]).collect();

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
            let found = indexes.iter().copied().find(|&i| {
                !used[i]
                    && targets[i]["kind"] == "other"
                    && targets[i]["expected"]["detail"] == detail
            });
            let Some(i) = found else {
                panic!(
                    "{context}: expected an unconsumed unsupported row with detail {detail:?}, got {candidates:#?}"
                );
            };
            used[i] = true;
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
    let prefix = format!("{IDENTITY_WARNING}{fixture}: ");
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

#[test]
fn matrix_rejects_incorrect_warning_counts() {
    let rows = read_rows();
    let fixture_rows: Vec<_> = rows
        .iter()
        .filter(|row| matches!(row.id.as_str(), "sl1-67" | "sl1-68" | "sl1-69"))
        .collect();
    assert_eq!(fixture_rows.len(), 3);
    let prefix = format!("{IDENTITY_WARNING}{}: ", fixture_rows[0].fixture);
    let expected = expected_identity_warnings(&fixture_rows);
    assert_eq!(
        expected,
        [format!("{prefix}3 (shadowed_root: 2, not_imported: 1)")]
    );
    let control = [Value::from(expected[0].clone())];
    assert_eq!(identity_warning_lines(&control), expected);
    for detail in [
        "4 (shadowed_root: 2, not_imported: 1)",
        "3 (shadowed_root: 1, not_imported: 2)",
        "3 (not_imported: 1, shadowed_root: 2)",
    ] {
        let altered = [Value::from(format!("{prefix}{detail}"))];
        assert_ne!(identity_warning_lines(&altered), expected);
    }
    let duplicated = [control[0].clone(), control[0].clone()];
    assert_ne!(identity_warning_lines(&duplicated), expected);
}

pub(super) fn check_field_matrix(prefix: &str) {
    let rows = read_rows();
    let files: std::collections::BTreeSet<_> = rows
        .iter()
        .filter(|row| row.fixture.starts_with(FIELD_FIXTURES) && row.id.starts_with(prefix))
        .map(|row| row.fixture.as_str())
        .collect();
    assert!(!files.is_empty(), "{prefix}");
    let selected: Vec<_> = rows
        .iter()
        .filter(|row| files.contains(row.fixture.as_str()))
        .collect();
    let files = files.into_iter().collect::<Vec<_>>().join(",");
    let doc = super::targets_inventory_field_rows_test::scan(Some("1"), &files, &[]);
    let targets = doc["targets"].as_array().unwrap();
    let mut used = vec![false; targets.len()];
    assert_eq!(
        identity_warning_lines(doc["warnings"].as_array().unwrap()),
        expected_identity_warnings(&selected),
        "{prefix}: per-file warning totals"
    );
    for row in &selected {
        check_row(row, targets, &doc["warnings"], &mut used);
    }
    let expected_targets = selected
        .iter()
        .filter(|row| !matches!(row.expectation(), Want::None(_)))
        .count();
    assert_eq!(targets.len(), expected_targets, "{prefix}: {targets:#?}");
    for target in targets {
        if target["confidence"] == "nameonly" {
            assert_eq!(target["tier"], "candidate", "{prefix}: {target:#}");
        }
    }
}
