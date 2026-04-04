# Spec: E12 — Multi-Language Type System with Trait-Based Providers

**Date:** April 4, 2026 (revised)
**Status:** Design — decisions confirmed
**Depends on:** E7 ✅ (CPG DFG queries), E6 ✅ (import resolution)

-----

## 1. Problem

`TypeDatabase` in `src/type_db.rs` (1,479 lines) is hardwired to C/C++. No
other language can provide type information to the CPG. This means:

- **TypeScript:** `utils.process()` where `utils: UtilService` can’t resolve
  through the interface to the concrete implementation.
- **Go:** `r.Read(buf)` where `r: io.Reader` can’t resolve to `os.File.Read`.
- **Java:** `service.find()` where `service: Repository` can’t resolve to
  `UserRepository.find()`.
- **All languages:** field layouts are only known for C/C++ structs.

-----

## 2. Confirmed Decisions

|# |Decision                |Choice                               |Rationale                                                             |
|--|------------------------|-------------------------------------|----------------------------------------------------------------------|
|Q1|TypeRegistry location   |`CpgContext`                         |Avoids lifetime entanglement with parsed files                        |
|Q2|CppTypeProvider approach|Wrapper first, refactor later        |Zero risk to existing tests                                           |
|Q3|`.d.ts` parsing mode    |Eager + Scoped (no lazy)             |Eager for reviews (~2s), scoped for incremental. Lazy has no use case.|
|Q4|Structural typing       |Interface supports full, start simple|`is_assignable_to` + `resolve_generic` API, simple property-name impl |
|Q5|RTA for dispatch        |Yes, all languages                   |Prunes dispatch targets to instantiated types                         |
|Q6|Types in scoped CPG     |All files (not scoped)               |Type defs are cheap; needed for correctness                           |

-----

## 3. Design: Trait-Based Providers with Capability Traits

### 3.1 Core Trait

```rust
/// Core type information provider. Implemented per-language.
pub trait TypeProvider: Send + Sync {
    /// Resolve the type of a variable/expression at a source location.
    fn resolve_type(&self, file: &str, expr: &str, line: usize) -> Option<ResolvedType>;

    /// Get the field layout of a named type.
    fn field_layout(&self, type_name: &str) -> Option<Vec<FieldInfo>>;

    /// Find all concrete types that satisfy a given type constraint.
    fn subtypes_of(&self, type_name: &str) -> Vec<String>;

    /// Resolve a type alias to its canonical name.
    fn resolve_alias(&self, type_name: &str) -> String;

    /// Which language(s) this provider covers.
    fn languages(&self) -> Vec<Language>;
}
```

### 3.2 Dispatch Capability

```rust
/// Languages with method dispatch resolution (Go, Java, C++, TS, Rust).
pub trait DispatchProvider: TypeProvider {
    fn resolve_dispatch(
        &self,
        receiver_type: &str,
        method: &str,
        live_types: &BTreeSet<String>,
    ) -> Vec<FunctionId>;
}
```

### 3.3 Structural Typing Capability (Full-Ready Interface)

The interface is designed to support full TypeScript structural checking
without refactoring. Simple implementation starts with property-name
comparison; medium adds generics; full adds conditional/mapped types. All
levels implement the same trait.

```rust
/// Languages with structural typing (TypeScript).
pub trait StructuralTypingProvider: TypeProvider {
    /// Check if value_type is assignable to target_type.
    ///
    /// Implementations can range from simple (property name comparison) to
    /// full (conditional types, mapped types, template literals). The
    /// Compatibility return carries a reason for incompatibility, enabling
    /// informative findings.
    fn is_assignable_to(
        &self,
        value_type: &ResolvedType,
        target_type: &ResolvedType,
    ) -> Compatibility;

    /// Resolve a generic type with concrete type arguments.
    ///
    /// `resolve_generic("Array", ["string"])` → ResolvedType with string
    /// element fields. Returns None if the base type is unknown or the
    /// implementation doesn't support generic resolution.
    ///
    /// Simple: returns None (no generic support).
    /// Medium: resolves built-in generics (Array, Map, Set, Promise).
    /// Full: resolves arbitrary generic instantiations including
    ///       conditional and mapped types.
    fn resolve_generic(
        &self,
        base_type: &str,
        type_args: &[String],
    ) -> Option<ResolvedType>;
}

/// Result of a structural compatibility check.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Compatibility {
    /// Types are structurally compatible.
    Compatible,
    /// Types are not compatible, with explanation.
    Incompatible { reason: String },
    /// Can't determine (missing type info, complex expression).
    Unknown,
}
```

