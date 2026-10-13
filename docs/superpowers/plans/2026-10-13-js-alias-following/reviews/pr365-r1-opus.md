# PR #365 review (Opus reviewer, round 1 of 2)

Head reviewed: `e9d411cc` on `test/js-alias-variant-matrix`, base `e2701e89` (`origin/main`). Worktree left unchanged (`git status` clean before and after).

**Verdict: FIX.** The design is sound and the port is faithful, but the main test aborts with a stack overflow on machines with four or fewer rayon threads, which includes the PR's own CI run. One truth family is wrong, the bless guard has two holes that the planned slice order will hit, and the enumeration misses the alias spelling that main already follows today.

Findings are tagged WRONG or SMELL, and MATERIAL or IMMATERIAL to the goal in the brief. WRONG items come first.

---

## WRONG findings

### W1. The matrix test overflows the stack at four or fewer rayon threads; the PR's CI is red — WRONG, MATERIAL

**Evidence**

- CI on the PR head: run `38458579818`, job "Test Suite + Coverage (1/4)", conclusion FAILURE. Log: `SIGABRT [2.465s] prism::integration js_alias_matrix_test::matrix_matches_pinned_expectations`, stderr `thread '<unknown>' has overflowed its stack / fatal runtime error: stack overflow, aborting`. Shard summary: 1,345 run, 1,344 passed, 1 failed. Runner image `ubuntu-24.04`. Control: CI on the base commit `e2701e89` (run `38447788112`) is green. Saved log: `opus-scratch/ci-shard1-failed.log`.
- Local reproduction (15-core macOS, debug build), `RAYON_NUM_THREADS=N cargo test --test integration js_alias_matrix_test::matrix_matches`:

  | N | Result |
  |---|---|
  | 1, 2, 3 | abort, exit 101, same message |
  | 4 | abort in 6 of 6 runs |
  | 5, 6, 8, 15 | pass in 6 of 6 runs each |
  | 64 | pass |
  | 1 with `RUST_MIN_STACK=268435456` | pass (10.5 s) |

- Mechanism. `actual_rows` (`tests/integration/js_alias_matrix_test.rs:86-91`) runs `par_iter` on rayon's global pool, whose workers have the default 2 MiB stack. Each item calls `nav_session`, which calls `build_pool::install` on a different pool (`src/api/nav.rs:25`). A cross-pool `install` parks the calling worker in `wait_until`, and that worker keeps executing other pending pieces of the same `par_iter` on the same stack. So fixtures nest inside each other. Depth grows with fixtures divided by global workers: 1,011 / 4 aborts here, 1,011 / 5 passes.
- Direct observation: a stack sample of the single-thread run (`opus-scratch/sample.txt`) shows this period repeated 14 times inside the sampler's roughly 500-frame cap: `site_records -> nav_session -> build_pool::install -> ThreadPool::install -> Registry::in_worker_cross -> WorkerThread::wait_until -> ... -> bridge_producer_consumer::helper -> actual_rows closure -> site_records`.

Hypothesis log:

| Probe | Expected if nesting on global-pool workers | Expected if a per-fixture recursion in prism | Got |
|---|---|---|---|
| Thread sweep | fails at low N, passes at high N | fails at every N | fails at 1-4, passes at 5+ |
| 256 MiB `RUST_MIN_STACK` at N=1 (changes default-stack threads only; the build pool already has 256 MiB) | passes | still fails | passes |
| CLI, one process per fixture, all 1,011 | no crash | some fixture crashes | 0 errors |
| Stack sample | repeated `site_records` frames | one `site_records` frame | 14 nested |

The aborting thread is `<unknown>` (rayon workers are unnamed); the libtest thread carries the test name, so it is not the test thread. An `lldb` attempt failed for its own reasons (no debug permission in a non-interactive session) and is not used as evidence.

**Consequences**

- No run on Linux has ever compared the rows, so there is no evidence the pinned rows hold on a case-sensitive filesystem.
- The threshold moves up as fixtures are added, so larger machines start failing later.
- Under plain `cargo test` the abort takes down the whole `integration` binary (inferred from the process-wide abort; I did not run the unfiltered suite).
- Each aborted run leaks its temp dirs.

