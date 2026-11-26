//! Shared expectations for the S2 literal-props tests.

use super::build::{ExactVarIndex, PendingEdge};
use super::var_index::{LegacyVarKey, VarIndex};
use super::{CpgEdge, CpgNode};
use petgraph::graph::{DiGraph, EdgeIndex, NodeIndex};
use serde_json::Value;
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};

type Graph = DiGraph<CpgNode, CpgEdge>;

pub(super) struct PreBinding {
    pub graph: Graph,
    pub lookup: BTreeMap<LegacyVarKey, NodeIndex>,
    pub exact: ExactVarIndex,
}

pub(super) struct GuardInput {
    pub graph: Graph,
    pub seeds: Vec<NodeIndex>,
    pub bare_literals: BTreeSet<EdgeIndex>,
    pub counters: BTreeMap<String, usize>,
}

#[derive(Default)]
pub(super) struct Assembly {
    pub pre_binding: Option<PreBinding>,
    pub pending: Vec<(PendingEdge, bool, bool)>,
    pub guard_input: Option<GuardInput>,
}

thread_local! {
    static ASSEMBLY: RefCell<Option<Assembly>> = const { RefCell::new(None) };
}

pub(super) fn capture<T: Send>(build: impl FnOnce() -> T + Send) -> (T, Assembly) {
    crate::build_pool::install(move || {
        struct Reset(Option<Assembly>);
        impl Drop for Reset {
            fn drop(&mut self) {
                ASSEMBLY.with(|slot| slot.replace(self.0.take()));
            }
        }
        let _reset = Reset(ASSEMBLY.with(|slot| slot.replace(Some(Assembly::default()))));
        let built = build();
        let observed = ASSEMBLY.with(|slot| slot.take().unwrap());
        (built, observed)
    })
}

pub(super) fn observe_pre_binding(graph: &Graph, lookup: &VarIndex, exact: &ExactVarIndex) {
    ASSEMBLY.with(|slot| {
        if let Some(capture) = slot.borrow_mut().as_mut() {
            assert!(capture.pre_binding.is_none());
            capture.pre_binding = Some(PreBinding {
                graph: graph.clone(),
                lookup: lookup.to_map(graph),
                exact: exact.clone(),
            });
        }
    });
}

pub(super) fn observe_pending(edge: &PendingEdge, foreign_seed: bool, bare_literal: bool) {
    ASSEMBLY.with(|slot| {
        if let Some(capture) = slot.borrow_mut().as_mut() {
            capture
                .pending
                .push((edge.clone(), foreign_seed, bare_literal));
        }
    });
}

pub(super) fn observe_guard_input(
    graph: &Graph,
    seeds: &[NodeIndex],
    bare_literals: &BTreeSet<EdgeIndex>,
    counters: &BTreeMap<String, usize>,
) {
    ASSEMBLY.with(|slot| {
        if let Some(capture) = slot.borrow_mut().as_mut() {
            assert!(capture.guard_input.is_none());
            capture.guard_input = Some(GuardInput {
                graph: graph.clone(),
                seeds: seeds.to_vec(),
                bare_literals: bare_literals.clone(),
                counters: counters.clone(),
            });
        }
    });
}

const PROP_COUNTER_KEYS: &[&str] = &[
    "sites_jsx",
    "sites_literal",
    "refused_not_exact_singleton",
    "refused_member_qualifier",
    "refused_member_not_export",
    "refused_target_identity",
    "refused_unknown_wrapper",
    "refused_no_props_formal",
    "refused_alias_pattern",
    "refused_argument_after_spread",
    "refused_zero_width_read",
    "refused_bracket_read",
    "refused_unproved_read",
    "spread_sources_unbound",
    "refused_overwritten",
    "refused_value_not_data",
    "refused_key_unknown",
    "refused_key_not_read",
    "refused_source_ambiguous",
    "refused_source_unconnected",
    "refused_source_in_enclosing_owner",
    "refused_source_not_found",
    "edges_to_def",
    "edges_to_rest",
    "edges_to_use",
    "edges_to_bare_use",
    "edges_to_container",
];

pub(super) fn check_counter(stats: &Value, key: &str, count: u64) -> Result<(), String> {
    if !PROP_COUNTER_KEYS.contains(&key) {
        return Err(format!("unregistered props counter {key:?}"));
    }
    let map = stats["dfg_props"]
        .as_object()
        .ok_or("missing dfg_props map")?;
    let actual = match map.get(key) {
        Some(value) => value
            .as_u64()
            .ok_or_else(|| format!("invalid counter {key}: {value}"))?,
        None => 0,
    };
    if actual != count {
        return Err(format!("{key}: expected {count}, observed {actual}"));
    }
    Ok(())
}

#[test]
fn props_s2_counter_validation() {
    let empty = serde_json::json!({"dfg_props": {}});
    assert_eq!(check_counter(&empty, "edges_to_def", 0), Ok(()));
    assert!(check_counter(&empty, "edges_to_defs", 0).is_err());
    assert!(check_counter(&Value::Null, "edges_to_def", 0).is_err());
    let invalid = serde_json::json!({"dfg_props": {"edges_to_def": "0"}});
    assert!(check_counter(&invalid, "edges_to_def", 0).is_err());
}