### 3.4 Common Types

```rust
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResolvedType {
    pub name: String,
    pub kind: ResolvedTypeKind,
    /// Generic type parameters (e.g., ["string", "number"] for Map<string, number>).
    pub type_params: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResolvedTypeKind {
    Concrete,    // class, struct — can be instantiated
    Interface,   // interface, protocol, trait — can't be instantiated
    Alias,       // type alias, typedef
    Enum,        // Go iota, Java enum, Rust enum, TS union
    Primitive,   // int, string, bool
    Unknown,     // couldn't determine
}
```

### 3.5 Provider Registry

```rust
pub struct TypeRegistry {
    providers: Vec<Box<dyn TypeProvider>>,
    dispatch_providers: Vec<Box<dyn DispatchProvider>>,
    structural_providers: Vec<Box<dyn StructuralTypingProvider>>,
    language_map: BTreeMap<Language, usize>,
    /// Target language versions for version-aware analysis.
    target_versions: BTreeMap<Language, LanguageVersion>,
    /// Type resolution mode.
    mode: TypeResolutionMode,
}

#[derive(Debug, Clone)]
pub enum TypeResolutionMode {
    /// Parse all type sources upfront (default for reviews).
    Eager,
    /// Parse only types in files relevant to the diff scope.
    Scoped,
}

impl TypeRegistry {
    pub fn build(files: &BTreeMap<String, ParsedFile>, mode: TypeResolutionMode) -> Self { ... }
    pub fn provider_for(&self, lang: Language) -> Option<&dyn TypeProvider> { ... }
    pub fn dispatch_for(&self, lang: Language) -> Option<&dyn DispatchProvider> { ... }
    pub fn structural_for(&self, lang: Language) -> Option<&dyn StructuralTypingProvider> { ... }
    pub fn target_version(&self, lang: Language) -> Option<&LanguageVersion> { ... }
}
```

### 3.6 CPG Integration

```rust
pub struct CpgContext<'a> {
    pub cpg: CodePropertyGraph,
    pub files: &'a BTreeMap<String, ParsedFile>,
    pub types: TypeRegistry,       // replaces type_db: Option<TypeDatabase>
    pub scope: Option<CpgScope>,
}
```

### 3.7 Backward Compatibility

The existing `TypeDatabase` struct is preserved internally as the backing
store for `CppTypeProvider`. The `--compile-commands` CLI flag continues to
work. Algorithms that call `ctx.cpg.all_fields_of()`, `ctx.cpg.resolve_type()`
etc. are updated to delegate through the registry.

-----

## 4. Target Language Version Support

### 4.1 Configuration

```rust
#[derive(Debug, Clone)]
pub struct LanguageVersion {
    /// Language version string (e.g., "3.8", "1.21", "18", "ES2022").
    pub version: String,
    /// Parsed major.minor for comparison.
    pub major: u32,
    pub minor: u32,
}
```

Set via CLI flags:

```
slicing --repo . --diff changes.patch \
    --python-version 3.8 \
    --node-version 18 \
    --go-version 1.21 \
    --typescript-version 5.0
```

Or via a config file (`.prism.toml` or `prism.json`) in the repo root:

```toml
[versions]
python = "3.8"
node = "18"
go = "1.21"
typescript = "5.0"
```

### 4.2 How Version Info is Used (Phase 1: Informational Only)

In the initial implementation, `target_version` is stored in the TypeRegistry
but not actively checked. Providers build type information from whatever is in
the source and `.d.ts` files. The version field is available for future use.

### 4.3 How Version Info Will Be Used (Future: CompatibilitySlice)

