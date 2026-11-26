use super::*;
use crate::revision::{
    exact_tree_at_cutoff, historical_head, instant, selection_conflicts, selection_required,
    Availability, ClaimDecision, ReceiptOrigin, Refusal, Role, ServiceSide, SnapshotInput,
};
use chrono::{DateTime, FixedOffset};
use std::collections::BTreeSet;

type Inputs<'a> = BTreeMap<&'a str, &'a DocumentInput>;

pub(super) fn input_refusals(doc: &ImpactDocument, input: &DocumentInput) -> Vec<Refusal> {
    use crate::model::SchemaOrigin;
    let id = &input.input_id;
    let Some((tree, rev)) = input.snapshot.identity() else {
        return vec![Refusal::new("unsupported_kind", id)];
    };
    if matches!(&input.snapshot, SnapshotInput::Commit { committed_at: Some(date), .. } if instant(date).is_none())
    {
        return vec![Refusal::new("slot_mismatch", id).dimension("committed_at")];
    }
    if input.api != doc.api {
        return vec![Refusal::new("slot_mismatch", id).dimension("api")];
    }
    if !matches!(
        (input.role, input.operation_authority.as_str()),
        (
            Role::Producer,
            "schema" | "route_table" | "annotation" | "registration"
        ) | (Role::Consumer, "calls")
    ) {
        return vec![Refusal::new("slot_mismatch", id).dimension("role_authority")];
    }
    let origin = match &input.origin {
        ReceiptOrigin::NamedTree {
            input_id,
            repo,
            tree: origin,
            path,
            blob,
            availability,
        } => {
            if input_id != id
                || repo != &input.repo
                || origin != tree
                || path.is_empty()
                || blob.is_empty()
            {
                Some(Refusal::new("slot_mismatch", id).dimension("origin"))
            } else {
                match availability {
                    Availability::Available => None,
                    Availability::Missing => Some(Refusal::new("missing_receipt", id)),
                    Availability::Unreadable => Some(Refusal::new("unreadable_bytes", id)),
                    Availability::Unsupported => Some(Refusal::new("unsupported_kind", id)),
                }
            }
        }
        ReceiptOrigin::IndependentCutoff {
            evidence,
            available_at,
            covered_tree,
            ..
        } => (evidence.is_empty()
            || covered_tree
                .as_deref()
                .is_some_and(|covered| covered != tree)
            || !instant(available_at)
                .zip(instant(&doc.cutoff_at))
                .is_some_and(|(at, cutoff)| at <= cutoff))
        .then(|| Refusal::new("missing_cutoff", id)),
        _ => Some(Refusal::new("unsupported_kind", id)),
    };
    if let Some(refusal) = origin {
        return vec![refusal];
    }
    if let Some(schema) = &input.schema {
        let (repo, source_tree) = match &schema.source {
            SchemaOrigin::File {
                rev: source_rev,
                committed_at,
                subject,
                ..
            } if source_rev.as_deref() != rev
                || match &input.snapshot {
                    crate::SnapshotInput::ComputedTree { .. } => {
                        committed_at.is_some() || subject.is_some()
                    }
                    crate::SnapshotInput::Commit {
                        committed_at: date,
                        subject: title,
                        ..
                    } => {
                        committed_at
                            .as_ref()
                            .is_some_and(|at| instant(at) != date.as_deref().and_then(instant))
                            || subject
                                .as_ref()
                                .is_some_and(|text| Some(text) != title.as_ref())
                    }
                    _ => true,
                } =>
            {
                return vec![Refusal::new("slot_mismatch", id).dimension("schema.source.metadata")];
            }
            SchemaOrigin::File { repo, tree, .. }
            | SchemaOrigin::RouteTable { repo, tree, .. }
            | SchemaOrigin::MissingExpectedSchema { repo, tree, .. } => (repo, tree),
        };
        if repo != &input.repo || source_tree != tree {
            return vec![Refusal::new("slot_mismatch", id).dimension("schema.source")];
        }
    }
    Vec::new()
}

