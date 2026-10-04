# S2 measurements — owner fail-closed repair

[MEASURED] Planning HEAD `6e4e0ef19de578d4865d7297eb5cf72a5b6a27a6`; immutable merged-P2 main `4e592daa7858a195eb3a9eb77c83dfbc763b49fa`. Product source is the uncommitted src/tests prototype plus the repaired whitelist. No Git writes or F reads. Current source/tool/receipt hashes are in BUILD-MANIFEST.json; older +179 receipts are historical controls, not current yield.

## Yield against main

All changed rows are new singleton Exact `ImportQualified` edges on base drops. Every row is CORRECT with native module, caller ProjectService owner and full FunctionId file/name/line-span agreement. X and installed X have the same source snapshot and are not additive.

| Mechanism | X | Installed X | R | T |
|---|---:|---:|---:|---:|
| Named class static method | 7 | 7 | 0 | 0 |
| Named class function-valued static field | 125 | 125 | 0 | 0 |
| Declared/re-exported namespace or literal object | 0 | 0 | 0 | 0 |
| **New CORRECT Exact rows** | **132** | **132** | **0** | **0** |
| Lost or changed populated base rows | **0** | **0** | **0** | **0** |

[MEASURED] `target/s2-plan/namespace-final-public/summary.json`. Complete base streams are replayed from the retained source-bound base, hash-checked against immutable main and byte-checked against S2-0. Fresh head/facts/native oracle runs require unchanged source and all native configuration/input hashes. Every corpus completed; no key/metadata/source/binary/config drift was admitted.

| Complete-stream check | X | Installed X | R | T |
|---|---:|---:|---:|---:|
| All source sites | 19,219 | 19,219 | 953 | 61,712 |
| Populated base rows, byte-preserved | 10,776 | 10,776 | 216 | 27,452 |
| Low associated S2 rows | 552 | 552 | 30 | 3,167 |
| Native-proven callable low rows | 303 | 303 | 0 | 2,086 |
| Prior lane-P gain rows, byte-preserved | 3,129 | 3,129 | 0 | 0 |
| Changed rows unproven or ownership-disagreeing | 0 | 0 | 0 | 0 |
| Whole-stream UNJOINABLE | 2,967 | 2,967 | 207 | 7,473 |

[MEASURED] Lane-P receipt `namespace-final-lane-p.json` joins all original reference/base/head keys and preserves every populated P2 row. The old supplied reference identifies the prior gain population; it is not used as S2's base. Its exact source revision remains inherited rather than independently reconstructed. Preserving all P2 bound rows also preserves the inherited P2 +8 subset. Whole-stream UNJOINABLE retains its denominator and cannot certify a gain.

## Cost of S2-O6: 132 of the former 179 survive

The refusal counts are site rows, with one primary cause per row. Independent causes can overlap; source witnesses and the actual head refusal facts are retained in each X corpus's `yield-refusals.json`. The partition is identical on installed X.

| Primary refusal cause | Rows | Qualifiers / evidence |
|---|---:|---|
| Qualifier member writes in defining module | 25 | ShapeCache 17, AnimationController 3, SnapCache 5. Examples: `ShapeCache.cache = ...`, `AnimationController.scheduledFrame = ...`, `SnapCache.referenceSnapPoints = ...`. Named self-writes inside class bodies now refuse lexically even when the old scoped walk found the class's own binding rather than the program binding. |
| Member value/chained reads, including conservatively mapped this | 11 | FileStatusStore 1, LocalData 7, LibraryIndexedDBAdapter 1, StoreDelta 2. Examples: `LibraryIndexedDBAdapter.idb_name`, `this.store`, `this.fileStorage`, `this.elements`. Instance `this` can be refused conservatively too; this is an explicit yield cost, not evidence of an actual class-object write. |
| Visible importer namespace argument escape | 11 | BoundElement 4, BindableElement 4, Delta 2, StoreChange 1. `vi.spyOn(sizeHelpers, "isInvisiblySmallElement")` passes the namespace imported from `@excalidraw/element`; its proven index barrel exports these qualifier identities. Unproved namespace-value closure revokes those identities conservatively, without claiming that this helper actually mutates each class. |
| **Refused** | **47** | **179 − 47 = 132** |

Survivors: UI 43, API 52, Keyboard 34 and EditorLocalStorage 3. The earlier 179 prototype was not safe admission evidence; the 47 are retained base outcomes, not losses of landed main edges. The other 124 native-callable opportunities outside the old 179 remain behind earlier proof boundaries (120 conservative class/module/capture cuts plus four call-result cases); no exhaustive recovery partition is claimed for them.

