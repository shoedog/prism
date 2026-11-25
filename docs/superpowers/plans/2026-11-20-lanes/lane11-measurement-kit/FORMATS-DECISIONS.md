# Measurement kit decisions

[Formats of record](FORMATS.md) · [Decision tables](FORMATS-DECISIONS.md) · [Fixture ledger](FORMATS-LEDGER.md)

## 8. Changes to DESIGN section 5

1. **Binding.** A git-clean query root excludes tracked edits as well as untracked/ignored files. Bind all command outputs **and stderr**, check the executable across the run, and compare effective analysis options before mode projection. This makes the source, options and trace used to claim a change inspectable.
2. **Edge identity.** Compress identical transitions into one row with a count. Combine one-sided residuals at a key into one row. Give each ambiguous bag both arm totals and a scalar maximum. This preserves both observed multisets and the matrix's row unit.
3. **Trace join.** Project the trace's byte fields away along with edge bytes in line mode. Count matching arm occurrences including cancelled ones. Ambiguous edge records fail independently; unmatched records require a lane veto to fail by themselves. Include unjoined `node` records. This permits conservative line-mode joins and spelling checks on nodes with no edge.
4. **Oracle questions.** Grade a uniquely proved in-root implementation body and fixed slot. A type-only declaration supplies no receiving body. The target filter defaults to gained exact head claims and can select lane-declared confidence/kind populations. Ambiguous raw arm-bag subsets refuse. These choices specify the population and the property each verdict can support.
5. **Review comparison.** Count each overlapping unmatched payload occurrence as one payload change, even in an ambiguous bag, and report absolute anchor-count surplus as presence. This keeps a zero meaningful without assigning speculative transitions.
6. **Benchmark bullets.** Resolve case ambiguity before testing for emissions, preserve one row per duplicate, use only the sink owner's tokens for `function_change`, and give moved/split/elsewhere/partial/incomplete cases explicit exclusion rules. A verified removal carries null fixed finding/reach. This keeps all denominators reconstructible.

**Provisional handoff change log**

| Provisional entry | Stage D disposition | Effect on an adopter |
|---|---|---|
| First issue, 2026-11-24, sections 1–3 | **Accept** wire endpoints, attributes, two key modes, main CLI, classifier exports, four edge-join counts and dedicated edge stderr. **Change** “one row per changed key” to the disjoint counted rows in section 3, add custody of stderr/changes, specify fields, unit and exits and restore the legacy trace line. | Existing `classify(change,context)` and `context.records(change)` calls work. The classifier can return a class for a whole ambiguous bag. |
| First issue, 2026-11-24, section 4 | **Accept** four verdicts, checker pin, self-check-before-grading, `relations` and `grade(row,ctx)`. **Change** bare key selection to matching arm bags, add explicit relation and class/kind filters, include denominators and receipt in aggregate mode, close `ts-calls` population rules. | One public module may export all five lane 9 relations. Its local helpers and case trees are bound. |
| Adopter amendment, 2026-11-24, `node` | **Accept.** A span can exist independently of an edge. | Inventory reaches `validate(bound)` even on an identity run. |
| Adopter amendment, 2026-11-24, `enclosing_call` | **Accept.** Full-span lookup distinguishes nested and curried calls, and site claims require exact call bounds. | Use the three-argument lookup in section 4. |
| Adopter amendment, 2026-11-24, `program_with_default_lib()` | **Accept.** A separate pinned program supplies default library identities without changing the first checker. | Return the `{program,checker}` pair in section 4. |
| Deferred items in provisional section 5 | **Settle** manifest, gate/report, review artifacts and generic classes in sections 2, 3, 5 and 6. | No generic attribution is applied before lane validation. |

**Inventory disagreements.** The numbers below are the inventory's original fork disagreements. These are the materialized fixtures that name each one.

