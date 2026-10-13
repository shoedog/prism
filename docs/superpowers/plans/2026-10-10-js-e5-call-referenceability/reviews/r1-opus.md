# E5 "call referenceability" — spec and prototype review, round 1 (Opus-5.5)

Reviewed: worktree `/Users/wesleyjinks/code/prism-e5-plan` at `ce6de397` against main `bab0dfbd`; packet `docs/superpowers/plans/2026-10-10-js-e5-call-referenceability/`; public evidence `~/prism-evidence/js-e5/step1/`.

**Verdict: FIX.** The union rule and the drop behaviour stand. Four findings need folding before the pull request: three narrow predicate changes and one test repair. None needs a redesign. The predicate changes need one paired re-capture to confirm the numbers predicted below; no further review round is needed beyond confirming the fold.

How I checked: read the product diff and the surrounding ladder; ran the two frozen binaries on 48 synthetic fixtures under `review-r1/opus-scratch/fx/`; ran the packet's pinned TypeScript 5.9.3 checker on the same synthetic fixtures (`opus-scratch/tools/tscheck.cjs`); aggregated the 890 public adjudication rows. I did not run cargo.

Reproduce any fixture result below with:
`~/prism-evidence/js-e5/review-r1/opus-scratch/probe.sh ~/prism-evidence/js-e5/review-r1/opus-scratch/fx/<id> caller`

---

## Findings

### F1 — WRONG, MATERIAL: TypeScript import-equals is not seen as import evidence, so arm (b) drops a correct binding

**Input and result (run).**

| Fixture | Caller (module) | Target | main | head | Pinned checker |
|---|---|---|---|---|---|
| `t04` | `import f = require("./callee"); export function caller(x){ return f(x); }` | `callee.ts`: `function f(x){…}  export = f;` | Exact free_single `callee.ts:f@1` | dropped, LocalBindingUnproven | `callee.ts:1 FunctionDeclaration` |
| `t04b` | same, `.ts` | `callee.js`: `module.exports = function f(x){…};` | Exact free_single | dropped | `callee.js:1` (the export assignment) |
| `t23` | `export import f = require("./callee")` | as `t04` | Exact free_single | dropped | `callee.ts:1 FunctionDeclaration` |
| `t24` | `import { N } from "./ns"; import f = N.f;` | `ns.ts`: `export namespace N { export function f … }` | Exact free_single | dropped | `ns.ts:1 FunctionDeclaration` |

`t04b` isolates arm (b): its target is a self-named function expression in a file with no root import/export, which is outside arm (a)'s negative set.

**Cause.** `ParsedFile::collect_js_imports` (`src/ast.rs:1703`) reads `import_statement` only through its `source` field. `import f = require(...)` carries the string inside `import_require_clause`, so nothing is collected. The spec's rule says "no syntactic import or require evidence for that name"; this is such evidence, and it is the TypeScript spelling of the scalar `const f = require(...)` that the rule exempts on purpose (the packet priced ten CORRECT scalar-require targets on SecBench).

**Why the corpora did not show it.** No measured row has this shape as a CORRECT target. The only rows whose binder state is `Unproven("import")` are two WRONG rows on T (`export import format = ts.getPathFromPathComponents`, `harness/vpathUtil.ts:109`). Zero CORRECT lost is true, but the corpora do not exercise the form.

**Recommended fix.** In `module_binding_evidence` (`src/call_graph_js_referenceability.rs`), add names bound by root import-equals: `import_require_clause`, and `import_alias` when the right-most name of the entity equals the local name. The binder already walks both node kinds (`src/ast/js_binding_decls.rs:181-207`). Add a kept-Exact control per grammar and one mutant.

**Options.**
- (A) `import_require_clause` only. Smallest change, exact twin of the scalar-require exemption. Leaves `t24` dropped.
- (B) A plus same-terminal-name `import_alias` (recommended). Keeps `t24`; still refuses the two T `format` rows, so predicted yield change is zero.
- (C) Exempt whenever `site.local_binding` is `Unproven("import")`. Simplest, reuses the binder, but gives up the two T WRONG rows and every renamed alias.

**Related, lower weight (SMELL).** `t29`: `const { f } = await import("./callee.js")` in an outer function with the call in a nested function is dropped by the head, while its `require` twin `t29r` is kept. The checker resolves `f` to the local binding element, so the oracle would not call it CORRECT. Either add dynamic-import destructuring to the evidence for parity or state the asymmetry in the spec.

**When this would not apply.** If the owner reads import-equals as out of scope for the JS/TS lane. The spec would then have to say that a syntactic import form is not counted as evidence.

---

