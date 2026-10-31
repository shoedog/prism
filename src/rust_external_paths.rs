//! Removal-only Rust external-prefix proof over existing scope facts.
use crate::call_graph::{CallGraph, CallSite, FunctionId};
use crate::name_resolution::engine::{resolve, resolve_path};
use crate::name_resolution::graph::ScopeGraph;
use crate::name_resolution::rust_policy::{crate_root_of, RustPolicy, NS_TYPE};
use crate::name_resolution::rust_populator::enclosing_scope;
use crate::name_resolution::types::*;
use crate::resolution::{DropReason, ReceiverRecovery, ResolutionOutcome};
use std::collections::{BTreeMap, BTreeSet};
use std::sync::OnceLock;

impl CallGraph {
    // Generate main's candidates first. Filtering whole sites before this point
    // loses impl-for-std targets; rebuilding a reduced pool would promote it.
    pub(crate) fn resolve_call_site_candidates_with_go_memo<'a>(
        &'a self,
        site: &CallSite,
        memo: Option<&crate::resolution::go_satisfiers::GoSatisfierMemo<'a>>,
    ) -> ResolutionOutcome<'a> {
        let mut out = self.resolve_call_site_candidates_unfiltered(site, memo);
        let Some((owners, reason)) = external_owners(self, site) else {
            return out;
        };
        let graph = self
            .scope_graph
            .as_ref()
            .expect("external proof has scope graph");
        let index = graph.declared_owners.get(graph);
        let before = out.resolved.len();
        out.resolved
            .retain(|r| !index.target_declares(graph, r.target, &owners));
        if out.resolved.is_empty()
            && (before != 0 || owners.iter().any(|owner| index.names.contains(owner)))
        {
            out.drop = Some(reason);
        }
        out
    }
}

// Cached once per immutable graph; clones and deserialization start cold.
#[derive(Debug, Default)]
pub struct GraphDeclaredOwners(OnceLock<DeclaredOwners>);
impl Clone for GraphDeclaredOwners {
    fn clone(&self) -> Self {
        Self::default()
    }
}
impl PartialEq for GraphDeclaredOwners {
    fn eq(&self, _: &Self) -> bool {
        true
    }
}
impl Eq for GraphDeclaredOwners {}
impl GraphDeclaredOwners {
    pub(crate) fn invalidate(&mut self) {
        self.0.take();
    }
    fn get<'a>(&'a self, graph: &ScopeGraph) -> &'a DeclaredOwners {
        self.0.get_or_init(|| DeclaredOwners::build(graph))
    }
}

#[derive(Debug, Default)]
struct DeclaredOwners {
    by_root: BTreeMap<ScopeId, BTreeSet<String>>,
    roots_by_file: BTreeMap<FileId, BTreeSet<ScopeId>>,
    names: BTreeSet<String>,
    binding_count: usize,
    scope_count: usize,
}
impl DeclaredOwners {
    fn build(graph: &ScopeGraph) -> Self {
        let mut index = Self {
            binding_count: graph.bindings.len(),
            scope_count: graph.scopes.len(),
            ..Self::default()
        };
        for scope in graph.scopes.values() {
            if let Some(root) = crate_root_of(graph, scope.id) {
                for extent in &scope.extents {
                    index
                        .roots_by_file
                        .entry(extent.range.lo.file)
                        .or_default()
                        .insert(root);
                }
            }
        }
        for b in &graph.bindings {
            if b.ns != NS_TYPE {
                continue;
            }
            let declaration = match &b.target {
                BindTarget::Resolved(Target::Item {
                    callable: false,
                    owns,
                    ..
                }) => {
                    // Generic barriers have no owned type scope and live in
                    // Callable ribs. Actual local structs/traits still count.
                    owns.is_some()
                        || graph
                            .scopes
                            .get(&b.scope)
                            .is_some_and(|s| s.kind != ScopeKind::Callable)
                }
                BindTarget::Resolved(Target::Scope(id)) => graph
                    .scopes
                    .get(id)
                    .is_some_and(|s| matches!(s.kind, ScopeKind::Module | ScopeKind::Root)),
                _ => false,
            };
            if declaration {
                if let Some(root) = crate_root_of(graph, b.scope) {
                    index
                        .by_root
                        .entry(root)
                        .or_default()
                        .insert(b.name.clone());
                    index.names.insert(b.name.clone());
                }
            }
        }
        index
    }
    fn target_declares(&self, graph: &ScopeGraph, target: &FunctionId, owners: &[String]) -> bool {
        // Direct public appends bypass invalidation. Fail open on drift; normal
        // builders invalidate on every add, and in-place edits use the graph API.
        if self.binding_count != graph.bindings.len() || self.scope_count != graph.scopes.len() {
            return false;
        }
        let Some(roots) = graph
            .file_paths
            .get(&target.file)
            .and_then(|f| self.roots_by_file.get(f))
        else {
            return false;
        };
        if roots.len() != 1 {
            return false;
        }
        roots.iter().any(|root| {
            self.by_root
                .get(root)
                .is_some_and(|names| owners.iter().any(|owner| names.contains(owner)))
        })
    }
}

