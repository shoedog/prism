//! Call graph construction from parsed files.
//!
//! Builds both forward (caller→callee) and reverse (callee→caller) call graphs
//! across all parsed files. Used by barrier slice, spiral slice, vertical slice,
//! circular slice, and 3D slice.

use crate::ast::ParsedFile;
use crate::name_resolution::graph::ScopeGraph;
use crate::name_resolution::rust_populator::{enclosing_scope, populate_rust, RustCrateConfig};
use crate::name_resolution::types::{ScopeId, ScopeKind};
use crate::resolution::go::concrete_receiver::GoConcreteReceiverRoute;
use crate::resolution_identity::{resolve_type_path_to_type_scope, TypeKey};
use rayon::prelude::*;
use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::PathBuf;

#[cfg(test)]
#[path = "call_graph_anon_prep_tests.rs"]
mod anon_prep_tests;
mod boundary;
#[path = "call_graph_profile_initializers.rs"]
mod init_profile;
mod js_ts_facts;
mod level4;
mod python_import_proofs;
pub(crate) use python_import_proofs::{
    python_imported_class_proof_keys, python_imported_class_route,
};

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

#[cfg(test)]
mod go_level3_callback_tests;

#[cfg(test)]
#[path = "call_graph/promoted_snapshot_consult_test.rs"]
mod promoted_snapshot_consult_test;

#[cfg(test)]
#[path = "call_graph/js_relative_candidates_test.rs"]
mod js_relative_candidates_test;

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
    /// node. See `crate::call_graph::rust::macro_args`.
    MacroArg,
}

/// A call site: where a function is called from.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct CallSite {
    pub caller: FunctionId,
    pub callee_name: String,
    /// Source expression name used to recover arguments for a synthetic call.
    /// Ordinary source sites keep `None` and use `callee_name` directly.
    #[serde(default)]
    pub source_callee_name: Option<String>,
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
    /// True when a simple JS/TS receiver identifier is bound by a parameter,
    /// function-scoped `var`, or reaching lexical declaration at this call.
    /// Derived at extraction time and excluded from `cmp_key`.
    #[serde(default)]
    pub receiver_lexically_bound: bool,
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
    /// True only for Go function-literal parameter recovery added on this P17
    /// branch (the parameter was deliberately unrecoverable on main). Serialized
    /// for CPG/sidecar parity; excluded from `cmp_key` like the other derived
    /// receiver provenance fields.
    #[serde(default)]
    pub receiver_newly_recovered: bool,
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
    /// Exact target for a proven derived call site. Go B1 Level-3 callback
    /// construction sets this while ordinary source sites leave it empty.
    #[serde(default)]
    pub pre_resolved_target: Option<FunctionId>,
    /// JS/JSX/TSX: the site is a JSX element (`<X/>`), not a call expression.
    /// Excluded from cmp_key. Only span-verified wrapped export targets consult it.
    #[serde(default)]
    pub jsx_element: bool,
    /// S1b-3b (SPEC §3.6, §3.8 (8)): the JS/TS static lexical binding of this site's callee,
    /// when it is a plain identifier (call `function` field or JSX `name`). `Unchecked` is the
    /// default for every other language, synthetic and indirect sites, and non-identifier
    /// callees. Excluded from `cmp_key`.
    #[serde(default)]
    pub local_binding: JsLocalBinding,
}

/// S1b-3b (SPEC §3.6): what a JS/TS unqualified callee statically denotes at its site.
#[derive(Debug, Clone, Default, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum JsLocalBinding {
    /// Not computed: another language, or a synthetic or non-identifier callee.
    #[default]
    Unchecked,
    /// The binding holds exactly this callable.
    Callable(crate::ast::JsTerminal),
    /// S1b-4: one unwritten program namespace import, proved at this site.
    NamespaceImport { module_path: String },
    /// May-call or unresolved alias: candidate generation retains the named contract.
    MayCall(String),
    /// No single proven in-file callable, by reason (`unbound` and `import` included).
    Unproven(String),
    /// Owner OQ1 (Option K): an unproven position keeps base behavior, counted by reason.
    Position(String),
    /// Callable terminal of a followed const chain; enables textual-path stand-down.
    AliasCallable(crate::ast::JsTerminal),
    /// Program import terminal of a const chain; its export proof decides the target.
    AliasImport { local: String },
    /// Namespace-member terminal of a const chain; requires the member export proof.
    AliasNamespaceMember { module_path: String, member: String },
    /// Returned terminal: hook identity, selected result path, and the fallback reason.
    /// Anonymous admission also checks the hook and return-proof dependency environments.
    Returned {
        hook: crate::ast::JsFollow,
        path: crate::ast::js_ts::returns::JsReturnPath,
        kept: String,
    },
    /// #598: a callee that names an export of an in-repo module by relative `require`: `member`
    /// is the exported member, or `None` for the module value itself (`m(..)`). The export
    /// facts decide the target; where they prove none, the site behaves as it did without this
    /// variant (`MayCall("alias")` for a bare callee, `Unchecked` for a member callee).
    RequireModule {
        module_path: String,
        member: Option<String>,
    },
}

impl JsLocalBinding {
    /// The binder followed this callee to something the alias or returned-value route resolves
    /// (slice L aliases and slice V returned values).
    pub fn is_followed(&self) -> bool {
        matches!(
            self,
            JsLocalBinding::AliasCallable(_)
                | JsLocalBinding::AliasImport { .. }
                | JsLocalBinding::AliasNamespaceMember { .. }
                | JsLocalBinding::Returned { .. }
                | JsLocalBinding::RequireModule { .. }
        )
    }
}

/// Per-file JS/TS facts extracted together: export facts by file, parameter/local binding names
/// and slice V's returned-value facts, each by function.
type JsTsResolutionFacts = (
    BTreeMap<String, crate::ast::js_ts::exports::JsExportFacts>,
    BTreeMap<FunctionId, BTreeSet<String>>,
    BTreeMap<FunctionId, crate::ast::js_ts::returns::JsReturnFacts>,
);

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

    pub module_path: String,
    /// The original member name (before alias); `None` for module imports.
    pub member: Option<String>,

    pub kind: ImportBindingKind,
    /// False if poisoned by wildcard or re-bound by another top-level binding.
    pub eligible: bool,
}

/// The kind of import binding.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ImportBindingKind {
    /// `from mod import func` / `from mod import func as f`
    MemberImport,
    /// `import mod`
    ModuleImport,
    /// Python `import mod as m`; distinct from an unaliased dotted import
    /// because the alias binds directly to the complete module path.
    AliasedModuleImport,
    /// `from mod import *`
    WildcardImport,
}

pub(crate) enum PythonImportedClassRoute {
    NotImported,
    Proven {
        defining_file: String,
        owner: String,
    },
    Blocked,
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
    pub js_paths_snapshot: crate::resolution::js_ts::paths_snapshot::JsPathsSnapshot,
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
            js_paths_snapshot: Default::default(),
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
    /// Number of unique synthetic Go B1 Level-3 callback sites installed.
    /// Unsupported languages and callback shapes remain fail-closed at zero.
    #[serde(default)]
    pub level3_indirect_resolved: usize,
    #[serde(default)]
    pub(crate) go_level3_b1_telemetry: go::callback::GoLevel3Telemetry,
    /// Strict callable parameter signatures for non-test Go free functions.
    /// Whole-program derived with interface dispatch; direct subsets leave it empty.
    #[serde(default)]
    pub(crate) go_callback_parameters: go::callback::GoCallbackParameterIndex,
    /// Strict canonical signatures for non-test Go free-function declarations.
    /// Whole-program derived with interface dispatch; direct subsets leave it empty.
    #[serde(default)]
    pub(crate) go_free_function_signatures: go::callback::GoFreeFunctionSignatureIndex,
    /// Exact per-file effective Go import paths retained for Level-3 occurrence proof.
    #[serde(default)]
    pub(crate) go_file_import_paths: BTreeMap<String, String>,
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
    /// Rust `Self::f` allow-list proofs (impl syntax, self-type headers, trait defaults).
    #[serde(default)]
    pub(crate) rust_self_proofs: crate::resolution::rust::self_calls::ProofIndex,
    /// Method FunctionId -> owner class definition node byte span (start, end).
    /// Py/JS/TS only; the self-receiver class identity.
    #[serde(default)]
    pub method_class_span: BTreeMap<FunctionId, (usize, usize)>,
    /// Method FunctionIds whose line-based identity maps to multiple class spans.
    /// These fail open to owner lookup because the class span is not trustworthy.
    #[serde(default)]
    pub method_class_span_ambiguous: BTreeSet<FunctionId>,
    /// JS/TS class methods declared with the `static` modifier. Recovered value
    /// receivers may target only direct instance methods.
    #[serde(default)]
    pub js_ts_static_methods: BTreeSet<FunctionId>,
    /// Methods whose instance slot has a visible override or accessor shape.
    #[serde(default)]
    pub js_ts_unproven_instance_methods: BTreeSet<FunctionId>,
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
    /// Complete-snapshot imported alias proofs. Replaced, never unioned, on merge.
    #[serde(default)]
    pub js_ts_imported_props: js_ts::props::ImportedPropsProofs,
    /// Disabled integration: ephemeral authority, absent from ordinary builds.
    #[serde(skip)]
    pub(crate) executable_owner:
        Option<std::sync::Arc<crate::executable_owner::integration::InstalledOwner>>,
    /// Refusal-only origin marker survives generic mutation, never grants proof.
    #[serde(skip)]
    pub(crate) owner_ephemeral: bool,
    /// Complete indexed Python initializer files proven syntactically inert.
    #[serde(default)]
    pub python_inert_initializers: BTreeSet<String>,
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
    /// P1 proven non-relative modules; empty for convention-only builds.
    #[serde(default)]
    pub js_ts_path_modules: BTreeMap<(String, String), (String, String)>,
    /// Alias-only export tables. Every hop uses P1 occupancy/Node10 proof;
    /// the config path partitions callers by their owning project's options.
    #[serde(default)]
    pub js_ts_path_exports: BTreeMap<
        String,
        BTreeMap<String, BTreeMap<String, crate::ast::js_ts::exports::ResolvedJsExport>>,
    >,
    /// P4: JS/TS raw (per-file, un-resolved) export facts — default exports,
    /// named export lists (incl. renames), exported const-arrow/
    /// function-expression declarations, CommonJS assignments, and re-export
    /// records. Incrementally maintained per file like `import_bindings`.
    #[serde(default)]
    pub js_ts_exports: BTreeMap<String, crate::ast::js_ts::exports::JsExportFacts>,
    /// P4: whole-program resolved JS/TS export facts — file -> exported name
    /// -> the concrete `(file, local_name)` it refers to after following
    /// re-export chains (depth-bounded, cycle-safe). Derived from
    /// `js_ts_exports`; recomputed by `apply_js_export_resolution` the same
    /// way `interface_impls`/`go_registrations` are (whole-program derived —
    /// a barrel's resolution can depend on an unchanged file elsewhere).
    /// S1b-4 namespace-only export projection (unspanned means keep base).
    #[serde(default)]
    pub js_ts_namespace_exports:
        BTreeMap<String, BTreeMap<String, crate::ast::js_ts::exports::ResolvedJsExport>>,
    #[serde(default)]
    pub js_ts_resolved_exports:
        BTreeMap<String, BTreeMap<String, crate::ast::js_ts::exports::ResolvedJsExport>>,
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
    /// Source-backed anonymous proof evidence; partial builds refresh it from all current files.
    #[serde(default)]
    pub(crate) js_anonymous: crate::resolution::js_ts::anonymous::ProofIndex,
    #[serde(default)]
    pub js_ts_return_facts: BTreeMap<FunctionId, crate::ast::js_ts::returns::JsReturnFacts>,
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
    pub go_file_profiles: BTreeMap<String, crate::resolution::go::build_profile::GoBuildProfile>,
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
    pub(crate) go_module_graph: go::module_graph::GoModuleGraphTelemetry,
    /// Loaded Go files whose effective import path was proven.
    #[serde(default)]
    pub(crate) go_import_path_proven_files: usize,
    /// Loaded Go files whose effective import path was not proven.
    #[serde(default)]
    pub(crate) go_import_path_unproven_files: usize,
    /// Fail-closed reason histogram for unproven loaded Go files.
    #[serde(default)]
    pub(crate) go_import_path_unproven_reasons: BTreeMap<String, usize>,
    /// Canonical signature leaves whose visible alias declarations expanded.
    #[serde(default)]
    pub go_alias_expanded: usize,
    /// Alias leaves rejected before Exact dispatch, grouped by stable reason.
    #[serde(default)]
    pub go_alias_unresolved: BTreeMap<String, usize>,
    /// Owner/profile-keyed promoted-selector facts, serialized for cache parity
    /// and consulted by deferred concrete promoted-selector routing.
    #[serde(default)]
    go_promoted_selector_snapshot:
        crate::resolution::go::promoted_snapshot::GoPromotedSelectorSnapshot,
    /// P10 build-time S2 consult decisions. Whole-program rematerialized with
    /// receiver keys; runtime S4/P5 decisions travel on ResolutionOutcome.
    #[serde(default)]
    pub go_owner_identity_partition:
        crate::resolution::go::owner_partition::GoOwnerPartitionTelemetry,
    /// P10 build-time S2 decisions keyed by their source call site. `call-stats`
    /// coalesces this with the same site's runtime S4/P5/direct-method decision
    /// so `affected_sites` is a cardinality, not a count of pipeline stages.
    #[serde(default)]
    pub(crate) go_owner_identity_partition_sites: BTreeMap<
        crate::resolution::go::owner_partition::GoOwnerPartitionSiteKey,
        crate::resolution::go::owner_partition::GoOwnerPartitionTelemetry,
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
    pub macro_arg_facts: BTreeMap<String, rust::macro_args::MacroArgFacts>,
    /// P8 F1 fix (BLOCKER, codex re-review): the repo-wide macro shadow set
    /// (`call_graph::rust::macro_args::collect_macro_shadow_set`), narrowed to its
    /// intersection with `TRANSPARENT_ARG_MACROS`
    /// (`call_graph::rust::macro_args::transparent_shadow_intersection`) — the only part
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
    pub go_return_types: go::receiver_index::GoReturnTypes,
    /// Go files containing at least one dot import. Kept separate from
    /// `imports` because `.` is evidence that a bare type may be externally
    /// supplied, not a qualifier that can be resolved by name.
    #[serde(default)]
    pub go_dot_import_files: BTreeSet<String>,
    /// Terminal receiver-prerequisite drops, grouped by stable reason. This is
    /// rebuilt alongside receiver rematerialization and is diagnostic only.
    #[serde(default)]
    pub go_receiver_prereq_drops: BTreeMap<String, usize>,
    /// P11 S2/P10: clause-keyed per-declaration struct snapshots. Field
    /// presence/absence and raw type remain attached to the defining file so
    /// receiver recovery can filter build profiles before requiring one value.
    #[serde(default)]
    pub go_field_types: crate::resolution::go::owner_partition::GoStructDeclarations,
    /// P17: serialized declaration-kind proof used by the shared recovered-Go
    /// receiver route. Exact CPG hits must retain the same R1/R2/R3 verdict.
    #[serde(default)]
    pub go_declaration_kind_index: crate::resolution::go::concrete_receiver::GoDeclarationKindIndex,
    /// P17 compatibility snapshot showing a concrete selector is supplied by
    /// embedding. The shared route separately checks the existing owner-index
    /// promotion lane before deferring a newly recovered miss.
    #[serde(default)]
    pub go_promoted_concrete_selectors: BTreeSet<(crate::resolution::GoOwnerIdentity, String)>,
    /// P10 S4 interface and method declaration provenance, captured from the
    /// provider alongside `go_field_types` for exact consult-time routing.
    #[serde(default)]
    pub go_interface_declarations: crate::resolution::go::owner_partition::GoInterfaceDeclarations,
    #[serde(default)]
    pub go_method_declarations: crate::resolution::go::owner_partition::GoMethodDeclarations,
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
    pub go_package_vars: BTreeMap<(String, String), BTreeSet<go::receiver_index::GoTypedFact>>,
    /// P15a-fix1: the plain (import-path-free) `GoTypeProvider` extracted once
    /// during this build, retained (Arc-backed, cheap clone) so
    /// `CpgContext::build_registry` can reuse it instead of re-extracting.
    /// Only populated by full-context build paths whose provider was built
    /// from the same complete `files` map; never serialized (`#[serde(skip)]`)
    /// so cache bytes are unchanged. Scoped / cache-loaded / AST-only paths
    /// leave it `None` and construct their own.
    #[serde(skip)]
    pub(crate) shared_plain_go_provider: Option<crate::type_providers::go::GoTypeProvider>,
}

