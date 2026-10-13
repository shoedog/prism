# Review of PR #368, slice I1 — round 2 of 2 (confirmation)

Reviewer: Opus. Product code reviewed at `a34a1d45`; the PR head moved to `5ee9b786` (tests only) while I was reviewing, and I covered that commit too (see "Binding").

**Verdict: APPROVE.** Every round-1 finding is closed or recorded as the brief says. The fold's two product changes (the namespace early return and the tiered fallback) are correct, and I could not make either one change an answer for a specifier or module that main already finds. I found no wrong edge that is material to the goal. What remains is wording, one small test gap and two merge-process notes; none needs another review round.

The one thing to do before merge is a description edit: point 3 of "What can change" lists three kinds of edge change and there are four (F2). The fourth is already pinned by the newest commit.

## Binding

- Worktree `/Users/wesleyjinks/code/prism-js-i1`. At the start: clean, HEAD `a34a1d45`, nine commits on `origin/main` `e2701e89`, as the brief states.
- **The head moved during the review.** Uncommitted edits to two test files appeared at about 15:52 MDT, and at 15:54:17 they were committed as `5ee9b786` ("pin namespace hop pruning, tier-policy corners end to end, explicit two-candidate rows") and pushed. I did not make them: I ran no git writes and edited nothing in the worktree. The commit's trailer names the same session as the PR description.
- `git diff a34a1d45 5ee9b786 -- src CLAUDE.md mutants Cargo.toml Cargo.lock build.rs scripts` is empty. The new commit changes only `tests/integration/js_relative_edges_test.rs` (+77) and `js_relative_receiver_test.rs` (+8/−6). So the product review below holds for both heads.
- Binaries: branch `i1c-prism` reports `a34a1d454ff5`; main `base-prism` reports `40ad8390d6cb`. `git diff --stat 40ad8390 origin/main -- src/` touches only MCP files, `cpg_cache.rs`, `navigation/call_edge_cache.rs` and `navigation/mod.rs`, so it is a valid control for call resolution.
- Every difference attributed to the branch below has the main binary's output on the same fixture beside it, each run with a fresh cache directory. 183 fixtures in six batches; definitions in `r2-opus-scratch/bin/fx_*.py`, outputs in `r2-opus-scratch/out/batch*.txt`, rerun with `bin/run.py <fixtures.py> [name]`. For each batch I wrote the expected rows before running. One hypothesis was refuted: I expected any type-only `f` in a newly resolved star to drop main's edge; only a forwarded type-only import does (F1). Every other prediction held.
- TypeScript evidence: the pinned 5.9.3 package (`~/prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3`), `tsprobe/probe.cjs` (`resolveModuleName`, 17 directories, 6 mode settings, `allowJs` off and on) and `tsprobe/check.cjs` (checker binding of a call).

## Findings

### F1. WRONG, IMMATERIAL. A correct Exact edge is dropped when the newly resolved star module forwards a type-only import of the same name

Fixture `e16_import_type_forward`:

```ts
// a.ts
export function f() { return 1; }
// types.ts
export type f = number;
// b.ts
import type { f } from './types';
export { f };
// barrel.ts
export * from './a';
export * from './b.js';
// app.ts
import { f } from './barrel';
export function g() { f(); }
```

| | `f()` |
|---|---|
| main | `a.ts:f exact/import_member` |
| branch | `DROP UnknownName` |
| main, canonical `./b` | `DROP UnknownName` |

- `b.ts` exports no runtime `f` (the type-only import is erased), so `a.ts:f` is the only binding. The 5.9.3 checker binds the call to `a.ts:f` (`tsprobe/check-edges.txt`). It also reports TS2308 on the barrel, as it does for every same-name pair of stars, including the type-alias case that prism keeps.
- Mechanism: a plain `export { f }` of a type-only import is not admitted as a forward (`src/ast/js_module_forwarding.rs:16`) and stays a claim on the name that prism cannot prove; one blocked star branch blocks the barrel (`src/js_exports.rs:665`). I read the refusal and observed the block; I did not trace which fact variant carries it.
- This is the star-conflict removal the PR already pins, but here the removed edge was right. The description's parenthesis "(the language's own answer)" does not cover it; "the canonical spelling's answer" does.
- Controls: a second module whose `f` is a function, a class, a `let` or a forwarded package import drops on the branch and on the canonical twin, which is right (`e0`, `e3`, `e4`, `e8`). A second module whose `f` is a type alias, interface, `export type { f }`, `declare function` or a default export keeps `a.ts:f` on both binaries (`e1`, `e2`, `e5`, `e7`, `e12`, `e15`).
- Not part of this finding: a `const`, `enum` or `namespace` named `f` in the second module registers no conflict on either binary or on the canonical twin (`e11`, `e17`, `e18`). That is main's export model; the PR does not change it.
- IMMATERIAL: it needs a type and a function of one name reaching one barrel through two stars, with the type forwarded by a plain `export { f }`. It equals the canonical twin, which is the rule the brief applies. Zero on X, T and F.
- **Fix (recommended):** no product change; widen the sentence as in F2.
- **Alternative:** treat `export { f }` of a type-only import as no value export. Cost: it changes the export model for the canonical spelling, so it needs its own slice and measurement.
- **Would not apply** when the second module's `f` is a function, class, `let` or forwarded package value: there the drop is correct.

