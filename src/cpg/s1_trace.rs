//! Opt-in, byte-keyed observations for the S1 downstream differential.

use super::{CpgEdge, CpgNode};
use crate::call_graph::{CallSite, FunctionId};
use petgraph::graph::{DiGraph, NodeIndex};
use petgraph::visit::EdgeRef;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::sync::OnceLock;

type Graph = DiGraph<CpgNode, CpgEdge>;
const PREFIX: &str = "PRISM_S1 ";

pub(crate) fn enabled() -> bool {
    #[cfg(test)]
    if capture::forced() {
        return true;
    }
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var("PRISM_S1_TRACE").as_deref() == Ok("1"))
}

fn emit(record: Value) {
    #[cfg(test)]
    if capture::offer(&record) {
        return;
    }
    eprintln!("{PREFIX}{record}");
}

pub(crate) fn report_final(cpg: &super::CodePropertyGraph) {
    if enabled() {
        emit(json!({
            "event": "final_return_flow_stats",
            "build": crate::cpg_cache::current_cache_build_identity(),
            "stats": cpg.return_flow_stats,
        }));
    }
}

pub(crate) fn report_projection_demotions(
    file: &str,
    evidence: &crate::data_flow::s1_evidence::FileEvidence,
) {
    for attempt in &evidence.attempts {
        if !attempt.projection_causes.is_empty() {
            emit(json!({
                "event": "projection_demotion",
                "file": file,
                "grammar": evidence.grammar,
                "source_digest": evidence.source_digest,
                "attempt": attempt,
            }));
        }
    }
}

fn reaching_defs(graph: &Graph, use_index: NodeIndex) -> Vec<Value> {
    graph
        .edges_directed(use_index, petgraph::Direction::Incoming)
        .filter_map(|edge| {
            let CpgEdge::DataFlow(label) = edge.weight() else {
                return None;
            };
            let def @ CpgNode::Variable {
                access: super::VarAccess::Def,
                ..
            } = &graph[edge.source()]
            else {
                return None;
            };
            Some(json!({ "def": def, "label": label }))
        })
        .collect()
}

pub(super) fn return_input(graph: &Graph, from: NodeIndex, to: NodeIndex) {
    emit(json!({
        "event": "return_input",
        "counter": "return_input_edges",
        "use": graph[from],
        "return_value": graph[to],
        "reaching_defs": reaching_defs(graph, from),
    }));
}

pub(super) fn argument(
    graph: &Graph,
    site: &CallSite,
    operand: &std::ops::Range<usize>,
    slot: usize,
    from: NodeIndex,
    to: NodeIndex,
) {
    emit(json!({
        "event": "ordinary_argument",
        "caller": site.caller,
        "call_bytes": [site.start_byte, site.end_byte],
        "argument_bytes": [operand.start, operand.end],
        "slot": slot,
        "use": graph[from],
        "callee_def": graph[to],
        "reaching_defs": reaching_defs(graph, from),
    }));
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub(crate) struct CallKey {
    caller: FunctionId,
    bytes: [usize; 2],
    callee: FunctionId,
}

#[derive(Serialize, Deserialize)]
struct Suppression {
    call: CallKey,
    reasons: Vec<String>,
}

fn baseline() -> Option<&'static BTreeMap<CallKey, Vec<String>>> {
    static BASELINE: OnceLock<Option<BTreeMap<CallKey, Vec<String>>>> = OnceLock::new();
    BASELINE
        .get_or_init(|| {
            let path = std::env::var_os("PRISM_S1_SUPPRESSION_BASE")?;
            match read_baseline(&path) {
                Ok(rows) => Some(rows),
                Err(error) => {
                    emit(json!({"event": "suppression_baseline_error", "error": error}));
                    None
                }
            }
        })
        .as_ref()
}

fn read_baseline(path: &std::ffi::OsStr) -> Result<BTreeMap<CallKey, Vec<String>>, String> {
    let input = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
    let mut rows = BTreeMap::new();
    for line in input.lines().filter_map(|line| line.strip_prefix(PREFIX)) {
        let row: Value = serde_json::from_str(line).map_err(|error| error.to_string())?;
        if row["event"] == "suppression_state" {
            let row: Suppression =
                serde_json::from_value(row).map_err(|error| error.to_string())?;
            if rows.insert(row.call, row.reasons).is_some() {
                return Err("duplicate suppression state for call".into());
            }
        }
    }
    Ok(rows)
}

