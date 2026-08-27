# Go receiver type-origin binding — Slice 0 implementation plan

> **For agentic workers:** implement this plan task-by-task with red-first tests. Do not combine Slice 0 with owner-carrying Slices 1–2 or the blanket no-owner terminal predicate in Slice 3.

**Status:** plan v5, review `APPROVE`; Task-1 compiled-reality correction folded. **Base:** `main` / merged design v3 at `4e60dfc52acd6d370b59feeca30f45d788dab02e`. **Branch:** `a-receiver-provenance-slice0-plan`.

**Goal:** Install the four prerequisites required before receiver owners may be populated: exact-import receiver-owner resolution, declaration-backed owner admission, Go type-parameter and dot-import evidence, and terminal local-type-declaration poisoning. The slice must eliminate the constructible wrong outputs in §2 while preserving ordinary declared local receivers and all existing value-rebinding behavior.

**Design-of-record:** `docs/superpowers/specs/2026-08-24-go-receiver-type-origin-binding-design.md`, Slice 0 and gates.

**Architecture:** Keep the public/legacy `resolve_go_owner_identity` contract unchanged. Add a receiver-only exact-import mode and a single post-merge prerequisite screen over every recovered Go receiver. A definitive barrier converts that classification to `recovered: None, materialized: true`; the existing resolver then drops at `resolve_call_site_full`'s materialized-receiver guard, the manifest omits the unrecovered site, and the navigation sidecar cannot contain an edge. This is deliberately narrower than Slice 3: an otherwise admissible declared local receiver may still proceed without a carried owner until Slice 2 supplies it.

**Review cap:** two rounds. At the cap, round 2 found one smaller, non-repeating, closed-enumerable bypass after round 1's three `WRONG` findings. The loop was therefore classified converging; the bounded correction was folded and one confirmation pass was disclosed. Any new or repeated `WRONG` in that pass parks the plan for owner/design adjudication.

**Round-1 correction record:** three closed-enumerable `WRONG` findings and one `SMELL` were folded: the S1 factory-callee reference now uses exact import identity; carried S1/S2 owners are validated before caller-lexical poison is considered; caller-local and cross-file-unowned facts now have a transient origin-scope distinction; and RED output is retained as handoff evidence without creating an intentionally red commit.

**Round-2 correction record:** one closed-enumerable `WRONG` was folded: an emitted prerequisite drop is now pushed unconditionally and cannot be swallowed by the pre-existing same-scope-reuse “skip update” exception.

**Confirmation record:** no new behavioral `WRONG`. The remaining instance of round 1's closed `SMELL` was enumerated and removed: Tasks 1–4 are one RED-to-GREEN custody unit, so neither failing tests nor new cache-relevant fields/topology are committed at an intermediate checkpoint.

**Task-1 compiled-reality correction:** the local-type fixture refuted the planned wrong-target claim: base already returns zero resolver targets but retains `receiver_type` and admits the exact call site to the manifest. The row and RED contract now name that observed incorrect public artifact; no mechanism or production scope changed.

---

## 1. Measured source boundary

The following facts were re-read at base `4e60dfc`; symbol names are authoritative and line numbers are advisory.

