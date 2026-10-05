# Offline SecBench measurement

From the repository root, run:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 -m eval.secbench
```

This builds the pinned SUT offline, inspects all 583 acquired packages, verifies
every source/exploit/metadata hash against `input-pins.json`, runs Prism, applies
the frozen manual labels, and writes JSONL, JSON, Markdown and raw compressed
observations to `/Users/wesleyjinks/prism-evidence/meas/secbench/rerun/`.
The corpus's 17 acquisition failures remain in the 600-row output.
No npm package or exploit is executed. Node and the already acquired TypeScript
5.9.3 compiler are required. Python uses only the standard library.

The installed eval CLI is also `secbench`; no installation is needed for the
command above. Options include `--out`, `--inputs`, `--packages`, `--compiler`,
`--workers`, `--timeout`, `--inspection`, `--sut-repo`, and `--no-build`. Reused inspections
rehash the complete input population. A different SUT requires a new explicit
benchmark revision and adjudication; do not silently refresh the pins or labels.
After the controller commits the harness, use `--sut-repo` to point at a
controller-provided checkout of the exact measured SUT commit. Harness HEAD and
SUT HEAD are separate; the measurement must not silently move with a docs commit.

Sources are exploit-linked exported functions and test-fed data parameters.
Callbacks, factory setup inputs, ambiguous payload parameters, and HTTP setup
arguments receive no payload credit. Sink metadata is resolved exactly or by a
unique recorded basename repair. CLI seeding is line based; credit requires a
matching parameter Def byte range and a matching terminal value occurrence in
the per-root witness. A synthetic zero-width sink use is accepted only for a
unique same-name AST identifier on that line and the same complete member path.
This is conservative reachability measurement, not proof of exploitability.
Object-property inputs currently seed the containing formal parameter, so the
benchmark does not establish field-selective input soundness.

`taint-reaches` is decisive because it exposes per-root/per-sink verdicts.
`dfg-stats` runs on every acquired package. Eligible cases also retain callers,
callees, ego, classic taint, and chop observations. Classic APIs use a synthetic
JSON diff touching only parameter declaration lines. A decisive query error is
`prism_error`; comparison-channel errors are retained separately. Function-only
credit uses call-graph enclosing spans and is explicitly weaker than value flow.

First-break labels outside the frozen sample are deterministic hypotheses.
The frontier contains the union of line seeds, which can include unrelated
callback parameters. Features anywhere in a candidate callable span are only
a syntax screen; the manual sample records constructs on the payload path.
Unsupported/unknown paths retain their separate denominator. The sample is
six lowest SHA256(class/entry) values per class from the initial available-GT
population, frozen before the full run; six later GT exclusions stay in it.
Adjudication is by the same worker and is not independent validation.

Replay the retained measurement with narrowed GT and refined sink occurrences:

```sh
PYTHONDONTWRITEBYTECODE=1 python3 -m eval.secbench.replay \
  --run /Users/wesleyjinks/prism-evidence/meas/secbench/run \
  --inspection /Users/wesleyjinks/prism-evidence/meas/secbench/inspection-final.jsonl \
  --adjudications eval/secbench/adjudications.json \
  --out /Users/wesleyjinks/prism-evidence/meas/secbench/final
```

Replay authenticates raw stdout even for errors, rehashes all input bytes, and
refuses a new source line or sink line. It never creates observations for a
different seed. Input narrowing is valid because witnesses retain each root's
individual verdict. Original run classifications are superseded by `final/`;
the original raw observations and binding remain unchanged.

Tests:

```sh
node --test eval/secbench/inspect.test.mjs
PYTHONDONTWRITEBYTECODE=1 python3 -m unittest eval.secbench.test_run eval.secbench.test_replay eval.secbench.test_io eval.secbench.test_cli
```