pub(super) fn validate_slots(
    doc: &ImpactDocument,
    inputs: &Inputs<'_>,
    report: &mut AdmissionReport,
) {
    for (name, role, slots, subject) in [
        (
            "producer",
            Role::Producer,
            &doc.change.producer,
            matches!(doc.change.subject_role.as_str(), "service" | "both"),
        ),
        (
            "consumer",
            Role::Consumer,
            &doc.change.consumer,
            matches!(doc.change.subject_role.as_str(), "client" | "both"),
        ),
    ] {
        let mut reasons = Vec::new();
        if let Some(slots) = slots {
            if !subject && slots.before_input_id != slots.after_input_id {
                reasons.push(Refusal::new("slot_mismatch", name).dimension("held_input"));
            }
            for (side, id) in [
                ("before", &slots.before_input_id),
                ("after", &slots.after_input_id),
            ] {
                if let Some(id) = id {
                    match inputs.get(id.as_str()) {
                        Some(input) if input.role == role => {}
                        Some(_) => {
                            reasons.push(Refusal::new("slot_mismatch", id).dimension("role"))
                        }
                        None => reasons.push(Refusal::new("missing_slot", id)),
                    }
                } else if subject
                    && (side != "before" || slots.reason.as_deref() != Some("no_first_parent"))
                {
                    reasons.push(Refusal::new("missing_slot", name).dimension(side));
                }
            }
        } else if subject {
            reasons.push(Refusal::new("missing_slot", name));
        }
        report.insert(
            &format!("slots:{name}"),
            ClaimDecision::from_reasons(None, reasons),
        );
    }
    if let Some(reference) = &doc.reference {
        if !inputs.contains_key(reference.as_str()) {
            report.insert(
                "reference",
                ClaimDecision::from_reasons(None, vec![Refusal::new("missing_slot", reference)]),
            );
        }
    }
}

pub(super) fn slot_cutoff(doc: &ImpactDocument, inputs: &Inputs<'_>) -> Vec<Refusal> {
    let missing = || vec![Refusal::new("missing_cutoff", "report-cutoff")];
    let Some((report_time, cutoff)) = instant(&doc.report_time).zip(instant(&doc.cutoff_at)) else {
        return missing();
    };
    let Some(mode) = doc.mode else {
        return missing();
    };
    if (mode == ReviewMode::Live && cutoff != report_time) || cutoff > report_time {
        return missing();
    }
    let selection = doc.change.selection.as_ref();
    let roles = [
        (
            &doc.change.producer,
            matches!(doc.change.subject_role.as_str(), "service" | "both"),
        ),
        (
            &doc.change.consumer,
            matches!(doc.change.subject_role.as_str(), "client" | "both"),
        ),
    ];
    let subject_id = selection.and_then(|selection| selection.subject_head_input_id.as_deref());
    let subject_after_ids: Vec<_> = roles
        .iter()
        .filter(|(_, subject)| *subject)
        .filter_map(|(slots, _)| slots.as_ref()?.after_input_id.as_deref())
        .collect();
    let subject = match subject_id {
        Some(id) if subject_after_ids.contains(&id) => inputs.get(id),
        Some(id) if mode == ReviewMode::HistoricalAsOf => inputs.get(id).filter(|head| {
            let Some(selection) = selection else { return false };
            let Some(head_rev) = historical_head(&head.snapshot, cutoff) else { return false };
            selection.proposed_head.as_deref() == Some(head_rev)
                && subject_after_ids.iter().filter_map(|id| inputs.get(id)).any(|after| {
                    matches!(&after.snapshot, SnapshotInput::ComputedTree { tree, merge_receipt, computed_at }
                        if after.repo == head.repo
                            && after.role == head.role
                            && selection.merge_receipt.as_ref() == Some(merge_receipt)
                            && instant(computed_at).is_some()
                            && exact_tree_at_cutoff(&after.origin, tree, cutoff))
                })
        }),
        None if mode == ReviewMode::HistoricalAsOf => return missing(),
        Some(id) if !inputs.contains_key(id) => {
            return vec![Refusal::new("missing_slot", id).dimension("subject_head_input_id")];
        }
        Some(id) => {
            return vec![Refusal::new("slot_mismatch", id).dimension("subject_head_input_id")];
        }
        None => subject_after_ids.first().and_then(|id| inputs.get(id)),
    };
    if mode == ReviewMode::HistoricalAsOf
        && subject
            .and_then(|head| historical_head(&head.snapshot, cutoff))
            .is_none()
    {
        return missing();
    }
    if mode == ReviewMode::VersionComparison
        && selection
            .and_then(|selection| selection.selection_time.as_deref())
            .and_then(instant)
            != Some(cutoff)
    {
        return missing();
    }
    let anchor = subject.map(|head| (head.role, head.repo.as_str()));
    let mut required = BTreeSet::new();
    for (slots, subject) in roles {
        if let Some(slots) = slots {
            for (id, side) in [
                (&slots.before_input_id, ServiceSide::Before),
                (&slots.after_input_id, ServiceSide::After),
            ] {
                if let Some(input) = id.as_deref().and_then(|id| inputs.get(id)) {
                    let side = if subject { side } else { ServiceSide::Held };
                    if selection_required(
                        mode,
                        Some(side),
                        input.role,
                        &input.repo,
                        anchor,
                        &input.snapshot,
                    ) {
                        required.insert(input.input_id.as_str());
                    }
                }
            }
        }
    }
    let mut reasons = Vec::new();
    if let Some(selection) = selection {
        reasons.extend(check_recorded_selection(selection, inputs));
        let mut observations = Vec::new();
        let receipts = selection
            .receipts
            .iter()
            .map(|(id, receipt)| (id, Some(receipt)))
            .chain(
                selection
                    .input_id
                    .iter()
                    .filter(|id| !selection.receipts.contains_key(*id))
                    .map(|id| (id, selection.receipt.as_ref())),
            );
        for (id, receipt) in receipts {
            let Some(input) = inputs.get(id.as_str()) else {
                reasons.push(Refusal::new("missing_slot", id).dimension("selection"));
                continue;
            };
            let singular = selection.input_id.as_ref() == Some(id);
            if singular && selection.receipts.contains_key(id) {
                reasons.push(Refusal::new("receipt_conflict", id).dimension("selection"));
                continue;
            }
            let proof = check_selection_receipt(selection, input, singular, receipt, mode, cutoff);
            match proof {
                Ok(proof) => observations.push((id.as_str(), input.repo.as_str(), proof)),
                Err(reason) => reasons.push(reason),
            }
        }
        reasons.extend(selection_conflicts(observations));
    }
    for id in required {
        if !selection.is_some_and(|selection| {
            selection.receipts.contains_key(id) || selection.input_id.as_deref() == Some(id)
        }) {
            reasons.push(Refusal::new("missing_cutoff", id).dimension("selection"));
        }
    }
    reasons
}

