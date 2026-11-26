use super::input_validation::{input_refusals, slot_cutoff, validate_slots, validate_status};
use super::*;
use crate::model::decode::{matches_contract, Kind, MAX_PORTABLE_COUNT};
use crate::revision::{ClaimDecision, Refusal, Role};
use std::collections::BTreeSet;

pub(super) fn validate(doc: &ImpactDocument) -> AdmissionReport {
    let mut report = AdmissionReport {
        mode: doc.mode,
        ..Default::default()
    };
    if !serde_json::to_value(doc).is_ok_and(|wire| matches_contract(&wire, Kind::Impact)) {
        report.settle(
            "document-schema",
            None,
            vec![Refusal::new("invalid_document", &doc.report_id)],
            [],
        );
    }
    let mut inputs = BTreeMap::new();
    let mut duplicates = BTreeSet::new();
    for input in &doc.inputs {
        if inputs.insert(input.input_id.as_str(), input).is_some() {
            duplicates.insert(input.input_id.as_str());
        }
    }
    settle_inputs(doc, &inputs, &duplicates, &mut report);
    let (reasons, required) = slot_cutoff(doc, &inputs);
    report.settle("report-cutoff", None, reasons, required);
    validate_slots(doc, &inputs, &mut report);
    validate_status(doc, &inputs, &mut report);
    for (name, metric) in &doc.summary {
        let reason = metric.reason.as_deref();
        let valid_total = match metric.total {
            Some(total) => {
                metric.observed <= total && total as u64 <= MAX_PORTABLE_COUNT && reason.is_none()
            }
            None => reason.is_some_and(|reason| !reason.is_empty()),
        };
        if !valid_total || metric.observed as u64 > MAX_PORTABLE_COUNT {
            report.settle(
                &format!("metric:{name}"),
                None,
                vec![Refusal::new("invalid_metric", name)],
                [],
            );
        }
    }

    let mut seen_comparisons = BTreeSet::new();
    for comparison in &doc.comparisons {
        let id = comparison.comparison_id.as_str();
        if !seen_comparisons.insert(id) {
            continue;
        }
        let key = comparison_key(id);
        if doc
            .comparisons
            .iter()
            .filter(|pair| pair.comparison_id == id)
            .count()
            > 1
        {
            report.settle(&key, None, vec![Refusal::new("receipt_conflict", id)], []);
            continue;
        }
        let mut reasons = Vec::new();
        for (input, role) in [
            (&comparison.service_input_id, Role::Producer),
            (&comparison.client_input_id, Role::Consumer),
        ] {
            match inputs.get(input.as_str()) {
                None => reasons.push(Refusal::new("missing_slot", input)),
                Some(input) if input.role != role => {
                    reasons.push(Refusal::new("slot_mismatch", &input.input_id).dimension("role"))
                }
                Some(_) => {}
            }
        }
        let expected = pair_for(&doc.change, id);
        if comparison.purpose != id
            || expected != Some((&comparison.service_input_id, &comparison.client_input_id))
        {
            reasons.push(Refusal::new("slot_mismatch", id).dimension("comparison"));
        }
        for (site, result) in &comparison.matches {
            if site.is_empty()
                || result.client_input_id != comparison.client_input_id
                || result.service_input_id != comparison.service_input_id
            {
                reasons.push(Refusal::new("slot_mismatch", site).dimension("comparison"));
            }
        }
        let inventory = inputs
            .get(comparison.client_input_id.as_str())
            .and_then(|input| input.occurrence_ids.as_ref());
        match inventory {
            None => reasons.push(
                Refusal::new("missing_dependency", &comparison.client_input_id)
                    .dimension("occurrence_inventory"),
            ),
            Some(ids) => {
                let occurrences: BTreeSet<_> = ids.iter().map(String::as_str).collect();
                if occurrences.len() != ids.len()
                    || occurrences
                        .iter()
                        .copied()
                        .ne(comparison.matches.keys().map(String::as_str))
                {
                    reasons.push(Refusal::new("slot_mismatch", id).dimension("occurrences"));
                }
            }
        }
        let decision = ClaimDecision::from_reasons(
            None,
            reasons,
            [
                report
                    .dependency("report-cutoff")
                    .with_refusal_code("missing_cutoff"),
                report.dependency(&format!("input:{}", comparison.service_input_id)),
                report.dependency(&format!("input:{}", comparison.client_input_id)),
            ],
        );
        report.insert(&key, decision);
    }

    for (index, usage) in doc.usages.iter().enumerate() {
        let mut reasons = Vec::new();
        let mut required = Vec::new();
        let id = usage
            .usage_id
            .clone()
            .unwrap_or_else(|| format!("usage:{index}"));
        for (basis, reference, presence, site) in [
            (
                "actual_before",
                &usage.match_before,
                &usage.before_presence,
                &usage.before_site,
            ),
            (
                "actual_after",
                &usage.match_after,
                &usage.after_presence,
                &usage.after_site,
            ),
            (
                "service_first",
                &usage.match_service_first,
                &usage.before_presence,
                &usage.before_site,
            ),
            (
                "client_first",
                &usage.match_client_first,
                &usage.after_presence,
                &usage.after_site,
            ),
        ] {
            let expected_pair = pair_for(&doc.change, basis);
            let match_required = expected_pair.is_some() && presence == "present";
            if match_required != reference.is_some() || (presence == "present") != site.is_some() {
                reasons.push(Refusal::new("missing_dependency", basis).dimension("usage_match"));
                continue;
            }
            if let Some(reference) = reference {
                check_reference(doc, basis, reference, &mut reasons);
                required.push(comparison_key(basis));
                if expected_pair
                    .and_then(|(_, client)| inputs.get(client.as_str()))
                    .zip(site.as_ref())
                    .is_none_or(|(input, site)| {
                        input
                            .snapshot
                            .identity()
                            .is_none_or(|(tree, _)| tree != site.tree)
                    })
                {
                    reasons.push(Refusal::new("slot_mismatch", &id).dimension("usage_site_tree"));
                }
            }
        }
        for (actual, alternate) in [
            (&usage.match_before, &usage.match_service_first),
            (&usage.match_after, &usage.match_client_first),
        ] {
            if actual
                .as_ref()
                .zip(alternate.as_ref())
                .is_some_and(|(a, b)| a.site_id != b.site_id)
            {
                reasons.push(Refusal::new("slot_mismatch", &id).dimension("site_id"));
            }
        }
        let key = format!("usage:{index}:{id}");
        if usage.usage_id.is_some()
            && doc
                .usages
                .iter()
                .filter(|item| item.usage_id == usage.usage_id)
                .count()
                > 1
        {
            reasons.push(Refusal::new("receipt_conflict", &id));
        }
        report.settle(&key, None, reasons, required);
    }

    if let Some(action) = &doc.change.review_action {
        let mut reasons = Vec::new();
        if action.evidence.is_empty() && action.supporting_row_ids.is_empty() {
            reasons.push(Refusal::new("missing_dependency", &action.claim_id));
        }
        for reference in &action.evidence {
            check_reference(doc, &action.comparison_id, reference, &mut reasons);
        }
        for id in &action.supporting_row_ids {
            if !doc
                .change
                .elements
                .iter()
                .chain(&doc.impacts)
                .any(|row| row.row_id == *id)
            {
                reasons.push(Refusal::new("missing_dependency", id));
            }
        }
        let key = if action.claim_id == "review-action" {
            "review-action".into()
        } else {
            format!("review-action:{}", action.claim_id)
        };
        report.settle(&key, None, reasons, [comparison_key(&action.comparison_id)]);
    }
    validate_auxiliary(doc, &inputs, &mut report);
    validate_links(doc, &mut report);
    for (index, warning) in doc.warnings.iter().enumerate() {
        if let Some(refusal) = &warning.refusal {
            report.settle(
                &format!("warning:{index}:{}", warning.claim_id),
                None,
                vec![refusal.clone()],
                [],
            );
        }
    }
    report
}

