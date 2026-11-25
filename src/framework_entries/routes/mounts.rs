//! Query-local route installation and mount composition.

mod events;
mod graph;

use super::*;

/// One native installation slot. Adapters split independently writable slots
/// and supply a proven setup order across files and invocations.
pub struct Event<K> {
    pub dispatcher: RouterId,
    /// Root whose setup executes this event, when separately proved.
    pub setup_root: Option<RouterId>,
    pub site: Site,
    /// Shared identifier for slots processed in one native call.
    pub call: Option<usize>,
    pub key: K,
    pub target: Target,
}

/// Indices refer to the input record arrays. A route seed describes its local
/// path and constraints, with `Full` indicating a literal path at this stage.
pub enum Target {
    Route(usize, Exposure),
    Mount(usize, MountEpoch),
    Internal,
}

#[derive(Clone, Copy)]
pub enum MountEpoch {
    /// Read the child at the parent's listing or capture epoch.
    Live,
    /// Read the child immediately before this mount event.
    Snapshot,
}

pub enum Interaction {
    Disjoint,
    CoInstall,
    Priority {
        methods: MethodSet,
        priority: Priority,
    },
    /// Replace the entire earlier native slot. The mask identifies the cut.
    Overwrite(MethodSet),
    Reject(MethodSet),
    Skip,
    Unknown {
        methods: MethodSet,
        keep_earlier: bool,
    },
}

pub enum Priority {
    Unknown,
    Earlier { entire: bool },
    Later { entire: bool },
}

pub trait Relation<K> {
    /// Prove the interaction between an existing slot and an attempted slot
    /// on the same dispatcher. Terminal outcomes apply to the attempted slot.
    fn interact(&self, earlier: &Event<K>, later: &Event<K>) -> Interaction;
    /// Apply native path/host/transform rules. Return false for a proved
    /// impossible chain. A preexisting suffix is only a known tail.
    fn join(&self, mount: &Mount, path: &mut Exposure) -> bool;
    fn path_tokens(&self, value: &str) -> Option<Vec<PathToken>>;
}

/// Compose records for already selected roots. Deferred setup contributes an
/// event at its proved activation point. An edge with no activation remains in
/// the mount inventory, and a declaration with no installation remains listed.
pub fn compose<K>(
    mut routes: Vec<Route>,
    mounts: Vec<Mount>,
    roots: &[RouterId],
    events: &[Event<K>],
    relation: &impl Relation<K>,
) -> RouteTable {
    let replay = events::replay(&mut routes, &mounts, roots, events, relation);
    graph::project(&mut routes, &mounts, roots, events, &replay, relation);
    RouteTable::new(routes, mounts, replay.diagnostics, replay.failures, [])
}
