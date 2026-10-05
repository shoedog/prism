# S2 specification review — round 1 of 2

**Findings: 5 WRONG / MATERIAL; 2 SMELL / IMMATERIAL.** The prototype adds reproducibly false Exact edges. The fixes below can be folded into the existing artifact; no restart is recommended.

## Review binding and evidence

Reviewed the requested SPEC, IMPLEMENTOR, OQ-s2, MEASUREMENTS, PROBES and VERIFICATION packet at clean branch `review`, HEAD `b8f5b2f3fdc39de32877952fdb4ccf01f1483548`. Product source references below refer to **`c35719e1132809dc835f64f476cf31e2c18bd35f`**, whose parent is immutable main `4e592daa7858a195eb3a9eb77c83dfbc763b49fa`; the new implementation files are absent from the review checkout's working tree.

All **770** manifest source inputs match the prototype's Git objects. The frozen base, head and head-facts executables match BUILD-MANIFEST hashes. I used those existing executables, without rebuilding or changing source. **31** retained receipt hashes also match. Details: [custody-binding.json](spec-r1-sol-evidence/custody-binding.json).

For every counterexample, `app.jsx` / `app.tsx` contains:

```js
import { C as X } from './m';
export function run() { return X.sm(); }
```

The default-export variant uses `import X from './m'`. Fixtures use an explicit local tsconfig with `allowJs`, `jsx: preserve`, Node module resolution and `include: ["**/*"]`. The same-environment immutable-main executable drops the affected `run` call; the prototype adds singleton Exact `import_qualified` targeting the original `m:sm` definition. Runtime controls execute the replacement. Provider mutations run during module initialization; importer mutations additionally have indexed entry modules that load/await the writer before invoking `run`.

Raw sources, complete base/head rows, facts and runtime outputs are retained under [spec-r1-sol-evidence](spec-r1-sol-evidence/). Representative cases also reproduce with `.js` and `.ts` files, beyond the `.jsx` / `.tsx` population. Commands were:

```text
<main-prism|head-prism> nav --no-cache call-stats --repo <fixture>/source --dump-sites
<head-dump_imports> <fixture>/source
node <runtime>/entry.mjs
```

Frozen executables are under `/Users/wesleyjinks/code/prism-s2-plan/target/s2-plan/bin/`. Hypothesis, prediction, falsifier, alternatives and results are recorded in the evidence's probe-log JSON files. No product edits, Git writes, installs, network access, subagents or private-F reads occurred.

## WRONG findings

### W1 — Dynamic namespace escapes do not join qualifier identity

**Tag: WRONG / MATERIAL. Confidence: 100/100. Reproduced.**

**Evidence:** `src/ast/js_import_qualifiers.rs:339-345` visits identifier tokens but not string keys; `src/call_graph.rs:2335-2343` joins global refusals only to `identity.local`; `src/call_graph.rs:2349-2364` obtains module refusal joins from static import bindings and forwarding exports. SPEC:30,44 claims closure for dynamic namespace paths, but these paths have no import binding.

```js
// m
export class C { static sm() { return 0; } }

// other
function replacement() { return 1; }
export async function mutate() {
  const key = String.fromCharCode(67);
  const A = (await import('./m'))[key];
  A.sm = replacement;
}

// entry
import { mutate } from './other';
import { run } from './app';
await mutate();
run();
```

The writer has no captured import bindings and no refused name `C`. `run` executes `replacement`, returning `1`, but head adds Exact to the original method returning `0`. Both grammars reproduce. The same failure occurs with `["C"]`, quoted destructuring `const {"C": A} = await import('./m')`, and an exported default class accessed through `(await import('./m')).default`. In the latter, the refusal is spelled `default`, while the identity's local name is `C`.

Receipts: [reachability-results.json](spec-r1-sol-evidence/reachability-results.json), `reachable-dynamic-*/`; same-object controls in `dynamic-*/runtime.json` establish `A === C` and result `1`.

