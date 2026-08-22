# Tier-C Part-D — structural blast-radius read-out (codex gpt-5.5 slate, 2026-08-21)

Companion to `docs/superpowers/specs/2026-07-05-tier-c-part-d-structural-tasks-design.md` (design-of-record, §7
pre-registered read-out) and `docs/superpowers/handoffs/2026-07-06-tier-c-part-d-run-handoff.md`. Run root:
`eval/tier_c/runs/partd/full-gpt-5.5-2026-08-21/` (local, git-ignored); the voided first attempt is kept beside it as
`…-VOID-relcache/`. Prism SUT: `main` @ 47e21ae (pre-#169/#170/#171), matched `prism`+`prism-mcp` release build.
Model: `gpt-5.5` (owner's choice after `gpt-5.3-codex` was found unavailable under ChatGPT-auth codex; `gpt-5.3-codex-spark`
exists and was smoke-verified). Codex CLI 0.147.0.

## TL;DR
On the 11-task corpus, **prism-on did not improve grep-hard D-recall over codex-off**: headline mean ΔdR **−0.059**
over the 9 administered cells, **+0.018** over the 3 discriminating cells (where off < 1.0), **median ΔdR = 0.0**;
6/9 tasks are **off-arm saturated** (codex with grep + reading already recovers every grep-hard D-site); mean Δfile-F1
−0.023. Against the pre-registered criteria (VALIDATED needs median ΔdR ≥ +0.2 and Δfile-F1 > 0 in ≥ 2/3 tasks) this is
**REFUTED — "grep suffices" on this corpus/model**, with two caveats below (TypeScript unmeasured; INSTRUMENT-FAIL
check not yet audited).

## Per-task
| task | lang / role | dR off | dR on | ΔdR | Δfile-F1 | dose | phantom on | note |
|---|---|---:|---:|---:|---:|---:|---:|---|
| django-check-registry-run-checks | Py headline | 0.980 | 1.000 | +0.020 | −0.119 | 5 | 0 | discriminating |
| hugo-converter-convert | Go strong | 0.857 | 1.000 | +0.143 | +0.000 | 8 | 6 | discriminating; 6 phantoms on |
| ruff-typechecker-match-annotation | Rust strong | 0.964 | 0.857 | −0.107 | +0.000 | 5 | 1 | discriminating |
| caddy-requestmatcher-migration | Go archetype-B | 1.000 | 1.000 | 0 | 0 | 7 | 1 | off-saturated |
| guava-equivalence-doequivalent | Java strong | 1.000 | 0.640 | −0.360 | +0.186 | 18 | 0 | off-saturated; on loses recall |
| mypy-meet-types | Py secondary | 1.000 | 1.000 | 0 | −0.152 | 23 | 2 | off-saturated |
| prometheus-matchstring | Go weak | 1.000 | 0.778 | −0.222 | −0.028 | 25 | 0 | off-saturated; on loses recall |
| prometheus-promql-walk | Go strong | 1.000 | 1.000 | 0 | −0.095 | 13 | 1 | off-saturated (as on 07-06) |
| ruff-imported-qualified-name | Rust precision | 1.000 | 1.000 | 0 | 0 | 15 | 1 | off-saturated |
| typescript-resolve-alias | TS weak | 0.765 | 0.824 | +0.059 | +0.226 | **0** | 1 | **not administered** (see §Caveats) |
| typescript-resolve-signature | TS strong | 0.857 | 0.857 | 0 | 0 | **0** | 1 | **not administered** |

Cost (tokens in/out, tool calls, wall): see `partd-cost` table in the ledger; prism-on typically spent 1.0–2.7× the input
tokens of off (e.g. ruff-qualified 1.18M → 3.16M; hugo 1.00M → 2.44M) with 5–25 prism calls per cell; wall times similar.

## Caveats (pre-registered alternatives)
1. **TypeScript unmeasured (instrument).** Both TS cells failed the 15 s warm gate on the first pass and, on the
   240 s-gate rerun, passed the gate (17–19 s warm load of a 2.2 GB CPG cache) yet ran **0-dose**: the codex transcripts
   say "prism tools are not exposed in this session" (retry included). Controlled probes with a deliberately 20 s-slow
   server show **codex 0.147 silently drops any MCP server not ready within ~10 s, regardless of `startup_timeout_sec`
   (600) or `startup_timeout_ms`** — the same mechanism that voided the first slate (relative `--cache-dir` → cold
   build at startup). Every administered cell initialized in ≤ 2.4 s. Product follow-up: **prism-mcp should answer the
   MCP handshake immediately and load/build the index lazily on first tool call** (queued as a roadmap item).
2. **INSTRUMENT-FAIL alternative not yet audited.** The read-out is REFUTED only if the saturated off arms genuinely
   searched structurally; the D-subset was built grep-hard by construction, so off-arm recall at 1.0 means codex-off
   reasoned through wrappers/dispatch by reading code. A sample audit of the saturated off-arm transcripts (commands +
   cited sites) is the remaining check before treating this as the final fork-B verdict.
3. One model (gpt-5.5) only; the claude (opus) slate was not run.
4. Harness: the first slate was voided by a relative `--run-store-root` (fixed: `tier_c/partd.py::resolve_run_paths`,
   absolute agent MCP paths, warm gate spawned from the checkout cwd). The warm gate's 15 s default is accidentally close
   to codex's effective ~10 s limit; a longer gate (240 s) admits servers codex then drops — keep ≤ 10 s until the lazy
   handshake ships, then gate on handshake time + first-call time separately.

## What this means for the roadmap fork
Part-C (debug-fix) was near-parity; Part-D (structural blast radius) is REFUTED on this corpus with gpt-5.5: codex-off
saturates the grep-hard sites by reading. Navigation accuracy work (forks A/C) should not be justified by end-task ΔdR
on these tasks. The measurable prism edge remains grounding/resolvability (Part-C) and token cost goes the wrong way.
The review-path items shipped today (#169 sanitizer language gate, #170 `--review-no-diagrams`, #171 multi-line
arg edges) and the queued precision items (P8 param slots + Level-3 identity, P9 pointer embeds, P10 owner partitions)
target correctness/noise of the review surface directly rather than nav ΔdR.
