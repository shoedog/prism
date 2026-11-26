use crate::schema::Inventory;
use crate::support::{index, json};
use prism_boundary::revision::{Blob, Format, ReferenceBlob};
use serde_json::{json, Value};
use std::path::Path;
use std::process::Command;

const CAPTURE_PATH: &str = "/reed/{id}";
const MIXED_CAPTURE_PATH: &str = "/reed/x{id}";
const LABEL_ID: &str = "op:GET /reed/.{}";
const PLAIN_ID: &str = "op:GET /reed/{}";
const ENTRY_URI: &str = "https://example.test/openapi.json";
const CUSTOM_DIALECT: &str = "https://example.test/dialect-without-validation";
const UNCHECKED: &str = "path_parameter_style_unchecked";
const UNSUPPORTED: &str = "path_style_unsupported";

fn label(schema: Value) -> Value {
    json!({"in": "path", "name": "id", "required": true, "style": "label", "schema": schema})
}

fn document(parameters: Value) -> Value {
    document_at(CAPTURE_PATH, parameters)
}

fn document_at(path: &str, parameters: Value) -> Value {
    json!({
        "openapi": "3.1.0",
        "paths": {
            (path): {"get": {"parameters": parameters, "responses": {"201": {}}}},
            "/control": {"get": {"responses": {"200": {}}}}
        }
    })
}

fn projection(document: &Value, expected: Option<&str>) {
    assert_projection(&json(&serde_json::to_vec(document).unwrap()), expected);
}

fn assert_projection(inventory: &Inventory, expected: Option<&str>) {
    assert_serialization(inventory, CAPTURE_PATH, expected.ok_or(UNCHECKED));
}

fn assert_serialization(inventory: &Inventory, path: &str, expected: Result<&str, &str>) {
    assert_eq!(
        inventory
            .operations
            .keys()
            .map(ToString::to_string)
            .collect::<Vec<_>>(),
        ["op:GET /control"]
            .into_iter()
            .chain(expected.ok())
            .collect::<Vec<_>>()
    );
    let diagnostics: Vec<_> = inventory
        .diagnostics
        .iter()
        .filter(|d| matches!(d.code, UNCHECKED | UNSUPPORTED))
        .collect();
    if let Err(reason) = expected {
        let [declaration] = inventory.unprojected.as_slice() else {
            panic!(
                "expected one withheld operation: {:?}",
                inventory.unprojected
            );
        };
        assert_eq!(declaration.source_path, path);
        assert_eq!(declaration.source_method, "get");
        assert_eq!(declaration.responses.keys().collect::<Vec<_>>(), ["201"]);
        let [diagnostic] = diagnostics.as_slice() else {
            panic!(
                "expected one path style diagnostic: {:?}",
                inventory.diagnostics
            );
        };
        assert_eq!(diagnostic.code, reason);
        assert_eq!(diagnostic.scope, "path_parameters");
        assert_eq!(diagnostic.site.pointer, ["paths", path]);
        assert_eq!(diagnostic.capture.as_deref(), Some("id"));
    } else {
        assert!(inventory.unprojected.is_empty());
        assert!(diagnostics.is_empty());
    }
}

fn check_parameter(parameter: Value, expected: Result<&str, &str>) {
    check_parameter_at(CAPTURE_PATH, parameter, expected);
}

fn check_parameter_at(path: &str, parameter: Value, expected: Result<&str, &str>) {
    let input = document_at(path, json!([parameter]));
    assert_serialization(&json(&serde_json::to_vec(&input).unwrap()), path, expected);
}

#[test]
fn path_serialization_withholds_json_content() {
    check_parameter(
        json!({
            "in": "path", "name": "id", "required": true,
            "content": {"application/json": {"schema": {"type": "string"}}}
        }),
        Err(UNSUPPORTED),
    );
}

#[test]
fn path_serialization_withholds_text_content_with_simple_style() {
    check_parameter_at(
        MIXED_CAPTURE_PATH,
        json!({
            "in": "path", "name": "id", "required": true, "style": "simple",
            "content": {"text/plain": {"schema": {"type": "string", "enum": [""]}}}
        }),
        Err(UNSUPPORTED),
    );
}

