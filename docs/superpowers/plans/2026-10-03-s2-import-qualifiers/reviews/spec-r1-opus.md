# S2 spec review, round 1 of 2: Opus

**Reviewer:** Claude Opus 5.5 (subagent), 2026-10-03
**Scope:** plan packet on branch `review` (= `plan/s2-import-qualifiers` `b8f5b2f3`, PR #343) and prototype `c35719e1` (`origin/proto/s2-import-qualifiers`, parent = main `4e592daa`).
**Goal under review:** correct Exact `ImportQualified` edges for JS/TS imported class/object qualifiers, with **zero false Exact edges** (S2-O6, "Ship X gain, fail-closed").

## How the evidence was produced

- I extracted `c35719e1` with `git archive` into `<checkout>/target/proto` (no git writes) and ran `cargo build --offline --release`.
- I added one scratch integration test, `s2_review_probe.rs`, to that extracted tree only. It uses the prototype's own harness (`js_paths_common::{graph,outcome,write}`, with `base()` clearing `js_ts_qualifier_modules/_exports`, exactly as `js_import_qualifiers_test.rs` does). For each scenario it compares the head and base outcome of `app.{jsx,tsx}`'s `X.sm()` row, with `app = import { C as X } from './m'; export function run() { X.sm(); }` unless a case overrides it.
- **REPRODUCED** below means: base has 0 targets, and head has 1 Exact `import_qualified` target on `m.*:sm`, in **both grammars**. The positive control (case 00) behaves exactly that way.
- **RUNTIME-CONFIRMED** means a Node 24 ESM script (`.mjs`) showed that `C.sm()` then calls the replacement rather than the body Prism points to. The script printed `inherited,valueOf,ctor,eval,dynstr,forin,getter` and `dyndefault`.
- All scratch files and `target/` were deleted afterward.
- I also checked that all 770 `BUILD-MANIFEST.json.source_inputs_sha256` entries match `c35719e1` byte-for-byte. The committed prototype is the measured source.

## Findings: WRONG first

### W1. WRONG · MATERIAL: the "direct call" arm admits calls to members C does not own, and those can return or mutate C

**Evidence.** `src/ast/js_import_qualifiers.rs:394-413` (`js_ts_qualifier_direct_use`) and `:415-418`. Any `Q.<anything>(...)` is allowed, whatever the member. But the qualifier object inherits callable members it does not own:
- `Object.prototype.valueOf` returns the receiver itself;
- `__defineGetter__` / `__defineSetter__` redefine a property on the receiver;
- a superclass's static methods run with `this === C`.

The `this` carrier mapping (`:347-370`, and `:488-513` in the write walk) maps `this` inside `class B` to `B`, never to the subclass `C` that is actually the receiver. Each of these reproduces in both grammars:

```js
// m.jsx / m.tsx (case 25, REPRODUCED, RUNTIME-CONFIRMED pattern)
// base.jsx: export class B { static self() { return this; } }
import { B } from './base';
export class C extends B { static sm() { return 1; } }
// other.jsx
import { C as Q } from './m'; Object.assign(Q.self(), { sm: () => 2 });
```
```js
// other.jsx (case 23 / 24, REPRODUCED, RUNTIME-CONFIRMED)
import { C as Q } from './m'; const v = Q.valueOf(); v.sm = () => 2;
import { C as Q } from './m'; Object.assign(Q.valueOf(), { sm: () => 2 });
// case 16 (REPRODUCED, RUNTIME-CONFIRMED)
import { C as Q } from './m'; Q.__defineGetter__('sm', () => () => 2);
```
```js
// m.jsx (case 04, REPRODUCED, RUNTIME-CONFIRMED): inherited static writes through `this`
class B { static install(f) { this.sm = f; } }
export class C extends B { static sm() { return 1; } }
C.install(() => 2);
// case 05: the same with B in another file and the call Q.install(...) in a third file. REPRODUCED.
```

In every case head adds Exact `X.sm() -> m:sm`, while at runtime `C.sm` is the replacement.

Note: `Q.valueOf().sm = ...` written as one chain (case 01) is already caught by the existing binding-write core. Splitting it through a variable or a call argument escapes.

**Fix (recommended).** Make the call arm identity-closed: allow `Q.m(...)` (and `this.m(...)` on a mapped carrier) only when `m` is in the captured own member set of the joined identity. Per file, record `(local name, member)` call pairs instead of a bare allowed bit. In `apply_js_paths`, revoke an identity if any joined pair names a member outside `identity.members`. Captured members are binding-core Callables whose own `this` uses are already whitelisted, so calling one cannot hand out C. Add a belt-and-braces rule: refuse class qualifiers that have any `class_heritage`. Inherited dispatch, `super.x()` with `this = C`, and base-class `this` writes then become structurally impossible.
- **Alternative A:** heritage refusal plus a denylist of `valueOf`, `__defineGetter__`, `__defineSetter__`, `__lookupGetter__`. It is cheaper, but it is a blacklist, contrary to S2-O6's "closed whitelist, not an expanding syntax blacklist", and it stays open to host-specific prototype additions.
- **Alternative B:** keep the current arm and disclose the gap. Not compatible with "zero false Exact".

**Yield impact (static grep, public X only).** The four survivor classes (`API` in `tests/helpers/api.ts:61`, `Keyboard` and `UI` in `tests/helpers/ui.ts:65/449`, `EditorLocalStorage` in `data/EditorLocalStorage.ts:5`) have no heritage, and there are no `.valueOf()`, `__define*`, `.constructor.x=` or `eval(` hits anywhere in X's `.ts/.tsx/.js/.jsx`. I expect the fix to keep +132, but the controller must re-measure.

**Self-critique.** This does not apply if the owner deliberately defined "direct call" as purely syntactic and accepts prototype-chain residue. SPEC §2 frames the arm as closing identity escapes, so I read it as covered by the zero-false-Exact goal. The own-member rule could lose rows where code calls a non-captured static, such as a getter-backed one. That must be measured, not assumed.

### W2. WRONG · MATERIAL: the "construction" arm is not identity-closed, because `instance.constructor === C`

**Evidence.** `src/ast/js_import_qualifiers.rs:396-397` allows `new C(...)` with no restriction on the resulting instance. `instance.constructor` is C.

```js
// other.jsx / other.tsx (case 15, REPRODUCED both grammars, RUNTIME-CONFIRMED)
import { C as Q } from './m'; new Q().constructor.sm = () => 2;
```

Head adds Exact `X.sm() -> m:sm`; the runtime calls the replacement. The same holds for `Object.getPrototypeOf(new Q()).constructor`, and for computed `inst['constructor']`, which no lexical token check can see.

**Fix (recommended).** Remove `new C(...)` from the whitelist for class qualifiers that S2 admits, measured first. On public X, the survivors are test-helper/static-utility classes, and I found no `new API/new UI/new Keyboard/new EditorLocalStorage` usage in a grep. I did not run an exhaustive check.
- **Alternative A:** keep `new`, but revoke every constructed class identity whenever any visible file spells a `constructor` property or string token outside a class-body method name. Cheaper on yield, but computed keys (`inst[k]`) still escape, so it is not closed.
- **Alternative B:** owner-signed disclosed residual.

**Self-critique.** This does not apply if a class sets its own non-writable/overridden `constructor` prototype property, or if the owner rules instance back-pointers out of scope. Both are unusual, and neither is in SPEC.

### W3. WRONG · MATERIAL: module namespaces reached without a named import binding escape. Dynamic `import()`, TS `import = require`, string keys, `for…in`, and the `default` exported name

**Evidence.**
- `src/call_graph.rs` S2 block in `apply_js_paths`: refusal joins come only from `import_bindings` or from the lexical set matched against `identity.local`.
- A dynamic `import('./m')` or `import K = require('./m')` creates a namespace value with no import binding, so it is never joined to module `m`. Its property accesses are caught only when they spell an identifier equal to the defining module's *local* name.

| Case | `other.{jsx,tsx}` | Result |
|---|---|---|
| 10 | `export async function f() { (await import('./m'))['C'].sm = () => 2; }` | REPRODUCED, RUNTIME-CONFIRMED |
| 11 | `export async function f() { const ns = await import('./m'); for (const k in ns) ns[k].sm = () => 2; }` | REPRODUCED, RUNTIME-CONFIRMED |
| 30 (tsx) | `import K = require('./m'); K['C'].sm = () => 2;` | REPRODUCED |
| 17 | m: `class C {static sm(){return 1}} export default C;` · app: `import X from './m'; X.sm()` · other: `(await import('./m')).default.sm = () => 2` | REPRODUCED, RUNTIME-CONFIRMED (`dyndefault`) |
| 18 | m: `export default class C {...}` · other: `const { default: K } = await import('./m'); K.sm = () => 2;` | REPRODUCED |

Cases 17 and 18 show the join is keyed on the defining *local* name: the refused token `default` never matches `identity.local == "C"`. SPEC §2 claims "Refused local names are applied across every visible file, including dynamic namespace accesses that lack named import bindings". That holds only for the spelled-local-name sub-case that `qualifier_whitelist_namespace_paths_reexports_and_this_keep_base` tests.

For contrast, these close correctly: the static `import * as ns` string key (case 12) and a script-file dynamic import (case 19).

**Fix (recommended).** Treat every `import(<string literal>)`, `require(<string literal>)` and `import K = require(...)` as a namespace binding for *refusal joins*, never for positive authority. Resolve it with the same retained hop / relative candidates. Any such namespace value revokes all qualifiers exported by that module, as `import * as ns` already does. In addition, match the lexical refusal set against each identity's exported names (including `default`) as well as `identity.local`. A non-literal `import(expr)` anywhere should be treated like an opaque namespace, because it can reach any visible module. My recommendation is to revoke every identity, measured first.
- **Alternative A:** revoke all qualifiers of any module that is dynamically imported anywhere, without trying to classify uses. This is simpler. Grep shows none of X's survivor modules is dynamically imported.
- **Alternative B:** whitelist `(await import(x)).C.m()` as a direct call. More yield, but more surface. Not recommended.

**Self-critique.** If the owner scopes "visible universe" to static ESM edges only, the computed/`for…in` part becomes a disclosed residual. But SPEC explicitly claims dynamic namespace coverage, and the `.default` sub-case is purely an identifier-token join bug.

### W4. WRONG · MATERIAL: refusing a re-exported namespace (`export * as M`) revokes only the namespace identity, not the classes in M's module

**Evidence.**
- `src/js_import_qualifiers.rs:135-159` builds `QualifierIdentity { file: m, local: "*namespace*" }`.
- The refusal loop in `call_graph.rs` revokes `(identity.file, identity.local)` for the matched export name only. A refused `M` therefore revokes `(m, "*namespace*")`, while `(m, "C")` survives.
- `Object.values(M)` exposes C without spelling it.

```js
// nsb.jsx: export * as M from './m';
// other.jsx (case 13, REPRODUCED both grammars)
import { M } from './nsb'; Object.values(M).forEach((c) => { c.sm = () => 2; });
// case 14 (REPRODUCED both grammars)
import * as B from './nsb'; Object.values(B.M).forEach((c) => { c.sm = () => 2; });
```

The same gap applies to a `this` occurrence whose carrier is unknown (`"*namespace*"` at `:370`). That cuts only namespace identities, not the class identities living in the receiving module. For example, `export function f(){ Object.values(this)… }` called as `ns.f()` where only `ns.f` is admitted. I did not reproduce this sub-case: my attempt (case 20) had a populated base.

**Fix (recommended).** Whenever any namespace-valued identity for module `m` is revoked (named `export * as`, a namespace import, a dynamic namespace from W3, or an unknown-carrier `this`), revoke every identity whose `file == m`, plus identities forwarded through `m`'s named or star re-exports, transitively within the existing hop cap.
- **Alternative:** also drop the declared/re-exported-namespace and object-literal *admission* mechanisms from this slice. They contribute **0** measured X/R/T rows (MEASUREMENTS table) and are the largest remaining surface (`*namespace*` carriers, two of the three repair extensions). This alternative alone does **not** fix W4, because the class-identity revocation is still needed.

**Self-critique.** None found. This is a join omission with a reproduced false Exact.

### W5. WRONG · MATERIAL: direct `eval` can mutate a qualifier, and string contents are never inspected

**Evidence.** Strict-mode (module) direct eval still resolves outer lexical bindings.

```js
// other.jsx (case 09, REPRODUCED both grammars, RUNTIME-CONFIRMED)
import { C as Q } from './m'; eval('Q.sm = () => 2');
// m.jsx (case 26, REPRODUCED both grammars)
export class C { static sm() { return 1; } }
eval('C.sm = () => 2');
```

The walk sees only the identifier `eval`, refuses the name "eval", and leaves `Q`/`C` untouched.

**Fix (recommended).** Treat any call whose callee is the bare identifier `eval` as `complete = false` for that file. That revokes every binding it imports, and in a provider it suppresses capture.
- **Alternative:** a global poison, where any visible direct eval revokes all identities. It is simpler and stricter, but costs yield in repos with eval in tooling.

**Self-critique.** Indirect eval (`(0, eval)(…)`) runs in global scope and cannot see module bindings, so it needs no rule. This is rare in application code but trivially closed.

## SMELLs

### S1. SMELL · MATERIAL: the closure argument rests on probe-enumerated families. The oracle is static by construction, and round 1 found five new families

**Evidence.**
- `compare-head.py` labels rows `CORRECT_STATIC_BINDING`. It correctly requires: empty base; one Exact target; a native Callable; native module == Prism proof module; native owner == Prism owner; a unique qualifier declaration; and file/name/start/end equality, over the complete site-key population with metadata equality.
- So "132 CORRECT" is a **complete population**, not a sample, and the measurement chain is admissible for what it claims: static binding correctness.
- But it cannot detect identity escapes, which the packet itself notes for S2-W1. The zero-false-Exact property therefore depends entirely on the whitelist, and W1–W5 show the whitelist arms were specified against observed probes rather than against the language's identity-exposing mechanisms.

**Fix.** Before round 2, write a short enumerated table in SPEC §2 of the ECMAScript mechanisms that expose an object without spelling its binding, each mapped to the arm or join that refuses it:
1. receiver-returning or receiver-mutating built-ins;
2. inherited/`super` dispatch;
3. instance back-pointers;
4. namespace objects, both static and dynamic;
5. eval;
6. string-keyed and reflective access.

This makes the closure claim reviewable rather than inductive.

**Self-critique.** If the owner accepts round 2 as the last discovery pass, this is process advice only. But at the review cap, an open-class population parks the slice, so doing this now protects convergence.

### S2. SMELL · MATERIAL (unreproduced): files with a parse error and no export/import facts are dropped, together with their refusals

**Evidence.** `src/call_graph.rs:2192` keeps facts only when `!exports.is_empty() || !has_error()`. `JsExportFacts::is_empty` now includes `!qualifiers.complete` but not `qualifiers.written`. A file that tree-sitter fails to parse (a grammar gap on valid JS, e.g. import attributes) and that has no static import/export, such as `import('./m').then(m => { m.C.sm = f })`, loses its lexical refusals.

**Fix.** Keep facts when `qualifiers.written` is non-empty. Better, treat any parse-error JS/TS file as an opaque namespace writer (revoke all qualifiers it names, and on any dynamic import revoke per W3).

**Self-critique.** If the file is genuinely invalid JS, the runtime never executes it, so there is no wrong edge. I did not reproduce a grammar-gap case.

### S3. SMELL · IMMATERIAL: the mutant registry is coarse on the predicate, and S2-02 is still unkilled

**Evidence.** `mutants/lane-s2-import-qualifiers.json`: S2-14 deletes the whole predicate, and S2-11 deletes all revocation. No mutant targets:
- an individual whitelist arm (the type-ancestor arm `:419-432`, the `export_specifier` alias equality `:449-453`, the default-plus-active rule `:390`);
- the `this` carrier mapping;
- the global lexical join `lexical_refusals.contains(&identity.local)`;
- the namespace-importer revocation branch (`module.is_none() && member.is_none()`).

On S2-02 (position proof): I tried two base-dropped shadow negatives (a param shadow and a local `const X = D` shadow, each with a second class owning `sm`). Base was populated (2 NameOnly targets) in both grammars, and applying the S2-02 mutant locally left them unchanged. This agrees with the planner's finding: still a coverage SMELL, with no equivalence claim.

**Fix.** Add four or five arm-level mutants with the existing E5 tests as killers. Keep S2-02 disclosed.

### S4. SMELL · IMMATERIAL: the positive whitelist test asserts only the first `X.sm()` row

**Evidence.** `js_paths_common::outcome` returns the first row matching `(caller file, callee_text)`. `qualifier_whitelist_allows_declarations_calls_new_types_and_own_export` has two `X.sm()` sites (`run` and the shadowing `shadow(X)`), and only one is checked.

**Fix.** Key the lookup by caller function name, and assert the shadow site keeps base.

### S5. SMELL · IMMATERIAL: IMPLEMENTOR.md is stale relative to the actual landing state

**Evidence.**
- It still says "Product src/tests remain uncommitted" and planning HEAD is `6e4e0ef1`. In fact the prototype is commit `c35719e1`, whose parent is main `4e592daa`, on a separate `proto/` branch, while the plan branch is at `b8f5b2f3`.
- No cherry-pick or rebase instruction says how the product commit and the plan docs land together.
- The mutgate command's `--since 6e4e0ef1` is not an ancestor of `c35719e1`.
- The reproduction commands need `target/s2-plan/bin/*` frozen tools that exist only in the planner clone.
- On the positive side, the 770 source hashes match `c35719e1` exactly, and "base result byte-identical; cache 108/64; S1b-4 projection unchanged" is unambiguous.

**Fix.** Add a "Landing" step:
1. cherry-pick `c35719e1` onto the plan branch, or rebase the proto branch onto main and open the product PR from it;
2. use `--since 4e592daa` for scoped mutgate;
3. rebuild and rebind head binaries per the existing rebind rule;
4. state where the immutable base binary lives (`reference-binaries.json`).

## Answers to the brief's questions

1. **Is the whitelist closed?** No. Lexically, the identifier/property/shorthand/`this` walk is closed:
   - JSX element names, `nested_identifier`, decorators, `as`/`satisfies`, `export =`, `import x = require` names and class heritage all refuse;
   - `new.target` and `super.x =` writes are refused by cleanliness (cases 06/07 keep base);
   - labels are `statement_identifier`, which is not a value.

   But two allowed arms (call and `new`) are not identity-closed (W1, W2). The join leaks for namespaces reached without bindings and for exported-name aliasing (W3, W4). `eval` is unhandled (W5).

   On name-keying: refusal is name-keyed per file and applied globally by the defining local name. Importer refusals join via import bindings across all project tables. Two different bindings named `C` in one file over-refuse, which costs yield but is safe.
2. **Pre-whitelist rules.**
   - The member-write closure is sound as a secondary guard: it even catches the one-chain `Q.valueOf().sm =`.
   - The `this` mapping is unsound for inherited statics (W1).
   - Default export plus active use is sound for static imports, but the join misses the `default` exported name via dynamic access (W3).
   - Namespace imports are sound for static `import * as`, but miss re-exported namespaces (W4) and dynamic namespaces (W3).
3. **Measurement/oracle chain.** Admissible as a complete-population static correctness claim, covering module, owner, full span, unique declaration, population and metadata equality, with loss/unproven asserted zero. It is not evidence of identity closure (S1). A static grep of public X found no instance of the W1–W5 patterns around the survivor classes, so I expect the fixes to keep +132. The controller must still re-measure.
4. **Tests/controls/mutants.** The positives fail on main, because they assert base empty and then the hit. The negatives are head==base preservation checks, RED on the pre-whitelist prototype per the receipts. W1–W5 need RED/GREEN controls in both grammars in provider/importer placements. See S3/S4 for the mutant and test gaps.
5. **IMPLEMENTOR dispatch.** The invariants (byte-identical base rows, S1b-4 parity, cache 108/64, no restart) are unambiguous. The landing mechanics are stale (S5).

## Verdict

**FIX.** Five closed, enumerable WRONGs, each with a reproduced false Exact in both grammars and a bounded structural fix:
- W1: own-member call arm plus heritage refusal;
- W2: drop or guard `new`;
- W3: join dynamic and require namespaces, and join by exported name;
- W4: namespace revocation cascades to the module's identities;
- W5: eval poisons the file.

None requires abandoning the closed-whitelist design. They tighten its arm definitions and joins. Caution for convergence: these are five *new families* found in round 1. If round 2 surfaces further families after the S1 mechanism table is written, classify open-class and park per the convergence rule, rather than folding per-case.

## Not checked

- Private F, Tier-A quick/full, and nextest/clippy/fmt on the prototype. I ran only my scratch probe test and a release build.
- Yield after the recommended fixes. My expectation that +132 holds is grep-based only.
- `tree-sitter-typescript` corner cases where a runtime expression might sit under a `type_annotation` / `type_arguments` ancestor. I found none by reasoning, but did not fuzz.
- The class field initializer / `accessor` keyword / TS parameter-property shapes.
- `.mts`/`.cts` and CommonJS `module.exports` interplay beyond the `require` string case.
- The js_paths `for_qualifiers` selector changes (`src/js_paths.rs`) and the `apply_js_paths` project partitioning for positive authority.
- CONTROLLER-s2.sh execution (I only read it).
- The unknown-carrier `this` sub-case of W4 (case 20 had a populated base).
- R/T corpora.
