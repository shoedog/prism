//! Attribution-mode lookup of a bare JS/TS specifier against the owning
//! tsconfig's `paths` and `baseUrl`. The strict `js_paths::Resolver` is a
//! proof used by call resolution: it refuses fallback arrays, a missing or
//! non-node `moduleResolution`, and `baseUrl` without `paths`. Attribution
//! wants candidates, not a proof, so this lookup accepts every shape TypeScript
//! accepts and reads only the captured config bytes (no probe is recorded in
//! the topology). `js_paths::Resolver` itself is unchanged.
//!
//! A config chain this lookup cannot read in full is `Alias::Unavailable`, not
//! "no alias": a relative `extends` parent the snapshot did not capture (it
//! keeps only tsconfig/jsconfig-named files), one outside the repo, an
//! unparseable file or an `extends` cycle. Parents merge field by field (a
//! later parent overrides only the fields it sets). A package `extends`
//! (`@tsconfig/...`) is outside the repo and ignored.
use crate::call_graph::js_ts_relative_module_candidates;
use crate::resolution::js_ts::paths_snapshot::JsPathsSnapshot;
use crate::resolution::js_ts::paths_syntax::{dir, jsonc, norm};
use crate::repo_loader::BUILTIN_SKIP_DIRS;
use std::collections::{BTreeMap, BTreeSet};
use std::rc::Rc;

/// What a bare specifier is, under the owning config.
#[derive(Debug, PartialEq, Eq)]
pub(super) enum Alias {
    /// No `paths` key (outside a catch-all `*`) or `baseUrl` file matches: a package.
    NotAlias,
    /// A `paths` key matched, but no listed file is behind it.
    Declared,
    /// Candidate files, in `paths` target order, then the `baseUrl` file.
    Files(Vec<String>),
    /// The owning config (or an `extends` parent) cannot be read in full, or an
    /// explicit `baseUrl` that `paths` targets hang off lies outside the repo.
    Unavailable,
}

/// The `baseUrl` an effective config has.
#[derive(Clone, Default, PartialEq, Eq)]
enum BaseUrl {
    /// No config in the chain sets one.
    #[default]
    Absent,
    /// A repo directory.
    Usable(String),
    /// Set, but it does not normalise to a repo directory (outside the root,
    /// absolute): no repo file lives there and nothing may stand in for it.
    Unusable,
}

#[derive(Clone, Default)]
struct AliasConfig {
    base: BaseUrl,
    /// Directory of the config that defined `paths`, and the entries in the
    /// order the JSON parser yields them (sorted by key: it keeps no file order).
    paths: Option<(String, Vec<(String, Vec<String>)>)>,
}

/// An owning-config lookup result.
#[derive(Clone)]
enum Owner {
    /// No tsconfig/jsconfig at or above the file.
    None,
    Config(Rc<AliasConfig>),
    Unavailable,
}

#[derive(Default)]
pub(super) struct AliasConfigs {
    loaded: BTreeMap<String, Option<Rc<AliasConfig>>>,
    owner: BTreeMap<String, Owner>,
}

