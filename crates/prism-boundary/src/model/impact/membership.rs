use super::*;
use crate::model::{BindingsDocument, ExtractionStatus};
use crate::revision::Role;
use serde::Deserialize;
use serde_json::json;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct MembershipEvidence {
    input_id: String,
    side: PopulationSide,
    complete: bool,
    #[serde(default)]
    excluded: bool,
    #[serde(default)]
    later: bool,
}

#[derive(Clone, Copy, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
enum PopulationSide {
    Before,
    After,
    Held,
}

pub fn unobserved_elements(request: &MembershipRequest<'_>) -> Value {
    let change = request.change;
    let evidence: Vec<_> = request
        .membership_evidence
        .iter()
        .filter_map(|item| serde_json::from_value::<MembershipEvidence>(item.clone()).ok())
        .collect();
    let mut basis = Vec::new();
    let mut reasons = BTreeMap::new();
    let mut seen = BTreeSet::new();
    let mut expected = request.expected_inputs.to_vec();
    for slots in std::iter::once(request.producer).chain(request.consumer) {
        for id in [&slots.before_input_id, &slots.after_input_id]
            .into_iter()
            .flatten()
        {
            if !expected.contains(id) {
                expected.push(id.clone());
            }
        }
    }

    for binding in request.admitted_bindings {
        if let Some(input) = &binding.input {
            if !expected.contains(&input.input_id) {
                expected.push(input.input_id.clone());
            }
        }
    }
    for (side, presence, service) in [
        (
            "before",
            &change.before_presence,
            &request.producer.before_input_id,
        ),
        (
            "after",
            &change.after_presence,
            &request.producer.after_input_id,
        ),
    ] {
        if presence != "absent" && service.is_none() {
            reasons.insert(format!("producer:{side}"), "missing_membership_evidence");
        }
    }

    for id in &expected {
        if !seen.insert(id) {
            continue;
        }
        let mut records = evidence.iter().filter(|record| &record.input_id == id);
        let population = records
            .next()
            .filter(|record| records.all(|other| other == *record));
        let mut documents = request.admitted_bindings.iter().filter(|doc| {
            doc.input
                .as_ref()
                .is_some_and(|input| &input.input_id == id)
        });
        let doc = documents.next().filter(|_| documents.next().is_none());
        if population.is_none() {
            reasons.insert(id.clone(), "missing_membership_evidence");
            continue;
        }
        let mut applicable = false;
        for (side, presence, element, service, client) in [
            (
                PopulationSide::Before,
                &change.before_presence,
                change.before_element_id.as_deref(),
                &request.producer.before_input_id,
                request
                    .consumer
                    .and_then(|slots| slots.before_input_id.as_ref()),
            ),
            (
                PopulationSide::After,
                &change.after_presence,
                change.after_element_id.as_deref(),
                &request.producer.after_input_id,
                request
                    .consumer
                    .and_then(|slots| slots.after_input_id.as_ref()),
            ),
        ] {
            let role = if service.as_ref() == Some(id) {
                Role::Producer
            } else if client == Some(id) {
                Role::Consumer
            } else if population
                .is_some_and(|record| record.side == side || record.side == PopulationSide::Held)
            {
                let Some(role) = doc
                    .and_then(|doc| doc.input.as_ref())
                    .map(|input| input.role)
                else {
                    reasons.insert(id.clone(), "missing_membership_evidence");
                    continue;
                };
                role
            } else {
                continue;
            };
            if presence == "absent" {
                continue;
            }
            let Some((record, doc)) = population.zip(doc) else {
                reasons.insert(id.clone(), "missing_membership_evidence");
                continue;
            };
            if (record.side != side && record.side != PopulationSide::Held)
                || record.excluded
                || record.later
            {
                if service.as_ref() == Some(id) || client == Some(id) {
                    reasons.insert(id.clone(), "missing_membership_evidence");
                }
                continue;
            }
            if doc.input.as_ref().is_none_or(|input| input.role != role) {
                reasons.insert(id.clone(), "missing_membership_evidence");
                continue;
            }
            applicable = true;
            let Some(element) = element.filter(|_| presence == "present") else {
                reasons.insert(id.clone(), "membership_possible");
                continue;
            };
            match membership(doc, &change.operation_id, element, service.as_deref()) {
                Membership::Present => return json!({"unobserved":[],"coverage":[]}),
                Membership::Possible(reason) => {
                    reasons.insert(id.clone(), reason);
                }
                Membership::Absent if covers(doc, &change.operation_id, element) => {}
                Membership::Absent => {
                    reasons.insert(id.clone(), "missing_membership_evidence");
                }
            }
        }
        if applicable {
            basis.push(id.clone());
        }
    }
    if !reasons.is_empty() {
        let coverage: Vec<_> = reasons
            .into_iter()
            .map(|(id, reason)| json!({"input_id":id,"reason":reason}))
            .collect();
        return json!({"unobserved":[],"coverage":coverage});
    }
    let unobserved = if basis.is_empty() {
        Vec::new()
    } else {
        vec![UnobservedElement {
            before_element_id: change.before_element_id.clone(),
            after_element_id: change.after_element_id.clone(),
            membership_basis: basis,
        }]
    };
    json!({"unobserved":unobserved,"coverage":[]})
}

