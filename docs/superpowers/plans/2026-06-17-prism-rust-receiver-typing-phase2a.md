# Rust Receiver-Typing (Phase 2a) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans. Steps use checkbox (`- [ ]`). Execution loop for this repo: codex gpt-5.5
> (implement effort=high) + codex gpt-5.5 (review effort=xhigh) via a2a-bridge, orchestrator verifies each
> diff vs base. **rev 4 — folds codex gpt-5.5 xhigh plan-review rounds 1+2+3 (implementation mechanics).**

**Goal:** resolve Rust `x.method()` by the receiver's static type — adding qualified type identity,
field/return receiver recovery, and a kind-aware confidence combine — so the standing receiver-method and
cross-module `prism_fp` over-claims drop and method recall rises, **without regressing any current behavior**.

**Architecture:** Receiver typing is a **build-time materialization** done in a **post-pass**. Because the live
build writes `receiver_type` during call-site collection (`call_graph.rs:519-540`) but only populates
`scope_graph` at the end (`:882-900`), the identity indices cannot exist during call-site collection — and,
critically, **the inline classifier + the legacy `receiver_type` field MUST stay untouched**, because
`resolve_call_site` already reads `receiver_type` (`resolution.rs:781`), so adding new recovery there would
change edges before PR-3 (round-2 BLOCKER #1). Therefore **all Phase-2a recovery lives in a new build-time
`RustReceiverTyper` invoked by a post-pass — `rematerialize_rust_receiver_keys`** — that runs after the scope
graph + identity indices are built and sets **only the new `receiver_outcome` field** (legacy `receiver_type`
unchanged), **updating it in place on both `cg.calls` and `cg.callers`** (order-preserving; do NOT rebuild
`callers` from the `BTreeSet`-backed `calls` — that reorders, round-2 #4). `resolve_call_site` (the read path,
which sees only a serialized `CallSite`) reads the key only in PR-3 and runs the kind-aware combine over
identity-keyed indices on `CallGraph`. Three PRs: **PR-1** indices + `MethodFacts` (scope-aware extraction) +
`resolve_type_path_to_type_scope` + the identity-complete guard (inert); **PR-2** the `RustReceiverTyper` +
the post-pass materializing `receiver_outcome` only (read-inert — `receiver_type` untouched); **PR-3** the
read path (kind-aware combine, identity-or-fall-through-to-bare, drop-on-miss) + the Tier-A 2a gate.
Design-of-record: `docs/superpowers/specs/2026-06-17-prism-rust-receiver-typing-design.md` (rev 7).

**Tech Stack:** Rust, tree-sitter, `petgraph`, `serde`/`bincode` (CPG cache), the Phase-1 scope graph
(`src/name_resolution/`), the Tier-A harness (`eval/`, Python/uv/pytest, oracle rust-analyzer).

**Non-regression contract:** PR-1/PR-2 are **inert** (no edge change; gated by `cargo test` green + a Tier-A
`--matrix-only` parity check). PR-3 is the only behavior change and is **Tier-A-measured** — two non-removing
levers (confidence refinement; identity-or-fall-through-to-bare with the identity-complete guard) plus the
**one** edge-removing lever (drop-on-miss for newly-recovered receivers, where the receiver-method FPs die) —
reported precision-up / recall-held in the PR description.

---

## File Structure

**PR-1:** `src/call_graph.rs` (`MethodFacts` + `method_facts`/`methods_by_scope`/`identity_complete`/
`field_types`/`return_types` indices; populate after `populate_scope_graph`), `src/resolution_identity.rs`
(new — `TypeKey`/`ReceiverTypeKey`, `resolve_type_path_to_type_scope`, `canonical_external`),
`src/type_providers/rust_provider.rs` (factor field/return/method-AST extraction for reuse; free-fn returns +
the `aliases` map), `src/cpg_cache.rs` (`CACHE_VERSION` 12→13), `src/languages/mod.rs` (param/self accessors).

**PR-2:** `src/call_graph.rs` (`CallSite.receiver_outcome` + all `CallSite` literals + the
`rematerialize_rust_receiver_keys` post-pass that updates `receiver_outcome` **in place** on `calls`+`callers`),
`src/resolution.rs` (a NEW build-time `RustReceiverTyper` with path-preserving `type_syntax` +
field/return/wrapper recovery — the inline `ExpandedClassifier`/`receiver_type` are **untouched**),
`src/navigation/queries.rs` (exhaustive `ReceiverRecovery` match), `src/name_resolution/binding_lookup.rs`
(new — direct visible-binding lookup, F5), `src/ast.rs` (surface binding/init facts), `src/cpg_cache.rs`
(`CACHE_VERSION` 13→14), test literals in `tests/name_resolution/` + `tests/navigation/`.

**PR-3:** `src/resolution.rs` (`combine_kind` + the Rust R6 read branch w/ identity-complete guard),
`src/cpg/build.rs` (F6: call `rematerialize_rust_receiver_keys` on incremental rebuild),
`eval/fixtures/rust/{field_typed_recovery,return_typed_recovery,extension_trait_method,
cross_module_no_collision}/`, `tests/integration/resolution_test.rs`.

*(Telemetry — `nav call-stats` recovery/kind histogram — is deferred to Phase 2b per spec §9; not in this plan.)*

---

## PR-1 — Foundation: facts + identity indices + the guard (inert)

### Task 1.1: `MethodFacts` + `method_facts` index (AST-derived; the Rust arity source)

**Files:** Modify `src/call_graph.rs` (`CallGraph` ~:114-170; the method-collection pass near `method_metadata`
~:1335-1352), `src/languages/mod.rs` (add `method_params`/`self_param` accessors if absent). Test: inline.

- [ ] **Step 1: Write the failing test** in `src/call_graph.rs` `#[cfg(test)]`:

```rust
#[test]
fn method_facts_distinguish_inherent_trait_self_arity() {
    use crate::ast::ParsedFile;
    use crate::languages::Language::Rust;
    let mut files = std::collections::BTreeMap::new();
    files.insert("a.rs".to_string(), ParsedFile::parse("a.rs",
        // `td` is a DEFAULT-BODY trait method (a `function_item`, hence a FunctionId);
        // a signature-only `fn d(&self);` is a `function_signature_item` and is NOT collected
        // as a FunctionId today (round-3 #2), so it is not a method_facts target — do not use it here.
        "struct S;\nimpl S { fn inh(&self, x: u8) {} fn assoc() {} }\n\
         trait T { fn td(&self) {} }\nimpl T for S { fn tm(&self) {} }\n", Rust).unwrap());
    let cg = CallGraph::build(&files);
    let fid = |n: &str| cg.functions.values().flatten().find(|f| f.name == n).cloned().unwrap();
    let inh = cg.method_facts.get(&fid("inh")).unwrap();
    assert_eq!(inh.kind, MethodKind::Inherent);
    assert!(inh.has_self && inh.arity_excl_self == 1);
    assert!(!cg.method_facts.get(&fid("assoc")).unwrap().has_self);
    // a default-body trait-declaration method -> Trait (NOT Inherent), via the enclosing-trait_item check
    assert!(matches!(cg.method_facts.get(&fid("td")).unwrap().kind, MethodKind::Trait(_)));
    // an impl-for-trait method -> Trait
    assert!(matches!(cg.method_facts.get(&fid("tm")).unwrap().kind, MethodKind::Trait(_)));
}
```

- [ ] **Step 2: Run to verify it fails.** `cargo test --lib method_facts_distinguish` → FAIL (undefined types).
- [ ] **Step 3: Implement.** Add `MethodKind { Inherent, Trait(String) }`, `RecvMode { None, SelfBy, SelfRef,
  SelfRefMut }`, `MethodFacts { kind, has_self, recv_mode, arity_excl_self, cfg: Option<String> }` (all
  `serde` + `PartialEq, Eq, Clone`), and `pub method_facts: BTreeMap<FunctionId, MethodFacts>` on `CallGraph`
  (init empty at every construction site). Derive facts from the AST during the existing method walk. **`kind`
  (round-2 #2 — three cases, not two):** if the method's enclosing item is a `trait_item` (`trait T { fn m
  … }` — declaration OR default body) → `Trait(trait_name)`; else if `rust_impl_trait(func_node)` is `Some`
  (`impl Tr for Type`) → `Trait(trait_name)`; else (inherent `impl Type { … }`) → `Inherent`. (A bare
  `rust_impl_trait`-only check would mis-tag trait-body methods as `Inherent` and let PR-3 emit a wrong
  `Exact` — the round-2 finding. Add a `languages` accessor or a node-kind check for the enclosing
  `trait_item` vs `impl_item`.) `has_self`/`recv_mode`/`arity_excl_self` from the param list (add
  `method_params`/`self_param` to `src/languages/mod.rs` mirroring `method_owner`); `cfg` from a `#[cfg(...)]`
  attr. **Do NOT alter the existing `methods`/`method_owners` population** (inertness — round-1 #8).
  `arity_excl_self` is the **Rust arity source** for PR-3's combine (the Go-only `method_arity` is unused for
  Rust). The Step-1 test already asserts a default-body trait method (`trait T { fn td(&self) {} }`) is
  `Trait(_)`, not `Inherent`. (Signature-only `fn d(&self);` items are `function_signature_item`, not collected
  as FunctionIds today — do NOT add them to function collection, which would break PR-1 inertness; round-3 #2.)
- [ ] **Step 4: Run to verify pass.** PASS.
- [ ] **Step 5: Commit.** `git commit -am "feat(rust-recv): MethodFacts (AST kind/has_self/recv_mode/arity_excl_self)"`

### Task 1.2: `TypeKey`/`ReceiverTypeKey` + `resolve_type_path_to_type_scope` (in-repo-first) + canonical External

**Files:** Create `src/resolution_identity.rs` (`pub mod` in `src/lib.rs`); re-export from `src/resolution.rs`.
Test: inline (use a real graph built via `CallGraph::build`, read `cg.scope_graph`).

- [ ] **Step 1: Write the failing test** (concrete — no placeholder helpers):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::ast::ParsedFile;
    use crate::languages::Language::Rust;
    fn graph_of(srcs: &[(&str, &str)]) -> crate::name_resolution::graph::ScopeGraph {
        let mut files = std::collections::BTreeMap::new();
        for (p, s) in srcs { files.insert(p.to_string(), ParsedFile::parse(p, s, Rust).unwrap()); }
        crate::call_graph::CallGraph::build(&files).scope_graph.expect("scope graph")
    }
    #[test]
    fn in_repo_first_then_external_and_unresolved_is_none() {
        let g = graph_of(&[("a.rs","pub struct Foo;"), ("b.rs","pub struct Foo;")]);
        let a = module_scope(&g, "a.rs"); let b = module_scope(&g, "b.rs");
        let ka = resolve_type_path_to_type_scope(&g, a, "Foo");
        let kb = resolve_type_path_to_type_scope(&g, b, "Foo");
        assert!(matches!((&ka,&kb),(Some(TypeKey::InRepo(x)),Some(TypeKey::InRepo(y))) if x!=y));
        assert_eq!(resolve_type_path_to_type_scope(&g, a, "String"), Some(TypeKey::External("String".into())));
        assert_eq!(resolve_type_path_to_type_scope(&g, a, "NoSuch"), None); // populator gap -> bare fallback
    }
    #[test]
    fn in_repo_struct_named_string_is_not_external() {
        let g = graph_of(&[("a.rs","pub struct String;")]);
        let a = module_scope(&g, "a.rs");
        assert!(matches!(resolve_type_path_to_type_scope(&g, a, "String"), Some(TypeKey::InRepo(_))));
    }
    #[test]
    fn external_canonicalization_unifies_std_paths() {
        assert_eq!(canonical_external("String"), canonical_external("std::string::String"));
        assert_eq!(canonical_external("std::string::String"), canonical_external("alloc::string::String"));
    }
}
```
(Provide a small `module_scope(&g, path)` test helper that finds the file's module scope via
`g.file_paths` + the root scope for that file.)

- [ ] **Step 2: Run to verify it fails.** `cargo test --lib resolution_identity` → FAIL.
- [ ] **Step 3: Implement.** `TypeKey { InRepo(ScopeId), External(String) }`,
  `ReceiverTypeKey { InRepo(ScopeId), External(String), Bare(String) }`, and the **self-contained materialized
  record** (round-3 BLOCKER — PR-3 needs the recovery + bare key, and the legacy `receiver_recovery` is read
  only by the legacy branch so it can't be reused):
  `ReceiverOutcome { key: ReceiverTypeKey, bare: String /* owner_key fallback for the identity-complete guard */,
  recovery: ReceiverRecovery /* drives StdWrapperPeel demotion in combine_kind */ }` (all serde + Eq). **In-repo-first
  ordering (round-1 #4):**

```rust
pub fn resolve_type_path_to_type_scope(graph, from: ScopeId, type_syntax: &str) -> Option<TypeKey> {
    let peeled = crate::resolution::peel_type(type_syntax);   // keeps the `::` path; strips refs/wrappers/dyn/generics
    if peeled.is_empty() { return None; }
    // 1. try in-repo resolution FIRST (so an in-repo `struct String` wins over the prelude)
    let target = if peeled.contains("::") {
        resolve_via_path(graph, from, &peeled)          // engine::resolve_path, NS_TYPE
    } else {
        resolve_via_lexical(graph, from, &peeled)       // engine::resolve, NS_TYPE
    };
    if let Some(t) = target.and_then(|t| type_scope_of_target(graph, &t)) {
        return Some(TypeKey::InRepo(t));
    }
    // 2. only if in-repo resolution failed: confidently-external -> External(canonical); else None (bare fallback)
    if is_confidently_external(&peeled) { return Some(TypeKey::External(canonical_external(&peeled))); }
    None
}
```
  `is_confidently_external` = leading segment ∈ {`std`,`core`,`alloc`} or a known dep, OR a known std bare
  type (`String`,`Vec`,`BTreeMap`,…) — a small explicit set, extended via Tier-A. `canonical_external` =
  last-segment normalization + a known-std canonical map (so `String`/`std::string::String`/
  `alloc::string::String` unify — G3). `type_scope_of_target`: `Target::Item{owns:Some(s),..}`→`s`,
  `Target::Scope(s)`→`s`, else `None`.
- [ ] **Step 4: Run to verify pass.** PASS.
- [ ] **Step 5: Commit.** `git commit -am "feat(rust-recv): TypeKey + in-repo-first resolve_type_path_to_type_scope + canonical External"`

### Task 1.3: `methods_by_scope` identity index + `identity_complete` guard (R1 + G2)

**Files:** Modify `src/call_graph.rs` (a step after `populate_scope_graph`). Test: inline.

- [ ] **Step 1: Write the failing test.**

```rust
#[test]
fn methods_by_scope_distinct_per_module_and_marks_complete() {
    use crate::ast::ParsedFile; use crate::languages::Language::Rust;
    let mut files = std::collections::BTreeMap::new();
    files.insert("a.rs".into(), ParsedFile::parse("a.rs","pub struct Foo; impl Foo { fn m(&self){} }\n", Rust).unwrap());
    files.insert("b.rs".into(), ParsedFile::parse("b.rs","pub struct Foo; impl Foo { fn m(&self){} }\n", Rust).unwrap());
    let cg = CallGraph::build(&files);
    assert_eq!(cg.methods_by_scope.keys().filter(|(_,n)| n=="m").count(), 2);
    // both Foo::m resolved their impl-header -> the bare ("Foo","m") bucket is identity-complete
    assert!(cg.identity_complete.contains(&("Foo".to_string(),"m".to_string())));
}
```

- [ ] **Step 2: Run to verify it fails.** FAIL.
- [ ] **Step 3: Implement.** Add `pub methods_by_scope: BTreeMap<(ScopeId,String), Vec<FunctionId>>` and
  `pub identity_complete: BTreeSet<(String,String)>` to `CallGraph`. After `populate_scope_graph` (when
  `graph.complete`), iterate methods: resolve each method's **impl-header type path** to
  `TypeKey::InRepo(scope)` via `resolve_type_path_to_type_scope(graph, impl_module_scope, impl_header_syntax)`
  (use the path-preserving impl-header text, not `owner_key`). On success, insert `(scope, name)->fid` (dual-
  key the trait's resolved scope too). **Guard (G2):** a bare bucket `(owner_key, name)` is added to
  `identity_complete` **iff EVERY** `fid` in `methods[(owner_key,name)]` resolved to an identity scope; if any
  impl-header failed to resolve, the bucket is NOT complete (PR-3 will bare-fall-back for it). Build only when
  `graph.complete`.
- [ ] **Step 4: Run to verify pass.** PASS.
- [ ] **Step 5: Commit.** `git commit -am "feat(rust-recv): methods_by_scope + identity_complete guard (R1 + G2)"`

### Task 1.4: `field_types` + `return_types` indices (Self/alias/cfg, def-scope)

**Files:** Modify `src/type_providers/rust_provider.rs` (factor + free-fn returns), `src/call_graph.rs` (store
resolved indices). Test: inline in `src/call_graph.rs` (concrete — resolve expected scopes from `cg.scope_graph`).

- [ ] **Step 1: Write the failing test.**

```rust
#[test]
fn field_and_return_indices_resolved_self_and_def_scope() {
    use crate::ast::ParsedFile; use crate::languages::Language::Rust;
    let mut files = std::collections::BTreeMap::new();
    files.insert("a.rs".into(), ParsedFile::parse("a.rs",
        "pub struct Inner; impl Inner { fn poke(&self){} pub fn new()->Self{Inner} }\n\
         pub struct Outer { pub inner: Inner }\npub fn make()->Inner{Inner}\n", Rust).unwrap());
    let cg = CallGraph::build(&files);
    let inner = match resolve_test_type(&cg, "a.rs", "Inner") { TypeKey::InRepo(s)=>s, _=>panic!() };
    let outer = match resolve_test_type(&cg, "a.rs", "Outer") { TypeKey::InRepo(s)=>s, _=>panic!() };
    assert_eq!(cg.field_types.get(&(outer,"inner".into())).and_then(|v|v.first()),
               Some(&(None, TypeKey::InRepo(inner))));
    let make_fid = cg.functions.values().flatten().find(|f| f.name=="make").cloned().unwrap();
    let new_fid  = cg.functions.values().flatten().find(|f| f.name=="new").cloned().unwrap();
    assert_eq!(cg.return_types.get(&make_fid).and_then(|v|v.first()), Some(&(None, TypeKey::InRepo(inner))));
    assert_eq!(cg.return_types.get(&new_fid).and_then(|v|v.first()),  Some(&(None, TypeKey::InRepo(inner)))); // Self->Inner
}
```
(`resolve_test_type` = a thin test helper wrapping `resolve_type_path_to_type_scope` on `cg.scope_graph`.)

- [ ] **Step 2: Run to verify it fails.** FAIL.
- [ ] **Step 3: Implement.** Add `pub field_types: BTreeMap<(ScopeId,String), Vec<(Option<String>,TypeKey)>>`
  and `pub return_types: BTreeMap<FunctionId, Vec<(Option<String>,TypeKey)>>`. Write a **new scope-aware
  extraction pass** that reuses `rust_provider`'s tree-walk *pattern* (struct/impl/fn visitation) but produces
  facts keyed by **module/scope + the defining item**, NOT by `rust_provider`'s existing **bare-name, global**
  `structs`/`methods`/`aliases` maps (round-2 #3 — reading those as-is corrupts cross-module identity). For
  each field/return type string: resolve **in the defining item's module-scope** (R9) via
  `resolve_type_path_to_type_scope` (which is in-repo-first, so it is collision-safe); resolve `Self` to the
  impl owner's scope (R6/§4); resolve a **type alias** by extracting `type Foo = Bar` **per defining module**
  (scope-local) and resolving the target in that scope — do NOT use `rust_provider`'s global bare alias table
  (round-2 #6); return the entry only if the target resolves, else omit (never guess — round-1 #6);
  **cfg-condition** (two `#[cfg]` alternatives → two entries, never merged). Unresolvable → omit (typer falls
  through). Build alongside Task 1.3 (needs the graph).