### F2. SMELL, IMMATERIAL. Point 3 of the compatibility statement is not true as written: there is a fourth kind of edge change

Fixture `n7_found_module_new_star_proof` (and `n7d` with `export { f } from './impl.js'`):

```ts
// lib/index.ts
export * from './impl.js';
// lib/impl.ts
export function f() { return 1; }
// lib/other.ts
export function f() { return 2; }
// app.ts
import * as N from './lib';
export function g() { N.f(); }
```

| | `N.f()` |
|---|---|
| main | `lib/impl.ts:f exact/import_qualified; lib/other.ts:f exact/import_qualified` |
| branch | `lib/impl.ts:f exact/import_qualified` |
| main, canonical `./impl` | `lib/impl.ts:f exact/import_qualified` |

- `./lib` is a module main already finds. On main the unresolved hop makes the namespace proof fail (`src/js_exports.rs:337`, `:370` return `Err`), so the legacy stem match grades every same-directory `f` Exact. On the branch the hop resolves, the proof succeeds and the decoy edge goes.
- The branch is right and equals the canonical twin. But it is "an existing Exact target removed", and the description says that happens only through a star conflict.
- This is the only route I found outside the three listed kinds. The reverse cannot happen: a proof that succeeded on main had every hop resolved, and resolved hops keep their file (claim 1).
- Status: `5ee9b786` pins exactly this (`namespace_proof_through_a_newly_resolved_named_reexport_hop_prunes_the_stem_fallback`). The description, read at 16:03 MDT, still lists three kinds.
- **Fix (recommended):** add to point 3: "and, on the namespace route, removal of a legacy stem-match Exact edge when a newly resolved re-export hop completes the positive proof (the canonical spelling's answer)". In the same edit change "(the language's own answer …)" to "(the canonical spelling's answer; when both stars export a function or class it is also the language's)", which covers F1.
- **Alternative:** define "gains" to include precision gains. Cost: the measured table reports removals in a separate row, so the two would use the word differently.
- **Would not apply** when the proven target is outside the stem pool: the proof then fails and the fallback is unchanged (`n5`).

### F3. SMELL, IMMATERIAL. Point 2 names one exception; the fail-closed forms are exceptions too

- Point 2 says a newly found module behaves as its canonical spelling "in every consumer … except the namespace case".
- Re-run at this head, same on main and branch: namespace through `.`, `..` and `./sub/` drops `ImportExternal` (`n8`, `n8b`, `n8c`); a default-import qualifier through `..` drops `ImportExternal` (`u1`); an import-forward through `..` drops `UnknownName` (`u2`). Each canonical twin resolves Exact (`u5c`–`u7c`, `u1c`, `u2c`).
- No wrong edge and no lost edge. The description lists them as follow-ups; point 2 should point at that list.
- **Fix:** "…except the namespace case above and the fail-closed forms listed under follow-ups". **Alternative:** do the follow-ups now; cost: they touch rungs that resolve on main.

### F4. SMELL, IMMATERIAL. Two statements about TypeScript are broader than the evidence

- **"`x.js.ts` resolves in Bundler, Node10 and NodeNext"** (brief; comment on `redundant_segments_follow_typescript_order_where_canonical_is_legacy`). My probe confirms Bundler, Node10 and NodeNext for a CommonJS-mode importer. For an ESM-mode importer under Node16 or NodeNext the lone `x.js.ts` is unresolved. See "Disputed point".
- **"This is TypeScript's own order"** (description, "Change"; comment at `src/call_graph.rs:6766-6769`). The first tier is prism's literal-first policy, not TypeScript's: with `x.ts` and `x.js` both present TypeScript picks `x.ts` in every mode, and the fallback picks the literal (`t4`: `./a//x.js` over `a/x.js`, `a/x.ts` gives `a/x.js`). The description's "Not modeled" paragraph says this correctly for the canonical spelling.
- **Fix:** qualify both sentences ("literal file first, as on main; then TypeScript's order"). **Alternative:** make the fallback prefer `x.ts` over a literal `x.js`. Cost: the redundant spelling would then disagree with the canonical one in the common layout where compiled output sits beside source. I would not.

