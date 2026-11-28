use super::composition::{Effect, Stop};
use super::constraints::{nonnegative, type_bits, valid_required};
use super::{Dialect, Node};
use serde_json::Value;

pub(super) fn reference_scope(node: &Node<'_, '_>) -> bool {
    [
        "$id",
        "$anchor",
        "$dynamicAnchor",
        "$dynamicRef",
        "$recursiveRef",
        "$recursiveAnchor",
    ]
    .iter()
    .any(|key| node.get(key).is_some())
        || node.resource_scope_unchecked()
}

pub(super) fn stops(
    node: &Node<'_, '_>,
    dialect: Dialect,
    property: bool,
    required: bool,
    request: bool,
    union: bool,
) -> Vec<Stop> {
    if node.duplicate_members() {
        return vec![Stop("duplicate_member", Effect::Field)];
    }
    if node.value.is_boolean() {
        return if dialect == Dialect::OpenApi31 || node.policy {
            Vec::new()
        } else {
            vec![Stop("schema_form_unchecked", Effect::Field)]
        };
    }
    let Some(object) = node.value.as_object() else {
        return vec![Stop("schema_form_unchecked", Effect::Field)];
    };
    let mut stops = Vec::new();
    if dialect_unchecked(node) {
        return vec![Stop("schema_dialect_unchecked", Effect::Field)];
    }
    if reference_scope(node) {
        stops.push(Stop("reference_scope", Effect::Children));
    }
    let read_only = node.get("readOnly");
    let write_only = node.get("writeOnly");
    if [read_only, write_only]
        .into_iter()
        .flatten()
        .any(|value| !value.is_boolean())
        || !property && (read_only.is_some() || write_only.is_some())
        || dialect == Dialect::OpenApi30
            && read_only == Some(&Value::Bool(true))
            && write_only == Some(&Value::Bool(true))
        || dialect == Dialect::Swagger2
            && (write_only.is_some() || request && read_only == Some(&Value::Bool(true)))
    {
        stops.push(Stop("direction_flags_unchecked", Effect::Field));
    } else if dialect == Dialect::OpenApi31
        && required
        && (if request { read_only } else { write_only }) == Some(&Value::Bool(true))
    {
        stops.push(Stop("direction_policy_unchecked", Effect::Field));
    }
    for (key, value) in object {
        let issue = match key.as_str() {
            "$ref" | "$schema" | "$vocabulary" | "$id" | "$anchor" | "$dynamicAnchor"
            | "$dynamicRef" | "$recursiveRef" | "$recursiveAnchor" | "readOnly" | "writeOnly"
            | "allOf" => None,
            "$defs" if dialect == Dialect::OpenApi31 && value.is_object() => None,
            "anyOf" | "oneOf" if dialect != Dialect::Swagger2 => None,
            "properties" if value.is_object() => None,
            "additionalProperties" if value.is_boolean() || value.is_object() => None,
            "required" if !valid_required(value, dialect) => Some("required_unreadable"),
            "required" => None,
            "type" if valid_types(value, dialect) => None,
            "items" if value.is_array() => Some("indexed_tuple"),
            "items" if value.is_object() || dialect == Dialect::OpenApi31 && value.is_boolean() => {
                None
            }
            "items" => Some("schema_form_unchecked"),
            "prefixItems" | "unevaluatedItems" if dialect == Dialect::OpenApi31 => {
                Some("indexed_tuple")
            }
            "additionalItems" if node.get("items").is_some_and(Value::is_array) => {
                Some("indexed_tuple")
            }
            "enum" => match value.as_array() {
                None => Some("enum_unreadable"),
                Some(items) if items.is_empty() && dialect == Dialect::Swagger2 => {
                    Some("enum_unreadable")
                }
                Some(items) if items.iter().any(|item| item.is_object() || item.is_array()) => {
                    Some("unsupported_schema_keyword")
                }
                Some(_) => None,
            },
            "const" if dialect == Dialect::OpenApi31 => {
                if value.is_object() || value.is_array() {
                    Some("unsupported_schema_keyword")
                } else if property && !union && node.get("enum").is_none() {
                    Some("const_signature_unchecked")
                } else {
                    None
                }
            }
            "nullable" if dialect == Dialect::OpenApi30 && value.is_boolean() => {
                if value == &Value::Bool(true) && node.get("type").is_none() {
                    Some("nullable_scope_unchecked")
                } else if value == &Value::Bool(true) && node.get("allOf").is_some() {
                    Some("nullable_composition_unchecked")
                } else {
                    None
                }
            }
            "minProperties" | "maxProperties" | "minItems" | "maxItems" | "minLength"
            | "maxLength"
                if nonnegative(value).is_some() =>
            {
                None
            }
            "minimum" | "maximum" if value.is_number() => None,
            "exclusiveMinimum" | "exclusiveMaximum"
                if dialect == Dialect::OpenApi31 && value.is_number() =>
            {
                None
            }
            "exclusiveMinimum"
                if dialect != Dialect::OpenApi31
                    && value.is_boolean()
                    && node.get("minimum").is_some() =>
            {
                None
            }
            "exclusiveMaximum"
                if dialect != Dialect::OpenApi31
                    && value.is_boolean()
                    && node.get("maximum").is_some() =>
            {
                None
            }
            "multipleOf"
                if value.is_number()
                    && !value.to_string().starts_with('-')
                    && super::super::signature::canonical_number(&value.to_string()) != "0" =>
            {
                None
            }
            "pattern" | "format" if value.is_string() => None,
            "uniqueItems" if value == &Value::Bool(true) => Some("array_uniqueness_unchecked"),
            "uniqueItems" if value == &Value::Bool(false) => None,
            "if" if dialect == Dialect::OpenApi31
                && node.get("then").is_none()
                && node.get("else").is_none() =>
            {
                None
            }
            "then" | "else" if dialect == Dialect::OpenApi31 && node.get("if").is_none() => None,
            "minContains" | "maxContains"
                if dialect == Dialect::OpenApi31 && node.get("contains").is_none() =>
            {
                None
            }
            "discriminator" if dialect == Dialect::Swagger2 && value.is_string() => {
                stops.push(Stop("discriminator_inheritance", Effect::Notice));
                None
            }
            "discriminator"
                if dialect != Dialect::Swagger2
                    && value.get("propertyName").is_some_and(Value::is_string) =>
            {
                if node.get("oneOf").is_none()
                    && node.get("anyOf").is_none()
                    && value.get("mapping").is_some()
                {
                    Some("discriminator_inheritance")
                } else {
                    None
                }
            }
            "default" | "description" | "title" | "example" | "examples" | "deprecated"
            | "externalDocs" | "xml" | "contentEncoding" | "contentMediaType" | "contentSchema"
            | "$comment" | "x-enum-varnames" => None,
            _ if key.starts_with("x-") => Some("schema_extension_unchecked"),
            "type"
            | "properties"
            | "additionalProperties"
            | "minProperties"
            | "maxProperties"
            | "minItems"
            | "maxItems"
            | "minLength"
            | "maxLength"
            | "minimum"
            | "maximum"
            | "exclusiveMinimum"
            | "exclusiveMaximum"
            | "multipleOf"
            | "uniqueItems"
            | "pattern"
            | "format"
            | "nullable"
            | "discriminator" => Some("schema_keyword_unreadable"),
            _ => Some("unsupported_schema_keyword"),
        };
        if let Some(issue) = issue {
            stops.push(Stop(issue, Effect::Field));
        }
    }
    if dialect != Dialect::OpenApi31
        && node.get("type").and_then(Value::as_str) == Some("array")
        && node.get("items").is_none()
        && node.get("prefixItems").is_none()
    {
        stops.push(Stop("schema_form_unchecked", Effect::Field));
    }
    stops
}

pub(super) fn dialect_unchecked(node: &Node<'_, '_>) -> bool {
    node.document
        .value
        .get("jsonSchemaDialect")
        .is_some_and(|value| !supported_dialect(value))
        || node
            .get("$schema")
            .is_some_and(|value| !supported_dialect(value))
        || node.get("$vocabulary").is_some()
}

fn supported_dialect(value: &Value) -> bool {
    matches!(
        value.as_str(),
        Some(
            "https://json-schema.org/draft/2020-12/schema"
                | "https://spec.openapis.org/oas/3.1/dialect/base"
        )
    )
}

fn valid_types(value: &Value, dialect: Dialect) -> bool {
    let valid = |value: &Value| {
        value.as_str().is_some_and(|name| {
            type_bits(name).is_some() && (name != "null" || dialect != Dialect::OpenApi30)
        })
    };
    valid(value)
        || dialect != Dialect::OpenApi30
            && value.as_array().is_some_and(|types| {
                !types.is_empty()
                    && types.iter().all(valid)
                    && types
                        .iter()
                        .filter_map(Value::as_str)
                        .collect::<std::collections::BTreeSet<_>>()
                        .len()
                        == types.len()
            })
}
