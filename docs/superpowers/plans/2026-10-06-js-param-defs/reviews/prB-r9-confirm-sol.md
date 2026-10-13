# PR-B R9 fold-confirmation review — Sol

**Verdict: FIX (closed, enumerable fixes). Three MATERIAL WRONG findings and one IMMATERIAL WRONG finding. Round 1/1; no extension, restart, implementation repair, Git writes, or PR comments.** The original W1–W3 failures close. R9 introduces a correct-row loss through labelled body functions, leaves an adjacent enum-name case unfixed, and overstates the type-annotation partition.

Review binding: clean plan clone `/Users/wesleyjinks/code/prism-pd-rev-sol`, HEAD `5ddbf1d64991b7b9f609b1643b0951bd475b996f`. Source delta is `2431cb104db7828d11ae409dc207c1e339270c93..96817370ffb65dd3de14fe574789b6bf31e99ac5`, six files, 472 additions/30 deletions. Source references below bind **96817370**, not the plan checkout's product files. I read my earlier `prB-final-sol.md`, the other review, SPEC §8, and R9 REPORT/HANDOFF. I exported exact head, pre-change and current main (`cb0eeec062053f3b7d29b5d5c5eda8efa30e69da`) with `git archive` and built offline in non-Git scratch copies inside this clone. All **727** source/test/mutant manifest entries match the head archive. The production source was not edited.

Evidence is retained beside this report under `prB-r9-confirm-sol.*`: complete fixture sources and hypothesis/expected/falsifier/alternative logs, full byte rows, parser trees, TS diagnostics/symbol declarations/emit, synthetic Node controls, builds, test logs, partition audit and samples. Corpus source was read statically; no corpus package was executed and no directory named `frontend-portal` was opened.

## R9-W1 — WRONG / MATERIAL: labelled body-function binding becomes a false block boundary

**Evidence: reproduced.** `src/ast_callback_identity.rs:701–716, 725–727`; missing behavioral negative at `src/cpg/callback_identity_tests.rs:1883–1885`. Minimal valid sloppy `case.js`:

```javascript
function q(y){
 lbl:function y(){}
 use(y);
}
```

Fresh **2431cb10** retains the parameter Def, line 1 bytes **11..12**, → line 3 Use **40..41**, **Exact**. Fresh R9 emits **no rows**. The retained main control has the same correct row; current-main's fresh build agrees with retained main on the complete 372-file batch. Single and chained labels reproduce in JS/TS/TSX, for named and synthetic owners. The JS source has no parser errors, no TS syntactic diagnostics, and no type annotations.

The callable-body filter excludes a direct `function_declaration` from the block's lexical binders, correctly leaving it in the callable environment. The new label recursion exposes the declaration **after** that filter: the direct child is `labeled_statement`, so it escapes the exclusion. The formal's binding scope becomes the callable, while the body's `use(y)` resolves to the body block. The fence consequently drops a correct row. With simple parameters, the labelled body declaration and the parameter are the same function-scoped binding. TS 5.9.3 resolves the formal and body read to the same Parameter declaration. A synthetic Node control saves `()=>y`, executes the labelled declaration, assigns `y=9`, and observes `[9,9]` through that closure and the body read. This distinguishes a shared binding from a value overwrite, which does not change Exact's accepted static-binding meaning.

**Fix:** unwrap label chains before the callable-body function-declaration exclusion, while retaining their function-scope classification and preserving lexical declaration handling in actual nested blocks. Alternative: distinguish body-level function declarations from block-level declarations explicitly in the scope classifier. Add full byte/label regressions for named and synthetic simple-parameter callables, single/chained labels, and an unrelated-name/nested-block negative. The new test only asserts `SEAM=false` for its simple-parameter negative; it never checks that the correct row survived.

**Self-critique:** strict/module labelled declarations are invalid and are not this input. Parameter-expression seams intentionally require a separate environment/refusal; those controls still work. This does not require restoring main's separate zero-width declaration-name endpoint.

**Classification:** CLOSED INSTANCE of Opus F6 / SEAM and body-binding classification, with a **new R9 regression**. Confidence **100/100**: fresh before/after rows plus clean parsing, checker identity and Node control agree. It would collapse if this body read resolved to a separate binding or if the same-environment pre-change control lacked the row; both were checked.

## R9-W2 — WRONG / MATERIAL: “type-annotated JavaScript” includes unannotated files

**Evidence: reproduced classifier; static supplied-row audit.** `repair-r9/independent-census.cjs:25–29`, `repair-r9/measure.py:45–51`; plan `probes/r9-admissibility.cjs:12–15`, SPEC §8's measured-boundary/extent paragraphs. The classifier places a `.js`/`.jsx` file inside whenever **any** TS parse/syntactic diagnostic exists. That is broader than the E13 type-annotation exception.