T remains base under the retained ownership guards. Its 707 facts files supply zero admitted module proofs; root/output default-exclusion, ambient and reference costs are not relaxed. The explicit-exclude synthetic control proves one ownership cut; it does not establish that this is T's sole blocker.

## Controls and final gates

All receipts below are under `target/s2-plan/` and bound to the final source manifest. The stop-hook coverage audit found the declared/module namespace implicit receiver omission. The bounded carrier repair and 18 valid RED/GREEN cases (plus grammar negatives and direct-this positives) precede this final run. Earlier full-suite receipts are historical and superseded; only namespace-final certifies these bytes.

| Gate | Result | Receipt |
|---|---|---|
| Final offline release + exact-lock facts build | PASS | namespace-final-release-build.log, namespace-final-facts-build.log |
| Full MCP nextest after last semantic edit | **5,154 passed /0 failed /1 existing skipped**, 170.696s | namespace-final-nextest.log |
| MCP doctests | **2 passed** | namespace-final-doctests.log |
| Targeted S2 integration | **17 passed**, entire escape population collected before asserting | namespace-whitelist-targeted.log |
| E5 closed identity controls, both grammars | **218 scenarios /218 keep base /0 wrong admissions**; old prototype admits 195 statically CORRECT wrong rows | namespace-e5-green/summary.json |
| Native synthetic base/head/oracle controls | **92 scenarios /27 new CORRECT /0 loss /0 unproven** | namespace-final-native-controls/summary.json |
| Advisory scoped mutgate, scope file, one worker | **8/14 selected /8 admissible /7 killed /S2-02 SURVIVED**; S2-14 whitelist KILLED | namespace-final-mutgate/summary.json |
| fmt | PASS | namespace-final-fmt.log |
| Same-environment base/head all-target MCP Clippy | PASS; **371/371 warnings /0 new** | failclosed-clippy-base.jsonl, namespace-final-clippy.jsonl, namespace-final-clippy-comparison.json |
| Immediate-rebuild Tier-A matrix | **182 OK /0 regression /0 skip**, existing baselines unchanged | namespace-final-matrix-build.log, namespace-final-matrix.log |
| Alias-write Tier-A fixtures on base/old/head | Base OK, old prototype RED, repaired head OK in both grammars | namespace-final-tier-a-red-green.json |
| S1b-4 controls | **411 controls /639 sites /822 byte-identical comparisons /stderr 0** | namespace-final-s1b.json |
| Lane-P complete public preservation | All prior gain and populated base rows unchanged | namespace-final-lane-p.json |
| Controller wrapper, public positive/negative self-tests | Changed 1 CORRECT and changed 0; aggregate schema contains no private paths | namespace-controller-public-selftest.json |

S2-02 is a **SMELL: coverage gap**, not a demonstrated wrong output and not proved equivalent. Six new-source anchors remain unselected in the advisory run; the tracked comment marker selects S2-14 without another behavioral mutation. Authoritative all-14 coverage is still a controller gate after commit. The existing skip is `resolution_test::slice_elem_variant_reserved`, reserved for a future increment. No unrelated failures were re-baselined or repaired.

Tier-A used the installed Python 3.12 CLI after the immediate release rebuild. No uv dependency installation/network was attempted. Tier-A quick remains required before independent review; no review was dispatched. Full multi-corpus Tier-A remains human-triggered.

## F and exclusions

[INHERITED from OQ-s2/controller] F base census: 974 low /32 callable-low, comprising 26 instance methods and six default function members; zero static-class rows and 1,271 UNJOINABLE. These counts are not a head result. The planner never opened F. Updated CONTROLLER-s2.sh accepts base/head/head-facts/TypeScript and reports changed-row correctness, complete population, losses, unproven rows and ownership/span agreement. The public self-tests exercise the wrapper only on public fixtures.

Not verified: private F head acceptance; authoritative all-14 mutation coverage; independent review or adoption; Tier-A quick/full multi-corpus; optional detached-owner/all-feature sweeps; Linux/case-sensitive/concurrent-tree behavior or quiet-host performance. O1/O2 retain controller interim positions pending owner confirmation for adoption. No commit/push/merge/remote backup was performed. Local snapshots and frozen binaries provide custody; the controller must preserve/commit them before removing this checkout.