**Recommended fix.** Run the loop inside prism's large-stack pool, which is also what the CLI does (`src/main.rs:46`):

```rust
prism::build_pool::install(|| fixtures.par_iter().map(|f| row_for(f, observe(&site_records(f)))).collect())
```

The nested `install` is then same-pool and runs inline, on 256 MiB workers. Checked in a scratch harness that copies the PR's loop and reuses `js_alias_matrix_common.rs` unchanged (`opus-scratch/fixcheck`, output in `loop-strategies-output.txt`): the PR's strategy aborts at 1, 2 and 4 threads; the fixed loop gives 0 differing rows at 1, 2, 4 and 15 threads in 12.0, 8.0, 4.1 and 2.2 s. Not checked inside the PR's own test file, which I may not edit.

**Alternative.** A sequential outer `iter()`. It also gives 0 differing rows at every thread count, at about 16 s for 1-4 threads and 9.7 s at 15, so four to five times slower on a large machine. Raising the stack (`RUST_MIN_STACK`, or a private outer pool with a big stack) only raises the ceiling; depth still grows with fixture count.

**When it would not apply.** Machines with five or more hardware threads at today's fixture count, which is why it passed for the author.

### W2. `local.react-memo.*.call.*` (12 fixtures) state a truth that is false — WRONG, MATERIAL

**Evidence**

- `generator/variants.cjs:116-123`: shape `react-memo` has `forms: ['call', 'jsx']` and `truth: 'T'`. The call form is:

  ```ts
  import { memo } from 'react';
  import { f as imp } from './callee';
  const decoy = memo(imp);
  export function caller(c: boolean, opts: any) {
    return decoy(1); // SITE
  }
  ```

  Truth is `one[callee.ts:f]`. `React.memo` returns an element-type object (`{ $$typeof, type, compare }`), not a function, so `decoy(1)` throws and calls nothing in the repo. I did not execute React (not installed here); this is from React's source semantics.
- prism's landed contract says the same. A memo/forwardRef-wrapped callable is "bindable from JSX element sites only" (`src/ast/js_binding.rs:21-29`), and non-JSX sites drop `WrappedExportNonJsx` (`src/resolution.rs:3669-3671`). Probe on main (`opus-scratch/probes-output.txt`, K1 and K2): `const D = memo(function Inner(){…}); D(1)` gives `drop=WrappedExportNonJsx`; `<D />` gives Exact `Inner`.
- Ids: `local.react-memo.{module,fn}.call.{same,trap,fresh}.{ts,js}`. All 12 are pinned `MISSING`. A later slice that bound them to `f` would score twelve `MISSING -> CORRECT` moves for an edge prism's own rule refuses.

The JSX form of the same shape (truth `F`, contract-flagged) is fine.

**Recommended fix.** Restrict `react-memo` to `forms: ['jsx']` and add a `react-memo-called` shape with truth `NONE` for the call form. The 12 rows become `NONE-OK` on main.

**Alternative.** Drop the call form. Cost: loses a trap that a wrapper-following slice needs.

**When it would not apply.** If the owner rules that a direct call through a `memo()` result should be graded as the wrapped callable. That would reverse `WrappedExportNonJsx`, and the fixture's contract note should then say so.

### W3. An accepted id is a permanent waiver, not a one-time acceptance — WRONG, MATERIAL

**Evidence**

- `unaccepted_regressions` (`tests/integration/js_alias_matrix_common.rs:257-268`) excuses a regression when `accepted.contains(&c.id)`. Entries are `{id, reason}` with no transition, and nothing removes or invalidates them. The normal run only checks that each id names a fixture (`unknown_accepted`, `:271-278`).
- Failing state: `accepted_changes.jsonl` still holds `{"id":"x"}` from an earlier slice; `x` has since returned to `CORRECT`; a later change moves it to `WRONG`. Bless writes it with no new entry and no diff in `accepted_changes.jsonl`. Demonstrated with the PR's own functions: `stale acceptance: unaccepted_regressions = []` (`opus-scratch/guard-demo-output.txt`).
- The planned slice order makes this the expected path. 45 `local` rows are `CORRECT` only because the repo-wide name search happens to find a function spelled like the alias: `kind: free_single`, 43 of them `outer` scope with `same` naming. The "refuse the name search for bound names" slice moves all 45 `CORRECT -> MISSING`, so it must add 45 entries. The alias-following slice then moves them back to `CORRECT`, and the 45 ids stay waived for good.