**Fix:** Capture refusal-only facts for runtime dynamic-import namespace consumption and join them to the module's qualifier identities, independently of identifier/property spelling. An unknown member must revoke the entire joined module's qualifier set; an unproved module selection must conservatively revoke its possible indexed targets. Preserve the existing base fence. Adding string tokens alone is insufficient: the computed-key control still escapes it.

**Alternative:** Refuse S2 admission wherever an unproved dynamic namespace escape could reach the qualifier, including a conservative whole-project cut for unresolved module expressions. This is simpler and sounder but can cost X yield; measure that cost before adoption.

**Self-critique:** This does not apply to a namespace whose source identity is proved unrelated to the qualifier, or to files outside the explicitly indexed universe. These writers and their execution entry are indexed, and the literal module resolves to the same class. No CommonJS or external runtime code is needed.

### W2 — Permitted construction exposes the class through the result

**Tag: WRONG / MATERIAL. Confidence: 100/100. Reproduced.**

**Evidence:** `src/ast/js_import_qualifiers.rs:394-397` admits `new C` without checking consumption of its result; SPEC:35 permits construction. Refusal of `A` or the property name `constructor` does not join back to identity `C`.

```js
// m
function replacement() { return 1; }
export class C { static sm() { return 0; } }
const A = (new C()).constructor;
A.sm = replacement;
```

All spelled uses of `C` pass the whitelist, yet the ordinary instance prototype exposes the class. Both grammars and `.js` / `.ts` controls add Exact to the original method on a base drop. Runtime proves `A === C` and `C.sm() === 1`.

Receipts: `instance-constructor-{jsx,tsx}/`, [third-results.json](spec-r1-sol-evidence/third-results.json), [extension-results.json](spec-r1-sol-evidence/extension-results.json).

**Fix:** Strengthen the construction proof: a consumed/escaping construction result cannot certify class member-write closure. A bounded conservative rule can permit only discarded direct construction results, with constructor/heritage closure separately proved as in W3. State the restriction in SPEC and add the result-to-constructor regression.

**Alternative:** Refuse a qualifier upon any construction use. This avoids instance identity analysis but loses permitted construction positives and must be reconciled with SPEC and measured yield.

**Self-critique:** The example needs an ordinary reachable `prototype.constructor` and consumption of the result. It does not refute discarded construction of a class whose constructor effects are independently closed. The existing positive test at `tests/integration/js_import_qualifiers_test.rs:775-780` exercises discarded constructions, so a consumed-result refusal can preserve that positive.

### W3 — `new.target` and `super` bypass both class write guards

**Tag: WRONG / MATERIAL. Confidence: 100/100. Reproduced.**

**Evidence:** `src/ast/js_import_qualifiers.rs:339-345` omits `meta_property` and `super`; `src/ast/js_import_qualifiers.rs:476-488` recognizes member-write roots only as identifier or `this`. Member capture at :241-242 skips instance constructors, but their effects can mutate the class's static methods.

```js
// m
function replacement() { return 1; }
export class C {
  constructor() { new.target.sm = replacement; }
  static sm() { return 0; }
}
new C();
```

Head adds Exact to the original `sm`, although module initialization has replaced it. Both grammars reproduce and runtime returns `1`. A constructor returning `new.target` also yields a writable alias. Mapping only to the constructor's nearest class is insufficient: `class B { constructor(){new.target.sm=replacement;} }` followed by `export class C extends B { static sm(){return 0;} }` and `new C()` reproduces too.

A second member-write root reproduces in both grammars:

```js
function replacement() { return 1; }
class B { static sm() { return 2; } }
export class C extends B {
  static sm() { return 0; }
  static init() { super.sm = replacement; }
}
C.init();
```

The assignment uses `C` as its receiver and replaces its own `sm`; head still targets the original body. Refusing the heritage identifier `B` does not revoke `C`.

