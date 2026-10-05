# S2 specification review — round 2 of 2, FINAL under the cap

**2 WRONG / MATERIAL; 3 SMELL / IMMATERIAL.** The candidate still admits Exact despite required static alias/write refusals. Both WRONGs are closed instances of covered static-binding channels. No restart or new review round is requested.

## Binding and method

Working checkout: `/Users/wesleyjinks/code/prism-s2-review-sol`, clean `review`, HEAD `ecdb6b0ffb62963e443027e5de07b4b6272f95c3`. Main is `4e592daa7858a195eb3a9eb77c83dfbc763b49fa`; cumulative product reviewed with `git diff origin/main origin/proto/s2-import-qualifiers`, prototype HEAD `75a35a5ed0145d12acaf2df33834f748704f8ef9`, following `c35719e1132809dc835f64f476cf31e2c18bd35f`. Product source/test line references below refer to **75a35a5e**, not the plan-only working tree.

All **769 non-registry** BUILD-MANIFEST inputs match prototype Git objects. The remaining entry, the repaired 22-mutant registry, matches retained repair source but differs from the prototype's original 14-mutant registry. Frozen main, R2 head and head-facts executables match manifest hashes. All **53** recorded receipt hashes match. [Custody binding](spec-r2-sol-evidence/custody-binding.json) records the distinction; no full 770-input committed-tree equivalence is claimed.

I used existing bound executables, native TypeScript 5.9.3 and native ESM controls. No builds, production edits, Git writes, installs, network, subagents or private-F reads. Commands and predictions/falsifiers/alternatives/results are retained in [probe-log.md](spec-r2-sol-evidence/probe-log.md); complete fixture sources, base/head rows, facts and runtime output remain under [review evidence](spec-r2-sol-evidence/).

S2-O7 is the review authority. Runtime mutation alone does not refute static Exact. The WRONGs below violate the explicit **retained in-model static alias/member-write refusal**, SPEC:9,33,37,47,59. Runtime controls establish reachability and shared identity; they are supplementary evidence, not a demand to restore runtime truth.

## WRONG findings

### W1 — A statically imported exported namespace revokes its wrapper but leaves its qualifier contents admitted

**Tag: WRONG / MATERIAL. Confidence: 100/100. Evidence: reproduced, both grammars.**

Minimal complete witness, with the ordinary local fixture tsconfig (`allowJs`, `jsx: preserve`, Node resolution, all files included):

```js
// m.jsx / m.tsx
export default class C { static sm() { return 0; } }
// barrel.jsx / barrel.tsx
export * as M from './m';
// other.jsx / other.tsx
import { M as Q } from './barrel';
Q.default.sm = () => 1;
// app.jsx / app.tsx
import X from './m';
export function run() { return X.sm(); }
```

An entry statically imports `other`, then calls `run`. Main has no target for the app call. R2 adds singleton Exact `import_qualified` to the original `m:sm`; native ESM executes the replacement and returns `1`. The required result under the retained lexical write rule is **the unchanged empty base outcome**.

Mechanism-level evidence:

- `src/js_import_qualifiers.rs:137-160` assigns the exported namespace identity `(m, "*namespace*")`, with a Callable-member table. It does not carry the qualifier identities exported by that namespace.
- `src/call_graph.rs:2415-2424` joins the refused imported member `M` to that wrapper and records only `(identity.file, identity.local)`. Retaining at :2431-2434 therefore removes the wrapper, while `(m, "C")` survives.
- `src/call_graph.rs:2352-2360` separately matches raw refused spellings to defining local names. `Q.default` produces `Q`, `default`, `sm`, not `C`; a string-keyed `Q['C']` likewise supplies no identifier token `C`. Plain `Q.C` happens to keep base through that global token match, masking the structural gap.

Representative captured facts, from `default-direct-tsx/facts.jsonl`:

```text
other: complete=true, syntax_incomplete=false,
       binding={local:Q, member:M, module_path:./barrel},
       written=[Q, default, sm]
m: named={default: Local(C)}, locals={C: {sm: ...}}
```

[Native binding](spec-r2-sol-evidence/native-binding.json) independently resolves `Q.default` to the default class declaration and `Q['C']` to named class C, with zero syntax diagnostics in both grammars. These are static imports/exports and selected members, with no dynamic acquisition, enumeration, eval or host lookup. SPEC:60-62's dynamic/default re-acquisition and enumeration exclusions cannot cover this static subset without contradicting SPEC:9,47,59,65.

Population: [eight default-path forms × two grammars](spec-r2-sol-evidence/default-paths-summary.json) reproduce, including aliases, computed default, reflective writes, renamed imports, a forwarding barrel, namespace-of-barrel access and default-exported namespaces. [Computed named access, an aliased namespace and destructured default](spec-r2-sol-evidence/namespace-content-summary.json) also reproduce in both grammars. Direct named/default imports and direct `import * as` writer controls keep base. Namespace `this` controls keep base and are **not** findings.

**Fix:** Retain refusal-only namespace-content relationships independently of the positive Callable-member map. Join in-model static member selection/write/extraction and aliases to the selected export's original qualifier identity, including `default` and literal string names; propagate through static namespace forwarding. When an in-model namespace escape makes selection unavailable, conservatively refuse its possible qualifier contents. Distinguish the expressly excluded enumeration channel rather than restoring F3/F4/F7 blanket runtime cuts. Add both-grammar regressions asserting actual main is empty and repaired head equals the entire base row, plus direct-namespace and clean-forwarding controls.

**Alternative and tradeoff:** Conservatively refuse S2 qualifier identities exposed by exported namespaces, without a precise selected-member join. This bounds the implementation but over-refuses clean uses and requires yield/pinned-control reconciliation. Merely removing positive *namespace-call admission* does not fix this witness: the app imports C directly.

**Self-critique:** Under a different owner contract excluding even visible static namespace member aliases/writes, this would cease to be a defect. That is not the current retained rule. The witness stays inside the indexed universe and has complete syntax, native bindings and same-environment base control. Its static native terminal still agrees with the original declaration; the defect is the omitted required refusal, which that terminal certificate cannot detect.

**Convergence classification:** Closed static-content instance of the round-1 namespace/content join family F4, also covered by §2a's static bindings row. The actual enumeration witnesses remain correctly out of model. No new language-mechanism family is claimed.

### W2 — A missing positive forwarding identity silently erases a required static writer refusal

**Tag: WRONG / MATERIAL. Confidence: 100/100. Evidence: reproduced, both grammars.**

```js
// m
export default class C { static sm() { return 0; } }
// b1
export { default } from './m';
// b2
export { default } from './b1';
// b3
export { default } from './b2';
// other
import Y from './b3'; Y.sm = () => 1;
// app
import X from './m'; export function run() { return X.sm(); }
```

All syntax is complete. The writer is an ordinary static default import of the same C. Main drops the app call; R2 emits Exact to original `m:sm`; native ESM returns replacement `1`. [Forward-depth population](spec-r2-sol-evidence/forward-depth-summary.json): zero/one/two forwarding hops keep base; three/four/five retain Exact, in both grammars.

**Static evidence:** `src/js_import_qualifiers.rs:55-64` inserts only successful identities; :102-103 returns `Err` above `MAX_REEXPORT_DEPTH`. In `src/call_graph.rs:2387-2427`, the writer module is known, so :2411-2417 restricts lookup to that module/export. When its positive identity is absent, the loop visits nothing and records no refusal. The directly imported `(m,C)` remains admitted. The two-hop positive admission cap must not turn an unjoined static writer into proof of no write to a different, directly admitted site.

A second instance has only one hop:

```js
// m
export class C { static sm() { return 0; } }
// barrel
export { \u0043 as default } from './m';
// other
import Y from './barrel'; Y.sm = () => 1;
// app
import { C as X } from './m';
export function run() { return X.sm(); }
```

