# JS/TS/React workstream value synthesis (controller, 2026-10-04)

## Inputs
- `survey-defects-sonnet.md` (Sonnet-5.5): general OSS defects. Ranking 3 > 6 > 2 > 5 > 4.
- `survey-cve-luna.md` (gpt-6-luna): npm/Node/React advisories. Ranking 3 > 6 > 2 > 5 > 4.
- Prism's own measurements: the #319 parameter-frequency census on Excalidraw, and the #324 positional-gap DEFER.

The two surveys reached the same ranking independently. I have not verified their citations myself. For example, arXiv:2601.21186 is a 2026 preprint, and BugsJS numbers were not extracted.

## Workstream assessment

| Workstream | Prevalence (prism-measured) | Defect and CVE weight (surveys) | Verdict |
|---|---|---|---|
| **3: member/occurrence flow (A, C)** | Not yet measured. `dfg-stats` NameOnly doubt counters give a cheap census. | Highest in both surveys: member flow is behind 68% of client JS faults (Ocariza) and is the prototype-pollution shape (11.3% of npm advisories). | **First**, but measure first. |
| **6: Node checkpoint (D, G, H)** | Partly done already: S1b, lane P, P2 and the CJS work cover a lot of D. | Second in both. Runtime sinks (exec/eval, fs traversal, SSRF, deserialization) are about 25–40% of advisories, and high severity. | **Second.** Mostly a measurement programme; G-sink coverage is likely the gap. |
| 2: parameter forms (B) | Excalidraw: object patterns 355/4,600 (7.7%, 296 in TSX components, i.e. React props); rest 13; unparenthesized arrows 2. Positional-gap DEFER (Y=1). | Library entry points (B+D) are prerequisites for most library CVEs. | **Fold into 5**: the remaining demand is React props destructuring. |
| 5: React provenance (F) | The private target corpus F is a React frontend; its S2 gain was 0. Bounded props proofs already exist (#246/#247). | XSS is 25.5% of npm advisories (only a React subset applies). No peer-reviewed hooks-bug prevalence exists. | **Third**, merged with WS2's props destructuring. Highest relevance to the owner's own corpus. |
| 4: compiler ownership (E) | 512-input owner-census cap; real roots refused. | Last in both: an enabler, not a source→sink class. | Defer. |
| 7: readiness / accuracy | Tier-A quick oracle still invalid (hung for over an hour in S2 R2). | Not a capability. | An enabler: needed to *measure* any of the above. |

## Recommended next step (decision it serves: choose between WS3 and WS6 on data, not literature)
Run a **SecBench.js ground-truth slice**: 600 executable server-side npm vulnerabilities with known source and sink, covering prototype pollution, command and code injection, traversal and ReDoS (ICSE 2023).
- Run prism taint/chop from the exported API to the sink.
- Count what fraction is traced end to end.
- Attribute each miss to a capability (A–J).

This turns literature weights into prism-specific defect-risk numbers for WS3 and WS6, and also yields a reusable accuracy benchmark (part of WS7). Expected size: 1–2 slices. In parallel, a cheap `dfg-stats` census of NameOnly member doubts on X/R/T (plus F, run by the controller) gives WS3 its prevalence figure.

## Measured outcome (2026-10-05): SecBench #347 and Tier-A #346, both merged
**Label reliability:** blind labels by Sonnet against the harness labels.
- Traced vs not traced: 28/30 agree.
- Outcome: κ 0.36.
- Mechanism: κ 0.26.

So label-based attribution is unreliable, and the decision rests on **demonstrated conversions**: semantics-preserving rewrites, re-run, with 17 patches checked by Opus.

| Workstream | Single conversions (weighted) | Joint |
|---|---|---|
| ws3 member-only parameter Def | 20 (35) | with ws6: 92 raw / 34 handler clusters |
| ws6 (callback-argument parameter registration, a syntactic gap) | 1 (2) | (same joint) |
| ws4 dist/build admission | 7 (14) | — |
| ws2 rest parameter | 6 (12) | — |

Also found: a single unparenthesised arrow parameter has no Def (Opus F2).

Tier-A cross-check: Node member recall 0/84; Node caller/callee recall 14% / 24%.

**Recommendation:** one product slice of "JS/TS parameter-Def gaps":
- callback-argument parameter registration;
- member-only formal root Def;
- unparenthesised arrow parameter;
- rest parameter Def.

ws4 admission is a separate small slice. ws6 runtime/async and ws5 React are deferred and unmeasured (SecBench bypasses ingress; the private F corpus is React).