### F2 — WRONG, MATERIAL: a module caller's call to a script-file global is dropped, and the spec does not say so

**Input and result (run).**

| Fixture | Caller (module) | Target (script file, no root import/export) | main | head | Pinned checker |
|---|---|---|---|---|---|
| `t02` | `caller.ts`: `export function caller(x){ return f(x); }` | `globals.ts`: `function f(x){…}` | Exact free_single | dropped | `globals.ts:1 FunctionDeclaration` |
| `t01` | same in `.js` | `callee.js`: `function f(x){…}` | Exact | dropped | `callee.js:1 FunctionDeclaration` |
| `t05` | `function caller…; export {};` | same | Exact | dropped | same |
| `t08` | `function caller…; export = caller;` (TS) | `callee.ts` script | Exact | dropped | `callee.ts:1` |
| `t27` | module `.js` | `var f = function(x){…};` | Exact | dropped | `globals.js:1 VariableDeclaration` |
| `t28` | `caller.tsx`: `return <F />` | `globals.tsx`: `function F(){…}` | Exact | dropped | `globals.tsx:1` |
| `t32`, `t33` | class-method caller; caller with an unrelated import | script `function f` | Exact | dropped | — |

**Answers to the brief's three sub-questions.**
- Is it dropped? Yes, by arm (b) alone. These targets are outside arm (a)'s negative set.
- Is it a static binding under the Exact contract? Yes for TypeScript: a file with no top-level import/export is a global script and its declarations are in scope in every module of the program. For plain JS it depends on the host (true for classic browser scripts, false for Node CommonJS), but main already grades script-to-script the same way and the spec keeps that. There is no reading under which the script-to-script call is a static binding and the module-to-script call to the same declaration is not.
- Does the spec say so? No. "Deliberately uncovered" lists "script-global declarations called from scripts" as kept; the loss from module callers is only implied. The unit test pins it as intended (`e5_module_caller_refuses_unbound_and_alias_singletons`, cell `export {}; function caller(x) { return f(x); }` against `function f(x) { return x; }`).

**The extension is unpriced.** In the 890 public rows, the removals that arm (b) adds over arm (a) for unbound names are 0 WRONG and 6 INADMISSIBLE (five unbound JSX sites with module-file targets, one JS call resolved to a C function). No removed row has an unbound module caller and a script-file declaration target. On F every removal has a local declaration. So this part of arm (b) has no measured yield on any corpus and a demonstrated correct-loss class.

**Recommended fix.** Arm (b) does not fire when all three hold: `site.local_binding` is `Unproven("unbound")`; the target file is JS/TS/TSX; the target is not in `js_ts_free_single_refusals`. For names with no own-file binding, the target-side test of arm (a) then decides, as it does for script callers. Predicted measured change: none on X, T, SB or F (the five JSX rows have module targets and stay refused; the C-target row stays refused because of the language clause). Flip the unit-test cell above from the refuse list to a kept-Exact control and add one mutant for the new conjunct.

**Options.**
- (A) The exemption above (recommended): zero measured yield cost, removes the correct-loss class.
- (B) Keep the rule and state the loss in the spec and the `CLAUDE.md` Exact paragraph as an accepted cost with zero measured population. Costs nothing now; leaves a known false refusal for TypeScript global scripts and browser-script repositories.
- (C) Exempt only when the target file also has no CommonJS markers (`require`, `module.exports`). Closer to Node semantics, but it needs a new per-file fact and has no measured population to tune against.

**When this would not apply.** If the owner's reading of "as prism models it" is that a module's free names must be imported, full stop. Then option B is the fold, and the spec must say it.

---

### F3 — WRONG, MATERIAL: for indirect records arm (b) tests a file where the name was never written (brief question 2)

**Input and result (run).**
- `t18`: `Menu.jsx` has `import { MenuItem } from "./MenuItem"; … Menu.Item = MenuItem;`. `App.jsx` has `import { Menu } from "./Menu"; export function caller(){ return <Menu.Item />; }`. main: Exact free_single `MenuItem.jsx:MenuItem@1`. head: dropped.
- `t19`: the same with an unrelated `import { MenuItem } from "./MenuItem"` added to `App.jsx`. main and head both resolve it.
- `t20`: ordinary call, `api.run = helper;` then `api.run(x)` in another module. main: Exact free_single `helper`. head: dropped.

The head's answer for `<Menu.Item />` flips on an import in the calling file that has nothing to do with the site. The rule's premise is "this file writes the name and does not bind it"; for an indirect record the name was written in the assigning file, which does import it.

**What the measured rows are.**