- [ ] **Step 4: Run to verify pass.** PASS.
- [ ] **Step 5: Commit.** `git commit -am "feat(rust-recv): field_types + return_types (Self/alias/cfg-resolved, def-scope)"`

### Task 1.5: CACHE_VERSION bump + inert parity → PR-1

- [ ] **Step 1:** `src/cpg_cache.rs:53` `CACHE_VERSION` 12→13.
- [ ] **Step 2:** `cargo fmt && cargo test` → green (indices populated, unread).
- [ ] **Step 3:** `cargo build --release && cd eval && uv run tier-a --matrix-only --allow-stale-sut` →
  identical matrix to base (0 regressions/flips). Paste in the PR.
- [ ] **Step 4: Commit + open PR-1.** `git commit -am "chore(rust-recv): CACHE_VERSION 12->13 (PR-1 indices)"`.
  PR body: "PR-1/3 FOUNDATION (inert): facts + methods_by_scope + identity_complete + field/return indices +
  resolve_type_path_to_type_scope. Unread; matrix parity attached."

---

## PR-2 — `RustReceiverTyper` + the post-pass (read-inert; legacy `receiver_type` untouched)

> **Inertness contract (round-2 BLOCKER #1):** the inline classifier (`ExpandedClassifier` at the build path
> `:528`) and the legacy `CallSite.receiver_type` field are **NOT modified** in PR-2. All Phase-2a recovery
> lives in a new `RustReceiverTyper` invoked by the post-pass, which sets **only `receiver_outcome`** — a
> field unread until PR-3. So PR-2 cannot change any edge.

