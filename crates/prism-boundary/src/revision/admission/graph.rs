use super::{projection::Projection, schema_claim::MediaDecision, selection, slots, *};
use crate::schema::parse_json;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub(in crate::revision) struct Ledger<'a> {
    pub request: &'a AdmissionRequest,
    pub bytes: &'a BTreeMap<String, Vec<u8>>,
    subject: Option<&'a AdmissionInput>,
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
            subject: selection::subject(request),
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
                            selection: selection::selection(
                                request,
                                ledger.subject,
                                input,
                                input.binding.service_side,
                            ),
                            merge_cutoff: selection::merge(request, &input.snapshot)?,
                        })
                    })
                });
        }
        let config_conflicts = ledger.conflicting_configs();
        for receipt in &request.receipts {
            if !ledger.receipts.contains_key(receipt.id.as_str()) {
                let result = slots::unique(
                    request.receipts.iter().filter(|item| item.id == receipt.id),
                    &receipt.id,
                    "missing_receipt",
                )
                .and_then(|record| {
                    match config_conflicts.get(record.id.as_str()) {
                        Some(conflict) => Err(conflict.clone()),
                        None => ledger.validate_receipt(record),
                    }
                });
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

    pub(super) fn validated_receipt(&self, id: &str) -> Option<&'a AdmissionReceipt> {
        Some(self.receipts.get(id)?.as_ref().ok()?.receipt)
    }

    fn validate_receipt(&self, receipt: &'a AdmissionReceipt) -> Result<ReceiptFacts<'a>, Refusal> {
        use ReceiptKind::*;
        let id = &receipt.id;
        let fail = |code| Refusal::new(code, id);
        let supported = match (&receipt.kind, &receipt.origin) {
            (Blob | Config | Media | Inventory | Mapping, ReceiptOrigin::NamedTree { .. }) => true,
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
            (Source, ReceiptOrigin::NamedTree { .. } | ReceiptOrigin::IndependentCutoff { .. }) => {
                true
            }
            (Derived, ReceiptOrigin::Derived { version }) => {
                version.as_deref() == Some("0.1") && !receipt.dependencies.is_empty()
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
                facts.selection.clone()?;
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
                    let entry = receipt
                        .entry
                        .filter(|entry| *entry > 0)
                        .ok_or_else(|| fail("missing_receipt").dimension("config.entry"))?;
                    let config = input
                        .binding
                        .config
                        .as_ref()
                        .ok_or_else(|| fail("slot_mismatch").dimension("config"))?;
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
                    if entry != config.entry {
                        return Err(fail("slot_mismatch").dimension("config.entry"));
                    }
                    if let Some(missing) = config
                        .dependencies
                        .iter()
                        .find(|key| !receipt.dependencies.contains(key))
                    {
                        return Err(Refusal::new("missing_dependency", missing));
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
                let cutoff = selection::cutoff(self.request);
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
        self.walk(roots, &BTreeSet::new())
    }

    pub(super) fn walk(
        &self,
        roots: impl IntoIterator<Item = (&'a str, &'static str)>,
        isolated: &BTreeSet<&str>,
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
                    if visited.contains(id) || isolated.contains(id) {
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

    fn claim(&self, claim: &'a AdmissionClaim) -> Result<ResolvedClaim<'a>, Refusal> {
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
        let purpose = match claim.purpose {
            Some(ClaimPurpose::Unsupported) => {
                return Err(Refusal::new("unsupported_kind", &claim.id))
            }
            Some(purpose) => purpose,
            None if input.binding.role == Role::Producer && input.binding.authority == "schema" => {
                ClaimPurpose::Schema
            }
            None => ClaimPurpose::Metadata,
        };
        if slot.service_side == input.binding.service_side {
            facts.selection.clone()?;
        } else {
            selection::selection(self.request, self.subject, input, slot.service_side)?;
        }
        if purpose == ClaimPurpose::Schema
            && self.request.mode == ReviewMode::HistoricalAsOf
            && selection::cutoff(self.request).is_none()
        {
            return Err(Refusal::new("missing_cutoff", &claim.input_id));
        }
        let mut media = Vec::new();
        let mut projection = None;
        let mut reasons = Vec::new();
        let roots = claim
            .receipt_ids
            .iter()
            .map(|id| (id.as_str(), "missing_receipt"))
            .chain(facts.merge_cutoff.map(|id| (id, "missing_dependency")));
        let isolated = if purpose == ClaimPurpose::Schema {
            self.request
                .receipts
                .iter()
                .filter(|record| record.kind == ReceiptKind::Media)
                .map(|record| record.id.as_str())
                .collect()
        } else {
            BTreeSet::new()
        };
        let (records, closure_refusals) = self.walk(roots.clone(), &isolated);
        reasons.extend(closure_refusals);
        if !isolated.is_empty() && reasons.is_empty() {
            let required: BTreeSet<_> = records.iter().map(|record| record.id.as_str()).collect();
            let (_, attached_refusals) = self.closure(roots);
            reasons.extend(attached_refusals.into_iter().filter(|refusal| {
                refusal.code == "dependency_cycle"
                    && refusal
                        .evidence
                        .iter()
                        .any(|id| required.contains(id.as_str()))
            }));
        }
        for record in &records {
            if claim.receipt_ids.contains(&record.id) {
                if let ReceiptOrigin::NamedTree { input_id, .. } = &record.origin {
                    if input_id != &claim.input_id {
                        reasons.push(Refusal::mismatch(
                            &record.id,
                            "input_id",
                            Some(&claim.input_id),
                            Some(input_id),
                        ));
                    }
                }
            }
        }
        if purpose == ClaimPurpose::Schema && reasons.is_empty() {
            let entry = self.schema_entry(claim, &records);
            match entry.and_then(|entry| {
                self.resolve_schema(input, entry, &records, &isolated, &scope, claim)
            }) {
                Ok((resolved, proofs)) => {
                    projection = Some(resolved);
                    media = proofs;
                }
                Err(reason) => reasons.push(reason),
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
        Ok(ResolvedClaim {
            decision,
            projection,
            media,
        })
    }

    fn schema_entry(
        &self,
        claim: &AdmissionClaim,
        records: &[&'a AdmissionReceipt],
    ) -> Result<&'a AdmissionReceipt, Refusal> {
        let entry = slots::unique(
            records.iter().copied().filter(|record| {
                record.kind == ReceiptKind::Blob && claim.receipt_ids.contains(&record.id)
            }),
            &claim.id,
            "missing_receipt",
        )?;
        if !matches!(entry.origin, ReceiptOrigin::NamedTree { .. }) {
            return Err(Refusal::new("slot_mismatch", &entry.id).dimension("origin"));
        }
        if !self.receipts[entry.id.as_str()]
            .as_ref()
            .is_ok_and(|facts| facts.supported_schema_entry)
        {
            return Err(Refusal::new("unsupported_schema_dialect", &entry.id));
        }
        Ok(entry)
    }

    pub fn report(&self) -> AdmissionReport {
        self.resolve().0
    }

    pub(in crate::revision) fn resolve(
        &self,
    ) -> (AdmissionReport, BTreeMap<&'a str, Projection<'a>>) {
        let mut report = AdmissionReport {
            mode: Some(self.request.mode),
            ..Default::default()
        };
        let mut projections = BTreeMap::new();
        let mut dependent = Vec::new();
        for claim in &self.request.claims {
            if report.claims.contains_key(&claim.id) {
                continue;
            }
            let resolved = slots::unique(
                self.request
                    .claims
                    .iter()
                    .filter(|item| item.id == claim.id),
                &claim.id,
                "missing_receipt",
            )
            .and_then(|claim| self.claim(claim));
            match resolved {
                Ok(resolved) => {
                    if let Some(projection) = resolved.projection {
                        projections.insert(claim.id.as_str(), projection);
                    }
                    report.insert(&claim.id, resolved.decision);
                    for media in resolved.media {
                        let id =
                            format!("media:{}:{}:{}", claim.id.len(), claim.id, media.receipt_id);
                        let mut decision =
                            ClaimDecision::from_reasons(Some(&claim.input_id), media.reasons);
                        decision.evidence.push(media.receipt_id.into());
                        decision.media_root = media.root;
                        dependent.push((id, decision));
                    }
                }
                Err(reason) => {
                    report.insert(&claim.id, ClaimDecision::from_reasons(None, vec![reason]))
                }
            }
        }
        for (mut id, decision) in dependent {
            while report.claims.contains_key(&id) {
                id.push(':');
            }
            report.insert(&id, decision);
        }
        (report, projections)
    }
}

struct ResolvedClaim<'a> {
    decision: ClaimDecision,
    projection: Option<Projection<'a>>,
    media: Vec<MediaDecision<'a>>,
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