| Fact | Current anchor | Planning consequence |
|---|---|---|
| Legacy Go owner resolution synthesizes every bare `T` as the caller package and qualified lookup falls back from an exact import-path key to a basename. | `src/resolution.rs:resolve_go_owner_identity` | Do not flip the public helper globally; introduce a receiver-only exact mode. |
| `resolve_go_owner_identity` has nine production callers in seven files. | prism `nav_callers` census | Strict mode is admitted only at the receiver-proof call sites named in §4. |
| `go_concrete_receiver_route` checks `go_declaration_kind_index` only after `go_receiver_owner` has synthesized/rebound an owner; `Unproven` may still enter R3's bare interface ladder. | `src/go_concrete_receiver.rs:go_concrete_receiver_route`; `src/resolution.rs:resolve_call_site_full`; `src/navigation/queries.rs:interface_dispatch_manifest` | A definitive prerequisite failure must clear recovery before the R3 ladder, not merely return `Unproven`. |
| `receiver_materialized == true` with no recovered type already drops in the resolver. The manifest enumerates only sites with both `receiver_type` and `receiver_recovery`. | `src/resolution.rs:resolve_call_site_full`; `src/navigation/queries.rs:interface_dispatch_manifest` | Reuse this existing terminal membrane; do not add a second resolver/manifest predicate in Slice 0. |
| `go_local_type_shadows` is consulted from full extraction, direct-subset extraction, and post-merge rematerialization, but its result is OR-ed with value-rebinding `proof_shadowed` into one `CallSite` boolean. | `src/ast.rs:go_local_type_shadows`; prism caller census | The prerequisite screen must call the AST helper directly. It must not terminalize the combined `receiver_local_type_shadowed` field. |
| Dot imports are parsed but deliberately discarded from the normal import map. | `src/ast.rs:extract_go_import_spec` | Record a separate marker; never insert `"."` into `CallGraph.imports`. |
| The Rust type-parameter helper matches Rust grammar kinds and is unreachable for Go. | `src/resolution_receiver.rs:enclosing_generic_type_params` | Add a Go-specific AST helper; do not reuse the Rust collector. |
| Whole-program Go declaration kinds are built before receiver rematerialization in full and incremental builds. | `src/call_graph.rs:apply_go_interface_dispatch_with_scope_inputs`; `src/cpg/build.rs:build_incremental_with_scope_graph_inputs` | The screen belongs in the post-merge receiver update pass, where the complete declaration index is available. |
| `build_direct_subset` intentionally leaves whole-program Go facts empty; incremental construction recomputes them after merge. | `src/call_graph.rs:build_direct_subset_with_receiver_config`; `src/cpg/build.rs` | Direct-subset output alone is not an authority artifact. The four-path gate tests the merged/cached public paths. |

### Explicit non-goals

- No owner population for package variables or caller-local bindings; those are Slices 1 and 2.
- No blanket `receiver_owner_identity.is_none() => drop`; that is Slice 3.
- No global signature or semantic change to `resolve_go_owner_identity`.
- No general Go type inference, constraint solving, dot-import target resolution, or local-type environment.
- No #16 interface-walk changes and no reactivation of PR #203.
- No re-baselining of Tier-A or corpus oracle results.

---

## 2. Required behavior matrix

Each negative names a constructible wrong result on the base tree. Every row must be RED before implementation and GREEN after it.

| Case | Base-tree incorrect result | Slice 0 result |
|---|---|---|
| Predeclared receiver | `func F(e error) { e.Error() }` plus unrelated source-declared `q.error`/implementer can reach the bare `("error", "Error")` interface table and mint `q`'s target. | Receiver stays materialized but unrecovered; zero resolved targets, no manifest dispatch site, no sidecar edge. |
| Same-basename external import | `import api "outside.example/api"; func F(v api.I){ v.M() }` plus a unique in-repo directory `api` with a different effective import path can be selected by basename and mint its implementer. | Exact import-path key is absent, so receiver proof fails terminally; the in-repo decoy never mints. |
| Same-basename external factory | `r := api.New(); r.M()` with `api` imported from `outside.example/api` plus a unique in-repo decoy `api.New` can select the decoy S1 return fact before the receiver owner is screened. | The callee reference itself requires an exact import-path key; no decoy return fact or target is admitted. |
| Dot-imported bare name | `import . "outside.example/api"; func F(v I){ v.M() }` plus an unrelated source `I` interface elsewhere can fall through by bare name. | The file's dot-import marker plus absent local/package declaration makes the recovered type inadmissible and terminal. |
| Go function type parameter | `func F[T any](v T){ v.M() }` plus a package-level source type `T` can bind `T` to the package declaration and mint its method. | The lexical type-parameter binder wins; terminal drop. |
| Generic method receiver parameter | `type Store[T any] struct{}; func (s *Store[T]) F(v T){ v.M() }` plus a package-level decoy `T` can be rebound. | The receiver-spec binder is recognized; terminal drop. |
| Local type declaration | A package-level `Iterator` plus `func F(v Iterator){ type Iterator interface{ Next() bool }; v.Next() }` already returns zero resolver targets, but retains `receiver_type = Some("Iterator")` and admits the exact call site to the interface manifest rather than applying the terminal prerequisite membrane. | Local type-declaration evidence clears recovery before owner/R3 routing; the exact site is absent from the manifest and sidecar. |

Required positive/edge controls:

1. `type Local struct{}; func (Local) M(){}; import . "outside.example/api"; func F(v Local){ v.M() }` keeps the exact local method edge.
2. A non-generic `func F(v T)` with a visible package declaration `type T struct{}` keeps its exact local edge.
3. A later value rebinding with no local type declaration retains the existing R2 collision/direct-reuse behavior; Slice 0 must not reinterpret `proof_shadowed` as a type barrier.
4. A qualified receiver whose import path has one exact effective-module directory and one visible declaration keeps its edge.
5. A missing/unparsed build profile is not declaration proof and drops.
6. Package variables whose raw type text came from another file remain a named Slice 1 gap. Add an ignored sentinel for the cross-file alias counterexample; Slice 0 must not claim it fixed or validate that text in the caller's namespace.

---

## 3. File boundary

Expected production files:

- `src/resolution.rs` — add the private exact-import receiver resolution mode/helper while preserving the public legacy wrapper.
- `src/ast.rs` — add dot-import detection and Go type-parameter binder detection.
- `src/go_receiver_index_visibility.rs` — make S1/S2 receiver-fact owner resolution use exact mode and declaration visibility.
- `src/go_receiver_index.rs` — extend `GoReceiverFacts`; add the one prerequisite screen/finalizer and return a bounded drop reason.
- `src/call_graph.rs` — persist/rebuild dot-import markers and prerequisite-drop counts; run the screen on every post-merge receiver update.
- `src/navigation/queries.rs` — expose prerequisite-drop counts in `call_stats`; do not add a second resolution path.
- `src/cpg_cache.rs` — topology/schema release bump `50 -> 51` and update its pin/history.
- `src/navigation/call_edge_cache.rs` — topology release bump `18 -> 19` and update its pin/history.

Expected test files:

- Add `tests/lang/go/receiver_origin_prereq_test.rs`.
- Modify `tests/lang/go/main.rs` to register it.
- Modify `tests/navigation/go_concrete_cache_test.rs` for no-cache/cold/exact-CPG/sidecar parity.
- Modify existing owner-resolution unit tests only for the new private exact helper; do not rewrite legacy expectations.
- Add one Tier-A fixture only if the existing Go matrix cannot express the seven-site negative/positive matrix without conflating lines. Prefer a Rust integration fixture first.

Files explicitly excluded from Slice 0:

- `src/go_receiver_index_visibility.rs:unique_visible_type` return shape and S3 owner carrying — Slice 1.
- `CallSite::cmp_key` / equality — Slice 1 competing-owner work.
- Blanket resolver/manifest predicate for absent carried provenance — Slice 3.
- #16 design/spec branches and `interface_impls` population.

---

## 4. Shared contracts

### 4.1 Receiver-only resolution mode

Keep this public contract byte-identical:

```rust
pub fn resolve_go_owner_identity(
    type_text: &str,
    file: &str,
    imports: &BTreeMap<String, BTreeMap<String, String>>,
    package_basenames: &BTreeMap<String, BTreeSet<String>>,
    go_file_profiles: &BTreeMap<String, GoBuildProfile>,
) -> Option<GoOwnerIdentity>
```

Implement it as the legacy wrapper around a private mode-aware helper. Add a crate-visible receiver helper whose qualified branch consults only `package_basenames[go_import_path_dir_key(import_path)]`; it never falls back to the import path's last segment. Bare names may form a same-package candidate, but that candidate is not admitted until §4.2 proves a declaration.

Strict mode is used only at these receiver-proof seams:

1. `CallGraph::go_receiver_owner` for on-demand recovered Go receivers.
2. `resolve_go_return_type_call` for the S1 factory callee reference, using the call-site import namespace.
3. `unique_visible_return_owner` for the selected S1 return fact, using `fact.defining_file`.
4. `resolve_go_struct_field_owner` for an S2 field fact, using `declaration.defining_file`.
5. `classify_nested_selector` when its base recovery has no carried owner.
6. Same-scope-reuse owner comparison in `classify_go_receiver_expanded_with_partition`.

Do not change the legacy mode used by `go_func_field_key`, alias/defined-type target chasing, function-type target chasing, or registration/value-reference resolution. Those callers have separate contracts and need their own blast-radius review.

### 4.2 Declaration-backed admission

Extend `GoReceiverFacts` with:

```rust
pub declaration_kinds: &'a GoDeclarationKindIndex,
pub dot_import_files: &'a BTreeSet<String>,
```

Add one helper in `go_receiver_index.rs` with this semantic shape:

```rust
enum GoReceiverPrereqOrigin {
    CallerFile,
    CarriedOwner,
    CrossFileUncarried,
}

fn screen_go_receiver_prerequisites(
    ctx: &GoReceiverCtx<'_>,
    classification: ReceiverClassification,
    origin: GoReceiverPrereqOrigin,
    facts: &GoReceiverFacts<'_>,
) -> (ReceiverClassification, GoPartitionEvidence, Option<GoReceiverPrereqDrop>)
```