**Recommended fix.** Make entries transition-scoped: `{"id", "from", "to", "reason"}`.

- Bless excuses a regression only when `(id, pinned verdict, new verdict)` matches an entry.
- The normal run fails when an entry's `to` differs from the pinned verdict of that row ("stale acceptance, delete it").

An entry then dies as soon as its row moves again, and it cannot excuse a different regression. Cost: about 20 lines plus two unit tests.

**Alternative.** Have the normal run require `accepted_changes.jsonl` to be empty, so acceptances live only in the bless working copy and the PR description. Cost: the reasons leave the tree.

**When it would not apply.** If the file is emptied by hand after every slice. Nothing enforces that.

### W4. The documented "Adding fixtures" flow replaces every pinned row with no guard — WRONG, MATERIAL

**Evidence**

- README lines 82-96: `export.cjs` "writes `fixtures.jsonl` and `expected.jsonl`. Copy both here."
- `generator/export.cjs:30,34` writes every row from whatever binary was passed.
- A slice author adding fixtures on their branch naturally passes their branch's binary. After copying both files, `cargo test` passes without a bless run, so `unaccepted_regressions` never executes and every regression the slice introduced is pinned with no `accepted_changes.jsonl` entry.

**Recommended fix.** Make `export.cjs` write only `fixtures.jsonl`, and change the README to: copy `fixtures.jsonl`, then run the bless command. Existing rows are then guarded, and new rows landing `WRONG` or `OVERCLAIM` need an entry. This also leaves one authoritative verdict implementation (the Rust one).

**Alternative.** Keep the export, and state in the README that `expected.jsonl` from `export.cjs` is for bootstrap only and must never be copied over an existing file. Cost: relies on reading the README.

**When it would not apply.** If fixtures are only ever regenerated with a binary built from `main`.

### W5. A pinned row deleted from `expected.jsonl` is treated as a new fixture — WRONG, IMMATERIAL

**Evidence.** `is_regression(None, new)` (`js_alias_matrix_common.rs:252`) flags only `WRONG` and `OVERCLAIM`. If a row pinned `CORRECT` at HEAD is removed from the working file and the actual verdict is `MISSING`, bless writes it with no entry: `deleted pinned row: unaccepted_regressions = []`. The normal run does catch the missing row; only the bless path is fooled. It needs a hand edit or a careless merge resolution, and the final diff still shows `CORRECT -> MISSING` on that line.

**Recommended fix.** In bless mode, take the "old" rows from `git show HEAD:tests/fixtures/js_alias_matrix/expected.jsonl` when git is available, falling back to the working file. Cost: the bless path, and only that path, depends on git.

**Alternative.** Document the limit in the README. Cost: none.

**When it would not apply.** When `expected.jsonl` is only ever written by bless.

---

## SMELL findings, MATERIAL

### S1. No fixture has an alias whose initializer is a plain function name, which is the shape main already follows — SMELL, MATERIAL

**Evidence**

