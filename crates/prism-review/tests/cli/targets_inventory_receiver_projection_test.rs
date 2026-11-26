use super::{
    targets_inventory_constructors_test::identity_warnings,
    targets_inventory_test::{matrix_doc, warning},
};
use serde_json::json;

#[test]
fn mixed_receiver_group_filters_occurrences_before_grouping() {
    let files = "http_receiver/esc22.py";
    for (filter, expected) in [
        (&[][..], "not_line_anchor; 2 calls"),
        (&["--min-confidence", "exact"][..], "not_line_anchor"),
        (&["--min-tier", "asserted"][..], "not_line_anchor"),
    ] {
        let doc = matrix_doc(files, filter);
        let [group] = doc["targets"].as_array().unwrap().as_slice() else {
            panic!("expected one unsupported group with {filter:?}: {doc:#}");
        };
        assert_eq!(
            group["site"],
            json!({"file":files,"line":7,"symbol":"fetch","language":"python"})
        );
        assert_eq!(group["kind"], "other");
        assert_eq!(group["severity"], "info");
        assert_eq!(group["dependency_hint"]["callee"], "requests.Session.get");
        assert_eq!(group["confidence"], "exact");
        assert_eq!(group["tier"], "asserted");
        assert_eq!(group["expected"]["detail"], expected);
        assert!(identity_warnings(&doc).is_empty());
    }
    let doc = matrix_doc(files, &["--min-severity", "suggestion"]);
    assert_eq!(doc["targets"], json!([]));
    warning(
        &doc,
        "inventory: 1 rows removed by --min-severity suggestion",
    );
}

#[test]
fn mixed_receiver_shape_losses_and_identity_counts_are_independent() {
    let files = "http_receiver/as1-19.py,http_receiver/as2-27.py";
    for filter in [
        &[][..],
        &["--min-confidence", "exact"][..],
        &["--min-tier", "asserted"][..],
        &["--min-severity", "suggestion"][..],
    ] {
        let doc = matrix_doc(files, filter);
        let rows = doc["targets"].as_array().unwrap();
        if filter.starts_with(&["--min-severity"]) {
            assert!(rows.is_empty());
            warning(
                &doc,
                "inventory: 2 rows removed by --min-severity suggestion",
            );
        } else {
            assert_eq!(rows.len(), 2, "{doc:#}");
            for (file, reason) in [
                ("http_receiver/as1-19.py", "not_line_anchor"),
                ("http_receiver/as2-27.py", "multiline_callee"),
            ] {
                let [row] = rows
                    .iter()
                    .filter(|row| row["site"]["file"] == file)
                    .collect::<Vec<_>>()
                    .try_into()
                    .unwrap();
                assert_eq!(row["confidence"], "exact");
                assert_eq!(row["expected"]["detail"], reason);
            }
        }
        assert_eq!(
            identity_warnings(&doc),
            [
                "inventory: identity unproven: http_receiver/as1-19.py: 1 (receiver_value_unproven: 1)",
                "inventory: identity unproven: http_receiver/as2-27.py: 1 (receiver_value_unproven: 1)",
            ],
        );
    }
}