Receipts: [initial-results.json](spec-r1-sol-evidence/initial-results.json), [second-results.json](spec-r1-sol-evidence/second-results.json), `new-target-*`, `super-write-*` and their runtime outputs.

**Fix:** Treat these as runtime class identity/receiver carriers in the refusal proof. Conservatively refuse the affected class for unproved `new.target` or `super` effects, and refuse classes with runtime heritage when superclass constructor/receiver effects cannot be closed. Include inherited `new.target` in the regression population; a nearest-class-only patch misses it.

**Alternative:** Temporarily exclude class qualifiers with explicit constructors or heritage. This is a smaller admission surface and requires a yield replay, but avoids introducing positive inheritance or instance analysis.

**Self-critique:** `new.target` in a constructor not reached by construction, or `super` used solely for a harmless read, need not cause an actual write. These controls execute the constructor/static initializer before the affected call and demonstrate replacement. The recommendation intentionally allows conservative refusal of harmless cases.

### W4 — A nested object initializer steals the surrounding `this` carrier

**Tag: WRONG / MATERIAL. Confidence: 100/100. Reproduced.**

**Evidence:** `src/ast/js_import_qualifiers.rs:353-366` chooses the nearest object variable declaration without accounting for the evaluation context of the occurrence. SPEC:42 treats that lexical carrier mapping as closure authority.

```js
// m
function replacement() { return 1; }
export class C {
  static sm() {
    const box = { value: this };
    return box.value;
  }
}
const A = C.sm();
A.sm = replacement;
```

Here `this` is `C`, not `box`. The walk refuses `box` but leaves `C` captured, so head adds Exact to the now-replaced method. Runtime establishes `A === C` and result `1`. The arrow variant `const box={leak:()=>this}; return box.leak();` reproduces too. Both grammars and the representative `.js` / `.ts` case fail. An anonymous object control correctly refuses `C`; that control does not cover the named nested-object mistake.

Receipts: `this-nested-object-*`, `this-nested-arrow-*`, [third-results.json](spec-r1-sol-evidence/third-results.json).

**Fix:** Determine `this` ownership from execution contexts: object value initializers do not create a receiver, and arrows inherit the enclosing receiver. When that proof is unavailable, revoke every plausible surrounding qualifier carrier rather than stopping at the nearest syntactic object. Apply the same rule to the namespace marker.

**Alternative:** Conservatively apply a refused `this` use to all enclosing class/object/namespace candidates, retaining an unknown-receiver cut where needed. This is easier to audit but over-refuses unrelated enclosing carriers.

**Self-critique:** A real method on `box` can have `box` as its receiver. These occurrences are an initializer and an arrow capturing the outer static-method receiver. A direct object-import control had a populated base edge and remained unchanged; I exclude that inherited behavior from this finding. The reported class cases are new S2 edges on actual main drops.

### W5 — Direct builtin `eval` leaves qualifier closure certified

**Tag: WRONG / MATERIAL. Confidence: 100/100. Reproduced.**

**Evidence:** `src/ast/js_import_qualifiers.rs:15-24,333-391` refuses only the spelled identifier `eval`; it leaves `complete=true` and the class locally captured. The importer join at `src/call_graph.rs:2350-2353` likewise does not revoke imported `C` when only `eval` is refused. B0 at `src/ast/js_binding_checks.rs:19-33` checks node classification and escaped names, not direct-eval effects.

```js
// m
function replacement() { return 1; }
export class C { static sm() { return 0; } }
eval('C.sm = replacement');
```

Strict ESM direct eval can mutate the class object's property. Both grammars add Exact to the original body on a base drop, and runtime returns `1`. A distinct importer also reproduces with `import {C as Y} from './m'; ... eval('Y.sm = replacement')`; an indexed entry imports that writer before invoking `run`.