### F5. SMELL, IMMATERIAL. The fold's tiering is not pinned for spelled extensions other than `.js`

- Mutant: `if has_module_extension {` to `if has_module_extension && target.ends_with(".js") {`. It survives all 35 unit tests in a scratch copy of the functions (`mut/run_mutants.py`, output `out/scratch-mutants.txt`). The corner tests added in `5ee9b786` spell `.js` only.
- Behaviour it would change: `./a//x.ts` over `a/x.ts` and `a/x.ts.ts` gives `a/x.ts` at this head (`t7`); the mutant gives `a/x.ts.ts`.
- The same run kills all 14 unit-level mutants of the new lane and 11 more of mine on the tier code. Three more survivors are narrower forms of the same gap (dropping `.ts`, `.tsx` or `.jsx` from the spelled-extension list). Two are equivalent for consumers (`None` versus `Some([])`; the redundant `!target.is_empty()`). One is a corner: widening `is_js_ts_relative_specifier` to any leading dot survives because no test indexes a dot-named file.
- **Fix:** one row, `list("./a//x.ts", APP, &["pkg/a/x.ts", "pkg/a/x.ts.ts"]) == ["pkg/a/x.ts"]`, and a lane mutant for it. **Alternative:** tier only `.js` spellings. Cost: one more place where a new spelling keeps the legacy order.

### F6. SMELL, IMMATERIAL. `found_on_main` names a branch state

- `src/resolution_js_namespace.rs:21-29` and `:86-87` say "found on main", "not found on main", "the pre-I1 module lookup". After merge, main contains this code and the name stops meaning anything.
- The check itself exists only because the canonical stem fallback is unsound. The follow-up that fixes that fallback should delete it.
- **Fix:** rename to `found_by_legacy_lookup`, reword the two comments, and update the anchor of mutant `I1-15` in the same commit. **Alternative:** leave it until the follow-up removes the branch; cost: a misleading name in the meantime.

### F7. SMELL, IMMATERIAL. The `CLAUDE.md` sentence is still slightly off

- It lists "import-member, re-export and namespace resolution". The class-identity consumers also use the list (typed-parameter and constructor-local receivers, imported props, the executable-owner proof); they produce the 97 receiver gains in T.
- "Import-forwarding keeps its own rules" understates it: a forward through `./x.js` or `./a/../x` uses the shared list and gains (`c22`, `u3`); only bare `.` and `..` forwards are refused (`u2`).
- **Suggested:** "…(import-member, re-export, namespace and class-identity lookups)… The tsconfig `paths` lane keeps its own rules, bare `.`/`..` are not accepted by import-forwarding or the qualifier rung, and a namespace import…".

### F8. SMELL, IMMATERIAL. Cache versions did not move across the fold

- 115 and 63 were introduced at the round-1 head. The fold changed namespace and tier behaviour and kept them. That is right for main, where 115 never shipped.
- A local cache written by the `i1` or `i1b` binary in the default cache directory would be accepted by the folded binary. The X and T measurements are not affected: `measure/X/i1c.stderr` and `measure/T/i1c.stderr` show a separate cache directory and a cold miss for each run. I did not open the F run's files.
- **Fix:** clear the default nav cache on any machine that ran a round-1 binary without `--cache-dir`. **Alternative:** bump to 116 and 64; cost: three registry anchors and two pin tests again. **Would not apply** to CI or a fresh machine.

## Answers to the five questions

### 1. Closure

