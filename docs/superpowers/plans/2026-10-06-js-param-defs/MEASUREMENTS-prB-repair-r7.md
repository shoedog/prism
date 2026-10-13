# PR-B R7 final measurements

Certificate COMPLETE and clean against f8c768b3 on every admitted root, but PR-B is STOPPED by the main byte-table gate. All583 SecBench roots plus X, Xi and T are accounted for:579 admitted,7 explicit main-producer exclusions,0 unprocessed. After documented ES overrides, SecBench retains6 LOST CORRECT and82 ADDED WRONG, all in redos/react-native_0.63.0-rc.0. The complete88-row STOP population is preserved; no repair, reclassification, new override or restart followed. Exclusions have no semantic credit.

One final owner-authorized R7 loop completed on2026-10-09/10 UTC; no restart, no Git writes, network, corpus-package execution, frontend-portal access or delegation. Actual transcript modelgpt-6.1-sol. Docs HEAD30c18a7ae210e342f3353d65eac1da6cf7c35788; inherited726 source/test/mutant files exactly127cc2573d70b0f61d4d2243727302bdd218735b, no extras. Certificate comparatorf8c768b3; main da0604b3. Final727-file source manifest, three binary SHAs, full586-root set, every input/output/producer binding and complete table partitions independently replay-audited with zero mismatches: final-invariants.json. Report written2026-10-10T02:30:22.765374+00:00.

## Complete Step1 failure population, sealed before product edits

The unchanged R6 binaries processed all191 previously unprocessed SecBench roots:188 admitted,186 passed,2 failed,3 excluded. Together with prior R6 outputs:583 accounted,576 captured,573 passed,3 failed,7 excluded. The following is the complete failure population, not a first-error sample.

|Root|Outside rows|Owners|Classification and demonstrated mechanism|
|---|---:|---:|---|
|path-traversal/node-http-server_8.1.2 (prior STOP)|595|2|WRONG: strict simple formals containing the tree-sitter undefined kind refused as non-simple|
|redos/ua-parser-js_0.7.22|526|1|WRONG: same predicate in function(window,undefined), strict UMD|
|redos/truncate_2.0.0|3|1|WRONG: same predicate in function(context,undefined), strict UMD|

All1124 outside rows have the authorized simple-parameter/early-error mechanism. No other mechanism was observed. Exact owners, source bytes/SHAs and all affected rows: step1-failure-population.json; parser-facts.jsonl binds8/8 whole-file owner/control ranges without recovery. Same-environment synthetic attribution control: strict undefined main3/R6zero; identifier strict3/3; undefined sloppy3/3; default strict3/0 with Node SyntaxError. An initial single-line0/0 probe did not discriminate loss and was inadmissible for attribution. Hypothesis/probe/alternative/results: PROBE-LOG.md. Step1 REPORT was written and sealed before edits in step1-sealed.tgz, SHA355f22123892ef63e1407e823eaa71df4bb865f895217d675f7bf8bcdfe1ac63.

## Bounded implementation and pin-derived parameter inventory

The strict-body/non-simple early error now requires positive default/destructure/rest proof; an unknown kind never satisfies it. A separate positive invalid-BindingElement proof rejects member/subscript/non-null assignment targets admitted by the parser but invalid in ES formals. It traverses binding positions only, ignoring initializer RHS, computed keys, types and decorators. BoundNames, fence matching and decoded-identifier validation recognize undefined. Known-simple recognizes undefined and erased no-value TS required/optional/this wrappers. No copy-environment model or positional binding extension was introduced.

|Kind / grammar field|JS0.23.1|TS/TSX0.23.2|Rule and audit|
|---|---|---|---|
|identifier, undefined|direct|wrapper.pattern|simple; undefined BoundName recognized|
|assignment_pattern|direct|wrapper.value provides default|positive non-simple proof|
|object_pattern, array_pattern, rest_pattern|direct|wrapper.pattern|positive non-simple proof|
|member_expression, subscript_expression|direct pattern alternatives|wrapper.pattern|invalid BindingElement proof; never inferred non-simple|
|non_null_expression|absent|wrapper.pattern|invalid BindingElement proof|
|required_parameter, optional_parameter|absent|direct formal children|value or non-simple pattern is positive proof; no-value erased simple forms stay simple|
|this|absent|wrapper.pattern|erased; no runtime BoundName|
|accessibility_modifier, override_modifier|absent|named wrapper children|public/private/protected/override are not defaults|
|readonly|absent|unnamed wrapper token|not a default|
|decorator|absent|wrapper.decorator|not a default|
|type_annotation; wrapper.name identifier/rest aliases|absent|types / tuple aliases|types fenced; name aliases derive from tuple parameters outside runtime formal lists|
|unrecognized kind|not grammar-listed|not grammar-listed|false for positive non-simple predicate; direct negative unit|

