
#[cfg(test)]
mod reviewer_genuine_v52_probe {
    use super::*;
    #[test]
    fn reviewer_genuine_navigation_v52_version_rejected() {
        let dir=std::path::Path::new("/private/tmp/prism-post316-orchestration/genuine-old-cache-pristine/prism/nav/f4503058c1f93d610b2749f25185af495a8dd9345f9239ef373d07ce72ce233c");
        let bytes=std::fs::read(dir.join(CACHE_BIN)).unwrap();
        let old:NavigationCallEdgeCache=bincode::deserialize(&bytes).unwrap();
        assert_eq!(old.nav_call_edge_cache_version,52);
        assert_eq!(NAV_CALL_EDGE_CACHE_VERSION,53);
        assert_eq!(old.prism_version,env!("CARGO_PKG_VERSION"));
        assert_eq!(old.grammar_fingerprint,env!("GRAMMAR_FINGERPRINT"));
        assert_eq!(old.skip_policy_version,SKIP_POLICY_VERSION);
        assert!(!old.index.outgoing_by_caller.is_empty());
        // Preserve the exact embedded fingerprint so build identity is not a competing rejection cause.
        assert!(load(dir,&old.fingerprint).unwrap().is_none());
        println!("REVIEW_GENUINE_NAV bytes={} cached_version=52 current_version=53 other_predicates=equal loader=None outgoing={:?}",bytes.len(),old.index.outgoing_by_caller);
        assert_eq!(std::fs::read(dir.join(CACHE_BIN)).unwrap(),bytes);
    }
}