| Round-1 finding | Result |
|---|---|
| Namespace upgrade to Exact on a decoy (sol W1, Opus W3) | **Closed.** `./a/../x.js`, `./a/../x`, `./lib//x`, `./lib/./x.js` and `./../x` over a module that lacks the member: `other/x.ts:f name_only` on both binaries (`n1`–`n1e`); type-only export the same (`n4`); member present: Exact on it alone (`n3`). Mutants `I1-15` and `I1-16` are killed in the stored run. |
| `x.ts` loses to `x.js.ts` in the redundant spelling (sol W2, second half) | **Closed.** `./a//x.js` over both gives `a/x.ts` (`t1`); TypeScript agrees in all six mode settings. |
| Lone `x.js.ts` (sol W2, first half) | Unchanged by design; see question 5. |
| `./x.d.js` selects `x.d.ts` (Opus S5) | **Closed.** No candidate for the plain or the redundant spelling (`t3`, `t3b`); `x.dd.js` still substitutes (`t3d`). |
| Star conflict removes an Exact edge (sol W3, Opus S1) | **Closed as a recorded decision.** Three spellings pinned with both controls; binaries agree (`e0`, `e13`, `e14`). See F1 for one shape where the removed edge was correct. |
| Props type with an inherited method (Opus W2) | **Closed as a recorded decision.** Pinned equal to the canonical twin (`c13`). Typed-parameter and constructor-local receivers do not have this loss: on a miss they fall through as on main (`c2`, `c4`, `c6`, `c7b`). |
| Orphaned mutation anchors (Opus W1) | **Closed.** `mutgate.py --plan-only` exits 0 over 247 mutants. The three mutants flip a constant that a pin test compares literally (`assert_eq!(…, 115)`, `(…, 63)`), so they are killed; I did not run them. |
| Survivors and untested consumers (sol S1, Opus S4) | **Closed.** The `..` and `.` arms, the `.js/` guard, `trim()` and all six index extensions have rows; receiver consumers have tests. Stored `target/mutgate/report/summary.json`: 17 selected, 17 admissible, 17 KILLED, authoritative. All 17 anchors are unique in their files. Residual gap: F5. |
| `CLAUDE.md` wording (sol S3, Opus S5) | **Closed**, with the small imprecision in F7. |
| Table units, stale adjudication line (Opus S5) | **Closed.** Site keys and targets are separate rows; `X/i1c.adjudicated.txt` reads 7 `agree`; T sums to 15,419 with 2,775 overload cases. |
| Follow-ups | **Recorded** in the description. Still fail closed at this head (F3). |

### 2. The fold's own code

- **"Found on main" is computed correctly.** `js_ts_literal_module_candidates` is main's candidate function unchanged (I compared the text), and `namespace_sibling` again calls it, so `found_on_main` equals main's `module_file.is_some()` by construction.
- **The early return cannot change a module main already found.** With a non-empty literal list the branch resolves the same first candidate. With an empty literal list and a sibling hit, the fallback's first candidate is that same file (`x.ts`, else `x.tsx`), or the fallback is empty (a `.d` stem, a `.jsx` spelling, a dotfile) and the sibling probe supplies it as on main. The proof function is main's inline code moved, so it returns the same value for the same file.
- Checked on the binaries: `.js` handled by the sibling on main keeps the legacy outcome (`n2`, `n2d`); the canonical spelling is unchanged (`n2c`) and `d5_authoritative_missing_member_and_fallback` passes; a literal `x.js` is unchanged (`n10`); `.jsx` through the sibling is unchanged and a redundant `.jsx` is again unresolved as on main (`n9b`, `n9`); a padded specifier that only the fallback finds keeps NameOnly (`n13`); a JSX element site follows the same rule (`n15`, `n15b`).
- **Tiering.** The fallback runs only when the literal list is empty, so no tier can change a canonical answer; `k1`–`k6` confirm at edge level. For a spelled `.js`, the order after the literal file matches TypeScript's Bundler and NodeNext (CommonJS-mode) answers in every case I probed, apart from declaration files, which prism never selects by design. Node10 differs in one case: with `x.js` beside `x.js.ts` it picks `x.js.ts`. The guards hold (`t14`, `t15`, `t19`, `t19b`, `t20`).
- Deliberate result: in four corners the redundant and canonical spellings of one specifier now bind different files, because the canonical list keeps its legacy order (`t1`/`t1c`, `t5`/`t5c`, `t6`/`t6c`, `t7`/`t7c`). Three are pinned; the fourth is F5. In each, TypeScript agrees with the redundant spelling (for `t6`, Bundler and NodeNext do and Node10 does not).

### 3. The restated claim

- Point 1: true by construction and pinned.
- Point 2: true for the consumers that resolve, with the fail-closed exceptions in F3.
- Point 3: not true as written; one more kind (F2), and one shape where the star-conflict removal takes a correct edge (F1).
- Across 58 consumer fixtures (29 cases, `.js` spelling and canonical twin), every changed row is a gain equal to the canonical twin on main, except the pinned props drop. Class consumers through `.`, `..` and redundant segments gain; two-candidate lists refuse as the twin does (`m1`–`m8`). A cycle through `.` terminates (`r4`, `r5`).

### 4. Anything new and material from the fold

Nothing material. The new items are F4 (wording), F5 (test gap), F6 (naming) and F8 (cache note). The fold removed no gain measured in round 1: `i1c.sites.jsonl` is byte-identical to `i1b.sites.jsonl` on X and T (`cmp`), and both were cold builds.

### 5. The disputed point: a lone `x.js.ts`