Receipts: `eval-write-*`, `reachable-eval-importer-*`, [second-results.json](spec-r1-sol-evidence/second-results.json), [reachability-results.json](spec-r1-sol-evidence/reachability-results.json).

**Fix:** An unproved possible builtin direct-eval call must invalidate the file's qualifier closure, suppress local captures and revoke its imported qualifier identities globally. Do not treat successful syntactic B0 cleanliness as proof against evaluated code. Account conservatively for possible indexed namespace access from evaluated code as well.

**Alternative:** Permit only calls whose binding is proved to be a local non-eval function; otherwise use a broader dynamic-code refusal. This retains harmless shadowed `eval` calls but adds binding-proof work.

**Self-critique:** A proved custom function named `eval` would not have builtin eval semantics. These examples use the actual builtin with a literal string and a legal property mutation; there is no imported-binding reassignment, invalid `with`, CommonJS or external code assumption.

## SMELL findings

### S1 — The positional-proof mutant and outstanding gates remain disclosed gaps

**Tag: SMELL / IMMATERIAL to the demonstrated goal failures.**

**Evidence:** VERIFICATION:55-57, PROBES:45, and `mutants/lane-s2-import-qualifiers.json:11-15`. The hash-matching advisory receipt reports **8 selected /8 admissible /7 killed**, with S2-02 **SURVIVED**. Six anchors were omitted. Tier-A quick and the authoritative all-14 registry gate remain unverified here.

**Fix:** Retain the survivor explicitly and seek a realistic shadowed call with an empty actual-main outcome that exercises the positional proof. Run the authoritative registry after corrective source edits and the required Tier-A quick before transferring acceptance.

**Alternative:** Retain the coverage gap as an explicit adoption condition while strengthening selectors and documenting which independent guards prevent the tested shadows from reaching S2. Do not call the mutant equivalent without a mechanism-level proof.

**Self-critique:** No false Exact attributable specifically to removing positional proof was demonstrated. Existing populated base results can keep a shadow test green through the base fence. This is not an additional blocker; W1-W5 independently establish the safety failures.

### S2 — IMPLEMENTOR is a stale workspace dispatch, not a complete committed-prototype dispatch

**Tag: SMELL / IMMATERIAL.**

**Evidence:** IMPLEMENTOR:5 says source/tests are uncommitted at planning HEAD `6e4e0ef1`; :11 directs the controller to commit those files. The supplied prototype is already committed as `c35719e1`, directly on `4e592daa`, while this clean plan checkout is `b8f5b2f3`. The reproduction commands at :17-24 assume implementation bytes and frozen tools already exist locally; neither the cherry-pick nor the frozen-tool handoff is specified for this checkout.

**Fix:** Name `c35719e1132809dc835f64f476cf31e2c18bd35f` as the controller's source application/cherry-pick artifact onto a bound implementation checkout, distinguish that action from adoption, and specify the tool/evidence handoff or fresh rebuild. State the preservation obligations: populated base rows, landed CallSite projection, existing Tier-A baselines, immutable base tools, and hash-identical tested source until an explicit corrective fold regenerates its manifest and gates.

**Alternative:** Dispatch a controller-prepared implementation checkout with explicit HEAD/tree/tool bindings. This reduces implementer steps but requires that prepared checkout to be named and verified.

**Self-critique:** The launch brief supplies the missing prototype identity, so this review is unambiguous. No mistaken implementation or measurement resulting from the stale instructions was observed; this is a standalone handoff risk, not a material WRONG.

## Answers to the review questions

