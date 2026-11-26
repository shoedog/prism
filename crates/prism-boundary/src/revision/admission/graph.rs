use super::{selection, slots, *};
use crate::schema::parse_json;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub(in crate::revision) struct Ledger<'a> {
    pub request: &'a AdmissionRequest,
    pub bytes: &'a BTreeMap<String, Vec<u8>>,
    inputs: BTreeMap<&'a str, Result<InputFacts<'a>, Refusal>>,
    receipts: BTreeMap<&'a str, Result<ReceiptFacts<'a>, Refusal>>,
}

struct InputFacts<'a> {
    input: &'a AdmissionInput,
    selection: Result<(), Refusal>,
    merge_cutoff: Option<&'a str>,
}

struct ReceiptFacts<'a> {
    receipt: &'a AdmissionReceipt,
    supported_schema_entry: bool,
}

impl<'a> Ledger<'a> {
    pub fn new(request: &'a AdmissionRequest, bytes: &'a BTreeMap<String, Vec<u8>>) -> Self {
        let mut ledger = Self {
            request,
            bytes,
            inputs: BTreeMap::new(),
            receipts: BTreeMap::new(),
        };
        for input in &request.inputs {
            ledger
                .inputs
                .entry(&input.binding.input_id)
                .or_insert_with(|| {
                    slots::unique(
                        request
                            .inputs
                            .iter()
                            .filter(|item| item.binding.input_id == input.binding.input_id),
                        &input.binding.input_id,
                        "missing_slot",
                    )
                    .and_then(|input| {
                        slots::validate_input(request, input)?;
                        Ok(InputFacts {
                            input,
                            selection: selection::selection(request, input),
                            merge_cutoff: selection::merge(request, &input.snapshot)?,
                        })
                    })
                });
        }
        for receipt in &request.receipts {
            if !ledger.receipts.contains_key(receipt.id.as_str()) {
                let result = slots::unique(
                    request.receipts.iter().filter(|item| item.id == receipt.id),
                    &receipt.id,
                    "missing_receipt",
                )
                .and_then(|record| ledger.validate_receipt(record));
                ledger.receipts.insert(&receipt.id, result);
            }
        }
        ledger
    }