- Every local alias shape initializes from a renamed import (`imp`) or a namespace member (`N.f`); the trap dimension is only on the alias name. Across all records in all 1,011 fixture repos, `IndirectResolution` appears in 2 fixtures (the static-property pair) and never from the local-variable path (`opus-scratch/cli-dump-no-cache.jsonl`).
- Main has a textual alias follower: `src/call_graph.rs:2744-2765` with `resolve_fptr_assignment` (`src/ast.rs:11993-12046`). When the callee is not a known function name and the function's source before the site has exactly one line `… h = <known function name>`, it adds a synthetic site that resolves as if the code called that name. It is scope-blind, and synthetic sites carry no lexical binding (`src/call_graph.rs:3023-3024`).
- Probes on main (`opus-scratch/probe2.cjs`, `probe4.cjs`, output in `probes-output.txt`):

  | Probe | Source (site is `h(1)`) | Main today | Truth |
  |---|---|---|---|
  | H1 | `import { f }…; const h = f;` in the caller | Exact `callee.ts:f`, origins Indirect + Source | `f`: CORRECT |
  | H1j | same, JSX `<H />` | Exact `F` | CORRECT |
  | H5 | `function helper…; const h = helper;` | Exact `helper` | CORRECT |
  | H12 | `import d from './dflt'; const h = d;` | Exact `dflt.ts:f` | CORRECT |
  | H13 | CJS `const { f } = require(…); const h = f;` | Exact `f` | CORRECT |
  | H6, H7 | same alias at module or outer scope | no target | MISSING |
  | H2 | `function outer(f) { function caller() { const h = f; … } }` | Exact `callee.ts:f` (free_single) | `may[]`: **WRONG** |
  | H2c | `const f = N.g` in outer; `const h = f` in nested caller | Exact `callee.ts:f` | `g`: **WRONG** |
  | H11 | `const h = f; try {…} catch (h) { h(1) }` | Exact `callee.ts:f` | `may[]`: **WRONG** |
  | L3 | `let h = f; for (…) { h(1); h = g; }` | Exact `f` only | `{f, g}`: PARTIAL |

  So the matrix shows `local.ident.fn.*` as `MISSING` and `catch-binding` as clean (`MAY-NONE`), while the commonest spelling of the same shapes is `CORRECT` or a false Exact on main.
- Why a planned slice could be wrong unnoticed:
  - The alias-following slice overlaps this mechanism (duplicate or conflicting records, or silently lost edges if it replaces it).
  - The name-search refusal keyed on the site's binding does not reach synthetic sites, so H2, H2c and H11 would survive it.
  - No fixture separates "resolve the initializer in its own scope and module" from "resolve it by name". With renamed or namespace initializers, a parameter shadowing the initializer (`import { f as imp }…; function caller(imp) { const h = imp; … }`, truth `may[]`), a shadowed namespace, a shadowed chain intermediate, and an exported alias whose initializer means something else in the importing file are all `MISSING` today and have no row (probes A1-A4).

**Recommended fix.** Add a bounded family, each with `trap` and `fresh` naming in `ts` and `js`:

1. `ident-direct`: `import { T } from './callee'; const L = T;` at scopes fn, module, outer; forms call and jsx.
2. `alias-of-same-file-fn`.
3. `alias-of-default-import`.
4. `initializer-shadowed-by-param`, in both the renamed-import and direct-name variants (A1, H2).
5. `initializer-shadowed-by-outer-const` (A3, H2c).
6. `catch-binding-direct` (H11).
7. `initializer-shadowed-namespace` (A2).
8. `exported-alias-initializer-other-module` (A4).
9. `let-written-after-site` and `let-written-in-sibling-closure` (L1-L3).

Roughly 80-120 rows, so land W1 first.

**Alternative.** Add only items 1, 4 and 6 (about 30 rows). Cost: the value-side traps for chains and cross-module aliases stay open.

**When it would not apply.** If the alias slice first deletes the Level-1 path for JS/TS. Then items 1-3 still matter as rows that must not regress from `CORRECT`, and 4-8 still matter as traps.

### S2. No fixture produces a name-only target; three verdicts are never pinned — SMELL, MATERIAL

**Evidence**

- `nameonly` is `[]` in all 1,011 rows, and no record anywhere in any fixture repo has a name-only target. `DEMOTED`, `MAY-NAMEONLY` and `NONE+NAMEONLY` never occur.
- Every by-name pool in the matrix has exactly one candidate, so only the `1 =>` arm of R5 (`src/resolution.rs:3650-3654`) runs.
- Probes on main: an outer-scope alias named `decoy` with two files defining `decoy` gives two `name_only/free_multi` targets (B1); an unbound `f` with two module targets gives two (B2); an outer destructured prop gives two (J2).
- A refusal slice that guarded only the Exact arm would leave wrong name-only edges for bound names and no row would change.

