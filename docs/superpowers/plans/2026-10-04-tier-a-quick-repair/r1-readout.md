# R1 repair readout

Bound to `595430e5545b13529bd9da2412342457da597630`, branch `feat/tier-a-quick-repair`. Changes remain uncommitted; controller owns Git writes. Patch: `/Users/wesleyjinks/prism-evidence/meas/tiera/repair-r1/R1.patch`. All artifacts below are under that evidence directory. No acquisition, Git writes, providers or application source execution occurred. The original artifact was repaired in place.

The requested quick/eval/matrix gates pass on this host with one fixed harness. The underlying native rust-analyzer cancellation cause remains UNKNOWN: the brief's request to settle that upstream mechanism is still open. The broader Node suite also has 18 out-of-scope native-membership failures. These limits prevent a whole-project or cross-host closure claim.

## Root causes and bounded repairs

Rust: a fresh external-copy, no-SUT probe returned persistent ContentModified on 20/30 sampled queries despite reported quiescence. Controls for configuration/status passed locally, but an exact cold base also passed; neither configuration nor a startup race is established as the native cause. `r1-diagnosis.md`, probe JSON and logs preserve predictions, alternatives and results. The settled harness gap was proceeding from inventory/readiness without checking the native session could answer the actual sample. Typed settings arrays, serverStatus/progress readiness, original-deadline retries, and a complete-sample preflight now guard it. One recovery is allowed only after persistent ContentModified exhausts a query deadline; the original session deadline/configuration are retained, the whole inventory must agree, and all sampled answers are revalidated in the new session. Failure/drift invalidates. quick2 demonstrated recovery; the final three did not require a restart.

TS: native incoming hierarchy returns the related symbol family, including impossible base/sibling/override and super callers. Every incoming span now receives a native definition query at its UTF-16 token start; only exact file/declaration-selection binding to the seed survives. `oracle_filtered` preserves dropped spans and definitions. The larger TS run filtered 36 sites. Native quoted-method, override/super, alias, import, property/getter and Unicode calibration controls are retained.

Other closed mechanisms: ambiguous-pin timeouts/errors were rescored as regressions; DFG edge/stats subprocesses bypassed deadlines; display names dropped quoted selections; string removal shortened M3 columns; tsserver startup used a longer query limit; ancestor symlinks outside a corpus caused false drift; full-run SUT timeouts could be excluded below the error floor; failure labels attributed input/SUT faults to the oracle. Each is repaired with negative/edge controls. Configuration/version and nested node_modules now affect identity/validity. Manifest observed identity is its actual digest. Metrics are labelled sample-pooled and caller-context, including outgoing function sets.

The larger Node run exposed an additional WRONG: 110 selected probes were reduced to 96 stored results by file:line IDs. A complete census enumerated the shared-line groups. IDs now retain column/name/kind/container. Exact file-scoped symbols select uniquely named shared-line SUT seeds; a uniquely innermost interior line selects an outer callable obscured by an inline default callback (Server.js:328 deploy -> line329). Equal-range/name ambiguity remains invalidating. decision2/3 were preserved INVALID; final decision4 stores all 110. Browser/minified dist duplicates were excluded from the Node runtime stratum while their source bytes remain pinned. The unused 2.3 MB diagnostic snapshot is deleted, with an evidence backup.

## Three consecutive fresh gates

Each invocation immediately rebuilt release, removed the external Rust copy's generated target, used a new SUT cache and a new oracle process, and verified source manifests before/after. The Rust copy is manifest-exact and outside the clone. Receipts hash every harness module, syntax helper and configuration before/after; all three hashes match each other and the final delivery. Whole-command wall time excludes the preceding build.

| Run | Whole wall | Rust | TS | Node | Result | Restarts |
|---|---:|---:|---:|---:|---|---:|
| quick12 | 255.250s | 135.157s | 116.626s | 3.441s | all VALID | 0 |
| quick13 | 236.247s | 103.124s | 129.327s | 3.770s | all VALID | 0 |
| quick14 | 222.395s | 105.529s | 113.450s | 3.393s | all VALID | 0 |

Diagnostic cap: four variants plus a disclosed three-probe extension, then park the unresolved upstream cause. Validation extensions followed closed, enumerable shared-line repairs on this artifact. quick1 INVALID, earlier source epochs and decision1/2/3 remain retained. They are not counted in this final consecutive sequence.

## Corrected JS/TS measurement

`--lang ts,js --sample 12`: 60 TS and 55 Node seeds (120/110 directional probes); no failures, no missing result IDs, both VALID. Seed 42. Native tsserver 6.0.3 with dependencies absent; corpus byte manifests and oracle environment identities agree. TS wall331.564s; Node9.393s. These are raw all-tier call-site counts after correcting the oracle binding policy, not a rebaseline or newly blessed adjudication. Exact-tier numbers are shown separately.

