# S1 — current search-boundary classification

Fresh replay of merged PR280 reproduces its public packet byte for byte:
SHA256 `47f41ee5d64aada696f89b7a3933f36d1a34c7cb59bd26789dceaf96664843de`.
A disposable instrumented copy changes only producer identity. No production worker,
schema, policy or application behavior changes in this increment.

## What the capture explains

| Outside phase | Ancestor module directory | Type-root directory | Peer metadata | Total |
|---|---:|---:|---:|---:|
| Module callback | 442 | 0 | 6 | 448 |
| Library replacement search | 328 | 0 | 0 | 328 |
| Source type directive | 2 | 24 | 0 | 26 |
| Configured type entry | 0 | 4 | 0 | 4 |
| Total | 772 | 28 | 6 | 806 |

All 1,269 refused boundary encounters have a module callback owner. They reconcile
to the existing 264 distinct refusal digests. The complete capture has 2,075 boundary
encounters and 6,893 module callback occurrences. These are different populations:
one request can make repeated probes, and different requests/phases share paths.

The public configuration explicitly sets `types`, so the four entry-type probes in
this capture are configured, not automatic. Future fixtures must cover automatic
discovery separately. All 806 outside encounters here are `directoryExists` checks;
this fact does not prove that external directories are absent or irrelevant.

The original mechanism totals agree with the earlier source-backed classification;
the new value is **current per-occurrence module ownership and finer phase evidence**.
Nearest compiler resolver frames matter: an outer source-type frame can surround a
nested library search. An initial membership-priority heuristic misclassified that
nesting; it was discarded and replaced before producing the report. It was an audit
probe defect, not a production regression.

## Proof boundary and next increment

The [contract](../../superpowers/specs/2026-09-08-callable-search-classification.md)
pins compiler source locations. The capture tool copies the observer and fails if
its exact instrumentation anchors drift. The audit checks receipt bytes, unchanged
packet fields, module multiset, ordinals/ownership, refusal coverage and outside flag.
Those checks establish consistency of this captured run, not authenticity of an
arbitrary attacker-authored trace or semantic admission of a path family.

The independent source review found no additional WRONG behavior in this diagnostic
slice. It identified three SMELLs relevant to implementation: aggregates do not carry
event ownership; only module callbacks currently expose a request boundary; shared
type/lib caches make a single mutable current-request label inadequate. S2 will use
a bounded context stack and record module-owned observations without changing any
closure decision. Type/lib cache-equivalence work follows separately.

No receiver recall gain is claimed. The fixed public packet remains unproven; all
existing closure barriers, the 14 module-literal gaps and unresolved react-scripts
remain. Runtime and class authority stay false. All 1,229 original source files
were freshly verified against manifest
`353187a695df2683a3631e4739c173cb6d33190901093549b61e28efeda60cbb`.

## Verification and custody

Audit controls pass 11/11 with no failures or skips; the
[compact evidence](2026-09-08-callable-search-evidence.json) records the fixed replay.
Review round one found a WRONG in the new audit: a changed request anchor could
pass its triple-only multiset comparison. The existing artifact was corrected to
compare anchor-bearing occurrences, with negative controls. Round two verifies
that repair plus mode, unknown-outside, population and receipt controls. These are
primary integration passes (NOT an independent review of the whole lane); the
compiler seam review was independently delegated. Cap two, no extension.
Full gates pass on stable clean implementation `d43dd54`: observer375/375,
default Rust4,017 and MCP4,207 passed (one known ignored test each, doctests included),
helper/audit18/18, authority40 with no failures, fmt/diff checks. Tier-A not triggered.
The [gate record](2026-09-08-callable-search-gates.json) retains raw log hashes.
The operational runner's ignored-name regex initially missed the compiler's appended
ignore reason. Its failed summary is preserved; a closed correction independently
recounts both Rust logs and recognizes only the exact known ignored test. No test
result changed, no failing test was re-baselined, and no suite was rerun for that
report-parser correction.
The active [handoff](../../superpowers/handoffs/2026-09-08-callable-autonomous-sequence.md)
is authoritative for operational state. Raw normal/captured packets, full stacks,
copied instrumentation and receipts are under `/private/tmp/prism-overnight-0zmx3n`;
archived as `s1-evidence.tgz`, 7,308,737 bytes, SHA256
`8f737f4947a69a8d6cbeacd9e504af0d95d236c98389ef6ea1351d176b0926b4`.
No private repository source or measurements are included in this public readout.
