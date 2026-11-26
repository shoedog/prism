// P1b accept-list
// Swagger 2 and OpenAPI 3.0/3.1 JSON with admitted entry bytes and static pointer dependencies.
// Dialect-eligible request bodies and explicit success keys with exact-over-range status selection.
// Exact application/json, effective Swagger media lists, or admitted root-scoped Swagger omission proof.
// Named property/required edges, homogeneous items, and schema-valued map remainders with typed pointers.
// Feasible scalar, object, array, allOf, anyOf, and provably disjoint oneOf transitions within 16 edges.
// Unsupported or unreadable inputs leave counted partial or withheld coverage at the affected scope.

// P1d document or saved-match controls
// control-document-client-only-held-service, control-document-computed-tree-borrowed-date,
// control-document-computed-tree-null-metadata, control-document-configured-provenance,
// control-document-joint-edit-four-comparisons, control-document-keyless-coverage,
// control-document-saved-inventory-withheld, control-document-side-presence,
// control-document-tracked-dirtiness, control-document-unbound-raw-call,
// control-document-zero-saved-matches
//
// P2 producer controls
// control-g01-method-annotation, control-g02-two-annotations,
// control-g03-detached-agreeing-registration, control-g04-contradicting-registration,
// control-g04-missing-registration, control-g04-partial-registration,
// control-g05-duplicate-annotations, control-g06-tagged-field, control-g07-ignored-field,
// control-g08-untagged-field, control-g09-embedded-field, control-g10-generated-annotation,
// control-g11-definition-rename, control-p2-agreeing-registration-no-annotation,
// control-p2-linked-method-drift, control-p2-linked-path-drift
//
// P1a route spelling, policy, authority or diagnostic
// control-gin-catchall-version-withheld, control-route-closed-wildcard-withheld,
// control-route-custom-keys-and-mount-join-policy-withheld,
// control-route-custom-keys-and-mount-method-key-withheld,
// control-route-express4-bare-star-version-withheld,
// control-route-express4-optional-version-withheld, control-route-hapi-catchall-version-withheld,
// control-route-hapi-fixed-catchall-version-withheld, control-route-hapi-insensitive-withheld,
// control-route-hapi-optional-version-withheld, control-route-hapi-strip-trailing-withheld,
// control-route-hapi-vhosts-withheld, control-route-hapi-wildcard-method-withheld,
// control-route-header-dispatch-withheld,
// control-route-method-lowercase-method-normalization-withheld,
// control-route-mount-parameter-path-dialect-withheld,
// control-route-mount-root-join-policy-withheld, control-route-plugin-mount-withheld,
// control-s1-hapi-internal-route-withheld,
// control-s1-hapi-optional-mixed-segment-version-withheld,
// control-s1-hapi-prefix-on-plugin-root-withheld, gin-catchall, gin-clean-slash-join, gin-colon,
// gin-distinct-head-options, gin-group-supplied-prefix, gin-handle-custom-method,
// gin-literal-nested-groups, gin-match-list, gin-mixed-segment, gin-strict-trailing-slash,
// route-closed-wildcard, route-colon-required, route-configured-external-base,
// route-custom-keys-and-mount, route-express4-bare-star, route-express4-optional,
// route-gateway-stage, route-hapi-braces, route-hapi-catchall, route-hapi-fixed-catchall,
// route-hapi-insensitive, route-hapi-optional, route-hapi-strip-trailing, route-hapi-vhosts,
// route-hapi-wildcard-method, route-header-dispatch, route-method-head-options, route-method-list,
// route-method-lowercase, route-mount-parameter, route-mount-root, route-plugin-mount,
// route-proxy-rewrite, s1-express4-constrained-captures, s1-express4-generated-options,
// s1-express4-query-dispatch, s1-express5-mount-aliases, s1-express5-optional-literal-group,
// s1-express5-quoted-parameter, s1-express5-root-inclusive-wildcard, s1-gin-raw-encoded-slash,
// s1-gin-static-file-methods, s1-hapi-internal-route, s1-hapi-optional-mixed-segment,
// s1-hapi-prefix-on-plugin-root, swag-detached-annotation, swag-method-doc-comment,
// swag-method-drift, swag-path-drift, swag-two-annotations, t1-express-array-paths,
// t1-express-implicit-head
//
// P1c retained producer edit
// control-retained-route-handler-edit
//
// P1a schema address, deployment, method, capture or reference identity
// control-t1-relative-server-document-base-withheld, oas-absolute-server, oas-adjacent-braces,
// oas-authority-only, oas-braces, oas-colon-literal, oas-encoded-slash, oas-implicit-server,
// oas-in-segment-braces, oas-literal-vs-template, oas-operation-keys,
// oas-operation-server-overrides, oas-operationid-attribute, oas-path-case,
// oas-path-item-metadata, oas-path-server-overrides, oas-relative-server,
// oas-server-host-variable, oas-server-order-north-first, oas-server-order-south-first,
// oas-server-partial-variable, oas-server-path-variable, oas-server-trailing-join,
// oas-server-variable-collapse, oas-server-variable-distinct-operations,
// oas-significant-repeated-slash, oas-significant-trailing-slash, oas-uppercase-key,
// s1-oas31-empty-server-list, s1-oas31-label-path-parameter, s1-oas31-network-path-server,
// swagger-base, swagger-braces, swagger-encoded-braces, swagger-no-base, swagger-operation-keys,
// swagger-path-item-metadata, swagger-repeated-name, swagger-root, swagger-root-base,
// swagger-root-with-base, swagger-trace-key, t1-array-prefix-index-collision,
// t1-path-item-reference, t1-relative-server-document-base
//

