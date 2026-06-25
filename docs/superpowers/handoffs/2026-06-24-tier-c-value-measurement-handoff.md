# Tier-C Value Measurement — Session Handoff (2026-06-24)

> Written near context compaction. Pairs with memory `project_prism_measurement_maturity.md` and the specs/plans below.

## ✅ RUN COMPLETE — `full-2026-06-24` (exit 0, 15:08→23:24, ~8h15m, 64 arm-runs)
Task `bybu6rj28` finished clean. Corpus: ruff#26287(rust), prometheus#18896(go), pydantic#13300(python), excalidraw#11479(ts) — 8 variants × 4 issues × 2 stages. Output: `…/tasks/bybu6rj28.output`. **All 16 cells NO-GO** — but read the next line before concluding anything about prism.

### ⛔ HEADLINE #1: prism was NEVER ACTUALLY INVOKED — the treatment was never administered
Recovered + audited all 64 arm transcripts (recovery below). **Across every "prism-on" arm: 0 `mcp__…nav_*` tool calls, and 0 of 195 claude Bash + 0 of 740 codex exec commands reference prism/nav** (`manifest.json` `prism_called=False` on all 32 prism-intended arms). The prism-on arms navigated with the same Read/rg/Bash/grep as prism-off → **identical behavior → no treatment contrast to measure, independent of any scoring.** The run-store `used_prism=True` was a HEURISTIC (`variant.prism AND tool_calls>0`), NOT evidence of a prism call — it is a false positive. Likely cause: prism MCP tools were deferred/unprompted (claude shows `ToolSearch` in its toolset), so the models never reached for them. **⇒ CO-P0 (ahead of everything): the harness must FORCE + verify prism usage — steer the stage prompt to the nav tool, pre-warm + ASSERT the MCP handshake, detect REAL prism calls (replace the `tool_calls>0` heuristic), and DISCARD any prism-on arm with 0 prism calls.** No rerun can measure prism value until this holds.

### ⛔ HEADLINE #2: the objective is also DEGENERATE (independent second defect — harness bug, not a prism result)
The gate is driven **solely by citation `precision`** (`report.py:96` `p(sm)=sm.precision`, `_MATERIAL=0.1`). **`precision==0.00` and `recall==0.00` for every arm, every stage** (verified on the surviving excalidraw arms; the other 3 repos differ only by rare single-citation blips). So every `prism_at_lsp_on` delta ≈ 0 → every cell is a **mechanical** NO-GO. Prism was never actually weighed.
- **Root cause** = the relevance oracle is **blind to the code it scores.** `LlmRelevanceJudge.is_relevant` (`judges_live.py:33-37`) is asked *"Is the code at `file:line` (symbol `None`) relevant to fixing this issue? YES/NO"* — **it is never shown the code at that line**, and `symbol` is usually null. Blind, it answers non-YES (`.startswith("YES")`, conservative) → `relevant=False` → `is_valid=(not hallucination) AND relevant = 0` (`investigator.py:20-21,51-53`) → `precision=valid/cited=0`. Confirmed: excalidraw structurally-valid citations (`file_ok/line_ok/symbol_ok=true`) **all** came back `relevant=false`; the only `relevant=true` verdicts are hallucinations (the judge is skipped for bad-structure cites → `relevant` keeps its default `True`), which is why `hallucinations==relevant_count` per arm.
- **Fix drafted** → `docs/superpowers/specs/2026-06-24-tier-c-investigator-relevance-oracle-fix-design.md` (thread the cited line + context window into the judge; `co.read_line` is already called 2 lines away in `verify_citation`). **This is a measurement-methodology change → re-baselines ALL numbers → spec+codex-review-gated, NOT a hotfix.**

