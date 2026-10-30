# R1 hypothesis / probe / result log

Bound source: 595430e5545b13529bd9da2412342457da597630. Both review reports
were read in full. Direct Rust probes use source-identical immutable corpora;
pre-change controls use the same Python/host and the archived base harness under
repair-r1/base-source. No SUT was invoked by the direct oracle probes.

Before edits, competing hypotheses were (1) null workspace/configuration resets
cargo features; (2) queries race startup/indexing; (3) session-specific native
cancellation survives initial readiness; (4) SUT activity causes the oracle fault.
A configuration reply control should remove failures under (1), a status-only
wait under (2), and a no-SUT reproduction falsifies (4). The log/scripts retain
predictions/falsifiers before their corresponding probes.

Diagnostic cap: four variants, then a disclosed three-probe extension because
a fresh external copy reproduced the finite failure condition. At that cap,
the upstream mechanism remained unresolved; it was not silently attributed to
configuration, indexing, cache priming, or the PR. A bounded recovery policy
was then implemented and independently exercised with positive/negative controls.

| Probe | Wall | Failed queries | Artifact in repair-r1 |
|---|---:|---:|---|
| base | 15.863s | 0/30 | probe-base.json |
| status | 17.543s | 0/30 | probe-status.json |
| config | 18.424s | 0/30 | probe-config.json |
| both | 21.303s | 20/30 | probe-both.json |
| postwait | 24.617s | 0/30 | probe-postwait.json |
| cold-base | 29.334s | 0/30 | probe-cold-base.json |

The base/status/config probes all passed locally. The base transport did not
advertise configuration support and received ZERO workspace/configuration
requests. Thus a null response did not cause failure in those measured sessions.
Configuration correctness is nevertheless repaired and tested with an exact
settings array; it is not presented as the settled server root cause.

The external fresh-copy `both` probe failed 20/30 queries despite startup status
quiescence. It had no SUT. This rules out the SUT as a necessary cause, and shows
that startup status alone is insufficient. A later 10s post-inventory wait passed,
but its target directory was already warm: that observation does not distinguish
readiness from build/session state. It is a weaker observation, not causal proof.

A separate cold base control and three cold debug-logged sessions all passed.
The priming-disabled second phase was never reached because those sessions had
no failures. Therefore there is NO evidence that disabling cache priming fixes
this failure. Debug logging may alter timing. All generated logs are retained.
The first readiness-only repair quick (`quick1`) still returned INVALID after
407.691s. Its timeout failures are retained, not discarded or rescored as empty
oracle sets. It is outside the final successful three-run sequence.

Settled harness failure mechanism: the runner proceeds from inventory/readiness
to scoring without checking whether the native session can answer the actual
sample. Persistent ContentModified exhausts retries in that session; another
session on unchanged source/config can answer. This is a reproducibility gap,
not proof that the original harness scored those unavailable answers as correct.
The underlying native-server cause remains UNKNOWN.

Repair: explicit Rust serverStatus plus progress readiness at startup AND after
inventory; correctly typed configuration arrays; retries under one original
query deadline; preflight every sampled incoming/outgoing answer before scoring.
If preflight exhausts a deadline specifically after ContentModified retries,
restart ONCE, preserve the original absolute corpus/session deadline/config,
reread every inventory document, require an identical inventory, and revalidate
the entire sample. Other timeouts do not trigger recovery; a second failure or
inventory drift invalidates the run. Failed-attempt retries/restarts remain in
metadata, and scored answers all come from one successful session.

Measured quick2 hit the persistent first-sample caller failure, restarted once,
reread the identical 4,547-definition inventory and produced zero failed final
sample answers. Freshness applies to the initial run state; recovery's warmed
build artifacts are visible and are not described as a second cold run.

Other closed findings have mechanism-level controls in test_r1_repair.py:
DFG expiry/hang at both command seams; a pin-only timeout/error vs SutAmbiguous;
quoted selection spans/alias rejection; native override and super callers;
source-width masking and UTF-16; ancestor vs internal symlinks; nested dependency
presence and version drift; startup shorter than query; per-stratum override;
low-rate full-run SUT timeout; failure origin labels; supplemental member sampling;
recovery success, persistent failure, ordinary timeout and inventory drift.
The prior pinned regression expectation was updated to sut_error because an
unavailable answer is not a semantic regression. Historical baselines and
adjudications were not changed.

Native calibration is recorded separately in native-calibration.json. Direct
imports resolve and appear in hierarchy. Const/destructuring aliases and object
property calls can resolve through definitions while remaining absent from
hierarchy; named function expressions are supported. These native limitations
remain disclosed rather than adjudicated away. Function-set metrics describe
caller contexts in BOTH directions and do not prove target identity.

The larger Node decision sample exposed a separate WRONG: file:line probe IDs
stored only 96 of 110 selected directional probes with zero failures. The complete
native census in collisions.json enumerated three shared-line groups (40 named
declarations), falsifying the alternative that failed probes explain the shortfall.
IDs now retain native column/name/kind/container at shared lines, including M3.
Browser/minified dist artifacts are excluded from the Node runtime stratum, with
their source bytes still pinned. Exact file-scoped symbols distinguish uniquely
named same-line SUT seeds; ambiguity remains invalidating rather than silently
measuring whichever function Prism picks.

Decision3 correctly invalidated one named seed: Server.js:328 deploy, whose inline
default callback shares its start line and whose name is duplicated by a method
at line91. addressability-census.json enumerates the entire filtered SUT population
(three initially rejected definitions, only one named outer seed). Native evidence
deploy-interior.json confirms line329 selects the outer deploy. The final repair
uses a uniquely smallest enclosing range at an interior line where possible,
matching the unchanged native resolver; equal-range ties remain invalid. A live
fixture verifies distinct callee output for that selector and an unaddressable tie.
The final validation cap was explicitly extended to three fixed-source runs after
each enumerable repair; earlier measurements were preserved, never restarted or
re-baselined as a new artifact.

The first low-rate-timeout RED fixture was confounded by the base's smaller sample:
one failure exceeded 5% there. Correcting both defaults to 32 probes discriminated
the mechanisms: base VALID with one timeout below the floor; repaired INVALID with
typed sut_timeout. The original confounded observation was not behavioral evidence
for timeout invalidation. New-API absence failures are identified separately from
behavioral assertions in repair-red-final.log.

Inadmissible probes: unsupported prism-mcp --version, misplaced Prism --no-cache,
shell globs that matched no path, and the sandbox-denied process-list query yielded
no evidence about the corresponding hypotheses. Correct commands were used for
build/native output; process-census capability remained unavailable. Only completed
task-owned cache receipts were used for generated cleanup; sources/logs are retained.
The initial project suites lacked explicit compiler/profile/helper variables.
suite-prerequisite-probe.json and cached-gate-inputs-verify.log then established
available cached pinned inputs, enabling the complete offline suite reruns.
