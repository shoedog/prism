use super::{projection::Projection, *};
use crate::revision::{Blob, DependencyReceipt, Format, ProducerMediaEvidence, ReferenceBlob};
use std::collections::BTreeSet;

pub(super) struct MediaDecision<'a> {
    pub receipt_id: &'a str,
    pub root: Option<MediaRootEvidence>,
    pub reasons: Vec<Refusal>,
}

impl<'a> Ledger<'a> {
    pub(super) fn conflicting_configs(&self) -> BTreeMap<&'a str, Refusal> {
        let mut keys = BTreeMap::<_, Vec<_>>::new();
        for receipt in &self.request.receipts {
            let ReceiptOrigin::NamedTree {
                tree, path, blob, ..
            } = &receipt.origin
            else {
                continue;
            };
            if receipt.kind != ReceiptKind::Config {
                continue;
            }
            if let Some(entry) = receipt.entry {
                let mut logical = receipt.clone();
                logical.id.clear();
                logical.entry = Some(entry);
                logical.dependencies.sort();
                logical.dependencies.dedup();
                keys.entry((tree, path, blob, entry))
                    .or_default()
                    .push((receipt.id.as_str(), logical));
            }
        }
        let mut conflicts = BTreeMap::new();
        for receipts in keys.into_values() {
            if receipts
                .iter()
                .skip(1)
                .any(|(_, logical)| logical != &receipts[0].1)
            {
                let evidence: BTreeSet<_> = receipts.iter().map(|(id, _)| *id).collect();
                for id in &evidence {
                    let mut refusal = Refusal::new("receipt_conflict", id);
                    refusal.evidence = evidence.iter().map(|id| (*id).to_owned()).collect();
                    conflicts.insert(*id, refusal);
                }
            }
        }
        conflicts
    }

    pub(super) fn resolve_schema(
        &self,
        input: &'a AdmissionInput,
        entry: &'a AdmissionReceipt,
        records: &[&'a AdmissionReceipt],
        isolated: &BTreeSet<&str>,
        scope: &Scope,
        claim: &'a AdmissionClaim,
    ) -> Result<(Projection<'a>, Vec<MediaDecision<'a>>), Refusal> {
        if input.binding.role != Role::Producer || input.binding.authority != "schema" {
            return Err(Refusal::new("slot_mismatch", &entry.id).dimension("authority"));
        }
        let mut output = Projection {
            scope: scope.clone(),
            api: &input.binding.api,
            authority: &input.binding.authority,
            snapshot: input
                .snapshot
                .as_snapshot()
                .ok_or_else(|| Refusal::new("unsupported_kind", &input.binding.input_id))?,
            entry: blob(entry, self.bytes)
                .ok_or_else(|| Refusal::new("unreadable_bytes", &entry.id))?,
            retrieval_uri: entry.retrieval_uri.as_deref(),
            references: Vec::new(),
            config_receipts: Vec::new(),
            media_receipts: Vec::new(),
        };
        let (entry_closure, _) = self.walk(edges(entry), isolated);
        let mut configs = BTreeSet::new();
        let mut matched_config = false;
        for record in records
            .iter()
            .copied()
            .filter(|record| record.id != entry.id)
        {
            match record.kind {
                ReceiptKind::Blob => {
                    let retrieval_uri = record
                        .retrieval_uri
                        .as_deref()
                        .filter(|uri| crate::schema::supported_retrieval_uri(uri))
                        .ok_or_else(|| {
                            Refusal::new("missing_receipt", &record.id).dimension("retrieval_uri")
                        })?;
                    let blob = blob(record, self.bytes)
                        .ok_or_else(|| Refusal::new("unreadable_bytes", &record.id))?;
                    output.references.push(ReferenceBlob {
                        retrieval_uri,
                        blob,
                    });
                }
                ReceiptKind::Config | ReceiptKind::Mapping => {
                    let ReceiptOrigin::NamedTree { input_id, .. } = &record.origin else {
                        return Err(
                            Refusal::new("unsupported_kind", &record.id).dimension("origin")
                        );
                    };
                    let owner = self.input(input_id)?;
                    let config = owner.binding.config.as_ref();
                    let receipt = dependency(
                        record,
                        record.entry.or_else(|| config.map(|config| config.entry)),
                    )
                    .ok_or_else(|| {
                        Refusal::new("missing_receipt", &record.id).dimension("entry")
                    })?;
                    if record.kind == ReceiptKind::Config {
                        let key = (receipt.tree, receipt.path, receipt.blob, receipt.entry);
                        matched_config |= entry_closure.contains(&record)
                            && input.binding.config.as_ref().is_some_and(|config| {
                                key == (
                                    config.tree.as_str(),
                                    config.path.as_str(),
                                    config.blob.as_str(),
                                    config.entry,
                                ) && config
                                    .dependencies
                                    .iter()
                                    .all(|edge| record.dependencies.contains(edge))
                            });
                        if !configs.insert(key) {
                            continue;
                        }
                    }
                    output.config_receipts.push(receipt);
                }
                _ => {}
            }
        }
        if input.binding.config.is_some() && !matched_config {
            return Err(Refusal::new("missing_dependency", &entry.id).dimension("config"));
        }

        let proof_ids: BTreeSet<_> = std::iter::once(entry)
            .chain(records.iter().copied())
            .flat_map(|record| &record.dependencies)
            .chain(&claim.receipt_ids)
            .map(String::as_str)
            .filter(|id| isolated.contains(id))
            .collect();
        let mut decisions = Vec::new();
        for id in proof_ids {
            let (proofs, mut reasons) = self.closure([(id, "missing_receipt")]);
            let catalog = self
                .validated_receipt(id)
                .and_then(|record| self.media_catalog(record).ok());
            let root = catalog.as_ref().and_then(|(_, entry)| media_root(entry));
            if reasons.is_empty() {
                let proof = catalog
                    .as_ref()
                    .ok_or_else(|| Refusal::new("missing_receipt", id).dimension("entry"))
                    .and_then(|(proof, entry)| {
                        self.media_proof(input, &output, proofs[0], &proofs[1..], *proof, entry)
                    });
                match proof {
                    Ok(proof) => output.media_receipts.push(proof),
                    Err(reason) => reasons.push(reason),
                }
            }
            for reason in &mut reasons {
                if !reason.evidence.iter().any(|item| item == id) {
                    reason.evidence.push(id.into());
                }
            }
            decisions.push(MediaDecision {
                receipt_id: id,
                root,
                reasons,
            });
        }
        Ok((output, decisions))
    }

    fn media_catalog(
        &self,
        record: &'a AdmissionReceipt,
    ) -> Result<(ProducerMediaEvidence<'a>, serde_json::Value), Refusal> {
        let fail = |code| Refusal::new(code, &record.id);
        let receipt = dependency(record, record.entry.or(Some(1)))
            .ok_or_else(|| fail("missing_receipt").dimension("entry"))?;
        let bytes = &self.bytes[&record.id];
        let catalog: serde_json::Value =
            serde_json::from_slice(bytes).map_err(|_| fail("invalid_json"))?;
        let entry = receipt
            .entry
            .checked_sub(1)
            .and_then(|entry| catalog.get(entry))
            .ok_or_else(|| fail("missing_receipt").dimension("entry"))?;
        Ok((ProducerMediaEvidence { receipt, bytes }, entry.clone()))
    }

    fn media_proof(
        &self,
        input: &AdmissionInput,
        projection: &Projection<'_>,
        record: &'a AdmissionReceipt,
        sources: &[&AdmissionReceipt],
        proof: ProducerMediaEvidence<'a>,
        entry: &serde_json::Value,
    ) -> Result<ProducerMediaEvidence<'a>, Refusal> {
        let fail = |code| Refusal::new(code, &record.id);
        if !matches!(&record.origin, ReceiptOrigin::NamedTree { input_id, .. } if input_id == &input.binding.input_id)
        {
            return Err(fail("slot_mismatch").dimension("origin"));
        }
        let paths = entry["coverage"]["paths"]
            .as_array()
            .filter(|paths| !paths.is_empty())
            .ok_or_else(|| fail("missing_dependency").dimension("media_source"))?;
        if !paths.iter().all(|site| {
            sources.iter().any(|source| {
                matches!(&source.origin, ReceiptOrigin::NamedTree { tree, path, blob, .. }
                if source.kind == ReceiptKind::Source && tree == proof.receipt.tree
                && site["file"] == path.as_str() && site["blob"] == blob.as_str())
            })
        }) {
            return Err(fail("missing_dependency").dimension("media_source"));
        }
        if !crate::schema::valid_media_proof(&projection.input(), proof.receipt, entry) {
            return Err(fail("slot_mismatch").dimension("media_proof"));
        }
        Ok(proof)
    }
}

fn media_root(entry: &serde_json::Value) -> Option<MediaRootEvidence> {
    let operation_id = entry["operation"].as_str()?.to_owned();
    let (direction, status) = match entry["direction"].as_str()? {
        "request" => (MediaDirection::Request, None),
        "response" => (
            MediaDirection::Response,
            Some(entry["status"].as_str()?.to_owned()),
        ),
        _ => return None,
    };
    Some(MediaRootEvidence {
        operation_id,
        direction,
        status,
    })
}

fn edges(record: &AdmissionReceipt) -> impl Iterator<Item = (&str, &'static str)> {
    record
        .dependencies
        .iter()
        .map(|id| (id.as_str(), "missing_dependency"))
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
