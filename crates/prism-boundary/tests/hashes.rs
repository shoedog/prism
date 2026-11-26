#[path = "support/admission.rs"]
mod admission;

use prism_boundary::revision::Format;
use prism_boundary::{index_schema, signature, EnumValue, FieldShape, Property, SchemaIndex};
use sha2::{Digest, Sha256};

fn shape() -> FieldShape {
    FieldShape {
        field_type: Property::Known(vec!["string".into()]),
        format: Property::Missing,
        required: Property::Known(false),
        enum_values: Property::Missing,
        item_type: Property::Missing,
    }
}

fn signature_with_enum(values: Vec<EnumValue>) -> String {
    signature(&FieldShape {
        enum_values: Property::Known(values),
        ..shape()
    })
}

fn index(bytes: &[u8]) -> SchemaIndex {
    serde_json::from_slice::<serde_json::Value>(bytes).expect("well-formed JSON fixture");
    index_schema(&admission::admitted(
        bytes,
        Format::Json,
        "test-api",
        None,
        &[],
    ))
    .unwrap()
}

fn digest(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

#[test]
fn signature_frames_properties_with_distinct_tags() {
    let cases = [
        ("missing", Property::Missing, r#"["missing"]"#),
        ("known empty", Property::Known(vec![]), r#"["known",[]]"#),
        (
            "branch-dependent",
            Property::BranchDependent,
            r#"["branch_dependent"]"#,
        ),
        ("unknown", Property::Unknown, r#"["unknown"]"#),
    ];
    let mut observed = Vec::new();
    let mut expected = Vec::new();
    for (label, value, tag) in cases {
        observed.push((
            label,
            signature(&FieldShape {
                enum_values: value,
                ..shape()
            }),
        ));
        expected.push((
            label,
            format!(r#"[["known","string"],["missing"],["known",false],{tag},["missing"]]"#),
        ));
    }
    assert_eq!(
        observed, expected,
        "tagged signature tuple in type/format/requiredness/enum/item order"
    );
}

#[test]
fn signature_deduplicates_and_sorts_enum_by_canonical_bytes() {
    let number = |literal: &str| EnumValue::Number(literal.into());
    let string = |literal: &str| EnumValue::String(literal.into());
    let members = vec![
        EnumValue::Null,
        number("2.00"),
        string("é"),
        number("1"),
        string("1"),
        EnumValue::Boolean(true),
        string("é"),
        string("e\u{301}"),
        string("\"\\\0\u{0008}\t\n\r\u{001f}"),
        number("2"),
        number("-0.0"),
        number("1200"),
        number("0.00120"),
        number("-12.00"),
    ];
    let observed = signature_with_enum(members.clone());
    let reversed = signature_with_enum(members.into_iter().rev().collect());
    assert_eq!(
        observed, reversed,
        "enum order is independent of declaration order"
    );
    assert_eq!(
        observed,
        concat!(
            r#"[["known","string"],["missing"],["known",false],["known",["1","#,
            r#""\"\\\u0000\u0008\u0009\u000a\u000d\u001f","é","é",-12,0,1,12e-4,12e2,2,null,true]],["missing"]]"#,
        ),
        "canonical typed, deduplicated enum bytes"
    );
}

#[test]
fn index_preserves_exact_numeric_members_before_signing() {
    fn schema(literal: &str) -> String {
        format!(
            r#"{{"openapi":"3.1.0","paths":{{"/p":{{"post":{{"requestBody":{{"content":{{"application/json":{{"schema":{{"type":"object","properties":{{"x":{{"enum":[{literal}]}}}}}}}}}}}}}}}}}}}}"#
        )
    }
    let spellings = [
        "1",
        "1.0",
        "1e0",
        "\"1\"",
        "9007199254740993",
        "9007199254740992",
    ];
    let indexes: Vec<_> = spellings
        .iter()
        .map(|number| index(schema(number).as_bytes()))
        .collect();
    let values: Vec<_> = indexes
        .iter()
        .filter_map(|index| index.fields.first())
        .map(|field| signature(&field.shape))
        .collect();
    assert_eq!(
        values.len(),
        6,
        "each exact numeric or string spelling emits its field"
    );
    assert_eq!(
        &values[..3],
        &[values[0].clone(), values[0].clone(), values[0].clone()]
    );
    assert_ne!(values[0], values[3], "number 1 differs from string 1");
    assert_ne!(
        values[4], values[5],
        "adjacent large integers remain distinct"
    );
    assert!(values[4].contains("9007199254740993") && values[5].contains("9007199254740992"));
    assert_eq!(indexes[0].content_hash, indexes[1].content_hash);
    assert_eq!(indexes[0].content_hash, indexes[2].content_hash);
    assert_ne!(indexes[0].content_hash, indexes[3].content_hash);
    assert_ne!(indexes[4].content_hash, indexes[5].content_hash);
}

const STATUS_SCHEMA: &str = r#"{"openapi":"3.1.0","paths":{"/z":{"post":{"responses":{
    "2XX":{"description":"range"},"204":{"description":"bodyless"},
    "200":{"description":"ok","content":{"application/json":{"schema":{"type":"object","properties":{"x":{"type":"string"}}}}}}
}}},"/a":{"get":{}}}}"#;