See §8 for the CompatibilitySlice stub. When implemented, the version config
determines which features and stdlib types are available:

- Python 3.8 → no `match` statement, no `TypeGuard`, no `ParamSpec`
- Go 1.18 → generics available; Go 1.17 → no generics
- Node 18 → `fetch()` available globally; Node 16 → requires import
- TypeScript 4.1 → template literal types; TypeScript 4.0 → not available

### 4.4 TypeScript Version and `.d.ts` Files

For TypeScript, the `@types/` packages in `node_modules` already reflect the
version the project depends on. If `package.json` pins `@types/react@18`,
the `.d.ts` files reflect React 18 types (where `FC<Props>` no longer includes
`children` by default). This makes TypeScript version-aware “by accident” —
the npm dependency graph encodes version choices. No special handling needed
beyond parsing the `.d.ts` files.

-----

## 5. Eager + Scoped Type Resolution

### 5.1 Two Modes, No Lazy

|Mode      |When                       |What It Does                                                                             |Cost                                 |
|----------|---------------------------|-----------------------------------------------------------------------------------------|-------------------------------------|
|**Eager** |Reviews, initial dev builds|Parse all source types + all `@types/` `.d.ts` files                                     |~2–3 seconds for typical node_modules|
|**Scoped**|Incremental dev changes    |Parse source types for all files, `.d.ts` only for packages imported by diff-scoped files|~0.5–1 second                        |

Lazy (resolve on first access) is eliminated — it provides inconsistent
latency and no meaningful cost advantage over scoped.

### 5.2 Import Map Gaps

The E6 import map does NOT handle:

- Dynamic imports (`import('./mod')`)
- Re-exports (`export { X } from './other'`)
- Barrel files (`index.ts` re-exporting from subdirectories)
- TypeScript path aliases (`@/utils` via `tsconfig.json` `paths`)

For **eager mode**, these gaps don’t matter — all types are parsed regardless
of import structure. For **scoped mode**, these gaps mean some types from
re-exported packages may not be resolved.

**Mitigation for scoped mode:** If a type can’t be resolved from scoped
sources, fall back to eager resolution for that specific type. This is a
per-type fallback, not a mode switch — the provider tracks which types were
resolved from scoped sources and which required fallback.

### 5.3 Future: Import Map Improvements

Addressing the import map gaps (re-exports, barrel files, path aliases) is a
separate E6 follow-on, not part of E12. These improvements benefit both type
resolution and call graph accuracy. When they land, scoped mode becomes more
precise automatically.

### 5.4 Serialized Type Cache (Extension to E13)

The TypeRegistry data (struct definitions, interface hierarchies, field
layouts, satisfaction mappings) serializes trivially — it’s all `String`s,
`Vec`s, and `BTreeMap`s. E13 (Serialized CPG for Caching) should be extended
to serialize the type cache alongside the CPG graph.

**Cache invalidation:** per-file content hash, same as the CPG. If a type
definition file changes, its types are re-extracted. If `node_modules/`
changes (different `@types/` versions), the `.d.ts` types are re-parsed.

**CLI:**

```bash
# First review: builds CPG + types, caches both
slicing --repo . --diff changes.patch --cache-dir .prism-cache

# Second review: loads cached CPG + types, rebuilds only changed files
slicing --repo . --diff changes2.patch --cache-dir .prism-cache
```

This is not part of E12 — it’s an extension to E13’s scope. Documented here
for architectural alignment.

-----

## 6. Per-Language Providers

### 6.1 C/C++ — `CppTypeProvider` (refactor of existing)

**Source:** `compile_commands.json` + clang AST dump, tree-sitter fallback.
**Implements:** `TypeProvider` + `DispatchProvider`
**Work:** Wrap existing `TypeDatabase` in trait-implementing struct.
**Effort:** 1–2 days

### 6.2 Go — `GoTypeProvider`

**Source:** tree-sitter AST (all types explicit in source).
**Key analysis:** Interface satisfaction — struct method set ⊇ interface method set.
**Implements:** `TypeProvider` + `DispatchProvider`
**Effort:** 1 week