pub(super) fn suppression(
    site: &CallSite,
    callee: &FunctionId,
    certified: bool,
    complete_returns: bool,
    all_rhs_uses_bound: bool,
) {
    suppression_against(
        baseline(),
        site,
        callee,
        certified,
        complete_returns,
        all_rhs_uses_bound,
    );
}

fn suppression_against(
    baseline: Option<&BTreeMap<CallKey, Vec<String>>>,
    site: &CallSite,
    callee: &FunctionId,
    certified: bool,
    complete_returns: bool,
    all_rhs_uses_bound: bool,
) {
    let call = CallKey {
        caller: site.caller.clone(),
        bytes: [site.start_byte, site.end_byte],
        callee: callee.clone(),
    };
    let flags = [
        ("return_flow_suppression_certified", certified),
        (
            "return_flow_suppression_void_incomplete_returns",
            !certified && !complete_returns,
        ),
        (
            "return_flow_suppression_void_unbound_uses",
            !certified && !all_rhs_uses_bound,
        ),
    ];
    let reasons: Vec<_> = flags
        .iter()
        .filter_map(|&(counter, active)| active.then_some(counter))
        .collect();
    emit(json!({"event": "suppression_state", "call": call, "reasons": reasons}));
    if let Some(base) = baseline {
        let previous = base.get(&call).map_or(&[][..], Vec::as_slice);
        for (counter, active) in flags {
            let was_active = previous.iter().any(|reason| reason == counter);
            if was_active != active {
                emit(json!({
                    "event": "suppression_transition",
                    "call": call,
                    "counter": counter,
                    "old_reason": previous,
                    "new_reason": reasons,
                    "delta": i8::from(active) - i8::from(was_active),
                }));
            }
        }
    }
}

/// Test seams: a process-wide forced `enabled()` and a capture sink. Process-wide
/// (not thread-local) because the DFG and Step 5c fact collection run on build-pool
/// workers. One holder at a time; concurrent tests may also deposit records while it
/// is held, so assertions filter by their own fixture's file names.
#[cfg(test)]
pub(crate) mod capture {
    use serde_json::Value;
    use std::sync::{Mutex, MutexGuard};

    static HOLDER: Mutex<()> = Mutex::new(());
    static SINK: Mutex<Option<Vec<Value>>> = Mutex::new(None);

    pub(super) fn forced() -> bool {
        SINK.lock().is_ok_and(|sink| sink.is_some())
    }

    pub(super) fn offer(record: &Value) -> bool {
        match SINK.lock().unwrap_or_else(|e| e.into_inner()).as_mut() {
            Some(sink) => {
                sink.push(record.clone());
                true
            }
            None => false,
        }
    }

    pub(crate) struct Capture(#[allow(dead_code)] MutexGuard<'static, ()>);

    pub(crate) fn start() -> Capture {
        let guard = HOLDER.lock().unwrap_or_else(|e| e.into_inner());
        *SINK.lock().unwrap_or_else(|e| e.into_inner()) = Some(Vec::new());
        Capture(guard)
    }

    /// Hold the capture lock with tracing off, so a control build is never traced.
    pub(crate) fn quiet() -> Capture {
        let guard = HOLDER.lock().unwrap_or_else(|e| e.into_inner());
        *SINK.lock().unwrap_or_else(|e| e.into_inner()) = None;
        Capture(guard)
    }

    impl Capture {
        pub(crate) fn events(&self) -> Vec<Value> {
            SINK.lock()
                .unwrap_or_else(|e| e.into_inner())
                .as_ref()
                .cloned()
                .unwrap_or_default()
        }
    }

    impl Drop for Capture {
        fn drop(&mut self) {
            *SINK.lock().unwrap_or_else(|e| e.into_inner()) = None;
        }
    }
}

#[cfg(test)]
pub(crate) fn suppression_for_test(
    baseline: Option<&BTreeMap<CallKey, Vec<String>>>,
    site: &CallSite,
    callee: &FunctionId,
    flags: (bool, bool, bool),
) {
    suppression_against(baseline, site, callee, flags.0, flags.1, flags.2);
}

#[cfg(test)]
pub(crate) fn read_baseline_for_test(
    path: &std::path::Path,
) -> Result<BTreeMap<CallKey, Vec<String>>, String> {
    read_baseline(path.as_os_str())
}
