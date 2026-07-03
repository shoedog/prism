//! JS/TS export-fact modeling (P4).
//!
//! Typed export facts consumed by the R4c import-member resolution rung
//! (`resolution.rs`), replacing the flat exported-name set that only supported
//! `export function name(...)`. Covers:
//!
//! - default exports (`export default function name() {}`, `export default name;`)
//! - named export lists, including renames (`export { a, b as c };`)
//! - exported const-arrow / function-expression declarations (`export const f = () => {}`)
//! - CommonJS assignments (`module.exports = f`, `module.exports = { a, b }`,
//!   `module.exports.f = f`, `exports.f = f`)
//! - re-export chains (`export { x } from './y'`, `export * from './y'`),
//!   depth-bounded and cycle-safe
//!
//! Extraction from tree-sitter ASTs lives in `ast.rs` (per repo convention: all
//! tree-sitter interaction goes through `ParsedFile`); this module owns the
//! typed data model and the whole-program resolution algorithm that follows
//! re-export chains, so `ast.rs` and `call_graph.rs` stay thin call sites.

use std::collections::{BTreeMap, BTreeSet};

/// Depth bound for re-export chain / barrel resolution. Mirrors
/// `name_resolution::engine::MAX_GLOB_DEPTH`'s fail-closed pattern: a chain
/// that would need a 3rd hop, or a cycle, resolves to nothing rather than
/// resolving unboundedly or looping.
pub const MAX_REEXPORT_DEPTH: usize = 2;

/// What a single exported name refers to, before whole-program chain
/// resolution.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum JsExportTarget {
    /// A function / const-arrow / function-expression declared in this same file.
    Local(String),
    /// `export { imported as exported_name } from './y'` (also used for the
    /// re-export half of barrel resolution). `imported` is the name as
    /// declared/exported in the target module (`"default"` for a default
    /// re-export).
    ReExport {
        module_path: String,
        imported: String,
    },
}

/// Raw (per-file, un-resolved) JS/TS export facts extracted from a single
/// file's AST. Incrementally maintained like other per-file R4c facts
/// (`import_bindings`, `module_bindings`): removed/merged wholesale per file.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct JsExportFacts {
    /// Exported name (what an importer writes: `"default"`, `"process"`,
    /// `"c"`, ...) -> target.
    pub named: BTreeMap<String, JsExportTarget>,
    /// `export * from './y'` module paths (barrel re-export-all; per ES
    /// module semantics this never carries `default`).
    pub star_reexports: BTreeSet<String>,
    /// Count of default-export / CommonJS assignment forms skipped because
    /// the RHS was not a plain identifier referencing an in-file declaration
    /// (e.g. `module.exports = someExpression()`). Telemetry only.
    pub skipped_expr_count: usize,
}

impl JsExportFacts {
    pub fn is_empty(&self) -> bool {
        self.named.is_empty() && self.star_reexports.is_empty() && self.skipped_expr_count == 0
    }
}

/// One resolved JS/TS export: the concrete file and local declared name an
/// exported name ultimately refers to, after following re-export chains.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ResolvedJsExport {
    pub file: String,
    pub local_name: String,
}

/// Whole-program resolution output: per-file resolved export tables plus
/// fail-closed telemetry.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct JsExportResolution {
    /// file -> exported name -> resolved target (present only when the chain
    /// resolved to exactly one target within the depth bound).
    pub resolved: BTreeMap<String, BTreeMap<String, ResolvedJsExport>>,
    /// Re-export chains that exceeded `MAX_REEXPORT_DEPTH` or hit a cycle.
    pub chain_unresolved: usize,
    /// Barrel names contributed by 2+ conflicting chains (different resolved
    /// targets for the same exported name) — fail-closed, no binding emitted.
    pub barrel_conflicts: usize,
}

