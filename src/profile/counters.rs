//! Caller-defined scalar counters in the existing process-wide profile session.
use serde::Serialize;
use std::collections::BTreeMap;

pub(super) type Counters = BTreeMap<&'static str, u64>;

/// Add a scalar to the active session, saturating at u64::MAX.
/// Join worker tasks before dropping the process-wide Session, as with phases.
/// With no active session this performs one atomic load and returns.
pub fn counter(name: &'static str, value: u64) {
    if !super::enabled() {
        return;
    }
    let Ok(mut slot) = super::collector().lock() else {
        return;
    };
    if let Some(profile) = slot.as_mut() {
        let total = profile.counters.entry(name).or_default();
        *total = total.saturating_add(value);
    }
}

#[derive(Serialize)]
pub(super) struct CounterRow {
    record: &'static str,
    name: &'static str,
    value: u64,
}

pub(super) fn rows(counters: &Counters) -> impl Iterator<Item = CounterRow> + '_ {
    counters.iter().map(|(&name, &value)| CounterRow {
        record: "counter",
        name,
        value,
    })
}

#[cfg(test)]
#[path = "counters_tests.rs"]
mod tests;
