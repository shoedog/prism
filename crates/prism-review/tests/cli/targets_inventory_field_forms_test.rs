//! Behaviour tests for the annotation, initializer-prefix and using-method
//! shapes of the stdlib R3 field route (DESIGN 6.12 items 2 to 4): a row is
//! emitted only for a single listed class, and a union, quoted or otherwise
//! unsupported form never produces a row.
use super::targets_inventory_test::document;
use serde_json::Value;
use std::fs;
use tempfile::TempDir;

fn field_doc(source: &str) -> Value {
    let repo = TempDir::new().unwrap();
    fs::write(repo.path().join("probe.py"), source).unwrap();
    document(
        &crate::prism_std()
            .env("PRISM_INVENTORY_FIELD_ROWS", "1")
            .args(["targets", "--repo"])
            .arg(repo.path())
            .args(["--inventory", "python-deps", "--files", "probe.py"])
            .output()
            .unwrap(),
    )
}

/// (rows as (line, callee, confidence), identity-unproven warning causes)
fn outcome(doc: &Value) -> (Vec<(u64, String, String)>, Vec<String>) {
    let rows = doc["targets"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| {
            (
                t["site"]["line"].as_u64().unwrap(),
                t["dependency_hint"]["callee"].as_str().unwrap().to_owned(),
                t["confidence"].as_str().unwrap().to_owned(),
            )
        })
        .collect();
    let warnings = doc["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(Value::as_str)
        .filter(|w| w.starts_with("inventory: identity unproven: probe.py: "))
        .map(|w| {
            w.trim_start_matches("inventory: identity unproven: probe.py: ")
                .to_owned()
        })
        .collect();
    (rows, warnings)
}

fn annotated(annotation: &str) -> Value {
    field_doc(&format!(
        "from pathlib import Path\nfrom typing import Optional, Union\nclass C:\n    def __init__(self, path: {annotation}):\n        self.path = path\n    def m(self):\n        return self.path.exists()\n"
    ))
}

const UNPROVEN: &str = "1 (receiver_value_unproven: 1)";

#[test]
fn single_class_annotation_forms_emit_a_candidate_row() {
    for annotation in ["Path", "\"Path\""] {
        let (rows, warnings) = outcome(&annotated(annotation));
        assert_eq!(
            rows,
            [(7, "pathlib.Path.exists".into(), "nameonly".into())],
            "{annotation}"
        );
        assert!(warnings.is_empty(), "{annotation}: {warnings:?}");
    }
}

#[test]
fn union_and_optional_annotations_are_a_diagnostic_never_a_row() {
    for annotation in [
        "Optional[Path]",
        "Union[Path, None]",
        "Path | None",
        "\"Path | None\"",
        "\"Optional[Path]\"",
        "\"Union[Path, str]\"",
        "Optional[\"Path\"]",
        "Union[\"Path\", None]",
    ] {
        let (rows, warnings) = outcome(&annotated(annotation));
        assert!(rows.is_empty(), "{annotation}: {rows:?}");
        assert_eq!(warnings, [UNPROVEN], "{annotation}");
    }
}

#[test]
fn unsupported_annotation_forms_stay_silent() {
    for annotation in [
        "\"Path; x\"",
        "\"Path |\"",
        "Path()",
        "os.PathLike",
        "Foo | None",
        "\"list[Path]\"",
        "list[Path]",
        "Optional[Optional[Path]]",
    ] {
        let (rows, warnings) = outcome(&annotated(annotation));
        assert!(rows.is_empty(), "{annotation}: {rows:?}");
        assert!(warnings.is_empty(), "{annotation}: {warnings:?}");
    }
    let (rows, warnings) = outcome(&annotated("pathlib.Path"));
    assert!(rows.is_empty());
    assert_eq!(warnings, ["1 (not_imported: 1)"]);
}

fn initializer(prefix: &str, using: &str) -> Value {
    field_doc(&format!(
        "from pathlib import Path\nclass C:\n    def __init__(self, path: Path):\n{prefix}        self.path = path\n    def m({using}):\n        return self.path.exists()\n"
    ))
}

#[test]
fn initializer_prefix_admits_only_one_bare_super_init() {
    let line = |prefix: &str| if prefix.is_empty() { 6 } else { 7 };
    for prefix in ["", "        super().__init__()\n"] {
        let (rows, warnings) = outcome(&initializer(prefix, "self"));
        assert_eq!(
            rows,
            [(
                line(prefix),
                "pathlib.Path.exists".into(),
                "nameonly".into()
            )],
            "{prefix:?}"
        );
        assert!(warnings.is_empty(), "{prefix:?}");
    }
    for prefix in [
        "        helper()\n",
        "        other.__init__()\n",
        "        x = 1\n",
        "        super().__init__()\n        super().__init__()\n",
    ] {
        let (rows, warnings) = outcome(&initializer(prefix, "self"));
        assert!(rows.is_empty(), "{prefix:?}: {rows:?}");
        assert_eq!(warnings, [UNPROVEN], "{prefix:?}");
    }
}

#[test]
fn using_method_self_forms() {
    for using in [
        "self",
        "self: 'C'",
        "self: C",
        "self=None",
        "self: C = None",
    ] {
        let (rows, warnings) = outcome(&initializer("", using));
        assert_eq!(
            rows,
            [(6, "pathlib.Path.exists".into(), "nameonly".into())],
            "{using}"
        );
        assert!(warnings.is_empty(), "{using}");
    }
    let (rows, warnings) = outcome(&initializer("", "*self"));
    assert!(rows.is_empty());
    assert_eq!(warnings, [UNPROVEN]);
}