**Keep the current behaviour. sol's premise is right for one mode and wrong for the others, and the controller's evidence sentence needs one qualifier.**

- Probe (`tsprobe/result.txt`), directory with only `x.js.ts`, specifier `./x.js`: Bundler `x.js.ts`; Node10 `x.js.ts`; NodeNext with a CommonJS-mode importer `x.js.ts`; NodeNext or Node16 with an ESM-mode importer **unresolved**. The redundant spelling `./a//x.js` gives the same four answers. `allowJs` makes no difference.
- Source, pinned `typescript.js:45423-45434`: `loadModuleFromFile` first replaces the extension, then adds extensions only `if (!(state.features & EsmMode))`.
- So "TypeScript never selects `x.js.ts`" is false in general, and true for ESM-mode files under Node16/NodeNext. The handbook section sol cites describes that mode.
- That does not make the edge wrong for prism. In the same ESM mode TypeScript also refuses `.` and a directory index (probe rows `bare_dot`, `dir_x_js_index_only`), and both are goal shapes of this slice. prism does not model the importer's module mode; it models the permissive resolution, and under it a lone `x.js.ts` is the module.
- A `.js` infix can be part of a name (`chart.js/index.ts`, `foo.js.ts`); the append step is how TypeScript finds those.
- The canonical `./a/x.js` already binds `a/x.js.ts` on main (`t2c`), and claim 1 keeps that. Refusing only the redundant spelling would make two spellings of one import disagree with no competing module to protect.
- In an ESM-mode project the import is a compile error and a runtime failure, so no well-formed program gets a false edge.
- Materiality either way: IMMATERIAL. If sol still contests after this evidence, the question for the owner is "should prism model the importer's module mode", which is a new contract, not a fix to this rung.

## Merge notes (not findings on the code)

- **Head.** The description's "Verification (head `a34a1d45`)" and test counts predate `5ee9b786`. At `5ee9b786` I ran `cargo test --lib js_relative_candidates` (35 passed), `cargo test --test integration js_relative` (37 passed), `cargo test --test integration js_binding_namespace` (37 passed) and `cargo fmt --check` (clean). CI on `5ee9b786`: all nine checks pass (read 16:03 MDT). CI on `a34a1d45` also completed green.
- **Mutation gate.** The stored report covers the 17-mutant lane only (452 s). `CLAUDE.md` names the full authoritative gate as the pre-merge check; neither the description nor the report shows a full run at this head. I did not run it: it is long, and the worktree was in use.

## Checked and found sound

- The fold diff `aa74eae0..a34a1d45` in full, and `5ee9b786`.
- `found_on_main`, the early return and the extracted proof, by reading against `origin/main` and on 29 namespace fixtures.
- The tier order, guards and index handling, on 33 fixtures and against TypeScript 5.9.3.
- Claim 1 at candidate level (by construction) and at edge level.
- Every consumer of the candidate list through `.js`, bare-dot and redundant spellings, with canonical twins: member import, re-export, star, import-forward, CommonJS, JSX, default and renamed imports, typed-parameter, constructor-local, type-only import, default-import class, props.
- The new lane file: 17 unique anchors, stored verdicts, and my scratch replica of the 14 unit-level mutants.
- The cache-anchor advance (`--plan-only`, exit 0).
- The measurement: cold caches, pre-fold and post-fold dumps byte-identical on X and T, adjudication totals; the stored 1,159-row matrix recounted (18 rows, all `import-path` MISSING to CORRECT; one `import-path` row left, `.jsx` naming a `.tsx` file).
- `nav module-deps` treats all spellings alike: it reports every JS import as unresolved on both binaries, canonical included, and gains the call-derived edge on the branch.

## Not checked

- The full test suite and clippy locally (CI only); `tier-a --quick`.
- The full authoritative mutation gate; the three cache-version mutants and `I1-15` to `I1-17` by execution (read from the stored report and the test text).
- The tsconfig-paths lane with `.js` hops at this head (round-1 result carried, not re-run); the executable-owner lane; `package.json` `main` for directory specifiers.
- `new` expressions: the fixture produced no `call_site` record on either binary, so it is no evidence.
- The checker adjudication of T (totals read, not re-run); corpus F (one aggregate file read, nothing else opened).
- Incremental (partial-hit) cache builds, symlinks, case-insensitive filesystems.
- Whether the follow-ups are tracked anywhere besides the PR description.
- The other round-2 reviewer's report and scratch directory (file names listed once, nothing read).

**Verdict: APPROVE** — no product change needed; amend point 3 of the description (F2, with F1's wording) before merge, and treat F3 to F8 as optional.