fn external_owners(cg: &CallGraph, site: &CallSite) -> Option<(Vec<String>, DropReason)> {
    if crate::languages::Language::from_path(&site.caller.file)
        != Some(crate::languages::Language::Rust)
    {
        return None;
    }
    let graph = cg.scope_graph.as_ref().filter(|g| g.complete)?;
    let file = *graph.file_paths.get(&site.caller.file)?;
    if let Some((owner, byte)) = constructor_owner(graph, file, site) {
        let from = enclosing_scope(graph, file, byte)?;
        if let Some(owners) = external_path(graph, from, &SourceLoc { file, byte }, &owner) {
            return Some((owners, DropReason::ExternalReceiver));
        }
    }
    let owner = site.callee_name.rsplit_once("::")?.0;
    let from = enclosing_scope(graph, file, site.start_byte)?;
    external_path(
        graph,
        from,
        &SourceLoc {
            file,
            byte: site.start_byte,
        },
        owner,
    )
    .map(|owners| (owners, DropReason::ImportExternal))
}

// This is the only constructor refusal rule. Annotation precedence and pattern
// binding semantics remain those of the receiver typer; do not change its key.
fn constructor_owner(graph: &ScopeGraph, file: FileId, site: &CallSite) -> Option<(String, usize)> {
    use crate::name_resolution::binding_lookup::{lookup_visible_binding, BindingKind, InitExpr};
    if site.receiver_outcome.as_ref()?.recovery != ReceiverRecovery::ConstructorLocal {
        return None;
    }
    let binding = lookup_visible_binding(graph, file, site.start_byte, site.qualifier.as_deref()?)?;
    let byte = binding.vis_extents.first()?.lo.byte;
    let fact = graph.local_facts.get(&(file, byte))?;
    if fact.kind != BindingKind::Let || fact.annotation.is_some() {
        return None;
    }
    let InitExpr::Ctor(expr) = fact.init.as_ref()? else {
        return None;
    };
    let owner = if let Some(function) = expr.strip_suffix("()") {
        function.rsplit_once("::")?.0
    } else {
        expr.strip_suffix("{}")?
    };
    Some((owner.to_string(), byte))
}

fn std_root(path: &str) -> bool {
    matches!(
        path.trim()
            .trim_start_matches("::")
            .split("::")
            .next()
            .map(str::trim)
            .map(|head| head.strip_prefix("r#").unwrap_or(head)),
        Some("std" | "core" | "alloc")
    )
}

// Return both canonical and source-selected owner keys. Aliases select the
// latter in main's fallback pool, including aliases of std types.
fn external_path(
    graph: &ScopeGraph,
    from: ScopeId,
    at: &SourceLoc,
    path: &str,
) -> Option<Vec<String>> {
    if !graph.complete {
        return None;
    }
    let absolute = path.trim().starts_with("::");
    let first = path
        .trim()
        .trim_start_matches("::")
        .split("::")
        .next()?
        .trim();
    if first.is_empty() || matches!(first, "crate" | "self" | "super" | "Self") {
        return None;
    }
    if std_root(path) && absolute && graph.edition >= 2018 {
        return Some(vec![crate::resolution::owner_key(path)]);
    }
    let rust = RustPolicy::new(graph, graph.edition);
    let policy = ExternalImports { graph, rust };
    let res = if absolute {
        resolve_path(
            graph,
            &RawPath(vec![first.to_string()]),
            NS_TYPE,
            &Anchor {
                kind: AnchorKind::LeadingColon,
                prelude: None,
            },
            from,
            NS_TYPE,
            at,
            &policy,
        )
    } else {
        resolve(
            graph,
            &ResolveQuery {
                name: first.to_string(),
                ns: NS_TYPE,
                from,
                at: at.clone(),
                cfg: CfgCtx::default(),
                ctx: PolicyQueryCtx::default(),
            },
            &policy,
        )
    };
    // A local root wins over the standard-library extern prelude. Ambiguous or
    // poisoned roots are not proof of an external selection either.
    if std_root(path) && res.status == ResStatus::Unresolved {
        return Some(vec![crate::resolution::owner_key(path)]);
    }
    if !matches!(res.status, ResStatus::Resolved | ResStatus::ResolvedSet)
        || res.candidates.is_empty()
    {
        return None;
    }
    let mut owners = vec![crate::resolution::owner_key(path)];
    for candidate in res.candidates {
        let Target::External(ext) = candidate.target else {
            return None;
        };
        let canonical = path.split_once("::").map_or(ext.name.clone(), |(_, tail)| {
            format!("{}::{tail}", ext.name)
        });
        owners.push(crate::resolution::owner_key(&canonical));
    }
    owners.sort();
    owners.dedup();
    Some(owners)
}

