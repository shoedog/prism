# Phase-IP receiver-expansion (PR-2) — cold-session HANDOFF

> **Read this, then write the implementation plan from the spec, then execute.** This is the
> orientation for a fresh session picking up PR-2. The design is **done + dual-reviewed + owner-approved**;
> your job is `writing-plans` → execute the slices.

## Where things stand (start here)

- **Spec (the contract):** `docs/superpowers/specs/2026-06-16-prism-phase-ip-receiver-expansion-design.md`
  **rev 2** — owner-approved 2026-06-16. Read §0–§14 fully; the design decisions are locked (§13).
- **Branch:** `phase-ip-receiver-expansion`, tip `368d198` (spec rev 2), stacked **directly on merged
  `main`** (`5cd1ac9`). PR-1 (#96) is **MERGED**. Do PR-2 work here. Don't push / open a PR until the
  owner asks (PR-1's rhythm).
- **Untracked eval artifacts** (`docs/eval/tier-a/2026-06-15-*`, `eval/snapshots/*`) are leftover run
  outputs — leave untracked, never commit (same as PR-1).
- **Review records:** `docs/archive/review-artifacts/prism-query-layer/phase-ip-pr2-spec-review-codex-2026-06-16.md` (codex xhigh);
  the claude-opus lens findings are folded into the spec's rev-2 header. Both verdicts: strategically
  sound, buildability gaps fixed.
- **Memory:** `~/.claude/projects/-Users-wesleyjinks-code-slicing/memory/` — `project_prism_phase_ip.md`
  (Phase-IP status), `feedback_workflow_preferences.md` (the "as written OR BETTER" bridge framing — use
  it).

## What PR-2 is (one paragraph)

Expand Go interface-dispatch **receiver type recovery** so more interface-method call-sites reach PR-1's
(unchanged) `interface_impls` dispatch engine. **Recover-and-route** (spec §2): recover the receiver's
static type *syntactically*, and let the existing `owner_lookup → interface_impls → drop` ladder route it —
a concrete type resolves at `owner_lookup` (recall win); an interface falls to `interface_impls` (dispatch);
a miss drops. New forms: **type-assertion** `x.(Module).M()` (all 57 caddy sites), **`var`-declared locals**
`var r Runner`; **interface-slice** is sketched only. All behind a swappable `ReceiverClassifier` seam
(`legacy ↔ expanded`).

## First action: write the plan

Use the **`superpowers:writing-plans`** skill to turn the spec into a task-by-task implementation plan at
`docs/superpowers/plans/2026-06-16-prism-phase-ip-receiver-expansion.md`. The spec's **§10 slices** are the
plan's backbone:

- **Slice A (MANDATORY FIRST — its own commit/PR):** the `ReceiverClassifier` seam + `legacy` impl + the
  **extraction-API change** (feed the receiver node into `recover_receiver`). **Pure refactor, byte-identical
  resolution.** Gate: a `legacy`-parity test reproducing PR-1's recovery on the existing P6-lite fixtures.
- **Slice B:** `TypeAssertion` form (grammar pinned in §3) + tests + `go/interface_dispatch_assert` fixture.
- **Slice C:** `VarDecl` form (§4) + tests + `go/interface_dispatch_var` fixture.
- **Slice D:** the in-scope manifest (byte-span keys + denominator predicate, §8a) + the gate **report**
  (FP rule, §8b). **Python harness — bridge can't verify it; run `uv run pytest` yourself.**
- **Slice E (human-gated):** caddy 57-site re-adjudication (dual-adjudicator κ) + 5-corpus rerun +
  re-baseline. Owner triggers the multi-corpus run.
- **Slice F:** `SliceElem` sketched only (reserved variant + `#[ignore]`d test).

## Execution approach (owner-approved mods over PR-1's rhythm)

Same subagent-driven rhythm as PR-1, with these performance mods (owner-approved 2026-06-16):

1. **Risk-route, don't bridge everything.** **Do Slice A IN-SESSION via TDD** — it's an
   integration-heavy refactor across `recover_receiver` (two call sites) + the extraction API + the parity
   gate, exactly the shape that burned the bridge's attempt-bound in PR-1 (Tasks 4/8/12). Hand the
   **mechanical forms (Slices B, C)** to the a2a-bridge. **Slice D (Python)**: bridge implements, you run
   `pytest`. Slice E is human-gated.
2. **a2a-bridge per task** (config `examples/a2a-bridge.slicing-implement-s2xhigh.toml`, codex gpt-5.5
   xhigh, `--base-ref phase-ip-receiver-expansion`): review each hand-off diff yourself, cherry-pick onto
   the branch (the PR-1 hand-off: `git fetch <clone> <branch> && git cherry-pick -n FETCH_HEAD && git commit
   -C FETCH_HEAD --reset-author`, then add the trailer). Containers up via
   `~/code/a2a-bridge/deploy/containers/compose.egress.yaml`; fall back to in-session TDD if down.
3. **Frame every bridge task "as written OR BETTER (named axis: cleaner/tighter/more-consistent/
   better-tested/more-sound) + no-new-scope guard"** — NOT "verbatim" (the PR-1 mid-stream fix that ended
   reviewer oscillation; see `feedback_workflow_preferences.md`).
