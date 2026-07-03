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
    Anchor, AnchorKind, BindTarget, Binding, Candidate, Edge, FileId, NamespaceId, RawPath,
    ResStatus, ResolutionPolicy, ResolveQuery, ScopeId, SourceLoc, Span, Target, TraversalCtx,
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
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
    /// R4c: resolved via import-member binding (Python/JS/TS).
    ImportMember,
    /// R6 residue candidate: unknown-receiver call with a same-name method on
    /// 2+ owner classes, emitted as a capped, labeled NameOnly edge instead of
    /// a silent drop. Gated to Python/JS/TS/Tsx (P3); Rust/Go keep the drop.
    R6MultiOwnerCandidate,
    /// P5 S2: a Go function-value registration site (composite-literal keyed
    /// field, field assignment, or bare call argument) surfaced as a NameOnly
    /// nav edge. Never produced by `resolve_call_site_full` — registrations
    /// are not `CallSite`s (architecture note: a synthetic CallSite here would
    /// resolve Exact via `free_single`, a soundness hole); this kind labels
    /// edges synthesized directly from `CallGraph::go_registrations` in
    /// `NavigationIndex::build_resolved_call_edges`.
    CallbackRegistration,
    /// P5 S3: a Go invocation `recv.Field(...)` resolved via the func-typed
    /// struct-field registration index (S1 field-typing + S2 registrations),
    /// gated inside the interface-consult miss path. NameOnly — the target is
    /// one of 1..=3 distinct registration targets recorded for the field.
    FuncValueField,
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
            ResolutionKind::ImportMember => "import_member",
            ResolutionKind::R6MultiOwnerCandidate => "r6_multi_owner_candidate",
            ResolutionKind::CallbackRegistration => "callback_registration",
            ResolutionKind::FuncValueField => "func_value_field",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedCallee<'a> {
    pub target: &'a FunctionId,
    pub confidence: ResolutionConfidence,
    pub kind: ResolutionKind,
}

enum RecoveredDirectMethod<'a> {
    Hit(Vec<ResolvedCallee<'a>>),
    Blocked,
    Miss,
}

/// A resolved caller edge: who calls the seed, with what confidence, at which line.
///
/// Carries the resolution `kind` (not just `confidence`) so a consumer that
/// needs to tell an unverified maybe-edge (e.g. `R6MultiOwnerCandidate`) apart
/// from a "normal" NameOnly demotion doesn't have to re-derive it from
/// confidence alone (F2, P3 review-fix wave). This struct is internal-only
/// (no Serialize) and not otherwise cached, so a plain enum field is the
/// simplest option — no derived `is_candidate: bool` needed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedCallEdge {
    pub caller: FunctionId,
    pub call_site_line: usize,
    pub confidence: ResolutionConfidence,
    pub kind: ResolutionKind,
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

fn supports_import_member_resolution(file: &str) -> bool {
    file.ends_with(".py") || is_js_ts_import_member_file(file)
}

fn is_js_ts_import_member_file(file: &str) -> bool {
    matches!(
        file.rsplit_once('.').map(|(_, ext)| ext),
        Some("js" | "jsx" | "mjs" | "cjs" | "ts" | "tsx")
    )
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

/// P5 (Go func-value callbacks): package-scoped owner identity for a Go
/// struct type. Deliberately distinct from the bare-name indices used
/// elsewhere (`GoTypeProvider`'s `structs`/`methods`, `CallGraph::methods` /
/// `interface_impls`) — those collapse same-named types across packages,
/// which is an existing, accepted approximation for method/interface
/// dispatch. The S1 func-typed-field index must NOT inherit that collision:
/// a callback registration in one package must never feed an S3 hit for a
/// same-named struct in another (spec-review MAJOR-1).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct GoOwnerIdentity {
    /// The struct's declaring directory (dir-as-package convention, matching
    /// `ResolutionKind::SamePackage` / R4.5 and `dir_of`).
    pub package_dir: String,
    /// Bare struct/type name (no package qualifier).
    pub name: String,
}

/// Resolve a Go type reference (`T` or `pkg.T`, as written at `file`) to a
/// package-scoped owner identity, for the S1 func-value-field index (S2
/// registration scan + S3 gated invocation).
///
/// - Bare `T` is unambiguous: Go scoping rules mean it names a type in the
///   SAME package as `file`, i.e. the same directory — no lookup needed.
/// - Qualified `pkg.T` resolves `pkg` via `file`'s import map to a Go import
///   path, then narrows to an indexed directory whose basename matches the
///   import path's last segment (the same dir-as-package convention `dir_of`
///   already encodes elsewhere in this ladder). Zero or multiple matching
///   directories is ambiguous -> `None` (fail closed): per spec, an
///   unknown/ambiguous owner identity may feed a nav-only registration
///   record at most, and NEVER S3.
/// - A generic instantiation (`T[X]`) is out of scope (named-type
///   indirection) -> `None`.
pub fn resolve_go_owner_identity(
    type_text: &str,
    file: &str,
    imports: &BTreeMap<String, BTreeMap<String, String>>,
    package_basenames: &BTreeMap<String, BTreeSet<String>>,
) -> Option<GoOwnerIdentity> {
    let t = type_text
        .trim()
        .trim_start_matches('&')
        .trim_start_matches('*')
        .trim();
    if t.is_empty() || t.contains('[') {
        return None;
    }
    match t.rsplit_once('.') {
        None => Some(GoOwnerIdentity {
            package_dir: dir_of(file).to_string(),
            name: t.to_string(),
        }),
        Some((pkg, name)) => {
            if pkg.is_empty() || name.is_empty() {
                return None;
            }
            let import_path = imports.get(file)?.get(pkg)?;
            let seg = import_path.rsplit('/').next().unwrap_or(import_path);
            let dirs = package_basenames.get(seg)?;
            if dirs.len() != 1 {
                return None; // ambiguous basename -> fail closed
            }
            Some(GoOwnerIdentity {
                package_dir: dirs.iter().next().unwrap().clone(),
                name: name.to_string(),
            })
        }
    }
}