Minimal valid sloppy JavaScript:

```javascript
register(function(stop){
 console.log("\033[2J");
 use(stop);
});
```

Node compiles this synthetic source successfully. TS 5.9.3 reports diagnostic **1487**, rejecting its legacy octal string escape. There are **zero TypeNodes and no annotations**, yet the independent census's predicate marks the file `type_annotated_JS`. This is a parser-policy diagnostic, not Flow/type syntax.

This occurs in the retained measurements, not just a hypothetical fixture. `command-injection/pidusage_1.0.0/test/stresstest.js:32` contains that escape; its line-71 `stop` formal → line-72 Use is an ADDED CORRECT Exact row counted **inside**. The two pidusage test files have only the octal diagnostic and no type syntax; their two added CORRECT rows are inside. `path-traversal/node-srv_2.0.0/tasks/srv.js:32` instead has `grunt.log.ok "..."`, a missing-call-parentheses syntax error with no annotations; its five added CORRECT rows are also inside. Thus **at least seven** supplied inside rows are demonstrably unrelated to type annotations.

Across all 541 diagnostic-tagged files, 86 have no TS AST type nodes. Their supplied tables contain **112 ADDED CORRECT and 4 LOST WRONG** rows. That is a **candidate textual-audit population**, not a claim that all 86 lack unsupported Flow syntax: absence of TypeNodes alone cannot prove that. The complete file/diagnostic/row list is retained in `typed.json` and `boundary-rows.json`. The concrete pidusage and node-srv examples were inspected separately.

**Fix:** independently classify actual type syntax, retain generic diagnostic failures in a separate disclosed bucket, and re-tabulate/reconcile the E13 extent and census-disagreement dispositions. Keep diagnostic codes/spans and the existing source bindings. Alternatives: obtain an explicit owner expansion of E13 to all JS syntactic-diagnostic files, or publish the existing split under the accurate name “JS with TS syntactic diagnostics” and additionally provide the actual annotation split. A regression should admit the octal fixture as unannotated, identify a genuine Flow/TS annotation without a pragma, and preserve INADMISSIBLE status for unrelated compilation failures.

**Self-critique:** the current algorithm is independent of the product, and the SPEC discloses its diagnostic-based implementation. The defect is treating that broader implementation as the extent of the owner's annotation exception. I found **no false ADDED or LOST CORRECT corpus row** among the concrete unannotated examples; this finding corrects disclosure/partition authority, not their binding verdicts. All 90 adverse E13 rows remain in genuinely annotated files. The generic-diagnostic bucket could be accepted by a new owner decision, but the existing Flow decision does not establish that acceptance.

**Classification:** CLOSED INSTANCE of Opus F3 / honest E13 extent and discriminating evidence. Confidence **100/100** for the concrete partition error; complete corrected annotation totals remain unmeasured. It would collapse if the inspected files contained annotation syntax or the owner had explicitly accepted the broader bucket.

## R9-W3 — WRONG / MATERIAL: quoted enum member names escape the new fence

**Evidence: reproduced.** `src/ast_callback_identity.rs:668–679`. Minimal parser-clean `case.ts`:

```typescript
register(function(y){
 enum E { "y"=1, z=y }
 use(y);
});
```

R9 emits formal Def bytes **18..19** → enum initializer Use **41..42**, **NameOnly(CfgIncomplete)**. That Use reads member **E.y**, not the callback formal. TS's checker resolves it to the EnumMember; TS emit assigns `E.z=1`. A synthetic emitted-code Node control invoked with formal `y=7` returns **[1,7]**. Single quotes and escaped quoted `"\u0079"` reproduce in TS/TSX, for named and synthetic owners. The legitimate outer line-3 read remains.

The new scope arm decodes a member's raw text as an **identifier**. An `enum_assignment` with a string key supplies `"y"`, whose quotes make identifier decoding fail. Unquoted `y` and unquoted identifier escapes are fenced correctly; unrelated member `x` correctly leaves a captured outer `y` read admitted.

**Fix:** derive enum member names from their syntax kind, decoding string-literal keys as strings and identifier keys as identifiers before comparison. Alternative: conservatively fence a matching quoted member using a validated literal decoder, or explicitly defer these forms with owner disclosure instead of claiming the whole F8 fix closed. Add double/single/escaped string-key positives plus unrelated-key/outer-read negatives; simple quote trimming is insufficient for escapes.

**Self-critique:** this wrong row is **unchanged from 2431cb10**, not a newly introduced edge. It is reported because the new F8 arm implements the expressly requested fix incompletely on a neighboring valid syntax form. Original Opus F8 was tagged IMMATERIAL; under this brief's unchanged **zero false edges** contract the reproduced cross-binding edge is material even though it is NameOnly. No incidence on the retained corpora was established, and I do not reopen unrelated legacy enum behavior.