JS formal children expand to8 kinds: array_pattern, assignment_pattern, identifier, member_expression, object_pattern, rest_pattern, subscript_expression, undefined. TS/TSX direct children are required_parameter/optional_parameter; their pattern field expands to9 kinds: array_pattern, identifier, member_expression, non_null_expression, object_pattern, rest_pattern, subscript_expression, this, undefined. Derived directly from pinned node-types.json: grammar-inventory-preaudit.json. JS SHA0d80ab597fcf1310efb9694d4276c407655d060b8bbab8f4ffda0223c43e94bf; TS SHAc790a733fc756b54d4e54dceeb7d2d51e40d8b57136e70277753a75804cce3e3; TSX SHA78b5789145286799a27a0a7ecc36cc1bcb151f94ec7fa631b248459867010c8c. Vendored TS grammar confirms runtime pattern fields vs tuple name aliases at common/define-grammar.js672–690,746–762.

Consumer audit:
- parameters_have_expressions already positively detects JS assignments/computed keys and TS value fields and fences erased types; unchanged.
- BoundNames/fence/decoder shared consumers were wrong for undefined and fixed. SEAM now sees that name and only refuses the design's formal/body-rebind seam; distinct body names remain admitted. Named seam paths equal main.
- EVAL detects lexical direct eval independently of parameter kinds; already right. Direct/member-call negative controls pass.
- ARGS uses known-simple classification; now covers undefined and erased wrappers. Strict/default/destructure/rest controls remain unaliased; arrows inherit arguments while nested ordinary callables are fenced.
- PR-A shape/occurrence helpers preserve conservative supported Def boundaries. undefined itself gets no ordinary-list formal Def; supported siblings and body locals are complete main-parity tables. Whole-callable early-error guards catch invalid undefined duplicates/lexical overlaps before registration; legal sloppy duplicates preserve main rows. Generic legacy AST collectors retain unsupported-kind limitations and were not expanded into a new positional model.

## Regression, matrix and mutation evidence

Prechange127cc257 plus R7 behavior tests:0/3 pass, expected REDs enumerated in red-r7.log. Final4/4 focused tests pass, including unknown-kind negative unit. Kind extension200/200 cells (JS40,TS80,TSX80), each kind × strict/sloppy × named/callback, pass fresh Node proof plus whole-file parser facts.132 admitted cells compare complete two-binding golden tables, including endpoint bytes/confidence/doubt/kill; named main and callback f8c768b3 goldens. TS erasure proves ES behavior, not TS type correctness. Invalid binding targets compile original annotation-free source, because transpilation was demonstrated to repair malformed syntax (20 initial observations excluded). Modifiers/decorators exercise parser fields, with valid surface placement on constructors where applicable. Extra12 frozen-binary duplicate controls:6 legal sloppy cases admitted,6 strict cases refused with Node SyntaxError; R6 incorrectly refused all12.

Existing matrix763/763 semantic cells,2289/2289 grammar cells, former272 failures all PASS,0 DESIGN-CHANGE; fresh Node763/763. Matrix tables: ADDED3141CORRECT; LOST427WRONG/45UNDECIDED/0CORRECT; RELABELLED/RE-OWNED0. Raw LOST233CORRECT/194WRONG/45UNDECIDED becomes effective ES via67 SyntaxError overrides (53 raw CORRECT) and180 catch-var overrides. These are explicit recorded rulings, not rebaselines. Fresh R7 head/main/census byte-equal retained adjudication; matrix-tables-result.json. MATRIX-param-env.md preserves the original matrix and adds all200 cells.

Six changed-predicate mutants PD94–99 killed. Whole registry effective93/93: original gate92 kills/1 survivor; complete survivor population was one equivalent inversion on known forms. Targeted registry rebind to the unknown positive-predicate branch was killed1/1 by direct negative/matrix controls. Production source unchanged; old/new raw mutant records preserved. PD65 anchor qualified for the added invalid-target match. No semantic STOP repair or review restart. The later corpus-table STOP below remains unrepaired.

## Full certificate against f8c768b3

|Corpus|Expected roots|Admitted|Excluded|Changed-row multiset|SyntaxError rows|Outside|Named mismatch|Forbidden synthetic|Non-JS delta|
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|
|X|1|1|0|0|0|0|0|0|0|
|Xi|1|1|0|0|0|0|0|0|0|
|T|1|1|0|0|0|0|0|0|0|
|SecBench|583|576|7|1195|765|0|0|0|0|

Every differing admitted row lies inside certificate classes; named SEAM bindings byte-equal main. Class counts and complete main/old/head row denominators are in final-corpus-totals.json. Former R6 STOP root and both new Step1 failing roots are included and pass the certificate. The independent main-table gate exposes the react-native STOP below.