#[test]
fn path_serialization_withholds_text_content_with_label_style() {
    check_parameter_at(
        MIXED_CAPTURE_PATH,
        json!({
            "in": "path", "name": "id", "required": true, "style": "label",
            "content": {"text/plain": {"schema": {"type": "string", "enum": [""]}}}
        }),
        Err(UNSUPPORTED),
    );
}

#[test]
fn path_serialization_counts_malformed_content_as_present() {
    for content in [json!(null), json!(["application/json"]), json!("bad")] {
        check_parameter(
            json!({"in": "path", "name": "id", "required": true, "content": content}),
            Err(UNSUPPORTED),
        );
    }
    check_parameter(label(json!({"type": "string"})), Ok(LABEL_ID));
}

#[test]
fn path_serialization_leaves_content_with_schema_unchecked() {
    let mut parameter = label(json!({"type": "string"}));
    parameter["content"] = json!({"application/json": {"schema": {"type": "string"}}});
    check_parameter(parameter, Err(UNCHECKED));
}

#[test]
fn path_serialization_leaves_missing_schema_and_content_unchecked() {
    check_parameter(
        json!({"in": "path", "name": "id", "required": true, "style": "simple"}),
        Err(UNCHECKED),
    );
}

#[test]
fn path_serialization_leaves_null_schema_unchecked() {
    check_parameter(
        json!({"in": "path", "name": "id", "required": true, "style": "simple", "schema": null}),
        Err(UNCHECKED),
    );
}

#[test]
fn path_serialization_leaves_string_schema_unchecked() {
    check_parameter(
        json!({"in": "path", "name": "id", "required": true, "style": "simple", "schema": "string"}),
        Err(UNCHECKED),
    );
}

#[test]
fn path_serialization_leaves_openapi_30_boolean_schema_unchecked() {
    let mut input = document(json!([
        {"in": "path", "name": "id", "required": true, "style": "simple", "schema": true}
    ]));
    input["openapi"] = json!("3.0.3");
    projection(&input, None);
}

#[test]
fn path_serialization_reads_openapi_31_boolean_schema_for_simple() {
    for schema in [true, false] {
        let mut parameter = label(json!(schema));
        check_parameter(parameter.clone(), Err(UNCHECKED));
        parameter["style"] = json!("simple");
        check_parameter(parameter, Ok(PLAIN_ID));
    }
}

#[test]
fn path_serialization_keeps_unbound_capture() {
    projection(&document(json!([])), Some(PLAIN_ID));
}

#[test]
fn known_limit_empty_string_enum_preserves_capture_spelling() {
    for (style, expected) in [
        // KNOWN-LIMIT: The ID records the template and its simple capture, with no claim
        // about which values the capture admits, the empty string included.
        ("simple", "op:GET /reed/x{}"),
        // KNOWN-LIMIT: The ID records the template and its label capture, with no claim
        // about which values the capture admits, the empty string included.
        ("label", "op:GET /reed/x.{}"),
    ] {
        let mut parameter = label(json!({"type": "string", "enum": [""]}));
        parameter["style"] = json!(style);
        check_parameter_at(MIXED_CAPTURE_PATH, parameter, Ok(expected));
    }
}

#[test]
fn path_serialization_withholds_array_label() {
    let mut parameter = label(json!({"type": ["null", "array"], "items": {"type": "string"}}));
    parameter["explode"] = json!(true);
    check_parameter(parameter, Err(UNSUPPORTED));
}

#[test]
fn path_serialization_withholds_object_label() {
    check_parameter(label(json!({"type": "object"})), Err(UNSUPPORTED));
}

#[test]
fn path_serialization_withholds_scalar_matrix() {
    let mut parameter = label(json!({"type": "string"}));
    parameter["style"] = json!("matrix");
    check_parameter(parameter, Err(UNSUPPORTED));
}

