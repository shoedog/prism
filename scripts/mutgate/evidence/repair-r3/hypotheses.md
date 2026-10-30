# Repair round 3 hypothesis/probe/result log

Cap: two implementation/verification cycles, declared before implementation.

1. Hypothesis: r2 schema rendering falsely kills equivalent unused-local edits
   through indirect location observation. Alternative: red original tests or
   invalid selectors. Expected: default schema exit 0; pure text exit 1 with
   green baselines and completed survivors. Falsifier: text also kills, or red/
   inadmissible original selectors. Probe: compiled six retained fixture bytes
   against unmodified r2 in this environment. Result: unit-prechange.log has
   six exit-0/expected-exit-1 failures; text-control.json has six SURVIVED, exit 1,
   each source selector green. The competing baseline explanation is ruled out.
2. Hypothesis: first candidate's six failures come from an empty synthetic
   scoped selection. Alternative: isolated workers apply wrong edits. Expected:
   full selection 1/SURVIVED, scoped selection 0/no results. Probe: helper replay
   prints exactly those outputs. Result: confirmed. Cycle-1 scoped observation
   is inadmissible for behavior (zero selected mutants). Bounded cycle-2 fix:
   use file scope, assert selected == 1. Full candidate suite: 55 tests pass
   in unit-cycle2.log, including default/scoped text survivors and advisory kills.
3. Resource capability hypothesis: process listing can sample aggregate RSS.
   Expected: ps rows; falsifier: environment refusal. Probe: certify.py warm.
   Result: sandbox PermissionError launching ps. Inadmissible for RSS/build
   behavior. Whole process-listing class is blocked for this session. Replaced
   sampler with macOS /usr/bin/time -lp, using locally read time(1) documentation.
   Rusage maximum RSS is reported, not aggregate concurrent RSS.

Schema spelling lists were not extended. Existing advisory demotion/confirmation
regressions remain. Certification and full suite results are recorded separately.

4. time(1) -l capability: expected an RSS rusage block; actual sysctl
   kern.clockrate permission refusal yielded only timing fields. RSS observation
   inadmissible; no further time -l attempts. Direct getrusage probe allocated
   80,000,000 bytes in a nested waited child: ru_maxrss 88,621,056 bytes, exit 0
   (rss-capability.json), proving this capability works and includes nested
   waited children. Future certification uses time -p plus getrusage in bytes.
   Aggregate concurrent RSS remains unverified. The already-running jobs1 run
   retains valid gate/timing outputs but its original metric wrapper cannot
   supply RSS; do not discard or rerun the valid gate because of that metric gap.

5. Worker-context attribution hypothesis: relocating equivalent text mutants
   can change compile-time manifest paths. Alternative: the unused-local edit
   actually changes the assertion, or Cargo path aliases confound the control.
   Expected: green shared baseline, candidate KILLED with differing manifest
   path in failure output, r2 text SURVIVED in exactly the same canonical path.
   Falsifier: source/worker baseline red, or r2 text also fails under the same
   spelling. Initial probe used /var vs /private/var expected paths: red source
   baseline, inadmissible (worker-baseline-invalid-path.json). A second control
   mixed those path spellings in the reused cache: inadmissible attribution
   (worker-baseline-invalid-control.json). Corrected probe canonicalized the
   entire fixture root for both artifacts: candidate KILLED exit 0, r2 SURVIVED
   exit 1 with green original selectors (worker-baseline-probe.json).
   WRONG: the worker-only manifest path change creates a false kill. Entire
   bounded population: context-induced compile or selector failures before
   any mutant; fix is original-source build/preflight in each private worker.
   At cap two, disclosed one converging bounded extension (cycle 3); no restart.
   Added compile- and test-failure negatives; the pre-fix worker regression is
   red (unit-worker-preflight-prechange.log), and all 56 tests now pass
   (unit-cycle3.log). Earlier full timings remain under before-worker-preflight
   and are superseded; final jobs1/2/4 certification is rerun on the fixed driver.
6. An attempted import of the hyphenated repair-r3 evidence directory as a
   Python module failed before execution: inadmissible, no behavioral evidence.
   Corrected empty-scope timing uses runpy.run_path instead.

7. Full suite first attempt used the mutation scratch manifest and stopped
   after lib: 1,469 passed/20 failed, all worker_schema. Expected under a path/
   context hypothesis: unchanged r2 Rust inputs reproduce those failures in
   that scratch environment, while the normal checkout manifest passes.
   Alternative: dependency/protocol mismatch also fails the normal checkout.
   Same-environment r2 Rust control (all 747 source/test/build hashes equal)
   reproduced the exact same 20 lib failures (scratch-suite-control.json).
   This rules out attribution to the mutation driver edits; the broader path
   hypothesis awaits the normal-root all-features/no-fail-fast suite. Failures
   are retained in scratch-suite/ and scratch-base-control.log, not re-baselined.
   No Rust code was edited. Full no-fail-fast suite is now run from ROOT.

8. Normal checkout library: 1,489 passed/0 failed, including all 20 scratch
   failures. Root/scratch comparison discriminates the context explanation
   from a common compiler/protocol mismatch. Mechanism: detached-owner-worker.mjs
   line 16 runs only if path.resolve(process.argv[1]) equals fileURLToPath(
   import.meta.url); the scratch scripts directory is a symlink, so invocation
   and canonical module paths differ, main emits no JSON, and acquisition.rs
   line 103 refuses as worker_schema. This is a scratch-harness limitation,
   reproduced by r2 bytes, not a mutation repair regression. Normal full suite
   continues with no-fail-fast; no production fix or re-baseline was made.
