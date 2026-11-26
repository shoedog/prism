//! Shared expectations for the S2 literal-props tests.

use super::build::{ExactVarIndex, PendingEdge};
use super::var_index::{LegacyVarKey, VarIndex};
use super::{CpgEdge, CpgNode};
use crate::ast::ParsedFile;
use petgraph::graph::{DiGraph, EdgeIndex, NodeIndex};
use serde_json::Value;
use std::cell::RefCell;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, Mutex};

type Graph = DiGraph<CpgNode, CpgEdge>;

pub(super) struct PreBinding {
    pub graph: Graph,
    pub lookup: BTreeMap<LegacyVarKey, NodeIndex>,
    pub live_lookup: VarIndex,
    pub exact: ExactVarIndex,
}

pub(super) struct GuardInput {
    pub graph: Graph,
    pub seeds: Vec<NodeIndex>,
    pub bare_literals: BTreeSet<EdgeIndex>,
    pub counters: BTreeMap<String, usize>,
}

#[derive(Debug, Eq, PartialEq)]
pub(super) struct LiteralAttempt {
    pub caller: crate::call_graph::FunctionId,
    pub bytes: (usize, usize),
    pub kind: crate::ast::js_ts::component_route::SiteKind,
    pub exact: bool,
    pub refusal: Option<&'static str>,
}

#[derive(Debug, Eq, PartialEq)]
pub(super) struct LiteralWitness {
    pub site: (usize, usize),
    pub operand: std::ops::Range<usize>,
    pub keys: Vec<String>,
    pub endpoints: (NodeIndex, NodeIndex),
    pub class: &'static str,
    pub container: bool,
}

#[derive(Default)]
pub(super) struct Assembly {
    pub pre_binding: Option<PreBinding>,
    pub pending: Vec<(PendingEdge, bool, bool)>,
    pub guard_input: Option<GuardInput>,
    pub literal_attempts: Vec<LiteralAttempt>,
    pub literal_witnesses: Vec<LiteralWitness>,
}

type Inputs = BTreeMap<String, ParsedFile>;

#[derive(Clone)]
pub(super) struct Observer(Arc<Mutex<Assembly>>);

thread_local! {
    static REQUEST: RefCell<Option<(usize, Observer)>> = const { RefCell::new(None) };
}

pub(super) fn capture<T: Send>(inputs: &Inputs, build: impl FnOnce() -> T + Send) -> (T, Assembly) {
    let input_id = std::ptr::from_ref(inputs).addr();
    crate::build_pool::install(move || {
        struct Reset(Option<(usize, Observer)>);
        impl Drop for Reset {
            fn drop(&mut self) {
                REQUEST.with(|slot| slot.replace(self.0.take()));
            }
        }
        let observer = Observer(Arc::default());
        let _reset = Reset(REQUEST.with(|slot| slot.replace(Some((input_id, observer.clone())))));
        let built = build();
        let observed = std::mem::take(&mut *observer.0.lock().unwrap());
        (built, observed)
    })
}

/// A Rayon worker may service a different test build while this one waits.
/// Only the assembly using the requested input map gets the observer.
pub(super) fn observer(inputs: &Inputs) -> Option<Observer> {
    let input_id = std::ptr::from_ref(inputs).addr();
    REQUEST.with(|slot| {
        slot.borrow()
            .as_ref()
            .filter(|(requested, _)| *requested == input_id)
            .map(|(_, observer)| observer.clone())
    })
}

impl Observer {
    pub(super) fn pre_binding(&self, graph: &Graph, lookup: &VarIndex, exact: &ExactVarIndex) {
        let mut capture = self.0.lock().unwrap();
        assert!(capture.pre_binding.is_none());
        capture.pre_binding = Some(PreBinding {
            graph: graph.clone(),
            lookup: lookup.to_map(graph),
            live_lookup: lookup.clone(),
            exact: exact.clone(),
        });
    }

    pub(super) fn pending(&self, edge: &PendingEdge, foreign_seed: bool, bare_literal: bool) {
        self.0
            .lock()
            .unwrap()
            .pending
            .push((edge.clone(), foreign_seed, bare_literal));
    }

    pub(super) fn literal_proofs(
        &self,
        attempts: Vec<LiteralAttempt>,
        witnesses: Vec<LiteralWitness>,
    ) {
        let mut capture = self.0.lock().unwrap();
        capture.literal_attempts = attempts;
        capture.literal_witnesses = witnesses;
    }

    pub(super) fn guard_input(
        &self,
        graph: &Graph,
        seeds: &[NodeIndex],
        bare_literals: &BTreeSet<EdgeIndex>,
        counters: &BTreeMap<String, usize>,
    ) {
        let mut capture = self.0.lock().unwrap();
        assert!(capture.guard_input.is_none());
        capture.guard_input = Some(GuardInput {
            graph: graph.clone(),
            seeds: seeds.to_vec(),
            bare_literals: bare_literals.clone(),
            counters: counters.clone(),
        });
    }
}

const PROP_COUNTER_KEYS: &[&str] = &[
    "sites_jsx",
    "sites_literal",
    "refused_not_exact_singleton",
    "refused_member_qualifier",
    "refused_member_not_export",
    "refused_target_identity",
    "refused_unknown_wrapper",
    "refused_pre_initializer_unproved",
    "refused_bare_formal_skipped",
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

#[test]
fn props_s2_capture_ignores_an_unrelated_assembly_on_the_same_worker() {
    use super::CodePropertyGraph;
    use crate::languages::Language;

    let inputs = |file: &str| {
        BTreeMap::from([(
            file.to_owned(),
            ParsedFile::parse(
                file,
                "function D({a}) { return use(a); }\nfunction App(x) { return D({a: x}); }",
                Language::JavaScript,
            )
            .unwrap(),
        )])
    };
    let requested = inputs("requested.js");
    let unrelated = inputs("unrelated.js");
    let (_, observed) = capture(&requested, || {
        CodePropertyGraph::build(&unrelated);
        CodePropertyGraph::build(&requested)
    });
    assert_eq!(observed.literal_attempts.len(), 1);
    assert_eq!(observed.literal_attempts[0].caller.file, "requested.js");
    assert!(observed.pre_binding.is_some());
    assert!(observed.guard_input.is_some());
}
