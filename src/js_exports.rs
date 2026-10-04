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
    /// A syntactic CJS claim without callable capture proof. Never authority;
    /// retained so duplicate and barrel conflicts cannot disappear.
    UnprovenLocal(String),
    /// Declaration-backed local class. Never a callable-function export.
    Class(String),
    /// `export const X = <admitted React wrapper>(fn)` (S1): the inner function's
    /// registered name and exact line span. R4c binds it by span, from JSX sites only.
    SpannedLocal {
        local: String,
        start_line: usize,
        end_line: usize,
    },
    /// S1b: an ESM local export whose module-scope binding holds exactly this plain
    /// callable (SPEC §3.2); R4c binds it by span from any site.
    VerifiedLocal {
        local: String,
        start_line: usize,
        end_line: usize,
    },
    /// `export { imported as exported_name } from './y'` (also used for the
    /// re-export half of barrel resolution). `imported` is the name as
    /// declared/exported in the target module (`"default"` for a default
    /// re-export).
    ReExport {
        module_path: String,
        imported: String,
    },
    /// A singleton, unwritten ESM imported binding forwarded by an ESM export.
    /// Unlike ReExport, requires source-backed terminal function proof.
    ImportForward {
        module_path: String,
        imported: String,
    },
}

/// Raw (per-file, un-resolved) JS/TS export facts extracted from a single
/// file's AST. Incrementally maintained like other per-file R4c facts
/// (`import_bindings`, `module_bindings`): removed/merged wholesale per file.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct JsExportFacts {
    /// S2 member authority, isolated from the landed S1b/P export projections.
    #[serde(default)]
    pub qualifiers: crate::js_import_qualifiers::QualifierFacts,
    /// R3 positive proof only: no unrecorded/skipped export or incomplete module.
    #[serde(default)]
    pub namespace_proof_complete: bool,
    /// Terminal identities admitted by the landed binding core as Callable.
    #[serde(default)]
    pub namespace_callable_locals: BTreeMap<String, Vec<ResolvedJsExport>>,
    /// Syntactic module value declarations, used only to forbid repo-global
    /// name fallback. This is not callable or forwarding authority.
    #[serde(default)]
    pub module_value_bindings: BTreeSet<String>,
    /// Unique, unwritten top-level named function declarations available to
    /// the bounded imported-local forwarding lane only; not class authority.
    #[serde(default)]
    pub forwardable_function_locals: BTreeSet<String>,
    /// Local names introduced by top-level ESM named or default value imports.
    /// Syntax provenance only; eligible binding and class proof are separate.
    pub esm_named_imports: BTreeSet<String>,
    /// Type namespace only: local -> (module, exported class name). None is a
    /// duplicate/conflicting or unsupported import, and must remain terminal.
    pub type_only_imports: BTreeMap<String, Option<(String, String)>>,
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
    /// Exported names with 2+ raw fact insertions in this file (F3: re-export
    /// lists, local named lists, or a mix -- e.g. `export { f } from './a';
    /// export { f } from './b';`). Populated by `insert_named` instead of
    /// silently overwriting `named`, so whole-program resolution (which sees
    /// both facts) can fail closed instead of picking a last-writer winner.
    pub conflicted: BTreeSet<String>,
    /// Exported declarators admitted as `SpannedLocal` (S1). Telemetry only.
    #[serde(default)]
    pub spanned_admitted: usize,
    /// Declarator skips (a subset of `skipped_expr_count`) keyed by refusal reason.
    #[serde(default)]
    pub skipped_decl_reasons: BTreeMap<String, usize>,
    /// S1b: ESM local exports whose module-scope binding holds no single proven callable,
    /// by refusal reason (SPEC §3.2).
    #[serde(default)]
    pub local_export_refusals: BTreeMap<String, usize>,
    /// S1b: ESM local exports kept at base `Local` because the binding is may-call (E5).
    #[serde(default)]
    pub local_export_may_call: usize,
}

