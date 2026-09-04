use crate::cpg_cache::{self, SKIP_POLICY_VERSION};
use crate::navigation::NavigationCallEdgeIndex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

// v1: initial cache (#148).
// v2: P3 R6MultiOwnerCandidate — Python/JS/TS/Tsx unknown-receiver multi-owner
// collisions resolve to a capped NameOnly candidate edge instead of a drop,
// changing which sites appear in incoming/outgoing indexes.
// v3: P5 — `callback_registration`/`func_value_field` nav edges added to
// `build_resolved_call_edges` (Go function-value callbacks).
// v4: P7 — `property_access` nav edges added to `build_resolved_call_edges`
// (Python `@property`/`@cached_property` access edges).
// v5: P4 — JS/TS export modeling changes which `import_member` call sites
// resolve (default exports, named-export-list renames, const-arrow exports,
// CommonJS exports, depth-2-bounded re-export chains), changing which sites
// appear in incoming/outgoing indexes.
// v6: P8 — Rust macro-argument call extraction mints new CallSite entries
// (assert!/vec!/format!/... argument calls) that resolve through the normal
// ladder, changing which sites appear in incoming/outgoing indexes.
// v7: P9 — `framework_entry` nav edges added to `build_resolved_call_edges`
// (Flask/FastAPI/Express route-registration edges).
// v8: P11 — Go receiver typing (S1 call-RHS/S2 nested-selector/S3
// package-var recoveries via the post-merge rematerialization pass, plus S4's
// additive embedded-interface Exact dispatch route) changes which Go call
// sites resolve and at what kind, changing nav topology.
// v9: P13 — Go package/build profile partitioning changes same-package
// free-function and receiver-fact resolution confidence/topology.
// v10: glob-anchor-fix — Rust `use super::*`/`crate::*`/`self::*` (an
// empty-path glob with a meaningful anchor) now resolves instead of
// poisoning, changing which Rust call sites resolve out of nested
// modules/blocks that relied on an anchor-only glob, changing nav topology.
// v11: fail-closed positional parameter slots and exact Level-3 callback
// identity change which synthetic callback edges enter the nav index.
// v12: Go pointer-embedded fields participate in embedding promotion / S2 / S4;
//      qualified and pointer-to-interface embedded targets fail closed (P9; one shipped transition on top of main's v11).
// v13: P10 clause-bearing Go owner identities and exact build-partition
// filtering change S2/S4/P5 resolved edge topology.
// v14: Go `testdata` exclusion and snapshot-bound, strict go.mod identity remove
// stale/invalid Exact edges; go.work and symlink-refused manifest topology now
// participates in the sidecar fingerprint (paired with CPG v45).
// v15: effective Go workspace/module/replacement identity changes interface
// dispatch topology (paired with CPG v46).
// v16: P17 proven Go concrete receivers route before the legacy bare-interface
// ladder. Value-rebinding shadow bail, effective-module qualified owner
// recovery, and new-recovery provenance are part of this one PR transition
// (paired with CPG v47).
// v17: alias-aware Go signature identity (paired with CPG v48).
// v18: deferred concrete promoted selectors consult the serialized snapshot;
// resolved targets change while CPG serialization remains v48.
// v19: receiver-origin prerequisite screening removes unproven Go receiver
// edges from the resolved navigation topology (paired with CPG v51).
// v20: package-variable receivers carry defining-file owners (paired with CPG v52).
// v21: admissible unshadowed caller-local receivers carry owners (paired with CPG v53).
// v22: recovered Go receivers without a proven owner drop terminally (paired with CPG v54).
// v23: #16 owner-proven R3 consult removes bare-interface fallback edges while
// preserving the existing func-value-field continuation (CPG remains v54).
// v24: Go B1 Level-3 callback edges and their source-callee identity enter the
// resolved navigation topology (paired with CPG v55).
// v25: Python unaliased dotted-module receiver ownership adds Exact edges
// (paired with CPG v56).
// v26: Python namespace-package submodule receiver ownership adds Exact edges
// (paired with CPG v57).
// v27: JS/TS lexical receiver bindings remove shadowed imported-module Exact
// edges (paired with CPG v58).
const NAV_CALL_EDGE_CACHE_VERSION: u32 = 27;
const CACHE_BIN: &str = "resolved-call-edge-index.bin";
const CACHE_META: &str = "resolved-call-edge-index-meta.json";
const LOAD_DIRTY_OVERRIDE: &str = "PRISM_NAV_EDGE_CACHE_LOAD_DIRTY";