[Opaque-forward controls](spec-r2-sol-evidence/opaque-forward-summary.json) show the escaped equivalent executes legally and retains Exact in both grammars; replacing `\u0043` with `C` keeps base. `src/ast/js_binding_checks.rs:29-36` correctly makes escaped spelling unprovable, but `src/ast/js_import_qualifiers.rs:19-25` returns before copying forwarding facts at :48-68. The barrel facts become `complete=false`, `syntax_incomplete=false`, `named={}`, `written=["\\u0043","default"]`. F8's parse-error cut at `src/call_graph.rs:2338-2350` does not apply. Again, the downstream static writer's target is absent from the positive table, and its refusal vanishes.

**Fix:** Give refusal traversal explicit `joined`, `proved unrelated/absent`, and `unavailable` outcomes. Traverse static import/re-export source relationships with a cycle-safe conservative target set independently of the positive hop limit and B0 capture success. Refuse possible admitted identities on unavailable name/scope/forwarding proof; preserve literal source edges as negative facts even when names cannot grant authority. Keep the positive two-hop cap. Add depth-boundary and escaped/unescaped regression pairs in both grammars, including default and named-forwarding variants, and a mutant removing the unavailable-join fallback.

**Alternative and tradeoff:** A global S2 revocation on an unavailable refused static import/forwarding target is simpler and sound, but can reduce yield substantially. Measure before adoption. Raising the positive hop cap alone moves the failing boundary and does not handle the one-hop opaque case.

**Self-critique:** This does not request positive admission through unsupported deep or escaped routes. The affected app uses a supported direct import; unsupported proof on the visible writer must preserve uncertainty. Files outside the proof universe would change the claim, but every witness file and static execution entry is included. The depth and B0 instances differ in why the positive export is absent; both fail at the same missing negative-join fallback.

**Convergence classification:** Closed instances of §2a:59's static import/forwarding/write channel and §1's unavailable-proof refusal. They are new counterexamples to that covered rule, not a new runtime or language-mechanism family. The bounded repair is the refusal-join failure policy, not an ever-growing spelling or depth exception list.

## SMELL findings

### S1 — S2-02 still survives; the retained mutation run covers 16 of 22

**Tag: SMELL / IMMATERIAL. Evidence: static inspection of supplied execution artifacts.**

`VERIFICATION.md:13,31` accurately discloses the denominator. `repair-r2/advisory-mutgate.log:7,23` records:

```text
S2-02-position-proof SURVIVED
ADVISORY TOTAL killed 15/16 admissible 16
```

I inspected the run's per-mutant test output, not just statuses: all eight added in-model guard mutants are killed. Six omitted anchors are S2-04/05/06/08/09/12. No equivalent-mutant claim or concrete wrong output from S2-02 is established.

**Fix:** Retain disclosure and add a realistic base-empty positional-binding regression that demonstrates a false Exact when only S2-02 is enabled. Run the complete registry after its source/registry anchors are committed. **Alternative:** Keep the advisory gap pending with a source-level binding proof; that supplies assurance but does not count as killing the mutant. **Self-critique:** The survivor is already acknowledged and is not a new blocker. W1/W2 supply independent failures; they are not evidence that the positional mutant is non-equivalent.

### S2 — Dispatch still describes an uncommitted repair and does not bind the committed branch plus repaired registry

**Tag: SMELL / IMMATERIAL. Evidence: static Git/document/hash inspection.**

`IMPLEMENTOR.md:3,5,7` says:

```text
... the new repair is not yet committed.
... continues the prototype branch with that same repair.
Cherry-pick c35719e1, then apply R2-src.patch ...
```

