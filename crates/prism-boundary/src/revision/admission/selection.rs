use super::*;
use crate::revision::ServiceSide;
use chrono::{DateTime, FixedOffset};

pub(crate) fn instant(value: &str) -> Option<DateTime<FixedOffset>> {
    DateTime::parse_from_rfc3339(value).ok()
}

pub(super) fn cutoff(request: &AdmissionRequest) -> Option<DateTime<FixedOffset>> {
    let cutoff = instant(&request.cutoff_at)?;
    match request.mode {
        ReviewMode::Live => (instant(&request.report_time)? == cutoff).then_some(cutoff),
        ReviewMode::HistoricalAsOf => {
            request.subject_head_input_id.as_ref()?;
            let subject = subject(request)?;
            historical_head(&subject.snapshot, cutoff)?;
            request
                .subject_head_committed_at
                .as_deref()
                .is_none_or(|legacy| instant(legacy) == Some(cutoff))
                .then_some(cutoff)
        }
        ReviewMode::VersionComparison => {
            (instant(request.selection_time.as_deref()?)? == cutoff).then_some(cutoff)
        }
    }
}

pub(crate) fn historical_head(
    snapshot: &SnapshotInput,
    cutoff: DateTime<FixedOffset>,
) -> Option<&str> {
    match snapshot {
        SnapshotInput::Commit {
            rev,
            committed_at: Some(date),
            ..
        } if instant(date) == Some(cutoff) && !rev.is_empty() => Some(rev),
        _ => None,
    }
}

pub(crate) fn exact_tree_at_cutoff(
    origin: &ReceiptOrigin,
    tree: &str,
    cutoff: DateTime<FixedOffset>,
) -> bool {
    matches!(origin, ReceiptOrigin::IndependentCutoff { evidence, available_at, covered_tree: Some(covered), .. }
        if covered == tree && !evidence.is_empty() && instant(available_at).is_some_and(|at| at <= cutoff))
}

pub(crate) fn check_merge_head(
    receipt: &str,
    topic: Option<&str>,
    proposed: Option<&str>,
) -> Result<(), Refusal> {
    if topic.is_some() && topic == proposed {
        Ok(())
    } else {
        Err(Refusal::new("invalid_merge_proof", receipt).dimension("head"))
    }
}

pub(crate) fn selection_required(
    mode: ReviewMode,
    side: Option<ServiceSide>,
    role: Role,
    repo: &str,
    subject: Option<(Role, &str)>,
    snapshot: &SnapshotInput,
) -> bool {
    mode == ReviewMode::VersionComparison
        || !matches!(side, Some(ServiceSide::Before | ServiceSide::After))
        || subject != Some((role, repo))
        || !matches!(
            snapshot,
            SnapshotInput::Commit { .. } | SnapshotInput::ComputedTree { .. }
        )
}

pub(super) fn subject(request: &AdmissionRequest) -> Option<&AdmissionInput> {
    let mut candidates = request.inputs.iter().filter(|input| {
        request.subject_head_input_id.as_ref().map_or_else(
            || {
                input.binding.role == Role::Producer
                    && input.binding.service_side == Some(ServiceSide::After)
            },
            |id| *id == input.binding.input_id,
        )
    });
    let head = candidates.next()?;
    if candidates.any(|input| input != head)
        || matches!(
            head.binding.service_side,
            Some(ServiceSide::Before | ServiceSide::Held)
        )
        || slots::validate_input(request, head).is_err()
    {
        return None;
    }
    Some(head)
}