The origin enum is transient prerequisite metadata, not persisted provenance and not Slice 1 owner population. Raw caller-local recoveries are `CallerFile`; S1/S2 recoveries with an owner are `CarriedOwner`; the S3 package-variable branch is `CrossFileUncarried`. Set the tag at the classification branch that knows which fact supplied the text rather than inferring it later from `ReceiverRecovery::VarDecl`, which is shared by local and package variables.

Rules, in precedence order:

1. No recovered receiver: preserve the input classification and evidence.
2. `CrossFileUncarried`: preserve the classification as the explicit Slice 1 gap. Never interpret its raw type text, type-parameter name, local shadow, or imports in the caller's namespace.
3. `CarriedOwner`: validate the carried identity directly against the declaration index and the exact visibility already selected by its S1/S2 producer. If invalid, clear recovery and report `DeclarationUnproven`; if valid, preserve it. Caller-lexical type parameters and local declarations do not override an origin-proven owner.
4. `CallerFile` plus a matching Go type-parameter binder on the recovered bare static type: clear `recovered`, force `materialized = true`, reason `TypeParameter`.
5. `CallerFile` plus matching `ParsedFile::go_local_type_shadows`: clear/force materialized, reason `LocalTypeDeclaration`. Do not inspect `classification.proof_shadowed` here.
6. For the remaining `CallerFile` classification, resolve the candidate owner with receiver-strict mode in `ctx.caller_file` and require a `GoDeclarationKindIndex` entry with a single `declaring_file` whose `exact_declaration_visibility` is visible and exact for the caller/reference mode. Missing profile, missing declaration, ambiguous profile, and absent exact import-path identity are not proof.
7. For a bare caller-local type in `dot_import_files`, absence of declaration proof reports `DotImportBareUnproven`; otherwise report `DeclarationUnproven` or `StrictImportUnresolved` as applicable.
8. An admitted caller-local candidate preserves its current `owner_identity`; Slice 0 validates but does not populate a formerly absent owner.

Apply this screen once in `go_receiver_updates_for_caller` immediately after raw classification. Carry its optional drop reason in the receiver-update tuple. When a reason is present, push the materialized no-recovery update unconditionally; do not enter the same-scope-reuse direct-route exception. Only an admitted/no-drop classification may use that legacy exception. The full/incremental public paths both rematerialize every Go call site after whole-program declaration facts exist.

### 4.3 Go AST evidence

Add:

```rust
pub(crate) fn go_has_dot_import(&self) -> bool;
pub(crate) fn go_type_parameter_binds_receiver(
    &self,
    func_node: &Node<'_>,
    receiver_type: &str,
) -> bool;
```

`go_has_dot_import` recognizes `import . "path"` in both single and grouped declarations. It does not alter `extract_imports`; named imports remain the only entries in `CallGraph.imports`.

`go_type_parameter_binds_receiver` accepts only a peeled bare identifier and checks the current Go function/method declaration's own binders:

- direct `type_parameter_list` on a generic function;
- receiver-spec type arguments that declare generic method receiver names (`*Store[T]`);
- no descendant scan through nested functions or type declarations;
- qualified, instantiated, empty, or non-Go receiver text returns false.

Pin the tree-sitter-go shapes with AST tests before relying on field names. If the grammar shape differs from this list, update the helper design before implementing it; do not broaden to an arbitrary identifier-descendant scan.

### 4.4 Diagnostics and cache contract

Define a closed internal enum:

```rust
enum GoReceiverPrereqDrop {
    StrictImportUnresolved,
    DeclarationUnproven,
    DotImportBareUnproven,
    TypeParameter,
    LocalTypeDeclaration,
}
```

Aggregate one reason per screened site into a serialized `CallGraph.go_receiver_prereq_drops: BTreeMap<String, usize>`, rebuilt from zero during every full or incremental receiver rematerialization. The compute/update tuple carries the optional reason through the parallel collection; rematerialization counts it before applying the corresponding `CallSite` mutation. Emit it as `call_stats["go_receiver_prereq_drops"]`. The map is diagnostic only; resolution behavior comes from the screened `CallSite` state.

The CPG bump to 51 covers the new serialized whole-program facts/counters and changed CallSite materialization. The sidecar bump to 19 covers changed resolved edge topology. Update the pin tests and history comments before the final slice commit; never create a committed state in which the schema/topology change still shares the old version.