impl CallGraph {
    fn ambiguous_js_ts_call_owner_ids(parsed: &ParsedFile) -> BTreeSet<(String, usize, usize)> {
        if !matches!(
            parsed.language,
            crate::languages::Language::JavaScript
                | crate::languages::Language::TypeScript
                | crate::languages::Language::Tsx
        ) {
            return BTreeSet::new();
        }
        let mut counts = BTreeMap::new();
        for node in parsed.all_functions() {
            let Some(name) = parsed.language.function_name(&node) else {
                continue;
            };
            let (start, end) = parsed.node_line_range(&node);
            *counts
                .entry((parsed.node_text(&name).to_string(), start, end))
                .or_insert(0usize) += 1;
        }
        counts
            .into_iter()
            .filter_map(|(identity, count)| (count > 1).then_some(identity))
            .collect()
    }

    /// Create an empty call graph with no functions or edges.
    pub fn empty() -> Self {
        CallGraph {
            functions: BTreeMap::new(),
            calls: BTreeMap::new(),
            callers: BTreeMap::new(),
            param_slots_unknown: BTreeMap::new(),
            level3_indirect_resolved: 0,
            go_level3_b1_telemetry: Default::default(),
            go_callback_parameters: BTreeMap::new(),
            go_free_function_signatures: BTreeMap::new(),
            go_file_import_paths: BTreeMap::new(),
            static_functions: BTreeSet::new(),
            imports: BTreeMap::new(),
            methods: BTreeMap::new(),
            method_owners: BTreeMap::new(),
            rust_self_proofs: Default::default(),
            method_class_span: BTreeMap::new(),
            method_class_span_ambiguous: BTreeSet::new(),
            js_ts_static_methods: BTreeSet::new(),
            js_ts_unproven_instance_methods: BTreeSet::new(),
            class_bases: BTreeMap::new(),
            clean_class_spans: BTreeMap::new(),
            js_ts_imported_props: BTreeMap::new(),
            executable_owner: None,
            owner_ephemeral: false,
            python_inert_initializers: BTreeSet::new(),
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
            js_ts_path_modules: BTreeMap::new(),
            js_ts_path_exports: BTreeMap::new(),
            js_ts_exports: BTreeMap::new(),
            js_ts_resolved_exports: BTreeMap::new(),
            js_ts_namespace_exports: BTreeMap::new(),
            js_export_chain_unresolved: 0,
            js_export_barrel_conflicts: 0,
            js_ts_function_locals: BTreeMap::new(),
            js_anonymous: Default::default(),
            js_ts_return_facts: BTreeMap::new(),
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
            go_alias_expanded: 0,
            go_alias_unresolved: BTreeMap::new(),
            go_promoted_selector_snapshot: Default::default(),
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
            go_dot_import_files: BTreeSet::new(),
            go_receiver_prereq_drops: BTreeMap::new(),
            go_field_types: BTreeMap::new(),
            go_declaration_kind_index: BTreeMap::new(),
            go_promoted_concrete_selectors: BTreeSet::new(),
            go_interface_declarations: BTreeMap::new(),
            go_method_declarations: BTreeMap::new(),
            go_interface_live_types: BTreeSet::new(),
            go_embedded_interface_methods: BTreeMap::new(),
            go_package_vars: BTreeMap::new(),
            shared_plain_go_provider: None,
        }
    }

    /// Read-only access to the serialized promoted-selector foundation facts.
    pub fn go_promoted_selector_snapshot(
        &self,
    ) -> &crate::resolution::go::promoted_snapshot::GoPromotedSelectorSnapshot {
        &self.go_promoted_selector_snapshot
    }

