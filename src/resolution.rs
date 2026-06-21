//! S3 call-resolution: confidence types, owner-key normalization, and the
//! R1-R7 resolution ladder (impl on CallGraph lives here to keep
//! call_graph.rs under the size cap).

use crate::call_graph::{CallGraph, CallSite, FunctionId, MethodArity, MethodFacts, MethodKind};
use crate::name_resolution::consumer::graph_callable_edge;
use crate::name_resolution::engine::resolve_path;
use crate::name_resolution::graph::ScopeGraph;
use crate::name_resolution::rust_policy::{RustPolicy, EK_GLOB, NS_TYPE, NS_VALUE};
use crate::name_resolution::rust_populator::enclosing_scope;
use crate::name_resolution::types::{
    Anchor, AnchorKind, BindTarget, Binding, Candidate, Edge, FileId, RawPath, ResStatus,
    ResolutionPolicy, ResolveQuery, ScopeId, SourceLoc, Span, Target, TraversalCtx,
};
use std::collections::{BTreeMap, BTreeSet};

pub use crate::resolution_disproof::{prune, DisproofCx, DisproofPredicate};
pub use crate::resolution_identity::{
    canonical_external, resolve_type_path_to_type_scope, ReceiverOutcome, ReceiverTypeKey, TypeKey,
};
pub use crate::resolution_receiver::{ReceiverTypeCtx, RustReceiverTyper};

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
pub enum ResolutionConfidence {
    Exact,
    NameOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolutionKind {
    StaticLinkage,
    QualifiedOwner,
    SelfReceiver,
    ImportQualified,
    QualifierOwner,
    LocalDef,
    SamePackage,
    ImplicitThis,
    FreeSingle,
    FreeMulti,
    TypedParam,
    ConstructorLocal,
    TraitCha,
    R6SingleOwner,
    StemSingle,
    StemMulti,
    EmbeddedPromotion,
    InterfaceDispatch,
}

impl ResolutionKind {
    pub fn as_str(&self) -> &'static str {
        match self {
            ResolutionKind::StaticLinkage => "static_linkage",
            ResolutionKind::QualifiedOwner => "qualified_owner",
            ResolutionKind::SelfReceiver => "self_receiver",
            ResolutionKind::ImportQualified => "import_qualified",
            ResolutionKind::QualifierOwner => "qualifier_owner",
            ResolutionKind::LocalDef => "local_def",
            ResolutionKind::SamePackage => "same_package",
            ResolutionKind::ImplicitThis => "implicit_this",
            ResolutionKind::FreeSingle => "free_single",
            ResolutionKind::FreeMulti => "free_multi",
            ResolutionKind::TypedParam => "typed_param",
            ResolutionKind::ConstructorLocal => "constructor_local",
            ResolutionKind::TraitCha => "trait_cha",
            ResolutionKind::R6SingleOwner => "r6_single_owner",
            ResolutionKind::StemSingle => "stem_single",
            ResolutionKind::StemMulti => "stem_multi",
            ResolutionKind::EmbeddedPromotion => "embedded_promotion",
            ResolutionKind::InterfaceDispatch => "interface_dispatch",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedCallee<'a> {
    pub target: &'a FunctionId,
    pub confidence: ResolutionConfidence,
    pub kind: ResolutionKind,
}

/// A resolved caller edge: who calls the seed, with what confidence, at which line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedCallEdge {
    pub caller: FunctionId,
    pub call_site_line: usize,
    pub confidence: ResolutionConfidence,
}

/// Normalize an owner type's source text to its bare index key:
/// strip refs/pointers, smart-pointer wrappers are NOT peeled here (that is
/// receiver peeling, `peel_type`), strip generic args, strip `dyn `/`impl `.
pub fn owner_key(text: &str) -> String {
    let t = text.trim();
    let t = t.trim_start_matches("&mut ").trim_start_matches('&');
    let t = t.trim_start_matches('*');
    let t = t.trim_start_matches("dyn ").trim_start_matches("impl ");
    let t = t.split('<').next().unwrap_or(t);
    // C++ out-of-line `ns::Foo` declarator prefix -> last segment.
    let t = t.rsplit("::").next().unwrap_or(t);
    t.trim().to_string()
}

/// Interface lookup key (Go): strip `&`/`*` and a `pkg.` qualifier to the bare
/// interface name. Returns `None` for a generic instantiation (`Foo[T]`), which
/// is non-dispatchable (a recorded gap, never a key) — spec §6/§10.
pub fn iface_key(text: &str) -> Option<String> {
    let t = text
        .trim()
        .trim_start_matches('&')
        .trim_start_matches('*')
        .trim();
    if t.contains('[') {
        return None; // generic instantiation -> gap, not a key
    }
    let bare = t.rsplit('.').next().unwrap_or(t).trim();
    if bare.is_empty() {
        None
    } else {
        Some(bare.to_string())
    }
}

/// Admission key (Go method-set asymmetry): a value-receiver satisfier admits as
/// `T`; a pointer-receiver-only satisfier admits as `*T` (spec §7). Bare `T` must
/// already be normalized (no `pkg.`).
pub fn admission_key(bare_type: &str, is_pointer: bool) -> String {
    if is_pointer {
        format!("*{bare_type}")
    } else {
        bare_type.to_string()
    }
}

/// Arity admission for interface-dispatch candidates (language-neutral; shared by
/// the resolution mint and the `interface_dispatch_manifest`). CONSERVATIVE on
/// purpose — the headline risk is recall loss (dropping a valid dispatch edge), so
/// a candidate is dropped ONLY on a confident exact mismatch and EVERY unknown
/// keeps it:
///   * a variadic candidate is never dropped (`a.variadic`),
///   * a spread call (`arg_spread`) drops nothing,
///   * an unknown `arg_count` (`None`) drops nothing,
///   * a candidate with no recorded `MethodArity` (`m == None`) is never dropped.
/// Only a known, non-spread call against a known, non-variadic method with a
/// different param count is provably-wrong and dropped.
pub fn arity_admits(arg_count: Option<usize>, arg_spread: bool, m: Option<&MethodArity>) -> bool {
    match (arg_count, m) {
        // Drop ONLY on a confident exact mismatch:
        (Some(n), Some(a)) if !arg_spread && !a.variadic => a.params == n,
        // Every unknown keeps the candidate (recall-safe):
        _ => true,
    }
}

/// Filter interface-dispatch candidate ids by call arity, keeping each `id` iff
/// `arity_admits` admits its recorded `MethodArity` (a missing entry → unknown →
/// kept). Order-preserving; borrows the input slice.
pub fn arity_filter<'a>(
    impls: &'a [FunctionId],
    arg_count: Option<usize>,
    arg_spread: bool,
    method_arity: &BTreeMap<FunctionId, MethodArity>,
) -> Vec<&'a FunctionId> {
    impls
        .iter()
        .filter(|id| arity_admits(arg_count, arg_spread, method_arity.get(id)))
        .collect()
}

/// Closed-list syntactic peel (spec section 2.3): refs/pointers and std wrappers,
/// recursively; then generic args; then dyn/impl. NEVER Deref-semantic.
pub fn peel_type(text: &str) -> String {
    let mut t = text.trim();
    loop {
        let before = t;
        t = t
            .trim_start_matches("&mut ")
            .trim_start_matches('&')
            .trim_start_matches("*const ")
            .trim_start_matches("*mut ")
            .trim_start_matches('*')
            .trim();
        // Rust lifetime after a reference: `&'a Sender` / `&'a mut Sender`.
        // Drop a leading `'lifetime` token (and an optional following `mut`).
        if let Some(rest) = t.strip_prefix('\'') {
            let rest = rest.trim_start_matches(|c: char| c.is_alphanumeric() || c == '_');
            t = rest.trim().trim_start_matches("mut ").trim();
        }
        for w in ["Box", "Arc", "Rc", "Pin"] {
            if let Some(inner) = t.strip_prefix(w) {
                if let Some(inner) = inner.trim().strip_prefix('<') {
                    if let Some(inner) = inner.strip_suffix('>') {
                        t = inner.trim();
                    }
                }
            }
        }
        if t == before {
            break;
        }
    }
    let t = t.trim_start_matches("dyn ").trim_start_matches("impl ");
    t.split('<').next().unwrap_or(t).trim().to_string()
}