| Inventory numbers | Row IDs in number order | Adopted behavior |
|---|---|---|
| 1–3 | cmp-002 / cmp-003 / cmp-004 | Preserve the numeric kill line and both roles as attributes. Decode raw or complete gzip streams. |
| 4–6 | join-011 / join-003, join-004 / cmp-017 | Match structured paths, enforce the declared stray policy and ignore the extra wire owner in edge identity. |
| 7–9 | cmp-047, join-012 / cmp-001 / cmp-016, cmp-028 | Keep files on their own endpoints and conserve all copies through residual pairing. |
| 10–12 | cmp-018, cmp-019 / join-013, join-014, join-015, join-016 / join-017 | Skip physical blanks, refuse malformed records, and match the complete mechanism set in the 4b classifier. Parse owner layouts by prefix and arity. |
| 13–15 | cmp-046 / cmp-020 / run-012 | Exclude booleans from counters, fail unattributed rows and require both successful bound arms. |
| 16–18 | run-013 / join-018, join-019, join-020 / oracle-017 | Use no-cache for each analysis, let the classifier select the mechanism vocabulary and verify the checker pin before Node. |

### Previously undetermined materialized rows

All 24 are decided. The kit file contains 15 such rows in 152, including the appended `cmp-053`. The benchmark copy contains nine in 45. Other asserted fields of a row still bind. Each reason below identifies the distinction the competing reading would lose.

| Row ID | Truth of record | Reason for the choice |
|---|---|---|
| cmp-017 | Exit 0, zero changes. | Owner is an extra endpoint field outside the projected identity, and rejecting it would preclude a backward-compatible enrichment. |
| cmp-018 | Exit 0, two occurrences per arm, zero changes. | An empty physical line carries no JSON value and can be skipped without losing an occurrence. |
| cmp-019 | Exit 2, no comparison. | A nonblank malformed record could be a lost occurrence if skipped. |
| cmp-046 | Exit 0, no deltas. | A JSON boolean encodes a flag and coercion to integers would manufacture a counter delta. |
| cmp-053 | One ambiguous row, bags 3/1, count 3, unattributed 1, exit 1. | Choosing an exact precursor in a partly repeated bag would discard the competing killed precursor. |
| join-002 | Attached 0, stray 0, ambiguous 1, unmatched 0, collisions 1, zero change rows, exit 1. | Projecting both sides to line identity prevents hidden bytes from resolving a collision in a line-mode result. |
| join-008 | One unmatched, one independently classified move, unattributed 0, exit 0. | Requiring every unmatched diagnostic trace to fail would impose a policy the supplied classifier did not request. |
| oracle-027 | One lost site/claim in the delta, zero verdicts. | A removed base target belongs in the loss denominator and has no new exact head claim to grade. |
| oracle-029 | One multi-target delta, one head WRONG (`sameB`), zero new CORRECT. | Grading the surviving `sameA` again would inflate changed-claim coverage. |
| oracle-032 | WRONG for `implementation-slot`, first overload formal. | The claimed bytes 16–21 lie on the first bodyless signature. The uniquely mapped receiving formal is on line 2. |
| oracle-034 | UNDECIDED, external callable. | A refutation using declaration data beyond the authenticated source boundary would enlarge the admitted evidence. |
| review-008 | Presence 0, payload 2, ambiguous bags 1, no invented transition. | Two uncancelled occurrences overlap on each side of the anchor, and reporting zero would conceal both differences. |
| gate-005 | Result `retrospective`, exit 1. | Reserving process success for prospective PASS protects callers that gate only on the exit code. |
| gate-006 | Result `fail`, failed step `compare`, exit 1. | The measured step and its zero threshold are both valid, so the failed expectation is a gate outcome. |
| gate-007 | Refused, exit 2, zero steps launched. | Whole-file preflight prevents prior steps from changing state for an invalid graph. |
| bf-011 | `sink_line`, two findings in emission order, `matched` then `off_target`, rules empty. | A single annotation credits one duplicate and the emissions denominator requires both copies. |
| bf-018 | `sink_file`, two `off_target` rows, rules empty. | The same-file finding establishes the closest observed location while the other-file emission remains visible. |
| bf-019 | `ambiguous`, zero finding rows, rules empty. | The second eligible source occurrence makes the line ambiguous independently of channel yield. |
| bn-001 | `sink_occurrence_unmapped`, four nulls. | The named owner survives but delegates to a new helper carrying the sink, which rules out verified deletion. |
| bn-002 | `function_moved`, four nulls. | A unique same-name owner in another file supplies more specific evidence than a missing name. |
| bn-006 | `function_not_found`, four nulls. | The same-file name is gone and body resemblance cannot authorize substituting the renamed callable. |
| bn-010 | `fix_elsewhere`, four nulls. | The changed caller stops passing the original value and the sink owner itself is unchanged. |
| bn-011 | `fix_elsewhere`, four nulls. | The edit is confined to the caller and that independently established exclusion precedes any partial-fix determination. |
| bn-012 | `partial_fix`, four nulls. | The bound adjudication supplied below identifies the wrapped first contribution and the unchanged second. Materialise it alongside the case. |