impl JsExportFacts {
    pub fn is_empty(&self) -> bool {
        !self.qualifiers.complete
            && !self.namespace_proof_complete
            && self.namespace_callable_locals.is_empty()
            && self.named.is_empty()
            && self.module_value_bindings.is_empty()
            && self.forwardable_function_locals.is_empty()
            && self.esm_named_imports.is_empty()
            && self.type_only_imports.is_empty()
            && self.star_reexports.is_empty()
            && self.skipped_expr_count == 0
            && self.conflicted.is_empty()
            && self.spanned_admitted == 0
            && self.skipped_decl_reasons.is_empty()
            && self.local_export_refusals.is_empty()
            && self.local_export_may_call == 0
    }

    /// Record a named-export raw fact, poisoning (marking conflicted) a name
    /// an earlier fact in this file already claimed, instead of silently
    /// overwriting it (F3). ANY duplicate insertion -- two re-export lists, a
    /// local declaration plus a re-export, even two identical re-exports of
    /// the same target -- poisons the name: resolution refuses to bind it.
    pub fn insert_named(&mut self, name: String, target: JsExportTarget) {
        if self.conflicted.contains(&name) {
            return; // already poisoned; nothing more to do
        }
        if self.named.remove(&name).is_some() {
            self.conflicted.insert(name);
        } else {
            self.named.insert(name, target);
        }
    }
}

/// One resolved JS/TS export: the concrete file and local declared name an
/// exported name ultimately refers to, after following re-export chains.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ResolvedJsExport {
    pub file: String,
    pub local_name: String,
    /// `SpannedLocal` / `VerifiedLocal` targets: the callable's exact `(start, end)` lines.
    #[serde(default)]
    pub span: Option<(usize, usize)>,
    /// S1b: the span is a wrapped React render function, bindable from JSX sites only.
    #[serde(default)]
    pub wrapped: bool,
    /// Alias routes decline any closure that skipped an unresolved/opaque star.
    #[serde(default)]
    pub via_unresolved_star: bool,
}

/// Whole-program resolution output: per-file resolved export tables plus
/// fail-closed telemetry.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct JsExportResolution {
    /// file -> exported name -> resolved target (present only when the chain
    /// resolved to exactly one target within the depth bound).
    pub resolved: BTreeMap<String, BTreeMap<String, ResolvedJsExport>>,
    /// Re-export chains that exceeded `MAX_REEXPORT_DEPTH` or hit a cycle
    /// (named chains AND star-only barrel chains — F5).
    pub chain_unresolved: usize,
    /// Barrel names contributed by 2+ conflicting chains (different resolved
    /// targets for the same exported name), OR a name with 2+ raw fact
    /// insertions in a single file (F3 — reuses this counter rather than
    /// adding a sibling one) — fail-closed, no binding emitted either way.
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
    resolve_js_exports_for_projection(raw, resolve_module, raw.keys(), ExportProjection::Legacy)
}

// Alias consumers query only admitted root modules. Keep the complete raw map
// for the same dependency traversal, conflicts and unresolved-star proofs.
pub(crate) fn resolve_js_exports_for<'a>(
    raw: &BTreeMap<String, JsExportFacts>,
    resolve_module: &dyn Fn(&str, &str) -> Option<String>,
    roots: impl IntoIterator<Item = &'a String>,
) -> JsExportResolution {
    resolve_js_exports_for_projection(raw, resolve_module, roots, ExportProjection::CallerPaths)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum ExportProjection {
    Legacy,
    CallerPaths,
}

struct ExportResolver<'a> {
    module: &'a dyn Fn(&str, &str) -> Option<String>,
    projection: ExportProjection,
}

// Import-forward eligibility is about the raw literal. The legacy module
// helper trims strings, so it cannot establish this boundary for us.
fn raw_relative_literal(spec: &str) -> bool {
    (spec.starts_with("./") || spec.starts_with("../")) && spec.trim() == spec
}

