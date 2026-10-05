# Tier-A quick repair readout

**Superseded by R1:** the table below is historical worker evidence. Both reviewers
reproduced Rust INVALID, and TS incoming calls included related implementations.
Current repairs/results: [R1 readout](r1-readout.md). Historical execution totals
are retained without claiming present reproducibility.

Written 2026-10-05T03:51:24.740379+00:00. Checkout HEAD `4e592daa7858a195eb3a9eb77c83dfbc763b49fa`, branch
`plan/tier-a-quick-repair`; working changes are uncommitted for the controller.
Execution authority: `/Users/wesleyjinks/prism-evidence/meas/tiera/plan-brief.md`.

Final bounded quick returned **0**, all three corpora **VALID**, in approximately
**162.24s** (log creation to last output; sum of monotonic corpus timings
**162.071s**). Rust 45.152s;
TS 67.867s;
JS 49.052s. All oracle and SUT error
rates were zero; source manifests matched before and after. This is local dirty
SUT verification, not human adoption of a new committed baseline.

## Root causes and repairs

**WRONG:** quick selected the moving checkout against the original fixed corpus
pin, producing deterministic corpus_sha_drift. JS/TS was unsupported and an
explicit quick corpus selection was ignored. Rust's syntax inventory included
an inactive mcp symbol that rust-analyzer could not prepare, leaving C-name 4/6.
The last issue has a same-environment feature-only control: no item versus four
callers at identical source coordinates. Details are in [diagnosis.md](diagnosis.md).

The silent long run also had quadratic whole-inventory matching and repeated
slow SUT commands. Indexed matching preserves the old match semantics. A
workspace SUT cache avoids the protected platform cache. Progress shows stages,
files, probes, and errors. LSP pipe writes, requests, startup, total session,
shutdown and SUT commands are bounded; timeouts retain their explicit outcome,
failed probe and error. Partial inventories are not promoted to snapshots.
ContentModified retries are limited to three attempts within one original query
deadline. Native tsserver supports TS/TSX and JS/JSX, including named callable
initializers; automatic package acquisition is disabled.

## JS/TS measurements

Pinned public Excalidraw `0642e72cfa2d9a71198200e52f37399384610ee3`, tsserver 6.0.3,
seed 42. 590 TS/TSX files and 25 JS/JSX files inventoried; 24 TS directional probes
and 18 JS directional probes. Raw **call-site** P/R below; full per-stratum
function metrics, Wilson intervals, confidence tiers and pending diffs are in
[evidence](evidence/). These are sampled local call-resolution observations,
not direct tests of member data-flow or async semantics. No Excalidraw dependency
install was present; broader project/type-context completeness is not certified.

| Corpus / direction | Raw precision | Raw recall | tp/fp/fn | Exact precision/recall |
|---|---:|---:|---|---|
| excalidraw-ts callers | 0.6429 | 0.5294 | 9/5/8 | 0.8333/0.2941 |
| excalidraw-ts callees | 0.5500 | 0.9167 | 11/9/1 | 1.0000/0.8333 |
| excalidraw-js callers | 1.0000 | 0.6250 | 5/0/3 | 1.0000/0.6250 |
| excalidraw-js callees | 1.0000 | 0.6000 | 3/0/2 | 1.0000/0.6000 |

Function-set totals compare the caller context containing each call site, in
both directions. They do not measure callee target identity. Historical TS callers
7/5/6 tp/fp/fn, TS callees 8/4/0; JS callers 4/0/2, JS callees 3/4/0. Call-site
agreement alone must not be read as target-identity precision.

## Validation and baseline accounting

- Full eval suite after the verification hook: **983 passed, 1 skipped** (explicit
  live-model opt-in exclusion); exact commands and exclusions are in the root
  [VERIFICATION.md](../../../../VERIFICATION.md).
- Repair regression suite: **43 pass** here; **42 fail and 1 passes on the bound
  original harness**. The passing original case is the unchanged-source positive
  control. Coverage includes blocked pipe writes, malformed framing, native
  protocol/definition mapping, source bookends after cleanup, empty inventories,
  invalid CLI limits and aggregate edge cases. The original malformed-frame
  reader also emits one thread-exception warning. Bounded test cleanup is retained.
- Matrix-only command: **178/178 ok**. Final quick embeds the same 178 ok results;
  regression list `[]`, matrix flip-candidate list `[]`.
- Full Rust all-feature, all-target run: **5139 passed, 23 failed, 1 ignored**;
  the 23 require an absent explicit pinned compiler. All were enumerated.
- Node script suite: **121 passed, 31 failed, 1 skipped**, missing explicit
  compiler/example inputs. Script Python mutgate suite: **57 passed, 82 subtests**.
- `git diff --check` passed. No Rust, Node, Cargo, baseline.md or adjudication
  source edits. No Git writes or network acquisition.

Committed baseline changes: **none**. The original 20c849 corpus was recovered,
not replaced; separate quick corpus names keep exploratory results distinct.
Rust oracle configuration now enables mcp and is recorded, so its oracle answers
are not directly comparable to historical default-feature answers. Pin readouts:
`target-c-method` remains a flip_candidate. `module-deps-feature-gated` and
`load-repo-feature-gated` report missing historical oracle-miss sites because
mcp now resolves them; both changes are disclosed rather than updating their
old expectations. `ambiguous-symbol-contract` remains ok. These configuration
flips are not evidence of SUT regressions; no comparable old JS/TS run exists.
Every raw pending difference remains in the evidence JSON for owner review.

## Files and suggested controller commits

`fix(eval): make Tier-A quick bounded, pinned, and measurable for JS/TS`

Files: eval/README.md, eval/corpora.toml, eval/tier_a/accounting.py, cli.py,
corpus.py, lsp_client.py, model.py, oracles.py, pinned.py, report.py, strata.py,
sut.py, tsserver.py; eval/tests/test_quick_repair.py and test_sut.py.

`docs(eval): record MEAS-B controls, measurements, and verification limits`

Files: root VERIFICATION.md; this packet (plan, diagnosis, recovery recipe, readout, handoff, verification
limits, evidence); the complete pinned snapshots
`eval/snapshots/prism-quick-20c8490591a3.json`,
`excalidraw-ts-0642e72cfa2d.json`, `excalidraw-js-0642e72cfa2d.json` preserve the
measured sampling pool. Exclude diagnostic `prism-4e592daa7858.json`, target
caches and exploratory runs from the commit. Controller owns committing.

## Open questions and work not verified

[unverified-checks.md](unverified-checks.md) names the compiler-limited tests.
Compiler-worker packaging, freshness-safe reuse and acquisition performance
remain out of scope. No human all-corpus run or baseline adoption, installed
Excalidraw dependencies, alternate server versions/platforms, or live Go/Python
oracle run was performed. No independent agent review was requested; refutation
was a test-backed self-pass. The three-round cap was extended once for the
single enumerable feature error; final validation is green without restarting
or discarding the reviewed artifact.
