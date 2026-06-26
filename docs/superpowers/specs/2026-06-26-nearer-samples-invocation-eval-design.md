# "Nearer Samples" — Prism Invocation De-Risk Eval (Design)

> Status: **DRAFT for owner review** (brainstormed 2026-06-26). Extends the adoption eval
> (`eval/adoption/`, spec `2026-06-25-prism-adoption-eval-design.md`). Pairs with memory
> `project_prism_measurement_maturity.md`.

## Why

The adoption eval proved the *mechanism* — the tuned `prism-code-navigation` skill drives
**90% nav-invocation pass^5 on all three models** (sonnet/opus/gpt-5.5). But that 90% is a
**ceiling measured under two artificial conditions**:
1. **Isolated env** — only the prism skill present, so no competition for the model's attention.
2. **Micro-probes** — pure single-question nav tasks ("who calls X"), which trivially trigger
   navigation.

Realistic deployment (and the Tier-C "prism never invoked" failure) has the opposite: **many
skills present** + **broad spec/plan/analysis tasks** where navigation is one sub-step the model
may never reach for. So the 90% likely overstates realistic invocation. Jumping to **part C**
(the expensive full Tier-C re-run that measures *value*) risks discovering invocation collapses
there. This eval bridges that gap with intermediate "nearer samples" — realistic prompts +
realistic skill competition — to de-risk **invocation** before part C.

## Scope

A **2×2 factorial** over **prompt-realism × skill-competition**, measuring **prism invocation**
(not value — value is part C). Sonnet runs the full 2×2 (the diagnostic); the headline Cell 4 is
then confirmed on cheaper-then-pricier models.

**Out of scope:** value measurement (does prism *improve* the spec/plan) — that is part C; the
full Tier-C arms/chain; non-prism-skill quality.

## Success criteria / how it reads

Primary metric per cell: **invocation rate** = fraction of (sample × trial) runs that fired any
`mcp__prism__nav_*`. Secondary: **`pass^5`** (per sample, all 5 trials invoked) and a
**which-skill-loaded** split (tuned `prism-code-navigation` vs `prism-nav` vs none).

- **Cell 1 = ceiling** (the existing 90% reference, reused from cache).
- **Cell 4 ≈ Cell 1** ⇒ prism survives realism ⇒ **green-light part C**.
- **Cell 4 collapses** ⇒ Cells 2 vs 3 localize the cause: **competition** (fix = skill
  discoverability/ranking among many skills) or **realistic prompts** (fix = the skill must fire
  mid-task, not only on pure-nav questions).

There is no fixed pass bar; this is diagnostic. A rough read: Cell 4 within ~15pts of Cell 1 = ok
to proceed; a large drop = fix the localized cause first.

## The 2×2

| Cell | Prompt | Env | Samples | Question |
|---|---|---|---|---|
| **1** | micro | isolated (only tuned skill) | 12 nav-probes | reference / ceiling (reuse cache) |
| **2** | micro | competing (real skills, no memory hint) | 12 nav-probes | does competition alone drop it? |
| **3** | realistic | isolated (only tuned skill) | 5 spec/plan/analysis | does prompt-realism alone drop it? |
| **4** | realistic | competing (real skills, no memory hint) | 5 spec/plan/analysis | **the de-risk** (closest to part C) |

Common cross-cell metric = "any `mcp__prism__nav_*` fired" (so all four cells are comparable). Micro
cells (1,2) additionally report tool-correctness vs `expected_tools`; realistic cells (3,4) cannot
(no single expected tool) and use the any-call metric only. **Note:** Cell 1's *reference for the
2×2* is its **any-call rate recomputed from the cached trajectories** — which is ≥ the adoption
eval's 90% (that 90% was the stricter tool-correctness pass^5; any-call counts a prism call even if
it isn't the exact expected tool). All four cells are compared on the any-call metric.

## Realistic prompts (5, on `tier_c`, nav-genuine)

Stored in `eval/adoption/goldens/realistic_prompts.toml`. Each is a spec/plan/analysis task where
navigation is the natural path, so non-use is a real miss (mirrors Tier-C issue shape):

1. `spec-runstage-tiebreak` — "Write a short implementation spec for changing `run_stage`'s
   tie-break to use a different seed." (needs callers / the tiebreak path)
2. `analysis-count-claims-blast` — "Analyze the blast radius of changing `count_claims`'s
   signature — what updates?" (needs callers)
3. `plan-split-chain` — "Plan the refactor that splits `chain.py` into stage-orchestration vs
   chaining modules." (needs module structure + callees)
4. `spec-sanitation-gate` — "Spec a fix for the sanitation gate in `run_spec_plan_chain`."
   (needs the call path / callees)
5. `analysis-dry-run-flag` — "Which functions would a new `--dry-run` flag for the tier-c CLI
   touch?" (needs callees/callers from `cli`)

Schema: `id`, `kind="realistic"`, `prompt`, `repo="tier_c"` (no `expected_tools`).