/// Resolve raw per-file export facts into concrete `(file, local_name)`
/// targets, following re-export chains and `export *` barrels.
///
/// `resolve_module` maps `(from_file, module_path) -> Option<target_file>`,
/// using the same exact relative-module resolution R4c already uses
/// (`call_graph::resolve_js_ts_relative_module`) against the indexed file set.
pub fn resolve_js_exports(
    raw: &BTreeMap<String, JsExportFacts>,
    resolve_module: &dyn Fn(&str, &str) -> Option<String>,
) -> JsExportResolution {
    let mut out = JsExportResolution::default();
    for file in raw.keys() {
        let mut visited_files = BTreeSet::new();
        let names = collect_candidate_names(raw, resolve_module, file, 0, &mut visited_files);
        let mut per_file = BTreeMap::new();
        for name in names {
            let mut visited = BTreeSet::new();
            if let Some(hit) = resolve_one(raw, resolve_module, file, &name, 0, &mut visited, &mut out)
            {
                per_file.insert(name, hit);
            }
        }
        if !per_file.is_empty() {
            out.resolved.insert(file.clone(), per_file);
        }
    }
    out
}

/// Collect the set of exported names a file can plausibly serve, including
/// names only reachable via `export * from` barrels (which don't enumerate
/// names syntactically — the candidate set has to be discovered from the
/// barrel targets' own facts). Depth-bounded and cycle-guarded the same way
/// as the value resolution below, since this is itself a chain traversal.
fn collect_candidate_names(
    raw: &BTreeMap<String, JsExportFacts>,
    resolve_module: &dyn Fn(&str, &str) -> Option<String>,
    file: &str,
    depth: usize,
    visited_files: &mut BTreeSet<String>,
) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    if !visited_files.insert(file.to_string()) {
        return names; // cycle in the barrel graph
    }
    if let Some(facts) = raw.get(file) {
        names.extend(facts.named.keys().cloned());
        if depth < MAX_REEXPORT_DEPTH {
            for module_path in &facts.star_reexports {
                if let Some(target_file) = resolve_module(file, module_path) {
                    let sub = collect_candidate_names(
                        raw,
                        resolve_module,
                        &target_file,
                        depth + 1,
                        visited_files,
                    );
                    names.extend(sub.into_iter().filter(|n| n != "default"));
                }
            }
        }
    }
    visited_files.remove(file);
    names
}

/// Resolve a single exported `name` in `file` to a concrete `(file,
/// local_name)`, following at most `MAX_REEXPORT_DEPTH` re-export hops
/// (`hops` counts hops already taken to reach this call). Cycle-guarded via
/// `visited` ((file, name) pairs currently on the resolution stack).
fn resolve_one(
    raw: &BTreeMap<String, JsExportFacts>,
    resolve_module: &dyn Fn(&str, &str) -> Option<String>,
    file: &str,
    name: &str,
    hops: usize,
    visited: &mut BTreeSet<(String, String)>,
    telemetry: &mut JsExportResolution,
) -> Option<ResolvedJsExport> {
    let key = (file.to_string(), name.to_string());
    if !visited.insert(key.clone()) {
        telemetry.chain_unresolved += 1;
        return None; // cycle
    }
    let result = resolve_one_inner(raw, resolve_module, file, name, hops, visited, telemetry);
    visited.remove(&key);
    result
}