#[test]
fn inventory_hash_frames_each_explicit_key_and_sorts_by_id() {
    let before = index(STATUS_SCHEMA.as_bytes());
    let without_204 = STATUS_SCHEMA.replace(r#""204":{"description":"bodyless"},"#, "");
    let after = index(without_204.as_bytes());
    const FIELD: &str = concat!(
        r#"["field","field:resp:POST /z:200#/x",[["known","string"],["missing"],["known",false],["missing"],["missing"]]]"#,
        "\n"
    );
    let head = format!("{FIELD}[\"op\",\"op:GET /a\"]\n[\"op\",\"op:POST /z\"]\n[\"status\",\"op:POST /z\",\"200\"]\n");
    let end = "[\"status\",\"op:POST /z\",\"2XX\"]\n";
    let framed = format!("{head}[\"status\",\"op:POST /z\",\"204\"]\n{end}");
    assert_eq!(
        before.content_hash,
        digest(framed.as_bytes()),
        "canonical inventory record framing and LF"
    );
    assert_eq!(
        after.content_hash,
        digest(format!("{head}{end}").as_bytes()),
        "bodyless 204 owns its own inventory record"
    );
    let fields = |index: &SchemaIndex| {
        index
            .fields
            .iter()
            .map(|field| (field.id.to_string(), signature(&field.shape)))
            .collect::<Vec<_>>()
    };
    assert_eq!(
        fields(&before),
        fields(&after),
        "removing explicit 204 changes no fields"
    );
    assert_ne!(before.content_hash, after.content_hash);
}

#[test]
fn empty_inventory_hashes_zero_bytes() {
    let empty = index(br#"{"openapi":"3.1.0","paths":{}}"#);
    assert!(
        empty.operations.is_empty() && empty.fields.is_empty() && empty.observed_fields.is_empty()
    );
    assert_eq!(
        empty.content_hash,
        digest(b""),
        "empty inventory SHA-256 hashes zero bytes"
    );
}

fn typed_field(version: &str, schema: serde_json::Value) -> SchemaIndex {
    let bytes = serde_json::to_vec(&serde_json::json!({
        "openapi":version,"paths":{"/p":{"post":{"responses":{"200":{"content":{
            "application/json":{"schema":{"properties":{"x":schema}}}
        }}}}}}
    }))
    .unwrap();
    index(&bytes)
}

#[test]
fn list_item_hashes_exclude_impossible_members_when_feasibility_is_unknown() {
    use serde_json::json;

    let mut failures = Vec::new();
    for (label, items, widened, remaining) in [
        (
            "contradictory string lengths",
            json!({"type":["string","number"],"minLength":2,"maxLength":1,"multipleOf":2}),
            json!({"type":["string","number"]}),
            "number",
        ),
        (
            "fractional bounds exclude integers",
            json!({"type":["string","integer"],"minimum":0.1,"maximum":0.9,"pattern":"^a"}),
            json!({"type":["string","integer"]}),
            "string",
        ),
        (
            "string enum excludes integers",
            json!({"type":["string","integer"],"enum":["a"],"pattern":"^(a|b)$"}),
            json!({"type":["string","integer"]}),
            "string",
        ),
    ] {
        let restricted = typed_field("3.1.0", json!({"type":"array","items":items}));
        let widened = typed_field("3.1.0", json!({"type":"array","items":widened}));
        let ([restricted_field], [widened_field]) =
            (restricted.fields.as_slice(), widened.fields.as_slice())
        else {
            panic!("{label}: both inventories must contain one comparable array field");
        };
        assert_eq!(restricted_field.id, widened_field.id, "{label}");
        if restricted_field.shape.item_type != Property::Known(vec![remaining.to_owned()])
            || restricted.content_hash == widened.content_hash
        {
            failures.push(format!(
                "{label}: expected only {remaining} and distinct hashes, got {:?} -> {:?}, {} -> {}",
                restricted_field.shape.item_type,
                widened_field.shape.item_type,
                restricted.content_hash,
                widened.content_hash,
            ));
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn nullable_type_edits_change_the_signature_and_hash() {
    use serde_json::json;

    let mut collisions = Vec::new();
    for (label, version, before, after) in [
        (
            "non-nullable control",
            "3.1.0",
            json!({"type":"string"}),
            json!({"type":"integer"}),
        ),
        (
            "3.1 nullable",
            "3.1.0",
            json!({"type":["string","null"]}),
            json!({"type":["integer","null"]}),
        ),
        (
            "3.0 nullable",
            "3.0.3",
            json!({"type":"string","nullable":true}),
            json!({"type":"integer","nullable":true}),
        ),
    ] {
        let before = typed_field(version, before);
        let after = typed_field(version, after);
        let ([before_field], [after_field]) = (before.fields.as_slice(), after.fields.as_slice())
        else {
            panic!("{label}: both inventories must contain one comparable field");
        };
        assert_eq!(before_field.id, after_field.id, "{label}");
        let before_signature = signature(&before_field.shape);
        let after_signature = signature(&after_field.shape);
        if before_signature == after_signature || before.content_hash == after.content_hash {
            collisions.push(format!(
                "{label}: {before_signature} -> {after_signature}, {} -> {}",
                before.content_hash, after.content_hash
            ));
        }
    }
    assert!(collisions.is_empty(), "{}", collisions.join("\n"));
}

#[test]
fn explicit_nullable_types_change_the_effective_signature() {
    for (version, nullable) in [
        ("3.1.0", serde_json::json!({"type":["string","null"]})),
        (
            "3.0.3",
            serde_json::json!({"type":"string","nullable":true}),
        ),
    ] {
        let plain = typed_field(version, serde_json::json!({"type":"string"}));
        let mut restricted = nullable.clone();
        restricted["enum"] = serde_json::json!(["a"]);
        let control = typed_field(version, restricted);
        assert_eq!(control.fields[0].shape.field_type, shape().field_type);
        let mixed = typed_field(version, nullable);
        assert_ne!(
            mixed.content_hash, plain.content_hash,
            "{version}: an admitted null must change content_hash"
        );
        assert_eq!(
            signature(&mixed.fields[0].shape),
            r#"[["known",["null","string"]],["missing"],["known",false],["missing"],["missing"]]"#
        );
    }

    let forward = typed_field(
        "3.1.0",
        serde_json::json!({"type":["integer","null","string"]}),
    );
    let reversed = typed_field(
        "3.1.0",
        serde_json::json!({"type":["string","null","integer"]}),
    );
    let expected = r#"[["known",["integer","null","string"]],["missing"],["known",false],["missing"],["missing"]]"#;
    assert_eq!(signature(&forward.fields[0].shape), expected);
    assert_eq!(signature(&reversed.fields[0].shape), expected);
    assert_eq!(forward.content_hash, reversed.content_hash);

    let only_null = typed_field(
        "3.1.0",
        serde_json::json!({"type":["null","string"],"enum":[null]}),
    );
    assert_eq!(
        signature(&only_null.fields[0].shape),
        r#"[["known","null"],["missing"],["known",false],["known",[null]],["missing"]]"#
    );
}

#[test]
fn duplicate_success_key_has_the_canonical_status_record() {
    let single = r#"{"openapi":"3.1.0","paths":{"/p":{"get":{"responses":{"2XX":{}}}}}}"#;
    let duplicate = single.replace(r#""2XX":{}"#, r#""2XX":{},"2XX":{}"#);
    let checked = index(single.as_bytes());
    let withheld = index(duplicate.as_bytes());
    let expected = digest(b"[\"op\",\"op:GET /p\"]\n[\"status\",\"op:GET /p\",\"2XX\"]\n");
    assert_eq!(checked.content_hash, expected);
    assert_eq!(withheld.content_hash, expected);
    assert_eq!(
        withheld.operations[0].messages[0].coverage,
        prism_boundary::Coverage::Withheld
    );
}