### Row truth updates

Replace the complete `truth` object for each ID with the JSON below when materialising the successor matrices. The preceding table gives the reason for each decision. Edge bags compare with multiplicity. The oracle keys identify the original claim. Named §4 boundaries use their exact reason codes. `steps_launched` counts observed launches in the gate fixture adapter.

| ID | New truth |
|---|---|
| cmp-017 | `{"exit":0,"changes":[],"unattributed":0}` |
| cmp-018 | `{"exit":0,"changes":[],"unattributed":0,"base_edges":2,"head_edges":2}` |
| cmp-019 | `{"exit":2}` |
| cmp-046 | `{"exit":0,"deltas":{}}` |
| cmp-053 | `{"exit":1,"unattributed":1,"occurrences":{"base":3,"head":1},"changes":[{"kind":"ambiguous","count":3,"base_bag":[{"from_access":"def","to_access":"use","confidence":"exact","doubt":null,"kill_line":null},{"from_access":"def","to_access":"use","confidence":"exact","doubt":null,"kill_line":null},{"from_access":"def","to_access":"use","confidence":"nameonly","doubt":"killed","kill_line":5}],"head_bag":[{"from_access":"def","to_access":"use","confidence":"nameonly","doubt":"sameline","kill_line":null}],"class":null}]}` |
| join-002 | `{"exit":1,"changes":[],"unattributed":0,"collisions":1,"records":{"attached":0,"stray":0,"ambiguous":1,"unmatched":0}}` |
| join-008 | `{"exit":0,"changes":[{"kind":"label_moved","count":1,"from":{"from_access":"def","to_access":"use","confidence":"exact","doubt":null,"kill_line":null},"to":{"from_access":"def","to_access":"use","confidence":"nameonly","doubt":"sameline","kill_line":null},"base_bag":[{"from_access":"def","to_access":"use","confidence":"exact","doubt":null,"kill_line":null}],"head_bag":[{"from_access":"def","to_access":"use","confidence":"nameonly","doubt":"sameline","kill_line":null}],"class":"fixture-change"}],"unattributed":0,"records":{"attached":0,"stray":0,"ambiguous":0,"unmatched":1}}` |
| oracle-027 | `{"delta":"lost","delta_count":1,"verdicts":[]}` |
| oracle-029 | `{"delta":"multi-target","delta_count":1,"verdicts":[{"key":{"site":{"file":"cases.ts","line":23,"start_byte":463,"end_byte":470},"target":{"file":"cases.ts","name":"sameB","start_line":22,"end_line":22}},"verdict":"WRONG","why":"callee-implementation"}]}` |
| oracle-032 | `{"verdicts":[{"key":{"from":{"file":"cases.ts","base":"actual","fields":[],"line":3,"start_byte":177,"end_byte":183},"to":{"file":"cases.ts","base":"value","fields":[],"line":1,"start_byte":16,"end_byte":21}},"property":"implementation-slot","verdict":"WRONG","why":"implementation-slot"}]}` |
| oracle-034 | `{"verdicts":[{"key":{"from":{"file":"cases.ts","base":"actual","fields":[],"line":3,"start_byte":138,"end_byte":144},"to":{"file":"cases.ts","base":"value","fields":[],"line":2,"start_byte":55,"end_byte":60}},"verdict":"UNDECIDED","why":"external-callable"}]}` |
| review-008 | `{"presence_changes":0,"payload_changes":2,"ambiguous_bags":1,"rule_transitions":[]}` |
| gate-005 | `{"exit":1,"reason":"retrospective","result":"retrospective"}` |
| gate-006 | `{"exit":1,"reason":"threshold-failed","result":"fail","failed_step":"compare"}` |
| gate-007 | `{"exit":2,"reason":"unknown-step","result":"refused","rejected_step_kind":true,"steps_launched":0}` |
| bf-011 | `{"line_hit":"sink_line","findings":[{"file":"lib/index.js","line":4,"match":"matched"},{"file":"lib/index.js","line":4,"match":"off_target"}],"rules":[]}` |
| bf-018 | `{"line_hit":"sink_file","findings":[{"file":"lib/index.js","line":3,"match":"off_target"},{"file":"lib/other.js","line":2,"match":"off_target"}],"rules":[]}` |
| bf-019 | `{"line_hit":"ambiguous","findings":[],"rules":[]}` |
| bn-001 | `{"outcome":"sink_occurrence_unmapped","columns":{"finding":null,"reach":null,"function_change":null,"sink_removed":null}}` |
| bn-002 | `{"outcome":"function_moved","columns":{"finding":null,"reach":null,"function_change":null,"sink_removed":null}}` |
| bn-006 | `{"outcome":"function_not_found","columns":{"finding":null,"reach":null,"function_change":null,"sink_removed":null}}` |
| bn-010 | `{"outcome":"fix_elsewhere","columns":{"finding":null,"reach":null,"function_change":null,"sink_removed":null}}` |
| bn-011 | `{"outcome":"fix_elsewhere","columns":{"finding":null,"reach":null,"function_change":null,"sink_removed":null}}` |
| bn-012 | `{"outcome":"partial_fix","columns":{"finding":null,"reach":null,"function_change":null,"sink_removed":null}}` |

