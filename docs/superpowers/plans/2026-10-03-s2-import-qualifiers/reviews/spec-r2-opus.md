# S2 spec review, round 2 of 2 (final under the cap): Opus

**Reviewer:** Claude Opus 5.5 (subagent), 2026-10-04
**Scope:** plan branch `review` = `plan/s2-import-qualifiers` `ecdb6b0f` (PR #343); prototype `origin/proto/s2-import-qualifiers` = `c35719e1` + `75a35a5e`.
**Goal (from brief):** correct Exact call edges for JS/TS import-qualifier calls, with **no false Exact edges within the static-binding contract** (S2-O7, CLAUDE.md:233-239). Measured yield: X +132, installed X +132, R 0, T 0.

**Verdict: FIX.** There is one material WRONG family, with closed and enumerable fixes. Namespace objects that reach the qualifier through static bindings are not joined back to the class identity. This is the static residue of round-1 Opus W4, which the repair classified wholly as F4 "enumeration". It is not a new family.

## How the evidence was produced

- I extracted `origin/proto/s2-import-qualifiers` with `git archive` into `<checkout>/target/proto`. I added one scratch integration test, `s2_r2_probe.rs`, which uses the prototype's own harness (`js_paths_common::{graph,outcome,write}` and a `base()` that clears `js_ts_qualifier_modules/_exports`, exactly as `js_import_qualifiers_test.rs` does). I ran it with `cargo test --offline --test integration s2_r2_probe -- --nocapture`. Each case was run in **both grammars** (`.jsx` and `.tsx` files), with the fixture's root `tsconfig.json`.
- **REPRODUCED** means: the base outcome for `app.*`'s `X.sm()` has 0 targets, and head has exactly 1 `exact`/`import_qualified` target on `m.*:sm`, in both grammars. The positive controls (`00-*`) behave exactly that way.
- **RUNTIME-CONFIRMED** means a Node 24 ESM `.mjs` replica of the same files printed `1`: `run()` executed the replacement, while Prism points to the body that returns `0`.
- Custody:
  - `75a35a5e` src/tests are byte-identical to `c35719e1` + `/Users/wesleyjinks/prism-evidence/s2/repair-r2/R2-src.patch` (I applied the patch and ran `diff -r`).
  - 769 of the 770 `BUILD-MANIFEST.json.source_inputs_sha256` entries match `75a35a5e`. The exception is the mutant registry (see W2).
  - The CLAUDE.md static-binding clause is on `origin/main` at lines 233-239, as SPEC §2a cites.
- RED control: I copied the R2 test file onto an extracted `c35719e1` tree. I stubbed the absent F8 schema field, `qualifiers.syntax_incomplete`, as the repair's VERIFICATION also discloses. Result: **26 pass / 5 fail**. The five failures are exactly the new-cut tests `repair_r1_{non_owned_calls,construction,receiver_carriers,nested_this,parse_incomplete}_keep_base`. The original-whitelist, static-alias, legacy-opacity and `*_out_of_model_*` tests pass on c357 as baseline parity. This reproduces VERIFICATION's claim.
- Scratch trees, `target/` and the runtime replicas were deleted afterward. No git writes.

## Findings: WRONG first

### W1. WRONG · MATERIAL: namespace objects reached through static bindings do not join back to class identities

The SPEC §2a row "Static named/default imports, direct namespace binding, named/star forwarding; lexically visible C alias/member write" is **in model, keeps base**. These shapes are exactly that: literal, lexically visible member writes, aliases or destructurings through static ESM bindings (named import, `export * as`, `import * as` + `export { N }`, tsconfig `paths`). None of them involves enumeration, reflection, `eval`, `require` or `import()`. All add a false Exact.

**Mechanism (static).** Refusals reach an identity `(file, local)` by only three joins:
1. The global lexical name join, `lexical_refusals.contains(&identity.local)` (`src/call_graph.rs:2359` at `75a35a5e`).
2. Import-binding joins via `table[module][member]`, or all of `table[module]` for a namespace binding.
3. Refused forward exports.

Three gaps follow:
- **(a) No cascade from a re-exported namespace to its classes.** `export * as M from './m'` produces the identity `(m', "*namespace*")` (`src/js_import_qualifiers.rs:157-160`). Revoking it does not revoke the class identities that M's module exports. A consumer that refuses `M` therefore cuts only the namespace identity. The spelled member (`default`, or an import-renamed forward name `D`) never equals `identity.local`, so join 1 misses as well. Renamed *exports* (`export {K as C}`) are safe only because they refuse `K` globally. `default` and import-renames (`import {C as D}; export {D}`) leave the exported name different from the local name without any refusal.
- **(b) Forwarded namespace bindings are not refused and create no identity.** In `fwd`, `import * as N from './m'; export { N }` / `export default N` pass the "own export" arm (`src/ast/js_import_qualifiers.rs:487-492`), so `N` is not refused in `fwd`. No `QualifierExport::Namespace` is created for it either, so a third file's refusal of `N` joins to nothing.
- **(c) A namespace refusal is resolved with the admitting project, not the writer's.** For a namespace binding, `proven` is never populated: `js_ts_qualifier_modules` records only `MemberImport`. The fallbacks are `qualifier_hop(<admitting project>, writer, spec)` and the relative resolver. When neither resolves the spec, `if module.is_none() && member.is_none() { continue; }` (`src/call_graph.rs:2405-2408`) silently skips the refusal. Prism's own P1 *does* resolve the writer's spec under the writer's project: the named-import control below shows `js_ts_path_modules[("w/other.*","#m")] = ("m.*","w/tsconfig.json")`.

**Evidence (REPRODUCED, both grammars).** `m` is one of:
- `export default class Api { static sm() { return 0; } }`, with `app`: `import X from './m'; export function run() { X.sm(); }`
- `export class C { static sm() { return 0; } }`, with `app`: `import { C as X } from './m'; export function run() { X.sm(); }`

| Case | Extra files | Result |
|---|---|---|
| 01 | `nsb`: `export * as M from './m';` · `other`: `import { M } from './nsb'; M.default.sm = () => 1;` | REPRODUCED, RUNTIME-CONFIRMED |
| 02 | same `nsb` · `other`: `import * as B from './nsb'; B.M.default.sm = () => 1;` | REPRODUCED |
| 08 | same `nsb` · `other`: `import { M } from './nsb'; const A = M.default; A.sm = () => 1;` (static alias) | REPRODUCED, RUNTIME-CONFIRMED |
| 22 | same `nsb` · `other`: `const { default: A } = M; A.sm = () => 1;` | REPRODUCED |
| 10 | `nsb` + `fwd`: `export { M } from './nsb';` · `other`: `import { M } from './fwd'; M.default.sm = () => 1;` | REPRODUCED |
| 11 | m: `class K {…} export default K;` · `export * as M` consumer writes `M.default.sm` | REPRODUCED |
| 30 | `nsb` (self-import): `export * as M from './m'; import { M as Z } from './nsb'; Z.default.sm = () => 1;` | REPRODUCED |
| 15 | named C · `fwd`: `import { C as D } from './m'; export { D };` · `nsb`: `export * as M from './fwd';` · `other`: `import { M } from './nsb'; M.D.sm = () => 1;` | REPRODUCED, RUNTIME-CONFIRMED. **No `default` involved.** |
| 20 | as 15, but nested: `nsa`: `export * as Q from './fwd'`, `nsb`: `export * as M from './nsa'` · `M.Q.D.sm = …` | REPRODUCED |
| 23 | as 15 · `other`: `import * as B from './nsb'; B.M.D.sm = () => 1;` | REPRODUCED |
| 18 | `export * as M` consumer: `Object.assign(M.default.valueOf(), { sm: () => 1 })` (F1-class non-owned call through the namespace) | REPRODUCED |
| 04 | `fwd`: `import * as N from './m'; export { N };` · `other`: `import { N } from './fwd'; N.default.sm = () => 1;` (gap b) | REPRODUCED, RUNTIME-CONFIRMED |
| 05 | `fwd`: `import * as N from './m'; export default N;` · `other`: `import N from './fwd'; N.default.sm = () => 1;` (gap b) | REPRODUCED, RUNTIME-CONFIRMED |
| 17 | named C · `fwd` import-rename forward · `fwd2`: `import * as N from './fwd'; export { N };` · `other`: `N.D.sm = …` (gap b) | REPRODUCED, RUNTIME-CONFIRMED |
| 12 | `w/tsconfig.json` `paths: {"#m": ["../m"]}` · `w/other`: `import * as ns from '#m'; ns.default.sm = () => 1;` (gap c) | REPRODUCED (Prism's P1 resolves the writer's `#m` to `m`) |

**Controls that correctly keep base, in both grammars:**
- 06: `export * as M` + `M.C.sm = …`, where the spelled name equals the local name.
- 07: `import * as ns from './m'; ns.default.sm = …`.
- 03 and 09: a renamed export, which refuses the local name globally.
- 13: named `import { default as Y } from '#m'` in the sub-project, which is P1-proven.
- 14: a root-alias namespace import.
- 16: `import * as F from './fwd'; F.D.sm = …`.
- **19: `import * as ns from './m'; Object.values(ns).forEach(c => { c.sm = … })`**.

**Disposition defect.** Control 19 shows that enumerating a *directly imported* namespace is already refused in model, as "reflective use of a statically bound qualifier". Yet `repair_r2_out_of_model_namespace_enumeration` (`tests/integration/js_import_qualifiers_test.rs:1141`) pins Exact for the identical shape when the namespace arrives through `export * as M`. The same static construct gets opposite dispositions depending on the hop. The F4 row should cover only what is genuinely not a static binding, and nothing is: `M` is a static named import.

**Fix (recommended), bounded, three edits in the existing join.** The namespace-object sources in ESM static syntax are enumerable:
- `import * as N`, which is already joined;
- `export * as N from`;
- forwarding of a namespace binding (`export { N }`, `export default N`; `export { N as P }` already refuses N);
- TS `import N = require` and `import()`, which are F3 and out of model.

The edits:
1. **Cascade.** When `written` gains `(f, "*namespace*")`, also revoke every identity in every `table[f]`, which covers both local and forwarded identities. Iterate to a fixpoint for nested namespaces, bounded by `MAX_REEXPORT_DEPTH`. Closes 01/02/08/10/11/15/18/20/22/23/30.
2. **Forwarded namespace bindings.** Refuse the own-export / default-export arm when the exported identifier's binding is a `namespace_import`, so that `N` is refused in `fwd` and the existing namespace-binding join revokes `table[m]`. Closes 04/05/17.
   - Alternative: emit `QualifierExport::Namespace(spec)` for forwarded namespace imports and rely on edit 1. This is more precise but adds surface.
3. **Writer-project resolution.** For refusal joins, resolve each binding first with `qualifier_resolver.project(writer)` and `qualifier_hop(writer_project, …)`, or record namespace bindings in `js_ts_qualifier_modules` as refusal-only proofs. Only then fall back to admitting projects and the relative resolver. Keep the "skip" only for specs that Prism models under no project (bare packages), and document that boundary. Closes 12.
   - Alternative: treat a refused namespace whose spec is unresolved under every project as revoking all identities. Simpler and stricter; costs yield only where such writers exist.

Then flip `repair_r2_out_of_model_namespace_enumeration` to a keep-base regression, and narrow the SPEC §2a F4 row to "no new runtime cut". Add a both-grammar regression population with cases 01, 04, 08, 12, 15, 17, 20, a control-19 parity case, and a positive control in which `export * as M` exists but no consumer refuses `M`. Add mutants for edit 1 (drop the cascade), edit 2 (re-allow the namespace own-export) and edit 3 (drop writer-project resolution).

**Yield (static grep, public X).** X has no `export * as`, no namespace import of the survivor modules (`helpers/api`, `helpers/ui`, `EditorLocalStorage`), and no forwarders of `API`/`UI`/`Keyboard`/`EditorLocalStorage`. All 132 changed rows are `named_class` (summary.json). I expect +132 to hold, but the controller must re-measure.

**Self-critique.**
- This would not apply if the owner deems `export * as` / namespace forwarding to be a "runtime" channel. Neither the CLAUDE.md clause nor the §2a table says so: §2a puts "named/star forwarding" and "direct namespace binding" in model.
- Case 12 depends on the writer's project being modeled. If the owner scopes visibility to the admitting project only, 12 becomes a disclosure item. But the existing `outside/` named-import control (`qualifier_whitelist_namespace_paths_reexports_and_this_keep_base`) shows that the design intends cross-project writers to be visible.
- The cascade over-refuses namespace consumers that only read (`M.default.sm()` refuses `M`). That matches existing direct-namespace behavior (`ns.C.sm()` already revokes `table[m]`).

### W2. WRONG · IMMATERIAL: the committed mutant registry is not the measured, manifest-bound registry

**Evidence (static, reproduced by hash).**
- `git show origin/proto/s2-import-qualifiers:mutants/lane-s2-import-qualifiers.json` hashes to `2d3de319…` and contains 14 mutations (S2-01..S2-14). The plan branch has no registry.
- BUILD-MANIFEST.json:31 and OWNED-FILES.json:47 bind `7f1d4e2c…`.
- The 22-entry registry that SPEC §5 and VERIFICATION describe (S2-15/16/17/18/26/27/29/30 added) exists only in `/Users/wesleyjinks/prism-evidence/s2/repair-r2/R2-docs.patch`, an evidence path.
- Incorrect result: running IMPLEMENTOR step 4's mutgate on the committed branch silently tests 14 mutants and omits every F1/F2/F5/F6/F8 guard.

**Fix.** Commit the 22-entry registry with the product commit, or with `75a35a5e`'s successor, and re-verify the manifest's 770 hashes against the committed tree.
- Alternative: rebind the manifest to `2d3de319` and record the eight F-rule mutants as unregistered. This is worse, because it drops the kill evidence from custody.

**Self-critique.** Immaterial to edge correctness: the source is identical and the advisory kills were observed on the patched tree. It matters only for dispatch and custody.

## SMELLs

### S1. SMELL · MATERIAL: the measurement certifies binding correctness, not in-model write closure

**Evidence.**
- The complete population is admissible. In every public stream I checked, base and head have unique, equal site keys: X 19,219/19,219, installed X the same, R 953, T 61,712. All 132 changed rows pass the full predicate (module, owner, unique declaration, file/name/span), and lost = unproven = 0.
- But `compare-head.py`'s `CORRECT_STATIC_BINDING` cannot see an in-model static writer. Every W1 fixture would pass it. The zero-false-Exact claim for the write-closure half therefore remains inductive (reviewer probes), as round 1 noted.

**Fix.** Extend `census.cjs` per changed row. Run TS LanguageService `findReferences` on the resolved member's property symbol and on the class/object symbol. Fail the row on any write reference (assignment, update, delete, `Object.defineProperty` argument) or any value escape of the class symbol outside the whitelist. This is static, uses the already pinned TS 5.9.3, and would have flagged W1's shapes on fixtures.
- Alternative: a Prism-side self-check that re-derives refusals from the native reference set on public corpora only.

**Self-critique.** If the owner accepts reviewer-probe closure for this slice, this is advisory. It costs only an oracle extension, with no product change.

### S2. SMELL · IMMATERIAL: IMPLEMENTOR dispatches from an evidence-path patch and names a stale planning HEAD

**Evidence.** IMPLEMENTOR.md:3,5,7 say "cherry-pick c35719e1, then apply `/Users/wesleyjinks/prism-evidence/s2/repair-r2/R2-src.patch`", and name planning HEAD `b8f5b2f3`. But `75a35a5e` is already committed on `origin/proto/s2-import-qualifiers`, and the plan HEAD is `ecdb6b0f`. I verified that `75a35a5e` = c357 + R2-src.patch byte-for-byte. Applying the patch on top of the proto branch tip would double-apply or fail, and the instruction offers two routes ("or continues the prototype branch").

**Fix.** Name a single source: `origin/proto/s2-import-qualifiers` @ `75a35a5e` (equivalently, cherry-pick `c35719e1..75a35a5e`) plus the registry from W2. Keep `R2-src.patch` only as an equivalence receipt. Update the planning HEAD to `ecdb6b0f`. Everything else (rebuild, rebind, `--since 4e592daa`, immutable main tools) is unambiguous.

**Self-critique.** The launch brief supplies the commits, so a careful controller can disambiguate. This is a handoff risk, not a wrong edge.

### S3. SMELL · IMMATERIAL: the indexed-universe boundary is undisclosed in §2a

**Evidence (REPRODUCED, both grammars).** A writer `other.mts` containing `import { C as Q } from './m'; Q.sm = () => 1;` leaves the head Exact (case 24). `.mts`/`.cts` are not indexed (`src/languages/mod.rs:27` maps only `js|mjs|cjs|jsx`). The same writer as `.mjs` keeps base (control 25). Prism also does not see `.vue`, `.svelte` and HTML script hosts.

**Fix.** Add a §2a row: "Writers in files outside the indexed universe (`.mts`/`.cts`, non-JS hosts, excluded directories): out of model, disclosed (as prism models it)". Optionally, refuse S2 admission project-wide when a project's include set contains `.mts`/`.cts` files.

**Self-critique.** CLAUDE.md's "as prism models it" arguably already excludes these files. In X, only `vite`/`vitest` `.mts` configs exist, and they do not touch the survivors.

### S4. SMELL · IMMATERIAL: `compare-head.py` keys rows by dict, so duplicate keys would collapse silently

**Evidence.** `compare-head.py:20` builds `{key(r): r}`. Duplicates would not trip `b.keys() == h.keys()`. I checked the retained streams, and all keys are unique, so this has no effect on the measured result.

**Fix.** Assert `len(rows) == len(set(keys))` per stream.

### S5. SMELL · IMMATERIAL (inherited, disclosed): the S2-02 position-proof mutant still survives

No new evidence. It remains disclosed without an equivalence claim. Nothing in W1 depends on it.

## Round-1 disposition check

| Round-1 finding | R2 disposition | Assessment |
|---|---|---|
| Opus W1 / non-owned calls | F1 in model | **Fixed.** Test RED on c357 (reproduced), GREEN on head. Case 18 shows the same class through a namespace hop; that is W1-here, not F1. |
| Opus W2 / Sol W2 construction | F2 in model | **Fixed.** RED on c357. Extra probe 27 (`new this()` → `.constructor` write) keeps base. |
| Opus W3 dynamic `import()`, `require`, TS `import = require`, `.default` | F3 out of model | **Correct** for the dynamic/require parts (CLAUDE.md names `require`/`import()`). The *static* half of W3's `.default` join fix (match exported names, not just locals) was dropped. W1 shows that `.default` through static namespace bindings is in model. |
| Opus W4 re-exported namespace does not cascade | F4 out of model (enumeration) | **Misclassified.** The mechanism (no cascade from `*namespace*` to the module's identities) is static. W1 reproduces it with literal member paths and aliases, and control 19 shows enumeration of a static namespace is refused in model elsewhere. |
| Opus W5 / Sol W5 `eval` | F7 out of model | **Correct** (CLAUDE.md names `eval`). |
| Opus S2 parse-error files | F8 in model | **Fixed.** RED on c357. |
| Sol W1 dynamic namespace | F3 out of model | **Correct.** |
| Sol W3 `new.target` / `super` | F5 in model | **Fixed.** RED on c357. Extra probe 26 (static-block `this.sm =`) keeps base. |
| Sol W4 nested `this` | F6 in model | **Fixed.** RED on c357. |
| Opus S4 positive test keyed by first row | — | **Fixed.** `row(&g, "shadow")` keys by caller. |
| Opus S5 / Sol S2 IMPLEMENTOR stale | partly | `--since 4e592daa` is fixed. A new staleness remains (S2 above). |

## Convergence classification

- **The material WRONG (W1) is a closed instance of an existing family**: round-1 Opus W4, namespace-object → class-identity join, plus the static half of round-1 W3's exported-name join. It is not a new family outside the round-1 families and the §2a table. §2a already declares these channels in model, and the implementation's join misses them.
- The defect population is enumerable from ESM static syntax:
  - `export * as` cascade;
  - forwarded namespace bindings;
  - writer-project resolution of namespace refusals.

  Each has a bounded fix in `apply_js_paths` / `js_ts_qualifier_use_allowed`, plus one test flip.
- Round trend: 5+5 material WRONGs in R1, 1 material family in R2. It is smaller and does not surface a new kind. It does repeat a prior finding, because that finding's disposition was wrong. I read this as converging. I recommend a targeted fix with controller-run regression/RED evidence and a re-measure, not another open-ended review round.

## Not checked

- Private F; Tier-A matrix/quick/full; full nextest/clippy/fmt on the prototype. I ran only the scratch probe, and the R2 test file against c357 and head.
- I did not rerun mutgate. Mutant kill claims are supplied receipts.
- Runtime confirmation for cases 02/10/11/12/18/20/22/23/30. They share 01/15's runtime semantics (ESM namespace member reads), but I did not execute them. Case 12 would need a paths-aware loader.
- Yield after the W1 fixes (grep-based expectation only).
- An exhaustive tree-sitter grammar census. TS-only `import A = M.default` (an `import_alias` through a re-exported namespace) is expected to behave like W1 but was not run.
- Outside S2's scope, observed incidentally: the landed base already binds an object-literal qualifier through `{ __proto__: B, sm(){…} }` with an inherited `this.sm =` writer (case 28, base populated and preserved byte-for-byte by the base fence). That is main's existing object rung, not an S2 edge. I did not investigate it further.
- CONTROLLER-s2.sh execution (read only); R and T corpora sources.