#[test]
fn path_serialization_withholds_array_matrix() {
    let mut parameter = label(json!({"type": "array", "items": {"type": "string"}}));
    parameter["style"] = json!("matrix");
    parameter["explode"] = json!(true);
    check_parameter(parameter, Err(UNSUPPORTED));
}

#[test]
fn path_serialization_withholds_unknown_style() {
    let mut parameter = label(json!({"type": "string"}));
    parameter["style"] = json!("other");
    check_parameter(parameter, Err(UNSUPPORTED));
}

#[test]
fn path_serialization_withholds_non_string_style() {
    let mut parameter = label(json!({"type": "string"}));
    parameter["style"] = Value::Null;
    check_parameter(parameter, Err(UNSUPPORTED));
}

#[test]
fn path_serialization_withholds_non_boolean_explode() {
    let mut parameter = label(json!({"type": "string"}));
    parameter["explode"] = json!("true");
    check_parameter(parameter.clone(), Err(UNSUPPORTED));
    parameter["style"] = json!("simple");
    check_parameter(parameter.clone(), Err(UNSUPPORTED));
    parameter["explode"] = json!(true);
    check_parameter(parameter, Ok(PLAIN_ID));
}

#[test]
fn path_serialization_leaves_undeterminable_label_unchecked() {
    check_parameter(label(json!({})), Err(UNCHECKED));
}

#[test]
fn path_serialization_keeps_simple_and_scalar_label() {
    let scalar = label(json!({"type": "string"}));
    check_parameter(scalar.clone(), Ok(LABEL_ID));
    let mut parameter = scalar;
    parameter["explode"] = json!(true);
    check_parameter(parameter.clone(), Ok(LABEL_ID));
    parameter["style"] = json!("simple");
    check_parameter(parameter.clone(), Ok(PLAIN_ID));
    let mut array_parameter = parameter.clone();
    array_parameter["schema"] = json!({"type": "array", "items": {"type": "string"}});
    check_parameter(array_parameter, Ok(PLAIN_ID));
    parameter.as_object_mut().unwrap().remove("style");
    check_parameter(parameter.clone(), Ok(PLAIN_ID));
    for required in [json!(false), json!("true")] {
        parameter["required"] = required;
        check_parameter(parameter.clone(), Ok(PLAIN_ID));
    }
    parameter.as_object_mut().unwrap().remove("required");
    check_parameter(parameter, Ok(PLAIN_ID));
}

#[test]
fn label_schema_scalar_reference_is_withheld() {
    let mut input = document(json!([label(json!({"type": "string"}))]));
    input["components"]["schemas"]["Scalar"] = json!({"type": "string"});
    projection(&input, Some(LABEL_ID));
    input["paths"][CAPTURE_PATH]["get"]["parameters"][0]["schema"] =
        json!({"$ref": "#/components/schemas/Scalar"});
    projection(&input, None);
}

#[test]
fn label_parameter_scalar_reference_is_withheld() {
    let scalar = label(json!({"type": "string"}));
    let mut input = document(json!([scalar]));
    input["components"]["parameters"]["Id"] = scalar;
    projection(&input, Some(LABEL_ID));
    input["paths"][CAPTURE_PATH]["get"]["parameters"][0] =
        json!({"$ref": "#/components/parameters/Id"});
    projection(&input, None);
}

#[test]
fn missing_parameter_reference_is_withheld() {
    let input = document(json!([label(json!({"type": "string"}))]));
    projection(&input, Some(LABEL_ID));
    projection(
        &document(json!([{"$ref": "#/components/parameters/Missing"}])),
        None,
    );
}

