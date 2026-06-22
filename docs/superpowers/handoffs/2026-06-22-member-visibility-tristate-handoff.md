# Handoff — Member-Visibility Tri-State Slice (glob-expansion §9 follow-on) — 2026-06-22

Resume doc for the prism owner-key-collision precision arc — the **next slice** after deferred-glob
member expansion (**#126, MERGED** to `main` `bad55a6`). Pairs with the authoritative arc log
`~/.claude/.../memory/project_prism_owner_key_collision.md`, the **codex architecture analysis** (§3,
READ IT FIRST), and the #126 spec/plan.

---

## 1. State (one screen)

- **#126 (deferred-glob member expansion) is MERGED to `main`** (`bad55a6`). `main` now expands
  `pub use mod::*` facades: bounded depth-2, a tri-state `glob_edge_visible` edge gate, fail-closed
  buckets surfaced as the `glob_expand` call-stats histogram, `CACHE_VERSION` 19. Acceptance on ruff
  was +1036 Exact / −955 unresolved / canary flat.
- **NEXT = the MEMBER-VISIBILITY TRI-STATE** (the §9 deferred lever from #126's spec). **GREENLIT by a
  spike** (§2). This fresh session should run the full brainstorm → spec → plan → implement pipeline.
- Soundness-sensitive: it is the **member-level mirror of #126's FINAL-BLOCKER** — the exact area the
  reviews caught three fall-through holes. Treat it with the full spec + codex-review discipline.

## 2. The lever (the spike finding)

#126's BLOCKER fix made `glob_lookup`'s member arm poison on a **claimed-but-visibility-filtered**
member: when a facade `pub use mod::*` lookup finds a rib for the name in the target but every
candidate fails `policy.visible` (`Unresolved` + `rib_present`), it does `record_ambiguous()` + Poison.
That is **recall-safe but CONSERVATIVE** — it blanket-poisons two cases:
- a **known-hidden** member (a `private` / `pub(crate)` / `pub(super)` item the glob soundly does NOT
  re-export — globs bring only `pub`) → the lookup could safely **CONTINUE** to a sibling glob / outer
  scope; and
- an **undecidable** `pub(in)`-no-restrict member (`vis_reaches` → `None`; `resolve_restrict` is a
  Phase-1 stub) → which **MUST** poison (can't prove not-visible).

**Spike (throwaway standalone counter splitting `ambiguous`, on a branch off #126, since reverted):**
on ruff, `glob_expand.ambiguous` = **14665** is **99.99% the claimed-filtered case (14663)** vs **2**
genuine-multi. So the conservative blanket-poison is the **dominant facade-resolution outcome** on
ruff — a target population that dwarfs #126's +1036 delivered.

The slice distinguishes them — a **member-visibility tri-state**, mirroring `glob_edge_visible` at the
member level: classify the claimed-filtered rib's bindings via `vis_reaches` → **all decidably-not-
visible (`Some(false)`) → CONTINUE** (sound, recall-positive); **any undecidable (`None`) → poison**.

**CAVEAT — verify-first (the arc's repeated over-estimation lesson: +428→+38, +1586→+0).** 14663 is
the **TARGET population, NOT the buy.** The recall gain is the subset where *continuing* reaches a
clean resolution; many of the 14663 may be `pub(crate)` cross-vantage names that resolve nowhere else
→ continue→unresolved, **no edge** (cf. #126's 2256 expansion-events → +1036 final edges). **Size the
real buy with a deeper instrumentation split inside the slice** (known-hidden vs undecidable, and —
harder — continue→resolves vs continue→unresolved) before claiming a number. The codex analysis (§3)
proposes how.

## 3. Design seed = the codex architecture analysis (READ FIRST)

A codex (gpt-5.5 xhigh) architecture analysis was commissioned on this exact lever and is committed at
**`docs/superpowers/specs/2026-06-22-member-visibility-tristate-analysis.md`**. It contains the
soundness walk (§2 of the analysis), the architecture options + a recommendation (§3), the telemetry
split (§4), tests/acceptance (§5), risks (§6), and the open questions for the formal spec (§7). **Use
it as the design seed** — your spec turns its recommendation into the design-of-record, after the
normal codex spec-review loop.

The architecture sketch it evaluates: a new `ResolutionPolicy::member_visible(binding, q, trav) ->`
tri-state (mirroring `glob_edge_visible`), consulted in the glob member arm; vs. extending
`scope_member_lookup_probed` to return a richer "rib outcome" instead of the `rib_present` bool; vs.
re-classifying in `glob_lookup`. The engine must stay language-neutral (it must NOT call `vis_reaches`
directly — that's Rust-policy-internal). Decide whether it touches only the glob member arm or also the
non-glob `resolve_bare` step-2 path.

## 4. Soundness (the core — DON'T hand-wave)

§7 cardinal invariant: **resolve-or-fall-through, NEVER a wrong target.** Continue-is-sound-when: a
private member is genuinely not re-exported by the glob → continuing past it is sound; an undecidable
member can't be proven not-visible → must poison. **The trap (the #126 FINAL-BLOCKER, re-examined at
the member level):** continuing past a filtered member must NOT let a *sibling* glob in the same scope
mint a wrong singleton. The spec + codex review must walk every `vis.kind` and prove the rule
recall-safe.

## 5. Process + standing constraints (carry forward verbatim)

- Pipeline: brainstorm → spec (codex xhigh review loop) → plan (codex xhigh review loop) → a2a-bridge
  codex `workspace-write` edit-only implement + **host commits** → host acceptance → final codex xhigh
  diff review → PR (owner-gated).
- A separate **Opus 4.8 subagent judges + folds** codex review findings (saves orchestrator context);
  on disagreement/uncertainty the judge escalates to the owner. BLOCKER/MAJOR → verify → fold →
  re-review; mechanical → fold without re-review. codex correctly HALTS mid-impl on a bad plan claim —
  trust that signal (fix the plan).
- Commit trailer: `Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>`.
- PR body ends: `🤖 Generated with [Claude Code](https://claude.com/claude-code)`.
- **No push / no PR until the owner explicitly asks.** Branch off `main` for source changes.
- **NEVER** stage `eval/` or `docs/eval/` run-artifacts (leave untracked).
- Acceptance: canary `multi_target_exact_sites` **byte-flat** on ruff + a 2nd Rust corpus (the
  soundness gate); the `kind_exact` buy; the new `glob_expand` bucket split (the sizing); Tier-A
  `--matrix-only` (0-regr) + ruff M2.

## 6. Environment gotchas (from #126, hard-won)

- **a2a-bridge:** `~/code/a2a-bridge/target/release/a2a-bridge run-workflow <review|implement>
  --config <abs.toml> --input <abs> --session-cwd /Users/wesleyjinks/code/slicing --out <out>`.
  `--input` does NOT reach codex (it reads `prompt_file` only) → **bake the task into the prompt**.
  Use a **fresh `[server] addr` port** each run (8189–8200 used this arc → start ≥8201). Wrap in
  `timeout 900`. Verify a real codex spawned (`pgrep -fl 'node.*codex-acp'`). A concurrent
  `cancel-tokens` codex workstream shares the box — don't disturb it; match yours by config/port.
- **Shared working tree:** ALWAYS `git add <explicit files>`, **never `-a`** — #126's rev-1 spec
  commit swept another workstream's staged test files → add/add conflict at rebase. Verify
  `git diff --cached --name-only` before every commit.
- **call-stats / Tier-A:** `./target/release/prism nav --no-cache call-stats --repo <ABS>` → JSON
  histogram to **stdout** (NO `timeout 300` — ruff is large, >5 min, the timeout kills it → empty
  output). `glob_expand` is a top-level object. **ruff M2** = `cd eval && uv run tier-a --corpus ruff
  --allow-stale-sut` → writes to `docs/eval/tier-a/<date>-ruff.{json,md}` **NOT stdout** (stdout is
  empty by design; read the dated file; `baseline_invalid=false` + `shortfall=0` = the no-harm gate).
  **`--corpus ruff --quick` runs PRISM not ruff** (harness bug: `--quick` forces names=[prism]).
  **prism corpus is ABSENT** from `~/code/bench-repos` → use **ripgrep** as the 2nd Rust corpus.
  Tier-A matrix: `cd eval && uv run tier-a --matrix-only --allow-stale-sut` (seconds).
- **`isolation: worktree` (Agent tool) branches off the DEFAULT branch (`main`), not the current
  feature branch** — for a spike that must build on a feature branch, use a manual branch off the
  right SHA.
- **macOS test targets:** `--lib`, `--test name_resolution`, `--test integration`, `--test ast` run
  fine; **run `--test ast`** before declaring green (it's a CPG consumer; a prior CI failed there).
  Bare `cargo test --test cli` stalls at `_dyld_start`. `cargo test` takes ONE name filter before `--`.

## 7. Key code seams

- `src/name_resolution/engine.rs`: `glob_lookup`'s deferred arm — **the member `match member_res.status`
  block is what you extend** (the `Unresolved if !member_rib_present => continue` vs
  `Unresolved => { record_ambiguous(); Poison }` arms). Also `scope_member_lookup_probed` (sets
  `rib_present` from explicit `(name,ns)` bindings before visibility), `resolve_rib` (returns
  `Unresolved` when all candidates fail `visible`), `CycleGuard`, `MAX_GLOB_DEPTH`.
- `src/name_resolution/rust_policy.rs`: `visible`, the tri-state `glob_edge_visible` (the pattern to
  mirror), and `vis_reaches(vis, def_scope, from) -> Option<bool>` (the `Some(true)`/`Some(false)`/
  `None` classifier the member tri-state reuses).
- `src/name_resolution/types.rs`: `ResolutionPolicy` trait, `GlobEdgeVis`, `Edge`/`Binding`/`Vis`.
- `src/navigation/queries.rs`: `call_stats` `glob_expand` histogram (where the new bucket split lands).
- #126 design-of-record: `docs/superpowers/{specs,plans}/2026-06-22-glob-export-member-expansion*`
  (spec §3.2 member arm, §3.4 visibility, §5 soundness, §9 deferred — this slice IS §9).

## 8. References

- Authoritative arc log: `~/.claude/projects/-Users-wesleyjinks-code-slicing/memory/project_prism_owner_key_collision.md` (+ `MEMORY.md`).
- **codex architecture analysis (the design seed):** `docs/superpowers/specs/2026-06-22-member-visibility-tristate-analysis.md`.
- #126 (merged): the deferred-glob member-expansion slice — the immediate parent.
