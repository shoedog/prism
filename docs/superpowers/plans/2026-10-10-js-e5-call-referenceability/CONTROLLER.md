# Controller-only private measurement — gated public Step1 candidate

**Status: command ready; actual-shell worker/controller parity PASS. F union remains controller-only and unmeasured by planner.** Supply a private root and a new private evidence directory. Only aggregates leave that directory; every source path, native detail, differing site/graph row, input/helper hash and producer log stays private. Static capture only; no corpus package execution.

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

All CLI producers bind ~/prism-evidence/js-e5/cache; direct helpers have no disk cache. Timeout1500s per producer; unsuccessful producers are INADMISSIBLE, never zero-yield observations. Native TypeScript5.9.3 remains pinned, raw and after-override are side by side. `R-local` and `R-local-no-require` remain in both controller summaries; their runtime adoption is outside E5.

Parity command and14/14 result are in REPORT-step1.md; step1/controller-parity.json binds actual shell script results with step1 head binaries. Final public receipts are in union-measurements.json. No private directory was opened.