#[path = "support/admission.rs"]
mod admission;
#[allow(dead_code)]
#[path = "support/matrix.rs"]
mod matrix;
#[path = "support/pinned_wire.rs"]
mod pinned_wire;
#[path = "support/wire_cases.rs"]
mod wire_cases;
#[path = "support/wire_media.rs"]
mod wire_media;

use matrix::{p1b_rows, rows, Row};
use serde_json::Value;
use std::collections::BTreeSet;
use wire_cases::{check, strings};

#[test]
fn pinned_message_status_and_wire_rows() {
    let expectations: Value =
        serde_json::from_str(include_str!("projections/pinned-wire.json")).unwrap();
    let companion = pinned_wire::companion();
    let driven: Vec<_> = rows().filter(pinned_wire::is_wire_row).collect();
    assert_eq!(driven.len(), 155, "P1b ids-matrix row selection");
    assert_eq!(
        driven
            .iter()
            .filter(|row| !row.id.starts_with("control-"))
            .count(),
        121
    );
    assert_eq!(
        driven.len(),
        expectations.as_object().unwrap().len(),
        "pinned wire expectation coverage"
    );
    let mut failures = Vec::new();
    for row in driven {
        let inputs = expectations[row.id]["inputs"].as_array().unwrap();
        let (operations, fields, _) = expected_ids(inputs);
        assert_eq!(
            operations,
            row.operations.iter().map(|id| id.to_string()).collect(),
            "{} operation TSV",
            row.id
        );
        let extra_observations: Vec<_> = companion[row.id]["observed"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
            .filter(|text| text.starts_with('#'))
            .map(|text| text.split(" (").next().unwrap())
            .collect();
        let mut amended_fields: BTreeSet<_> = row.fields.iter().map(|id| id.to_string()).collect();
        for pointer in extra_observations {
            let op = row.operations[0].strip_prefix("op:").unwrap();
            amended_fields.insert(format!("field:resp:{op}:200{pointer}"));
        }
        assert_eq!(
            fields
                .into_iter()
                .map(str::to_string)
                .collect::<BTreeSet<_>>(),
            amended_fields,
            "{} field TSV and companion observations",
            row.id
        );
        pinned_wire::check_companion(&row, &companion[row.id], inputs);
        for input in inputs {
            check(&row, input, &mut failures);
        }
    }
    assert!(
        failures.is_empty(),
        "pinned message/status/wire inventory\n{}",
        failures.join("\n")
    );
}

#[test]
fn enumerated_wire_cases() {
    check_enumeration(
        |row| !matches!(row.class, "known_limits" | "known_limits_control"),
        40,
    );
}

#[test]
fn known_limits() {
    check_enumeration(
        |row| matches!(row.class, "known_limits" | "known_limits_control"),
        72,
    );
}

fn check_enumeration(select: impl Fn(&Row<'_>) -> bool, count: usize) {
    let expectations: Value =
        serde_json::from_str(include_str!("fixtures/p1b_rows/expectations.json")).unwrap();
    let mut failures = Vec::new();
    let mut checked = 0;
    for row in p1b_rows().filter(select) {
        checked += 1;
        let entry = &expectations[row.id];
        let inputs = entry["inputs"].as_array().unwrap();
        let (operations, fields, warnings) = expected_ids(inputs);
        assert_eq!(
            operations,
            row.operations.iter().map(|id| id.to_string()).collect(),
            "{} operation TSV",
            row.id
        );
        assert_eq!(
            fields,
            row.fields.iter().copied().collect(),
            "{} field TSV",
            row.id
        );
        assert_eq!(
            warnings,
            row.warnings.iter().copied().collect(),
            "{} warning TSV",
            row.id
        );
        if row.class == "known_limits" {
            let control = entry["control"].as_str().unwrap();
            assert_eq!(
                entry["reason"].as_str(),
                row.reason,
                "{} limit reason",
                row.id
            );
            assert!(
                wire_cases::names_reason(inputs, row.reason.unwrap()),
                "{} counted limit",
                row.id
            );
            assert!(
                expectations[control]["inputs"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|input| !strings(&input["fields"]).is_empty()),
                "{control} emits a field"
            );
        }
        for input in inputs {
            check(&row, input, &mut failures);
        }
    }
    assert_eq!(checked, count);
    assert!(
        failures.is_empty(),
        "enumerated wire inventory\n{}",
        failures.join("\n")
    );
}

fn expected_ids(inputs: &[Value]) -> (BTreeSet<String>, BTreeSet<&str>, BTreeSet<&str>) {
    let operations = inputs
        .iter()
        .flat_map(|input| wire_cases::members(&input["operations"]))
        .map(|(key, _)| format!("op:{key}"))
        .collect();
    let fields = inputs
        .iter()
        .filter(|input| input.get("error").is_none())
        .flat_map(|input| {
            strings(&input["fields"])
                .into_iter()
                .chain(strings(&input["observed"]))
        })
        .collect();
    let warnings = inputs
        .iter()
        .flat_map(|input| wire_cases::members(&input["warnings"]))
        .map(|(key, _)| key.as_str())
        .collect();
    (operations, fields, warnings)
}
