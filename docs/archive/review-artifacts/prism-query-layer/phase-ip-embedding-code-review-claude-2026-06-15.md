# Phase-IP Go embedding — CODE review — claude (correctness + architecture) — 2026-06-15

Operator subagent, max reasoning, read-only. Diff: `origin/main..phase-ip` code only (9 files).
Codex companion: `phase-ip-embedding-code-review-codex-2026-06-15.md`.

## Item-by-item (all verified against real code)

1. **`walk_embedding` field-shadow — CORRECT.** Records ALL fields incl. embedded-as-field
   (go.rs:300-305 pushes embedded into `fields`); path-local `path` insert/remove (go.rs:646-650) →
   diamonds surface both equal-depth paths + cycles bounded; depth-0 own methods excluded (`depth>=1`,
   go.rs:624); `field_depth` keeps MIN; shadow check `*fd <= pm.depth` (go.rs:589) is the Go selector
   rule. Value/pointer receiver irrelevant to target (same FunctionId).
2. **`apply_go_embedding_promotion` — CORRECT, compiles.** Idempotent removal strips only promoted
   fids, preserves direct; non-Go early return after cleanup; direct-wins via `method_owners==owner`;
   uniquely-shallowest-else-ambiguous; alias key `owner_key(struct_name)`; borrows disjoint (has_direct
   immutable ends before entry() mutable); the two `unwrap`s provably safe.
3. **`owner_lookup` relabel — CORRECT.** Direct-wins guarantees a promoted key has no direct method →
   relabeling all callees is safe; fires on every path through owner_lookup (self/qualifier/P6-lite/
   implicit-this); the `T::m` path bypasses owner_lookup but Go calls never carry `::`.
4. **`build_incremental` replace-not-merge — CORRECT.** Runs over full merged `files`; `remove_files`/
   `merge` deliberately leave `promoted_aliases` for step-1 cleanup; cross-file stale case tested.
5. **Confidence — CORRECT, no laundering.** EmbeddedPromotion → `exact()` → `Exact`; relabel changes
   only `kind`. Deterministic, legitimate in ExactOnly slices.
6. **Non-Go — CONFIRMED no-op.** Empty maps; early return; relabel no-ops; `#[serde(default)]` +
   CACHE_VERSION 7→8; all 4 CallGraph literals init the fields.
7. **Tests — non-vacuous**, assert the right behavior across all 10.
8. **Other — clean** (no dead code/missing arms/unguarded panics; clippy allow justified).

## Findings (all MINOR / conservative known-gaps; none block)

- **MINOR-1 generics (pre-existing):** `data.methods` keyed by raw receiver text; `Box[T]` methods
  won't promote. Conservative miss, consistent with existing generic limitation.
- **MINOR-2 scoped-mode:** `build_scoped` runs promotion over the filtered subset → cross-file
  embedding misses in scoped (diff-review) mode, resolves in full mode. Conservative, documented
  tradeoff.
- **MINOR-3 mixed Go+Rust same-name:** a Rust `Wrap::Ping` sets `has_direct` and skips the Go
  promotion for `("Wrap","Ping")`. Pathological cross-language clash; conservative (no wrong edge).

None produce an unsound edge, panic, compile failure, cache corruption, or non-Go regression.

## Verdict: **ship it.** Core logic correct + well-tested; only conservative known-gaps.