| Corpus / direction | TP / FP / FN | Precision (95% Wilson CI) | Recall (95% Wilson CI) |
|---|---:|---|---|
| excalidraw-ts callers | 56 / 16 / 93 | 77.8% [66.9, 85.8] | 37.6% [30.2, 45.6] |
| excalidraw-ts callees | 203 / 47 / 14 | 81.2% [75.9, 85.6] | 93.5% [89.5, 96.1] |
| secbench-node callers | 9 / 0 / 56 | 100.0% [70.1, 100.0] | 13.8% [7.5, 24.3] |
| secbench-node callees | 11 / 2 / 35 | 84.6% [57.8, 95.7] | 23.9% [13.9, 37.9] |

| Exact tier | TP / FP / FN | Precision (95% CI) | Recall (95% CI) |
|---|---:|---|---|
| excalidraw-ts callers | 49 / 10 / 100 | 83.1% [71.5, 90.5] | 32.9% [25.9, 40.8] |
| excalidraw-ts callees | 178 / 31 / 39 | 85.2% [79.7, 89.3] | 82.0% [76.4, 86.6] |
| secbench-node callers | 9 / 0 / 56 | 100.0% [70.1, 100.0] | 13.8% [7.5, 24.3] |
| secbench-node callees | 11 / 2 / 35 | 84.6% [57.8, 95.7] | 23.9% [13.9, 37.9] |

Quick is a smoke check. Larger samples and Wilson intervals make uncertainty visible; they do not guarantee power for any chosen effect size. Intervals condition on this sampled native frame and collapsed site counts. Pooling is not population weighting; binomial intervals do not adjust for function clustering or the stratum sampling design. The SecBench selection is a five-package vulnerable Node/CommonJS stratum, spanning process discovery/invocation, HTTP, configuration and search; it is not a general JavaScript population estimate. Native hierarchy omissions for const/destructuring aliases and property invocations remain a calibration limit (native-calibration.json), not a Prism truth claim.

## Unsampled population and supplemental measurement

| Corpus | Native hierarchy named | Prism named | Prism outside hierarchy | Independent property callables | Independent getters |
|---|---:|---:|---:|---:|---:|
| excalidraw-ts | 2908 | 4062 | 1207 (29.7%) | 661 | 10 |
| secbench-node | 121 | 356 | 235 (66.0%) | 98 | 6 |

TS remainder: 1,011 arrows, 124 function expressions, 64 methods, eight function declarations. Node remainder: 12 arrows, 208 function expressions, ten methods, five function declarations. These are named Prism definitions without a native hierarchy match; the independent syntax census is a different population and includes properties/getters explicitly. Anonymous Prism definitions (TS 4,459 / Node 159) are disclosed separately in M1, not part of the named denominators.

The independent AST chooses member-call/getter-access tokens, then raw native definition spans bind exact columns to concrete census declarations. A separate sample of 100 sites per shape (getter Node population 37) uses the same final source and byte pins, with no measurement errors. `measure-members.py`, members100.json/log record reproduction and every exclusion. This is conditional detection recall; it cannot estimate incoming-set precision. A missing SUT declaration is a miss; nonconcrete/interface/external/unsupported bindings are excluded explicitly.

| Corpus / shape | Candidate sites | Sampled | Concrete evaluable | Found / missing | Excluded nonconcrete | Detection recall (95% CI) |
|---|---:|---:|---:|---:|---:|---|
| excalidraw-ts property_callable | 6118 | 100 | 3 | 0 / 3 | 97 | 0.0% [0.0, 56.1] |
| excalidraw-ts getter | 2282 | 100 | 5 | 0 / 5 | 95 | 0.0% [0.0, 43.4] |
| secbench-node property_callable | 277 | 100 | 39 | 0 / 39 | 61 | 0.0% [0.0, 9.0] |
| secbench-node getter | 37 | 37 | 15 | 0 / 15 | 22 | 0.0% [0.0, 20.4] |

Most TS sampled members resolve to nonconcrete definitions, so this sample has weak TS power. The primary decision4 supplemental sample of 12 had TS property 0/1 and no concrete getter denominator; its `null` recall is retained, not manufactured as a zero-score estimate. The 100-site sample exposes concrete Node member gaps, but does not assign one implementation to a generic/interface callback.

## Verification and admissibility