### 6.3 TypeScript — `TypeScriptTypeProvider`

**Source:** tree-sitter AST + `.d.ts` files (eager or scoped).
**Key analysis:** Structural typing via `is_assignable_to()` (simple initially).
**Implements:** `TypeProvider` + `DispatchProvider` + `StructuralTypingProvider`
**Key patterns:** React component props, Express/Hapi handler types, interface
implementations.
**Effort:** 2–3 weeks

### 6.4 Java — `JavaTypeProvider`

**Source:** tree-sitter AST (all types explicit).
**Key analysis:** Class hierarchy from `extends`/`implements`.
**Implements:** `TypeProvider` + `DispatchProvider`
**Effort:** 1 week

### 6.5 Rust — `RustTypeProvider`

**Source:** tree-sitter AST (annotated types; inferred types not available).
**Key analysis:** `impl Trait for Type` mapping.
**Implements:** `TypeProvider` + `DispatchProvider`
**Limitation:** Local variable types often inferred — returns `Unknown`.
**Effort:** 1 week

### 6.6 Python — `PythonTypeProvider`

**Source:** tree-sitter AST (PEP 484 annotations), optional `.pyi` stubs.
**Implements:** `TypeProvider` only (no dispatch — duck typing).
**Limitation:** Coverage depends on annotation density in repo.
**Effort:** 3–5 days

### 6.7 Languages Without Providers

**Lua, Terraform, Bash:** No type system. No provider registered.
`TypeRegistry` returns `None`. All analysis falls back to existing name-based
and AccessPath-based approaches. No degradation from current behavior.

-----

## 7. Implementation Phases

### Phase 1: Trait Definitions + CppTypeProvider Wrapper (1–2 days)

**Files:** `src/type_provider.rs` (new), `src/type_providers/cpp.rs` (new),
`src/cpg.rs`, `src/lib.rs`

1. Define traits, `ResolvedType`, `Compatibility`, `LanguageVersion` in
   `src/type_provider.rs`.
1. Create `CppTypeProvider` wrapping `TypeDatabase`.
1. Create `TypeRegistry` with provider registration and language routing.
1. Update `CpgContext` to hold `TypeRegistry`.
1. Update CPG type-enriched queries to delegate through registry.
1. Add `--python-version`, `--go-version`, etc. CLI flags (stored, not used).
1. All existing tests pass — no behavioral change.

### Phase 2: Go Type Provider (1 week)

**Files:** `src/type_providers/go.rs` (new)

1. Extract struct/interface definitions from tree-sitter.
1. Build method-receiver mapping.
1. Compute interface satisfaction.
1. Implement `TypeProvider` + `DispatchProvider`.
1. Tests: interface resolution, embedded struct fields, dispatch.

### Phase 3: TypeScript Type Provider (2–3 weeks)

**Files:** `src/type_providers/typescript.rs` (new)

1. Extract interface/class/type-alias declarations.
1. Parse function parameter/return type annotations.
1. Implement `.d.ts` resolution (eager + scoped modes).
1. Implement simple structural typing (`is_assignable_to` with property-name
   comparison, `resolve_generic` returns None).
1. Implement `TypeProvider` + `DispatchProvider` + `StructuralTypingProvider`.
1. Tests: React props, Express handlers, interface dispatch, structural compat.

### Phase 4: Java Type Provider (1 week)

**Files:** `src/type_providers/java.rs` (new)

### Phase 5: Rust Type Provider (1 week)

**Files:** `src/type_providers/rust.rs` (new)

### Phase 6: Python Type Provider (3–5 days)

**Files:** `src/type_providers/python.rs` (new)

### Phase 7: RTA Across All Dispatch Languages (2–3 days)

Generalize `collect_live_classes` across languages. Each `DispatchProvider`
receives the live-type set and filters dispatch targets.

-----

## 8. Future: CompatibilitySlice Algorithm (Stub)

### 8.1 Concept

A new slicing algorithm that cross-references code patterns against a
per-language version feature matrix. Flags code using features or stdlib
APIs newer than the project’s target runtime.

### 8.2 What It Catches

