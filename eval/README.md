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

### Confidence-stratified M2 (P3 gating)

Each M2 direction x stratum entry carries two additive fields alongside the
legacy `raw`/`corrected`/`function`/`pending`/`shortfall` (which stay computed
over ALL edges, exactly as before, so existing baselines remain comparable):
`exact_tier` (raw P/R + tp/fp/fn over prism edges at Exact confidence only,
oracle set unchanged) and `candidate_tier` (`count`/`oracle_confirmed`/
`oracle_unconfirmed` over labeled NameOnly candidate edges — e.g. the capped
Python/JS/TS unknown-receiver edges P3 emits instead of silently dropping).
A P3-style change is gated on **exact-tier P/R being unchanged vs. the
pre-change run; candidate tier is informational and adjudication-fed only**,
never a pass/fail input — compare `exact_tier` across two run JSONs, no new
CLI needed.

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

### Hydrating pending diffs — `tools/hydrate_pending.py`

A pending record is the minimal SHA-agnostic key above; an adjudicator (human or an
LLM running `a2a-bridge/prompts/adjudicate-sample.md`) needs the surrounding source to
judge it. `tools/hydrate_pending.py` is that bridge: it reads the `pending` list from
one or more per-corpus report JSONs and emits, per corpus, the rich evidence shape the
adjudicator prompt consumes (`seed_context` ±2 lines, `site_context` ±3 lines with the
exact line `>`-marked, a per-corpus `id` that verdicts join back on). It is kept out of
`tier_a` proper because it reads corpus *source*, which the metric pipeline never does.

It resolves corpus roots from `corpora.toml` and refuses to run if a corpus checkout's
`HEAD` doesn't match the report's `meta.corpus_sha` (line numbers would be wrong) unless
`--allow-sha-drift` is passed. Run it after a `tier-a` run, before dispatching
adjudicators (needs the harness env for `tomllib`):

```bash
uv run python tools/hydrate_pending.py \
  --report ../docs/eval/tier-a/2026-06-13-prism.json \
  --report ../docs/eval/tier-a/2026-06-13-caddy.json \
  --out /tmp/tier-a-adj
```

The loop: hydrate → adjudicate (per `adjudicate-sample.md`) → append verdicts to
`adjudications.jsonl` → `uv run tier-a --report-only` to fold corrected metrics. See
`docs/eval/tier-a/re-anchor-adjudication-2026-06-14.md` for a worked example.

### Dispatch oracle (`tools/dispatch_oracle.py`)

The §8 **dispatch precision/recall regression gate** for prism's Go interface-dispatch
resolution (Phase-IP). When prism resolves a Go dispatch site
(`x.(caddy.Module).CaddyModule()`), it *mints an implementer set* — every in-repo type it
believes satisfies the interface, RTA-pruned to live/constructed types. This tool checks
that set against gopls `textDocument/implementation` (the ground truth for "what satisfies
interface I") to decide whether prism's set is **sound** (a subset of the real satisfiers,
no false edges) or **over-approximates** (mints a non-satisfier = a `prism_fp` candidate).
Current manifests compare each satisfier by `(package_dir, package_clause, type_name)` and
also require prism's method target `(file, span)` to match gopls's implementation location.
This prevents a same-named type in another package, an external `_test` package, or a
build-tagged sibling method from scoring as a false sound result. Manifests emitted by old
Prism binaries have no identities, so the oracle falls back to names and marks the summary
`identity_mode: "name_only"`; identity-aware output is `"qualified"`.

It is **re-usable on any Go corpus** in `corpora.toml` and is the gate future baselines
must hold. Zero-fanout manifest sites are included: if gopls has satisfiers they are a visible,
non-gating `recall_gap`. `summary.overall.dispatch_precision` is the edge-weighted precision
figure — the aggregate `|P ∩ G| / |P|` over scored sites — and is `null` (not a vacuous 1.0)
when its scored edge denominator is empty. The separate `sound_site_rate` is the site-level
`sound / scored_sites` rate; it is not a precision substitute. `scored_sites` distinguishes an
empty denominator from a timeout-only run.

Regenerate the manifest with the current prism, then run the oracle (needs `gopls` on PATH
and the harness env; gopls can be slow on large corpora, so the per-group timeout is
generous — a group that still times out is recorded `oracle_timeout`, never fatal):