**Adjudication input for bn-012.** Materialise the following JSONL row with the benchmark fixture and pass it as `--adjudications`. The spans identify the two contributions in order in each tree. An invocation omitting this input has no partial-fix adjudication. Its ordinary scoring control is bn-031.

```json
{"class":"code-injection","entry":"bn-012-fixture_1.19.0","id":"CVE-0000-012","vulnerable_tree_sha256":"46226178ad429efef79ea0f93e0e4d3c3887ad6cc068aa559266e74c9ece8403","fixed_tree_sha256":"2983aa38dbd1dabc46ca025933101e1b745b901d6bf3d160aca7a7cc187812cd","sink":"lib/index.js:3:3","fact":"partial_fix","evidence":{"before":[{"file":"lib/index.js","start_byte":72,"end_byte":83},{"file":"lib/index.js","start_byte":84,"end_byte":87}],"after":[{"file":"lib/index.js","start_byte":151,"end_byte":159},{"file":"lib/index.js","start_byte":160,"end_byte":163}],"explanation":"The first parameter contribution is wrapped in quote in the fixed call. The second parameter is still appended directly to the same command argument."}}
```

### Truth disputes

Each mention in each round is represented below. T1–T5 are the five terra rounds, S1–S2 in this table are sol rounds, A1 is astra. T1–T3 filed no disputes. The 33 mentions address 18 IDs.

| Row ID | Rounds raising dispute | Ruling and reason |
|---|---|---|
| cmp-030 | S1, S2, A1 | Matrix wins on representation: one row with count 2 preserves both transitions and pins the classifier's call unit. |
| cmp-053 | A1 | Astra wins because both possible predecessors survive, so an unclassified 3/1 bag fails. |
| join-002 | S1, A1 | PLAN's line projection wins over the raw-byte stray reading because the selected mode governs both sides of the join. |
| join-008 | S2, A1 | Sol and astra win because one unmatched record cannot invalidate an independently classified move without a validator veto. |
| oracle-017 | S1 | Sol's refusal agrees with the folded matrix because the checker digest and self-check tree are prerequisites even at the same version. |
| oracle-020 | S1 | Sol's invocation failure agrees with the folded matrix because UNKNOWN violates the module contract and yields no verdict. |
| oracle-034 | A1 | Choose the authenticated in-root reading over astra's WRONG because the external callable is outside the proof domain. |
| review-008 | S1, A1 | Both enumerators win on nonzero payload difference because cancellation leaves two unmatched overlapping occurrences. |
| terra2-review-004 | T4, T5, S1, S2, A1 | All five disputants win because serialization preserves omitted versus explicit null, giving one payload change. |
| terra3-oracle-003 | S1, S2, A1 | Both sol rounds and astra win because a later spread leaves the already established first slot intact (CORRECT). |
| sol-review-006 | A1 | Astra wins because omission and an empty `related_lines` array remain different serialized states. |
| sol2-review-005 | A1 | Astra wins because omission and an empty `related_files` array remain different serialized states. |
| bf-011 | S2, A1 | Both win on two emitted rows because one-to-one annotation credit leaves the second identical copy off-target. |
| bf-019 | S2, A1 | Both win on `ambiguous` because occurrence eligibility is a source property even with no emitted finding. |
| bn-001 | S2, A1 | Both win on exclusion because the helper retains the former sink and the retained outer name distinguishes this from an owner move. |
| bn-010 | A1 | Astra wins on exclusion because the fix is outside the sink function. |
| bn-011 | A1 | Astra wins on exclusion because the caller-only edit establishes `fix_elsewhere` first. |
| bn-012 | S2, A1 | Both win on `partial_fix` with the adjudication in Row truth updates. Its before/after spans identify the surviving second contribution. |

