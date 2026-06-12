# prism-eval — Tier-A accuracy harness

> **Disambiguation (separation contract, spec §2.1):** this directory is prism's
> self-contained evaluation harness. It is **unrelated to `~/code/agent-eval`**
> (the review-agent harness for `~/code/agent-knowledge`). Nothing here imports
> from, writes to, or depends on agent-eval/agent-knowledge; corpus repos were
> one-time *copied*. If shared needs emerge, split to `~/code/prism-eval` —
> do not entangle the two.

Tier-A measures Prism navigation accuracy against language-server oracles and
by-construction micro-cases. It is host-side, not CI-side: live corpus runs need
bench repos, oracle binaries, and a fresh Prism release binary.

## Oracle Install

Put these servers on `PATH` before live runs:

```bash
rust-analyzer --version
go install golang.org/x/tools/gopls@latest
npm i -g pyright
pyright-langserver --version
```

`--matrix-only` does not start oracle servers. It only needs the Prism SUT binary.

## Corpus Prep

The corpus list lives in `eval/corpora.toml`. `pinned_sha` is intentionally empty
until the first baseline run records the live SHAs.

Expected local paths:

```text
.                                 # prism itself
~/code/bench-repos/tokio
~/code/bench-repos/caddy
~/code/bench-repos/flask
~/code/bench-repos/click
```

`caddy` may be copied from `~/code/agent-eval/cache/repos/caddy` at `77e9ce74`;
then run `go mod download`. For Python corpora, create local virtualenvs if the
language server benefits from installed dependencies. Flask and Click are small
typed corpora; Pyright oracle noise remains part of the reported findings.

## SUT Build

Build Prism before running the harness:

```bash
cargo build --release
```

SUT discovery order:

```text
--sut-bin > PRISM_BIN > target/release/prism
```

By default the harness rejects stale binaries, dirty Prism worktrees, and binaries
whose embedded git SHA does not match the repo. Use `--allow-stale-sut` only for
local debugging, not a baseline.

## Runner

Run from `eval/`:

```bash
uv run tier-a --corpus all              # full run, 5 corpora
uv run tier-a --corpus prism            # one corpus
uv run tier-a --quick                   # prism corpus, reduced M2/M3 sample, plus matrix
uv run tier-a --matrix-only             # capability matrix only, no LSP
uv run tier-a --report-only <run.json>  # replay metrics and re-render reports
```

For pre-commit matrix checks on a dirty worktree, rebuild immediately first and
then pass `--allow-stale-sut` so the dirty Prism binary is accepted:

```bash
cargo build --release
uv run tier-a --matrix-only --allow-stale-sut
```

`--report-only` is the G3 replay path: M2 raw metric inputs come from the stored
run JSON `probes` block; corrected metrics additionally apply the current
`adjudications.jsonl`. If `probes` is absent, the JSON is rendered unchanged.

## Adjudication

Pending diffs are triaged into `eval/adjudications.jsonl`. Each record is keyed by
`(corpus, measurement, direction, seed_def, site)`, where `measurement` is
`callers`, `callees`, or `m3`; `direction` is `prism_only` or `oracle_only`;
`seed_def` is `file:selection_line`; and `site` is `file:line`.

Legal verdicts are enforced by `tier_a.adjudication`:

```text
prism_only  -> oracle_miss | prism_fp | oracle_artifact | ambiguous | alias_site
oracle_only -> prism_fn | oracle_artifact | ambiguous
```

All Rust/Go pending diffs and up to 25 sampled Python pending diffs per corpus are
the v1 adjudication budget. Paste regressions and flip-candidates into review
notes; do not re-baseline to hide them.

## Snapshots and Baselines

Oracle inventories are snapshotted under `eval/snapshots/<corpus>-<sha>.json` so
sampling is stable for a corpus SHA. Raw run JSON is written under `eval/runs/`.
Per-corpus reports are written under `docs/eval/tier-a/` as dated `.json` and
`.md` files.

`docs/eval/tier-a/baseline.md` is the deliberate comparison anchor. Update it only
after a human-triggered full run, adjudication, and validity-floor review.
