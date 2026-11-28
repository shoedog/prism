# Paths on Excalidraw (pinned 0642e72c, non-test and test files as in the product run)

Run: `tools/paths.py` on the final post-guard edges of the S3-head product run (`s3/out9/X`, binary 25459f77). The same measurement on the merged main binary (f720d717, `s4/bin/main`, `s4/out-main/X`) gave byte-identical edge dumps, identical `dfg-stats` and `call-stats` files and an identical `paths --aggregate-only` result, so the table is the merged main's.
Inputs: `head/edges.jsonl` (final post-guard edges) and `analysis/oracle-verdicts-X.jsonl` (oracle verdicts, every added props edge) of that run. Source of this table: `g/paths-X.json`.

Every hop of a counted chain is a props edge the oracle judged CORRECT, and a chain that enters a container or rest local by one key and leaves through another key is not counted (DESIGN 5.3).

| Component hops | Chains ending at a sink line | Chains ending at another element's attribute | Other chains | All paths of that length |
|---|---|---|---|---|
| 1 | 0 | 420 | 1646 | 2066 |
| 2 | 0 | 108 | 180 | 288 |
| 3 | 0 | 14 | 72 | 86 |
| 4 | 0 | 2 | 10 | 12 |

- Verified props edges (every one CORRECT, 0 WRONG, 0 UNDECIDED): 615.
- Chains of two or more component hops, every hop CORRECT: **386** (gate: at least 25 on the public corpora together).
- Connected share: 615 of 615 eligible props edges have a source node with an incoming data-flow edge in the final graph (100%; root unresolved: 0). Gate: at least 90% on each public corpus.
- First failures and post-S1 recoveries (DESIGN: reported apart from baseline predictions): this run is the final head only; the per-slice pre/post differences are in the lane evidence (`s3/out*`, `s1/out*`) and are not repeated here. The pre-guard versus final comparison is not measured here (`unmeasured`).
