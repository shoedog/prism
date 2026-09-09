# PR297 Tier-A disposition: review readiness, not a new baseline

[PR297](https://github.com/shoedog/prism/pull/297) is ready for review. The original
quick run remains **baseline_invalid**. No production/harness code, corpus pin,
oracle feature profile, error floor, expected outcome or adjudication changed.
Activation remains disabled; no merge was performed.

## Correction to the initial draft assumption

The approved [harness spec](../../superpowers/specs/2026-06-11-prism-tier-a-accuracy-harness-design.md)
§2.2 makes an invalid run ineligible as an S3/B2 accuracy baseline. Its §2.11 and
AGENTS.md require quick execution and reporting before review; they do not make
every invalid baseline an automatic PR blocker. The initial closeout conflated
these two decisions. Matched controls now support this bounded increment's review
readiness without treating its invalid live-checkout run as a passing baseline.
This is not a waiver, an accuracy-improvement claim, or proof of zero Prism defects.

## Source-backed findings and controls

Base `a892b67ddec0883fc0ba7cfaae7fa33fdfb97eaf`; candidate
`997b39d0a3b13262a933450dabf17e4ce70f7faf`. Both release binaries were freshly built
offline and pass the harness's strict clean-SHA checks. Both evaluate the identical
detached corpus `20c8490591a3`, which satisfies the existing pin without allow-drift.
The evaluation harness and MCP source files are unchanged between these revisions.

| Observation | Disposition and evidence |
|---|---|
| Current-checkout SHA versus historical pin | `run_corpus` selects the current repo while `check_pinned` enforces20c8490591a3. The exact pinned worktree removes drift from the comparison, not from the historical run. |
| Six missing probes in the original report | All belong to `auto_refresh_index`, `publication_hooks`, or `item` under MCP-gated modules. Fresh base and candidate oracle replays each reproduce six immediate `prepareCallHierarchy: no item` errors. Timeouts are not needed to explain this population; the old report discarded exception detail, so its precise transient errors cannot be recovered. |
| `module-deps-feature-gated=missing` | The fixed probe expects `src/mcp/tools.rs:162`; the current source call is at230. On the original pinned source, the expected oracle-missing site162 is present and the control is `ok`. |
| `load-repo-feature-gated=missing` | The fixed probe expects historical bootstrap call `src/mcp/session.rs:28`; current loader/publication paths have moved. On the pinned source, site28 is present and the control is `ok`. New audit-test call sites are feature-gated fixtures, not a loss of production call resolution. |
| `target-c-method=flip_candidate` | Unchanged base/candidate responses. On the pinned corpus, all-confidence output has5TP/23FP/0FN; exact-only has5TP/0FP/0FN. The legacy known-failure threshold checks both P and R≤0.2; R=1 explains the flip label. This is not a newly introduced gain, nor permission to change the expected outcome. |
| Ambiguous-symbol contract | `ok`; both binaries preserve the same safe failure. |

**WRONG attributable to PR297 in this bounded control: 0 demonstrated.**
**SMELL:** the moving quick corpus/historical pin mismatch, syntactic inventory
sampling of cfg-disabled symbols, and omitted per-probe error details complicate
diagnosis. No production fix follows from these observations. Broad all-confidence
false positives on the old corpus are not relabeled correct; they are unchanged
and separate from the exact-tier result and this disabled slice.

## Complete bounded comparison

One fixed oracle inventory and seed42 define the same15 symbols for both binaries.
Every one of the30 caller/callee requests is run for both, including the symbols
for which the oracle cannot answer. Those requests establish **SUT parity only**;
they do not fabricate ground truth for unsupported oracle sites.

- **140 raw response pairs equal:** inventory1, sampled requests30, pinned requests5,
  ordinary matrix adapter requests104. A second decoder checks actual JSON equality,
  not merely the recorded `equal` booleans.
- **159 complete matrix records equal and `ok` on each binary.** The55 DFG cases
  bypass the ordinary SUT adapter, so a separate full-matrix comparison includes
  their actual results. These counts overlap; they are not299 distinct tests.
- Corpus requests use separate cache directories created empty for this task;
  neither binary reads the other's cache. Matrix requests use no-cache. A flag-only
  setup correction retains the same owned per-binary cache lineage.
- A live oracle-only replay of all30 fixed sample probes yields28 answers and two
  no-item errors, both for pinned MCP-gated `output_verbosity`. Applying the current
  quick floor would still reject C-name4/6; no valid new quick baseline is claimed.

The stock cold paired quick attempt was stopped as redundant after these complete
controls. Its partial requests/log are retained, not counted as a completed run or
used to publish new metrics. The prior turn's required quick execution remains the
historical run-and-report evidence; this turn adds complete differential controls.

## Verification, custody and limits

Both complete evaluation suites: **883 passed, two skipped**. The skips explicitly
require a matched real CLI/MCP binary pair for the optional Tier-C live hardening
checks; no new MCP build/activation was performed. The base matrix self-test was
given the freshly built base binary so it has the same runnable coverage.

The first pytest attempt was blocked by its plugin's localhost socket setup. The
repo-root invocation then produced nine fixture-path errors on both revisions;
correct eval-directory invocations passed without code changes. These setup
failures are not product regressions. The first supplemental matrix attempt mixed
mutually exclusive no-cache/cache-dir flags; its archived failure is likewise
inadmissible. Its36 preceding corpus requests passed, and the corrected full
comparison passed. No rejected implementation was discarded or rewritten.

Two diagnostic rounds completed, SELF-PASS (not independent-agent review).
Prism navigation was stale/truncated; LSP navigation tools were unavailable, so
the navigation skills led to direct source checks and the actual harness oracle.
CI run34316205421 passed on candidate997b39d. The previous full Rust/observer/helper
receipts remain valid historical evidence for unchanged executable sources; those
large suites were not rerun in this documentation-only disposition turn.

[Machine-readable receipt](2026-09-09-pr297-tier-a-disposition.json).
Evidence archive `/private/tmp/prism-tier-a-297-dSTyZl-evidence.tgz`, SHA256
`6e3df1e3cae462a68c4c48aaadfda04ad97e5a8737fd92123bb9833605173b75`.
It contains scripts, raw responses, logs, snapshots and both tested binaries;
temporary worktrees and generated caches are excluded. No tracked baseline was
rewritten. The workspace release binary was rebuilt back to candidate997b39d and
its bytes match the saved candidate binary.

Next: review/merge the disabled integration, then separately approve bounded
CLI/MCP opt-in activation and its refresh/failure publication contract. Explicit
corpus selection and persisted oracle-error details are useful harness follow-ups,
not prerequisites obtained by silently changing this run's policy.