/// Which syntactic fact recovered a receiver type (stored on CallSite).
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ReceiverRecovery {
    TypedParam,
    ConstructorLocal,
    /// Go type assertion: `x.(T).M()` — `T` is the statically-asserted type.
    TypeAssertion,
    /// Go `var r T` declaration — `T` is the declared type of the local.
    VarDecl,
    /// Reserved (spec §5/§10): interface-slice element receiver, e.g.
    /// `for _, r := range xs { r.M() }`. The classifier returns `None` for it
    /// (sketched only); the variant exists so the wire/manifest shape is settled.
    SliceElem,
    FieldTyped,
    ReturnTyped,
    StdWrapperPeel,
    TypedLet,
}

/// S3 receiver-recovery: a syntactically-recovered static receiver type plus the
/// fact that recovered it. Routing (owner_lookup → interface_impls → drop) happens
/// downstream in `resolve_call_site` (spec §2 recover-and-route); this is recovery only.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RecoveredReceiver {
    pub static_type: String,
    pub recovery: ReceiverRecovery,
}

/// Inputs a `ReceiverClassifier` needs to recover a receiver's static type. Borrows
/// from the ParsedFile/tree of the call's enclosing function. Carries `recv_var` +
/// `file_imports` because the legacy gate tests `is_recv`/`is_import`
/// (call_graph.rs). Recover-and-route needs NO GoTypeProvider here.
#[derive(Clone, Copy)]
pub struct ReceiverCtx<'a> {
    /// Receiver/selector-operand node (e.g. the `type_assertion_expression` in
    /// `x.(Module).M()`). `None` on the manual-fallback path / unqualified calls.
    pub receiver_expr: Option<tree_sitter::Node<'a>>,
    /// Qualifier text (e.g. `x` in `x.M()`), as today.
    pub qualifier: Option<&'a str>,
    /// Enclosing function node.
    pub fn_node: tree_sitter::Node<'a>,
    /// 1-indexed call line.
    pub call_line: usize,
    /// For node_text + the legacy `receiver_type_in_fn` scan.
    pub parsed: &'a crate::ast::ParsedFile,
    /// Go receiver variable of the enclosing method (legacy gate: `is_recv`).
    pub recv_var: Option<&'a str>,
    /// Per-file import map (legacy gate: `is_import`).
    pub file_imports: Option<&'a std::collections::BTreeMap<String, String>>,
}

/// Swappable receiver-recovery strategy (strangler seam, spec §2). `Sync` because
/// the CPG build extracts call sites with rayon (`call_graph.rs` par_iter).
pub trait ReceiverClassifier: Sync {
    fn classify(&self, ctx: ReceiverCtx<'_>) -> Option<RecoveredReceiver>;
}

/// Receiver-recovery mode (spec §13.3). `Expanded` (default) turns the implemented
/// forms on; `Legacy` is the granular fall-back / parity-test mode.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReceiverRecoveryMode {
    Legacy,
    Expanded,
}

/// Build-time receiver-recovery config. Default = `Expanded` with all forms on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ReceiverRecoveryConfig {
    pub mode: ReceiverRecoveryMode,
    pub type_assertion: bool,
    pub var_local: bool,
}

impl Default for ReceiverRecoveryConfig {
    fn default() -> Self {
        Self {
            mode: ReceiverRecoveryMode::Expanded,
            type_assertion: true,
            var_local: true,
        }
    }
}

impl ReceiverRecoveryConfig {
    /// The granular fall-back: PR-1 behavior, no new forms.
    pub fn legacy() -> Self {
        Self {
            mode: ReceiverRecoveryMode::Legacy,
            type_assertion: false,
            var_local: false,
        }
    }

    /// The classifier this config selects (built once per CPG build).
    pub fn classifier(&self) -> Box<dyn ReceiverClassifier> {
        match self.mode {
            ReceiverRecoveryMode::Legacy => Box::new(LegacyClassifier),
            ReceiverRecoveryMode::Expanded => Box::new(ExpandedClassifier {
                type_assertion: self.type_assertion,
                var_local: self.var_local,
            }),
        }
    }
}

/// Inner gate + scan shared by `legacy_recover` and `ExpandedClassifier`.
/// Runs the qualifier/keyword/recv-var/import gate, then the typed-param /
/// constructor-local scan (and optionally `var` declarations when `recover_var`
/// is true), peeled + owner-keyed.
fn recover_simple_ident(ctx: &ReceiverCtx<'_>, recover_var: bool) -> Option<RecoveredReceiver> {
    use crate::languages::Language;
    if !matches!(ctx.parsed.language, Language::Rust | Language::Go) {
        return None;
    }
    let q = ctx.qualifier?;
    let simple = !q.is_empty() && q.chars().all(|c| c.is_alphanumeric() || c == '_');
    let is_kw = matches!(q, "self" | "this" | "cls");
    let is_recv = ctx.recv_var == Some(q);
    let is_import = ctx.file_imports.map(|m| m.contains_key(q)).unwrap_or(false);
    if !(simple && !is_kw && !is_recv && !is_import) {
        return None;
    }
    ctx.parsed
        .receiver_type_in_fn(&ctx.fn_node, q, ctx.call_line, recover_var)
        .map(|(ty, how)| RecoveredReceiver {
            static_type: owner_key(&peel_type(&ty)),
            recovery: how,
        })
}

/// PR-1 P6-lite recovery, extracted verbatim from the former
/// `call_graph::recover_receiver` (the qualifier/keyword/recv-var/import gate, then
/// the typed-param / constructor-local scan, peeled + owner-keyed).
/// Byte-identical to PR-1: `recover_var = false`.
pub fn legacy_recover(ctx: &ReceiverCtx<'_>) -> Option<RecoveredReceiver> {
    recover_simple_ident(ctx, false)
}

/// `legacy` — PR-1 behavior, no new forms.
pub struct LegacyClassifier;
impl ReceiverClassifier for LegacyClassifier {
    fn classify(&self, ctx: ReceiverCtx<'_>) -> Option<RecoveredReceiver> {
        legacy_recover(&ctx)
    }
}

/// `expanded` — `legacy` ∪ the new forms.
pub struct ExpandedClassifier {
    pub type_assertion: bool,
    pub var_local: bool,
}
impl ReceiverClassifier for ExpandedClassifier {
    fn classify(&self, ctx: ReceiverCtx<'_>) -> Option<RecoveredReceiver> {
        if let Some(r) = recover_simple_ident(&ctx, self.var_local) {
            return Some(r);
        }
        if self.type_assertion {
            if let Some(r) = recover_type_assertion(&ctx) {
                return Some(r);
            }
        }
        None
    }
}

/// Recover the statically-asserted type from a Go `x.(T).M()` call.
///
/// The grammar for `type_assertion_expression` (tree-sitter-go §primary):
/// `operand '.' '(' type ')'`. The `type` field is any `_type`, including
/// `parenthesized_type` (`(T)`), `pointer_type` (`*T`), `qualified_type`
/// (`pkg.T`), or `type_identifier` (`T`).
///
/// Normalization:
/// - `parenthesized_type` is unwrapped one level at a time.
/// - `peel_type` strips `*` (pointer); `owner_key` strips the remaining `::` paths.
/// - `pkg.T` is NOT stripped here — `iface_key` handles that at route time in
///   `resolve_call_site` so owner-lookup routes correctly for concrete types too.
///
/// Deferred gap (D2): cross-package concrete `pkg.T` where `T` is not an interface
/// will fail `owner_lookup` (key `"pkg.T"` has no owner entry). Recorded in spec §D2.
fn recover_type_assertion(ctx: &ReceiverCtx<'_>) -> Option<RecoveredReceiver> {
    use crate::languages::Language;
    if ctx.parsed.language != Language::Go {
        return None;
    }
    let node = ctx.receiver_expr?;
    if node.kind() != "type_assertion_expression" {
        return None;
    }
    let mut ty = node.child_by_field_name("type")?;
    // Unwrap parenthesized_type: `(T)` → `T` (handles `x.((Runner)).M()`)
    while ty.kind() == "parenthesized_type" {
        ty = ty.named_child(0)?;
    }
    // Same normalization as the legacy path → consistent routing: an interface
    // (`Runner`/`pkg.Module`) routes via iface_key→interface_impls; a same-package
    // concrete (`*Fast`) owner_lookup-resolves. Cross-package concrete `pkg.T`
    // does NOT owner-resolve (owner_key keeps `pkg.`) — deferred gap D2.
    let static_type = owner_key(&peel_type(ctx.parsed.node_text(&ty)));
    if static_type.is_empty() {
        return None;
    }
    Some(RecoveredReceiver {
        static_type,
        recovery: ReceiverRecovery::TypeAssertion,
    })
}

