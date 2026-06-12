# Handoff: Tier-A SHIPPED → next phase is S3 (2026-06-12)

Orientation doc for a fresh/compacted session. Read order: this file →
`docs/eval/tier-a/baseline.md` → `tier-a-followups.md`. The spec/plan are reference,
not reading list (both carry outcome banners).

## State of the world

`main` @ `e063543`, pushed. The Tier-A initiative (spec
`docs/superpowers/specs/2026-06-11-prism-tier-a-accuracy-harness-design.md` rev 4, plan
`docs/superpowers/plans/2026-06-11-prism-tier-a-harness.md` rev 2) is **fully executed,
final-reviewed, fixed, merged in a 9-commit shape** (5 bisectable WP2 batches +
profile + timing docs + `feat(nav)` 6dc2336 + `feat(eval)` e8f91f7 + baseline e063543).

Two deliverables now exist that did not before:

1. **WP2:** 121→24 umbrella test targets (`cargo test --test <dir> <file_stem>::`
   filter syntax; CLAUDE.md updated). Full `cargo test` ≈ 47s clean / 37s lib-touch.
   A guard test (`tests/integration/umbrella_completeness_test.rs`) fails if a test
   file lacks its `mod` line — when adding a test file, add `mod <stem>;` to that
   directory's `main.rs`.
2. **The Tier-A accuracy harness** (`eval/`, uv Python 3.12, 95 pytest tests) with its
   first adjudicated baseline committed. `docs/eval/tier-a/baseline.md` is the anchor;
   `eval/adjudications.jsonl` holds 1,218 verdicts; snapshots under `eval/snapshots/`.

## How to run the harness (operational knowledge, expensive to re-derive)

```bash
cargo build --release                      # ALWAYS first — SutStale aborts on binary != HEAD
cd eval
uv run tier-a --matrix-only                # seconds, no LSP; regression gate (exit 1 on regression)
uv run tier-a --quick                      # minutes; prism corpus, rust-analyzer needed
uv run tier-a --corpus all --date YYYY-MM-DD   # full baseline rerun (5 corpora)
uv run tier-a --report-only runs/<f>.json  # recompute metrics from stored probes (G3 replay)
uv run pytest -q                           # harness self-tests
```

Gotchas that cost real time this session:
- **`uv run` must execute from `eval/` or with `--directory eval`** (heredoc + --directory is flaky; use a temp script file).
- **`SutStale` is strict by design**: binary sha must equal HEAD and tree must be
  tracked-clean (`-uno`). After ANY commit, rebuild release before harness runs.
  `--allow-stale-sut` / `allow_stale=True` for reconcile-style local loops.
- **Corpora are machine-local** (`~/code/bench-repos/{tokio,caddy,flask,click}` + prism
  itself), SHAs pinned in `eval/corpora.toml` (drift ⇒ `baseline_invalid` unless
  `--allow-drift`). flask/click have `.venv` (uv 3.12, editable). gopls is in mise's
  go bin — `export PATH="$PATH:$(go env GOPATH)/bin"` before live runs.
- **Replay is the cheap path**: metric/logic changes never need oracle re-runs —
  `recompute_metrics_from_stored` over `eval/runs/*.json` (runs/ is gitignored; the
  current run JSONs exist locally only — a fresh clone must re-run live to get them,
  but baseline.md + reports + adjudications are committed).
- Oracle bring-up traps (all FIXED in code, listed so nobody "simplifies" them away):
  hierarchical documentSymbol capability MUST be declared; prepareCallHierarchy needs
  the name-token column (`selection_char`); server→client requests must be answered;
  `target/` excluded from the universe; gopls method names `(*T).m` normalized.

## What changed vs the original analyses (do NOT trust the older claims)

- **Meta-analysis §3 "bimodal: perfect on unique names"** — REVISED by the baseline:
  precision holds (U corr P 0.99 callees / 0.89 callers after honest 1:1 matching);
  *recall does not* (0.89/0.70): the prototype never tested callee-direction or
  qualified calls. Use baseline.md numbers, never the §3 table.
- **s1-followups item 4 "21-min cargo test"** — was spindump-contaminated; corrected
  in place. Real pre-WP2 baseline was ~2 min; WP2 still cut it 3× (execution
  parallelism, not link time).
- **Spec §2.8 budget & G1 gates** — amended in practice: class-based bulk adjudication
  (owner-approved) replaced item-by-item; G1(a) is recorded NOT MET (see baseline.md
  "Post-final-review amendment").