### Units and compared text

| Representative row IDs | Matrix assertion |
|---|---|
| cmp-001, cmp-030, cmp-051, cmp-052 | Compare change **row** multiplicities and complete per-arm bags. `unattributed` counts rows, even for a whole ambiguous bag. Check scalar occurrences and both side totals as defined in section 3. |
| join-003, join-017, join-021 | Count every typed occurrence in its own category. Compare arm, join state and required tag/mechanism; preserve order/multiplicity of the unjoined records provided to the hook. |
| oracle-010, oracle-016, oracle-020 | Stage-T `why` strings are explanatory labels. Compare verdict and keyed occurrence, arm when selected, tuple/exit and any pinned denominator. Require a nonempty explanation on every actual verdict. Question `property` is an exact field in the verdict and self-check goldens. Successor rows that name a §4 built-in boundary also check its literal `why` code. |
| gate-001, gate-006, gate-009 | Stage-T `reason` is a diagnostic label, not a string golden. Compare exit, result, failed step, report sections, exact prediction bytes and digest where asserted. |
| bf-011, bn-002, bn-026 | Compare `line_hit,match,outcome` (including the **exact** closed exclusion name) and columns, counts and RuleRefs. `reason_detail` and exception message prose are diagnostics. |
| join-007, cmp-021 | Validation and classifier diagnostics must identify the failing counter/class or exception type when asserted. The separately predicted G-repro control failure counts/causes are compared, not punctuation in diagnostics. |

### Remaining materialized fixtures

The following 173 rows and the 24 decisions above exhaust the two input matrices. For a completed comparison, a classified result without a listed failure exits 0. Unmentioned join counts are zero. In benchmark tuples the order is finding, reach, function_change, sink_removed. Each exclusion has four nulls. The matrix checks its supplied keys, complete residual bags and pinned fields, in addition to the observables shown here.