#[test]
fn label_schema_id_changing_reference_base_is_withheld() {
    let input = document(json!([label(json!({"type": "string"}))]));
    projection(&input, Some(LABEL_ID));
    let input = document(json!([label(
        json!({"$id": "nested/base.json", "$ref": "types.json#/$defs/Id"})
    )]));
    let dependencies = [
        (
            "https://example.test/types.json",
            "root-types",
            br#"{"$defs":{"Id":{"type":"array"}}}"#.as_slice(),
        ),
        (
            "https://example.test/nested/types.json",
            "nested-types",
            br#"{"$defs":{"Id":{"type":"string"}}}"#.as_slice(),
        ),
    ]
    .map(|(retrieval_uri, oid, bytes)| ReferenceBlob {
        retrieval_uri,
        blob: Blob {
            path: "types.json",
            tree: "test-tree",
            oid,
            bytes,
            format: Format::Json,
        },
    });
    let inventory = index(
        &serde_json::to_vec(&input).unwrap(),
        Format::Json,
        "test-api",
        Some(ENTRY_URI),
        &dependencies,
    )
    .unwrap();
    assert_projection(&inventory, None);
}

#[test]
fn unadmitted_override_does_not_fall_back_to_inherited_label() {
    let mut input = document(
        json!([{"in": "path", "name": "id", "required": true, "style": "simple", "schema": {"type": "string"}}]),
    );
    input["paths"][CAPTURE_PATH]["parameters"] = json!([label(json!({"type": "string"}))]);
    projection(&input, Some(PLAIN_ID));
    input["paths"][CAPTURE_PATH]["get"]["parameters"][0] = json!({"$ref": "unadmitted.json#/Id"});
    let inventory = index(
        &serde_json::to_vec(&input).unwrap(),
        Format::Json,
        "test-api",
        Some(ENTRY_URI),
        &[],
    )
    .unwrap();
    assert_projection(&inventory, None);
}

#[test]
fn label_schema_type_sibling_of_reference_is_withheld() {
    let mut input = document(json!([label(json!({"type": "string"}))]));
    input["components"]["schemas"]["Mixed"] = json!({"type": ["string", "array"]});
    projection(&input, Some(LABEL_ID));
    input["paths"][CAPTURE_PATH]["get"]["parameters"][0]["schema"]["$ref"] =
        json!("#/components/schemas/Mixed");
    projection(&input, None);
}

#[test]
fn label_type_admits_one_recognized_non_null_member() {
    for (version, kind, expected) in [
        ("3.1.0", json!(["null", "string"]), Some(LABEL_ID)),
        ("3.1.0", json!(["string"]), Some(LABEL_ID)),
        ("3.0.3", json!("integer"), Some(LABEL_ID)),
        ("3.1.0", json!(["string", "integer"]), None),
        ("3.1.0", json!(["array", "object"]), None),
        ("3.1.0", json!(["null", "string", "array"]), None),
        ("3.1.0", json!(["null"]), None),
        ("3.1.0", Value::Null, None),
        ("3.1.0", json!("unknown"), None),
        ("3.0.3", json!(["string"]), None),
    ] {
        let mut input = document(json!([label(json!({"type": kind}))]));
        input["openapi"] = json!(version);
        projection(&input, expected);
    }
    projection(
        &document(json!([label(json!({"$ref": "#/missing"}))])),
        None,
    );
}

#[test]
fn label_dialect_document_default_is_withheld() {
    let mut input = document(json!([label(
        json!({"type": "string", "not": {"items": false}})
    )]));
    input["jsonSchemaDialect"] = json!(CUSTOM_DIALECT);
    projection(&input, None);
}

#[test]
fn label_dialect_schema_override_is_withheld() {
    let input = document(json!([label(
        json!({"type": "string", "$schema": CUSTOM_DIALECT})
    )]));
    projection(&input, None);
}

#[test]
fn label_dialect_implicit_default_stays_checked() {
    let mut input = document(json!([label(
        json!({"type": "string", "not": {"items": false}})
    )]));
    projection(&input, Some(LABEL_ID));
    input["x-metadata"] = json!({"jsonSchemaDialect": CUSTOM_DIALECT});
    projection(&input, Some(LABEL_ID));
}