fn validate_auxiliary(
    doc: &ImpactDocument,
    inputs: &BTreeMap<&str, &DocumentInput>,
    report: &mut AdmissionReport,
) {
    let consumer = doc.change.consumer.as_ref();
    for (index, call) in doc.change.call_changes.iter().enumerate() {
        let key = format!("call-change:{index}");
        let mut reasons = Vec::new();
        let mut required = Vec::new();
        if !match call.kind.as_str() {
            "inserted" => call.before_site_id.is_none() && call.after_site_id.is_some(),
            "deleted" => call.before_site_id.is_some() && call.after_site_id.is_none(),
            "retargeted" => call.before_site_id.is_some() && call.after_site_id.is_some(),
            "unknown" => call
                .reason
                .as_deref()
                .is_some_and(|reason| !reason.is_empty()),
            _ => false,
        } {
            reasons.push(Refusal::new("slot_mismatch", &key).dimension("call_change"));
        }
        for (occurrence, input_id) in [
            (
                call.before_site_id.as_ref(),
                consumer.and_then(|slots| slots.before_input_id.as_ref()),
            ),
            (
                call.after_site_id.as_ref(),
                consumer.and_then(|slots| slots.after_input_id.as_ref()),
            ),
        ] {
            if let Some(occurrence) = occurrence {
                required.extend(input_id.map(|id| format!("input:{id}")));
                if !input_id
                    .and_then(|id| inputs.get(id.as_str()))
                    .and_then(|input| input.occurrence_ids.as_ref())
                    .is_some_and(|inventory| inventory.contains(occurrence))
                {
                    reasons.push(Refusal::new("missing_dependency", occurrence));
                }
            }
        }
        report.settle(&key, None, reasons, required);
    }
    let rows: BTreeSet<_> = doc
        .change
        .elements
        .iter()
        .chain(&doc.impacts)
        .map(|row| &row.row_id)
        .collect();
    for (index, risk) in doc.change.risks.iter().enumerate() {
        let reasons = risk
            .supporting_row_ids
            .iter()
            .filter(|id| !rows.contains(id))
            .map(|id| Refusal::new("missing_dependency", id))
            .collect();
        report.settle(&format!("risk:{index}"), None, reasons, []);
    }
    for (index, observation) in doc.unobserved.iter().enumerate() {
        let reasons = observation
            .membership_basis
            .iter()
            .filter(|id| !inputs.contains_key(id.as_str()))
            .map(|id| Refusal::new("missing_slot", id))
            .collect();
        report.settle(
            &format!("membership:{index}"),
            None,
            reasons,
            observation
                .membership_basis
                .iter()
                .map(|id| format!("input:{id}")),
        );
    }
}