### Task 2.1: `CallSite.receiver_outcome` (serde default; excluded from `cmp_key`; CACHE_VERSION 13→14)

**Files:** `src/call_graph.rs` (`CallSite` :43-78; the `cmp_key` :1606-1628), `src/cpg_cache.rs`, **and every
`CallSite` literal in the tree** — `call_graph.rs` construction sites (:276, :528, :609, :642, :728, :824,
:850, :1260) PLUS test literals `tests/name_resolution/consumer_test.rs:90-108` and
`tests/navigation/scoped_calls_test.rs:49-61` (round-2 #5 — these are missed by an "8 sites" count). Test: inline.

- [ ] **Step 1: Write the failing test** (`callsite_receiver_outcome_serde_default_and_excluded_from_cmp_key`)
  asserting two `CallSite`s differing only in `receiver_outcome` have equal `cmp_key()`, and the field
  serde-round-trips. Build the sites via a real `CallGraph::build` + `site_in` (not a placeholder `mk_test_site`).
- [ ] **Step 2: Run to verify it fails.** FAIL.
- [ ] **Step 3: Implement.** Add `#[serde(default)] pub receiver_outcome: Option<crate::resolution_identity::
  ReceiverOutcome>` to `CallSite` (the self-contained `{ key, bare, recovery }` record — round-3 BLOCKER); set
  `None` at **every** literal above (grep `CallSite {` across the whole tree to enumerate — incl. the test
  files). **EXCLUDE it from `cmp_key`** (round-1 #8 — `receiver_type` is *included* in `cmp_key`; the new field
  must be *excluded* so set membership/iteration is unperturbed → inertness). The legacy `receiver_recovery`
  field is **left semantically unchanged** (it is read by the legacy `receiver_type` branch AND by nav
  interface telemetry at `navigation/queries.rs:82` — round-4 MINOR; NOT an inertness breach because PR-2
  writes only `receiver_outcome`); the new path uses `receiver_outcome.recovery`. Keep the exhaustive
  `ReceiverRecovery` match (`queries.rs:82-88`) updated for the new variants (Task 2.3). Bump
  `src/cpg_cache.rs` `CACHE_VERSION` 13→14 + update the `CACHE_VERSION == N` pin test (round-2 #5/#7).
- [ ] **Step 4: Run to verify pass.** `cargo test` (incl. the moved test literals compile). PASS.
- [ ] **Step 5: Commit.** `git commit -am "feat(rust-recv): CallSite.receiver_outcome (serde default, cmp_key-excluded); CACHE_VERSION 13->14"`

### Task 2.2: Direct visible-binding lookup (F5) + def-byte local facts

**Files:** Create `src/name_resolution/binding_lookup.rs`. Test: inline (concrete graph).

- [ ] **Step 1: Write the failing test** asserting `lookup_visible_binding(&g, file, call_byte, "b")` returns
  the `Binding` whose `Span.lo.byte` is the `let b` def site (build a real graph; compute the call byte from
  the source).
- [ ] **Step 2: Run to verify it fails.** FAIL.
- [ ] **Step 3: Implement.** `pub fn lookup_visible_binding(graph, file: FileId, at_byte: usize, name: &str)
  -> Option<&Binding>` — `enclosing_scope`, walk inner→outer ribs, return the nearest `Binding` matching
  `name` whose `vis_extents` cover `at_byte` (same range gate as `resolve`). Returns the `Binding` (carrying
  the `Span`/`def_byte`), NOT a `Candidate` (which drops binding identity — round-1 #5). Add
  `local_facts: BTreeMap<(FileId, usize /*def_byte*/), LocalFact>`,
  `LocalFact { kind: Param|Let|Pattern, annotation: Option<String>, init: Option<InitExpr> }`,
  `InitExpr` capturing `T::new()`/`T{…}`/`e.f`/`g(…)` shapes — built from the populator's binding spans + AST.
- [ ] **Step 4: Run to verify pass.** PASS.
- [ ] **Step 5: Commit.** `git commit -am "feat(rust-recv): direct visible-binding lookup + def-byte local facts (F5)"`

### Task 2.3: The build-time `RustReceiverTyper` (distinct from the inline classifier; path-preserving)

**Files:** `src/resolution.rs` (a NEW `RustReceiverTyper` — do NOT modify `ExpandedClassifier`/`recover_simple_
ident`/the `:528` write of `receiver_type`); `src/ast.rs`; `src/navigation/queries.rs` (exhaustive
`ReceiverRecovery` match :82-88). Test: inline (the typer is tested directly; materialization is Task 2.4).

- [ ] **Step 1: Write the failing tests** (call the typer directly on a built graph + indices; assert the
  recovered `type_syntax` + `recovery` for each form). E.g. for `fn run(o: Outer){ let x=o.inner; x.poke(); }`
  the typer returns `recovery == FieldTyped` and a `type_syntax` resolving to `Inner`'s scope; for `let x =
  make(); x.run()` → `ReturnTyped`; for `fn f(x: Box<W>){ x.go() }` → `StdWrapperPeel`; for `fn f(x:
  crate::a::Foo){ x.m() }` → `type_syntax` retains the path (`crate::a::Foo`, NOT `owner_key`'d to `Foo`).
- [ ] **Step 2: Run to verify they fail.** FAIL.
- [ ] **Step 3: Implement.** Add `ReceiverRecovery::{FieldTyped, ReturnTyped, StdWrapperPeel}` and update the
  exhaustive match at `navigation/queries.rs:82-88` (round-2 #5). Add a `RustReceiverTyper` that, given a call
  site + the scope graph + the identity indices (`field_types`/`return_types`) + `local_facts`, returns a
  recovered receiver `{ type_syntax: String /* path-preserving peel_type, NOT owner_key — round-2 #5 */,
  static_type: String /* bare fallback */, recovery }`, recovering in order: `self`/`Self`→owner;
  param/typed-let/constructor (reuse the helper logic shared with `recover_simple_ident`, but produce
  `type_syntax`); **field-typed** (`local_facts.init == e.f` / receiver is `self.f`/`e.f` → resolve
  `typeof(e)` → `field_types[(typeof(e), f)]`); **return-typed** (`init == g(…)` / receiver is `g(…)` → resolve
  `g` → `return_types[fid(g)]`, **depth-cap ≤4, revisited `(FileId,def_byte)`/`FunctionId` → None**);
  **wrapper-peel** (`Box/Arc/Rc/Pin` peeled → tag `StdWrapperPeel`, inner type preserved); else None. This
  typer is the SOLE Phase-2a recovery; the inline `ExpandedClassifier` + `receiver_type` are untouched.
- [ ] **Step 4: Run to verify pass.** PASS.
- [ ] **Step 5: Commit.** `git commit -am "feat(rust-recv): build-time RustReceiverTyper (field/return/wrapper, path-preserving type_syntax)"`

### Task 2.4: `rematerialize_rust_receiver_keys` post-pass — set `receiver_outcome` in place

**Files:** `src/call_graph.rs` (new post-pass, called at the end of the build after `populate_scope_graph` +
the identity indices, before `assemble`/return). Test: inline + integration + inert parity.

- [ ] **Step 1: Write the failing tests** (concrete, via `build`/`site_in`):

```rust
#[test]
fn rematerialize_sets_receiver_outcome_and_keeps_calls_callers_in_sync() {
    use prism::languages::Language::Rust;
    let (cg, _) = build(&[("a.rs",
        "struct Inner; impl Inner { fn poke(&self){} } struct Outer{inner:Inner}\n\
         fn run(o: Outer){ let x=o.inner; x.poke(); }\n", Rust)]);
    let oc = site_in(&cg, "run", "poke").receiver_outcome.unwrap();
    assert!(matches!(oc.key, prism::resolution_identity::ReceiverTypeKey::InRepo(_)));
    assert_eq!(oc.recovery, prism::resolution::ReceiverRecovery::FieldTyped);
    // updated IN PLACE on both maps -> callers carries the same outcome (order preserved)
    let from_callers = cg.callers.get("poke").unwrap().iter()
        .find(|s| s.caller.name=="run").unwrap().receiver_outcome.clone();
    assert_eq!(Some(oc), from_callers);
}

#[test]
fn bare_fallback_when_receiver_type_unresolved() {
    use prism::languages::Language::Rust;
    let (cg, _) = build(&[("a.rs","fn f(x: Unresolvable){ x.m(); }\n", Rust)]);
    let oc = site_in(&cg,"f","m").receiver_outcome.unwrap();
    assert!(matches!(oc.key, prism::resolution_identity::ReceiverTypeKey::Bare(ref s) if s=="Unresolvable"));
    assert_eq!(oc.bare, "Unresolvable"); // bare key carried for the identity-complete guard
}
```

- [ ] **Step 2: Run to verify they fail.** FAIL.
- [ ] **Step 3: Implement.** Add `fn rematerialize_rust_receiver_keys(&mut self, files: &BTreeMap<String,
  ParsedFile>)`: for each Rust `CallSite`, run the `RustReceiverTyper` (it has the scope graph + identity
  indices + local facts) to get `{ type_syntax, static_type (bare), recovery }`; resolve `type_syntax` via
  `resolve_type_path_to_type_scope(graph, caller_module_scope, type_syntax)` → `InRepo(s)` / `External(e)` /
  `None`⇒`Bare(static_type)`; assemble `ReceiverOutcome { key, bare: static_type, recovery }` and **set
  `receiver_outcome` IN PLACE on the matching `CallSite` in BOTH `self.calls` and `self.callers`** (match by
  the site's logical identity / `cmp_key` + caller). Because `self.calls` is `BTreeMap<FunctionId,
  BTreeSet<CallSite>>`, the set element must be **removed and re-inserted** with the field set — and since
  `receiver_outcome` is `cmp_key`-excluded, the element's set position is unchanged, so iteration order is
  preserved (round-3 #4 note). The `self.callers` `Vec<CallSite>` is mutated in place. Do NOT rebuild
  `callers` from `calls` (that reorders — round-2 #4). **Leave `receiver_type` UNCHANGED.** Call at the end of
  the build (after `populate_scope_graph` + Tasks 1.3/1.4). Non-Rust sites are skipped.
- [ ] **Step 4: Run + inert parity.** `cargo test`; `cargo build --release && cd eval && uv run tier-a
  --matrix-only --allow-stale-sut` → identical matrix (read path still ignores `receiver_outcome`; `calls`/
  `callers` order unchanged). Paste in PR.
- [ ] **Step 5: Commit + open PR-2.** `git commit -am "feat(rust-recv): rematerialize_rust_receiver_keys post-pass (in-place; receiver_type untouched)"`.
  PR body: "PR-2/3: RustReceiverTyper + post-pass setting only receiver_outcome (in place, order-preserving).
  Read-inert (receiver_type + inline classifier untouched); matrix parity attached."

---

## PR-3 — Read path + the Tier-A gate (the behavior change)

### Task 3.1: `combine_kind` (arity-aware; MethodFacts-driven)

**Files:** `src/resolution.rs`. Test: inline (build a tiny `CallGraph`, use real `FunctionId`s + `method_facts`).

- [ ] **Step 1: Write the failing test** (concrete: build a graph with one inherent + one trait `m`, fetch
  their `FunctionId`s + `method_facts`, assert: single inherent→Exact; single trait→NameOnly; single
  StdWrapperPeel→NameOnly; multi→all NameOnly; empty→None; arity-mismatch single→None).
- [ ] **Step 2: Run to verify it fails.** FAIL.
- [ ] **Step 3: Implement.** `fn combine_kind(cands: &[FunctionId], facts: &BTreeMap<FunctionId,MethodFacts>,
  recovery: ReceiverRecovery, arg_count: Option<usize>, arg_spread: bool) -> Option<Vec<ResolvedCallee>>`:
  filter to `facts[fid].has_self`; **arity-filter using `MethodFacts.arity_excl_self`** (plan-review #9 — NOT
  the Go-only `method_arity`): drop a candidate only on a confident exact mismatch (`Some(n)` &&
  `!arg_spread` && `arity_excl_self != n`). Then: empty→`None` (drop); single→`Exact` iff
  `kind==Inherent && recovery != StdWrapperPeel`, else `NameOnly`; multi→`TraitCha` demote (NameOnly all).
  Kind label: a new `ResolutionKind::ReceiverTyped` (or reuse TypedParam/FieldTyped/ReturnTyped per recovery).
- [ ] **Step 4: Run to verify pass.** PASS.
- [ ] **Step 5: Commit.** `git commit -am "feat(rust-recv): combine_kind (arity via MethodFacts; inherent->Exact, trait/wrapper->NameOnly)"`

### Task 3.2: Rust R6 read branch (identity-or-fall-through-to-bare with the identity-complete guard)

**Files:** `src/resolution.rs` (R6 path ~:780-835). Test: `tests/integration/resolution_test.rs`.

- [ ] **Step 1: Write the failing tests** (the behavior change + non-regression + the G2 guard):

```rust
#[test] fn cross_module_collision_not_minted_when_both_resolve() { /* x: b::Foo; x.m() -> 1 edge, not 2 */ }
#[test] fn trait_static_dispatch_found_as_nameonly() { /* f: Fast; f.go() -> 1 edge, NameOnly */ }
#[test] fn trait_dyn_dispatch_found_as_nameonly() { /* r: &dyn Runner; r.go() -> 1 edge, NameOnly */ }
#[test] fn external_recv_no_in_repo_method_drops() { /* m: BTreeMap; m.target() with fn target() -> empty */ }
#[test] fn extension_trait_on_external_resolves() { /* impl Ext for String { fn ext } ; s.ext() -> 1 edge */ }
#[test] fn unrecovered_receiver_still_hits_residue() { /* let x=mystery(); x.frobnicate() -> R6SingleOwner */ }
#[test] fn incomplete_identity_bucket_falls_back_to_bare() {
    // Construct it so the receiver resolves InRepo(scope) but the method's impl-header fails identity
    // resolution and lands ONLY in the bare `methods` bucket (round-4 do-now): e.g. the impl is in a nested/
    // macro-emitted position the populator does not resolve, so methods_by_scope[(scope,"name")] is empty
    // while methods[("Owner","name")] has it and ("Owner","name") is NOT in identity_complete ->
    // bare owner_lookup fires, the edge is present (NOT dropped).
}
```
(Write each with the `build`/`site_in` helpers + concrete sources, asserting `r.len()` and
`r[0].confidence`/`r[0].kind`.)

- [ ] **Step 2: Run to verify they fail.** FAIL.
- [ ] **Step 3: Implement.** In `resolve_call_site_full`'s R6 branch, BEFORE the existing `site.receiver_type`
  string path, add (Rust only) a branch on `site.receiver_outcome` (`oc = { key, bare, recovery }` — all
  carried on the record, round-3 BLOCKER, so no reliance on the legacy `receiver_recovery`):
  - `Some(oc)` with `oc.key == InRepo(scope)` → `cands = methods_by_scope[(scope, name)]`;
    `combine_kind(cands, &self.method_facts, oc.recovery, site.arg_count, site.arg_spread)` (the `oc.recovery`
    drives the `StdWrapperPeel`→NameOnly demotion). `Some`→hit. `None` (empty): **if** the bare bucket
    `(oc.bare, name)` is in `identity_complete` → drop (`ExternalReceiver`); **else** fall back to
    `self.owner_lookup(oc.bare, name)` (the G2 guard — uses `oc.bare`, round-1 #3 / round-3 BLOCKER).
  - `Some(oc)` with `oc.key == External(canon)` → `cands = self.methods[(canon, name)]` (catches in-repo
    extension impls keyed under the external type); `combine_kind(cands, …, oc.recovery, …)`; empty→drop.
  - `Some(oc)` with `oc.key == Bare(s)` → today's behavior: `self.owner_lookup(s, name)` + the existing
    Go-interface/`ExternalReceiver` fall-through.
  - `None` → existing residue block (unchanged).
  The Go path (Go sets `receiver_type`, not `receiver_outcome`) is untouched. (`oc.bare` makes the G2 guard
  directly checkable — no need to back-derive a key from a string; the test
  `incomplete_identity_bucket_falls_back_to_bare` pins it.)
- [ ] **Step 4: Run to verify pass.** `cargo test --test integration resolution_test::` + full `cargo test` green.
- [ ] **Step 5: Commit.** `git commit -am "feat(rust-recv): R6 reads ReceiverTypeKey — kind-aware combine + identity-complete-guarded bare fallback + drop-on-miss"`

### Task 3.3: F6 — re-materialize on incremental rebuild (rebuild-together)

**Files:** `src/cpg/build.rs` (incremental path ~:221-234). Test: inline (incremental harness).

- [ ] **Step 1: Write the failing test** — build, edit a file changing a field's type, incremental-rebuild,
  assert the dependent caller's `receiver_outcome` reflects the new type (no stale `ScopeId`).
- [ ] **Step 2: Run to verify it fails.** FAIL.
- [ ] **Step 3: Implement.** In the incremental path, after `merge` + `rebuild_scope_graph`, **rebuild the
  identity indices (`methods_by_scope`/`identity_complete`/`field_types`/`return_types`) whole-program and call
  `rematerialize_rust_receiver_keys(files)`** (which updates `receiver_outcome` in place on `calls`+`callers`)
  — the same replace-not-merge
  discipline as `apply_go_embedding_promotion`/`apply_go_interface_dispatch`. This must run before any read
  path uses the keys (plan-review #7).
- [ ] **Step 4: Run to verify pass.** PASS.
- [ ] **Step 5: Commit.** `git commit -am "fix(rust-recv): re-materialize receiver keys on incremental rebuild (F6)"`

### Task 3.4: Capability fixtures + matrix

- [ ] Add `eval/fixtures/rust/{field_typed_recovery,return_typed_recovery,extension_trait_method,
  cross_module_no_collision}/{main.rs,expected.toml}` (positive golden set; `exact=true`). `cross_module_no_
  collision`: two `Foo::m`, assert the caller set is exactly the correct module's. `cd eval && uv run tier-a
  --matrix-only --allow-stale-sut` → new fixtures `ok`; **no existing fixture regresses** (trait fixtures stay
  `ok` as NameOnly — caller-set preserved). Commit: `test(rust-recv): capability fixtures`.

### Task 3.5: The Tier-A 2a gate (measurement) → PR-3

- [ ] `cargo fmt && cargo test` green. `cargo build --release`; `cd eval && uv run tier-a --quick
  --allow-stale-sut` (rust-analyzer) + `uv run tier-a --corpus prism`. Record: receiver-method `prism_fp`
  that drop (drop-on-miss), cross-module FPs that drop (identity), field/return recall gains; confirm
  **recall held-or-up** + **no fixture regression**. Adjudicate flips (dual-adjudicator) → `eval/
  adjudications.jsonl` if needed; paste deltas in the PR (re-anchor `baseline.md` only with owner approval).
  PR-3 body: the 2a gate table (precision Δ, recall Δ, FPs killed, no regression). Note Phase 2b (residue
  removal + call-stats telemetry) and Phase 3 (raise trait/wrapper→Exact, applicability, trait-in-scope,
  wrapper/Deref dispatch) as gated follow-ons.

---

## Final review (after all 3 PRs)
- [ ] Dispatch a final codex xhigh code-reviewer over the whole branch — confirm spec §7 invariants hold:
  identity-or-fall-through-to-bare (the identity-complete guard; no removed bare edge), kind-aware confidence,
  External-via-lookup (extension traits preserved), residue only on unrecovered, drop-on-miss measured,
  rebuild-together determinism, Go untouched.
- [ ] Use superpowers:finishing-a-development-branch.

## Self-review (plan vs spec + plan-review folds — completed)
- **Spec coverage:** §1 build-time materialization → the post-pass (Task 2.4); §2.1 recovery → the
  `RustReceiverTyper` (2.3); §2.2 kind-aware combine + has_self + arity → 1.1/3.1; §2.3 stance → 3.2/3.5; §3.1
  resolve_type_path + in-repo-first + identity-or-bare → 1.2/3.2; §3.2 indices (Self/alias/cfg/AST facts,
  scope-aware) → 1.1/1.3/1.4; §3.2b def-byte lookup → 2.2; §8 cache + rebuild-together → 1.5/2.1/3.3; §9
  phasing → PR split; §5 Go untouched.
- **Plan-review R1 folds:** #1/#2/#7 post-pass + callers + cache bumps (2.1/2.4/3.3); #3 identity-complete
  guard (1.3/3.2); #4 in-repo-first (1.2); #5 type_syntax preservation (2.3); #6 alias scoped (1.4); #8
  cmp_key exclusion (2.1); #9 arity via MethodFacts (3.1); #10 concrete test helpers (throughout); telemetry
  deferred to Phase 2b.
- **Plan-review R2 folds:** BLOCKER #1 PR-2 read-inertness — all recovery in a NEW `RustReceiverTyper`/post-pass
  writing ONLY `receiver_outcome`; inline classifier + `receiver_type` untouched (PR-2 header + 2.3/2.4);
  MAJOR #2 trait-body methods classified `Trait` not `Inherent` (1.1); MAJOR #3 scope-aware extraction, not
  `rust_provider` bare maps (1.4); MINOR #4 in-place `receiver_outcome` update (order-preserving), not
  `callers` rebuild (2.4/3.3); MINOR #5 all `CallSite` literals incl. test files + exhaustive `ReceiverRecovery`
  match enumerated (2.1/2.3).
- **Plan-review R3 folds:** BLOCKER #1 materialized shape — the new field is a self-contained
  `ReceiverOutcome { key, bare, recovery }` (NOT a bare key), so PR-3's `combine_kind` reads `oc.recovery`
  (StdWrapperPeel demotion) and the identity-complete guard reads `oc.bare` — without touching the legacy
  `receiver_recovery` (inert preserved) (1.2/2.1/2.4/3.2); MINOR #2 the trait-facts test uses a default-body
  trait method (signature-only items aren't FunctionId targets — not added to collection) (1.1).
- **Type consistency:** `MethodFacts`/`MethodKind`/`RecvMode` (1.1), `TypeKey`/`ReceiverTypeKey` (1.2),
  `identity_complete` (1.3), `RustReceiverTyper`+`type_syntax` (2.3), `combine_kind` (3.1) used consistently.
- **Non-regression:** PR-1/PR-2 inert (matrix parity, 1.5/2.4; `receiver_type` + inline classifier untouched);
  PR-3 measured (3.5); trait fixtures preserved
  as NameOnly; G2 guard prevents dropping correct bare edges. No gaps identified.
