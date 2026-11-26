use super::*;
use chrono::{DateTime, FixedOffset};

pub(crate) fn instant(value: &str) -> Option<DateTime<FixedOffset>> {
    DateTime::parse_from_rfc3339(value).ok()
}

pub(super) fn cutoff(request: &AdmissionRequest) -> Option<DateTime<FixedOffset>> {
    let cutoff = instant(&request.cutoff_at)?;
    match request.mode {
        ReviewMode::Live => (instant(&request.report_time)? == cutoff).then_some(cutoff),
        ReviewMode::HistoricalAsOf => {
            let id = request.subject_head_input_id.as_deref()?;
            let subject = slots::unique(
                request
                    .inputs
                    .iter()
                    .filter(|input| input.binding.input_id == id),
                id,
                "missing_slot",
            )
            .ok()?;
            slots::validate_input(request, subject).ok()?;
            let SnapshotInput::Commit {
                committed_at: Some(date),
                ..
            } = &subject.snapshot
            else {
                return None;
            };
            (instant(date)? == cutoff
                && request
                    .subject_head_committed_at
                    .as_deref()
                    .is_none_or(|legacy| instant(legacy) == Some(cutoff)))
            .then_some(cutoff)
        }
        ReviewMode::VersionComparison => {
            (instant(request.selection_time.as_deref()?)? == cutoff).then_some(cutoff)
        }
    }
}

pub(super) fn selection(request: &AdmissionRequest, input: &AdmissionInput) -> Result<(), Refusal> {
    let id = &input.binding.input_id;
    let missing = || Refusal::new("missing_cutoff", id);
    let basis = input.binding.selection_basis;
    let required = match basis {
        Some(SelectionBasis::Snapshot) => false,
        Some(SelectionBasis::Counterpart) => {
            matches!(request.mode, ReviewMode::Live | ReviewMode::HistoricalAsOf)
        }
        Some(SelectionBasis::SelectedVersion) => request.mode == ReviewMode::VersionComparison,
        None => input.binding.branch.is_some() || request.mode == ReviewMode::VersionComparison,
        Some(SelectionBasis::Unsupported) => {
            return Err(Refusal::new("unsupported_kind", id).dimension("selection_basis"))
        }
    };
    if matches!(
        basis,
        Some(SelectionBasis::Counterpart | SelectionBasis::SelectedVersion)
    ) && !required
    {
        return Err(Refusal::new("slot_mismatch", id).dimension("selection_basis"));
    }
    if !required && input.selection.is_none() {
        return Ok(());
    }
    let recorded_cutoff = cutoff(request).ok_or_else(missing)?;
    let receipt = input.selection.as_ref().ok_or_else(missing)?;
    let (selected_rev, selected_branch) =
        receipt
            .proof(request.mode, recorded_cutoff)
            .ok_or_else(|| {
                if *receipt == SelectionReceipt::Unsupported {
                    Refusal::new("unsupported_kind", id)
                } else {
                    missing()
                }
            })?;
    if selected_branch.is_some() && selected_branch != input.binding.branch.as_deref() {
        return Err(missing());
    }
    if Some(selected_rev) != input.binding.rev.as_deref() {
        return Err(Refusal::mismatch(
            id,
            "rev",
            input.binding.rev.as_deref(),
            Some(selected_rev),
        ));
    }
    Ok(())
}