fn resolve_one_inner(
    raw: &BTreeMap<String, JsExportFacts>,
    resolve_module: &dyn Fn(&str, &str) -> Option<String>,
    file: &str,
    name: &str,
    hops: usize,
    visited: &mut BTreeSet<(String, String)>,
    telemetry: &mut JsExportResolution,
) -> Option<ResolvedJsExport> {
    let facts = raw.get(file)?;

    if let Some(target) = facts.named.get(name) {
        return match target {
            JsExportTarget::Local(local) => Some(ResolvedJsExport {
                file: file.to_string(),
                local_name: local.clone(),
            }),
            JsExportTarget::ReExport {
                module_path,
                imported,
            } => {
                if hops + 1 > MAX_REEXPORT_DEPTH {
                    telemetry.chain_unresolved += 1;
                    return None;
                }
                let target_file = resolve_module(file, module_path)?;
                resolve_one(
                    raw,
                    resolve_module,
                    &target_file,
                    imported,
                    hops + 1,
                    visited,
                    telemetry,
                )
            }
        };
    }

    // Not a direct named export: fall back to `export * from` barrels.
    // `export *` never re-exports `default` (ES module semantics).
    if name == "default" || facts.star_reexports.is_empty() {
        return None;
    }
    if hops + 1 > MAX_REEXPORT_DEPTH {
        telemetry.chain_unresolved += 1;
        return None;
    }
    let mut candidates: BTreeSet<(String, String)> = BTreeSet::new();
    for module_path in &facts.star_reexports {
        let Some(target_file) = resolve_module(file, module_path) else {
            continue;
        };
        // Fork the visited set per barrel branch: sibling barrels shouldn't
        // spuriously "cycle" each other out, only a genuine repeated
        // (file, name) on ONE path should.
        let mut branch_visited = visited.clone();
        if let Some(hit) = resolve_one(
            raw,
            resolve_module,
            &target_file,
            name,
            hops + 1,
            &mut branch_visited,
            telemetry,
        ) {
            candidates.insert((hit.file, hit.local_name));
        }
    }
    match candidates.len() {
        0 => None,
        1 => {
            let (file, local_name) = candidates.into_iter().next().unwrap();
            Some(ResolvedJsExport { file, local_name })
        }
        _ => {
            telemetry.barrel_conflicts += 1;
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(named: &[(&str, JsExportTarget)], star: &[&str]) -> JsExportFacts {
        JsExportFacts {
            named: named
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect(),
            star_reexports: star.iter().map(|s| s.to_string()).collect(),
            skipped_expr_count: 0,
        }
    }

    fn local(name: &str) -> JsExportTarget {
        JsExportTarget::Local(name.to_string())
    }

    fn reexport(module_path: &str, imported: &str) -> JsExportTarget {
        JsExportTarget::ReExport {
            module_path: module_path.to_string(),
            imported: imported.to_string(),
        }
    }

    /// A trivial resolver for these unit tests: `./x` maps to file `x.ts`.
    fn resolve_dot(from: &str, module_path: &str) -> Option<String> {
        let _ = from;
        module_path
            .strip_prefix("./")
            .map(|stem| format!("{stem}.ts"))
    }

    #[test]
    fn direct_local_export_resolves() {
        let mut raw = BTreeMap::new();
        raw.insert("util.ts".to_string(), facts(&[("process", local("process"))], &[]));
        let out = resolve_js_exports(&raw, &resolve_dot);
        assert_eq!(
            out.resolved["util.ts"]["process"],
            ResolvedJsExport {
                file: "util.ts".to_string(),
                local_name: "process".to_string()
            }
        );
        assert_eq!(out.chain_unresolved, 0);
        assert_eq!(out.barrel_conflicts, 0);
    }

    #[test]
    fn one_hop_reexport_resolves() {
        let mut raw = BTreeMap::new();
        raw.insert("impl.ts".to_string(), facts(&[("process", local("process"))], &[]));
        raw.insert(
            "index.ts".to_string(),
            facts(&[("process", reexport("./impl", "process"))], &[]),
        );
        let out = resolve_js_exports(&raw, &resolve_dot);
        assert_eq!(
            out.resolved["index.ts"]["process"],
            ResolvedJsExport {
                file: "impl.ts".to_string(),
                local_name: "process".to_string()
            }
        );
        assert_eq!(out.chain_unresolved, 0);
    }

    #[test]
    fn two_hop_reexport_resolves_at_depth_bound() {
        let mut raw = BTreeMap::new();
        raw.insert("impl.ts".to_string(), facts(&[("process", local("process"))], &[]));
        raw.insert(
            "mid.ts".to_string(),
            facts(&[("process", reexport("./impl", "process"))], &[]),
        );
        raw.insert(
            "index.ts".to_string(),
            facts(&[("process", reexport("./mid", "process"))], &[]),
        );
        let out = resolve_js_exports(&raw, &resolve_dot);
        assert_eq!(
            out.resolved["index.ts"]["process"],
            ResolvedJsExport {
                file: "impl.ts".to_string(),
                local_name: "process".to_string()
            }
        );
        assert_eq!(out.chain_unresolved, 0);
    }

    #[test]
    fn three_hop_reexport_fails_closed() {
        let mut raw = BTreeMap::new();
        raw.insert("impl.ts".to_string(), facts(&[("process", local("process"))], &[]));
        raw.insert(
            "mid2.ts".to_string(),
            facts(&[("process", reexport("./impl", "process"))], &[]),
        );
        raw.insert(
            "mid.ts".to_string(),
            facts(&[("process", reexport("./mid2", "process"))], &[]),
        );
        raw.insert(
            "index.ts".to_string(),
            facts(&[("process", reexport("./mid", "process"))], &[]),
        );
        let out = resolve_js_exports(&raw, &resolve_dot);
        assert!(!out.resolved.contains_key("index.ts"));
        assert!(out.chain_unresolved > 0);
    }

    #[test]
    fn direct_cycle_fails_closed() {
        let mut raw = BTreeMap::new();
        raw.insert(
            "a.ts".to_string(),
            facts(&[("x", reexport("./b", "x"))], &[]),
        );
        raw.insert(
            "b.ts".to_string(),
            facts(&[("x", reexport("./a", "x"))], &[]),
        );
        let out = resolve_js_exports(&raw, &resolve_dot);
        assert!(!out.resolved.contains_key("a.ts"));
        assert!(!out.resolved.contains_key("b.ts"));
        assert!(out.chain_unresolved > 0);
    }

    #[test]
    fn star_reexport_surfaces_target_names() {
        let mut raw = BTreeMap::new();
        raw.insert("impl.ts".to_string(), facts(&[("process", local("process"))], &[]));
        raw.insert("index.ts".to_string(), facts(&[], &["./impl"]));
        let out = resolve_js_exports(&raw, &resolve_dot);
        assert_eq!(
            out.resolved["index.ts"]["process"],
            ResolvedJsExport {
                file: "impl.ts".to_string(),
                local_name: "process".to_string()
            }
        );
    }

    #[test]
    fn star_reexport_never_surfaces_default() {
        let mut raw = BTreeMap::new();
        raw.insert("impl.ts".to_string(), facts(&[("default", local("process"))], &[]));
        raw.insert("index.ts".to_string(), facts(&[], &["./impl"]));
        let out = resolve_js_exports(&raw, &resolve_dot);
        assert!(!out
            .resolved
            .get("index.ts")
            .is_some_and(|m| m.contains_key("default")));
    }

    #[test]
    fn conflicting_star_reexports_fail_closed() {
        let mut raw = BTreeMap::new();
        raw.insert("a.ts".to_string(), facts(&[("process", local("process"))], &[]));
        raw.insert(
            "b.ts".to_string(),
            facts(&[("process", local("otherProcess"))], &[]),
        );
        raw.insert("index.ts".to_string(), facts(&[], &["./a", "./b"]));
        let out = resolve_js_exports(&raw, &resolve_dot);
        assert!(!out
            .resolved
            .get("index.ts")
            .is_some_and(|m| m.contains_key("process")));
        assert_eq!(out.barrel_conflicts, 1);
    }

    #[test]
    fn identical_star_reexports_are_not_a_conflict() {
        // Two barrels that happen to agree on the same underlying target are
        // redundant, not conflicting.
        let mut raw = BTreeMap::new();
        raw.insert("impl.ts".to_string(), facts(&[("process", local("process"))], &[]));
        raw.insert(
            "reexport_a.ts".to_string(),
            facts(&[("process", reexport("./impl", "process"))], &[]),
        );
        raw.insert("index.ts".to_string(), facts(&[], &["./impl", "./reexport_a"]));
        let out = resolve_js_exports(&raw, &resolve_dot);
        assert_eq!(
            out.resolved["index.ts"]["process"],
            ResolvedJsExport {
                file: "impl.ts".to_string(),
                local_name: "process".to_string()
            }
        );
        assert_eq!(out.barrel_conflicts, 0);
    }

    #[test]
    fn unresolved_module_path_yields_no_binding_without_telemetry() {
        // A re-export pointing at an external / untracked module is an
        // ordinary miss, not a depth-exceeded or cycle case.
        let mut raw = BTreeMap::new();
        raw.insert(
            "a.ts".to_string(),
            facts(&[("process", reexport("external-pkg", "process"))], &[]),
        );
        let out = resolve_js_exports(&raw, &resolve_dot);
        assert!(!out.resolved.contains_key("a.ts"));
        assert_eq!(out.chain_unresolved, 0);
        assert_eq!(out.barrel_conflicts, 0);
    }
}