**Recommended fix.** Add a `trap2` naming (a second file defining the decoy name) for `ident.outer`, `object-param-destructure.outer` and `module-unbound-module-target`. About 12 rows.

**Alternative.** One row per mechanism (3 rows). Cost: no `ts`/`js` or form spread.

**When it would not apply.** If the refusal is placed before the pool-size match, which is likely but currently unpinned.

### S3. Import-path: one suffix trap and the currently-correct multi-segment controls are missing — SMELL, MATERIAL

**Evidence**

- Every multi-segment specifier in the matrix is `MISSING` today (`'.'`, `'..'`, `./lib//callee`, `./lib/./callee`, `./lib/../callee`). There is no `CORRECT` control for the ordinary forms, although main resolves them (probes D1, D4, D9, D6):
  - `./pkg` to `pkg/index.ts`
  - `../callee`
  - `./lib` with both `lib.ts` and `lib/index.ts` present (file wins)
  - explicit `./callee.ts`
  
  A normalization change could break any of these with no row moving.
- Trap: `import { f } from './callee.js'` where only `callee/index.ts` exists (truth `none`, main `UnknownName`, probe I1). A "strip the suffix and re-resolve" implementation would bind `callee/index.ts:f` Exact, and it passes all three existing suffix traps.
- New-behaviour shapes with no row: `../callee.js` (D10), `./pkg/index.js` (D3, I6), `./comp.js` with both `comp.ts` and `comp.tsx` (I3). All `MISSING` today.

**Recommended fix.** Add the trap, the four controls and the three new-behaviour shapes in `ts`, and `js` where meaningful. About 14 rows.

**Alternative.** Add the trap and the `./pkg` and `../callee` controls only (about 6 rows). Cost: precedence cases stay unpinned.

**When it would not apply.** If the import-path slice is limited to a suffix table applied after an unchanged resolver. The trap still applies.

---

## SMELL findings, IMMATERIAL (recorded, not blockers)

**S4. The guard's definition of regression omits exits from `NONE-OK`, moves into `NO-RECORD`, and same-verdict target changes.**
- Evidence: `is_regression` is false for `NONE-OK -> NONE+NAMEONLY`, `NONE-OK -> NO-RECORD`, `PARTIAL -> MISSING` and `MISSING -> NO-RECORD`; a wrong name-only target appearing on a `MISSING` row is reported but not guarded (`guard-demo-output.txt`).
- Fix: also treat leaving `NONE-OK` as a regression (one condition).
- Alternative: rank verdicts per truth kind and guard any rank drop. Cost: a table to maintain.
- Would not apply while no fixture can produce a name-only target; it becomes reachable once S2 lands.

**S5. `param.shadows-import.call.same.{ts,js}` truth `none` is inconsistent with sibling fixtures.**
- Evidence: an exported `caller(f)` with no visible caller. `param.inner-param-shadows-alias` (same situation) uses `may[]`, and `plain-exported` carries "callers may be invisible" (`variants.cjs:556` against `:568` and `:494`).
- Fix: `may[]`.
- Alternative: document that unreferenced exported parameters are `none`. Cost: must then change the other two.
- Would not apply to any Exact outcome: both truths grade Exact as `WRONG`; they differ only for name-only targets.

**S6. Script-global truths for `.js` fixtures hold only under script loading.**
- Evidence: `global.module-to-script-global.call.js` and `global.script-to-script-global.call.js` have truth `glob.js:f`; under Node module loading `f` is not global. This matches TypeScript's program model and the E5 rulings.
- Fix: one README sentence stating the model.
- Alternative: mark them with a `contract`. Cost: none.
- Would not apply if the project's stated model is TypeScript's.

**S7. `local.react-lazy` (6 rows) is pinned `WRONG` for behaviour that the may-call ruling accepts in `call-with-inline-fn`.**
- Evidence: both bind the inline function argument (`caller.tsx:Decoy@2/local_def`). The truth is correct in both; only the label differs. Both are contract-flagged.
- Fix: none needed before merge; the contract decision settles it.
- Alternative: truth `may[dflt.F]` until decided. Cost: hides a real question.