| Arm (b), origin IndirectResolution | WRONG | UNDECIDED | INADMISSIBLE |
|---|---|---|---|
| X, JSX | 8 | 23 | 35 |
| X, qualified | 0 | 0 | 1 |
| SB, qualified | 1 | 0 | 64 |
| T, F | 0 | 0 | 0 |

- The 23 UNDECIDED are single static assignments (`DropdownMenu.Item = DropdownMenuItem`, `DropdownMenu.tsx:60`; `DropdownMenuItem.Badge`, `DropdownMenuItem.tsx:117`; `DropdownMenuSub.Trigger/Content`, `DropdownMenuSub.tsx:21-22`). By the source they are right.
- The 8 WRONG are the indirect producer choosing by property name alone (`<DropdownMenu.Trigger>` resolved to `DropdownMenuSubTrigger`). The writing file imports that name too, so a referenceability test in the right file would not remove them either.
- Correction to the controller note: of X's 13 JSX WRONG, 8 are indirect. The other 5 are Source-origin JSX in `examples/with-script-in-browser/components/ExampleApp.tsx` with binder state `MayCall alias`. Those are a sound use of arm (b) and should stay refused.

**Comparison.**

| Option | Yield kept | Exposure | Cost |
|---|---|---|---|
| Keep as is | all | removes 23 by-source-correct UNDECIDED and 100 INADMISSIBLE on a premise that does not hold; outcome depends on unrelated imports | none |
| Arm (b) only for `site.origin == Source` | gives up 8 W on X and 1 W on SB; F and T unchanged | none of the above | one conjunct, one control, one mutant |
| Test the file where the name is written | about 0 of the 9 W (the writer imports the name) | none | writer file on the indirect record, cache change |

**Recommended fix for this slice.** Add `site.origin == CallSiteOrigin::Source` to arm (b). Cut on origin, not on the packet's "ordinary" class: Source-origin JSX (5 W on X) and Source-origin `super(...)` should stay refused.

**Qualified fallback rows.** They are two different things.
- 66 are indirect records; they follow the cut above.
- 23 are Source-origin `super(...)` constructor calls in one SecBench root, resolved on main to a function named `super`. Refusing them is right: `super` is written in the calling file and cannot be an import. They stay refused under the origin cut. I ran 14 physical member-call shapes (`t21`, `t22`: `this.f()`, `a?.f()`, `(a.f)()`, `a["f"]()`, `super.f()` and others); every one keeps a product qualifier or an unresolvable callee text and never reaches the free-function rung, so `super(...)` is the whole class.

**Follow-up, not this slice.** (1) The indirect producer should key on the assigned object and the property, not the property name alone; that is where the 9 WRONG come from. (2) `super` should never be offered to the free-function rungs; in script callers it still resolves today.

**Predicted totals after F1 to F3** (from the public rows; to be confirmed by one paired capture, since the binary changes):

| Corpus | WRONG removed | CORRECT lost | UNDECIDED | INADMISSIBLE |
|---|---|---|---|---|
| X | 19 | 0 | 11 | 1 |
| T | 201 | 0 | 0 | 2 |
| SB | 170 | 0 | 0 | 354 |
| F | 109 | 0 | 4 | 0 |

**When this would not apply.** If the owner decides static-property indirect records are outside the Exact contract altogether. Then they should be refused at the producer for every caller, not by a test on the calling file's imports.

---

### F4 — WRONG, MATERIAL: E5 makes the six D12 tests pass without exercising D12, so mutant PD-29 would survive

**State and result (verified by reading, plus binary runs of the fixtures; cargo not run).**
- `mutants/js-param-defs.json`, `PD-29-cross-file-free-single-predicate-off`, sets `cross_file_free_single_exact = false` in `src/cpg/build.rs:1617`. Its selectors are `r2_w2m_{js,ts,tsx}` and `r2_w2d_{js,ts,tsx}`.
- The prototype adds an early return to their shared helper (`src/cpg/js_param_defs_tests.rs:619-633`): when the call resolves to nothing, assert the E5 drop and `return false`.
- Fixtures `w2m1-3` and `w2d1-3` are those six inputs. main resolves each Exact free_single; the head drops each. So all six tests now return at line 632, before the DataFlow walk that D12 affects.
- With PD-29 applied the resolution is still dropped by E5, the tests still pass, and the gate's rule ("at least one selector must fail") reports SURVIVED.

The packet's mutation run was scoped (`--since d01fd65f --scope fn`, 4 selected), so PD-29 was not re-run. The spec's "D12 remains unchanged" is true of the code, but D12 is no longer pinned by any registered test. I checked every registered mutant in both registries for selectors touched by this change; PD-29 is the only one.

