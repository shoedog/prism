> Historical record. S2-O7/R2 supersede current-state runtime-scope, STOP, binary and completion claims below. F3/F4/F7 are now out of model by owner decision; their old witnesses remain valid under the old contract. Read REPAIR-R2.md and VERIFICATION.md for current bytes.

> Historical R1 snapshot. R1b authorization and measurements supersede current-state, pending-family, gate, binary and completion assertions here. Read [REPAIR-R1b.md](REPAIR-R1b.md) and VERIFICATION for current bytes.

# Specification R1 repair — mandatory yield STOP

2026-10-04T06:49:39.150311+00:00. Source starts from c35719e1; docs from b8f5b2f3. All 770 initial inputs were verified against committed prototype Git objects. Read both complete reviews before repair; SPEC §2a was written before source edits. Review cap remains two: R1 used; no R2 dispatched here.

The channel table covers module namespaces/static and dynamic imports/require/default/re-export; inherited and own receiver calls/this/super/new.target; instances/constructor/prototype walks; Object/Reflect/global lookups; evaluated code/eval/Function/with; and incomplete syntax. Every channel has a closed-rule/test obligation. F4–F8 obligations remain pending, not implied complete by a table entry.

| Family | Choice and reason / disposition |
|---|---|
| F1 | Captured own Callable-member pairs joined locally and through every importer; class heritage refuses. This closes the reviewer prototype helpers and inherited-static repros without blacklist spellings. Measured cost 0. |
| F2 | Drop every construction use. Also measured discard-only construction with no explicit constructor/heritage; equal yield, so drop has the simpler sound proof. Guard source and executable receipts retained. |
| F3 | Refusal-only dynamic import/require/TS import-require source facts, exported-name joins, and conservative possible-indexed-target revocation for opaque or unresolved namespace selection. Namespace declaration names have own-declaration authority only. Implemented as its own candidate patch; **not selected/adopted** because yield STOP triggered. |
| F4 | Pending namespace-identity cascade, including forwarding/named/star exports. No option chosen after STOP. |
| F5 | Pending explicit new.target/super carrier proof and regressions. Heritage/construction cuts protect some examples incidentally; no complete certificate. |
| F6 | Pending execution-context or all-plausible-carrier mapping for initializer/arrow this. |
| F7 | Pending evaluated-code closure refusal. |
| F8 | Pending global incomplete-file refusal. F3 retains its acquisition facts but does not close arbitrary parse-incomplete writers. |

| Cut / option | X | Installed X | R | T | Delta against +132/+132/0/0 |
|---|---:|---:|---:|---:|---|
| c35719e1 reference | 132 | 132 | 0 | 0 | baseline |
| F1 own-members + heritage | 132 | 132 | 0 | 0 | 0/0/0/0 |
| F2 discard-only + no explicit constructor/heritage | 132 | 132 | 0 | 0 | 0/0/0/0 |
| F2 drop-all (chosen before STOP) | 132 | 132 | 0 | 0 | 0/0/0/0 |
| F3 complete conservative candidate | 0 | 0 | 0 | 0 | -132/-132/0/0 |

All four full corpus replays completed for every cut/option. Every changed row is CORRECT_STATIC_BINDING with matching native module, caller owner and full span; zero losses/unproven rows or input drift. X snapshots are not additive. F3 final raw streams equal main byte-for-byte. **Single X cut 132 >10 and cumulative X 0 <110: mandatory STOP.** No weaker source-spelling exception, new refusal cut, F4–F8 fold or F3 option selection followed. F1/F2-only +132 remains an incomplete safety option; F3 +0 is the measured conservative candidate. A yield-preserving closed namespace-target proof is a controller/owner design option with yield **unmeasured**, not a selected implementation.

Gates: full MCP nextest 5,160 pass/0 fail/1 existing skip; doctests 2 pass; fmt PASS; all-target MCP Clippy PASS (371 warnings, no fresh attribution); advisory mutgate 17/23 selected, 17 admissible, 16 killed, S2-02 survives, all nine new mutants killed; matrix 182 OK; S1b-4 411 controls/639 sites/822 byte comparisons; all lane-P/public main rows unchanged. 62 retained executable control rows keep actual main on final tools. VERIFICATION gives exact commands, caveats and receipts.

Product repair delta (eight files; other dirty paths are pre-existing prototype bytes):

- `mutants/lane-s2-import-qualifiers.json`
- `src/ast/js_import_qualifiers.rs`
- `src/call_graph.rs`
- `src/cpg_cache.rs`
- `src/js_exports.rs`
- `src/js_import_qualifiers.rs`
- `src/navigation/call_edge_cache.rs`
- `tests/integration/js_import_qualifiers_test.rs`

Suggested product commits, controller only: `fix(resolution): close S2 own-member and construction escapes` from `/Users/wesleyjinks/prism-evidence/s2/repair-r1/F1-F2.patch`; separate candidate `fix(resolution): fail closed on unproved runtime namespaces` from `/Users/wesleyjinks/prism-evidence/s2/repair-r1/F3-only.patch`. The second remains unadopted at STOP. The combined patch reproduces tested current bytes. Any changed variant requires fresh cache epoch/source/binary binding, measures and gates before acceptance; current full gates certify the combined candidate.

Docs/metadata changed: SPEC, IMPLEMENTOR, MEASUREMENTS, PROBES, README, OQ-s2, HANDOFF, FILES, VERIFICATION, REPAIR-R1, BUILD-MANIFEST and OWNED-FILES in this packet, plus local excluded root VERIFICATION.md. Suggested commit: `docs(plan): record S2 R1 yield stop and bound repair handoff`. No Git writes performed; controller commits after preserving custody and owner selection.

Excluded: F4–F8 completion, private F, authoritative all-23 gate, Tier-A quick/full, R2 review/adoption, all-feature/platform/performance and runtime execution of every synthetic/public row. Source/evidence remain under `/Users/wesleyjinks/prism-evidence/s2/repair-r1`; hashes and patches are reviewable without private corpus access.