### The ONE working signal: the prism-blind RANK judge (excalidraw only — see audit-recovery below)
The Borda rank judge (substance-only, gets no prism) DID differentiate, and on the **plan** stage it favored prism:
- **plan/ts CONSENSUS** (best→worst): `opus+prism` › `opus` › `opus+lsp` › `gpt+prism+lsp` › `gpt+prism` › `opus+prism+lsp` › `gpt` › `gpt+lsp` — **both judges ranked `opus-4.8+prism` #1.** The broken precision gate threw this away.
- **spec/ts CONSENSUS**: `opus` › `gpt` › `opus+lsp` › `opus+prism` › … — bare opus won; judges *disagreed* (anthropic #1=`opus`, openai #1=`opus+prism+lsp`).
- ⇒ **fold the rank-judge consensus in as a co-primary objective** — it's the channel that actually carried signal.

### Detectability — CLEAN (the one well-powered result)
`32/64 correct, p=0.55, detectable=False` → the condition-guesser cannot tell prism-on from prism-off → the judge-based delta methodology is **unbiased and valid**. (n=64 pools across issues×stages×arms, so it IS powered, unlike the per-cell deltas.)

### Per-repo `prism_at_lsp_on` deltas (the ONLY cross-repo numbers that survived — all noise-level)
| repo/stage | gpt-5.5 | opus-4.8 |  | repo/stage | gpt-5.5 | opus-4.8 |
|---|---|---|---|---|---|---|
| ruff spec | 0 | 0 |  | ruff plan | −0.067 | 0 |
| prometheus spec | **+0.043** | 0 |  | prometheus plan | 0 | 0 |
| pydantic spec | 0 | 0 |  | pydantic plan | +0.013 | 0 |
| excalidraw spec | 0 | 0 |  | excalidraw plan | 0 | 0 |

All ≤ |0.067| = a single citation flipping (0.067≈1/15, 0.043≈1/23) — sub-`_MATERIAL`, pure noise.

### 🗂️ AUDIT-TRAIL RECOVERY — the overwrite bug did NOT destroy the specs/plans
Run-store stage artifacts are NOT per-issue namespaced → each issue overwrote the prior → **only excalidraw/ts survives in `eval/tier_c/runs/full-2026-06-24/stages/`** (arm text + `judges.json` + `investigator.json`). BUT the arms ran as real `claude -p` / `codex exec` subprocesses that keep their OWN session logs, which survived. **✅ RECOVERED & REORGANIZED** by `eval/tier_c/recover_sessions.py` → `runs/full-2026-06-24/recovered/<model>/<repo>/<stage>/<variant>.md` (64 files) + `manifest.json` + `README.md` (gitignored; rerun the script to regenerate):
- **All 64 arms recovered** (32 opus + 32 gpt): opus from `~/.claude/projects/*-T-tc-co-*/` (one checkout dir/issue by mtime: `5l-uc8qb`=ruff, `x1z2unhw`=prometheus, `g-bezuym`=pydantic, `mw7roa6v`=excalidraw; 8 each); gpt = the 32 of 58 `~/.codex/sessions/` window sessions whose cwd is a `tc-co-*` checkout. Full spec/plan text + tool traces + reasoning.
- **Mapping**: model/repo/stage **certain** (CLI log / file-path fingerprint / SPEC-vs-PLAN prompt); **prism+lsp variant by EXECUTION ORDER** within each model·repo·stage group of 4 (`[base,+lsp,+prism,+prism+lsp]`) — **VALIDATED text-exact (sim=1.00) vs the 8 surviving excalidraw run-store arms.** (Variant is NOT detectable from tool calls — no arm called prism; see Headline #1.)
- **GONE / never-existed**: the 3 non-excalidraw repos' **judge verdicts + grounding** (overwritten, but **re-derivable** by re-running the cheap judge step on recovered specs — no model-arm re-run); **"code"** (there is NO code stage — Phase 1 is spec→plan only; develop/code is Phase 2, unbuilt).
- **Fix** = namespace `stages/<language>/<stage>/` (do it with Phase-1d-replay) so future runs are auditable without this recovery.

### Secondary limitations (real, but now SECONDARY to the degenerate objective)
- **n = 1 issue per (stage×language) cell** (4 issues, 1/lang) → every per-cell delta is a single judge ranking; no per-cell statistical content. Scale to ≥2/lang.
- **LSP on/off is INERT in spec/plan** — no `shim-log.jsonl` was ever written; per-arm `commands` are all `rg/git/Read/Grep` (and never prism — Headline #1), never a denied type-checker (`lsp_leak=False`, `compiler_assisted=False` everywhere). Neither model runs rust-analyzer/gopls/tsc/mypy while *writing a spec or plan*. ⇒ drop the 4 LSP arms from spec/plan (halve cost); LSP matters only in Phase-2 develop/review where a checker actually runs.

## Repo state
- **origin/main = `89436c04`** (this run's synthesis + oracle-fix spec landed; in sync). prism-mcp rebuilt locally from main (`sha256:c2e3172b…`, NOT committed — it's a binary; rebuild with `cargo build --release --bin prism-mcp --features mcp`).
- Concurrent MCP work (#138–#142, `src/mcp/`+CPG) landed mid-session — always disjoint from `eval/tier_c/` (rebase clean).
- **Recovery utility**: `eval/tier_c/recover_sessions.py` rebuilds any overwritten run's arms from the CLI session logs → `runs/<id>/recovered/` (used for `full-2026-06-24`).

## What this session built (the arc)
1. **Buy measurement** of the 5 Python maturity slices (call-stats `10572e3`→`b4741fe`, 4 corpora): `import_member` +360 fastapi/+680 pydantic (new sound recall, already incl. aliases on the forward side); **unresolved RATE FLAT** (59→59, 54→55) — the 5 slices = precision-correction + small recall, NOT a rate move → diminishing returns on more Python recall rungs. Tier-A: 41/42 ok after the alias flip.
2. **Alias callers-index fix → PR #137 MERGED** (`fbb0ac9`): `from m import f as g; g()` was invisible to `callers(f)` (callers index keyed by syntactic name); forward resolution already handled it. Query-layer fix in `scoped_caller_sites`; flipped Tier-A `from_import_alias`.
3. **Tier-C value-measurement harness** — designed + built across 4 phases, subagent-driven TDD, codex `gpt-5.5` xhigh reviews at each spec + final:
   - **Phase-1** (spec+plan stage scaffold + investigator + judges + chain + report): plan `docs/superpowers/plans/2026-06-23-tier-c-phase1-harness.md`.
   - **Phase-1b** (live codex/claude drivers + parsers + claim_count + tie-break + detectability + report cells + run orchestrator): `…2026-06-23-tier-c-phase1b-live-and-report.md`.
   - **Phase-1c** (live loop + real LLM judges: `ask` seam, LlmRankJudge/Relevance/ConditionGuesser, RoutingArmRunner, pooled detectability, `run_live`, `tier-c run --live`): `…2026-06-24-tier-c-phase1c-live-loop-judges.md`.
   - **Phase-1d-core** (LSP **2×2** {prism off/on}×{lsp off/on} via deny-shim PATH; per-command logging + classify; 5-contrast `Cell2x2`; deterministic **run-store**): spec `…2026-06-24-tier-c-phase1d-lsp-matrix-and-runstore-design.md` (rev-2), plan `…2026-06-24-tier-c-phase1d-core.md`.
   - **Corpus** (4 OPEN issues, 1/lang, leakage-safe, picked via 4 parallel `gh` subagents): `eval/tier_c/issues/issues.toml` — ruff#26287(rust), prometheus#18896(go), pydantic#13300(python), excalidraw#11479(ts). Pinned SHAs; ~25 already-PR'd candidates excluded.
   - **Smokes**: 1-issue (4-variant, Phase-1c) + 1-issue (8-variant, Phase-1d) both PASSED; caught+fixed real bugs (below).

## Design-of-record (READ THESE for the model)
- `docs/superpowers/specs/2026-06-23-tier-c-value-measurement-design.md` (rev-3): the core value-spike — open-issue oracle (leakage-safe), **citation parity** (both arms cite → detectability becomes a citation-ACCURACY test), independent **investigator** (neutral primitives, NEVER prism), dual blind judges with **measured family bias**, reset-to-same-frame chain, GO/NO-GO gate. Value-spike NOT academic; ablation deferred.
- `docs/superpowers/specs/2026-06-24-tier-c-phase1d-lsp-matrix-and-runstore-design.md` (rev-2): the LSP 2×2 + run-store/replay. Replay is **frozen-control** (only prism changes; assert model/prompt/evaluator/seed identity, re-score all with current evaluator).

## `eval/tier_c/` module map
`model`(Variant{model,prism,lsp}/id, Issue, ArmOutput, Citation) · `corpus`(Goldilocks load) · `citations`(parse file:line[:sym]) · `checkout`(git worktree at SHA) · `interfaces`(ArmRunner/RelevanceJudge/RankJudge Protocols) · `investigator`(citation precision/recall, prism-FREE) · `planted`(salt+catch+sanitation; INERT in the run, corpus has no plants) · `judges`(borda_consensus(seed), family_bias, detectability_pvalue) · `prompts`(citation-parity stage prompts) · `parse`(claude json / codex jsonl → ModelResult+commands) · `llm`(`live_ask` + `MODEL_CLI`/`cli_model_flag` — SINGLE SOURCE OF TRUTH for CLI model flags) · `judges_live`(LLM judges behind `ask`) · `arm_runner`(ClaudeRunner/CodexRunner/RoutingArmRunner/FakeArmRunner; `_prism_mcp_bin`, `lsp_deny_dir`) · `lspshim`(`make_lsp_deny_shim`, `DENIED`, `LAUNCHERS`) · `classify`(lsp_leak/compiler_assisted) · `detect`(`run_detectability` pooled) · `chain`(run_stage + run_spec_plan_chain, reset-to-cleaned-best, records seeds) · `report`(Cell2x2, assemble_cell_2x2, prism_delta, gate_decision) · `store`(RunStore) · `run`(run_issue, run_live, LiveComponents, Report) · `cli`(`tier-c run --live --run-id … --bench-root … [--force-new]`, `--list`). Tests: `eval/tests/test_tc_*.py` (89 tier_c / 231 full eval green).

## DURABLE gotchas (cost real debugging this session)
- **claude rejects `--model opus-4.8`** (exit 1, EMPTY stderr) — needs alias `opus`. ALL arms + judges map via `cli_model_flag()` (`llm.py MODEL_CLI`). Adding a model? add it to MODEL_CLI.
- **prism-mcp is NOT on PATH** (built at `target/release/prism-mcp`); `_prism_mcp_bin()` resolves it ($PRISM_MCP_BIN→PATH→repo build). prism-ON arms launch it; **rebuild it from current main before any real run** (the binary can be stale even when main moved).
- **LSP control = deny-shim PATH** (`lspshim`): lsp-off arms get a temp dir of failing stubs for `DENIED` (incl. launchers npx/uvx/mise/**pnpm/yarn**). `classify._LSP` excludes `LAUNCHERS` (shared constant — they drifted once → pnpm/yarn false-positive lsp_leak). Compilers (cargo check/go vet/tsc) intentionally NOT denied → `none*` = "no dedicated LSP"; `compiler_assisted` flag + per-protocol view handle the leak.
- **run-store single-owner**: the cli builds the manifest + `RunStore`; `run_live` REUSES `comps.store` (do not re-create — that clobbered the manifest to `{}`). prism build-id = sha256 of the binary (prism-mcp has no `--version`).
- **detectability MUST be pooled** across issues×stages (single-stage n=4 maxes at p=0.0625 > 0.05 → can never fire). `run_live` pools all outputs into one `run_detectability`.
- codex `--json` JSONL field names (command_execution, agent_message, usage) + `num_turns-1` claude tool-proxy are best-effort — verified working in the smokes; claude full per-command (stream-json) is a deferred follow-up (`parse_claude_json` sets `commands=[]`).
- ENV: this shell's gh needs `/opt/homebrew/bin/gh`; **zsh does NOT word-split unquoted vars** (use `${=var}` or explicit args); `cargo test | tail` masks exit (no pipefail) — capture real `$?`; **shared-tree untracked-doc contamination** on rebase (move colliding untracked aside: `git ls-files --others` ∩ `git cat-file -e origin/main:$f`, then rebase); `--quick` tier-a forces `names=[prism]` (Rust only).

## Standing constraints (carry forward)
- Commit trailer: `Co-Authored-By: Claude Opus 4.8 (1M context) <noreply@anthropic.com>`. PR body ends `🤖 Generated with [Claude Code](https://claude.com/claude-code)`.
- `git add <explicit paths>` — NEVER `-a`. **NEVER stage** `eval/snapshots/`, `docs/eval/`, or `eval/tier_c/runs/`.
- Workflow: brainstorm→spec→**codex gpt-5.5 xhigh review** (a2a/`codex exec -m gpt-5.5 -c model_reasoning_effort=xhigh -s read-only -o <file> - < prompt`)→plan→subagent-driven TDD (fresh impl per group + spec+quality review)→merge on green. Owner-approved: submit PRs after reviews settle, merge on green CI. Rust/Go byte-identical = enough (skip `--quick` for non-nav work). Verify codex's findings before folding.
- Live runs cost real spend — owner-triggered only.

## NEXT STEPS (run synthesized 2026-06-24 — the objective is broken; fix it before any more spend)
1. **P0 #1 — FORCE + verify prism usage** (Headline #1: NO arm invoked prism this run → the contrast was empty). Make prism reachable + used: steer the stage prompt to the nav tool and/or make prism non-deferred (claude put it behind `ToolSearch`); pre-warm + **ASSERT** the MCP handshake before each prism-on arm; replace the `used_prism = variant.prism AND tool_calls>0` heuristic (`arm_runner`) with **real prism-call detection** (`mcp__…nav_*`); **DISCARD/retry any prism-on arm with 0 prism calls**. Re-validate with a cheap 4-arm trivial-prompt probe (the Phase-1c smoke pattern) before re-spending. **Nothing below can measure prism until prism-on arms actually call it.**
2. **P0 #2 — fix the relevance oracle** (spec drafted: `docs/superpowers/specs/2026-06-24-tier-c-investigator-relevance-oracle-fix-design.md`). Thread the cited code (line + context window) into `is_relevant`; without it `precision≡0` and **no GO/NO-GO is meaningful**. Subagent-TDD + codex `gpt-5.5` xhigh. Then a cheap **re-score-only** pass on the *recovered* specs (`runs/full-2026-06-24/recovered/`) for the first real grounding numbers — no model-arm re-run.
3. **Add the prism-blind RANK-judge consensus as a co-primary objective** (`report.py` gate currently precision-only). It was the only channel that carried signal (favored `opus+prism` on plan/ts). Decide: max(precision_delta, rank_delta) or a blend; keep detectability guarding the judge channel.
4. **Per-issue namespacing** `stages/<language>/<stage>/` (fold into Phase-1d-replay) so a run is auditable arm-by-arm without the `recover_sessions.py` spelunk.
5. **Scale to ≥2 issues/lang** (recorded 2nd picks: tokio#8182, prometheus#18972, mypy#21583, excalidraw#11313) so each cell pools >1 — paired-per-issue contrasts needed at N>1. **Drop the 2 LSP arms from spec/plan** (inert here → 8→4 variants, halves cost); reintroduce LSP in Phase-2 develop/review.
6. **Then the owner decides** among: **Phase-1d-replay** (frozen-control re-score engine, spec §5); **Phase 2** (develop+review + per-repo build sandboxes — where LSP/compiler signal and a real "code" artifact finally exist); wiring the **cost/analyze-failure gate arms** (`cost_ok=True`/`analyze_failure_rate=0.0` → non-functional); populating the **planted-error** taxonomy (probe built, corpus has no plants → inert); **claude full per-command** logging (stream-json).
