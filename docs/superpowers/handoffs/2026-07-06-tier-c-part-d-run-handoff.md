# Tier-C Part-D — Run & Analysis Handoff (2026-07-06)

**Status:** corpus FROZEN, scorer FROZEN, run pipeline VALIDATED end-to-end on one
codex cell. **Not yet run at scale.** Owner is holding the full run; owner will run
**codex first**, claude later.

Branch `tier-c-part-d` (15 commits ahead of `main`). This doc + the aggregator
(`eval/tier_c/partd_aggregate.py`) are the last additions before the PR.

---

## 1. What Part-D measures (one paragraph)

Part-C (debug-fix, citation precision) closed at near-parity: prism *grounds* specs
but doesn't flip the answer, at +12% cost (see
`docs/analysis/2026-07-05-tier-c-partc-fair-scorecard.md`). Part-D is the real test:
a **structural "refactor blast-radius"** task. Each task renames/changes the contract
of one symbol `S`; the arm must enumerate every site that must change. We score the
arm's claimed sites by **deterministic set-math** against a frozen, adjudicated gold
set — no judges, no LLM, no prism in the scorer. The **headline metric is site-level
ΔD-recall**: recall over the **grep-hard D-subset** (gold sites in files where `S`'s
name never appears with a word boundary → a plain text search cannot find them). This
is where structural navigation should win if it wins anywhere.

Gold construction = Fable's **renaming-forwarder closure**, grep-per-hop, source-verified,
**prism-independent** (0 gold sites are prism-only-provenance — programmatically checked).
Method of record: `docs/superpowers/specs/2026-07-05-tier-c-part-d-gold-methodology.md`.

---

## 2. The corpus (11 runnable cells/model + 1 excluded)

Denominators below are the **frozen, authoritative** values (perfect-arm dry run →
dR=1.0 / file-F1=1.0 / phantom=0 for every task). `Dsite` = site-level D-recall
denominator (the headline); `Dfile` = secondary file-level; `bait` = phantom-baited
excluded sites.

| task id | lang / role | \|gold\| | Dsite | Dfile | bait |
|---|---|---:|---:|---:|---:|
| `prometheus-promql-walk` | Go **strong** | 16 | 6 | 5 | 10 |
| `hugo-converter-convert` | Go **strong** | 10 | 7 | 4 | 13 |
| `prometheus-matchstring` | Go weak | 12 | 9 | 5 | 13 |
| `caddy-requestmatcher-migration` | Go **archetype-B** (interface migration) | 26 | 3 | 3 | 15 |
| `ruff-typechecker-match-annotation` | Rust **strong** | 32 | 28 | 19 | 6 |
| `ruff-imported-qualified-name` | Rust precision/D2 | 17 | 16 | 11 | 21 |
| `typescript-resolve-signature` | TS **strong** | 19 | 7 | 7 | 6 |
| `typescript-resolve-alias` | TS weak | 41 | 17 | 13 | 3 |
| `django-check-registry-run-checks` | Py **headline** | 59 | 51 | 19 | 4 |
| `mypy-meet-types` | Py secondary | 15 | 3 | 3 | 7 |
| `guava-equivalence-doequivalent` | Java strong-only | 36 | 25 | 3 | 11 |

**Excluded (admission FAIL — kept as failure records, DO NOT run):**
`guava-converter-doforward` (D1=0, no gold.json), `guava-forwardingmap-standard-containskey`
(|gold|=2, not in the TOML). Java is **strong-only** (no weak Java task; owner-approved).

A "full run" = these **11 tasks × 1 model** = 11 cells; **each cell runs BOTH arms**
(prism-off + prism-on) internally. So codex-slate = 11 cells / 22 arms, claude-slate
likewise.

---

## 3. Preflight (do this ONCE per machine/session, before either slate)