**Recommended fix.** Give PD-29 fixtures that survive E5: script caller, script top-level declaration target. Fixture `d12s` (`const entry = input =>\n escape(input);` against `function escape(s) {\n sink(s);\n}`) stays Exact free_single on the head, and `nav dfg-stats --edges` shows the argument edge present and the new formal's outgoing edge absent, which is D12 acting. Keep the six E5-refused inputs as an E5 consumer test under a name that says so. Run the full authoritative gate before merge.

**Options.**
- (A) Add script-global variants and point PD-29 at them (recommended).
- (B) Retire D12 if the owner judges its remaining population (script-to-script globals, and names with require evidence) too small. That is an owner decision outside this slice.

**When this would not apply.** If another test outside the registry's selectors kills PD-29. The registry counts only listed selectors, so the gate would still report it.

---

### F5 — SMELL, IMMATERIAL: one of the two guard sites is dead, and two conjuncts are always true

- `graph_target_resolution` (`src/resolution.rs:1404-1408`) is reached only through `rust_scope_graph_resolution`, which is called only under `Language::from_path(caller) == Rust` (`src/resolution.rs:2349-2408`). The predicate requires a JS/TS caller file, so this guard can never fire. The spec's "at both existing FreeSingle arms" and "graph-target resolution declines the refused singleton" describe a path that does not exist.
- At the live site the code is inside `match site.qualifier.as_deref() { … None => … }` and after the same-file return at R4, so `site.qualifier.is_none()` and `target.file != site.caller.file` are always true there.

**Recommended fix.** Remove the dead guard and the spec sentence; drop or comment the two conjuncts so nobody registers unkillable mutants for them.
**When this would not apply.** If JS/TS scope-graph resolution is about to ship and the guard is deliberate forward cover; then say so in a comment.

### F6 — SMELL, MATERIAL (test adequacy; by reading, not run): four mutants pin arm (b) only

Mutations I expect to survive the two E5 test files (other suites may kill them; I could not run the gate):
- Remove the `unbound` conjunct from arm (a). No test asserts that a script caller with an alias and a negative-set target keeps its edge. Fixtures `m1` and `m3` show the head keeps it. The one candidate cell, in `e5_preserves_script_binding_imports_aliases_and_same_file_controls`, says "existing alias/may-call … fallbacks are preserved" but asserts only the binder state, not the resolution.
- Remove `!site.jsx_element` from arm (a). No script-caller JSX control exists (`m2` shows the head keeps it).
- Delete `js_ts_free_single_refusals.extend(...)` in `merge`. The lifecycle test changes the target from module to script only; the script-to-module direction through `merge` is untested for arm (a). The product is right (see question 4); only the pin is missing.

Three of the "negative controls" (import member, same-file LocalDef, namespace-qualified) return on earlier rungs and cannot fail from any broadening of the predicate. They pin ladder order, which is worth having, but they are not breadth controls.

**Recommended fix.** Register arm (a) off, the arm (a) `unbound` and JSX conjuncts with controls `m1`/`m2`, the Source-origin conjunct from F3 with `t18`, the F1 and F2 conjuncts, and one `merge` direction test.
**When this would not apply.** If the full authoritative gate, run with these added, shows them killed by existing integration tests.

### F7 — SMELL, IMMATERIAL: SecBench counts one package twice

`SB__prototype-pollution__total.js_3.4.6` and `SB__command-injection__total.js_3.4.6` contribute identical rows (97 each: 36 WRONG, 61 INADMISSIBLE; compared by site, target and verdict). Distinct-source SecBench yield is at most 135 WRONG, not 171. All 589 SecBench removals come from 14 of 579 roots. The decision does not change; the spec should state both.

### F8 — SMELL, IMMATERIAL: the b5 table is measured against the step-0 head, not main

Against main, `b5_nested_declarators_keep_base` changes ten expectation values (twenty across grammars), not six. Four come from arm (a) (bare and `if` L6, the second same-line L2 call, module-scope `const` L5); the spec's "the second L2 call was already drop" is true only relative to the step-0 prototype (checked at commit `65086ee6`). The pull-request description should list changes against main. The test name no longer matches what it asserts.

---

## Answers to the six questions