4. **Bake the spec's file:line anchors into each task body** (`recover_receiver` `call_graph.rs:1384` + the
   two call sites `:373`/`:1023`; `type_assertion_expression` fields `operand`/`type`, precedent
   `taint.rs:5567`; byte-span `CallSite` `:27-31`/`:1347-1357`; the `ast.rs:313/3816` legacy scanners) — PR-1
   sped up once exact anchors were fed in.
5. **If the a2a-bridge claude reviewer leg is fixed** (owner was addressing the `{{reviewer_claude}}`
   defect — check first): switch the per-task review to the **dual codex+claude** variant instead of
   codex-only. If still broken, run the claude lens as an operator subagent on risky slices.
6. **TDD per task** (failing test → confirm fail → implement → confirm pass → commit); **one commit per
   slice**; commit messages end with `Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>`.

## Load-bearing gotchas (from the dual review — full detail in the spec)

1. **Recover-and-route: no `GoTypeProvider` at extraction.** Recovery is syntactic; the seam routes
   interface-vs-concrete. Do NOT reintroduce an interface-set predicate at extraction (that was the rev-1
   blocker — the provider is built *after* extraction at `call_graph.rs:846/899`).
2. **The seam is `recover_receiver` (`call_graph.rs:1384`), fed a NEW receiver-node field** — today it gets
   only the qualifier string and rejects type-assertions at its simple-ident gate. The call extractor
   (`function_calls_with_qualifier_and_spans_on_lines`) must surface the receiver expr node. `legacy` must
   preserve `recover_receiver`'s gate logic verbatim, not just the `ast.rs` scanners.
3. **Type-assertion grammar pinned:** `call_expression.function = selector_expression`, selector `operand =
   type_assertion_expression` (fields `operand`, `type`); recover `type` (strip `*`, bare `pkg.Module`,
   unwrap `(T)`); **exclude comma-ok** `v, ok := x.(T)`.
4. **Manifest = byte-span keys** (`file:start_byte:end_byte`), not `file:line`; the **denominator predicate**
   (§8a) is "recognized receiver shape AND method name ∈ some known interface (checked at manifest-build,
   post-extraction)".
5. **Gate is a REPORT, not gating** (§8b); `corrected_fp` is meaningful only after Slice-E re-adjudication.
6. **`ReceiverRecovery` additive variants** (`TypeAssertion`, `VarDecl`, reserved `SliceElem`); no `CallSite`
   wire change; `CACHE_VERSION` 9→10 optional (GIT_SHA covers the built case; dirty dev needs `--no-cache`).
7. **Config surface:** `ReceiverRecoveryMode{Legacy|Expanded}` + per-form booleans, default `Expanded`,
   `Legacy` is the granular fall-back.

## Acceptance (after the engine slices A–D)

`cargo fmt && cargo test && cargo test --features mcp && cargo build --release`; the tier-A matrix
(`cd eval && uv run tier-a --matrix-only --allow-stale-sut`) — the new `go/interface_dispatch_assert` /
`_var` fixtures should be `ok`, no other flips; `uv run tier-a --quick --allow-stale-sut` (baseline_invalid
is the expected stale-baseline signal — don't re-baseline outside Slice E). Then a whole-branch dual code
review (codex + claude) before the PR, same as PR-1. **Slice E (caddy re-baseline) is owner-gated.**

## Done = owner opens the PR

PR-2 will be its own PR stacked on merged `main`. Report results; the owner opens/merges (PR-1 norm).
