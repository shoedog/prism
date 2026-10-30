# P3 — public source packet and worker-outcome classification pilot

**Planning only; not dispatched.** The exact-caller-read final gates take priority. This brief recommends a small research observation increment, not production ownership or dependency acquisition. An independent planning review and controller dispatch must precede implementation/execution.

## Recommendation

Run one bounded **source-only packet/outcome pilot** for the existing `packages/fractional-indexing/tsconfig.json` inside the complete authenticated public Excalidraw archive. Preserve the full root and inherited configuration. Add only an external diagnostic driver that records the actual worker process outcome alongside the unchanged existing research packet. Use deterministic synthetic worker controls to prove the classification. Do not attempt the unavailable installed-root/private reproduction or a production owner-admission repair.

Why this is useful: source custody now exists, while historical failure labels collapse distinct causes. `scripts/callable-observations/index.mjs:38–46` maps timeout and output-buffer overflow to `budget_exceeded`, and other nonzero/parse failures to `worker_failed`; `membership.mjs:110` onward similarly aggregates failures. Historical private membership was `unavailable/worker_failed`, not proof of timeout or absent Program. A source-only public control can now establish which phase actually completes under an existing cap, and the external driver can preserve discriminating evidence without changing those established schemas or inventing the historical private cause.

Fractional-indexing is appropriate here as the smallest historical config-boundary falsification/control target, unlike P2 demand sampling. Its config extends `../tsconfig.base.json`; do not reroot to the package. Its historical 304-file Program (2 repository,216 dependency,86 compiler) and zero historical real receiver sites are historical observations on a different installed root, not current expected counts or a value claim.

## Current evidence and constraints

- Source archive: Excalidraw `0642e72cfa2d9a71198200e52f37399384610ee3`, tree `709e9146b0fbd78c3ebf0d77e67143b2fbc43e4a`; complete regular-file manifest SHA `afb0c3e9172fd66ff805e114a6413de6a9aef00f42c2491e8b67bb5900de0eba`. Custody receipt: `/private/tmp/prism-post317-measurement-inputs/RECEIPT.md`. This is1229 regular files/54,407,283 bytes, not a compiler membership count.
- Original public installed dependency root is unavailable; no installation is authorized. Private inputs remain unavailable. The entire historical installed/private experiment remains **input-blocked**.
- Pinned TypeScript5.9.3 compiler entry SHA `3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675` is available. Execution must additionally bind all compiler-library bytes read by the worker, the worker/transitive-module hashes and accepted Prism source; entry hash alone does not bind the library directory.
- Current native production admission independently requires bounded complete inputs, parse success, reproduced compiler evidence and exact Program/input equality (`src/executable_owner.rs:122–145,225–260`; `src/executable_owner/acquisition.rs:105–156`). Its full-root budget and `default/reject` policy remain unchanged. Research success never supplies an `AuthenticatedProgramEpoch` or executable-owner proof.
- Existing source classification/grammar repair followed PR303/304. Do not repeat that work merely because historical docs list parser errors; current implementation has changed. See `docs/eval/receiver-closure/2026-09-09-project-boundary-feasibility.md`, `2026-09-09-project-membership-observations.md`, and `2026-09-10-native-parser-representation-classification.md` for historical boundaries, not current results.

## Exact future scope

1. An isolated archive of the finally accepted Prism source, with production and existing research workers byte-unchanged. Own at most one external Node driver, one synthetic test module and receipts under a new `/private/tmp` runner. No Cargo/native helper build, no membership/owner acquisition, no repository edits.
2. Authenticate the complete source root, chosen original config and parent chain, compiler/library tree and worker assets. Snapshot flavor is explicitly `public_source_archive_without_installed_dependencies`; retain package/lock declarations as obligations, not materialized bytes. Never call this the historical installed snapshot.
3. Use existing `settings()` with `profile=default`, `links=reject`, unchanged limits; invoke the exact existing `worker.mjs` with the same normalized JSON and pinned Node binary. Clear `NODE_OPTIONS`/`NODE_PATH`; execute no package/source scripts. Record driver identity separately from worker/packet producer identity.
4. Capture process spawn error code, exit status, signal, wall duration, configured timeout/output/heap limits, exact captured stdout/stderr byte lengths and hashes, truncation flags, then strict existing `parsePacket` result. Preserve raw bounded bytes. Classification is an external sidecar; it must not rewrite the worker packet, bump its schema, relax its validator or mint authority.
5. For a valid packet preserve it completely, including diagnostics, config provenance, entry/type-lib/search obligations, outside/refused lookups, snapshots, Program members and semantic-closure fields. Project/dependency/compiler sets are descriptive source-only observations. Missing dependency installation is an independent readiness failure regardless of a packet's own closure field.
6. At most two public invocations (cold/repeat), under unchanged per-invocation limits. Compare canonical packets and all stable classification/provenance fields; timing, OS process IDs and other explicitly volatile telemetry are separate. Refusal/unavailable is a valid completed measurement outcome. No automatic profile increase, retry until success, alternate config, smaller root or dependency acquisition.

