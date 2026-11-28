//! `ResolutionKind` and `DropReason`: the classification vocabulary of call resolution.

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
    /// P7 S2: a Python `@property`/`@cached_property` LOAD access
    /// (`self.attr` same-class/single-base narrowed, or an unknown/`cls`
    /// receiver capped fanout) surfaced as a NameOnly nav edge. Never
    /// produced by `resolve_call_site_full` — mirrors `CallbackRegistration`:
    /// property accesses are not `CallSite`s (a synthetic CallSite here could
    /// mint a wrong-kind/Exact edge through the ordinary call ladder); this
    /// kind labels edges synthesized directly from
    /// `CallGraph::property_accesses` in
    /// `NavigationIndex::build_resolved_call_edges`. Unlike `FuncValueField`,
    /// there is no S3 resolve-time consult path at all (nav-only, no CPG/
    /// DataFlow consumer ever sees it).
    PropertyAccess,
    /// P9 S3: a Flask/FastAPI/Express route registration
    /// (`@app.route("/x")`, `app.get("/x", handler)`) surfaced as a NameOnly
    /// nav edge. Never produced by `resolve_call_site_full` — mirrors
    /// `PropertyAccess`: a route registration is not a `CallSite` (the
    /// registration line does not itself invoke the handler — it is an
    /// entrypoint/discoverability fact, not dataflow), so this kind labels
    /// edges synthesized directly from `CallGraph::framework_entries` in
    /// `NavigationIndex::build_resolved_call_edges`. Nav-only per the
    /// consumer-visibility doctrine — no S3 resolve-time consult path,
    /// never fed to taint/slice consumers (feeding these edges there would
    /// assert dataflow that doesn't exist at the registration line).
    FrameworkEntry,
    /// P11 S1/S5: a Go receiver recovered via a call-RHS short-var binding
    /// resolved through the `go_return_types` index (`d := newDemux(...)`).
    /// Split out of the generic `TypedParam` bucket so call-stats/nav telemetry
    /// can tell it apart from Rust's Lane-A `ReceiverRecovery::ReturnTyped`
    /// (scope-graph field/return-typed receivers), which this ALSO now labels
    /// distinctly for the same reason (was previously collapsed to
    /// `TypedParam`, hiding it from telemetry — S5 telemetry-honesty fix).
    ReturnTyped,
    /// P11 S2/S5: a Go receiver recovered via a 1-2 hop field-selector chain
    /// (`l.Listener.Accept()`) resolved through the `go_field_types` index.
    /// Also now the distinct label for Rust's Lane-A
    /// `ReceiverRecovery::FieldTyped` (previously collapsed to `TypedParam`).
    FieldTyped,
    /// P8: a parameter callback whose argument was resolved as one Exact
    /// FunctionId in the inbound caller's lexical context before minting the
    /// synthetic call site.
    ParameterCallback,
    /// E2: a JS-family call through a parameter, credited with the callables its admitted
    /// Exact Source callers supplied at the holder's stable slot. Always NameOnly.
    ArgumentFollow,
    /// Disabled staged contextual source owner; never persisted.
    ContextualOwner,
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
            ResolutionKind::PropertyAccess => "property_access",
            ResolutionKind::FrameworkEntry => "framework_entry",
            ResolutionKind::ReturnTyped => "return_typed",
            ResolutionKind::FieldTyped => "field_typed",
            ResolutionKind::ParameterCallback => "parameter_callback",
            ResolutionKind::ArgumentFollow => "argument_follow",
            ResolutionKind::ContextualOwner => "contextual_owner",
        }
    }
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
    /// P13: same-directory Go candidates existed, but package/build profile
    /// filtering proved none visible; do not fall through to FreeSingle.
    GoSamePkgAllFiltered,
    /// P17 R1(b): the selector is promoted from embedded concrete state whose
    /// true edge is deferred to the owner/profile-keyed promotion slice.
    ConcreteReceiverPromotedDeferred,
    /// P17 R1(e): a proven concrete receiver has no admissible selector lane.
    ConcreteReceiverNoSelector,
    /// S1: a span-verified wrapped React export reached by a non-JSX site.
    WrappedExportNonJsx,
    /// S1b: a lowercase, dashed or namespaced plain JSX tag names an intrinsic element.
    JsxIntrinsic,
    /// S1b: a JS/TS unqualified call whose lexical binding holds no single proven callable.
    LocalBindingUnproven,
    /// E2: the argument-follow target set of a parameter call exceeds the cap of eight.
    ArgFanout,
}

#[cfg(test)]
#[path = "kinds_tests.rs"]
mod tests;
