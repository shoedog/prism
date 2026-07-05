# Handoff — Tier-C Part-C prism-value measurement, 2026-07-05

Cold-start map to continue the prism end-task value measurement. Durable ledger:
**`.superpowers/sdd/progress.md`** (git-ignored; read the tail first — it has the
blow-by-blow). This handoff is the resume map. Resume with a plain "continue".

## Where the whole effort is

The ranked accuracy plan (P1–P14) is COMPLETE and merged (main). Follow-up #1 (anchor-only
globs) shipped (#166). We then started measuring **whether the accuracy wave moved
end-task value** via the Tier-C Part-C harness (`eval/tier_c/`, already on main, PR #143).
This handoff covers that measurement arc.

## State of main (tip ~08c70cf at write; may have advanced by docs commits)

- Harness hardening MERGED (`cbec5a3`→`da7e4d0`): matched-binary preflight + shared
  `--cache-dir` + codex/claude MCP timeouts (600s) + a per-cell warm-`initialize` GATE
  that FAILS the cell loud instead of a silent 0-dose arm. **This unblocked prism
  engagement** — the prior full run was voided because prism was never invoked (root
  cause: prism-mcp cold-builds the CPG during the MCP handshake, 263s on ruff >
  codex's default 10s startup timeout; matched binaries + prewarm + the timeout bump
  fix it — warm init is <3s).
- Part-D design-of-record COMMITTED (`08c70cf`, `docs/superpowers/specs/2026-07-05-tier-c-
  part-d-structural-tasks-design.md`).
- Harness-hardening report at `docs/superpowers/specs/2026-07-05-tier-c-harness-
  hardening-report.md`.

## The two live branches (NOT merged)

1. **`partc-rubric-v2`** (worktree `/private/tmp/prism-partc-rubric`): the scorecard-v2
   rubric (Fable design) — D0 recall-denominator repair (WRONG bug: `cli.py:683`
   made recall==precision), D1 citation-validity (judged claim-support rate), D2
   relational-fact (mechanical, `depends()` full / `calls` UNKNOWN-stub), D3 fact-
   annotated head-to-head, D4 nav-efficiency. 651 pytest passed. **A resolver-fix
   implementer (Sonnet) is IN FLIGHT on this branch at write time** — see "In flight".
2. Nothing else outstanding.

## THE CRITICAL FINDING (read before trusting any pilot number)

The pilot (5 live cells, ruff/pydantic × opus-4.8/gpt-5.5, spec stage) showed prism
engages (dose 2–8) and the v2 rescore of ruff:spec:opus-4.8 LOOKED like a dramatic
pro-prism flip (off precision 0.000 / 19 "hallucinated"; D1 validity off 0.000 vs on
0.842; D3 annotated-H2H flipped to on). **The owner was rightly suspicious, and it is
substantially a CITATION-RESOLUTION ARTIFACT, not real prism value:**
- Re-resolving the off-arm citations with a candidate-check: **19/20 (95%) are REAL** —
  real, on-topic lines in `crates/ruff_linter/src/noqa.rs`. The off-arm cited real code
  with BARE filenames (`noqa.rs:1014`); the resolver (`checkout.py::resolve_rel`) marks a
  bare filename unresolved when the basename is non-unique (6 `noqa.rs` files; only the
  3023-line one has line 1014), and the scorer counted unresolved as "hallucination."
- The on-arm cites FULL paths **because prism nav output gives it full paths** → 100%
  resolve. That is prism's real edge (resolvable-path formatting) — marginal here.
- ~5% unresolved << the owner's 25% "sufficient negative evidence" threshold. **The
  "grep fabricates / prism grounds" story is DEAD** — competent grep cites real code.
- COST cuts the other way: prism-on uses **2–4× input tokens** (nav output + tool loop;
  gpt cells off 148–605k in / on 580–882k in) and **~+12% $** (opus cell). Debug-fix
  ROI is negative-to-break-even.
- **Net: debug-fix UNDER-samples prism.** Both arms converge on the same root cause +
  same fix (verified in every cell's judge reasons); prism changes grounding, not the
  answer; and even the grounding edge evaporates once resolution is fair. Prism's value
  needs a task family where grep genuinely can't find the answer → **Part-D**.

## In flight at write time (survives via the subagent runner; poll on resume)

**Resolver-fix implementer (Sonnet), worktree `/private/tmp/prism-partc-rubric`, branch
`partc-rubric-v2`.** Spec: `scratchpad/resolver-fix-spec.md`. Implements R1–R5:
- R1 layered resolver (`checkout.py`): exact → unique-basename → ambiguous-but-line-in-
  range-in-exactly-one-candidate (+symbol/token tie-break) → Q3 disambiguator → ABSENT;
  new `resolve_cite(file,line,symbol,claim)` returning `(status, path)`; `resolve_rel`
  kept as a back-compat shim.
- R2 Q3 LLM disambiguator (model **claude-haiku-4-5**, behind the `ask()` seam, ONLY on
  genuine ≥2-candidate ties). (owner chose haiku.)
- R3 three-way classification (`investigator.py`): **valid / hallucinated (ABSENT or
  out-of-range) / ambiguous (real line, unpinnable)**. precision = valid/(valid+halluc);
  **ambiguous EXCLUDED from precision**. recall denom stays `count_claims`.
- R4 new RESOLVABILITY axis (full-path / bare-resolved / ambiguous / absent per arm) —
  prism's honest edge, kept DISTINCT from precision/validity.
- R5 rewire D1 validity + D3 annotation onto the FIXED resolver (bare-but-real resolves
  so the validity judge reads its window; D3 only tags `[CITED LOCATION DOES NOT EXIST]`
  for truly-ABSENT — stops the false-tag that drove the flip).
- Diagnostics fired mid-run (new files partial); those are Pyright path-resolution noise.
- On completion: verify (pytest on the branch; `cd eval && uv run pytest -q
  --ignore=adoption`), then **RE-RESCORE** the pilot with the fixed resolver
  (`scratchpad/cite-reresolve.py` is the quick mechanical check; the full rescore is
  `uv run tier-c rescore --run-dir <BASE>/pilot-0705 --out-run-id <new>
  --run-store-root <worktree>/eval/tier_c/runs/partc` from the worktree eval/, env
  PRISM_BIN/PRISM_MCP_BIN=main's target/release; base-root at
  `/Users/wesleyjinks/code/slicing/eval/tier_c/runs/partc/<dir>` since runs/ is git-
  ignored/local-only). Expect precision to converge near-parity, the annotated-H2H flip
  to revert, and report the TRUE accuracy(validity) + resolvability gaps (the numbers
  worth keeping). DISCARD the earlier artifact-tainted rescores.

## Immediate next steps (in order)

1. Verify the resolver fix + re-rescore the pilot → bring the owner the FAIR scorecard.
2. Merge `partc-rubric-v2` (harness/test infra — no prism source, no review pipeline
   needed; verify by rescore). Move `rubric-v2-report.md` into `docs/superpowers/specs/`.
3. **Build the Part-D first slice** (owner-greenlit; spec = the committed design doc).
   Build order (§8 of the spec): (a) `tier-c build-gold` tooling wrapping `tier_a`
   `LspOracle` + prism candidate merge; adjudicate the 3 tasks' LSP/prism disagreement
   bands SOURCE-VERIFIED (owner caveat §5a: LSP is NOT ground truth — Tier-A `oracle_miss`
   = prism right/LSP wrong; prism-only-real ENTERS gold as prism's CREDIT; phantom only
   on adjudicated-nonexistent). Freeze `gold.json`. Bring the owner the gold sets to
   review. (b) `structural.py` scorer (set math: file-F1, symbol-F1, **D-recall =
   headline**, phantom count), `impact` stage in `prompts.py`, `run-partd` cloning
   `_run_partc_live`. (c) run 12 arms (3 tasks archetype-A: ruff/prometheus[MatchString]
   /excalidraw × 2 models × {on,off}) + ≤3 GO-only sentinel, UNIQUE run-id per cell
   (run-partc is one-cell-per-run-id; `--force-new` CLOBBERS — the pilot loop bug).
   Pre-registered read-out (can REFUTE the thesis) in the spec §7.

## Owner decisions already made (binding)
- Full rubric incl. validity dimension; greenlight Part-D first slice; haiku (or
  codex-5-3 medium) as the Q3 disambiguator; LSP-fallibility MUST be factored (gold =
  adjudicated truth, not LSP output).

## Harness gotchas (durable)
- Every Part-C/D live cell needs matched `prism`+`prism-mcp` from ONE build (skew →
  cross-cache-miss → cold build → warm-gate trips). Rebuild both: `cargo build --release
  --features mcp`. Pin via `--prism-build-dir` / `$PRISM_BIN`.
- `run-partc`/`run-partd` = ONE cell per `--run-id`; UNIQUE run-id per cell; `--force-new`
  rmtree-clobbers.
- Watcher loops must NOT `pgrep -f "<their own launch cmdline>"` — self-match → immortal.
- Bench repos at `~/code/bench-repos/{ruff,prometheus,pydantic,excalidraw}`; the recovered
  prism-OFF baseline (reused) at
  `eval/tier_c/runs/full-2026-06-24/recovered/{opus-4.8,gpt-5.5}` (git-ignored, local).
- Fable was the design consultant (rubric + Part-D); its full proposal is compressed in
  the ledger. codex gpt-5.5 xhigh via a2a-bridge for adversarial reviews.

## Memory
`~/.claude/projects/-Users-wesleyjinks-code-slicing/memory/project_prism_llm_accuracy_plan.md`
+ MEMORY.md line carry the plan-complete + post-plan state; add a Tier-C-value topic at
close-out if this arc concludes.
