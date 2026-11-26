use super::*;
use crate::revision::{
    AdmittedSchemaInput, Blob, DependencyReceipt, Format, ProducerMediaEvidence, ReferenceBlob,
    Snapshot,
};

pub struct SchemaAdmission<'a> {
    projection: Option<Projection<'a>>,
    pub report: AdmissionReport,
}

struct Projection<'a> {
    api: &'a str,
    authority: &'a str,
    snapshot: Snapshot<'a>,
    entry: Blob<'a>,
    retrieval_uri: Option<&'a str>,
    references: Vec<ReferenceBlob<'a>>,
    config_receipts: Vec<DependencyReceipt<'a>>,
    media_receipts: Vec<ProducerMediaEvidence<'a>>,
}

impl SchemaAdmission<'_> {
    /// Borrow the reader input, including its admitted reference and media slices.
    pub fn input(&self) -> Option<AdmittedSchemaInput<'_>> {
        let input = self.projection.as_ref()?;
        Some(AdmittedSchemaInput {
            api: input.api,
            authority: input.authority,
            snapshot: input.snapshot,
            entry: input.entry,
            retrieval_uri: input.retrieval_uri,
            references: &input.references,
            config_receipts: &input.config_receipts,
            media_receipts: &input.media_receipts,
        })
    }
}

pub fn admit_schema_input<'a>(
    request: &'a AdmissionRequest,
    bytes: &'a BTreeMap<String, Vec<u8>>,
    claim_id: &str,
) -> SchemaAdmission<'a> {
    let ledger = Ledger::new(request, bytes);
    let mut report = ledger.report();
    let projection = match report.claims.get(claim_id) {
        Some(decision) if decision.status == "admitted" => match project(&ledger, claim_id) {
            Ok(projection) => Some(projection),
            Err(refusal) => {
                report.insert(claim_id, ClaimDecision::from_reasons(None, vec![refusal]));
                None
            }
        },
        Some(_) => None,
        None => {
            report.insert(
                claim_id,
                ClaimDecision::from_reasons(None, vec![Refusal::new("missing_receipt", claim_id)]),
            );
            None
        }
    };
    SchemaAdmission { projection, report }
}

fn project<'a>(ledger: &Ledger<'a>, claim_id: &str) -> Result<Projection<'a>, Refusal> {
    let claim = ledger
        .request
        .claims
        .iter()
        .find(|claim| claim.id == claim_id)
        .ok_or_else(|| Refusal::new("missing_receipt", claim_id))?;
    let input = ledger.input(&claim.input_id)?;
    if input.binding.role != Role::Producer || input.binding.authority != "schema" {
        return Err(Refusal::new("slot_mismatch", claim_id).dimension("authority"));
    }
    let (records, _) = ledger.closure(
        claim
            .receipt_ids
            .iter()
            .map(|id| (id.as_str(), "missing_receipt")),
    );
    let entry_id = claim.receipt_ids.iter().filter_map(|id| {
        records
            .iter()
            .copied()
            .find(|record| record.id == *id && record.kind == ReceiptKind::Blob)
    });
    let entry = slots::unique(entry_id, claim_id, "missing_receipt")?;
    let blob =
        blob(entry, ledger.bytes).ok_or_else(|| Refusal::new("unreadable_bytes", &entry.id))?;
    let mut output = Projection {
        api: &input.binding.api,
        authority: &input.binding.authority,
        snapshot: input
            .snapshot
            .as_snapshot()
            .ok_or_else(|| Refusal::new("unsupported_kind", &claim.input_id))?,
        entry: blob,
        retrieval_uri: entry.retrieval_uri.as_deref(),
        references: Vec::new(),
        config_receipts: Vec::new(),
        media_receipts: Vec::new(),
    };
    for record in records {
        if record.id == entry.id {
            continue;
        }
        match record.kind {
            ReceiptKind::Blob => {
                if let Some((retrieval_uri, blob)) = record
                    .retrieval_uri
                    .as_deref()
                    .zip(self::blob(record, ledger.bytes))
                {
                    output.references.push(ReferenceBlob {
                        retrieval_uri,
                        blob,
                    });
                }
            }
            ReceiptKind::Config | ReceiptKind::Mapping => {
                if let Some(receipt) = dependency(
                    record,
                    record
                        .entry
                        .or_else(|| input.binding.config.as_ref().map(|config| config.entry)),
                ) {
                    output.config_receipts.push(receipt);
                }
            }
            ReceiptKind::Media => {
                if let Some(receipt) = dependency(record, record.entry.or(Some(1))) {
                    let bytes = &ledger.bytes[&record.id];
                    let (sources, _) = ledger.closure(
                        record
                            .dependencies
                            .iter()
                            .map(|id| (id.as_str(), "missing_dependency")),
                    );
                    if media_sources_match(bytes, receipt, &sources) {
                        output
                            .media_receipts
                            .push(ProducerMediaEvidence { receipt, bytes });
                    }
                }
            }
            ReceiptKind::Source
            | ReceiptKind::Inventory
            | ReceiptKind::Derived
            | ReceiptKind::Unsupported => {}
        }
    }
    Ok(output)
}

fn dependency(record: &AdmissionReceipt, entry: Option<usize>) -> Option<DependencyReceipt<'_>> {
    let ReceiptOrigin::NamedTree {
        tree, path, blob, ..
    } = &record.origin
    else {
        return None;
    };
    Some(DependencyReceipt {
        input_id: &record.id,
        tree,
        path,
        blob,
        entry: entry?,
    })
}

fn blob<'a>(
    record: &'a AdmissionReceipt,
    bytes: &'a BTreeMap<String, Vec<u8>>,
) -> Option<Blob<'a>> {
    let (tree, path, blob) = match &record.origin {
        ReceiptOrigin::NamedTree {
            tree, path, blob, ..
        }
        | ReceiptOrigin::IndependentCutoff {
            covered_tree: Some(tree),
            path: Some(path),
            blob: Some(blob),
            ..
        } => (tree, path, blob),
        _ => return None,
    };
    Some(Blob {
        path,
        tree,
        oid: blob,
        bytes: bytes.get(&record.id)?,
        format: Format::Json,
    })
}

fn media_sources_match(
    bytes: &[u8],
    receipt: DependencyReceipt<'_>,
    sources: &[&AdmissionReceipt],
) -> bool {
    let Ok(catalog) = serde_json::from_slice::<serde_json::Value>(bytes) else {
        return false;
    };
    receipt
        .entry
        .checked_sub(1)
        .and_then(|entry| catalog.get(entry))
        .and_then(|record| record["coverage"]["paths"].as_array())
        .is_some_and(|paths| {
            paths.iter().all(|site| {
                sources.iter().any(|source| {
                    matches!(&source.origin, ReceiptOrigin::NamedTree { tree, path, blob, .. }
                    if source.kind == ReceiptKind::Source && tree == receipt.tree
                    && site["file"] == path.as_str() && site["blob"] == blob.as_str())
                })
            })
        })
}