/// P5 (Go func-value callbacks, S2): resolve a bare identifier used as a
/// VALUE (not a call) to the unique in-repo free function it names, using the
/// SAME same-package/import conventions as ordinary Go name resolution
/// (R4/R4.5) — same-file wins, else a single same-directory (package) free
/// function. Deliberately stops there: "NEVER bare cross-package name
/// matching" (spec-review MAJOR) means we do not fall through to a
/// repo-wide/FreeMulti search the way R5 does for calls.
pub fn resolve_go_bare_value_ref(
    functions: &BTreeMap<String, Vec<FunctionId>>,
    method_owners: &BTreeMap<FunctionId, String>,
    caller_file: &str,
    name: &str,
) -> Option<FunctionId> {
    let ids = functions.get(name)?;
    let free: Vec<&FunctionId> = ids
        .iter()
        .filter(|fid| !method_owners.contains_key(*fid))
        .collect();
    let local: Vec<&FunctionId> = free
        .iter()
        .copied()
        .filter(|f| f.file == caller_file)
        .collect();
    if local.len() == 1 {
        return Some(local[0].clone());
    }
    if !local.is_empty() {
        return None; // >1 same-file free fn of this name: ambiguous, not our problem to disambiguate
    }
    let dir = dir_of(caller_file);
    let same_pkg: Vec<&FunctionId> = free
        .iter()
        .copied()
        .filter(|f| dir_of(&f.file) == dir)
        .collect();
    if same_pkg.len() == 1 {
        return Some(same_pkg[0].clone());
    }
    None
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

/// Receiver classifier output. `materialized` means the qualifier was proven to
/// be a local receiver binding even when its static type is unresolved/poisoned.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ReceiverClassification {
    pub recovered: Option<RecoveredReceiver>,
    pub materialized: bool,
}

impl ReceiverClassification {
    fn none() -> Self {
        Self::default()
    }

    fn recovered(recovered: RecoveredReceiver) -> Self {
        Self {
            recovered: Some(recovered),
            materialized: true,
        }
    }