impl AliasConfigs {
    pub(super) fn lookup(
        &mut self,
        snapshot: &JsPathsSnapshot,
        listing: &BTreeSet<String>,
        file: &str,
        spec: &str,
    ) -> Alias {
        if spec.is_empty() || spec.starts_with(['.', '/']) || spec.contains(['\\', ':']) {
            return Alias::NotAlias;
        }
        let config = match self.owner_of(snapshot, file) {
            Owner::None => return Alias::NotAlias,
            Owner::Unavailable => return Alias::Unavailable,
            Owner::Config(config) => config,
        };
        let mut found: Vec<String> = Vec::new();
        let mut declared = false;
        if let Some((origin, entries)) = &config.paths {
            let matched = match matching_key(entries, spec) {
                KeyMatch::Tied => return Alias::Unavailable,
                KeyMatch::None => None,
                KeyMatch::One(key, targets, capture) => Some((key, targets, capture)),
            };
            if let Some((key, targets, capture)) = matched {
                let base = match &config.base {
                    BaseUrl::Absent => origin.as_str(),
                    BaseUrl::Usable(base) => base.as_str(),
                    // The targets hang off a directory outside the repo.
                    BaseUrl::Unusable => return Alias::Unavailable,
                };
                let probed: Vec<Option<String>> = targets
                    .iter()
                    .map(|target| norm(base, &target.replacen('*', capture, 1)))
                    .collect();
                // A key that only reaches skipped directories (a `node_modules`
                // type stub) names no source file: not an alias.
                if probed.iter().flatten().any(|p| !in_skipped_dir(p)) {
                    declared = key != "*";
                    found.extend(probed.iter().flatten().filter_map(|p| winner(p, listing)));
                }
            }
        }
        if let BaseUrl::Usable(base) = &config.base {
            if let Some(path) = norm(base, spec).filter(|p| !in_skipped_dir(p)) {
                found.extend(winner(&path, listing));
            }
        }
        let mut seen = BTreeSet::new();
        found.retain(|f| seen.insert(f.clone()));
        if !found.is_empty() {
            Alias::Files(found)
        } else if declared {
            Alias::Declared
        } else {
            Alias::NotAlias
        }
    }

    /// The nearest `tsconfig.json` (then `jsconfig.json`) at or above `file`'s
    /// directory, with relative `extends` followed. No `include` check: this is
    /// candidate attribution, so a nearer config that lists the file elsewhere
    /// still owns it.
    fn owner_of(&mut self, snapshot: &JsPathsSnapshot, file: &str) -> Owner {
        if snapshot.configs.is_empty() {
            return Owner::None;
        }
        let start = dir(file).to_string();
        if let Some(found) = self.owner.get(&start) {
            return found.clone();
        }
        let mut directory = start.as_str();
        let result = loop {
            let found = ["tsconfig.json", "jsconfig.json"].iter().find_map(|name| {
                let path = if directory.is_empty() {
                    (*name).to_string()
                } else {
                    format!("{directory}/{name}")
                };
                snapshot.configs.get(&path)?.as_ref()?;
                Some(path)
            });
            if let Some(path) = found {
                break match self.load(snapshot, &path, &mut Vec::new()) {
                    Some(config) => Owner::Config(config),
                    None => Owner::Unavailable,
                };
            }
            if directory.is_empty() {
                break Owner::None;
            }
            directory = dir(directory);
        };
        self.owner.insert(start, result.clone());
        result
    }

    /// The effective config at `path`, or `None` when it or any relative
    /// `extends` ancestor cannot be read (not captured, unparseable, outside
    /// the repo, or a cycle: `visiting` is the chain being loaded).
    fn load(
        &mut self,
        snapshot: &JsPathsSnapshot,
        path: &str,
        visiting: &mut Vec<String>,
    ) -> Option<Rc<AliasConfig>> {
        if let Some(found) = self.loaded.get(path) {
            return found.clone();
        }
        if visiting.iter().any(|p| p == path) {
            return None;
        }
        visiting.push(path.to_string());
        let result = self.load_uncached(snapshot, path, visiting);
        visiting.pop();
        self.loaded.insert(path.to_string(), result.clone());
        result
    }