**Classification:** CLOSED INSTANCE of Opus F8 / E3 enum-member binders. Confidence **100/100** for the fixture; it would collapse if the checker/emit read the outer formal or the edge were absent, both directly checked.

## R9-W4 — WRONG / IMMATERIAL: Unicode redeclarations are classified INADMISSIBLE

**Evidence: reproduced.** Plan `probes/r9-early-errors.cjs:7, 11–15`; identical worker `repair-r9/early-errors.cjs:7`, with the same ASCII rule used by `certificate.py`/`part_a.py`.

Minimal callable: `function q(π){let π;}`. The applicable expression wrapper reports `SyntaxError: Identifier 'π' has already been declared`. This is the same genuine parameter/lexical early error as the allowlisted ASCII fixture. The anchored pattern accepts only `[A-Za-z_$][A-Za-z0-9_$]*`, so `classify` returns **INADMISSIBLE**, while `function q(a){let a;}` returns EARLY_ERROR. The second object wrapper's generic rejection supplies no independent proof and was not credited.

**Fix:** retain the exact anchored message shape but support ECMAScript Unicode identifiers, preferably validating the extracted name and redeclaration against an independent AST. Apply the correction consistently in all worker/controller implementations. Alternatives: enumerate additional proven names conservatively or keep Unicode failures explicitly disclosed as an assurance limitation. Test ASCII, Unicode/escaped-name positives and the existing colon/static/super/member-target negative controls.

**Self-critique:** this is conservative under-credit, not a false WRONG override. The supplied corpus certificate has zero genuine-early-error overrides, and I found no affected corpus total. It is nonblocking by itself. Version-specific Node wording outside this exact message is a different issue.

**Classification:** CLOSED INSTANCE of Opus F3 / genuine-early-error allowlist completeness. Confidence **100/100** for the returned classification; corpus impact is unproved. It would collapse if the intended rule explicitly excluded Unicode names, or an independent proof path classified this input correctly; no such path is supplied here.

## Fix-table disposition

| Claimed repair | Disposition | Evidence |
|---|---|---|
| Opus F2: recovered parameter-list early-error refusal | **CLOSED for the original failure** | Original annotated-JS fixture restores both retained rows; named local `x` → read is Exact. Array annotation and recovery controls admit locals in applicable JS/TS/TSX forms; clean duplicate-arrow, non-simple/strict and lexical-error controls still refuse. Broad arbitrary-recovery assurance is limited below. |
| Opus F3: early-error evidence, independent census, honest split | **PARTIAL** | Four supplied positives/four parser-limitation negatives re-run correctly; colon failures stay INADMISSIBLE; worker raw-directive test is sound. Census is independently implemented. R9-W2 and R9-W4 remain. |
| Opus F1: JSX tag names | **CLOSED** | Exact original JSX reproducer loses only the intrinsic-tag endpoint. Opening/closing/self-close, namespace and hyphen controls; capitalized components, member tags and embedded runtime reads remain. Named legacy controls keep their rows. |
| Opus F5 / Sol W3: wrapped eval | **CLOSED** | Exact old fixture and neighboring transparent/composed wrappers refuse formals in TS/TSX; parentheses cover JS; angle assertion only TS. Local rows survive; sequence/member/optional controls remain admitted. Synthetic TS emit/Node controls distinguish direct result 2 from indirect result 1. Named controls are byte-identical to pre-change. |
| Sol W1: class receiver writes | **CLOSED** | Original false Def and kill vanish, leaving outer line-2 → line-6 Exact. Instance/static fields, static blocks, methods and arrow-valued fields stay receiver-local; computed field/method keys and heritage retain outer receiver effects. Captured lexical-variable control remains admitted. |
| Sol W2: class self-name | **CLOSED** | Original cross-binding row vanishes; mutable outer read survives. Declaration/expression, abstract TS/TSX, field/static field, computed-key/heritage and unrelated captured-name controls checked. |
| Opus F6: labelled body functions | **PARTIAL** | Original expression-parameter SEAM reproducer closes, including label chains. R9-W1 shows the adjacent simple-parameter binding regression. |
| Opus F7: duplicate formal function kinds | **CLOSED** | Original async/generator cases restore; normal/async/generator/async-generator × strict/sloppy × simple/non-simple controls checked in JS/TS/TSX. |
| Opus F8: enum members | **PARTIAL** | Original unquoted-member reproducer closes; ordinary outer-variable negative survives. R9-W3's quoted forms remain wrong. |
| Opus F9: predicate/asserts read role | **CLOSED** | Original predicate and asserts tokens disappear from synthetic Uses in TS/TSX; runtime reads and named legacy rows remain. All 62 exempt bookkeeping rows independently audited: 34 predicate endpoints removed, 28 replacement runtime identifiers, with added endpoints joined to removed endpoints by the same Def identity. All are F9; no bookkeeping finding. |