1. **The whitelist is not closed over runtime qualifier identity.** W1-W5 demonstrate escapes or writes without a refused occurrence of the correct binding name. The refusal set is name-keyed per file and then also flattened globally by local name. Importer/forwarder joins revoke identities across all project tables when their refusal facts exist. Two unrelated bindings with the same spelling can conservatively lose S2 gains; that permitted cost does not change a populated base edge. JSX names, escaped identifiers, TS import aliases, `export =`, and `as`/`satisfies` value escapes in my controls kept base. `nested_identifier` contains identifier/property children; excluded type and label token kinds are not by themselves a demonstrated runtime escape. These observations are not a universal grammar-closure certificate.
2. **The member-write and carrier rules are unsound as currently composed:** W2-W4. Bare default export plus active-use checks do not close a dynamic `.default` importer: W1. Static namespace-import joins are conservative, but dynamic namespace consumption lacks the necessary refusal join. The populated-base fence at `src/resolution.rs:2304-2307` is sound for preservation and remained intact in every audited public row.
3. **The static measurement population is complete and correctly joined, but is insufficient evidence of mutation closure.** I independently re-read all retained base/head rows, checked unique equal key populations, unchanged metadata and every populated base row, and recomputed every changed row's module/owner/full-span predicate. Current source hashes and native-host-decoded input hashes match the receipts; binary bindings match. Results:

   | Corpus | Sites | Populated base rows preserved | Changes agreeing statically | UNJOINABLE |
   |---|---:|---:|---:|---:|
   | X | 19,219 | 10,776 | 132 | 2,967 |
   | Installed X | 19,219 | 10,776 | 132 | 2,967 |
   | R | 953 | 216 | 0 | 207 |
   | T | 61,712 | 27,452 | 0 | 7,473 |

   Receipt: [public-audit.json](spec-r1-sol-evidence/public-audit.json). X snapshots are not additive. This is an audit of retained measurements, not a fresh corpus replay. No particular public-X survivor was demonstrated wrong. However, fresh pinned-native runs on ten false-Exact controls satisfy the very same static CORRECT predicate, including module, owner and full span: [oracle-results.json](spec-r1-sol-evidence/oracle-results.json). Static agreement cannot replace the missing source-effect proof. The first raw-byte native-input hash probe was inadmissible because TypeScript strips a UTF-8 BOM; repeating with the pinned host's text decoding resolved it without an input-drift claim.
4. **Existing controls are useful but insufficient.** Productive class/field/namespace/object controls compare to drops; the clean class control here also fails admission on actual main and succeeds on head. Refusal-only tests can pass on main because the feature is absent; their meaningful RED control is an admitting pre-whitelist prototype, not necessarily main. The supplied source-bound receipts support 218 E5 controls, S2-14's kill, 5,154 suite passes plus two doctests, but omit W1-W5. Add each displayed case, its realistic execution path and an edge/positive control, and accumulate the complete failing population before any corrective retry. The in-test `base(g)` helper clears S2 tables; retain actual-main executable controls for attribution rather than treating that helper as an independent historical build.
5. **Dispatch is not self-contained after publication:** S2. Gates and controller ownership are described, but the exact committed-source application and tool handoff need refreshing. Fold fixes into the existing artifact, then rebuild/rebind and replay all affected measurements. Do not transfer the current zero-false-Exact claim to corrected bytes without new evidence.

## What I did not check

- No fresh build, full test-suite execution, Clippy, Tier-A matrix/quick/full run, mutation run or fresh public corpus execution. Full-suite and mutation totals above are supplied hash-matching receipts, not my executions.
- No private F source or receipts. The launch's F zero-change result is supplied information only; the packet's earlier F census is not substituted for head acceptance.
- No exhaustive JS/TS/TSX grammar census, compiler-cleanliness certification of every synthetic TypeScript program, or runtime execution of every public-X survivor. Label/type/decorator/`arguments`/`with` coverage was not independently certified exhaustively.
- No Linux/case-sensitive, concurrent-tree, performance, cache-reload or detached-owner validation.
- No repair, commit, push, adoption, merge, memory change or second review round. The source remains unchanged and the review cap remains two.

**Verdict: FIX — five closed, enumerable material WRONG findings. Fold fail-closed fixes and regressions into the existing prototype, refresh the specification and custody/measurement chain, then use round 2 of 2 for the cumulative review.**
