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
    pub cfg: RustCrateConfig,
    pub complete: bool,
}

impl ScopeGraphBuildInputs {
    pub fn from_files_convention(files: &BTreeMap<String, ParsedFile>) -> Self {
        ScopeGraphBuildInputs {
            repo_root: PathBuf::new(),
            all_file_paths: files.keys().cloned().collect(),
            manifest_hashes: BTreeMap::new(),
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
/// consult path at all, unlike `go_func_typed_fields`/`FuncValueField`).
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
    /// JS/TS R4c: file -> exported function declarations.
    #[serde(default)]
    pub js_ts_exported_functions: BTreeMap<String, BTreeSet<String>>,
    /// JS/TS R4c: caller function -> conservative parameter/local binding names.
    #[serde(default)]
    pub js_ts_function_locals: BTreeMap<FunctionId, BTreeSet<String>>,
    /// P5 S1 (Go func-value callbacks): directory basename -> set of
    /// directories containing Go files sharing that basename. Used to resolve
    /// a qualified `pkg.T` reference's import path to a package-scoped owner
    /// identity (`resolve_go_owner_identity`); ambiguous (>1) or absent (0) is
    /// unresolved, which the S1/S2/S3 pipeline treats as an unknown/ambiguous
    /// owner (never feeds S3). Whole-program derived like
    /// `interface_impls`/`promoted_aliases`; recomputed by
    /// `apply_go_func_value_fields`.
    #[serde(default)]
    pub go_package_basenames: BTreeMap<String, BTreeSet<String>>,
    /// P5 S1: every package-scoped struct identity the Go type provider
    /// extracted (regardless of func-typed fields). Whole-program derived;
    /// recomputed by `apply_go_func_value_fields`.
    #[serde(default)]
    pub go_known_struct_identities: BTreeSet<crate::resolution::GoOwnerIdentity>,
    /// P5 S1: `(owner_identity, field_name)` pairs whose declared type begins
    /// with `func(`. Whole-program derived; recomputed by
    /// `apply_go_func_value_fields`.
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
}

impl CallGraph {
    /// Create an empty call graph with no functions or edges.
    pub fn empty() -> Self {
        CallGraph {
            functions: BTreeMap::new(),
            calls: BTreeMap::new(),
            callers: BTreeMap::new(),
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
            js_ts_exported_functions: BTreeMap::new(),
            js_ts_function_locals: BTreeMap::new(),
            go_package_basenames: BTreeMap::new(),
            go_known_struct_identities: BTreeSet::new(),
            go_func_typed_fields: BTreeSet::new(),
            go_registrations: BTreeSet::new(),
            go_registration_shadowed_skips: 0,
            go_registration_ambiguous_owner_skips: 0,
            go_registration_unknown_owner_recorded: 0,
            property_getters: BTreeMap::new(),
            cached_property_getters: BTreeSet::new(),
            property_accesses: BTreeSet::new(),
            property_access_fanout_skips: 0,
            property_access_store_skips: 0,
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
                let call_sites = parsed.function_calls_with_spans_on_lines(&func_node, &all_lines);

                for (callee_name, line, start_byte, end_byte) in call_sites {
                    let site = CallSite {
                        caller: caller_id.clone(),
                        callee_name: callee_name.clone(),
                        line,
                        kind: Self::call_kind_at(parsed, start_byte, end_byte),
                        start_byte,
                        end_byte,
                        qualifier: Self::recover_self_receiver_qualifier(
                            parsed,
                            &callee_name,
                            line,
                            None,
                        ),
                        receiver_type: None,
                        receiver_recovery: None,
                        receiver_materialized: false,
                        arg_count: None,
                        arg_spread: false,
                        receiver_outcome: None,
                        origin: CallSiteOrigin::Source,
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
            js_ts_exported_functions: BTreeMap::new(),
            js_ts_function_locals: BTreeMap::new(),
            go_package_basenames: BTreeMap::new(),
            go_known_struct_identities: BTreeSet::new(),
            go_func_typed_fields: BTreeSet::new(),
            go_registrations: BTreeSet::new(),
            go_registration_shadowed_skips: 0,
            go_registration_ambiguous_owner_skips: 0,
            go_registration_unknown_owner_recorded: 0,
            property_getters: BTreeMap::new(),
            cached_property_getters: BTreeSet::new(),
            property_accesses: BTreeSet::new(),
            property_access_fanout_skips: 0,
            property_access_store_skips: 0,
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
            call_sites: Vec<(FunctionId, CallSite)>,
        }

        // Phase 2: Find all call sites within each function in parallel, then
        // flatten serially in file order to preserve insertion order.
        let per_file_calls: Vec<FileCallSites> = ordered_files
            .par_iter()
            .map(|entry| {
                let (file_path, parsed) = *entry;
                let mut file_call_sites = Vec::new();
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
                    let call_sites = parsed
                        .function_calls_with_qualifier_and_spans_on_lines(&func_node, &all_lines);
                    let recv_var = parsed
                        .language
                        .go_receiver_var(&func_node)
                        .map(|n| parsed.node_text(&n).to_string());

                    for (
                        callee_name,
                        line,
                        qualifier,
                        start_byte,
                        end_byte,
                        receiver_expr,
                        arg_count,
                        arg_spread,
                    ) in call_sites
                    {
                        let qualifier = Self::recover_self_receiver_qualifier(
                            parsed,
                            &callee_name,
                            line,
                            qualifier,
                        );
                        let classification = classifier.classify(crate::resolution::ReceiverCtx {
                            receiver_expr,
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
                            kind: Self::call_kind_at(parsed, start_byte, end_byte),
                            start_byte,
                            end_byte,
                            qualifier,
                            receiver_type: recovered.as_ref().map(|r| r.static_type.clone()),
                            receiver_recovery: recovered.as_ref().map(|r| r.recovery),
                            receiver_materialized: classification.materialized,
                            arg_count,
                            arg_spread,
                            receiver_outcome: None,
                            origin: CallSiteOrigin::Source,
                        };
                        file_call_sites.push((caller_id.clone(), site));
                    }
                }

                FileCallSites {
                    call_sites: file_call_sites,
                }
            })
            .collect();

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
        let (js_ts_exported_functions, js_ts_function_locals) =
            Self::extract_js_ts_resolution_facts(files);
        let indexed_files: BTreeSet<String> = files.keys().cloned().collect();

        let mut cg = CallGraph {
            functions,
            calls,
            callers,
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
            js_ts_exported_functions,
            js_ts_function_locals,
            go_package_basenames: BTreeMap::new(),
            go_known_struct_identities: BTreeSet::new(),
            go_func_typed_fields: BTreeSet::new(),
            go_registrations: BTreeSet::new(),
            go_registration_shadowed_skips: 0,
            go_registration_ambiguous_owner_skips: 0,
            go_registration_unknown_owner_recorded: 0,
            property_getters: BTreeMap::new(),
            cached_property_getters: BTreeSet::new(),
            property_accesses: BTreeSet::new(),
            property_access_fanout_skips: 0,
            property_access_store_skips: 0,
        };
        cg.recompute_indirect_calls(files);
        cg.refresh_rust_receiver_state(files);
        cg.apply_go_embedding_promotion(files);
        cg.apply_go_interface_dispatch(files);
        // P5: S1 func-typed-field index, then S2 registration scan (needs S1
        // already applied — registrations are keyed against it).
        cg.apply_go_func_value_fields(files);
        cg.apply_go_registrations(files);
        // P7: python property-access state — needs the complete method_owners
        // / method_class_span / class_bases indexes already populated above,
        // so it runs last, same rationale as the Go passes.
        cg.apply_python_property_accesses(files);
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
        BTreeMap<String, BTreeSet<String>>,
        BTreeMap<FunctionId, BTreeSet<String>>,
    ) {
        Self::extract_js_ts_resolution_facts_from_iter(files.iter())
    }

    fn extract_js_ts_resolution_facts_from_iter<'a, I>(
        files: I,
    ) -> (
        BTreeMap<String, BTreeSet<String>>,
        BTreeMap<FunctionId, BTreeSet<String>>,
    )
    where
        I: IntoIterator<Item = (&'a String, &'a ParsedFile)>,
    {
        let mut exported_functions: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();
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

            let exports = parsed.extract_js_ts_exported_functions();
            if !exports.is_empty() {
                exported_functions.insert(file_path.clone(), exports);
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

        (exported_functions, function_locals)
    }

    // -----------------------------------------------------------------------
    // Incremental cache support (Phase 2)
    // -----------------------------------------------------------------------

    /// Remove all entries originating from the given files.
    ///
    /// Used by incremental cache update: when a file changes, its call graph
    /// contributions are stripped out before fresh data is merged in.
    pub fn remove_files(&mut self, exclude: &BTreeSet<String>) {
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
        self.js_ts_exported_functions
            .retain(|f, _| !exclude.contains(f));
        self.js_ts_function_locals
            .retain(|fid, _| !exclude.contains(&fid.file));
        // indexed_files tracks the file set; removed files are no longer indexed.
        self.indexed_files.retain(|f| !exclude.contains(f));

        // P5: Go func-value state (S1 field-typing index + S2 registration
        // table) is whole-program derived, same rationale as the promoted
        // embedding aliases above — a registration's field-key validity can
        // depend on a struct declared in an UNCHANGED file, and a
        // registration's target FunctionId resolution can depend on the
        // complete function index. Drop it all; `apply_go_func_value_fields` /
        // `apply_go_registrations` repopulate from the merged graph.
        self.clear_go_func_value_fields();
        self.clear_go_registrations();

        // P7: Python property-access state is ALSO whole-program derived —
        // same rationale as the Go func-value state directly above: a
        // getter's owner class can live in an unchanged file, and unknown-
        // receiver fanout needs the complete S1 index across every file.
        // Drop it all; `apply_python_property_accesses` repopulates from the
        // merged graph.
        self.clear_python_property_accesses();
    }

    /// Merge another CallGraph into this one.
    ///
    /// Entries from `other` are added to the existing data. Typically called
    /// after `remove_files` to splice in freshly-built data for changed files.
    pub fn merge(&mut self, other: CallGraph) {
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
        self.js_ts_exported_functions
            .extend(other.js_ts_exported_functions);
        self.js_ts_function_locals
            .extend(other.js_ts_function_locals);

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
    }

    pub(crate) fn recompute_indirect_calls(&mut self, files: &BTreeMap<String, ParsedFile>) {
        self.clear_indirect_calls();
        let sites = self.compute_indirect_call_sites(files);
        self.apply_indirect_call_sites(sites);
    }

    fn clear_indirect_calls(&mut self) {
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
    ) -> Vec<(FunctionId, CallSite)> {
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
                    let func_source = Self::extract_func_source(parsed, caller_id);
                    if let Some(resolved) = crate::ast::resolve_fptr_assignment(
                        &func_source,
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

        // Level 3: parameter-passed function pointers (1-hop interprocedural).
        let mut level3_sites: Vec<(FunctionId, CallSite)> = Vec::new();
        for (caller_id, sites) in &self.calls {
            let parsed = match files.get(&caller_id.file) {
                Some(p) => p,
                None => continue,
            };

            let func_node = parsed.find_function_by_name(&caller_id.name);
            let param_names = match func_node {
                Some(ref f) => parsed.function_parameter_names(f),
                None => continue,
            };
            if param_names.is_empty() {
                continue;
            }

            for site in sites {
                if self.functions.contains_key(&site.callee_name) {
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

                let param_idx = match param_names.iter().position(|p| p == &site.callee_name) {
                    Some(idx) => idx,
                    None => continue,
                };

                if let Some(caller_sites) = self.callers.get(&caller_id.name) {
                    for caller_site in caller_sites {
                        let caller_parsed = match files.get(&caller_site.caller.file) {
                            Some(p) => p,
                            None => continue,
                        };

                        if let Some(arg_text) = caller_parsed.call_argument_text_at(
                            caller_site.line,
                            &caller_id.name,
                            param_idx,
                        ) {
                            if known_fn_names.contains(&arg_text) {
                                level3_sites.push((
                                    caller_id.clone(),
                                    Self::indirect_call_site(caller_id, arg_text, site),
                                ));
                            } else {
                                let caller_func_source =
                                    Self::extract_func_source(caller_parsed, &caller_site.caller);
                                if let Some(resolved) = crate::ast::resolve_fptr_assignment(
                                    &caller_func_source,
                                    &arg_text,
                                    &known_fn_names,
                                ) {
                                    level3_sites.push((
                                        caller_id.clone(),
                                        Self::indirect_call_site(caller_id, resolved, site),
                                    ));
                                }
                            }
                        }
                    }
                }
            }
        }

        extra_sites.extend(level3_sites);
        extra_sites
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
            receiver_recovery: None,
            receiver_materialized: false,
            arg_count: None,
            arg_spread: false,
            receiver_outcome: None,
            origin: CallSiteOrigin::IndirectResolution,
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
        let ordered: Vec<(&FunctionId, &BTreeSet<CallSite>)> = self.calls.iter().collect();
        ordered
            .par_iter()
            .copied() // (&FunctionId, &BTreeSet) is Copy -> avoid &&-destructuring
            .map(|(caller, sites)| Self::receiver_updates_for_caller(caller, sites, &typer, files))
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
        let ast_calls =
            parsed.function_calls_with_qualifier_and_spans_on_lines(&fn_node, &all_lines);
        for site in sites {
            let Some((_, _, qualifier, start_byte, _, receiver_expr, _, _)) = ast_calls
                .iter()
                .find(|(callee_name, _, _, start_byte, end_byte, _, _, _)| {
                    callee_name == &site.callee_name
                        && *start_byte == site.start_byte
                        && *end_byte == site.end_byte
                })
            else {
                continue;
            };
            if receiver_expr.is_none() && qualifier.is_none() {
                continue;
            }
            let outcome = typer.type_of_receiver(crate::resolution_receiver::ReceiverTypeCtx {
                parsed,
                caller,
                fn_node,
                receiver_expr: *receiver_expr,
                qualifier: qualifier.as_deref(),
                call_start_byte: *start_byte,
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
            let ast_calls =
                parsed.function_calls_with_qualifier_and_spans_on_lines(&fn_node, &all_lines);
            for site in sites {
                let Some((_, _, qualifier, start_byte, _, receiver_expr, _, _)) = ast_calls
                    .iter()
                    .find(|(callee_name, _, _, start_byte, end_byte, _, _, _)| {
                        callee_name == &site.callee_name
                            && *start_byte == site.start_byte
                            && *end_byte == site.end_byte
                    })
                else {
                    continue;
                };
                if receiver_expr.is_none() && qualifier.is_none() {
                    continue;
                }
                let outcome = typer.type_of_receiver(crate::resolution_receiver::ReceiverTypeCtx {
                    parsed,
                    caller,
                    fn_node,
                    receiver_expr: *receiver_expr,
                    qualifier: qualifier.as_deref(),
                    call_start_byte: *start_byte,
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

    pub fn apply_go_interface_dispatch(&mut self, files: &BTreeMap<String, ParsedFile>) {
        self.clear_interface_dispatch();
        // The dispatch pass ran (even if there are no Go files → empty result); a raw
        // build_direct_subset graph leaves this false (review MINOR 6 signal).
        self.interface_dispatch_computed = true;
        if !files
            .values()
            .any(|p| p.language == crate::languages::Language::Go)
        {
            return;
        }
        let live = crate::live_types::go_admission_live_set(files);
        let provider = crate::type_providers::go::GoTypeProvider::from_parsed_files(files);
        let table = provider.compute_interface_dispatch(&live);
        self.interface_impls = table.impls;
        // Capture the interface-method-name set for the PR-2 manifest denominator
        // (§8a) while the provider is live (it is dropped after this fn).
        self.interface_method_names = provider.interface_method_names();
        // Capture per-method arity for later arity-filtered dispatch (Task 2).
        self.method_arity = provider.method_arities();
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

    fn clear_go_func_value_fields(&mut self) {
        self.go_package_basenames.clear();
        self.go_known_struct_identities.clear();
        self.go_func_typed_fields.clear();
    }

    /// P5 S1: recompute the Go func-typed-field index (package-scoped owner
    /// identity -> which struct fields are func-typed) over `files`.
    /// Whole-program derived, same shape as `apply_go_embedding_promotion` /
    /// `apply_go_interface_dispatch`: clears first (idempotent), then
    /// recomputes from scratch — never incrementally patched.
    pub fn apply_go_func_value_fields(&mut self, files: &BTreeMap<String, ParsedFile>) {
        self.clear_go_func_value_fields();
        if !files
            .values()
            .any(|p| p.language == crate::languages::Language::Go)
        {
            return;
        }
        // Directory-basename index for qualified `pkg.T` owner resolution
        // (`resolve_go_owner_identity`) — one basename can map to multiple
        // directories, which is deliberately treated as ambiguous downstream.
        for (path, parsed) in files {
            if parsed.language != crate::languages::Language::Go {
                continue;
            }
            let dir = crate::resolution::dir_of(path).to_string();
            let basename = dir.rsplit('/').next().unwrap_or(&dir).to_string();
            self.go_package_basenames
                .entry(basename)
                .or_default()
                .insert(dir);
        }
        let provider = crate::type_providers::go::GoTypeProvider::from_parsed_files(files);
        self.go_known_struct_identities = provider.go_known_struct_identities();
        self.go_func_typed_fields = provider.go_func_typed_fields();
    }

    fn clear_go_registrations(&mut self) {
        self.go_registrations.clear();
        self.go_registration_shadowed_skips = 0;
        self.go_registration_ambiguous_owner_skips = 0;
        self.go_registration_unknown_owner_recorded = 0;
    }

    /// P5 S2: scan `files` for recognized Go function-value registrations
    /// (composite-literal keyed field, field assignment, bare call argument)
    /// and record them in `go_registrations`. Whole-program derived (needs
    /// `go_func_typed_fields`/`go_known_struct_identities` already applied by
    /// `apply_go_func_value_fields`, and target resolution needs the complete
    /// `functions`/`method_owners` index) — clears and recomputes from
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
        use crate::resolution::{resolve_go_bare_value_ref, resolve_go_owner_identity};

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
            } => {
                match resolve_go_owner_identity(
                    struct_type_text,
                    &caller_id.file,
                    &self.imports,
                    &self.go_package_basenames,
                ) {
                    Some(owner) if self.go_known_struct_identities.contains(&owner) => {
                        if self
                            .go_func_typed_fields
                            .contains(&(owner.clone(), field_name.clone()))
                        {
                            Some((owner, field_name.clone()))
                        } else {
                            // Known struct, field not func-typed: not a
                            // registration candidate at all — silently skip
                            // (not a "fallback", not a "skip" to count).
                            return;
                        }
                    }
                    _ => {
                        // Unknown/ambiguous struct: fall back to recording
                        // WITHOUT a field key (nav-only, never feeds S3),
                        // since the shared gate above already established the
                        // value resolves unambiguously and isn't shadowed.
                        self.go_registration_unknown_owner_recorded += 1;
                        None
                    }
                }
            }
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
                match resolve_go_owner_identity(
                    &operand_type,
                    &caller_id.file,
                    &self.imports,
                    &self.go_package_basenames,
                ) {
                    Some(owner)
                        if self
                            .go_func_typed_fields
                            .contains(&(owner.clone(), field_name.clone())) =>
                    {
                        Some((owner, field_name.clone()))
                    }
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
        for (file_path, parsed) in files {
            if !only_files.contains(file_path) {
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
                let call_sites =
                    parsed.function_calls_with_qualifier_and_spans_on_lines(&func_node, &all_lines);
                let recv_var = parsed
                    .language
                    .go_receiver_var(&func_node)
                    .map(|n| parsed.node_text(&n).to_string());
                let file_imports_ref = imports.get(file_path);

                for (
                    callee_name,
                    line,
                    qualifier,
                    start_byte,
                    end_byte,
                    receiver_expr,
                    arg_count,
                    arg_spread,
                ) in call_sites
                {
                    let qualifier = Self::recover_self_receiver_qualifier(
                        parsed,
                        &callee_name,
                        line,
                        qualifier,
                    );
                    let classification = classifier.classify(crate::resolution::ReceiverCtx {
                        receiver_expr,
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
                        kind: Self::call_kind_at(parsed, start_byte, end_byte),
                        start_byte,
                        end_byte,
                        qualifier,
                        receiver_type: recovered.as_ref().map(|r| r.static_type.clone()),
                        receiver_recovery: recovered.as_ref().map(|r| r.recovery),
                        receiver_materialized: classification.materialized,
                        arg_count,
                        arg_spread,
                        receiver_outcome: None,
                        origin: CallSiteOrigin::Source,
                    };
                    calls
                        .entry(caller_id.clone())
                        .or_default()
                        .insert(site.clone());
                    callers.entry(callee_name).or_default().push(site);
                }
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
        let (js_ts_exported_functions, js_ts_function_locals) =
            Self::extract_js_ts_resolution_facts_from_iter(
                subset_files.iter().map(|(fp, parsed)| (fp, *parsed)),
            );
        let indexed_files: BTreeSet<String> = files.keys().cloned().collect();

        CallGraph {
            functions,
            calls,
            callers,
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
            js_ts_exported_functions,
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
            // P7: whole-program Python property-access state — left empty
            // here for the same reason as the Go func-value state above;
            // the caller re-applies `apply_python_property_accesses` on the
            // merged graph.
            property_getters: BTreeMap::new(),
            cached_property_getters: BTreeSet::new(),
            property_accesses: BTreeSet::new(),
            property_access_fanout_skips: 0,
            property_access_store_skips: 0,
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
    let module_path = module_path.trim();
    if !(module_path.starts_with("./") || module_path.starts_with("../")) {
        return false;
    }

    let caller_dir = caller_file.rsplit_once('/').map(|(d, _)| d).unwrap_or("");
    let mut rel = module_path;
    let mut base_parts: Vec<&str> = if caller_dir.is_empty() {
        Vec::new()
    } else {
        caller_dir.split('/').collect()
    };
    while let Some(rest) = rel.strip_prefix("../") {
        if base_parts.pop().is_none() {
            return false;
        }
        rel = rest;
    }
    rel = rel.strip_prefix("./").unwrap_or(rel);
    if rel.is_empty() {
        return false;
    }
    let base = base_parts.join("/");

    for ext in &[".js", ".jsx", ".mjs", ".cjs", ".ts", ".tsx", ""] {
        let candidate = if base.is_empty() {
            format!("{rel}{ext}")
        } else {
            format!("{base}/{rel}{ext}")
        };
        if indexed_files.contains(&candidate) && candidate == file {
            return true;
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
        if indexed_files.contains(&candidate) && candidate == file {
            return true;
        }
    }

    false
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
    fn recompute_indirect_calls_is_idempotent_and_post_merge_matches_full_build() {
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
        assert!(once.iter().any(|entry| entry.contains(":new_handler:")));

        merged.recompute_indirect_calls(&files_v2);
        assert_eq!(once, indirect_call_dump(&merged));
        assert_eq!(callers_once, indirect_caller_dump(&merged));

        let full = CallGraph::build(&files_v2);
        assert_eq!(once, indirect_call_dump(&full));
        assert_eq!(callers_once, indirect_caller_dump(&full));
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
        assert!(cg.go_known_struct_identities.is_empty());
        assert!(cg.go_func_typed_fields.is_empty());
        assert_eq!(cg.go_registration_shadowed_skips, 0);
        assert_eq!(cg.go_registration_ambiguous_owner_skips, 0);
        assert_eq!(cg.go_registration_unknown_owner_recorded, 0);
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
}