## Verification, evidence strength and limits

- **Fresh execution:** all 64 saved earlier Sol fixture sources re-run verbatim; their fresh pre-change outputs exactly match the earlier fresh-head outputs. Six additional exact Opus reproducer sources re-run, plus **372 neighboring fixture files** and 24 targeted neighboring files. Full rows agree between fresh and frozen R9 on the 372-file batch, between fresh and retained pre-change, and between fresh current main and retained main. These compare complete byte/owner/path/label multisets, not exit status.
- **Full head suite:** `cargo test --offline --locked --features mcp`: **5,243 passed (5,241 tests + 2 doctests), 0 failed, 1 ignored**, no filtering. The nine R9 tests are included. The same nine tests overlaid on the exact pre-change production archive give **0 passed/9 failed**, with behavioral assertion output in `red.log`, not compilation failures. No production repair was made. Optional detached compiler-audit feature was not enabled.
- **Tier-A:** immediate completed head release rebuild, then matrix-only run: **178/178 cells `ok`**. An earlier matrix invocation started before its rebuild finished; it is retained as `matrix-premature.log` and earns no gate credit. The corrected run follows the completed rebuild. No LSP/quick/full-corpus Tier-A run was performed.
- **Mutants:** all ten new PD100–109 anchors are unique and target actual changed decisions; PD105's paired binding/seam edit is also unique. They are meaningful mechanism removals, selected against the new tests. They do not supply the missing simple-parameter row assertion or quoted-member cases. The R9 **103/103 killed**, coupled cache mutant, and clippy parity are **supplied, source-manifest-bound receipts**, not my mutant/clippy executions.
- **Reported tables:** I independently sum all **586 unique per-root results: 578 admitted**, including 575 SecBench plus X/Xi/T, and reproduce the brief's corpus counts exactly. This is ledger verification, not fresh whole-corpus production or re-adjudication. The new CEjs timeout remains excluded with no semantic credit; I did not re-measure it. Its bookkeeping and the known escaped-directive controller correction are not findings here.
- **E13 adverse rows:** effective **84 ADDED WRONG + 6 LOST CORRECT**, all in 22 annotated React Native files, with their current bytes matching supplied input hashes. Added-reason counts are 63 `synthetic_owner_scope` and 21 `not_exact_identifier_bytes`. I inspect all six loss contexts and representative additions, including the two F2-added `error` rows. The loss endpoints include `DangerouslyImpreciseStyle` type tokens, a recovered callable named `string`, and `EventSender` type text. This supports the disclosed misparse class; it does not independently certify every oracle ruling. R9-W2 concerns additional files in the broader partition.
- **F2 guard limitation:** a malformed, unannotated `register((a,a,broken.)=>{use(a);});` is admitted after recovery and emits two formal `a` rows in JS. Its generic Node syntax rejection is **INADMISSIBLE**, not a discriminating early-error proof. I do not promote it to a separate WRONG or use failure to find a valid recovered early-error counterexample as proof that the guard is universally safe. Exhaustive recovery outside actual annotated JS remains unchecked.
- Minor metadata inspections initially used the wrong registry key and indexed an empty STOP list; these were corrected and yielded no semantic attribution. No sandbox-denied command class was retried, no install/provider/network step was used, and no corpus source was invoked.

What I did not check: fresh full X/T/SecBench row/call-site/non-JS/navigation captures; every corpus oracle verdict; exhaustive annotation-boundary text classification; every census-disagreement disposition; complete recovery/grammar correctness; performance/O1/MCP-specific live behavior; fresh mutation/clippy/quick-LSP gates; private F; semantics of the eight excluded roots. Existing navigation/non-JS tests ran as part of the full suite, which is narrower than fresh whole-corpus byte-identity capture. The current-main MCP-only source changes do not affect the measured helper outputs; they are not a claim that current main and the older packet main have identical whole trees.

Cleanup: all isolated source/fixture/build scratch and its three target trees were removed; final Git status remains clean at the bound plan HEAD. The report, evidence, handoff and snapshot remain in the evidence directory. Controller owns repairs/commits and landing; owner E13 reaffirmation and known STOP disposition are still required by the brief.

**Verdict: FIX (closed, enumerable fixes).** At the declared **1/1 cap**, repair R9-W1's callable-body label classification, close/dispose of R9-W3's quoted enum forms, and correct/dispose of R9-W2's annotation partition on the retained artifact. R9-W4 is a nonblocking conservative classifier defect. These bounded instances do not justify an artifact restart or reopening settled design. No additional round was dispatched.
