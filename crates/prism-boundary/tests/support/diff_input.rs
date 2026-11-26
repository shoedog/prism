use crate::{admission, matrix};
use prism_boundary::element::{schema_path, OperationId};
use prism_boundary::revision::{
    Blob, DependencyReceipt, Format, ProducerMediaEvidence, ReferenceBlob, Snapshot,
};
use prism_boundary::{
    index_schema, AttributeValue, ComparisonScope, Coverage, DiffBasis, DiffSide, Fact,
    HandlerBody, HandlerEndpoint, HandlerReference, ProducerFact, ProducerInventory,
    ProducerOperation, ProducerSite, ProducerSiteKind, Property, RouteDeclaration, SchemaIndex,
    Validator,
};
use serde_json::Value;
use std::fs;

pub fn fixture(matrix_name: &str, name: &str) -> Vec<u8> {
    fs::read(matrix::matrix_path(
        matrix_name,
        &format!("fixtures/{name}"),
    ))
    .unwrap()
}

pub fn schema(bytes: &[u8], api: &str, config: &Value, side: usize) -> SchemaIndex {
    let tree = ["before-tree", "after-tree"][side];
    let blob = ["before-blob", "after-blob"][side];
    let dependencies: Vec<_> = config["references"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|reference| {
            let name = reference["fixture"].as_str().unwrap();
            (
                name,
                reference["uri"].as_str().unwrap(),
                fixture("p1c_rows", name),
            )
        })
        .collect();
    let references: Vec<_> = dependencies
        .iter()
        .map(|(name, uri, bytes)| ReferenceBlob {
            retrieval_uri: uri,
            blob: Blob {
                path: name,
                tree,
                oid: name,
                bytes,
                format: Format::Json,
            },
        })
        .collect();
    let mut input = admission::admitted(bytes, Format::Json, api, None, &references);
    input.authority = config["authority"].as_str().unwrap_or("schema");
    input.snapshot = Snapshot::ComputedTree {
        tree,
        computed_at: "2026-11-25T00:00:00Z",
        merge_receipt: "diff-fixture",
    };
    input.entry.tree = tree;
    input.entry.oid = blob;
    let proof = config["media_proof"]
        .as_str()
        .map(|name| fixture("p1c_rows", name));
    let receipts = proof.as_ref().map(|bytes| ProducerMediaEvidence {
        receipt: DependencyReceipt {
            input_id: "media-catalog",
            tree,
            path: config["media_proof"].as_str().unwrap(),
            blob: "media-catalog-blob",
            entry: 1,
        },
        bytes,
    });
    input.media_receipts = receipts.as_slice();
    let mut index = index_schema(&input).expect("fixture produces a schema inventory");
    if config["missing_required"] == true {
        index.fields[0].shape.required = Property::Missing;
    }
    if config["missing_source"] == true {
        index.source = None;
    }
    index
}

pub struct Input<'a> {
    pub schema: Option<SchemaIndex>,
    pub producer: Option<ProducerInventory<'a>>,
    pub basis: Option<DiffBasis<'a>>,
}

impl<'a> Input<'a> {
    pub fn load(config: &'a Value, side: usize) -> Self {
        let name = config["fixture"].as_str().unwrap();
        let bytes = fixture("p1c_rows", name);
        let api = config["api"].as_str().unwrap_or("test-api");
        let tree = ["before-tree", "after-tree"][side];
        let scope = || {
            config["scope"]
                .as_str()
                .map_or(ComparisonScope::Api, |prefix| {
                    ComparisonScope::PathPrefix(schema_path(prefix, "", &[]))
                })
        };
        if config["producer"] == true {
            let basis = DiffBasis {
                api,
                authority: "route_table",
                tree,
                scope: scope(),
            };
            let value = serde_json::from_slice(&bytes).expect("typed producer fixture");
            let mut producer = producer(basis.clone(), &value);
            let op = &mut producer.operations[0];
            if let Some(reason) = config["declaration_withheld"].as_str() {
                op.declaration.fact = Fact::Withheld {
                    reason: reason.into(),
                };
            } else if config["declaration_absent"] == true {
                op.declaration.fact = Fact::Checked(None);
            }
            if config["declaration_unbound"] == true {
                op.declaration.sites[0].tree = "unbound-tree".into();
            }
            if config["operations_empty"] == true {
                producer.operations.clear();
            }
            return Self {
                schema: None,
                producer: Some(producer),
                basis: Some(basis),
            };
        }
        let explicit_basis = config["scope"].as_str().or(config["basis_tree"].as_str());
        let basis = explicit_basis.map(|_| DiffBasis {
            api,
            authority: config["authority"].as_str().unwrap_or("schema"),
            tree: config["basis_tree"].as_str().unwrap_or(tree),
            scope: scope(),
        });
        Self {
            schema: Some(schema(&bytes, api, config, side)),
            producer: None,
            basis,
        }
    }

    pub fn side(&'a self) -> DiffSide<'a> {
        DiffSide {
            basis: self.basis.clone(),
            schema: self.schema.as_ref(),
            producer: self.producer.as_ref(),
        }
    }
}

fn producer<'a>(basis: DiffBasis<'a>, value: &Value) -> ProducerInventory<'a> {
    use ProducerSiteKind::*;

    let original_path = value["path"].as_str().unwrap_or("/kiln");
    let canonical_path = schema_path(original_path, "", &[]);
    let start = value["start"].as_u64().unwrap() as usize;
    let site = |kind, offset| ProducerSite {
        kind,
        tree: basis.tree.into(),
        file: value["file"].as_str().unwrap().into(),
        blob: format!("{}-source-blob", basis.tree),
        span: (start + offset)..(start + offset + 8),
    };
    let endpoint = match value["endpoint"].as_str().unwrap() {
        "inline" => HandlerEndpoint::Inline,
        name => HandlerEndpoint::Resolved(format!("module::{name}")),
    };
    let validation = match value["validation"]["withheld"].as_str() {
        Some(reason) => Fact::Withheld {
            reason: reason.into(),
        },
        None => Fact::Checked(value["validation"].as_str().map(|digest| Validator {
            identity: None,
            digest: Some(digest.into()),
        })),
    };
    let code = match value["code"]["withheld"].as_str() {
        Some(reason) => Fact::Withheld {
            reason: reason.into(),
        },
        None => Fact::Checked(Some(HandlerBody {
            endpoint: endpoint.clone(),
            digest: value["code"].as_str().unwrap().into(),
        })),
    };
    ProducerInventory {
        coverage: Coverage::Checked,
        operations: vec![ProducerOperation {
            operation: OperationId::new("GET", &canonical_path),
            declaration: fact(
                Fact::Checked(Some(RouteDeclaration {
                    original_method: "GET".into(),
                    original_path: original_path.into(),
                    canonical_path,
                    attributes: [(
                        "priority".into(),
                        AttributeValue::Integer(value["priority"].as_i64().unwrap()),
                    )]
                    .into(),
                })),
                vec![site(Declaration, 0), site(Registration, 10)],
            ),
            validation: fact(validation, vec![site(Validation, 20)]),
            reference: fact(
                Fact::Checked(Some(HandlerReference {
                    expression: value["expression"].as_str().unwrap().into(),
                    endpoint,
                })),
                vec![site(Reference, 30), site(Endpoint, 40)],
            ),
            code: fact(code, vec![site(Endpoint, 40)]),
        }],
        basis,
    }
}

fn fact<T>(fact: Fact<T>, sites: Vec<ProducerSite>) -> ProducerFact<'static, T> {
    ProducerFact {
        fact,
        sites,
        dependencies: Vec::new(),
    }
}