    fn materialized_only() -> Self {
        Self {
            recovered: None,
            materialized: true,
        }
    }
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
    /// 0-indexed call start byte. Binding recovery only considers local bindings
    /// starting before this byte.
    pub call_start_byte: usize,
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
    fn classify(&self, ctx: ReceiverCtx<'_>) -> ReceiverClassification;
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
/// Runs the qualifier/keyword/recv-var gate, then the typed-param /
/// constructor-local scan (and optionally `var` declarations when `recover_var`
/// is true), peeled + owner-keyed. Python still scans when the qualifier also
/// names an import so local receiver bindings can suppress R3.
fn classify_simple_ident(ctx: &ReceiverCtx<'_>, recover_var: bool) -> ReceiverClassification {
    use crate::languages::Language;
    if !matches!(
        ctx.parsed.language,
        Language::Rust | Language::Go | Language::Python
    ) {
        return ReceiverClassification::none();
    }
    let Some(q) = ctx.qualifier else {
        return ReceiverClassification::none();
    };
    let simple = !q.is_empty() && q.chars().all(|c| c.is_alphanumeric() || c == '_');
    let is_kw = matches!(q, "self" | "this" | "cls");
    let is_recv = ctx.recv_var == Some(q);
    let is_import = ctx.file_imports.map(|m| m.contains_key(q)).unwrap_or(false);
    if !(simple && !is_kw && !is_recv) {
        return ReceiverClassification::none();
    }
    if is_import && !matches!(ctx.parsed.language, Language::Python) {
        return ReceiverClassification::none();
    }
    let (type_found, binding_count) = ctx.parsed.receiver_type_in_fn(
        &ctx.fn_node,
        q,
        ctx.call_line,
        ctx.call_start_byte,
        recover_var,
    );
    let Some((ty, how)) = type_found else {
        // Bindings exist but type is unrecoverable (e.g. `for x in items:`,
        // `with ... as x:`, shadow/destructure) — signal materialized so R3/R3b
        // rungs are suppressed, preventing false edges from import/owner-key
        // collision with the receiver variable name.
        if binding_count > 0 && matches!(ctx.parsed.language, Language::Python) {
            return ReceiverClassification::materialized_only();
        }
        return ReceiverClassification::none();
    };
    let static_type = owner_key(&peel_type(&ty));
    if matches!(ctx.parsed.language, Language::Python)
        && ctx
            .file_imports
            .is_some_and(|m| m.contains_key(&static_type) || m.contains_key("*"))
    {
        return ReceiverClassification::materialized_only();
    }
    ReceiverClassification::recovered(RecoveredReceiver {
        static_type,
        recovery: how,
    })
}

/// PR-1 P6-lite recovery shape with `recover_var = false`. Python keeps the
/// materialized-receiver shadowing fix from the shared classifier.
pub fn legacy_recover(ctx: &ReceiverCtx<'_>) -> Option<RecoveredReceiver> {
    classify_simple_ident(ctx, false).recovered
}

/// `legacy` — no expanded forms such as Go var/type assertions.
pub struct LegacyClassifier;
impl ReceiverClassifier for LegacyClassifier {
    fn classify(&self, ctx: ReceiverCtx<'_>) -> ReceiverClassification {
        classify_simple_ident(&ctx, false)
    }
}

/// `expanded` — `legacy` ∪ the new forms.
pub struct ExpandedClassifier {
    pub type_assertion: bool,
    pub var_local: bool,
}
impl ReceiverClassifier for ExpandedClassifier {
    fn classify(&self, ctx: ReceiverCtx<'_>) -> ReceiverClassification {
        let simple = classify_simple_ident(&ctx, self.var_local);
        if simple.materialized {
            return simple;
        }
        if self.type_assertion {
            if let Some(r) = recover_type_assertion(&ctx) {
                return ReceiverClassification::recovered(r);
            }
        }
        ReceiverClassification::none()
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
    /// P5 S3: `(recovered_recv_type, call_name)` is a known Go func-typed
    /// struct field, but its recorded registration targets exceed the fan-out
    /// cap (>3 distinct callees) — too diffuse for a useful NameOnly edge.
    /// Kept dropped (not resolved), but classified separately from
    /// `ExternalReceiver` for call-stats telemetry.
    FuncValueFanout,
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

    /// P5 S3 (Go func-value callbacks): consulted from the Go interface-consult
    /// miss path (resolve_call_site_full), ONLY after concrete `owner_lookup`
    /// AND `interface_impls` have both missed or arity-filtered to empty.
    /// `(recv_ty, name)` is the receiver's recovered static type and the
    /// called method/field name at the call site being resolved — e.g.
    /// `cmd.Run()` with `recv_ty = "Command"`, `name = "Run"`.
    ///
    /// If S1 (`go_func_typed_fields`) confirms `(owner, name)` is a func-typed
    /// struct field, resolve to the DISTINCT S2 registration targets recorded
    /// for that field: 1..=3 -> `demoted(.., FuncValueField)` (NameOnly);
    /// >3 -> keep dropped, but reclassified `DropReason::FuncValueFanout`
    /// (too diffuse for a useful edge). Zero known registrations, an
    /// unresolvable/ambiguous owner identity, or a field that isn't
    /// func-typed all fall through to the existing `ExternalReceiver` drop —
    /// unchanged behavior for every case S1/S2 can't confirm.
    fn func_value_field_or_external_drop(
        &self,
        recv_ty: &str,
        name: &str,
        caller_file: &str,
    ) -> ResolutionOutcome<'_> {
        let Some(owner) = resolve_go_owner_identity(
            recv_ty,
            caller_file,
            &self.imports,
            &self.go_package_basenames,
        ) else {
            return ResolutionOutcome::dropped(DropReason::ExternalReceiver);
        };
        if !self
            .go_func_typed_fields
            .contains(&(owner.clone(), name.to_string()))
        {
            return ResolutionOutcome::dropped(DropReason::ExternalReceiver);
        }
        let field_key = (owner, name.to_string());
        let targets: BTreeSet<&FunctionId> = self
            .go_registrations
            .iter()
            .filter(|r| r.field_key.as_ref() == Some(&field_key))
            .map(|r| &r.target)
            .collect();
        match targets.len() {
            0 => ResolutionOutcome::dropped(DropReason::ExternalReceiver),
            1..=3 => ResolutionOutcome::hit(demoted(targets, ResolutionKind::FuncValueField)),
            _ => ResolutionOutcome::dropped(DropReason::FuncValueFanout),
        }
    }

    fn self_owner_lookup_same_class(
        &self,
        owner: &str,
        name: &str,
        caller: &FunctionId,
    ) -> Option<Vec<ResolvedCallee<'_>>> {
        let ids = self.methods.get(&(owner.to_string(), name.to_string()))?;
        if self.method_class_span_ambiguous.contains(caller)
            || ids
                .iter()
                .any(|fid| self.method_class_span_ambiguous.contains(fid))
        {
            return self.owner_lookup(owner, name);
        }

        let caller_span = *self.method_class_span.get(caller)?;
        let same_class: Vec<&FunctionId> = ids
            .iter()
            .filter(|fid| {
                fid.file == caller.file && self.method_class_span.get(*fid) == Some(&caller_span)
            })
            .collect();
        match same_class.len() {
            0 => None,
            1 => Some(exact(same_class, ResolutionKind::QualifiedOwner)),
            _ => Some(demoted(same_class, ResolutionKind::QualifiedOwner)),
        }
    }

    /// Slice 1b: inherited-self resolution hook.
    ///
    /// When `self.method()` has no same-class definition, check if the caller's
    /// class has exactly ONE direct same-file base that provides exactly one
    /// method with the given name. Depth-1 only: never recurses to grandparents.
    fn inherited_direct_base<'a>(
        &'a self,
        caller: &FunctionId,
        name: &str,
    ) -> Option<Vec<ResolvedCallee<'a>>> {
        use crate::call_graph::ClassBaseLink;

        // Same ambiguous guard as self_owner_lookup_same_class: if the caller's
        // FunctionId has an ambiguous class span, we cannot trust the span for
        // base-class lookup either.
        if self.method_class_span_ambiguous.contains(caller) {
            return None;
        }

        let caller_span = *self.method_class_span.get(caller)?;
        let bases = self.class_bases.get(&(caller.file.clone(), caller_span))?;

        // Single-inheritance only: >1 base slots → drop.
        if bases.len() != 1 {
            return None;
        }

        let base = &bases[0];
        let (base_span, base_owner) = match base {
            ClassBaseLink::SameFile { span, owner } => (*span, owner.as_str()),
            ClassBaseLink::Barrier => return None,
        };

        // Look up (base_owner, name) in the methods index, filtered to the
        // base class's exact span in the same file.
        let ids = self
            .methods
            .get(&(base_owner.to_string(), name.to_string()))?;
        let in_base: Vec<&FunctionId> = ids
            .iter()
            .filter(|fid| {
                fid.file == caller.file
                    && self.method_class_span.get(*fid) == Some(&base_span)
                    && !self.method_class_span_ambiguous.contains(*fid)
            })
            .collect();