**S8. An expected row does not carry its truth, so a truth edit changes a verdict with unchanged targets and looks like an improvement.**
- Evidence: editing a truth from `may[f]` to `one[f]` turns `OVERCLAIM` into `CORRECT` with no guard; the reviewer must find the change in an 869 KB `fixtures.jsonl`.
- Fix: add a compact `truth` string to each row.
- Alternative: have bless refuse when `fixtures.jsonl` differs from HEAD for an existing id. Cost: git dependency.
- Would not apply if truth edits are always reviewed in `variants.cjs`.

**S9. No structural check on truth targets, and the TypeScript cross-check is not asserted.**
- Evidence: a misspelt target (`callee.ts:ff`) would silently make a row uncorrectable; one target already has `line: null` (`import.super-extends-member-named-super.call.js`, intended). `run.cjs` stores `oracle` but never compares it. Nothing checks that `fixtures.jsonl` matches `variants.cjs` (it does today, byte for byte).
- Fix: a Rust test that every truth target's file is in `files`, and have `run.cjs` print oracle disagreements.
- Alternative: leave to review. Cost: slips surface late.

**S10. `generator/variants.cjs` is 687 lines** against the repo's 600-line rule (CLAUDE.md, design decision 7); `js_alias_matrix_common.rs` is at 550.
- Fix: split by family (local, cjs, param, import).
- Alternative: declare generator scripts exempt. Cost: none.

**S11. Bless writes `expected.jsonl` non-atomically while sibling tests may read it.**
- Evidence: `std::fs::write` at `js_alias_matrix_test.rs:149`; the README command filters to one test, but `PRISM_BLESS_JS_ALIAS_MATRIX=1 cargo test --test integration js_alias_matrix` runs the format and id tests concurrently.
- Fix: write to a temp file and rename.
- Alternative: keep the README's narrower filter. Cost: none.

**S12. PR description.**
- It says the `WRONG` rows are "two families"; there are five groups: 65 outer-scope name search, 36 shadowed same-file function, 6 `react-lazy`, 5 unbound or lib-global, 1 `import decoy = N.f` at module scope.
- It reports a local full-suite pass and does not mention that CI is red on the head commit.
- Fix: correct the text. Alternative: none needed.

---

## Answers to the six questions

**1. Harness fidelity.** Faithful.
- The test reaches the same code as the CLI (`src/main.rs:246-253` against `js_alias_matrix_test.rs:72-83`).
- I ran the built `prism` binary on all 1,011 fixtures in three modes (`--no-cache`, cold `--cache-dir`, warm cache) and derived rows independently: 0 rows differ from `expected.jsonl` in each, and the full record dumps are byte-identical across modes.
- A differential test of the Rust `observe` + `verdict` against `run.cjs`'s `matcher` + `verdict` (source lifted from the file) plus `export.cjs`'s normalisation: 20,000 random cases, 0 disagreements, all 11 verdicts covered.
- By reading, the only inputs that would diverge are a non-string `origin` and target names that sort differently in UTF-16 and UTF-8; neither occurs.
- The real divergence is environmental: the CLI runs inside the build pool and the test does not (W1).

**2. Reporting and the bless guard.** The normal run reports every differing row, missing rows and orphan rows; reordering and non-canonical bytes are caught by the two structural tests. The guard has the holes in W3, W4, W5, S4 and S8.

**3. Ground truth.** I read one fixture of every shape, including every contract-flagged shape and every shape in `global`, `import-path`, `none` and `param-lookup`, plus scope and form variants of the scope-rich shapes.
- The TypeScript 5.9.3 checker agrees on 611 fixtures it can type (562 `one`, 49 `set`). The 9 it answers differently are type-level artefacts with the authored truth right: 6 `destructured-props-default-overridden.js`, 1 `.d.ts`-only, 2 lib `isNaN`.
- Wrong: W2 (12 fixtures).
- Arguable: S5 (2), S6 (2), S7 (6).

**4. Pinned WRONG rows.** Pinning them is the right call for a characterization test, and the label is right on every family I sampled (the five groups in S12). Two cautions:
- 45 `CORRECT` rows are spelling coincidences that the refusal slice will correctly turn `MISSING` (see W3).
- The matrix under-reports main's `WRONG` set: the Level-1 false Exacts in S1 are not pinned.