/// Why a call site resolved to nothing - the classification API that
/// collision warnings and call-stats telemetry consume.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropReason {
    /// R6: method name defined on multiple owner types, receiver unknown.
    MultiOwnerCollision,
    /// P6-lite: receiver type recovered but (T, m) has no entry - provably
    /// external (Vec::truncate class) or wrong-name.
    ExternalReceiver,
    /// R3: qualifier is an import that narrows to no in-repo candidate.
    ImportExternal,
    /// Name not defined in-repo at all (ordinary unresolved call).
    UnknownName,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolutionOutcome<'a> {
    pub resolved: Vec<ResolvedCallee<'a>>,
    /// Some(..) iff `resolved` is empty for a classified reason.
    pub drop: Option<DropReason>,
}

impl<'a> ResolutionOutcome<'a> {
    pub fn hit(resolved: Vec<ResolvedCallee<'a>>) -> Self {
        Self {
            resolved,
            drop: None,
        }
    }

    pub fn dropped(reason: DropReason) -> Self {
        Self {
            resolved: Vec::new(),
            drop: Some(reason),
        }
    }
}

fn exact<'a>(
    ids: impl IntoIterator<Item = &'a FunctionId>,
    kind: ResolutionKind,
) -> Vec<ResolvedCallee<'a>> {
    ids.into_iter()
        .map(|target| ResolvedCallee {
            target,
            confidence: ResolutionConfidence::Exact,
            kind,
        })
        .collect()
}

fn demoted<'a>(
    ids: impl IntoIterator<Item = &'a FunctionId>,
    kind: ResolutionKind,
) -> Vec<ResolvedCallee<'a>> {
    ids.into_iter()
        .map(|target| ResolvedCallee {
            target,
            confidence: ResolutionConfidence::NameOnly,
            kind,
        })
        .collect()
}

fn receiver_resolution_kind(recovery: ReceiverRecovery) -> ResolutionKind {
    match recovery {
        ReceiverRecovery::ConstructorLocal => ResolutionKind::ConstructorLocal,
        ReceiverRecovery::TypedParam
        | ReceiverRecovery::TypeAssertion
        | ReceiverRecovery::VarDecl
        | ReceiverRecovery::SliceElem
        | ReceiverRecovery::FieldTyped
        | ReceiverRecovery::ReturnTyped
        | ReceiverRecovery::StdWrapperPeel
        | ReceiverRecovery::TypedLet => ResolutionKind::TypedParam,
    }
}

/// Kind-aware combine for a receiver-typed candidate set (the PR-3 read-path core).
#[allow(dead_code)]
fn combine_kind<'a>(
    cands: &'a [FunctionId],
    facts: &BTreeMap<FunctionId, MethodFacts>,
    recovery: ReceiverRecovery,
    arg_count: Option<usize>,
    arg_spread: bool,
) -> Option<Vec<ResolvedCallee<'a>>> {
    let kept: Vec<&FunctionId> = cands
        .iter()
        .filter(|fid| {
            let Some(fact) = facts.get(*fid) else {
                return false;
            };
            fact.has_self
                && !matches!(
                    arg_count,
                    Some(n) if !arg_spread && fact.arity_excl_self != n
                )
        })
        .collect();

    match kept.len() {
        0 => None,
        1 => {
            let fid = kept[0];
            let kind = receiver_resolution_kind(recovery);
            match facts.get(fid) {
                Some(MethodFacts {
                    kind: MethodKind::Inherent,
                    ..
                }) if recovery != ReceiverRecovery::StdWrapperPeel => Some(exact(kept, kind)),
                Some(_) => Some(demoted(kept, kind)),
                None => None,
            }
        }
        _ => Some(demoted(kept, ResolutionKind::TraitCha)),
    }
}

fn is_simple_ident(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c.is_alphanumeric() || c == '_')
}