```bash
cargo build --release
target/release/prism nav interface-manifest --repo ~/code/bench-repos/caddy \
  > /tmp/caddy-manifest.json
cd eval && uv run python tools/dispatch_oracle.py \
  --manifest /tmp/caddy-manifest.json \
  --repo ~/code/bench-repos/caddy \
  --corpus caddy \
  --out /tmp/caddy-dispatch-oracle.json
```

To gate a branch's newly Exact edges, pass a hardened-oracle output from the same corpus as
the baseline:

```bash
cd eval && uv run python tools/dispatch_oracle.py \
  --manifest /tmp/caddy-branch-manifest.json \
  --repo ~/code/bench-repos/caddy \
  --corpus caddy \
  --baseline /tmp/caddy-main-dispatch-oracle.json \
  --out /tmp/caddy-branch-dispatch-oracle.json
```

The delta contains `newly_exact_sites` for `fanout: 0 → >0` transitions or new full
implementer identities. Its `gate_ok` is true only when (a) none of those delta sites is `over_approx`, `oracle_timeout`, `oracle_unresolved`, or `target_mismatch` — `recall_gap` remains visible but non-gating — **and** (b) the run's fanout-positive coverage meets both floors: site coverage ≥ 0.90 and edge coverage ≥ 0.90 (`summary.fanout_positive_coverage`; `--site-coverage-floor` / `--edge-coverage-floor`, defaults 0.90). A run with no delta blockers but 50% fanout-positive coverage therefore FAILS the gate; `not_dispatch`, `interface_zero_fanout`, `external_definition` and `unknown_definition` are reported exclusions, not failures. The baseline pair is refused unless its pins agree exactly:
corpus SHA, Go/gopls versions, `GOOS`, `GOARCH`, tags, and `GOWORK`. The tool forces `GOWORK`
to the corpus-root `go.work` when present, otherwise `off`, so an ambient parent workspace
cannot alter the comparison universe.

**Build-tag coverage.** gopls only type-checks the files the current build configuration
selects, so every dispatch site in a file behind a `//go:build` line the default (empty) tag
set does not satisfy used to fail at the `definition` stage and stand as an unadjudicated
`oracle_unresolved` — hugo's `scss/tocss.go` (`//go:build extended`) was exactly that. After
the default pass the oracle derives each such file's required tag set from its constraint
expression (and its `_<goos>_<goarch>` filename) and re-adjudicates **only those
still-unadjudicated sites** in one extra gopls session per tag set (`GOFLAGS=-tags=...`, with
its own decl cache — a different build configuration can have a different satisfier set).
Sites are never removed from the denominator: a file whose constraint no tag set can satisfy
under the pinned `GOOS`/`GOARCH` (`//go:build linux` on darwin, `sync_darwin.go` on linux)
stays counted, stays `oracle_unresolved`, and is named in
`summary.build_constraints.unadjudicated_sites`. A prism identity whose defining file the
adjudicating tag set *excludes* cannot be judged absent by that session, so it fails closed
to `oracle_unresolved` instead of minting a false `over_approx`; a legacy name-only manifest
has no file evidence at all, so under a tag set *any* prism-only name fails closed the same
way. Platform aliases follow the go command (`foo_linux.go` **is** selected for `GOOS=android`,
`foo_darwin.go` for `ios`, `foo_solaris.go` for `illumos`), and both directives require the go
separator — `// +buildextended` is an ordinary comment, not an `extended` constraint.
`summary.build_constraints`
is emitted only when some in-scope file needed the pass, so corpora without one are
byte-identical to a pre-build-tag run. The environment pins are unchanged — the derived tag
sets are corpus content, not a pin — so existing baselines stay comparable.

The summary (printed to stdout and in `comparison.json` under `summary`) reports edge-weighted
`dispatch_precision`, separate `sound_site_rate`, per-`(interface, method)` metrics,
`scored_sites`, all classifications, and the offending identity/target evidence. Per-site
records include legacy display names plus qualified identities, targets, and classifications;
a site re-adjudicated under build tags also carries `build_tags`, `build_constraint`, and
`build_tag_status`.
The full taxonomy and gopls-query design are in the `tools/dispatch_oracle.py` module docstring.

## Snapshots and Baselines

Oracle inventories are snapshotted under `eval/snapshots/<corpus>-<sha>.json` so
sampling is stable for a corpus SHA. Raw run JSON is written under `eval/runs/`.
Per-corpus reports are written under `docs/eval/tier-a/` as dated `.json` and
`.md` files.

`docs/eval/tier-a/baseline.md` is the deliberate comparison anchor. Update it only
after a human-triggered full run, adjudication, and validity-floor review.
