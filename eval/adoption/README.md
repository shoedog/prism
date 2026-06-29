# Prism Adoption Eval — Iteration Protocol

This package measures whether Claude spontaneously loads and uses the
`prism-code-navigation` skill when answering codebase-navigation questions.

## Run command

```bash
cd eval && ADOPT_ID=round-N uv run deepeval test run \
  adoption/tests/test_prism_adoption.py \
  --identifier prism-adoption-round-N -n 5 -i -s
```

Replace `N` with the round number (e.g. `round-1`, `round-2`).

## Where results land

Each run writes a benchmark JSON to:

```
adoption/results/benchmark-<ADOPT_ID>.json
```

Cached trajectories (one `.json` per probe × trial × skill-hash) live under:

```
adoption/results/cache/<24-hex-key>.json
```

The cache key includes the SKILL.md bytes, probe ID, prompt, repo, trial
index, and model. Editing `skills/prism-code-navigation/SKILL.md` changes the
hash, so all affected cache entries are automatically bypassed and new
trajectories are generated. Unedited probes retain their cached results, so
re-scoring after a skill edit costs only the invalidated trials.

## Iteration loop

1. Run the suite (command above). Read the printed summary line:

   ```
   nav invocation pass^5 = 42%  (activation 70%)  -> adoption/results/benchmark-round-N.json
   ```

2. Open failing probes: look up their IDs in
   `adoption/results/benchmark-round-N.json`, then read the corresponding
   cached trajectories under `adoption/results/cache/` to see what the model
   actually did (which tools it called, whether the skill loaded).

3. Edit `skills/prism-code-navigation/SKILL.md` ONLY. Do not edit metrics,
   thresholds, or golden probes — those are fixed for the duration of the
   benchmark.

4. Re-run. The cache auto-invalidates for the edited skill; all probes are
   re-generated. Confirm `nav_invocation_pass5_rate` rose without any
   previously-passing probes regressing.

5. Repeat until `nav_invocation_pass5_rate >= 0.80` on Sonnet.

6. Run a final confirmation pass on Opus (change `--model` in
   `run_trial` calls or set the model env var per the runner's signature).

## Guardrail

**Never edit** `adoption/metrics.py`, `adoption/goldens/probes.toml`,
`adoption/aggregate.py`, or any threshold constant during an iteration round.
The only lever is `skills/prism-code-navigation/SKILL.md`.

## Key metrics

| Metric | Meaning | Gate |
|---|---|---|
| `nav_invocation_pass5_rate` | Fraction of nav probes where the model invoked a prism nav tool in all 5 trials | >= 0.80 |
| `nav_activation_pass5_rate` | Fraction of nav probes where the skill loaded in all 5 trials | informational |

Negative probes (`kind="negative"`) are excluded from the rate calculation;
they verify that the model does NOT call nav tools for irrelevant prompts.

## 2x2 nearer-samples

The 2x2 eval (prompt-realism x skill-competition) measures whether prism
invocation survives realistic conditions. It is a prerequisite before part C
(end-task value measurement).

### Cells

| Cell | Probes | Env | Notes |
|---|---|---|---|
| 1 | micro (12 probes, `goldens/probes.toml`) | isolated | no competing skills; prism hint present |
| 2 | micro (12) | competing | real skills loaded, SessionStart hook stripped |
| 3 | realistic (5 probes, `goldens/realistic_prompts.toml`) | isolated | open-ended spec/plan/analysis tasks |
| 4 | realistic (5) | competing | hardest cell: realistic prompt + full skill competition |

Cells 1/3 use `build_isolated_config` / `build_isolated_codex_home`.
Cells 2/4 use `build_competing_config` / `build_competing_codex_home` (real
skills present, no memory-injection hook).

### Metric

Per cell, per sample, per trial (k=5): did the model fire **any** prism nav
tool call? (`prism_invoked(traj)` in `aggregate.py`). This any-call measure
suits open-ended tasks where there is no single required tool.

```
invocation_rate  -- fraction of sample*trial runs that invoked prism (all cells)
pass5_rate       -- fraction of samples where ALL 5 trials invoked prism
skill_attribution -- Counter of which skill name the model loaded (per invoked run)
```

### Driver call

```python
from adoption.twobytwo import run_2x2

# Sonnet full 2x2 (owner-triggered; ~25 claude calls)
summary, path = run_2x2(
    model="sonnet",
    cells=("1", "2", "3", "4"),
    eval_root="/path/to/eval",
    results_root="adoption/results",
    skill_src="/path/to/skills/prism-code-navigation",
    prism_mcp_bin="/path/to/target/release/prism-mcp",
    identifier="sonnet-2x2",
)

# Codex-spark or Opus on Cell 4 only (if Cell 4 survived Sonnet):
summary4, _ = run_2x2(model="gpt-5.5", cells=("4",), ...)
summary4o, _ = run_2x2(model="opus", cells=("4",), ...)
```

Results land in `adoption/results/twobytwo-<identifier>.json` with the full
`{model, cells: {summary}, raw}` structure.

### Cheap-to-expensive model order

1. **Sonnet full 2x2** (`cells=("1","2","3","4")`) — cheapest; read the cell
   table: invocation_rate + pass5_rate + skill_attribution per cell. If Cell 4
   holds (rate comparable to Cell 1), proceed.
2. **codex-spark Cell 4** (`model="gpt-5.5", cells=("4",)`) — mid-tier.
3. **Opus Cell 4** + **gpt-5.5 Cell 4** (`cells=("4",)`) — most expensive; run
   only if the pattern holds at the Sonnet tier.

If Cells 2 or 3 show a drop relative to Cell 1, diagnose before spending on
Cell 4 (the competing env or prompt realism is the variable to fix).

### Smoke verification (Task 6)

Cell-3 one-probe smoke (5 Sonnet trials, `analysis-count-claims-blast`,
isolated env) confirmed the realistic-prompt path runs end-to-end:

```
{'analysis-count-claims-blast': [(True, 'prism-code-navigation'),
  (True, 'prism-code-navigation'), (True, 'prism-code-navigation'),
  (True, 'prism-code-navigation'), (True, 'prism-code-navigation')]}
```

5/5 invoked. The real rates come from the full 2x2 (owner-triggered).