fn check_recorded_selection(selection: &ChangeSelection, inputs: &Inputs<'_>) -> Vec<Refusal> {
    let mut reasons = Vec::new();
    if let Some(merge_id) = &selection.merge_receipt {
        if !inputs.values().any(|input| {
            matches!(&input.snapshot,
                SnapshotInput::ComputedTree { merge_receipt, .. } if merge_receipt == merge_id)
        }) {
            reasons.push(Refusal::new("missing_dependency", merge_id).dimension("merge_receipt"));
        }
    }
    let has_singular_details = selection.branch.is_some()
        || selection.observed_tip.is_some()
        || selection.observed_at.is_some()
        || selection.requested_rev.is_some()
        || selection.receipt.is_some()
        || selection.availability_status.is_some();
    if selection.input_id.is_none() && has_singular_details {
        reasons.push(Refusal::new("missing_slot", "change.selection").dimension("selection"));
    }
    reasons
}

fn check_selection_receipt<'a>(
    selection: &ChangeSelection,
    input: &DocumentInput,
    singular: bool,
    receipt: Option<&'a crate::revision::SelectionReceipt>,
    mode: ReviewMode,
    cutoff: DateTime<FixedOffset>,
) -> Result<(&'a str, Option<&'a str>), Refusal> {
    let id = &input.input_id;
    let missing = || Refusal::new("missing_cutoff", id).dimension("selection");
    let rev = input.snapshot.identity().and_then(|(_, rev)| rev);
    let expected_branch = input.branch.as_deref().or_else(|| {
        if singular {
            selection.branch.as_deref()
        } else {
            None
        }
    });
    let proof = receipt
        .ok_or_else(missing)?
        .check(mode, cutoff, id, rev, expected_branch)
        .map_err(|reason| {
            if reason.dimension.as_deref() == Some("branch") {
                reason
            } else {
                missing()
            }
        })?;
    if singular
        && (selection.availability_status.as_deref() == Some("unchecked")
            || selection
                .branch
                .as_deref()
                .is_some_and(|branch| proof.1 != Some(branch))
            || selection
                .requested_rev
                .as_deref()
                .is_some_and(|requested| rev != Some(requested))
            || selection
                .observed_tip
                .as_deref()
                .is_some_and(|tip| rev != Some(tip))
            || selection
                .observed_at
                .as_deref()
                .is_some_and(|at| instant(at) != Some(cutoff)))
    {
        return Err(missing());
    }
    Ok(proof)
}