fn validate_links(doc: &ImpactDocument, report: &mut AdmissionReport) {
    let limits: BTreeSet<_> = doc
        .coverage
        .iter()
        .map(|limit| limit.limit_id.as_str())
        .collect();
    let keys: BTreeSet<_> = doc.coverage.iter().map(CoverageLimit::key).collect();
    let mut rows = BTreeMap::new();
    let mut duplicate_rows = BTreeSet::new();
    for (id, links) in doc
        .change
        .elements
        .iter()
        .chain(&doc.impacts)
        .map(|row| (&row.row_id, &row.coverage_limits))
        .chain(
            doc.warnings
                .iter()
                .map(|claim| (&claim.claim_id, &claim.coverage_limits)),
        )
    {
        if rows.insert(id, links).is_some() {
            duplicate_rows.insert(id);
        }
        let reasons = links
            .iter()
            .filter(|link| {
                !doc.coverage
                    .iter()
                    .any(|limit| limit.limit_id == **link && limit.affected_claims.contains(id))
            })
            .map(|link| Refusal::new("missing_dependency", link))
            .collect::<Vec<_>>();
        if !reasons.is_empty() {
            report.settle(&format!("row:{id}"), None, reasons, []);
        }
    }
    for id in duplicate_rows {
        report.settle(
            &format!("row-duplicate:{id}"),
            None,
            vec![Refusal::new("receipt_conflict", id)],
            [],
        );
    }
    for limit in &doc.coverage {
        let id = &limit.limit_id;
        let mut reasons = Vec::new();
        if limits.len() != doc.coverage.len()
            && doc
                .coverage
                .iter()
                .filter(|item| item.limit_id == *id)
                .count()
                > 1
            || keys.len() != doc.coverage.len()
                && doc
                    .coverage
                    .iter()
                    .filter(|item| item.key() == limit.key())
                    .count()
                    > 1
        {
            reasons.push(Refusal::new("receipt_conflict", id));
        }
        if !doc
            .inputs
            .iter()
            .any(|input| input.input_id == limit.input_id)
        {
            reasons.push(Refusal::new("missing_slot", &limit.input_id));
        }
        let linked_basis = if limit.comparison_id == "service_diff" {
            doc.change
                .producer
                .as_ref()
                .is_some_and(|slots| match limit.side.as_str() {
                    "before" => slots.before_input_id.as_ref() == Some(&limit.input_id),
                    "after" => slots.after_input_id.as_ref() == Some(&limit.input_id),
                    _ => false,
                })
        } else {
            doc.comparisons
                .iter()
                .filter(|pair| pair.comparison_id == limit.comparison_id)
                .count()
                == 1
                && doc.comparisons.iter().any(|pair| {
                    pair.comparison_id == limit.comparison_id
                        && (pair.client_input_id == limit.input_id
                            || pair.service_input_id == limit.input_id)
                })
        };
        if !linked_basis {
            reasons.push(
                Refusal::new("missing_dependency", &limit.comparison_id).dimension("comparison_id"),
            );
        }
        for claim in &limit.affected_claims {
            if !rows.get(claim).is_some_and(|links| links.contains(id)) {
                reasons.push(Refusal::new("missing_dependency", claim));
            }
        }
        if !reasons.is_empty() {
            report.settle(&format!("limit:{id}"), None, reasons, []);
        }
    }
}