        if in_base.len() == 1 {
            Some(exact(in_base, ResolutionKind::SelfReceiver))
        } else {
            None
        }
    }

    fn recovered_receiver_direct_method<'a>(
        &'a self,
        caller_file: &str,
        receiver_owner: &str,
        method_name: &str,
        recovered_kind: ResolutionKind,
    ) -> RecoveredDirectMethod<'a> {
        let Some(receiver_span) = self
            .clean_class_spans
            .get(&(caller_file.to_string(), receiver_owner.to_string()))
        else {
            return RecoveredDirectMethod::Miss;
        };
        let Some(ids) = self
            .methods
            .get(&(receiver_owner.to_string(), method_name.to_string()))
        else {
            return RecoveredDirectMethod::Miss;
        };
        let same_class: Vec<&FunctionId> = ids
            .iter()
            .filter(|fid| {
                fid.file == caller_file && self.method_class_span.get(*fid) == Some(receiver_span)
            })
            .collect();
        if same_class.is_empty() {
            return RecoveredDirectMethod::Miss;
        }
        if same_class
            .iter()
            .any(|fid| self.method_class_span_ambiguous.contains(*fid))
            || same_class.len() != 1
        {
            return RecoveredDirectMethod::Blocked;
        }
        RecoveredDirectMethod::Hit(exact(same_class, recovered_kind))
    }

    fn inherited_recovered_receiver_direct_base<'a>(
        &'a self,
        caller_file: &str,
        receiver_owner: &str,
        method_name: &str,
        recovered_kind: ResolutionKind,
    ) -> Option<Vec<ResolvedCallee<'a>>> {
        use crate::call_graph::ClassBaseLink;

        let receiver_span = *self
            .clean_class_spans
            .get(&(caller_file.to_string(), receiver_owner.to_string()))?;
        let bases = self
            .class_bases
            .get(&(caller_file.to_string(), receiver_span))?;
        if bases.len() != 1 {
            return None;
        }
        let (base_span, base_owner) = match &bases[0] {
            ClassBaseLink::SameFile { span, owner } => (*span, owner.as_str()),
            ClassBaseLink::Barrier => return None,
        };
        let ids = self
            .methods
            .get(&(base_owner.to_string(), method_name.to_string()))?;
        let in_base: Vec<&FunctionId> = ids
            .iter()
            .filter(|fid| {
                fid.file == caller_file
                    && self.method_class_span.get(*fid) == Some(&base_span)
                    && !self.method_class_span_ambiguous.contains(*fid)
            })
            .collect();
        if in_base.len() == 1 {
            Some(exact(in_base, recovered_kind))
        } else {
            None
        }
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
                    let narrow = matches!(
                        crate::languages::Language::from_path(&caller.file),
                        Some(
                            crate::languages::Language::Python
                                | crate::languages::Language::JavaScript
                                | crate::languages::Language::TypeScript
                                | crate::languages::Language::Tsx
                        )
                    );
                    let looked_up = if narrow {
                        self.self_owner_lookup_same_class(owner, name, caller)
                    } else {
                        self.owner_lookup(owner, name)
                    };
                    if let Some(mut resolved) = looked_up {
                        for callee in &mut resolved {
                            if callee.kind == ResolutionKind::QualifiedOwner {
                                callee.kind = ResolutionKind::SelfReceiver;
                            }
                        }
                        return ResolutionOutcome::hit(resolved);
                    }
                    // Slice 1b: inherited-self hook. When same-class returns None
                    // for a Py/JS/TS self-call, check if the caller's class has
                    // exactly ONE direct same-file base that provides the method.
                    if narrow {
                        if let Some(inherited) = self.inherited_direct_base(caller, name) {
                            return ResolutionOutcome::hit(inherited);
                        }
                    }
                }
                ResolutionOutcome::dropped(DropReason::UnknownName)
            }
            Some(q) => {
                let caller_lang = crate::languages::Language::from_path(&site.caller.file);
                // A materialized Rust receiver outcome means this is a value-method call
                // `recv.method()`: the qualifier is a receiver expression, NOT a module or
                // type name. The receiver's static type (the Rust branch below) is
                // authoritative and must pre-empt both the import-qualifier (R3) and the
                // owner-key (R3b) interpretations. Recall-safe: receiver_outcome == Some
                // only for value-method receiver syntax, so R3/R3b never held a correct edge
                // for these sites.
                let rust_recv_materialized = caller_lang == Some(crate::languages::Language::Rust)
                    && site.receiver_outcome.is_some();
                let recovered_recv_materialized =
                    matches!(caller_lang, Some(crate::languages::Language::Python))
                        && site.receiver_materialized;
                let recv_materialized = rust_recv_materialized || recovered_recv_materialized;

                // R3: imported-module qualifier. If an import matches, the
                // narrowed set is final; empty means the call is external.
                if !recv_materialized {
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
                if !recv_materialized && is_simple_ident(q) {
                    if let Some(mut resolved) = self.owner_lookup(q, name) {
                        for callee in &mut resolved {
                            if callee.kind == ResolutionKind::QualifiedOwner {
                                callee.kind = ResolutionKind::QualifierOwner;
                            }
                        }
                        return ResolutionOutcome::hit(resolved);
                    }
                }

                if caller_lang == Some(crate::languages::Language::Rust) {
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
                    if caller_lang == Some(crate::languages::Language::Python) {
                        let clean_key = (caller.file.clone(), recv_ty.to_string());
                        if self.clean_class_spans.contains_key(&clean_key) {
                            match self.recovered_receiver_direct_method(
                                &caller.file,
                                recv_ty,
                                name,
                                recovered_kind,
                            ) {
                                RecoveredDirectMethod::Hit(resolved) => {
                                    return ResolutionOutcome::hit(resolved)
                                }
                                RecoveredDirectMethod::Blocked => {}
                                RecoveredDirectMethod::Miss => {
                                    if let Some(resolved) = self
                                        .inherited_recovered_receiver_direct_base(
                                            &caller.file,
                                            recv_ty,
                                            name,
                                            recovered_kind,
                                        )
                                    {
                                        return ResolutionOutcome::hit(resolved);
                                    }
                                }
                            }
                        } else if let Some(mut resolved) = self.owner_lookup(recv_ty, name) {
                            for callee in &mut resolved {
                                if callee.kind == ResolutionKind::QualifiedOwner {
                                    callee.kind = recovered_kind;
                                }
                                // Trait-CHA hits keep TraitCha (dyn Trait receivers).
                            }
                            return ResolutionOutcome::hit(resolved);
                        }
                    } else {
                        match self.owner_lookup(recv_ty, name) {
                            Some(mut resolved) => {
                                for callee in &mut resolved {
                                    if callee.kind == ResolutionKind::QualifiedOwner {
                                        callee.kind = recovered_kind;
                                    }
                                    // Trait-CHA hits keep TraitCha (dyn Trait receivers).
                                }
                                return ResolutionOutcome::hit(resolved);
                            }
                            // Gate the interface consult to Go callers: P6-lite receiver
                            // recovery also fires for Rust, and `interface_impls` is Go-only,
                            // so an un-gated consult could mint a cross-language edge (e.g. a
                            // Rust `x.Go()` matching a Go interface named the same). Mirrors the
                            // language gate at the C-only free-fn fallback below.
                            None if caller_lang == Some(crate::languages::Language::Go) => {
                                match crate::resolution::iface_key(recv_ty) {
                                    Some(k) => {
                                        match self.interface_impls.get(&(k, name.to_string())) {
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
                                                    // P5 S3: interface dispatch arity-filtered to
                                                    // empty — try the func-typed-field registration
                                                    // index before the ExternalReceiver drop.
                                                    return self.func_value_field_or_external_drop(
                                                        recv_ty,
                                                        name,
                                                        &site.caller.file,
                                                    );
                                                } else {
                                                    return ResolutionOutcome::hit(exact(
                                                        kept,
                                                        ResolutionKind::InterfaceDispatch,
                                                    ));
                                                }
                                            }
                                            _ => {
                                                // P5 S3: no interface impls at all for this
                                                // (interface, method) — try the func-typed-field
                                                // registration index before the drop.
                                                return self.func_value_field_or_external_drop(
                                                    recv_ty,
                                                    name,
                                                    &site.caller.file,
                                                );
                                            }
                                        }
                                    }
                                    None => {
                                        // `recv_ty` had no bare name at all (`iface_key` returns
                                        // `None` only for a generic instantiation, e.g. `Foo[T]`) —
                                        // still worth an S3 attempt since func-value-field owner
                                        // resolution works directly off `recv_ty`, not `iface_key`.
                                        return self.func_value_field_or_external_drop(
                                            recv_ty,
                                            name,
                                            &site.caller.file,
                                        );
                                    }
                                }
                            }
                            None => {
                                return ResolutionOutcome::dropped(DropReason::ExternalReceiver);
                            }
                        }
                    }
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

                // P3: unknown-receiver, multi-owner collision — a labeled
                // maybe-edge beats a silent drop for Python/JS/TS/Tsx, where
                // this is the dominant unresolved-rate driver. Rust/Go keep
                // the precision-floor drop (fixture-pinned:
                // r6_multi_owner_drop). Cap on per-site TARGET fanout
                // (method_ids, after the deterministic filtering above), not
                // owner count: 2 owners can still hold >3 same-name defs.
                if owners.len() >= 2
                    && method_ids.len() <= 3
                    && matches!(
                        crate::languages::Language::from_path(&caller.file),
                        Some(
                            crate::languages::Language::Python
                                | crate::languages::Language::JavaScript
                                | crate::languages::Language::TypeScript
                                | crate::languages::Language::Tsx
                        )
                    )
                {
                    return ResolutionOutcome::hit(demoted(
                        method_ids,
                        ResolutionKind::R6MultiOwnerCandidate,
                    ));
                }
                ResolutionOutcome::dropped(DropReason::MultiOwnerCollision)
            }
            None => {
                // R4c: import-member resolution (Python/JS/TS).
                // Must fire before the functions.get(name) check because aliases
                // mean the call-site name ("p") differs from the function name
                // ("process"), so the index won't have a hit on the aliased name.
                if site.qualifier.is_none() && supports_import_member_resolution(&caller.file) {
                    if let Some(bindings) = self.import_bindings.get(&caller.file) {
                        if let Some(binding) = bindings.iter().find(|b| {
                            b.local == name
                                && b.eligible
                                && matches!(
                                    b.kind,
                                    crate::call_graph::ImportBindingKind::MemberImport
                                )
                        }) {
                            let is_js_ts = is_js_ts_import_member_file(&caller.file);
                            if is_js_ts
                                && self
                                    .js_ts_function_locals
                                    .get(caller)
                                    .is_some_and(|locals| locals.contains(name))
                            {
                                // A parameter or local binding shadows the imported
                                // local name; do not mint an exact import edge.
                            } else {
                                let member = binding.member.as_deref().unwrap_or(name);
                                if let Some(ids) = self.functions.get(member) {
                                    // Filter to free functions in matching files.
                                    // Python `from m import f` imports a module-level
                                    // name, so methods (class-scoped) are excluded.
                                    let matched: Vec<&FunctionId> = ids
                                    .iter()
                                    .filter(|fid| {
                                        if self.method_owners.contains_key(*fid) {
                                            return false;
                                        }
                                        if is_js_ts {
                                            crate::call_graph::file_matches_js_ts_relative_module_exact(
                                                &fid.file,
                                                &binding.module_path,
                                                &caller.file,
                                                &self.indexed_files,
                                            )
                                                && self
                                                    .js_ts_exported_functions
                                                    .get(&fid.file)
                                                    .is_some_and(|exports| exports.contains(member))
                                        } else {
                                            crate::call_graph::file_matches_module(
                                                &fid.file,
                                                &binding.module_path,
                                                &caller.file,
                                                &self.indexed_files,
                                            )
                                                // Only accept module-level functions, not nested defs.
                                                && self
                                                .module_bindings
                                                .get(&fid.file)
                                                .and_then(|mb| mb.get(member))
                                                .map_or(false, |k| {
                                                    matches!(
                                                        k,
                                                        crate::call_graph::ModuleBindingKind::FunctionDef
                                                    )
                                                })
                                        }
                                    })
                                    .collect();
                                    match matched.len() {
                                        1 => {
                                            return ResolutionOutcome::hit(exact(
                                                matched,
                                                ResolutionKind::ImportMember,
                                            ))
                                        }
                                        n if n > 1 => {
                                            return ResolutionOutcome::hit(demoted(
                                                matched,
                                                ResolutionKind::ImportMember,
                                            ))
                                        }
                                        _ => {} // fall through to R5
                                    }
                                }
                            }
                        }
                    }
                }

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
                            kind: r.kind,
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

        // ②B: the leading segment must bind directly to one in-repo Item -- either
        // a resolved `Item` binding, or a single `use`/re-export `Pending` hop that
        // resolves unambiguously to one in-repo `Item` ((A) slice). Prove this from
        // the binding shape, not Candidate provenance.
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

/// Prove the leading type segment binds directly at the call site: the single
/// binding the call site sees for `leading` is either a non-glob
/// `BindTarget::Resolved(Target::Item)`, or a single `use`/re-export
/// `BindTarget::Pending` that resolves unambiguously to one scope-bearing in-repo
/// `Item` ((A) slice, via `pending_resolves_to_single_in_repo_item`)
/// (§8.2 decision ②B).
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
        [b] => match &b.target {
            // Directly bound in-repo type -- unchanged (②B).
            BindTarget::Resolved(Target::Item { .. }) => true,
            // (A) slice: a single `use`/re-export chain. Fold THIS binding's own
            // import path via the engine; prune only if it resolves UNAMBIGUOUSLY
            // to one scope-bearing in-repo `Item` (Rust `use` resolution is
            // deterministic). Ambiguous / poisoned / unresolved / `Target::External`
            // / a non-scope-bearing item / multiple -> keep-all (we do NOT prune).
            BindTarget::Pending(path, anchor) => pending_resolves_to_single_in_repo_item(
                graph, path, anchor, b.scope, b.ns, &q.at, &policy,
            ),
            _ => false,
        },
        _ => false,
    }
}