```bash
cd /private/tmp/prism-partd        # the tier-c-part-d worktree (or wherever it lives)

# 1. Build BOTH binaries from ONE tree (the matched-binary preflight will FAIL LOUD otherwise)
cargo build --release --bin prism --bin prism-mcp --features mcp

# 2. Pin them so prewarm + the agent's prism-mcp provably share one build + nav cache
export PRISM_BIN=/private/tmp/prism-partd/target/release/prism
export PRISM_MCP_BIN=/private/tmp/prism-partd/target/release/prism-mcp
# (both must resolve to the SAME dir — the preflight enforces this)

# 3. Verify inputs
ls ~/code/bench-repos/{prometheus,hugo,caddy,ruff,TypeScript,django,mypy,guava}   # all present
ls ~/.codex/auth.json                 # codex arms seed from here
ls ~/.claude/.credentials.json        # claude arms seed from here (needed for the claude slate)
```

**Sanity of the prism engagement path** (optional, ~30s, no LLM spend) — this is exactly
what de-risked the validation:
```bash
cd ~/code/bench-repos/prometheus && git worktree add --detach -q /tmp/pw 505095b
$PRISM_BIN nav --cache-dir /tmp/pwc repo-map --repo /tmp/pw >/dev/null   # cold build, ~25s on prometheus
cd /private/tmp/prism-partd/eval && PRISM_MCP_BIN=$PRISM_MCP_BIN uv run python -c \
  "from tier_c.arm_runner import warm_gate_check; print(warm_gate_check('/tmp/pw', cache_dir='/tmp/pwc'))"
# expect {'ok': True, ..., 'tools_count': 8}
cd ~/code/bench-repos/prometheus && git worktree remove --force /tmp/pw; rm -rf /tmp/pwc
```

---

## 4. Running the full run

### 4a. codex slate (do this FIRST) — `--model gpt-5.5`

```bash
cd /private/tmp/prism-partd/eval
ROOT=tier_c/runs/partd/full-codex-2026-07-06     # one root groups the whole slate
TASKS="prometheus-promql-walk hugo-converter-convert prometheus-matchstring \
caddy-requestmatcher-migration ruff-typechecker-match-annotation ruff-imported-qualified-name \
typescript-resolve-signature typescript-resolve-alias django-check-registry-run-checks \
mypy-meet-types guava-equivalence-doequivalent"

for t in $TASKS; do
  echo "=== $t (codex) ==="
  uv run tier-c run-partd --task "$t" --model gpt-5.5 --live \
    --run-id "$t" --run-store-root "$ROOT" \
    --bench-root ~/code/bench-repos \
    --prism-build-dir /private/tmp/prism-partd/target/release \
    || echo "CELL FAILED: $t (see $ROOT/$t/status.json)"
done
```

- **Use `gpt-5.5`, NOT `gpt-5.5-xhigh`.** The `-xhigh` reasoning-effort flag is wired only
  into the judge path (`live_ask`), **never into the arm command** (`build_codex_cmd`).
  Part-D has no judges (pure set-math), so `-xhigh` would change nothing while looking
  like it does. Codex arms run at codex's default effort. (Wiring xhigh into arms is a
  deferred item — §6.)
- Each cell = both arms. `--run-id <task>` under a shared `--run-store-root` puts every
  cell in `$ROOT/<task>/` so the aggregator (§5) rolls them up.
- Re-running a cell: add `--force-new` (clobbers `$ROOT/<task>/`).
- **Cost/wall (observed, codex/prometheus):** off arm ~162s + prewarm ~25s + on arm ~208s
  ≈ **6 min/cell**; ~365k in-tok off / 667k in-tok on. 11 cells ≈ **65–75 min serial**.
  Bigger repos (kubernetes-scale) would prewarm slower, but none of these are that large.

### 4b. claude slate (LATER) — `--model opus-4.8`

**Identical command, `--model opus-4.8` and a distinct root:**
```bash
ROOT=tier_c/runs/partd/full-claude-2026-07-06
for t in $TASKS; do
  uv run tier-c run-partd --task "$t" --model opus-4.8 --live \
    --run-id "$t" --run-store-root "$ROOT" \
    --bench-root ~/code/bench-repos \
    --prism-build-dir /private/tmp/prism-partd/target/release \
    || echo "CELL FAILED: $t"
done
```

**How codex vs claude differ under the hood** (both go through the SAME `run-partd`
path; the model string routes the runner — `model.startswith("opus")` → claude, else codex):