pub(super) fn selection(
    request: &AdmissionRequest,
    subject: Option<&AdmissionInput>,
    input: &AdmissionInput,
    side: Option<ServiceSide>,
) -> Result<(), Refusal> {
    let binding = &input.binding;
    let id = &binding.input_id;
    let missing = || Refusal::new("missing_cutoff", id);
    let position = if side == Some(ServiceSide::Held) {
        side
    } else if subject.is_some_and(|head| head.binding.input_id == *id) {
        Some(ServiceSide::After)
    } else if side.is_none()
        && binding.role == Role::Consumer
        && subject.is_some_and(|head| {
            head.binding.role == binding.role && head.binding.repo == binding.repo
        })
    {
        Some(ServiceSide::Before)
    } else {
        side
    };
    let required = selection_required(
        request.mode,
        position,
        binding.role,
        &binding.repo,
        subject.map(|head| (head.binding.role, head.binding.repo.as_str())),
        &input.snapshot,
    );
    if !required && input.selection.is_none() {
        return Ok(());
    }
    let recorded_cutoff = cutoff(request).ok_or_else(missing)?;
    let receipt = input.selection.as_ref().ok_or_else(missing)?;
    receipt.check(
        request.mode,
        recorded_cutoff,
        id,
        binding.rev.as_deref(),
        binding.branch.as_deref(),
    )?;
    Ok(())
}

impl SelectionReceipt {
    pub(crate) fn check(
        &self,
        mode: ReviewMode,
        cutoff: DateTime<FixedOffset>,
        id: &str,
        rev: Option<&str>,
        branch: Option<&str>,
    ) -> Result<(&str, Option<&str>), Refusal> {
        let proof = self.proof(mode, cutoff).ok_or_else(|| {
            let code = if *self == Self::Unsupported {
                "unsupported_kind"
            } else {
                "missing_cutoff"
            };
            Refusal::new(code, id)
        })?;
        if proof.1.is_some() && proof.1 != branch {
            return Err(Refusal::new("missing_cutoff", id).dimension("branch"));
        }
        if rev != Some(proof.0) {
            return Err(Refusal::mismatch(id, "rev", rev, Some(proof.0)));
        }
        Ok(proof)
    }

    pub(super) fn proof(
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

// Each proof has already been checked against the same report instant.
pub(crate) fn selection_conflicts<'a>(
    selections: impl IntoIterator<Item = (&'a str, &'a str, (&'a str, Option<&'a str>))>,
) -> Vec<Refusal> {
    let mut by_branch = BTreeMap::<_, Vec<_>>::new();
    for (id, repo, (rev, branch)) in selections {
        if let Some(branch) = branch {
            by_branch.entry((repo, branch)).or_default().push((id, rev));
        }
    }
    let mut conflicts = Vec::new();
    for observations in by_branch.into_values() {
        if observations
            .iter()
            .any(|(_, rev)| *rev != observations[0].1)
        {
            let evidence: Vec<_> = observations
                .iter()
                .map(|(id, _)| (*id).to_owned())
                .collect();
            for (id, _) in observations {
                let mut refusal = Refusal::new("receipt_conflict", id).dimension("selection");
                refusal.evidence.clone_from(&evidence);
                conflicts.push(refusal);
            }
        }
    }
    conflicts
}

pub(super) fn merge<'a>(
    request: &'a AdmissionRequest,
    input: &AdmissionInput,
    slot: &InputSlot,
) -> Result<Option<&'a str>, Refusal> {
    let SnapshotInput::ComputedTree {
        tree,
        computed_at,
        merge_receipt,
    } = &input.snapshot
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
        let topic = subject(request)
            .filter(|topic| topic.binding.role == slot.role && topic.binding.repo == slot.repo);
        // Subject merge results have an explicit service After slot or are
        // sideless computed client-after inputs in the subject repository.
        // Computed Before/Held inputs carry their own receipts for earlier or
        // counterpart trees, independently of the subject's merge head.
        let subject_result = slot.service_side == Some(ServiceSide::After)
            || (slot.service_side.is_none() && slot.role == Role::Consumer && topic.is_some());
        if subject_result {
            check_merge_head(
                merge_receipt,
                topic.and_then(|topic| historical_head(&topic.snapshot, cutoff)),
                merge.head.as_deref(),
            )?;
        }
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
        if !exact_tree_at_cutoff(&receipt.origin, tree, cutoff) {
            return Err(invalid("cutoff_evidence"));
        }
        return Ok(Some(id));
    }
    Ok(None)
}
