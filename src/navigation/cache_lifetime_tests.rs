//! Test-only loader adapter: each real stale graph owns a lifetime witness.
//! The enum preserves the production match's temporary/drop semantics without
//! adding instrumentation to the graph types or to non-test builds.
use super::*;
use crate::call_graph::CallGraph;
use crate::cpg::CodePropertyGraph;
use crate::data_flow::DataFlowGraph;
use crate::navigation::{queries, NavigationSession};
use crate::repo_loader::load_repo;
use std::cell::RefCell;
use std::collections::BTreeMap;
use std::rc::{Rc, Weak};
use std::sync::Arc;

// Keep the real cache result's inline ownership shape for the lifetime check.
#[allow(clippy::large_enum_variant)]
pub(super) enum CacheResult {
    Hit(CodePropertyGraph),
    PartialHit {
        cached_call_graph: (CallGraph, Rc<()>),
        cached_dfg: (DataFlowGraph, Rc<()>),
    },
    Miss,
}

#[derive(Default)]
struct Probe {
    payloads: Option<[Weak<()>; 2]>,
    rebuilds: Vec<[bool; 2]>,
}

thread_local! {
    static PROBE: RefCell<Probe> = RefCell::new(Probe::default());
}

pub(super) fn load_cache_with_topology(
    hashes: &BTreeMap<String, String>,
    topology: &BTreeMap<String, String>,
    has_type_db: bool,
    cache_dir: &Path,
) -> CacheResult {
    match cpg_cache::load_cache_with_topology(hashes, topology, has_type_db, cache_dir) {
        cpg_cache::CacheResult::Hit(cpg) => CacheResult::Hit(cpg),
        cpg_cache::CacheResult::Miss => CacheResult::Miss,
        cpg_cache::CacheResult::PartialHit {
            cached_call_graph,
            cached_dfg,
            ..
        } => {
            let cg_owner = Rc::new(());
            let dfg_owner = Rc::new(());
            PROBE.with_borrow_mut(|probe| {
                probe.payloads = Some([Rc::downgrade(&cg_owner), Rc::downgrade(&dfg_owner)]);
            });
            CacheResult::PartialHit {
                cached_call_graph: (cached_call_graph, cg_owner),
                cached_dfg: (cached_dfg, dfg_owner),
            }
        }
    }
}

pub(super) fn record_rebuild_start() {
    PROBE.with_borrow_mut(|probe| {
        let payloads = probe.payloads.as_ref().expect("real PartialHit payloads");
        probe
            .rebuilds
            .push(payloads.each_ref().map(|weak| weak.upgrade().is_some()));
    });
}

fn wire_output(repo: &Arc<LoadedRepo>, index: NavigationIndex) -> Vec<u8> {
    let session = NavigationSession {
        repo: repo.clone(),
        index: Arc::new(index),
    };
    serde_json::to_vec(&[
        queries::callees(&session, Some("run"), None, None, 1).unwrap(),
        queries::callers(&session, Some("helper"), None, None, 1).unwrap(),
        queries::ego_graph(&session, Some("run"), None, None, 1, &["Call"]).unwrap(),
    ])
    .unwrap()
}

#[test]
fn partial_hit_drops_stale_payloads_before_rebuild() {
    PROBE.with_borrow_mut(|probe| *probe = Probe::default());
    let root = tempfile::tempdir().unwrap();
    let cache = tempfile::tempdir().unwrap();
    let path = root.path().join("a.py");
    std::fs::write(
        &path,
        "def helper():\n    return 1\n\ndef run():\n    return helper()\n",
    )
    .unwrap();
    let repo = Arc::new(load_repo(root.path()).unwrap());
    assert!(matches!(
        cpg_cache::load_cache(&repo.file_hashes, false, cache.path()),
        cpg_cache::CacheResult::Miss
    ));
    let miss = wire_output(&repo, NavigationIndex::build_cached_at(&repo, cache.path()));
    assert!(matches!(
        cpg_cache::load_cache(&repo.file_hashes, false, cache.path()),
        cpg_cache::CacheResult::Hit(_)
    ));
    let hit = wire_output(&repo, NavigationIndex::build_cached_at(&repo, cache.path()));
    assert_eq!(miss, hit);
    PROBE.with_borrow(|probe| {
        assert!(
            probe.payloads.is_none(),
            "Miss/Hit must not attach witnesses"
        );
        assert!(
            probe.rebuilds.is_empty(),
            "Miss/Hit must not trip the probe"
        );
    });

    std::fs::write(
        &path,
        "def helper():\n    return 2\n\ndef other():\n    return 3\n\ndef run():\n    return other()\n",
    )
    .unwrap();
    let edited = Arc::new(load_repo(root.path()).unwrap());
    assert_eq!(
        repo.file_hashes.keys().collect::<Vec<_>>(),
        edited.file_hashes.keys().collect::<Vec<_>>()
    );
    assert_ne!(repo.file_hashes, edited.file_hashes);
    let rebuilt = wire_output(
        &edited,
        NavigationIndex::build_cached_at(&edited, cache.path()),
    );
    PROBE.with_borrow(|probe| {
        assert_eq!(
            probe.rebuilds,
            vec![[false, false]],
            "stale CG and DFG must both be dropped before PartialHit cold rebuild"
        );
    });
    let fresh = wire_output(&edited, NavigationIndex::build(&edited));
    assert_eq!(
        rebuilt, fresh,
        "PartialHit and no-cache navigation bytes differ"
    );
    assert_ne!(rebuilt, miss, "fixture edit must change navigation output");
}
