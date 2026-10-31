//! Removal-only Rust external-prefix proof. No source scanning or new facts:
//! imports, visibility, shadowing and module boundaries use the scope engine.
use crate::call_graph::{CallGraph, CallSite};
use crate::name_resolution::engine::{resolve, resolve_path};
use crate::name_resolution::graph::ScopeGraph;
use crate::name_resolution::rust_policy::{RustPolicy, NS_TYPE};
use crate::name_resolution::rust_populator::enclosing_scope;
use crate::name_resolution::types::*;
use crate::resolution::{DropReason, ResolutionOutcome};

pub(crate) fn resolve_call<'a>(
    cg: &'a CallGraph,
    site: &CallSite,
) -> Option<ResolutionOutcome<'a>> {
    refuse_call(cg, site).or_else(|| cg.rust_authoritative_resolution(site))
}

pub(crate) fn refuse_call<'a>(cg: &'a CallGraph, site: &CallSite) -> Option<ResolutionOutcome<'a>> {
    if crate::languages::Language::from_path(&site.caller.file)
        != Some(crate::languages::Language::Rust)
    {
        return None;
    }
    if external_constructor_receiver(cg, site) {
        return Some(ResolutionOutcome::dropped(DropReason::ExternalReceiver));
    }
    if !site.callee_name.contains("::") {
        return None;
    }
    let graph = cg.scope_graph.as_ref().filter(|g| g.complete)?;
    let file = *graph.file_paths.get(&site.caller.file)?;
    let from = enclosing_scope(graph, file, site.start_byte)?;
    let owner = site.callee_name.rsplit_once("::")?.0;
    let external = external_path(
        graph,
        from,
        &SourceLoc {
            file,
            byte: site.start_byte,
        },
        owner,
    );
    external.then(|| ResolutionOutcome::dropped(DropReason::ImportExternal))
}

// A recovered constructor must not acquire a new extension-method edge merely
// because its identity changed from Bare/InRepo to External. Refuse at the same
// routing boundary, using the local fact at its definition (not the use site).
fn external_constructor_receiver(cg: &CallGraph, site: &CallSite) -> bool {
    use crate::name_resolution::binding_lookup::{lookup_visible_binding, InitExpr};
    let check = || {
        let graph = cg.scope_graph.as_ref().filter(|g| g.complete)?;
        let file = *graph.file_paths.get(&site.caller.file)?;
        let binding =
            lookup_visible_binding(graph, file, site.start_byte, site.qualifier.as_deref()?)?;
        let byte = binding.vis_extents.first()?.lo.byte;
        let InitExpr::Ctor(expr) = graph.local_facts.get(&(file, byte))?.init.as_ref()? else {
            return None;
        };
        let syntax = if let Some(function) = expr.strip_suffix("()") {
            function.rsplit_once("::")?.0
        } else {
            expr.strip_suffix("{}")?
        };
        let from = enclosing_scope(graph, file, byte)?;
        Some(external_path(
            graph,
            from,
            &SourceLoc { file, byte },
            syntax,
        ))
    };
    check().unwrap_or(false)
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

// Impl scopes/owner keys are insufficient: only a declaration binding proves
// that the selected external owner has an in-repo look-alike. Incomplete graphs
// deliberately retain main's behavior; do not extract missing declarations.
fn declared_owner(graph: &ScopeGraph, path: &str) -> bool {
    let owner = crate::resolution::owner_key(path);
    graph.bindings.iter().any(|b| {
        b.ns == NS_TYPE
            && b.name == owner
            && match &b.target {
                BindTarget::Resolved(Target::Item {
                    callable: false, ..
                }) => true,
                BindTarget::Resolved(Target::Scope(id)) => graph
                    .scopes
                    .get(id)
                    .is_some_and(|s| matches!(s.kind, ScopeKind::Module | ScopeKind::Root)),
                _ => false,
            }
    })
}

/// Is this owner path external and also declared in-repo?
/// A specialized policy gives already-extracted std imports an external target;
/// the ordinary engine still chooses the rib and combines conflicts. This probe
/// never supplies candidates to call resolution, so it cannot promote an edge.
pub(crate) fn external_path(graph: &ScopeGraph, from: ScopeId, at: &SourceLoc, path: &str) -> bool {
    if !graph.complete {
        return false;
    }
    if std_root(path) {
        return declared_owner(graph, path);
    }
    let first = path.split("::").next().unwrap_or("").trim();
    if first.is_empty() || matches!(first, "crate" | "self" | "super" | "Self") {
        return false;
    }
    let q = ResolveQuery {
        name: first.to_string(),
        ns: NS_TYPE,
        from,
        at: at.clone(),
        cfg: CfgCtx::default(),
        ctx: PolicyQueryCtx::default(),
    };
    let policy = ExternalImports {
        graph,
        rust: RustPolicy::new(graph, graph.edition),
    };
    let res = resolve(graph, &q, &policy);
    matches!(res.status, ResStatus::Resolved | ResStatus::ResolvedSet)
        && !res.candidates.is_empty()
        && res.candidates.iter().all(|c| match &c.target {
            Target::External(ext) => {
                let suffix = path.split_once("::").map(|(_, tail)| tail);
                let owner =
                    suffix.map_or_else(|| ext.name.clone(), |tail| format!("{}::{tail}", ext.name));
                // An extern-crate alias can itself be the selected owner key
                // (`use outside as CliTest; CliTest::m()`), unlike a renamed
                // std type whose canonical name supplies declaration evidence.
                declared_owner(graph, &owner)
                    || (!std_root(&ext.name) && declared_owner(graph, path))
            }
            _ => false,
        })
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
        let external = std_root(first) || {
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
            matches!(res.status, ResStatus::Resolved)
                && matches!(
                    res.candidates.as_slice(),
                    [Candidate {
                        target: Target::External(_),
                        ..
                    }]
                )
        };
        external.then(|| {
            Target::External(ExternRef {
                name: path.0.join("::"),
            })
        })
    }
}

/// Constructor typing must retain external identity, rather than using a bare
/// same-name owner. Routing will drop unless an existing extension route applies.
pub(crate) fn constructor_type(
    graph: &ScopeGraph,
    file: FileId,
    byte: usize,
    syntax: &str,
) -> Option<crate::resolution_identity::ReceiverOutcome> {
    let from = enclosing_scope(graph, file, byte)?;
    external_path(graph, from, &SourceLoc { file, byte }, syntax).then(|| {
        let bare = crate::resolution::owner_key(syntax);
        crate::resolution_identity::ReceiverOutcome {
            key: crate::resolution_identity::ReceiverTypeKey::External(bare.clone()),
            bare,
            recovery: crate::resolution::ReceiverRecovery::ConstructorLocal,
        }
    })
}