| | codex (`gpt-5.5`) | claude (`opus-4.8`) |
|---|---|---|
| runner | `CodexRunner` (`codex exec --json`, prompt on stdin) | `ClaudeRunner` (`claude -p --output-format stream-json`) |
| isolation | `CODEX_HOME` (skill + `config.toml` MCP for ON; auth-only for OFF) | `CLAUDE_CONFIG_DIR` (skill + `mcp__prism` allow for ON; deny Write/Edit for OFF) |
| prism wiring (ON) | `[mcp_servers.prism]` in `CODEX_HOME/config.toml` | `--mcp-config <per-checkout>.json --strict-mcp-config` |
| auth seed | `~/.codex/auth.json` | `~/.claude/.credentials.json` |
| MCP timeout | codex default (generous startup in isolated home) | `MCP_TIMEOUT=MCP_TOOL_TIMEOUT=600000` set by runner |

Everything else — pinned worktree checkout, prewarm+warm-gate (ON only), impact-JSON
parse with one retry, set-math scoring, persistence — is shared and model-agnostic.

---

## 5. Analyzing / evaluating a run

### 5a. Corpus roll-up (the tool)

```bash
cd /private/tmp/prism-partd/eval
uv run python -m tier_c.partd_aggregate tier_c/runs/partd/full-codex-2026-07-06
# ...and once both slates exist, pass BOTH roots to see them side by side:
uv run python -m tier_c.partd_aggregate \
  tier_c/runs/partd/full-codex-2026-07-06 tier_c/runs/partd/full-claude-2026-07-06
```

It prints, per model: a per-task table (dR off / dR on / **ΔdR** / Δfile-F1 / dose /
adm / leak / phantom) plus the **headline summary**:
- `valid-headline (administered & no-leak)` — the cells that count. A cell with
  **dose 0 (`!ADM`)** or **`!LEAK`** is EXCLUDED from the headline (a 0-dose "on" arm
  silently ran without prism; a leak broke blinding). Investigate and re-run those.
- `off-saturated` (`sat` flag, dR_off=1.0) — cells the OFF arm already maxed, which
  **cannot discriminate prism** regardless of the on arm (see §6 caveat).
- **two means:** `all valid` and `discriminating only` (excludes saturated). Report both;
  the discriminating-only mean is the honest headline for "does prism help where it can."

### 5b. Per-cell detail

Each cell prints its own `render_partd` block at run end and persists:
`$ROOT/<task>/<task>-impact-<model>.json` (the scored cell — `report_off`/`report_on`,
`d_recall_delta`, `dose`, `administered`, `leaked`), plus per-arm `*.off/on.{meta,prompt,out.md,raw.jsonl}`
and `*.on.prewarm.json` (prewarm + warm-gate telemetry). `status.json` is `success`/`failed`.

### 5c. Integrity gates (check BEFORE trusting any Δ)

1. `administered == true` for every ON arm (dose > 0). A 0-dose ON arm is not a prism
   measurement — the aggregator flags it `!ADM` and drops it from the headline.
