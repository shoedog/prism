//! Immutable, assembly-owned Go satisfier memo. No graph-persistent derived state.
use super::*;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex, OnceLock};

#[path = "resolution_go_satisfiers/compute.rs"]
mod compute;
#[cfg(test)]
#[path = "resolution_go_satisfiers/reference.rs"]
mod reference;
#[cfg(test)]
#[path = "resolution_go_satisfiers/tests.rs"]
mod tests;

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct GoCallerScopedSatisfiers<'a> {
    pub(super) interface_selected: bool,
    pub(super) selection:
        crate::go_owner_partition::GoPartitionSelection<Vec<(String, &'a FunctionId)>>,
    pub(super) promoted: GoProvenPromotedSupplyTelemetry,
}

type Key = (GoOwnerIdentity, String, String, bool);
type Entry<'a> = Arc<OnceLock<GoCallerScopedSatisfiers<'a>>>;
type Shard<'a> = Mutex<BTreeMap<Key, Entry<'a>>>;
const SHARDS: usize = 16;

pub(crate) struct GoSatisfierMemo<'a> {
    graph: &'a CallGraph,
    entries: [Shard<'a>; SHARDS],
    #[cfg(test)]
    computations: std::sync::atomic::AtomicUsize,
    #[cfg(test)]
    reference: bool,
    #[cfg(test)]
    phase: std::sync::atomic::AtomicUsize,
    #[cfg(test)]
    requests: [std::sync::atomic::AtomicUsize; 3],
}

impl<'a> GoSatisfierMemo<'a> {
    pub(crate) fn new(graph: &'a CallGraph) -> Self {
        Self {
            graph,
            entries: std::array::from_fn(|_| Mutex::new(BTreeMap::new())),
            #[cfg(test)]
            computations: std::sync::atomic::AtomicUsize::new(0),
            #[cfg(test)]
            reference: reference_enabled(),
            #[cfg(test)]
            phase: std::sync::atomic::AtomicUsize::new(0),
            #[cfg(test)]
            requests: std::array::from_fn(|_| std::sync::atomic::AtomicUsize::new(0)),
        }
    }

    pub(crate) fn graph(&self) -> &'a CallGraph {
        self.graph
    }

    #[cfg(test)]
    pub(crate) fn set_phase(&self, phase: usize) {
        self.phase
            .store(phase, std::sync::atomic::Ordering::Relaxed);
    }

    pub(super) fn get(
        &self,
        owner: &GoOwnerIdentity,
        method: &str,
        caller_file: &str,
        include_promoted: bool,
    ) -> GoCallerScopedSatisfiers<'a> {
        #[cfg(test)]
        self.requests[self.phase.load(std::sync::atomic::Ordering::Relaxed)]
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let key = (
            owner.clone(),
            method.to_string(),
            caller_file.to_string(),
            include_promoted,
        );
        let mut hasher = DefaultHasher::new();
        (
            &key.0.package_dir,
            &key.0.package_clause,
            &key.0.name,
            &key.1,
            &key.2,
            key.3,
        )
            .hash(&mut hasher);
        let shard = hasher.finish() as usize % SHARDS;
        let entry = self.entries[shard]
            .lock()
            .expect("Go satisfier memo lock")
            .entry(key)
            .or_default()
            .clone();
        // Release the shard lock before computing. Only identical keys wait on
        // one another; distinct keys can initialize concurrently, even in a shard.
        let result = entry.get_or_init(|| {
            #[cfg(test)]
            self.computations
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            self.graph.go_compute_caller_scoped_satisfiers(
                owner,
                method,
                caller_file,
                include_promoted,
            )
        });
        result.clone()
    }

    pub(crate) fn resolve_call_site_full(&self, site: &CallSite) -> ResolutionOutcome<'a> {
        #[cfg(test)]
        if self.reference {
            return with_reference(|| self.graph.resolve_call_site_full(site));
        }
        let outcome = self
            .graph
            .resolve_call_site_candidates_with_go_memo(site, Some(self));
        self.graph.admit_anonymous_exact(site, outcome)
    }
}

#[cfg(test)]
thread_local! {
    static REFERENCE: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

#[cfg(test)]
pub(super) fn reference_enabled() -> bool {
    REFERENCE.with(std::cell::Cell::get)
}

#[cfg(test)]
pub(crate) fn with_reference<T>(f: impl FnOnce() -> T) -> T {
    struct Reset(bool);
    impl Drop for Reset {
        fn drop(&mut self) {
            REFERENCE.with(|flag| flag.set(self.0));
        }
    }
    let _reset = Reset(REFERENCE.with(|flag| flag.replace(true)));
    f()
}

/// Shared CPG emission seam; the ordinary graph implementation keeps serial
/// reference helpers and their fanout filtering identical.
pub(crate) trait CpgCallResolver {
    fn resolve_call_site(&self, site: &CallSite) -> Vec<ResolvedCallee<'_>>;
}

impl CpgCallResolver for CallGraph {
    fn resolve_call_site(&self, site: &CallSite) -> Vec<ResolvedCallee<'_>> {
        CallGraph::resolve_call_site(self, site)
    }
}

impl CpgCallResolver for GoSatisfierMemo<'_> {
    fn resolve_call_site(&self, site: &CallSite) -> Vec<ResolvedCallee<'_>> {
        filter_func_value_fanout(self.resolve_call_site_full(site).resolved)
    }
}

#[cfg(test)]
#[derive(Debug, PartialEq, Eq)]
struct MemoObservation {
    computations: usize,
    requests: [usize; 3],
}

#[cfg(test)]
struct PhaseObservation {
    files: Vec<String>,
    counts: Vec<MemoObservation>,
}

#[cfg(test)]
thread_local! {
    static PHASE_COUNTS: std::cell::RefCell<Option<PhaseObservation>> = const { std::cell::RefCell::new(None) };
}

#[cfg(test)]
fn observe_phases<T>(files: Vec<String>, f: impl FnOnce() -> T) -> (T, Vec<MemoObservation>) {
    struct Reset(Option<PhaseObservation>);
    impl Drop for Reset {
        fn drop(&mut self) {
            PHASE_COUNTS.with(|slot| slot.replace(self.0.take()));
        }
    }
    let _reset = Reset(PHASE_COUNTS.with(|slot| {
        slot.replace(Some(PhaseObservation {
            files,
            counts: Vec::new(),
        }))
    }));
    let result = f();
    let counts = PHASE_COUNTS.with(|slot| slot.replace(None).unwrap().counts);
    (result, counts)
}

#[cfg(test)]
impl Drop for GoSatisfierMemo<'_> {
    fn drop(&mut self) {
        PHASE_COUNTS.with(|slot| {
            if let Some(observation) = slot.borrow_mut().as_mut() {
                if observation
                    .files
                    .iter()
                    .eq(self.graph.go_file_profiles.keys())
                {
                    observation.counts.push(MemoObservation {
                        computations: self.computations.load(std::sync::atomic::Ordering::Relaxed),
                        requests: std::array::from_fn(|i| {
                            self.requests[i].load(std::sync::atomic::Ordering::Relaxed)
                        }),
                    });
                }
            }
        });
    }
}
