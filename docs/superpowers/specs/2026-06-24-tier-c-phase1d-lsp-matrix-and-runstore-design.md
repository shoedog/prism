# Tier-C Phase-1d — LSP 2×2 matrix + run-artifact store/replay (Design, rev-2)

**Status:** design-of-record **rev-2** (owner-approved decisions 2026-06-24; codex gpt-5.5 xhigh review folded — verdict SHIP-WITH-FIXES, 9 findings incl. 2 BLOCKER, all incorporated). Pre-requisite for the first full multi-language `--live` run.
**Builds on:** the Tier-C harness (spec `2026-06-23-tier-c-value-measurement-design.md`; Phase-1/1b/1c on `origin/main`, live-smoke-passed).

## 1. Why (two pre-run blockers the smoke surfaced)
A live run *now* would be (a) **confounded** — LSP/type-checker CLIs (rust-analyzer, gopls, pyright, basedpyright, tsserver, clangd) are installed and on PATH, so the current `prism-OFF` arm is "grep/read + whatever type-intelligence the agent shells out to," not a clean baseline; and (b) **non-reproducible/non-replayable** — runs print a report and vanish, so we can't re-use a run's spec/plan to cheaply re-run a later stage after a prism enhancement. Phase-1d fixes both.

## 2. LSP as a controlled 2×2 dimension
**Treatment becomes `{LSP off, LSP on} × {prism off, prism on}` per model** → the four cells the owner named: **none (grep/read) · LSP · prism · prism+LSP**. The decision-critical cell is **prism+LSP vs LSP** (the realistic IDE deployment: does prism add value *on top of* an LSP the agent already has).

### 2.1 Variant model
`Variant` gains `lsp: bool` (default `False`, back-compat): `(model, prism, lsp)`. `id = f"{model}{'+prism' if prism}{'+lsp' if lsp}"` (e.g. `opus-4.8+prism+lsp`). `family` unchanged (model-based). The spec→plan chain runs **8 variants** = {opus-4.8, gpt-5.5} × {prism F/T} × {lsp F/T}. (2× the Phase-1c arm count → 2× spend; owner-accepted.)

### 2.2 LSP control mechanism — shim-deny PATH (model-agnostic) + honest naming
**Naming (codex new-3):** the lsp-off cell is **"no dedicated LSP"**, NOT "zero type-intelligence" — see the compiler caveat below. Call the four cells `none*` / `lsp` / `prism` / `prism+lsp`, where `none*` = "grep/read, no dedicated LSP server/type-checker (compiler still on PATH)".