## Environments

- **isolated** (Cells 1, 3): the EXISTING recipe — `env.py` (claude `CLAUDE_CONFIG_DIR`) /
  `codex_env.py` (`CODEX_HOME`) with ONLY the tuned skill + prism MCP, no other skills/hooks. Reuse.
- **competing** (Cells 2, 4): NEW — the realistic skill set **minus the memory injection** (the
  load-bearing piece; verify FIRST, like the prior isolation gates):
  - **claude** (`competing_env.py`): build a `CLAUDE_CONFIG_DIR` that carries the *real* skills +
    plugins (superpowers, `prism-nav`, `lsp-nav`, user skills) by copying/symlinking from `~/.claude`,
    **plus** the tuned `prism-code-navigation`, **plus** seeded creds + prism `--mcp-config` — but
    with the **SessionStart hook stripped from `settings.json`** (no memory injection). Verify: the
    real skills are discoverable AND no SessionStart hook fires (no prism-naming text injected).
  - **codex** (`codex_competing_env.py`): a `CODEX_HOME` carrying the real `~/.codex` config +
    skills (which auto-loads `knowledge-ref`'s `prism-nav`/`lsp-nav`) + prism MCP in config.toml —
    but with the prism-hinting project/global instruction (the `AGENTS.md`/memory analog) **excluded**.
    Verify: real skills present AND no instruction directly names the prism nav tools.
  - **VERIFICATION GATE (Task 1 of the plan):** one forced probe per CLI confirming "real competing
    skills present + zero prism memory-hint." If the no-hint condition can't be cleanly met, STOP and
    adjust before running Cells 2/4.

## Models & cost (cheap → expensive)

- **Sonnet 4.6** — full 2×2 (Cells 1–4). Cell 1 reuses cache; ~110 new fast runs. The diagnostic.
- **codex-spark (gpt-5.3-spark)** — **Cell 4 only** (5×5=25). Cheap codex-family check of the headline.
- **then Opus 4.8 + codex gpt-5.5** — **Cell 4 only** (~50). Expensive deployment-model confirmation,
  run *after* the cheap tier; **skip if Cell 4 already collapsed cheaply**.

Cell 4 gets 4-model coverage; Cells 1–3 are Sonnet-only. Codex is slow (≤3 workers; spec/plan
prompts longer than micro-probes). All runs cache-keyed (skill bytes + env-id + prompt + trial +
model) so re-scoring is free.

## Architecture / what to build vs reuse

**Reuse:** `runner.py` (claude+codex paths, cache), `trajectory.py` (both parsers — already capture
`skill_loads` + prism `tool_calls`), `aggregate.py` (`pass^k`), the isolated env builders, deepeval
metric scaffolding.

**New:**
- `competing_env.py` + `codex_competing_env.py` — the competing-env builders (load-bearing).
- `goldens/realistic_prompts.toml` + a loader (or extend `goldens.py`).
- An **invocation metric**: `prism_invoked(traj) = bool(traj.prism_nav_calls())` (any-call,
  deterministic); per-cell `invocation_rate` + `pass^5` + `skill_loaded` attribution in `aggregate.py`.
- A **2×2 driver** (`twobytwo.py` or a script): runs each (cell, model) over its samples × 5 trials
  via the runner, writes `results/twobytwo-<id>.json` with the 4-cell table + per-cell breakdown.
- The runner's env selection must accept an **env builder** per cell (isolated vs competing) — a
  small param so `run_trial` (or a thin wrapper) uses the right `CLAUDE_CONFIG_DIR`/`CODEX_HOME`.

## Testing

- Unit-test the competing-env builders' file layout (real skills present, SessionStart hook absent,
  creds 0600, prism MCP configured) on a fake config tree.
- Unit-test `prism_invoked` + the 2×2 aggregation (cell table, invocation_rate, which-skill-loaded)
  on synthetic trajectories.
- Reuse the committed stream-json / codex fixtures for parser coverage (unchanged).
- One live verification per CLI (Task 1 gate) before any cell run; one cheap Sonnet Cell-3 smoke
  before the full 2×2.

## Risks

1. **Competing-env "no memory injection" recipe** (load-bearing) — stripping exactly the SessionStart
   memory hint (claude) and the prism-naming instruction (codex) while keeping the real skills.
   Verify Task 1; if not cleanly separable, document what leaks and interpret accordingly.
2. **Open-ended-task metric noise** — realistic spec/plan trajectories vary more than micro-probes;
   `k=5` + invocation_rate (not strict pass^5) is the primary read to absorb noise.
3. **Codex slowness/cost** on Cell 4 — mitigated by cheap-tier-first (spark) + skip-on-collapse +
   ≤3 workers + cache.
4. **Attribution** — with `prism-nav` present in competing cells, `which-skill-loaded` (from the
   trajectory's skill-read) disambiguates whether the tuned skill or `prism-nav` drove invocation.
