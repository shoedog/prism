# Phase-IP Go embedding — PLAN review — claude opus (exec + coverage) — 2026-06-15

Operator subagent, read-only. Plan: `docs/superpowers/plans/2026-06-15-prism-phase-ip-go-embedding.md`;
spec: `…/specs/2026-06-15-prism-phase-ip-go-embedding-design.md`. Verified every load-bearing claim
against real source. Codex companion: `phase-ip-embedding-plan-review-codex-2026-06-15.md`.

## BLOCKERs (won't compile / red tree as written)

1. **Task 4 Step 1 — `go_direct_method_wins_over_promoted` asserts the wrong line.** Asserts
   `r[0].target.start_line == 6`, but in the test source the direct `func (w Wrap) Ping()` is on
   **line 7** (1 `package`, 2 `Base struct{}`, 3 `Base.Ping`, 4 `Wrap struct {`, 5 `\tBase`, 6 `}`,
   7 `func (w Wrap) Ping()`). Test FAILS even with a correct impl. **Fix:** assert `== 7` (+ comment).

2. **Task 5 Step 1 — `cpg.call_graph()` is a FIELD, not a method.** `CodePropertyGraph` exposes
   `pub call_graph: CallGraph` (cpg/build.rs:50); every reader uses the field form `cpg.call_graph`
   (no parens). There is no `fn call_graph()`. Won't compile. **Fix:** `cpg.call_graph.promoted_aliases…`.

## MAJOR

3. **Spec §6/§7 "generic structs resolve via the `[…]` strip" is only half-implemented.** Promotion /
   alias / recovered-receiver keys use `normalize_go_struct_key` (strips `[…]`), but **direct-method
   owner keys** come from the untouched `method_metadata`→`owner_key` path (call_graph.rs:957-960), and
   `owner_key` splits on `<` not `[` (resolution.rs:75-84) → a generic struct `Wrap[T]`'s own method
   keys `("Wrap[T]","M")` while the recovered receiver normalizes to `"Wrap"` — they never meet. Net:
   (a) `has_direct` can miss a direct method on a generic struct (wrongly promote); (b) a generic
   struct's own method still won't resolve through the seam (pre-existing gap). Not a compile blocker /
   not exercised by tests, but contradicts the spec's explicit "generics are resolved, not gapped."
   **Fix (pick one):** Go-gate `method_metadata`'s owner key through `normalize_go_struct_key` too (one
   normalizer for all Go owner keys — makes the claim true), OR downgrade the spec to "generic-struct
   *embedding* promotes; a generic struct's *own* method dispatch via recovered receiver remains a gap."

4. **Plan ↔ spec `build_scoped` (§9) conflict.** Spec §9 says build_scoped "**Requires** threading the
   full `files` into the Go-promotion precompute," but the plan deliberately does NOT modify
   build_scoped (Self-Review §1: best-effort). Verified build_scoped (context.rs:135) runs promotion
   over the filtered subset only. The plan's best-effort stance is defensible (nav re-resolves on the
   full index → metric safe) and matches §9's own "best-effort in scoped mode" sentence — but it
   contradicts the "Requires threading full files" sentence in the same section. **Fix:** strike the
   "Requires threading full files" clause from §9 (best-effort is the chosen reading).

## MINOR
5. **Task 3 Step 1 prose vs code:** prose says "add to `#[cfg(test)] mod kind_tests`" but the code
   declares a separate `mod go_key_tests`, and resolution.rs has no pre-existing test module. Code is
   authoritative + compiles; only the prose is loose.
6. **Task 5 builds the DFG from v1 `files`** then passes `&files2` to `build_incremental` — compiles
   and the assertion holds (apply receives files2), but use `DataFlowGraph::build(&files2)` for clarity.

## Verified non-issues (do not re-litigate)
- **No missing `ResolutionKind` ripple** — the only exhaustive `match` is `as_str` (resolution.rs:35-54);
  reason/stat helpers call `.as_str()`. Task 2 complete.
- **All 4 `CallGraph { }` literals** enumerated: empty(76), build_skeleton(194), build(693),
  build_direct_subset(909); plan inits both new fields in each.
- **`apply_go_embedding_promotion` borrow-checks:** `std::mem::take` ends the mut borrow before the
  removal loop; `has_direct` holds two *immutable* borrows of disjoint `self` fields (`methods` +
  `method_owners`) — allowed; final `entry().push()` are sequential on different fields.
- **Seam edit (3e) matches resolution.rs:404-424** byte-for-byte (`Some(mut resolved)=>{…}`/`None=>…
  ExternalReceiver`, `recovered_kind`, `recv_ty`/`name`); `is_embed` immutable borrow of
  `promoted_aliases` coexists with `resolved`'s immutable borrow.
- **3d `recover_receiver` edit matches call_graph.rs:1260-1265**; `parsed.language` in scope.
- **P6-lite fires for the fixture** (`receiver_type_in_fn` handles Go typed params, ast.rs:351-355).
- **Task 1 helper compiles** (`FunctionId`/`BTreeSet` imported; `GoStruct.embedded` bare names;
  private-field access from same-module fn; internal `mod embedding_tests` sees the items via
  `use super::*`). **`build()` tail (call_graph.rs:693-703) is a plain `CallGraph { … }` literal with
  `files` in scope** — 3c bind-then-apply drops in cleanly. **FunctionId identity holds**
  (extract_method row+1 == node_line_range). **Task 6/7 infra exists** (`cpg_cache::tests`;
  `call_stats` keys off `as_str` → `kinds["embedded_promotion"]` automatic; json block 38-46 is the
  edit site). **`build_incremental` signature** matches the Task 5 call.

## Verdict
**fix before building** — both BLOCKERs are one-line test fixes (Task 4 `6`→`7`; Task 5
`cpg.call_graph()`→`cpg.call_graph`); two MAJORs are spec reconciliations (generic-struct owner key
§6/§7; build_scoped §9). The integration core (provider helper, owner-index aliasing, P6-lite seam
relabel, replace-not-merge, cache bump, telemetry) is sound and compiles as written.