Exclusions (SecBench only, inherited unchanged main-producer facts, no semantic credit):
- prototype-pollution/swiper_6.5.0: main600s producer timeout.
- prototype-pollution/total.js_3.4.6: main600s producer timeout.
- command-injection/total.js_3.4.6: main600s producer timeout.
- path-traversal/atropa-ide_0.2.2-2: main600s producer timeout.
- redos/clean-css_4.1.10: main exit-9 after443.236s; cause UNKNOWN, not a timeout.
- redos/natural_5.1.0: main600s producer timeout.
- redos/three_0.122.0: main600s producer timeout.

## Complete byte tables vs main, after documented ES overrides

|Corpus|Class|CORRECT|WRONG|UNDECIDED|
|---|---|---:|---:|---:|
|X|ADDED|15513|0|0|
|X|LOST|0|3415|2|
|X|RELABELLED|0|0|0|
|X|RE-OWNED|0|0|0|
|Xi|ADDED|15513|0|0|
|Xi|LOST|0|3415|2|
|Xi|RELABELLED|0|0|0|
|Xi|RE-OWNED|0|0|0|
|T|ADDED|19434|0|0|
|T|LOST|0|374016|0|
|T|RELABELLED|0|0|0|
|T|RE-OWNED|0|0|0|
|SecBench|ADDED|288223|82|0|
|SecBench|LOST|6|948304|25|
|SecBench|RELABELLED|1|0|1|
|SecBench|RE-OWNED|0|0|0|

Every row in all four diff populations has a details.jsonl verdict; raw TS and effective ES partitions have identical complete denominators. X, Xi and T have LOST CORRECT=0 and ADDED WRONG=0. SecBench has6 LOST CORRECT and82 ADDED WRONG: STOP. UNDECIDED rows remain explicitly uncertain. Raw counts, per-root SyntaxError overrides and complete class details are preserved in final/ and final-corpus-totals.json. No rebaselining.

## Complete STOP population (unrepaired)

One root fails the main-table gate: redos/react-native_0.63.0-rc.0,88 rows in22 files. Partition:82 ADDED WRONG (21 not_exact_identifier_bytes,61 synthetic_owner_scope) and6 LOST CORRECT (local/collapsed rows). All88 complete byte/owner/label rows are in final/redos__react-native_0.63.0-rc.0/tables/STOP-rows.json, SHA recorded in STOP.json. Existing917 SyntaxError overrides were applied before this population was counted; none was added after STOP.

WRONG example: AndroidCheckBoxNativeComponent.js byte1082..1089 contains the Flow type token boolean in onValueChange?: ?(value: boolean) => mixed. The head emits a parameter Def named boolean and links it to other boolean type tokens at bytes1194..1201 and1215..1222. That is a type token represented as a runtime formal binding, not the parameter value. The61 synthetic_owner_scope checker WRONG rows retain their original verdicts and source-bound endpoints; they are not downgraded. The6 oracle LOST CORRECT rows include collapsed/zero-width endpoints in Flow type syntax; their runtime meaning is unresolved (SMELL), but the recorded CORRECT classifications are not changed and still trigger the owner gate. No claim that all6 demonstrate a new R7 runtime regression.

Same-environment attribution uses the actual unchanged-R6 Step1 capture on identical inputs: STOP-pre-fix-control.json verifies every complete STOP endpoint/label: all82 additions already existed and all6 losses were already absent before R7. Thus this complete population is inherited, not a regression introduced by the bounded repair. This does not waive the main-table STOP or alter verdicts. No corpus package was executed.

## Identity, navigation and O1

Call-site multisets identical, full byte/wire projections agree on both sides, non-JS identical for all579 identity-admitted roots. Identity accounting, including producer exclusions, is separate from certificate admission: identity-final-totals.json. 
|Corpus|Identity expected|Admitted|Excluded|
|---|---:|---:|---:|
|X|1|1|0|
|Xi|1|1|0|
|T|1|1|0|
|SecBench|583|576|7|

Fresh R7 head CLI sites/wire never reused from R6. Unchanged main reused only with exact current input manifest, original result/producer provenance, binary SHA and output SHA; retained-table adjudication requires newly generated R7 head/main/census byte equality. All receipt/control-script lineage versions are retained and audited. The original parent collector encountered the legacy tables.admitted schema omission; its already queued root workers completed, and watch_final.py reconstructed the complete terminal ledger with explicit admission normalization. This orchestration exception supplies no SUT evidence; no successful head producer was rerun or discarded. Cache uses the required path; cache source build identity prevents stale R6 reuse.