static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct NavigationCallEdgeCacheFingerprint {
    cache_build_identity: String,
    file_hashes_digest: String,
    topology_key_digest: String,
    has_type_db: bool,
}

impl NavigationCallEdgeCacheFingerprint {
    pub(crate) fn current(
        file_hashes: &BTreeMap<String, String>,
        topology_key: &BTreeMap<String, String>,
        has_type_db: bool,
    ) -> Self {
        Self {
            cache_build_identity: cpg_cache::current_cache_build_identity().to_string(),
            file_hashes_digest: digest_map(file_hashes),
            topology_key_digest: digest_map(topology_key),
            has_type_db,
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
struct NavigationCallEdgeCache {
    nav_call_edge_cache_version: u32,
    prism_version: String,
    grammar_fingerprint: String,
    skip_policy_version: u32,
    fingerprint: NavigationCallEdgeCacheFingerprint,
    git_sha: String,
    index: NavigationCallEdgeIndex,
}

#[derive(Debug, Serialize)]
struct NavigationCallEdgeCacheMeta {
    prism_version: String,
    nav_call_edge_cache_version: u32,
    cache_build_identity: String,
    git_sha: String,
    cache_size_bytes: u64,
    outgoing_callers: usize,
    incoming_targets: usize,
    collision_sites: usize,
    created: String,
}

#[derive(Debug, Clone)]
pub(crate) struct NavigationCallEdgeCacheStore {
    cache_dir: PathBuf,
    fingerprint: NavigationCallEdgeCacheFingerprint,
    cpg_cache_hit: bool,
}

impl NavigationCallEdgeCacheStore {
    pub(crate) fn new(
        cache_dir: &Path,
        file_hashes: &BTreeMap<String, String>,
        topology_key: &BTreeMap<String, String>,
        has_type_db: bool,
        cpg_cache_hit: bool,
    ) -> Self {
        Self {
            cache_dir: cache_dir.to_path_buf(),
            fingerprint: NavigationCallEdgeCacheFingerprint::current(
                file_hashes,
                topology_key,
                has_type_db,
            ),
            cpg_cache_hit,
        }
    }

    pub(crate) fn load_best_effort(&self) -> Option<NavigationCallEdgeIndex> {
        if !self.cpg_cache_hit {
            return None;
        }
        let binary_input_dirty = cpg_cache::binary_input_dirty();
        if binary_input_dirty && !dirty_load_override_enabled() {
            eprintln!(
                "nav edge cache: skipping warm sidecar load because prism was built from dirty binary inputs; recomputing resolved call edges"
            );
            return None;
        }
        if binary_input_dirty {
            eprintln!(
                "nav edge cache: {LOAD_DIRTY_OVERRIDE}=1 set; loading sidecar from dirty binary-input build may serve stale resolved edges"
            );
        }
        match load(&self.cache_dir, &self.fingerprint) {
            Ok(Some(index)) => Some(index),
            Ok(None) => None,
            Err(e) => {
                eprintln!(
                    "nav edge cache: failed to load sidecar in {}: {e}; rebuilding",
                    self.cache_dir.display()
                );
                None
            }
        }
    }

    pub(crate) fn save_best_effort(&self, index: &NavigationCallEdgeIndex) {
        if let Err(e) = save(&self.cache_dir, &self.fingerprint, index) {
            eprintln!(
                "nav edge cache: failed to save sidecar in {}: {e}",
                self.cache_dir.display()
            );
        }
    }
}

fn dirty_load_override_enabled() -> bool {
    matches!(std::env::var(LOAD_DIRTY_OVERRIDE), Ok(value) if value == "1")
}

fn load(
    cache_dir: &Path,
    fingerprint: &NavigationCallEdgeCacheFingerprint,
) -> io::Result<Option<NavigationCallEdgeIndex>> {
    let data = match std::fs::read(cache_dir.join(CACHE_BIN)) {
        Ok(data) => data,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e),
    };
    let cache: NavigationCallEdgeCache = match bincode::deserialize(&data) {
        Ok(cache) => cache,
        Err(_) => return Ok(None),
    };
    if cache.nav_call_edge_cache_version != NAV_CALL_EDGE_CACHE_VERSION
        || cache.prism_version != env!("CARGO_PKG_VERSION")
        || cache.grammar_fingerprint != env!("GRAMMAR_FINGERPRINT")
        || cache.skip_policy_version != SKIP_POLICY_VERSION
        || cache.fingerprint != *fingerprint
    {
        return Ok(None);
    }
    Ok(Some(cache.index))
}

fn save(
    cache_dir: &Path,
    fingerprint: &NavigationCallEdgeCacheFingerprint,
    index: &NavigationCallEdgeIndex,
) -> io::Result<()> {
    std::fs::create_dir_all(cache_dir)?;
    let cache = NavigationCallEdgeCache {
        nav_call_edge_cache_version: NAV_CALL_EDGE_CACHE_VERSION,
        prism_version: env!("CARGO_PKG_VERSION").to_string(),
        grammar_fingerprint: env!("GRAMMAR_FINGERPRINT").to_string(),
        skip_policy_version: SKIP_POLICY_VERSION,
        fingerprint: fingerprint.clone(),
        git_sha: env!("GIT_SHA").to_string(),
        index: index.clone(),
    };
    let encoded = bincode::serialize(&cache).map_err(|e| {
        io::Error::new(
            io::ErrorKind::Other,
            format!("bincode serialize error: {e}"),
        )
    })?;
    let bin_path = cache_dir.join(CACHE_BIN);
    let tmp_path = unique_tmp_path(cache_dir);
    std::fs::write(&tmp_path, &encoded)?;
    std::fs::rename(&tmp_path, &bin_path)?;

    let meta = NavigationCallEdgeCacheMeta {
        prism_version: cache.prism_version,
        nav_call_edge_cache_version: cache.nav_call_edge_cache_version,
        cache_build_identity: fingerprint.cache_build_identity.clone(),
        git_sha: cache.git_sha,
        cache_size_bytes: encoded.len() as u64,
        outgoing_callers: cache.index.outgoing_by_caller.len(),
        incoming_targets: cache.index.incoming_by_target.len(),
        collision_sites: cache.index.multi_owner_collision_sites.len(),
        created: chrono_free_timestamp(),
    };
    let meta_json = serde_json::to_string_pretty(&meta).unwrap_or_default();
    let _ = std::fs::write(cache_dir.join(CACHE_META), meta_json);
    Ok(())
}

fn unique_tmp_path(cache_dir: &Path) -> PathBuf {
    let nonce = TMP_COUNTER.fetch_add(1, Ordering::Relaxed);
    let pid = std::process::id();
    cache_dir.join(format!("{CACHE_BIN}.{pid}.{nonce}.tmp"))
}

fn digest_map(values: &BTreeMap<String, String>) -> String {
    let mut hasher = Sha256::new();
    for (key, value) in values {
        hasher.update(key.as_bytes());
        hasher.update([0]);
        hasher.update(value.as_bytes());
        hasher.update([0xff]);
    }
    format!("{:x}", hasher.finalize())
}

fn chrono_free_timestamp() -> String {
    use std::time::SystemTime;
    match SystemTime::now().duration_since(SystemTime::UNIX_EPOCH) {
        Ok(d) => format!("unix:{}", d.as_secs()),
        Err(_) => "unknown".to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::call_graph::{CallKind, CallSiteOrigin, FunctionId};
    use crate::navigation::{
        CallSiteKey, IndexedIncomingCall, IndexedOutgoingCallSite, IndexedResolvedTarget,
    };
    use crate::resolution::{ResolutionConfidence, ResolutionKind};
    use std::sync::{Mutex, OnceLock};

    static ENV_LOCK: OnceLock<Mutex<()>> = OnceLock::new();

    fn fid(file: &str, name: &str, line: usize) -> FunctionId {
        FunctionId {
            file: file.into(),
            name: name.into(),
            start_line: line,
            end_line: line + 1,
        }
    }

    fn env_lock() -> std::sync::MutexGuard<'static, ()> {
        ENV_LOCK.get_or_init(|| Mutex::new(())).lock().unwrap()
    }

    fn fixture_index() -> (NavigationCallEdgeIndex, FunctionId) {
        let target = fid("api.py", "target", 1);
        let caller = fid("app.py", "run", 3);
        let collision_caller = caller.clone();
        let incoming = IndexedIncomingCall {
            caller: caller.clone(),
            callee_name: "target".into(),
            call_site_line: 4,
            start_byte: 10,
            end_byte: 18,
            qualifier: None,
            confidence: ResolutionConfidence::Exact,
            kind: ResolutionKind::ImportMember,
        };
        let mut index = NavigationCallEdgeIndex::default();
        index
            .incoming_by_target
            .entry(target.clone())
            .or_default()
            .extend([incoming.clone(), incoming]);
        index
            .outgoing_by_caller
            .entry(caller)
            .or_default()
            .push(IndexedOutgoingCallSite {
                callee_name: "target".into(),
                call_site_line: 4,
                start_byte: 10,
                end_byte: 18,
                qualifier: None,
                resolved: vec![IndexedResolvedTarget {
                    target: target.clone(),
                    confidence: ResolutionConfidence::Exact,
                    kind: ResolutionKind::ImportMember,
                }],
            });
        index.multi_owner_collision_sites.insert(CallSiteKey {
            caller: collision_caller,
            callee_name: "target".into(),
            source_callee_name: Some("cb".into()),
            line: 4,
            kind: CallKind::Call,
            start_byte: 10,
            end_byte: 18,
            qualifier: None,
            receiver_type: None,
            receiver_owner_identity: None,
            receiver_recovery: None,
            receiver_materialized: false,
            receiver_newly_recovered: false,
            arg_count: None,
            arg_spread: false,
            receiver_outcome: None,
            origin: CallSiteOrigin::Source,
            pre_resolved_target: Some(target.clone()),
        });
        (index, target)
    }

    fn fixture_fingerprint() -> NavigationCallEdgeCacheFingerprint {
        let mut file_hashes = BTreeMap::new();
        file_hashes.insert("a.py".to_string(), "h-a".to_string());
        let topology = crate::cpg_cache::compute_topology_key(&file_hashes, &BTreeMap::new());
        NavigationCallEdgeCacheFingerprint::current(&file_hashes, &topology, false)
    }

    fn rewrite_cache(dir: &Path, mutate: impl FnOnce(&mut NavigationCallEdgeCache)) {
        let bin_path = dir.join(CACHE_BIN);
        let data = std::fs::read(&bin_path).unwrap();
        let mut cache: NavigationCallEdgeCache = bincode::deserialize(&data).unwrap();
        mutate(&mut cache);
        std::fs::write(bin_path, bincode::serialize(&cache).unwrap()).unwrap();
    }

    #[test]
    fn sidecar_round_trip_preserves_duplicate_incoming_edges() {
        let dir = tempfile::tempdir().unwrap();
        let fingerprint = fixture_fingerprint();
        let (index, target) = fixture_index();
        save(dir.path(), &fingerprint, &index).unwrap();
        let loaded = load(dir.path(), &fingerprint).unwrap().unwrap();

        assert_eq!(loaded.incoming_by_target[&target].len(), 2);
        assert_eq!(loaded.outgoing_by_caller.len(), 1);
        assert_eq!(loaded.multi_owner_collision_sites.len(), 1);
        let key = loaded
            .multi_owner_collision_sites
            .first()
            .expect("round-tripped collision key");
        assert_eq!(key.source_callee_name.as_deref(), Some("cb"));
        assert_eq!(key.pre_resolved_target.as_ref(), Some(&target));
    }

    #[test]
    fn sidecar_round_trip_preserves_python_dotted_receiver_edge() {
        use crate::ast::ParsedFile;
        use crate::cpg::CpgContext;
        use crate::languages::Language::Python;

        let files = BTreeMap::from([
            (
                "app.py".to_string(),
                ParsedFile::parse(
                    "app.py",
                    "import pkg.models\ndef run(client: pkg.models.Client):\n    client.send()\n",
                    Python,
                )
                .unwrap(),
            ),
            (
                "pkg/models.py".to_string(),
                ParsedFile::parse(
                    "pkg/models.py",
                    "class Client:\n    def send(self):\n        pass\n",
                    Python,
                )
                .unwrap(),
            ),
        ]);
        let navigation =
            crate::navigation::NavigationIndex::from_ctx(CpgContext::build(&files, None));
        let index = navigation.build_resolved_call_edges();
        let target = index
            .incoming_by_target
            .keys()
            .find(|target| target.file == "pkg/models.py" && target.name == "send")
            .cloned()
            .expect("dotted receiver target");
        let incoming = &index.incoming_by_target[&target];
        assert_eq!(incoming.len(), 1);
        assert_eq!(incoming[0].confidence, ResolutionConfidence::Exact);
        assert_eq!(incoming[0].kind, ResolutionKind::TypedParam);

        let dir = tempfile::tempdir().unwrap();
        let fingerprint = fixture_fingerprint();
        save(dir.path(), &fingerprint, &index).unwrap();
        let loaded = load(dir.path(), &fingerprint).unwrap().unwrap();
        assert_eq!(loaded.incoming_by_target[&target].len(), 1);
        assert_eq!(
            loaded.incoming_by_target[&target][0].kind,
            ResolutionKind::TypedParam
        );
        assert_eq!(
            loaded.incoming_by_target[&target][0].confidence,
            ResolutionConfidence::Exact
        );
    }

    #[test]
    fn sidecar_round_trip_preserves_python_namespace_submodule_receiver_edge() {
        use crate::ast::ParsedFile;
        use crate::cpg::CpgContext;
        use crate::languages::Language::Python;

        let files = BTreeMap::from([
            (
                "app.py".to_string(),
                ParsedFile::parse(
                    "app.py",
                    "from pkg import models\ndef run(client: models.Client):\n    client.send()\n",
                    Python,
                )
                .unwrap(),
            ),
            (
                "pkg/models.py".to_string(),
                ParsedFile::parse(
                    "pkg/models.py",
                    "class Client:\n    def send(self):\n        pass\n",
                    Python,
                )
                .unwrap(),
            ),
        ]);
        let navigation =
            crate::navigation::NavigationIndex::from_ctx(CpgContext::build(&files, None));
        let index = navigation.build_resolved_call_edges();
        let target = index
            .incoming_by_target
            .keys()
            .find(|target| target.file == "pkg/models.py" && target.name == "send")
            .cloned()
            .expect("namespace submodule receiver target");
        let incoming = &index.incoming_by_target[&target];
        assert_eq!(incoming.len(), 1);
        assert_eq!(incoming[0].confidence, ResolutionConfidence::Exact);
        assert_eq!(incoming[0].kind, ResolutionKind::TypedParam);

        let dir = tempfile::tempdir().unwrap();
        let fingerprint = fixture_fingerprint();
        save(dir.path(), &fingerprint, &index).unwrap();
        let loaded = load(dir.path(), &fingerprint).unwrap().unwrap();
        assert_eq!(loaded.incoming_by_target[&target].len(), 1);
        assert_eq!(
            loaded.incoming_by_target[&target][0].kind,
            ResolutionKind::TypedParam
        );
        assert_eq!(
            loaded.incoming_by_target[&target][0].confidence,
            ResolutionConfidence::Exact
        );
    }

    #[test]
    fn sidecar_round_trip_preserves_js_ts_shadowed_receiver_edge_absence() {
        use crate::ast::ParsedFile;
        use crate::cpg::CpgContext;
        use crate::languages::Language::TypeScript;

        let files = BTreeMap::from([
            (
                "api.ts".to_string(),
                ParsedFile::parse("api.ts", "export function m() {}\n", TypeScript).unwrap(),
            ),
            (
                "svc.ts".to_string(),
                ParsedFile::parse(
                    "svc.ts",
                    "import api from './api';\nfunction shadow(api: object) { api.m(); }\nfunction visible(other: object) { api.m(); }\n",
                    TypeScript,
                )
                .unwrap(),
            ),
        ]);
        let navigation =
            crate::navigation::NavigationIndex::from_ctx(CpgContext::build(&files, None));
        let index = navigation.build_resolved_call_edges();
        let target = index
            .incoming_by_target
            .keys()
            .find(|target| target.file == "api.ts" && target.name == "m")
            .cloned()
            .expect("visible imported-module target");
        let incoming = &index.incoming_by_target[&target];
        assert_eq!(incoming.len(), 1);
        assert_eq!(incoming[0].caller.name, "visible");
        assert_eq!(incoming[0].kind, ResolutionKind::ImportQualified);

        let dir = tempfile::tempdir().unwrap();
        let fingerprint = fixture_fingerprint();
        save(dir.path(), &fingerprint, &index).unwrap();
        let loaded = load(dir.path(), &fingerprint).unwrap().unwrap();
        let loaded_incoming = &loaded.incoming_by_target[&target];
        assert_eq!(loaded_incoming.len(), 1);
        assert_eq!(loaded_incoming[0].caller.name, "visible");
        assert_eq!(loaded_incoming[0].kind, ResolutionKind::ImportQualified);
    }

    #[test]
    fn sidecar_round_trip_preserves_js_ts_recovered_receiver_edge() {
        use crate::ast::ParsedFile;
        use crate::cpg::CpgContext;
        use crate::languages::Language::TypeScript;

        let files = BTreeMap::from([(
            "svc.ts".to_string(),
            ParsedFile::parse(
                "svc.ts",
                "class Foo { m() {} }\nfunction run(x: Foo) { x.m(); }\n",
                TypeScript,
            )
            .unwrap(),
        )]);
        let navigation =
            crate::navigation::NavigationIndex::from_ctx(CpgContext::build(&files, None));
        let index = navigation.build_resolved_call_edges();
        let target = index
            .incoming_by_target
            .keys()
            .find(|target| target.file == "svc.ts" && target.name == "m")
            .cloned()
            .expect("typed receiver target");
        let incoming = &index.incoming_by_target[&target];
        assert_eq!(incoming.len(), 1);
        assert_eq!(incoming[0].kind, ResolutionKind::TypedParam);
        assert_eq!(incoming[0].confidence, ResolutionConfidence::Exact);

        let dir = tempfile::tempdir().unwrap();
        let fingerprint = fixture_fingerprint();
        save(dir.path(), &fingerprint, &index).unwrap();
        let loaded = load(dir.path(), &fingerprint).unwrap().unwrap();
        assert_eq!(loaded.incoming_by_target[&target][0].kind, ResolutionKind::TypedParam);
        assert_eq!(
            loaded.incoming_by_target[&target][0].confidence,
            ResolutionConfidence::Exact
        );
    }

    #[test]
    fn sidecar_version_is_pinned_for_js_ts_lexical_receiver_binding() {
        assert_eq!(NAV_CALL_EDGE_CACHE_VERSION, 28);
    }

    #[test]
    fn dirty_override_requires_exact_one() {
        let _guard = env_lock();
        std::env::remove_var(LOAD_DIRTY_OVERRIDE);
        assert!(!dirty_load_override_enabled());
        std::env::set_var(LOAD_DIRTY_OVERRIDE, "");
        assert!(!dirty_load_override_enabled());
        std::env::set_var(LOAD_DIRTY_OVERRIDE, "0");
        assert!(!dirty_load_override_enabled());
        std::env::set_var(LOAD_DIRTY_OVERRIDE, "1");
        assert!(dirty_load_override_enabled());
        std::env::remove_var(LOAD_DIRTY_OVERRIDE);
    }

    #[test]
    fn store_load_respects_dirty_gate_and_exact_override() {
        let _guard = env_lock();
        let dir = tempfile::tempdir().unwrap();
        let fingerprint = fixture_fingerprint();
        let (index, _) = fixture_index();
        save(dir.path(), &fingerprint, &index).unwrap();
        let store = NavigationCallEdgeCacheStore {
            cache_dir: dir.path().to_path_buf(),
            fingerprint,
            cpg_cache_hit: true,
        };

        std::env::remove_var(LOAD_DIRTY_OVERRIDE);
        if cpg_cache::binary_input_dirty() {
            assert!(store.load_best_effort().is_none());
            std::env::set_var(LOAD_DIRTY_OVERRIDE, "0");
            assert!(store.load_best_effort().is_none());
            std::env::set_var(LOAD_DIRTY_OVERRIDE, "1");
            assert!(store.load_best_effort().is_some());
        } else {
            assert!(store.load_best_effort().is_some());
        }
        std::env::remove_var(LOAD_DIRTY_OVERRIDE);
    }

    #[test]
    fn sidecar_fingerprint_mismatches_miss() {
        let dir = tempfile::tempdir().unwrap();
        let fingerprint = fixture_fingerprint();
        let (index, _) = fixture_index();
        save(dir.path(), &fingerprint, &index).unwrap();

        let mut bad_build = fingerprint.clone();
        bad_build.cache_build_identity = "stale-binary-inputs".into();
        assert!(load(dir.path(), &bad_build).unwrap().is_none());

        let mut bad_file_hash = fingerprint.clone();
        bad_file_hash.file_hashes_digest = digest_map(&BTreeMap::from([(
            "a.py".to_string(),
            "changed".to_string(),
        )]));
        assert!(load(dir.path(), &bad_file_hash).unwrap().is_none());

        let mut bad_topology = fingerprint.clone();
        bad_topology.topology_key_digest = digest_map(&BTreeMap::from([(
            "manifest:Cargo.toml".to_string(),
            "changed".to_string(),
        )]));
        assert!(load(dir.path(), &bad_topology).unwrap().is_none());

        let mut bad_type_db = fingerprint;
        bad_type_db.has_type_db = true;
        assert!(load(dir.path(), &bad_type_db).unwrap().is_none());
    }

    #[test]
    fn sidecar_metadata_mismatches_miss() {
        let fingerprint = fixture_fingerprint();
        let (index, _) = fixture_index();
        for mutate in [
            |cache: &mut NavigationCallEdgeCache| cache.nav_call_edge_cache_version += 1,
            |cache: &mut NavigationCallEdgeCache| cache.prism_version = "stale".into(),
            |cache: &mut NavigationCallEdgeCache| cache.grammar_fingerprint = "stale".into(),
            |cache: &mut NavigationCallEdgeCache| cache.skip_policy_version += 1,
        ] {
            let dir = tempfile::tempdir().unwrap();
            save(dir.path(), &fingerprint, &index).unwrap();
            rewrite_cache(dir.path(), mutate);
            assert!(load(dir.path(), &fingerprint).unwrap().is_none());
        }
    }

    #[test]
    fn corrupt_sidecar_file_is_cache_miss() {
        let dir = tempfile::tempdir().unwrap();
        let fingerprint = fixture_fingerprint();
        std::fs::create_dir_all(dir.path()).unwrap();
        std::fs::write(dir.path().join(CACHE_BIN), b"not bincode").unwrap();

        assert!(load(dir.path(), &fingerprint).unwrap().is_none());
    }
}