- **a2a-bridge claude reviewer model override** (`model = "sonnet[1m]"`) — defect on
  the bridge side; owner filed it. `examples/a2a-bridge.slicing-implement-sonnet.toml`
  is parked-but-correct; `-fast.toml` (codex-only review) works and is the low-risk
  tier. Re-enable sonnet when the bridge fix lands.

## Next phase: S3 — call-resolution precision floor

The work-list is **measured** (baseline.md "S3 work-list distilled"), ranked by impact:

1. **Collision-method caller claims** — corr P=0.00 at tokio scale (390 FPs:
   `poll`/`as_fd`/`write` across receiver types), caddy 441 (`Error` class). Fix
   shape per substrate analysis: receiver/type-aware filtering or confidence demotion
   of name-only method edges (ties into `Source`/`score` on nav evidence; SCIP-oracle
   seam exists as `Source::ExternalIndex`).
2. **Qualified `Type::fn`/`mod::Type::fn` binding** — flips the matrix
   `type_method_qualified` known_fail; dominant U-callee recall gap.
3. **Constructor edges** (`Self(..)`, enum variants, Python class/exception calls).
4. **decl→impl seed mapping** (Go interface/Rust trait declarations) — harness-side.
5. **Nested-def attribution** (callees inside nested defs credited to outer fn).

**S3 acceptance = before/after vs baseline.md** via full corpus rerun + matrix flips
(`known_fail`→pass must be status-updated, regressions fail the run). Start S3 with
brainstorm→spec→plan→subagent ritual; the harness gives objective gates for free.

## Roadmap after S3 (sequencing unchanged, reconfirmed by owner)

S3 → **S2 node-identity hardening** (byte-ranges, span-keyed function identity —
MUST land before Plan B; deletes Plan B Slices 3d/5) → **Plan B taint_reaches**
(plan rev 4 exists) → Tier-B (flow-level taint fixtures; inputs already located:
`~/code/agent-eval/cache/prism-cwe-fixtures` CWE sets + `cvefixes` dataset) →
Tier-C value A/B (`~/code/agent-eval/cache/martian-bench`) — the go/no-go for the
extended thesis (LLM dev/planning/architecture support, not just review).
Language posture (owner): Rust+Go = confident change-making now; Python =
matrix-protected, oracle blocked on followup 9 (basedpyright / references fallback);
TS/JS = next oracle slot (high priority, prevalence rationale in spec §8.1);
C/C++/Java = review-only, unplanned (clangd exists if ever wanted).

## Deferred (full list: `tier-a-followups.md`)

Highest-value three: (1) Python oracle fallback → gives Python an anchor corpus;
(2) adjudication content-fingerprint re-anchoring → verdicts survive corpus bumps
(line-keyed today; 13 went stale during this session's own fixes); (3) matrix v2
collision-rich fixtures (current micro-fixtures certify name-reachability, not
dispatch — and the decorator B2 flip-indicator fixture doesn't actually trigger the
func_index quirk).

## Process facts that save tokens next session

- **a2a-bridge ritual** (all from `~/code/a2a-bridge`): implement =
  `./target/release/a2a-bridge implement "$(cat brief.md)" --repo <abs> --base-ref <branch> --config examples/a2a-bridge.slicing-implement[-fast].toml`;
  merge = `git fetch <clone> <branch> && git cherry-pick -n FETCH_HEAD && git commit -C FETCH_HEAD --reset-author && rm -rf <clone>`;
  reviews advisory, operator accepts; max 3 attempts then operator triages.
  Workflow prompts need a **`{{input}}` placeholder** to receive `--input` content.
  Codex truncates structured output around ~150 list items — batch at ≤80–150 and
  retry missing IDs.
- **Dual-adjudicator protocol validated** (κ=0.74, zero FP↔FN flips, codex
  conservative-toward-ambiguous): reusable pattern — claude judges a blinded sample,
  codex bulk-labels, escalations return to claude, records carry adjudicator identity.
  Prompt: `~/code/a2a-bridge/prompts/adjudicate-sample.md` + config
  `examples/a2a-bridge.slicing-adjudicate.toml`.
- Briefs for containerized codex can POINT at committed plan sections (the clone has
  the repo) — paste only amendments/constraints verbatim.
- Timing protocol caveat: ambient rust-analyzer races `cargo clean`/`rm -rf target`
  (it recreates metadata mid-delete); validate clean-build legs by `Finished`-line +
  user-time coherence, not empty-dir purity.
