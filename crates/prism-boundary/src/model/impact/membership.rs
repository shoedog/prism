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
    for id in request.expected_inputs {
        if !seen.insert(id) {
            continue;
        }
        let mut records = evidence.iter().filter(|record| &record.input_id == id);
        let Some(record) = records
            .next()
            .filter(|record| records.all(|other| other == *record))
        else {
            reasons.insert(id, "missing_membership_evidence");
            continue;
        };
        if record.excluded || record.later {
            continue;
        }
        let mut bindings = request.admitted_bindings.iter().filter(|doc| {
            doc.input
                .as_ref()
                .is_some_and(|input| &input.input_id == id)
        });
        let Some(doc) = bindings.next().filter(|_| bindings.next().is_none()) else {
            reasons.insert(id, "missing_membership_evidence");
            continue;
        };
        let mut applicable = false;
        for (side, presence, element) in [
            (
                PopulationSide::Before,
                &change.before_presence,
                change.before_element_id.as_deref(),
            ),
            (
                PopulationSide::After,
                &change.after_presence,
                change.after_element_id.as_deref(),
            ),
        ] {
            if side != record.side && record.side != PopulationSide::Held {
                continue;
            }
            if presence == "absent" {
                continue;
            }
            applicable = true;
            if let Some(element) = element.filter(|_| presence == "present") {
                match membership(doc, &change.operation_id, element) {
                    Membership::Present => return json!({"unobserved":[],"coverage":[]}),
                    Membership::Possible(reason) => {
                        reasons.insert(id, reason);
                    }
                    Membership::Absent
                        if record.complete && covers(doc, &change.operation_id, element) => {}
                    Membership::Absent => {
                        reasons.insert(id, "missing_membership_evidence");
                    }
                }
            } else {
                reasons.insert(id, "membership_possible");
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

fn membership(doc: &BindingsDocument, operation_id: &str, element: &str) -> Membership {
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
            if operation_id == element || op.fields.iter().any(|field| field.field_id == element) {
                return Membership::Present;
            }
            if op
                .observed_fields
                .iter()
                .any(|field| field.field_id == element)
            {
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
        if bound.grade == "bound" {
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
        } else if contains(element)
            || contains(operation_id)
            || bound
                .overlaps
                .iter()
                .any(|overlap| overlap.element_id == element || overlap.element_id == operation_id)
        {
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
    doc.expected_files
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

impl ImpactDocument {
    pub fn with_usage_counts(mut self, bindings: &[BindingsDocument]) -> Self {
        let mut unmatched = BTreeSet::new();
        let mut unknown = BTreeSet::new();
        let mut complete = self.comparisons.iter().any(|pair| {
            matches!(
                pair.comparison_id.as_str(),
                "actual_before" | "actual_after"
            )
        });
        let mut target_checks_complete = true;
        for comparison in &mut self.comparisons {
            let client_input = self
                .inputs
                .iter()
                .find(|input| input.input_id == comparison.client_input_id);
            let mut candidates = bindings.iter().filter(|doc| {
                doc.input
                    .as_ref()
                    .zip(client_input)
                    .is_some_and(|(input, slot)| {
                        input.input_id == slot.input_id
                            && input.role == Role::Consumer
                            && input.api == slot.api
                            && input.repo == slot.repo
                            && input.snapshot.identity() == slot.snapshot.identity()
                    })
            });
            let calls = candidates
                .next()
                .filter(|_| candidates.next().is_none())
                .and_then(|doc| doc.calls.as_ref().map(|calls| (doc, calls)));
            let actual = matches!(
                comparison.comparison_id.as_str(),
                "actual_before" | "actual_after"
            );
            let Some((doc, calls)) = calls else {
                if actual {
                    complete = false;
                }
                continue;
            };
            if actual && !covers(doc, "", "") {
                complete = false;
            }

            let mut unchecked = BTreeSet::new();
            let mut seen = BTreeSet::new();
            for call in calls {
                if !seen.insert(&call.site_id) {
                    if actual {
                        complete = false;
                    }
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
                let key = (comparison.client_input_id.clone(), call.site_id.clone());
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
                    if actual {
                        unknown.insert(key);
                    }
                } else if outcome.is_none_or(|record| record.status != "checked") {
                    unchecked.insert(call.site_id.clone());
                    if actual {
                        target_checks_complete = false;
                    }
                } else if actual
                    && outcome.is_some_and(|record| {
                        record.grade.as_deref() == Some("unbound")
                            && record.reason.as_deref() == Some("no_operation")
                    })
                {
                    unmatched.insert(key);
                }
            }
            comparison.unchecked_count = unchecked.len();
        }

        for (name, observed, complete) in [
            (
                "known_unmatched_count",
                unmatched.len(),
                complete && target_checks_complete,
            ),
            ("unknown_target_count", unknown.len(), complete),
        ] {
            if let Some(metric) = self.summary.get_mut(name) {
                metric.observed = observed;
                metric.total = complete.then_some(observed);
                metric.reason = (!complete).then(|| "missing_relevant_population".into());
            }
        }
        self
    }
}
