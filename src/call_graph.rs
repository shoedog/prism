//! Call graph construction from parsed files.
//!
//! Builds both forward (caller→callee) and reverse (callee→caller) call graphs
//! across all parsed files. Used by barrier slice, spiral slice, vertical slice,
//! circular slice, and 3D slice.

use crate::ast::ParsedFile;
use crate::name_resolution::graph::ScopeGraph;
use crate::name_resolution::rust_populator::{enclosing_scope, populate_rust, RustCrateConfig};
use crate::name_resolution::types::{ScopeId, ScopeKind};
use crate::resolution_identity::{resolve_type_path_to_type_scope, TypeKey};
use rayon::prelude::*;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::PathBuf;

/// A node in the call graph: a function identified by file path and name.
#[derive(
    Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, serde::Serialize, serde::Deserialize,
)]
pub struct FunctionId {
    pub file: String,
    pub name: String,
    pub start_line: usize,
    pub end_line: usize,
}

#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    serde::Serialize,
    serde::Deserialize,
)]
pub enum CallKind {
    #[default]
    Call,
    MacroInvocation,
}

#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    serde::Serialize,
    serde::Deserialize,
)]
pub enum CallSiteOrigin {
    #[default]
    Source,
    IndirectResolution,
    /// Minted from a Rust macro argument (transparency-allowlisted, e.g.
    /// `assert!(check(x))`) rather than from the grammar's own call/method
    /// node. See `crate::rust_macro_args`.
    MacroArg,
}

/// A call site: where a function is called from.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CallSite {
    pub caller: FunctionId,
    pub callee_name: String,
    pub line: usize,
    // Populated in PR-3 (macro-invocation routing); always Call in PR-2 (macro
    // invocations are not yet call sites).
    #[serde(default)]
    pub kind: CallKind,
    #[serde(default)]
    pub start_byte: usize,
    #[serde(default)]
    pub end_byte: usize,
    /// Module/object qualifier for the call (e.g., `utils` in `utils.process()`).
    /// `None` for unqualified calls like `process()`.
    pub qualifier: Option<String>,
    /// S3 P6-lite: receiver type recovered syntactically at extraction time
    /// (typed param / constructor local, peeled). None = unrecovered.
    #[serde(default)]
    pub receiver_type: Option<String>,
    /// Package-scoped Go owner proven during receiver recovery. This is kept
    /// separate from `receiver_type` so a bare type returned by a qualified
    /// factory cannot be rebound in the caller's namespace. Older cache rows
    /// deserialize to `None` and therefore cannot invent this provenance.
    #[serde(default)]
    pub receiver_owner_identity: Option<crate::resolution::GoOwnerIdentity>,
    /// A same-function local Go type declaration or later value rebinding
    /// invalidates on-demand R1/R2 proof at this call. The recovered first type
    /// remains available only to the unchanged legacy R3 ladder.
    #[serde(default)]
    pub receiver_local_type_shadowed: bool,
    /// S3 P6-lite: which syntactic fact recovered `receiver_type`
    /// (telemetry + ResolutionKind split). Excluded from cmp_key —
    /// derived from the same scan as receiver_type.
    #[serde(default)]
    pub receiver_recovery: Option<crate::resolution::ReceiverRecovery>,
    /// True when the qualifier is proven to be a local receiver binding, even if
    /// `receiver_type` was not populated because the static type was poisoned or
    /// unresolved. Excluded from cmp_key like receiver recovery metadata.
    #[serde(default)]
    pub receiver_materialized: bool,
    /// Number of arguments at the call site. `None` = not captured / unknown
    /// (the arity-disambiguation filter treats `None` as "keep").
    /// Excluded from cmp_key — positional data, not part of logical identity.
    #[serde(default)]
    pub arg_count: Option<usize>,
    /// `true` when the last argument is a Go spread (`xs...`).
    /// Excluded from cmp_key — same rationale as `arg_count`.
    #[serde(default)]
    pub arg_spread: bool,
    /// Phase-2a PR-2 receiver identity materialization for the later post-pass.
    /// Excluded from cmp_key so logical CallSite identity/order stays inert.
    #[serde(default)]
    pub receiver_outcome: Option<crate::resolution_identity::ReceiverOutcome>,
    /// Provenance for derived call sites. Excluded from cmp_key so a derived
    /// edge cannot coexist with an identical source call-site identity.
    #[serde(default)]
    pub origin: CallSiteOrigin,
    /// Exact target field retained for serialized-call-site compatibility.
    /// Level-3 minting is disabled, so new construction leaves this empty.
    #[serde(default)]
    pub pre_resolved_target: Option<FunctionId>,
}

/// Parameter arity for a method definition (language-agnostic shape).
///
/// `params` is the count of parameter NAMES (not declarations) excluding the Go
/// receiver (or `this`/`self` for other languages).  A variadic declaration
/// contributes exactly 1 to `params` and sets `variadic = true`.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MethodArity {
    /// Number of declared parameter names, excluding the receiver.
    pub params: usize,
    /// True if the last parameter is a variadic (`...T` in Go, `...` in C++/Java).
    pub variadic: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MethodKind {
    Inherent,
    Trait(String),
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RecvMode {
    None,
    SelfBy,
    SelfRef,
    SelfRefMut,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct MethodFacts {
    pub kind: MethodKind,
    pub has_self: bool,
    pub recv_mode: RecvMode,
    pub arity_excl_self: usize,
    pub cfg: Option<String>,
}

// -----------------------------------------------------------------------
// Import-binding types (R4c: Python/JS/TS import-member resolution)
// -----------------------------------------------------------------------

/// A single import binding extracted from a Python/JS/TS file.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct ImportBinding {
    /// The name as used in the caller file (after `as` aliasing).
    pub local: String,
    /// Raw module string from the import statement.
    pub module_path: String,
    /// The original member name (before alias); `None` for module imports.
    pub member: Option<String>,
    /// What kind of import this is.
    pub kind: ImportBindingKind,
    /// False if poisoned by wildcard or re-bound by another top-level binding.
    pub eligible: bool,
}

/// The kind of import binding.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ImportBindingKind {
    /// `from mod import func` / `from mod import func as f`
    MemberImport,
    /// `import mod` / `import mod as m`
    ModuleImport,
    /// `from mod import *`
    WildcardImport,
}

/// The kind of a module-scope binding (for occurrence-clean eligibility checking).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ModuleBindingKind {
    Import,
    ClassDef,
    FunctionDef,
    Assignment,
    Other,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ScopeGraphBuildInputs {
    pub repo_root: PathBuf,
    pub all_file_paths: BTreeSet<String>,
    pub manifest_hashes: BTreeMap<String, String>,
    #[serde(default)]
    pub manifest_snapshot: crate::ManifestSnapshot,
    #[serde(default)]
    pub skipped_go_testdata_files: usize,
    pub cfg: RustCrateConfig,
    pub complete: bool,
}

impl ScopeGraphBuildInputs {
    pub fn from_files_convention(files: &BTreeMap<String, ParsedFile>) -> Self {
        ScopeGraphBuildInputs {
            repo_root: PathBuf::new(),
            all_file_paths: files.keys().cloned().collect(),
            manifest_hashes: BTreeMap::new(),
            manifest_snapshot: crate::ManifestSnapshot::default(),
            skipped_go_testdata_files: 0,
            cfg: RustCrateConfig::from_convention(files),
            complete: true,
        }
    }
}

/// A resolved base-class slot for single-inheritance lookup.
///
/// Each slot in a class's base list becomes one `ClassBaseLink`:
/// - `SameFile` when the base is a simple identifier that resolves uniquely to a
///   top-level class in the same file (no imports, aliases, or star-imports shadow it).
/// - `Barrier` for any base we cannot confidently resolve (non-simple expression,
///   imported name, ambiguous name, wildcard import present, etc.).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ClassBaseLink {
    /// The base class is in the same file and uniquely identified.
    SameFile {
        /// Byte span (start, end) of the base class definition node.
        span: (usize, usize),
        /// Owner key (class name) of the base class.
        owner: String,
    },
    /// Cannot resolve confidently: imported, non-simple expression, ambiguous, etc.
    Barrier,
}

/// P5 S2 (Go func-value callbacks): the source location of a recognized
/// registration reference (composite-literal keyed field value, field
/// assignment RHS, or bare call argument). Byte range included so distinct
/// registrations at the same line still get distinct, deterministic identity
/// (re-review MINOR-2).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct RegistrationSite {
    pub file: String,
    pub line: usize,
    pub start_byte: usize,
    pub end_byte: usize,
}

/// P5 S2: one recognized Go function-value registration.
///
/// Deliberately NOT a `CallSite` — see the architecture note on
/// `CallGraph::go_registrations`: minting a synthetic CallSite here would
/// resolve Exact via `free_single`, a soundness hole. Surfaced as NameOnly
/// `callback_registration` nav edges at query time
/// (`NavigationIndex::build_resolved_call_edges`), and consulted by S3
/// (`resolve_call_site_full`'s Go interface-consult miss path) via
/// `field_key` only.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct RegistrationRecord {
    /// The function whose body contains the registration reference.
    pub enclosing: FunctionId,
    /// The registered (bare-identifier) function value.
    pub target: FunctionId,
    pub site: RegistrationSite,
    /// Package-scoped `(owner_identity, field_name)` key, when knowable —
    /// forms (a)/(b) with a uniquely-recovered owner. `None` for form (c)
    /// (bare call-argument registration, which carries no field) and for
    /// form (a)'s unknown-struct/ambiguous-owner fallback. A `None`-keyed
    /// registration surfaces in nav but NEVER feeds S3 (spec: "ambiguous-owner
    /// records likewise never feed S3").
    pub field_key: Option<(crate::resolution::GoOwnerIdentity, String)>,
}

/// P7 S2: the source location of a recognized Python `@property`/
/// `@cached_property` LOAD access (mirrors `RegistrationSite`, kept as a
/// separate type so the two features' wire shapes can evolve independently).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct PropertyAccessSite {
    pub file: String,
    pub line: usize,
    pub start_byte: usize,
    pub end_byte: usize,
}

/// P7 S2: one recognized Python `@property`/`@cached_property` LOAD access.
///
/// Deliberately NOT a `CallSite` — mirrors the architecture note on
/// `CallGraph::go_registrations`: minting a synthetic CallSite here would
/// resolve through the ordinary call ladder and could mint a wrong-kind/
/// Exact edge (the exact hole P5's spec review caught). Surfaced as NameOnly
/// `property_access` nav edges at query time
/// (`NavigationIndex::build_resolved_call_edges`). Nav-only per the
/// consumer-visibility doctrine — never consulted by CPG Call/Return edges,
/// Step-5b DataFlow, or any non-nav consumer (there is no S3 resolve-time
/// consult path at all, unlike Go func-field snapshots/`FuncValueField`).
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize)]
pub struct PropertyAccessRecord {
    /// The function whose body contains the access.
    pub enclosing: FunctionId,
    /// The `@property`/`@cached_property`-decorated getter.
    pub getter: FunctionId,
    pub site: PropertyAccessSite,
}

/// The call graph for a set of parsed files.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CallGraph {
    /// All known functions.
    pub functions: BTreeMap<String, Vec<FunctionId>>,
    /// Forward edges: function → set of functions it calls.
    pub calls: BTreeMap<FunctionId, BTreeSet<CallSite>>,
    /// Reverse edges: function name → set of call sites that invoke it.
    pub callers: BTreeMap<String, Vec<CallSite>>,
    /// Functions whose positional slots cannot be represented exactly, grouped
    /// by source language. Positional consumers fail closed for these functions.
    #[serde(default)]
    pub param_slots_unknown: BTreeMap<crate::languages::Language, usize>,
    /// Reserved Level-3 telemetry. This remains zero while callback minting is
    /// disabled fail-closed.
    #[serde(default)]
    pub level3_indirect_resolved: usize,
    /// Functions with file-local (static) linkage: `(file, name)` pairs.
    /// Used to disambiguate same-named functions across files.
    pub static_functions: BTreeSet<(String, String)>,
    /// Per-file import maps: file_path → (local_alias → module_path).
    /// Used for import-aware call resolution (E6).
    pub imports: BTreeMap<String, BTreeMap<String, String>>,
    /// S3: (owner_key, method_name) → definitions. Trait impls dual-keyed
    /// under both the impl type and the trait name.
    #[serde(default)]
    pub methods: BTreeMap<(String, String), Vec<FunctionId>>,
    /// S3: owning type per method FunctionId (primary owner, not the trait).
    #[serde(default)]
    pub method_owners: BTreeMap<FunctionId, String>,
    /// Method FunctionId -> owner class definition node byte span (start, end).
    /// Py/JS/TS only; the self-receiver class identity.
    #[serde(default)]
    pub method_class_span: BTreeMap<FunctionId, (usize, usize)>,
    /// Method FunctionIds whose line-based identity maps to multiple class spans.
    /// These fail open to owner lookup because the class span is not trustworthy.
    #[serde(default)]
    pub method_class_span_ambiguous: BTreeSet<FunctionId>,
    /// Slice 1b: per-class base-slot links for inherited-self resolution.
    /// Keyed by `(file_path, class_byte_span)`, value is one `ClassBaseLink` per
    /// base slot (count preserved). Only populated for module-scope classes in
    /// Python/JS/TS.
    #[serde(default)]
    pub class_bases: BTreeMap<(String, (usize, usize)), Vec<ClassBaseLink>>,
    /// Python recovered-receiver class identity: `(file_path, owner_name)` to
    /// module-scope class byte span, only when the owner is occurrence-clean.
    #[serde(default)]
    pub clean_class_spans: BTreeMap<(String, String), (usize, usize)>,
    /// Phase-2a PR-1: (defining-type scope, method_name) -> definitions.
    /// Inert until the receiver-typed read path lands.
    #[serde(default)]
    pub methods_by_scope: BTreeMap<(ScopeId, String), Vec<FunctionId>>,
    /// Phase-2a PR-3 fix: (canonical external type, method_name) -> extension
    /// impl definitions. Kept separate from the bare methods index so external
    /// receivers cannot collide with same-bare in-repo types.
    #[serde(default)]
    pub extension_methods: BTreeMap<(String, String), Vec<FunctionId>>,
    /// Phase-2a PR-1: bare (owner_key, method_name) buckets fully mirrored in
    /// the identity-keyed method index.
    #[serde(default)]
    pub identity_complete: BTreeSet<(String, String)>,
    /// Phase-2a PR-1: (owner type scope, field name) -> cfg-conditioned types.
    /// Inert until the receiver-typed read path lands.
    #[serde(default)]
    pub field_types: BTreeMap<(ScopeId, String), Vec<(Option<String>, TypeKey)>>,
    /// Phase-2a PR-1: function/method -> cfg-conditioned return types.
    /// Inert until the receiver-typed read path lands.
    #[serde(default)]
    pub return_types: BTreeMap<FunctionId, Vec<(Option<String>, TypeKey)>>,
    /// S3 (Go): receiver variable name per method FunctionId.
    #[serde(default)]
    pub receiver_vars: BTreeMap<FunctionId, String>,
    /// Phase-IP (Go embedding): promoted alias `(owner_key, method)` → embedded
    /// methods' FunctionIds. Key set is the EmbeddedPromotion label set; carries
    /// fids for clean incremental replace.
    #[serde(default)]
    pub promoted_aliases: BTreeMap<(String, String), Vec<FunctionId>>,
    /// Phase-IP (Go embedding): gap telemetry, e.g. {"ambiguous": n}.
    #[serde(default)]
    pub embedding_gaps: BTreeMap<String, usize>,
    #[serde(default)]
    pub interface_impls: BTreeMap<(String, String), Vec<FunctionId>>,
    #[serde(default)]
    pub interface_gaps: BTreeMap<String, usize>,
    #[serde(default)]
    pub interface_overapprox: BTreeMap<String, usize>,
    /// Phase-IP PR-2 (manifest §8a): method names declared on some known Go
    /// interface, captured at build (the GoTypeProvider is not retained). The
    /// denominator predicate for the interface-dispatch in-scope manifest.
    #[serde(default)]
    pub interface_method_names: BTreeSet<String>,
    /// Phase-IP PR-2 (review MINOR 6): true once `apply_go_interface_dispatch` has run
    /// (even on a non-Go repo → empty result). Left `false` on a raw `build_direct_subset`
    /// graph, so the manifest can signal "dispatch not computed" vs "computed, none found".
    #[serde(default)]
    pub interface_dispatch_computed: bool,
    /// Arity (param count + variadic flag) per method FunctionId, populated from Go type
    /// provider. Receiver is excluded from `params`. Cleared / rebuilt in lock-step with
    /// `interface_impls` via `clear_interface_dispatch` / `apply_go_interface_dispatch`.
    #[serde(default)]
    pub method_arity: BTreeMap<FunctionId, MethodArity>,
    #[serde(default)]
    pub method_facts: BTreeMap<FunctionId, MethodFacts>,
    #[serde(default)]
    pub scope_graph: Option<ScopeGraph>,
    /// R4c: per-file import bindings for Python/JS/TS import-member resolution.
    #[serde(default)]
    pub import_bindings: BTreeMap<String, Vec<ImportBinding>>,
    /// R4c: per-file module-scope binding kinds for occurrence-clean eligibility.
    #[serde(default)]
    pub module_bindings: BTreeMap<String, BTreeMap<String, ModuleBindingKind>>,
    /// R4c: authority flag — which files prism has actually indexed.
    #[serde(default)]
    pub indexed_files: BTreeSet<String>,
    /// P4: JS/TS raw (per-file, un-resolved) export facts — default exports,
    /// named export lists (incl. renames), exported const-arrow/
    /// function-expression declarations, CommonJS assignments, and re-export
    /// records. Incrementally maintained per file like `import_bindings`.
    #[serde(default)]
    pub js_ts_exports: BTreeMap<String, crate::js_exports::JsExportFacts>,
    /// P4: whole-program resolved JS/TS export facts — file -> exported name
    /// -> the concrete `(file, local_name)` it refers to after following
    /// re-export chains (depth-bounded, cycle-safe). Derived from
    /// `js_ts_exports`; recomputed by `apply_js_export_resolution` the same
    /// way `interface_impls`/`go_registrations` are (whole-program derived —
    /// a barrel's resolution can depend on an unchanged file elsewhere).
    #[serde(default)]
    pub js_ts_resolved_exports:
        BTreeMap<String, BTreeMap<String, crate::js_exports::ResolvedJsExport>>,
    /// P4 telemetry: re-export chains that exceeded the depth bound or hit a
    /// cycle while resolving `js_ts_resolved_exports`. Whole-program derived,
    /// recomputed alongside `js_ts_resolved_exports`.
    #[serde(default)]
    pub js_export_chain_unresolved: usize,
    /// P4 telemetry: barrel names contributed by 2+ conflicting re-export
    /// chains (fail-closed — no binding emitted for that name). Whole-program
    /// derived, recomputed alongside `js_ts_resolved_exports`.
    #[serde(default)]
    pub js_export_barrel_conflicts: usize,
    /// JS/TS R4c: caller function -> conservative parameter/local binding names.
    #[serde(default)]
    pub js_ts_function_locals: BTreeMap<FunctionId, BTreeSet<String>>,
    /// Go package directory index. Plain keys are legacy directory basenames;
    /// `@go-import:` keys carry effective module import paths proven by the
    /// module graph. Qualified `pkg.T` lookup prefers the exact key and uses a
    /// basename only when module identity is unavailable. Ambiguous keys fail
    /// closed. Whole-program derived alongside interface dispatch and retained
    /// for P5/receiver rematerialization. Lifecycle invariant: exact
    /// `@go-import:` keys are populated only by the full dispatch pass after its
    /// clear, and every dispatch clear invalidates this entire map. P5 may
    /// preserve only a snapshot whose package directories still match `files`.
    #[serde(default)]
    pub go_package_basenames: BTreeMap<String, BTreeSet<String>>,
    /// Compatibility projection retained for downstream callers. Production
    /// P5 consults use `go_field_types` declaration snapshots.
    #[serde(default)]
    pub go_known_struct_identities: BTreeSet<crate::resolution::GoOwnerIdentity>,
    /// Compatibility projection retained for downstream callers. Production
    /// P5 consults use all-field declaration snapshots and presence/absence.
    #[serde(default)]
    pub go_func_typed_fields: BTreeSet<(crate::resolution::GoOwnerIdentity, String)>,
    /// P5 S2: recognized Go function-value registration sites. Whole-program
    /// derived (needs S1's package-scoped field-typing, and target resolution
    /// must see the complete function index) — like
    /// `interface_impls`/`promoted_aliases`, recomputed from scratch by
    /// `apply_go_registrations` rather than incrementally patched.
    #[serde(default)]
    pub go_registrations: BTreeSet<RegistrationRecord>,
    /// P5 S2 telemetry: registration candidates skipped because the target
    /// identifier was shadowed by a local binding in the enclosing function.
    #[serde(default)]
    pub go_registration_shadowed_skips: usize,
    /// P5 S2 telemetry: form-(b) (field assignment) candidates skipped
    /// because the LHS operand's owner type could not be uniquely recovered.
    #[serde(default)]
    pub go_registration_ambiguous_owner_skips: usize,
    /// P5 S2 telemetry: form-(a) (composite-literal) fallback registrations
    /// recorded WITHOUT a field key because S1 could not type the literal's
    /// struct (unknown/ambiguous owner) — recorded (nav-only), not skipped,
    /// but counted separately per spec.
    #[serde(default)]
    pub go_registration_unknown_owner_recorded: usize,
    /// P13: per-Go-file package/build profile facts used only to partition
    /// same-directory same-name candidates at resolution consult sites.
    #[serde(default)]
    pub go_file_profiles: BTreeMap<String, crate::go_build_profile::GoBuildProfile>,
    /// P13 telemetry: files whose build expression was unparsed. Stored
    /// per-file so incremental remove/merge preserves exact counts.
    #[serde(default)]
    pub go_build_profile_unparsed: BTreeMap<String, usize>,
    /// P13/P10 legacy support-set diagnostic: count `(directory, bare name)`
    /// owners whose declarations span more than one clause/build profile. This
    /// is not a count of affected consult sites or edges.
    #[serde(default)]
    pub go_owner_identity_profile_conflict: usize,
    /// Go source files excluded because a path segment is exactly `testdata`.
    /// Loader-derived and propagated through scope inputs so call-stats can
    /// account for files that never enter the parsed-file map.
    #[serde(default)]
    pub skipped_go_testdata_files: usize,
    /// Effective Go workspace/module/replacement graph summary for call-stats.
    #[serde(default)]
    pub(crate) go_module_graph: crate::go_module_graph::GoModuleGraphTelemetry,
    /// Loaded Go files whose effective import path was proven.
    #[serde(default)]
    pub(crate) go_import_path_proven_files: usize,
    /// Loaded Go files whose effective import path was not proven.
    #[serde(default)]
    pub(crate) go_import_path_unproven_files: usize,
    /// Fail-closed reason histogram for unproven loaded Go files.
    #[serde(default)]
    pub(crate) go_import_path_unproven_reasons: BTreeMap<String, usize>,
    /// P10 build-time S2 consult decisions. Whole-program rematerialized with
    /// receiver keys; runtime S4/P5 decisions travel on ResolutionOutcome.
    #[serde(default)]
    pub go_owner_identity_partition: crate::go_owner_partition::GoOwnerPartitionTelemetry,
    /// P10 build-time S2 decisions keyed by their source call site. `call-stats`
    /// coalesces this with the same site's runtime S4/P5/direct-method decision
    /// so `affected_sites` is a cardinality, not a count of pipeline stages.
    #[serde(default)]
    pub(crate) go_owner_identity_partition_sites: BTreeMap<
        crate::go_owner_partition::GoOwnerPartitionSiteKey,
        crate::go_owner_partition::GoOwnerPartitionTelemetry,
    >,
    /// P13 telemetry: `resolve_go_bare_value_ref` saw multiple same-package
    /// value candidates before profile filtering.
    #[serde(default)]
    pub go_bare_value_ref_ambiguous: usize,
    /// P7 S1: python `@property`/`@cached_property` getter definitions.
    /// Key mirrors `methods`: (owner_key, method_name) -> defining
    /// FunctionIds. Only exact-match decorated getters are indexed;
    /// `@x.setter`/`@x.deleter`-decorated methods (and everything else) are
    /// excluded by construction (see `ParsedFile::python_property_kind`).
    /// Whole-program derived like `go_registrations`; recomputed from
    /// scratch by `apply_python_property_accesses`.
    #[serde(default)]
    pub property_getters: BTreeMap<(String, String), Vec<FunctionId>>,
    /// P7 S1: subset of `property_getters` values decorated
    /// `@cached_property`/`@functools.cached_property` (vs. plain
    /// `@property`) — S3 telemetry counts property-access records
    /// attributed to a cached_property getter separately.
    #[serde(default)]
    pub cached_property_getters: BTreeSet<FunctionId>,
    /// P7 S2: recognized Python property/cached_property LOAD access sites.
    /// Whole-program derived (unknown-receiver fanout needs the complete S1
    /// index across every file); recomputed from scratch, never
    /// incrementally patched — mirrors `go_registrations`.
    #[serde(default)]
    pub property_accesses: BTreeSet<PropertyAccessRecord>,
    /// P7 S2 telemetry: unknown-receiver (incl. `cls`) accesses skipped
    /// because more than 3 distinct classes define the accessed property
    /// name (P3 fanout doctrine — cap is on distinct getter TARGETS).
    #[serde(default)]
    pub property_access_fanout_skips: usize,
    /// P7 S2 telemetry (F5): store/delete-context attribute accesses whose
    /// name is S1-indexed, skipped because a store/delete is never a getter
    /// load — assignment/augmented_assignment LHS, `del` targets, `for`/
    /// comprehension targets, and `with ... as` alias targets (F4).
    #[serde(default)]
    pub property_access_store_skips: usize,
    /// P8: Rust macro-argument call-extraction telemetry, keyed by file path.
    /// Unlike the whole-program-derived P5/P7 facts above, this is purely
    /// per-file/per-function derived (a pure function of that file's own
    /// AST, no cross-file index needed) — `remove_files` retains by file and
    /// `merge` extends the map directly (no incremental drift risk), and
    /// callers sum `.values()` on demand (see `js_ts_exports`/
    /// `js_export_skipped_exprs` for the same on-demand-sum pattern). Only
    /// files with at least one non-default fact are present.
    #[serde(default)]
    pub macro_arg_facts: BTreeMap<String, crate::rust_macro_args::MacroArgFacts>,
    /// P8 F1 fix (BLOCKER, codex re-review): the repo-wide macro shadow set
    /// (`rust_macro_args::collect_macro_shadow_set`), narrowed to its
    /// intersection with `TRANSPARENT_ARG_MACROS`
    /// (`rust_macro_args::transparent_shadow_intersection`) — the only part
    /// of the shadow set that can change macro-arg call-extraction behavior.
    /// Unlike `macro_arg_facts` above, this is whole-program derived (like
    /// `interface_impls`/`js_ts_resolved_exports`): recomputed from scratch
    /// on every full build and on every `build_direct_subset` call (which
    /// always scans the COMPLETE `files` map for this, not just its
    /// `only_files` subset — see its call site). `merge` overwrites (not
    /// extends) this field from the incoming graph for exactly that reason:
    /// the incoming `build_direct_subset` graph already carries the fresh,
    /// correct whole-program value. `build_incremental_with_scope_graph_inputs`
    /// compares the value persisted here (from the PREVIOUS build) against a
    /// fresh computation before doing any incremental work, and falls back to
    /// a full rebuild on mismatch — without this guard, an unchanged file's
    /// retained call sites/macro-arg facts (see `remove_files`'s P8 comment)
    /// would go stale whenever a `macro_rules!` definition anywhere in the
    /// repo flips an allowlisted macro name's shadowed status.
    #[serde(default)]
    pub macro_shadow_intersection: BTreeSet<String>,
    /// P9 S1/S2: recognized Flask/FastAPI/Express route-registration edges
    /// (framework-entry). Whole-program derived like `go_registrations`/
    /// `property_accesses` — Express identifier-arg resolution needs the
    /// complete `functions`/`js_ts_function_locals` index — so this clears
    /// and recomputes from scratch via `apply_framework_entries` rather than
    /// being incrementally patched.
    #[serde(default)]
    pub framework_entries: BTreeSet<crate::framework_entries::FrameworkEntryRecord>,
    /// P9 S1 telemetry: Express handler-argument positions identified but
    /// left unresolved — an inline arrow/function-expression argument
    /// (grounding-verified to never receive an inferred FunctionId), or a
    /// bare identifier with zero, multiple, or a locally-shadowed same-file
    /// function match.
    #[serde(default)]
    pub framework_entry_unresolved_handlers: usize,
    /// P11/P10 S1: clause-bearing function identity -> declared return type for Go
    /// free functions/methods whose `result` is a single type or `(T,
    /// error)`. Whole-program derived (a consuming file's call-RHS receiver
    /// recovery can depend on a function declared in a DIFFERENT file of the
    /// same package) — recomputed from scratch by
    /// `apply_go_receiver_indices` in the post-merge rematerialization pass,
    /// never incrementally patched.
    #[serde(default)]
    pub go_return_types: crate::go_receiver_index::GoReturnTypes,
    /// P11 S2/P10: clause-keyed per-declaration struct snapshots. Field
    /// presence/absence and raw type remain attached to the defining file so
    /// receiver recovery can filter build profiles before requiring one value.
    #[serde(default)]
    pub go_field_types: crate::go_owner_partition::GoStructDeclarations,
    /// P17: serialized declaration-kind proof used by the shared recovered-Go
    /// receiver route. Exact CPG hits must retain the same R1/R2/R3 verdict.
    #[serde(default)]
    pub go_declaration_kind_index: crate::go_concrete_receiver::GoDeclarationKindIndex,
    /// P17 compatibility snapshot showing a concrete selector is supplied by
    /// embedding. The shared route separately checks the existing owner-index
    /// promotion lane before deferring a newly recovered miss.
    #[serde(default)]
    pub go_promoted_concrete_selectors: BTreeSet<(crate::resolution::GoOwnerIdentity, String)>,
    /// P10 S4 interface and method declaration provenance, captured from the
    /// provider alongside `go_field_types` for exact consult-time routing.
    #[serde(default)]
    pub go_interface_declarations: crate::go_owner_partition::GoInterfaceDeclarations,
    #[serde(default)]
    pub go_method_declarations: crate::go_owner_partition::GoMethodDeclarations,
    /// P10 S4: RTA admission keys used by snapshot-derived interface dispatch.
    #[serde(default)]
    pub go_interface_live_types: BTreeSet<String>,
    /// Compatibility projection retained for downstream callers. Production
    /// S4 routing consults the declaration snapshots above.
    #[serde(default)]
    pub go_embedded_interface_methods:
        BTreeMap<crate::resolution::GoOwnerIdentity, BTreeMap<String, String>>,
    /// P11 S3: `(package_dir, var_name) -> declared type` for package-scope
    /// (top-level) Go `var` declarations with an explicit type. Whole-program
    /// derived (a package var can be declared in a different file of the
    /// same package); recomputed from scratch by `apply_go_receiver_indices`.
    #[serde(default)]
    pub go_package_vars:
        BTreeMap<(String, String), BTreeSet<crate::go_receiver_index::GoTypedFact>>,
}

impl CallGraph {
    /// Create an empty call graph with no functions or edges.
    pub fn empty() -> Self {
        CallGraph {
            functions: BTreeMap::new(),
            calls: BTreeMap::new(),
            callers: BTreeMap::new(),
            param_slots_unknown: BTreeMap::new(),
            level3_indirect_resolved: 0,
            static_functions: BTreeSet::new(),
            imports: BTreeMap::new(),
            methods: BTreeMap::new(),
            method_owners: BTreeMap::new(),
            method_class_span: BTreeMap::new(),
            method_class_span_ambiguous: BTreeSet::new(),
            class_bases: BTreeMap::new(),
            clean_class_spans: BTreeMap::new(),
            methods_by_scope: BTreeMap::new(),
            extension_methods: BTreeMap::new(),
            identity_complete: BTreeSet::new(),
            field_types: BTreeMap::new(),
            return_types: BTreeMap::new(),
            receiver_vars: BTreeMap::new(),
            promoted_aliases: BTreeMap::new(),
            embedding_gaps: BTreeMap::new(),
            interface_impls: BTreeMap::new(),
            interface_gaps: BTreeMap::new(),
            interface_overapprox: BTreeMap::new(),
            interface_method_names: BTreeSet::new(),
            interface_dispatch_computed: false,
            method_arity: BTreeMap::new(),
            method_facts: BTreeMap::new(),
            scope_graph: None,
            import_bindings: BTreeMap::new(),
            module_bindings: BTreeMap::new(),
            indexed_files: BTreeSet::new(),
            js_ts_exports: BTreeMap::new(),
            js_ts_resolved_exports: BTreeMap::new(),
            js_export_chain_unresolved: 0,
            js_export_barrel_conflicts: 0,
            js_ts_function_locals: BTreeMap::new(),
            go_package_basenames: BTreeMap::new(),
            go_known_struct_identities: BTreeSet::new(),
            go_func_typed_fields: BTreeSet::new(),
            go_registrations: BTreeSet::new(),
            go_registration_shadowed_skips: 0,
            go_registration_ambiguous_owner_skips: 0,
            go_registration_unknown_owner_recorded: 0,
            go_file_profiles: BTreeMap::new(),
            go_build_profile_unparsed: BTreeMap::new(),
            go_owner_identity_profile_conflict: 0,
            skipped_go_testdata_files: 0,
            go_module_graph: Default::default(),
            go_import_path_proven_files: 0,
            go_import_path_unproven_files: 0,
            go_import_path_unproven_reasons: BTreeMap::new(),
            go_owner_identity_partition: Default::default(),
            go_owner_identity_partition_sites: BTreeMap::new(),
            go_bare_value_ref_ambiguous: 0,
            property_getters: BTreeMap::new(),
            cached_property_getters: BTreeSet::new(),
            property_accesses: BTreeSet::new(),
            property_access_fanout_skips: 0,
            property_access_store_skips: 0,
            macro_arg_facts: BTreeMap::new(),
            macro_shadow_intersection: BTreeSet::new(),
            framework_entries: BTreeSet::new(),
            framework_entry_unresolved_handlers: 0,
            go_return_types: BTreeMap::new(),
            go_field_types: BTreeMap::new(),
            go_declaration_kind_index: BTreeMap::new(),
            go_promoted_concrete_selectors: BTreeSet::new(),
            go_interface_declarations: BTreeMap::new(),
            go_method_declarations: BTreeMap::new(),
            go_interface_live_types: BTreeSet::new(),
            go_embedded_interface_methods: BTreeMap::new(),
            go_package_vars: BTreeMap::new(),
        }
    }

    /// Build a lightweight call graph with only direct calls (Phases 1-2).
    ///
    /// Skips Phase 3 (indirect call resolution: function pointers, dispatch
    /// tables, parameter-passed callbacks). Used by `CpgContext::build_scoped()`
    /// to quickly identify which files are in the dependency neighborhood of a
    /// diff, before committing to a full CPG build on the scoped subset.
    pub fn build_skeleton(files: &BTreeMap<String, ParsedFile>) -> Self {
        let mut functions: BTreeMap<String, Vec<FunctionId>> = BTreeMap::new();
        let mut calls: BTreeMap<FunctionId, BTreeSet<CallSite>> = BTreeMap::new();
        let mut callers: BTreeMap<String, Vec<CallSite>> = BTreeMap::new();
        let mut static_functions: BTreeSet<(String, String)> = BTreeSet::new();
        let mut methods: BTreeMap<(String, String), Vec<FunctionId>> = BTreeMap::new();
        let mut method_owners: BTreeMap<FunctionId, String> = BTreeMap::new();
        let mut method_class_span: BTreeMap<FunctionId, (usize, usize)> = BTreeMap::new();
        let mut method_class_span_ambiguous: BTreeSet<FunctionId> = BTreeSet::new();
        let mut receiver_vars: BTreeMap<FunctionId, String> = BTreeMap::new();
        let mut method_facts: BTreeMap<FunctionId, MethodFacts> = BTreeMap::new();
        // Repo-wide macro-name shadow set (P8 F1 BLOCKER) — computed once
        // from the full `files` map, mirroring the existing per-build
        // whole-program-fact pattern (e.g. `extract_js_ts_resolution_facts`).
        let macro_shadow = crate::rust_macro_args::collect_macro_shadow_set(files);
        let (go_file_profiles, go_build_profile_unparsed) =
            Self::extract_go_build_profiles(files.iter().map(|(p, f)| (p.as_str(), f)));

        // Phase 1: Collect all function definitions
        for (file_path, parsed) in files {
            for func_node in parsed.all_functions() {
                if let Some(name_node) = parsed.language.function_name(&func_node) {
                    let name = parsed.node_text(&name_node).to_string();
                    let (start, end) = parsed.node_line_range(&func_node);
                    let func_id = FunctionId {
                        file: file_path.clone(),
                        name: name.clone(),
                        start_line: start,
                        end_line: end,
                    };
                    functions
                        .entry(name.clone())
                        .or_default()
                        .push(func_id.clone());
                    let (owner, trait_key, recv_var, class_span) =
                        Self::method_metadata(parsed, &func_node);
                    if let Some(o) = owner {
                        methods
                            .entry((o.clone(), name.clone()))
                            .or_default()
                            .push(func_id.clone());
                        method_owners.insert(func_id.clone(), o);
                        if let Some(s) = class_span {
                            record_method_class_span(
                                &mut method_class_span,
                                &mut method_class_span_ambiguous,
                                &func_id,
                                s,
                            );
                        }
                    }
                    if let Some(t) = trait_key {
                        methods
                            .entry((t, name.clone()))
                            .or_default()
                            .push(func_id.clone());
                    }
                    if let Some(rv) = recv_var {
                        receiver_vars.insert(func_id.clone(), rv);
                    }
                    if let Some(facts) = Self::method_facts(parsed, &func_node) {
                        method_facts.insert(func_id.clone(), facts);
                    }

                    if matches!(
                        parsed.language,
                        crate::languages::Language::C | crate::languages::Language::Cpp
                    ) {
                        if has_static_specifier(parsed, &func_node) {
                            static_functions.insert((file_path.clone(), name));
                        }
                    }
                }
            }
        }

        // Phase 2: Find all call sites within each function
        for (file_path, parsed) in files {
            for func_node in parsed.all_functions() {
                let func_name = match parsed.language.function_name(&func_node) {
                    Some(n) => parsed.node_text(&n).to_string(),
                    None => continue,
                };
                let (start, end) = parsed.node_line_range(&func_node);
                let caller_id = FunctionId {
                    file: file_path.clone(),
                    name: func_name,
                    start_line: start,
                    end_line: end,
                };

                let all_lines: BTreeSet<usize> = (start..=end).collect();
                let call_sites = parsed.function_calls_with_spans_on_lines(
                    &func_node,
                    &all_lines,
                    &macro_shadow,
                );

                for meta in call_sites {
                    let callee_name = meta.callee_name;
                    let start_byte = meta.start_byte;
                    let end_byte = meta.end_byte;
                    let line = meta.line;
                    let site = CallSite {
                        caller: caller_id.clone(),
                        callee_name: callee_name.clone(),
                        line,
                        kind: meta
                            .kind_override
                            .unwrap_or_else(|| Self::call_kind_at(parsed, start_byte, end_byte)),
                        start_byte,
                        end_byte,
                        // build_skeleton is a lightweight scope-computation pass;
                        // preserve its pre-existing behavior of ignoring the
                        // extraction-time qualifier (None input) and falling back
                        // to the self/this/cls line-text heuristic only.
                        qualifier: Self::recover_self_receiver_qualifier(
                            parsed,
                            &callee_name,
                            line,
                            None,
                        ),
                        receiver_type: None,
                        receiver_owner_identity: None,
                        receiver_local_type_shadowed: false,
                        receiver_recovery: None,
                        receiver_materialized: false,
                        arg_count: None,
                        arg_spread: false,
                        receiver_outcome: None,
                        origin: meta.origin_override.unwrap_or(CallSiteOrigin::Source),
                        pre_resolved_target: None,
                    };
                    calls
                        .entry(caller_id.clone())
                        .or_default()
                        .insert(site.clone());
                    callers.entry(callee_name).or_default().push(site);
                }
            }
        }

        // Skip Phase 3 (indirect call resolution) — skeleton only needs direct calls.

        CallGraph {
            functions,
            calls,
            callers,
            param_slots_unknown: Self::parameter_slots_unknown(files),
            level3_indirect_resolved: 0,
            static_functions,
            imports: BTreeMap::new(),
            methods,
            method_owners,
            method_class_span,
            method_class_span_ambiguous,
            class_bases: BTreeMap::new(),
            clean_class_spans: BTreeMap::new(),
            methods_by_scope: BTreeMap::new(),
            extension_methods: BTreeMap::new(),
            identity_complete: BTreeSet::new(),
            field_types: BTreeMap::new(),
            return_types: BTreeMap::new(),
            receiver_vars,
            promoted_aliases: BTreeMap::new(),
            embedding_gaps: BTreeMap::new(),
            interface_impls: BTreeMap::new(),
            interface_gaps: BTreeMap::new(),
            interface_overapprox: BTreeMap::new(),
            interface_method_names: BTreeSet::new(),
            interface_dispatch_computed: false,
            method_arity: BTreeMap::new(),
            method_facts,
            scope_graph: None,
            import_bindings: BTreeMap::new(),
            module_bindings: BTreeMap::new(),
            indexed_files: BTreeSet::new(),
            js_ts_exports: BTreeMap::new(),
            js_ts_resolved_exports: BTreeMap::new(),
            js_export_chain_unresolved: 0,
            js_export_barrel_conflicts: 0,
            js_ts_function_locals: BTreeMap::new(),
            go_package_basenames: BTreeMap::new(),
            go_known_struct_identities: BTreeSet::new(),
            go_func_typed_fields: BTreeSet::new(),
            go_registrations: BTreeSet::new(),
            go_registration_shadowed_skips: 0,
            go_registration_ambiguous_owner_skips: 0,
            go_registration_unknown_owner_recorded: 0,
            go_file_profiles,
            go_build_profile_unparsed,
            go_owner_identity_profile_conflict: 0,
            skipped_go_testdata_files: 0,
            go_module_graph: Default::default(),
            go_import_path_proven_files: 0,
            go_import_path_unproven_files: 0,
            go_import_path_unproven_reasons: BTreeMap::new(),
            go_owner_identity_partition: Default::default(),
            go_owner_identity_partition_sites: BTreeMap::new(),
            go_bare_value_ref_ambiguous: 0,
            property_getters: BTreeMap::new(),
            cached_property_getters: BTreeSet::new(),
            property_accesses: BTreeSet::new(),
            property_access_fanout_skips: 0,
            property_access_store_skips: 0,
            macro_arg_facts: BTreeMap::new(),
            macro_shadow_intersection: crate::rust_macro_args::transparent_shadow_intersection(
                &macro_shadow,
            ),
            // P9: whole-program framework-entry state — left empty here, same
            // reason as the Go/Python whole-program facts above:
            // `build_skeleton` never computes whole-program derived state.
            framework_entries: BTreeSet::new(),
            framework_entry_unresolved_handlers: 0,
            go_return_types: BTreeMap::new(),
            go_field_types: BTreeMap::new(),
            go_declaration_kind_index: BTreeMap::new(),
            go_promoted_concrete_selectors: BTreeSet::new(),
            go_interface_declarations: BTreeMap::new(),
            go_method_declarations: BTreeMap::new(),
            go_interface_live_types: BTreeSet::new(),
            go_embedded_interface_methods: BTreeMap::new(),
            go_package_vars: BTreeMap::new(),
        }
    }

    /// Build a call graph from all parsed files (default receiver-recovery config).
    pub fn build(files: &BTreeMap<String, ParsedFile>) -> Self {
        let inputs = ScopeGraphBuildInputs::from_files_convention(files);
        Self::build_with_receiver_config_and_scope_graph_inputs(
            files,
            &crate::resolution::ReceiverRecoveryConfig::default(),
            Some(&inputs),
        )
    }

    /// Build a call graph with an explicit receiver-recovery config (spec §2 seam).
    /// The classifier is built once and shared across the rayon extraction loop.
    pub fn build_with_receiver_config(
        files: &BTreeMap<String, ParsedFile>,
        receiver_config: &crate::resolution::ReceiverRecoveryConfig,
    ) -> Self {
        let inputs = ScopeGraphBuildInputs::from_files_convention(files);
        Self::build_with_receiver_config_and_scope_graph_inputs(
            files,
            receiver_config,
            Some(&inputs),
        )
    }

    pub fn build_with_scope_graph_inputs(
        files: &BTreeMap<String, ParsedFile>,
        inputs: Option<&ScopeGraphBuildInputs>,
    ) -> Self {
        Self::build_with_receiver_config_and_scope_graph_inputs(
            files,
            &crate::resolution::ReceiverRecoveryConfig::default(),
            inputs,
        )
    }

    pub fn build_with_receiver_config_and_scope_graph_inputs(
        files: &BTreeMap<String, ParsedFile>,
        receiver_config: &crate::resolution::ReceiverRecoveryConfig,
        scope_inputs: Option<&ScopeGraphBuildInputs>,
    ) -> Self {
        let classifier = receiver_config.classifier();
        let classifier: &dyn crate::resolution::ReceiverClassifier = classifier.as_ref();
        let mut functions: BTreeMap<String, Vec<FunctionId>> = BTreeMap::new();
        let mut calls: BTreeMap<FunctionId, BTreeSet<CallSite>> = BTreeMap::new();
        let mut callers: BTreeMap<String, Vec<CallSite>> = BTreeMap::new();
        let mut static_functions: BTreeSet<(String, String)> = BTreeSet::new();
        let mut methods: BTreeMap<(String, String), Vec<FunctionId>> = BTreeMap::new();
        let mut method_owners: BTreeMap<FunctionId, String> = BTreeMap::new();
        let mut method_class_span: BTreeMap<FunctionId, (usize, usize)> = BTreeMap::new();
        let mut method_class_span_ambiguous: BTreeSet<FunctionId> = BTreeSet::new();
        let mut receiver_vars: BTreeMap<FunctionId, String> = BTreeMap::new();
        let mut method_facts: BTreeMap<FunctionId, MethodFacts> = BTreeMap::new();
        // Repo-wide macro-name shadow set (P8 F1 BLOCKER) — computed once
        // from the full `files` map, mirroring the existing per-build
        // whole-program-fact pattern (e.g. `extract_js_ts_resolution_facts`
        // below). `BTreeSet<String>` is `Sync`, so it can be captured by
        // reference into the `par_iter` closure below.
        let macro_shadow = crate::rust_macro_args::collect_macro_shadow_set(files);
        let (go_file_profiles, go_build_profile_unparsed) =
            Self::extract_go_build_profiles(files.iter().map(|(p, f)| (p.as_str(), f)));

        // Collect per-file import maps for import-aware call resolution.
        let mut imports: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
        for (file_path, parsed) in files {
            let file_imports = parsed.extract_imports();
            if !file_imports.is_empty() {
                imports.insert(file_path.clone(), file_imports);
            }
        }

        let ordered_files: Vec<(&String, &ParsedFile)> = files.iter().collect();

        struct FileFunctions {
            functions: Vec<(
                String,
                FunctionId,
                Option<String>,
                Option<String>,
                Option<String>,
                Option<MethodFacts>,
                Option<(usize, usize)>,
            )>,
            static_functions: Vec<(String, String)>,
        }

        // Phase 1: Collect all function definitions per file in parallel, then
        // flatten serially in file order to preserve insertion order.
        let per_file_functions: Vec<FileFunctions> = ordered_files
            .par_iter()
            .map(|entry| {
                let (file_path, parsed) = *entry;
                let mut file_functions = Vec::new();
                let mut file_static_functions = Vec::new();

                for func_node in parsed.all_functions() {
                    if let Some(name_node) = parsed.language.function_name(&func_node) {
                        let name = parsed.node_text(&name_node).to_string();
                        let (start, end) = parsed.node_line_range(&func_node);
                        let func_id = FunctionId {
                            file: file_path.clone(),
                            name: name.clone(),
                            start_line: start,
                            end_line: end,
                        };
                        let (owner, trait_key, recv_var, class_span) =
                            Self::method_metadata(parsed, &func_node);
                        let facts = Self::method_facts(parsed, &func_node);
                        file_functions.push((
                            name.clone(),
                            func_id,
                            owner,
                            trait_key,
                            recv_var,
                            facts,
                            class_span,
                        ));

                        // Detect C/C++ static linkage
                        if matches!(
                            parsed.language,
                            crate::languages::Language::C | crate::languages::Language::Cpp
                        ) {
                            if has_static_specifier(parsed, &func_node) {
                                file_static_functions.push((file_path.clone(), name));
                            }
                        }
                    }
                }

                FileFunctions {
                    functions: file_functions,
                    static_functions: file_static_functions,
                }
            })
            .collect();

        for file_functions in per_file_functions {
            for (name, func_id, owner, trait_key, recv_var, facts, class_span) in
                file_functions.functions
            {
                functions
                    .entry(name.clone())
                    .or_default()
                    .push(func_id.clone());
                if let Some(o) = owner {
                    methods
                        .entry((o.clone(), name.clone()))
                        .or_default()
                        .push(func_id.clone());
                    method_owners.insert(func_id.clone(), o);
                    if let Some(s) = class_span {
                        record_method_class_span(
                            &mut method_class_span,
                            &mut method_class_span_ambiguous,
                            &func_id,
                            s,
                        );
                    }
                }
                if let Some(t) = trait_key {
                    methods
                        .entry((t, name.clone()))
                        .or_default()
                        .push(func_id.clone());
                }
                if let Some(rv) = recv_var {
                    receiver_vars.insert(func_id.clone(), rv);
                }
                if let Some(facts) = facts {
                    method_facts.insert(func_id.clone(), facts);
                }
            }
            for static_function in file_functions.static_functions {
                static_functions.insert(static_function);
            }
        }

        struct FileCallSites {
            file_path: String,
            call_sites: Vec<(FunctionId, CallSite)>,
            macro_arg_facts: crate::rust_macro_args::MacroArgFacts,
        }

        // Phase 2: Find all call sites within each function in parallel, then
        // flatten serially in file order to preserve insertion order.
        let per_file_calls: Vec<FileCallSites> = ordered_files
            .par_iter()
            .map(|entry| {
                let (file_path, parsed) = *entry;
                let mut file_call_sites = Vec::new();
                let mut file_macro_arg_facts = crate::rust_macro_args::MacroArgFacts::default();
                let file_imports_ref = imports.get(file_path);

                for func_node in parsed.all_functions() {
                    let func_name = match parsed.language.function_name(&func_node) {
                        Some(n) => parsed.node_text(&n).to_string(),
                        None => continue,
                    };
                    let (start, end) = parsed.node_line_range(&func_node);
                    let caller_id = FunctionId {
                        file: file_path.clone(),
                        name: func_name,
                        start_line: start,
                        end_line: end,
                    };

                    let all_lines: BTreeSet<usize> = (start..=end).collect();
                    let (call_sites, facts) = parsed
                        .function_calls_with_qualifier_and_spans_on_lines(
                            &func_node,
                            &all_lines,
                            &macro_shadow,
                        );
                    file_macro_arg_facts.calls_recorded += facts.calls_recorded;
                    file_macro_arg_facts.skipped_macros += facts.skipped_macros;
                    file_macro_arg_facts.ctor_skips += facts.ctor_skips;
                    let recv_var = parsed
                        .language
                        .go_receiver_var(&func_node)
                        .map(|n| parsed.node_text(&n).to_string());

                    for meta in call_sites {
                        let callee_name = meta.callee_name;
                        let line = meta.line;
                        let start_byte = meta.start_byte;
                        let end_byte = meta.end_byte;
                        let qualifier = Self::recover_self_receiver_qualifier(
                            parsed,
                            &callee_name,
                            line,
                            meta.qualifier,
                        );
                        let classification = classifier.classify(crate::resolution::ReceiverCtx {
                            receiver_expr: meta.receiver_node,
                            qualifier: qualifier.as_deref(),
                            fn_node: func_node,
                            call_line: line,
                            call_start_byte: start_byte,
                            parsed,
                            recv_var: recv_var.as_deref(),
                            file_imports: file_imports_ref,
                        });
                        let recovered = classification.recovered.as_ref();
                        let site = CallSite {
                            caller: caller_id.clone(),
                            callee_name,
                            line,
                            kind: meta.kind_override.unwrap_or_else(|| {
                                Self::call_kind_at(parsed, start_byte, end_byte)
                            }),
                            start_byte,
                            end_byte,
                            qualifier,
                            receiver_type: recovered.as_ref().map(|r| r.static_type.clone()),
                            receiver_owner_identity: recovered
                                .as_ref()
                                .and_then(|r| r.owner_identity.clone()),
                            receiver_local_type_shadowed: classification.proof_shadowed
                                || recovered.as_ref().is_some_and(|r| {
                                    parsed.go_local_type_shadows(
                                        &func_node,
                                        &r.static_type,
                                        start_byte,
                                    )
                                }),
                            receiver_recovery: recovered.as_ref().map(|r| r.recovery),
                            receiver_materialized: classification.materialized,
                            arg_count: meta.arg_count,
                            arg_spread: meta.arg_spread,
                            receiver_outcome: None,
                            origin: meta.origin_override.unwrap_or(CallSiteOrigin::Source),
                            pre_resolved_target: None,
                        };
                        file_call_sites.push((caller_id.clone(), site));
                    }
                }

                FileCallSites {
                    file_path: file_path.clone(),
                    call_sites: file_call_sites,
                    macro_arg_facts: file_macro_arg_facts,
                }
            })
            .collect();

        let mut macro_arg_facts: BTreeMap<String, crate::rust_macro_args::MacroArgFacts> =
            BTreeMap::new();
        for file_calls in &per_file_calls {
            if file_calls.macro_arg_facts != crate::rust_macro_args::MacroArgFacts::default() {
                macro_arg_facts.insert(file_calls.file_path.clone(), file_calls.macro_arg_facts);
            }
        }

        for file_calls in per_file_calls {
            for (caller_id, site) in file_calls.call_sites {
                calls
                    .entry(caller_id.clone())
                    .or_default()
                    .insert(site.clone());
                callers
                    .entry(site.callee_name.clone())
                    .or_default()
                    .push(site);
            }
        }

        // Phase 5: Build class facts for inherited-self and recovered receivers.
        let (class_bases, clean_class_spans) = Self::build_class_facts(files);

        // R4c: populate import bindings for Python/JS/TS import-member resolution.
        let (import_bindings, module_bindings) = Self::extract_all_import_bindings(files);
        let (js_ts_exports, js_ts_function_locals) = Self::extract_js_ts_resolution_facts(files);
        let indexed_files: BTreeSet<String> = files.keys().cloned().collect();

        let mut cg = CallGraph {
            functions,
            calls,
            callers,
            param_slots_unknown: Self::parameter_slots_unknown(files),
            level3_indirect_resolved: 0,
            static_functions,
            imports,
            methods,
            method_owners,
            method_class_span,
            method_class_span_ambiguous,
            class_bases,
            clean_class_spans,
            methods_by_scope: BTreeMap::new(),
            extension_methods: BTreeMap::new(),
            identity_complete: BTreeSet::new(),
            field_types: BTreeMap::new(),
            return_types: BTreeMap::new(),
            receiver_vars,
            promoted_aliases: BTreeMap::new(),
            embedding_gaps: BTreeMap::new(),
            interface_impls: BTreeMap::new(),
            interface_gaps: BTreeMap::new(),
            interface_overapprox: BTreeMap::new(),
            interface_method_names: BTreeSet::new(),
            interface_dispatch_computed: false,
            method_arity: BTreeMap::new(),
            method_facts,
            scope_graph: Self::populate_scope_graph(files, scope_inputs),
            import_bindings,
            module_bindings,
            indexed_files,
            js_ts_exports,
            js_ts_resolved_exports: BTreeMap::new(),
            js_export_chain_unresolved: 0,
            js_export_barrel_conflicts: 0,
            js_ts_function_locals,
            go_package_basenames: BTreeMap::new(),
            go_known_struct_identities: BTreeSet::new(),
            go_func_typed_fields: BTreeSet::new(),
            go_registrations: BTreeSet::new(),
            go_registration_shadowed_skips: 0,
            go_registration_ambiguous_owner_skips: 0,
            go_registration_unknown_owner_recorded: 0,
            go_file_profiles,
            go_build_profile_unparsed,
            go_owner_identity_profile_conflict: 0,
            skipped_go_testdata_files: 0,
            go_module_graph: Default::default(),
            go_import_path_proven_files: 0,
            go_import_path_unproven_files: 0,
            go_import_path_unproven_reasons: BTreeMap::new(),
            go_owner_identity_partition: Default::default(),
            go_owner_identity_partition_sites: BTreeMap::new(),
            go_bare_value_ref_ambiguous: 0,
            property_getters: BTreeMap::new(),
            cached_property_getters: BTreeSet::new(),
            property_accesses: BTreeSet::new(),
            property_access_fanout_skips: 0,
            property_access_store_skips: 0,
            macro_arg_facts,
            macro_shadow_intersection: crate::rust_macro_args::transparent_shadow_intersection(
                &macro_shadow,
            ),
            // P9: whole-program framework-entry state — left empty here,
            // same reason as the Go/Python whole-program facts below:
            // `apply_framework_entries` populates it after `functions` /
            // `js_ts_function_locals` are already complete.
            framework_entries: BTreeSet::new(),
            framework_entry_unresolved_handlers: 0,
            go_return_types: BTreeMap::new(),
            go_field_types: BTreeMap::new(),
            go_declaration_kind_index: BTreeMap::new(),
            go_promoted_concrete_selectors: BTreeSet::new(),
            go_interface_declarations: BTreeMap::new(),
            go_method_declarations: BTreeMap::new(),
            go_interface_live_types: BTreeSet::new(),
            go_embedded_interface_methods: BTreeMap::new(),
            go_package_vars: BTreeMap::new(),
        };
        cg.refresh_rust_receiver_state(files);
        cg.apply_go_embedding_promotion(files);
        cg.apply_go_interface_dispatch_with_scope_inputs(files, scope_inputs);
        // P5: S1 func-typed-field index, then S2 registration scan (needs S1
        // already applied — registrations are keyed against it).
        cg.apply_go_func_value_fields(files);
        cg.apply_go_registrations(files);
        // P11: Go receiver-typing indices (S1 return-types, S3 package vars)
        // + the post-merge rematerialization pass (S1 call-RHS, S2
        // nested-selector, S3 package var). Needs `go_field_types`/
        // declaration snapshots, already captured by
        // `apply_go_interface_dispatch` above. Uses the SAME `receiver_config`
        // this build used at extraction time (spec-parity fix: an earlier
        // draft hardcoded `ReceiverRecoveryConfig::default()` here, silently
        // re-enabling var_local/type_assertion recovery even when this build
        // explicitly disabled them via `build_with_receiver_config`).
        cg.apply_go_receiver_indices(files, receiver_config);
        // P7: python property-access state — needs the complete method_owners
        // / method_class_span / class_bases indexes already populated above,
        // so it runs last, same rationale as the Go passes.
        cg.apply_python_property_accesses(files);
        // P9: framework-entry state (Flask/FastAPI/Express route
        // registrations) needs the complete `functions`/
        // `js_ts_function_locals` index already populated above (Express
        // identifier-arg resolution reads both) — runs after the Go/Python
        // whole-program passes, same rationale.
        cg.apply_framework_entries(files);
        // P4: JS/TS export-fact resolution (re-export chains/barrels) is ALSO
        // whole-program derived, same rationale as the Go passes above.
        cg.apply_js_export_resolution();
        // Recompute the remaining whole-program indirect passes after all
        // resolution facts are installed. Level-3 callback minting is disabled.
        cg.recompute_indirect_calls(files);
        cg
    }

    // -----------------------------------------------------------------------
    // R4c: import-binding extraction for Python/JS/TS
    // -----------------------------------------------------------------------

    /// Extract import bindings and module bindings for all eligible files.
    fn extract_all_import_bindings(
        files: &BTreeMap<String, ParsedFile>,
    ) -> (
        BTreeMap<String, Vec<ImportBinding>>,
        BTreeMap<String, BTreeMap<String, ModuleBindingKind>>,
    ) {
        let mut import_bindings: BTreeMap<String, Vec<ImportBinding>> = BTreeMap::new();
        let mut module_bindings: BTreeMap<String, BTreeMap<String, ModuleBindingKind>> =
            BTreeMap::new();

        for (file_path, parsed) in files {
            if matches!(
                parsed.language,
                crate::languages::Language::Python
                    | crate::languages::Language::JavaScript
                    | crate::languages::Language::TypeScript
                    | crate::languages::Language::Tsx
            ) {
                let bindings = parsed.extract_import_bindings();
                if !bindings.is_empty() {
                    import_bindings.insert(file_path.clone(), bindings);
                }
                let mbindings = parsed.extract_module_bindings();
                if !mbindings.is_empty() {
                    module_bindings.insert(file_path.clone(), mbindings);
                }
            }
        }
        mark_import_binding_eligibility(&mut import_bindings, &module_bindings);
        (import_bindings, module_bindings)
    }

    fn extract_js_ts_resolution_facts(
        files: &BTreeMap<String, ParsedFile>,
    ) -> (
        BTreeMap<String, crate::js_exports::JsExportFacts>,
        BTreeMap<FunctionId, BTreeSet<String>>,
    ) {
        Self::extract_js_ts_resolution_facts_from_iter(files.iter())
    }

    fn parameter_slots_unknown(
        files: &BTreeMap<String, ParsedFile>,
    ) -> BTreeMap<crate::languages::Language, usize> {
        let mut unknown = BTreeMap::new();
        for parsed in files.values() {
            for function in parsed.all_functions() {
                if parsed.function_parameter_slots(&function).is_none() {
                    *unknown.entry(parsed.language).or_default() += 1;
                }
            }
        }
        unknown
    }

    fn extract_js_ts_resolution_facts_from_iter<'a, I>(
        files: I,
    ) -> (
        BTreeMap<String, crate::js_exports::JsExportFacts>,
        BTreeMap<FunctionId, BTreeSet<String>>,
    )
    where
        I: IntoIterator<Item = (&'a String, &'a ParsedFile)>,
    {
        let mut export_facts: BTreeMap<String, crate::js_exports::JsExportFacts> = BTreeMap::new();
        let mut function_locals: BTreeMap<FunctionId, BTreeSet<String>> = BTreeMap::new();

        for (file_path, parsed) in files {
            if !matches!(
                parsed.language,
                crate::languages::Language::JavaScript
                    | crate::languages::Language::TypeScript
                    | crate::languages::Language::Tsx
            ) {
                continue;
            }

            let exports = parsed.extract_js_ts_export_facts();
            if !exports.is_empty() {
                export_facts.insert(file_path.clone(), exports);
            }

            for func_node in parsed.all_functions() {
                let Some(name_node) = parsed.language.function_name(&func_node) else {
                    continue;
                };
                let name = parsed.node_text(&name_node).to_string();
                let (start, end) = parsed.node_line_range(&func_node);
                let func_id = FunctionId {
                    file: file_path.clone(),
                    name,
                    start_line: start,
                    end_line: end,
                };
                let locals = parsed.js_ts_function_local_bindings(&func_node);
                if !locals.is_empty() {
                    function_locals.insert(func_id, locals);
                }
            }
        }

        (export_facts, function_locals)
    }

    /// P4: whole-program resolution of JS/TS export facts — recomputed from
    /// `js_ts_exports` (raw, per-file) into `js_ts_resolved_exports`
    /// (following re-export chains/barrels, depth-bounded, cycle-safe), the
    /// same "recompute from scratch on the merged graph" pattern as
    /// `apply_go_registrations`/`apply_go_interface_dispatch`: a barrel's
    /// resolution can depend on an unchanged file elsewhere, so it can't be
    /// incrementally patched per changed file.
    pub fn apply_js_export_resolution(&mut self) {
        self.clear_js_export_resolution();
        let indexed_files = &self.indexed_files;
        let resolve_module = |from: &str, module_path: &str| {
            resolve_js_ts_relative_module(module_path, from, indexed_files)
        };
        let resolution =
            crate::js_exports::resolve_js_exports(&self.js_ts_exports, &resolve_module);
        self.js_ts_resolved_exports = resolution.resolved;
        self.js_export_chain_unresolved = resolution.chain_unresolved;
        self.js_export_barrel_conflicts = resolution.barrel_conflicts;
    }

    fn clear_js_export_resolution(&mut self) {
        self.js_ts_resolved_exports.clear();
        self.js_export_chain_unresolved = 0;
        self.js_export_barrel_conflicts = 0;
    }

    // -----------------------------------------------------------------------
    // Incremental cache support (Phase 2)
    // -----------------------------------------------------------------------

    /// Remove all entries originating from the given files.
    ///
    /// Used by incremental cache update: when a file changes, its call graph
    /// contributions are stripped out before fresh data is merged in.
    pub fn remove_files(&mut self, exclude: &BTreeSet<String>) {
        // Both are whole-program pass products and are repopulated from the
        // complete files map by `build_direct_subset` / `recompute_indirect_calls`.
        self.param_slots_unknown.clear();
        self.level3_indirect_resolved = 0;
        // functions: remove FunctionId entries from excluded files.
        for func_ids in self.functions.values_mut() {
            func_ids.retain(|fid| !exclude.contains(&fid.file));
        }
        self.functions.retain(|_, v| !v.is_empty());

        // calls: remove entries where the caller is in an excluded file.
        self.calls
            .retain(|caller, _| !exclude.contains(&caller.file));

        // callers: remove CallSite entries where the caller is in an excluded file.
        for sites in self.callers.values_mut() {
            sites.retain(|s| !exclude.contains(&s.caller.file));
        }
        self.callers.retain(|_, v| !v.is_empty());

        // static_functions: remove entries for excluded files.
        self.static_functions.retain(|(f, _)| !exclude.contains(f));

        // imports: remove entries for excluded files.
        self.imports.retain(|f, _| !exclude.contains(f));
        self.go_file_profiles.retain(|f, _| !exclude.contains(f));
        self.go_build_profile_unparsed
            .retain(|f, _| !exclude.contains(f));

        // methods: remove FunctionId entries from excluded files.
        for func_ids in self.methods.values_mut() {
            func_ids.retain(|fid| !exclude.contains(&fid.file));
        }
        self.methods.retain(|_, v| !v.is_empty());

        // method_owners / method_class_span / receiver_vars: keyed by FunctionId.
        self.method_owners
            .retain(|fid, _| !exclude.contains(&fid.file));
        self.method_class_span
            .retain(|fid, _| !exclude.contains(&fid.file));
        self.method_class_span_ambiguous
            .retain(|fid| !exclude.contains(&fid.file));
        self.class_bases.retain(|(f, _), _| !exclude.contains(f));
        self.clean_class_spans
            .retain(|(f, _), _| !exclude.contains(f));
        self.methods_by_scope.clear();
        self.extension_methods.clear();
        self.identity_complete.clear();
        self.field_types.clear();
        self.return_types.clear();
        self.receiver_vars
            .retain(|fid, _| !exclude.contains(&fid.file));
        self.method_facts
            .retain(|fid, _| !exclude.contains(&fid.file));

        // Promoted embedding aliases are whole-program; drop them all so no caller
        // that merges without re-applying promotion leaves a stale alias (by-fid.file
        // pruning can't catch an alias whose target fid lives in an unchanged file).
        // `apply_go_embedding_promotion` repopulates from all files.
        self.clear_promoted_embedding();
        self.clear_interface_dispatch();
        self.scope_graph = None;

        // R4c: remove import/module bindings for excluded files.
        self.import_bindings.retain(|f, _| !exclude.contains(f));
        self.module_bindings.retain(|f, _| !exclude.contains(f));
        self.js_ts_exports.retain(|f, _| !exclude.contains(f));
        self.js_ts_function_locals
            .retain(|fid, _| !exclude.contains(&fid.file));
        // indexed_files tracks the file set; removed files are no longer indexed.
        self.indexed_files.retain(|f| !exclude.contains(f));

        // P8: macro-arg extraction telemetry (`macro_arg_facts`) retains by
        // file here, same as `js_ts_exports`/`import_bindings` above. BUT
        // (F1 fix, codex re-review BLOCKER) whether an unchanged file's
        // RETAINED call sites/facts are still valid depends on the
        // repo-wide `macro_shadow_intersection` (below) staying unchanged
        // across this rebuild — a `macro_rules!` def added/removed
        // anywhere in the repo can flip an allowlisted macro name's
        // shadowed status and make a retained site (or a retained skip)
        // wrong even though the file it lives in never changed. Verifying
        // that is NOT this method's job: the guard lives in
        // `build_incremental_with_scope_graph_inputs`, which compares the
        // intersection persisted on `macro_shadow_intersection` against a
        // fresh whole-files computation BEFORE calling `remove_files`/
        // `merge` at all, and falls back to a full rebuild on any
        // mismatch. So retain-by-file here is exactly correct only because
        // that caller-side guard guarantees the shadow set never actually
        // drifts underneath an incremental rebuild.
        self.macro_arg_facts.retain(|f, _| !exclude.contains(f));

        // `macro_shadow_intersection` itself is deliberately left untouched
        // here (no per-file breakdown exists to retain-by-file) — `merge`
        // below unconditionally overwrites it with the fresh value the
        // incoming `build_direct_subset` graph always carries; see that
        // field's doc comment.

        // P4: JS/TS resolved export facts are whole-program derived, same
        // rationale as the Go func-value state below — a barrel/re-export
        // chain's resolution can depend on an unchanged file elsewhere.
        // `apply_js_export_resolution` repopulates from the merged
        // `js_ts_exports` after remove_files + merge.
        self.clear_js_export_resolution();

        // P5: Go func-value state (S1 field-typing index + S2 registration
        // table) is whole-program derived, same rationale as the promoted
        // embedding aliases above — a registration's field-key validity can
        // depend on a struct declared in an UNCHANGED file, and a
        // registration's target FunctionId resolution can depend on the
        // complete function index. Drop it all; `apply_go_func_value_fields` /
        // `apply_go_registrations` repopulate from the merged graph.
        self.clear_go_func_value_fields();
        self.clear_go_registrations();

        // P11: Go receiver-typing indices (S1 return-types, S3 package vars)
        // are ALSO whole-program derived — same rationale as the Go
        // func-value state directly above: a return/package-var type can be
        // declared in an unchanged file. Drop it all;
        // `apply_go_receiver_indices` repopulates from the merged graph
        // (the Go owner declaration snapshots are cleared by
        // `clear_interface_dispatch` above and repopulated by
        // `apply_go_interface_dispatch`).
        self.clear_go_receiver_indices();

        // P7: Python property-access state is ALSO whole-program derived —
        // same rationale as the Go func-value state directly above: a
        // getter's owner class can live in an unchanged file, and unknown-
        // receiver fanout needs the complete S1 index across every file.
        // Drop it all; `apply_python_property_accesses` repopulates from the
        // merged graph.
        self.clear_python_property_accesses();

        // P9: framework-entry state is ALSO whole-program derived — same
        // rationale: an Express handler identifier's resolution can depend
        // on the complete `functions`/`js_ts_function_locals` index across
        // every file. Drop it all; `apply_framework_entries` repopulates
        // from the merged graph.
        self.clear_framework_entries();
    }

    /// Merge another CallGraph into this one.
    ///
    /// Entries from `other` are added to the existing data. Typically called
    /// after `remove_files` to splice in freshly-built data for changed files.
    pub fn merge(&mut self, other: CallGraph) {
        self.param_slots_unknown = other.param_slots_unknown;
        self.level3_indirect_resolved = other.level3_indirect_resolved;
        for (name, fids) in other.functions {
            self.functions.entry(name).or_default().extend(fids);
        }
        for (caller, sites) in other.calls {
            self.calls.entry(caller).or_default().extend(sites);
        }
        for (name, sites) in other.callers {
            self.callers.entry(name).or_default().extend(sites);
        }
        self.static_functions.extend(other.static_functions);
        self.imports.extend(other.imports);
        self.go_file_profiles.extend(other.go_file_profiles);
        self.go_build_profile_unparsed
            .extend(other.go_build_profile_unparsed);
        self.go_owner_identity_profile_conflict += other.go_owner_identity_profile_conflict;
        self.go_bare_value_ref_ambiguous += other.go_bare_value_ref_ambiguous;
        for (key, fids) in other.methods {
            self.methods.entry(key).or_default().extend(fids);
        }
        self.method_owners.extend(other.method_owners);
        self.method_class_span_ambiguous
            .extend(other.method_class_span_ambiguous);
        for (fid, span) in other.method_class_span {
            record_method_class_span(
                &mut self.method_class_span,
                &mut self.method_class_span_ambiguous,
                &fid,
                span,
            );
        }
        self.class_bases.extend(other.class_bases);
        self.clean_class_spans.extend(other.clean_class_spans);
        for (key, fids) in other.methods_by_scope {
            self.methods_by_scope.entry(key).or_default().extend(fids);
        }
        for (key, fids) in other.extension_methods {
            self.extension_methods.entry(key).or_default().extend(fids);
        }
        self.identity_complete.extend(other.identity_complete);
        for (key, types) in other.field_types {
            self.field_types.entry(key).or_default().extend(types);
        }
        self.return_types.extend(other.return_types);
        self.receiver_vars.extend(other.receiver_vars);
        self.method_facts.extend(other.method_facts);
        self.scope_graph = None;

        // R4c: merge import bindings.
        self.import_bindings.extend(other.import_bindings);
        self.module_bindings.extend(other.module_bindings);
        self.indexed_files.extend(other.indexed_files);
        self.js_ts_exports.extend(other.js_ts_exports);
        self.js_ts_function_locals
            .extend(other.js_ts_function_locals);
        // P8: per-file macro-arg facts extend directly -- `other` only ever
        // carries entries for files it (re)built, so this can never
        // double-count a file that remove_files didn't first drop. (This
        // is safe from stale drift ONLY because
        // `build_incremental_with_scope_graph_inputs` already verified the
        // repo-wide shadow set is unchanged before reaching this call --
        // see `macro_shadow_intersection`'s doc comment and the guard
        // immediately below.)
        self.macro_arg_facts.extend(other.macro_arg_facts);
        // P8 F1 fix: `macro_shadow_intersection` is OVERWRITTEN, not
        // extended/unioned -- `other` (a `build_direct_subset` graph)
        // always computed it from the COMPLETE `files` map (not its
        // `only_files` subset), so `other`'s value is already the fresh,
        // correct whole-program fact and simply replaces whatever `self`
        // was carrying from its own last build.
        self.macro_shadow_intersection = other.macro_shadow_intersection;

        // P4 (JS/TS resolved export facts): deliberately NOT merged here,
        // same rationale as the Go func-value callbacks note below — the
        // caller (`build_incremental_with_scope_graph_inputs`) re-applies
        // `apply_js_export_resolution` on the merged graph, which repopulates
        // `js_ts_resolved_exports` from `js_ts_exports` above.

        // P5 (Go func-value callbacks): deliberately NOT merged here, same as
        // `interface_impls`/`promoted_aliases`/`interface_method_names` above
        // (also absent from this method) — these are whole-program derived,
        // `other` (a `build_direct_subset` graph) always carries them empty,
        // and the sole caller (`build_incremental_with_scope_graph_inputs`)
        // re-applies `apply_go_func_value_fields` / `apply_go_registrations`
        // on the merged graph immediately after, exactly like the embedding/
        // interface passes. Extending here would just be overwritten.
        //
        // P7 (Python property accesses): deliberately NOT merged here either,
        // same reasoning — `apply_python_property_accesses` re-applies on the
        // merged graph right after `apply_go_registrations` in
        // `build_incremental_with_scope_graph_inputs`.
        //
        // P9 (framework-entry edges): deliberately NOT merged here either,
        // same reasoning — Express identifier-arg resolution needs the
        // complete `functions`/`js_ts_function_locals` index, so
        // `apply_framework_entries` re-applies on the merged graph right
        // after `apply_python_property_accesses` in
        // `build_incremental_with_scope_graph_inputs`.
        //
        // P11 (Go receiver-typing indices `go_return_types`/`go_package_vars`,
        // and the Go owner declaration snapshots captured
        // alongside `interface_impls`): deliberately NOT merged here either,
        // same reasoning as P5/P7/P9 above — `apply_go_interface_dispatch` +
        // `apply_go_receiver_indices` re-apply on the merged graph, and the
        // rematerialization pass itself patches every retained Go
        // `CallSite`'s `receiver_type`/`receiver_recovery` in place (not just
        // `other`'s), so extending here would be both redundant and
        // insufficient.
    }

    pub(crate) fn recompute_indirect_calls(&mut self, files: &BTreeMap<String, ParsedFile>) {
        self.clear_indirect_calls();
        let (sites, level3_resolved) = self.compute_indirect_call_sites(files);
        self.apply_indirect_call_sites(sites);
        self.level3_indirect_resolved = level3_resolved;
    }

    fn clear_indirect_calls(&mut self) {
        self.level3_indirect_resolved = 0;
        for sites in self.calls.values_mut() {
            sites.retain(|site| site.origin != CallSiteOrigin::IndirectResolution);
        }
        self.calls.retain(|_, sites| !sites.is_empty());

        for sites in self.callers.values_mut() {
            sites.retain(|site| site.origin != CallSiteOrigin::IndirectResolution);
        }
        self.callers.retain(|_, sites| !sites.is_empty());
    }

    fn compute_indirect_call_sites(
        &self,
        files: &BTreeMap<String, ParsedFile>,
    ) -> (Vec<(FunctionId, CallSite)>, usize) {
        // Resolve indirect call sites (function pointer variables and dispatch
        // tables). Preserve the historical level ordering:
        // 1/2 local function pointer and array dispatch, 4 struct callbacks,
        // then 3 parameter-passed function pointers.
        let known_fn_names: BTreeSet<String> = self.functions.keys().cloned().collect();
        let mut extra_sites: Vec<(FunctionId, CallSite)> = Vec::new();

        for (caller_id, sites) in &self.calls {
            for site in sites {
                if self.functions.contains_key(&site.callee_name) {
                    continue;
                }

                let parsed = match files.get(&caller_id.file) {
                    Some(p) => p,
                    None => continue,
                };

                // Level 2: array dispatch table — callee_name like "handlers[0]".
                if site.callee_name.contains('[') {
                    let array_name = site.callee_name.split('[').next().unwrap_or("");
                    if array_name.is_empty() {
                        continue;
                    }

                    let func_source = Self::extract_func_source(parsed, caller_id);
                    let targets = crate::ast::resolve_array_dispatch(
                        &func_source,
                        array_name,
                        &known_fn_names,
                    );
                    let file_targets = if targets.is_empty() {
                        crate::ast::resolve_array_dispatch(
                            &parsed.source,
                            array_name,
                            &known_fn_names,
                        )
                    } else {
                        Vec::new()
                    };
                    for target in targets.iter().chain(file_targets.iter()) {
                        extra_sites.push((
                            caller_id.clone(),
                            Self::indirect_call_site(caller_id, target.clone(), site),
                        ));
                    }
                    continue;
                }

                // Level 1: local variable function pointer — callee_name is a plain identifier.
                if site
                    .callee_name
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '_')
                {
                    let Some(func_source) =
                        Self::extract_func_source_before(parsed, caller_id, site.start_byte)
                    else {
                        continue;
                    };
                    if let Some(resolved) = crate::ast::resolve_fptr_assignment(
                        func_source,
                        &site.callee_name,
                        &known_fn_names,
                    ) {
                        extra_sites.push((
                            caller_id.clone(),
                            Self::indirect_call_site(caller_id, resolved, site),
                        ));
                    }
                }
            }
        }

        // Level 4: struct field callback resolution (interprocedural).
        type Level4Index = BTreeMap<String, BTreeMap<String, BTreeSet<String>>>;
        let mut level4_index: Level4Index = BTreeMap::new();
        for (path, parsed) in files {
            let mut per_field: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
            for line in parsed.source.lines() {
                let trimmed = line.trim();
                for field in crate::ast::candidate_fields_on_line(trimmed) {
                    let targets = per_field.entry(field.clone()).or_default();
                    crate::ast::line_field_targets(trimmed, &field, &known_fn_names, targets);
                }
            }
            for (field, targets) in per_field {
                if !targets.is_empty() {
                    level4_index
                        .entry(field)
                        .or_default()
                        .insert(path.clone(), targets);
                }
            }
        }

        let mut level4_sites: Vec<(FunctionId, CallSite)> = Vec::new();
        for (caller_id, sites) in &self.calls {
            for site in sites {
                if self.functions.contains_key(&site.callee_name) {
                    continue;
                }
                if site.qualifier.is_none() {
                    continue;
                }
                if !site
                    .callee_name
                    .chars()
                    .all(|c| c.is_alphanumeric() || c == '_')
                {
                    continue;
                }

                let already_resolved = extra_sites.iter().any(|(cid, es)| {
                    cid == caller_id
                        && es.line == site.line
                        && known_fn_names.contains(&es.callee_name)
                });
                if already_resolved {
                    continue;
                }

                if let Some(by_file) = level4_index.get(&site.callee_name) {
                    for targets in by_file.values() {
                        for target in targets {
                            level4_sites.push((
                                caller_id.clone(),
                                Self::indirect_call_site(caller_id, target.clone(), site),
                            ));
                        }
                    }
                }
            }
        }
        extra_sites.extend(level4_sites);

        // Level-3 parameter callback minting is disabled fail-closed.
        (extra_sites, 0)
    }

    fn apply_indirect_call_sites(&mut self, sites: Vec<(FunctionId, CallSite)>) {
        for (caller_id, site) in sites {
            let callee_name = site.callee_name.clone();
            self.calls
                .entry(caller_id)
                .or_default()
                .insert(site.clone());
            self.callers.entry(callee_name).or_default().push(site);
        }
    }

    fn indirect_call_site(
        caller_id: &FunctionId,
        target: String,
        source_site: &CallSite,
    ) -> CallSite {
        CallSite {
            caller: caller_id.clone(),
            callee_name: target,
            line: source_site.line,
            kind: CallKind::Call,
            // Carry the source call site span so same-line indirect dups do not collapse.
            start_byte: source_site.start_byte,
            end_byte: source_site.end_byte,
            qualifier: None,
            receiver_type: None,
            receiver_owner_identity: None,
            receiver_local_type_shadowed: false,
            receiver_recovery: None,
            receiver_materialized: false,
            arg_count: None,
            arg_spread: false,
            receiver_outcome: None,
            origin: CallSiteOrigin::IndirectResolution,
            pre_resolved_target: None,
        }
    }

    /// Build class facts for inherited-self and recovered receiver resolution.
    ///
    /// For each module-scope class definition with base slots, resolves each
    /// base to `SameFile` or `Barrier` based on occurrence-clean checks. Also
    /// records occurrence-clean module-scope class spans by `(file, owner)`.
    fn build_class_facts(
        files: &BTreeMap<String, ParsedFile>,
    ) -> (
        BTreeMap<(String, (usize, usize)), Vec<ClassBaseLink>>,
        BTreeMap<(String, String), (usize, usize)>,
    ) {
        use crate::languages::Language;
        let mut class_bases = BTreeMap::new();
        let mut clean_class_spans = BTreeMap::new();

        for (file_path, parsed) in files {
            if !matches!(
                parsed.language,
                Language::Python | Language::JavaScript | Language::TypeScript | Language::Tsx
            ) {
                continue;
            }

            let root = parsed.tree.root_node();
            let has_wildcard_import = Self::has_wildcard_import(parsed);
            let has_module_scope_match = Self::has_module_scope_match(parsed);

            // Collect all top-level class definitions with their names and spans.
            // "Top-level" = direct child of module root (or inside a decorated_definition
            // that is a direct child of module root). Not nested inside functions or
            // other classes.
            let mut top_level_classes = Vec::new();
            let mut cursor = root.walk();
            for child in root.children(&mut cursor) {
                let class_node = if child.kind() == "decorated_definition" {
                    // Python: decorated class — unwrap
                    let mut inner_cursor = child.walk();
                    let mut found = None;
                    for inner in child.children(&mut inner_cursor) {
                        if inner.kind() == "class_definition" {
                            found = Some(inner);
                            break;
                        }
                    }
                    match found {
                        Some(n) => n,
                        None => continue,
                    }
                } else if matches!(
                    child.kind(),
                    "class_definition" | "class_declaration" | "class"
                ) {
                    child
                } else {
                    continue;
                };

                if let Some(name_node) = class_node.child_by_field_name("name") {
                    let name =
                        parsed.source[name_node.start_byte()..name_node.end_byte()].to_string();
                    let span = (class_node.start_byte(), class_node.end_byte());
                    top_level_classes.push((name, span, class_node));
                }
            }

            // Count top-level binding occurrences of each name for occurrence-clean check.
            let top_level_bindings = Self::top_level_bindings(parsed);

            if !has_wildcard_import && !has_module_scope_match {
                for (class_name, class_span, _) in &top_level_classes {
                    let class_matches = top_level_classes
                        .iter()
                        .filter(|(name, _, _)| name == class_name)
                        .count();
                    let binding_count = top_level_bindings.get(class_name).copied().unwrap_or(0);
                    if class_matches == 1 && binding_count == 1 {
                        clean_class_spans
                            .insert((file_path.clone(), class_name.clone()), *class_span);
                    }
                }
            }

            // Now iterate all module-scope class definitions that have base slots.
            for (_, class_span, class_node) in &top_level_classes {
                let base_slots = parsed.language.class_base_names(class_node, &parsed.source);
                if base_slots.is_empty() {
                    continue;
                }

                let links: Vec<ClassBaseLink> = base_slots
                    .into_iter()
                    .map(|slot| {
                        let Some(base_name) = slot else {
                            return ClassBaseLink::Barrier;
                        };

                        // Wildcard imports and module-scope match captures poison
                        // simple class identity for this precision slice.
                        if has_wildcard_import || has_module_scope_match {
                            return ClassBaseLink::Barrier;
                        }

                        // Check occurrence-clean: the name must appear exactly once as
                        // a top-level class definition and have no other top-level bindings.
                        let class_matches: Vec<&(String, (usize, usize), tree_sitter::Node)> =
                            top_level_classes
                                .iter()
                                .filter(|(n, _, _)| *n == base_name)
                                .collect();

                        if class_matches.len() != 1 {
                            return ClassBaseLink::Barrier;
                        }

                        // Check no other top-level binding (import, assignment, function, etc.)
                        let binding_count =
                            top_level_bindings.get(&base_name).copied().unwrap_or(0);
                        // binding_count includes the class definition itself, so
                        // exactly 1 = only the class.
                        if binding_count != 1 {
                            return ClassBaseLink::Barrier;
                        }

                        ClassBaseLink::SameFile {
                            span: class_matches[0].1,
                            owner: base_name,
                        }
                    })
                    .collect();

                class_bases.insert((file_path.clone(), *class_span), links);
            }
        }

        (class_bases, clean_class_spans)
    }

    /// Check if a Python file has a module-scope `match` statement. Pattern
    /// captures bind at module scope; model that conservatively as a class-fact
    /// barrier for this precision slice.
    fn has_module_scope_match(parsed: &ParsedFile) -> bool {
        if !matches!(parsed.language, crate::languages::Language::Python) {
            return false;
        }

        fn check_block(node: tree_sitter::Node) -> bool {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if check_module_scope_stmt(child) {
                    return true;
                }
            }
            false
        }

        fn check_module_scope_stmt(child: tree_sitter::Node) -> bool {
            if child.kind() == "match_statement" {
                return true;
            }
            // Clause nodes wrap their statements in a `block` child. Recurse
            // transparently so nested module-scope match statements are seen.
            if child.kind() == "block" {
                return check_block(child);
            }
            // Module-scope compound statements execute their bodies at module
            // scope in Python, so a match nested inside can still bind names.
            if matches!(
                child.kind(),
                "if_statement"
                    | "try_statement"
                    | "for_statement"
                    | "while_statement"
                    | "with_statement"
            ) {
                let mut bcursor = child.walk();
                for block_child in child.children(&mut bcursor) {
                    if matches!(
                        block_child.kind(),
                        "block"
                            | "else_clause"
                            | "elif_clause"
                            | "except_clause"
                            | "finally_clause"
                    ) && check_block(block_child)
                    {
                        return true;
                    }
                }
            }
            false
        }

        let root = parsed.tree.root_node();
        let mut cursor = root.walk();
        for child in root.children(&mut cursor) {
            if check_module_scope_stmt(child) {
                return true;
            }
        }
        false
    }

    /// Check if a Python file has `from x import *` (wildcard import).
    fn has_wildcard_import(parsed: &ParsedFile) -> bool {
        if !matches!(parsed.language, crate::languages::Language::Python) {
            return false;
        }

        fn is_wildcard_import(node: tree_sitter::Node) -> bool {
            if node.kind() == "import_from_statement" {
                let mut inner = node.walk();
                for c in node.children(&mut inner) {
                    if c.kind() == "wildcard_import" {
                        return true;
                    }
                }
            }
            false
        }

        fn check_block(node: tree_sitter::Node) -> bool {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if check_module_scope_stmt(child) {
                    return true;
                }
            }
            false
        }

        fn check_module_scope_stmt(child: tree_sitter::Node) -> bool {
            if is_wildcard_import(child) {
                return true;
            }
            // Clause nodes (else_clause, except_clause, …) wrap their
            // statements in a `block` child — recurse transparently so the
            // actual statements are reached.
            if child.kind() == "block" {
                return check_block(child);
            }
            // Module-scope compound statements: their block bodies are still
            // module scope in Python, so wildcard imports inside them count.
            if matches!(
                child.kind(),
                "if_statement"
                    | "try_statement"
                    | "for_statement"
                    | "while_statement"
                    | "with_statement"
            ) {
                let mut bcursor = child.walk();
                for block_child in child.children(&mut bcursor) {
                    if matches!(
                        block_child.kind(),
                        "block"
                            | "else_clause"
                            | "elif_clause"
                            | "except_clause"
                            | "finally_clause"
                    ) {
                        if check_block(block_child) {
                            return true;
                        }
                    }
                }
            }
            false
        }

        // Walk module-level statements looking for `from ... import *`
        let root = parsed.tree.root_node();
        let mut cursor = root.walk();
        for child in root.children(&mut cursor) {
            if check_module_scope_stmt(child) {
                return true;
            }
        }
        false
    }

    /// Count top-level binding occurrences per name in a file.
    ///
    /// A "binding" is any top-level statement that introduces a name:
    /// class definition, function definition, import, assignment, etc.
    /// Also counts names inside top-level if/try/for/with blocks
    /// (these are still module-scope in Python).
    fn top_level_bindings(parsed: &ParsedFile) -> BTreeMap<String, usize> {
        let root = parsed.tree.root_node();
        let mut counts: BTreeMap<String, usize> = BTreeMap::new();

        fn count_bindings_in_block(
            node: tree_sitter::Node,
            source: &str,
            lang: &crate::languages::Language,
            counts: &mut BTreeMap<String, usize>,
        ) {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                count_binding_stmt(child, source, lang, counts);
            }
        }

        fn count_binding_stmt(
            child: tree_sitter::Node,
            source: &str,
            lang: &crate::languages::Language,
            counts: &mut BTreeMap<String, usize>,
        ) {
            match child.kind() {
                "class_definition" | "class_declaration" | "class" => {
                    if let Some(name_node) = child.child_by_field_name("name") {
                        let name = source[name_node.start_byte()..name_node.end_byte()].to_string();
                        *counts.entry(name).or_default() += 1;
                    }
                }
                "decorated_definition" => {
                    // Unwrap: could be a decorated class or function
                    let mut inner = child.walk();
                    for c in child.children(&mut inner) {
                        if matches!(c.kind(), "class_definition" | "function_definition") {
                            if let Some(name_node) = c.child_by_field_name("name") {
                                let name = source[name_node.start_byte()..name_node.end_byte()]
                                    .to_string();
                                *counts.entry(name).or_default() += 1;
                            }
                        }
                    }
                }
                "function_definition" => {
                    if let Some(name_node) = child.child_by_field_name("name") {
                        let name = source[name_node.start_byte()..name_node.end_byte()].to_string();
                        *counts.entry(name).or_default() += 1;
                    }
                }
                "import_statement" | "import_from_statement" => {
                    // Python: `import foo` or `from foo import bar, baz`
                    if matches!(lang, crate::languages::Language::Python) {
                        let mut icursor = child.walk();
                        for c in child.children(&mut icursor) {
                            match c.kind() {
                                "dotted_name" if child.kind() == "import_statement" => {
                                    // `import foo.bar` binds `foo`
                                    if let Some(first) = c.child(0) {
                                        if first.kind() == "identifier" {
                                            let name = source[first.start_byte()..first.end_byte()]
                                                .to_string();
                                            *counts.entry(name).or_default() += 1;
                                        }
                                    }
                                }
                                "aliased_import" => {
                                    // `import foo as bar` or `from x import y as z`
                                    if let Some(alias) = c.child_by_field_name("alias") {
                                        let name = source[alias.start_byte()..alias.end_byte()]
                                            .to_string();
                                        *counts.entry(name).or_default() += 1;
                                    } else if let Some(name_node) = c.child_by_field_name("name") {
                                        let name = source
                                            [name_node.start_byte()..name_node.end_byte()]
                                            .to_string();
                                        *counts.entry(name).or_default() += 1;
                                    }
                                }
                                "dotted_name" if child.kind() == "import_from_statement" => {
                                    // `from x import y` — each imported name
                                    let txt = &source[c.start_byte()..c.end_byte()];
                                    if !txt.contains('.') {
                                        *counts.entry(txt.to_string()).or_default() += 1;
                                    }
                                }
                                _ => {}
                            }
                        }
                    }
                }
                // Assignments: `x = ...`, `x: int = ...`
                "expression_statement" | "assignment" => {
                    if matches!(lang, crate::languages::Language::Python) {
                        extract_assignment_targets(child, source, counts);
                    }
                }
                "type_alias_statement" => {
                    if matches!(lang, crate::languages::Language::Python) {
                        if let Some(name_node) = child
                            .child_by_field_name("name")
                            .or_else(|| child.child_by_field_name("left"))
                            .or_else(|| child.named_child(0))
                        {
                            if matches!(name_node.kind(), "identifier" | "type_identifier") {
                                count_identifier_like(name_node, source, counts);
                            } else if let Some(found) = first_identifier_like(name_node) {
                                count_identifier_like(found, source, counts);
                            }
                        } else if let Some(found) = first_identifier_like(child) {
                            count_identifier_like(found, source, counts);
                        }
                    }
                }
                "delete_statement" => {
                    if matches!(lang, crate::languages::Language::Python) {
                        let mut dcursor = child.walk();
                        for target in child.children(&mut dcursor) {
                            if target.kind() != "del" && target.kind() != "," {
                                collect_identifiers_from_pattern(target, source, counts);
                            }
                        }
                    }
                }
                // JS/TS variable declarations
                "variable_declaration" | "lexical_declaration" => {
                    let mut vcursor = child.walk();
                    for c in child.children(&mut vcursor) {
                        if c.kind() == "variable_declarator" {
                            if let Some(name_node) = c.child_by_field_name("name") {
                                if name_node.kind() == "identifier" {
                                    let name = source[name_node.start_byte()..name_node.end_byte()]
                                        .to_string();
                                    *counts.entry(name).or_default() += 1;
                                }
                            }
                        }
                    }
                }
                // Python top-level compound statements whose bodies are still module-scope.
                // The statement HEADERS also bind names at module scope:
                //   for_statement: iteration target (`for x in ...`)
                //   with_statement: `as` alias (`with ctx() as x`)
                //   try_statement > except_clause: `as` alias (`except E as x`)
                "if_statement" | "try_statement" | "for_statement" | "while_statement"
                | "with_statement" => {
                    if matches!(lang, crate::languages::Language::Python) {
                        // Extract header bindings first.
                        if child.kind() == "for_statement" {
                            // `for x in ...`: the `left` field is the iteration target
                            if let Some(left) = child.child_by_field_name("left") {
                                collect_identifiers_from_pattern(left, source, counts);
                            }
                        } else if child.kind() == "with_statement" {
                            // `with expr as name`: walk for as_pattern aliases
                            fn extract_with_aliases(
                                node: tree_sitter::Node,
                                source: &str,
                                counts: &mut BTreeMap<String, usize>,
                            ) {
                                let mut cur = node.walk();
                                for c in node.children(&mut cur) {
                                    if c.kind() == "as_pattern" {
                                        // The alias is typically the last identifier child
                                        if let Some(alias) = c.child_by_field_name("alias") {
                                            collect_identifiers_from_pattern(alias, source, counts);
                                        }
                                    } else if c.kind() == "with_clause" || c.kind() == "with_item" {
                                        extract_with_aliases(c, source, counts);
                                    }
                                }
                            }
                            extract_with_aliases(child, source, counts);
                        }
                        // Walk into block children (bodies + else/except/finally).
                        // For try_statement, also extract except_clause header aliases.
                        let mut bcursor = child.walk();
                        for block_child in child.children(&mut bcursor) {
                            if block_child.kind() == "except_clause" {
                                // `except E as x`: the `as` identifier binds at module scope
                                let mut ecur = block_child.walk();
                                for ec in block_child.children(&mut ecur) {
                                    if ec.kind() == "as_pattern" {
                                        if let Some(alias) = ec.child_by_field_name("alias") {
                                            collect_identifiers_from_pattern(alias, source, counts);
                                        }
                                    }
                                    // In some tree-sitter-python versions, `except E as x`
                                    // stores the alias as a direct identifier child after `as`.
                                    // Check for an identifier that follows an `as` keyword.
                                    if ec.kind() == "identifier" {
                                        // Check if the previous sibling is `as`
                                        if let Some(prev) = ec.prev_sibling() {
                                            if prev.kind() == "as" {
                                                let name = source[ec.start_byte()..ec.end_byte()]
                                                    .to_string();
                                                *counts.entry(name).or_default() += 1;
                                            }
                                        }
                                    }
                                }
                            }
                            if matches!(
                                block_child.kind(),
                                "block"
                                    | "else_clause"
                                    | "elif_clause"
                                    | "except_clause"
                                    | "finally_clause"
                            ) {
                                count_bindings_in_block(block_child, source, lang, counts);
                            }
                        }
                    }
                }
                // Clause nodes (else_clause, except_clause, …) wrap
                // statements in a `block` child — recurse transparently.
                "block" => {
                    count_bindings_in_block(child, source, lang, counts);
                }
                _ => {}
            }
        }

        let mut cursor = root.walk();
        for child in root.children(&mut cursor) {
            count_binding_stmt(child, &parsed.source, &parsed.language, &mut counts);
        }

        counts
    }

    pub fn rebuild_scope_graph(
        &mut self,
        files: &BTreeMap<String, ParsedFile>,
        inputs: Option<&ScopeGraphBuildInputs>,
    ) {
        self.scope_graph = Self::populate_scope_graph(files, inputs);
        self.refresh_rust_receiver_state(files);
    }

    fn refresh_rust_receiver_state(&mut self, files: &BTreeMap<String, ParsedFile>) {
        self.populate_method_identity_indices(files);
        self.rematerialize_rust_receiver_keys(files);
    }

    fn populate_scope_graph(
        files: &BTreeMap<String, ParsedFile>,
        inputs: Option<&ScopeGraphBuildInputs>,
    ) -> Option<ScopeGraph> {
        let Some(inputs) = inputs else {
            return None;
        };
        if !inputs.complete {
            return None;
        }
        Some(populate_rust(files, &inputs.cfg, None))
    }

    fn rematerialize_rust_receiver_keys(&mut self, files: &BTreeMap<String, ParsedFile>) {
        if self.scope_graph.is_none() {
            return;
        }

        let updates = self.compute_rust_receiver_updates(files);

        for (caller, old_site, outcome) in updates {
            let mut updated = old_site.clone();
            updated.receiver_outcome = outcome;
            if let Some(sites) = self.calls.get_mut(&caller) {
                if sites.take(&old_site).is_some() {
                    sites.insert(updated.clone());
                }
            }
            if let Some(sites) = self.callers.get_mut(&old_site.callee_name) {
                for site in sites {
                    if site.caller == old_site.caller && site.cmp_key() == old_site.cmp_key() {
                        site.receiver_outcome = updated.receiver_outcome.clone();
                    }
                }
            }
        }
    }

    /// Phase 1 of receiver re-typing: read-only per-caller receiver outcomes,
    /// parallel-collected over the `self.calls`-ordered caller list using the
    /// shared `Sync` receiver typer. Only valid when `self.scope_graph` is `Some`
    /// (`RustReceiverTyper::new` requires it) -- the caller guards that.
    pub(crate) fn compute_rust_receiver_updates(
        &self,
        files: &BTreeMap<String, ParsedFile>,
    ) -> Vec<(
        FunctionId,
        CallSite,
        Option<crate::resolution_identity::ReceiverOutcome>,
    )> {
        use rayon::prelude::*;

        let typer = crate::resolution_receiver::RustReceiverTyper::new(self);
        // Repo-wide macro-name shadow set (P8 F1 BLOCKER) — computed once,
        // shared (by reference) across every parallel per-caller task below.
        let macro_shadow = crate::rust_macro_args::collect_macro_shadow_set(files);
        let ordered: Vec<(&FunctionId, &BTreeSet<CallSite>)> = self.calls.iter().collect();
        ordered
            .par_iter()
            .copied() // (&FunctionId, &BTreeSet) is Copy -> avoid &&-destructuring
            .map(|(caller, sites)| {
                Self::receiver_updates_for_caller(caller, sites, &typer, files, &macro_shadow)
            })
            .collect::<Vec<Vec<_>>>()
            .into_iter()
            .flatten()
            .collect()
    }

    /// Per-caller receiver outcomes -- verbatim semantics of the original Phase-1
    /// inner body. Caller-level skips return the empty `out`; site-level skips
    /// `continue` (exactly the original `continue` placements).
    fn receiver_updates_for_caller(
        caller: &FunctionId,
        sites: &BTreeSet<CallSite>,
        typer: &crate::resolution_receiver::RustReceiverTyper<'_>,
        files: &BTreeMap<String, ParsedFile>,
        macro_shadow: &BTreeSet<String>,
    ) -> Vec<(
        FunctionId,
        CallSite,
        Option<crate::resolution_identity::ReceiverOutcome>,
    )> {
        let mut out = Vec::new();
        let Some(parsed) = files.get(&caller.file) else {
            return out;
        };
        if !matches!(parsed.language, crate::languages::Language::Rust) {
            return out;
        }
        let Some(fn_node) = Self::function_node_for_id(parsed, caller) else {
            return out;
        };
        let all_lines: BTreeSet<usize> = (caller.start_line..=caller.end_line).collect();
        let (ast_calls, _facts) = parsed.function_calls_with_qualifier_and_spans_on_lines(
            &fn_node,
            &all_lines,
            macro_shadow,
        );
        for site in sites {
            let Some(meta) = ast_calls.iter().find(|meta| {
                meta.callee_name == site.callee_name
                    && meta.start_byte == site.start_byte
                    && meta.end_byte == site.end_byte
            }) else {
                continue;
            };
            if meta.receiver_node.is_none() && meta.qualifier.is_none() {
                continue;
            }
            let outcome = typer.type_of_receiver(crate::resolution_receiver::ReceiverTypeCtx {
                parsed,
                caller,
                fn_node,
                receiver_expr: meta.receiver_node,
                qualifier: meta.qualifier.as_deref(),
                call_start_byte: meta.start_byte,
            });
            out.push((caller.clone(), site.clone(), outcome));
        }
        out
    }

    /// Frozen verbatim copy of the ORIGINAL Phase-1 loop, returning the same Vec.
    /// The old-order oracle compares the production collect against this.
    #[cfg(test)]
    pub(crate) fn compute_rust_receiver_updates_reference(
        &self,
        files: &BTreeMap<String, ParsedFile>,
    ) -> Vec<(
        FunctionId,
        CallSite,
        Option<crate::resolution_identity::ReceiverOutcome>,
    )> {
        let mut updates = Vec::new();
        let typer = crate::resolution_receiver::RustReceiverTyper::new(self);
        let macro_shadow = crate::rust_macro_args::collect_macro_shadow_set(files);
        for (caller, sites) in &self.calls {
            let Some(parsed) = files.get(&caller.file) else {
                continue;
            };
            if !matches!(parsed.language, crate::languages::Language::Rust) {
                continue;
            }
            let Some(fn_node) = Self::function_node_for_id(parsed, caller) else {
                continue;
            };
            let all_lines: BTreeSet<usize> = (caller.start_line..=caller.end_line).collect();
            let (ast_calls, _facts) = parsed.function_calls_with_qualifier_and_spans_on_lines(
                &fn_node,
                &all_lines,
                &macro_shadow,
            );
            for site in sites {
                let Some(meta) = ast_calls.iter().find(|meta| {
                    meta.callee_name == site.callee_name
                        && meta.start_byte == site.start_byte
                        && meta.end_byte == site.end_byte
                }) else {
                    continue;
                };
                if meta.receiver_node.is_none() && meta.qualifier.is_none() {
                    continue;
                }
                let outcome = typer.type_of_receiver(crate::resolution_receiver::ReceiverTypeCtx {
                    parsed,
                    caller,
                    fn_node,
                    receiver_expr: meta.receiver_node,
                    qualifier: meta.qualifier.as_deref(),
                    call_start_byte: meta.start_byte,
                });
                updates.push((caller.clone(), site.clone(), outcome));
            }
        }
        updates
    }

    fn function_node_for_id<'a>(
        parsed: &'a ParsedFile,
        fid: &FunctionId,
    ) -> Option<tree_sitter::Node<'a>> {
        parsed.all_functions().into_iter().find(|node| {
            let Some(name_node) = parsed.language.function_name(node) else {
                return false;
            };
            if parsed.node_text(&name_node) != fid.name {
                return false;
            }
            let (start, end) = parsed.node_line_range(node);
            start == fid.start_line && end == fid.end_line
        })
    }

    /// Remove all promoted embedding aliases from the owner index (preserving any
    /// direct method on the same key) and clear the alias + gap maps. Idempotent.
    /// Shared by `apply_go_embedding_promotion` (step 1) and `remove_files`, so no
    /// incremental path can leave a stale promoted alias even if it never re-applies.
    fn clear_promoted_embedding(&mut self) {
        let prior = std::mem::take(&mut self.promoted_aliases);
        for (key, fids) in &prior {
            if let Some(v) = self.methods.get_mut(key) {
                v.retain(|f| !fids.contains(f));
                if v.is_empty() {
                    self.methods.remove(key);
                }
            }
        }
        self.embedding_gaps.clear();
    }

    fn clear_interface_dispatch(&mut self) {
        self.interface_impls.clear();
        self.interface_gaps.clear();
        self.interface_overapprox.clear();
        self.interface_method_names.clear();
        self.interface_dispatch_computed = false;
        self.method_arity.clear();
        // P11 S2/S4: captured alongside the fields above in
        // `apply_go_interface_dispatch` — clear here too so the no-Go-files
        // early return in that function (which runs AFTER this clear but
        // BEFORE the fresh capture) leaves them empty rather than stale.
        self.go_field_types.clear();
        self.go_declaration_kind_index.clear();
        self.go_promoted_concrete_selectors.clear();
        self.go_interface_declarations.clear();
        self.go_method_declarations.clear();
        self.go_interface_live_types.clear();
        self.go_embedded_interface_methods.clear();
        self.go_package_basenames.clear();
        self.go_owner_identity_profile_conflict = 0;
        self.go_module_graph = Default::default();
        self.go_import_path_proven_files = 0;
        self.go_import_path_unproven_files = 0;
        self.go_import_path_unproven_reasons.clear();
        debug_assert!(
            self.go_package_basenames.is_empty(),
            "dispatch clear must invalidate every Go package/import-path key"
        );
    }

    /// Recompute Go embedding promotions over `files` and write owner-index aliases.
    /// Idempotent: clears prior aliases first (incremental replace).
    pub fn apply_go_embedding_promotion(&mut self, files: &BTreeMap<String, ParsedFile>) {
        // 1. Remove prior promoted aliases (preserving direct methods on the key).
        self.clear_promoted_embedding();
        if !files
            .values()
            .any(|p| p.language == crate::languages::Language::Go)
        {
            return;
        }
        // 2. Group promotions by (owner_key(struct), method).
        let provider = crate::type_providers::go::GoTypeProvider::from_parsed_files(files);
        let mut by_key: BTreeMap<(String, String), Vec<(usize, FunctionId)>> = BTreeMap::new();
        for pm in provider.promoted_struct_methods() {
            let key = (crate::resolution::owner_key(&pm.struct_name), pm.method);
            by_key.entry(key).or_default().push((pm.depth, pm.func_id));
        }
        // 3. Direct-method-wins, then uniquely-shallowest else ambiguous-drop.
        let mut ambiguous = 0usize;
        for ((owner, method), mut cands) in by_key {
            let has_direct = self
                .methods
                .get(&(owner.clone(), method.clone()))
                .map(|v| v.iter().any(|f| self.method_owners.get(f) == Some(&owner)))
                .unwrap_or(false);
            if has_direct {
                continue;
            }
            cands.sort_by_key(|(d, _)| *d);
            let min_depth = cands[0].0;
            let shallow: Vec<FunctionId> = cands
                .iter()
                .filter(|(d, _)| *d == min_depth)
                .map(|(_, f)| f.clone())
                .collect();
            if shallow.len() > 1 {
                ambiguous += 1;
                continue;
            }
            let fid = shallow.into_iter().next().unwrap();
            self.methods
                .entry((owner.clone(), method.clone()))
                .or_default()
                .push(fid.clone());
            self.promoted_aliases
                .entry((owner, method))
                .or_default()
                .push(fid);
        }
        if ambiguous > 0 {
            self.embedding_gaps
                .insert("ambiguous".to_string(), ambiguous);
        }
    }

    fn count_go_owner_identity_profile_conflicts(
        &self,
        files: &BTreeMap<String, ParsedFile>,
    ) -> usize {
        // Deliberately retain the legacy `(dir, name)` support-set diagnostic:
        // after package clause enters `GoOwnerIdentity`, this counter must still
        // reveal the cross-clause population that motivated the partition cut.
        let mut by_owner: BTreeMap<(String, String), BTreeSet<String>> = BTreeMap::new();
        for (path, parsed) in files {
            if parsed.language != crate::languages::Language::Go {
                continue;
            }
            let Some(profile) = self.go_file_profiles.get(path) else {
                continue;
            };
            let sig = format!(
                "{}|{}|{:?}|{:?}|{:?}",
                profile.package_clause,
                profile.is_test_file,
                profile.goos,
                profile.goarch,
                profile.build_expr
            );
            let root = parsed.tree.root_node();
            let mut cursor = root.walk();
            for child in root.children(&mut cursor) {
                if child.kind() != "type_declaration" {
                    continue;
                }
                let mut tcur = child.walk();
                for spec in child.children(&mut tcur) {
                    if !matches!(spec.kind(), "type_spec" | "type_alias") {
                        continue;
                    }
                    let Some(name_node) = spec.child_by_field_name("name") else {
                        continue;
                    };
                    let name = parsed.node_text(&name_node).trim();
                    if name.is_empty() {
                        continue;
                    }
                    by_owner
                        .entry((
                            crate::resolution::dir_of(path).to_string(),
                            name.to_string(),
                        ))
                        .or_default()
                        .insert(sig.clone());
                }
            }
        }
        by_owner
            .values()
            .filter(|profiles| profiles.len() > 1)
            .count()
    }

    pub fn apply_go_interface_dispatch(&mut self, files: &BTreeMap<String, ParsedFile>) {
        self.apply_go_interface_dispatch_with_scope_inputs(files, None);
    }

    pub(crate) fn apply_go_interface_dispatch_with_scope_inputs(
        &mut self,
        files: &BTreeMap<String, ParsedFile>,
        scope_inputs: Option<&ScopeGraphBuildInputs>,
    ) {
        self.clear_interface_dispatch();
        self.skipped_go_testdata_files = scope_inputs
            .map(|inputs| inputs.skipped_go_testdata_files)
            .unwrap_or(0);
        // The dispatch pass ran (even if there are no Go files → empty result); a raw
        // build_direct_subset graph leaves this false (review MINOR 6 signal).
        self.interface_dispatch_computed = true;
        if !files
            .values()
            .any(|p| p.language == crate::languages::Language::Go)
        {
            return;
        }
        self.go_owner_identity_profile_conflict =
            self.count_go_owner_identity_profile_conflicts(files);
        let live = crate::live_types::go_admission_live_set(files);
        self.go_interface_live_types = live.clone();
        let package_import_paths = Self::go_package_import_paths(files, scope_inputs);
        self.go_module_graph = package_import_paths.graph.clone();
        self.go_import_path_proven_files = package_import_paths.proven_files;
        self.go_import_path_unproven_files = package_import_paths.unproven_files;
        self.go_import_path_unproven_reasons = package_import_paths.reasons.clone();
        let provider =
            crate::type_providers::go::GoTypeProvider::from_parsed_files_with_package_import_paths(
                files,
                &package_import_paths.paths,
            );
        let table = provider.compute_interface_dispatch(&live);
        self.interface_impls = table.impls;
        // Capture per-method arity for later arity-filtered dispatch (Task 2).
        self.method_arity = provider.method_arities();
        // P11 S2/S4: capture the field re-projection and the embedded-interface
        // routing map while the provider is live, same pattern as above.
        self.go_field_types = provider.go_struct_declarations();
        let mut package_basenames = Self::go_package_basenames(files);
        Self::add_go_package_import_paths(&mut package_basenames, &package_import_paths.paths);
        self.go_package_basenames = package_basenames.clone();
        self.go_declaration_kind_index = provider.go_declaration_kind_index(
            &self.imports,
            &package_basenames,
            &self.go_file_profiles,
        );
        self.go_promoted_concrete_selectors = provider.go_promoted_concrete_selectors();
        self.go_interface_declarations = provider.go_interface_declarations();
        self.go_method_declarations = provider.go_method_declarations();
        self.go_embedded_interface_methods = provider.embedded_interface_method_routes();
        // Manifest denominator from raw snapshots, before any clause/profile
        // last-writer collapse in the provider compatibility maps.
        self.interface_method_names = self
            .go_interface_declarations
            .values()
            .flatten()
            .flat_map(|declaration| declaration.methods.iter().cloned())
            .collect();
        for g in &table.gaps {
            *self.interface_gaps.entry(format!("{g:?}")).or_insert(0) += 1;
        }
        for o in &table.overapprox {
            *self
                .interface_overapprox
                .entry(format!("{o:?}"))
                .or_insert(0) += 1;
        }
    }

    fn go_package_import_paths(
        files: &BTreeMap<String, ParsedFile>,
        scope_inputs: Option<&ScopeGraphBuildInputs>,
    ) -> crate::go_module_graph::GoImportPathResolution {
        let Some(scope_inputs) = scope_inputs else {
            let unproven_files = files
                .values()
                .filter(|parsed| parsed.language == crate::languages::Language::Go)
                .count();
            let mut resolution = crate::go_module_graph::GoImportPathResolution {
                unproven_files,
                ..Default::default()
            };
            if resolution.unproven_files > 0 {
                resolution.reasons.insert(
                    crate::go_module_graph::GoImportPathReason::NoGoMod
                        .as_str()
                        .to_string(),
                    resolution.unproven_files,
                );
            }
            return resolution;
        };
        let mut graph = crate::go_module_graph::GoModuleGraph::new(
            &scope_inputs.repo_root,
            &scope_inputs.manifest_snapshot,
        );
        graph.resolve_files(files)
    }

    fn clear_go_func_value_fields(&mut self) {
        self.go_known_struct_identities.clear();
        self.go_func_typed_fields.clear();
    }

    fn go_package_basenames(
        files: &BTreeMap<String, ParsedFile>,
    ) -> BTreeMap<String, BTreeSet<String>> {
        let mut package_basenames = BTreeMap::<String, BTreeSet<String>>::new();
        for (path, parsed) in files {
            if parsed.language != crate::languages::Language::Go {
                continue;
            }
            let dir = crate::resolution::dir_of(path).to_string();
            let basename = dir.rsplit('/').next().unwrap_or(&dir).to_string();
            package_basenames.entry(basename).or_default().insert(dir);
        }
        package_basenames
    }

    fn go_package_basename_snapshot_matches_files(
        index: &BTreeMap<String, BTreeSet<String>>,
        fallback: &BTreeMap<String, BTreeSet<String>>,
    ) -> bool {
        let current_dirs: BTreeSet<&str> = fallback
            .values()
            .flat_map(|dirs| dirs.iter().map(String::as_str))
            .collect();
        fallback
            .iter()
            .all(|(basename, dirs)| index.get(basename) == Some(dirs))
            && index.iter().all(|(key, dirs)| {
                if key.starts_with("@go-import:") {
                    dirs.iter().all(|dir| current_dirs.contains(dir.as_str()))
                } else {
                    fallback.get(key) == Some(dirs)
                }
            })
    }

    fn add_go_package_import_paths(
        index: &mut BTreeMap<String, BTreeSet<String>>,
        package_import_paths: &BTreeMap<String, String>,
    ) {
        for (file, import_path) in package_import_paths {
            if import_path.trim().is_empty() {
                continue;
            }
            index
                .entry(crate::resolution::go_import_path_dir_key(import_path))
                .or_default()
                .insert(crate::resolution::dir_of(file).to_string());
        }
    }

    /// P5 S1: recompute the Go func-typed-field index (package-scoped owner
    /// identity -> which struct fields are func-typed) over `files`.
    /// Whole-program derived, same shape as `apply_go_embedding_promotion` /
    /// `apply_go_interface_dispatch`: clears first (idempotent), then
    /// recomputes from scratch — never incrementally patched.
    pub fn apply_go_func_value_fields(&mut self, files: &BTreeMap<String, ParsedFile>) {
        self.clear_go_func_value_fields();
        let fallback = Self::go_package_basenames(files);
        if !self.go_package_basenames.is_empty()
            && !Self::go_package_basename_snapshot_matches_files(
                &self.go_package_basenames,
                &fallback,
            )
        {
            // A direct reapply after an edit did not run the full dispatch
            // clear/rebuild contract. Drop the entire snapshot rather than
            // retain a stale-but-unique exact import key and prove the wrong
            // concrete owner. Rebuilding the basename-only fallback is safe.
            self.go_package_basenames.clear();
        }
        // Full builds populated exact module import-path keys during interface
        // dispatch, immediately after `clear_interface_dispatch`. This pass
        // never constructs `@go-import:` keys; it preserves a file-matching
        // snapshot for cached-CPG parity or rebuilds the fail-closed basename
        // fallback for direct callers.
        if self.go_package_basenames.is_empty() {
            self.go_package_basenames = fallback.clone();
        }
        debug_assert!(Self::go_package_basename_snapshot_matches_files(
            &self.go_package_basenames,
            &fallback,
        ));
        debug_assert!(
            self.go_package_basenames
                .keys()
                .all(|key| !key.starts_with("@go-import:"))
                || self.interface_dispatch_computed,
            "exact Go import-path keys must originate in the full dispatch pass"
        );
        if fallback.is_empty() {
            return;
        }
        self.go_known_struct_identities = self.go_field_types.keys().cloned().collect();
        let mut func_typed_fields = BTreeSet::new();
        for (owner, declarations) in &self.go_field_types {
            for declaration in declarations {
                for (name, ty) in &declaration.fields {
                    if self.go_field_type_is_func(owner, ty) {
                        func_typed_fields.insert((owner.clone(), name.clone()));
                    }
                }
            }
        }
        self.go_func_typed_fields = func_typed_fields;
    }

    fn clear_go_registrations(&mut self) {
        self.go_registrations.clear();
        self.go_registration_shadowed_skips = 0;
        self.go_registration_ambiguous_owner_skips = 0;
        self.go_registration_unknown_owner_recorded = 0;
        self.go_bare_value_ref_ambiguous = 0;
    }

    /// P5 S2: scan `files` for recognized Go function-value registrations
    /// (composite-literal keyed field, field assignment, bare call argument)
    /// and record them in `go_registrations`. Whole-program derived (needs
    /// declaration snapshots already applied by `apply_go_interface_dispatch`,
    /// and target resolution needs the complete `functions`/`method_owners`
    /// index) — clears and recomputes from
    /// scratch, never incrementally patched.
    ///
    /// Walks `parsed.all_functions()` per file (the same per-function
    /// iteration convention Phase 1/2 and `apply_go_embedding_promotion` use)
    /// so `caller_id` falls out naturally — no line-based `enclosing_function`
    /// lookup (spec: prefer the existing per-function extraction convention).
    pub fn apply_go_registrations(&mut self, files: &BTreeMap<String, ParsedFile>) {
        self.clear_go_registrations();
        if !files
            .values()
            .any(|p| p.language == crate::languages::Language::Go)
        {
            return;
        }
        for (file_path, parsed) in files {
            if parsed.language != crate::languages::Language::Go {
                continue;
            }
            for func_node in parsed.all_functions() {
                let func_name = match parsed.language.function_name(&func_node) {
                    Some(n) => parsed.node_text(&n).to_string(),
                    None => continue,
                };
                let (start, end) = parsed.node_line_range(&func_node);
                let caller_id = FunctionId {
                    file: file_path.clone(),
                    name: func_name,
                    start_line: start,
                    end_line: end,
                };
                let all_lines: BTreeSet<usize> = (start..=end).collect();
                for cand in parsed.go_registration_candidates(&func_node, &all_lines) {
                    self.apply_go_registration_candidate(parsed, &func_node, &caller_id, cand);
                }
            }
        }
    }

    /// Resolve one P5 field key against all visible declaration snapshots.
    /// `Err` means the owner itself is unknown (composite literals retain the
    /// legacy nav-only fallback); `Ok(None)` means a known owner whose visible
    /// field cannot be proven func-typed and must not register.
    fn go_func_field_key(
        &self,
        owner_type_text: &str,
        caller_file: &str,
        field_name: &str,
    ) -> Result<Option<(crate::resolution::GoOwnerIdentity, String)>, ()> {
        let owner = crate::resolution::resolve_go_owner_identity(
            owner_type_text,
            caller_file,
            &self.imports,
            &self.go_package_basenames,
            &self.go_file_profiles,
        )
        .ok_or(())?;
        let Some(declarations) = self.go_field_types.get(&owner) else {
            return Err(());
        };
        let selected = crate::go_owner_partition::select_struct_field(
            &owner,
            caller_file,
            owner_type_text,
            field_name,
            declarations,
            &self.go_file_profiles,
        );
        Ok(selected
            .value
            .as_deref()
            .is_some_and(|ty| self.go_field_type_is_func(&owner, ty))
            .then(|| (owner, field_name.to_string())))
    }

    /// One raw S2 candidate -> zero or one `RegistrationRecord`, applying the
    /// shared gates (target resolution, shadow check) and the per-form owner
    /// recovery / field-typing gate.
    fn apply_go_registration_candidate(
        &mut self,
        parsed: &ParsedFile,
        func_node: &tree_sitter::Node,
        caller_id: &FunctionId,
        cand: crate::ast::GoRegistrationCandidate,
    ) {
        use crate::ast::GoRegistrationForm;
        use crate::resolution::resolve_go_bare_value_ref;

        // Shared gate (ALL forms): the value must resolve to a known in-repo
        // free function via same-package/import helpers (never bare
        // cross-package matching), and must not be shadowed by a local
        // binding in the enclosing function (checked with the SAME Go
        // binding machinery `receiver_type_in_fn` uses for receiver recovery,
        // src/ast.rs — a real local-binding check, not a same-file/package
        // substitute; spec-review MAJOR).
        let Some(target) = resolve_go_bare_value_ref(
            &self.functions,
            &self.method_owners,
            &self.go_file_profiles,
            &mut self.go_bare_value_ref_ambiguous,
            &caller_id.file,
            &cand.value_name,
        ) else {
            return;
        };
        let (_, binding_count) = parsed.receiver_type_in_fn(
            func_node,
            &cand.value_name,
            cand.line,
            cand.start_byte,
            true,
        );
        if binding_count > 0 {
            self.go_registration_shadowed_skips += 1;
            return;
        }

        let site = RegistrationSite {
            file: caller_id.file.clone(),
            line: cand.line,
            start_byte: cand.start_byte,
            end_byte: cand.end_byte,
        };

        let field_key = match &cand.form {
            GoRegistrationForm::CompositeLiteralField {
                struct_type_text,
                field_name,
            } => match self.go_func_field_key(struct_type_text, &caller_id.file, field_name) {
                Ok(Some(key)) => Some(key),
                Ok(None) => return,
                Err(()) => {
                    // Unknown/ambiguous struct: fall back to recording WITHOUT
                    // a field key (nav-only, never feeds S3).
                    self.go_registration_unknown_owner_recorded += 1;
                    None
                }
            },
            GoRegistrationForm::FieldAssignment {
                operand_name,
                field_name,
                assign_line,
                assign_start_byte,
            } => {
                let (type_found, op_bindings) = parsed.receiver_type_in_fn(
                    func_node,
                    operand_name,
                    *assign_line,
                    *assign_start_byte,
                    true,
                );
                let Some((operand_type, _)) = type_found else {
                    self.go_registration_ambiguous_owner_skips += 1;
                    return;
                };
                if op_bindings != 1 {
                    self.go_registration_ambiguous_owner_skips += 1;
                    return;
                }
                match self.go_func_field_key(&operand_type, &caller_id.file, field_name) {
                    Ok(Some(key)) => Some(key),
                    _ => {
                        self.go_registration_ambiguous_owner_skips += 1;
                        return;
                    }
                }
            }
            GoRegistrationForm::CallArgument => None,
        };

        self.go_registrations.insert(RegistrationRecord {
            enclosing: caller_id.clone(),
            target,
            site,
            field_key,
        });
    }

    fn clear_go_receiver_indices(&mut self) {
        self.go_return_types.clear();
        self.go_package_vars.clear();
        self.go_owner_identity_partition = Default::default();
        self.go_owner_identity_partition_sites.clear();
    }

    /// P11 S1/S2/S3: recompute the Go return-type (S1) and package-var (S3)
    /// indices, then rematerialize every Go call site's receiver
    /// classification against the fresh whole-program facts (S1 call-RHS,
    /// S2 nested-selector via the Go owner declaration snapshots
    /// already captured by `apply_go_interface_dispatch`, S3 package var).
    ///
    /// Whole-program derived — MUST run after `apply_go_interface_dispatch`
    /// (needs `go_field_types`) in both the full-build and incremental-rebuild
    /// sequences (mirrors `apply_go_func_value_fields`/`apply_go_registrations`
    /// ordering). Recomputes from scratch every time (never incrementally
    /// patched) so an edit to a type/return/package-var-DEFINING file always
    /// updates every retained CONSUMING file's recovery, even when the
    /// consuming file itself did not change — the required bidirectional
    /// incremental-parity guarantee.
    ///
    /// `receiver_config` MUST be the same config the caller used to build
    /// this graph (`build_with_receiver_config[_and_scope_graph_inputs]`
    /// passes its own `receiver_config` through; every other caller passes
    /// `ReceiverRecoveryConfig::default()`, matching the config those build
    /// paths already use) — otherwise a Legacy-mode build's intentionally
    /// disabled `var_local`/`type_assertion` forms would be silently
    /// re-enabled by this pass reusing a different classifier.
    pub fn apply_go_receiver_indices(
        &mut self,
        files: &BTreeMap<String, ParsedFile>,
        receiver_config: &crate::resolution::ReceiverRecoveryConfig,
    ) {
        self.clear_go_receiver_indices();
        if !files
            .values()
            .any(|p| p.language == crate::languages::Language::Go)
        {
            return;
        }
        self.go_return_types = crate::go_receiver_index::extract_go_return_types(files);
        self.go_package_vars = crate::go_receiver_index::extract_go_package_vars(files);
        let field_targets =
            crate::type_providers::go::GoTypeProvider::from_parsed_files(files).go_field_targets();
        self.rematerialize_go_receiver_keys(files, receiver_config, &field_targets);
    }

    fn rematerialize_go_receiver_keys(
        &mut self,
        files: &BTreeMap<String, ParsedFile>,
        receiver_config: &crate::resolution::ReceiverRecoveryConfig,
        field_targets: &BTreeMap<
            (crate::resolution::GoOwnerIdentity, String),
            crate::resolution::GoFieldTarget,
        >,
    ) {
        let updates = {
            let facts = crate::go_receiver_index::GoReceiverFacts {
                return_types: &self.go_return_types,
                package_vars: &self.go_package_vars,
                field_types: &self.go_field_types,
                field_targets,
                package_basenames: &self.go_package_basenames,
                imports: &self.imports,
                go_file_profiles: &self.go_file_profiles,
            };
            self.compute_go_receiver_updates(files, &facts, receiver_config)
        };
        for (caller, old_site, classification, evidence) in updates {
            let mut site_telemetry =
                crate::go_owner_partition::GoOwnerPartitionTelemetry::default();
            site_telemetry.observe(evidence, 1);
            self.go_owner_identity_partition.merge(site_telemetry);
            if site_telemetry.affected_sites() > 0 {
                self.go_owner_identity_partition_sites.insert(
                    crate::go_owner_partition::site_key(&old_site),
                    site_telemetry,
                );
            }
            let mut updated = old_site.clone();
            updated.receiver_type = classification
                .recovered
                .as_ref()
                .map(|r| r.static_type.clone());
            updated.receiver_owner_identity = classification
                .recovered
                .as_ref()
                .and_then(|r| r.owner_identity.clone());
            updated.receiver_local_type_shadowed = classification.proof_shadowed
                || updated.receiver_type.as_ref().is_some_and(|ty| {
                    files
                        .get(&old_site.caller.file)
                        .and_then(|parsed| {
                            Self::function_node_for_id(parsed, &old_site.caller).map(|func_node| {
                                parsed.go_local_type_shadows(&func_node, ty, old_site.start_byte)
                            })
                        })
                        .unwrap_or(false)
                });
            updated.receiver_recovery = classification.recovered.as_ref().map(|r| r.recovery);
            updated.receiver_outcome = classification
                .recovered
                .as_ref()
                .and_then(|r| r.go_field_target.as_ref())
                .map(crate::resolution::go_field_target_outcome);
            updated.receiver_materialized = classification.materialized;
            if updated == old_site {
                continue; // no change -- skip the take/insert churn.
            }
            if let Some(sites) = self.calls.get_mut(&caller) {
                if sites.take(&old_site).is_some() {
                    sites.insert(updated.clone());
                }
            }
            if let Some(sites) = self.callers.get_mut(&old_site.callee_name) {
                for site in sites {
                    if site.caller == old_site.caller && site.cmp_key() == old_site.cmp_key() {
                        site.receiver_type = updated.receiver_type.clone();
                        site.receiver_owner_identity = updated.receiver_owner_identity.clone();
                        site.receiver_local_type_shadowed = updated.receiver_local_type_shadowed;
                        site.receiver_recovery = updated.receiver_recovery;
                        site.receiver_outcome = updated.receiver_outcome.clone();
                        site.receiver_materialized = updated.receiver_materialized;
                    }
                }
            }
        }
    }

    /// Parallel per-caller recompute, mirroring `compute_rust_receiver_updates`'s
    /// shape exactly (rayon over the `self.calls`-ordered caller list).
    pub(crate) fn compute_go_receiver_updates(
        &self,
        files: &BTreeMap<String, ParsedFile>,
        facts: &crate::go_receiver_index::GoReceiverFacts<'_>,
        receiver_config: &crate::resolution::ReceiverRecoveryConfig,
    ) -> Vec<(
        FunctionId,
        CallSite,
        crate::resolution::ReceiverClassification,
        crate::go_owner_partition::GoPartitionEvidence,
    )> {
        use rayon::prelude::*;

        let classifier = receiver_config.classifier();
        let ordered: Vec<(&FunctionId, &BTreeSet<CallSite>)> = self.calls.iter().collect();
        ordered
            .par_iter()
            .copied()
            .map(|(caller, sites)| {
                Self::go_receiver_updates_for_caller(
                    caller,
                    sites,
                    files,
                    classifier.as_ref(),
                    facts,
                    receiver_config.var_local,
                )
            })
            .collect::<Vec<Vec<_>>>()
            .into_iter()
            .flatten()
            .collect()
    }

    fn go_receiver_updates_for_caller(
        caller: &FunctionId,
        sites: &BTreeSet<CallSite>,
        files: &BTreeMap<String, ParsedFile>,
        classifier: &dyn crate::resolution::ReceiverClassifier,
        facts: &crate::go_receiver_index::GoReceiverFacts<'_>,
        var_local: bool,
    ) -> Vec<(
        FunctionId,
        CallSite,
        crate::resolution::ReceiverClassification,
        crate::go_owner_partition::GoPartitionEvidence,
    )> {
        let mut out = Vec::new();
        let Some(parsed) = files.get(&caller.file) else {
            return out;
        };
        if parsed.language != crate::languages::Language::Go {
            return out;
        }
        let Some(fn_node) = Self::function_node_for_id(parsed, caller) else {
            return out;
        };
        let all_lines: BTreeSet<usize> = (caller.start_line..=caller.end_line).collect();
        let macro_shadow: BTreeSet<String> = BTreeSet::new(); // Go has no macro-arg extraction.
        let (ast_calls, _facts) = parsed.function_calls_with_qualifier_and_spans_on_lines(
            &fn_node,
            &all_lines,
            &macro_shadow,
        );
        let recv_var = parsed
            .language
            .go_receiver_var(&fn_node)
            .map(|n| parsed.node_text(&n).to_string());
        let file_imports = facts.imports.get(&caller.file);
        for site in sites {
            let Some(qualifier) = site.qualifier.as_deref() else {
                continue; // unqualified call -- nothing to retype.
            };
            let Some(meta) = ast_calls.iter().find(|meta| {
                meta.callee_name == site.callee_name
                    && meta.start_byte == site.start_byte
                    && meta.end_byte == site.end_byte
            }) else {
                continue;
            };
            let Some(receiver_expr) = meta.receiver_node else {
                continue;
            };
            let ctx = crate::go_receiver_index::GoReceiverCtx {
                parsed,
                fn_node,
                qualifier,
                receiver_expr,
                call_line: meta.line,
                call_start_byte: meta.start_byte,
                recv_var: recv_var.as_deref(),
                file_imports,
                caller_file: &caller.file,
            };
            let (classification, evidence) =
                crate::go_receiver_index::classify_go_receiver_expanded_with_partition(
                    &ctx, classifier, facts, var_local,
                );
            out.push((caller.clone(), site.clone(), classification, evidence));
        }
        out
    }

    fn clear_python_property_accesses(&mut self) {
        self.property_getters.clear();
        self.cached_property_getters.clear();
        self.property_accesses.clear();
        self.property_access_fanout_skips = 0;
        self.property_access_store_skips = 0;
    }

    /// P7: scan `files` for Python `@property`/`@cached_property` getters
    /// (S1) and the LOAD access sites that reach them (S2). Whole-program
    /// derived (S2's unknown-receiver fanout tier needs S1's complete
    /// cross-file index) — clears and recomputes from scratch, mirroring
    /// `apply_go_registrations`. Must run after `method_owners` /
    /// `method_class_span` / `class_bases` are already populated (S1/S2 both
    /// read them instead of re-deriving owner/class-span facts).
    pub fn apply_python_property_accesses(&mut self, files: &BTreeMap<String, ParsedFile>) {
        self.clear_python_property_accesses();
        if !files
            .values()
            .any(|p| p.language == crate::languages::Language::Python)
        {
            return;
        }
        self.apply_python_property_getters(files);
        self.apply_python_property_access_sites(files);
    }

    fn clear_framework_entries(&mut self) {
        self.framework_entries.clear();
        self.framework_entry_unresolved_handlers = 0;
    }

    /// P9: scan `files` for recognized Flask/FastAPI/Express route
    /// registrations and record them in `framework_entries`. Whole-program
    /// derived (Express identifier-arg resolution needs the complete
    /// `functions`/`js_ts_function_locals` index) — clears and recomputes
    /// from scratch, mirroring `apply_go_registrations`/
    /// `apply_python_property_accesses`. Candidate detection itself lives in
    /// `crate::framework_entries` (kept out of this already-large module);
    /// this method only orchestrates it against the CallGraph's own state.
    pub fn apply_framework_entries(&mut self, files: &BTreeMap<String, ParsedFile>) {
        self.clear_framework_entries();
        let (entries, unresolved) =
            crate::framework_entries::apply(files, &self.functions, &self.js_ts_function_locals);
        self.framework_entries = entries;
        self.framework_entry_unresolved_handlers = unresolved;
    }

    /// S1: index every exact-match `@property`/`@cached_property`-decorated
    /// method by `(owner_key, method_name)`. Reuses the already-populated
    /// `method_owners` index (Phase 1 records an owner for every method
    /// regardless of decoration) rather than re-deriving the owner from the
    /// AST — the FunctionId built here matches Phase 1's exactly (same
    /// `all_functions()` node, same name/line-range calls), so the lookup is
    /// a guaranteed hit whenever an owner exists at all.
    fn apply_python_property_getters(&mut self, files: &BTreeMap<String, ParsedFile>) {
        for (file_path, parsed) in files {
            if parsed.language != crate::languages::Language::Python {
                continue;
            }
            for func_node in parsed.all_functions() {
                let Some(kind) = parsed.python_property_kind(&func_node) else {
                    continue;
                };
                let Some(name_node) = parsed.language.function_name(&func_node) else {
                    continue;
                };
                let name = parsed.node_text(&name_node).to_string();
                let (start, end) = parsed.node_line_range(&func_node);
                let fid = FunctionId {
                    file: file_path.clone(),
                    name: name.clone(),
                    start_line: start,
                    end_line: end,
                };
                let Some(owner) = self.method_owners.get(&fid).cloned() else {
                    continue;
                };
                self.property_getters
                    .entry((owner, name))
                    .or_default()
                    .push(fid.clone());
                if kind == crate::ast::PythonPropertyKind::CachedProperty {
                    self.cached_property_getters.insert(fid);
                }
            }
        }
    }

    /// S2: walk every Python function body for LOAD accesses of an S1-indexed
    /// property name and record them, narrowed per the receiver tiers (see
    /// `self_property_owner_getters` for tier 1; tier 2 — persisted receiver-
    /// type recovery — is deliberately never consulted here, since there is
    /// no `CallSite` to recover a type onto; tier 3 is the capped unknown-
    /// receiver fanout below).
    fn apply_python_property_access_sites(&mut self, files: &BTreeMap<String, ParsedFile>) {
        if self.property_getters.is_empty() {
            return;
        }
        let attr_names: BTreeSet<String> = self
            .property_getters
            .keys()
            .map(|(_, name)| name.clone())
            .collect();
        for (file_path, parsed) in files {
            if parsed.language != crate::languages::Language::Python {
                continue;
            }
            for func_node in parsed.all_functions() {
                let Some(name_node) = parsed.language.function_name(&func_node) else {
                    continue;
                };
                let func_name = parsed.node_text(&name_node).to_string();
                let (start, end) = parsed.node_line_range(&func_node);
                let caller_id = FunctionId {
                    file: file_path.clone(),
                    name: func_name,
                    start_line: start,
                    end_line: end,
                };
                // F2 (codex MAJOR 2; re-fixed per codex re-review): tier-1
                // same-class narrowing requires a genuine instance method —
                // not `@staticmethod`/`@classmethod`, first POSITIONAL param
                // literally `self` (`python_is_self_instance_method`), AND
                // owned by a class at all (`method_owners.contains_key`). A
                // `self`-named parameter in a function `method_owners` has
                // no entry for (e.g. a top-level `def f(self)`) is not a
                // genuine receiver — it must route straight to the tier-3
                // fanout below, not enter tier-1 only to drop when
                // `self_property_owner_getters` finds no owner. Computed
                // once per function, not per candidate.
                let is_instance_method = parsed.python_is_self_instance_method(&func_node)
                    && self.method_owners.contains_key(&caller_id);
                let scan = parsed.python_attribute_load_candidates(&func_node, &attr_names);
                // F5: every store/delete-context access of an indexed
                // property name is telemetry, whole-program derived like the
                // rest of this table (recomputed from scratch each build).
                self.property_access_store_skips += scan.store_skips;
                for cand in scan.candidates {
                    self.apply_python_property_access_candidate(
                        &caller_id,
                        is_instance_method,
                        cand,
                    );
                }
            }
        }
    }

    /// One raw S2 candidate -> zero or more `PropertyAccessRecord`s (fanout
    /// tier can mint up to 3).
    fn apply_python_property_access_candidate(
        &mut self,
        caller_id: &FunctionId,
        is_instance_method: bool,
        cand: crate::ast::PythonAttributeLoadCandidate,
    ) {
        let getters: BTreeSet<FunctionId> =
            if is_instance_method && cand.receiver_identifier.as_deref() == Some("self") {
                // Tier 1: `self.attr` narrows to the enclosing class's own
                // getter (or its single same-file base's). A known, non-matching
                // receiver type is a NEGATIVE result, not "unknown" — it must
                // NOT fall through to the tier-3 fanout (that would misattribute
                // the access to unrelated classes we positively know it isn't).
                match self.self_property_owner_getters(caller_id, &cand.attr_name) {
                    Some(fids) => fids,
                    None => return,
                }
            } else {
                // Tier 3: unknown receiver, `cls` INCLUDED (spec-review MAJOR —
                // no cls-like-self shortcut here; class access returns the
                // descriptor object, not the getter, so `cls` gets no narrowing
                // privilege over any other unrecognized receiver). Also reached
                // by `self.attr` when `is_instance_method` is false (F2:
                // `@staticmethod`/`@classmethod` — `self` there is an ordinary
                // parameter of unknown type, not a receiver). All classes
                // defining the property name, capped.
                self.property_getters
                    .iter()
                    .filter(|((_, name), _)| name == &cand.attr_name)
                    .flat_map(|(_, fids)| fids.iter().cloned())
                    .collect()
            };
        if getters.is_empty() {
            return;
        }
        if getters.len() > 3 {
            self.property_access_fanout_skips += 1;
            return;
        }
        let site = PropertyAccessSite {
            file: caller_id.file.clone(),
            line: cand.line,
            start_byte: cand.start_byte,
            end_byte: cand.end_byte,
        };
        for getter in getters {
            self.property_accesses.insert(PropertyAccessRecord {
                enclosing: caller_id.clone(),
                getter,
                site: site.clone(),
            });
        }
    }

    /// Tier-1 owner lookup for `self.attr`: the enclosing method's own class,
    /// falling back to its single same-file base (mirrors
    /// `inherited_direct_base`'s depth-1, single-inheritance-only limits).
    /// `None` means neither the class nor its base defines the property —
    /// the caller must NOT fall through to the unknown-receiver fanout tier.
    fn self_property_owner_getters(
        &self,
        caller: &FunctionId,
        attr: &str,
    ) -> Option<BTreeSet<FunctionId>> {
        let owner = self.method_owners.get(caller)?;
        // F1 (codex MAJOR 1): the ambiguity check must gate the OWN-CLASS
        // hit too, not just the inherited-base fallback below — checked
        // before either lookup.
        if self.method_class_span_ambiguous.contains(caller) {
            return None;
        }
        let caller_span = *self.method_class_span.get(caller)?;
        if let Some(fids) = self
            .property_getters
            .get(&(owner.clone(), attr.to_string()))
        {
            // The bare `(owner, attr)` key collides across files (two
            // same-named classes in different files both defining the
            // property) — filter to getters in the CALLER's own file whose
            // class span equals the caller's, the same discipline the
            // inherited-base fallback below already applies.
            let same_class: BTreeSet<FunctionId> = fids
                .iter()
                .filter(|fid| {
                    fid.file == caller.file
                        && self.method_class_span.get(*fid) == Some(&caller_span)
                        && !self.method_class_span_ambiguous.contains(*fid)
                })
                .cloned()
                .collect();
            if !same_class.is_empty() {
                return Some(same_class);
            }
        }

        let bases = self.class_bases.get(&(caller.file.clone(), caller_span))?;
        if bases.len() != 1 {
            return None;
        }
        let (base_span, base_owner) = match &bases[0] {
            ClassBaseLink::SameFile { span, owner } => (*span, owner.as_str()),
            ClassBaseLink::Barrier => return None,
        };
        let ids = self
            .property_getters
            .get(&(base_owner.to_string(), attr.to_string()))?;
        let in_base: BTreeSet<FunctionId> = ids
            .iter()
            .filter(|fid| {
                fid.file == caller.file
                    && self.method_class_span.get(*fid) == Some(&base_span)
                    && !self.method_class_span_ambiguous.contains(*fid)
            })
            .cloned()
            .collect();
        if in_base.is_empty() {
            None
        } else {
            Some(in_base)
        }
    }

    fn extract_go_build_profiles<'a>(
        files: impl Iterator<Item = (&'a str, &'a ParsedFile)>,
    ) -> (
        BTreeMap<String, crate::go_build_profile::GoBuildProfile>,
        BTreeMap<String, usize>,
    ) {
        let mut profiles = BTreeMap::new();
        let mut unparsed = BTreeMap::new();
        for (path, parsed) in files {
            if parsed.language != crate::languages::Language::Go {
                continue;
            }
            let (profile, count) = crate::go_build_profile::extract_go_file_profile(path, parsed);
            profiles.insert(path.to_string(), profile);
            if count > 0 {
                unparsed.insert(path.to_string(), count);
            }
        }
        (profiles, unparsed)
    }

    /// Build a call graph from only the specified files (Phases 1+2: direct calls only).
    ///
    /// Unlike `build()`, this skips Phase 3 (indirect call resolution) because
    /// that requires knowledge of all functions, not just the subset. The caller
    /// should run `recompute_indirect_calls` on the merged result.
    pub fn build_direct_subset(
        files: &BTreeMap<String, ParsedFile>,
        only_files: &BTreeSet<String>,
    ) -> Self {
        Self::build_direct_subset_with_receiver_config(
            files,
            only_files,
            &crate::resolution::ReceiverRecoveryConfig::default(),
        )
    }

    /// `build_direct_subset` with an explicit receiver-recovery config (spec §2 seam).
    pub fn build_direct_subset_with_receiver_config(
        files: &BTreeMap<String, ParsedFile>,
        only_files: &BTreeSet<String>,
        receiver_config: &crate::resolution::ReceiverRecoveryConfig,
    ) -> Self {
        let classifier = receiver_config.classifier();
        let classifier: &dyn crate::resolution::ReceiverClassifier = classifier.as_ref();
        let mut functions: BTreeMap<String, Vec<FunctionId>> = BTreeMap::new();
        let mut calls: BTreeMap<FunctionId, BTreeSet<CallSite>> = BTreeMap::new();
        let mut callers: BTreeMap<String, Vec<CallSite>> = BTreeMap::new();
        let mut static_functions: BTreeSet<(String, String)> = BTreeSet::new();
        let mut imports: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
        let mut methods: BTreeMap<(String, String), Vec<FunctionId>> = BTreeMap::new();
        let mut method_owners: BTreeMap<FunctionId, String> = BTreeMap::new();
        let mut method_class_span: BTreeMap<FunctionId, (usize, usize)> = BTreeMap::new();
        let mut method_class_span_ambiguous: BTreeSet<FunctionId> = BTreeSet::new();
        let mut receiver_vars: BTreeMap<FunctionId, String> = BTreeMap::new();
        let mut method_facts: BTreeMap<FunctionId, MethodFacts> = BTreeMap::new();
        // Repo-wide macro-name shadow set (P8 F1 BLOCKER) — deliberately
        // scanned over the FULL `files` map (not `only_files`): a
        // `macro_rules!` def outside the changed subset must still shadow.
        let macro_shadow = crate::rust_macro_args::collect_macro_shadow_set(files);
        let (go_file_profiles, go_build_profile_unparsed) = Self::extract_go_build_profiles(
            files
                .iter()
                .filter(|(path, _)| only_files.contains(*path))
                .map(|(p, f)| (p.as_str(), f)),
        );

        for (file_path, parsed) in files {
            if !only_files.contains(file_path) {
                continue;
            }
            let file_imports = parsed.extract_imports();
            if !file_imports.is_empty() {
                imports.insert(file_path.clone(), file_imports);
            }
        }

        // Phase 1: Collect function definitions from subset.
        for (file_path, parsed) in files {
            if !only_files.contains(file_path) {
                continue;
            }
            for func_node in parsed.all_functions() {
                if let Some(name_node) = parsed.language.function_name(&func_node) {
                    let name = parsed.node_text(&name_node).to_string();
                    let (start, end) = parsed.node_line_range(&func_node);
                    let func_id = FunctionId {
                        file: file_path.clone(),
                        name: name.clone(),
                        start_line: start,
                        end_line: end,
                    };
                    functions
                        .entry(name.clone())
                        .or_default()
                        .push(func_id.clone());
                    let (owner, trait_key, recv_var, class_span) =
                        Self::method_metadata(parsed, &func_node);
                    if let Some(o) = owner {
                        methods
                            .entry((o.clone(), name.clone()))
                            .or_default()
                            .push(func_id.clone());
                        method_owners.insert(func_id.clone(), o);
                        if let Some(s) = class_span {
                            record_method_class_span(
                                &mut method_class_span,
                                &mut method_class_span_ambiguous,
                                &func_id,
                                s,
                            );
                        }
                    }
                    if let Some(t) = trait_key {
                        methods
                            .entry((t, name.clone()))
                            .or_default()
                            .push(func_id.clone());
                    }
                    if let Some(rv) = recv_var {
                        receiver_vars.insert(func_id.clone(), rv);
                    }
                    if let Some(facts) = Self::method_facts(parsed, &func_node) {
                        method_facts.insert(func_id.clone(), facts);
                    }

                    if matches!(
                        parsed.language,
                        crate::languages::Language::C | crate::languages::Language::Cpp
                    ) {
                        if has_static_specifier(parsed, &func_node) {
                            static_functions.insert((file_path.clone(), name));
                        }
                    }
                }
            }
        }

        // Phase 2: Find call sites from subset.
        let mut macro_arg_facts: BTreeMap<String, crate::rust_macro_args::MacroArgFacts> =
            BTreeMap::new();
        for (file_path, parsed) in files {
            if !only_files.contains(file_path) {
                continue;
            }
            let mut file_macro_arg_facts = crate::rust_macro_args::MacroArgFacts::default();
            for func_node in parsed.all_functions() {
                let func_name = match parsed.language.function_name(&func_node) {
                    Some(n) => parsed.node_text(&n).to_string(),
                    None => continue,
                };
                let (start, end) = parsed.node_line_range(&func_node);
                let caller_id = FunctionId {
                    file: file_path.clone(),
                    name: func_name,
                    start_line: start,
                    end_line: end,
                };
                let all_lines: BTreeSet<usize> = (start..=end).collect();
                let (call_sites, facts) = parsed.function_calls_with_qualifier_and_spans_on_lines(
                    &func_node,
                    &all_lines,
                    &macro_shadow,
                );
                file_macro_arg_facts.calls_recorded += facts.calls_recorded;
                file_macro_arg_facts.skipped_macros += facts.skipped_macros;
                file_macro_arg_facts.ctor_skips += facts.ctor_skips;
                let recv_var = parsed
                    .language
                    .go_receiver_var(&func_node)
                    .map(|n| parsed.node_text(&n).to_string());
                let file_imports_ref = imports.get(file_path);

                for meta in call_sites {
                    let callee_name = meta.callee_name;
                    let line = meta.line;
                    let start_byte = meta.start_byte;
                    let end_byte = meta.end_byte;
                    let qualifier = Self::recover_self_receiver_qualifier(
                        parsed,
                        &callee_name,
                        line,
                        meta.qualifier,
                    );
                    let classification = classifier.classify(crate::resolution::ReceiverCtx {
                        receiver_expr: meta.receiver_node,
                        qualifier: qualifier.as_deref(),
                        fn_node: func_node,
                        call_line: line,
                        call_start_byte: start_byte,
                        parsed,
                        recv_var: recv_var.as_deref(),
                        file_imports: file_imports_ref,
                    });
                    let recovered = classification.recovered.as_ref();
                    let site = CallSite {
                        caller: caller_id.clone(),
                        callee_name: callee_name.clone(),
                        line,
                        kind: meta
                            .kind_override
                            .unwrap_or_else(|| Self::call_kind_at(parsed, start_byte, end_byte)),
                        start_byte,
                        end_byte,
                        qualifier,
                        receiver_type: recovered.as_ref().map(|r| r.static_type.clone()),
                        receiver_owner_identity: recovered
                            .as_ref()
                            .and_then(|r| r.owner_identity.clone()),
                        receiver_local_type_shadowed: classification.proof_shadowed
                            || recovered.as_ref().is_some_and(|r| {
                                parsed.go_local_type_shadows(&func_node, &r.static_type, start_byte)
                            }),
                        receiver_recovery: recovered.as_ref().map(|r| r.recovery),
                        receiver_materialized: classification.materialized,
                        arg_count: meta.arg_count,
                        arg_spread: meta.arg_spread,
                        receiver_outcome: None,
                        origin: meta.origin_override.unwrap_or(CallSiteOrigin::Source),
                        pre_resolved_target: None,
                    };
                    calls
                        .entry(caller_id.clone())
                        .or_default()
                        .insert(site.clone());
                    callers.entry(callee_name).or_default().push(site);
                }
            }
            if file_macro_arg_facts != crate::rust_macro_args::MacroArgFacts::default() {
                macro_arg_facts.insert(file_path.clone(), file_macro_arg_facts);
            }
        }

        // Phase 5 (direct-subset): build class facts from the complete files map
        // so unchanged class owners remain available after incremental merges.
        let (class_bases, clean_class_spans) = Self::build_class_facts(files);

        // R4c: populate import bindings for subset.
        let subset_files: BTreeMap<String, &ParsedFile> = files
            .iter()
            .filter(|(k, _)| only_files.contains(*k))
            .map(|(k, v)| (k.clone(), v))
            .collect();
        let mut import_bindings_map: BTreeMap<String, Vec<ImportBinding>> = BTreeMap::new();
        let mut module_bindings_map: BTreeMap<String, BTreeMap<String, ModuleBindingKind>> =
            BTreeMap::new();
        for (fp, parsed) in &subset_files {
            if matches!(
                parsed.language,
                crate::languages::Language::Python
                    | crate::languages::Language::JavaScript
                    | crate::languages::Language::TypeScript
                    | crate::languages::Language::Tsx
            ) {
                let bindings = parsed.extract_import_bindings();
                if !bindings.is_empty() {
                    import_bindings_map.insert(fp.clone(), bindings);
                }
                let mbindings = parsed.extract_module_bindings();
                if !mbindings.is_empty() {
                    module_bindings_map.insert(fp.clone(), mbindings);
                }
            }
        }
        mark_import_binding_eligibility(&mut import_bindings_map, &module_bindings_map);
        let (js_ts_exports, js_ts_function_locals) = Self::extract_js_ts_resolution_facts_from_iter(
            subset_files.iter().map(|(fp, parsed)| (fp, *parsed)),
        );
        let indexed_files: BTreeSet<String> = files.keys().cloned().collect();

        CallGraph {
            functions,
            calls,
            callers,
            param_slots_unknown: Self::parameter_slots_unknown(files),
            level3_indirect_resolved: 0,
            static_functions,
            imports,
            methods,
            method_owners,
            method_class_span,
            method_class_span_ambiguous,
            class_bases,
            clean_class_spans,
            methods_by_scope: BTreeMap::new(),
            extension_methods: BTreeMap::new(),
            identity_complete: BTreeSet::new(),
            field_types: BTreeMap::new(),
            return_types: BTreeMap::new(),
            receiver_vars,
            promoted_aliases: BTreeMap::new(),
            embedding_gaps: BTreeMap::new(),
            interface_impls: BTreeMap::new(),
            interface_gaps: BTreeMap::new(),
            interface_overapprox: BTreeMap::new(),
            interface_method_names: BTreeSet::new(),
            interface_dispatch_computed: false,
            method_arity: BTreeMap::new(),
            method_facts,
            scope_graph: None,
            import_bindings: import_bindings_map,
            module_bindings: module_bindings_map,
            indexed_files,
            js_ts_exports,
            // P4: whole-program resolved export facts — left empty here,
            // exactly like the Go whole-program state below:
            // `build_direct_subset` never computes whole-program facts
            // itself; the caller re-applies `apply_js_export_resolution` on
            // the merged graph.
            js_ts_resolved_exports: BTreeMap::new(),
            js_export_chain_unresolved: 0,
            js_export_barrel_conflicts: 0,
            js_ts_function_locals,
            // P5: whole-program Go func-value state — left empty here, exactly
            // like `interface_impls`/`interface_dispatch_computed: false`
            // above: `build_direct_subset` never computes whole-program Go
            // facts itself; the caller re-applies `apply_go_func_value_fields`
            // / `apply_go_registrations` on the merged graph (mirrors
            // `apply_go_embedding_promotion` / `apply_go_interface_dispatch`).
            go_package_basenames: BTreeMap::new(),
            go_known_struct_identities: BTreeSet::new(),
            go_func_typed_fields: BTreeSet::new(),
            go_registrations: BTreeSet::new(),
            go_registration_shadowed_skips: 0,
            go_registration_ambiguous_owner_skips: 0,
            go_registration_unknown_owner_recorded: 0,
            go_file_profiles,
            go_build_profile_unparsed,
            go_owner_identity_profile_conflict: 0,
            skipped_go_testdata_files: 0,
            go_module_graph: Default::default(),
            go_import_path_proven_files: 0,
            go_import_path_unproven_files: 0,
            go_import_path_unproven_reasons: BTreeMap::new(),
            go_owner_identity_partition: Default::default(),
            go_owner_identity_partition_sites: BTreeMap::new(),
            go_bare_value_ref_ambiguous: 0,
            // P7: whole-program Python property-access state — left empty
            // here for the same reason as the Go func-value state above;
            // the caller re-applies `apply_python_property_accesses` on the
            // merged graph.
            property_getters: BTreeMap::new(),
            cached_property_getters: BTreeSet::new(),
            property_accesses: BTreeSet::new(),
            property_access_fanout_skips: 0,
            property_access_store_skips: 0,
            macro_arg_facts,
            // P8: like the JS/Go/Python whole-program facts left empty above,
            // this is NOT left empty — `build_direct_subset` scans the FULL
            // `files` map (not `only_files`) for the macro shadow set (see
            // `macro_shadow` above), so this is always the fresh, correct
            // whole-program value. `merge` overwrites the retained graph's
            // field with this one for exactly that reason.
            macro_shadow_intersection: crate::rust_macro_args::transparent_shadow_intersection(
                &macro_shadow,
            ),
            // P9: whole-program framework-entry state — left empty here for
            // the same reason as the Go/Python whole-program facts above; the
            // caller re-applies `apply_framework_entries` on the merged
            // graph.
            framework_entries: BTreeSet::new(),
            framework_entry_unresolved_handlers: 0,
            go_return_types: BTreeMap::new(),
            go_field_types: BTreeMap::new(),
            go_declaration_kind_index: BTreeMap::new(),
            go_promoted_concrete_selectors: BTreeSet::new(),
            go_interface_declarations: BTreeMap::new(),
            go_method_declarations: BTreeMap::new(),
            go_interface_live_types: BTreeSet::new(),
            go_embedded_interface_methods: BTreeMap::new(),
            go_package_vars: BTreeMap::new(),
        }
    }

    /// Recall-biased name+static resolver for scope computation and Phase-3
    /// indirect resolution only. Edge creation uses `resolve_call_site`.
    pub fn resolve_callees(&self, callee_name: &str, caller_file: &str) -> Vec<&FunctionId> {
        let func_ids = match self.functions.get(callee_name) {
            Some(ids) => ids,
            None => return Vec::new(),
        };

        // If there's a static function with this name in the caller's file, use only that one
        if self
            .static_functions
            .contains(&(caller_file.to_string(), callee_name.to_string()))
        {
            return func_ids
                .iter()
                .filter(|fid| fid.file == caller_file)
                .collect();
        }

        // Otherwise, return all definitions that are NOT static in other files
        func_ids
            .iter()
            .filter(|fid| {
                // Include if: it's in the same file, OR it's not static
                fid.file == caller_file
                    || !self
                        .static_functions
                        .contains(&(fid.file.clone(), callee_name.to_string()))
            })
            .collect()
    }

    fn populate_method_identity_indices(&mut self, files: &BTreeMap<String, ParsedFile>) {
        let Some(graph) = self.scope_graph.as_ref().filter(|g| g.complete) else {
            self.methods_by_scope.clear();
            self.extension_methods.clear();
            self.identity_complete.clear();
            self.field_types.clear();
            self.return_types.clear();
            return;
        };

        let mut methods_by_scope: BTreeMap<(ScopeId, String), Vec<FunctionId>> = BTreeMap::new();
        let mut extension_methods: BTreeMap<(String, String), Vec<FunctionId>> = BTreeMap::new();
        let mut bucket_coverage: BTreeMap<(String, String), BTreeSet<FunctionId>> = BTreeMap::new();
        let aliases = Self::rust_type_aliases_by_module(graph, files);
        let mut field_types: BTreeMap<(ScopeId, String), Vec<(Option<String>, TypeKey)>> =
            BTreeMap::new();
        let mut return_types: BTreeMap<FunctionId, Vec<(Option<String>, TypeKey)>> =
            BTreeMap::new();

        for (file_path, parsed) in files {
            if !matches!(parsed.language, crate::languages::Language::Rust) {
                continue;
            }
            let Some(file_id) = graph.file_paths.get(file_path).copied() else {
                continue;
            };

            for func_node in parsed.all_functions() {
                let Some(name_node) = parsed.language.function_name(&func_node) else {
                    continue;
                };
                let name = parsed.node_text(&name_node).to_string();
                let (start, end) = parsed.node_line_range(&func_node);
                let fid = FunctionId {
                    file: file_path.clone(),
                    name: name.clone(),
                    start_line: start,
                    end_line: end,
                };
                if !self.method_owners.contains_key(&fid) {
                    continue;
                }
                let Some(enclosing) = parsed.language.rust_enclosing_method_item(&func_node) else {
                    continue;
                };
                let Some(module_scope) =
                    Self::module_scope_for_byte(graph, file_id, enclosing.start_byte())
                else {
                    continue;
                };

                if let Some(type_syntax) = Self::rust_method_impl_type_syntax(parsed, &func_node) {
                    let concrete_owner_key = crate::resolution::owner_key(&type_syntax);
                    match resolve_type_path_to_type_scope(graph, module_scope, &type_syntax) {
                        Some(TypeKey::InRepo(scope)) => {
                            Self::insert_method_by_scope(
                                &mut methods_by_scope,
                                scope,
                                name.clone(),
                                fid.clone(),
                            );
                            bucket_coverage
                                .entry((concrete_owner_key, name.clone()))
                                .or_default()
                                .insert(fid.clone());
                        }
                        Some(TypeKey::External(canon)) => {
                            Self::insert_extension_method(
                                &mut extension_methods,
                                canon,
                                name.clone(),
                                fid.clone(),
                            );
                        }
                        None => {}
                    }
                }

                if let Some(trait_syntax) = parsed
                    .language
                    .rust_impl_trait(&func_node)
                    .map(|n| parsed.node_text(&n).to_string())
                {
                    let trait_key = crate::resolution::owner_key(&trait_syntax);
                    if let Some(TypeKey::InRepo(scope)) =
                        resolve_type_path_to_type_scope(graph, module_scope, &trait_syntax)
                    {
                        Self::insert_method_by_scope(
                            &mut methods_by_scope,
                            scope,
                            name.clone(),
                            fid.clone(),
                        );
                        bucket_coverage
                            .entry((trait_key, name.clone()))
                            .or_default()
                            .insert(fid.clone());
                    }
                }
            }

            Self::extract_rust_field_types(graph, &aliases, file_id, parsed, &mut field_types);
            Self::extract_rust_return_types(
                graph,
                &aliases,
                file_path,
                file_id,
                parsed,
                &mut return_types,
            );
        }

        let identity_complete = self
            .methods
            .iter()
            .filter_map(|(key, fids)| {
                let expected: BTreeSet<_> = fids.iter().cloned().collect();
                bucket_coverage
                    .get(key)
                    .filter(|covered| **covered == expected)
                    .map(|_| key.clone())
            })
            .collect();

        self.methods_by_scope = methods_by_scope;
        self.extension_methods = extension_methods;
        self.identity_complete = identity_complete;
        self.field_types = field_types;
        self.return_types = return_types;
    }

    pub(crate) fn module_scope_for_byte(
        graph: &ScopeGraph,
        file: crate::name_resolution::types::FileId,
        byte: usize,
    ) -> Option<ScopeId> {
        let mut scope = enclosing_scope(graph, file, byte)?;
        loop {
            let record = graph.scope(scope)?;
            if matches!(record.kind, ScopeKind::Root | ScopeKind::Module) {
                return Some(scope);
            }
            scope = graph.parent_of(scope)?;
        }
    }

    fn rust_method_impl_type_syntax(
        parsed: &ParsedFile,
        func_node: &tree_sitter::Node<'_>,
    ) -> Option<String> {
        let enclosing = parsed.language.rust_enclosing_method_item(func_node)?;
        let node = match enclosing.kind() {
            "impl_item" => enclosing.child_by_field_name("type")?,
            "trait_item" => enclosing.child_by_field_name("name")?,
            _ => return None,
        };
        Some(parsed.node_text(&node).to_string())
    }

    fn method_metadata(
        parsed: &ParsedFile,
        func_node: &tree_sitter::Node<'_>,
    ) -> (
        Option<String>,
        Option<String>,
        Option<String>,
        Option<(usize, usize)>,
    ) {
        let owner = parsed
            .language
            .method_owner(func_node)
            .map(|n| crate::resolution::owner_key(parsed.node_text(&n)));
        let trait_key = parsed
            .language
            .rust_impl_trait(func_node)
            .map(|n| crate::resolution::owner_key(parsed.node_text(&n)));
        let recv_var = parsed
            .language
            .go_receiver_var(func_node)
            .map(|n| parsed.node_text(&n).to_string());
        let class_span = parsed
            .language
            .method_owner_class_node(func_node)
            .map(|c| (c.start_byte(), c.end_byte()));
        (owner, trait_key, recv_var, class_span)
    }

    fn recover_self_receiver_qualifier(
        parsed: &ParsedFile,
        callee_name: &str,
        line: usize,
        qualifier: Option<String>,
    ) -> Option<String> {
        if qualifier.is_some() {
            return qualifier;
        }
        if !matches!(parsed.language, crate::languages::Language::Rust) {
            return None;
        }
        let line_text = parsed.source.lines().nth(line.saturating_sub(1))?;
        ["self", "this", "cls"]
            .into_iter()
            .find(|receiver| line_text.contains(&format!("{receiver}.{callee_name}")))
            .map(str::to_string)
    }

    fn call_kind_at(parsed: &ParsedFile, start_byte: usize, end_byte: usize) -> CallKind {
        if !matches!(parsed.language, crate::languages::Language::Rust) {
            return CallKind::Call;
        }
        parsed
            .tree
            .root_node()
            .descendant_for_byte_range(start_byte, end_byte)
            .map(|mut node| loop {
                if node.kind() == "macro_invocation" {
                    return CallKind::MacroInvocation;
                }
                if node.start_byte() <= start_byte && node.end_byte() >= end_byte {
                    if let Some(parent) = node.parent() {
                        node = parent;
                        continue;
                    }
                }
                return CallKind::Call;
            })
            .unwrap_or(CallKind::Call)
    }

    /// Exact function-source prefix ending immediately before `before_byte`.
    /// Assignment-based callback resolution must never inspect later writes.
    fn extract_func_source_before<'a>(
        parsed: &'a ParsedFile,
        func_id: &FunctionId,
        before_byte: usize,
    ) -> Option<&'a str> {
        let function = Self::function_node_for_id(parsed, func_id)?;
        if before_byte < function.start_byte() || before_byte > function.end_byte() {
            return None;
        }
        parsed.source.get(function.start_byte()..before_byte)
    }

    /// Extract the source text for a function from its parsed file.
    fn extract_func_source(parsed: &ParsedFile, func_id: &FunctionId) -> String {
        let lines: Vec<&str> = parsed.source.lines().collect();
        let start = func_id.start_line.saturating_sub(1); // 1-indexed to 0-indexed
        let end = func_id.end_line.min(lines.len());
        lines[start..end].join("\n")
    }

    /// Caller sites targeting a function named `name`, including `::`-scoped
    /// keys: the `callers` map is keyed by the raw callee text, so a qualified
    /// call `T::name()` lives under `"T::name"`, not `"name"`. Returns sites
    /// under both the bare key and any `"*::name"` key. Over-collection is safe:
    /// every consumer re-resolves each site via `resolve_call_site` and filters
    /// by exact target, so scoped keys only add *true* callers (S3 — mirrors the
    /// navigation-side `scoped_caller_sites`).
    fn caller_sites_scoped(&self, name: &str) -> Vec<&CallSite> {
        let suffix = format!("::{name}");
        let mut out: Vec<&CallSite> = Vec::new();
        for (key, sites) in &self.callers {
            if key == name || key.ends_with(&suffix) {
                out.extend(sites.iter());
            }
        }
        out
    }

    /// Find all callers of a function by name, up to a given depth.
    ///
    /// Respects static linkage: a call to `func_name` in file X only counts
    /// if `resolve_callees(func_name, X)` includes a function in `target_file`
    /// (when provided). This prevents static functions in other files from
    /// being falsely reported as callers.
    pub fn callers_of(&self, func_name: &str, max_depth: usize) -> Vec<(FunctionId, usize)> {
        self.callers_of_in_file(func_name, max_depth, None)
    }

    /// Like `callers_of`, but only returns callers whose call actually resolves
    /// to a function in `target_file`.
    pub fn callers_of_in_file(
        &self,
        func_name: &str,
        max_depth: usize,
        target_file: Option<&str>,
    ) -> Vec<(FunctionId, usize)> {
        let mut result = Vec::new();
        let mut visited: BTreeSet<FunctionId> = BTreeSet::new();
        let mut queue: VecDeque<(FunctionId, usize)> = VecDeque::new();

        if let Some(func_ids) = self.functions.get(func_name) {
            for fid in func_ids {
                let in_target_file = match target_file {
                    Some(tf) => fid.file == tf,
                    None => true,
                };
                if in_target_file && visited.insert(fid.clone()) {
                    queue.push_back((fid.clone(), 0));
                }
            }
        }

        while let Some((target, depth)) = queue.pop_front() {
            if depth > 0 {
                result.push((target.clone(), depth));
            }

            if depth >= max_depth {
                continue;
            }

            for site in self.caller_sites_scoped(&target.name) {
                let resolved = self.resolve_call_site(site);
                let hit = resolved.iter().any(|c| c.target == &target);
                if !hit {
                    continue;
                }

                if visited.insert(site.caller.clone()) {
                    queue.push_back((site.caller.clone(), depth + 1));
                }
            }
        }

        result
    }

    /// Resolve callers of a specific function, respecting static linkage.
    ///
    /// Returns only CallSites where the call to `callee_name` from the caller's
    /// file actually resolves to the function in `target_file`.
    pub fn resolve_callers(&self, callee_name: &str, target_file: &str) -> Vec<&CallSite> {
        self.caller_sites_scoped(callee_name)
            .into_iter()
            .filter(|site| {
                let resolved = self.resolve_call_site(site);
                resolved.iter().any(|c| c.target.file == target_file)
            })
            .collect()
    }

    /// Find all callees of a function by name, up to a given depth.
    pub fn callees_of(
        &self,
        func_name: &str,
        file: &str,
        max_depth: usize,
    ) -> Vec<(FunctionId, usize)> {
        let mut result = Vec::new();
        let mut visited: BTreeSet<FunctionId> = BTreeSet::new();
        let mut queue: VecDeque<(FunctionId, usize)> = VecDeque::new();

        // Find the starting function
        if let Some(func_ids) = self.functions.get(func_name) {
            for fid in func_ids {
                if fid.file == file {
                    queue.push_back((fid.clone(), 0));
                    visited.insert(fid.clone());
                }
            }
        }

        while let Some((func_id, depth)) = queue.pop_front() {
            if depth > 0 {
                result.push((func_id.clone(), depth));
            }

            if depth >= max_depth {
                continue;
            }

            if let Some(sites) = self.calls.get(&func_id) {
                for site in sites {
                    let callee_ids = self.resolve_call_site(site);
                    for c in callee_ids {
                        if visited.insert(c.target.clone()) {
                            queue.push_back((c.target.clone(), depth + 1));
                        }
                    }
                }
            }
        }

        result
    }

    /// Find the function containing a specific line in a file.
    pub fn function_at(&self, file: &str, line: usize) -> Option<&FunctionId> {
        for func_ids in self.functions.values() {
            for fid in func_ids {
                if fid.file == file && line >= fid.start_line && line <= fid.end_line {
                    return Some(fid);
                }
            }
        }
        None
    }

    /// Detect cycles in the call graph reachable from a set of functions.
    pub fn find_cycles_from(&self, start_funcs: &[&str]) -> Vec<Vec<FunctionId>> {
        let mut cycles = Vec::new();

        for &start_name in start_funcs {
            let mut path: Vec<FunctionId> = Vec::new();
            let mut visited: BTreeSet<FunctionId> = BTreeSet::new();

            if let Some(func_ids) = self.functions.get(start_name) {
                for fid in func_ids {
                    self.dfs_cycles(fid, &mut path, &mut visited, &mut cycles);
                }
            }
        }

        cycles
    }

    fn dfs_cycles(
        &self,
        node: &FunctionId,
        path: &mut Vec<FunctionId>,
        visited: &mut BTreeSet<FunctionId>,
        cycles: &mut Vec<Vec<FunctionId>>,
    ) {
        if let Some(pos) = path.iter().position(|f| f == node) {
            // Found a cycle
            let cycle: Vec<FunctionId> = path[pos..].to_vec();
            if !cycle.is_empty() {
                cycles.push(cycle);
            }
            return;
        }

        if visited.contains(node) {
            return;
        }

        visited.insert(node.clone());
        path.push(node.clone());

        if let Some(sites) = self.calls.get(node) {
            for site in sites {
                let callee_ids = self.resolve_call_site(site);
                for c in callee_ids {
                    self.dfs_cycles(c.target, path, visited, cycles);
                }
            }
        }

        path.pop();
    }
}

impl CallSite {
    fn cmp_key(
        &self,
    ) -> (
        &str,
        &str,
        usize,
        CallKind,
        usize,
        usize,
        Option<&str>,
        Option<&str>,
        Option<&FunctionId>,
    ) {
        (
            &self.caller.name,
            &self.callee_name,
            self.line,
            self.kind,
            self.start_byte,
            self.end_byte,
            self.qualifier.as_deref(),
            self.receiver_type.as_deref(),
            self.pre_resolved_target.as_ref(),
        )
    }
}

impl PartialOrd for CallSite {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for CallSite {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.cmp_key().cmp(&other.cmp_key())
    }
}

fn record_method_class_span(
    spans: &mut BTreeMap<FunctionId, (usize, usize)>,
    ambiguous: &mut BTreeSet<FunctionId>,
    fid: &FunctionId,
    span: (usize, usize),
) {
    if let Some(existing) = spans.get(fid) {
        if *existing != span {
            ambiguous.insert(fid.clone());
        }
    } else {
        spans.insert(fid.clone(), span);
    }
}

/// Extract assignment target names from a Python assignment or expression statement.
/// Handles `x = ...`, `x: int = ...`, `x, y = ...` (tuple unpacking).
fn extract_assignment_targets(
    node: tree_sitter::Node,
    source: &str,
    counts: &mut BTreeMap<String, usize>,
) {
    // expression_statement may wrap an assignment
    let assign = if node.kind() == "expression_statement" {
        // Walk children to find assignment
        let mut found = None;
        let count = node.child_count();
        for i in 0..count {
            if let Some(c) = node.child(i) {
                if c.kind() == "assignment" || c.kind() == "augmented_assignment" {
                    found = Some(c);
                    break;
                }
            }
        }
        found
    } else if node.kind() == "assignment" {
        Some(node)
    } else {
        None
    };
    if let Some(assign_node) = assign {
        if let Some(left) = assign_node.child_by_field_name("left") {
            collect_identifiers_from_pattern(left, source, counts);
        }
    }
}

/// Recursively collect identifier names from an assignment target pattern.
fn collect_identifiers_from_pattern(
    node: tree_sitter::Node,
    source: &str,
    counts: &mut BTreeMap<String, usize>,
) {
    match node.kind() {
        "identifier" | "type_identifier" => count_identifier_like(node, source, counts),
        "as_pattern" | "as_pattern_target" | "expression_list" | "list_pattern"
        | "pattern_list" | "tuple_pattern" => {
            let mut cursor = node.walk();
            for child in node.children(&mut cursor) {
                if child.kind() != "," {
                    collect_identifiers_from_pattern(child, source, counts);
                }
            }
        }
        _ => {}
    }
}

fn count_identifier_like(
    node: tree_sitter::Node,
    source: &str,
    counts: &mut BTreeMap<String, usize>,
) {
    if matches!(node.kind(), "identifier" | "type_identifier") {
        let name = source[node.start_byte()..node.end_byte()].to_string();
        *counts.entry(name).or_default() += 1;
    }
}

fn first_identifier_like<'a>(node: tree_sitter::Node<'a>) -> Option<tree_sitter::Node<'a>> {
    if matches!(node.kind(), "identifier" | "type_identifier") {
        return Some(node);
    }
    let mut cursor = node.walk();
    for child in node.children(&mut cursor) {
        if let Some(found) = first_identifier_like(child) {
            return Some(found);
        }
    }
    None
}

/// Mark import-binding eligibility: wildcard in file poisons all; re-bound name
/// (any non-Import module binding with the same local name) makes that binding
/// ineligible. Duplicate import bindings for the same local name also make both
/// ineligible (ambiguous import).
fn mark_import_binding_eligibility(
    import_bindings: &mut BTreeMap<String, Vec<ImportBinding>>,
    module_bindings: &BTreeMap<String, BTreeMap<String, ModuleBindingKind>>,
) {
    for (file, bindings) in import_bindings.iter_mut() {
        // Check for wildcard imports in this file.
        let has_wildcard = bindings
            .iter()
            .any(|b| matches!(b.kind, ImportBindingKind::WildcardImport));

        // Count how many import bindings share the same local name.
        let mut local_counts: BTreeMap<String, usize> = BTreeMap::new();
        for b in bindings.iter() {
            *local_counts.entry(b.local.clone()).or_default() += 1;
        }

        let file_module_bindings = module_bindings.get(file);

        for binding in bindings.iter_mut() {
            if has_wildcard {
                binding.eligible = false;
                continue;
            }
            // Duplicate import bindings for the same local -> ineligible.
            if local_counts.get(&binding.local).copied().unwrap_or(0) > 1 {
                binding.eligible = false;
                continue;
            }
            // Re-bound by a non-Import module binding -> ineligible.
            if let Some(mb) = file_module_bindings {
                if let Some(kind) = mb.get(&binding.local) {
                    if !matches!(kind, ModuleBindingKind::Import) {
                        binding.eligible = false;
                        continue;
                    }
                }
            }
            // Member imports start eligible; module/wildcard do not (R4c
            // only handles unqualified calls, which come from member imports).
            binding.eligible = matches!(binding.kind, ImportBindingKind::MemberImport);
        }
    }
}

/// Check if a file path matches a module path for R4c resolution.
///
/// Handles two module-path styles:
/// - **Python**: dotted (`myapp.utils`), relative (`.utils`, `..pkg`)
/// - **JS/TS**: slash-based (`./utils`, `../pkg/utils`, `@scope/pkg`)
///
/// Strategy: extract the last path component ("stem") of the module path
/// and match it against the file's stem or directory name (for packages).
pub fn file_matches_module(
    file: &str,
    module_path: &str,
    caller_file: &str,
    indexed_files: &BTreeSet<String>,
) -> bool {
    let module_path = module_path.trim();
    if module_path.is_empty() {
        return false;
    }

    // Determine if this is a JS/TS-style path (contains `/`) or a Python-style
    // dotted module path.
    let is_js_path = module_path.contains('/');

    // Relative path resolution: try to construct the exact candidate first.
    let is_relative = if is_js_path {
        module_path.starts_with("./") || module_path.starts_with("../")
    } else {
        module_path.starts_with('.')
    };

    if is_relative {
        let caller_dir = caller_file.rsplit_once('/').map(|(d, _)| d).unwrap_or("");

        if is_js_path {
            // JS/TS relative: strip leading "./" or count "../" levels.
            let mut rel = module_path;
            let mut base = caller_dir.to_string();
            while let Some(rest) = rel.strip_prefix("../") {
                base = base
                    .rsplit_once('/')
                    .map(|(d, _)| d)
                    .unwrap_or("")
                    .to_string();
                rel = rest;
            }
            rel = rel.strip_prefix("./").unwrap_or(rel);
            // Try with common extensions.
            for ext in &[".js", ".ts", ".tsx", ".mjs", ".cjs", ""] {
                let candidate = if base.is_empty() {
                    format!("{rel}{ext}")
                } else {
                    format!("{base}/{rel}{ext}")
                };
                if indexed_files.contains(&candidate) && candidate == file {
                    return true;
                }
            }
            // Try index file inside directory
            for index in &["index.js", "index.ts", "index.tsx"] {
                let candidate = if base.is_empty() {
                    format!("{rel}/{index}")
                } else {
                    format!("{base}/{rel}/{index}")
                };
                if indexed_files.contains(&candidate) && candidate == file {
                    return true;
                }
            }
        } else {
            // Python relative: `.utils` or `..pkg.utils`
            let stripped = module_path.trim_start_matches('.');
            let dot_count = module_path.len() - stripped.len();
            let mut base = caller_dir.to_string();
            for _ in 1..dot_count {
                base = base
                    .rsplit_once('/')
                    .map(|(d, _)| d)
                    .unwrap_or("")
                    .to_string();
            }
            // Convert remaining dotted path to FULL relative file path
            // (e.g. `pkg.utils` → `pkg/utils`), not just the last component.
            let rel = stripped.replace('.', "/");
            for ext in &[".py"] {
                let candidate = if base.is_empty() {
                    format!("{rel}{ext}")
                } else {
                    format!("{base}/{rel}{ext}")
                };
                if indexed_files.contains(&candidate) && candidate == file {
                    return true;
                }
            }
            // __init__.py for the full path
            let init_candidate = if base.is_empty() {
                format!("{rel}/__init__.py")
            } else {
                format!("{base}/{rel}/__init__.py")
            };
            if indexed_files.contains(&init_candidate) && init_candidate == file {
                return true;
            }
            // Relative imports NEVER fall through to stem — they must resolve
            // relative to the caller's directory or not at all.
            return false;
        }
    }

    // For multi-component Python dotted absolute imports (e.g. `myapp.utils`),
    // try converting to a path and checking indexed_files. Do NOT use the stem
    // fallback — it would match ANY file named `utils.py` regardless of package.
    if !is_js_path && !is_relative {
        let stripped = module_path.trim_start_matches('.');
        if stripped.contains('.') {
            // Multi-component absolute import: try full path candidates.
            let rel = stripped.replace('.', "/");
            // Try `myapp/utils.py`
            let py_candidate = format!("{rel}.py");
            if indexed_files.contains(&py_candidate) && py_candidate == file {
                return true;
            }
            // Try `myapp/utils/__init__.py`
            let init_candidate = format!("{rel}/__init__.py");
            if indexed_files.contains(&init_candidate) && init_candidate == file {
                return true;
            }
            // No stem fallback for dotted imports — fail open to R5.
            return false;
        }
    }

    // Stem-based fallback: only for single-component imports (e.g. `utils`
    // in Python, `./utils` in JS) where the module IS the stem.
    let last_component = if is_js_path {
        // JS: last path segment, strip leading @ for scoped packages.
        module_path
            .rsplit('/')
            .next()
            .unwrap_or(module_path)
            .trim_start_matches('.')
    } else {
        // Python: last dotted component, strip leading dots for relative.
        let stripped = module_path.trim_start_matches('.');
        stripped.rsplit('.').next().unwrap_or(stripped)
    };

    if last_component.is_empty() {
        return false;
    }

    // File stem: strip directory and extension.
    let file_name = file.rsplit('/').next().unwrap_or(file);
    let file_stem = file_name
        .rsplit_once('.')
        .map(|(s, _)| s)
        .unwrap_or(file_name);
    if file_stem == last_component {
        return true;
    }

    // Package directories: `utils/__init__.py` matches module "utils",
    // `utils/index.js` matches module "utils".
    if file.ends_with("/__init__.py") || file.ends_with("\\__init__.py") {
        let dir = file
            .trim_end_matches("/__init__.py")
            .trim_end_matches("\\__init__.py");
        let dir_name = dir.rsplit('/').next().unwrap_or(dir);
        if dir_name == last_component {
            return true;
        }
    }
    for index in &["/index.js", "/index.ts", "/index.tsx"] {
        if file.ends_with(index) {
            let dir = &file[..file.len() - index.len()];
            let dir_name = dir.rsplit('/').next().unwrap_or(dir);
            if dir_name == last_component {
                return true;
            }
        }
    }
    false
}

/// Resolve a JS/TS relative import (`./util`, `../pkg/util`) to its single
/// exact candidate file, if indexed. Unlike `file_matches_module`, this has
/// no stem fallback: `./util` from `pkg/app.ts` can match `pkg/util.ts` or
/// `pkg/util/index.ts`, but never an unrelated `elsewhere/util.ts`.
///
/// Shared by `file_matches_js_ts_relative_module_exact` (a membership test
/// against one candidate `file`) and P4's JS/TS export-fact chain resolution
/// (`CallGraph::apply_js_export_resolution`), which needs the actual target
/// file, not just a yes/no match against a proposed one.
pub fn resolve_js_ts_relative_module(
    module_path: &str,
    caller_file: &str,
    indexed_files: &BTreeSet<String>,
) -> Option<String> {
    let module_path = module_path.trim();
    if !(module_path.starts_with("./") || module_path.starts_with("../")) {
        return None;
    }

    let caller_dir = caller_file.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
    let mut rel = module_path;
    let mut base_parts: Vec<&str> = if caller_dir.is_empty() {
        Vec::new()
    } else {
        caller_dir.split('/').collect()
    };
    while let Some(rest) = rel.strip_prefix("../") {
        base_parts.pop()?;
        rel = rest;
    }
    rel = rel.strip_prefix("./").unwrap_or(rel);
    if rel.is_empty() {
        return None;
    }
    let base = base_parts.join("/");

    for ext in &[".js", ".jsx", ".mjs", ".cjs", ".ts", ".tsx", ""] {
        let candidate = if base.is_empty() {
            format!("{rel}{ext}")
        } else {
            format!("{base}/{rel}{ext}")
        };
        if indexed_files.contains(&candidate) {
            return Some(candidate);
        }
    }

    for index in &[
        "index.js",
        "index.jsx",
        "index.mjs",
        "index.cjs",
        "index.ts",
        "index.tsx",
    ] {
        let candidate = if base.is_empty() {
            format!("{rel}/{index}")
        } else {
            format!("{base}/{rel}/{index}")
        };
        if indexed_files.contains(&candidate) {
            return Some(candidate);
        }
    }

    None
}

/// Exact relative JS/TS module match for R4c `ImportMember` resolution.
///
/// Unlike `file_matches_module`, this helper deliberately has no stem fallback:
/// `./util` from `pkg/app.ts` can match `pkg/util.ts` or `pkg/util/index.ts`,
/// but never an unrelated `elsewhere/util.ts`.
pub fn file_matches_js_ts_relative_module_exact(
    file: &str,
    module_path: &str,
    caller_file: &str,
    indexed_files: &BTreeSet<String>,
) -> bool {
    resolve_js_ts_relative_module(module_path, caller_file, indexed_files).as_deref() == Some(file)
}

/// Check if a C/C++ function definition has a `static` storage class specifier.
fn has_static_specifier(parsed: &ParsedFile, func_node: &tree_sitter::Node<'_>) -> bool {
    let mut cursor = func_node.walk();
    for child in func_node.children(&mut cursor) {
        if child.kind() == "storage_class_specifier" && parsed.node_text(&child) == "static" {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn method_class_span_populated_for_python_methods() {
        use crate::languages::Language::Python;
        use std::collections::BTreeMap;

        let mut files = BTreeMap::new();
        files.insert(
            "a.py".to_string(),
            ParsedFile::parse(
                "a.py",
                "class C:\n    def f(self):\n        return 1\n",
                Python,
            )
            .unwrap(),
        );
        let cg = CallGraph::build(&files);
        let fid = cg
            .functions
            .get("f")
            .unwrap()
            .iter()
            .find(|f| f.file == "a.py")
            .unwrap();
        let span = cg.method_class_span.get(fid).expect("span recorded");
        assert_eq!(span.0, 0);
        assert!(span.1 > span.0);
    }

    fn build_rust_call_graph(source: &str) -> CallGraph {
        use crate::ast::ParsedFile;
        use crate::languages::Language::Rust;
        use crate::name_resolution::rust_populator::RustCrateConfig;

        let mut files = std::collections::BTreeMap::new();
        files.insert(
            "a.rs".into(),
            ParsedFile::parse("a.rs", source, Rust).unwrap(),
        );
        let mut inputs = ScopeGraphBuildInputs::from_files_convention(&files);
        inputs.cfg = RustCrateConfig {
            crate_roots: files.keys().cloned().collect(),
            ..RustCrateConfig::default()
        };
        CallGraph::build_with_scope_graph_inputs(&files, Some(&inputs))
    }

    fn build_complete(srcs: &[(&str, &str)]) -> CallGraph {
        use crate::ast::ParsedFile;
        use crate::languages::Language::Rust;
        use crate::name_resolution::rust_populator::RustCrateConfig;

        let mut files = std::collections::BTreeMap::new();
        for (path, source) in srcs {
            files.insert(
                path.to_string(),
                ParsedFile::parse(path, source, Rust).unwrap(),
            );
        }
        let mut inputs = ScopeGraphBuildInputs::from_files_convention(&files);
        inputs.cfg = RustCrateConfig {
            crate_roots: files.keys().cloned().collect(),
            ..RustCrateConfig::default()
        };
        CallGraph::build_with_scope_graph_inputs(&files, Some(&inputs))
    }

    #[test]
    fn rust_receiver_updates_parallel_matches_serial_reference() {
        // Real-dir load -> populated scope_graph (in-memory ad-hoc Rust fixtures have no
        // crate root, so RustReceiverTyper::new would panic). src/navigation: 397 updates.
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/navigation");
        let repo = crate::repo_loader::load_repo(&dir).unwrap();
        let ctx = crate::cpg::CpgContext::build(&repo.files, None);
        let cg = &ctx.cpg.call_graph;
        let par = cg.compute_rust_receiver_updates(&repo.files);
        let serial = cg.compute_rust_receiver_updates_reference(&repo.files);
        assert_eq!(par, serial, "receiver update sequence diverged");
        assert!(!par.is_empty(), "fixture produced no receiver updates");
    }

    #[test]
    fn rematerialize_corpus_has_rust_receiver_updates() {
        // Floors the cache-byte gate's corpus (src/cpg) so it actually exercises remat.
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/cpg");
        let repo = crate::repo_loader::load_repo(&dir).unwrap();
        let ctx = crate::cpg::CpgContext::build(&repo.files, None);
        let n = ctx
            .cpg
            .call_graph
            .compute_rust_receiver_updates(&repo.files)
            .len();
        assert!(
            n > 50,
            "too few receiver updates to surface a remat divergence: {n}"
        ); // measured 1251
    }

    fn call_graph_type_scope(cg: &CallGraph, path: &str, name: &str) -> ScopeId {
        let graph = cg.scope_graph.as_ref().expect("scope graph");
        let file = graph.file_paths.get(path).copied().expect("file id");
        let scope = CallGraph::module_scope_for_byte(graph, file, 0).expect("module scope");
        match resolve_type_path_to_type_scope(graph, scope, name) {
            Some(TypeKey::InRepo(scope)) => scope,
            other => panic!("expected in-repo type scope for {name}, got {other:?}"),
        }
    }

    #[test]
    fn field_and_return_indices_resolved_self_and_def_scope() {
        let cg = build_complete(&[(
            "a.rs",
            "pub struct Inner; impl Inner { fn poke(&self){} pub fn new()->Self{Inner} }\n\
             pub struct Outer { pub inner: Inner }\npub fn make()->Inner{Inner}\n",
        )]);
        let inner = call_graph_type_scope(&cg, "a.rs", "Inner");
        let outer = call_graph_type_scope(&cg, "a.rs", "Outer");
        assert_eq!(
            cg.field_types
                .get(&(outer, "inner".into()))
                .and_then(|v| v.first()),
            Some(&(None, TypeKey::InRepo(inner)))
        );
        let fid = |n: &str| {
            cg.functions
                .values()
                .flatten()
                .find(|f| f.name == n)
                .cloned()
                .unwrap()
        };
        assert_eq!(
            cg.return_types.get(&fid("make")).and_then(|v| v.first()),
            Some(&(None, TypeKey::InRepo(inner)))
        );
        assert_eq!(
            cg.return_types.get(&fid("new")).and_then(|v| v.first()),
            Some(&(None, TypeKey::InRepo(inner)))
        );
    }

    #[test]
    fn field_and_return_indices_alias_cfg_and_omit_fallthrough_types() {
        let cg = build_complete(&[(
            "a.rs",
            "pub struct Inner;\ntype Alias = Inner;\n\
             pub struct Outer {\n\
                 #[cfg(feature = \"a\")]\n\
                 pub aliased: Alias,\n\
                 pub missing: Missing,\n\
                 pub text: String,\n\
             }\n\
             #[cfg(feature = \"a\")]\n\
             pub fn make_alias()->Alias{Inner}\n\
             pub fn make_text()->String{String::new()}\n",
        )]);
        let inner = call_graph_type_scope(&cg, "a.rs", "Inner");
        let outer = call_graph_type_scope(&cg, "a.rs", "Outer");
        assert_eq!(
            cg.field_types
                .get(&(outer, "aliased".into()))
                .and_then(|v| v.first()),
            Some(&(
                Some("#[cfg(feature = \"a\")]".to_string()),
                TypeKey::InRepo(inner)
            ))
        );
        assert!(!cg.field_types.contains_key(&(outer, "missing".into())));
        assert!(!cg.field_types.contains_key(&(outer, "text".into())));

        let fid = |n: &str| {
            cg.functions
                .values()
                .flatten()
                .find(|f| f.name == n)
                .cloned()
                .unwrap()
        };
        assert_eq!(
            cg.return_types
                .get(&fid("make_alias"))
                .and_then(|v| v.first()),
            Some(&(
                Some("#[cfg(feature = \"a\")]".to_string()),
                TypeKey::InRepo(inner)
            ))
        );
        assert!(!cg.return_types.contains_key(&fid("make_text")));
    }

    #[test]
    fn cfg_gated_alias_does_not_collapse_to_one_unconditional_target() {
        let cg = build_complete(&[(
            "a.rs",
            "pub struct A;\npub struct B;\n\
             #[cfg(feat_a)]\n\
             pub type Alias = A;\n\
             #[cfg(feat_b)]\n\
             pub type Alias = B;\n\
             pub struct Holder { pub f: Alias }\n",
        )]);
        let holder = call_graph_type_scope(&cg, "a.rs", "Holder");
        let entries = cg
            .field_types
            .get(&(holder, "f".into()))
            .cloned()
            .unwrap_or_default();

        assert!(
            entries.iter().all(|(cfg, _)| cfg.is_some()) || entries.is_empty(),
            "cfg-gated alias collapsed to unconditional: {entries:?}"
        );
        assert!(entries.len() != 1 || entries[0].0.is_some());
    }

    #[test]
    fn callsite_origin_receiver_materialized_and_outcome_serde_default_and_excluded_from_cmp_key() {
        let cg = build_rust_call_graph(
            "struct Engine;\nimpl Engine { fn go(&self) {} }\nfn run(e: Engine) { e.go(); }\n",
        );
        let mut a = cg
            .calls
            .values()
            .flat_map(|sites| sites.iter())
            .find(|site| site.callee_name == "go")
            .cloned()
            .expect("go call site");
        let b = a.clone();
        a.receiver_outcome = Some(crate::resolution_identity::ReceiverOutcome {
            key: crate::resolution_identity::ReceiverTypeKey::Bare("Engine".to_string()),
            bare: "Engine".to_string(),
            recovery: crate::resolution::ReceiverRecovery::TypedParam,
        });
        a.receiver_materialized = true;
        a.origin = CallSiteOrigin::IndirectResolution;

        assert_eq!(a.cmp_key(), b.cmp_key());
        let mut legacy_json = serde_json::to_value(&b).unwrap();
        legacy_json
            .as_object_mut()
            .unwrap()
            .remove("receiver_outcome");
        legacy_json
            .as_object_mut()
            .unwrap()
            .remove("receiver_materialized");
        legacy_json.as_object_mut().unwrap().remove("origin");
        let defaulted: CallSite = serde_json::from_value(legacy_json).unwrap();
        assert_eq!(defaulted.receiver_outcome, None);
        assert!(!defaulted.receiver_materialized);
        assert_eq!(defaulted.origin, CallSiteOrigin::Source);

        let back: CallSite = bincode::deserialize(&bincode::serialize(&a).unwrap()).unwrap();
        assert_eq!(a, back);
    }

    /// P8: a macro-arg-minted call site carries `kind: Call` / `origin:
    /// MacroArg` -- NOT `call_kind_at`'s ancestor-walk classification, which
    /// would otherwise tag any span nested under a `macro_invocation` as
    /// `MacroInvocation` (routing it to the wrong NS_MACRO namespace).
    #[test]
    fn macro_arg_call_site_carries_call_kind_and_macro_arg_origin() {
        let cg = build_rust_call_graph(
            "fn check(x: i32) -> bool { x > 0 }\nfn host() { assert!(check(1)); }\n",
        );
        let site = site_in(&cg, "host", "check");
        assert_eq!(site.kind, CallKind::Call);
        assert_eq!(site.origin, CallSiteOrigin::MacroArg);
        assert_eq!(site.arg_count, None);
        assert!(!site.arg_spread);
    }

    /// Companion pin: an ordinary (non-macro) call site's `kind`/`origin`
    /// derivation is byte-for-byte unchanged by the P8 plumbing -- still
    /// `Call`/`Source` via the pre-existing `call_kind_at` path.
    #[test]
    fn ordinary_call_site_kind_and_origin_unchanged() {
        let cg =
            build_rust_call_graph("fn check(x: i32) -> bool { x > 0 }\nfn host() { check(1); }\n");
        let site = site_in(&cg, "host", "check");
        assert_eq!(site.kind, CallKind::Call);
        assert_eq!(site.origin, CallSiteOrigin::Source);
    }

    /// A macro invocation that mints nothing (non-allowlisted, or allowlisted
    /// but with no call-shaped args) still produces zero `CallSite`s for the
    /// macro name itself -- `call_kind_at`'s classification of the macro's
    /// OWN span is never exercised by a real CallSite because no site is ever
    /// minted at that span (pre-existing PR-2 behavior, unchanged).
    #[test]
    fn nonallowlisted_macro_still_mints_nothing_for_its_own_name() {
        let cg = build_rust_call_graph("fn host() { my_undefined_macro!(check(1)); }\n");
        assert!(
            !cg.calls
                .values()
                .flat_map(|s| s.iter())
                .any(|s| s.callee_name == "my_undefined_macro" || s.callee_name == "check"),
            "non-allowlisted macro must not mint any call site"
        );
    }

    fn site_in(cg: &CallGraph, caller: &str, callee: &str) -> CallSite {
        cg.calls
            .iter()
            .find(|(fid, _)| fid.name == caller)
            .and_then(|(_, sites)| sites.iter().find(|site| site.callee_name == callee))
            .cloned()
            .unwrap_or_else(|| panic!("missing call site {caller}->{callee}"))
    }

    fn c_files(srcs: &[(&str, &str)]) -> BTreeMap<String, ParsedFile> {
        srcs.iter()
            .map(|(path, source)| {
                (
                    (*path).to_string(),
                    ParsedFile::parse(path, source, crate::languages::Language::C).unwrap(),
                )
            })
            .collect()
    }

    fn indirect_call_dump(cg: &CallGraph) -> Vec<String> {
        let mut out = Vec::new();
        for (caller, sites) in &cg.calls {
            for site in sites {
                if site.origin == CallSiteOrigin::IndirectResolution {
                    out.push(format!(
                        "{}:{}:{}:{}:{}-{}",
                        caller.file,
                        caller.name,
                        site.callee_name,
                        site.line,
                        site.start_byte,
                        site.end_byte
                    ));
                }
            }
        }
        out.sort();
        out
    }

    fn indirect_caller_dump(cg: &CallGraph) -> Vec<String> {
        let mut out = Vec::new();
        for (callee, sites) in &cg.callers {
            for site in sites {
                if site.origin == CallSiteOrigin::IndirectResolution {
                    out.push(format!(
                        "{}:{}:{}:{}:{}-{}",
                        callee,
                        site.caller.file,
                        site.caller.name,
                        site.line,
                        site.start_byte,
                        site.end_byte
                    ));
                }
            }
        }
        out.sort();
        out
    }

    #[test]
    fn clear_indirect_calls_removes_only_synthetic_entries_from_calls_and_callers() {
        let mut cg = CallGraph::build(&c_files(&[(
            "callbacks.c",
            "void target() {}\nvoid run() { target(); }\n",
        )]));
        let source = site_in(&cg, "run", "target");
        assert_eq!(source.origin, CallSiteOrigin::Source);

        let mut synthetic = source.clone();
        synthetic.origin = CallSiteOrigin::IndirectResolution;
        let original_call_count = cg.calls.get(&source.caller).unwrap().len();
        let inserted = cg
            .calls
            .get_mut(&source.caller)
            .unwrap()
            .insert(synthetic.clone());
        assert!(
            !inserted,
            "origin is excluded from CallSite identity so source wins in calls"
        );
        assert_eq!(
            cg.calls.get(&source.caller).unwrap().len(),
            original_call_count
        );
        cg.callers
            .entry("target".to_string())
            .or_default()
            .push(synthetic);

        assert!(cg
            .callers
            .get("target")
            .unwrap()
            .iter()
            .any(|site| site.origin == CallSiteOrigin::IndirectResolution));

        cg.clear_indirect_calls();

        let calls = cg.calls.get(&source.caller).unwrap();
        assert!(calls
            .iter()
            .any(|site| site.origin == CallSiteOrigin::Source));
        assert!(!calls
            .iter()
            .any(|site| site.origin == CallSiteOrigin::IndirectResolution));
        assert!(cg
            .callers
            .get("target")
            .unwrap()
            .iter()
            .all(|site| site.origin == CallSiteOrigin::Source));
    }

    #[test]
    fn recompute_indirect_calls_is_idempotent_with_level3_disabled() {
        let files_v1 = c_files(&[(
            "callbacks.c",
            "void old_handler() {}\nvoid execute(void (*cb)()) { cb(); }\nvoid outer() { execute(old_handler); }\n",
        )]);
        let files_v2 = c_files(&[(
            "callbacks.c",
            "void new_handler() {}\nvoid execute(void (*cb)()) { cb(); }\nvoid outer() { execute(new_handler); }\n",
        )]);

        let mut merged = CallGraph::build(&files_v1);
        let changed = BTreeSet::from(["callbacks.c".to_string()]);
        merged.remove_files(&changed);
        merged.merge(CallGraph::build_direct_subset(&files_v2, &changed));
        merged.recompute_indirect_calls(&files_v2);
        let once = indirect_call_dump(&merged);
        let callers_once = indirect_caller_dump(&merged);
        assert!(!once.iter().any(|entry| entry.contains(":new_handler:")));
        assert!(!once.iter().any(|entry| entry.contains(":old_handler:")));

        merged.recompute_indirect_calls(&files_v2);
        assert_eq!(once, indirect_call_dump(&merged));
        assert_eq!(callers_once, indirect_caller_dump(&merged));

        let full = CallGraph::build(&files_v2);
        assert_eq!(once, indirect_call_dump(&full));
        assert_eq!(callers_once, indirect_caller_dump(&full));
    }

    /// P8: at the CallGraph-internal mechanics level, `remove_files`
    /// retaining `macro_arg_facts` by file and `merge` extending the map
    /// (the same `js_ts_exports` pattern) is exactly correct in isolation --
    /// this test pins that low-level plumbing. It does NOT by itself prove
    /// the extracted facts stay valid: `macro_arg_facts`' CONTENT depends on
    /// the repo-wide macro shadow set (F1 fix, codex re-review BLOCKER), so
    /// the higher-level `build_incremental_with_scope_graph_inputs` caller
    /// must additionally guard against that set drifting underneath an
    /// incremental rebuild and fall back to a full rebuild when it does --
    /// see `macro_shadow_intersection` and the
    /// `incremental_from_previous_falls_back_to_full_rebuild_on_*_macro_shadow`
    /// tests in `src/navigation/mod.rs`.
    #[test]
    fn macro_arg_facts_remove_files_retains_by_file_and_merge_extends() {
        let files = build_complete(&[
            (
                "a.rs",
                "fn check(x: i32) -> bool { x > 0 }\nfn host() { assert!(check(1)); }\n",
            ),
            ("b.rs", "fn other() {}\n"),
        ]);
        let mut files_map: std::collections::BTreeMap<String, ParsedFile> = Default::default();
        for (path, source) in [
            (
                "a.rs",
                "fn check(x: i32) -> bool { x > 0 }\nfn host() { assert!(check(1)); }\n",
            ),
            ("b.rs", "fn other() {}\n"),
        ] {
            files_map.insert(
                path.to_string(),
                ParsedFile::parse(path, source, crate::languages::Language::Rust).unwrap(),
            );
        }

        assert!(files.macro_arg_facts.contains_key("a.rs"));
        assert!(!files.macro_arg_facts.contains_key("b.rs"));
        assert_eq!(files.macro_arg_facts["a.rs"].calls_recorded, 1);

        let mut merged = files;
        let changed = BTreeSet::from(["a.rs".to_string()]);
        merged.remove_files(&changed);
        assert!(
            !merged.macro_arg_facts.contains_key("a.rs"),
            "remove_files must retain by file, dropping the removed file's facts"
        );

        merged.merge(CallGraph::build_direct_subset(&files_map, &changed));
        assert!(
            merged.macro_arg_facts.contains_key("a.rs"),
            "merge must extend the map from the rebuilt subset"
        );
        assert_eq!(merged.macro_arg_facts["a.rs"].calls_recorded, 1);
    }

    #[test]
    fn rematerialize_sets_receiver_outcome_and_keeps_calls_callers_in_sync() {
        let cg = build_complete(&[(
            "a.rs",
            "struct Inner; impl Inner { fn poke(&self){} }\n\
             struct Outer { inner: Inner }\n\
             fn run(o: Outer) { let x = o.inner; x.poke(); }\n",
        )]);

        let site = site_in(&cg, "run", "poke");
        assert_eq!(site.receiver_type, None);
        assert_eq!(site.receiver_recovery, None);
        let outcome = site.receiver_outcome.expect("materialized outcome");
        assert!(matches!(
            outcome.key,
            crate::resolution_identity::ReceiverTypeKey::InRepo(_)
        ));
        assert_eq!(
            outcome.recovery,
            crate::resolution::ReceiverRecovery::FieldTyped
        );

        let from_callers = cg
            .callers
            .get("poke")
            .unwrap()
            .iter()
            .find(|site| site.caller.name == "run")
            .unwrap()
            .receiver_outcome
            .clone();
        assert_eq!(Some(outcome), from_callers);
    }

    #[test]
    fn rematerialize_unresolved_receiver_falls_back_to_bare_outcome() {
        let cg = build_complete(&[("a.rs", "fn f(x: Unresolvable) { x.m(); }\n")]);
        let site = site_in(&cg, "f", "m");

        assert_eq!(site.receiver_type.as_deref(), Some("Unresolvable"));
        let outcome = site.receiver_outcome.expect("materialized outcome");
        assert_eq!(
            outcome.key,
            crate::resolution_identity::ReceiverTypeKey::Bare("Unresolvable".to_string())
        );
        assert_eq!(outcome.bare, "Unresolvable");
        assert_eq!(
            outcome.recovery,
            crate::resolution::ReceiverRecovery::TypedParam
        );
    }

    #[test]
    fn rematerialize_generic_receiver_keeps_none_outcome() {
        let cg = build_complete(&[("a.rs", "fn g<U>(x: U) { x.m(); }\n")]);
        let site = site_in(&cg, "g", "m");

        assert_eq!(site.receiver_type.as_deref(), Some("U"));
        assert_eq!(site.receiver_outcome, None);
    }

    #[test]
    fn rematerialize_typed_annotations_use_qualified_identity_not_same_name_decoy() {
        let cg = build_complete(&[(
            "a.rs",
            "mod a { pub struct Inner; impl Inner { pub fn m(&self){} } }\n\
             mod b { pub struct Inner; impl Inner { pub fn m(&self){} } }\n\
             fn f(x: crate::a::Inner) { x.m(); }\n\
             fn l() { let x: crate::a::Inner = crate::a::Inner; x.m(); }\n",
        )]);
        let a_inner = call_graph_type_scope(&cg, "a.rs", "crate::a::Inner");
        let b_inner = call_graph_type_scope(&cg, "a.rs", "crate::b::Inner");
        assert_ne!(a_inner, b_inner);

        let param_outcome = site_in(&cg, "f", "m")
            .receiver_outcome
            .expect("typed-param materialized outcome");
        assert_eq!(
            param_outcome.key,
            crate::resolution_identity::ReceiverTypeKey::InRepo(a_inner)
        );
        assert_ne!(
            param_outcome.key,
            crate::resolution_identity::ReceiverTypeKey::InRepo(b_inner)
        );
        assert_eq!(param_outcome.bare, "Inner");
        assert_eq!(
            param_outcome.recovery,
            crate::resolution::ReceiverRecovery::TypedParam
        );

        let let_outcome = site_in(&cg, "l", "m")
            .receiver_outcome
            .expect("typed-let materialized outcome");
        assert_eq!(
            let_outcome.key,
            crate::resolution_identity::ReceiverTypeKey::InRepo(a_inner)
        );
        assert_ne!(
            let_outcome.key,
            crate::resolution_identity::ReceiverTypeKey::InRepo(b_inner)
        );
        assert_eq!(let_outcome.bare, "Inner");
        assert_eq!(
            let_outcome.recovery,
            crate::resolution::ReceiverRecovery::TypedLet
        );
    }

    #[test]
    fn rematerialize_preserves_call_iteration_order() {
        use crate::ast::ParsedFile;
        use crate::languages::Language::Rust;
        use crate::name_resolution::rust_populator::RustCrateConfig;

        let mut files = std::collections::BTreeMap::new();
        files.insert(
            "a.rs".into(),
            ParsedFile::parse(
                "a.rs",
                "struct Inner; impl Inner { fn poke(&self){} fn tap(&self){} }\n\
                 struct Outer { inner: Inner }\n\
                 fn run(o: Outer) { let x = o.inner; x.poke(); x.tap(); }\n",
                Rust,
            )
            .unwrap(),
        );
        fn order_key(
            site: &CallSite,
        ) -> (
            String,
            String,
            usize,
            CallKind,
            usize,
            usize,
            Option<String>,
            Option<String>,
        ) {
            (
                site.caller.name.clone(),
                site.callee_name.clone(),
                site.line,
                site.kind,
                site.start_byte,
                site.end_byte,
                site.qualifier.clone(),
                site.receiver_type.clone(),
            )
        }
        let only = files.keys().cloned().collect();
        let mut cg = CallGraph::build_direct_subset(&files, &only);
        let before: Vec<_> = cg
            .calls
            .iter()
            .find(|(fid, _)| fid.name == "run")
            .unwrap()
            .1
            .iter()
            .map(order_key)
            .collect();

        let mut inputs = ScopeGraphBuildInputs::from_files_convention(&files);
        inputs.cfg = RustCrateConfig {
            crate_roots: files.keys().cloned().collect(),
            ..RustCrateConfig::default()
        };
        cg.rebuild_scope_graph(&files, Some(&inputs));

        let sites: Vec<_> = cg
            .calls
            .iter()
            .find(|(fid, _)| fid.name == "run")
            .unwrap()
            .1
            .iter()
            .collect();
        let after: Vec<_> = sites.iter().map(|site| order_key(site)).collect();
        assert_eq!(before, after);

        let poke = sites
            .iter()
            .position(|site| site.callee_name == "poke")
            .unwrap();
        let tap = sites
            .iter()
            .position(|site| site.callee_name == "tap")
            .unwrap();
        assert!(poke < tap);
        assert!(sites[poke].receiver_outcome.is_some());
        assert!(sites[tap].receiver_outcome.is_some());
    }

    #[test]
    fn level4_index_matches_legacy_oracle_over_full_universe() {
        // Corpus: quirk fixtures + prism's own top-level src/*.rs sources.
        let mut sources: Vec<(String, String)> = vec![(
            "quirks.c".into(),
            "s.cb = f; t->cb = g;\ns.cb = f; t->cbx = g;\nstatic struct ops o = { .open = do_open, .close = do_close };\na.cb == nope;\nb.cb = &handler;\n".into(),
        )];
        let src_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        for entry in std::fs::read_dir(&src_dir).unwrap().flatten() {
            let p = entry.path();
            if p.extension().and_then(|e| e.to_str()) == Some("rs") {
                sources.push((
                    p.display().to_string(),
                    std::fs::read_to_string(&p).unwrap(),
                ));
            }
        }
        let known: std::collections::BTreeSet<String> = [
            "f", "g", "do_open", "do_close", "handler", "build", "new", "slice", "run",
        ]
        .iter()
        .map(|s| s.to_string())
        .collect();

        // Universe: ALL post-accessor identifiers across the corpus (index-independent)
        // + explicit negatives.
        let mut universe: std::collections::BTreeSet<String> =
            ["no_such_field", "cb", "cbx", "open", "close"]
                .iter()
                .map(|s| s.to_string())
                .collect();
        for (_, src) in &sources {
            for line in src.lines() {
                universe.extend(crate::ast::candidate_fields_on_line(line.trim()));
            }
        }

        // Build the index exactly as CallGraph::build does.
        let mut index: std::collections::BTreeMap<
            String,
            std::collections::BTreeMap<String, std::collections::BTreeSet<String>>,
        > = Default::default();
        for (path, src) in &sources {
            let mut per_field: std::collections::BTreeMap<
                String,
                std::collections::BTreeSet<String>,
            > = Default::default();
            for line in src.lines() {
                let trimmed = line.trim();
                for field in crate::ast::candidate_fields_on_line(trimmed) {
                    let t = per_field.entry(field.clone()).or_default();
                    crate::ast::line_field_targets(trimmed, &field, &known, t);
                }
            }
            for (field, t) in per_field {
                if !t.is_empty() {
                    index.entry(field).or_default().insert(path.clone(), t);
                }
            }
        }

        // Half 1 — EXCESS: every (field, file) the index claims must equal the legacy scan.
        for (field, by_file) in &index {
            for (path, targets) in by_file {
                let src = &sources.iter().find(|(p, _)| p == path).unwrap().1;
                let legacy = crate::ast::resolve_struct_field_assignment(src, field, &known);
                let got: Vec<String> = targets.iter().cloned().collect();
                assert_eq!(got, legacy, "excess: field={field} file={path}");
            }
        }
        // Half 2 — MISSES: per universe field, legacy-scan ONLY files containing the
        // ->field / .field substring (the legacy has_field check hoisted to file level —
        // provably outcome-preserving), and assert the index agrees (absent key == empty).
        for field in &universe {
            let arrow = format!("->{field}");
            let dot = format!(".{field}");
            for (path, src) in &sources {
                if !(src.contains(&arrow) || src.contains(&dot)) {
                    continue; // legacy provably returns empty; index has no entry by construction
                }
                let legacy = crate::ast::resolve_struct_field_assignment(src, field, &known);
                let from_index: Vec<String> = index
                    .get(field)
                    .and_then(|m| m.get(path))
                    .map(|s| s.iter().cloned().collect())
                    .unwrap_or_default();
                assert_eq!(from_index, legacy, "miss: field={field} file={path}");
            }
        }
    }
}

// ---------------------------------------------------------------------------
// P5 S2: Go func-value registration index tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod go_registration_tests {
    use super::*;
    use crate::languages::Language::{Go, C};
    use std::collections::BTreeMap;

    fn build_go(files: &[(&str, &str)]) -> CallGraph {
        let mut map = BTreeMap::new();
        for (path, src) in files {
            map.insert(path.to_string(), ParsedFile::parse(path, src, Go).unwrap());
        }
        CallGraph::build(&map)
    }

    fn fid<'a>(cg: &'a CallGraph, name: &str) -> &'a FunctionId {
        cg.functions
            .get(name)
            .unwrap_or_else(|| panic!("no function named {name}"))
            .first()
            .unwrap()
    }

    #[test]
    fn form_a_composite_literal_field_recorded_with_field_key() {
        let cg = build_go(&[(
            "main.go",
            "package main\n\
type Command struct {\n\tRun func()\n}\n\
func helper() {}\n\
func main() {\n\tc := Command{Run: helper}\n\t_ = c\n}\n",
        )]);
        let target = fid(&cg, "helper").clone();
        let enclosing = fid(&cg, "main").clone();
        let reg = cg
            .go_registrations
            .iter()
            .find(|r| r.target == target && r.enclosing == enclosing)
            .expect("form (a) registration recorded");
        assert!(reg.field_key.is_some(), "known struct+field -> field-keyed");
        assert_eq!(reg.field_key.as_ref().unwrap().1, "Run");
        assert_eq!(reg.site.line, 7); // `c := Command{Run: helper}` line
        assert_eq!(cg.go_registration_unknown_owner_recorded, 0);
    }

    #[test]
    fn form_b_field_assignment_recorded_with_field_key() {
        let cg = build_go(&[(
            "main.go",
            "package main\n\
type Command struct {\n\tRun func()\n}\n\
func helper() {}\n\
func register() {\n\tvar cmd Command\n\tcmd.Run = helper\n}\n",
        )]);
        let target = fid(&cg, "helper").clone();
        let enclosing = fid(&cg, "register").clone();
        let reg = cg
            .go_registrations
            .iter()
            .find(|r| r.target == target && r.enclosing == enclosing)
            .expect("form (b) registration recorded");
        assert!(reg.field_key.is_some());
        assert_eq!(reg.field_key.as_ref().unwrap().1, "Run");
        assert_eq!(cg.go_registration_ambiguous_owner_skips, 0);
    }

    #[test]
    fn form_b_unrecoverable_owner_type_is_skipped_and_counted() {
        // `cmd := getCommand()` — a short var decl whose RHS is a plain call
        // (not the `New*` constructor-name heuristic Go recovery uses), so
        // the owner type is NOT recoverable at all. Form (b) must skip
        // (never fall back the way form (a) does), and count it.
        let cg = build_go(&[(
            "main.go",
            "package main\n\
type Command struct {\n\tRun func()\n}\n\
func helper() {}\n\
func getCommand() Command { return Command{} }\n\
func register() {\n\tcmd := getCommand()\n\tcmd.Run = helper\n}\n",
        )]);
        let target = fid(&cg, "helper").clone();
        assert!(
            !cg.go_registrations.iter().any(|r| r.target == target),
            "unrecoverable owner type must not be recorded"
        );
        assert_eq!(cg.go_registration_ambiguous_owner_skips, 1);
    }

    #[test]
    fn form_b_multiple_bindings_before_assignment_is_ambiguous() {
        // Two `var cmd Command` bindings of the SAME name before the
        // assignment (tree-sitter doesn't enforce Go's real "no redeclare"
        // rule) forces `receiver_type_in_fn`'s existing shadow-bail
        // (`bindings > 1 -> None`) — the owner type is not UNIQUELY
        // recovered, so form (b) must skip and count, not record.
        let cg = build_go(&[(
            "main.go",
            "package main\n\
type Command struct {\n\tRun func()\n}\n\
func helper() {}\n\
func register() {\n\tvar cmd Command\n\tvar cmd Command\n\tcmd.Run = helper\n}\n",
        )]);
        let target = fid(&cg, "helper").clone();
        assert!(
            !cg.go_registrations.iter().any(|r| r.target == target),
            "ambiguous (multiply-bound) owner must not be recorded"
        );
        assert_eq!(cg.go_registration_ambiguous_owner_skips, 1);
    }

    #[test]
    fn form_c_call_argument_recorded_without_field_key() {
        let cg = build_go(&[(
            "main.go",
            "package main\n\
func helper() {}\n\
func Register(f func()) {}\n\
func main() {\n\tRegister(helper)\n}\n",
        )]);
        let target = fid(&cg, "helper").clone();
        let enclosing = fid(&cg, "main").clone();
        let reg = cg
            .go_registrations
            .iter()
            .find(|r| r.target == target && r.enclosing == enclosing)
            .expect("form (c) registration recorded");
        assert!(
            reg.field_key.is_none(),
            "form (c) never carries a field key"
        );
    }

    #[test]
    fn shadowed_identifier_is_skipped_and_counted() {
        // `helper` is locally re-bound inside `main`, so the reference at the
        // call-argument site names the LOCAL, not the free function -> must
        // be skipped, not recorded.
        let cg = build_go(&[(
            "main.go",
            "package main\n\
func helper() {}\n\
func Register(f func()) {}\n\
func main() {\n\thelper := func() {}\n\t_ = helper\n\tRegister(helper)\n}\n",
        )]);
        let target = fid(&cg, "helper").clone();
        assert!(
            !cg.go_registrations.iter().any(|r| r.target == target),
            "shadowed identifier must not be recorded as a registration"
        );
        assert_eq!(cg.go_registration_shadowed_skips, 1);
    }

    #[test]
    fn cross_package_bare_name_is_never_matched() {
        // `helper` exists only in package `other` (a different directory);
        // package `main`'s `Register(helper)` must NOT resolve cross-package
        // by bare name (spec-review MAJOR: never bare cross-package matching).
        // Since `helper` isn't in-repo from `main`'s package/file perspective,
        // no registration should be recorded at all (no false cross-package
        // hit, and no false-negative panic either).
        let cg = build_go(&[
            ("other/helper.go", "package other\nfunc helper() {}\n"),
            (
                "main/main.go",
                "package main\nfunc Register(f func()) {}\nfunc main() {\n\tRegister(helper)\n}\n",
            ),
        ]);
        assert!(
            cg.go_registrations.is_empty(),
            "cross-package bare name must never resolve into a registration"
        );
    }

    #[test]
    fn non_go_files_leave_registration_state_empty() {
        let mut files = BTreeMap::new();
        files.insert(
            "main.c".to_string(),
            ParsedFile::parse(
                "main.c",
                "struct Command { void (*run)(); };\nvoid helper() {}\nvoid main() { struct Command c = { .run = helper }; }\n",
                C,
            )
            .unwrap(),
        );
        let cg = CallGraph::build(&files);
        assert!(cg.go_registrations.is_empty());
        assert_eq!(cg.go_registration_shadowed_skips, 0);
        assert_eq!(cg.go_registration_ambiguous_owner_skips, 0);
        assert_eq!(cg.go_registration_unknown_owner_recorded, 0);
    }
}

#[cfg(test)]
mod go_receiver_typing_tests {
    use super::*;
    use crate::languages::Language::Go;
    use crate::resolution::{ReceiverRecovery, ResolutionConfidence, ResolutionKind};
    use std::collections::BTreeMap;

    fn build_go(files: &[(&str, &str)]) -> CallGraph {
        let mut map = BTreeMap::new();
        for (path, src) in files {
            map.insert(path.to_string(), ParsedFile::parse(path, src, Go).unwrap());
        }
        CallGraph::build(&map)
    }

    fn site_in<'a>(cg: &'a CallGraph, caller: &str, callee: &str) -> &'a CallSite {
        cg.calls
            .iter()
            .find(|(fid, _)| fid.name == caller)
            .and_then(|(_, sites)| sites.iter().find(|s| s.callee_name == callee))
            .unwrap_or_else(|| panic!("no call site {caller} -> {callee}"))
    }

    fn main_owner(name: &str) -> crate::resolution::GoOwnerIdentity {
        crate::resolution::GoOwnerIdentity {
            package_dir: String::new(),
            package_clause: "main".to_string(),
            name: name.to_string(),
        }
    }

    // ---- S1: call-RHS return-typed recovery ----------------------------

    #[test]
    fn s1_call_rhs_bare_name_same_file_recovers_return_typed() {
        let cg = build_go(&[(
            "main.go",
            "package main\n\
             type Demux struct{}\n\
             func (d *Demux) Init(n int) {}\n\
             func newDemux(a, b int) *Demux { return &Demux{} }\n\
             func run() {\n\td := newDemux(16, 16)\n\td.Init(1)\n}\n",
        )]);
        let site = site_in(&cg, "run", "Init");
        assert_eq!(site.receiver_type.as_deref(), Some("Demux"));
        assert_eq!(site.receiver_recovery, Some(ReceiverRecovery::ReturnTyped));
        let out = cg.resolve_call_site_full(site);
        assert!(out.resolved.iter().any(|c| c.target.name == "Init"
            && c.confidence == ResolutionConfidence::Exact
            && c.kind == ResolutionKind::ReturnTyped));
    }

    #[test]
    fn s1_call_rhs_cross_file_same_package_recovers() {
        // The adjudicated etcd shape: the constructor and the receiver
        // methods live in the SAME package (directory) but DIFFERENT files.
        let cg = build_go(&[
            (
                "demux.go",
                "package demux\n\
                 type Demux struct{}\n\
                 func (d *Demux) Init(n int) {}\n\
                 func (d *Demux) Register(w int, id int) {}\n\
                 func newDemux(a, b int) *Demux { return &Demux{} }\n",
            ),
            (
                "demux_test.go",
                "package demux\n\
                 func run() {\n\td := newDemux(16, 16)\n\td.Init(1)\n\td.Register(2, 0)\n}\n",
            ),
        ]);
        let init_site = site_in(&cg, "run", "Init");
        assert_eq!(init_site.receiver_type.as_deref(), Some("Demux"));
        assert_eq!(
            init_site.receiver_recovery,
            Some(ReceiverRecovery::ReturnTyped)
        );
        let reg_site = site_in(&cg, "run", "Register");
        assert_eq!(reg_site.receiver_type.as_deref(), Some("Demux"));
        let out = cg.resolve_call_site_full(reg_site);
        assert!(out
            .resolved
            .iter()
            .any(|c| c.target.name == "Register" && c.confidence == ResolutionConfidence::Exact));
    }

    #[test]
    fn s1_multi_return_beyond_t_error_drops() {
        let cg = build_go(&[(
            "main.go",
            "package main\n\
             type Demux struct{}\n\
             func (d *Demux) Init() {}\n\
             func newDemux() (*Demux, int, error) { return nil, 0, nil }\n\
             func run() {\n\td := newDemux()\n\td.Init()\n}\n",
        )]);
        let site = site_in(&cg, "run", "Init");
        assert_eq!(site.receiver_type, None);
    }

    #[test]
    fn s1_paired_error_return_type_recovers() {
        // Opus impl-review Important 1: the mandated `(T, error)` POSITIVE
        // test — a well-formed 2-tuple return with `error` correctly in the
        // second position must still recover T (pins the take-T branch of
        // `extract_one_return_type`'s `parameter_list` match arm; the
        // existing negative tests only cover the 3-tuple-drop and
        // wrong-position-drop shapes, never this one actually recovering).
        let cg = build_go(&[(
            "main.go",
            "package main\n\
             type Demux struct{}\n\
             func (d *Demux) Init() {}\n\
             func newDemux() (*Demux, error) { return &Demux{}, nil }\n\
             func run() {\n\td, err := newDemux()\n\t_ = err\n\td.Init()\n}\n",
        )]);
        let site = site_in(&cg, "run", "Init");
        assert_eq!(site.receiver_type.as_deref(), Some("Demux"));
        assert_eq!(site.receiver_recovery, Some(ReceiverRecovery::ReturnTyped));
    }

    #[test]
    fn s1_non_first_lhs_position_does_not_recover() {
        // Opus impl-review minor: strengthened to ISOLATE the first-LHS-
        // position guard. `newDemux2`'s return is a WELL-FORMED `(*Demux,
        // error)` pair (error correctly in the second position) -- if it
        // were instead the wrong-order `(error, *Demux)` shape (the
        // original fixture), `extract_one_return_type`'s OWN
        // `second_bare != "error"` gate would already reject it, so the
        // test would keep passing even if the first-LHS-position check were
        // weakened/removed (a false sense of coverage). With a
        // well-formed pair, a weakened guard would flip this to
        // `Some("Demux")`, so the assertion is now tied directly to the
        // position check.
        let cg = build_go(&[(
            "main.go",
            "package main\n\
             type Demux struct{}\n\
             func (d *Demux) Init() {}\n\
             func newDemux2() (*Demux, error) { return nil, nil }\n\
             func run() {\n\terr, d := newDemux2()\n\t_ = err\n\td.Init()\n}\n",
        )]);
        // `d` is bound at LHS position 1 (not first) -> S1 must not recover it,
        // regardless of what the callee returns.
        let site = site_in(&cg, "run", "Init");
        assert_eq!(site.receiver_type, None);
    }

    #[test]
    fn s1_generic_function_return_drops() {
        let cg = build_go(&[(
            "main.go",
            "package main\n\
             type Demux struct{}\n\
             func (d *Demux) Init() {}\n\
             func newDemux[T any]() *Demux { return &Demux{} }\n\
             func run() {\n\td := newDemux[int]()\n\td.Init()\n}\n",
        )]);
        let site = site_in(&cg, "run", "Init");
        assert_eq!(site.receiver_type, None);
    }

    #[test]
    fn s1_import_qualified_call_rhs_resolves_via_package_dir() {
        let cg = build_go(&[
            (
                "factory/factory.go",
                "package factory\n\
                 type Widget struct{}\n\
                 func (w *Widget) Use() {}\n\
                 func New() *Widget { return &Widget{} }\n",
            ),
            (
                "main.go",
                "package main\n\
                 import \"example.com/repo/factory\"\n\
                 func run() {\n\tw := factory.New()\n\tw.Use()\n}\n",
            ),
        ]);
        let site = site_in(&cg, "run", "Use");
        assert_eq!(site.receiver_type.as_deref(), Some("Widget"));
        assert_eq!(site.receiver_recovery, Some(ReceiverRecovery::ReturnTyped));
    }

    #[test]
    fn s1_cross_package_same_name_constructor_pins_own_package_type() {
        // Two UNRELATED packages each declare a bare (same-name, unqualified)
        // `New()` constructor returning their OWN package's type. S1's index
        // is keyed by clause-bearing package/function identity
        // (`extract_go_return_types`),
        // so this must not collide -- each package's `x := New()` recovers
        // to ITS OWN type, never the other's.
        let cg = build_go(&[
            (
                "pkg1/pkg1.go",
                "package pkg1\n\
                 type T1 struct{}\n\
                 func (t *T1) M() {}\n\
                 func New() *T1 { return &T1{} }\n\
                 func run1() {\n\tx := New()\n\tx.M()\n}\n",
            ),
            (
                "pkg2/pkg2.go",
                "package pkg2\n\
                 type T2 struct{}\n\
                 func (t *T2) M() {}\n\
                 func New() *T2 { return &T2{} }\n\
                 func run2() {\n\tx := New()\n\tx.M()\n}\n",
            ),
        ]);
        let site1 = site_in(&cg, "run1", "M");
        assert_eq!(site1.receiver_type.as_deref(), Some("T1"));
        let site2 = site_in(&cg, "run2", "M");
        assert_eq!(site2.receiver_type.as_deref(), Some("T2"));
    }

    // ---- S2: nested-selector field-typed recovery -----------------------

    #[test]
    fn s2_one_hop_field_selector_recovers_field_typed() {
        let cg = build_go(&[(
            "main.go",
            "package main\n\
             type Inner struct{}\n\
             func (i *Inner) M() {}\n\
             type Outer struct {\n\tField *Inner\n}\n\
             func run(o *Outer) {\n\to.Field.M()\n}\n",
        )]);
        let site = site_in(&cg, "run", "M");
        assert_eq!(site.receiver_type.as_deref(), Some("Inner"));
        assert_eq!(site.receiver_recovery, Some(ReceiverRecovery::FieldTyped));
        let out = cg.resolve_call_site_full(site);
        assert!(out.resolved.iter().any(|c| c.target.name == "M"
            && c.confidence == ResolutionConfidence::Exact
            && c.kind == ResolutionKind::FieldTyped));
    }

    #[test]
    fn s2_two_hop_field_selector_recovers() {
        let cg = build_go(&[(
            "main.go",
            "package main\n\
             type Innermost struct{}\n\
             func (i *Innermost) M() {}\n\
             type Middle struct {\n\tLeaf *Innermost\n}\n\
             type Outer struct {\n\tMid *Middle\n}\n\
             func run(o *Outer) {\n\to.Mid.Leaf.M()\n}\n",
        )]);
        let site = site_in(&cg, "run", "M");
        assert_eq!(site.receiver_type.as_deref(), Some("Innermost"));
        assert_eq!(site.receiver_recovery, Some(ReceiverRecovery::FieldTyped));
    }

    #[test]
    fn s2_three_hop_field_selector_drops() {
        let cg = build_go(&[(
            "main.go",
            "package main\n\
             type L4 struct{}\n\
             func (l *L4) M() {}\n\
             type L3 struct {\n\tNext *L4\n}\n\
             type L2 struct {\n\tNext *L3\n}\n\
             type L1 struct {\n\tNext *L2\n}\n\
             func run(o *L1) {\n\to.Next.Next.Next.M()\n}\n",
        )]);
        let site = site_in(&cg, "run", "M");
        assert_eq!(site.receiver_type, None, "3-hop chain must not recover");
    }

    #[test]
    fn s2_embedded_field_selector_recovers() {
        // A TRUE anonymous/embedded field (`Listener`, no field name,
        // non-pointer) — its implicit field name is the bare type name
        // `Listener` (`GoStruct::extract_one_field`), so
        // `w.Listener.Accept()` is the correct accessor shape (mirrors the
        // adjudicated caddy `l.Listener.Accept()` case, but with an IN-REPO
        // `Listener` so this one DOES flip to a FieldTyped Exact edge).
        //
        // Non-pointer embedding deliberately, not `*Listener`: grounding
        // (tree-sitter dump) found `extract_one_field` does not recognize a
        // POINTER-embedded field at all (tree-sitter-go represents it as a
        // bare `*` token sibling, not a `pointer_type` wrapper it expects) —
        // a PRE-EXISTING gap in the already-shipped struct-embedding
        // extraction, untouched by P11 and out of scope here; see the task
        // report.
        let cg = build_go(&[(
            "main.go",
            "package main\n\
             type Listener struct{}\n\
             func (l *Listener) Accept() {}\n\
             type Wrap struct {\n\tListener\n}\n\
             func run(w *Wrap) {\n\tw.Listener.Accept()\n}\n",
        )]);
        let site = site_in(&cg, "run", "Accept");
        assert_eq!(site.receiver_type.as_deref(), Some("Listener"));
        assert_eq!(site.receiver_recovery, Some(ReceiverRecovery::FieldTyped));
    }

    #[test]
    fn s2_field_miss_at_any_hop_drops_entirely() {
        let cg = build_go(&[(
            "main.go",
            "package main\n\
             type Outer struct {\n\tField int\n}\n\
             func run(o *Outer) {\n\to.Missing.M()\n}\n",
        )]);
        // `Missing` is not a field of `Outer` at all -- the field_types
        // lookup at this hop misses, so the whole chain must drop (no
        // partial recovery), not panic.
        let site = site_in(&cg, "run", "M");
        assert_eq!(site.receiver_type, None);
    }

    #[test]
    fn s2_closure_param_shadowing_base_does_not_leak_stale_recovery() {
        let cg = build_go(&[(
            "main.go",
            "package main\n\
             type Inner struct{}\n\
             func (i *Inner) M() {}\n\
             type Outer struct {\n\tField *Inner\n}\n\
             func run(o *Outer) {\n\
             \tfn := func(o int) {\n\t\t_ = o\n\t}\n\
             \tfn(1)\n\
             \to.Field.M()\n\
             }\n",
        )]);
        // The closure param `o int` does NOT shadow the OUTER `o.Field.M()`
        // call (it's a sibling statement, not enclosing it) -- must still
        // recover normally. This is the companion positive case to the
        // fail-closed shadow test below (proves the closure fix doesn't
        // over-suppress unrelated recoveries).
        let site = site_in(&cg, "run", "M");
        assert_eq!(site.receiver_type.as_deref(), Some("Inner"));
    }

    #[test]
    fn s2_closure_param_rebinding_base_inside_closure_fails_closed() {
        let cg = build_go(&[(
            "main.go",
            "package main\n\
             type Inner struct{}\n\
             func (i *Inner) M() {}\n\
             type Other struct{}\n\
             func (x Other) M() {}\n\
             type Outer struct {\n\tO *Inner\n}\n\
             func run(o *Outer) {\n\
             \tfn := func(o Other) {\n\t\to.M()\n\t}\n\
             \t_ = fn\n\
             }\n",
        )]);
        // The call site `o.M()` is INSIDE the closure, whose OWN parameter
        // `o Other` shadows the outer `o *Outer`. Without the closure-shadow
        // fix, the outer-function walk would see only the OUTER `o` binding
        // (the closure's own param is invisible to `function_node_types()`)
        // and could leak a stale/wrong R1/R2 proof. P17 retains the first type
        // only as the legacy R3 input, and marks that proof shadowed so it
        // cannot mint a direct edge against the outer declaration.
        let site = site_in(&cg, "run", "M");
        assert_eq!(site.receiver_type.as_deref(), Some("Outer"));
        assert!(site.receiver_local_type_shadowed);
        assert!(cg.resolve_call_site(site).is_empty());
    }

    // ---- B1 (codex impl-review BLOCKER): func_literal lexical-scope fence -

    #[test]
    fn b1_s1_sibling_closure_short_var_does_not_leak_across_lexical_scope() {
        let cg = build_go(&[(
            "main.go",
            "package main\n\
             type Demux struct{}\n\
             func (d *Demux) Init(n int) {}\n\
             func newDemux(a, b int) *Demux { return &Demux{} }\n\
             func run() {\n\
             \tfn := func() {\n\t\td := newDemux(16, 16)\n\t\t_ = d\n\t}\n\
             \t_ = fn\n\
             \td.Init(1)\n\
             }\n",
        )]);
        // `d` in `run`'s OWN scope has no binding at all -- only the SIBLING
        // closure's `d := newDemux(...)` does, and that closure does not
        // contain this call. Without the lexical-scope fence, the walk
        // descends into the sibling closure anyway and finds `d`'s binding
        // there, minting a false `ReturnTyped` recovery for an unrelated
        // (undefined-here) `d`. Must NOT recover.
        let site = site_in(&cg, "run", "Init");
        assert_eq!(site.receiver_type, None);
    }

    #[test]
    fn b1_s1_call_inside_closure_recovers_from_its_own_binding() {
        // Positive control: the call IS inside the closure, so the closure's
        // OWN `d := newDemux(...)` binding is genuinely in scope and must
        // still recover (proves the fence doesn't over-suppress).
        let cg = build_go(&[(
            "main.go",
            "package main\n\
             type Demux struct{}\n\
             func (d *Demux) Init(n int) {}\n\
             func newDemux(a, b int) *Demux { return &Demux{} }\n\
             func run() {\n\
             \tfn := func() {\n\t\td := newDemux(16, 16)\n\t\td.Init(1)\n\t}\n\
             \tfn()\n\
             }\n",
        )]);
        let site = site_in(&cg, "run", "Init");
        assert_eq!(site.receiver_type.as_deref(), Some("Demux"));
        assert_eq!(site.receiver_recovery, Some(ReceiverRecovery::ReturnTyped));
    }

    #[test]
    fn b1_s2_sibling_closure_base_short_var_does_not_leak_across_lexical_scope() {
        let cg = build_go(&[(
            "main.go",
            "package main\n\
             type Inner struct{}\n\
             func (i *Inner) M() {}\n\
             type Outer struct {\n\tField *Inner\n}\n\
             func newOuter() *Outer { return &Outer{} }\n\
             func run() {\n\
             \tfn := func() {\n\t\to := newOuter()\n\t\t_ = o\n\t}\n\
             \t_ = fn\n\
             \to.Field.M()\n\
             }\n",
        )]);
        // Same lexical-scope leak, via S2's base recovery (which recurses
        // through the SAME S1 machinery for the base identifier `o`).
        let site = site_in(&cg, "run", "M");
        assert_eq!(site.receiver_type, None);
    }

    #[test]
    fn b1_real_outer_binding_survives_sibling_closure_same_name_binding() {
        // Pins the 903->939 anomaly mechanism (codex re-review MINOR): pre-fix,
        // `walk_receiver_bindings`'s unfenced `func_literal` arm counted the
        // sibling closure's OWN `d := other()` binding as a SECOND binding of
        // `d` in `run`'s scope (on top of the genuine outer `d :=
        // newDemux(...)`), inflating `bindings` from the correct 1 to 2.
        // `go_receiver_index.rs`'s partition-aware classifier then bails
        // at its `if bindings > 1 { return baseline; }` gate (~line 389)
        // BEFORE ever attempting the S1 call-RHS retry -- so the outer,
        // wholly unambiguous `d.Init(1)` lost its recovery entirely, not just
        // a false one. Post-fix (B1 lexical-scope fence), the sibling
        // closure's subtree is skipped outright, `bindings` is correctly 1,
        // and the S1 call-RHS retry recovers `Demux`/`ReturnTyped` as it
        // should.
        let cg = build_go(&[(
            "main.go",
            "package main\n\
             type Demux struct{}\n\
             func (d *Demux) Init(n int) {}\n\
             func newDemux(a, b int) *Demux { return &Demux{} }\n\
             func other() *Demux { return &Demux{} }\n\
             func run() {\n\
             \td := newDemux(16, 16)\n\
             \tgo func() {\n\t\td := other()\n\t\t_ = d\n\t}()\n\
             \td.Init(1)\n\
             }\n",
        )]);
        let site = site_in(&cg, "run", "Init");
        assert_eq!(
            site.receiver_type.as_deref(),
            Some("Demux"),
            "the real outer `d := newDemux(...)` binding must recover \
             despite the sibling closure's own same-name `d` binding: {:?}",
            site
        );
        assert_eq!(site.receiver_recovery, Some(ReceiverRecovery::ReturnTyped));
    }

    // ---- S3: package-level var receivers ---------------------------------

    #[test]
    fn s3_package_level_var_recovers_var_decl() {
        let cg = build_go(&[(
            "main.go",
            "package main\n\
             type Runner interface { Go() }\n\
             type Impl struct{}\n\
             func (i Impl) Go() {}\n\
             var r Runner\n\
             func run() {\n\tr.Go()\n}\n",
        )]);
        let site = site_in(&cg, "run", "Go");
        assert_eq!(site.receiver_type.as_deref(), Some("Runner"));
        assert_eq!(site.receiver_recovery, Some(ReceiverRecovery::VarDecl));
        let out = cg.resolve_call_site_full(site);
        assert!(out
            .resolved
            .iter()
            .any(|c| c.target.name == "Go" && c.confidence == ResolutionConfidence::Exact));
    }

    #[test]
    fn s3_function_local_binding_shadows_package_var() {
        let cg = build_go(&[(
            "main.go",
            "package main\n\
             type Runner interface { Go() }\n\
             type Impl struct{}\n\
             func (i Impl) Go() {}\n\
             type Other struct{}\n\
             var r Runner\n\
             func run() {\n\tr := Other{}\n\t_ = r\n}\n\
             func run2() {\n\tr.Go()\n}\n",
        )]);
        // `run2` has no local `r` -- package var recovers.
        let site2 = site_in(&cg, "run2", "Go");
        assert_eq!(site2.receiver_type.as_deref(), Some("Runner"));
    }

    #[test]
    fn s3_same_function_local_binding_shadows_package_var() {
        // Minor (both reviews): the test above isn't a REAL shadow test --
        // `run2` has no local `r` binding AT ALL, so it only proves S3
        // recovers in the ABSENCE of a local, not that a local correctly
        // shadows the package var. This is the real same-function case: `r`
        // is bound locally IN THE SAME function that calls `r.Go()` -- the
        // local (`Other`) must win, never falling back to the package var's
        // `Runner` type.
        let cg = build_go(&[(
            "main.go",
            "package main\n\
             type Runner interface { Go() }\n\
             type Impl struct{}\n\
             func (i Impl) Go() {}\n\
             type Other struct{}\n\
             func (o Other) Go() {}\n\
             var r Runner\n\
             func run() {\n\tr := Other{}\n\tr.Go()\n}\n",
        )]);
        let site = site_in(&cg, "run", "Go");
        assert_eq!(
            site.receiver_type.as_deref(),
            Some("Other"),
            "local `r := Other{{}}` must shadow the package `var r Runner`"
        );
    }

    #[test]
    fn s3_package_var_cross_file_in_same_package() {
        let cg = build_go(&[
            (
                "vars.go",
                "package main\n\
                 type Runner interface { Go() }\n\
                 var r Runner\n",
            ),
            (
                "impl.go",
                "package main\n\
                 type Impl struct{}\n\
                 func (i Impl) Go() {}\n\
                 func run() {\n\tr.Go()\n}\n",
            ),
        ]);
        let site = site_in(&cg, "run", "Go");
        assert_eq!(site.receiver_type.as_deref(), Some("Runner"));
    }

    // ---- S4: embedded-interface satisfaction/routing ---------------------

    #[test]
    fn s4_struct_embeds_interface_routes_to_concrete_implementer() {
        let cg = build_go(&[(
            "main.go",
            "package main\n\
             type Doer interface { Do() }\n\
             type Concrete struct{}\n\
             func (c Concrete) Do() {}\n\
             type Holder struct {\n\tDoer\n}\n\
             func run(h Holder) {\n\th.Do()\n}\n",
        )]);
        let site = site_in(&cg, "run", "Do");
        let out = cg.resolve_call_site_full(site);
        assert!(
            out.resolved.iter().any(|c| c.target.name == "Do"
                && c.confidence == ResolutionConfidence::Exact
                && c.kind == ResolutionKind::InterfaceDispatch),
            "expected Exact InterfaceDispatch via embedded interface, got {:?}",
            out
        );
        assert_eq!(
            cg.go_embedded_interface_route("Holder", None, "Do", "main.go")
                .value
                .as_deref(),
            Some("Doer")
        );
    }

    #[test]
    fn s2_pointer_embedded_field_recovers_nested_selector_receiver() {
        let cg = build_go(&[(
            "main.go",
            "package main\n\
             type Listener struct{}\n\
             func (l *Listener) Serve() {}\n\
             type Holder struct { *Listener }\n\
             func run(h Holder) { h.Listener.Serve() }\n",
        )]);
        let site = site_in(&cg, "run", "Serve");
        assert_eq!(site.receiver_type.as_deref(), Some("Listener"));
        assert_eq!(site.receiver_recovery, Some(ReceiverRecovery::FieldTyped));
        let out = cg.resolve_call_site_full(site);
        assert!(out.resolved.iter().any(|candidate| {
            candidate.target.name == "Serve"
                && candidate.confidence == ResolutionConfidence::Exact
                && candidate.kind == ResolutionKind::FieldTyped
        }));
    }

    #[test]
    fn s2_pointer_embedded_field_routes_only_to_its_proven_package() {
        let cg = build_go(&[
            (
                "a/types.go",
                "package a\n\
                 type Listener struct{}\n\
                 func (*Listener) Serve() {}\n\
                 type Holder struct { *Listener }\n\
                 func run(h Holder) { h.Listener.Serve() }\n",
            ),
            (
                "b/types.go",
                "package b\n\
                 type Listener struct{}\n\
                 func (*Listener) Serve() {}\n",
            ),
        ]);
        let site = site_in(&cg, "run", "Serve");
        assert_eq!(site.receiver_type.as_deref(), Some("Listener"));
        assert_eq!(site.receiver_recovery, Some(ReceiverRecovery::FieldTyped));
        assert!(
            site.receiver_outcome.is_some(),
            "target proof was not carried"
        );
        let round_tripped: CallSite =
            bincode::deserialize(&bincode::serialize(site).unwrap()).unwrap();
        assert_eq!(round_tripped.receiver_outcome, site.receiver_outcome);

        let out = cg.resolve_call_site_full(&round_tripped);
        assert_eq!(out.resolved.len(), 1, "package decoy leaked: {out:?}");
        assert_eq!(out.resolved[0].target.file, "a/types.go");
        assert_eq!(out.resolved[0].confidence, ResolutionConfidence::Exact);
        assert_eq!(out.resolved[0].kind, ResolutionKind::FieldTyped);
    }

    #[test]
    fn s2_value_embedded_field_routes_only_to_its_proven_package() {
        let cg = build_go(&[
            (
                "a/types.go",
                "package a\n\
                 type Listener struct{}\n\
                 func (Listener) Serve() {}\n\
                 type Holder struct { Listener }\n\
                 func run(h Holder) { h.Listener.Serve() }\n",
            ),
            (
                "b/types.go",
                "package b\n\
                 type Listener struct{}\n\
                 func (Listener) Serve() {}\n",
            ),
        ]);
        let site = site_in(&cg, "run", "Serve");
        assert_eq!(site.receiver_type.as_deref(), Some("Listener"));
        assert_eq!(site.receiver_recovery, Some(ReceiverRecovery::FieldTyped));

        let out = cg.resolve_call_site_full(site);
        assert_eq!(out.resolved.len(), 1, "package decoy leaked: {out:?}");
        assert_eq!(out.resolved[0].target.file, "a/types.go");
        assert_eq!(out.resolved[0].confidence, ResolutionConfidence::Exact);
        assert_eq!(out.resolved[0].kind, ResolutionKind::FieldTyped);
    }

    #[test]
    fn s2_embedded_field_drops_incompatible_target_method_profile() {
        let cg = build_go(&[
            (
                "a/types_linux.go",
                "package a\n\
                 type Listener struct{}\n\
                 type Holder struct { *Listener }\n\
                 func run(h Holder) { h.Listener.Serve() }\n",
            ),
            (
                "a/method_darwin.go",
                "package a\nfunc (*Listener) Serve() {}\n",
            ),
        ]);
        let site = site_in(&cg, "run", "Serve");
        assert_eq!(site.receiver_recovery, Some(ReceiverRecovery::FieldTyped));
        let out = cg.resolve_call_site_full(site);
        assert!(
            out.resolved.is_empty(),
            "darwin-only method leaked into linux target route: {out:?}"
        );
    }

    #[test]
    fn s2_embedded_field_drops_external_test_package_method_candidate() {
        let cg = build_go(&[
            (
                "p/types.go",
                "package p\n\
                 type Listener struct{}\n\
                 type Holder struct { *Listener }\n\
                 func run(h Holder) { h.Listener.Serve() }\n",
            ),
            (
                "p/method_test.go",
                "package p_test\nfunc (*Listener) Serve() {}\n",
            ),
        ]);
        let site = site_in(&cg, "run", "Serve");
        assert_eq!(site.receiver_recovery, Some(ReceiverRecovery::FieldTyped));
        let out = cg.resolve_call_site_full(site);
        assert!(
            out.resolved.is_empty(),
            "p_test method leaked into package p target route: {out:?}"
        );
    }

    #[test]
    fn s2_embedded_field_drops_conflicting_compatible_target_methods() {
        let cg = build_go(&[
            (
                "a/types.go",
                "package a\n\
                 type Listener struct{}\n\
                 type Holder struct { Listener }\n\
                 func run(h Holder) { h.Listener.Serve() }\n",
            ),
            ("a/method_one.go", "package a\nfunc (Listener) Serve() {}\n"),
            ("a/method_two.go", "package a\nfunc (Listener) Serve() {}\n"),
        ]);
        let site = site_in(&cg, "run", "Serve");
        assert_eq!(site.receiver_recovery, Some(ReceiverRecovery::FieldTyped));
        let out = cg.resolve_call_site_full(site);
        assert!(
            out.resolved.is_empty(),
            "conflicting compatible methods must fail closed: {out:?}"
        );
    }

    #[test]
    fn s2_pointer_embed_struct_in_external_test_package_never_recovers() {
        let cg = build_go(&[
            (
                "p/holder.go",
                "package p\n\
                 type Holder struct { *Listener }\n\
                 func run(h Holder) { h.Listener.Do() }\n",
            ),
            (
                "p/listener_test.go",
                "package p_test\n\
                 type Listener struct{}\n\
                 func (*Listener) Do() {}\n",
            ),
        ]);
        let site = site_in(&cg, "run", "Do");
        assert_eq!(site.receiver_type, None);
        assert_eq!(site.receiver_recovery, None);
        let out = cg.resolve_call_site_full(site);
        assert!(
            out.resolved
                .iter()
                .all(|candidate| candidate.confidence != ResolutionConfidence::Exact),
            "a p_test struct must not prove p.Holder's pointer embed or emit Exact: {out:?}"
        );
    }

    #[test]
    fn s2_pointer_embed_struct_in_incompatible_build_profile_never_recovers() {
        let cg = build_go(&[
            (
                "p/holder_linux.go",
                "package p\n\
                 type Holder struct { *Listener }\n\
                 func run(h Holder) { h.Listener.Do() }\n",
            ),
            (
                "p/listener_darwin.go",
                "package p\n\
                 type Listener struct{}\n\
                 func (*Listener) Do() {}\n",
            ),
        ]);
        let site = site_in(&cg, "run", "Do");
        assert_eq!(site.receiver_type, None);
        assert_eq!(site.receiver_recovery, None);
        let out = cg.resolve_call_site_full(site);
        assert!(
            out.resolved
                .iter()
                .all(|candidate| candidate.confidence != ResolutionConfidence::Exact),
            "a darwin-only struct must not prove a linux holder's pointer embed: {out:?}"
        );
    }

    #[test]
    fn s2_pointer_embed_struct_with_uncertain_embedding_profile_never_recovers() {
        let cg = build_go(&[
            (
                "p/holder.go",
                "//go:build linux &&\n\n\
                 package p\n\
                 type Holder struct { *Listener }\n\
                 func run(h Holder) { h.Listener.Do() }\n",
            ),
            (
                "p/listener.go",
                "package p\n\
                 type Listener struct{}\n\
                 func (*Listener) Do() {}\n",
            ),
        ]);
        let site = site_in(&cg, "run", "Do");
        assert_eq!(site.receiver_type, None);
        assert_eq!(site.receiver_recovery, None);
        assert!(
            cg.resolve_call_site_full(site)
                .resolved
                .iter()
                .all(|candidate| candidate.confidence != ResolutionConfidence::Exact),
            "an unparsed embedding profile cannot prove pointer-target visibility"
        );
    }

    #[test]
    fn s2_pointer_embed_struct_in_same_build_profile_still_recovers() {
        let cg = build_go(&[
            (
                "p/holder_linux.go",
                "package p\n\
                 type Holder struct { *Listener }\n\
                 func run(h Holder) { h.Listener.Do() }\n",
            ),
            (
                "p/listener_linux.go",
                "package p\n\
                 type Listener struct{}\n\
                 func (*Listener) Do() {}\n",
            ),
        ]);
        let site = site_in(&cg, "run", "Do");
        assert_eq!(site.receiver_type.as_deref(), Some("Listener"));
        assert_eq!(site.receiver_recovery, Some(ReceiverRecovery::FieldTyped));
        assert!(cg
            .resolve_call_site_full(site)
            .resolved
            .iter()
            .any(|candidate| {
                candidate.target.name == "Do"
                    && candidate.confidence == ResolutionConfidence::Exact
                    && candidate.kind == ResolutionKind::FieldTyped
            }));
    }

    #[test]
    fn s2_pointer_embed_unconstrained_cross_file_struct_still_recovers() {
        let cg = build_go(&[
            (
                "p/holder.go",
                "package p\n\
                 type Holder struct { *Listener }\n\
                 func run(h Holder) { h.Listener.Do() }\n",
            ),
            (
                "p/listener.go",
                "package p\n\
                 type Listener struct{}\n\
                 func (*Listener) Do() {}\n",
            ),
        ]);
        let site = site_in(&cg, "run", "Do");
        assert_eq!(site.receiver_type.as_deref(), Some("Listener"));
        assert_eq!(site.receiver_recovery, Some(ReceiverRecovery::FieldTyped));
        assert!(cg
            .resolve_call_site_full(site)
            .resolved
            .iter()
            .any(|candidate| {
                candidate.target.name == "Do"
                    && candidate.confidence == ResolutionConfidence::Exact
                    && candidate.kind == ResolutionKind::FieldTyped
            }));
    }

    #[test]
    fn s2_pointer_embed_external_test_clause_does_not_conflict_with_owner() {
        let cg = build_go(&[
            (
                "p/holder.go",
                "package p\n\
                 type Holder struct { *Listener }\n\
                 func run(h Holder) { h.Listener.Do() }\n",
            ),
            (
                "p/listener.go",
                "package p\n\
                 type Listener struct{}\n\
                 func (*Listener) Do() {}\n",
            ),
            (
                "p/listener_test.go",
                "package p_test\n\
                 type Listener interface { Do() }\n",
            ),
        ]);
        let site = site_in(&cg, "run", "Do");
        assert_eq!(site.receiver_type.as_deref(), Some("Listener"));
        assert_eq!(site.receiver_recovery, Some(ReceiverRecovery::FieldTyped));
        let outcome = cg.resolve_call_site_full(site);
        assert!(
            outcome.resolved.iter().any(|candidate| {
                candidate.target.file == "p/listener.go"
                    && candidate.target.name == "Do"
                    && candidate.confidence == ResolutionConfidence::Exact
                    && candidate.kind == ResolutionKind::FieldTyped
            }),
            "the p_test clause must not suppress the proven p.Listener target: {outcome:?}"
        );
        assert!(
            outcome
                .resolved
                .iter()
                .all(|candidate| candidate.target.file != "p/listener_test.go"),
            "the external-test clause must never donate the pointer-embed target: {outcome:?}"
        );
    }

    #[test]
    fn s2_pointer_embed_interface_alias_collision_never_recovers() {
        let cg = build_go(&[
            (
                "a/a.go",
                "package a\n\
                 type Alias = interface { Do() }\n\
                 type Holder struct { *Alias }\n\
                 func run(h Holder) { h.Alias.Do() }\n",
            ),
            (
                "z/z.go",
                "package z\n\
                 type Alias interface { Do() }\n\
                 type Impl struct{}\n\
                 func (Impl) Do() {}\n",
            ),
        ]);
        let site = site_in(&cg, "run", "Do");
        assert_eq!(site.receiver_type, None);
        assert_eq!(site.receiver_recovery, None);
        let out = cg.resolve_call_site_full(site);
        assert!(
            out.resolved
                .iter()
                .all(|candidate| candidate.confidence != ResolutionConfidence::Exact),
            "an invalid pointer-to-interface alias must not emit Exact z.Impl.Do: {out:?}"
        );
    }

    #[test]
    fn s2_pointer_embed_struct_alias_fails_closed() {
        let cg = build_go(&[(
            "main.go",
            "package main\n\
             type Listener struct{}\n\
             func (*Listener) Serve() {}\n\
             type Alias = Listener\n\
             type Holder struct { *Alias }\n\
             func run(h Holder) { h.Alias.Serve() }\n",
        )]);
        let site = site_in(&cg, "run", "Serve");
        assert_eq!(site.receiver_type, None);
        assert_eq!(site.receiver_recovery, None);
        assert!(
            cg.resolve_call_site_full(site)
                .resolved
                .iter()
                .all(|candidate| candidate.confidence != ResolutionConfidence::Exact),
            "struct aliases intentionally fail closed for S2"
        );
    }

    #[test]
    fn s2_pointer_embed_defined_interface_collision_never_recovers() {
        let cg = build_go(&[
            (
                "a/a.go",
                "package a\n\
                 type I interface { Do() }\n\
                 type I2 I\n\
                 type Holder struct { *I2 }\n\
                 func run(h Holder) { h.I2.Do() }\n",
            ),
            (
                "z/z.go",
                "package z\n\
                 type I2 interface { Do() }\n\
                 type Impl struct{}\n\
                 func (Impl) Do() {}\n",
            ),
        ]);
        let site = site_in(&cg, "run", "Do");
        assert_eq!(site.receiver_type, None);
        assert_eq!(site.receiver_recovery, None);
        let out = cg.resolve_call_site_full(site);
        assert!(
            out.resolved
                .iter()
                .all(|candidate| candidate.confidence != ResolutionConfidence::Exact),
            "a defined interface type must not emit an Exact edge through z.I2: {out:?}"
        );
    }

    #[test]
    fn cross_package_unqualified_embed_never_mints_embedded_promotion() {
        let cg = build_go(&[
            (
                "a/types.go",
                "package a\n\
                 type Base struct{}\n\
                 type S struct { *Base }\n",
            ),
            (
                "a/run.go",
                "package a\n\
                 func run(s S) { s.Serve() }\n",
            ),
            (
                "b/types.go",
                "package b\n\
                 type Base struct{}\n\
                 func (b *Base) Serve() {}\n",
            ),
        ]);
        assert!(
            cg.resolve_call_site_full(site_in(&cg, "run", "Serve"))
                .resolved
                .iter()
                .all(|candidate| candidate.kind != ResolutionKind::EmbeddedPromotion),
            "a.S must not promote b.(*Base).Serve"
        );
    }

    #[test]
    fn s2_unrelated_package_interface_does_not_remove_local_pointer_embed() {
        let cg = build_go(&[
            (
                "a/types.go",
                "package a\n\
                 type Base struct{}\n\
                 func (b *Base) Serve() {}\n\
                 type S struct { *Base }\n",
            ),
            (
                "a/run.go",
                "package a\n\
                 func run(s S) { s.Base.Serve() }\n",
            ),
            ("z/types.go", "package z\ntype Base interface { Serve() }\n"),
        ]);
        let site = site_in(&cg, "run", "Serve");
        assert_eq!(site.receiver_type.as_deref(), Some("Base"));
        assert_eq!(site.receiver_recovery, Some(ReceiverRecovery::FieldTyped));
        assert!(cg
            .resolve_call_site_full(site)
            .resolved
            .iter()
            .any(|candidate| {
                candidate.target.name == "Serve"
                    && candidate.confidence == ResolutionConfidence::Exact
                    && candidate.kind == ResolutionKind::FieldTyped
            }));
    }

    #[test]
    fn qualified_pointer_embed_field_shadows_concrete_promotion() {
        let cg = build_go(&[
            ("ext/types.go", "package ext\ntype Listener struct{}\n"),
            (
                "main.go",
                "package main\n\
                 import \"example.com/ext\"\n\
                 type D struct{}\n\
                 func (d D) Listener() {}\n\
                 type S struct { *ext.Listener; D }\n\
                 func run(s S) { s.Listener() }\n",
            ),
        ]);
        let out = cg.resolve_call_site_full(site_in(&cg, "run", "Listener"));
        assert!(
            out.resolved.is_empty(),
            "the depth-0 ext.Listener field must shadow promoted D.Listener: {out:?}"
        );
        assert!(!cg
            .promoted_aliases
            .contains_key(&("S".to_string(), "Listener".to_string())));
    }

    #[test]
    fn duplicate_outer_struct_name_never_cross_routes_promoted_alias() {
        let cg = build_go(&[
            (
                "a/types.go",
                "package a\n\
                 type I interface { Serve(x int) }\n\
                 type Concrete struct{}\n\
                 func (c Concrete) Serve(x int) {}\n\
                 type S struct { I }\n",
            ),
            ("a/run.go", "package a\nfunc run(s S) { s.Serve(1) }\n"),
            (
                "z/types.go",
                "package z\n\
                 type Base struct{}\n\
                 func (b *Base) Serve(x string) {}\n\
                 type S struct { *Base }\n",
            ),
        ]);
        let out = cg.resolve_call_site_full(site_in(&cg, "run", "Serve"));
        assert!(
            out.resolved.is_empty()
                || out.resolved.iter().all(|candidate| {
                    candidate.target.file == "a/types.go"
                        && candidate.target.name == "Serve"
                        && candidate.kind == ResolutionKind::InterfaceDispatch
                }),
            "a.S may route through its own I or fail closed, but must never reach z.Base.Serve: \
             {out:?}"
        );
    }

    #[test]
    fn s4_pointer_embedded_interface_never_routes_or_satisfies() {
        let cg = build_go(&[(
            "main.go",
            "package main\n\
             type I interface { Do() }\n\
             type Concrete struct{}\n\
             func (c Concrete) Do() {}\n\
             type Holder struct { *I }\n\
             func direct(h Holder) { h.Do() }\n\
             func nested(h Holder) { h.I.Do() }\n",
        )]);
        assert!(
            cg.go_embedded_interface_methods
                .get(&main_owner("Holder"))
                .and_then(|methods| methods.get("Do"))
                .is_none(),
            "invalid *I embed must not mint an S4 InterfaceDispatch route"
        );
        assert!(
            cg.resolve_call_site_full(site_in(&cg, "direct", "Do"))
                .resolved
                .is_empty(),
            "invalid *I embed must not satisfy I or resolve h.Do()"
        );
        let nested = site_in(&cg, "nested", "Do");
        assert_eq!(nested.receiver_type, None);
        assert_eq!(nested.receiver_recovery, None);
        let nested_outcome = cg.resolve_call_site_full(nested);
        assert!(
            nested_outcome
                .resolved
                .iter()
                .all(|candidate| candidate.kind != ResolutionKind::InterfaceDispatch),
            "invalid *I embed must not recover h.I as I and dispatch it: \
             site={nested:?}, outcome={nested_outcome:?}"
        );
    }

    #[test]
    fn s2_qualified_pointer_embedded_interface_never_recovers() {
        let cg = build_go(&[
            (
                "ext/types.go",
                "package ext\n\
                 type I interface { Do() }\n\
                 type Concrete struct{}\n\
                 func (c Concrete) Do() {}\n",
            ),
            (
                "main.go",
                "package main\n\
                 import \"github.com/x/y/ext\"\n\
                 type Holder struct { *ext.I }\n\
                 func nested(h Holder) { h.I.Do() }\n",
            ),
        ]);
        let nested = site_in(&cg, "nested", "Do");
        assert_eq!(nested.receiver_type, None);
        assert_eq!(nested.receiver_recovery, None);
        assert!(
            cg.resolve_call_site_full(nested)
                .resolved
                .iter()
                .all(|candidate| candidate.kind != ResolutionKind::InterfaceDispatch),
            "invalid *ext.I embed must not recover h.I for interface dispatch"
        );
    }

    #[test]
    fn qualified_embedded_struct_does_not_promote_unrelated_local_bare_target() {
        let cg = build_go(&[(
            "main.go",
            "package main\n\
             type Listener struct{}\n\
             func (l Listener) Serve() {}\n\
             type S struct { ext.Listener }\n\
             func run(s S) { s.Serve() }\n",
        )]);
        assert!(
            cg.resolve_call_site_full(site_in(&cg, "run", "Serve"))
                .resolved
                .is_empty(),
            "qualified ext.Listener must not promote an unrelated local Listener.Serve"
        );
    }

    #[test]
    fn qualified_embedded_interface_does_not_route_local_bare_interface() {
        let cg = build_go(&[(
            "main.go",
            "package main\n\
             type I interface { Do() }\n\
             type Concrete struct{}\n\
             func (c Concrete) Do() {}\n\
             type Holder struct { ext.I }\n\
             func run(h Holder) { h.Do() }\n",
        )]);
        assert!(
            cg.go_embedded_interface_methods
                .get(&main_owner("Holder"))
                .and_then(|methods| methods.get("Do"))
                .is_none(),
            "qualified ext.I must not mint an S4 route through unrelated local I"
        );
        assert!(
            cg.resolve_call_site_full(site_in(&cg, "run", "Do"))
                .resolved
                .is_empty(),
            "qualified ext.I must not dispatch h.Do() to local I implementers"
        );
    }

    #[test]
    fn s4_own_method_shadows_embedded_interface_promotion() {
        let cg = build_go(&[(
            "main.go",
            "package main\n\
             type Doer interface { Do() }\n\
             type Concrete struct{}\n\
             func (c Concrete) Do() {}\n\
             type Holder struct {\n\tDoer\n}\n\
             func (h Holder) Do() {}\n\
             func run(h Holder) {\n\th.Do()\n}\n",
        )]);
        // Holder has its OWN direct `Do` -- must NOT appear in the
        // embedded-interface promotion map (own method wins, ordinary
        // owner_lookup already resolves it).
        assert!(cg
            .go_embedded_interface_route("Holder", None, "Do", "main.go")
            .value
            .is_none());
        let site = site_in(&cg, "run", "Do");
        let out = cg.resolve_call_site_full(site);
        assert!(out.resolved.iter().any(|c| c.target.file == "main.go"
            && c.confidence == ResolutionConfidence::Exact
            && c.kind != ResolutionKind::InterfaceDispatch));
    }

    #[test]
    fn s4_external_embedded_interface_drops_without_wrong_edge() {
        let cg = build_go(&[(
            "main.go",
            "package main\n\
             import \"net\"\n\
             type Wrap struct {\n\tnet.Listener\n}\n\
             func run(w Wrap) {\n\tw.Accept()\n}\n",
        )]);
        // `net.Listener` is external (not in `data.interfaces`) -- no in-repo
        // satisfier set. Must drop cleanly: no panic, no wrong edge, no
        // Exact flip.
        let site = site_in(&cg, "run", "Accept");
        let out = cg.resolve_call_site_full(site);
        assert!(out.resolved.is_empty());
        assert!(cg
            .go_embedded_interface_route("Wrap", None, "Accept", "main.go")
            .value
            .is_none());
    }

    // ---- B2/M1 (codex impl-review): S4 package scoping + gate-failure drop

    #[test]
    fn b2_embedded_interface_route_is_package_scoped_not_bare_name() {
        // Two packages each define `Holder`; only the SECOND-inserted one
        // (by path order -- `BTreeMap<String, ParsedFile>` iterates
        // lexicographically, and `zzz_embed/...` sorts after
        // `aaa_plain/...`) embeds `Doer`. Pre-fix, the bare-keyed
        // `data.structs["Holder"]` extraction collapses to whichever file's
        // struct is processed LAST, so `zzz_embed`'s embedding data
        // overwrites `aaa_plain`'s (non-embedding) entry entirely --
        // `aaa_plain`'s `Holder.Do()` call site then incorrectly inherits
        // `zzz_embed`'s `Doer` donation and routes to `Concrete.Do`.
        let cg = build_go(&[
            (
                "zzz_embed/holder.go",
                "package zzzembed\n\
                 type Doer interface { Do() }\n\
                 type Concrete struct{}\n\
                 func (c Concrete) Do() {}\n\
                 type Holder struct {\n\tDoer\n}\n",
            ),
            (
                "aaa_plain/holder.go",
                "package aaaplain\n\
                 type Holder struct{}\n\
                 func run(h Holder) {\n\th.Do()\n}\n",
            ),
        ]);
        let site = site_in(&cg, "run", "Do");
        let out = cg.resolve_call_site_full(site);
        assert!(
            out.resolved.is_empty(),
            "aaa_plain's non-embedding Holder must not route via zzz_embed's \
             Doer donation, got {:?}",
            out
        );
    }

    #[test]
    fn m1_s4_gate_failure_drops_not_falls_through_to_alternate_route() {
        let cg = build_go(&[
            (
                "holder/holder.go",
                "package holder\n\
                 type Doer interface { Do(x int) }\n\
                 type Concrete struct{}\n\
                 func (c Concrete) Do(x int) {}\n\
                 type Holder struct {\n\tDoer\n}\n\
                 func run(h Holder) {\n\th.Do()\n}\n",
            ),
            (
                "other/other.go",
                "package other\n\
                 type Holder interface { Do() }\n\
                 type OtherImpl struct{}\n\
                 func (o OtherImpl) Do() {}\n",
            ),
        ]);
        let site = site_in(&cg, "run", "Do");
        let out = cg.resolve_call_site_full(site);
        // S4 routes `h.Do()` to the embedded `Doer.Do(x int)`, but the call
        // has 0 args -- arity-rejected. Without the M1 fix this falls
        // through to the bare `iface_key("Holder")` ladder and picks up the
        // UNRELATED `other.Holder` interface's `OtherImpl.Do` (a bare-name
        // collision with the STRUCT `holder.Holder`) -- must drop instead.
        assert!(
            out.resolved.is_empty(),
            "arity-rejected S4 route must drop, not mint an alternate-route edge: {:?}",
            out
        );
    }

    #[test]
    fn s4_ambiguous_two_embedded_interfaces_supplying_same_method_drops() {
        let cg = build_go(&[(
            "main.go",
            "package main\n\
             type A interface { M() }\n\
             type B interface { M() }\n\
             type ImplA struct{}\n\
             func (c ImplA) M() {}\n\
             type Holder struct {\n\tA\n\tB\n}\n\
             func run(h Holder) {\n\th.M()\n}\n",
        )]);
        let route = cg.go_embedded_interface_route("Holder", None, "M", "main.go");
        assert!(route.value.is_none());
        assert!(route.evidence.conflict);
    }

    // ---- S5: telemetry ----------------------------------------------------

    #[test]
    fn s5_resolution_kind_as_str_field_and_return_typed() {
        assert_eq!(ResolutionKind::FieldTyped.as_str(), "field_typed");
        assert_eq!(ResolutionKind::ReturnTyped.as_str(), "return_typed");
    }

    #[test]
    fn s5_call_stats_reports_dropped_go_receiver_bucket() {
        let cg = build_go(&[(
            "main.go",
            "package main\n\
             type Outer struct {\n\tField int\n}\n\
             func run(o *Outer) {\n\to.Field.M()\n}\n",
        )]);
        let stats = crate::navigation::queries::call_stats(&cg);
        let dropped = stats
            .get("dropped_go_receiver")
            .and_then(|v| v.as_object())
            .expect("dropped_go_receiver present");
        let total: u64 = dropped.values().filter_map(|v| v.as_u64()).sum();
        assert!(
            total > 0,
            "expected at least one attributed Go drop: {stats}"
        );
    }

    // ---- incremental parity (bidirectional) -------------------------------

    #[test]
    fn incremental_parity_edit_type_defining_file_updates_consumer() {
        use crate::cpg::CodePropertyGraph;
        use crate::data_flow::DataFlowGraph;
        use std::collections::BTreeSet;

        let mut files: BTreeMap<String, ParsedFile> = BTreeMap::new();
        files.insert(
            "demux.go".to_string(),
            ParsedFile::parse(
                "demux.go",
                "package demux\n\
                 type Demux struct{}\n\
                 func (d *Demux) Init() {}\n\
                 func newDemux() *Demux { return &Demux{} }\n",
                Go,
            )
            .unwrap(),
        );
        files.insert(
            "consumer.go".to_string(),
            ParsedFile::parse(
                "consumer.go",
                "package demux\n\
                 func run() {\n\td := newDemux()\n\td.Init()\n}\n",
                Go,
            )
            .unwrap(),
        );
        let cg0 = CallGraph::build(&files);
        let site0 = site_in(&cg0, "run", "Init");
        assert_eq!(site0.receiver_type.as_deref(), Some("Demux"));

        // Edit ONLY the type-defining file: `newDemux` now returns a
        // DIFFERENT type. `consumer.go` is untouched.
        files.insert(
            "demux.go".to_string(),
            ParsedFile::parse(
                "demux.go",
                "package demux\n\
                 type Demux struct{}\n\
                 func (d *Demux) Init() {}\n\
                 type Other struct{}\n\
                 func newDemux() *Other { return &Other{} }\n",
                Go,
            )
            .unwrap(),
        );
        let changed: BTreeSet<String> = ["demux.go".to_string()].into_iter().collect();
        let cpg = CodePropertyGraph::build_incremental(
            cg0,
            DataFlowGraph::build(&BTreeMap::new()),
            &changed,
            &files,
            None,
        );
        let cg1 = &cpg.call_graph;
        let site1 = site_in(cg1, "run", "Init");
        // `consumer.go` never changed, but its recovery MUST update: `Demux`
        // no longer has an `Init` method reachable from `d`'s new type.
        assert_ne!(
            site1.receiver_type.as_deref(),
            Some("Demux"),
            "consumer.go's retained call site kept a stale recovery after \
             the type-defining file changed"
        );
    }

    #[test]
    fn incremental_parity_edit_pointer_embed_defining_file_updates_retained_consumer() {
        use crate::cpg::CodePropertyGraph;
        use crate::data_flow::DataFlowGraph;
        use std::collections::BTreeSet;

        let mut files = BTreeMap::new();
        files.insert(
            "listener.go".to_string(),
            ParsedFile::parse(
                "listener.go",
                "package p\ntype Listener struct{}\nfunc (l *Listener) Serve() {}\n",
                Go,
            )
            .unwrap(),
        );
        files.insert(
            "embed.go".to_string(),
            ParsedFile::parse("embed.go", "package p\ntype S struct{}\n", Go).unwrap(),
        );
        files.insert(
            "consumer.go".to_string(),
            ParsedFile::parse(
                "consumer.go",
                "package p\nfunc run(s S) { s.Listener.Serve() }\n",
                Go,
            )
            .unwrap(),
        );
        files.insert(
            "b/listener.go".to_string(),
            ParsedFile::parse(
                "b/listener.go",
                "package b\ntype Listener struct{}\nfunc (*Listener) Serve() {}\n",
                Go,
            )
            .unwrap(),
        );
        let cg0 = CallGraph::build(&files);
        let site0 = site_in(&cg0, "run", "Serve");
        assert_eq!(site0.receiver_type, None);
        assert_eq!(site0.receiver_recovery, None);
        assert_eq!(site0.receiver_outcome, None);

        files.insert(
            "embed.go".to_string(),
            ParsedFile::parse("embed.go", "package p\ntype S struct { *Listener }\n", Go).unwrap(),
        );
        let full = CallGraph::build(&files);
        let full_out = full.resolve_call_site_full(site_in(&full, "run", "Serve"));
        assert_eq!(
            full_out.resolved.len(),
            1,
            "full-build decoy leaked: {full_out:?}"
        );
        assert_eq!(full_out.resolved[0].target.file, "listener.go");
        assert_eq!(full_out.resolved[0].confidence, ResolutionConfidence::Exact);

        let changed = BTreeSet::from(["embed.go".to_string()]);
        let incremental = CodePropertyGraph::build_incremental(
            cg0,
            DataFlowGraph::build(&BTreeMap::new()),
            &changed,
            &files,
            None,
        );
        let incremental_out = incremental.call_graph.resolve_call_site_full(site_in(
            &incremental.call_graph,
            "run",
            "Serve",
        ));
        assert_eq!(
            incremental_out.resolved.len(),
            1,
            "incremental decoy leaked: {incremental_out:?}"
        );
        assert_eq!(incremental_out.resolved[0].target.file, "listener.go");
        assert_eq!(
            incremental_out.resolved[0].confidence,
            ResolutionConfidence::Exact
        );
    }

    #[test]
    fn incremental_parity_edit_consuming_file_recomputes() {
        use crate::cpg::CodePropertyGraph;
        use crate::data_flow::DataFlowGraph;
        use std::collections::BTreeSet;

        let mut files: BTreeMap<String, ParsedFile> = BTreeMap::new();
        files.insert(
            "demux.go".to_string(),
            ParsedFile::parse(
                "demux.go",
                "package demux\n\
                 type Demux struct{}\n\
                 func (d *Demux) Init() {}\n\
                 func newDemux() *Demux { return &Demux{} }\n",
                Go,
            )
            .unwrap(),
        );
        files.insert(
            "consumer.go".to_string(),
            ParsedFile::parse("consumer.go", "package demux\nfunc run() {}\n", Go).unwrap(),
        );
        let cg0 = CallGraph::build(&files);
        assert!(cg0
            .calls
            .iter()
            .all(|(fid, sites)| fid.name != "run" || sites.is_empty()));

        // Edit ONLY the consuming file: add the call-RHS receiver usage.
        files.insert(
            "consumer.go".to_string(),
            ParsedFile::parse(
                "consumer.go",
                "package demux\n\
                 func run() {\n\td := newDemux()\n\td.Init()\n}\n",
                Go,
            )
            .unwrap(),
        );
        let changed: BTreeSet<String> = ["consumer.go".to_string()].into_iter().collect();
        let cpg = CodePropertyGraph::build_incremental(
            cg0,
            DataFlowGraph::build(&BTreeMap::new()),
            &changed,
            &files,
            None,
        );
        let cg1 = &cpg.call_graph;
        let site1 = site_in(cg1, "run", "Init");
        assert_eq!(site1.receiver_type.as_deref(), Some("Demux"));
        assert_eq!(site1.receiver_recovery, Some(ReceiverRecovery::ReturnTyped));
    }

    #[test]
    fn incremental_parity_edit_unrelated_file_survives() {
        use crate::cpg::CodePropertyGraph;
        use crate::data_flow::DataFlowGraph;
        use std::collections::BTreeSet;

        let mut files: BTreeMap<String, ParsedFile> = BTreeMap::new();
        files.insert(
            "demux.go".to_string(),
            ParsedFile::parse(
                "demux.go",
                "package demux\n\
                 type Demux struct{}\n\
                 func (d *Demux) Init() {}\n\
                 func newDemux() *Demux { return &Demux{} }\n",
                Go,
            )
            .unwrap(),
        );
        files.insert(
            "consumer.go".to_string(),
            ParsedFile::parse(
                "consumer.go",
                "package demux\n\
                 func run() {\n\td := newDemux()\n\td.Init()\n}\n",
                Go,
            )
            .unwrap(),
        );
        files.insert(
            "unrelated.go".to_string(),
            ParsedFile::parse("unrelated.go", "package other\nfunc noop() {}\n", Go).unwrap(),
        );
        let cg0 = CallGraph::build(&files);
        let site0 = site_in(&cg0, "run", "Init");
        assert_eq!(site0.receiver_type.as_deref(), Some("Demux"));

        files.insert(
            "unrelated.go".to_string(),
            ParsedFile::parse(
                "unrelated.go",
                "package other\nfunc noop() {}\nfunc noop2() {}\n",
                Go,
            )
            .unwrap(),
        );
        let changed: BTreeSet<String> = ["unrelated.go".to_string()].into_iter().collect();
        let cpg = CodePropertyGraph::build_incremental(
            cg0,
            DataFlowGraph::build(&BTreeMap::new()),
            &changed,
            &files,
            None,
        );
        let cg1 = &cpg.call_graph;
        let site1 = site_in(cg1, "run", "Init");
        assert_eq!(site1.receiver_type.as_deref(), Some("Demux"));
        assert_eq!(site1.receiver_recovery, Some(ReceiverRecovery::ReturnTyped));
    }
}

#[cfg(test)]
mod python_property_access_tests {
    use super::*;
    use crate::languages::Language::{JavaScript, Python};
    use std::collections::BTreeMap;

    fn build_py(files: &[(&str, &str)]) -> CallGraph {
        let mut map = BTreeMap::new();
        for (path, src) in files {
            map.insert(
                path.to_string(),
                ParsedFile::parse(path, src, Python).unwrap(),
            );
        }
        CallGraph::build(&map)
    }

    fn fid_named<'a>(cg: &'a CallGraph, name: &str) -> &'a FunctionId {
        cg.functions
            .get(name)
            .unwrap_or_else(|| panic!("no function named {name}"))
            .first()
            .unwrap()
    }

    // ---- S1: property getter index -----------------------------------

    #[test]
    fn property_getter_is_recorded_in_index() {
        let cg = build_py(&[(
            "resp.py",
            "class Response:\n    @property\n    def text(self):\n        return self._text\n",
        )]);
        let getters = cg
            .property_getters
            .get(&("Response".to_string(), "text".to_string()))
            .expect("@property getter recorded");
        assert_eq!(getters.len(), 1);
        assert!(cg.cached_property_getters.is_empty());
    }

    #[test]
    fn cached_property_getter_is_recorded_and_tracked_separately() {
        let cg = build_py(&[(
            "resp.py",
            "import functools\n\nclass Response:\n    @functools.cached_property\n    def text(self):\n        return self._text\n",
        )]);
        let getters = cg
            .property_getters
            .get(&("Response".to_string(), "text".to_string()))
            .expect("@functools.cached_property getter recorded");
        assert_eq!(getters.len(), 1);
        assert_eq!(cg.cached_property_getters.len(), 1);
        assert!(cg.cached_property_getters.contains(&getters[0]));
    }

    #[test]
    fn bare_cached_property_decorator_is_recorded_and_tracked_separately() {
        let cg = build_py(&[(
            "resp.py",
            "from functools import cached_property\n\nclass Response:\n    @cached_property\n    def text(self):\n        return self._text\n",
        )]);
        let getters = cg
            .property_getters
            .get(&("Response".to_string(), "text".to_string()))
            .expect("@cached_property getter recorded");
        assert_eq!(cg.cached_property_getters.len(), 1);
        assert!(cg.cached_property_getters.contains(&getters[0]));
    }

    #[test]
    fn setter_decorated_method_never_pollutes_getter_index() {
        let cg = build_py(&[(
            "resp.py",
            "class Response:\n    @property\n    def text(self):\n        return self._text\n\n    @text.setter\n    def text(self, value):\n        self._text = value\n",
        )]);
        let getters = cg
            .property_getters
            .get(&("Response".to_string(), "text".to_string()))
            .expect("getter recorded");
        // Exactly the getter, never the `@text.setter`-decorated definition.
        assert_eq!(getters.len(), 1);
        assert_eq!(getters[0].start_line, 2);
    }

    #[test]
    fn non_decorated_same_name_method_is_not_indexed() {
        let cg = build_py(&[(
            "resp.py",
            "class Response:\n    def text(self):\n        return self._text\n",
        )]);
        assert!(cg
            .property_getters
            .get(&("Response".to_string(), "text".to_string()))
            .is_none());
    }

    #[test]
    fn non_python_files_leave_property_state_empty() {
        let mut files = BTreeMap::new();
        files.insert(
            "resp.js".to_string(),
            ParsedFile::parse(
                "resp.js",
                "class Response {\n  get text() { return this._text; }\n}\nfunction f(r) {\n  return r.text;\n}\n",
                JavaScript,
            )
            .unwrap(),
        );
        let cg = CallGraph::build(&files);
        assert!(cg.property_getters.is_empty());
        assert!(cg.cached_property_getters.is_empty());
        assert!(cg.property_accesses.is_empty());
        assert_eq!(cg.property_access_fanout_skips, 0);
        assert_eq!(cg.property_access_store_skips, 0);
    }

    // ---- S2: access-site extraction ------------------------------------

    #[test]
    fn self_attr_records_against_own_class_getter() {
        let cg = build_py(&[(
            "resp.py",
            "class Response:\n    @property\n    def text(self):\n        return self._text\n\n    def dump(self):\n        return self.text\n",
        )]);
        let getter = fid_named(&cg, "text").clone();
        let enclosing = fid_named(&cg, "dump").clone();
        assert!(cg
            .property_accesses
            .iter()
            .any(|a| a.getter == getter && a.enclosing == enclosing));
    }

    #[test]
    fn unknown_receiver_single_owner_is_recorded() {
        let cg = build_py(&[(
            "resp.py",
            "class Response:\n    @property\n    def text(self):\n        return self._text\n\n\ndef f(r):\n    return r.text\n",
        )]);
        let getter = fid_named(&cg, "text").clone();
        let enclosing = fid_named(&cg, "f").clone();
        assert!(cg
            .property_accesses
            .iter()
            .any(|a| a.getter == getter && a.enclosing == enclosing));
        assert_eq!(cg.property_access_fanout_skips, 0);
    }

    #[test]
    fn plain_assignment_target_is_not_recorded() {
        let cg = build_py(&[(
            "resp.py",
            "class Response:\n    @property\n    def text(self):\n        return self._text\n\n\ndef f(r):\n    r.text = \"v\"\n",
        )]);
        assert!(cg.property_accesses.is_empty());
    }

    #[test]
    fn augmented_assignment_target_is_not_recorded() {
        let cg = build_py(&[(
            "resp.py",
            "class Response:\n    @property\n    def text(self):\n        return self._text\n\n\ndef f(r):\n    r.text += \"v\"\n",
        )]);
        assert!(cg.property_accesses.is_empty());
    }

    // ---- F5: property_access_store_skips telemetry ---------------------

    #[test]
    fn store_target_with_indexed_name_increments_store_skip_counter() {
        let cg = build_py(&[(
            "resp.py",
            "class Response:\n    @property\n    def text(self):\n        return self._text\n\n\ndef f(r):\n    r.text = \"v\"\n",
        )]);
        assert_eq!(cg.property_access_store_skips, 1);
    }

    #[test]
    fn store_target_with_non_indexed_name_does_not_increment_store_skip_counter() {
        let cg = build_py(&[(
            "resp.py",
            "class Response:\n    @property\n    def text(self):\n        return self._text\n\n\ndef f(r):\n    r.other = \"v\"\n",
        )]);
        assert_eq!(cg.property_access_store_skips, 0);
    }

    #[test]
    fn delete_for_and_with_alias_store_skips_are_all_counted() {
        // F4 + F5 together: every one of the store/delete contexts F4 fences
        // (assignment/augmented left, del target, for target, with-alias)
        // increments the same counter when the name is indexed.
        let cg = build_py(&[(
            "resp.py",
            "class Response:\n    @property\n    def text(self):\n        return self._text\n\n\n\
def f(r, xs, cm):\n    r.text = \"v\"\n    r.text += \"v\"\n    del r.text\n    for r.text in xs:\n        pass\n    with cm() as r.text:\n        pass\n",
        )]);
        assert_eq!(cg.property_access_store_skips, 5);
    }

    // ---- F4: delete/for/with-target store contexts ---------------------

    #[test]
    fn delete_statement_target_is_not_recorded() {
        let cg = build_py(&[(
            "resp.py",
            "class Response:\n    @property\n    def text(self):\n        return self._text\n\n\ndef f(r):\n    del r.text\n",
        )]);
        assert!(cg.property_accesses.is_empty());
    }

    #[test]
    fn for_target_attribute_is_not_recorded() {
        let cg = build_py(&[(
            "resp.py",
            "class Response:\n    @property\n    def text(self):\n        return self._text\n\n\ndef f(r, xs):\n    for r.text in xs:\n        pass\n",
        )]);
        assert!(cg.property_accesses.is_empty());
    }

    #[test]
    fn comprehension_for_target_attribute_is_not_recorded() {
        let cg = build_py(&[(
            "resp.py",
            "class Response:\n    @property\n    def text(self):\n        return self._text\n\n\ndef f(r, xs):\n    return [x for r.text in xs]\n",
        )]);
        assert!(cg.property_accesses.is_empty());
    }

    #[test]
    fn for_iterable_attribute_load_is_still_recorded() {
        // The right-hand/iterable side of a `for` remains LOAD context.
        let cg = build_py(&[(
            "resp.py",
            "class Response:\n    @property\n    def text(self):\n        return self._text\n\n\ndef f(r):\n    for x in r.text:\n        pass\n",
        )]);
        let getter = fid_named(&cg, "text").clone();
        let enclosing = fid_named(&cg, "f").clone();
        assert!(cg
            .property_accesses
            .iter()
            .any(|a| a.getter == getter && a.enclosing == enclosing));
    }

    #[test]
    fn with_alias_target_attribute_is_not_recorded() {
        let cg = build_py(&[(
            "resp.py",
            "class Response:\n    @property\n    def text(self):\n        return self._text\n\n\ndef f(r, cm):\n    with cm() as r.text:\n        pass\n",
        )]);
        assert!(cg.property_accesses.is_empty());
    }

    #[test]
    fn with_item_value_attribute_load_is_still_recorded() {
        // The context-manager-expression side of `with ... as x:` remains
        // LOAD context, even though the alias target does not.
        let cg = build_py(&[(
            "resp.py",
            "class Response:\n    @property\n    def text(self):\n        return self._text\n\n\ndef f(r):\n    with r.text as x:\n        pass\n",
        )]);
        let getter = fid_named(&cg, "text").clone();
        let enclosing = fid_named(&cg, "f").clone();
        assert!(cg
            .property_accesses
            .iter()
            .any(|a| a.getter == getter && a.enclosing == enclosing));
    }

    #[test]
    fn call_of_attribute_is_not_double_recorded() {
        // `r.text()` — the attribute is the function child of a call, not a
        // load; the property-access table must not record it (the ordinary
        // call-resolution path, independent of this table, is what handles
        // `r.text()` syntactically as a method call).
        let cg = build_py(&[(
            "resp.py",
            "class Response:\n    @property\n    def text(self):\n        return self._text\n\n\ndef f(r):\n    return r.text()\n",
        )]);
        assert!(cg.property_accesses.is_empty());
    }

    #[test]
    fn fanout_at_cap_records_all_three() {
        let cg = build_py(&[(
            "resp.py",
            "class A:\n    @property\n    def text(self):\n        return 1\n\n\n\
class B:\n    @property\n    def text(self):\n        return 2\n\n\n\
class C:\n    @property\n    def text(self):\n        return 3\n\n\n\
def f(r):\n    return r.text\n",
        )]);
        assert_eq!(cg.property_accesses.len(), 3);
        assert_eq!(cg.property_access_fanout_skips, 0);
    }

    #[test]
    fn fanout_over_cap_is_skipped_and_counted() {
        let cg = build_py(&[(
            "resp.py",
            "class A:\n    @property\n    def text(self):\n        return 1\n\n\n\
class B:\n    @property\n    def text(self):\n        return 2\n\n\n\
class C:\n    @property\n    def text(self):\n        return 3\n\n\n\
class D:\n    @property\n    def text(self):\n        return 4\n\n\n\
def f(r):\n    return r.text\n",
        )]);
        assert!(cg.property_accesses.is_empty());
        assert_eq!(cg.property_access_fanout_skips, 1);
    }

    #[test]
    fn cls_attr_does_not_narrow_to_its_own_class() {
        // `cls.text` inside a classmethod of `A` must NOT be narrowed to just
        // A's getter (the self-like shortcut is explicitly excluded) — with
        // 2 classes defining `text`, it must fan out to BOTH (under cap),
        // pinning that `cls` gets no special treatment vs. any other
        // unrecognized receiver.
        let cg = build_py(&[(
            "resp.py",
            "class A:\n    @property\n    def text(self):\n        return 1\n\n    @classmethod\n    def make(cls):\n        return cls.text\n\n\n\
class B:\n    @property\n    def text(self):\n        return 2\n",
        )]);
        let a_getter = cg
            .property_getters
            .get(&("A".to_string(), "text".to_string()))
            .unwrap()[0]
            .clone();
        let b_getter = cg
            .property_getters
            .get(&("B".to_string(), "text".to_string()))
            .unwrap()[0]
            .clone();
        let enclosing = fid_named(&cg, "make").clone();
        assert!(cg
            .property_accesses
            .iter()
            .any(|a| a.getter == a_getter && a.enclosing == enclosing));
        assert!(
            cg.property_accesses
                .iter()
                .any(|a| a.getter == b_getter && a.enclosing == enclosing),
            "cls.text must fan out to B's getter too, not narrow to A alone"
        );
    }

    #[test]
    fn staticmethod_self_param_does_not_get_same_class_narrowing() {
        // F2 (codex MAJOR 2): `@staticmethod def make(self)` — Python's
        // generic `method_owner` marks it as owned by `A` (any
        // class-contained function), but `self` here is an ordinary
        // parameter of unknown type, not a receiver. It must NOT get tier-1
        // same-class narrowing; it must route to tier-3 (unknown-receiver,
        // capped fanout) like any other unrecognized receiver — so with
        // A and B both defining `text`, it must fan out to BOTH.
        let cg = build_py(&[(
            "resp.py",
            "class A:\n    @property\n    def text(self):\n        return 1\n\n    @staticmethod\n    def make(self):\n        return self.text\n\n\n\
class B:\n    @property\n    def text(self):\n        return 2\n",
        )]);
        let a_getter = cg
            .property_getters
            .get(&("A".to_string(), "text".to_string()))
            .unwrap()[0]
            .clone();
        let b_getter = cg
            .property_getters
            .get(&("B".to_string(), "text".to_string()))
            .unwrap()[0]
            .clone();
        let enclosing = fid_named(&cg, "make").clone();
        assert!(
            cg.property_accesses
                .iter()
                .any(|a| a.getter == a_getter && a.enclosing == enclosing),
            "staticmethod self.text must still fan out to A's getter"
        );
        assert!(
            cg.property_accesses
                .iter()
                .any(|a| a.getter == b_getter && a.enclosing == enclosing),
            "staticmethod self.text must fan out to B's getter too, not narrow to A alone"
        );
    }

    #[test]
    fn top_level_self_param_routes_to_tier3_fanout_not_drop() {
        // Item 1 (MAJOR, codex re-review): a top-level `def f(self)` is not
        // contained in any class, so `method_owners` has no entry for it.
        // Tier-1 eligibility now requires `method_owners.contains_key`, so
        // this must NOT enter tier-1 (and then drop when
        // `self_property_owner_getters` finds no owner) — it must route to
        // tier-3 unknown-receiver fanout, same as any other unrecognized
        // receiver. With exactly one class defining `text`, that fanout
        // yields a single edge, not a drop.
        let cg = build_py(&[(
            "resp.py",
            "class Response:\n    @property\n    def text(self):\n        return self._text\n\n\ndef f(self):\n    return self.text\n",
        )]);
        let getter = fid_named(&cg, "text").clone();
        let enclosing = fid_named(&cg, "f").clone();
        assert!(
            cg.property_accesses
                .iter()
                .any(|a| a.getter == getter && a.enclosing == enclosing),
            "top-level def f(self) reading self.text must fan out to Response's getter, not drop"
        );
        assert_eq!(cg.property_access_fanout_skips, 0);
    }

    #[test]
    fn keyword_only_self_param_does_not_narrow_to_owning_class() {
        // Item 1 (MAJOR): `def make(*, self)` inside class A — `self` is
        // keyword-only (appears after the bare `*` separator), so it is NOT
        // the first positional parameter and must fail the instance-method
        // gate even though `make` IS owned by A (`method_owners` has an
        // entry). Must NOT narrow to A alone; must fan out to both A and B,
        // proving the keyword-only receiver gets no special treatment.
        let cg = build_py(&[(
            "resp.py",
            "class A:\n    @property\n    def text(self):\n        return 1\n\n    def make(*, self):\n        return self.text\n\n\n\
class B:\n    @property\n    def text(self):\n        return 2\n",
        )]);
        let a_getter = cg
            .property_getters
            .get(&("A".to_string(), "text".to_string()))
            .unwrap()[0]
            .clone();
        let b_getter = cg
            .property_getters
            .get(&("B".to_string(), "text".to_string()))
            .unwrap()[0]
            .clone();
        let enclosing = fid_named(&cg, "make").clone();
        assert!(
            cg.property_accesses
                .iter()
                .any(|a| a.getter == a_getter && a.enclosing == enclosing),
            "keyword-only self.text must still fan out to A's getter"
        );
        assert!(
            cg.property_accesses
                .iter()
                .any(|a| a.getter == b_getter && a.enclosing == enclosing),
            "keyword-only self.text must fan out to B's getter too, not narrow to A alone"
        );
    }

    #[test]
    fn positional_only_self_param_is_still_tier1_narrowed() {
        // Item 1 (c): `self` before a bare `/` positional-only separator
        // still counts as the first positional parameter (`def m(self, /)`)
        // — must still get tier-1 same-class narrowing, not fan out to an
        // unrelated class's getter of the same name.
        let cg = build_py(&[(
            "resp.py",
            "class Response:\n    @property\n    def text(self):\n        return self._text\n\n    def dump(self, /):\n        return self.text\n\n\n\
class Other:\n    @property\n    def text(self):\n        return 2\n",
        )]);
        let response_getter = cg
            .property_getters
            .get(&("Response".to_string(), "text".to_string()))
            .unwrap()[0]
            .clone();
        let other_getter = cg
            .property_getters
            .get(&("Other".to_string(), "text".to_string()))
            .unwrap()[0]
            .clone();
        let enclosing = fid_named(&cg, "dump").clone();
        assert!(
            cg.property_accesses
                .iter()
                .any(|a| a.getter == response_getter && a.enclosing == enclosing),
            "positional-only self.text must narrow to Response's own getter"
        );
        assert!(
            !cg.property_accesses
                .iter()
                .any(|a| a.getter == other_getter && a.enclosing == enclosing),
            "positional-only self must NOT fan out to Other's unrelated getter"
        );
    }

    #[test]
    fn property_getter_reading_self_other_prop_is_still_tier1_narrowed() {
        // F2: the enclosing function MAY legitimately carry other
        // decorators, including `@property` itself — a getter reading
        // `self.other_prop` is a genuine instance method and must still get
        // tier-1 same-class narrowing (not fan out to an unrelated class
        // that also happens to define `other_prop`).
        let cg = build_py(&[(
            "resp.py",
            "class A:\n    @property\n    def other_prop(self):\n        return 1\n\n    @property\n    def text(self):\n        return self.other_prop\n\n\n\
class B:\n    @property\n    def other_prop(self):\n        return 2\n",
        )]);
        let a_other_prop = cg
            .property_getters
            .get(&("A".to_string(), "other_prop".to_string()))
            .unwrap()[0]
            .clone();
        let b_other_prop = cg
            .property_getters
            .get(&("B".to_string(), "other_prop".to_string()))
            .unwrap()[0]
            .clone();
        let enclosing = fid_named(&cg, "text").clone();
        assert!(
            cg.property_accesses
                .iter()
                .any(|a| a.getter == a_other_prop && a.enclosing == enclosing),
            "text() reading self.other_prop must narrow to A's own other_prop getter"
        );
        assert!(
            !cg.property_accesses
                .iter()
                .any(|a| a.getter == b_other_prop && a.enclosing == enclosing),
            "must NOT fan out to B's unrelated other_prop getter"
        );
    }

    #[test]
    fn self_attr_with_no_matching_class_does_not_fan_out() {
        // `self.text` inside `A` (which does NOT define `text`) must not be
        // attributed to `B`'s unrelated getter, even though the name is
        // indexed globally — a known non-matching receiver is a negative
        // result, not "unknown".
        let cg = build_py(&[(
            "resp.py",
            "class A:\n    def dump(self):\n        return self.text\n\n\n\
class B:\n    @property\n    def text(self):\n        return 2\n",
        )]);
        assert!(cg.property_accesses.is_empty());
    }

    #[test]
    fn self_attr_own_class_hit_filtered_by_caller_file_and_span() {
        // F1 (codex MAJOR 1): two files each define `class Response` with
        // `@property def text`. The bare `(owner, attr)` index key collides
        // across files, so an unfiltered own-class hit would fan out to
        // BOTH getters when a method in file A's Response reads `self.text`.
        // The own-class hit must be filtered to the caller's own file AND
        // class span, exactly like the inherited-base fallback already is.
        let cg = build_py(&[
            (
                "resp_a.py",
                "class Response:\n    @property\n    def text(self):\n        return self._text\n\n    def dump(self):\n        return self.text\n",
            ),
            (
                "resp_b.py",
                "class Response:\n    @property\n    def text(self):\n        return self._text\n",
            ),
        ]);
        let getter_a = cg
            .property_getters
            .get(&("Response".to_string(), "text".to_string()))
            .expect("text getters indexed")
            .iter()
            .find(|fid| fid.file == "resp_a.py")
            .expect("resp_a.py getter present")
            .clone();
        let enclosing = cg
            .functions
            .get("dump")
            .expect("dump indexed")
            .iter()
            .find(|fid| fid.file == "resp_a.py")
            .expect("dump in resp_a.py")
            .clone();
        let matches: Vec<_> = cg
            .property_accesses
            .iter()
            .filter(|a| a.enclosing == enclosing)
            .collect();
        assert_eq!(
            matches.len(),
            1,
            "self.text in resp_a.py's Response.dump must produce exactly one edge, got {:?}",
            matches
        );
        assert_eq!(matches[0].getter, getter_a);
    }

    #[test]
    fn self_attr_narrows_via_same_file_single_base() {
        let cg = build_py(&[(
            "resp.py",
            "class Base:\n    @property\n    def text(self):\n        return self._text\n\n\n\
class Child(Base):\n    def dump(self):\n        return self.text\n",
        )]);
        let getter = fid_named(&cg, "text").clone();
        let enclosing = fid_named(&cg, "dump").clone();
        assert!(cg
            .property_accesses
            .iter()
            .any(|a| a.getter == getter && a.enclosing == enclosing));
    }

    // ---- F3: nested callable/class scope fencing -----------------------

    #[test]
    fn nested_def_self_attr_not_attributed_to_outer_method() {
        // F3 (codex MAJOR 3): a nested `def` has the SAME kind as the outer
        // function but its own scope — `self.text` inside it must not be
        // recorded against the OUTER method's FunctionId (misattribution +
        // double-scan, since `all_functions()` visits the nested def
        // separately anyway).
        //
        // Item 2 (MINOR, codex re-review): this pin is only load-bearing if
        // it also proves the access isn't dropped entirely — the nested
        // def's OWN walk must record it under the nested FunctionId (via
        // tier-3: `helper` has no `self` parameter of its own, so it never
        // qualifies for tier-1 in the first place — this is unaffected by
        // which routing failure sends it to tier-3). With exactly one class
        // defining `text`, tier-3 fanout yields a single owner edge.
        let cg = build_py(&[(
            "resp.py",
            "class Response:\n    @property\n    def text(self):\n        return self._text\n\n    def dump(self):\n        def helper():\n            return self.text\n        return helper()\n",
        )]);
        let dump = fid_named(&cg, "dump").clone();
        let helper = fid_named(&cg, "helper").clone();
        let getter = fid_named(&cg, "text").clone();
        assert!(
            !cg.property_accesses.iter().any(|a| a.enclosing == dump),
            "self.text inside a nested def must not be attributed to the outer method"
        );
        assert!(
            cg.property_accesses
                .iter()
                .any(|a| a.getter == getter && a.enclosing == helper),
            "self.text inside the nested def must be recorded under the nested function's own FunctionId, not dropped"
        );
    }

    #[test]
    fn lambda_body_self_attr_is_not_scanned() {
        // F3: lambda bodies are fenced out of the walk entirely (accepted
        // recall gap — no other scanner covers lambda bodies either, since
        // lambdas never get their own FunctionId from `all_functions()`).
        let cg = build_py(&[(
            "resp.py",
            "class Response:\n    @property\n    def text(self):\n        return self._text\n\n    def dump(self):\n        f = lambda: self.text\n        return f()\n",
        )]);
        assert!(
            cg.property_accesses.is_empty(),
            "self.text inside a lambda body must not surface any edge"
        );
    }

    /// P8 non-Rust guard: the macro-arg extractor is gated on
    /// `self.language == Language::Rust` AND a real `macro_invocation` node
    /// kind (which only tree-sitter-rust ever produces) -- literal
    /// `assert!(f())`-shaped TEXT sitting inside a JS template literal or a
    /// Python string must never mint a call for `f`/`assert`/`check`.
    #[test]
    fn non_rust_assert_bang_text_in_string_or_template_literal_mints_nothing() {
        let py = build_py(&[(
            "notes.py",
            "def host():\n    s = \"assert!(check(1))\"\n    return s\n",
        )]);
        assert!(
            !py.functions.contains_key("check"),
            "a Python string literal containing assert!(check(1)) text must not mint a call"
        );
        assert!(py
            .calls
            .values()
            .flat_map(|s| s.iter())
            .all(|s| s.callee_name != "check" && s.callee_name != "assert"));

        let mut js_map = BTreeMap::new();
        js_map.insert(
            "notes.js".to_string(),
            ParsedFile::parse(
                "notes.js",
                "function host() {\n    const s = `assert!(check(1))`;\n    return s;\n}\n",
                JavaScript,
            )
            .unwrap(),
        );
        let js = CallGraph::build(&js_map);
        assert!(js
            .calls
            .values()
            .flat_map(|s| s.iter())
            .all(|s| s.callee_name != "check" && s.callee_name != "assert"));
    }
}

// ---------------------------------------------------------------------------
// P9: framework-entry (Flask/FastAPI/Express route registration) tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod framework_entry_tests {
    use super::*;
    use crate::framework_entries::MODULE_PSEUDO_CALLER_NAME;
    use crate::languages::Language::{Go, JavaScript, Python};
    use std::collections::BTreeMap;

    fn build_py(files: &[(&str, &str)]) -> CallGraph {
        let mut map = BTreeMap::new();
        for (path, src) in files {
            map.insert(
                path.to_string(),
                ParsedFile::parse(path, src, Python).unwrap(),
            );
        }
        CallGraph::build(&map)
    }

    fn build_js(files: &[(&str, &str)]) -> CallGraph {
        let mut map = BTreeMap::new();
        for (path, src) in files {
            map.insert(
                path.to_string(),
                ParsedFile::parse(path, src, JavaScript).unwrap(),
            );
        }
        CallGraph::build(&map)
    }

    fn fid<'a>(cg: &'a CallGraph, name: &str) -> &'a FunctionId {
        cg.functions
            .get(name)
            .unwrap_or_else(|| panic!("no function named {name}"))
            .first()
            .unwrap()
    }

    // ---- Python: Flask / FastAPI ------------------------------------------

    #[test]
    fn flask_route_recorded_with_decorator_line() {
        let cg = build_py(&[(
            "app.py",
            "from flask import Flask\napp = Flask(__name__)\n\n@app.route(\"/x\")\ndef handler():\n    return \"ok\"\n",
        )]);
        let handler = fid(&cg, "handler").clone();
        let rec = cg
            .framework_entries
            .iter()
            .find(|r| r.handler == handler)
            .expect("flask route recorded");
        assert_eq!(rec.site.line, 4);
        assert_eq!(rec.framework, "flask");
        assert_eq!(rec.caller.name, MODULE_PSEUDO_CALLER_NAME);
        assert_eq!(rec.caller.start_line, 1);
    }

    #[test]
    fn fastapi_route_recorded() {
        let cg = build_py(&[(
            "app.py",
            "from fastapi import FastAPI\napp = FastAPI()\n\n@app.get(\"/x\")\ndef handler():\n    return \"ok\"\n",
        )]);
        let handler = fid(&cg, "handler").clone();
        let rec = cg
            .framework_entries
            .iter()
            .find(|r| r.handler == handler)
            .expect("fastapi route recorded");
        assert_eq!(rec.framework, "fastapi");
        assert_eq!(rec.site.line, 4);
    }

    #[test]
    fn two_route_decorators_yield_two_records() {
        let cg = build_py(&[(
            "app.py",
            "from flask import Flask\napp = Flask(__name__)\n\n@app.route(\"/a\")\n@app.route(\"/b\")\ndef handler():\n    return \"ok\"\n",
        )]);
        let handler = fid(&cg, "handler").clone();
        let recs: Vec<_> = cg
            .framework_entries
            .iter()
            .filter(|r| r.handler == handler)
            .collect();
        assert_eq!(recs.len(), 2, "two route decorators -> two records");
    }

    #[test]
    fn non_route_decorator_not_recorded() {
        let cg = build_py(&[(
            "app.py",
            "from flask import Flask\napp = Flask(__name__)\n\nclass Foo:\n    @property\n    def value(self):\n        return 1\n",
        )]);
        assert!(cg.framework_entries.is_empty());
    }

    #[test]
    fn python_nested_registration_enclosing_is_factory_function() {
        let cg = build_py(&[(
            "app.py",
            "from flask import Flask\n\n\ndef create_app():\n    app = Flask(__name__)\n\n    @app.route(\"/x\")\n    def handler():\n        return \"ok\"\n\n    return app\n",
        )]);
        let handler = fid(&cg, "handler").clone();
        let rec = cg
            .framework_entries
            .iter()
            .find(|r| r.handler == handler)
            .expect("nested route recorded");
        assert_eq!(rec.caller.name, "create_app");
    }

    #[test]
    fn python_decorated_factory_enclosing_caller_matches_canonical_function_id() {
        // The factory `make_app` is ITSELF decorated -- `CallGraph::functions`
        // keys decorated Python functions by the `decorated_definition`
        // WRAPPER range (the Functions query captures `decorated_definition`;
        // `all_functions_via_tree` filters out the inner `function_definition`
        // for a decorated function), so the nested registration's `caller`
        // FunctionId must match that SAME wrapper range exactly, or
        // `nav callees(make_app)` can never find the outgoing edge (F4).
        let cg = build_py(&[(
            "app.py",
            "from flask import Flask\n\napp = Flask(__name__)\n\n\n@some_decorator\ndef make_app():\n    @app.route(\"/x\")\n    def handler():\n        return \"ok\"\n    return app\n",
        )]);
        let canonical_make_app = cg
            .functions
            .get("make_app")
            .and_then(|v| v.first())
            .expect("make_app indexed")
            .clone();
        let handler = fid(&cg, "handler").clone();
        let rec = cg
            .framework_entries
            .iter()
            .find(|r| r.handler == handler)
            .expect("nested route recorded");
        assert_eq!(
            rec.caller, canonical_make_app,
            "caller FunctionId must match the canonical (decorated-wrapper) range"
        );
        assert_eq!(rec.caller.start_line, 6);
    }

    // ---- Express -----------------------------------------------------------

    #[test]
    fn express_named_handler_recorded_at_module_level() {
        let cg = build_js(&[(
            "app.js",
            "const express = require(\"express\");\nconst app = express();\n\nfunction handler(req, res) {}\n\napp.get(\"/x\", handler);\n",
        )]);
        let handler = fid(&cg, "handler").clone();
        let rec = cg
            .framework_entries
            .iter()
            .find(|r| r.handler == handler)
            .expect("express handler recorded");
        assert_eq!(rec.framework, "express");
        assert_eq!(rec.caller.name, MODULE_PSEUDO_CALLER_NAME);
        assert_eq!(rec.caller.start_line, 1);
        assert_eq!(rec.site.line, 6);
    }

    #[test]
    fn express_multi_arg_middleware_and_handler_both_recorded() {
        let cg = build_js(&[(
            "app.js",
            "const express = require(\"express\");\nconst app = express();\n\nfunction mw(req, res, next) {}\nfunction handler(req, res) {}\n\napp.get(\"/x\", mw, handler);\n",
        )]);
        let mw = fid(&cg, "mw").clone();
        let handler = fid(&cg, "handler").clone();
        assert!(
            cg.framework_entries.iter().any(|r| r.handler == mw),
            "middleware arg must be recorded"
        );
        assert!(
            cg.framework_entries.iter().any(|r| r.handler == handler),
            "handler arg must be recorded"
        );
    }

    #[test]
    fn express_shadowed_identifier_is_skipped_and_counted() {
        // `handler` is a PARAMETER of `setup`, locally shadowing the
        // top-level `handler` function -- the reference inside `setup` names
        // the local, not the free function, so it must be skipped even
        // though exactly one same-file function named `handler` exists.
        let cg = build_js(&[(
            "app.js",
            "const express = require(\"express\");\nconst app = express();\n\nfunction handler(req, res) {}\n\nfunction setup(handler) {\n    app.get(\"/x\", handler);\n}\n",
        )]);
        assert!(
            cg.framework_entries.is_empty(),
            "shadowed identifier must not be recorded"
        );
        assert_eq!(cg.framework_entry_unresolved_handlers, 1);
    }

    #[test]
    fn express_inline_arrow_arg_is_skipped_and_counted() {
        let cg = build_js(&[(
            "app.js",
            "const express = require(\"express\");\nconst app = express();\n\napp.get(\"/x\", (req, res) => {});\n",
        )]);
        assert!(cg.framework_entries.is_empty());
        assert_eq!(cg.framework_entry_unresolved_handlers, 1);
    }

    #[test]
    fn express_zero_match_identifier_is_skipped_and_counted() {
        let cg = build_js(&[(
            "app.js",
            "const express = require(\"express\");\nconst app = express();\n\napp.get(\"/x\", missingHandler);\n",
        )]);
        assert!(cg.framework_entries.is_empty());
        assert_eq!(cg.framework_entry_unresolved_handlers, 1);
    }

    #[test]
    fn express_multi_match_identifier_is_skipped_and_counted() {
        // Two same-named `handler` function declarations in the same file
        // (annex-B block-scoped `function` inside `if`) -- ambiguous, must
        // not record, must count.
        let cg = build_js(&[(
            "app.js",
            "const express = require(\"express\");\nconst app = express();\n\nfunction handler(req, res) {}\n\nif (true) {\n    function handler() {}\n}\n\napp.get(\"/x\", handler);\n",
        )]);
        assert_eq!(cg.framework_entry_unresolved_handlers, 1);
        assert!(!cg
            .framework_entries
            .iter()
            .any(|r| r.framework == "express"));
    }

    #[test]
    fn express_registration_inside_setup_function_has_enclosing_caller() {
        let cg = build_js(&[(
            "app.js",
            "const express = require(\"express\");\nconst app = express();\n\nfunction handler(req, res) {}\n\nfunction setup() {\n    app.get(\"/x\", handler);\n}\n",
        )]);
        let handler = fid(&cg, "handler").clone();
        let rec = cg
            .framework_entries
            .iter()
            .find(|r| r.handler == handler)
            .expect("recorded");
        assert_eq!(rec.caller.name, "setup");
        assert_ne!(rec.caller.name, MODULE_PSEUDO_CALLER_NAME);
    }

    // ---- F1: bare-binding-only identifier resolution ------------------------

    #[test]
    fn express_identifier_matching_method_only_is_unresolved() {
        // `handler` exists ONLY as a class METHOD in this file -- a bare
        // identifier reference (`app.get("/x", handler)`) can never mean a
        // method (methods need a receiver), so this must be unresolved, not
        // a false edge to `C.handler`.
        let cg = build_js(&[(
            "app.js",
            "const express = require(\"express\");\nconst app = express();\n\nclass C {\n    handler() {}\n}\n\napp.get(\"/x\", handler);\n",
        )]);
        assert!(
            !cg.framework_entries
                .iter()
                .any(|r| r.framework == "express"),
            "method-only same-name match must not mint a framework_entry edge"
        );
        assert_eq!(cg.framework_entry_unresolved_handlers, 1);
    }

    #[test]
    fn express_identifier_matching_prefers_bare_binding_over_same_name_method() {
        // A top-level function AND a same-name class method both named
        // `handler` -- the bare-binding filter must exclude the method,
        // leaving exactly one match (the top-level function) instead of
        // dropping the identifier for false ambiguity.
        let cg = build_js(&[(
            "app.js",
            "const express = require(\"express\");\nconst app = express();\n\nfunction handler(req, res) {}\n\nclass C {\n    handler() {}\n}\n\napp.get(\"/x\", handler);\n",
        )]);
        let free_handler = cg
            .functions
            .get("handler")
            .expect("handler name indexed")
            .iter()
            .find(|f| !cg.method_owners.contains_key(*f))
            .expect("free function handler must exist")
            .clone();
        let rec = cg
            .framework_entries
            .iter()
            .find(|r| r.framework == "express")
            .expect("express route recorded against the free function");
        assert_eq!(rec.handler, free_handler);
        assert_eq!(cg.framework_entry_unresolved_handlers, 0);
    }

    #[test]
    fn express_identifier_matching_object_property_arrow_only_is_unresolved() {
        // M1 (a): `handler` exists ONLY as an object-literal property arrow
        // (`{ handler: () => {} }`) -- `languages::function_name` Pattern 2
        // still infers the name `handler` for it (so it enters
        // `CallGraph::functions`), but a bare `handler` identifier can never
        // reach a `pair` value (only `api.handler` can), so this must be
        // unresolved, not a false edge into the object property.
        let cg = build_js(&[(
            "app.js",
            "const express = require(\"express\");\nconst app = express();\n\nconst api = { handler: () => {} };\n\napp.get(\"/x\", handler);\n",
        )]);
        assert!(
            !cg.framework_entries
                .iter()
                .any(|r| r.framework == "express"),
            "object-property arrow must not mint a framework_entry edge"
        );
        assert_eq!(cg.framework_entry_unresolved_handlers, 1);
    }

    #[test]
    fn express_identifier_matching_member_expression_assignment_only_is_unresolved() {
        // M1 (b): `handler` exists ONLY as `exports.handler = () => {}` --
        // Pattern 5 name inference (`languages::function_name`) names it
        // `handler` via the assignment's member-expression LHS property, but
        // a bare `handler` identifier can never reach `exports.handler`
        // (only the qualified reference can), so this must be unresolved.
        let cg = build_js(&[(
            "app.js",
            "const express = require(\"express\");\nconst app = express();\n\nexports.handler = () => {};\n\napp.get(\"/x\", handler);\n",
        )]);
        assert!(
            !cg.framework_entries
                .iter()
                .any(|r| r.framework == "express"),
            "member-expression-LHS assignment must not mint a framework_entry edge"
        );
        assert_eq!(cg.framework_entry_unresolved_handlers, 1);
    }

    #[test]
    fn express_identifier_matching_variable_declarator_arrow_resolves() {
        // M1 (c) positive: `const handler = () => {}` -- Pattern 1 name
        // inference (bound via a `variable_declarator`) IS a genuine bare
        // binding (a bare `handler` reference really does reach it), so it
        // must keep resolving under the rewritten allow-list-based
        // `is_bare_binding_function`.
        let cg = build_js(&[(
            "app.js",
            "const express = require(\"express\");\nconst app = express();\n\nconst handler = (req, res) => {};\n\napp.get(\"/x\", handler);\n",
        )]);
        let handler = fid(&cg, "handler").clone();
        assert!(
            cg.framework_entries.iter().any(|r| r.handler == handler),
            "variable-declarator-bound arrow must still resolve"
        );
        assert_eq!(cg.framework_entry_unresolved_handlers, 0);
    }

    // ---- F2: Express receiver local-shadow guard ----------------------------

    #[test]
    fn express_receiver_shadowed_by_enclosing_parameter_is_skipped_and_counted() {
        // `app` is REBOUND as a parameter of `setup`, shadowing the
        // module-level `const app = express()` -- the receiver in
        // `app.get("/x", handler)` inside `setup` names the non-grounded
        // PARAMETER, not the express instance, so this must not mint an
        // edge even though `app` is a recognized express receiver name
        // file-wide.
        let cg = build_js(&[(
            "app.js",
            "const express = require(\"express\");\nconst app = express();\n\nfunction handler(req, res) {}\n\nfunction setup(app) {\n    app.get(\"/x\", handler);\n}\n",
        )]);
        assert!(
            cg.framework_entries.is_empty(),
            "shadowed receiver must not be recorded"
        );
        assert_eq!(cg.framework_entry_unresolved_handlers, 1);
    }

    #[test]
    fn express_receiver_not_shadowed_when_setup_does_not_rebind_app() {
        // Non-shadowed control: `setup` does NOT take an `app` parameter, so
        // the receiver inside it is still the module-level express
        // instance -- must record normally. (Distinct from the pre-existing
        // `express_registration_inside_setup_function_has_enclosing_caller`
        // in that it asserts the unresolved counter stays at 0.)
        let cg = build_js(&[(
            "app.js",
            "const express = require(\"express\");\nconst app = express();\n\nfunction handler(req, res) {}\n\nfunction setup() {\n    app.get(\"/x\", handler);\n}\n",
        )]);
        let handler = fid(&cg, "handler").clone();
        assert!(
            cg.framework_entries.iter().any(|r| r.handler == handler),
            "non-shadowed receiver must still record"
        );
        assert_eq!(cg.framework_entry_unresolved_handlers, 0);
    }

    // ---- M2: direct-constructor receiver local-shadow guard -----------------

    #[test]
    fn express_direct_constructor_receiver_shadowed_by_enclosing_parameter_is_skipped_and_counted()
    {
        // `express` is REBOUND as a parameter of `setup`, shadowing the
        // module-level `express` import -- `express()` inside `setup` calls
        // the non-grounded PARAMETER, not the express factory, so this must
        // not mint an edge even though the receiver is a direct-constructor
        // form (`express().get(...)`), which the old
        // `express_receiver_identifier_name` returned `None` for and so
        // never shadow-checked at all.
        let cg = build_js(&[(
            "app.js",
            "const express = require(\"express\");\nconst app = express();\n\nfunction handler(req, res) {}\n\nfunction setup(express) {\n    express().get(\"/x\", handler);\n}\n",
        )]);
        assert!(
            cg.framework_entries.is_empty(),
            "direct constructor receiver shadowed by enclosing param must not record"
        );
        assert_eq!(cg.framework_entry_unresolved_handlers, 1);
    }

    #[test]
    fn express_direct_constructor_receiver_not_shadowed_at_module_level_still_records() {
        // Non-shadowed control: a module-level direct-constructor receiver
        // (no enclosing function to shadow `express` at all) must still
        // record normally.
        let cg = build_js(&[(
            "app.js",
            "const express = require(\"express\");\nconst app = express();\n\nfunction handler(req, res) {}\n\nexpress().get(\"/x\", handler);\n",
        )]);
        let handler = fid(&cg, "handler").clone();
        assert!(
            cg.framework_entries.iter().any(|r| r.handler == handler),
            "non-shadowed module-level direct constructor receiver must still record"
        );
        assert_eq!(cg.framework_entry_unresolved_handlers, 0);
    }

    // ---- M3: shadow guard must see anonymous enclosing scopes ---------------

    #[test]
    fn express_receiver_shadowed_by_anonymous_iife_parameter_is_skipped_and_counted() {
        // `app` is REBOUND as a parameter of an ANONYMOUS IIFE (no
        // inferable name, so it can never be a `FunctionId` and is
        // invisible to the FunctionId-keyed `js_ts_function_locals` index)
        // -- the old shadow guard walked `enclosing_chain`, which only ever
        // contains NAMED enclosing functions, so this shadow was missed
        // entirely and a false edge was recorded.
        let cg = build_js(&[(
            "app.js",
            "const express = require(\"express\");\nconst app = express();\n\nfunction handler(req, res) {}\n\n(function (app) {\n    app.get(\"/x\", handler);\n})(express());\n",
        )]);
        assert!(
            cg.framework_entries.is_empty(),
            "anonymous-scope receiver shadow must not be recorded"
        );
        assert_eq!(cg.framework_entry_unresolved_handlers, 1);
    }

    #[test]
    fn express_receiver_not_shadowed_by_anonymous_iife_without_param_records_at_module_caller() {
        // Non-shadowed control: the anonymous IIFE does NOT rebind `app`, so
        // the receiver inside it is still the module-level express
        // instance -- must record normally. Because the wrapper is
        // anonymous, `enclosing` is still `None` for it (name inference
        // doesn't apply to an un-bound IIFE), so the caller stays the
        // `<module>` pseudo-caller -- verifies the M3 AST-walk rewrite left
        // the enclosing-CALLER determination for anonymous wrappers
        // unchanged, only the shadow/binding collection moved.
        let cg = build_js(&[(
            "app.js",
            "const express = require(\"express\");\nconst app = express();\n\nfunction handler(req, res) {}\n\n(function () {\n    app.get(\"/x\", handler);\n})();\n",
        )]);
        let handler = fid(&cg, "handler").clone();
        let rec = cg
            .framework_entries
            .iter()
            .find(|r| r.handler == handler)
            .expect("non-shadowed anonymous-wrapper registration must still record");
        assert_eq!(rec.caller.name, MODULE_PSEUDO_CALLER_NAME);
        assert_eq!(cg.framework_entry_unresolved_handlers, 0);
    }

    // ---- M-A (codex re-review of fix wave 2): caller attribution walks past
    // anonymous scopes to the nearest NAMED enclosing function ---------------

    #[test]
    fn express_registration_inside_anonymous_iife_nested_in_named_function_attributes_outer_function(
    ) {
        // The registration's DEEPEST enclosing function/method node is an
        // anonymous IIFE with no `FunctionId` -- the pre-fix
        // `enclosing_function(site_line)` (a top-down smallest-containing-
        // node search) stopped at that deepest node and returned `None`,
        // misattributing the caller to `<module>` even though `setup` truly
        // encloses the registration. The fix walks the call node's ACTUAL
        // AST ancestor chain and skips past anonymous function-like
        // ancestors to the nearest one that can be named.
        let cg = build_js(&[(
            "app.js",
            "const express = require(\"express\");\nconst app = express();\n\nfunction handler(req, res) {}\n\nfunction setup() {\n    (() => {\n        app.get(\"/x\", handler);\n    })();\n}\n",
        )]);
        let handler = fid(&cg, "handler").clone();
        let rec = cg
            .framework_entries
            .iter()
            .find(|r| r.handler == handler)
            .expect("registration inside the IIFE nested in setup() must still record");
        assert_eq!(rec.caller.name, "setup");
        assert_ne!(rec.caller.name, MODULE_PSEUDO_CALLER_NAME);
    }

    #[test]
    fn express_registration_inside_top_level_iife_with_no_named_ancestor_attributes_module() {
        // Control: a top-level IIFE with NO named function anywhere in its
        // ancestor chain must still attribute to the `<module>` pseudo-
        // caller, unchanged by the M-A ancestor-walk rewrite.
        let cg = build_js(&[(
            "app.js",
            "const express = require(\"express\");\nconst app = express();\n\nfunction handler(req, res) {}\n\n(() => {\n    app.get(\"/x\", handler);\n})();\n",
        )]);
        let handler = fid(&cg, "handler").clone();
        let rec = cg
            .framework_entries
            .iter()
            .find(|r| r.handler == handler)
            .expect("top-level IIFE registration must still record");
        assert_eq!(rec.caller.name, MODULE_PSEUDO_CALLER_NAME);
    }

    // ---- M-B (codex re-review of fix wave 2): `require("express").Router()`
    // constructor grounding-id peel ------------------------------------------

    #[test]
    fn express_require_dot_router_constructor_receiver_shadowed_by_enclosing_parameter_is_skipped_and_counted(
    ) {
        // `require` is REBOUND as a parameter of `setup`, shadowing the
        // global Node.js `require` -- `require("express").Router()` inside
        // `setup` calls the non-grounded PARAMETER, not the real module
        // loader, so this must not mint an edge. The grounding-identifier
        // peel previously only handled a bare-identifier member-expression
        // object (`express.Router()`); a require-CALL object
        // (`require("express").Router()`) fell through and returned `None`,
        // silently skipping the shadow check entirely (M-B).
        let cg = build_js(&[(
            "app.js",
            "const express = require(\"express\");\nconst app = express();\n\nfunction handler(req, res) {}\n\nfunction setup(require) {\n    require(\"express\").Router().get(\"/x\", handler);\n}\n",
        )]);
        assert!(
            cg.framework_entries.is_empty(),
            "shadowed require-constructor receiver must not be recorded"
        );
        assert_eq!(cg.framework_entry_unresolved_handlers, 1);
    }

    #[test]
    fn express_require_dot_router_constructor_receiver_not_shadowed_at_module_level_still_records()
    {
        // Non-shadowed control: a module-level
        // `require("express").Router().get(...)` (no enclosing function to
        // shadow `require` at all) must still record normally.
        let cg = build_js(&[(
            "app.js",
            "const express = require(\"express\");\nconst app = express();\n\nfunction handler(req, res) {}\n\nrequire(\"express\").Router().get(\"/x\", handler);\n",
        )]);
        let handler = fid(&cg, "handler").clone();
        assert!(
            cg.framework_entries.iter().any(|r| r.handler == handler),
            "non-shadowed module-level require-constructor receiver must still record"
        );
        assert_eq!(cg.framework_entry_unresolved_handlers, 0);
    }

    // ---- F3: method-aware arg positioning for `app.use(...)` ----------------

    #[test]
    fn express_use_single_identifier_arg_is_recorded() {
        // `app.use(loggerFn)` -- no path argument at all; arg 0 IS the
        // handler. The uniform "always skip arg 0" rule used to drop this
        // entirely.
        let cg = build_js(&[(
            "app.js",
            "const express = require(\"express\");\nconst app = express();\n\nfunction loggerFn(req, res, next) {}\n\napp.use(loggerFn);\n",
        )]);
        let logger_fn = fid(&cg, "loggerFn").clone();
        let rec = cg
            .framework_entries
            .iter()
            .find(|r| r.handler == logger_fn)
            .expect("app.use(loggerFn) must record the single identifier arg");
        assert_eq!(rec.site.line, 6);
    }

    #[test]
    fn express_use_with_path_arg_still_skips_the_path() {
        // `app.use('/api', mw)` -- arg 0 IS a string path, so the existing
        // skip-arg-0 behavior must still apply; only `mw` is recorded.
        let cg = build_js(&[(
            "app.js",
            "const express = require(\"express\");\nconst app = express();\n\nfunction mw(req, res, next) {}\n\napp.use(\"/api\", mw);\n",
        )]);
        let mw = fid(&cg, "mw").clone();
        assert!(
            cg.framework_entries.iter().any(|r| r.handler == mw),
            "mw must be recorded"
        );
        assert_eq!(
            cg.framework_entries
                .iter()
                .filter(|r| r.framework == "express")
                .count(),
            1,
            "the path argument itself must not be recorded as a handler"
        );
    }

    #[test]
    fn express_use_identifier_router_var_fails_bare_binding_resolution() {
        // `app.use(routerVar)` where `routerVar` is bound to
        // `express.Router()` -- arg 0 is method-aware-scanned as a handler
        // candidate, but `routerVar` is never a function/method definition
        // at all, so it fails resolution (0 matches) and must not record.
        let cg = build_js(&[(
            "app.js",
            "const express = require(\"express\");\nconst app = express();\nconst routerVar = express.Router();\n\napp.use(routerVar);\n",
        )]);
        assert!(
            !cg.framework_entries
                .iter()
                .any(|r| r.framework == "express"),
            "a router/config identifier must not resolve to a handler"
        );
        assert_eq!(cg.framework_entry_unresolved_handlers, 1);
    }

    // ---- Non-target-language guard -----------------------------------------

    #[test]
    fn non_target_language_go_file_never_mints() {
        let mut map = BTreeMap::new();
        map.insert(
            "main.go".to_string(),
            ParsedFile::parse("main.go", "package main\n\nfunc main() {}\n", Go).unwrap(),
        );
        let cg = CallGraph::build(&map);
        assert!(cg.framework_entries.is_empty());
        assert_eq!(cg.framework_entry_unresolved_handlers, 0);
    }
}