impl CallGraph {
    fn rust_scope_graph_resolution(
        &self,
        graph: &ScopeGraph,
        site: &CallSite,
        file: FileId,
        from: ScopeId,
    ) -> Option<Vec<ResolvedCallee<'_>>> {
        // TODO: extract scope-graph bridge once the alias-target identity fix
        // has settled; keep this behavior-change PR narrowly scoped.
        let target = if site.callee_name.contains("::") {
            rust_graph_qualified_callable_edge(graph, site, file, from)
        } else if site.qualifier.is_none() {
            graph_callable_edge(graph, site)
        } else {
            return None;
        }?;
        self.graph_target_resolution(graph, site, &target)
    }

    /// Does the authoritative graph resolve this `::` call's qualified-callable
    /// path to a free function (vs a method)? Used to divert free-function paths
    /// whose owner-segment name collides with a method bucket away from the
    /// owner-prune.
    fn rust_graph_qualified_target_is_free_fn(
        &self,
        graph: &ScopeGraph,
        site: &CallSite,
        file: FileId,
        from: ScopeId,
    ) -> bool {
        if !site.callee_name.contains("::") {
            return false;
        }
        let Some(target) = rust_graph_qualified_callable_edge(graph, site, file, from) else {
            return false;
        };
        let ids = self.graph_target_ids(graph, &target);
        !ids.is_empty() && ids.iter().all(|fid| !self.method_owners.contains_key(*fid))
    }

    /// Owner-keyed disproof prune (spec §4). Fetch the bare `(owner, method)` pool
    /// from `self.methods`, run the `ScopeResolution` predicate, and decide:
    /// 1 survivor -> Exact; >1 -> demoted; unchanged from the bare pool -> `None`
    /// so the caller can fail-open to the #120 demote floor.
    fn rust_scope_prune_owner(
        &self,
        graph: &ScopeGraph,
        site: &CallSite,
        file: FileId,
        from: ScopeId,
        name: &str,
    ) -> Option<Vec<ResolvedCallee<'_>>> {
        let (owner, method) = owner_method_key(name)?;
        let pool_ids = self.methods.get(&(owner, method))?;
        let pool: Vec<&FunctionId> = pool_ids.iter().collect();
        let pred = ScopeResolution::new(self);
        let cx = crate::resolution_disproof::DisproofCx { graph, file, from };
        let pruned = crate::resolution_disproof::prune(
            pool.clone(),
            site,
            &cx,
            &[&pred as &dyn crate::resolution_disproof::DisproofPredicate],
        );
        if pruned.len() == pool.len() {
            return None;
        }
        Some(match pruned.len() {
            1 => exact(pruned, ResolutionKind::QualifiedOwner),
            _ => demoted(pruned, ResolutionKind::QualifiedOwner),
        })
    }

    /// The in-repo `FunctionId`s a resolved callable `Target` maps to, applying
    /// the same per-binding file + owner narrowing `graph_target_resolution`
    /// uses. Shared by the `ScopeResolution` predicate and `graph_target_resolution`.
    fn graph_target_ids<'b>(&'b self, graph: &ScopeGraph, target: &Target) -> Vec<&'b FunctionId> {
        let mut ids: Vec<&FunctionId> = Vec::new();
        if !matches!(target, Target::Item { callable: true, .. }) {
            return ids;
        }
        for binding in graph.bindings.iter() {
            if !matches!(&binding.target, BindTarget::Resolved(t) if t == target) {
                continue;
            }
            let Some(file) = graph_file_for_scope(graph, binding.scope) else {
                continue;
            };
            let owner = graph_owner_name_for_scope(graph, binding.scope);
            if let Some(functions) = self.functions.get(&binding.name) {
                for fid in functions
                    .iter()
                    .filter(|fid| graph.file_paths.get(&fid.file).copied() == Some(file))
                {
                    match owner.as_deref() {
                        Some(owner)
                            if self.method_owners.get(fid).map(String::as_str) != Some(owner) =>
                        {
                            continue;
                        }
                        None if self.method_owners.contains_key(fid) => {
                            continue;
                        }
                        _ => {}
                    }
                    if !ids.contains(&fid) {
                        ids.push(fid);
                    }
                }
            }
        }
        ids
    }

    fn graph_target_resolution(
        &self,
        graph: &ScopeGraph,
        site: &CallSite,
        target: &Target,
    ) -> Option<Vec<ResolvedCallee<'_>>> {
        let ids = self.graph_target_ids(graph, target);
        if ids.is_empty() {
            return None;
        }
        let qualified = site.callee_name.contains("::");
        match ids.len() {
            1 => {
                let kind = if qualified {
                    ResolutionKind::QualifiedOwner
                } else if ids[0].file == site.caller.file {
                    ResolutionKind::LocalDef
                } else {
                    ResolutionKind::FreeSingle
                };
                Some(exact(ids, kind))
            }
            _ => {
                // >1: a `::`-qualified owner that owns inherent + trait (or cfg
                // variants) demotes to NameOnly (recall-safe - keep every edge).
                // The unqualified LocalDef/FreeSingle arms stay singleton-only:
                // a >1 unqualified free-fn set routes through the existing free-fn
                // rungs, so decline here (return None) to fall through.
                if qualified {
                    Some(demoted(ids, ResolutionKind::QualifiedOwner))
                } else {
                    None
                }
            }
        }
    }

    /// Owner-index lookup that knows whether the key is a multi-impl trait key.
    fn owner_lookup(&self, owner: &str, name: &str) -> Option<Vec<ResolvedCallee<'_>>> {
        let mut resolved = self.owner_lookup_in_modules(owner, name, &[])?;
        // Relabel ONLY the promoted FunctionIds (defensive: direct-wins means a
        // promoted key has no direct method, but label by fid so a future mixed
        // bucket can't mislabel a non-promoted callee).
        if let Some(fids) = self
            .promoted_aliases
            .get(&(owner.to_string(), name.to_string()))
        {
            for c in &mut resolved {
                if fids.contains(c.target) {
                    c.kind = ResolutionKind::EmbeddedPromotion;
                }
            }
        }
        Some(resolved)
    }

    /// Like `owner_lookup`, but for a qualified `mod::T::m` call the preceding
    /// module segments narrow candidates to files under that module — so
    /// `foo::Engine::start()` does NOT also resolve `bar::Engine::start()` (same
    /// bare owner key, different module) as Exact. Narrowing only applies when
    /// module segments are present AND there is >1 candidate; if it eliminates
    /// everything it is ignored (a wrong module hint must not drop a real edge).
    fn owner_lookup_in_modules(
        &self,
        owner: &str,
        name: &str,
        module_segs: &[&str],
    ) -> Option<Vec<ResolvedCallee<'_>>> {
        let ids = self.methods.get(&(owner.to_string(), name.to_string()))?;
        let pool: Vec<&FunctionId> = if !module_segs.is_empty() && ids.len() > 1 {
            let narrowed: Vec<&FunctionId> = ids
                .iter()
                .filter(|fid| {
                    module_segs
                        .iter()
                        .any(|seg| file_has_path_segment(&fid.file, seg))
                })
                .collect();
            if narrowed.is_empty() {
                ids.iter().collect()
            } else {
                narrowed
            }
        } else {
            ids.iter().collect()
        };
        let primary_owners: BTreeSet<&str> = pool
            .iter()
            .filter_map(|fid| self.method_owners.get(*fid).map(|s| s.as_str()))
            .collect();
        Some(if pool.len() > 1 && primary_owners.len() > 1 {
            // Multiple DISTINCT primary owners — trait-CHA (dyn Trait). Unchanged.
            demoted(pool, ResolutionKind::TraitCha)
        } else if pool.len() > 1 {
            // Non-trait multi-candidate owner-key ambiguity: >1 candidate under one
            // primary owner name with no scope proof reached here — same-name-type
            // collisions, overloads, or inherent+trait same-name dups. Demote: keep
            // every edge (recall) but not at full confidence. Kind stays
            // QualifiedOwner so caller relabels (R3b/Self::/R6/implicit-this) fire
            // unchanged; only the confidence rides through as NameOnly. Recoverable
            // to Exact once an upstream capability supplies the discrimination.
            demoted(pool, ResolutionKind::QualifiedOwner)
        } else {
            // Single candidate — Exact, unchanged.
            exact(pool, ResolutionKind::QualifiedOwner)
        })
    }

    /// S3 R1-R7 ladder. This is the single full-resolution entry point for the
    /// new precision ladder. Legacy callers continue to use the old resolver
    /// until Tasks 9-11 migrate them.
    pub fn resolve_call_site_full(&self, site: &CallSite) -> ResolutionOutcome<'_> {
        let name = site.callee_name.as_str();
        let caller = &site.caller;

        if let Some(graph) = self.scope_graph.as_ref() {
            if crate::languages::Language::from_path(&site.caller.file)
                == Some(crate::languages::Language::Rust)
                && (name.contains("::") || site.qualifier.is_none())
            {
                if let Some((file, from)) = rust_authoritative_scope(graph, site) {
                    let owner_method = if name.contains("::") {
                        owner_method_key(name)
                    } else {
                        None
                    };
                    let has_bare_pool = owner_method.as_ref().is_some_and(|(owner, method)| {
                        self.methods.contains_key(&(owner.clone(), method.clone()))
                    });

                    if has_bare_pool
                        && name.contains("::")
                        && self.rust_graph_qualified_target_is_free_fn(graph, site, file, from)
                    {
                        return match self.rust_scope_graph_resolution(graph, site, file, from) {
                            Some(resolved) => ResolutionOutcome::hit(resolved),
                            None => ResolutionOutcome::dropped(DropReason::UnknownName),
                        };
                    }

                    if has_bare_pool {
                        if let Some(resolved) =
                            self.rust_scope_prune_owner(graph, site, file, from, name)
                        {
                            return ResolutionOutcome::hit(resolved);
                        }

                        let (owner, method) = owner_method.as_ref().expect("has_bare_pool");
                        let segs: Vec<&str> = name.split("::").collect();
                        let mut prefix: Vec<&str> = segs[..segs.len() - 1].to_vec();
                        while matches!(prefix.first(), Some(&"crate") | Some(&"super")) {
                            prefix.remove(0);
                        }
                        let module_segs = &prefix[..prefix.len().saturating_sub(1)];
                        if let Some(resolved) =
                            self.owner_lookup_in_modules(owner, method, module_segs)
                        {
                            // Authoritative scope site: a distinct-owner (trait-CHA)
                            // pool must DECLINE so the CHA fan-out emits Exact — the
                            // legacy TraitCha NameOnly edge is intentionally disabled
                            // at authoritative scope sites (matches the pre-existing
                            // `Render::draw(x)` behavior). Only the same-owner
                            // collision floor (#120 QualifiedOwner) is a valid
                            // fail-open here.
                            if !resolved.iter().any(|c| c.kind == ResolutionKind::TraitCha) {
                                return ResolutionOutcome::hit(resolved);
                            }
                        }
                        return ResolutionOutcome::dropped(DropReason::UnknownName);
                    }

                    if let Some(resolved) =
                        self.rust_scope_graph_resolution(graph, site, file, from)
                    {
                        return ResolutionOutcome::hit(resolved);
                    }
                    return ResolutionOutcome::dropped(DropReason::UnknownName);
                }
            }
        }

        // R1/R2/R7: Rust/C++ `::` path shapes where the raw name carries the path.
        if name.contains("::") {
            let mut segs: Vec<&str> = name.split("::").collect();
            let fn_name = segs.pop().unwrap_or(name);
            while matches!(segs.first(), Some(&"crate") | Some(&"super")) {
                segs.remove(0);
            }

            if segs.as_slice() == ["self"] {
                if let Some(ids) = self.functions.get(fn_name) {
                    let local: Vec<&FunctionId> =
                        ids.iter().filter(|fid| fid.file == caller.file).collect();
                    if !local.is_empty() {
                        return ResolutionOutcome::hit(exact(local, ResolutionKind::LocalDef));
                    }
                }
                return ResolutionOutcome::dropped(DropReason::UnknownName);
            }

            if let Some(&head) = segs.last() {
                if head == "Self" {
                    if let Some(owner) = self.method_owners.get(caller) {
                        if let Some(mut resolved) = self.owner_lookup(owner, fn_name) {
                            for callee in &mut resolved {
                                if callee.kind == ResolutionKind::QualifiedOwner {
                                    callee.kind = ResolutionKind::SelfReceiver;
                                }
                            }
                            return ResolutionOutcome::hit(resolved);
                        }
                    }
                    return ResolutionOutcome::dropped(DropReason::UnknownName);
                }

                // R1: `T::m` / `mod::T::m` — the segments before the type narrow
                // candidates by module (so same-named types in different modules
                // don't all resolve Exact).
                let module_segs = &segs[..segs.len() - 1];
                if let Some(resolved) = self.owner_lookup_in_modules(head, fn_name, module_segs) {
                    return ResolutionOutcome::hit(resolved);
                }

                if let Some(ids) = self.functions.get(fn_name) {
                    let matched: Vec<&FunctionId> = ids
                        .iter()
                        .filter(|fid| file_stem(&fid.file) == head)
                        // Static-linkage preservation (matches the legacy
                        // resolver's final filter): a `static` C/C++ function
                        // in another file is not callable from here.
                        .filter(|fid| {
                            fid.file == caller.file
                                || !self
                                    .static_functions
                                    .contains(&(fid.file.clone(), fn_name.to_string()))
                        })
                        .collect();
                    return match matched.len() {
                        0 => ResolutionOutcome::dropped(DropReason::UnknownName),
                        1 => ResolutionOutcome::hit(exact(matched, ResolutionKind::StemSingle)),
                        _ => ResolutionOutcome::hit(demoted(matched, ResolutionKind::StemMulti)),
                    };
                }
            }

            return ResolutionOutcome::dropped(DropReason::UnknownName);
        }

        // Static linkage preservation from the legacy resolver.
        if site.qualifier.is_none()
            && self
                .static_functions
                .contains(&(caller.file.clone(), name.to_string()))
        {
            if let Some(ids) = self.functions.get(name) {
                let local: Vec<&FunctionId> =
                    ids.iter().filter(|fid| fid.file == caller.file).collect();
                if !local.is_empty() {
                    return ResolutionOutcome::hit(exact(local, ResolutionKind::StaticLinkage));
                }
            }
            return ResolutionOutcome::dropped(DropReason::UnknownName);
        }

        match site.qualifier.as_deref() {
            Some(q)
                if q == "self"
                    || q == "this"
                    || q == "cls"
                    || self.receiver_vars.get(caller).map(String::as_str) == Some(q) =>
            {
                if let Some(owner) = self.method_owners.get(caller) {
                    if let Some(mut resolved) = self.owner_lookup(owner, name) {
                        for callee in &mut resolved {
                            if callee.kind == ResolutionKind::QualifiedOwner {
                                callee.kind = ResolutionKind::SelfReceiver;
                            }
                        }
                        return ResolutionOutcome::hit(resolved);
                    }
                }
                ResolutionOutcome::dropped(DropReason::UnknownName)
            }
            Some(q) => {
                // A materialized Rust receiver outcome means this is a value-method call
                // `recv.method()`: the qualifier is a receiver expression, NOT a module or
                // type name. The receiver's static type (the Rust branch below) is
                // authoritative and must pre-empt both the import-qualifier (R3) and the
                // owner-key (R3b) interpretations. Recall-safe: receiver_outcome == Some
                // only for value-method receiver syntax, so R3/R3b never held a correct edge
                // for these sites.
                let rust_recv_materialized =
                    crate::languages::Language::from_path(&site.caller.file)
                        == Some(crate::languages::Language::Rust)
                        && site.receiver_outcome.is_some();

                // R3: imported-module qualifier. If an import matches, the
                // narrowed set is final; empty means the call is external.
                if !rust_recv_materialized {
                    if let Some(file_imports) = self.imports.get(&caller.file) {
                        if let Some(module_path) = file_imports.get(q) {
                            let ids = match self.functions.get(name) {
                                Some(v) => v.as_slice(),
                                None => return ResolutionOutcome::dropped(DropReason::UnknownName),
                            };
                            let module_last = module_path.rsplit('/').next().unwrap_or(module_path);
                            let module_stem = module_last.rsplit('.').last().unwrap_or(module_last);
                            let matched: Vec<&FunctionId> = ids
                                .iter()
                                // `pkg.f()` names a module-level function, never a method
                                // on a class defined in that module — exclude methods so
                                // an imported module with a same-named method can't forge
                                // a false package-function edge.
                                .filter(|fid| !self.method_owners.contains_key(*fid))
                                .filter(|fid| {
                                    let stem_hit = file_stem(&fid.file) == module_stem;
                                    let dir_hit = fid
                                        .file
                                        .rsplit('/')
                                        .nth(1)
                                        .map(|d| d == module_last)
                                        .unwrap_or(false);
                                    stem_hit || dir_hit
                                })
                                .collect();
                            if matched.is_empty() {
                                return ResolutionOutcome::dropped(DropReason::ImportExternal);
                            }
                            return ResolutionOutcome::hit(exact(
                                matched,
                                ResolutionKind::ImportQualified,
                            ));
                        }
                    }
                }

                // R3b: qualifier text is itself an owner key.
                if !rust_recv_materialized && is_simple_ident(q) {
                    if let Some(mut resolved) = self.owner_lookup(q, name) {
                        for callee in &mut resolved {
                            if callee.kind == ResolutionKind::QualifiedOwner {
                                callee.kind = ResolutionKind::QualifierOwner;
                            }
                        }
                        return ResolutionOutcome::hit(resolved);
                    }
                }

                if crate::languages::Language::from_path(&site.caller.file)
                    == Some(crate::languages::Language::Rust)
                {
                    if let Some(oc) = site.receiver_outcome.as_ref() {
                        let name_key = name.to_string();
                        return match &oc.key {
                            ReceiverTypeKey::InRepo(scope) => {
                                match self
                                    .methods_by_scope
                                    .get(&(*scope, name_key.clone()))
                                    .and_then(|cands| {
                                        combine_kind(
                                            cands,
                                            &self.method_facts,
                                            oc.recovery,
                                            site.arg_count,
                                            site.arg_spread,
                                        )
                                    }) {
                                    Some(resolved) => ResolutionOutcome::hit(resolved),
                                    None => {
                                        if self
                                            .identity_complete
                                            .contains(&(oc.bare.clone(), name_key))
                                        {
                                            ResolutionOutcome::dropped(DropReason::ExternalReceiver)
                                        } else {
                                            match self.owner_lookup(&oc.bare, name) {
                                                Some(resolved) => ResolutionOutcome::hit(resolved),
                                                None => ResolutionOutcome::dropped(
                                                    DropReason::ExternalReceiver,
                                                ),
                                            }
                                        }
                                    }
                                }
                            }
                            ReceiverTypeKey::External(canon) => {
                                match self
                                    .extension_methods
                                    .get(&(canon.clone(), name_key))
                                    .and_then(|cands| {
                                        combine_kind(
                                            cands,
                                            &self.method_facts,
                                            oc.recovery,
                                            site.arg_count,
                                            site.arg_spread,
                                        )
                                    }) {
                                    Some(resolved) => ResolutionOutcome::hit(resolved),
                                    None => {
                                        ResolutionOutcome::dropped(DropReason::ExternalReceiver)
                                    }
                                }
                            }
                            ReceiverTypeKey::Bare(s) => match self.owner_lookup(s, name) {
                                Some(resolved) => ResolutionOutcome::hit(resolved),
                                None => ResolutionOutcome::dropped(DropReason::ExternalReceiver),
                            },
                        };
                    }
                }

                // R6 step 1: P6-lite recovered receiver.
                if let Some(recv_ty) = site.receiver_type.as_deref() {
                    let recovered_kind = match site.receiver_recovery {
                        Some(ReceiverRecovery::ConstructorLocal) => {
                            ResolutionKind::ConstructorLocal
                        }
                        _ => ResolutionKind::TypedParam,
                    };
                    return match self.owner_lookup(recv_ty, name) {
                        Some(mut resolved) => {
                            for callee in &mut resolved {
                                if callee.kind == ResolutionKind::QualifiedOwner {
                                    callee.kind = recovered_kind;
                                }
                                // Trait-CHA hits keep TraitCha (dyn Trait receivers).
                            }
                            ResolutionOutcome::hit(resolved)
                        }
                        // Gate the interface consult to Go callers: P6-lite receiver
                        // recovery also fires for Rust, and `interface_impls` is Go-only,
                        // so an un-gated consult could mint a cross-language edge (e.g. a
                        // Rust `x.Go()` matching a Go interface named the same). Mirrors the
                        // language gate at the C-only free-fn fallback below.
                        None if crate::languages::Language::from_path(&site.caller.file)
                            == Some(crate::languages::Language::Go) =>
                        {
                            match crate::resolution::iface_key(recv_ty) {
                                Some(k) => match self.interface_impls.get(&(k, name.to_string())) {
                                    Some(ids) if !ids.is_empty() => {
                                        // Arity-disambiguate the name-keyed candidate set
                                        // (shared helper; same filter runs in
                                        // interface_dispatch_manifest). An emptied set takes
                                        // the existing no-impl drop path — do NOT fall through.
                                        let kept = crate::resolution::arity_filter(
                                            ids,
                                            site.arg_count,
                                            site.arg_spread,
                                            &self.method_arity,
                                        );
                                        if kept.is_empty() {
                                            ResolutionOutcome::dropped(DropReason::ExternalReceiver)
                                        } else {
                                            ResolutionOutcome::hit(exact(
                                                kept,
                                                ResolutionKind::InterfaceDispatch,
                                            ))
                                        }
                                    }
                                    _ => ResolutionOutcome::dropped(DropReason::ExternalReceiver),
                                },
                                None => ResolutionOutcome::dropped(DropReason::ExternalReceiver),
                            }
                        }
                        None => ResolutionOutcome::dropped(DropReason::ExternalReceiver),
                    };
                }

                // R6 residue (P2): method candidates only, never free fns.
                let method_ids: Vec<&FunctionId> = self
                    .functions
                    .get(name)
                    .map(|v| {
                        v.iter()
                            .filter(|fid| self.method_owners.contains_key(*fid))
                            .collect()
                    })
                    .unwrap_or_default();
                if method_ids.is_empty() {
                    // C has no methods: a receiver-syntax call `ptr->field()` /
                    // `s.field()` is a FUNCTION-POINTER field access, not a method
                    // call, so R6's "never bind to free functions" rule does not
                    // apply. Bind to a single same-named free function, demoted
                    // (the struct-callback heuristic membrane relies on; spec
                    // tiering: a single candidate is not provably-wrong → demote,
                    // not drop). Gated to C only — method-languages keep the
                    // method-only rule, since there `x.m()` is syntactically a
                    // method call.
                    if matches!(
                        crate::languages::Language::from_path(&caller.file),
                        Some(crate::languages::Language::C)
                    ) {
                        if let Some(ids) = self.functions.get(name) {
                            let free: Vec<&FunctionId> = ids
                                .iter()
                                .filter(|fid| {
                                    fid.file == caller.file
                                        || !self
                                            .static_functions
                                            .contains(&(fid.file.clone(), name.to_string()))
                                })
                                .collect();
                            if free.len() == 1 {
                                return ResolutionOutcome::hit(demoted(
                                    free,
                                    ResolutionKind::R6SingleOwner,
                                ));
                            }
                        }
                    }
                    return ResolutionOutcome::dropped(DropReason::UnknownName);
                }

                // Caller's-own-file preference: exactly one owner defining it there.
                let local: Vec<&FunctionId> = method_ids
                    .iter()
                    .copied()
                    .filter(|f| f.file == caller.file)
                    .collect();
                let local_owners: BTreeSet<&str> = local
                    .iter()
                    .filter_map(|f| self.method_owners.get(*f).map(String::as_str))
                    .collect();
                if local_owners.len() == 1 {
                    return ResolutionOutcome::hit(demoted(local, ResolutionKind::R6SingleOwner));
                }

                let owners: BTreeSet<&str> = method_ids
                    .iter()
                    .filter_map(|f| self.method_owners.get(*f).map(String::as_str))
                    .collect();
                if owners.len() == 1 {
                    return ResolutionOutcome::hit(demoted(
                        method_ids,
                        ResolutionKind::R6SingleOwner,
                    ));
                }
                ResolutionOutcome::dropped(DropReason::MultiOwnerCollision)
            }
            None => {
                let ids = match self.functions.get(name) {
                    Some(v) => v,
                    None => return ResolutionOutcome::dropped(DropReason::UnknownName),
                };

                let free: Vec<&FunctionId> = ids
                    .iter()
                    .filter(|fid| !self.method_owners.contains_key(*fid))
                    .collect();

                // R4: local free definition wins alone.
                let local: Vec<&FunctionId> = free
                    .iter()
                    .copied()
                    .filter(|f| f.file == caller.file)
                    .collect();
                if !local.is_empty() {
                    return ResolutionOutcome::hit(exact(local, ResolutionKind::LocalDef));
                }

                // R4.5: a Go unqualified call resolves within its own package
                // (= directory). A real cross-package Go call is qualified
                // (`pkg.Func`) and binds on an earlier rung; an unqualified call
                // cannot reach a function in another directory, so prefer
                // same-directory free definitions over R5's repo-wide set
                // (FreeMulti). Go forbids two same-named funcs in one package, so
                // the same-dir set is normally one (-> Exact); the rare cases
                // where a directory holds more (a black-box `_test` package, or
                // mutually-exclusive build-tag files) can't be separated by this
                // build-agnostic whole-text scan, so demote those rather than
                // over-claim Exact (package-clause identity would refine it).
                if caller.file.ends_with(".go") {
                    let dir = dir_of(&caller.file);
                    let same_pkg: Vec<&FunctionId> = free
                        .iter()
                        .copied()
                        .filter(|f| dir_of(&f.file) == dir)
                        .collect();
                    match same_pkg.len() {
                        0 => {}
                        1 => {
                            return ResolutionOutcome::hit(exact(
                                same_pkg,
                                ResolutionKind::SamePackage,
                            ))
                        }
                        _ => {
                            return ResolutionOutcome::hit(demoted(
                                same_pkg,
                                ResolutionKind::SamePackage,
                            ))
                        }
                    }
                }

                // R4b: Java/C++ unqualified calls inside methods are implicit-this.
                if let Some(owner) = self.method_owners.get(caller) {
                    if caller.file.ends_with(".java")
                        || caller.file.ends_with(".cpp")
                        || caller.file.ends_with(".cc")
                        || caller.file.ends_with(".cxx")
                        || caller.file.ends_with(".hpp")
                        || caller.file.ends_with(".h")
                    {
                        if let Some(mut resolved) = self.owner_lookup(owner, name) {
                            for callee in &mut resolved {
                                if callee.kind == ResolutionKind::QualifiedOwner {
                                    callee.kind = ResolutionKind::ImplicitThis;
                                }
                            }
                            return ResolutionOutcome::hit(resolved);
                        }
                    }
                }

                // R5: cross-file free functions only, preserving legacy static exclusion.
                let nonstatic: Vec<&FunctionId> = free
                    .into_iter()
                    .filter(|fid| {
                        !self
                            .static_functions
                            .contains(&(fid.file.clone(), name.to_string()))
                    })
                    .collect();
                match nonstatic.len() {
                    0 => ResolutionOutcome::dropped(DropReason::UnknownName),
                    1 => ResolutionOutcome::hit(exact(nonstatic, ResolutionKind::FreeSingle)),
                    _ => ResolutionOutcome::hit(demoted(nonstatic, ResolutionKind::FreeMulti)),
                }
            }
        }
    }

    pub fn resolve_call_site(&self, site: &CallSite) -> Vec<ResolvedCallee<'_>> {
        self.resolve_call_site_full(site).resolved
    }

    /// All call sites that resolve to `callee`, with caller, line, and confidence.
    /// The site-line source for slice witnesses; CPG edges carry no line.
    pub fn resolved_caller_edges(&self, callee: &FunctionId) -> Vec<ResolvedCallEdge> {
        let mut out = Vec::new();
        for sites in self.calls.values() {
            for site in sites {
                for r in self.resolve_call_site(site) {
                    if r.target == callee {
                        out.push(ResolvedCallEdge {
                            caller: site.caller.clone(),
                            call_site_line: site.line,
                            confidence: r.confidence,
                        });
                    }
                }
            }
        }
        out
    }
}