---

## Task 1: Add the complete RED behavior matrix

**Files:** add `tests/lang/go/receiver_origin_prereq_test.rs`; modify `tests/lang/go/main.rs`; extend `tests/navigation/go_concrete_cache_test.rs`.

- [ ] Add one build helper that accepts `.go` sources plus `go.mod`/`go.work` inputs through the real `repo_loader` path. Strict-import tests without an authority file are invalid probes.
- [ ] Add the seven negative cases and six controls from §2. Assert all three public observables: `resolve_call_site_full`, `interface_dispatch_manifest`, and navigation `callees`/sidecar output.
- [ ] For six target-mint negatives, assert the pre-change wrong target by exact file/owner, not merely non-empty fanout. For the local-type negative, assert the exact retained `CallSite` identity and manifest admission observed in compiled reality.
- [ ] Add the ignored Slice 1 cross-file alias sentinel, labeled with the exact future condition required to unignore it.
- [ ] Extend the four-path cache test fixture with one prerequisite drop and one positive retained edge.

Run before production edits:

```bash
cargo test --test lang_go receiver_origin_prereq -- --nocapture
cargo test --test navigation concrete_receiver_outputs_match_no_cache_cold_create_exact_cpg_and_sidecar_hits -- --nocapture
```

Expected RED: six target-mint negatives resolve to the named unrelated target; the local-type negative retains the named site in the manifest; the missing-profile edge control also mints its named target; positive controls remain green. A test that selects zero sites or fails fixture parsing is inadmissible and must be repaired before proceeding.

**Task boundary:** capture the RED output in the handoff, then keep the tests uncommitted through Tasks 2–4. Tests, production correction, cache fences, and final evidence form one slice commit so no committed branch point is intentionally red. Do not weaken assertions to make the base green.

---

## Task 2: Add exact receiver resolution and declaration admission

**Files:** `src/resolution.rs`, `src/go_receiver_index_visibility.rs`, `src/go_receiver_index.rs`, focused unit tests.

- [ ] Refactor `resolve_go_owner_identity` behind a private mode without changing legacy results; prove byte/struct equality over all existing owner-partition unit cases.
- [ ] Add the exact-import receiver helper and negative unit poles: exact key hit, exact key absent with unique basename present, two exact dirs, missing profile, and ordinary-vs-test clause ambiguity.
- [ ] Switch only the six receiver-proof seams in §4.1, including the S1 factory callee reference before return-fact lookup.
- [ ] Add declaration-backed selection using the already-built `go_declaration_kind_index` and exact declaration visibility.
- [ ] Ensure strict S1/S2 failures return uncertainty/conflict evidence that their existing callers convert into materialized no-recovery, rather than falling back to the extraction-time classification.

Run:

```bash
cargo test --test lang_go owner_partition -- --nocapture
cargo test --test lang_go receiver_origin_prereq -- --nocapture
cargo test --lib go_receiver_index -- --nocapture
```

Expected GREEN for strict/basename and predeclared cases; dot/type-parameter/local-type rows may remain RED until Task 3. All pre-existing legacy owner-resolution tests remain byte-identical.

Intermediate verification checkpoint — do not commit while the remaining Task 3 rows are RED:

```bash
cargo build --release
cd eval && uv run tier-a --matrix-only --allow-stale-sut
```

Record every regression/flip candidate; do not re-baseline.

---

## Task 3: Add dot/type-parameter/local-type evidence and the single screen

**Files:** `src/ast.rs`, `src/go_receiver_index.rs`, `src/call_graph.rs`, `src/navigation/queries.rs`, behavior tests.

- [ ] Land grammar-pinning tests for single/grouped dot imports, generic free-function binders, generic method receiver binders, nested-function exclusion, and qualified/bare controls.
- [ ] Implement the two AST helpers in §4.3.
- [ ] Rebuild `go_dot_import_files` from the complete files map at receiver-index application time; clear it and prerequisite-drop counts with the other whole-program receiver facts.
- [ ] Extend `GoReceiverFacts` and apply `screen_go_receiver_prerequisites` exactly once after raw post-merge classification.
- [ ] Tag the raw fact source with `GoReceiverPrereqOrigin` at its classification branch; prove a local `VarDecl` is `CallerFile` and an S3 package variable with the same `ReceiverRecovery::VarDecl` is `CrossFileUncarried`.
- [ ] Preserve value-rebinding behavior: `classification.proof_shadowed` alone is not a prerequisite drop. Keep the existing same-scope-reuse direct-route exception only for no-drop classifications; an actual prerequisite drop must always update the site.
- [ ] Aggregate one closed reason per dropped site and expose the map in `call_stats`.
- [ ] Confirm poisoned sites become `receiver_type == None`, retain `receiver_materialized == true`, produce `ExternalReceiver`, are absent from the manifest denominator, and create no nav edge.