The source repair is committed as `75a35a5e`. Its 769 non-registry inputs match the candidate. Its registry remains the original **14**, while the tested registry has **22** and resides in `R2-docs.patch`/retained repair source. The planning review tree has no registry file. Thus continuing the current prototype and applying a c357-relative source patch is not the same operation as starting at c357; an implementer must infer which repair bytes are already present and where to obtain the tested registry.

**Fix:** Name the committed path `c35719e1` then `75a35a5e`, planning docs `ecdb6b0f`, and a separately hashed registry-only addition. Explicitly skip the already-folded source patch on that path and bind the resulting composite tree before rebuilding/replaying. **Alternative:** Preserve the archival c357-plus-patches path as a separate, exact-parent replay recipe; it remains useful but should not be the current committed-branch dispatch. **Tradeoff:** Commit-based dispatch removes external patch dependence for source; the registry still needs controller landing. **Self-critique:** This brief supplies the correct product commits, so this review is unambiguous. No wrong implementation caused by the dispatch was observed; this is a handoff risk, not a material WRONG.

### S3 — Historical installed-X oracle inputs no longer match the live root byte-for-byte

**Tag: SMELL / IMMATERIAL. Evidence: reproduced read-only rehash.**

`repair-r2/public/installed-X/oracle-inputs.json:1768` records:

```text
node_modules/@types/json5/index.d.ts: 96d14f21...9538
```

The live file hashes to `c5a14bde...1542`. Removing its initial UTF-8 BOM in memory yields **exactly** the recorded hash; no declaration-body change was observed. [Drift receipt](spec-r2-sol-evidence/installed-input-drift.json) retains full hashes. All other captured oracle/source inputs match.

**Fix:** Rebind and replay installed-X in a fresh evidence directory before claiming a measurement against today's live inputs, particularly after W1/W2 repair. **Alternative:** Retain the existing comparison as historical evidence for its recorded input bytes and explicitly avoid transferring it to current live input identity. **Tradeoff:** A fresh replay establishes current custody; historical qualification preserves existing evidence without altering third-party inputs. **Self-critique:** This does not invalidate the original source-bound execution retrospectively or demonstrate an incorrect binding. It does mean strict input-continuity checks cannot certify today's live installed root from that receipt; SPEC:87 rejects such drift for a current replay.

## Rule/test and round-1 dispositions

| Family / rule | Review result |
|---|---|
| Original lexical whitelist and visible alias/write cuts | Retained; direct named/default, renamed writer and direct namespace controls keep base. Original controls already pass c357 and must not be advertised as new RED regressions. Cross-file closure is incomplete at W1/W2. |
| F1 own-member calls / class heritage | Captured-member validation locally and at import joins; heritage refusal present. Supplied c357 RED output enumerates both-grammar failures. Clean own-call controls stay Exact. |
| F2 construction | Every construction use refuses. Both-grammar c357 RED population includes discarded and consumed construction; no construction admission remains. |
| F5 new.target / super | Explicit carrier refusal present. c357 test fails in both grammars, including the uncalled new.target carrier assertion. Some super/inherited subcases were already kept base by other guards; the whole test being RED does not mean every subcase was RED. Added carrier mutant is killed. |
| F6 this | All plausible enclosing carriers retained; nested initializer/arrow value escapes fail c357. Nested member-write and namespace-this controls keep base; the carrier mutant is killed. |
| F8 parse-incomplete / legacy opacity | Incomplete facts retained and all possible S2 identities revoked; legacy refusal-only star opacity filter present. New parse-incomplete behavior is RED on c357; legacy opacity is intentionally GREEN there and kills S2-30 when removed. Escaped cleanly parsed forwarding uncertainty in W2 is separate from syntax-error revocation. |
| F3 dynamic require/import()/default re-acquisition | Original dynamic witnesses correctly excluded by S2-O7 and explicitly pinned. W1's static namespace `.default` path and W2's static forwarding are not those excluded witnesses. |
| F4 namespace enumeration | Original Object.values/enumeration witnesses correctly excluded and disclosed. Static content writes in W1 remain in model; excluding the entire old content-join obligation was too broad. |
| F7 eval / Function / reflected codegen / hosts | Correctly excluded/disclosed. Original same-object runtime witnesses remain valid under the superseded runtime contract; no inability-to-reproduce downgrade or runtime safety claim is made. |

