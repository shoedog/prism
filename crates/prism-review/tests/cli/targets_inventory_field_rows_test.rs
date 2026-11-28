use super::targets_inventory_matrix_test::{check_field_matrix, check_receiver_pins, FIXTURE_REPO};
use super::targets_inventory_test::document;
use serde_json::{json, Value};

pub(super) fn scan(files: &str, options: &[&str]) -> Value {
    scan_with_env(None, files, options)
}

/// Runs the scan with the removed slice-1 switch set, to prove it no longer
/// changes anything (DESIGN 6.12 item 13).
fn scan_with_env(retired_switch: Option<&str>, files: &str, options: &[&str]) -> Value {
    let mut command = crate::prism_std();
    if let Some(value) = retired_switch {
        command.env("PRISM_INVENTORY_FIELD_ROWS", value);
    } else {
        command.env_remove("PRISM_INVENTORY_FIELD_ROWS");
    }
    document(
        &command
            .args([
                "targets",
                "--repo",
                FIXTURE_REPO,
                "--inventory",
                "python-deps",
            ])
            .args(["--files", files])
            .args(options)
            .output()
            .unwrap(),
    )
}

macro_rules! matrix {
    ($($id:ident),+ $(,)?) => {
        $(#[test]
        fn $id() {
            check_field_matrix(stringify!($id));
        })+
    };
}

matrix!(pf01, pf02, pf03, pf04, pf05, pf06, pf07, pf08, pf09, pf10, pf11, pf12, pf13,);

#[test]
fn conditional_class_suite_initializer_limit() {
    check_field_matrix("pf-ifTrue-init");
}

#[test]
fn field_row_known_limits() {
    check_field_matrix("pf-limit-");
}

#[test]
fn field_rows_are_default_and_the_retired_switch_is_inert() {
    let files = "stdlib/pfield/pf01.py,stdlib/pfield/pf04c.py,stdlib/pfield/pf12a.py,stdlib/pfield/pf12b.py,stdlib/pfield/pf12c.py";
    let default = scan(files, &[]);
    for value in ["", "0", "1", "true", "01", "1 ", " 1"] {
        assert_eq!(scan_with_env(Some(value), files, &[]), default, "{value:?}");
    }
    let fields: Vec<_> = default["targets"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| {
            row["site"]["file"] == "stdlib/pfield/pf01.py" && row["site"]["symbol"] != "local"
        })
        .collect();
    assert_eq!(fields.len(), 2, "{default:#}");
    for row in fields {
        assert_eq!(row["confidence"], "nameonly");
        assert_eq!(row["tier"], "candidate");
    }
    let warnings: Vec<_> = default["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(Value::as_str)
        .filter(|warning| warning.contains("identity unproven"))
        .collect();
    assert_eq!(
        warnings,
        [
            "inventory: identity unproven: stdlib/pfield/pf04c.py: 1 (receiver_not_unique: 1)",
            "inventory: identity unproven: stdlib/pfield/pf12a.py: 1 (receiver_value_unproven: 1)",
            "inventory: identity unproven: stdlib/pfield/pf12b.py: 1 (receiver_value_unproven: 1)",
            "inventory: identity unproven: stdlib/pfield/pf12c.py: 1 (receiver_value_unproven: 1)",
        ]
    );
    check_receiver_pins(&["h07", "sl4-09", "sl5-13", "as1-20", "wf40", "as4-15"]);
}

#[test]
fn default_projects_candidate_filters_and_grouped_details() {
    let files = "stdlib/pfield/pf01.py,stdlib/pfield/pf11.py";
    let doc = scan(files, &[]);
    assert_eq!(doc["schema_version"], "1.1");
    let targets = doc["targets"].as_array().unwrap();
    assert_eq!(targets.len(), 4, "{doc:#}");
    let unsupported = targets.iter().find(|row| row["kind"] == "other").unwrap();
    assert_eq!(
        unsupported["expected"]["detail"],
        "not_line_anchor; 2 calls"
    );
    assert_eq!(unsupported["confidence"], "nameonly");
    assert_eq!(unsupported["tier"], "candidate");
    assert_eq!(
        unsupported["site"],
        json!({
            "file":"stdlib/pfield/pf11.py", "line":8, "symbol":"is_new", "language":"python"
        })
    );
    assert_eq!(
        unsupported["dependency_hint"],
        json!({
            "kind":"filesystem", "callee":"pathlib.Path.exists"
        })
    );
    for args in [["--min-confidence", "exact"], ["--min-tier", "asserted"]] {
        let filtered = scan(files, &args);
        let rows = filtered["targets"].as_array().unwrap();
        assert!(!rows.is_empty(), "{args:?}: the exact local row stays");
        assert!(
            rows.iter()
                .all(|row| row["confidence"] != "nameonly" && row["tier"] != "candidate"),
            "{args:?}: {filtered:#}"
        );
    }
    assert_eq!(
        scan(files, &["--inventory-kinds", "http"])["targets"],
        json!([])
    );
}
