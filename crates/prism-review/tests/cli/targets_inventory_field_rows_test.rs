use super::targets_inventory_matrix_test::{check_field_matrix, check_receiver_pins, FIXTURE_REPO};
use super::targets_inventory_test::document;
use serde_json::{json, Value};

pub(super) fn scan(gate: Option<&str>, files: &str, options: &[&str]) -> Value {
    let mut command = crate::prism_std();
    if let Some(value) = gate {
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
fn gate_off_keeps_the_base_constructor_diagnostics() {
    let files = "stdlib/pfield/pf01.py,stdlib/pfield/pf04c.py,stdlib/pfield/pf12a.py,stdlib/pfield/pf12b.py,stdlib/pfield/pf12c.py";
    let off = scan(None, files, &[]);
    for value in ["", "0", "true", "01", "1 ", " 1"] {
        assert_eq!(scan(Some(value), files, &[]), off, "{value:?}");
    }
    let [row] = off["targets"].as_array().unwrap().as_slice() else {
        panic!("expected exactly the local construction site: {off:#}");
    };
    assert_eq!(row["site"]["symbol"], "local");
    assert_eq!(row["confidence"], "exact");
    let warnings: Vec<_> = off["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(Value::as_str)
        .filter(|warning| warning.contains("identity unproven"))
        .collect();
    assert_eq!(
        warnings,
        [
            "inventory: identity unproven: stdlib/pfield/pf12a.py: 1 (receiver_value_unproven: 1)",
            "inventory: identity unproven: stdlib/pfield/pf12b.py: 1 (receiver_value_unproven: 1)",
        ]
    );
    check_receiver_pins(&["h07", "sl4-09", "sl5-13", "as1-20", "wf40", "as4-15"]);
    let inherited = "http_receiver/h07.py,http_receiver/sl4-09.py,http_receiver/sl5-13.py,http_receiver/as1-20/child.py,http_receiver/wf40.py,http_receiver/as4-15.py";
    assert_eq!(scan(Some("1"), inherited, &[]), scan(None, inherited, &[]));
}

#[test]
fn gate_on_projects_candidate_filters_and_grouped_details() {
    let files = "stdlib/pfield/pf01.py,stdlib/pfield/pf11.py";
    let doc = scan(Some("1"), files, &[]);
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
        let filtered = scan(Some("1"), files, &args);
        assert_eq!(filtered["targets"], scan(None, files, &args)["targets"]);
    }
    assert_eq!(
        scan(Some("1"), files, &["--inventory-kinds", "http"])["targets"],
        json!([])
    );
}
