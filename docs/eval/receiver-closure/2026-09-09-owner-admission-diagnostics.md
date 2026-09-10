# Bounded owner admission diagnostics

Implementation/test checkpoint `75d6e746`, based on PR299 merge `3e1306a3`.
The opt-in owner path now explains acquisition refusals through CLI/API, eager
MCP startup and served MCP errors. This is diagnostic plumbing, not broader
owner resolution or new real-receiver recall.
The [machine receipt](2026-09-09-owner-admission-diagnostics.json) records exact
binary identities, gate totals, report observations and quick discrepancies.

## Contract and measured value

`prism.owner-admission/1` reports the first failed phase, prior completion and
downstream `not_reached`. Fixed aggregate observations describe the existing
loaded census, not the complete filesystem or successful proof predicates.
`authorizes_runtime_edge` is always false. See the
[usage contract](../../experimental-owner-navigation.md#admission-diagnostics).

Reports contain no source paths, receiver names or compiler diagnostic text.
CLI/API/startup errors retain their category prefix and append JSON. Served MCP
receives the original internal typed report in a separate field: the existing
256-byte untrusted-text clamp remains, with a 2048-byte report cap and explicit
oversize omission. No report is deserialized into proof authority or consulted
by admission. Stale reports clear before reacquisition; failure cannot serve an
old owner session.

| Read-only replay | First failed phase | First refusal | Compiler evidence |
|---|---|---|---|
| Installed pinned Excalidraw | prepare_inputs | input_budget | not_reached |
| Approved private frontend | select_inputs | owner_requires_js_ts_only | not_reached |

For each root, three CLI refusals and one eager MCP refusal have identical
reports and empty stdout. Excalidraw reports 628 loaded JS/TS files and
6,928,124 bytes: count exceeds 512, bytes are below 8,388,608. The private report
also observes a count excess, but its language gate ran first. Only its coarse
disposition is published. Before/after tracked content, status and non-following
filesystem metadata matched. No installs, scripts, config edits or source pruning.

Both real roots still stop before compiler acquisition. Receiver eligibility is
unmeasured, not zero recall. Compiler-evidence failure elsewhere is an aggregate
phase, not evidence that every inner closure gate or compiler process ran.

## Verification

Full clean-HEAD gate runner, zero failures/filtered tests:

| Suite | Passed | Ignored/skipped |
|---|---:|---:|
| Rust default | 4,037 | 1 |
| Rust MCP | 4,230 | 1 |
| Rust MCP + detached-owner-audit | 4,253 | 1 |
| Callable observers | 694 | 0 |
| Receiver helpers | 18 | 0 |
| Authority profiles | 40 | 0 |
| Python eval/tests | 885 | 0 |
| Python adoption/tests/unit | 44 | 0 |

The ignored Rust case is `resolution_test::slice_elem_variant_reserved` in each
run. Format and whitespace checks pass. Clippy completed with warnings; this is
not a warning-free claim or an attribution of those warnings to this change.
Precommit release matrix: 159 ok. Same-environment baseline/candidate replay:
36/36 raw navigation pairs and 159/159 complete matrix records identical.
The inherited oracle inventory selects those requests; equality is compatibility
evidence, not a new accuracy baseline. Preserved baseline binaries identify
`0807d7de`; executable source is identical to merged `3e1306a3`. Candidate parity
uses binaries built from clean `75d6e746`; real-root replay used source-identical
precommit binaries. The receipt records both identities.

Initial nonexistent test-target and baseline freshness-check failures were
inadmissible setup probes, retained alongside corrections. Corrected behavioral
RED captured three API/CLI report-absence failures, then 3/3 GREEN. A later
served-MCP RED captured truncated JSON; typed bounded report transport fixes it.
Tests cover exact/overflow count and byte boundaries, simultaneous language/count
failures, empty/load/parse/compiler failures, phase ordering, absence of source
leakage, wire caps, stale-slot/report invalidation and restoration. Existing
positive ownership, closure, duplicate/write and cross-epoch controls remain green.

Prism structural navigation returned SymbolNotFound for the requested private
consumer. Direct source inspection and compiler/process tests were the fallback;
that tool result did not establish a no-callers claim. Two self-review rounds,
SELF-PASS, not independent review. The candidate MCP truncation WRONG was closed
in place; no open correctness findings at the cap.

### Tier-A quick is not a valid accuracy anchor

Fresh release rebuild immediately preceded quick on clean `75d6e746`. Exit2,
oracle errors6/30 (20%, above10% floor), SUT errors0; corpus differs from pinned
`20c8490591a3`. C-method has4/6 and C-name2/6 successful probes. Matrix159ok;
`target-c-method` is a flip_candidate (5TP/0FP/0FN), module-deps-feature-gated
reports missing (17TP/1FP/0FN; Prism-only src/mcp/tools.rs:230), and
load-repo-feature-gated reports missing (115TP/9FP/3FN). Ambiguous-symbol contract
is ok. Inventory matched8,227 with29 missing;23 differences remain pending;
no M3 spot checks executed. Exact site lists are in the machine receipt.
These current quick discrepancies are not newly adjudicated regressions or fixes.
Committed Tier-A baselines remain unchanged; raw reports and snapshot are archived
privately. Full multi-corpus runs were not requested or executed.

## Verification execution incident

An accidentally unscoped `pytest -q` collected the optional live-agent adoption
suite and launched Claude Sonnet trials. It was stopped; 18 completed cached
trajectories were inspected and preserved privately. Recorded tools were read-only
and the tracked checkout remained clean, but actual model activity occurred and
cost is unknown. The interrupted run emitted `FF` without a complete report; it
is neither a passing suite nor an attributed Prism regression. No live base
control was run. The generated credential copy was removed after fresh process
and open-file checks; original credentials were untouched and never archived.

Corrected verification explicitly selected `pytest -q tests` and
`pytest -q adoption/tests/unit`. The first sandboxed unit invocation failed in a
pytest plugin's localhost bind before tests; its permitted host rerun passed.
Do not use unscoped pytest here. Follow-up safety hardening should make live model
tests explicit opt-in before another broad Python verification workflow.

## Next decision

Use the admission facts to assess a genuine complete project boundary and
acquisition feasibility. Do not raise the count limit alone, filter out inconvenient
files, switch dependency/link profiles or admit incomplete closure. Packaging and
session reuse remain separate work; reuse needs freshness/epoch proof. The
unresolved react-scripts disposition and no React.FC expansion remain unchanged.