    pub fn input(&self, id: &str) -> Result<&'a AdmissionInput, Refusal> {
        self.input_facts(id).map(|facts| facts.input)
    }

    fn input_facts(&self, id: &str) -> Result<&InputFacts<'a>, Refusal> {
        self.inputs
            .get(id)
            .ok_or_else(|| Refusal::new("missing_slot", id))?
            .as_ref()
            .map_err(Clone::clone)
    }

    fn validate_receipt(&self, receipt: &'a AdmissionReceipt) -> Result<ReceiptFacts<'a>, Refusal> {
        use ReceiptKind::*;
        let id = &receipt.id;
        let fail = |code| Refusal::new(code, id);
        let supported = match (&receipt.kind, &receipt.origin) {
            (Blob | Media | Inventory, ReceiptOrigin::NamedTree { .. }) => true,
            (
                Blob,
                ReceiptOrigin::IndependentCutoff {
                    repo: Some(repo),
                    covered_tree: Some(tree),
                    path: Some(path),
                    blob: Some(blob),
                    ..
                },
            ) => {
                [repo, tree, path, blob]
                    .iter()
                    .all(|value| !value.is_empty())
                    && receipt
                        .retrieval_uri
                        .as_deref()
                        .is_some_and(crate::schema::supported_retrieval_uri)
            }
            (
                Source | Config | Mapping,
                ReceiptOrigin::NamedTree { .. } | ReceiptOrigin::IndependentCutoff { .. },
            ) => true,
            (Derived, ReceiptOrigin::Derived { version }) => {
                version.as_deref().is_some_and(|v| !v.is_empty())
                    && !receipt.dependencies.is_empty()
            }
            _ => false,
        };
        if !supported || id.is_empty() {
            return Err(fail("unsupported_kind"));
        }
        match &receipt.origin {
            ReceiptOrigin::NamedTree {
                input_id,
                repo,
                tree,
                path,
                blob,
                availability,
            } => {
                let facts = self.input_facts(input_id)?;
                let input = facts.input;
                if input.selection.is_some() {
                    facts.selection.clone()?;
                }
                for (dimension, expected, received) in [
                    ("repo", input.binding.repo.as_str(), repo.as_str()),
                    ("tree", input.binding.tree.as_str(), tree.as_str()),
                ] {
                    if expected != received {
                        return Err(Refusal::mismatch(
                            id,
                            dimension,
                            Some(expected),
                            Some(received),
                        ));
                    }
                }
                if path.is_empty() || blob.is_empty() {
                    return Err(fail("missing_receipt"));
                }
                if receipt.kind == Config {
                    if let Some(config) = &input.binding.config {
                        for (dimension, expected, actual) in [
                            ("config.tree", &config.tree, tree),
                            ("config.path", &config.path, path),
                            ("config.blob", &config.blob, blob),
                        ] {
                            if expected != actual {
                                return Err(Refusal::mismatch(
                                    id,
                                    dimension,
                                    Some(expected),
                                    Some(actual),
                                ));
                            }
                        }
                        if let Some(missing) = config
                            .dependencies
                            .iter()
                            .find(|key| !receipt.dependencies.contains(key))
                        {
                            return Err(Refusal::new("missing_dependency", missing));
                        }
                    }
                }
                match availability {
                    Availability::Available => {}
                    Availability::Missing => return Err(fail("missing_receipt")),
                    Availability::Unreadable => return Err(fail("unreadable_bytes")),
                    Availability::Unsupported => return Err(fail("unsupported_kind")),
                }
            }
            ReceiptOrigin::IndependentCutoff {
                evidence,
                available_at,
                ..
            } => {
                // Selection failure and record availability are independent facts.
                let cutoff = selection::instant(&self.request.cutoff_at);
                if evidence.is_empty()
                    || !selection::instant(available_at)
                        .zip(cutoff)
                        .is_some_and(|(at, cutoff)| at <= cutoff)
                {
                    return Err(fail("missing_cutoff"));
                }
            }
            ReceiptOrigin::Derived { .. } => {}
            ReceiptOrigin::Unsupported => return Err(fail("unsupported_kind")),
        }
        match receipt.format.as_deref() {
            Some("json") => {}
            Some("yaml") => return Err(fail("yaml_format")),
            Some(_) => return Err(fail("unsupported_format")),
            None if matches!(receipt.kind, Blob | Media) => return Err(fail("unsupported_format")),
            None => {}
        }
        let bytes = self.bytes.get(id);
        if matches!(receipt.kind, Blob | Media | Inventory) && bytes.is_none() {
            return Err(fail("unreadable_bytes"));
        }
        if let Some(digest) = &receipt.digest {
            if digest.algorithm != "sha256" || digest.framing != "raw_bytes" {
                return Err(fail("unsupported_kind"));
            }
            let bytes = bytes.ok_or_else(|| fail("unreadable_bytes"))?;
            if digest.value != format!("{:x}", Sha256::digest(bytes)) {
                return Err(fail("digest_mismatch"));
            }
        } else if matches!(receipt.kind, Blob | Media) {
            return Err(fail("missing_receipt").dimension("digest"));
        }
        let mut supported_schema_entry = false;
        if let Some(bytes) = bytes.filter(|_| receipt.format.as_deref() == Some("json")) {
            let parsed = parse_json(bytes).map_err(|error| {
                let mut refused = fail("invalid_json");
                refused.evidence.push(error.to_string());
                refused
            })?;
            if !parsed.duplicates.is_empty() {
                return Err(fail("duplicate_json_member"));
            }
            supported_schema_entry = schema_dialect(&parsed.value);
        }
        Ok(ReceiptFacts {
            receipt,
            supported_schema_entry,
        })
    }

    pub fn closure(
        &self,
        roots: impl IntoIterator<Item = (&'a str, &'static str)>,
    ) -> (Vec<&'a AdmissionReceipt>, Vec<Refusal>) {
        enum Step<'a> {
            Enter(&'a str, &'static str),
            Leave(&'a str),
        }
        let mut active = BTreeSet::new();
        let mut visited = BTreeSet::new();
        let mut path: Vec<&str> = Vec::new();
        let mut records = Vec::new();
        let mut refusals = Vec::new();
        let mut steps: Vec<_> = roots
            .into_iter()
            .map(|(id, missing)| Step::Enter(id, missing))
            .collect();
        steps.reverse();
        while let Some(step) = steps.pop() {
            match step {
                Step::Leave(id) => {
                    active.remove(id);
                    path.pop();
                    visited.insert(id);
                }
                Step::Enter(id, missing) => {
                    if visited.contains(id) {
                        continue;
                    }
                    if active.contains(id) {
                        let mut refusal = Refusal::new("dependency_cycle", id);
                        refusal.evidence = path
                            .iter()
                            .skip_while(|key| **key != id)
                            .map(|key| (*key).to_owned())
                            .collect();
                        refusals.push(refusal);
                        continue;
                    }
                    match self.receipts.get(id) {
                        None => refusals.push(Refusal::new(missing, id)),
                        Some(Err(refusal)) => refusals.push(refusal.clone()),
                        Some(Ok(facts)) => {
                            let record = facts.receipt;
                            active.insert(id);
                            path.push(id);
                            records.push(record);
                            steps.push(Step::Leave(id));
                            if let ReceiptOrigin::NamedTree { input_id, .. } = &record.origin {
                                if let Ok(facts) = self.input_facts(input_id) {
                                    if let Some(merge) = facts.merge_cutoff {
                                        steps.push(Step::Enter(merge, "missing_dependency"));
                                    }
                                }
                            }
                            steps.extend(
                                record
                                    .dependencies
                                    .iter()
                                    .rev()
                                    .map(|dep| Step::Enter(dep, "missing_dependency")),
                            );
                        }
                    }
                }
            }
        }
        (records, refusals)
    }

    fn claim(&self, claim: &'a AdmissionClaim) -> Result<ClaimDecision, Refusal> {
        let slot_id = claim
            .slot
            .as_deref()
            .ok_or_else(|| Refusal::new("missing_slot", &claim.input_id))?;
        let slot = slots::unique(
            self.request
                .slots
                .iter()
                .filter(|slot| slot.slot == slot_id),
            slot_id,
            "missing_slot",
        )?;
        let facts = self.input_facts(&claim.input_id)?;
        let input = facts.input;
        slots::compare_slot(slot, &input.binding)?;
        let (scope, remainder) =
            slots::project_scope(slot, &input.binding, claim.projection.as_ref())?;
        if claim.requires_selection || input.selection.is_some() {
            facts.selection.clone()?;
        }
        let roots = claim
            .receipt_ids
            .iter()
            .map(|id| (id.as_str(), "missing_receipt"))
            .chain(facts.merge_cutoff.map(|id| (id, "missing_dependency")));
        let (records, mut reasons) = self.closure(roots);
        for id in &claim.receipt_ids {
            if let Some(record) = records.iter().find(|record| record.id == *id) {
                if let ReceiptOrigin::NamedTree { input_id, .. } = &record.origin {
                    if *input_id != claim.input_id {
                        reasons.push(Refusal::mismatch(
                            id,
                            "input_id",
                            Some(&claim.input_id),
                            Some(input_id),
                        ));
                    }
                }
            }
        }
        if input.binding.role == Role::Producer && input.binding.authority == "schema" {
            let blobs: Vec<_> = claim
                .receipt_ids
                .iter()
                .filter_map(|id| {
                    self.receipts
                        .get(id.as_str())
                        .and_then(|entry| entry.as_ref().ok())
                        .filter(|facts| facts.receipt.kind == ReceiptKind::Blob)
                })
                .collect();
            if !blobs.is_empty() {
                match slots::unique(
                    blobs.iter().map(|fact| fact.receipt),
                    &claim.id,
                    "missing_receipt",
                ) {
                    Ok(entry)
                        if !self.receipts[entry.id.as_str()]
                            .as_ref()
                            .is_ok_and(|facts| facts.supported_schema_entry) =>
                    {
                        reasons.push(Refusal::new("unsupported_schema_dialect", &entry.id))
                    }
                    Err(refusal) => reasons.push(refusal),
                    _ => {}
                }
            }
        }
        for id in &claim.receipt_ids {
            if records.iter().any(|record| {
                record.id == *id
                    && record.kind == ReceiptKind::Blob
                    && matches!(record.origin, ReceiptOrigin::IndependentCutoff { .. })
            }) {
                reasons.push(Refusal::new("slot_mismatch", id).dimension("origin"));
            }
        }
        let mut decision = ClaimDecision::from_reasons(Some(&claim.input_id), reasons);
        decision.evidence = records
            .into_iter()
            .map(|record| record.id.clone())
            .collect();
        decision.requested_scope = claim
            .projection
            .as_ref()
            .map(|value| value.requested.clone());
        decision.selection = input.selection.clone();
        decision.effective_scope = (decision.status == "admitted").then_some(scope);
        decision.uncovered_scope = remainder;
        decision.dirty = input.dirty;
        Ok(decision)
    }

    pub fn report(&self) -> AdmissionReport {
        let mut report = AdmissionReport {
            mode: Some(self.request.mode),
            ..Default::default()
        };
        for claim in &self.request.claims {
            if report.claims.contains_key(&claim.id) {
                continue;
            }
            let decision = slots::unique(
                self.request
                    .claims
                    .iter()
                    .filter(|item| item.id == claim.id),
                &claim.id,
                "missing_receipt",
            )
            .and_then(|claim| self.claim(claim))
            .unwrap_or_else(|reason| ClaimDecision::from_reasons(None, vec![reason]));
            report.insert(&claim.id, decision);
        }
        report
    }
}

fn schema_dialect(json: &serde_json::Value) -> bool {
    match (json.get("swagger"), json.get("openapi")) {
        (Some(version), None) => version == "2.0",
        (None, Some(version)) => version.as_str().is_some_and(|version| {
            version
                .strip_prefix("3.0.")
                .or_else(|| version.strip_prefix("3.1."))
                .is_some_and(|patch| {
                    !patch.is_empty() && patch.bytes().all(|digit| digit.is_ascii_digit())
                })
        }),
        _ => false,
    }
}