/// The one shipped disproof predicate (spec §3). Disproves a candidate only when
/// the owner type-path's leading segment binds directly to an in-repo item (②B),
/// has no block-local shadow at the call site (①C), and the graph is
/// edition-uniform (§2 guard). On all uncertainty it disproves nothing.
pub struct ScopeResolution<'a> {
    cg: &'a CallGraph,
}

impl<'a> ScopeResolution<'a> {
    pub fn new(cg: &'a CallGraph) -> Self {
        ScopeResolution { cg }
    }
}

impl crate::resolution_disproof::DisproofPredicate for ScopeResolution<'_> {
    fn disproves(
        &self,
        cand: &FunctionId,
        site: &CallSite,
        cx: &crate::resolution_disproof::DisproofCx<'_>,
    ) -> bool {
        let graph = cx.graph;
        // §2 guard: a non-uniform-edition workspace is non-authoritative for
        // disproof. Keep-all.
        if !graph.edition_uniform {
            return false;
        }

        // Only `T::m` / `mod::T::m` owner paths are in scope for this predicate.
        let Some((anchor, path)) = rust_call_path_anchor(site.callee_name.as_str()) else {
            return false;
        };
        // The leading type segment is the path's first segment; the trailing
        // segment is the method. A path with <2 segments has no owner type.
        if path.0.len() < 2 {
            return false;
        }
        let leading = &path.0[0];

        // ①C: any potential block-local shadow of the leading ident -> keep-all.
        if leading_segment_has_block_local_shadow(graph, cx.from, cx.file, site.start_byte, leading)
        {
            return false;
        }

        // ②B: the leading segment must bind directly to an in-repo Item, with no
        // Pending hop. Prove this from the binding shape, not Candidate provenance.
        if !leading_segment_binds_directly(
            graph,
            cx.file,
            cx.from,
            site.start_byte,
            &anchor,
            leading,
        ) {
            return false;
        }

        // Both contracts hold. Resolve the final callable target and disprove
        // `cand` iff it is not in that target's in-repo id set.
        let Some(target) = rust_graph_qualified_callable_edge(graph, site, cx.file, cx.from) else {
            return false;
        };
        let ids = self.cg.graph_target_ids(graph, &target);
        if ids.is_empty() {
            return false;
        }
        !ids.contains(&cand)
    }
}