fn resolve_js_exports_for_projection<'a>(
    raw: &BTreeMap<String, JsExportFacts>,
    resolve_module: &dyn Fn(&str, &str) -> Option<String>,
    roots: impl IntoIterator<Item = &'a String>,
    projection: ExportProjection,
) -> JsExportResolution {
    let resolver = ExportResolver {
        module: resolve_module,
        projection,
    };
    let mut out = JsExportResolution::default();
    for file in roots {
        let mut visited_files = BTreeSet::new();
        let names =
            collect_candidate_names(raw, resolve_module, file, 0, &mut visited_files, &mut out);
        let mut per_file = BTreeMap::new();
        for name in names {
            let mut visited = BTreeSet::new();
            if let ExportLookup::Resolved(mut hit, false) =
                resolve_one(raw, &resolver, file, &name, 0, &mut visited, &mut out)
            {
                hit.via_unresolved_star =
                    skipped_star(raw, resolve_module, file, &name, &mut BTreeSet::new());
                per_file.insert(name, hit);
            }
        }
        if !per_file.is_empty() {
            out.resolved.insert(file.clone(), per_file);
        }
    }
    out
}

/// Namespace-only positive proof. Unknown branches invalidate uniqueness even
/// when another branch has a terminal. Absence is never row authority.
pub fn resolve_js_namespace_exports(
    raw: &BTreeMap<String, JsExportFacts>,
    resolve_module: &dyn Fn(&str, &str) -> Option<String>,
) -> BTreeMap<String, BTreeMap<String, ResolvedJsExport>> {
    let mut resolved = BTreeMap::new();
    for file in raw.keys() {
        let mut telemetry = JsExportResolution::default();
        let names = collect_candidate_names(
            raw,
            resolve_module,
            file,
            0,
            &mut BTreeSet::new(),
            &mut telemetry,
        );
        for name in names {
            if let Ok(Some(target)) =
                namespace_identity(raw, resolve_module, file, &name, 0, &mut BTreeSet::new())
            {
                resolved
                    .entry(file.clone())
                    .or_insert_with(BTreeMap::new)
                    .insert(name, target);
            }
        }
    }
    resolved
}

fn namespace_identity(
    raw: &BTreeMap<String, JsExportFacts>,
    resolve_module: &dyn Fn(&str, &str) -> Option<String>,
    file: &str,
    name: &str,
    depth: usize,
    visiting: &mut BTreeSet<(String, String)>,
) -> Result<Option<ResolvedJsExport>, ()> {
    if depth > MAX_REEXPORT_DEPTH {
        return Err(());
    }
    let key = (file.to_string(), name.to_string());
    if !visiting.insert(key.clone()) {
        return Err(());
    }
    let result = (|| {
        let facts = legacy_export_facts(raw, file)
            .filter(|f| f.namespace_proof_complete)
            .ok_or(())?;
        if facts.conflicted.contains(name) {
            return Err(());
        }
        if let Some(target) = facts.named.get(name) {
            return match target {
                JsExportTarget::ReExport {
                    module_path,
                    imported,
                }
                | JsExportTarget::ImportForward {
                    module_path,
                    imported,
                } => {
                    // Namespace exports also use the legacy relative projection.
                    if matches!(target, JsExportTarget::ImportForward { .. })
                        && !raw_relative_literal(module_path)
                    {
                        return Err(());
                    }
                    let next = resolve_module(file, module_path).ok_or(())?;
                    namespace_identity(raw, resolve_module, &next, imported, depth + 1, visiting)
                }
                // Local/UnprovenLocal/Class are never a proven Callable terminal
                // (MayCall, import, refusal, class): keep base.
                JsExportTarget::Local(_)
                | JsExportTarget::UnprovenLocal(_)
                | JsExportTarget::Class(_) => Err(()),
                JsExportTarget::SpannedLocal {
                    local,
                    start_line,
                    end_line,
                }
                | JsExportTarget::VerifiedLocal {
                    local,
                    start_line,
                    end_line,
                } => {
                    let candidates = facts.namespace_callable_locals.get(local).ok_or(())?;
                    let matching = candidates
                        .iter()
                        .filter(|t| t.span == Some((*start_line, *end_line)))
                        .collect::<Vec<_>>();
                    match matching.as_slice() {
                        [only] => Ok(Some((*only).clone())),
                        _ => Err(()),
                    }
                }
            };
        }
        let mut unique = None;
        if name != "default" {
            for module in &facts.star_reexports {
                let next = resolve_module(file, module).ok_or(())?;
                if let Some(target) =
                    namespace_identity(raw, resolve_module, &next, name, depth + 1, visiting)?
                {
                    if unique.as_ref().is_some_and(|prior| prior != &target) {
                        return Err(());
                    }
                    unique = Some(target);
                }
            }
        }
        Ok(unique)
    })();
    visiting.remove(&key);
    result
}

