# MEAS-B hypothesis / probe / result log

Base: `4e592daa7858a195eb3a9eb77c83dfbc763b49fa`, initially clean branch
`plan/tier-a-quick-repair`. Evidence: `/Users/wesleyjinks/prism-evidence/meas/tiera/`.
No network installs or Git writes. Cap declared before work: three rounds. At
round 3 only a closed MCP configuration issue remained (two directions at one
seed); one extension was disclosed and completed on the existing artifact.

## P1 — startup versus inventory/query delay

Expected: if cumulative LSP inventory latency explained the silence, requests
would be slow after startup; remaining in startup would instead support the
indexing alternative. Probe: base release build, then instrumented quick with
180-second bound and 60-second stack dumps (`base_probe.py`, `base-quick.log`).
Result: startup completed in 20.235s; most document-symbol requests were
milliseconds. Stacks spent tens of seconds in greedy inventory matching, then
in SUT subprocess communication. The base run timed out without a verdict.
The historical one-hour duration was not independently reproduced.

## P2 — moving corpus identity versus oracle floors

Expected: the original pin check must reject actual HEAD independently of oracle
answers if identity drift is a cause. Probe: exact original `check_pinned` against
this checkout (`pin-control.json`). Result: rejects `4e592daa7858` against
`20c8490591a3`. The later fixed-corpus run still failed C-name 4/6, separating the
additional oracle configuration cause from SHA drift.

## P3 — JS/TS support versus another hidden selection mode

Expected: discoverable language extensions, adapter, and selector would falsify
missing support. Probe: original corpus/strata/factory/CLI reads. Result: only
Rust, Go, Python supported; quick hard-codes Prism and ignores corpus selection.
Corrected native tsserver probe (`tsserver-navtree.json`) shows named functions
and callable const initializers at 1-based line/offset coordinates.

## P4 — matching algorithm versus server delay

Expected: scaling original matching should be quadratic; an indexed implementation
must retain exactly the same one-use, tie-break, and anonymous semantics. Probe:
exact original function from Git, compared in the same process/environment
(`matching-control.json`). Result: 1000/2000/4000 original records took
0.173/0.710/2.500s; indexed matching took 0.001/0.002/0.011s with identical matches.
Negative tie/duplicate/anonymous tests independently preserve the contract.

## P5 — repeated cold SUT builds versus oracle delay

Expected: a writable cache should produce cache artifacts and make subsequent
SUT calls faster; oracle-side timing would not explain waits inside subprocess
communication. Probe: source cache directory and error handling plus timed live
runs, then workspace cache (`target/tier-a-nav-cache`). Result: platform cache
is outside permitted roots; cache failures continue into rebuilding. Original
per-call waits were roughly 45–60s; workspace-cached calls roughly 0.6s Rust and
2.5s Excalidraw. The direct platform-cache warning was not retained, so the
cache explanation combines source mechanism and the allowed-root boundary,
not a claimed captured warning. Scheduling/load also changes timings; no exact
speedup attribution is made. The cache artifact and successful reuse are real.

## P6 — transient indexing errors versus permanent unsupported query

Expected: ContentModified can succeed on a fresh bounded attempt; other errors
must remain failures. Probe: round 2 live failures and deterministic transient /
permanent fake-server controls. Result: round 2 received -32801 ContentModified;
unit controls verify at most three attempts sharing one query deadline. Round 3
had no such errors. Settling alone could explain that live disappearance, so
live evidence does not isolate the benefit of retrying. Retried errors and
unrecovered timeouts remain explicit in accounting and reports.

## P7 — inactive Cargo feature versus bad selection position

Expected: the same source symbol/selection must fail without mcp and resolve
with mcp if feature configuration is the cause; failure in both configurations
would support a bad-position alternative. Probe: same corpus, same fd, same
server/environment with only initialize cargo.features changed
(`feature-control.json`). Result: default reports no item for output_verbosity;
['mcp'] reports four callers and zero callees. Source places module mcp behind
#[cfg(feature = "mcp")]. Enabling the feature preserves all six C-name probes;
the final Rust floor is valid, not resampled or lowered.

## Inadmissible probes and corrections

- Python tests from repo root used incorrect fixture-relative paths: reran from eval.
- Shared editable env imported sibling code: bound PYTHONPATH to the exact checkout.
- Initial base export omitted the tracked skill directory: restored it, then used
  a distinct final control log (`base-python-final.log`: 937 passed, 4 skips;
  one extra skip is the base export's absent SUT binary).
- tsserver initially opened a nonexistent path: reran against packages/math/src/point.ts.
- Archive extraction used Python 3.9 with a 3.12-only filter: reran with cached 3.12.
- ps was sandbox-denied: no further process-inventory probes; superseded sessions
  were stopped via the owning execution tool.
- Real-binary Python tests used mismatched release/debug binaries: built only the
  release MCP binary, then reran with a matched release pair; both checks pass.
- Verification-document generation first used a colliding nested heredoc marker:
  shell parsing failed before execution. This was inadmissible evidence about
  the implementation; a unique outer marker corrected the document probe.

## Verification hook closure

Coverage added after the measured quick run changes tests only. Expected:
malformed framing must wake pending requests just like EOF, and source mutation
performed by oracle cleanup must invalidate the final bookend. A query-only
timeout or a pre-cleanup source check would fail these edge cases. Result:
43 repair cases pass, 42 fail on the original harness, and the unchanged-source
positive control passes on both. The original malformed-frame reader emits an
unhandled-thread warning and the timing assertion fails; the candidate has no
warning. Full eval is 983 passed, 1 explicit live-model skip; fresh matrix 178 ok.
All quick implementation hashes still match the measured source snapshot.
Root VERIFICATION.md records exact commands and the broader environment limits.