/// Prove the leading type segment binds directly at the call site: the binding
/// the call site sees for `leading` is itself a non-glob, non-Pending
/// `BindTarget::Resolved(Target::Item)` (§8.2 decision ②B).
fn leading_segment_binds_directly(
    graph: &ScopeGraph,
    file: FileId,
    from: ScopeId,
    byte: usize,
    anchor: &Anchor,
    leading: &str,
) -> bool {
    let at = SourceLoc { file, byte };
    let policy = RustPolicy::new(graph, graph.edition);
    let Some((start, _)) = policy.anchor(anchor, from) else {
        return false;
    };

    let rib: Vec<&Binding> = graph
        .bindings
        .iter()
        .filter(|b| b.scope == start && b.name == leading && b.ns == NS_TYPE)
        .collect();
    if rib.is_empty() {
        return false;
    }

    let q = ResolveQuery {
        name: leading.to_string(),
        ns: NS_TYPE,
        from,
        at: at.clone(),
        cfg: Default::default(),
        ctx: Default::default(),
    };
    let visible: Vec<&Binding> = rib
        .into_iter()
        .filter(|b| {
            let trav = TraversalCtx {
                lookup_scope: Some(b.scope),
                via_glob: false,
                edge_kind: None,
            };
            policy.visible(b, &q, &trav)
        })
        .collect();

    match visible.as_slice() {
        [b] => matches!(&b.target, BindTarget::Resolved(Target::Item { .. })),
        _ => false,
    }
}

