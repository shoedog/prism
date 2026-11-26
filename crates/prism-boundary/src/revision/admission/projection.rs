use super::*;
use crate::revision::{
    AdmittedSchemaInput, Blob, DependencyReceipt, ProducerMediaEvidence, ReferenceBlob, Snapshot,
};

pub struct SchemaAdmission<'a> {
    projection: Option<Projection<'a>>,
    pub report: AdmissionReport,
}

pub(in crate::revision) struct Projection<'a> {
    pub scope: Scope,
    pub api: &'a str,
    pub authority: &'a str,
    pub snapshot: Snapshot<'a>,
    pub entry: Blob<'a>,
    pub retrieval_uri: Option<&'a str>,
    pub references: Vec<ReferenceBlob<'a>>,
    pub config_receipts: Vec<DependencyReceipt<'a>>,
    pub media_receipts: Vec<ProducerMediaEvidence<'a>>,
}

impl SchemaAdmission<'_> {
    /// Build the inventory scoped to the admitted claim.
    pub fn index(&self) -> Result<Option<crate::SchemaIndex>, crate::schema::SchemaError> {
        self.projection.as_ref().map(Projection::index).transpose()
    }

    /// Borrow the reader input, including its admitted reference and media slices.
    pub fn input(&self) -> Option<AdmittedSchemaInput<'_>> {
        Some(self.projection.as_ref()?.input())
    }
}

impl Projection<'_> {
    pub(in crate::revision) fn index(
        &self,
    ) -> Result<crate::SchemaIndex, crate::schema::SchemaError> {
        let index = crate::index_schema(&self.input())?;
        Ok(project_to_scope(index, self.scope.clone()))
    }

    pub(super) fn input(&self) -> AdmittedSchemaInput<'_> {
        AdmittedSchemaInput {
            api: self.api,
            authority: self.authority,
            snapshot: self.snapshot,
            entry: self.entry,
            retrieval_uri: self.retrieval_uri,
            references: &self.references,
            config_receipts: &self.config_receipts,
            media_receipts: &self.media_receipts,
        }
    }
}

pub fn admit_schema_input<'a>(
    request: &'a AdmissionRequest,
    bytes: &'a BTreeMap<String, Vec<u8>>,
    claim_id: &str,
) -> SchemaAdmission<'a> {
    let ledger = Ledger::new(request, bytes);
    let (mut report, mut projections) = ledger.resolve();
    let projection = projections.remove(claim_id);
    let failure = match report.claims.get(claim_id) {
        None => Some(Refusal::new("missing_receipt", claim_id)),
        Some(decision) if decision.status == "admitted" && projection.is_none() => {
            Some(Refusal::new("slot_mismatch", claim_id).dimension("purpose"))
        }
        _ => None,
    };
    if let Some(failure) = failure {
        report.settle(claim_id, None, vec![failure], []);
    }
    SchemaAdmission { projection, report }
}

fn project_to_scope(mut index: crate::SchemaIndex, scope: Scope) -> crate::SchemaIndex {
    if scope == crate::revision::Scope::Api {
        return index;
    }
    index.operations.retain(|op| {
        op.declarations
            .iter()
            .any(|declaration| scope.contains_prefix(&declaration.canonical_path.segments))
    });
    let ids: std::collections::BTreeSet<_> = index.operations.iter().map(|op| &op.id).collect();
    let owned = |field: &crate::inventory::Field| match &field.id.root {
        crate::element::MessageRoot::Request(op) | crate::element::MessageRoot::Response(op, _) => {
            ids.contains(op)
        }
    };
    index.fields.retain(owned);
    index.observed_fields.retain(owned);
    index.reached.retain(|op, _| ids.contains(op));
    for operations in index.inverse.values_mut() {
        operations.retain(|op| ids.contains(op));
    }
    let visible: std::collections::BTreeSet<_> = index
        .fields
        .iter()
        .chain(&index.observed_fields)
        .map(|field| field.id.to_string())
        .collect();
    index.public_fields.retain(|id| visible.contains(id));
    index.content_hash = crate::schema::signature::content_hash(&index);
    index.scope = scope;
    index
}
