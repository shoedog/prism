# Controller-only private measurement — step2b

Status: actual-shell parity and census tests14/14 PASS with step2b binaries; private F remains controller-only/unmeasured. Supply the private root and a new evidence directory. Only aggregates leave that directory; raw paths, site/graph rows, native details and binding/producers stay private. Static capture only.

```bash
CORPUS_F_ROOT="$CORPUS_F_ROOT" \
PRIVATE_EVIDENCE_ROOT="$HOME/prism-evidence/js-e5/f-e5-step2b-3df0ddfa-a2b4841d" \
bash /Users/wesleyjinks/code/prism-e5-plan/docs/superpowers/plans/2026-10-10-js-e5-call-referenceability/CONTROLLER-e5.sh diff \
  /Users/wesleyjinks/prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js \
  /Users/wesleyjinks/prism-evidence/js-e5/step0/bin/base-prism \
  /Users/wesleyjinks/prism-evidence/js-e5/step2b/bin/head-prism \
  /Users/wesleyjinks/prism-evidence/js-e5/step0/bin/base-facts-v2 \
  /Users/wesleyjinks/prism-evidence/js-e5/step2b/bin/head-facts \
  /Users/wesleyjinks/prism-evidence/js-e5/step0/bin/base-graph \
  /Users/wesleyjinks/prism-evidence/js-e5/step2b/bin/head-graph
```

Every CLI producer gets a new cache under ~/prism-evidence/js-e5/step2b/; direct helpers have no disk cache. Timeout1500s; failed producers are INADMISSIBLE. Native TypeScript5.9.3 is pinned; raw and after-override summaries remain side by side. R-local runtime adoption stays outside this slice. Exact parity command/results: REPORT-step2b.md; step2b/controller-parity.json. Historical step1 private F is not transferred to this new head. No private directory was opened by this planner.