    /// Build a lightweight call graph with only direct calls (Phases 1-2).
    ///
    /// Skips Phase 3 (indirect call resolution: function pointers, dispatch
    /// tables, parameter-passed callbacks). Used by `CpgContext::build_scoped()`
    /// to quickly identify which files are in the dependency neighborhood of a
    /// diff, before committing to a full CPG build on the scoped subset.
    fn build_skeleton_inner(files: &BTreeMap<String, ParsedFile>) -> Self {
        let mut p = crate::profile::Stages::new("call_graph", "call_graph.skeleton.prelude");
        let mut functions: BTreeMap<String, Vec<FunctionId>> = BTreeMap::new();
        let mut calls: BTreeMap<FunctionId, BTreeSet<CallSite>> = BTreeMap::new();
        let mut callers: BTreeMap<String, Vec<CallSite>> = BTreeMap::new();
        let mut static_functions: BTreeSet<(String, String)> = BTreeSet::new();
        let mut methods: BTreeMap<(String, String), Vec<FunctionId>> = BTreeMap::new();
        let mut method_owners: BTreeMap<FunctionId, String> = BTreeMap::new();
        let mut method_class_span: BTreeMap<FunctionId, (usize, usize)> = BTreeMap::new();
        let mut method_class_span_ambiguous: BTreeSet<FunctionId> = BTreeSet::new();
        let mut js_ts_static_methods: BTreeSet<FunctionId> = BTreeSet::new();
        let mut js_ts_unproven_instance_methods = BTreeSet::new();
        let mut receiver_vars: BTreeMap<FunctionId, String> = BTreeMap::new();
        let mut method_facts: BTreeMap<FunctionId, MethodFacts> = BTreeMap::new();
        // Repo-wide macro-name shadow set (P8 F1 BLOCKER) — computed once
        // from the full `files` map, mirroring the existing per-build
        // whole-program-fact pattern (e.g. `extract_js_ts_resolution_facts`).
        let macro_shadow = rust::macro_args::collect_macro_shadow_set(files);
        let (go_file_profiles, go_build_profile_unparsed) =
            Self::extract_go_build_profiles(files.iter().map(|(p, f)| (p.as_str(), f)));

        p.next("call_graph.skeleton.definitions");
        // Phase 1: Collect all function definitions
        for (file_path, parsed) in files {
            for func_node in crate::anon_owner::owner_nodes(parsed) {
                if let Some(func_id) =
                    crate::anon_owner::emitted_identity(parsed, &func_node, file_path)
                {
                    let name = func_id.name.clone();
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
                    if parsed.js_ts_method_is_static(&func_node) {
                        js_ts_static_methods.insert(func_id.clone());
                    }
                    if parsed.js_ts_method_slot_unproven(&func_node) {
                        js_ts_unproven_instance_methods.insert(func_id.clone());
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

        p.next("call_graph.skeleton.calls");
        // Phase 2: Find all call sites within each function
        for (file_path, parsed) in files {
            let ambiguous_owner_ids = Self::ambiguous_js_ts_call_owner_ids(parsed);
            let named_claims =
                crate::anon_owner::named_claims(parsed, crate::anon_owner::EMIT_ANONYMOUS_OWNERS);
            let mut binding_cache = crate::ast::JsBindingCache::for_sites();
            for func_node in crate::anon_owner::owner_nodes(parsed) {
                let Some(caller_id) =
                    crate::anon_owner::emitted_identity(parsed, &func_node, file_path)
                else {
                    continue;
                };
                let (start, end) = (caller_id.start_line, caller_id.end_line);
                if ambiguous_owner_ids.contains(&(
                    caller_id.name.clone(),
                    caller_id.start_line,
                    caller_id.end_line,
                )) {
                    continue;
                }

                let all_lines: BTreeSet<usize> = (start..=end).collect();
                let call_sites = parsed.function_calls_with_spans_on_lines(
                    &func_node,
                    &all_lines,
                    &macro_shadow,
                );

                for meta in call_sites {
                    if !crate::anon_owner::may_mint_call(
                        parsed,
                        &func_node,
                        meta.start_byte,
                        meta.end_byte,
                        &named_claims,
                    ) {
                        continue;
                    }
                    let callee_name = meta.callee_name;
                    let start_byte = meta.start_byte;
                    let end_byte = meta.end_byte;
                    let line = meta.line;
                    let site = CallSite {
                        caller: caller_id.clone(),
                        callee_name: callee_name.clone(),
                        source_callee_name: None,
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
                        receiver_lexically_bound: false,
                        receiver_type: None,
                        receiver_owner_identity: None,
                        receiver_local_type_shadowed: false,
                        receiver_recovery: None,
                        receiver_materialized: false,
                        receiver_newly_recovered: false,
                        arg_count: None,
                        arg_spread: false,
                        receiver_outcome: None,
                        origin: meta.origin_override.unwrap_or(CallSiteOrigin::Source),
                        pre_resolved_target: None,
                        jsx_element: Self::jsx_element_at(parsed, start_byte, end_byte),
                        local_binding: Self::js_local_binding_at(
                            parsed,
                            start_byte,
                            end_byte,
                            &callee_name,
                            &mut binding_cache,
                        ),
                    };
                    calls
                        .entry(caller_id.clone())
                        .or_default()
                        .insert(site.clone());
                    callers.entry(callee_name).or_default().push(site);
                }
            }
        }

        p.next("call_graph.skeleton.assembly");
        // Skip Phase 3 (indirect call resolution) — skeleton only needs direct calls.

        CallGraph {
            functions,
            calls,
            callers,
            param_slots_unknown: Self::parameter_slots_unknown(files),
            level3_indirect_resolved: 0,
            go_level3_b1_telemetry: Default::default(),
            go_callback_parameters: BTreeMap::new(),
            go_free_function_signatures: BTreeMap::new(),
            go_file_import_paths: BTreeMap::new(),
            static_functions,
            imports: BTreeMap::new(),
            methods,
            method_owners,
            rust_self_proofs: Default::default(),
            method_class_span,
            method_class_span_ambiguous,
            js_ts_static_methods,
            js_ts_unproven_instance_methods,
            class_bases: BTreeMap::new(),
            clean_class_spans: BTreeMap::new(),
            js_ts_imported_props: BTreeMap::new(),
            executable_owner: None,
            owner_ephemeral: false,
            python_inert_initializers: BTreeSet::new(),
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
            js_ts_path_modules: BTreeMap::new(),
            js_ts_path_exports: BTreeMap::new(),
            js_ts_exports: BTreeMap::new(),
            js_ts_resolved_exports: BTreeMap::new(),
            js_ts_namespace_exports: BTreeMap::new(),
            js_export_chain_unresolved: 0,
            js_export_barrel_conflicts: 0,
            js_ts_function_locals: BTreeMap::new(),
            js_anonymous: crate::resolution::js_ts::anonymous::ProofIndex::build(files),
            js_ts_return_facts: BTreeMap::new(),
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
            go_alias_expanded: 0,
            go_alias_unresolved: BTreeMap::new(),
            go_promoted_selector_snapshot: Default::default(),
            go_owner_identity_partition: Default::default(),
            go_owner_identity_partition_sites: BTreeMap::new(),
            go_bare_value_ref_ambiguous: 0,
            property_getters: BTreeMap::new(),
            cached_property_getters: BTreeSet::new(),
            property_accesses: BTreeSet::new(),
            property_access_fanout_skips: 0,
            property_access_store_skips: 0,
            macro_arg_facts: BTreeMap::new(),
            macro_shadow_intersection: rust::macro_args::transparent_shadow_intersection(
                &macro_shadow,
            ),
            // P9: whole-program framework-entry state — left empty here, same
            // reason as the Go/Python whole-program facts above:
            // `build_skeleton` never computes whole-program derived state.
            framework_entries: BTreeSet::new(),
            framework_entry_unresolved_handlers: 0,
            go_return_types: BTreeMap::new(),
            go_dot_import_files: BTreeSet::new(),
            go_receiver_prereq_drops: BTreeMap::new(),
            go_field_types: BTreeMap::new(),
            go_declaration_kind_index: BTreeMap::new(),
            go_promoted_concrete_selectors: BTreeSet::new(),
            go_interface_declarations: BTreeMap::new(),
            go_method_declarations: BTreeMap::new(),
            go_interface_live_types: BTreeSet::new(),
            go_embedded_interface_methods: BTreeMap::new(),
            go_package_vars: BTreeMap::new(),
            shared_plain_go_provider: None,
        }
    }

    // The public `build*` constructors live in `call_graph/boundary.rs`: each
    // enters the large-stack build pool once, then calls this body.
    fn build_inner(
        files: &BTreeMap<String, ParsedFile>,
        receiver_config: &crate::resolution::ReceiverRecoveryConfig,
        scope_inputs: Option<&ScopeGraphBuildInputs>,
    ) -> Self {
        let mut profile = crate::profile::Stages::new("call_graph", "call_graph.prelude");
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
        let mut js_ts_static_methods: BTreeSet<FunctionId> = BTreeSet::new();
        let mut js_ts_unproven_instance_methods = BTreeSet::new();
        let mut receiver_vars: BTreeMap<FunctionId, String> = BTreeMap::new();
        let mut method_facts: BTreeMap<FunctionId, MethodFacts> = BTreeMap::new();
        // P8 F1 BLOCKER: full-files macro shadow set; Sync BTreeSet shared by parallel jobs.
        let macro_shadow = rust::macro_args::collect_macro_shadow_set(files);
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
        // Build structured imports before call-site classification so Python can
        // retain only eligible module-scope member-import receiver types.
        let (import_bindings, module_bindings) = Self::extract_all_import_bindings(files);
        let indexed_files: BTreeSet<String> = files.keys().cloned().collect();
        let (class_bases, clean_class_spans) = Self::build_class_facts(files);
        let python_inert_initializers = python_inert_initializers(files);
        let proven_python_imported_receiver_types = python_imported_receiver_types(
            &import_bindings,
            &indexed_files,
            &clean_class_spans,
            &python_inert_initializers,
        );

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
            js_ts_static_methods: Vec<FunctionId>,
            js_ts_unproven_instance_methods: Vec<FunctionId>,
        }

        profile.next("call_graph.definitions");
        // Phase 1: Collect all function definitions per file in parallel, then
        // flatten serially in file order to preserve insertion order.
        let per_file_functions: Vec<FileFunctions> = crate::build_pool::per_item(&ordered_files)
            .map(|entry| {
                let (file_path, parsed) = *entry;
                let mut file_functions = Vec::new();
                let mut file_static_functions = Vec::new();
                let mut file_js_ts_static_methods = Vec::new();
                let mut file_js_ts_unproven_instance_methods = Vec::new();

                for func_node in crate::anon_owner::owner_nodes(parsed) {
                    if let Some(func_id) =
                        crate::anon_owner::emitted_identity(parsed, &func_node, file_path)
                    {
                        let name = func_id.name.clone();
                        let (owner, trait_key, recv_var, class_span) =
                            Self::method_metadata(parsed, &func_node);
                        let facts = Self::method_facts(parsed, &func_node);
                        if parsed.js_ts_method_is_static(&func_node) {
                            file_js_ts_static_methods.push(func_id.clone());
                        }
                        if parsed.js_ts_method_slot_unproven(&func_node) {
                            file_js_ts_unproven_instance_methods.push(func_id.clone());
                        }
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
                    js_ts_static_methods: file_js_ts_static_methods,
                    js_ts_unproven_instance_methods: file_js_ts_unproven_instance_methods,
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
            js_ts_static_methods.extend(file_functions.js_ts_static_methods);
            js_ts_unproven_instance_methods.extend(file_functions.js_ts_unproven_instance_methods);
        }

        struct FileCallSites {
            file_path: String,
            call_sites: Vec<(FunctionId, CallSite)>,
            macro_arg_facts: rust::macro_args::MacroArgFacts,
        }

        profile.next("call_graph.calls");
        // Collect parallel call sites in file order.
        let per_file_calls: Vec<FileCallSites> = crate::build_pool::per_item(&ordered_files)
            .map(|entry| {
                let (file_path, parsed) = *entry;
                let mut file_call_sites = Vec::new();
                let mut file_macro_arg_facts = rust::macro_args::MacroArgFacts::default();
                let file_imports_ref = imports.get(file_path);
                let proven_imported_receiver_types =
                    proven_python_imported_receiver_types.get(file_path);
                let ambiguous_owner_ids = Self::ambiguous_js_ts_call_owner_ids(parsed);
                let named_claims = crate::anon_owner::named_claims(
                    parsed,
                    crate::anon_owner::EMIT_ANONYMOUS_OWNERS,
                );
                let mut binding_cache = crate::ast::JsBindingCache::for_sites();

                for func_node in crate::anon_owner::owner_nodes(parsed) {
                    let Some(caller_id) =
                        crate::anon_owner::emitted_identity(parsed, &func_node, file_path)
                    else {
                        continue;
                    };
                    let (start, end) = (caller_id.start_line, caller_id.end_line);
                    if ambiguous_owner_ids.contains(&(
                        caller_id.name.clone(),
                        caller_id.start_line,
                        caller_id.end_line,
                    )) {
                        continue;
                    }

                    let _go_walk = crate::ast::go_receiver_walk::Session::new(parsed, func_node);
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
                        if !crate::anon_owner::may_mint_call(
                            parsed,
                            &func_node,
                            meta.start_byte,
                            meta.end_byte,
                            &named_claims,
                        ) {
                            continue;
                        }
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
                            proven_imported_receiver_types,
                        });
                        let recovered = classification.recovered.as_ref();
                        let receiver_newly_recovered = recovered.is_some()
                            && qualifier.as_deref().is_some_and(|receiver| {
                                parsed.go_func_literal_parameter_binds_receiver(
                                    &func_node, receiver, start_byte,
                                )
                            });
                        let receiver_lexically_bound = parsed
                            .js_ts_receiver_lexically_bound_at_call(&func_node, meta.receiver_node);
                        let site = CallSite {
                            caller: caller_id.clone(),
                            callee_name: callee_name.clone(),
                            source_callee_name: None,
                            line,
                            kind: meta.kind_override.unwrap_or_else(|| {
                                Self::call_kind_at(parsed, start_byte, end_byte)
                            }),
                            start_byte,
                            end_byte,
                            qualifier,
                            receiver_lexically_bound,
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
                            receiver_newly_recovered,
                            arg_count: meta.arg_count,
                            arg_spread: meta.arg_spread,
                            receiver_outcome: None,
                            origin: meta.origin_override.unwrap_or(CallSiteOrigin::Source),
                            pre_resolved_target: None,
                            jsx_element: Self::jsx_element_at(parsed, start_byte, end_byte),
                            local_binding: Self::js_local_binding_at(
                                parsed,
                                start_byte,
                                end_byte,
                                &callee_name,
                                &mut binding_cache,
                            ),
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

        profile.next("call_graph.merge");
        let mut macro_arg_facts: BTreeMap<String, rust::macro_args::MacroArgFacts> =
            BTreeMap::new();
        for file_calls in &per_file_calls {
            if file_calls.macro_arg_facts != rust::macro_args::MacroArgFacts::default() {
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

        profile.next("call_graph.js_ts_facts");
        let (js_ts_exports, js_ts_function_locals, js_ts_return_facts) =
            Self::extract_js_ts_resolution_facts(files);

        profile.next("call_graph.parameter_slots");
        let param_slots_unknown = Self::parameter_slots_unknown(files);
        let initial = init_profile::collect(files, scope_inputs, &macro_shadow, &mut profile);
        let mut cg = CallGraph {
            functions,
            calls,
            callers,
            param_slots_unknown,
            level3_indirect_resolved: 0,
            go_level3_b1_telemetry: Default::default(),
            go_callback_parameters: BTreeMap::new(),
            go_free_function_signatures: BTreeMap::new(),
            go_file_import_paths: BTreeMap::new(),
            static_functions,
            imports,
            methods,
            method_owners,
            rust_self_proofs: Default::default(),
            method_class_span,
            method_class_span_ambiguous,
            js_ts_static_methods,
            js_ts_unproven_instance_methods,
            class_bases,
            clean_class_spans,
            js_ts_imported_props: initial.props,
            executable_owner: None,
            owner_ephemeral: false,
            python_inert_initializers,
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
            scope_graph: initial.scope,
            import_bindings,
            module_bindings,
            indexed_files,
            js_ts_path_modules: BTreeMap::new(),
            js_ts_path_exports: BTreeMap::new(),
            js_ts_exports,
            js_ts_resolved_exports: BTreeMap::new(),
            js_ts_namespace_exports: BTreeMap::new(),
            js_export_chain_unresolved: 0,
            js_export_barrel_conflicts: 0,
            js_ts_function_locals,
            js_anonymous: initial.proofs,
            js_ts_return_facts,
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
            go_alias_expanded: 0,
            go_alias_unresolved: BTreeMap::new(),
            go_promoted_selector_snapshot: Default::default(),
            go_owner_identity_partition: Default::default(),
            go_owner_identity_partition_sites: BTreeMap::new(),
            go_bare_value_ref_ambiguous: 0,
            property_getters: BTreeMap::new(),
            cached_property_getters: BTreeSet::new(),
            property_accesses: BTreeSet::new(),
            property_access_fanout_skips: 0,
            property_access_store_skips: 0,
            macro_arg_facts,
            macro_shadow_intersection: initial.macro_shadow,
            // P9: whole-program framework-entry state — left empty here,
            // same reason as the Go/Python whole-program facts below:
            // `apply_framework_entries` populates it after `functions` /
            // `js_ts_function_locals` are already complete.
            framework_entries: BTreeSet::new(),
            framework_entry_unresolved_handlers: 0,
            go_return_types: BTreeMap::new(),
            go_dot_import_files: BTreeSet::new(),
            go_receiver_prereq_drops: BTreeMap::new(),
            go_field_types: BTreeMap::new(),
            go_declaration_kind_index: BTreeMap::new(),
            go_promoted_concrete_selectors: BTreeSet::new(),
            go_interface_declarations: BTreeMap::new(),
            go_method_declarations: BTreeMap::new(),
            go_interface_live_types: BTreeSet::new(),
            go_embedded_interface_methods: BTreeMap::new(),
            go_package_vars: BTreeMap::new(),
            shared_plain_go_provider: None,
        };
        profile.next("call_graph.rust_receiver_refresh");
        cg.refresh_rust_receiver_state(files);
        // P15a-fix1: interface dispatch runs BEFORE the shared plain provider
        // is constructed. Dispatch and embedding promotion are mutually
        // independent — each reads only `files` (+ scope inputs for dispatch)
        // and writes disjoint CallGraph fields, and every downstream consumer
        // (`apply_go_func_value_fields` / `apply_go_registrations` /
        // receiver rematerialization) runs after BOTH regardless of their
        // relative order. Constructing the import-path-aware provider first
        // guarantees at most ONE full `GoTypeData` is alive at any moment
        // (the plain one is built only after the dispatch construction has
        // been dropped).
        profile.next("call_graph.go_dispatch_embedding");
        cg.apply_go_interface_dispatch_with_scope_inputs(files, scope_inputs);
        // P15a: extract the plain Go type data ONCE per build and share it
        // between embedding promotion and receiver rematerialization (both use
        // the import-path-free `from_parsed_files` extraction). A cheap
        // Arc-backed clone is retained on the graph so `CpgContext::
        // build_registry` reuses the same extraction instead of a fourth
        // construction on the no-cache navigation path.
        let shared_go_provider = Self::plain_go_provider_for_build(files);
        cg.shared_plain_go_provider = shared_go_provider.clone();
        cg.apply_go_embedding_promotion_with_provider(files, shared_go_provider.as_ref());
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
        cg.apply_go_receiver_indices_with_provider(
            files,
            receiver_config,
            shared_go_provider.as_ref(),
        );
        // P7: python property-access state — needs the complete method_owners
        // / method_class_span / class_bases indexes already populated above,
        // so it runs last, same rationale as the Go passes.
        profile.next("call_graph.python_properties");
        cg.apply_python_property_accesses(files);
        // P9: framework-entry state (Flask/FastAPI/Express route
        // registrations) needs the complete `functions`/
        // `js_ts_function_locals` index already populated above (Express
        // identifier-arg resolution reads both) — runs after the Go/Python
        // whole-program passes, same rationale.
        profile.next("call_graph.framework_entries");
        cg.apply_framework_entries(files);
        // P4: JS/TS export-fact resolution (re-export chains/barrels) is ALSO
        // whole-program derived, same rationale as the Go passes above.
        profile.next("call_graph.apply_js_paths");
        cg.apply_js_paths(scope_inputs);
        profile.next("call_graph.apply_js_export_resolution");
        cg.apply_js_export_resolution();
        // Recompute the remaining whole-program indirect passes after all
        // resolution facts are installed, including Go B1 Level-3 callbacks.
        profile.next("call_graph.indirect");
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

    fn extract_js_ts_resolution_facts(files: &BTreeMap<String, ParsedFile>) -> JsTsResolutionFacts {
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

    fn extract_js_ts_resolution_facts_from_iter<'a, I>(files: I) -> JsTsResolutionFacts
    where
        I: IntoIterator<Item = (&'a String, &'a ParsedFile)>,
    {
        js_ts_facts::extract(files)
    }

    /// P4: whole-program resolution of JS/TS export facts — recomputed from
    /// `js_ts_exports` (raw, per-file) into `js_ts_resolved_exports`
    /// (following re-export chains/barrels, depth-bounded, cycle-safe), the
    /// same "recompute from scratch on the merged graph" pattern as
    /// `apply_go_registrations`/`apply_go_interface_dispatch`: a barrel's
    /// resolution can depend on an unchanged file elsewhere, so it can't be
    /// incrementally patched per changed file.
    pub(crate) fn apply_js_paths(&mut self, inputs: Option<&ScopeGraphBuildInputs>) {
        self.js_ts_path_modules.clear();
        self.js_ts_path_exports.clear();
        let Some(inputs) = inputs else {
            return;
        };
        let mut resolver =
            crate::resolution::js_ts::paths::Resolver::new(&inputs.js_paths_snapshot);
        for (file, bindings) in &self.import_bindings {
            if !matches!(
                crate::languages::Language::from_path(file),
                Some(
                    crate::languages::Language::JavaScript
                        | crate::languages::Language::TypeScript
                        | crate::languages::Language::Tsx
                )
            ) {
                continue;
            }
            for b in bindings {
                if b.eligible
                    && b.kind == ImportBindingKind::MemberImport
                    && self
                        .js_ts_exports
                        .get(file)
                        .is_some_and(|exports| exports.esm_named_imports.contains(&b.local))
                {
                    if let Some(target) =
                        resolver.resolve(file, &b.module_path, &self.indexed_files)
                    {
                        self.js_ts_path_modules.insert(
                            (file.clone(), b.module_path.clone()),
                            (
                                target,
                                resolver.project(file).expect("resolved caller project"),
                            ),
                        );
                    }
                }
            }
        }
        for project in self
            .js_ts_path_modules
            .values()
            .map(|(_, project)| project.clone())
            .collect::<BTreeSet<_>>()
        {
            let resolve_module =
                |from: &str, spec: &str| resolver.hop(&project, from, spec, &self.indexed_files);
            let roots = self
                .js_ts_path_modules
                .values()
                .filter_map(|(file, owner)| (*owner == project).then_some(file))
                .collect::<BTreeSet<_>>();
            let resolution = crate::ast::js_ts::exports::resolve_js_exports_for(
                &self.js_ts_exports,
                &resolve_module,
                roots,
            );
            self.js_ts_path_exports.insert(project, resolution.resolved);
        }
    }

    pub(crate) fn resolve_js_ts_member_module(&self, module: &str, caller: &str) -> Option<String> {
        let relative = module.trim();
        if is_js_ts_relative_specifier(relative) {
            resolve_js_ts_relative_module(relative, caller, &self.indexed_files)
        } else {
            self.js_ts_path_modules
                .get(&(caller.into(), module.into()))
                .map(|(target, _)| target.clone())
        }
    }

    pub fn apply_js_export_resolution(&mut self) {
        self.clear_js_export_resolution();
        let indexed_files = &self.indexed_files;
        let resolve_module = |from: &str, module_path: &str| {
            resolve_js_ts_relative_module(module_path, from, indexed_files)
        };
        let resolution =
            crate::ast::js_ts::exports::resolve_js_exports(&self.js_ts_exports, &resolve_module);
        self.js_ts_namespace_exports = crate::ast::js_ts::exports::resolve_js_namespace_exports(
            &self.js_ts_exports,
            &resolve_module,
        );
        self.js_ts_resolved_exports = resolution.resolved;
        self.js_export_chain_unresolved = resolution.chain_unresolved;
        self.js_export_barrel_conflicts = resolution.barrel_conflicts;
    }

    fn clear_js_export_resolution(&mut self) {
        self.js_ts_resolved_exports.clear();
        self.js_ts_namespace_exports.clear();
        self.js_export_chain_unresolved = 0;
        self.js_export_barrel_conflicts = 0;
    }

    /// Terminal owner proof for the bounded recovered-instance lane. Classes
    /// are consulted from raw direct exports, never the function/barrel table.
    pub(crate) fn js_ts_recovered_class_owner(
        &self,
        caller: &str,
        spelling: &str,
        typed_parameter: bool,
    ) -> Option<(String, String)> {
        let imports: Vec<_> = self
            .import_bindings
            .get(caller)
            .into_iter()
            .flatten()
            .filter(|b| b.local == spelling)
            .collect();
        if let Some(type_import) = self
            .js_ts_exports
            .get(caller)
            .and_then(|facts| facts.type_only_imports.get(spelling))
        {
            if !typed_parameter || !imports.is_empty() {
                return None;
            }
            let (module, member) = type_import.as_ref()?;
            return self.js_ts_direct_exported_class_owner(caller, module, member);
        }
        if imports.is_empty() {
            let key = (caller.to_string(), spelling.to_string());
            return self.clean_class_spans.contains_key(&key).then_some(key);
        }
        if imports.len() != 1 {
            return None;
        }
        let binding = imports[0];
        if !binding.eligible
            || !self
                .js_ts_exports
                .get(caller)?
                .esm_named_imports
                .contains(spelling)
        {
            return None;
        }
        self.js_ts_direct_exported_class_owner(
            caller,
            &binding.module_path,
            binding.member.as_deref()?,
        )
    }

    fn js_ts_direct_exported_class_owner(
        &self,
        caller: &str,
        module: &str,
        member: &str,
    ) -> Option<(String, String)> {
        let candidates = js_ts_relative_module_candidates(module, caller, &self.indexed_files)?;
        if candidates.len() != 1 {
            return None;
        }
        let defining_file = &candidates[0];
        let facts = self.js_ts_exports.get(defining_file)?;
        if facts.conflicted.contains(member) {
            return None;
        }
        let crate::ast::js_ts::exports::JsExportTarget::Class(owner) = facts.named.get(member)?
        else {
            return None;
        };
        let key = (defining_file.clone(), owner.clone());
        self.clean_class_spans.contains_key(&key).then_some(key)
    }

    // -----------------------------------------------------------------------
    // Incremental cache support (Phase 2)
    // -----------------------------------------------------------------------

    /// Remove all entries originating from the given files.
    ///
    /// Used by incremental cache update: when a file changes, its call graph
    /// contributions are stripped out before fresh data is merged in.
    pub fn remove_files(&mut self, exclude: &BTreeSet<String>) {
        self.js_anonymous.remove_files(exclude);
        self.executable_owner = None;
        // Both are whole-program pass products and are repopulated from the
        // complete files map by `build_direct_subset` / `recompute_indirect_calls`.
        self.param_slots_unknown.clear();
        self.level3_indirect_resolved = 0;
        self.go_level3_b1_telemetry = Default::default();
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
        self.js_ts_static_methods
            .retain(|fid| !exclude.contains(&fid.file));
        self.js_ts_unproven_instance_methods
            .retain(|fid| !exclude.contains(&fid.file));
        self.class_bases.retain(|(f, _), _| !exclude.contains(f));
        self.clean_class_spans
            .retain(|(f, _), _| !exclude.contains(f));
        self.python_inert_initializers
            .retain(|f| !exclude.contains(f));
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
        self.js_ts_return_facts
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

        // Imported proofs depend on the complete input universe, not just callers.
        self.js_ts_imported_props.clear();

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
        self.executable_owner = None;
        self.owner_ephemeral |= other.owner_ephemeral;
        self.param_slots_unknown = other.param_slots_unknown;
        self.level3_indirect_resolved = other.level3_indirect_resolved;
        self.go_level3_b1_telemetry = other.go_level3_b1_telemetry;
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
        self.js_ts_static_methods.extend(other.js_ts_static_methods);
        self.js_ts_unproven_instance_methods
            .extend(other.js_ts_unproven_instance_methods);
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
        self.python_inert_initializers = other.python_inert_initializers;
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
        self.js_anonymous.merge(other.js_anonymous);
        self.js_ts_return_facts.extend(other.js_ts_return_facts);
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
        self.js_ts_imported_props = other.js_ts_imported_props;

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
        self.executable_owner = None;
        self.clear_indirect_calls();
        let (sites, level3_telemetry) = self.compute_indirect_call_sites(files, true);
        self.apply_indirect_call_sites(sites);
        self.level3_indirect_resolved = level3_telemetry.edges;
        self.go_level3_b1_telemetry = level3_telemetry;
    }

    fn clear_indirect_calls(&mut self) {
        self.level3_indirect_resolved = 0;
        self.go_level3_b1_telemetry = Default::default();
        for sites in self.calls.values_mut() {
            sites.retain(|site| site.origin != CallSiteOrigin::IndirectResolution);
        }
        self.calls.retain(|_, sites| !sites.is_empty());

        for sites in self.callers.values_mut() {
            sites.retain(|site| site.origin != CallSiteOrigin::IndirectResolution);
        }
        self.callers.retain(|_, sites| !sites.is_empty());
    }

    /// Audit seam for the structural rule test: the indirect records a rebuild would add with the
    /// textual-path stand-down on (`true`, the product) or forced off (`false`, main's behavior),
    /// computed on a copy with the held indirect records cleared first.
    #[doc(hidden)]
    pub fn audit_indirect_records(
        &self,
        files: &BTreeMap<String, ParsedFile>,
        stand_down: bool,
    ) -> Vec<(FunctionId, CallSite)> {
        let mut copy = self.clone();
        copy.executable_owner = None;
        copy.clear_indirect_calls();
        copy.compute_indirect_call_sites(files, stand_down).0
    }

    fn compute_indirect_call_sites(
        &self,
        files: &BTreeMap<String, ParsedFile>,
        stand_down: bool,
    ) -> (Vec<(FunctionId, CallSite)>, go::callback::GoLevel3Telemetry) {
        // Resolve indirect call sites (function pointer variables and dispatch
        // tables). Preserve the historical level ordering:
        // 1/2 local function pointer and array dispatch, 4 struct callbacks,
        // then 3 parameter-passed function pointers.
        let known_fn_names: BTreeSet<String> = self.functions.keys().cloned().collect();
        let mut extra_sites: Vec<(FunctionId, CallSite)> = Vec::new();

        for (caller_id, sites) in self
            .calls
            .iter()
            .filter(|(id, _)| crate::anon_owner::permits_legacy_indirect(id))
        {
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
                        crate::anon_owner::source_before(parsed, caller_id, site.start_byte)
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
        let level4_index = level4::build(files, &known_fn_names);

        let mut level4_sites: Vec<(FunctionId, CallSite)> = Vec::new();
        for (caller_id, sites) in self
            .calls
            .iter()
            .filter(|(id, _)| crate::anon_owner::permits_legacy_indirect(id))
        {
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

        // Level 3: Go bare free-function values passed to proven free-function HOFs.
        // Enumerate only original source sites; synthetic sites never donate evidence.
        let proof = go::callback::GoCallbackProofContext::new(
            files,
            &self.functions,
            &self.method_owners,
            &self.go_file_profiles,
            &self.go_dot_import_files,
            &self.go_file_import_paths,
        );
        let mut callback_slots_by_name: BTreeMap<String, BTreeSet<usize>> = BTreeMap::new();
        for (hof, parameters) in &self.go_callback_parameters {
            callback_slots_by_name
                .entry(hof.name.clone())
                .or_default()
                .extend(parameters.iter().map(|parameter| parameter.slot));
        }
        let mut telemetry = go::callback::GoLevel3Telemetry::default();
        let mut accepted_targets = BTreeSet::new();
        let mut level3_sites = BTreeSet::new();
        for (caller, sites) in &self.calls {
            let Some(parsed) = files.get(&caller.file) else {
                continue;
            };
            for site in sites {
                if site.origin != CallSiteOrigin::Source || site.kind != CallKind::Call {
                    continue;
                }
                let Some(candidate_slots) = callback_slots_by_name.get(&site.callee_name) else {
                    continue;
                };
                let arguments =
                    parsed.call_argument_texts_and_spans_at(site.start_byte, &site.callee_name);
                for slot in candidate_slots {
                    telemetry.candidates += 1;
                    let mut record = go::callback::GoLevel3SiteRecord {
                        inbound_caller: caller.clone(),
                        inbound_line: site.line,
                        inbound_start_byte: site.start_byte,
                        inbound_end_byte: site.end_byte,
                        hof_name: site.callee_name.clone(),
                        slot: *slot,
                        argument: arguments
                            .get(*slot)
                            .map(|(argument, _)| argument.trim().to_string()),
                        hof: None,
                        target: None,
                        callback_parameter: None,
                        invocation_spans: Vec::new(),
                        accepted: false,
                        drop_reason: None,
                    };
                    let result: Result<_, go::callback::GoLevel3DropReason> = (|| {
                        if parsed.language != crate::languages::Language::Go {
                            return Err(go::callback::GoLevel3DropReason::NonGoOrTest);
                        }
                        if site.arg_spread {
                            return Err(
                                go::callback::GoLevel3DropReason::MissingSlotOrArgumentIdentity,
                            );
                        }
                        let hof = proof.hof_at(caller, site)?;
                        record.hof = Some(hof.clone());
                        let resolved = self.resolve_call_site_full(site);
                        if resolved.resolved.len() != 1
                            || resolved.resolved[0].confidence
                                != crate::resolution::ResolutionConfidence::Exact
                            || resolved.resolved[0].target != &hof
                        {
                            return Err(go::callback::GoLevel3DropReason::HofNotSingletonExact);
                        }
                        telemetry.exact_inbound_sites += 1;
                        let parameter = self
                            .go_callback_parameters
                            .get(&hof)
                            .and_then(|parameters| {
                                parameters.iter().find(|parameter| parameter.slot == *slot)
                            })
                            .ok_or(go::callback::GoLevel3DropReason::MissingCallbackParameter)?;
                        record.callback_parameter = Some(parameter.name.clone());
                        let (argument, span) = arguments.get(*slot).ok_or(
                            go::callback::GoLevel3DropReason::MissingSlotOrArgumentIdentity,
                        )?;
                        let target =
                            proof.bare_free_function_at(caller, span.clone(), argument.trim())?;
                        record.target = Some(target.clone());
                        let target_signature = self
                            .go_free_function_signatures
                            .get(&target)
                            .ok_or(go::callback::GoLevel3DropReason::SignatureUnprovenOrMismatch)?;
                        if !go::callback::signatures_match_strict(
                            &parameter.signature,
                            target_signature,
                        ) {
                            return Err(
                                go::callback::GoLevel3DropReason::SignatureUnprovenOrMismatch,
                            );
                        }
                        let invocations =
                            proof.direct_parameter_invocations(&self.calls, &hof, parameter)?;
                        record.invocation_spans = invocations
                            .iter()
                            .map(|invocation| go::callback::GoLevel3InvocationSpan {
                                line: invocation.line,
                                start_byte: invocation.start_byte,
                                end_byte: invocation.end_byte,
                            })
                            .collect();
                        Ok((hof, target, parameter.clone(), invocations))
                    })(
                    );
                    match result {
                        Ok((hof, target, parameter, invocations)) => {
                            telemetry.accepted_inbound_sites += 1;
                            record.accepted = true;
                            accepted_targets.insert(target.clone());
                            for invocation in invocations {
                                let mut synthetic = Self::indirect_call_site(
                                    &hof,
                                    target.name.clone(),
                                    &invocation,
                                );
                                synthetic.source_callee_name = Some(parameter.name.clone());
                                synthetic.pre_resolved_target = Some(target.clone());
                                level3_sites.insert((hof.clone(), synthetic));
                            }
                        }
                        Err(reason) => {
                            record.drop_reason = Some(reason.as_str().to_string());
                            *telemetry
                                .drops
                                .entry(reason.as_str().to_string())
                                .or_default() += 1;
                        }
                    }
                    telemetry.sites.push(record);
                }
            }
        }
        telemetry.unique_targets = accepted_targets.len();
        telemetry.edges = level3_sites.len();
        extra_sites.extend(level3_sites);
        // Slice L stand-down: every level above ran unchanged, so each read of the Level-1/2
        // output (Level 4's `already_resolved`; Level 3 reads `self.calls`, not `extra_sites`)
        // saw main's population. Only now, on the finished output, are the records whose source
        // site is an alias site with a proven hit (the one predicate) removed, by exact site
        // identity (caller, start byte, end byte).
        if stand_down {
            let proven: BTreeSet<(&FunctionId, usize, usize)> = self
                .calls
                .iter()
                .flat_map(|(caller, sites)| {
                    sites
                        .iter()
                        .filter(|s| {
                            s.origin == CallSiteOrigin::Source && self.js_ts_alias_route_hit(s)
                        })
                        .map(move |s| (caller, s.start_byte, s.end_byte))
                })
                .collect();
            if !proven.is_empty() {
                extra_sites.retain(|(caller, record)| {
                    !proven.contains(&(caller, record.start_byte, record.end_byte))
                });
            }
        }
        debug_assert_eq!(
            telemetry.candidates,
            telemetry.accepted_inbound_sites + telemetry.drops.values().sum::<usize>()
        );
        (extra_sites, telemetry)
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
            source_callee_name: None,
            line: source_site.line,
            kind: CallKind::Call,
            // Carry the source call site span so same-line indirect dups do not collapse.
            start_byte: source_site.start_byte,
            end_byte: source_site.end_byte,
            qualifier: None,
            receiver_lexically_bound: false,
            receiver_type: None,
            receiver_owner_identity: None,
            receiver_local_type_shadowed: false,
            receiver_recovery: None,
            receiver_materialized: false,
            receiver_newly_recovered: false,
            arg_count: None,
            arg_spread: false,
            receiver_outcome: None,
            origin: CallSiteOrigin::IndirectResolution,
            pre_resolved_target: None,
            jsx_element: false,
            // E10 non-goal: indirect-resolution sites never carry a lexical binding.
            local_binding: JsLocalBinding::Unchecked,
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
                let class_node = if let Some(declaration) = parsed.js_ts_named_exported_class(child)
                {
                    declaration
                } else if child.kind() == "decorated_definition" {
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
                    if class_matches == 1
                        && binding_count == 1
                        && !parsed.js_ts_module_value_written(class_name)
                    {
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
                "export_statement" => {
                    if let Some(declaration) = child.child_by_field_name("declaration") {
                        count_binding_stmt(declaration, source, lang, counts);
                    }
                }
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
        self.rust_self_proofs = crate::resolution::rust::self_calls::ProofIndex::build(files);
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
        let _profile = crate::profile::Phase::serial("scope_graph");
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
        let macro_shadow = rust::macro_args::collect_macro_shadow_set(files);
        let ordered: Vec<(&FunctionId, &BTreeSet<CallSite>)> = self.calls.iter().collect();
        crate::build_pool::per_item(&ordered)
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
        let Some(fn_node) = crate::anon_owner::node_for_id(parsed, caller) else {
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
        let macro_shadow = rust::macro_args::collect_macro_shadow_set(files);
        for (caller, sites) in &self.calls {
            let Some(parsed) = files.get(&caller.file) else {
                continue;
            };
            if !matches!(parsed.language, crate::languages::Language::Rust) {
                continue;
            }
            let Some(fn_node) = crate::anon_owner::node_for_id(parsed, caller) else {
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
        self.go_alias_expanded = 0;
        self.go_alias_unresolved.clear();
        self.go_callback_parameters.clear();
        self.go_free_function_signatures.clear();
        self.go_file_import_paths.clear();
        self.go_promoted_selector_snapshot = Default::default();
        debug_assert!(
            self.go_package_basenames.is_empty(),
            "dispatch clear must invalidate every Go package/import-path key"
        );
    }

    /// P15a: lazily construct the import-path-free Go provider for the whole
    /// build only when the file set contains Go files (non-Go repos pay
    /// nothing). Shared by embedding promotion + receiver rematerialization.
    pub(crate) fn plain_go_provider_for_build(
        files: &BTreeMap<String, ParsedFile>,
    ) -> Option<crate::type_providers::go::GoTypeProvider> {
        if files
            .values()
            .any(|p| p.language == crate::languages::Language::Go)
        {
            Some(crate::type_providers::go::GoTypeProvider::from_parsed_files(files))
        } else {
            None
        }
    }

    /// Recompute Go embedding promotions over `files` and write owner-index aliases.
    /// Idempotent: clears prior aliases first (incremental replace).
    pub fn apply_go_embedding_promotion(&mut self, files: &BTreeMap<String, ParsedFile>) {
        self.apply_go_embedding_promotion_with_provider(files, None);
    }

    /// Build-path variant that reuses a caller-constructed plain
    /// `GoTypeProvider` (`from_parsed_files`) instead of re-extracting the
    /// whole Go type data. `shared` MUST have been built from the same `files`
    /// map; `None` constructs one internally (public-API behavior).
    pub(crate) fn apply_go_embedding_promotion_with_provider(
        &mut self,
        files: &BTreeMap<String, ParsedFile>,
        shared: Option<&crate::type_providers::go::GoTypeProvider>,
    ) {
        // 1. Remove prior promoted aliases (preserving direct methods on the key).
        self.clear_promoted_embedding();
        if !files
            .values()
            .any(|p| p.language == crate::languages::Language::Go)
        {
            return;
        }
        let owned;
        let provider = match shared {
            Some(p) => p,
            None => {
                owned = crate::type_providers::go::GoTypeProvider::from_parsed_files(files);
                &owned
            }
        };
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
        self.go_file_import_paths = package_import_paths.paths.clone();
        let callback_indices =
            go::callback::extract_go_callback_indices(files, &package_import_paths.paths);
        self.go_callback_parameters = callback_indices.parameters;
        self.go_free_function_signatures = callback_indices.free_functions;
        let provider =
            crate::type_providers::go::GoTypeProvider::from_parsed_files_with_package_import_paths(
                files,
                &package_import_paths.paths,
            );
        let alias_telemetry = provider.alias_telemetry();
        self.go_alias_expanded = alias_telemetry.expanded;
        self.go_alias_unresolved = alias_telemetry.unresolved;
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
        let go_type_declaration_files = provider.go_type_declaration_files();
        self.go_promoted_selector_snapshot = crate::resolution::go::promoted_snapshot::build(
            files,
            &package_import_paths.paths,
            &self.go_file_profiles,
            &go_type_declaration_files,
            &self.go_field_types,
            &self.go_interface_declarations,
            &self.go_method_declarations,
        );
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
    ) -> go::module_graph::GoImportPathResolution {
        let Some(scope_inputs) = scope_inputs else {
            let unproven_files = files
                .values()
                .filter(|parsed| parsed.language == crate::languages::Language::Go)
                .count();
            let mut resolution = go::module_graph::GoImportPathResolution {
                unproven_files,
                ..Default::default()
            };
            if resolution.unproven_files > 0 {
                resolution.reasons.insert(
                    go::module_graph::GoImportPathReason::NoGoMod
                        .as_str()
                        .to_string(),
                    resolution.unproven_files,
                );
            }
            return resolution;
        };
        let mut graph = go::module_graph::GoModuleGraph::new(
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
        let selected = crate::resolution::go::owner_partition::select_struct_field(
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
        // src/ast/receiver_typing.rs — a real local-binding check, not a same-file/package
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
        self.go_dot_import_files.clear();
        self.go_receiver_prereq_drops.clear();
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
        self.apply_go_receiver_indices_with_provider(files, receiver_config, None);
    }

    /// Build-path variant that reuses a caller-constructed plain
    /// `GoTypeProvider` (see `apply_go_embedding_promotion_with_provider`).
    pub(crate) fn apply_go_receiver_indices_with_provider(
        &mut self,
        files: &BTreeMap<String, ParsedFile>,
        receiver_config: &crate::resolution::ReceiverRecoveryConfig,
        shared: Option<&crate::type_providers::go::GoTypeProvider>,
    ) {
        self.clear_go_receiver_indices();
        if !files
            .values()
            .any(|p| p.language == crate::languages::Language::Go)
        {
            return;
        }
        self.go_return_types = go::receiver_index::extract_go_return_types(files);
        self.go_package_vars = go::receiver_index::extract_go_package_vars(files);
        self.go_dot_import_files = files
            .iter()
            .filter(|(_, parsed)| {
                parsed.language == crate::languages::Language::Go && parsed.go_has_dot_import()
            })
            .map(|(path, _)| path.clone())
            .collect();
        let owned;
        let provider = match shared {
            Some(p) => p,
            None => {
                owned = crate::type_providers::go::GoTypeProvider::from_parsed_files(files);
                &owned
            }
        };
        let field_targets = provider.go_field_targets();
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
            let facts = go::receiver_index::GoReceiverFacts {
                return_types: &self.go_return_types,
                package_vars: &self.go_package_vars,
                field_types: &self.go_field_types,
                field_targets,
                package_basenames: &self.go_package_basenames,
                imports: &self.imports,
                go_file_profiles: &self.go_file_profiles,
                declaration_kinds: &self.go_declaration_kind_index,
                dot_import_files: &self.go_dot_import_files,
            };
            self.compute_go_receiver_updates(files, &facts, receiver_config)
        };
        for (caller, old_site, classification, evidence, prereq_drop) in updates {
            if let Some(reason) = prereq_drop {
                *self
                    .go_receiver_prereq_drops
                    .entry(reason.as_str().to_string())
                    .or_default() += 1;
            }
            let mut site_telemetry =
                crate::resolution::go::owner_partition::GoOwnerPartitionTelemetry::default();
            site_telemetry.observe(evidence, 1);
            self.go_owner_identity_partition.merge(site_telemetry);
            if site_telemetry.affected_sites() > 0 {
                self.go_owner_identity_partition_sites.insert(
                    crate::resolution::go::owner_partition::site_key(&old_site),
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
                            crate::anon_owner::node_for_id(parsed, &old_site.caller).map(
                                |func_node| {
                                    parsed.go_local_type_shadows(
                                        &func_node,
                                        ty,
                                        old_site.start_byte,
                                    )
                                },
                            )
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
            let function_literal_parameter_recovery = classification.recovered.is_some()
                && old_site.qualifier.as_deref().is_some_and(|receiver| {
                    files
                        .get(&old_site.caller.file)
                        .and_then(|parsed| {
                            crate::anon_owner::node_for_id(parsed, &old_site.caller).map(
                                |func_node| {
                                    parsed.go_func_literal_parameter_binds_receiver(
                                        &func_node,
                                        receiver,
                                        old_site.start_byte,
                                    )
                                },
                            )
                        })
                        .unwrap_or(false)
                });
            updated.receiver_newly_recovered =
                old_site.receiver_newly_recovered || function_literal_parameter_recovery;
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
                        site.receiver_newly_recovered = updated.receiver_newly_recovered;
                    }
                }
            }
        }
    }

    /// Parallel per-caller recompute, mirroring compute_rust_receiver_updates in calls order.
    pub(crate) fn compute_go_receiver_updates(
        &self,
        files: &BTreeMap<String, ParsedFile>,
        facts: &go::receiver_index::GoReceiverFacts<'_>,
        receiver_config: &crate::resolution::ReceiverRecoveryConfig,
    ) -> Vec<(
        FunctionId,
        CallSite,
        crate::resolution::ReceiverClassification,
        crate::resolution::go::owner_partition::GoPartitionEvidence,
        Option<go::receiver_index::GoReceiverPrereqDrop>,
    )> {
        use rayon::prelude::*;

        let classifier = receiver_config.classifier();
        let ordered: Vec<(&FunctionId, &BTreeSet<CallSite>)> = self.calls.iter().collect();
        crate::build_pool::per_item(&ordered)
            .copied()
            .map(|(caller, sites)| {
                self.go_receiver_updates_for_caller(
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
        &self,
        caller: &FunctionId,
        sites: &BTreeSet<CallSite>,
        files: &BTreeMap<String, ParsedFile>,
        classifier: &dyn crate::resolution::ReceiverClassifier,
        facts: &go::receiver_index::GoReceiverFacts<'_>,
        var_local: bool,
    ) -> Vec<(
        FunctionId,
        CallSite,
        crate::resolution::ReceiverClassification,
        crate::resolution::go::owner_partition::GoPartitionEvidence,
        Option<go::receiver_index::GoReceiverPrereqDrop>,
    )> {
        let mut out = Vec::new();
        let Some(parsed) = files.get(&caller.file) else {
            return out;
        };
        if parsed.language != crate::languages::Language::Go {
            return out;
        }
        let Some(fn_node) = crate::anon_owner::node_for_id(parsed, caller) else {
            return out;
        };
        let _go_walk = crate::ast::go_receiver_walk::Session::new(parsed, fn_node);
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
            let ctx = go::receiver_index::GoReceiverCtx {
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
            let (raw_classification, mut evidence, origin) =
                go::receiver_index::classify_go_receiver_expanded_with_partition(
                    &ctx, classifier, facts, var_local,
                );
            let (classification, screen_evidence, prereq_drop) =
                go::receiver_index::screen_go_receiver_prerequisites(
                    &ctx,
                    raw_classification,
                    origin,
                    facts,
                );
            evidence.merge(screen_evidence);
            let same_scope_reuse = parsed
                .go_same_scope_short_var_reuse_calls(&fn_node, qualifier, meta.start_byte)
                .is_ok_and(|calls| !calls.is_empty());
            if prereq_drop.is_none() && same_scope_reuse && !classification.proof_shadowed {
                let direct = classification.recovered.as_ref().is_some_and(|recovered| {
                    matches!(
                        self.go_concrete_receiver_route(
                            &recovered.static_type,
                            recovered.owner_identity.as_ref(),
                            false,
                            site.receiver_newly_recovered || site.receiver_type.is_none(),
                            &site.callee_name,
                            &caller.file,
                        ),
                        GoConcreteReceiverRoute::ConcreteDirect { .. }
                    )
                });
                if !direct {
                    // Wave 3 narrows the exception to the constructible failure:
                    // same-block `x, err := reset(x)` may restore an existing
                    // concrete own-method edge, but must not broaden R2 or any
                    // non-direct route that f16663e kept behind the R3 shadow
                    // bail. Skip the update entirely so every legacy CallSite
                    // field and drop bucket remains byte-identical.
                    continue;
                }
            }
            out.push((
                caller.clone(),
                site.clone(),
                classification,
                evidence,
                prereq_drop,
            ));
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
        BTreeMap<String, crate::resolution::go::build_profile::GoBuildProfile>,
        BTreeMap<String, usize>,
    ) {
        let mut profiles = BTreeMap::new();
        let mut unparsed = BTreeMap::new();
        for (path, parsed) in files {
            if parsed.language != crate::languages::Language::Go {
                continue;
            }
            let (profile, count) =
                crate::resolution::go::build_profile::extract_go_file_profile(path, parsed);
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
    fn build_direct_subset_inner(
        files: &BTreeMap<String, ParsedFile>,
        only_files: &BTreeSet<String>,
        receiver_config: &crate::resolution::ReceiverRecoveryConfig,
    ) -> Self {
        let mut p = crate::profile::Stages::new("call_graph", "call_graph.direct_subset.prelude");
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
        let mut js_ts_static_methods: BTreeSet<FunctionId> = BTreeSet::new();
        let mut js_ts_unproven_instance_methods = BTreeSet::new();
        let mut receiver_vars: BTreeMap<FunctionId, String> = BTreeMap::new();
        let mut method_facts: BTreeMap<FunctionId, MethodFacts> = BTreeMap::new();
        // Repo-wide macro-name shadow set (P8 F1 BLOCKER) — deliberately
        // scanned over the FULL `files` map (not `only_files`): a
        // `macro_rules!` def outside the changed subset must still shadow.
        let macro_shadow = rust::macro_args::collect_macro_shadow_set(files);
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

        // Build the subset's structured imports before call-site classification,
        // mirroring the full constructor.
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
        let indexed_files: BTreeSet<String> = files.keys().cloned().collect();
        // Build class facts from the complete files map so unchanged class owners
        // remain available after incremental merges.
        let (class_bases, clean_class_spans) = Self::build_class_facts(files);
        let python_inert_initializers = python_inert_initializers(files);
        let proven_python_imported_receiver_types = python_imported_receiver_types(
            &import_bindings_map,
            &indexed_files,
            &clean_class_spans,
            &python_inert_initializers,
        );

        p.next("call_graph.direct_subset.definitions");
        // Phase 1: Collect function definitions from subset.
        for (file_path, parsed) in files {
            if !only_files.contains(file_path) {
                continue;
            }
            for func_node in crate::anon_owner::owner_nodes(parsed) {
                if let Some(func_id) =
                    crate::anon_owner::emitted_identity(parsed, &func_node, file_path)
                {
                    let name = func_id.name.clone();
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
                    if parsed.js_ts_method_is_static(&func_node) {
                        js_ts_static_methods.insert(func_id.clone());
                    }
                    if parsed.js_ts_method_slot_unproven(&func_node) {
                        js_ts_unproven_instance_methods.insert(func_id.clone());
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

        p.next("call_graph.direct_subset.calls");
        // Phase 2: Find call sites from subset.
        let mut macro_arg_facts: BTreeMap<String, rust::macro_args::MacroArgFacts> =
            BTreeMap::new();
        for (file_path, parsed) in files {
            if !only_files.contains(file_path) {
                continue;
            }
            let mut file_macro_arg_facts = rust::macro_args::MacroArgFacts::default();
            let ambiguous_owner_ids = Self::ambiguous_js_ts_call_owner_ids(parsed);
            let named_claims =
                crate::anon_owner::named_claims(parsed, crate::anon_owner::EMIT_ANONYMOUS_OWNERS);
            let mut binding_cache = crate::ast::JsBindingCache::for_sites();
            for func_node in crate::anon_owner::owner_nodes(parsed) {
                let Some(caller_id) =
                    crate::anon_owner::emitted_identity(parsed, &func_node, file_path)
                else {
                    continue;
                };
                let (start, end) = (caller_id.start_line, caller_id.end_line);
                if ambiguous_owner_ids.contains(&(
                    caller_id.name.clone(),
                    caller_id.start_line,
                    caller_id.end_line,
                )) {
                    continue;
                }
                let _go_walk = crate::ast::go_receiver_walk::Session::new(parsed, func_node);
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
                let proven_imported_receiver_types =
                    proven_python_imported_receiver_types.get(file_path);

                for meta in call_sites {
                    if !crate::anon_owner::may_mint_call(
                        parsed,
                        &func_node,
                        meta.start_byte,
                        meta.end_byte,
                        &named_claims,
                    ) {
                        continue;
                    }
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
                        proven_imported_receiver_types,
                    });
                    let recovered = classification.recovered.as_ref();
                    let receiver_newly_recovered = recovered.is_some()
                        && qualifier.as_deref().is_some_and(|receiver| {
                            parsed.go_func_literal_parameter_binds_receiver(
                                &func_node, receiver, start_byte,
                            )
                        });
                    let receiver_lexically_bound = parsed
                        .js_ts_receiver_lexically_bound_at_call(&func_node, meta.receiver_node);
                    let site = CallSite {
                        caller: caller_id.clone(),
                        callee_name: callee_name.clone(),
                        source_callee_name: None,
                        line,
                        kind: meta
                            .kind_override
                            .unwrap_or_else(|| Self::call_kind_at(parsed, start_byte, end_byte)),
                        start_byte,
                        end_byte,
                        qualifier,
                        receiver_lexically_bound,
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
                        receiver_newly_recovered,
                        arg_count: meta.arg_count,
                        arg_spread: meta.arg_spread,
                        receiver_outcome: None,
                        origin: meta.origin_override.unwrap_or(CallSiteOrigin::Source),
                        pre_resolved_target: None,
                        jsx_element: Self::jsx_element_at(parsed, start_byte, end_byte),
                        local_binding: Self::js_local_binding_at(
                            parsed,
                            start_byte,
                            end_byte,
                            &callee_name,
                            &mut binding_cache,
                        ),
                    };
                    calls
                        .entry(caller_id.clone())
                        .or_default()
                        .insert(site.clone());
                    callers.entry(callee_name).or_default().push(site);
                }
            }
            if file_macro_arg_facts != rust::macro_args::MacroArgFacts::default() {
                macro_arg_facts.insert(file_path.clone(), file_macro_arg_facts);
            }
        }

        p.next("call_graph.direct_subset.resolution_facts");
        let (js_ts_exports, js_ts_function_locals, js_ts_return_facts) =
            Self::extract_js_ts_resolution_facts_from_iter(
                subset_files.iter().map(|(fp, parsed)| (fp, *parsed)),
            );

        CallGraph {
            functions,
            calls,
            callers,
            param_slots_unknown: Self::parameter_slots_unknown(files),
            level3_indirect_resolved: 0,
            go_level3_b1_telemetry: Default::default(),
            go_callback_parameters: BTreeMap::new(),
            go_free_function_signatures: BTreeMap::new(),
            go_file_import_paths: BTreeMap::new(),
            static_functions,
            imports,
            methods,
            method_owners,
            rust_self_proofs: Default::default(),
            method_class_span,
            method_class_span_ambiguous,
            js_ts_static_methods,
            js_ts_unproven_instance_methods,
            class_bases,
            clean_class_spans,
            js_ts_imported_props: js_ts::props::collect(files),
            executable_owner: None,
            owner_ephemeral: false,
            python_inert_initializers,
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
            js_ts_path_modules: BTreeMap::new(),
            js_ts_path_exports: BTreeMap::new(),
            js_ts_exports,
            // P4: whole-program resolved export facts — left empty here,
            // exactly like the Go whole-program state below:
            // `build_direct_subset` never computes whole-program facts
            // itself; the caller re-applies `apply_js_export_resolution` on
            // the merged graph.
            js_ts_resolved_exports: BTreeMap::new(),
            js_ts_namespace_exports: BTreeMap::new(),
            js_export_chain_unresolved: 0,
            js_export_barrel_conflicts: 0,
            js_ts_function_locals,
            js_anonymous: crate::resolution::js_ts::anonymous::ProofIndex::build(files),
            js_ts_return_facts,
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
            go_alias_expanded: 0,
            go_alias_unresolved: BTreeMap::new(),
            go_promoted_selector_snapshot: Default::default(),
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
            macro_shadow_intersection: rust::macro_args::transparent_shadow_intersection(
                &macro_shadow,
            ),
            // P9: whole-program framework-entry state — left empty here for
            // the same reason as the Go/Python whole-program facts above; the
            // caller re-applies `apply_framework_entries` on the merged
            // graph.
            framework_entries: BTreeSet::new(),
            framework_entry_unresolved_handlers: 0,
            go_return_types: BTreeMap::new(),
            go_dot_import_files: BTreeSet::new(),
            go_receiver_prereq_drops: BTreeMap::new(),
            go_field_types: BTreeMap::new(),
            go_declaration_kind_index: BTreeMap::new(),
            go_promoted_concrete_selectors: BTreeSet::new(),
            go_interface_declarations: BTreeMap::new(),
            go_method_declarations: BTreeMap::new(),
            go_interface_live_types: BTreeSet::new(),
            go_embedded_interface_methods: BTreeMap::new(),
            go_package_vars: BTreeMap::new(),
            shared_plain_go_provider: None,
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
        if parsed.language.function_name(func_node).is_none() {
            return (None, None, None, None);
        }
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

    /// Source-byte JS binding query; unsupported/synthetic callees remain Unchecked.
    pub(crate) fn js_local_binding_at<'a>(
        parsed: &'a ParsedFile,
        start_byte: usize,
        end_byte: usize,
        callee: &str,
        cache: &mut crate::ast::JsBindingCache<'a>,
    ) -> JsLocalBinding {
        use crate::ast::{JsBinding, JsFollow};
        use crate::languages::Language;
        if !matches!(
            parsed.language,
            Language::JavaScript | Language::TypeScript | Language::Tsx
        ) {
            return JsLocalBinding::Unchecked;
        }
        let ident = parsed
            .tree
            .root_node()
            .descendant_for_byte_range(start_byte, end_byte)
            .filter(|n| n.start_byte() == start_byte && n.end_byte() == end_byte)
            .and_then(|site| match site.kind() {
                "call_expression" => site.child_by_field_name("function"),
                "jsx_self_closing_element" | "jsx_opening_element" => {
                    site.child_by_field_name("name")
                }
                _ => None,
            });
        let Some(ident) = ident else {
            return JsLocalBinding::Unchecked;
        };
        let binding = if ident.kind() == "identifier" && parsed.node_text(&ident) == callee {
            let binding = parsed.js_ts_site_binding(ident, callee, cache);
            // Slice L: a `const` alias is followed to what it names; an alias that cannot be
            // followed keeps `Alias` (base behavior).
            if binding == JsBinding::Alias {
                match parsed.js_ts_follow_alias(ident, callee, cache) {
                    Some(JsFollow::Callable(t)) => return JsLocalBinding::AliasCallable(t),
                    Some(JsFollow::ImportLocal(local)) => {
                        return JsLocalBinding::AliasImport { local }
                    }
                    Some(JsFollow::NamespaceMember {
                        module_path,
                        member,
                    }) => {
                        return JsLocalBinding::AliasNamespaceMember {
                            module_path,
                            member,
                        }
                    }
                    None => {}
                }
                // #598: a top-level `require` of a relative module, or of a member of one.
                if let Some((module_path, member)) = parsed.js_ts_cjs_require_binding(ident, cache)
                {
                    return JsLocalBinding::RequireModule {
                        module_path,
                        member,
                    };
                }
            }
            // Slice V: a `const` taking its value from a call to one in-repo function `H` is
            // followed to `H`'s returned-value fact. A site the fact does not prove resolves
            // as `kept` says (the base reason).
            if matches!(binding, JsBinding::Alias | JsBinding::MayCall) {
                if let Some((hook, path)) = parsed.js_ts_follow_returned(ident, callee, cache) {
                    let kept = if binding == JsBinding::Alias {
                        "alias"
                    } else {
                        "may_call"
                    };
                    return JsLocalBinding::Returned {
                        hook,
                        path,
                        kept: kept.to_string(),
                    };
                }
            }
            binding
        } else if ident.kind() == "member_expression" {
            if let Some(binding) = parsed.js_ts_cjs_member_binding(ident, callee, cache) {
                return binding;
            }
            let qualifier = ident
                .child_by_field_name("object")
                .filter(|n| n.kind() == "identifier");
            let member = ident.child_by_field_name("property");
            let Some(q) =
                qualifier.filter(|_| member.is_some_and(|m| parsed.node_text(&m) == callee))
            else {
                return JsLocalBinding::Unchecked;
            };
            let Some((binding, module)) = parsed.js_ts_namespace_binding(q, cache) else {
                return JsLocalBinding::Unchecked;
            };
            if let Some(module_path) = module {
                return JsLocalBinding::NamespaceImport { module_path };
            }
            binding
        } else {
            return JsLocalBinding::Unchecked;
        };
        match binding {
            JsBinding::Callable(t) => JsLocalBinding::Callable(t),
            JsBinding::MayCall => JsLocalBinding::MayCall("may_call".to_string()),
            JsBinding::Alias => JsLocalBinding::MayCall("alias".to_string()),
            // Unbound or an import: no in-file function is the binding (R4 drops); the
            // import and repo-wide rungs keep base behavior.
            JsBinding::Import => JsLocalBinding::Unproven("import".to_string()),
            JsBinding::Refused(r) => JsLocalBinding::Unproven(r.to_string()),
            JsBinding::Unchecked(r) => JsLocalBinding::Position(r.to_string()),
        }
    }

    /// True iff the node spanning exactly these bytes is a JSX element tag (S1).
    fn jsx_element_at(parsed: &ParsedFile, start_byte: usize, end_byte: usize) -> bool {
        parsed
            .tree
            .root_node()
            .descendant_for_byte_range(start_byte, end_byte)
            .is_some_and(|n| {
                n.start_byte() == start_byte
                    && n.end_byte() == end_byte
                    && matches!(n.kind(), "jsx_self_closing_element" | "jsx_opening_element")
            })
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
    pub fn effective_source_callee_name(&self) -> &str {
        self.source_callee_name
            .as_deref()
            .unwrap_or(&self.callee_name)
    }

    fn cmp_key(
        &self,
    ) -> (
        &str,
        &str,
        Option<&str>,
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
            self.source_callee_name.as_deref(),
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
            // Eligibility records only occurrence-clean binding authority. R4c,
            // navigation, and Python imported receiver proof apply their own
            // import-kind and path-shape requirements below.
            binding.eligible = !matches!(binding.kind, ImportBindingKind::WildcardImport);
        }
    }
}

pub(crate) fn python_qualified_receiver_parts(receiver_type: &str) -> Option<(&str, &str)> {
    let (qualifier, owner) = receiver_type.rsplit_once('.')?;
    (qualifier.split('.').all(python_identifier) && python_identifier(owner))
        .then_some((qualifier, owner))
}

pub(crate) fn python_identifier(part: &str) -> bool {
    let mut chars = part.chars();
    chars
        .next()
        .is_some_and(|ch| ch == '_' || ch.is_alphabetic())
        && chars.all(|ch| ch == '_' || ch.is_alphanumeric())
}

fn python_inert_initializers(files: &BTreeMap<String, ParsedFile>) -> BTreeSet<String> {
    files
        .iter()
        .filter(|(path, parsed)| {
            (path.as_str() == "__init__.py" || path.ends_with("/__init__.py"))
                && parsed.python_inert_initializer()
        })
        .map(|(path, _)| path.clone())
        .collect()
}

fn python_namespace_submodule_path(
    binding: &ImportBinding,
    caller_file: &str,
    indexed_files: &BTreeSet<String>,
    inert_initializers: &BTreeSet<String>,
) -> Option<String> {
    if !matches!(binding.kind, ImportBindingKind::MemberImport) {
        return None;
    }
    let module_path = if binding.module_path.starts_with('.') {
        let suffix = binding.module_path.trim_start_matches('.');
        let dots = binding.module_path.len() - suffix.len();
        if !suffix.is_empty() && !suffix.split('.').all(python_identifier) {
            return None;
        }
        let (directory, _) = caller_file.rsplit_once('/')?;
        let mut parts: Vec<_> = directory.split('/').collect();
        if !parts.iter().all(|part| python_identifier(part)) {
            return None;
        }
        // Filename proximity alone cannot prove a relative import's package.
        for level in 0..dots {
            if level > 0 {
                parts.pop()?;
            }
            if parts.is_empty()
                || !inert_initializers.contains(&format!("{}/__init__.py", parts.join("/")))
            {
                return None;
            }
        }
        if !suffix.is_empty() {
            parts.extend(suffix.split('.'));
        }
        parts.join(".")
    } else {
        binding.module_path.clone()
    };
    if !module_path.split('.').all(python_identifier) {
        return None;
    }
    let member = binding
        .member
        .as_deref()
        .filter(|part| python_identifier(part))?;
    let mut parent = String::new();
    for part in module_path.split('.') {
        if !parent.is_empty() {
            parent.push('/');
        }
        parent.push_str(part);
        let initializer = format!("{parent}/__init__.py");
        if indexed_files.contains(&format!("{parent}.py"))
            || (indexed_files.contains(&initializer) && !inert_initializers.contains(&initializer))
        {
            return None;
        }
    }
    Some(format!("{module_path}.{member}"))
}

fn python_imported_receiver_types(
    import_bindings: &BTreeMap<String, Vec<ImportBinding>>,
    indexed_files: &BTreeSet<String>,
    clean_class_spans: &BTreeMap<(String, String), (usize, usize)>,
    inert_initializers: &BTreeSet<String>,
) -> BTreeMap<String, BTreeSet<String>> {
    let mut proven = BTreeMap::new();
    for (caller_file, receiver_type, _, _) in python_imported_class_proof_keys(
        import_bindings,
        indexed_files,
        clean_class_spans,
        inert_initializers,
    ) {
        proven
            .entry(caller_file)
            .or_insert_with(BTreeSet::new)
            .insert(receiver_type);
    }
    proven
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
    js_ts_relative_module_candidates(module_path, caller_file, indexed_files)?
        .into_iter()
        .next()
}

const JS_TS_MODULE_EXTENSIONS: [&str; 7] = [".js", ".jsx", ".mjs", ".cjs", ".ts", ".tsx", ""];
const JS_TS_INDEX_FILES: [&str; 6] = [
    "index.js",
    "index.jsx",
    "index.mjs",
    "index.cjs",
    "index.ts",
    "index.tsx",
];

/// Preserve the existing function-import precedence; class identity callers
/// additionally require candidate cardinality one before consulting a class.
///
/// Additive rule (I1): the literal candidate list is returned unchanged
/// whenever it is non-empty. Only a specifier with no literal candidate falls
/// through to the normalized form, which (a) resolves bare `.`/`..` and
/// redundant `//`, `/./`, `/../` segments within the repo root, and (b) maps a
/// `.js` spelling whose file is not indexed to the same-stem `.ts` then
/// `.tsx` (the TypeScript ESM convention). `.jsx`/`.mjs`/`.cjs` and `.d.ts`
/// are not substituted.
pub(crate) fn js_ts_relative_module_candidates(
    module_path: &str,
    caller_file: &str,
    indexed_files: &BTreeSet<String>,
) -> Option<Vec<String>> {
    let literal = js_ts_literal_module_candidates(module_path, caller_file, indexed_files);
    if literal.as_ref().is_some_and(|found| !found.is_empty()) {
        return literal;
    }
    match js_ts_normalized_module_candidates(module_path, caller_file, indexed_files) {
        Some(found) if !found.is_empty() => Some(found),
        _ => literal,
    }
}

/// True for the specifier spellings the relative candidate function accepts:
/// `./x`, `../x`, and the bare directory forms `.` and `..`.
pub(crate) fn is_js_ts_relative_specifier(module_path: &str) -> bool {
    let module_path = module_path.trim();
    module_path == "."
        || module_path == ".."
        || module_path.starts_with("./")
        || module_path.starts_with("../")
}

fn js_ts_normalized_module_candidates(
    module_path: &str,
    caller_file: &str,
    indexed_files: &BTreeSet<String>,
) -> Option<Vec<String>> {
    let module_path = module_path.trim();
    if !is_js_ts_relative_specifier(module_path) {
        return None;
    }
    let caller_dir = caller_file.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
    let mut parts: Vec<&str> = if caller_dir.is_empty() {
        Vec::new()
    } else {
        caller_dir.split('/').collect()
    };
    let segments: Vec<&str> = module_path.split('/').collect();
    // `.`, `..`, `./` and `./sub/` name a directory, never a file.
    let directory_only = matches!(segments.last(), Some(&("" | "." | "..")));
    for segment in segments {
        match segment {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            name => parts.push(name),
        }
    }
    let target = parts.join("/");
    let indexed = |candidate: &String| indexed_files.contains(candidate);
    let mut candidates: Vec<String> = Vec::new();
    if !directory_only && !target.is_empty() {
        let has_module_extension = [".js", ".jsx", ".mjs", ".cjs", ".ts", ".tsx"]
            .iter()
            .any(|ext| target.ends_with(ext));
        if has_module_extension {
            // TypeScript order for a spelled extension: the literal file, then
            // (`.js` only) the emitted-name substitution `.ts` before `.tsx`, never
            // through a `.d` stem, then the spelling with an extension appended.
            // The first non-empty tier wins; index files follow below.
            let literal = Some(target.clone()).filter(indexed);
            let substituted = target
                .strip_suffix(".js")
                .filter(|stem| !stem.is_empty() && !stem.ends_with('/') && !stem.ends_with(".d"))
                .map(|stem| {
                    [".ts", ".tsx"]
                        .iter()
                        .map(|ext| format!("{stem}{ext}"))
                        .filter(indexed)
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let appended = JS_TS_MODULE_EXTENSIONS
                .iter()
                .map(|ext| format!("{target}{ext}"))
                .filter(indexed)
                .collect::<Vec<_>>();
            candidates = literal
                .map(|file| vec![file])
                .into_iter()
                .chain([substituted, appended])
                .find(|tier| !tier.is_empty())
                .unwrap_or_default();
        } else {
            candidates = JS_TS_MODULE_EXTENSIONS
                .iter()
                .map(|ext| format!("{target}{ext}"))
                .filter(indexed)
                .collect();
        }
    }
    for index in JS_TS_INDEX_FILES {
        let candidate = if target.is_empty() {
            index.to_string()
        } else {
            format!("{target}/{index}")
        };
        if indexed_files.contains(&candidate) {
            candidates.push(candidate);
        }
    }
    Some(candidates)
}

pub(crate) fn js_ts_literal_module_candidates(
    module_path: &str,
    caller_file: &str,
    indexed_files: &BTreeSet<String>,
) -> Option<Vec<String>> {
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
    let mut candidates = Vec::new();

    for ext in &JS_TS_MODULE_EXTENSIONS {
        let candidate = if base.is_empty() {
            format!("{rel}{ext}")
        } else {
            format!("{base}/{rel}{ext}")
        };
        if indexed_files.contains(&candidate) {
            candidates.push(candidate);
        }
    }

    for index in &JS_TS_INDEX_FILES {
        let candidate = if base.is_empty() {
            format!("{rel}/{index}")
        } else {
            format!("{base}/{rel}/{index}")
        };
        if indexed_files.contains(&candidate) {
            candidates.push(candidate);
        }
    }

    Some(candidates)
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
mod tests;

// ---------------------------------------------------------------------------
// P5 S2: Go func-value registration index tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod go_registration_tests;

#[cfg(test)]
mod go_receiver_typing_tests;

#[cfg(test)]
mod python_property_access_tests;

// ---------------------------------------------------------------------------
// P9: framework-entry (Flask/FastAPI/Express route registration) tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod framework_entry_tests;

pub(crate) mod go;

pub mod js_ts;

pub mod rust;
