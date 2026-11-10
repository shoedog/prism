use super::records::*;
use super::recovery::RecoveryContext;
use crate::algorithms::taint::TaintTrace;
use prism::{
    api::{ReviewBoundaryHints, ReviewImportHint, ReviewInputs},
    call_graph::{CallSite, CallSiteOrigin},
    cpg::CpgContext,
    data_flow::VarLocation,
    languages::Language,
};
use std::collections::BTreeMap;

/// Inputs, context and trace must belong to the same completed review run.
pub(crate) fn collect(
    inputs: &ReviewInputs,
    ctx: &CpgContext<'_>,
    trace: &TaintTrace,
) -> BoundaryCollection {
    // The current factory's sole error is the captured source-listing failure.
    let lookup = ReviewBoundaryHints::new(inputs)
        .map(|mut hints| move |site: &CallSite| hints.hint_for_call(site))
        .map_err(|error| error.to_string());
    collect_with(ctx, trace, lookup)
}
fn collect_with<F: FnMut(&CallSite) -> Option<ReviewImportHint>>(
    ctx: &CpgContext<'_>,
    trace: &TaintTrace,
    lookup: Result<F, String>,
) -> BoundaryCollection {
    let mut lookup = match lookup {
        Ok(lookup) => lookup,
        Err(message) => {
            return BoundaryCollection {
                availability: Availability::ListingUnavailable(message),
                records: vec![],
            }
        }
    };
    let contexts: BTreeMap<_, _> = ctx
        .files
        .iter()
        .map(|(file, parsed)| (file.as_str(), RecoveryContext::new(parsed)))
        .collect();
    let prepared: Vec<_> = trace
        .roots
        .iter()
        .map(|root| {
            contexts
                .get(root.location.file.as_str())
                .map(|context| context.prepare(root))
        })
        .collect();
    let mut values: BTreeMap<&str, Vec<&VarLocation>> = BTreeMap::new();
    for (root, prepared) in trace.roots.iter().zip(&prepared) {
        let reached = root
            .reached
            .iter()
            .enumerate()
            .filter_map(|(index, reached)| {
                prepared.as_ref().map_or_else(
                    || {
                        (reached.location.start_byte < reached.location.end_byte)
                            .then_some(&reached.location)
                    },
                    |prepared| prepared.value(index),
                )
            });
        for value in std::iter::once(&root.location).chain(reached) {
            values.entry(value.file.as_str()).or_default().push(value);
        }
    }
    let mut groups: BTreeMap<PhysicalSite, Vec<&CallSite>> = BTreeMap::new();
    for call in ctx.cpg.call_graph.calls.values().flatten() {
        groups
            .entry(PhysicalSite {
                caller: call.caller.clone(),
                start_byte: call.start_byte,
                end_byte: call.end_byte,
            })
            .or_default()
            .push(call);
    }
    let mut records = Vec::new();
    for (site, group) in groups {
        let Some(context) = contexts.get(site.caller.file.as_str()) else {
            continue;
        };
        let Some(values) = values.get(site.caller.file.as_str()) else {
            continue;
        };
        let qualifies = |call: &CallSite| {
            values
                .iter()
                .any(|value| context.matches_argument(call, value))
        };
        if !group.iter().any(|call| qualifies(call))
            || group.iter().any(|call| loaded_callable(ctx, call))
        {
            continue;
        }
        let ordinary = group.iter().copied().filter(|call| ordinary_source(call));
        let Some(first) = ordinary.clone().next() else {
            continue;
        };
        if !ordinary
            .clone()
            .all(|call| source_tuple(call) == source_tuple(first))
        {
            continue;
        }
        let Some(representative) = ordinary.into_iter().find(|call| qualifies(call)) else {
            continue;
        };
        // Each physical group is visited once, including a silent None lookup.
        let Some(hint) = lookup(representative) else {
            continue;
        };
        let language = ctx.files[&site.caller.file].language;
        let kind = match &hint {
            ReviewImportHint::InRepo { declaring, .. }
                if language == Language::Go && declaring.is_empty() =>
            {
                BoundaryKind::GoDeclarationUnknown
            }
            ReviewImportHint::InRepo { .. } => BoundaryKind::InRepo,
            ReviewImportHint::Unresolvable { .. } => BoundaryKind::Unresolved,
            _ => continue,
        };
        records.push(BoundaryRecord {
            site,
            line: representative.line,
            callee: representative.effective_source_callee_name().into(),
            language,
            hint,
            kind,
        });
    }
    BoundaryCollection {
        availability: Availability::Available,
        records,
    }
}
fn loaded_callable(ctx: &CpgContext<'_>, call: &CallSite) -> bool {
    ctx.cpg
        .call_graph
        .resolve_call_site_full(call)
        .resolved
        .iter()
        .any(|resolved| ctx.files.contains_key(&resolved.target.file))
}
fn ordinary_source(site: &CallSite) -> bool {
    site.origin == CallSiteOrigin::Source && site.source_callee_name.is_none()
}
fn source_tuple(site: &CallSite) -> (usize, &str, Option<&str>) {
    (site.line, &site.callee_name, site.qualifier.as_deref())
}
#[cfg(test)]
#[path = "collect_tests.rs"]
mod tests;
