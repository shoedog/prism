//! Assembly-local visibility cache. No path/profile facts survive their graph.
use crate::go_build_profile::{self, GoBuildProfile};
use std::collections::{hash_map::DefaultHasher, BTreeMap};
use std::hash::{Hash, Hasher};
use std::sync::{Arc, Mutex, OnceLock};

#[cfg(test)]
mod tests;

type Answer = (bool, bool);
type Key = (usize, usize, bool);
type Shard = Mutex<BTreeMap<Key, Arc<OnceLock<Answer>>>>;
const SHARDS: usize = 16;
// Bound retained pair state even when every file has a distinct build profile.
const ENTRIES_PER_SHARD: usize = 4096;

struct FileFacts<'a> {
    dir: &'a str,
    profile: &'a GoBuildProfile,
    profile_id: usize,
}

pub(crate) struct VisibilityMemo<'a> {
    files: BTreeMap<&'a str, FileFacts<'a>>,
    entries: [Shard; SHARDS],
    capacity: usize,
    #[cfg(test)]
    computations: std::sync::atomic::AtomicUsize,
}

impl<'a> VisibilityMemo<'a> {
    pub(crate) fn new(profiles: &'a BTreeMap<String, GoBuildProfile>) -> Self {
        let mut ids = BTreeMap::new();
        let files = profiles
            .iter()
            .map(|(file, profile)| {
                let next = ids.len();
                let profile_id = *ids.entry(profile).or_insert(next);
                (
                    file.as_str(),
                    FileFacts {
                        dir: crate::resolution::dir_of(file),
                        profile,
                        profile_id,
                    },
                )
            })
            .collect();
        Self {
            files,
            entries: std::array::from_fn(|_| Mutex::new(BTreeMap::new())),
            capacity: ENTRIES_PER_SHARD,
            #[cfg(test)]
            computations: std::sync::atomic::AtomicUsize::new(0),
        }
    }

    pub(crate) fn for_caller(&self, file: &str) -> CallerVisibility<'_, 'a> {
        CallerVisibility {
            memo: self,
            caller: self.files.get(file),
        }
    }
}

pub(crate) struct CallerVisibility<'m, 'a> {
    memo: &'m VisibilityMemo<'a>,
    caller: Option<&'m FileFacts<'a>>,
}

impl CallerVisibility<'_, '_> {
    pub(crate) fn get(&self, defining_file: &str) -> Answer {
        let (Some(caller), Some(defining)) = (self.caller, self.memo.files.get(defining_file))
        else {
            return (true, false);
        };
        let owns_test_namespace = caller.dir == defining.dir
            && caller.profile.package_clause == defining.profile.package_clause;
        // Exact key proof: the original helper reads only the two complete
        // profiles and this path-derived boolean. SAT ignores package clauses;
        // namespace ownership matters only for a test declaration. Keep it in
        // the key unconditionally; no assumptions about cgo/vendor/internal.
        let key = (caller.profile_id, defining.profile_id, owns_test_namespace);
        let mut hash = DefaultHasher::new();
        key.hash(&mut hash);
        let entry = {
            let mut shard = self.memo.entries[hash.finish() as usize % SHARDS]
                .lock()
                .expect("Go visibility memo lock");
            if let Some(entry) = shard.get(&key) {
                Some(entry.clone())
            } else if shard.len() < self.memo.capacity {
                let entry = Arc::new(OnceLock::new());
                shard.insert(key, entry.clone());
                Some(entry)
            } else {
                None
            }
        };
        let compute = || {
            #[cfg(test)]
            self.memo
                .computations
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            from_profiles(caller.profile, defining.profile, owns_test_namespace)
        };
        match entry {
            Some(entry) => *entry.get_or_init(compute),
            None => compute(),
        }
    }
}

pub(super) fn uncached(
    caller_file: &str,
    defining_file: &str,
    profiles: &BTreeMap<String, GoBuildProfile>,
) -> Answer {
    let (Some(caller), Some(defining)) = (profiles.get(caller_file), profiles.get(defining_file))
    else {
        return (true, false);
    };
    let owns_test_namespace = crate::resolution::dir_of(caller_file)
        == crate::resolution::dir_of(defining_file)
        && caller.package_clause == defining.package_clause;
    from_profiles(caller, defining, owns_test_namespace)
}

fn from_profiles(
    caller: &GoBuildProfile,
    defining: &GoBuildProfile,
    owns_test_namespace: bool,
) -> Answer {
    if defining.is_test_file && !owns_test_namespace {
        return (false, true);
    }
    let visibility =
        go_build_profile::go_visible_with_package(caller, defining, &defining.package_clause);
    let exact = visibility.visible
        && go_build_profile::profile_allows_exact(Some(caller))
        && go_build_profile::visibility_allows_exact(Some(defining), &visibility);
    (visibility.visible, exact)
}
