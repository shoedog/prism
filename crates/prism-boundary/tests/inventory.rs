#[path = "support/admission.rs"]
mod admission;

use prism_boundary::element::{schema_path, OperationId};
use prism_boundary::revision::{Format, Snapshot};
use prism_boundary::{
    definitions_of, index_schema, operations_reaching, DefinitionId, Lookup, SchemaIndex,
    SchemaSource,
};
use sha2::{Digest, Sha256};

const URI: &str = "https://schema.invalid/spec.json";
const SHARED: &[u8] = br##"{"openapi":"3.1.0","info":{"title":"Fixture","version":"7"},"paths":{
    "/a":{"post":{"requestBody":{"content":{"application/json":{"schema":{"$ref":"#/components/schemas/Shared"}}}}}},
    "/b":{"get":{"responses":{"200":{"description":"ok","content":{"application/json":{"schema":{
        "type":"object","properties":{"left":{"$ref":"#/components/schemas/Shared"},"right":{"$ref":"#/components/schemas/Shared"}}
    }}}}}}},
    "/clean":{"get":{"responses":{"204":{"description":"empty"}}}}
},"components":{"schemas":{
    "Shared":{"properties":{"child":{"$ref":"#/components/schemas/Child"}}},
    "Child":{"type":"object","properties":{"leaf":{"type":"string"}}},
    "Unused":{"type":"object","properties":{"unused":{"type":"string"}}}
}}}"##;

fn admit(bytes: &[u8]) -> SchemaIndex {
    serde_json::from_slice::<serde_json::Value>(bytes).expect("well-formed JSON fixture");
    let mut input = admission::admitted(bytes, Format::Json, "test-api", Some(URI), &[]);
    input.snapshot = Snapshot::Commit {
        tree: "test-tree",
        commit: "test-revision",
        subject: None,
        committer_date: None,
    };
    input.entry.path = "docs/spec.json";
    index_schema(&input).unwrap()
}

fn op(method: &str, path: &str) -> OperationId {
    OperationId::new(method, &schema_path(path, "", &[]))
}

fn definition(name: &str) -> DefinitionId {
    DefinitionId {
        resource: URI.into(),
        pointer: format!("/components/schemas/{name}"),
    }
}

#[test]
fn schema_index_binds_byte_hash_source_and_info_version() {
    let index = admit(SHARED);
    assert_eq!(
        index.source,
        Some(SchemaSource {
            api: "test-api".into(),
            authority: "schema".into(),
            tree: "test-tree".into(),
            commit: Some("test-revision".into()),
            path: "docs/spec.json".into(),
            blob: "entry-blob".into(),
        }),
        "inventory binds the admitted schema source and revision"
    );
    assert_eq!(index.info_version.as_deref(), Some("7"));
    assert_eq!(index.byte_sha256, format!("{:x}", Sha256::digest(SHARED)));
    let respaced = [SHARED, b"\n"].concat();
    let other = admit(&respaced);
    assert_ne!(index.byte_sha256, other.byte_sha256);
    assert_eq!(index.content_hash, other.content_hash);
}

#[test]
fn reached_definitions_and_inverse_deduplicate_repeated_paths() {
    let index = admit(SHARED);
    let a = op("POST", "/a");
    let b = op("GET", "/b");
    let expected = [definition("Child"), definition("Shared")];
    assert_eq!(
        definitions_of(&index, &a),
        Lookup::Checked(&expected),
        "/a reaches Shared and Child"
    );
    assert_eq!(
        definitions_of(&index, &b),
        Lookup::Checked(&expected),
        "/b resolves two uses of Shared once each"
    );
    for def in expected {
        assert_eq!(
            operations_reaching(&index, &def),
            Lookup::Checked(&[b.clone(), a.clone()]),
            "inverse definition relation lists both operations exactly once"
        );
    }
    let mut child_fields: Vec<_> = index
        .fields
        .iter()
        .filter(|field| field.definitions.contains(&definition("Child")))
        .map(|field| field.id.to_string())
        .collect();
    child_fields.sort();
    assert_eq!(
        child_fields,
        [
            "field:req:POST /a#/child/leaf",
            "field:resp:GET /b:200#/left/child/leaf",
            "field:resp:GET /b:200#/right/child/leaf",
        ],
        "repeated references expand at each field use"
    );
}

#[test]
fn checked_empty_lookup_and_unknown_scope_are_distinct() {
    let index = admit(SHARED);
    let clean = op("GET", "/clean");
    let a = op("POST", "/a");
    let b = op("GET", "/b");
    let unused = definition("Unused");
    assert_eq!(
        definitions_of(&index, &clean),
        Lookup::Checked(&[]),
        "checked bodyless operation reaches no definitions"
    );
    assert_eq!(
        operations_reaching(&index, &unused),
        Lookup::Checked(&[]),
        "declared unused definition has a checked empty inverse"
    );
    assert_eq!(
        definitions_of(&index, &op("GET", "/unindexed")),
        Lookup::Unknown
    );
    assert_eq!(
        operations_reaching(&index, &definition("Unbound")),
        Lookup::Unknown
    );

    let blocked = String::from_utf8(SHARED.to_vec()).unwrap().replace(
        r##""left":{"$ref":"#/components/schemas/Shared"}"##,
        r#""left":{"$ref":"unavailable.json"}"#,
    );
    let partial = admit(blocked.as_bytes());
    let expected = [definition("Child"), definition("Shared")];
    assert_eq!(
        definitions_of(&partial, &b),
        Lookup::Partial(&expected),
        "scoped stop retains independently reached definitions"
    );
    assert_eq!(definitions_of(&partial, &a), Lookup::Checked(&expected));
    assert_eq!(definitions_of(&partial, &clean), Lookup::Checked(&[]));
    assert_eq!(
        operations_reaching(&partial, &definition("Shared")),
        Lookup::Checked(&[b, a]),
        "known inverse has no unchecked operation outside it"
    );
    assert!(
        matches!(
            operations_reaching(&partial, &unused),
            Lookup::Unknown | Lookup::Partial([])
        ),
        "a scoped stop may hide a reach, so empty is no longer checked"
    );
}