enum Membership {
    Present,
    Possible(&'static str),
    Absent,
}

fn membership(
    doc: &BindingsDocument,
    operation_id: &str,
    element: &str,
    service: Option<&str>,
) -> Membership {
    if doc
        .input
        .as_ref()
        .is_some_and(|input| input.role == Role::Producer)
    {
        for op in doc
            .operations
            .iter()
            .flatten()
            .filter(|op| op.operation_id == operation_id)
        {
            if operation_id == element
                || op.fields.iter().any(|field| {
                    field.field_id == element
                        && field
                            .presence
                            .as_deref()
                            .is_none_or(|presence| presence == "present")
                })
            {
                return Membership::Present;
            }
            if op.observed_fields.iter().chain(&op.fields).any(|field| {
                field.field_id == element && field.presence.as_deref() != Some("absent")
            }) || op.message_roots.iter().any(|root| {
                let prefix = match (
                    root.direction.as_str(),
                    root.status.as_deref(),
                    operation_id.strip_prefix("op:"),
                ) {
                    ("request", _, Some(op)) => format!("field:req:{op}#"),
                    ("response", Some(status), Some(op)) => format!("field:resp:{op}:{status}#"),
                    _ => return false,
                };
                element.starts_with(&prefix)
                    && (root.coverage != "checked" || !root.reader_limits.is_empty())
            }) {
                return Membership::Possible("membership_possible");
            }
        }
        return Membership::Absent;
    }

    let mut possible = Membership::Absent;
    for call in doc.calls.iter().flatten() {
        let Some(bound) = &call.saved_match else {
            if call.mapped_path.is_none() || may_target(call, operation_id) {
                possible = Membership::Possible("membership_possible");
            }
            continue;
        };
        let contains = |id: &str| bound.target_ids.iter().any(|target| target == id);
        let overlaps = bound
            .overlaps
            .iter()
            .any(|overlap| overlap.element_id == element || overlap.element_id == operation_id);
        if overlaps {
            possible = Membership::Possible("membership_possible");
        }
        if bound.grade == "bound"
            && service.is_some()
            && bound.service_input_id.as_deref() == service
        {
            if element == operation_id && contains(element) {
                return Membership::Present;
            }
            if contains(element) || contains(operation_id) {
                if bound.field_use.iter().any(|use_| {
                    use_.site_id == call.site_id
                        && use_.field_id == element
                        && matches!(use_.kind.as_str(), "read" | "send")
                        && !use_.evidence.is_empty()
                }) {
                    return Membership::Present;
                }
                possible = Membership::Possible("field_use_unchecked");
            }
        } else if contains(element) || contains(operation_id) {
            possible = Membership::Possible("membership_possible");
        }
    }
    possible
}

fn may_target(call: &crate::model::Unbound, operation: &str) -> bool {
    operation
        .strip_prefix("op:")
        .and_then(|operation| operation.split_once(' '))
        .is_none_or(|(method, path)| {
            call.requested_method
                .as_deref()
                .is_none_or(|value| value == method)
                && call
                    .mapped_path
                    .as_deref()
                    .is_none_or(|value| value == path || path.contains('{'))
        })
}

fn covers(doc: &BindingsDocument, operation: &str, element: &str) -> bool {
    let Some(input) = &doc.input else {
        return false;
    };
    if !matches!(input.scope, Scope::Api) {
        // A spelled ID cannot distinguish literal braces from a typed capture.
        return false;
    }
    !doc.expected_files.is_empty()
        && doc
            .expected_files
            .iter()
            .all(|path| doc.file_status.iter().any(|region| region.path == *path))
        && doc
            .file_status
            .iter()
            .chain(&doc.entries)
            .chain(&doc.unaccounted_scopes)
            .all(|region| {
                region.status == ExtractionStatus::Complete
                    || !region.possible_keys.is_empty()
                        && !region
                            .possible_keys
                            .iter()
                            .any(|key| key == element || key == operation)
            })
}

fn covers_usage_population(doc: &BindingsDocument) -> bool {
    let Some(input) = doc.input.as_ref() else {
        return false;
    };
    !doc.expected_files.is_empty()
        && doc
            .expected_files
            .iter()
            .all(|path| doc.file_status.iter().any(|region| region.path == *path))
        && doc
            .file_status
            .iter()
            .chain(&doc.entries)
            .chain(&doc.unaccounted_scopes)
            .all(|region| {
                region.status == ExtractionStatus::Complete
                    || region.scope.intersect(&input.scope).is_none()
            })
}

#[derive(Default)]
struct UsagePopulation {
    client_input_id: String,
    scope: Scope,
    complete: bool,
    target_checks_complete: bool,
    unknown: BTreeSet<String>,
    unmatched: BTreeSet<String>,
}

impl ImpactDocument {
    pub fn with_usage_counts(mut self, bindings: &[BindingsDocument]) -> Self {
        let mut populations = BTreeMap::new();
        for comparison in &mut self.comparisons {
            let expected = super::validation::pair_for(&self.change, &comparison.comparison_id);
            if expected != Some((&comparison.service_input_id, &comparison.client_input_id)) {
                continue;
            }
            let client = self
                .inputs
                .iter()
                .find(|input| input.input_id == comparison.client_input_id);
            let mut supplied = bindings.iter().filter(|doc| {
                doc.input.as_ref().zip(client).is_some_and(|(input, slot)| {
                    input.input_id == slot.input_id
                        && input.role == Role::Consumer
                        && input.api == slot.api
                        && input.repo == slot.repo
                        && input.snapshot == slot.snapshot
                        && input.origin == slot.origin
                        && input.scope.same_region(&slot.scope)
                })
            });
            let Some(doc) = supplied.next().filter(|_| supplied.next().is_none()) else {
                continue;
            };
            let Some(calls) = &doc.calls else {
                continue;
            };
            let mut population = UsagePopulation {
                client_input_id: comparison.client_input_id.clone(),
                scope: doc.input.as_ref().expect("admitted client").scope.clone(),
                complete: covers_usage_population(doc),
                target_checks_complete: true,
                ..UsagePopulation::default()
            };
            let mut unchecked = BTreeSet::new();
            let mut seen = BTreeSet::new();
            for call in calls {
                if !seen.insert(call.site_id.as_str()) {
                    population.complete = false;
                    continue;
                }
                let outcome = comparison.matches.get(&call.site_id).filter(|outcome| {
                    outcome.client_input_id == comparison.client_input_id
                        && outcome.service_input_id == comparison.service_input_id
                });
                if outcome.is_some_and(|record| {
                    record.status == "checked" && record.reason.as_deref() == Some("other_api")
                }) {
                    continue;
                }
                let readable = call
                    .requested_method
                    .as_deref()
                    .is_some_and(|method| !method.is_empty())
                    && (call.mapped_path.is_some()
                        || matches!(
                            call.target_reason.as_deref(),
                            Some("origin_unchecked" | "mapping_unchecked")
                        ) && call
                            .original_url_text
                            .as_deref()
                            .is_some_and(|target| !target.is_empty()));
                if !readable {
                    population.unknown.insert(call.site_id.clone());
                } else if outcome.is_none_or(|record| record.status != "checked") {
                    unchecked.insert(call.site_id.as_str());
                    population.target_checks_complete = false;
                } else if outcome.is_some_and(|record| {
                    record.grade.as_deref() == Some("unbound")
                        && record.reason.as_deref() == Some("no_operation")
                }) {
                    population.unmatched.insert(call.site_id.clone());
                }
            }
            population.complete &= seen
                .iter()
                .copied()
                .eq(comparison.matches.keys().map(String::as_str));
            if let Some(ids) = client.and_then(|input| input.occurrence_ids.as_ref()) {
                population.complete &=
                    ids.len() == seen.len() && ids.iter().all(|id| seen.contains(id.as_str()));
            }
            comparison.unchecked_count = unchecked.len();
            let key = comparison.comparison_id.as_str();
            if populations.insert(key, Some(population)).is_some() {
                populations.insert(key, None);
            }
        }
        for (name, unmatched) in [
            ("known_unmatched_count", true),
            ("unknown_target_count", false),
        ] {
            let Some(metric) = self.summary.get_mut(name) else {
                continue;
            };
            let sides = metric_populations(&self.change, &metric.version);
            let mut observed = BTreeSet::new();
            let mut complete = !sides.is_empty();
            for side in sides {
                let Some(population) = populations.get(side).and_then(Option::as_ref) else {
                    complete = false;
                    continue;
                };
                if !population.scope.same_region(&metric.scope) {
                    complete = false;
                    if !metric
                        .scope
                        .intersect(&population.scope)
                        .is_some_and(|scope| scope.same_region(&population.scope))
                    {
                        continue;
                    }
                }
                if !population.complete || (unmatched && !population.target_checks_complete) {
                    complete = false;
                }
                let occurrences = if unmatched {
                    &population.unmatched
                } else {
                    &population.unknown
                };
                observed.extend(
                    occurrences
                        .iter()
                        .map(|site| (&population.client_input_id, site)),
                );
            }
            metric.observed = observed.len();
            metric.total = complete.then_some(metric.observed);
            metric.reason = (!complete).then(|| "missing_relevant_population".into());
        }
        self
    }
}

fn metric_populations(change: &Change, version: &str) -> Vec<&'static str> {
    [
        "actual_before",
        "actual_after",
        "service_first",
        "client_first",
    ]
    .into_iter()
    .filter(|&side| {
        let expected = super::validation::pair_for(change, side);
        let actual = matches!(side, "actual_before" | "actual_after");
        match version {
            "joint" | "client-pair" => {
                actual
                    && (expected.is_some()
                        || (side == "actual_after"
                            || change
                                .producer
                                .as_ref()
                                .and_then(|slots| slots.reason.as_deref())
                                != Some("no_first_parent")
                                && change
                                    .consumer
                                    .as_ref()
                                    .and_then(|slots| slots.reason.as_deref())
                                    != Some("no_first_parent")))
            }
            "before" | "client-before" => side == "actual_before",
            "after" | "client-after" => side == "actual_after",
            version => {
                version == side || actual && expected.is_some_and(|(_, client)| version == client)
            }
        }
    })
    .collect()
}