/// Does the lexical scope chain from `from` up to but excluding the enclosing
/// module/root contain any potential block-local shadow of `leading` at `byte`?
/// Three shapes (§8.1 decision ①C): exact `NS_TYPE` binding, block-local glob
/// edge covering the call byte, or covering `NS_TYPE` macro wildcard.
fn leading_segment_has_block_local_shadow(
    graph: &ScopeGraph,
    from: ScopeId,
    file: FileId,
    byte: usize,
    leading: &str,
) -> bool {
    let at = SourceLoc { file, byte };
    for scope in scope_chain_below_module(graph, from) {
        let exact_binding = graph.bindings.iter().any(|b| {
            b.scope == scope && b.name == leading && b.ns == NS_TYPE && binding_vis_covers(b, &at)
        });
        if exact_binding {
            return true;
        }

        let glob_shadow = graph.edges.iter().any(|e: &Edge| {
            e.from == scope
                && e.kind == EK_GLOB
                && e.vis_range.as_ref().is_some_and(|s| span_covers(s, &at))
        });
        if glob_shadow {
            return true;
        }

        let macro_shadow = graph
            .macro_wildcards
            .iter()
            .any(|m| m.scope == scope && m.ns == NS_TYPE && span_covers(&m.range, &at));
        if macro_shadow {
            return true;
        }
    }
    false
}

/// The lexical scope chain from `from` up to but excluding the enclosing
/// `Module`/`Root`. The module/root scope itself is not scanned: the resolved
/// type's own def-binding lives there and would self-shadow every direct type.
fn scope_chain_below_module(graph: &ScopeGraph, from: ScopeId) -> Vec<ScopeId> {
    use crate::name_resolution::types::ScopeKind;

    let mut out = Vec::new();
    let mut cur = Some(from);
    while let Some(id) = cur {
        let Some(s) = graph.scope(id) else { break };
        if matches!(s.kind, ScopeKind::Module | ScopeKind::Root) {
            break;
        }
        out.push(id);
        cur = s.parent;
    }
    out
}

/// `Binding::vis_extents` cover `at` (empty extents means scope-wide).
fn binding_vis_covers(b: &Binding, at: &SourceLoc) -> bool {
    if b.vis_extents.is_empty() {
        return true;
    }
    b.vis_extents.iter().any(|s| span_covers(s, at))
}

/// Half-open `[lo, hi)` same-file span cover.
fn span_covers(s: &Span, at: &SourceLoc) -> bool {
    s.lo.file == at.file && at.byte >= s.lo.byte && at.byte < s.hi.byte
}

/// The directory component of a path (`a/b/c.go` -> `a/b`; `c.go` -> ``). For
/// Go, a package occupies exactly one directory.
fn dir_of(path: &str) -> &str {
    path.rsplit_once('/').map(|(d, _)| d).unwrap_or("")
}

/// File stem matching `resolve_callees_qualified`'s idiom (`a.b.rs` -> `a`).
pub fn file_stem(path: &str) -> &str {
    path.rsplit('/')
        .next()
        .unwrap_or(path)
        .rsplit('.')
        .last()
        .unwrap_or(path)
}

/// Whether `seg` (a module path segment) names a directory component of `path`
/// or its file stem — e.g. `worker` matches both `src/worker.rs` and
/// `src/worker/mod.rs`. Used to narrow `mod::T::m` candidates by module.
fn file_has_path_segment(path: &str, seg: &str) -> bool {
    file_stem(path) == seg || path.split('/').any(|c| c == seg)
}

fn rust_authoritative_scope(graph: &ScopeGraph, site: &CallSite) -> Option<(FileId, ScopeId)> {
    if !graph.complete {
        return None;
    }
    let file = graph.file_paths.get(&site.caller.file).copied()?;
    enclosing_scope(graph, file, site.start_byte).map(|scope| (file, scope))
}

fn rust_graph_qualified_callable_edge(
    graph: &ScopeGraph,
    site: &CallSite,
    file: FileId,
    from: ScopeId,
) -> Option<Target> {
    let (anchor, path) = rust_call_path_anchor(site.callee_name.as_str())?;
    if path.0.is_empty() {
        return None;
    }
    let at = SourceLoc {
        file,
        byte: site.start_byte,
    };
    let policy = RustPolicy::new(graph, graph.edition);
    let res = resolve_path(graph, &path, NS_VALUE, &anchor, from, NS_TYPE, &at, &policy);
    match (res.status, res.candidates.as_slice()) {
        (ResStatus::Resolved, [Candidate { target, .. }]) => match target {
            Target::Item { callable: true, .. } => Some(target.clone()),
            _ => None,
        },
        _ => None,
    }
}

/// Split an owner-keyed `mod::T::m` call name into the bare `(owner, method)`
/// key after stripping leading `crate::`/`super::`. `self`/`Self` paths are
/// handled by their dedicated rungs.
fn owner_method_key(name: &str) -> Option<(String, String)> {
    let mut segs: Vec<&str> = name.split("::").collect();
    let method = segs.pop()?;
    while matches!(segs.first(), Some(&"crate") | Some(&"super")) {
        segs.remove(0);
    }
    let owner = *segs.last()?;
    if owner == "self" || owner == "Self" {
        return None;
    }
    Some((owner.to_string(), method.to_string()))
}