2. `leaked == false` (no `mcp__prism`/`nav_*` tool names in the arm's prose).
3. `phantom` — a wrong claim of a phantom-baited excluded site. Non-zero is informative
   (precision leak), not disqualifying; note it.
4. `status.json == success`. A `failed` cell wrote a partial dir; re-run with `--force-new`.

---

## 6. Caveats & findings

- **Off-saturation is real and already observed.** The validation cell
  (`prometheus-promql-walk`, codex) scored **dR off 1.000 / on 1.000 → ΔdR +0.000**: the
  off arm already recovered all 6 grep-hard D-sites (codex reasons through the Go
  AST-walker's call sites). The on arm cast a *wider* net (35 claims vs 22 → lower
  precision, Δfile-F1 −0.115) without gaining D-recall. **A "strong" task can still be a
  non-discriminator.** Cheap mitigation: the aggregator surfaces `sat` cells and reports a
  discriminating-only mean; if you want to save spend, you *could* run all 11 OFF arms
  first and skip the ON arm for any task the off arm already maxes — but the current
  driver runs both per cell for simplicity. n=1/codex; opus-off may whiff more, so don't
  drop tasks from the corpus on codex evidence alone.
- **Every ON arm cold-builds the CPG.** Checkout is a fresh `git worktree` per run, so the
  nav cache (keyed by canonical worktree path) is always cold; prewarm rebuilds it (~25s
  on prometheus, more on bigger repos, 900s budget). The warm-gate then confirms a warm
  prism-mcp handshake (<15s) before the agent launches — a 0-dose arm is caught here, not
  scored.
- **`caddy-requestmatcher-migration` is archetype-B** (interface migration, not pure
  rename) with a documented substring-D1 caveat; **`guava-equivalence-doequivalent`** has
  a D-file denominator of 3 (documented, not a defect).
- **All golds are prism-independent** (0 prism-only sites) and phantom-bait uses REAL
  enclosing symbols (Fable's phantom-channel calibration). The `exclusion_table` dict in
  each gold is INERT (documentation only); only `sites[]` entries with
  `adjudication:"excluded"` are scored as phantom bait.

---

## 7. Deferred work

1. **codex xhigh arm effort NOT wired.** `model_reasoning_effort` is applied only in
   `live_ask` (judges). To run codex arms at xhigh, add the effort flag to
   `build_codex_cmd` (`arm_runner.py`) keyed off `_MODEL_EFFORT`. Not needed for the
   current run (default effort is the intended arm setting).
2. **22× prism cold-build regression** (perf, pre-existing): parallelize
   `build_resolved_call_edges` + share the resolution memo. Cold prewarm dominates ON-arm
   wall; not blocking, but every ON arm pays it.
3. **No Part-D path in `rescore.py`.** Analysis is via the cell JSONs + `partd_aggregate`.
   The persistence shape clones Part-C so a generic rescore *should* work, but it's
   untested for Part-D — don't assume `rescore` works on partd runs without checking.
4. **Java is strong-only.** Both weak Java candidates failed admission
   (`guava-converter-doforward` D1=0; `guava-forwardingmap-standard-containskey` |gold|=2).
   If Java needs a second (weak) task, a NEW forwarder target must be found + gold-built.
5. **Tier-C Part-C end-task harness (fork B)** is a separate, unmerged track (branch
   `tier-c-part-c`); not part of this run.

---

## 8. File map / provenance

- **Corpus:** `eval/tier_c/issues/structural.toml` (12 tasks, 11 runnable) — forwarder-blind prompts.
- **Golds (frozen):** `eval/tier_c/gold/<task>/{gold.json,ADJUDICATION.md}` +
  `GOLDBUILD-PROCEDURE.md`, `FABLE-FIXES.md` (build + review provenance).
- **Runner:** `eval/tier_c/partd.py` (cell + persistence), `arm_runner.py` (Codex/Claude
  runners, preflight, prewarm, warm-gate), `impact.py` (JSON parse + retry),
  `structural.py` (set-math scorer, site-level dR headline), `prompts.py` (impact contract + steer).
- **Aggregator (NEW):** `eval/tier_c/partd_aggregate.py` + `tests/test_tc_partd_aggregate.py`.
- **CLI:** `tier-c run-partd --task … --model … --live --run-id … --run-store-root …`.
- **Validation run:** `eval/tier_c/runs/partd/partd-validate-promql-01/` (gitignored) —
  status success, all stages green, safety props fired (dose 10, administered, no-leak).
- **Methodology / scorecard:**
  `docs/superpowers/specs/2026-07-05-tier-c-part-d-gold-methodology.md`,
  `docs/analysis/2026-07-05-tier-c-partc-fair-scorecard.md`.
- **Durable ledger (gitignored, main worktree):** `.superpowers/sdd/progress.md` — blow-by-blow.

---

## 9. TL;DR to resume

1. Preflight (§3): build matched binaries, export `PRISM_BIN`/`PRISM_MCP_BIN`, verify creds.
2. Run codex slate (§4a): the `for t in $TASKS` loop, `--model gpt-5.5`, into `full-codex-<date>`.
3. Analyze (§5): `uv run python -m tier_c.partd_aggregate <root>`; check adm/leak gates;
   report the discriminating-only mean ΔdR as the headline.
4. Later: claude slate (§4b), `--model opus-4.8`, distinct root; aggregate both roots together.