Navigation X241/241,T81/81 admitted: X3 differences,T2, all removed-WRONG nodes/edges/items exactly joined to checker LOST-WRONG details, no additions or non-DFG delta. Call queries identical; no unexplained difference. Original paired scheduling caused repeated cache107/112 rebuilds. Grouped fresh captures preserved scope and published only missing terminal files. Original X base controls8/9 were interrupted with empty stdout/rc-2 and excluded as inadmissible; their originals are held in nav-X/interrupted-original, two fresh controls generated in nav-resumed-X, then the full ledger/proofs revalidated. This corrected the premature241/241 checkpoint claim.

O1 same97 selector population in each variant, all2720 raw invocations admitted and replay-audited; unchanged inputs, zero timeout/exclusion. Outcomes:

|Variant|Traced|Partial|Function only|Prism error|
|---|---:|---:|---:|---:|
|o1-standalone-main|0|0|1|96|
|o1-standalone-head|0|2|4|91|
|o1-joint-main|1|0|0|96|
|o1-joint-head|90|2|5|0|

O1 errors are measured semantic outcomes, not dropped observations; no standalone traced success is claimed. Raw invocation stdout/stderr/command/input/binary SHA records and outcome replay: o1-audit.json.

## Current R7 gates

|Gate|Measured result|
|---|---|
|Full nextest offline locked --features mcp|5232 passed,1 skipped,0 failed|
|Doctests|2 passed,0 failed|
|Focused R7 tests|4 passed;3 behavior tests RED on127cc257|
|Authoritative mutants|93/93 killed (92 initial +1 targeted rebound)|
|cargo fmt --check|PASS|
|Clippy --all-targets --features mcp|235 head/235 main; exact warning-headline multiset parity, no delta|
|Tier-A matrix|178/178 pass|
|Tier-A TS quick|VALID;0 oracle/SUT error rate;47 pending adjudications|
|Tier-A Node quick|VALID;0 oracle/SUT error rate;24 pending adjudications|
|Collect-all runner controls|5 new +13 inherited +4 final pass|
|Existing Node matrix / new grammar cells|763/763 and200/200 pass|

Main clippy control ran in the same environment at da0604b3; no observed nextest regression needs attribution. Quick VALID is harness validity, not perfect accuracy. Immediate release rebuild preceded allow-stale-SUT checks and production source stayed frozen. Initial UV-cache denial and incorrect quick language selector selected no valid SUT observation; logs preserved, direct installed entry and corrected ts,js selector used. Full Rust quick requires the controller outside sandbox; no baseline changed.

## Frozen artifacts and delivery

|Binary|SHA256|
|---|---|
|bin/prism-head-r7|cfcb6b17d4229a6e6770f81aef0372e2ac247662c492f076156e192d047e54c7|
|bin/seam-census|f33075d5aff2318e206b8ed30f00785bc526141c37014bf649044ea2e29f6ae8|
|bin/prism-head-r7-bytes|673400c5b58d40e611970f449856d8dd9b576c4e4437073d39288ec4708cd9c4|

Build: cargo build --release --offline --locked --features mcp; cached locked libprism plus held byte/census helper sources, compiler/build arguments and source SHAs in binary-binding.json/build-artifacts.jsonl. Registry-only mutant rebind did not touch build.rs binary inputs; final source and bin hashes audited.

R7-src.patch relative127cc257 changes exactly four paths: src/ast_callback_identity.rs, src/cpg/callback_identity_tests.rs, src/cpg/fixtures/js_param_r7_kind_matrix.json, mutants/js-param-defs.json. Patch applies in a scratch non-Git tree and reconstructs all727 files byte-equal (src-patch-manifest.json). Candidate source commit message (NOT adoption-ready because of STOP): `fix(js-param-defs): prove non-simple parameter early errors`.

R7-docs.patch relative30c18a7a adds MEASUREMENTS-prB-repair-r7.md and MATRIX-param-env-r7.md under docs/superpowers/plans/2026-10-06-js-param-defs. Patch application and every added byte verified. Documentation commit message: `docs(js-param-defs): preserve R7 table STOP and complete census`.

REPORT.md, HANDOFF.md, final-corpus-totals.json, final-invariants.json, patch manifests, raw captures/proofs/gates, sealed Step1 and final snapshot preserve custody. Controller owns commits/push/merge. No adoption or independent-review claim.

## Not verified and STOP

Seven excluded roots' semantics remain unknown; their main producer failures supplied no evidence. Rust quick and full human-triggered multi-corpus Tier-A were not run. Corpus packages were never executed; frontend-portal was never opened. TS erasure is not TypeScript type-check validation. Existing UNDECIDED byte rows and quick pending adjudications remain unresolved. No independent review, merge or deployment was performed.

STOP: redos/react-native_0.63.0-rc.0;6 LOST CORRECT +82 ADDED WRONG after existing ES overrides. Complete population and all remaining collect-all gates recorded; no repair or restart. This is the owner final loop; PR-B remains parked and the source candidate is not adoption-ready.