/// (A) slice helper: does the leading type segment's single visible `Pending`
/// `use`/re-export binding resolve UNAMBIGUOUSLY to exactly one scope-bearing
/// in-repo `Item`? Re-resolves the binding's **own anchored import path** via the
/// same `resolve_path` call shape the final callable step uses (`resolution.rs`
/// final step), so the gate follows the same `use` chain. Returns `true` only on
/// `ResStatus::Resolved` with a single `Target::Item { owns: Some(scope), .. }`
/// whose defining `scope` maps to a known in-repo `FileId`; every other shape
/// (`ResolvedSet`/`Ambiguous`/`Poisoned`/`Unresolved`, a `Target::External`/
/// `Target::Local` candidate, `owns: None`, or >1) -> `false` -> keep-all. Note
/// there is no `External` engine *status*: externals surface as a
/// `Target::External` candidate *target* under a `Resolved` result, so we inspect
/// the candidate target, not the status (spec §3/§4).
#[allow(clippy::too_many_arguments)]
fn pending_resolves_to_single_in_repo_item(
    graph: &ScopeGraph,
    path: &RawPath,
    anchor: &Anchor,
    from: ScopeId,
    final_ns: NamespaceId,
    at: &SourceLoc,
    policy: &RustPolicy,
) -> bool {
    // `from` is the re-export author's scope (`b.scope`); `final_ns` is the final
    // segment's namespace (`b.ns`, NS_TYPE for the type binding); the prefix
    // (scope-bearing) segments use NS_TYPE.
    let res = resolve_path(graph, path, final_ns, anchor, from, NS_TYPE, at, policy);
    matches!(
        (res.status, res.candidates.as_slice()),
        (
            ResStatus::Resolved,
            [Candidate {
                target: Target::Item { owns: Some(scope), .. },
                ..
            }],
        ) if graph_file_for_scope(graph, *scope).is_some()
    )
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
///
/// `pub(crate)` (widened from private for P5): `GoOwnerIdentity` resolution
/// and the S1 package-basename index (`call_graph.rs`) reuse this exact
/// dir-as-package convention rather than inventing a second one.
pub(crate) fn dir_of(path: &str) -> &str {
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
mod self_receiver_same_class_tests {
    use super::*;
    use crate::ast::ParsedFile;
    use crate::languages::Language;
    use std::collections::BTreeMap;

    fn files(pairs: &[(&str, &str)]) -> BTreeMap<String, ParsedFile> {
        pairs
            .iter()
            .map(|(p, s)| {
                let lang = Language::from_path(p).expect("known extension");
                (
                    (*p).to_string(),
                    ParsedFile::parse(p, s, lang).expect("parse"),
                )
            })
            .collect()
    }

    fn resolve_self_call<'a>(
        cg: &'a CallGraph,
        caller_file: &str,
        caller_name: &str,
        callee: &str,
    ) -> ResolutionOutcome<'a> {
        let caller = cg
            .functions
            .get(caller_name)
            .and_then(|v| v.iter().find(|f| f.file == caller_file))
            .expect("caller fn");
        let site = cg
            .calls
            .get(caller)
            .and_then(|sites| sites.iter().find(|s| s.callee_name == callee))
            .expect("call site");
        cg.resolve_call_site_full(site)
    }

    #[test]
    fn self_call_cross_file_collision_resolves_exact_to_caller_class() {
        let cg = CallGraph::build(&files(&[
            (
                "a.py",
                "class C:\n    def m(self):\n        return 1\n    def run(self):\n        return self.m()\n",
            ),
            ("b.py", "class C:\n    def m(self):\n        return 2\n"),
        ]));
        let out = resolve_self_call(&cg, "a.py", "run", "m");
        assert_eq!(out.resolved.len(), 1, "single same-class target");
        assert_eq!(out.resolved[0].target.file, "a.py");
        assert_eq!(out.resolved[0].confidence, ResolutionConfidence::Exact);
        assert_eq!(out.resolved[0].kind, ResolutionKind::SelfReceiver);
    }

    #[test]
    fn self_call_absent_on_caller_class_cross_file_drops() {
        let cg = CallGraph::build(&files(&[
            (
                "a.py",
                "class Widget:\n    def render(self):\n        return 1\n",
            ),
            (
                "b.py",
                "class Widget:\n    def draw(self):\n        return self.render()\n",
            ),
        ]));
        let out = resolve_self_call(&cg, "b.py", "draw", "render");
        assert!(
            out.resolved.is_empty(),
            "must NOT bind to a.py's unrelated Widget"
        );
        assert_eq!(out.drop, Some(DropReason::UnknownName));
    }

    #[test]
    fn self_call_same_file_nested_duplicate_class_drops() {
        let cg = CallGraph::build(&files(&[
            (
                "a.py",
                "def o1():\n    class C:\n        def f(self):\n            return self.m()\ndef o2():\n    class C:\n        def m(self):\n            return 1\n",
            ),
            ("b.py", "class C:\n    def m(self):\n        return 2\n"),
        ]));
        let out = resolve_self_call(&cg, "a.py", "f", "m");
        assert!(
            out.resolved.is_empty(),
            "o1.C has no m; must not bind to o2.C or b.C"
        );
        assert_eq!(out.drop, Some(DropReason::UnknownName));
    }

    #[test]
    fn self_call_same_line_duplicate_class_js_drops() {
        let cg = CallGraph::build(&files(&[(
            "a.js",
            "class C { f() { return this.m(); } } class C { m() { return 1; } }\n",
        )]));
        let out = resolve_self_call(&cg, "a.js", "f", "m");
        assert!(
            out.resolved.is_empty(),
            "f's class has no m; the same-line other C must not bind"
        );
        assert_eq!(out.drop, Some(DropReason::UnknownName));
    }

    #[test]
    fn self_call_same_line_distinct_class_method_collision_fails_open() {
        let cg = CallGraph::build(&files(&[(
            "a.js",
            "class A { m() { return 1; } run() { return this.m(); } } class B { m() { return 2; } }\n",
        )]));
        let out = resolve_self_call(&cg, "a.js", "run", "m");
        assert_eq!(out.resolved.len(), 1, "ambiguous class span must fail open");
        assert_eq!(out.resolved[0].target.file, "a.js");
        assert_eq!(out.resolved[0].confidence, ResolutionConfidence::Exact);
        assert_eq!(out.resolved[0].kind, ResolutionKind::SelfReceiver);
    }

    #[test]
    fn self_call_static_plus_instance_same_name_nameonly() {
        let cg = CallGraph::build(&files(&[(
            "a.js",
            "class C { static m() {} m() {} run() { return this.m(); } }\n",
        )]));
        let out = resolve_self_call(&cg, "a.js", "run", "m");
        assert_eq!(
            out.resolved.len(),
            2,
            "both same-class same-name candidates kept"
        );
        assert!(out
            .resolved
            .iter()
            .all(|c| c.confidence == ResolutionConfidence::NameOnly));
    }

    #[test]
    fn go_receiver_var_call_unchanged() {
        let cg = CallGraph::build(&files(&[(
            "a.go",
            "package p\ntype T struct{}\nfunc (r T) other() int { return 1 }\nfunc (r T) run() int { return r.other() }\n",
        )]));
        let out = resolve_self_call(&cg, "a.go", "run", "other");
        assert_eq!(out.resolved.len(), 1);
        assert_eq!(out.resolved[0].confidence, ResolutionConfidence::Exact);
    }
}