fn rust_call_path_anchor(raw: &str) -> Option<(Anchor, RawPath)> {
    let mut segs: Vec<String> = raw.split("::").map(str::to_string).collect();
    if segs.is_empty() {
        return None;
    }
    let anchor = match segs.first().map(String::as_str) {
        Some("") => {
            segs.remove(0);
            Anchor {
                kind: AnchorKind::LeadingColon,
                prelude: None,
            }
        }
        Some("crate") => {
            segs.remove(0);
            Anchor::crate_root()
        }
        Some("self") => {
            segs.remove(0);
            Anchor::self_mod()
        }
        Some("super") => {
            let mut n = 0u32;
            while matches!(segs.first().map(String::as_str), Some("super")) {
                segs.remove(0);
                n += 1;
            }
            Anchor::super_n(n)
        }
        Some(_) => Anchor::bare(),
        None => return None,
    };
    Some((anchor, RawPath(segs)))
}

fn graph_file_for_scope(graph: &ScopeGraph, scope: ScopeId) -> Option<FileId> {
    graph
        .scope(scope)?
        .extents
        .first()
        .map(|extent| extent.file)
}

fn graph_owner_name_for_scope(graph: &ScopeGraph, scope: ScopeId) -> Option<String> {
    graph
        .bindings
        .iter()
        .find_map(|binding| match &binding.target {
            BindTarget::Resolved(Target::Item {
                owns: Some(owner), ..
            }) if *owner == scope => Some(binding.name.clone()),
            _ => None,
        })
}

#[cfg(test)]
mod embedding_kind_tests {
    use super::*;
    use crate::call_graph::{MethodFacts, MethodKind, RecvMode};
    use std::collections::BTreeMap;

    #[test]
    fn embedded_promotion_as_str() {
        assert_eq!(
            ResolutionKind::EmbeddedPromotion.as_str(),
            "embedded_promotion"
        );
    }

    fn fid(name: &str) -> FunctionId {
        FunctionId {
            file: "a.rs".to_string(),
            name: name.to_string(),
            start_line: 1,
            end_line: 1,
        }
    }

    fn facts(kind: MethodKind, has_self: bool, arity_excl_self: usize) -> MethodFacts {
        MethodFacts {
            kind,
            has_self,
            recv_mode: if has_self {
                RecvMode::SelfRef
            } else {
                RecvMode::None
            },
            arity_excl_self,
            cfg: None,
        }
    }

    #[test]
    fn combine_kind_single_inherent_typed_param_is_exact() {
        let method = fid("inherent");
        let cands = vec![method.clone()];
        let mut method_facts = BTreeMap::new();
        method_facts.insert(method.clone(), facts(MethodKind::Inherent, true, 1));

        let resolved = combine_kind(
            &cands,
            &method_facts,
            ReceiverRecovery::TypedParam,
            Some(1),
            false,
        )
        .expect("inherent receiver candidate");

        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].target, &method);
        assert_eq!(resolved[0].confidence, ResolutionConfidence::Exact);
        assert_eq!(resolved[0].kind, ResolutionKind::TypedParam);
    }

    #[test]
    fn combine_kind_single_trait_is_name_only() {
        let method = fid("trait_method");
        let cands = vec![method.clone()];
        let mut method_facts = BTreeMap::new();
        method_facts.insert(
            method.clone(),
            facts(MethodKind::Trait("Trait".to_string()), true, 0),
        );

        let resolved = combine_kind(
            &cands,
            &method_facts,
            ReceiverRecovery::TypedParam,
            Some(0),
            false,
        )
        .expect("trait receiver candidate");

        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].target, &method);
        assert_eq!(resolved[0].confidence, ResolutionConfidence::NameOnly);
        assert_eq!(resolved[0].kind, ResolutionKind::TypedParam);
    }

    #[test]
    fn combine_kind_single_inherent_std_wrapper_peel_is_name_only() {
        let method = fid("wrapped");
        let cands = vec![method.clone()];
        let mut method_facts = BTreeMap::new();
        method_facts.insert(method.clone(), facts(MethodKind::Inherent, true, 0));

        let resolved = combine_kind(
            &cands,
            &method_facts,
            ReceiverRecovery::StdWrapperPeel,
            Some(0),
            false,
        )
        .expect("wrapper peeled candidate");

        assert_eq!(resolved.len(), 1);
        assert_eq!(resolved[0].target, &method);
        assert_eq!(resolved[0].confidence, ResolutionConfidence::NameOnly);
        assert_eq!(resolved[0].kind, ResolutionKind::TypedParam);
    }

    #[test]
    fn combine_kind_multi_demotes_all() {
        let a = fid("a");
        let b = fid("b");
        let cands = vec![a.clone(), b.clone()];
        let mut method_facts = BTreeMap::new();
        method_facts.insert(a.clone(), facts(MethodKind::Inherent, true, 0));
        method_facts.insert(
            b.clone(),
            facts(MethodKind::Trait("Trait".to_string()), true, 0),
        );

        let resolved = combine_kind(
            &cands,
            &method_facts,
            ReceiverRecovery::TypedParam,
            Some(0),
            false,
        )
        .expect("multi candidate set");

        assert_eq!(resolved.len(), 2);
        assert!(resolved
            .iter()
            .all(|r| r.confidence == ResolutionConfidence::NameOnly));
        assert!(resolved.iter().all(|r| r.kind == ResolutionKind::TraitCha));
        assert_eq!(resolved[0].target, &a);
        assert_eq!(resolved[1].target, &b);
    }

    #[test]
    fn combine_kind_empty_after_has_self_filter_drops() {
        let assoc = fid("assoc");
        let cands = vec![assoc.clone()];
        let mut method_facts = BTreeMap::new();
        method_facts.insert(assoc, facts(MethodKind::Inherent, false, 0));

        assert_eq!(
            combine_kind(
                &cands,
                &method_facts,
                ReceiverRecovery::TypedParam,
                Some(0),
                false
            ),
            None
        );
    }

    #[test]
    fn combine_kind_arity_mismatch_drops_but_unknown_keeps() {
        let method = fid("arity");
        let cands = vec![method.clone()];
        let mut method_facts = BTreeMap::new();
        method_facts.insert(method.clone(), facts(MethodKind::Inherent, true, 2));

        assert_eq!(
            combine_kind(
                &cands,
                &method_facts,
                ReceiverRecovery::TypedParam,
                Some(1),
                false
            ),
            None
        );

        let unknown = combine_kind(
            &cands,
            &method_facts,
            ReceiverRecovery::TypedParam,
            None,
            false,
        )
        .expect("unknown arity keeps candidate");
        assert_eq!(unknown.len(), 1);
        assert_eq!(unknown[0].target, &method);
        assert_eq!(unknown[0].confidence, ResolutionConfidence::Exact);
    }
}

#[cfg(test)]
mod scope_resolution_predicate_tests {
    use super::*;
    use crate::call_graph::{CallSite, FunctionId};
    use crate::name_resolution::graph::ScopeGraph;
    use crate::name_resolution::types::{FileId, ScopeId};
    use crate::resolution_disproof::{DisproofCx, DisproofPredicate};

    fn fid(name: &str) -> FunctionId {
        FunctionId {
            file: "a.rs".to_string(),
            name: name.to_string(),
            start_line: 1,
            end_line: 1,
        }
    }

    fn site(callee: &str) -> CallSite {
        CallSite {
            caller: fid("caller"),
            callee_name: callee.to_string(),
            line: 1,
            kind: Default::default(),
            start_byte: 0,
            end_byte: 0,
            qualifier: None,
            receiver_type: None,
            receiver_recovery: None,
            arg_count: None,
            arg_spread: false,
            receiver_outcome: None,
        }
    }

    #[test]
    fn non_uniform_edition_disproves_nothing() {
        // §2 guard: a mixed-edition graph is non-authoritative for disproof.
        let cg = CallGraph::build(&std::collections::BTreeMap::new());
        let mut graph = ScopeGraph::new();
        graph.edition_uniform = false;
        let cx = DisproofCx {
            graph: &graph,
            file: FileId(0),
            from: ScopeId(0),
        };
        let pred = ScopeResolution::new(&cg);
        let cand = fid("with_file");
        assert!(
            !pred.disproves(&cand, &site("CliTest::with_file"), &cx),
            "non-uniform edition must disprove nothing (keep-all)"
        );
    }
}