## Worker outcome contract

Classify only what raw process evidence supports, preserving all observations when predicates overlap:

| Observation | Sidecar classification | Must not claim |
|---|---|---|
| Spawn error before child starts | `spawn_failed`, exact code | compiler/Program failure |
| `ETIMEDOUT` from enforced parent deadline | `deadline_exceeded` | algorithmic slowdown cause |
| `ENOBUFS` from configured capture cap | `capture_limit_exceeded` | Program too large without separate evidence |
| Nonzero exit or signal without above | `child_failed`, status/signal/stderr digest | OOM, timeout or private-history cause merely from signal |
| Successful exit, invalid UTF8/JSON/schema | `invalid_packet`, validation stage | empty or complete Program |
| Valid unproven/refusal packet | `packet_observed_unproven`, exact reasons | worker crash or zero demand |
| Valid observed packet | `packet_observed`, retained closure fields | installed-root/semantic readiness, runtime authority or production eligibility |

No-packet states carry `program_observation=null`; a valid packet's empty list remains distinguishable from an unmeasured Program. Raw stderr is evidence, not a guessed root cause. Preserve combined facts without pretending one wrapper label discriminates all causes.

## Limits and prerequisite gates

Use existing default research ceilings from `schema.mjs:19–24`:20,000 files,128MiB snapshot/read bytes,30s worker timeout,8MiB stdout capture,512MiB Node heap; never change these numbers to obtain success. Bound stderr capture explicitly within the same recorded transport policy; outer supervisor at35s only guards termination of the owned process tree, not an extra compiler budget. Root source size alone does not prove the combined compiler/input read budget will pass. Driver≤300 non-test lines, tests/fixtures≤450 lines, total≤750; stop for design before exceeding them. No benchmarks or claims from elapsed telemetry.

Authenticate symlink/path/regular-file policy before invocation. Any source/compiler/worker drift, missing config, unbounded output, ambiguous snapshot identity, new dependency need, or attempt to consume a packet as owner authority stops execution with an explicit incomplete/input-blocked receipt. Do not delete inconvenient files or normalize source. If an existing worker internally collapses an exception into a refusal packet, retain that reason as opaque; do not patch the worker to recover finer detail in this slice.

## Testable acceptance and RED

- Unchanged accepted-worker characterization is a passing control, not RED. The new sidecar's same-interface baseline classifier deliberately collapses timeout/output/nonzero/invalid-packet states; complete output assertions must fail on the missing distinctions. Preserve baseline source and each behavioral failure; no missing executable/schema or compile error counts as RED.
- Deterministic injected process-result cases plus tiny owned child fixtures cover spawn failure, actual bounded timeout, actual capture overflow, nonzero exit, signal with unknown cause, valid JSON/wrong schema, invalid JSON/UTF8, valid refusal and valid observed packets. Include mixed error/status evidence and exact null-versus-empty semantics. Synthetic limits may be lower solely for controls and must never be recorded as public-profile measurements.
- Complete packet bytes must equal the unchanged worker's output; schema validation remains the existing validator. Driver mutation/forged producer/config/snapshot identity must fail custody; telemetry edits must not manufacture packet validity.
- Public cold/repeat retains the full source root and exact config. Either canonical packet parity or an explicit nondeterministic/incomplete outcome is acceptable; never label unstable output reproduced. Before/after source/config/compiler/worker hashes must match.
- Final receipt includes commands, source/config/asset hashes, raw output custody, classifier-control totals, complete public outcomes and field-level comparison. No historical304 count assertion, private reproduction, dependency closure, semantic-validity, recall, production capability or publication claim.

## Successor decision

If the source-only packet is reproducible, it is a **diagnostic/control success only**. A later installed-root closure experiment still needs separately authorized scripts-disabled locked dependency acquisition, link/budget policy, full dependency/compiler custody and effect/config obligations (including unresolved root/editor `react-scripts`). If the worker refuses under existing caps, retain the now-discriminated phase/outcome and stop; do not fix transport, cap policy, parser or dependency authority here. Private worker diagnosis waits for private custody and cannot be inferred from public/synthetic controls.

The highest-value feasible deliverable is thus a trustworthy bounded refusal/observation receipt, not a claim that source acquisition solved compiler ownership. If the controller does not value that diagnostic distinction, park P3 as input-blocked rather than perform a redundant seven-config census.