PATH-dir stripping is **not viable** (LSP binaries share dirs with essential tools: `cargo` in `~/.cargo/bin`, `node` in the mise node dir, everything in `/usr/bin`). Instead, for **`lsp=False`** arms, prepend a **deny-shim dir** to `PATH`: a temp dir of executable stubs named for each type-intelligence binary that print `"<tool> disabled (Tier-C lsp=off)"` to stderr, **log the attempt** to `shim-log.jsonl`, and exit non-zero. Real binaries shadowed; `cargo`/`node`/`git` untouched. `lsp=True` arms run with normal PATH.
- **Denied (configurable):** `rust-analyzer, gopls, pyright, pyright-langserver, basedpyright, pylsp, ruff-lsp, typescript-language-server, tsserver, tsc, clangd, mypy` — AND the common **launchers** that bypass bare-name shims (codex new-4): shim `npx`, `pnpm`, `yarn`(dlx), `uvx`, `mise`(`x`/`exec`), and a `python`/`python3` wrapper that denies `-m {pyright,mypy,...}` while passing other `python` calls through. Absolute-path invocation can still bypass (low likelihood) — the command log (§3) **flags any lsp-off arm whose recorded commands reached a denied tool by any route**, and such arms are marked `lsp_leak=true` and excluded from the clean-baseline contrast.
- **Compiler caveat (codex new-3):** `cargo check`/`go vet`/`go build`/`rustc` give type diagnostics (LSP-like) but stay available in BOTH conditions (they're build tools, needed for develop in Phase 2). So `none*` is "no dedicated LSP", not "no type-intelligence". **Per-protocol handling:** the command log classifies each arm's tool usage; arms in an lsp-off cell that invoked a compiler type-check are flagged `compiler_assisted=true` and reported **both** in-cell and in a per-protocol view that excludes them, so the "prism vs grep/read" claim is auditable.
- **Symmetry (codex new-5):** the **shim is the sole, symmetric enforcement** across claude AND codex (env PATH). We **drop** the claude-only `--disallowedTools` (it made the two CLIs asymmetric). Command logging is normalized across both (§3). Cross-model deltas remain confounded by family-bias + harness regardless (per the base spec) — **only within-model prism/LSP deltas are trusted**; cross-model is reported caveated.

### 2.3 Report rework for the 2×2 (codex new-6)
Effects are **paired per-issue contrasts** (compute the delta within each issue, then aggregate across issues — NOT a difference of pooled means, which is a difference-of-differences trap). `assemble_cell` reports, **per model**, the full set:
- **prism @ LSP-off:** `{m}+prism` − `{m}` (prism vs grep/read).
- **prism @ LSP-on:** `{m}+prism+lsp` − `{m}+lsp` (prism *on top of* LSP — **the primary gate**, deployment-realistic).
- **LSP @ prism-off:** `{m}+lsp` − `{m}`.
- **LSP @ prism-on:** `{m}+prism+lsp` − `{m}+prism`.
- **interaction:** `({m}+prism+lsp − {m}+lsp) − ({m}+prism − {m})` (does prism's value depend on whether an LSP is present).
The GO/NO-GO gate keys primarily on **prism @ LSP-on**, with prism @ LSP-off alongside. Pooled detectability is reported but is **not** a substitute for these cell-specific effects. ITT/per-protocol + family-bias unchanged. Detectability pooled over 8 variants × stages × issues.

## 3. Per-command logging (turns "what did the arm use" observable)
Capture, per arm output, the **list of commands/tools the arm invoked**:
- **codex** `--json` JSONL already emits `command_execution` items → parse the command strings (extend `parse.py`).
- **claude**: switch the arm call to `--output-format stream-json` (emits `tool_use` events with inputs) and parse the tool commands; if that proves heavy, fall back to the **deny-shim log** (catches denied-tool attempts) + the existing `tool_calls` count (claude per-command full detail = a noted follow-up).
- **Plus** the deny-shim invocation log (§2.2) — directly verifies the lsp-off control held (no successful LSP calls) and catches attempts.
`ArmOutput` gains `commands: list[str]` (default `[]`). Stored per variant; surfaced in the run artifacts (§4). Used to **classify** each arm's actual tool usage (prism / LSP / compiler / grep-read) post-hoc — the audit that makes a confounded result detectable.

## 4. Run-artifact store
Each run writes `eval/tier_c/runs/<run-id>/` (**gitignored** — relocatable/committable later; `<run-id>` supplied via CLI, §7). The store MUST be sufficient for **deterministic replay**, not just audit (codex new-2):
- `manifest.json` — models + **model params** (temp etc.), the 8 tool-conditions, **per-variant prism build SHA**, **harness git SHA**, **CLI versions** (`claude --version`, `codex --version`), **env/PATH + shim config** (denied list, shim dir), bench-root, **run timestamps**, **parent run-id + replay command** (if a replay, §7), and the **corpus snapshot with clean/dirty status + content hashes** of each issue payload (issue text) and each repo's pinned state (codex new-8).
- `stages/<stage>/prompt.json` — the **exact rendered stage prompt** + the **upstream frame** fed to all variants (required for faithful replay — codex new-1).
- `stages/<stage>/seeds.json` — the **blind-shuffle seed + the label→variant anonymization map + the tie-break seed** for that stage (so replay's judging is reproducible, not re-randomized — codex new-2).
- `stages/<stage>/<variant-id>.json` — full output: text, citations, tokens, tool_calls, **commands**, used_prism, `lsp_leak`/`compiler_assisted` flags, wall_s, **raw runner transcript** (the JSONL/stream-json), and **the prism build SHA used by that variant**.
- `stages/<stage>/judges.json` — each judge's full ranking + **judge model IDs/params + the exact judge prompt** + consensus + owner adjudication.
- `stages/<stage>/investigator.json` — per-variant citation verdicts + **the investigator/relevance prompt + version**.
- `stages/<stage>/best.json` — carried cleaned-best (id + cleaned text) + provenance.
- `detectability.json`, `report.json` — pooled detectability + assembled cells.
- `shim-log.jsonl` — deny-shim attempts.
All JSON, deterministic, diff-able. A run is fully **reconstructable, auditable, and replayable** from this dir.

## 5. Replay — per-stage AND per-variant (FROZEN-CONTROL semantics)
`tier-c replay <run-id> --from-stage <stage> [--variants <id,...>] --out-run-id <new>`. **It is a frozen-control replay, NOT a fresh experiment** (codex new-1): the ONLY thing allowed to differ from the source run is the **prism build** behind the re-run variants; everything else (rendered prompt, upstream frame, models + params, judge/investigator prompts + versions, seeds, tool-conditions) is **reused from the source run and asserted-identical**.
- **Compatibility gate (codex new-1):** before reusing anything, replay **asserts** the current CLI versions / model IDs / judge+investigator prompt+version / harness evaluator version **match the source manifest**. On mismatch it **refuses** (or requires `--allow-drift`, which forces a *fresh* full re-run of the stage, not a partial one) — so a partial replay can never silently confound prism-change with environment drift.
- **What's reused vs fresh:** the saved **upstream frame + rendered prompt + seeds** are reused (so judging order/anonymization is reproducible — codex new-2). The **arm OUTPUTS** of the non-re-run variants are reused (the expensive model calls). The named variants' arms are **re-run** against the same upstream+prompt with the new prism build.
- **Re-evaluation (codex new-7):** to avoid mixed-generation scoring, the investigator AND judges are **re-run over ALL variants** (fresh + reused outputs) with the **current** evaluator — we reuse expensive arm OUTPUTS, but never reuse stale verdicts. Detectability recomputed over the merged pool.
- Writes a **new** `<out-run-id>` (never mutates the source); the manifest records **parent run-id, the replay command, and per-variant `provenance: fresh|reused`**; the report **labels partial-replay cells distinctly** from fresh-full-run cells.
- **Owner's use case:** "good lift everywhere except develop → enhance prism → `replay <run> --from-stage develop --variants <the four prism-bearing develop ids>`" — re-runs only the prism develop arms against the same saved plan; no re-spend on spec/plan or the baseline/LSP-only arms. (Develop is Phase 2; replay is built spec→plan-capable now, forward-compatible.)
- **Chain extension:** `run_stage` gains a seed mode — `(upstream_frame, prompt, seeds, reused_outputs: dict[vid, ArmOutput], rerun_variants: list[Variant])`: run only `rerun_variants`, merge `reused_outputs`, then **re-score (investigator + judges) over the union with current evaluators** using the reused seeds. Fresh end-to-end = `rerun_variants = all, reused = {}`.

## 6. Scope / non-goals
- **In:** the 2×2 LSP variant + shim-deny control + claude `--disallowedTools`; per-command logging (codex full; claude via stream-json or shim-log fallback); the run-artifact store (gitignored); per-stage+per-variant replay; the 2×2 report rework; `.gitignore` entry.
- **Out (unchanged):** develop+review stages + per-repo build sandboxes (Phase 2 — store/replay are forward-compatible); the non-functional cost/analyze-failure gate arms (still need prism-analyze-failure + cost tracking); relevance audit-sampling cadence.
- **Cost note:** the full run is now 8 variants × 2 stages × N issues + judges — ~2× the 4-variant estimate. Replay makes subsequent prism-enhancement re-runs cheap (only the prism variants of one stage).

## 7. Run-id, determinism, residual risks
- **Run-id (codex new-9):** `--run-id` supplied at the CLI (the shell stamps it `date`-derived; no in-process clock). Replay **rejects an existing run-id** unless `--force-new`, and the child manifest stores `parent_run_id` + the exact replay command (lineage).
- **Determinism:** all seeds (blind-shuffle, anonymization map, tie-break) are persisted per stage (§4) and reused on replay (§5) so re-judging is reproducible, not re-randomized.
- **Residual:** claude `stream-json` per-command parsing may be heavier than expected → fallback is shim-log + `tool_calls` count + codex-style best-effort (claude full per-command = follow-up). Deny-shim bypass via absolute path is possible but the command log flags it (`lsp_leak`, §2.2). The compiler-as-type-intelligence leak is bounded by the `compiler_assisted` flag + per-protocol reporting (§2.2), not eliminated — `none*` is honestly "no dedicated LSP".