The five added in-model test functions are demonstrably RED in supplied **same-environment c357** output: `repair-r2/c357-red.log:64-212`; the seven-test population is 5 expected failures and 2 preservation passes. The overlay omits only c357's absent new schema-field assertion. Those artifacts' hashes match, and their production inputs bind to the reviewed candidate; these are **supplied executions, not reviewer-run tests**. W1/W2 need additional failing regressions; existing green tests do not cover their counterexamples. The former R1 positive test's shadow-site lookup issue is corrected at `tests/integration/js_import_qualifiers_test.rs:781-795`.

## Measurement admissibility

I independently replayed classification over **every retained site**, checked duplicate keys before dictionary construction, compared complete base/head populations and metadata, and rechecked native module/owner/full-span certificates on every changed row. [Replay receipt](spec-r2-sol-evidence/measurement-replay.json):

| Corpus | All sites | Populated base | Changed / certified | Changed populated base |
|---|---:|---:|---:|---:|
| X | 19,219 | 10,776 | 132 / 132 | 0 |
| Installed X | 19,219 | 10,776 | 132 / 132 | 0 |
| R | 953 | 216 | 0 / 0 | 0 |
| T | 61,712 | 27,452 | 0 / 0 | 0 |

No duplicate site/native keys, population drift, metadata mismatch or uncertified changed row. All **49,220** populated-base comparisons are unchanged; X snapshots are not additive. The retained populations are complete, not a sample. Fresh rehash covered 2,013 source entries and 3,520 oracle-input entries across streams; only S3 differs. Base/facts streams and oracle probe source match their receipts.

This validates the historical **static terminal correspondence** counts. It does not prove refusal closure: the native checker can identify the original method even where a static writer requires prism to keep base. W1/W2 demonstrate that distinction. I did not independently rerun public corpus tools or certify unchanged yield after either fix. F aggregates are inherited from the brief only.

Supplied final gates report **5,168 passed / 0 failed / 1 existing skipped**, 2 doctests, fmt/clippy pass, matrix 182 OK and S1b-4 411 controls / 639 sites / 822 byte-identical comparisons. I read their receipts and verified hashes; I did not execute those gates. Cache epochs 118/74 match the candidate. These gates do not eliminate W1/W2.

## Controller convergence classification

**Two closed, enumerable material WRONGs; no new language-mechanism family outside the round-1/channel-table scope.** W1 concerns static namespace contents; W2 concerns unavailable static forwarding proofs. Both violate existing refusal obligations and have concrete inputs, same-environment empty-main controls, incorrect head admission and bounded code seams. Fold targeted fixes into the existing artifact, update the static/runtime distinction in §2a, and rebind regression, measurement and gate evidence. Do not restart, silently dispatch round 3, or infer adoption from this review. Any cap extension is the controller's explicit convergence decision.

## Not checked

- No independent build/full suite, doctests, clippy/fmt, mutation execution, Tier-A matrix/quick/full, or new binary build provenance beyond input/hash binding to retained receipts.
- No fresh public X/installed-X/R/T tool/native census execution or post-fix yield measurement.
- No private F access or independent verification of the brief's F aggregates.
- No exhaustive grammar fuzzing, all static forwarding/configuration variants, platform/performance behavior or full cache-invalidation replay. I read the cumulative selector/admission/cache changes; the focused probes cover the stated joins, not every landed resolver assumption.
- No claim of runtime safety for the expressly excluded F3/F4/F7 channels; no Git writes, source repair, implementation dispatch, adoption, publication or merge.

**Verdict: FIX — two closed, enumerable WRONG / MATERIAL refusal-join defects. Round 2 of 2 is complete.**
