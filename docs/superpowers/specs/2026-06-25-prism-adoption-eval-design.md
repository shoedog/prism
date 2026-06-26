# Prism Adoption Eval (Claude + Sonnet, v1) — Design

> Status: **DRAFT for owner review** (brainstormed 2026-06-25). Pairs with the Tier-C handoff
> (`docs/superpowers/handoffs/2026-06-24-tier-c-value-measurement-handoff.md`, §"RUN COMPLETE" Headline #1)
> and memory `project_prism_measurement_maturity.md`.

## Why

The Tier-C full run (`full-2026-06-24`) measured **nothing** about prism's value because **no arm ever
invoked prism** — 0 `mcp__…nav_*` calls across all 64 arms (0/195 claude Bash + 0/740 codex exec referenced
prism/nav). Root cause (diagnosed via reproduction): the stage prompts never mention prism, and prism's MCP
tools are deferred behind `ToolSearch`, so the model used its Read/Grep habit. **Forcing prism via a strong
prompt is the wrong fix** — it wastes spend and doesn't mirror real usage. If prism is valuable but never
naturally invoked, it is equivalent to not existing, and the work is futile.

The realistic deployment mechanism is the **prism-code-navigation skill** (`skills/prism-code-navigation/
SKILL.md`) — it gives the agent the judgment to reach for the `nav_*` tools. The right thing to do is
**deploy the skill realistically and EVALUATE** (a) that it loads/triggers and (b) that the MCP tools get
invoked correctly — then **iterate the skill description until that is reliable** (`pass^5`). Only then is a
Tier-C value measurement meaningful.

## Scope

**v1 (this spec):** a Claude + **Sonnet** adoption eval + the deepeval-driven iteration loop to reach the
reliability bar. Sonnet is the cheap/fast iteration vehicle; an Opus confirmation pass closes v1.

**Out of scope (follow-on specs):**
- Codex side (its MCP-in-`exec` showed *zero* MCP tools in the run — needs its own investigation/fix).
- Wiring the adoption gate into Tier-C (part "C": prism-on arms only count when skill+MCP engaged).
- The relevance-oracle fix (separate spec, already drafted).

## Success criteria

Two **distinct** reliability signals (your two asks: skill loads, and MCP gets invoked), each as `pass^k`
with k=5 (all 5 independent trials pass):

- **HEADLINE bar — MCP-invocation `pass^5 ≥ 80%`** of the nav goldens: a trial passes iff the correct
  `mcp__prism__nav_*` tool fired (matching `expected_tools`). This is the realistic-usage outcome that
  matters — "with the skill installed, does prism actually get invoked."
- **Mechanism signal — skill-activation `pass^5`** (reported alongside): did the prism-nav skill load. This
  diagnoses *why* invocation succeeds/fails. The two normally correlate; divergence is itself signal (skill
  loads but no tool fires → skill triggers but doesn't drive use; tool fires without skill → prism reached
  another way). v1 reports both; the headline gate is invocation.
- Negative goldens (prism *not* needed) pass iff the agent does **not** over-reach for prism.
- **Quality read** (deepeval `MCPUse`, LLM-judge): when invoked, args are sane + the task is answered.
  Reported, not gated, in v1.

## Architecture

deepeval is the **build-loop ground truth** (run evals → read failures/`reason` → change the smallest thing
→ re-run, ~5 rounds). The only "app" lever we edit is **`skills/prism-code-navigation/SKILL.md`** (and, if
needed, the prism-mcp tool descriptions). We never edit metrics/thresholds or delete goldens.

Because the agent-under-test is the **external `claude` CLI** (not a Python-framework agent), no deepeval
framework-integration and no `@observe` applies. We use the **no-tracing path**: drive the CLI, parse its
`--output-format stream-json` trajectory, and construct `LLMTestCase(input, actual_output, tools_called=[…])`.

### Components (`eval/adoption/`, mirroring `eval/tier_a` / `eval/tier_c` style; `uv`)

```
eval/adoption/
  goldens/probes.toml          # the micro-probes (schema below)
  config/                      # isolated CLAUDE_CONFIG_DIR template (prism skill + mcp config, NO hooks/other skills)
  trajectory.py                # parse claude stream-json -> Trajectory{skill_loads, tool_calls:[(name,input)]}
  runner.py                    # drive `claude -p --model sonnet` in the isolated env; k trials; returns Trajectories
  testcase.py                  # Trajectory + golden -> deepeval LLMTestCase (tools_called, expected_tools)
  metrics.py                   # SkillActivationMetric (custom) + ToolCorrectness + MCPUse (deepeval)
  tests/test_prism_adoption.py # `deepeval test run` target; pass^5 aggregation per golden + overall
  results/                     # gitignored: per-round benchmark.json (pass^5 %, per-probe, per-metric)
```

Add `deepeval` to `eval/pyproject.toml`. Run: `cd eval && uv run deepeval test run adoption/tests/test_prism_adoption.py`.

### Golden schema (`probes.toml`)

```toml
[[probe]]
id = "callers-count-claims"
kind = "nav"                       # nav | negative
prompt = "List every call site of `count_claims` as file:line. Answer in <=3 lines."
repo = "eval/tier_c"               # small Python target (fast prism index, low tokens)
expected_tools = ["nav_callers"]   # bare nav_* names; matched against mcp__prism__nav_* fired
expected_symbol = "count_claims"   # for arg-correctness (MCPUse)
```

### Representative probes (full ~12 land in `probes.toml` during the plan)

| id | kind | prompt (abbrev) | expected_tools |
|---|---|---|---|
| callers-count-claims | nav | "call sites of `count_claims`" | nav_callers |
| callees-run-stage | nav | "what does `run_stage` call" | nav_callees |
| impact-stage-prompt | nav | "what breaks if I change `stage_prompt`'s signature" | nav_callers / nav_ego_graph |
| nodes-at-chain-35 | nav | "what's defined at chain.py:35" | nav_nodes_at |
| repo-map-top3 | nav | "3 most-depended-on modules here" | nav_repo_map / nav_module_deps |
| module-deps-cli | nav | "what does cli.py depend on within this pkg" | nav_module_deps |
| ego-chain | nav | "local call graph around `run_spec_plan_chain`" | nav_ego_graph |
| compound-run35 | nav | "what's at run.py:35 and who calls it" | nav_nodes_at, nav_callers |
| neg-docstring | negative | "add a one-line docstring to `_salt` in chain.py" | (none) |
| neg-readme-typo | negative | "fix a typo in README.md" | (none) |

(~2 more nav probes to cover nav_callees/nav_callers breadth → 12 total.)

## Deployment (the clean env)

**Isolated `CLAUDE_CONFIG_DIR`** seeded with *only*: the prism-code-navigation skill (symlink/copy of the
repo's `skills/prism-code-navigation`), and **no** SessionStart hooks, **no** other skills (the run leaked
`superpowers:writing-plans` — a confound and not how a prism user is configured). prism MCP via
`--mcp-config <cfg> --strict-mcp-config` pointing prism-mcp `--repo <probe repo>` (proven to connect +
expose all 8 `mcp__prism__nav_*` tools in reproduction).

**LOAD-BEARING ASSUMPTION — RESOLVED 2026-06-25 (spike):** `claude -p` honors `CLAUDE_CONFIG_DIR` and
isolates skills (superpowers does NOT leak), but the isolated home needs two more seeds to be functional:
**(1)** a copy of `~/.claude/.credentials.json` (overriding `CLAUDE_CONFIG_DIR` loses auth → "Not logged in"
→ MCP never connects), and **(2)** a permission allow-list in `settings.json`
(`allow: [Read, Grep, Glob, Bash, mcp__prism]`, `deny: [Write, Edit]`) — without it the prism tool call is
**denied** by permissions. With both, a forced prism call succeeds (`is_error: False`, real graph) and
superpowers stays excluded. The allow-list is faithful (a prism user approves the tools) and the Write/Edit
deny keeps the eval from modifying the target repo. `.credentials.json` is a secret → temp dir only, never
committed. (Init `mcp_servers` may read `pending` pre-handshake; gate on an actual successful prism call.)

## Data flow (one trial)

1. `runner.py` launches `CLAUDE_CONFIG_DIR=<iso> claude -p --output-format stream-json --verbose
   --model sonnet --mcp-config <cfg> --strict-mcp-config "<probe prompt>"` with `cwd` = probe repo, a turn
   cap, and a terse-answer instruction (token control).
2. `trajectory.py` parses the stream-json: the `system/init` record (MCP connected? tools listed), `Skill`
   tool loads (did `prism-nav` load?), and every `tool_use` name+input (did `mcp__prism__nav_*` fire? args?).
3. `testcase.py` builds an `LLMTestCase` with `tools_called` = parsed prism calls, `expected_tools` = golden.
4. `metrics.py` scores: **SkillActivation** (prism-nav loaded), **ToolCorrectness** (right nav tool vs
   expected, deterministic), **MCPUse** (args/task quality, LLM-judge, cheap model).
5. Repeat ×5 → `pass^5` for that golden. Aggregate → `results/benchmark.json`.

## The iteration loop (deepeval as ground truth)

Per round (default 5): `deepeval test run …` → read per-probe `pass^5` + the failing metrics' `reason`
strings + the offending trajectories → identify the smallest change to **`SKILL.md`** (trigger wording,
when-to-use table, tool-name hints) that would fix the lowest-scoring probes → edit SKILL.md only → re-run →
confirm the failing probes improved without regressing others → summarize (what failed / changed / moved).
Stop at `pass^5 ≥ 80%` (Sonnet) or when improvement plateaus. Close v1 with one Opus confirmation run.

## Cost controls (pass^5 × ~12 × ~5 rounds is real)

- Sonnet (not Opus) for iteration; micro-probes (short prompt, ≤3-line answers, hard turn cap); one small
  target repo; warm the prism index once. MCPUse judge on a cheap model, and only on nav probes that fired a
  tool. SkillActivation + ToolCorrectness are **deterministic parses** (no LLM cost) — they carry the `pass^5`
  signal; the LLM-judge is quality-only.

## Testing

- `trajectory.py` parser: unit-tested against the captured repro stream-json fixtures (`/tmp/r_E.jsonl` etc.
  → committed under `eval/adoption/tests/fixtures/`): asserts skill-load detection + `mcp__prism__nav_*`
  extraction + arg capture + the negative case (no prism).
- `metrics.py`: SkillActivation/ToolCorrectness unit-tested on synthetic trajectories (pass + fail).
- `runner.py` subprocess seam is exercised only in a live round (like tier_c's arm_runner).
- One live smoke (1 probe × Sonnet) confirms the isolated env + MCP + parse end-to-end before the full loop.

## Risks

1. **`CLAUDE_CONFIG_DIR` isolation** (above) — load-bearing; verify Task 1.
2. **Deferred-tool friction** — even with the skill, Sonnet may not ToolSearch for prism. That is exactly the
   signal the eval exists to surface; the lever is the SKILL.md description (and possibly making prism MCP
   non-deferred). If no SKILL.md wording reaches `pass^5 ≥ 80%`, that's a real finding about prism's
   discoverability, not a thing to force.
3. **Sonnet↔Opus drift** — a skill tuned on Sonnet may behave differently on Opus; the Opus confirmation run
   guards this.
```