fn check_reference(
    doc: &ImpactDocument,
    comparison_id: &str,
    reference: &MatchRef,
    reasons: &mut Vec<Refusal>,
) {
    if reference.comparison_id != comparison_id {
        reasons
            .push(Refusal::new("slot_mismatch", &reference.comparison_id).dimension("comparison"));
    } else if !doc.comparisons.iter().any(|pair| {
        pair.comparison_id == comparison_id && pair.matches.contains_key(&reference.site_id)
    }) {
        reasons.push(Refusal::new("missing_dependency", &reference.site_id));
    }
}

pub(super) fn pair_for<'a>(change: &'a Change, id: &str) -> Option<(&'a String, &'a String)> {
    let producer = change.producer.as_ref()?;
    let consumer = change.consumer.as_ref()?;
    let (service, client) = match id {
        "actual_before" => (&producer.before_input_id, &consumer.before_input_id),
        "actual_after" => (&producer.after_input_id, &consumer.after_input_id),
        "service_first" if change.subject_role == "both" => {
            (&producer.after_input_id, &consumer.before_input_id)
        }
        "client_first" if change.subject_role == "both" => {
            (&producer.before_input_id, &consumer.after_input_id)
        }
        _ => return None,
    };
    if change.subject_role == "client" && producer.before_input_id != producer.after_input_id
        || change.subject_role == "service" && consumer.before_input_id != consumer.after_input_id
    {
        return None;
    }
    service.as_ref().zip(client.as_ref())
}

fn comparison_key(id: &str) -> String {
    match id {
        "actual_before" | "actual_after" | "service_first" | "client_first" => id.into(),
        _ => format!("comparison:{id}"),
    }
}

fn settle_inputs(
    doc: &ImpactDocument,
    inputs: &BTreeMap<&str, &DocumentInput>,
    duplicates: &BTreeSet<&str>,
    report: &mut AdmissionReport,
) {
    fn dependencies(input: &DocumentInput) -> impl DoubleEndedIterator<Item = &str> {
        input
            .dependencies
            .iter()
            .chain(
                input
                    .config_sources
                    .iter()
                    .flat_map(|config| &config.dependencies),
            )
            .map(String::as_str)
    }
    let mut active = BTreeSet::new();
    let mut steps: Vec<_> = inputs.keys().rev().map(|&id| (id, true)).collect();
    while let Some((id, enter)) = steps.pop() {
        let key = format!("input:{id}");
        let Some(input) = inputs.get(id) else {
            continue;
        };
        if enter {
            if !report.claims.contains_key(&key) && active.insert(id) {
                steps.push((id, false));
                steps.extend(dependencies(input).rev().map(|id| (id, true)));
            }
        } else {
            let mut reasons = if duplicates.contains(id) {
                vec![Refusal::new("receipt_conflict", id)]
            } else {
                input_refusals(doc, input)
            };
            if let Some(cycle) = dependencies(input).find(|id| active.contains(id)) {
                reasons.push(Refusal::new("dependency_cycle", cycle));
            }
            active.remove(id);
            report.settle(
                &key,
                Some(id),
                reasons,
                dependencies(input).map(|id| format!("input:{id}")),
            );
        }
    }
}
