use prism::api::ReviewOccurrenceFacts;
use prism::call_graph::CallSite;
use prism::data_flow::{VarAccessKind, VarLocation};

pub(super) fn matches_argument(
    facts: &ReviewOccurrenceFacts<'_>,
    call: &CallSite,
    value: &VarLocation,
) -> bool {
    if value.start_byte >= value.end_byte
        || value.kind != VarAccessKind::Use
        || value.file != facts.file()
        || call.caller.file != facts.file()
        || call.jsx_element
    {
        return false;
    }
    let Some(site) = facts.call(call.start_byte..call.end_byte) else {
        return false;
    };
    let Some(owner) = site.owner.as_ref().filter(|owner| owner.unique) else {
        return false;
    };
    let Some(function) = &owner.function else {
        return false;
    };
    if function != &call.caller
        || value.function != function.name
        || value.function_start_line != function.start_line
        || site.line != call.line
        || site.callee
            != call
                .source_callee_name
                .as_deref()
                .unwrap_or(&call.callee_name)
        || !site
            .arguments
            .iter()
            .any(|arg| arg.start <= value.start_byte && value.end_byte <= arg.end)
    {
        return false;
    }
    facts
        .values(value.start_byte..value.end_byte)
        .iter()
        .any(|fact| {
            fact.semantic_value
                && fact.owner.as_ref() == Some(owner)
                && fact.occurrence.path == value.path
                && fact.occurrence.line == value.line
        })
}

#[cfg(test)]
#[path = "occurrences_tests.rs"]
mod tests;