| Row ID | Input and expected observable |
|---|---|
| run-001 | Ignored generated file under the git query root → 3 before version, no receipt. |
| run-002 | Last command rewrites a source → 4, input_changed. |
| run-003 | Clean pinned corpus and successful commands → 0, ok, binary hash and corpus commit bound. |
| run-004 | Checked-out second commit differs → 3 before version, no receipt. |
| run-005 | Untracked source under root → 3 before version, no receipt. |
| run-006 | Prediction rewritten during analysis → 0, ok, SHA-256 of the original bytes. |
| run-007 | Only the edges command gets the requested trace switch → 0, ok. |
| run-008 | Guard refuses before launch → 75, no binary invocation or receipt. |
| run-009 | Verify prints failing key and exits 3 for the second commit. |
| run-010 | Archive verifies, exit 0, perf manifest SHA-256 `1cde38ecf4ebe4cb0b36c72fbbae296fc0e34215d19116dbdb624800650b4d36`. |
| run-011 | The sample match statement fails the Python 3.9 grammar check. |
| run-012 | Middle command exits 7 → stop, 4, failed receipt with the child exit. |
| run-013 | Call-stats argv uses --no-cache → 0, ok. |
| run-014 | Link target text changes → 4, input_changed. |
| run-015 | Tracked edit with matching HEAD → 3 before version, no receipt. |
| run-016 | Only .git contents change → 0, ok. |
| run-017 | Wrong version → 3 after version only, no measurement receipt. |
| run-018 | Existing output file → 2, preserve its contents. |
| cmp-001 | One exact → two identical nameonly: one move and one addition on one key, 2 occurrences, 0 unattributed. |
| cmp-002 | Kill line 13→14 → one attributed move. |
| cmp-003 | Source access use→def → one attributed move. |
| cmp-004 | Row permutation and different gzip headers → zero. |
| cmp-005 | Same-line label swap at distinct bytes → two moves. |
| cmp-006 | Same swap projected to line → zero changes, one collision. |
| cmp-007 | Mixed dumps, explicit bytes → 2. |
| cmp-008 | Mixed dumps, auto → 2. |
| cmp-009 | Same mixed-capability data with compatible receipts and forced line → zero changes, one collision. |
| cmp-010 | Partial end bound, bytes → 2. |
| cmp-011 | Partial end bound, forced line → zero changes, zero collisions. |
| cmp-012 | Dotted single field versus two fields → one removal and one addition. |
| cmp-013 | Two distinct residuals on both sides → ambiguous 2/2, one unattributed row, exit 1. |
| cmp-014 | One identical occurrence → zero changed keys. |
| cmp-015 | A zero-width destination with a new label → one move. |
| cmp-016 | Two identical copies unchanged per arm in line mode → zero changes, one collision. |
| cmp-020 | Unclassified removal → one unattributed row, exit 1. |
| cmp-021 | Classifier divides by zero → one unattributed removal, ZeroDivisionError recorded, exit 1. |
| cmp-022 | Undeclared class → one unattributed removal, returned name recorded, exit 1. |
| cmp-023 | Different corpus input digests → 2. |
| cmp-024 | Stale output digest → 2. |
| cmp-025 | Failed receipt → 2. |
| cmp-026 | Sites option differs → 2. |
| cmp-027 | Only binary and trace environment differ → admitted, zero changes. |
| cmp-028 | Two copies to one → one removed occurrence. |
| cmp-029 | One copy to two → one added occurrence. |
| cmp-030 | Two identical transitions → one move row, count 2. |
| cmp-031 | Two distinct to three identical → two moves, one addition. |
| cmp-032 | Three identical to two distinct → two moves, one removal. |
| cmp-033 | Two distinct to two identical → two moves. |
| cmp-034 | Two identical to two distinct → two moves. |
| cmp-035 | After cancellation the residual is mixed 2/3 → one ambiguous unattributed row, exit 1. |
| cmp-036 | Reversed order, same 2/3 bag attributed as a whole → exit 0. |
| cmp-037 | Three independent keys → one addition, one removal, one move. |
| cmp-038 | Fixed case → two moves, one addition, one removal. Both bags rebuild for all 200 additional seed-731 pairs. |
| cmp-039 | Line-only data on both arms → automatic line, zero changes, one collision. |
| cmp-040 | Line-only nonempty data and explicit bytes → 2. |
| cmp-041 | 100,000 distinct keys per arm → zero changes, Python traced peak <100,663,296 bytes. |
| cmp-042 | Truncated gzip after signing → 2 on digest. |
| cmp-043 | Truncated gzip signed as-is → 2 on decoding. |
| cmp-044 | Allowed nested x 1→2 and y 2→1 → both printed with signed deltas, exit 0. |
| cmp-045 | Allow x.* only → report x.a and y, y forbidden, exit 1. |
| cmp-047 | Destination file differs → one removal and one addition in line mode, zero collisions. |
| cmp-049 | JSON member order changes → zero. |
| cmp-050 | Destination access changes → one move. |
| cmp-051 | One shared copy cancels → a single removed bag of two residual attributes, one unattributed row, exit 1. |
| cmp-052 | One versus two different successors → ambiguous 1/2, one unattributed row, exit 1. |
| join-001 | Two same-line moves, trace at byte 154 → one attached, other move unattributed, exit 1. |
| join-003 | One unchanged match → one stray, zero changes, exit 1. |
| join-004 | Same match, strays allowed → one stray, exit 0. |
| join-005 | One census edge attaches to kill:alias, two writes unjoined → one changed key, exit 0. |
| join-006 | Two edge records attach to one change, one cert unjoined → two attachments, one changed key. |
| join-007 | Attached change classifies. Observed counter +20 versus predicted +1 → validation failure, exit 1. |
| join-009 | Trace matches unchanged 158 only → one stray, one unattributed move, exit 1. |
| join-010 | Two distinct destination bytes → two moves each with one attachment, exit 0. |
| join-011 | Structured [] field disambiguates the indexed edge → one attachment and classified move. |
| join-012 | Trace names consumer file only → one cross-file attachment and classified move. |
| join-013 | Sole expected kill:alias tag → one attributed move. |
| join-014 | Sole expected alias-d3 tag for the fixture transition → one attributed move. |
| join-015 | No required tag → one unattributed move, exit 1. |
| join-016 | Both competing tags attach → one unattributed move, exit 1. |
| join-017 | Four owners retained in trace order, guard/arrow/parameter/loop predictions sum to −1 → no edge records or failures. |
| join-018 | Lane declares alias-d2c → one classified move, one attachment. |
| join-019 | Lane omits alias-d2c → one attachment, one unattributed move, exit 1. |
| join-020 | alias-d2c fires on unchanged occurrence → one stray, exit 1. |
| join-021 | Identical head copies → one ambiguous record, no attachment, move plus addition unattributed, exit 1. |
| oracle-001 | Actual → matching formal in identity → CORRECT. |
| oracle-002 | Read of same declaration after overwrite → CORRECT for binding only. |
| oracle-003 | Rest formal → UNDECIDED. |
| oracle-004 | Spread actual → UNDECIDED. |
| oracle-005 | Outer shadow claim WRONG, inner declaration to its read CORRECT. |
| oracle-006 | Identifier in type query → WRONG. |
| oracle-007 | Same-line other declaration → WRONG. |
| oracle-008 | Inner read and local declaration agree → CORRECT. |
| oracle-009 | Missing destination bounds → INADMISSIBLE. |
| oracle-010 | Authenticated span covers other identifier → WRONG. |
| oracle-011 | Assignment target → WRONG. |
| oracle-012 | Use→use with no extension → UNDECIDED. |
| oracle-013 | Different argument position → WRONG. |
| oracle-014 | Unique different in-root callable → WRONG. |
| oracle-015 | Unresolved identifier at call → UNDECIDED. |
| oracle-016 | Changed live source digest → INADMISSIBLE for selected edge. |
| oracle-017 | Correct version string, wrong checker bytes → 3 before Node, no verdicts. |
| oracle-018 | Fixture edited after passing self-check → 3 before Node, no verdicts. |
| oracle-019 | Two lexical reads → two CORRECT, all other buckets zero. |
| oracle-020 | Module returns UNKNOWN → 1, no published verdicts. |
| oracle-021 | Swapped targets at same-line calls → two WRONG. |
| oracle-022 | Unique inner declaration, exact resolved target, null convenience field → CORRECT. |
| oracle-023 | Outer declaration encloses actual inner target → WRONG for the outer claim. |
| oracle-024 | Replace outer claim by inner → one retargeted delta, one head CORRECT. |
| oracle-025 | One-run census → one CORRECT, no deltas. |
| oracle-026 | Add sameB site → one new delta, one head CORRECT. |
| oracle-028 | Demote exact to nameonly → one confidence-only delta, one site and one head nonexact site, zero verdicts. |
| oracle-030 | Dropped call → one site, one drop, zero verdicts. |
| oracle-031 | No checker declaration for callee → UNDECIDED. |
| oracle-033 | Union has two callable owners → UNDECIDED. |
| oracle-035 | Call target resolves outside root → UNDECIDED. |
| oracle-036 | Two declarations share name/file/start line → UNDECIDED. |
| review-001 | One related line disappears → presence 0, payload 1. |
| review-002 | Added rule ID → presence 0, payload 1, one sorted-ID transition. |
| review-003 | Cancel same-line peer, revised description → presence 0, payload 1. |
| review-004 | Multi arm uses aggregate once → one presence loss, zero payload changes. |
| review-005 | Permuted findings and set members → zero presence and payload. |
| review-006 | Missing rules → empty list → presence 0, payload 1, transition null→[]. |
| review-007 | Same-anchor peer removed → presence 1, payload 0. |
| review-009 | Single envelope reads only findings → zero differences. |
| review-010 | Head exits 7 → ordinal 2 failed, listed in failed totals. |
| review-011 | Spec digest differs → ordinal 1 refused before either arm. |
| review-012 | Spec includes only src/a.ts, Modified, lines [3,4] sorted and unique. |
| review-013 | Scratch reviews leave source-clone HEAD and dirty bytes intact → zero differences. |
| review-014 | Candidate ordinals [5,3,1,4,2], size 2, seed 11 → publish ordinals 4 and 5 on both draws. |
| review-015 | All 120 ordinals accounted, manifest contains neither files nor diff_lines. |
| gate-001 | Changed prediction bytes, preserved mtime → refusal 2. |
| gate-002 | Missing prediction digest in reused prospective receipt → refusal 2. |
| gate-003 | Prediction changes between steps → refusal 2. |
| gate-004 | Missing prediction file → 2 before measuring. |
| gate-008 | Matching digest → 0, all eight report sections in section 6 order. |
| gate-009 | Prediction text including final LF and SHA-256 `def6242ecf18779072eca006fc2cc09caed1a5320557d4a20a28aeb5ed8df493` appear verbatim, exit 0. |
| gate-010 | Exact bound prediction, decision digest, step ID, corpus/options and observed executable SHA/version → admissible reuse, exit 0. The adapter completes the pre-fold stub as specified in §6. |
| bf-001 | One sink-line match, source omitted → sink_line, one matched, rules []. |
| bf-002 | Other line same file → sink_file, one off_target. |
| bf-003 | Same line number in a different file → elsewhere, one off_target. |
| bf-004 | Shell and source categories only → none, no sink rows. |
| bf-005 | Source finding only → none, no sink rows. |
| bf-006 | Child error → error, no sink rows. |
| bf-007 | Timeout → error, no sink rows. |
| bf-008 | Two eligible calls on one line → ambiguous, one ambiguous row. |
| bf-009 | Two distinct emissions on ambiguous GT line → ambiguous, two ambiguous rows. |
| bf-010 | Annotated line among 21 findings → sink_line, one matched and 20 off_target. |
| bf-012 | Match copies full js-child-process RuleRef with CWE-78 → sink_line. |
| bf-013 | Only off-target has rules → sink_line, one matched, one off_target, case rules []. |
| bf-014 | No ground truth and no invocation → not_scored, no rows. |
| bf-015 | Successful empty list → none, no rows. |
| bf-016 | GT-line source plus other-line sink → sink_file, one off_target. |
| bf-017 | Four emitted sinks → sink_line, only line 5 matched, three off_target. |
| bn-003 | Mapped sink line 3, unrelated observation on line 4 → verified_pair (false,false,changed,false). |
| bn-004 | Literal-only edit → verified_pair (true,true,changed,false). |
| bn-005 | Verified deletion → verified_pair (null,null,changed,true). |
| bn-007 | Duplicate same-file owner → function_ambiguous. |
| bn-008 | Two same-callee calls in mapped owner → sink_occurrence_ambiguous. |
| bn-009 | Version metadata alone changes → verified_pair (true,true,unchanged,false). |
| bn-013 | Trim whitespace and leading equals → 1.20.0 newer, verified_pair (true,true,changed,false). |
| bn-014 | Nonversion text → fixed_version_unparseable. |
| bn-015 | Range → fixed_version_unparseable. |
| bn-016 | Equal version → fixed_not_newer. |
| bn-017 | Earlier version → fixed_not_newer. |
| bn-018 | Prerelease below corresponding release → fixed_not_newer. |
| bn-019 | Commit without version → fix_commit_only. |
| bn-020 | Neither version nor commit → no_fixed_version. |
| bn-021 | Neither vulnerable channel positive → no_vulnerable_positive. |
| bn-022 | Function-only reach plus same-file off-target finding → no_vulnerable_positive. |
| bn-023 | Vulnerable finding alone supplies control → verified_pair (true,true,changed,false). |
| bn-024 | Vulnerable traced result alone supplies control → verified_pair (true,true,changed,false). |
| bn-025 | Missing sink/GT → gt_unavailable. |
| bn-026 | Three entries yield no_fixed_version, fixed_not_newer and no_vulnerable_positive respectively. Zero verified, unmet. |