Run:

```bash
cargo test --test lang_go receiver_origin_prereq -- --nocapture
cargo test --test lang_go concrete_receiver_fix3 -- --nocapture
cargo test --test lang_go concrete_receiver_fix4 -- --nocapture
cargo test --test navigation go_concrete_cache -- --nocapture
```

Expected: full §2 matrix GREEN, including the value-rebinding non-regression.

Intermediate GREEN checkpoint — do not commit cache-relevant fields/topology under the old versions; proceed directly to Task 4:

```bash
cargo build --release
cd eval && uv run tier-a --matrix-only --allow-stale-sut
```

---

## Task 4: Seal cache parity, census, and final verification

**Files:** `src/cpg_cache.rs`, `src/navigation/call_edge_cache.rs`, cache tests, PR/handoff evidence.

- [ ] Bump CPG 50→51 and sidecar 18→19 with accurate history comments and pin tests.
- [ ] Assert the exact serialized `go_dot_import_files` and prerequisite-drop count map survive cold/exact-CPG paths.
- [ ] Assert no-cache, cold-create, exact-CPG-hit, and sidecar-hit outputs are byte-equal for manifest and `call_stats`, and edge-equal for navigation queries.
- [ ] Cut a same-base five-corpus control at implementation base `4e60dfc` before measuring the candidate. Report per corpus: screened sites by reason, removed targets, total CallSites, and owner-partition counters. No aggregate-only conclusion.
- [ ] Recut oracle baselines at the same base and report `sound` / `recall_gap` / `over_approx` movement. The ignored Slice 1 sentinel is not a waiver for a new over-approx elsewhere.

Final required gates, in this order:

```bash
cargo fmt --check
cargo check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo build --release
cd eval && uv run tier-a --matrix-only --allow-stale-sut
cd eval && uv run tier-a --quick --allow-stale-sut
git diff --check
```

`--allow-stale-sut` is admissible only because the release build immediately precedes both Tier-A runs in the same worktree. Full multi-corpus Tier-A remains human-triggered and is not silently substituted.

**Custody boundary:** after every gate above is read and recorded, commit Tasks 1–4 together with the refreshed handoff. If any required gate is unavailable, carry the exact exclusion in the handoff before asking the owner whether the slice may be committed with that open condition.

Done means:

- every added behavior has a pre-change RED capture and a positive/negative control;
- full suite totals and exclusions are recorded;
- no Slice 1–3 code or #16 behavior landed;
- cache and sidecar pins match the serialized/topology change;
- the handoff names any corpus/oracle gate not run and carries it as OPEN.

---

## 5. Review checklist

Round reviewers must answer these mechanism questions, not merely restate the design:

1. Can any raw classification bypass the single post-merge screen on a public full, incremental, cache-hit, or sidecar path?
2. Does strict mode leak into any of the four excluded low-level caller families?
3. Can a missing declaration/profile/exact module key still become `Unproven` and re-enter R3 rather than becoming materialized no-recovery?
4. Does Go type-parameter detection cover both generic free functions and generic method receiver specs without scanning nested declarations?
5. Is dot-import evidence separate from named imports, and does the local `Local.M` control prove the poison is not file-wide?
6. Is local type-declaration poison independent of later value rebinding?
7. Are full and incremental runs using complete declaration/dot-import facts before screening retained callers?
8. Do CPG and sidecar versions change in the same commit as their schema/topology?
9. Does the plan leave the cross-file S3 alias counterexample explicitly open for Slice 1?
10. Can the same-scope-reuse exception or any unchanged-site optimization swallow a prerequisite drop or its diagnostic count?

Terminal review verdict: **`APPROVE`**. Round-1: three `WRONG`/`CLOSED-ENUMERABLE`, one `SMELL`/`CLOSED-ENUMERABLE`, all folded. Round-2: one `WRONG`/`CLOSED-ENUMERABLE`, folded. Confirmation: zero new behavioral `WRONG`; the fully enumerated residual of the prior commit-boundary `SMELL` was folded.