|Category                   |Example                                                                                                                             |
|---------------------------|------------------------------------------------------------------------------------------------------------------------------------|
|**Syntax features**        |Python `match` statement used with `python_version = "3.8"` (requires 3.10)                                                         |
|**Stdlib additions**       |`asyncio.TaskGroup` used with `python_version = "3.10"` (requires 3.11)                                                             |
|**Type system features**   |TypeScript template literal types used with `typescript_version = "4.0"` (requires 4.1)                                             |
|**Runtime globals**        |`fetch()` used with `node_version = "16"` (global in 18+)                                                                           |
|**Deprecated APIs**        |`os.popen()` in Python, `substr()` in JS                                                                                            |
|**Type definition changes**|React 18 `FC<Props>` no longer includes `children` — component that accesses `props.children` without declaring it breaks on upgrade|

### 8.3 Architecture

```rust
pub struct CompatibilitySlice;

impl CompatibilitySlice {
    pub fn slice(
        ctx: &CpgContext,
        diff: &DiffInput,
    ) -> Result<SliceResult> {
        // For each file in the diff:
        // 1. Get the file's language and target version from TypeRegistry
        // 2. Load the feature matrix for that language version
        // 3. Walk the AST for patterns newer than the target version
        // 4. Emit findings for version-incompatible patterns
        todo!()
    }
}
```

### 8.4 Feature Matrix Data Source

The version feature matrix is a data file (JSON or TOML) per language:

```toml
# python_features.toml
[features]
match_statement = { since = "3.10", node_type = "match_statement" }
walrus_operator = { since = "3.8", node_type = "named_expression" }
exception_groups = { since = "3.11", node_type = "try_star" }

[stdlib]
"asyncio.TaskGroup" = { since = "3.11" }
"tomllib" = { since = "3.11" }
"typing.TypeGuard" = { since = "3.10" }
```

Tree-sitter node types are used for syntax feature detection (fast, no false
positives). Stdlib detection uses the taint/call infrastructure to identify
calls to version-specific functions.

### 8.5 Scope

**Not part of E12.** CompatibilitySlice is a follow-on that depends on:

- E12 Phase 1 (TypeRegistry with `target_version` config) ✅ designed
- Feature matrix data files (new data, not code)
- Algorithm implementation (~1 week)

### 8.6 Connection to Type Version Changes

The most subtle bugs come from type definition changes across library
versions. React 18’s removal of `children` from `FC<Props>` is the canonical
example. Detecting this requires:

1. **Two versions of the type definition** — the old `.d.ts` and the new one
1. **Delta comparison** — which fields/methods changed between versions
1. **Usage analysis** — does the code use any changed field

This is essentially DeltaSlice applied to type definitions rather than source
code. The CompatibilitySlice could compose with DeltaSlice:

```
CompatibilitySlice = feature_matrix_check + type_delta_check
```

Where `type_delta_check` compares the type definitions from two versions of a
dependency and flags code that uses changed interfaces. This is architecturally
complex (needs old+new `node_modules/@types/`) and is a Phase 2 for
CompatibilitySlice.

-----

## 9. Future: Type-Enriched Finding Descriptions

### 9.1 Concept

Currently, findings describe code patterns:

```
"taint flows from user_input through db.query() to execute()"
```

With type info available, findings can include type context:

```
"taint flows from user_input (string) through db.query(sql: string) to
execute(query: string) — all parameters accept raw strings with no
parameterized query enforcement"
```

### 9.2 Value

Type-enriched descriptions help the LLM reviewer understand WHY a finding
matters without re-reading the code. The type annotation makes the severity
assessment more precise — “raw string flows to SQL” is a clear injection
vector, while “SafeQuery flows to SQL” is a false positive.

### 9.3 Implementation

Not a type system change — it’s an enhancement to finding formatters in
each algorithm. Each algorithm’s `SliceFinding` construction would call
`ctx.types.resolve_type()` for relevant variables and append type info to
the description string.

**Scope:** Separate follow-on, not part of E12. Depends on E12 being
available to provide type info.

### 9.4 Candidate Algorithms for Type-Enriched Descriptions