#[cfg(test)]
mod scope_resolution_predicate_tests {
    use super::*;
    use crate::call_graph::{CallSite, CallSiteOrigin, FunctionId};
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
            receiver_materialized: false,
            arg_count: None,
            arg_spread: false,
            receiver_outcome: None,
            origin: CallSiteOrigin::Source,
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

// ---------------------------------------------------------------------------
// P5 S3: gated func-value-field invocation resolution
// ---------------------------------------------------------------------------

#[cfg(test)]
mod go_func_value_field_resolution_tests {
    use crate::ast::ParsedFile;
    use crate::call_graph::CallGraph;
    use crate::languages::Language::Go;
    use crate::resolution::{DropReason, ResolutionConfidence, ResolutionKind};
    use std::collections::BTreeMap;

    fn build(files: &[(&str, &str)]) -> CallGraph {
        let mut map = BTreeMap::new();
        for (path, src) in files {
            map.insert(path.to_string(), ParsedFile::parse(path, src, Go).unwrap());
        }
        CallGraph::build(&map)
    }

    /// The ORIGINAL (qualified, receiver-typed) call site for `fn_name`'s
    /// invocation — as opposed to any Level-4 struct-callback synthetic site
    /// `recompute_indirect_calls` may ALSO add alongside it (pre-existing,
    /// unrelated to P5: Level-4 is a language-general text scan over `.field =
    /// value` assignment syntax, so it also fires on Go's field-assignment
    /// registration form; see the P5 report for detail). Selecting the
    /// qualified site directly makes this test immune to that interaction.
    fn qualified_site<'a>(
        cg: &'a CallGraph,
        caller_fn: &str,
        callee: &str,
    ) -> &'a crate::call_graph::CallSite {
        let caller_id = cg.functions.get(caller_fn).unwrap().first().unwrap();
        cg.calls
            .get(caller_id)
            .unwrap()
            .iter()
            .find(|s| s.callee_name == callee && s.qualifier.is_some())
            .expect("qualified call site present")
    }

    #[test]
    fn unique_registration_target_demotes_to_func_value_field() {
        let cg = build(&[(
            "main.go",
            "package main\n\
type Command struct {\n\tRun func()\n}\n\
func helper() {}\n\
func register() *Command {\n\treturn &Command{Run: helper}\n}\n\
func invoke(cmd *Command) {\n\tcmd.Run()\n}\n",
        )]);
        let site = qualified_site(&cg, "invoke", "Run");
        let outcome = cg.resolve_call_site_full(site);
        assert_eq!(outcome.drop, None);
        assert_eq!(outcome.resolved.len(), 1);
        assert_eq!(outcome.resolved[0].kind, ResolutionKind::FuncValueField);
        assert_eq!(
            outcome.resolved[0].confidence,
            ResolutionConfidence::NameOnly
        );
        assert_eq!(outcome.resolved[0].target.name, "helper");
    }

    #[test]
    fn two_registration_targets_both_demote_to_func_value_field() {
        let cg = build(&[(
            "main.go",
            "package main\n\
type Command struct {\n\tRun func()\n}\n\
func h1() {}\n\
func h2() {}\n\
func register_a() *Command { return &Command{Run: h1} }\n\
func register_b() *Command { return &Command{Run: h2} }\n\
func invoke(cmd *Command) {\n\tcmd.Run()\n}\n",
        )]);
        let site = qualified_site(&cg, "invoke", "Run");
        let outcome = cg.resolve_call_site_full(site);
        assert_eq!(outcome.drop, None);
        assert_eq!(outcome.resolved.len(), 2);
        let names: std::collections::BTreeSet<&str> = outcome
            .resolved
            .iter()
            .map(|c| c.target.name.as_str())
            .collect();
        assert!(names.contains("h1") && names.contains("h2"));
        assert!(outcome
            .resolved
            .iter()
            .all(|c| c.kind == ResolutionKind::FuncValueField
                && c.confidence == ResolutionConfidence::NameOnly));
    }

    /// F1 (BLOCKER fix): binding adjudication — nav (`resolve_call_site_full`)
    /// keeps all 1..=3 registered targets unfiltered (asserted above, byte
    /// unchanged), but non-nav consumers going through the thin
    /// `resolve_call_site` wrapper must accept a `FuncValueField` hit only
    /// when there is exactly ONE registered target. Two registered targets on
    /// the same field (the `Command.Run = safe` / `Command.Run = sink`
    /// scenario) must not create a Call/DataFlow edge into EITHER target for
    /// non-nav consumers, even though nav still shows both.
    #[test]
    fn two_target_func_value_field_is_filtered_from_resolve_call_site_but_not_from_full() {
        let cg = build(&[(
            "main.go",
            "package main\n\
type Command struct {\n\tRun func()\n}\n\
func h1() {}\n\
func h2() {}\n\
func register_a() *Command { return &Command{Run: h1} }\n\
func register_b() *Command { return &Command{Run: h2} }\n\
func invoke(cmd *Command) {\n\tcmd.Run()\n}\n",
        )]);
        let site = qualified_site(&cg, "invoke", "Run");

        // Nav path (resolve_call_site_full) is UNCHANGED: both targets, unfiltered.
        let full = cg.resolve_call_site_full(site);
        assert_eq!(full.resolved.len(), 2, "nav path must keep both targets");

        // Non-nav consumer path (resolve_call_site, the thin wrapper): the
        // fanout must be filtered out entirely, since every resolved entry
        // here is FuncValueField.
        let consumer = cg.resolve_call_site(site);
        assert!(
            consumer.is_empty(),
            "resolve_call_site must drop a 2-target FuncValueField fanout for non-nav \
             consumers, got {consumer:?}"
        );
    }

    /// F1 companion: the singleton case must be UNCHANGED end-to-end — all
    /// consumers (nav and non-nav) still see the edge when there is exactly
    /// one registered target.
    #[test]
    fn single_target_func_value_field_survives_resolve_call_site() {
        let cg = build(&[(
            "main.go",
            "package main\n\
type Command struct {\n\tRun func()\n}\n\
func helper() {}\n\
func register() *Command {\n\treturn &Command{Run: helper}\n}\n\
func invoke(cmd *Command) {\n\tcmd.Run()\n}\n",
        )]);
        let site = qualified_site(&cg, "invoke", "Run");

        let full = cg.resolve_call_site_full(site);
        assert_eq!(full.resolved.len(), 1);

        let consumer = cg.resolve_call_site(site);
        assert_eq!(
            consumer.len(),
            1,
            "resolve_call_site must keep the singleton FuncValueField hit"
        );
        assert_eq!(consumer[0].kind, ResolutionKind::FuncValueField);
        assert_eq!(consumer[0].target.name, "helper");
    }

    #[test]
    fn more_than_three_registration_targets_drops_as_fanout() {
        let cg = build(&[(
            "main.go",
            "package main\n\
type Command struct {\n\tRun func()\n}\n\
func h1() {}\n\
func h2() {}\n\
func h3() {}\n\
func h4() {}\n\
func register_a() *Command { return &Command{Run: h1} }\n\
func register_b() *Command { return &Command{Run: h2} }\n\
func register_c() *Command { return &Command{Run: h3} }\n\
func register_d() *Command { return &Command{Run: h4} }\n\
func invoke(cmd *Command) {\n\tcmd.Run()\n}\n",
        )]);
        let site = qualified_site(&cg, "invoke", "Run");
        let outcome = cg.resolve_call_site_full(site);
        assert!(outcome.resolved.is_empty());
        assert_eq!(outcome.drop, Some(DropReason::FuncValueFanout));
    }

    #[test]
    fn zero_known_registrations_keeps_external_receiver_drop() {
        let cg = build(&[(
            "main.go",
            "package main\n\
type Command struct {\n\tRun func()\n}\n\
func invoke(cmd *Command) {\n\tcmd.Run()\n}\n",
        )]);
        let site = qualified_site(&cg, "invoke", "Run");
        let outcome = cg.resolve_call_site_full(site);
        assert!(outcome.resolved.is_empty());
        assert_eq!(outcome.drop, Some(DropReason::ExternalReceiver));
    }

    #[test]
    fn non_go_caller_unaffected() {
        // A Rust receiver-typed call must never consult the Go-only
        // func-value-field index (mirrors the existing interface-consult
        // language gate).
        let mut map = BTreeMap::new();
        map.insert(
            "a.rs".to_string(),
            ParsedFile::parse(
                "a.rs",
                "struct Command { run: fn() }\nfn helper() {}\nfn invoke(cmd: &Command) { cmd.run(); }\n",
                crate::languages::Language::Rust,
            )
            .unwrap(),
        );
        let cg = CallGraph::build(&map);
        // No panics, and definitely no FuncValueField kind anywhere.
        for sites in cg.calls.values() {
            for site in sites {
                let outcome = cg.resolve_call_site_full(site);
                assert!(outcome
                    .resolved
                    .iter()
                    .all(|c| c.kind != ResolutionKind::FuncValueField));
            }
        }
    }
}