#[test]
fn schema_keywords_anywhere_disqualify_inline_labels_only() {
    for keyword in ["$id", "$anchor", "$dynamicRef", "$schema"] {
        let mut input = document(json!([label(json!({"type": "string"}))]));
        input["components"]["schemas"]["Other"] = json!({(keyword): "foreign"});
        projection(&input, None);
        let parameter = &mut input["paths"][CAPTURE_PATH]["get"]["parameters"][0];
        parameter["style"] = json!("simple");
        parameter["schema"] = json!({"$ref": "#/missing"});
        projection(&input, Some(PLAIN_ID));
        input["paths"][CAPTURE_PATH]["get"]["parameters"][0]
            .as_object_mut()
            .unwrap()
            .remove("style");
        projection(&input, Some(PLAIN_ID));
    }
}

#[test]
fn label_requires_readable_lists_at_both_precedence_levels() {
    for bad_list in [json!({}), json!([null]), json!([{"in": "path"}])] {
        let mut input = document(json!([label(json!({"type": "string"}))]));
        input["paths"][CAPTURE_PATH]["parameters"] = bad_list;
        projection(&input, None);
        input["paths"][CAPTURE_PATH]["get"]["parameters"][0]["style"] = json!("simple");
        projection(&input, Some(PLAIN_ID));
    }
}

#[test]
fn label_with_an_inline_schema_containing_a_reference_is_withheld() {
    let mut input = document(json!([label(
        json!({"type": "string", "allOf": [{"minLength": 1}]})
    )]));
    projection(&input, Some(LABEL_ID));
    input["components"]["schemas"]["Length"] = json!({"minLength": 1});
    input["paths"][CAPTURE_PATH]["get"]["parameters"][0]["schema"]["allOf"][0] =
        json!({"$ref": "#/components/schemas/Length"});
    projection(&input, None);
}

#[test]
fn label_inherited_from_another_document_is_withheld() {
    let scalar = label(json!({"type": "string"}));
    let mut input = document(json!([scalar]));
    let parts = serde_json::to_vec(&json!({"item": {"parameters": [scalar]}})).unwrap();
    let references = [ReferenceBlob {
        retrieval_uri: "https://example.test/parts.json",
        blob: Blob {
            path: "parts.json",
            tree: "test-tree",
            oid: "parts",
            bytes: &parts,
            format: Format::Json,
        },
    }];
    input["paths"][CAPTURE_PATH]["$ref"] = json!("parts.json#/item");
    let read = |input: &Value| {
        index(
            &serde_json::to_vec(input).unwrap(),
            Format::Json,
            "test-api",
            Some(ENTRY_URI),
            &references,
        )
        .unwrap()
    };
    assert_projection(&read(&input), Some(LABEL_ID));
    input["paths"][CAPTURE_PATH]["get"]["parameters"] = json!([]);
    assert_projection(&read(&input), None);
}

#[test]
fn path_parameter_override_is_keyed_by_name_and_location() {
    let local = json!({"in": "query", "name": "id", "schema": {"type": "string"}});
    let mut input = document(json!([local]));
    input["paths"][CAPTURE_PATH]["parameters"] = json!([label(json!({"type": "string"}))]);
    projection(&input, Some(LABEL_ID));
    let override_parameter = &mut input["paths"][CAPTURE_PATH]["get"]["parameters"][0];
    override_parameter["in"] = json!("path");
    override_parameter["required"] = json!(true);
    projection(&input, Some(PLAIN_ID));
}

#[test]
fn boundary_has_no_prism_dependency_before_adapters() {
    let manifest = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let output = Command::new(env!("CARGO"))
        .args([
            "metadata",
            "--offline",
            "--no-deps",
            "--format-version=1",
            "--manifest-path",
        ])
        .arg(manifest)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let metadata: Value = serde_json::from_slice(&output.stdout).unwrap();
    let package = metadata["packages"]
        .as_array()
        .unwrap()
        .iter()
        .find(|pkg| pkg["name"] == "prism-boundary")
        .unwrap();
    assert!(
        package["dependencies"]
            .as_array()
            .unwrap()
            .iter()
            .all(|dep| dep["name"] != "prism"),
        "add the prism dependency when adapters use prism::api"
    );
}