// A skipped branch can have no candidate of its own, so propagate provenance
// over the complete star closure, including empty nested branches. Direct named
// exports override stars; named reexports carry their target's provenance.
fn legacy_export_facts<'a>(
    raw: &'a BTreeMap<String, JsExportFacts>,
    file: &str,
) -> Option<&'a JsExportFacts> {
    // Retaining S2 refusal facts must not turn a formerly missing parse-error
    // branch into a proven-empty branch for the landed export resolver.
    raw.get(file)
        .filter(|f| !f.qualifiers.syntax_incomplete || !f.is_empty())
}

fn skipped_star(
    raw: &BTreeMap<String, JsExportFacts>,
    resolve_module: &dyn Fn(&str, &str) -> Option<String>,
    file: &str,
    name: &str,
    visited: &mut BTreeSet<(String, String)>,
) -> bool {
    let key = (file.to_owned(), name.to_owned());
    if visited.len() > MAX_REEXPORT_DEPTH || !visited.insert(key.clone()) {
        return true;
    }
    let result = match legacy_export_facts(raw, file) {
        None => true,
        Some(facts) => match facts.named.get(name) {
            Some(
                JsExportTarget::ReExport {
                    module_path,
                    imported,
                }
                | JsExportTarget::ImportForward {
                    module_path,
                    imported,
                },
            ) => resolve_module(file, module_path)
                .is_none_or(|target| skipped_star(raw, resolve_module, &target, imported, visited)),
            Some(_) => false,
            None => {
                facts.skipped_expr_count > facts.skipped_decl_reasons.values().sum::<usize>()
                    || (!facts.skipped_decl_reasons.is_empty()
                        && facts.module_value_bindings.contains(name))
                    || facts.star_reexports.iter().any(|module_path| {
                        resolve_module(file, module_path).is_none_or(|target| {
                            skipped_star(raw, resolve_module, &target, name, visited)
                        })
                    })
            }
        },
    };
    visited.remove(&key);
    result
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
    telemetry: &mut JsExportResolution,
) -> BTreeSet<String> {
    let mut names = BTreeSet::new();
    if !visited_files.insert(file.to_string()) {
        telemetry.chain_unresolved += 1; // cycle in the barrel graph
        return names;
    }
    if let Some(facts) = legacy_export_facts(raw, file) {
        names.extend(facts.named.keys().cloned());
        // F3: conflicted names are attempted (and counted) too, not silently
        // dropped -- `resolve_one_inner` sees `conflicted` before `named`.
        names.extend(facts.conflicted.iter().cloned());
        if !facts.star_reexports.is_empty() {
            if depth < MAX_REEXPORT_DEPTH {
                for module_path in &facts.star_reexports {
                    if let Some(target_file) = resolve_module(file, module_path) {
                        let sub = collect_candidate_names(
                            raw,
                            resolve_module,
                            &target_file,
                            depth + 1,
                            visited_files,
                            telemetry,
                        );
                        names.extend(sub.into_iter().filter(|n| n != "default"));
                    }
                }
            } else {
                // F5: a star-only chain needing a hop beyond
                // MAX_REEXPORT_DEPTH to even discover its candidate names
                // previously vanished with no telemetry trace -- a named
                // chain at least gets attempted (and counted) once
                // `resolve_one` sees the candidate name; count this the
                // same way.
                telemetry.chain_unresolved += 1;
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
enum ExportLookup {
    NoTarget,
    BlockedClaim,
    Resolved(ResolvedJsExport, bool),
}

/// A star-barrel candidate: `(file, local_name, is_class, span, wrapped)`.
type BarrelCandidate = (String, String, bool, Option<(usize, usize)>, bool);

fn resolve_one(
    raw: &BTreeMap<String, JsExportFacts>,
    resolver: &ExportResolver<'_>,
    file: &str,
    name: &str,
    hops: usize,
    visited: &mut BTreeSet<(String, String)>,
    telemetry: &mut JsExportResolution,
) -> ExportLookup {
    let key = (file.to_string(), name.to_string());
    if !visited.insert(key.clone()) {
        telemetry.chain_unresolved += 1;
        return ExportLookup::NoTarget; // cycle; existing bounded traversal policy
    }
    let result = resolve_one_inner(raw, resolver, file, name, hops, visited, telemetry);
    visited.remove(&key);
    result
}

fn resolve_one_inner(
    raw: &BTreeMap<String, JsExportFacts>,
    resolver: &ExportResolver<'_>,
    file: &str,
    name: &str,
    hops: usize,
    visited: &mut BTreeSet<(String, String)>,
    telemetry: &mut JsExportResolution,
) -> ExportLookup {
    let Some(facts) = legacy_export_facts(raw, file) else {
        return ExportLookup::NoTarget;
    };

    // F3 (review-fix wave, codex MAJOR 1): a name with 2+ raw fact
    // insertions in this file is poisoned -- it must resolve to NO binding,
    // fail-closed before any target is emitted.
    if facts.conflicted.contains(name) {
        telemetry.barrel_conflicts += 1;
        return ExportLookup::BlockedClaim;
    }

    if let Some(target) = facts.named.get(name) {
        return match target {
            JsExportTarget::UnprovenLocal(_) => ExportLookup::BlockedClaim,
            JsExportTarget::SpannedLocal {
                local,
                start_line,
                end_line,
            }
            | JsExportTarget::VerifiedLocal {
                local,
                start_line,
                end_line,
            } => ExportLookup::Resolved(
                ResolvedJsExport {
                    file: file.to_string(),
                    local_name: local.clone(),
                    span: Some((*start_line, *end_line)),
                    via_unresolved_star: false,
                    wrapped: matches!(target, JsExportTarget::SpannedLocal { .. }),
                },
                false,
            ),
            // Class identity participates in conflicts, never callable projection.
            JsExportTarget::Class(local) | JsExportTarget::Local(local) => ExportLookup::Resolved(
                ResolvedJsExport {
                    file: file.to_string(),
                    local_name: local.clone(),
                    span: None,
                    via_unresolved_star: false,
                    wrapped: false,
                },
                matches!(target, JsExportTarget::Class(_)),
            ),
            JsExportTarget::ReExport {
                module_path,
                imported,
            }
            | JsExportTarget::ImportForward {
                module_path,
                imported,
            } => {
                let nonrelative_forward = matches!(target, JsExportTarget::ImportForward { .. })
                    && !raw_relative_literal(module_path);
                if hops + 1 > MAX_REEXPORT_DEPTH {
                    telemetry.chain_unresolved += 1;
                    // Preserve the claim even when the next hop exceeds the bound.
                    if nonrelative_forward {
                        return ExportLookup::BlockedClaim;
                    }
                    return ExportLookup::NoTarget;
                }
                let target_file =
                    if nonrelative_forward && resolver.projection == ExportProjection::Legacy {
                        None
                    } else {
                        (resolver.module)(file, module_path)
                    };
                let Some(target_file) = target_file else {
                    // A non-relative imported-local claim must not disappear
                    // from a star barrel when this projection cannot resolve it.
                    if nonrelative_forward {
                        return ExportLookup::BlockedClaim;
                    }
                    return ExportLookup::NoTarget;
                };
                let result = resolve_one(
                    raw,
                    resolver,
                    &target_file,
                    imported,
                    hops + 1,
                    visited,
                    telemetry,
                );
                // Every failed recursive exit retains a newly admitted claim:
                // absent member/facts, empty stars, cycles and the depth bound.
                if nonrelative_forward && matches!(result, ExportLookup::NoTarget) {
                    return ExportLookup::BlockedClaim;
                }
                let ExportLookup::Resolved(hit, is_class) = result else {
                    return result;
                };
                if matches!(target, JsExportTarget::ImportForward { .. })
                    && (is_class
                        || !legacy_export_facts(raw, &hit.file).is_some_and(|f| {
                            f.forwardable_function_locals.contains(&hit.local_name)
                        }))
                {
                    return ExportLookup::BlockedClaim;
                }
                ExportLookup::Resolved(hit, is_class)
            }
        };
    }

    // Not a direct named export: fall back to `export * from` barrels.
    // `export *` never re-exports `default` (ES module semantics).
    if name == "default" || facts.star_reexports.is_empty() {
        return ExportLookup::NoTarget;
    }
    if hops + 1 > MAX_REEXPORT_DEPTH {
        telemetry.chain_unresolved += 1;
        return ExportLookup::NoTarget;
    }
    // The span is part of the key: two claims on one `(file, local)` with different
    // spans are different targets (S1).
    let mut candidates: BTreeSet<BarrelCandidate> = BTreeSet::new();
    for module_path in &facts.star_reexports {
        let Some(target_file) = (resolver.module)(file, module_path) else {
            continue;
        };
        // Fork the visited set per barrel branch: sibling barrels shouldn't
        // spuriously "cycle" each other out, only a genuine repeated
        // (file, name) on ONE path should.
        let mut branch_visited = visited.clone();
        match resolve_one(
            raw,
            resolver,
            &target_file,
            name,
            hops + 1,
            &mut branch_visited,
            telemetry,
        ) {
            ExportLookup::Resolved(hit, is_class) => {
                candidates.insert((hit.file, hit.local_name, is_class, hit.span, hit.wrapped));
            }
            ExportLookup::BlockedClaim => return ExportLookup::BlockedClaim,
            ExportLookup::NoTarget => {}
        }
    }
    match candidates.len() {
        0 => ExportLookup::NoTarget,
        1 => {
            let (file, local_name, is_class, span, wrapped) =
                candidates.into_iter().next().unwrap();
            ExportLookup::Resolved(
                ResolvedJsExport {
                    file,
                    local_name,
                    span,
                    via_unresolved_star: false,
                    wrapped,
                },
                is_class,
            )
        }
        _ => {
            telemetry.barrel_conflicts += 1;
            ExportLookup::BlockedClaim
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn facts(named: &[(&str, JsExportTarget)], star: &[&str]) -> JsExportFacts {
        JsExportFacts {
            qualifiers: Default::default(),
            namespace_proof_complete: true,
            namespace_callable_locals: BTreeMap::new(),
            module_value_bindings: BTreeSet::new(),
            forwardable_function_locals: BTreeSet::new(),
            esm_named_imports: BTreeSet::new(),
            type_only_imports: BTreeMap::new(),
            named: named
                .iter()
                .map(|(k, v)| (k.to_string(), v.clone()))
                .collect(),
            star_reexports: star.iter().map(|s| s.to_string()).collect(),
            skipped_expr_count: 0,
            conflicted: BTreeSet::new(),
            spanned_admitted: 0,
            skipped_decl_reasons: BTreeMap::new(),
            local_export_refusals: BTreeMap::new(),
            local_export_may_call: 0,
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
        raw.insert(
            "util.ts".to_string(),
            facts(&[("process", local("process"))], &[]),
        );
        let out = resolve_js_exports(&raw, &resolve_dot);
        assert_eq!(
            out.resolved["util.ts"]["process"],
            ResolvedJsExport {
                file: "util.ts".to_string(),
                local_name: "process".to_string(),
                span: None,
                via_unresolved_star: false,
                wrapped: false,
            }
        );
        assert_eq!(out.chain_unresolved, 0);
        assert_eq!(out.barrel_conflicts, 0);
    }

    #[test]
    fn one_hop_reexport_resolves() {
        let mut raw = BTreeMap::new();
        raw.insert(
            "impl.ts".to_string(),
            facts(&[("process", local("process"))], &[]),
        );
        raw.insert(
            "index.ts".to_string(),
            facts(&[("process", reexport("./impl", "process"))], &[]),
        );
        let out = resolve_js_exports(&raw, &resolve_dot);
        assert_eq!(
            out.resolved["index.ts"]["process"],
            ResolvedJsExport {
                file: "impl.ts".to_string(),
                local_name: "process".to_string(),
                span: None,
                via_unresolved_star: false,
                wrapped: false,
            }
        );
        assert_eq!(out.chain_unresolved, 0);
    }

    #[test]
    fn two_hop_reexport_resolves_at_depth_bound() {
        let mut raw = BTreeMap::new();
        raw.insert(
            "impl.ts".to_string(),
            facts(&[("process", local("process"))], &[]),
        );
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
                local_name: "process".to_string(),
                span: None,
                via_unresolved_star: false,
                wrapped: false,
            }
        );
        assert_eq!(out.chain_unresolved, 0);
    }

    #[test]
    fn three_hop_reexport_fails_closed() {
        let mut raw = BTreeMap::new();
        raw.insert(
            "impl.ts".to_string(),
            facts(&[("process", local("process"))], &[]),
        );
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
        raw.insert(
            "impl.ts".to_string(),
            facts(&[("process", local("process"))], &[]),
        );
        raw.insert("index.ts".to_string(), facts(&[], &["./impl"]));
        let out = resolve_js_exports(&raw, &resolve_dot);
        assert_eq!(
            out.resolved["index.ts"]["process"],
            ResolvedJsExport {
                file: "impl.ts".to_string(),
                local_name: "process".to_string(),
                span: None,
                wrapped: false,
                via_unresolved_star: false,
            }
        );
    }

    #[test]
    fn star_reexport_never_surfaces_default() {
        let mut raw = BTreeMap::new();
        raw.insert(
            "impl.ts".to_string(),
            facts(&[("default", local("process"))], &[]),
        );
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
        raw.insert(
            "a.ts".to_string(),
            facts(&[("process", local("process"))], &[]),
        );
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
        raw.insert(
            "impl.ts".to_string(),
            facts(&[("process", local("process"))], &[]),
        );
        raw.insert(
            "reexport_a.ts".to_string(),
            facts(&[("process", reexport("./impl", "process"))], &[]),
        );
        raw.insert(
            "index.ts".to_string(),
            facts(&[], &["./impl", "./reexport_a"]),
        );
        let out = resolve_js_exports(&raw, &resolve_dot);
        assert_eq!(
            out.resolved["index.ts"]["process"],
            ResolvedJsExport {
                file: "impl.ts".to_string(),
                local_name: "process".to_string(),
                span: None,
                wrapped: false,
                via_unresolved_star: false,
            }
        );
        assert_eq!(out.barrel_conflicts, 0);
    }

    /// S1b-2b (§7 B-11): `wrapped` is part of a barrel candidate's identity. The same
    /// `(file, local, span)` claimed wrapped and plain is two targets (fail closed); a single
    /// wrapped claim keeps `wrapped` through the barrel.
    #[test]
    fn barrel_candidate_key_includes_wrapped() {
        let span = |wrapped: bool| {
            let (local, start_line, end_line) = ("g".to_string(), 1, 1);
            match wrapped {
                true => JsExportTarget::SpannedLocal {
                    local,
                    start_line,
                    end_line,
                },
                false => JsExportTarget::VerifiedLocal {
                    local,
                    start_line,
                    end_line,
                },
            }
        };
        let mut raw = BTreeMap::new();
        raw.insert(
            "impl.ts".into(),
            facts(&[("W", span(true)), ("P", span(false))], &[]),
        );
        raw.insert("a.ts".into(), facts(&[("X", reexport("./impl", "W"))], &[]));
        raw.insert("b.ts".into(), facts(&[("X", reexport("./impl", "P"))], &[]));
        raw.insert("index.ts".into(), facts(&[], &["./a", "./b"]));
        raw.insert("one.ts".into(), facts(&[], &["./a"]));
        let out = resolve_js_exports(&raw, &resolve_dot);
        assert!(!out
            .resolved
            .get("index.ts")
            .is_some_and(|m| m.contains_key("X")));
        assert_eq!(out.barrel_conflicts, 1);
        assert!(out.resolved["one.ts"]["X"].wrapped);
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

    // F3: a name marked `conflicted` must resolve to nothing, counted via
    // the (reused) `barrel_conflicts` counter.
    #[test]
    fn conflicted_name_fails_closed() {
        let mut raw = BTreeMap::new();
        let mut f = JsExportFacts::default();
        f.conflicted.insert("process".to_string());
        raw.insert("util.ts".to_string(), f);
        let out = resolve_js_exports(&raw, &resolve_dot);
        assert!(!out
            .resolved
            .get("util.ts")
            .is_some_and(|m| m.contains_key("process")));
        assert_eq!(out.barrel_conflicts, 1);
    }

    #[test]
    fn insert_named_poisons_on_second_insertion() {
        let mut f = JsExportFacts::default();
        f.insert_named("f".to_string(), local("a"));
        assert_eq!(f.named.get("f"), Some(&local("a")));
        f.insert_named("f".to_string(), local("b"));
        assert!(!f.named.contains_key("f"));
        assert!(f.conflicted.contains("f"));
        // A third insertion is a no-op on an already-poisoned name.
        f.insert_named("f".to_string(), local("c"));
        assert!(!f.named.contains_key("f"));
    }

    // F5: a star-only chain needing a 3rd hop to discover its candidate
    // names must still count `chain_unresolved` (mirrors
    // `three_hop_reexport_fails_closed` above).
    #[test]
    fn three_hop_star_only_chain_fails_closed_and_counts() {
        let mut raw = BTreeMap::new();
        raw.insert(
            "impl.ts".to_string(),
            facts(&[("process", local("process"))], &[]),
        );
        raw.insert("mid2.ts".to_string(), facts(&[], &["./impl"]));
        raw.insert("mid.ts".to_string(), facts(&[], &["./mid2"]));
        raw.insert("index.ts".to_string(), facts(&[], &["./mid"]));
        let out = resolve_js_exports(&raw, &resolve_dot);
        assert!(!out.resolved.contains_key("index.ts"));
        assert!(out.chain_unresolved > 0);
    }

    #[test]
    fn namespace_forward_uses_raw_literal() {
        let terminal = ResolvedJsExport {
            file: "leaf.ts".into(),
            local_name: "real".into(),
            span: Some((1, 1)),
            wrapped: false,
            via_unresolved_star: false,
        };
        let mut leaf = facts(
            &[(
                "real",
                JsExportTarget::VerifiedLocal {
                    local: "real".into(),
                    start_line: 1,
                    end_line: 1,
                },
            )],
            &[],
        );
        leaf.namespace_callable_locals
            .insert("real".into(), vec![terminal.clone()]);
        // Model the legacy helper's trim; the raw fact must control admission.
        let resolve = |_: &str, spec: &str| (spec.trim() == "./leaf").then(|| "leaf.ts".into());
        for spec in ["./leaf", " ./leaf", "./leaf "] {
            for forward in [true, false] {
                let target = if forward {
                    JsExportTarget::ImportForward {
                        module_path: spec.into(),
                        imported: "real".into(),
                    }
                } else {
                    reexport(spec, "real")
                };
                let raw = BTreeMap::from([
                    ("barrel.ts".into(), facts(&[("real", target)], &[])),
                    ("leaf.ts".into(), leaf.clone()),
                ]);
                let out = resolve_js_namespace_exports(&raw, &resolve);
                let hit = out.get("barrel.ts").and_then(|e| e.get("real"));
                assert_eq!(
                    hit,
                    (!forward || spec == "./leaf").then_some(&terminal),
                    "{spec:?} forward={forward}"
                );
            }
        }
    }
}
