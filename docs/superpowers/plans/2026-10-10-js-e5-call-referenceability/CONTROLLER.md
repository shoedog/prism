# Controller-only private measurement — Step 1 candidate

**Status: STOP; command prepared, not authorized as an accepted/gated candidate.** The union full suite stopped on an unrelated MCP readiness failure. Head binaries are frozen under step1/bin with hashes in step1/binary-binding.json. Step-1 worker/controller parity was updated but not run. Reopen the stopped lane and complete public measurements/gates before using this private command for acceptance.

Supply the private root and a **new** private evidence directory. The planner must not run this command. It performs static capture only, never loading/executing corpus packages. All source paths, sites, graph rows, native details, logs, hashes and producer wall times stay in the private directory (umask077). Only aggregate JSON reaches stdout; generic failures produce only status/stage. The helper receives an explicit controller-only private-scope flag; ordinary planner workers reject any frontend-portal directory before traversal.

```bash
CORPUS_F_ROOT="/path/to/private/root" \
PRIVATE_EVIDENCE_ROOT="/path/to/new/private/evidence" \
bash /Users/wesleyjinks/code/prism-e5-plan/docs/superpowers/plans/2026-10-10-js-e5-call-referenceability/CONTROLLER-e5.sh diff \
  /Users/wesleyjinks/prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js \
  /Users/wesleyjinks/prism-evidence/js-e5/step0/bin/base-prism \
  /Users/wesleyjinks/prism-evidence/js-e5/step1/bin/head-prism \
  /Users/wesleyjinks/prism-evidence/js-e5/step0/bin/base-facts-v2 \
  /Users/wesleyjinks/prism-evidence/js-e5/step1/bin/head-facts \
  /Users/wesleyjinks/prism-evidence/js-e5/step0/bin/base-graph \
  /Users/wesleyjinks/prism-evidence/js-e5/step1/bin/head-graph
```

For census only: `CONTROLLER-e5.sh census TS_JS BASE_BIN BASE_FACTS_BIN` with the same environment. Diff mode freezes input/helper hashes before capture and verifies them before publishing; both CLI producers bind the E5 cache. Every producer has a 1500-second timeout. A failed producer is INADMISSIBLE; it is not assigned zero yield. Raw and after-override native verdicts are published side by side (the public run has no overrides). Full private differing call/flow rows are preserved for the controller's decision; only counts escape.

Public parity is exercised by `probes/controller-parity.test.cjs` against the **actual shell script**, real base/head call/fact/graph binaries, and a synthetic public TypeScript fixture. It compares the worker census, changed target, graph adjudication, raw/after-override summaries and stdout redaction. Historical step-0 combined native/controller suite:12 passed,0 failed. Step-1 parity is NOT RUN because of STOP; those historical results do not validate the new binaries. This does not verify the private corpus.

The census helper now publishes `R-local`, `R-local-no-require` and `R-union` pricing. The historical binding axis calls scalar `require` bindings `local_declaration`; their cost and the explicit every-require-initialized exemption are separate census predicates, not adopted product behavior.