**5. Coverage.** S1, S2 and S3 are the material holes. Considered and not material for the named slices:
- Sites inside anonymous inline callbacks (`items.map((x) => h(x))`, `useEffect`, JSX attribute arrows) produce no call-site record on main at all.
- `new` expressions likewise: all 34 `new` rows are `NO-RECORD`, and so are a direct `new K()` on an import and on a same-file class.
- `export * as N from` (drops `ImportExternal`), `??` fallbacks, typed `const` aliases and cross-scope chains all behave like shapes already present.

**6. Hygiene.**
- Hermetic: temp dirs; no cache files written (checked `~/Library/Caches/prism` before and after a run); no parent-directory reads without a tsconfig.
- Deterministic across repeated runs at 5 or more threads; not robust at 4 or fewer (W1).
- `tests/integration/main.rs` wiring is correct and `coverage_test.rs` lists are unaffected.
- File-length rule: S10.
- Likely to rot: the duplicated verdict in `run.cjs` (W4's fix removes the need), the unasserted oracle (S9), the permanent acceptance list (W3).

---

## Checked and found sound

- `fixtures.jsonl` regenerates byte-identically from `generator/variants.cjs` (`regen-check-output.txt`).
- Exactly one `// SITE` marker per fixture; no fixture has two same-named declarations in a truth file or two callables on an `// ANON` line, so the name-or-line matcher is unambiguous on the present fixtures.
- The test passes locally at the default thread count: 20 tests, 1,011 fixtures in 2.2-3.9 s.
- `changes`, `orphan_expected` and `summary` do what the README says; `pinned_file_is_in_the_format_bless_writes` plus the id-order test catch reordered, duplicated or hand-formatted rows.
- All truths in `import`, `import-path`, `same-file`, `global`, `none`, `param-args`, `param-lookup` and the CJS shapes, apart from the items listed.
- No TypeScript syntax errors in any fixture site file.

## Not checked

- Row parity on Linux or any case-sensitive filesystem: the only CI run aborted before comparing.
- The unfiltered suite (`cargo test`, `--features mcp`, nextest), clippy, fmt, doctests, the mutation gate.
- The bless write path end to end on the real files (it writes into the worktree).
- The W1 fix inside the PR's own test file; it was validated only in the scratch harness.
- Every one of the 1,011 fixtures by hand: one per shape plus variants was read, and 611 were machine-checked.
- React at runtime: W2 rests on React's source semantics and prism's own rule and probes.
- Behaviour under `cargo llvm-cov` locally; Windows; the first commit on its own.

## Side effects of this review

- Reproducing W1 aborts the test mid-run, and each abort leaks `tempfile` directories. At the time of checking, `$TMPDIR` held 8,331 fixture-shaped `.tmp*/repo` directories (mine, plus any from earlier aborted runs). I did not delete them because another reviewer may be running the same test; they are tiny and safe to remove once no run is active.
- Nothing was written to the worktree apart from cargo's own `target/` output from the permitted test command. No git writes, nothing posted to GitHub. CI logs were read with `gh` (read-only).
- Scratch contents: scripts, saved outputs, and a scratch crate `fixcheck` built offline with a path dependency on the worktree. `oracle.cjs` reads fixture copies under `work/`, which `node cli_parity.cjs no-cache` recreates.

---

## Verdict

**FIX.** Must change before merge:

1. **W1**: run the fixture loop inside `prism::build_pool::install` (or sequentially), and get one green CI run so Linux parity is established.
2. **W2**: correct the truth of the 12 `react-memo` call-form fixtures.
3. **W3**: make acceptances transition-scoped (or require the file empty in the normal run).
4. **W4**: stop the regeneration flow from overwriting `expected.jsonl`; route new rows through bless.
5. **S1, S2, S3**: add the listed fixture families. The reduced sets named as alternatives are acceptable if the controller prefers a smaller diff.

Everything else is recorded for the controller's discretion.

**Verdict: FIX**