pub(super) fn validate_status(
    doc: &ImpactDocument,
    inputs: &Inputs<'_>,
    report: &mut AdmissionReport,
) {
    for (index, status) in doc.binding_status.iter().enumerate() {
        let key = format!("binding-status:{index}");
        let mut reasons = Vec::new();
        let selected = doc
            .change
            .producer
            .as_ref()
            .and_then(|slots| match status.side {
                ServiceSide::Before => slots.before_input_id.as_deref(),
                ServiceSide::After => slots.after_input_id.as_deref(),
                ServiceSide::Held if slots.before_input_id == slots.after_input_id => {
                    slots.after_input_id.as_deref()
                }
                ServiceSide::Held => None,
            });
        if selected != Some(&status.selected_input_id)
            || report
                .claims
                .get(&format!("input:{}", status.selected_input_id))
                .is_none_or(|decision| decision.status != "admitted")
            || inputs
                .get(status.selected_input_id.as_str())
                .is_none_or(|input| input.role != Role::Producer)
        {
            reasons.push(Refusal::new("missing_slot", &status.selected_input_id));
        }
        let mut scans = doc
            .saved_scans
            .iter()
            .filter(|scan| Some(&scan.scan_id) == status.scan_id.as_ref());
        let scan = scans.next().filter(|_| scans.next().is_none());
        if let Some(scan) = scan {
            if inputs
                .get(scan.client_input_id.as_str())
                .is_none_or(|input| {
                    input.role != Role::Consumer
                        || input
                            .snapshot
                            .identity()
                            .is_none_or(|(tree, _)| tree != scan.client_tree)
                })
            {
                reasons.push(Refusal::new("slot_mismatch", &scan.scan_id).dimension("scan_client"));
            }
        }
        let located = status.saved_location.as_ref().and_then(|id| {
            let mut locations = scan?
                .matches
                .iter()
                .filter(|location| &location.location_id == id);
            locations.next().filter(|location| {
                locations.next().is_none() && location.element_id == status.element_id
            })
        });
        if status.status != "not_checked" && scan.is_none() {
            reasons.push(Refusal::new(
                "missing_dependency",
                status.scan_id.as_deref().unwrap_or(&key),
            ));
        }
        if matches!(status.status.as_str(), "binding_current" | "binding_stale") {
            let shapes = status
                .saved_shape
                .as_ref()
                .zip(status.selected_shape.as_ref());
            if status.selected_presence != "present"
                || status.reason.is_some()
                || located.is_none_or(|location| location.grade != "bound")
                || shapes.is_none_or(|(before, after)| {
                    let current = status.status == "binding_current";
                    (before == after) != current
                        || current
                            && located.is_none_or(|location| {
                                crate::revision::normalize_saved_shape(
                                    &status.element_id,
                                    &location.signature,
                                )
                                .as_ref()
                                    != Some(before)
                            })
                })
            {
                reasons.push(Refusal::new("slot_mismatch", &key).dimension("binding_status"));
            }
        } else if status.status == "not_checked"
            && status.reason.as_deref().is_none_or(str::is_empty)
        {
            reasons.push(Refusal::new("missing_dependency", &key).dimension("reason"));
        } else if matches!(
            status.status.as_str(),
            "no_saved_match" | "element_absent_from_scan_schema"
        ) && (status.saved_location.is_some() || status.reason.is_some())
        {
            reasons.push(Refusal::new("slot_mismatch", &key).dimension("binding_status"));
        }
        report.insert(&key, ClaimDecision::from_reasons(None, reasons));
    }
}