    fn load_uncached(
        &mut self,
        snapshot: &JsPathsSnapshot,
        path: &str,
        visiting: &mut Vec<String>,
    ) -> Option<Rc<AliasConfig>> {
        let value = jsonc(snapshot.configs.get(path)?.as_ref()?)?;
        let object = value.as_object()?;
        let mut config = AliasConfig::default();
        let extends: Vec<&str> = match object.get("extends") {
            Some(serde_json::Value::String(one)) => vec![one.as_str()],
            Some(serde_json::Value::Array(many)) => {
                many.iter().filter_map(|v| v.as_str()).collect()
            }
            _ => Vec::new(),
        };
        for parent in extends.into_iter().filter(|e| e.starts_with('.')) {
            let mut parent_path = norm(dir(path), parent)?;
            if !parent_path.ends_with(".json") {
                parent_path.push_str(".json");
            }
            let inherited = self.load(snapshot, &parent_path, visiting)?;
            // A later parent overrides only the fields it sets.
            if inherited.base != BaseUrl::Absent {
                config.base = inherited.base.clone();
            }
            if inherited.paths.is_some() {
                config.paths = inherited.paths.clone();
            }
        }
        if let Some(options) = object.get("compilerOptions").and_then(|v| v.as_object()) {
            if let Some(base) = options.get("baseUrl").and_then(|v| v.as_str()) {
                config.base = norm(dir(path), base).map_or(BaseUrl::Unusable, BaseUrl::Usable);
            }
            if let Some(paths) = options.get("paths").and_then(|v| v.as_object()) {
                let entries = paths
                    .iter()
                    .filter_map(|(key, value)| {
                        let targets = value
                            .as_array()?
                            .iter()
                            .filter_map(|t| t.as_str().map(str::to_string))
                            .collect();
                        Some((key.clone(), targets))
                    })
                    .collect();
                config.paths = Some((dir(path).to_string(), entries));
            }
        }
        Some(Rc::new(config))
    }
}

/// TypeScript's key choice: an exact key, else the pattern with the longest
/// literal prefix; of equal prefixes the first key wins, as in TypeScript. The
/// entries are in parser order (sorted by key: the JSON reader keeps no file
/// order), so a tie is broken by key order, not file order. Returns the key,
/// its targets and the text its `*` captured.
/// The `paths` key a specifier matches: an exact key, else the wildcard key
/// with the longest literal prefix. Two wildcard keys tied on that prefix are
/// `Tied`: TypeScript takes the first in source order, which the sorted JSON
/// reader does not keep, so the lookup refuses rather than guess.
enum KeyMatch<'a> {
    None,
    One(&'a str, &'a [String], &'a str),
    Tied,
}

fn matching_key<'a>(entries: &'a [(String, Vec<String>)], spec: &'a str) -> KeyMatch<'a> {
    if let Some((key, targets)) = entries.iter().find(|(k, _)| k == spec) {
        return KeyMatch::One(key, targets, "");
    }
    let mut best: Option<(usize, &str, &[String], &str)> = None;
    let mut tied = false;
    for (key, targets) in entries {
        let Some((pre, post)) = key.split_once('*') else {
            continue;
        };
        let fits =
            spec.len() >= pre.len() + post.len() && spec.starts_with(pre) && spec.ends_with(post);
        if !fits {
            continue;
        }
        match best {
            Some((prefix, ..)) if pre.len() == prefix => tied = true,
            Some((prefix, ..)) if pre.len() < prefix => {}
            _ => {
                let capture = &spec[pre.len()..spec.len() - post.len()];
                best = Some((pre.len(), key.as_str(), targets.as_slice(), capture));
                tied = false;
            }
        }
    }
    match best {
        Some(_) if tied => KeyMatch::Tied,
        Some((_, key, targets, capture)) => KeyMatch::One(key, targets, capture),
        None => KeyMatch::None,
    }
}

/// The file TS probing picks for repo path `path` (extensions, then index
/// files), by the same precedence prism's relative resolver uses.
fn winner(path: &str, listing: &BTreeSet<String>) -> Option<String> {
    js_ts_relative_module_candidates(&format!("./{path}"), "", listing)?
        .into_iter()
        .next()
}

/// True when the repo walk never lists `path`: a built-in skipped directory or a
/// hidden one.
fn in_skipped_dir(path: &str) -> bool {
    path.split('/')
        .any(|part| BUILTIN_SKIP_DIRS.contains(&part) || (part.starts_with('.') && part.len() > 1))
}