1. **Can the rule drop a static binding?** Yes, in two classes: TypeScript import-equals (F1) and module callers of script-file globals (F2). Other items checked:
   - Import forms seen: default (`t10`), named, renamed (`t11`), namespace (`t12`, qualified), scalar, destructured, renamed and nested `require` (unit tests and `t29r`). Not seen: `import x = require()`, `export import x = require()`, `import x = N.y` (F1); dynamic-import destructuring in the closure shape (`t29`). Top-level and same-function dynamic-import or `require` bindings are already dropped on main by the lexical guard (`t09`, `t14`, `t16`, `t31`), so the head changes nothing there. Re-exports bind nothing locally.
   - Module detection: `.cjs` (`t06`) and `.mjs` without import/export statements (`t07`) are treated as scripts, so the rule under-applies and nothing is lost. `export {}` only (`t05`) and TypeScript `export =` (`t08`) are modules, which matches the language. Root-level `namespace` and `declare module` blocks are not root export statements, so namespace-style TypeScript scripts are untouched. `.d.ts` files hold no bodies or calls. `.mts`/`.cts` are not parsed.
   - Same-file and hoisted targets (`t13`) and earlier rungs are unaffected: the only live guard is the last match of R5, after every earlier return.
2. **Indirect records and member JSX.** A defect in the rule as specified (F3). Ship the Source-origin cut in this slice; the producer fix is a follow-up.
3. **Tests.**
   - Refusal paths: the packet's `main-red-final.log` shows 7 of 10 E5 tests red on main and 3 preservation tests green; I read it and did not re-run.
   - Repaired fixtures: all three still test their purpose. `c41` still separates `Unproven("with")` from the three R5-narrowed reasons (and `with` is a syntax error in module code, so the script form is the more honest fixture). `td_duplicate…` and `rep_unbound…` still show the call binding once the duplicate head or the unrelated parameter is out of scope. All three now rely on script-to-script globals staying Exact.
   - b5: each of the six arm (b) changes is a consequence of the owner's alias decision, and each is also a correct refusal in its own right (the name is bound to a value destructured from `o`, not to `lib`'s `f`). The count against main is ten (F8).
   - `r2_w2m`/`r2_w2d` no longer test D12 (F4).
   - Four mutants are not enough (F6).
4. **Lifecycle.** Sound. On the head binary with one cache directory (`fx/life1`): cold, full hit, and partial hits for script to module, module with `require` evidence, back to script, target to module and back, an unrelated file edit, and a rename all gave the right answer. A cache written by the base binary is a miss for the head. By reading: every `CallGraph` literal fills both facts; `remove_files` and `merge` replace them; the MCP incremental refresh goes through the same `build_incremental` path with a cloned call graph (`src/navigation/mod.rs:148-158`). The navigation edge cache needs no bump because its fingerprint includes the build identity.
5. **Measurement.** The tables match the 890 rows. No UNDECIDED or INADMISSIBLE row is counted as yield. Three things are disclosed too weakly: zero CORRECT lost says nothing about F1 and F2 because those shapes do not occur in any corpus; SecBench's zero rests on 171 adjudicable rows of 589, with 210 of the 418 INADMISSIBLE from one root (react-native, 209 of them for parse diagnostics); and one package is counted twice (F7). Most INADMISSIBLE removals under arm (a) are structurally wrong whatever the oracle says (bare calls resolved to object-literal methods or member assignments); after the F3 cut the genuinely unknown arm (b) exposure on SecBench is about five JSX sites.
6. **Readiness.** Beyond F1 to F6: update the cache-version comment (it describes arm (a) only); add an E5 sentence to the R5 description in `CLAUDE.md`; put both fact builders and one shared "is module" helper in `call_graph_js_referenceability.rs` (the root import/export test is written twice today); run the full authoritative mutation gate; re-freeze binaries and re-capture once after the predicate changes.

## Checked and found sound
- Both sites that mint `free_single` are guarded; no third path exists.
- Arm (a) and arm (b) behave as specified on every fixture not listed as a finding.
- Lifecycle across cold, full-hit, partial-hit and cross-binary cache states.
- The measured tables against the raw public rows, including the arm and invocation splits.
- The three repaired fixtures and the six b5 arm (b) changes.
- Cache version 115 and both registry pins.

## Not checked
- Cargo, the test suite and the mutation gate were not run. F4 and F6 are by reading plus binary runs of the fixtures.
- The step-0 head binary (arm (a) alone) was not run; arm attribution is by fixture design and reading.
- Private corpus F evidence was not opened; F numbers are the packet's.
- The 2,208 graph-row adjudications, the non-JS byte-identity claim, inventories, module-dependency deltas and Tier-A outputs were not re-derived.
- The census and controller scripts under `probes/` and `CONTROLLER-e5.sh` were not reviewed; oracle verdicts are taken as given.
- MCP refresh was read, not executed.
- Arm (a)'s negative-set construction was not audited beyond what F5 and F6 note.
- Files with parse errors, Flow syntax and non-registered tests that may have become vacuous were not probed.
- The other reviewer's report and scratch directory were not opened.
