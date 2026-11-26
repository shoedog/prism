use super::*;
use crate::revision::{instant, Availability, ClaimDecision, ReceiptOrigin, Refusal, Role};
use std::collections::BTreeSet;

pub(super) fn validate(doc: &ImpactDocument) -> AdmissionReport {
    let mut report = AdmissionReport {
        mode: doc.mode,
        ..Default::default()
    };
    let mut inputs = BTreeMap::new();
    let mut duplicates = BTreeSet::new();
    for input in &doc.inputs {
        if inputs.insert(input.input_id.as_str(), input).is_some() {
            duplicates.insert(input.input_id.as_str());
        }
    }
    let cutoff = instant(&doc.cutoff_at);
    let report_time = instant(&doc.report_time);
    let cutoff_ok = cutoff
        .zip(report_time)
        .is_some_and(|(cutoff, report_time)| match doc.mode {
            Some(ReviewMode::Live) => cutoff == report_time,
            Some(ReviewMode::HistoricalAsOf | ReviewMode::VersionComparison) => {
                cutoff <= report_time
            }
            None => false,
        });
    report.insert(
        "report-cutoff",
        ClaimDecision::from_reasons(
            None,
            if cutoff_ok {
                Vec::new()
            } else {
                vec![Refusal::new("missing_cutoff", "report-cutoff")]
            },
        ),
    );

    let local: BTreeMap<_, _> = inputs
        .iter()
        .map(|(&id, input)| {
            let reasons = if duplicates.contains(id) {
                vec![Refusal::new("receipt_conflict", id)]
            } else {
                input_refusals(doc, input)
            };
            (id, reasons)
        })
        .collect();
    for comparison in &doc.comparisons {
        let id = comparison.comparison_id.as_str();
        if report.claims.contains_key(id)
            && doc
                .comparisons
                .iter()
                .filter(|pair| pair.comparison_id == id)
                .count()
                == 1
        {
            continue;
        }
        if doc
            .comparisons
            .iter()
            .filter(|pair| pair.comparison_id == id)
            .count()
            > 1
        {
            if !report.claims.contains_key(id) {
                report.insert(
                    id,
                    ClaimDecision::from_reasons(None, vec![Refusal::new("receipt_conflict", id)]),
                );
            }
            continue;
        }
        let mut reasons = Vec::new();
        if !cutoff_ok {
            reasons.push(Refusal::new("missing_cutoff", "report-cutoff"));
        }
        for (input, role) in [
            (&comparison.service_input_id, Role::Producer),
            (&comparison.client_input_id, Role::Consumer),
        ] {
            match inputs.get(input.as_str()) {
                None => reasons.push(Refusal::new("missing_slot", input)),
                Some(input) if input.role != role => {
                    reasons.push(Refusal::new("slot_mismatch", &input.input_id).dimension("role"))
                }
                Some(input) => {
                    reasons.extend(walk_input(&input.input_id, &inputs, &local));
                }
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
        let actual_id = match id {
            "service_first" => Some("actual_before"),
            "client_first" => Some("actual_after"),
            _ => None,
        };
        if let Some(actual_id) = actual_id {
            match doc
                .comparisons
                .iter()
                .find(|actual| actual.comparison_id == actual_id)
            {
                Some(actual)
                    if actual.client_input_id != comparison.client_input_id
                        || actual.matches.keys().ne(comparison.matches.keys()) =>
                {
                    reasons.push(Refusal::new("slot_mismatch", id).dimension("occurrences"));
                }
                None => reasons.push(Refusal::new("missing_dependency", actual_id)),
                _ => {}
            }
        }
        report.insert(id, ClaimDecision::from_reasons(None, reasons));
    }

    for (index, usage) in doc.usages.iter().enumerate() {
        let mut reasons = Vec::new();
        let id = usage
            .usage_id
            .clone()
            .unwrap_or_else(|| format!("usage:{index}"));
        for (basis, reference) in [
            ("actual_before", &usage.match_before),
            ("actual_after", &usage.match_after),
            ("service_first", &usage.match_service_first),
            ("client_first", &usage.match_client_first),
        ] {
            if let Some(reference) = reference {
                check_reference(doc, &report, basis, reference, &mut reasons);
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
        report.insert(&id, ClaimDecision::from_reasons(None, reasons));
    }

    if let Some(action) = &doc.change.review_action {
        let mut reasons = Vec::new();
        if action.evidence.is_empty() && action.supporting_row_ids.is_empty() {
            reasons.push(Refusal::new("missing_dependency", &action.claim_id));
        }
        for reference in &action.evidence {
            check_reference(doc, &report, &action.comparison_id, reference, &mut reasons);
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
        report.insert(&action.claim_id, ClaimDecision::from_reasons(None, reasons));
    }
    validate_links(doc, &mut report);
    report
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
            report.insert(id, ClaimDecision::from_reasons(None, reasons));
        }
    }
    for id in duplicate_rows {
        report.insert(
            id,
            ClaimDecision::from_reasons(None, vec![Refusal::new("receipt_conflict", id)]),
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
        for claim in &limit.affected_claims {
            if !rows.get(claim).is_some_and(|links| links.contains(id)) {
                reasons.push(Refusal::new("missing_dependency", claim));
            }
        }
        if !reasons.is_empty() {
            report.insert(id, ClaimDecision::from_reasons(None, reasons));
        }
    }
}

fn check_reference(
    doc: &ImpactDocument,
    report: &AdmissionReport,
    comparison_id: &str,
    reference: &MatchRef,
    reasons: &mut Vec<Refusal>,
) {
    if reference.comparison_id != comparison_id {
        reasons
            .push(Refusal::new("slot_mismatch", &reference.comparison_id).dimension("comparison"));
    } else if let Some(decision) = report
        .claims
        .get(comparison_id)
        .filter(|decision| decision.status != "admitted")
    {
        reasons.extend(decision.reasons.clone());
    } else if !doc.comparisons.iter().any(|pair| {
        pair.comparison_id == comparison_id && pair.matches.contains_key(&reference.site_id)
    }) {
        reasons.push(Refusal::new("missing_dependency", &reference.site_id));
    }
}

fn pair_for<'a>(change: &'a Change, id: &str) -> Option<(&'a String, &'a String)> {
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

fn input_refusals(doc: &ImpactDocument, input: &DocumentInput) -> Vec<Refusal> {
    use crate::model::SchemaOrigin;
    let id = &input.input_id;
    let Some((tree, rev)) = input.snapshot.identity() else {
        return vec![Refusal::new("unsupported_kind", id)];
    };
    if input.api != doc.api {
        return vec![Refusal::new("slot_mismatch", id).dimension("api")];
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
                rev: source_rev, ..
            } if source_rev.as_deref() != rev => {
                return vec![Refusal::new("slot_mismatch", id).dimension("rev")];
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

fn walk_input<'a>(
    id: &'a str,
    inputs: &BTreeMap<&'a str, &'a DocumentInput>,
    local: &BTreeMap<&str, Vec<Refusal>>,
) -> Vec<Refusal> {
    let mut active = BTreeSet::new();
    let mut visited = BTreeSet::new();
    let mut steps = vec![(id, true)];
    let mut reasons = Vec::new();
    while let Some((key, enter)) = steps.pop() {
        if !enter {
            active.remove(key);
            visited.insert(key);
        } else if !visited.contains(key) {
            if !active.insert(key) {
                reasons.push(Refusal::new("dependency_cycle", key));
            } else if let Some(input) = inputs.get(key) {
                reasons.extend(local[key].clone());
                steps.push((key, false));
                steps.extend(
                    input
                        .dependencies
                        .iter()
                        .rev()
                        .map(|id| (id.as_str(), true)),
                );
                steps.extend(
                    input
                        .config_sources
                        .iter()
                        .flat_map(|config| &config.dependencies)
                        .map(|id| (id.as_str(), true)),
                );
            } else {
                reasons.push(Refusal::new("missing_dependency", key));
                active.remove(key);
                visited.insert(key);
            }
        }
    }
    reasons
}