| Check | Result | Evidence |
|---|---|---|
| Repair tests | 90 passed | repair-tests-final.log |
| Exact base RED control | 40 failed: 30 behavioral/contract assertions, 10 new-API absence controls | repair-red-final.log, red-classification.json |
| Full eval | 1023 passed / 1 explicitly opt-in live-adoption skip | eval-suite-final.log |
| Matrix after immediate rebuild | 178/178 ok; no regressions or flip candidates | matrix-final2-build.log, matrix-final2.log |
| Full Rust, all features, no-fail-fast | 5162 passed / 0 failed / 1 existing ignore, 31 targets | rust-suite-pinned.log |
| Full Node population, cached pinned inputs, canonical concurrency2 | 777 passed / 18 failed / 2 skips, 797 tests in 44 files | node-suite-canonical.log |
| Script Python | 57 passed +82 subtests | scripts-python.log |
| Anchors / adjudications | all 167 tracked files byte-identical to base | anchors-unchanged.json |
| Whitespace / patch | git diff --check; patch apply-check on exact changed-path base export | patch-check.log, delivery-receipt.json |

The initial Rust/Node runs lacked explicit prerequisite environment and are retained (5139/23/1 and121/31/1). A verified cached 5.9.3 compiler and authenticated profiles/archives, plus freshly built helpers, enabled larger complete reruns. The oracle measurement uses tsserver 6.0.3; the 5.9.3 compiler is for the separate project suites. Concurrent and canonical-concurrency Node reruns have the same 18 native-membership failures. A simple helper probe succeeds with JSON/empty stderr, which does not discriminate the failing full-input/refusal mechanism. No source changes were made to Rust/scripts/tests outside eval; no regression attribution is asserted without a same-environment pre-change full-suite control. No out-of-scope fix or rebaseline was performed.

Failed Node cases, all in `scripts/callable-observations/membership.test.mjs`:

- full compiler/native/compiler ts keeps old packet and loaded census
- full compiler/native/compiler tsx keeps old packet and loaded census
- whole native census retains excluded Bash and Terraform outside Program
- parent config and sibling import remain inside immutable audit root
- exclude never drops imported source or its changed hash
- same-named dependency declarations and JSON/mts stay distinct from native bodies
- automatic type/lib obligations remain incomplete without authority
- configured type/lib obligations remain incomplete without authority
- source type/lib obligations remain incomplete without authority
- ambient/prototype effects remain in dependency/sibling evidence after classification
- in-root link aliases are observed but default rejection and escape/cycle remain
- parse errors and skipped invalid UTF8 are explicit incomplete census
- missing/extra/duplicate/relabeled members and genuine config/epoch substitutions refuse
- CLI enforces non-authorizing envelope and strict option selection
- Unicode native ordering matches the complete compiler identity census
- explicit symlink root uses the same canonical identity as the existing observer
- case-alternate root follows measured compiler filesystem policy
- well-shaped compiler facets and native producer identity cannot be counterfeited

The two Node skips remain visible in the complete log; no tests were deselected. Unsupported flags/imports, unmatched shell globs and sandbox refusals were inadmissible diagnostic probes; corrected probes replaced them. The final low-rate-timeout control uses equal 32-probe defaults to remove the sample-size confound. Source-width and M3 UTF-16 checks assert behavioral output before new-helper imports; the ancestor-symlink fixture places the symlink above root.parent to exercise the original miswalk.

## Controller commits / files

1. `fix(tier-a): bind oracle queries and preserve bounded failure accounting` — `eval/tier_a/{lsp_client,oracles,tsserver,sut,matrix,pinned,accounting,corpus,spotcheck,strata,cli,report}.py`, `eval/tests/{test_r1_repair,test_quick_repair,test_pinned}.py`. Includes the seed-ID/addressability seam, typed timeouts, source columns and lifecycle errors.
2. `feat(tier-a): measure Node and member frames with disclosed uncertainty` — `eval/tier_a/{member_sample.py,ts_syntax.cjs}`, `eval/corpora.toml`, `eval/manifests/secbench-node-r1.sha256`, `eval/README.md`, R1 diagnosis/prepare/readout and superseded historical handoff/readout; delete `eval/snapshots/prism-4e592daa7858.json`. `cli.py` is shared by both groups, so controller may combine the commits or split its hunks.

## Still open / not verified

- Native rust-analyzer root cause and reproduction on either reviewer's different host; the final three and recovery evidence are host-bound.
- The 18 native-membership Node failures; their full-input native refusal mechanism is not settled. The full Node suite is not green.
- Go/Python live oracle paths, a human-triggered all-corpus run, mutation campaigns and statistical power/design/cluster correction.
- Concrete TS member implementations behind nonconcrete/interface definitions and alias/dynamic/async/module-runtime truth beyond calibrated native support.
- The shared reader-thread write-lock stall SMELL remains; it fails closed and is outside these material repairs.
- Git commits/push/review acceptance: controller-owned. No memory changes were authorized.

Delivery source hashes, exact patch apply-check, anchor byte checks and handoff live in repair-r1. Generated task caches were removed after completed receipts; source, manifests, failed/superseded runs and logs remain.
