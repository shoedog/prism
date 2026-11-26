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
            (instant(request.subject_head_committed_at.as_deref()?)? == cutoff).then_some(cutoff)
        }
        ReviewMode::VersionComparison => {
            (instant(request.selection_time.as_deref()?)? == cutoff).then_some(cutoff)
        }
    }
}

pub(super) fn selection(request: &AdmissionRequest, input: &AdmissionInput) -> Result<(), Refusal> {
    let id = &input.binding.input_id;
    let missing = || Refusal::new("missing_cutoff", id);
    let recorded_cutoff = cutoff(request).ok_or_else(missing)?;
    let selected_rev = match (&request.mode, &input.selection) {
        (
            ReviewMode::Live,
            Some(SelectionReceipt::LiveRef {
                branch,
                rev,
                observed_at,
            }),
        ) if Some(branch.as_str()) == input.binding.branch.as_deref()
            && instant(observed_at) == Some(recorded_cutoff) =>
        {
            rev
        }
        (
            ReviewMode::HistoricalAsOf,
            Some(SelectionReceipt::HistoricalPin {
                branch,
                rev,
                available_at,
                evidence,
            }),
        ) if Some(branch.as_str()) == input.binding.branch.as_deref()
            && instant(available_at) == Some(recorded_cutoff)
            && !evidence.is_empty() =>
        {
            rev
        }
        (
            ReviewMode::HistoricalAsOf,
            Some(SelectionReceipt::HistoricalSnapshot {
                branch,
                rev,
                captured_at,
                evidence,
            }),
        ) if Some(branch.as_str()) == input.binding.branch.as_deref()
            && instant(captured_at) == Some(recorded_cutoff)
            && !evidence.is_empty() =>
        {
            rev
        }
        (
            ReviewMode::HistoricalAsOf,
            Some(SelectionReceipt::HistoricalRefHistory {
                branch,
                rev,
                valid_from,
                valid_until,
                evidence,
            }),
        ) if Some(branch.as_str()) == input.binding.branch.as_deref()
            && instant(valid_from)
                .zip(instant(valid_until))
                .is_some_and(|(start, end)| start <= recorded_cutoff && recorded_cutoff < end)
            && !evidence.is_empty() =>
        {
            rev
        }
        (
            ReviewMode::VersionComparison,
            Some(SelectionReceipt::ExplicitPin {
                rev,
                selection_time: Some(time),
                selected_by: Some(selector),
                selection_reason,
            }),
        ) if instant(time) == Some(recorded_cutoff)
            && !selector.is_empty()
            && !selection_reason.is_empty() =>
        {
            rev
        }
        (
            ReviewMode::VersionComparison,
            Some(SelectionReceipt::DateRule {
                rev,
                selection_time,
                selected_by,
                selection_reason,
                rule: Some(rule),
                history_tip: Some(tip),
            }),
        ) if instant(selection_time) == Some(recorded_cutoff)
            && !selected_by.is_empty()
            && !selection_reason.is_empty()
            && rule == "last_qualifying_first_parent"
            && !tip.is_empty() =>
        {
            rev
        }
        (_, Some(SelectionReceipt::Unsupported)) => {
            return Err(Refusal::new("unsupported_kind", id))
        }
        _ => return Err(missing()),
    };
    if Some(selected_rev.as_str()) != input.binding.rev.as_deref() {
        return Err(Refusal::mismatch(
            id,
            "rev",
            input.binding.rev.as_deref(),
            Some(selected_rev),
        ));
    }
    Ok(())
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