impl SelectionReceipt {
    pub(crate) fn proof(
        &self,
        mode: ReviewMode,
        cutoff: DateTime<FixedOffset>,
    ) -> Option<(&str, Option<&str>)> {
        let selected = match (mode, self) {
            (
                ReviewMode::Live,
                Self::LiveRef {
                    branch,
                    rev,
                    observed_at,
                },
            ) if instant(observed_at) == Some(cutoff) => (rev, Some(branch)),
            (
                ReviewMode::HistoricalAsOf,
                Self::HistoricalPin {
                    branch,
                    rev,
                    available_at,
                    evidence,
                },
            ) if instant(available_at) == Some(cutoff) && !evidence.is_empty() => {
                (rev, Some(branch))
            }
            (
                ReviewMode::HistoricalAsOf,
                Self::HistoricalSnapshot {
                    branch,
                    rev,
                    captured_at,
                    evidence,
                },
            ) if instant(captured_at) == Some(cutoff) && !evidence.is_empty() => {
                (rev, Some(branch))
            }
            (
                ReviewMode::HistoricalAsOf,
                Self::HistoricalRefHistory {
                    branch,
                    rev,
                    valid_from,
                    valid_until,
                    evidence,
                },
            ) if instant(valid_from)
                .zip(instant(valid_until))
                .is_some_and(|(start, end)| start <= cutoff && cutoff < end)
                && !evidence.is_empty() =>
            {
                (rev, Some(branch))
            }
            (
                ReviewMode::VersionComparison,
                Self::ExplicitPin {
                    rev,
                    selection_time: Some(time),
                    selected_by: Some(selector),
                    selection_reason,
                },
            ) if instant(time) == Some(cutoff)
                && !selector.is_empty()
                && !selection_reason.is_empty() =>
            {
                (rev, None)
            }
            (
                ReviewMode::VersionComparison,
                Self::DateRule {
                    rev,
                    selection_time,
                    selected_by,
                    selection_reason,
                    rule: Some(rule),
                    history_tip: Some(tip),
                },
            ) if instant(selection_time) == Some(cutoff)
                && !selected_by.is_empty()
                && !selection_reason.is_empty()
                && rule == "last_qualifying_first_parent"
                && !tip.is_empty() =>
            {
                (rev, None)
            }
            _ => return None,
        };
        (!selected.0.is_empty() && selected.1.is_none_or(|branch| !branch.is_empty()))
            .then_some((selected.0.as_str(), selected.1.map(String::as_str)))
    }
}

pub(super) fn merge<'a>(
    request: &'a AdmissionRequest,
    snapshot: &SnapshotInput,
) -> Result<Option<&'a str>, Refusal> {
    let SnapshotInput::ComputedTree {
        tree,
        computed_at,
        merge_receipt,
    } = snapshot
    else {
        return Ok(None);
    };
    let invalid =
        |dimension| Refusal::new("invalid_merge_proof", merge_receipt).dimension(dimension);
    let merge = request
        .merge
        .as_ref()
        .filter(|merge| merge.id == *merge_receipt)
        .ok_or_else(|| invalid("merge_receipt"))?;
    let expected = request
        .expected_merge
        .as_ref()
        .filter(|merge| merge.id == *merge_receipt)
        .ok_or_else(|| invalid("merge_receipt"))?;
    for (dimension, actual, expected) in [
        ("base", &merge.base, &expected.base),
        ("head", &merge.head, &expected.head),
        ("target", &merge.target, &expected.target),
        ("procedure", &merge.procedure, &expected.procedure),
        ("result_tree", &merge.result_tree, &expected.result_tree),
        ("computed_at", &merge.computed_at, &expected.computed_at),
    ] {
        if actual != expected || actual.as_deref().is_none_or(str::is_empty) {
            return Err(invalid(dimension));
        }
    }
    if merge.result_tree.as_ref() != Some(tree) {
        return Err(invalid("result_tree"));
    }
    if merge.computed_at.as_ref() != Some(computed_at) || instant(computed_at).is_none() {
        return Err(invalid("computed_at"));
    }
    if request.mode == ReviewMode::HistoricalAsOf {
        let cutoff = cutoff(request).ok_or_else(|| invalid("cutoff_evidence"))?;
        let id = merge
            .cutoff_evidence
            .as_deref()
            .ok_or_else(|| invalid("cutoff_evidence"))?;
        let receipt = super::slots::unique(
            request.receipts.iter().filter(|r| r.id == id),
            id,
            "missing_receipt",
        )
        .map_err(|_| invalid("cutoff_evidence"))?;
        if !matches!(&receipt.origin, ReceiptOrigin::IndependentCutoff { evidence, available_at, covered_tree: Some(covered), .. }
            if covered == tree && !evidence.is_empty() && instant(available_at).is_some_and(|at| at <= cutoff))
        {
            return Err(invalid("cutoff_evidence"));
        }
        return Ok(Some(id));
    }
    Ok(None)
}