struct ExternalImports<'g> {
    graph: &'g ScopeGraph,
    rust: RustPolicy<'g>,
}

impl ResolutionPolicy for ExternalImports<'_> {
    fn namespaces(&self) -> Vec<NamespaceId> {
        self.rust.namespaces()
    }
    fn edge_order(&self) -> Vec<EdgeKindId> {
        self.rust.edge_order()
    }
    fn combine(&self, candidates: Vec<Candidate>) -> Resolution {
        self.rust.combine(candidates)
    }
    fn visible(&self, b: &Binding, q: &ResolveQuery, t: &TraversalCtx) -> bool {
        self.rust.visible(b, q, t)
    }
    fn member_visible(
        &self,
        b: &Binding,
        q: &ResolveQuery,
        t: &TraversalCtx,
    ) -> VisibilityDecision {
        self.rust.member_visible(b, q, t)
    }
    fn glob_edge_visible(
        &self,
        e: &Edge,
        q: &ResolveQuery,
        t: &TraversalCtx,
    ) -> VisibilityDecision {
        self.rust.glob_edge_visible(e, q, t)
    }
    fn anchor(&self, a: &Anchor, from: ScopeId) -> Option<(ScopeId, NamespaceId)> {
        self.rust.anchor(a, from)
    }
    fn ascend_to_parent(&self, k: &ScopeKind) -> bool {
        self.rust.ascend_to_parent(k)
    }
    fn extern_crate_root(
        &self,
        g: &ScopeGraph,
        n: &str,
        a: &Anchor,
        from: ScopeId,
    ) -> Option<ScopeId> {
        self.rust.extern_crate_root(g, n, a, from)
    }
    fn glob_anchor_expands(&self, a: &Anchor) -> bool {
        self.rust.glob_anchor_expands(a)
    }
    fn pending_target(&self, b: &Binding, at: &SourceLoc) -> Option<Target> {
        let BindTarget::Pending(path, anchor) = &b.target else {
            return None;
        };
        if !matches!(anchor.kind, AnchorKind::UsePath | AnchorKind::LeadingColon) {
            return None;
        }
        let first = path.0.first()?;
        let external = {
            // Existing explicit `extern crate` targets are authoritative. Cargo
            // registry/git dependency names are not recorded by this model.
            let prefix = RawPath(vec![first.clone()]);
            let res = if matches!(anchor.kind, AnchorKind::UsePath) && self.graph.edition >= 2018 {
                resolve(
                    self.graph,
                    &ResolveQuery {
                        name: first.clone(),
                        ns: NS_TYPE,
                        from: b.scope,
                        at: at.clone(),
                        cfg: CfgCtx::default(),
                        ctx: PolicyQueryCtx::default(),
                    },
                    &self.rust,
                )
            } else {
                resolve_path(
                    self.graph, &prefix, NS_TYPE, anchor, b.scope, NS_TYPE, at, &self.rust,
                )
            };
            // 2015 use/absolute paths are crate-root anchored; 2018+ use
            // paths are lexical, while leading :: selects the extern prelude.
            let absolute_extern =
                matches!(anchor.kind, AnchorKind::LeadingColon) && self.graph.edition >= 2018;
            (std_root(first) && (absolute_extern || res.status == ResStatus::Unresolved))
                || (res.status == ResStatus::Resolved
                    && matches!(
                        res.candidates.as_slice(),
                        [Candidate {
                            target: Target::External(_),
                            ..
                        }]
                    ))
        };
        external.then(|| {
            Target::External(ExternRef {
                name: path.0.join("::"),
            })
        })
    }
}