|Algorithm|Current Description           |With Type Info                                                                      |
|---------|------------------------------|------------------------------------------------------------------------------------|
|Taint    |“value reaches sink X”        |“value (type: string) reaches sink X (param: string) — no sanitization type barrier”|
|Contract |“non-null constraint on ‘x’”  |“non-null constraint on ‘x’ (type: User | null)”                                    |
|Echo     |“caller doesn’t handle return”|“caller doesn’t handle return (type: Result<T, Error>) — error case unhandled”      |
|Absence  |“open without close”          |“File::open() returns File (implements Drop — auto-closes)” → suppress finding      |
|Membrane |“API boundary crossed”        |“API boundary: param type changed from string to number — 3 callers pass string”    |

-----

## 10. Directory Structure

```
src/
  type_provider.rs          # Trait definitions, TypeRegistry, ResolvedType,
                            # Compatibility, LanguageVersion
  type_db.rs                # Existing C/C++ TypeDatabase (unchanged, internal)
  type_providers/
    mod.rs                  # pub mod cpp; pub mod go; etc.
    cpp.rs                  # CppTypeProvider wrapping TypeDatabase
    go.rs                   # GoTypeProvider
    typescript.rs           # TypeScriptTypeProvider
    java.rs                 # JavaTypeProvider
    rust_provider.rs        # RustTypeProvider (rust.rs is a keyword conflict)
    python.rs               # PythonTypeProvider
```

-----

## 11. Estimated Impact on Slice Precision

|Language   |Before        |After                              |Key Improvement             |
|-----------|--------------|-----------------------------------|----------------------------|
|C/C++      |Full type info|Same (refactored)                  |—                           |
|TypeScript |Name-only     |Interface dispatch, prop validation|React/Hapi analysis         |
|Go         |Name-only     |Interface satisfaction dispatch    |`io.Reader` resolution      |
|Java       |Name-only     |Class hierarchy dispatch           |`Repository` resolution     |
|Rust       |Name-only     |Trait dispatch                     |`dyn Read` resolution       |
|Python     |Name-only     |Annotated parameter types          |Partial — coverage dependent|
|Lua/TF/Bash|Name-only     |Same                               |No type system              |

-----

## 12. Risk Assessment

|Risk                                       |Severity|Mitigation                                             |
|-------------------------------------------|--------|-------------------------------------------------------|
|Trait API doesn’t cover a future language  |Low     |Capability traits are additive                         |
|TS structural typing produces false matches|Medium  |Start simple, deepen only on measured FP               |
|`.d.ts` parsing slow for large node_modules|Low     |Measured at ~2s; acceptable for reviews                |
|Refactoring CpgContext breaks tests        |Low     |Phase 1 is pure wrapper delegation                     |
|Go interface satisfaction expensive        |Low     |Precompute once, cache in provider                     |
|Python hints too sparse                    |Low     |Returns Unknown — no false positives                   |
|Import map gaps affect scoped mode         |Medium  |Per-type fallback to eager; E6 follow-on for re-exports|

-----

## 13. Roadmap Summary

```
E12 Phase 1: Trait plumbing + CppTypeProvider         (1-2 days)
E12 Phase 2: GoTypeProvider                           (1 week)
E12 Phase 3: TypeScriptTypeProvider                   (2-3 weeks)
E12 Phase 4: JavaTypeProvider                         (1 week)
E12 Phase 5: RustTypeProvider                         (1 week)
E12 Phase 6: PythonTypeProvider                       (3-5 days)
E12 Phase 7: RTA across all dispatch languages        (2-3 days)
    ↓
E13 extended: Serialized CPG + type cache             (1-2 weeks)
    ↓
Follow-on: Type-enriched finding descriptions         (1 week)
Follow-on: Import map improvements (re-exports, etc.) (1 week)
    ↓
CompatibilitySlice Phase 1: Feature matrix checks     (1 week)
CompatibilitySlice Phase 2: Type definition deltas    (2-3 weeks)
```

Phases 2+3 can partially parallelize (Go and TS providers share no code).
Phases 4–6 are independent and can run in any order or batch.
