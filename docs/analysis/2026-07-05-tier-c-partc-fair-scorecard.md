# Tier-C Part-C — the fair scorecard (prism value on debug-and-fix)

Date: 2026-07-05. Companion to the Part-C design + the scorecard-v2 rubric (PR #167) +
the Part-D design-of-record. This records the **corrected** measurement of prism's value
on debug-and-fix tasks, after a citation-resolution artifact was found and fixed.

## TL;DR

On debug-and-fix, **prism grounds the answer (resolvable file paths) but does not change
it, improve its accuracy, or improve claim-validity — and it costs more.** An earlier
pro-prism "flip" was substantially a **citation-resolution artifact**, not real value.
This is the evidence that pushed the measurement to **Part-D** (structural analyze/refactor
tasks, where the enumerated answer itself should change).

## The artifact (why the first v2 rescore was wrong)

The Part-C citation scorer resolved a citation's file by basename. When an arm cited a
**bare filename whose basename is non-unique** (e.g. `noqa.rs:1014`, and the repo has 6
`noqa.rs` files), the resolver returned "unresolved" and the scorer counted it as a
**hallucination**. The grep/off arm cites real code with bare filenames; the prism/on arm
cites full paths (prism nav hands it resolvable paths). So the off arm was penalized for
*real* citations, manufacturing a prism win.

Fixed in PR #167 (resolver R1–R5): a layered resolver (exact → unique-basename →
line-range → symbol → token → haiku-disambiguator → `AMBIGUOUS`/`ABSENT`), a three-way
classification (valid / hallucinated / **ambiguous**, ambiguous excluded from precision),
and a separate **resolvability** axis so path-formatting is never conflated with truth.

## Mechanical rescore (no-LLM, resolution axis isolated) — both pilot cells

| cell | arm | precision | valid/halluc/ambiguous | full-path resolvability | recall |
|---|---|---|---|---|---|
| ruff:opus-4.8 | off | **1.000** (was 0.050) | 19/0/1 | 0.05 | 0.333 |
| ruff:opus-4.8 | on | 1.000 | 14/0/0 | 1.00 | 0.292 |
| pydantic:gpt-5.5 | off | **1.000** | 40/0/0 | 0.50 | 0.494 |
| pydantic:gpt-5.5 | on | 1.000 | 32/0/0 | 0.50 | 0.640 |

**Zero true fabrications in any arm.** The buggy resolver's "19 hallucinated" (ruff) and
"31 hallucinated" (pydantic gpt) citations were *all real* bare-filename citations. Prism's
resolvability edge is **corpus-dependent** — large on ruff (0.05→1.00), *absent* on
pydantic (0.50=0.50).

## Fair judged scorecard — ruff:spec:opus-4.8 (the flagship 19/20 cell)

Rescored with the fixed resolver + batched judging (one call/arm/dimension):

| dimension | off (grep) | on (prism) | reading |
|---|---|---|---|
| precision (relevance-adjusted) | 0.600 | 0.643 | near-parity; **0 hallucinations both** |
| D1 citation validity | **0.955** | 0.947 | **near-parity** — the v2 "off 0.000 vs on 0.842" was entirely the artifact |
| D0 recall (claim-coverage) | 0.211 | 0.188 | wash |
| D3 annotated head-to-head | — | nominal "on" | now driven by **1** contradicted-tag (vs the v2 flip's **19 fake** "does-not-exist" tags) → marginal at n=1; the dramatic flip **reverted** |
| R4 resolvability | 5% full-path | **100%** | +95% — prism's real *and only* edge here |
| cost | $0.71 | $0.79 (+12%) | $/valid-cite off $0.059 vs on **$0.088** |

Blind head-to-head = tie (both arms pin the same root cause + same fix).

## Cost (on vs off)

prism-on uses **2–4× input tokens** on the gpt cells (nav output + tool loop) and **~+12%
dollars** on the opus cell; $/valid-citation is higher on. On debug-fix the ROI is
negative-to-break-even.

## Conclusion → Part-D

Debug-and-fix is localizable (grep + read one function suffices), so prism's
structural-navigation surface (repo-map, callers/callees, ego, module-deps = architecture,
dependency, blast-radius reasoning) is never exercised. **Part-D** poses structural
analyze/refactor tasks where the enumerated impact set *is* the answer and grep genuinely
cannot find the name-absent (dispatch/alias/re-export) sites. If prism has end-task value,
that is where it must appear. Pre-registered read-out (can refute the thesis) in the
Part-D design-of-record.
