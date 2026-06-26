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
