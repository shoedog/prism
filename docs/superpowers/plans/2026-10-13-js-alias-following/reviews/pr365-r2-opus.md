# PR #365 review (Opus reviewer, round 2 of 2, confirmation)

Head reviewed: `35557277` on `test/js-alias-variant-matrix` (four commits on `origin/main` `e2701e89`; no product source change). CI on that head: nine of nine checks pass. Worktree is clean at the same head now (see "Side effects" for one mistake of mine that I undid).

**Verdict: FIX, with no further review needed.** Every round-1 WRONG finding is closed, the fold's own code is sound, and every new or corrected truth I read is right. No finding is both WRONG and MATERIAL. What remains is closed and enumerable:

- **S1** (SMELL, MATERIAL): one round-1 shape family was left open. Ten rows close it; a verified generator patch is supplied.
- **W1, W2** (WRONG, IMMATERIAL): two one-line corrections.

S1 is a SMELL, so under the owner's severity rule it does not block the merge. Fold it here, or make it the first commit of the next slice, before that slice's rows are reviewed.

Findings are tagged WRONG or SMELL, and MATERIAL or IMMATERIAL to the goal in the brief. WRONG items come first.

---

## WRONG findings

### W1. The README's "Adding fixtures" command does not work as written — WRONG, IMMATERIAL

New text from the fold (`README.md:105-108`):

```bash
node generator/export.cjs tests/fixtures/js_alias_matrix
```

**Evidence** (run on a scratch copy of the fixture directory, `readme-check/`):

| Working directory | Result |
|---|---|
| repo root | `Error: Cannot find module '<root>/generator/export.cjs'` |
| `tests/fixtures/js_alias_matrix` | prints `fixtures 1159`, but writes to a new nested `tests/fixtures/js_alias_matrix/tests/fixtures/js_alias_matrix/fixtures.jsonl`; the real file is untouched |

In the second case the following bless sees no new fixture, so a generator edit silently does not reach `fixtures.jsonl`. `git status` does show the stray directory.

- **Fix:** `node tests/fixtures/js_alias_matrix/generator/export.cjs tests/fixtures/js_alias_matrix` (checked from the repo root: output is byte-identical to the committed file).
- **Alternative:** make `export.cjs` default to its own parent directory and refuse an out-dir that holds no `fixtures.jsonl`. Cost: about three lines.
- **Would not apply:** never; neither working directory works.

### W2. A `// TARGET:` marker for a longer name is read as the marker for its prefix — WRONG, IMMATERIAL (latent)

New code from the fold: `generator/lib.cjs:12` finds the marker with `l.includes('// TARGET:' + name)`.

**Evidence:** for the source `function fx() {} // TARGET:fx` (line 1) and `function f() {}` (line 2), `declLine(src, 'f')` returns 1; the right answer is 2. The generator's own names include such pairs (`K` and `K2`, `Decoy` and `DecoyK`, `H` and `HK`). The Rust check would not catch the result, because it only tests that the line contains the name as a substring.

No fixture uses a marker today (0 of 1,159), so no row is affected.

- **Fix:** match with a boundary: `new RegExp('// TARGET:' + name + '(?![\\w$])')`.
- **Alternative:** remove the marker branch until a fixture needs it. Cost: loses the escape hatch that the tightened identity rule relies on for duplicate names.
- **Would not apply:** while no fixture uses a marker.

---

## SMELL findings

### S1. Main's false Exact when the alias name itself is rebound has no row — SMELL, MATERIAL (round-1 Opus S1 item 6, left open)

Round 1 listed `catch-binding-direct` (probe H11) in its reduced set. The fold added `ident-rhs-is-catch-param`, which is a different shape: it rebinds the initializer's name, not the alias, and it is pinned `MAY-NONE`, so it does not show the false edge.

**Evidence.** Probes on main, run through the PR's own `observe` and `verdict` code (`probes2-out.txt`, `probes3-out.txt`):

| Source (site is the marked call) | Main today | Truth | Verdict |
|---|---|---|---|
| `const h = f; try { opts(h); } catch (h) { return h(1); }` | Exact `callee.ts:f`, `import_member`, origin IndirectResolution | `may[]` | WRONG |
| `caller(c, h) { if (c) { const h = g; return h; } else { return h(1); } }` | Exact `callee.ts:g`, same path | `may[]` | WRONG |
| `const h = f; if (c) { const h = g; void h; } return h(1);` | no target | `f` | MISSING |

- A scan of all 1,159 fixtures finds none with a bare-identifier alias of the called name plus a second binding of that name (`scan_rebound.cjs`; the same scan flags all six of my probes of that kind).
- All four pinned Indirect `WRONG` rows rebind the initializer's name (outer parameter, outer `const`).

**Why a planned slice could be wrong with no row noticing.**

- The natural repair of the four pinned rows is to resolve the synthetic site's name (`f`) with its lexical binding at the site. That turns all four into `MAY-NONE`. In the first two shapes above, `f` or `g` is the import at the site, so the false edge stays, and the matrix would show no Indirect `WRONG` row at all.
- A new alias follower that treats a function body as flat (nearest preceding declaration) passes `block-shadow`, whose site is inside the block. It fails only when the site comes after the block, which is the third shape.

**Recommended fix.** Add three shapes, 10 rows. The patch is `r2-opus-scratch/recommended-generator.diff` (41 lines, two generator files). Checked in a scratch copy of the generator:

- It exports 1,169 fixtures; the existing 1,159 lines are byte-identical and in the same order.
- `run.cjs` reports 745 typed, 14 differing, 0 unexplained; the checker confirms `f` for the four typed new rows.
- New fixtures need no ledger entry.

Rows to expect from the bless:

| Fixture ids | Rows | Today |
|---|---:|---|
| `local.ident-alias-rebound-by-catch.fn.call.fresh.{ts,js}` | 2 | `WRONG`, Exact `callee:f`, origins IndirectResolution + Source |
| `local.ident-alias-rebound-by-catch.fn.call.trap.{ts,js}` | 2 | `MAY-NONE` |
| `param.alias-in-sibling-block.call.fresh.{ts,js}` | 2 | `WRONG`, Exact `callee:g`, same origins |
| `local.ident-inner-block-does-not-leak.fn.call.{trap,fresh}.{ts,js}` | 4 | `MISSING` |

The PR description's counts then change: 1,159 to 1,169 fixtures, 131 to 135 `WRONG`, 4 to 8 through the textual alias path.

- **Alternative:** land the same patch as the first commit of the refusal or alias slice. Cost: the hole is open until then, and it depends on someone remembering.
- **Would not apply:** if the alias slice removes the textual alias path for JS/TS outright. The third shape still matters as a trap for the new follower.

### S2. Bless parses the committed baseline with the full row type — SMELL, IMMATERIAL

`baseline()` deserializes each line of `git show HEAD:…` as `Row` (`js_alias_matrix_test.rs:79`), but the guard only needs `id` and `verdict`.

**Evidence:** with a committed baseline lacking one field (the state any later PR that adds a field to `Row` will be in), bless refuses with `the committed baseline is unavailable (baseline line 1: missing field …)` (`bless-SB.log`). That PR could then only update the pins by hand, outside the guard.

- **Fix:** parse the baseline into a two-field struct.
- **Alternative:** give every later field `#[serde(default)]`. Cost: relies on remembering.
- **Would not apply:** while `Row`'s fields never change.

### S3. The guard acts only when bless runs — SMELL, IMMATERIAL (known design limit, carried from round 1)

**Evidence** (scratch repo):

- A pinned row hand-edited to the actual row leaves 0 differing rows, so the normal run passes with no ledger entry (`bless-B3.log`). The failing test prints that row ready to paste.
- After a guarded bless is committed, deleting its ledger entry still passes the normal run (`normal-N2.log`).
- A renamed fixture id is a delete plus an add; a new id is never a regression.

The stated control is review of the `expected.jsonl` diff, and all three cases show there.

- **Fix (optional):** let the normal run apply `bless_problems` against a base revision named by an environment variable, and set it in CI. Cost: about 15 lines and a deeper CI checkout.
- **Alternative:** leave as is. Cost: none; the ledger stays advisory.
- **Would not apply:** when authors always go through bless.

### S4. Recorded, no action needed before merge — SMELL, IMMATERIAL

- **Two permanent ledger entries.** The `param.shadows-import` entries can never go stale, because `MAY-NONE` is the best verdict for a `may[]` truth. Decide later whether truth corrections belong in a ledger of "regressions still in effect".
- **Duplicate function name.** The four `local.ident-rhs-is-outer-const` fixtures declare `function caller` twice (the wrapper and the nested function). A probe with the wrapper renamed gives the identical row, so today it is cosmetic.
- **Weak spots in the two new checks.** `ORACLE_NOTES` exempts a whole shape, including its TS variants that agree with the checker. `every_truth_points_at_real_source` tests the name as a substring, which is nearly always true for `f`.
- **Verdicts still never pinned.** `DEMOTED`, `MAY-NAMEONLY` and `NONE+NAMEONLY` occur in no row; they are covered by unit tests only. The 12 name-only rows are `MISSING` or `MAY-NONE`.
- **One failing fixture at a time.** A prism panic or `nav_session` error inside the loop reports only that fixture. This is unchanged code, outside the fold.
- **Guard gaps left by the widened set.** `PARTIAL -> MISSING` and `DEMOTED -> MISSING` are not regressions, so an accepted `CORRECT -> MISSING` followed by `MISSING -> PARTIAL` ends with an empty ledger.

Other shapes without a row, with what main does today (`probes-out.txt`). None is a case where I can name a wrong result going unnoticed for the next two slices:

| Shape | Main today | Matters when |
|---|---|---|
| `const { f } = require('./callee')` at module scope, `const h = f` in the caller | CORRECT through the textual path | the alias slice replaces that path (a lost edge, not a wrong one) |
| `import { renamed }` where the importing file has its own `f` | MISSING | a slice follows exported aliases across modules |
| `let h = f` written after the site in a loop, or in a closure | MISSING | a slice follows `let` |
| explicit `./callee.ts` | CORRECT | #368 is additive by design, so it cannot change this |
| `./comp.js` with both `comp.ts` and `comp.tsx` | MISSING | covered by #368's own unit tests |
| hook imported from another file | MISSING | the later hook slice |
| alias of a default import | MISSING; CORRECT only when some repo function shares the import's local name (round 1's probe had one) | nothing stable to protect |

---

## Answers to the five questions

### 1. Closure

| Round-1 finding | State | How I checked |
|---|---|---|
| Stack overflow (Opus W1, sol S2) | Closed | see question 2 |
| `memo()` call truth (sol R1, Opus W2) | Closed | read the fixtures: 12 call rows have truth `none`, pinned `NONE-OK`; JSX rows unchanged |
| `param.shadows-import` (sol R2, Opus S5) | Closed | truth `may[]`, pinned `MAY-NONE`, two ledger entries match the transition |
| Identity rule (sol R3) | Closed | see question 2 |
| Deleted pinned row (sol R4, Opus W5) | Closed | scratch bless with the row deleted: refused, `CORRECT -> MISSING` |
| Permanent waiver (Opus W3) | Closed | entry with wrong `from` refused; exact transition accepted; stale entry fails the normal run |
| Exported pins (Opus W4) | Closed | `export.cjs` writes `fixtures.jsonl` only; regeneration is byte-identical. The new command line is W1 |
| Sort order (sol R5) | Moot | confirmed by reading that Rust is the only writer of pins; not tested with non-ASCII names |
| 600 lines (sol R6, Opus S10) | Closed | largest generator file 364 lines, largest Rust file 452 |
| Missing shapes (sol S1; Opus S1, S2, S3) | Closed except S1 above | every listed shape exists and its truth is right; Opus S1 item 6 is open; items 8 and 9 were not added (conditional, see S4) |
| Policy rows (sol S3) | Closed | one target with policy text; `Ctx` is exported |
| Script-global truths (Opus S6) | Closed | contract text present on the four rows |
| Structural check, checker assert (Opus S9) | Closed | new Rust test passes; `run.cjs` exits 1 on a mutated truth (18 unexplained) and 0 on the real generator |
| PR description (Opus S12) | Closed | recomputed the table, 131 = 85 + 42 + 4, 12 name-only rows, 727 of 741: all match |
| Regression set (Opus S4), atomic write (Opus S11) | Closed | unit tests; no temp file left after a bless |
| Not changed by decision (Opus S7, S8) | Accepted | no new evidence against either |

### 2. The fold's own code

**Guard logic and git.** Right in every case I ran. The bless runs used a scratch crate that compiles the PR's three test files unmodified, inside a scratch git repository. Three rows, marked, were checked with the `git show` command alone.

| State | Result |
|---|---|
| nothing changed | writes a byte-identical file |
| regression, no entry | refused, nothing written |
| pinned row deleted, or hand-edited to the actual row | refused (baseline is `HEAD`) |
| entry with wrong `from` | refused |
| entry with the exact transition | written |
| orphan pinned row | refused |
| ledger entry for an unknown id | refused in bless, fails in the normal run |
| new fixture landing `WRONG` | written, no entry needed (as documented) |
| repository with no commit | refused: `invalid object name 'HEAD'` |
| `expected.jsonl` not tracked at `HEAD` | refused: `exists on disk, but not in 'HEAD'` |
| detached `HEAD` | works |
| launched from a subdirectory, or the test binary run from `/` | works (the directory is fixed at compile time) |
| depth-1 shallow clone (`git show` only) | returns all 1,159 lines |
| crate in a subdirectory of the git root (`git show` only) | works; the `./` prefix is what makes it work |
| linked worktree, the PR's own (`git show` only) | works |
| CI (by reading) | never calls git; the normal run reads files only |

**`build_pool::install`.** A real fix, not a moved threshold. Inside the pool, each `nav_session` runs inline, so the per-fixture cross-pool wait is gone. Measured with a depth counter around the PR's loop:

| Loop | Threads | Fixtures | Deepest nesting | Stack used by nesting |
|---|---:|---:|---:|---:|
| fixed | 1, 2, 4 | 1,159 | 1 | 0 |
| fixed | 8 | 1,159 | 2 | 63 KB |
| fixed | 15 (unset) | 1,159 | 4 | 154 KB of 256 MiB |
| fixed | 4 and 15, three runs each | 4,636 | 2 | 64 KB |
| pre-fix | 15 | 1,159 | 113 | 1.11 MB of 2 MiB |
| pre-fix | 2 | 1,159 | aborts, stack overflow | 362 fixtures open at the abort |

- Residual nesting is opportunistic work stealing. It does not grow with the fixture count.
- Same-environment control: the pre-fix loop at 2 threads aborts; the fixed loop at 2 threads passes with depth 1.
- In the worktree all 23 matrix tests pass at 1, 2, 3, 4 and 8 threads and at the default. Six further default runs of the matrix test alone also pass.
- On CI (ubuntu-24.04) `matrix_matches_pinned_expectations` passes in 11.2 s. This is the first Linux comparison of the rows.

**Identity rule.** Consistent.

- Rust `hit` and the reference `hit` in `run.cjs` are the same condition.
- The reference functions, lifted from `run.cjs`, grade both of sol's counterexamples `WRONG` and reject an unmarked duplicate declaration.
- The reference run over the CLI (the worktree's debug binary, Node verdict and matcher) against the pinned file: 1,159 rows, 0 differing fields.
- No row pinned before the fold changed verdict because of the rule: the 15 changed rows are the 12 `memo`, 2 `shadows-import` and 1 policy row.

### 3. New fixtures and corrected truths

No wrong truth found. One arguable row, already marked as policy.

- I read the source of every TS variant of the 148 new fixtures, and the JS variants of the single-naming shapes. The JS variants come from the same templates.
- The TypeScript 5.9.3 checker agrees on 116 of the 148. Six differ (`hook-result-two-slots` in JS, where the checker names the first member of an untyped array; the authored `f` is right by reading). The other 26 are `may[]` or `none` truths the checker cannot type.
- Corrected truths: `memo` call (`none`), `shadows-import` (`may[]`), context default with `Ctx` exported (`may[f]`): all right.
- Arguable: `import.named-js-suffix-literal-and-ts.call.ts` now has the literal `callee.js` as its one target. TypeScript binds `callee.ts`; bundlers and Node bind the literal. The row says it is policy, and a change of policy would now show as `CORRECT -> WRONG`, which needs a ledger entry. That is better than the old two-target truth.

### 4. New and material from the fold

Nothing material. W1 and W2 are new and immaterial. S2 and the first three notes in S4 are new and immaterial.

### 5. Remaining holes

S1 only. The table in S4 lists the other shapes I probed and why each is not a hole for the next two slices.

---

## Checked and found sound

- All round-1 WRONG findings closed (table above).
- Worktree test: 23 passed at 1, 2, 3, 4, 8 threads and default; six extra default runs of the matrix test, all pass.
- CI: nine checks pass; the four test-shard logs show all 23 matrix tests passing on Linux.
- Fixed loop: nesting depth at most 4, independent of fixture count; control abort reproduced only with the pre-fix loop.
- Bless end to end through the real test code, in the twelve states of the table in question 2 that are not marked otherwise.
- `fixtures.jsonl` regenerates byte-identically from the generator; `export.cjs` writes that one file.
- Fold diff of fixtures: 148 added, 0 removed, 31 changed, all intended; order of the old ids preserved; `expected.jsonl` is in fixture order.
- CLI reference run agrees with all 1,159 pinned rows.
- `run.cjs` checker cross-check: 741 typed, 14 differing, 0 unexplained; exits 1 on a mutated truth.
- PR description numbers.
- One `// SITE` per fixture; no truth target with two declarations; one intended null line (`super`).

## Not checked

- The full suite, nextest, clippy, fmt, doctests, the mutation gate: I relied on CI for these and read only the matrix lines of the shard logs.
- The unfiltered `integration` binary in one process (matrix test sharing the pool with other tests).
- Every JS variant of the new fixtures by hand, and any truth the fold did not touch.
- Windows, CRLF checkouts, `cargo llvm-cov` locally, the `mcp` feature.
- Bless in the worktree itself (not allowed); it ran in the scratch crate.
- #368's branch against this matrix; I read its description only.
- The written plans for the alias and refusal slices. S1's materiality rests on main's observed behavior and the most natural repair, not on those plans.
- React at runtime.

## Side effects of this review

- **A mistake, undone.** At 15:21 a scratch script of mine wrote a 6 KB untracked file named `x` into the worktree root (wrong argument index while its working directory was the worktree). I moved it to my scratch directory a few minutes later; `git status` was clean after that and is clean now. A reviewer who ran `git status` in between may have seen `?? x`. After that I ran every script from the scratch directory.
- `cargo test` in the worktree wrote only to its `target/`. No git writes in the worktree; nothing posted to GitHub (`gh` was used read-only).
- The one abort I reproduced ran in the scratch crate with `TMPDIR` pointed into scratch. Its 362 leaked temp directories are under `r2-opus-scratch/tmp-abort`; none are in the system temp directory.
- Scratch is about 1.4 GB, nearly all of it `harness/target` (build output, safe to delete). It also holds a scratch git repository (`harness/`), the probe inputs and outputs, and `recommended-generator.diff`.

---

**Verdict: FIX** (no further review needed) — S1: add the 10 rows from `recommended-generator.diff` and re-bless, here or as the first commit of the next slice; W1: correct the README regenerate command; W2: boundary-match the `// TARGET:` marker. No finding is both WRONG and MATERIAL.
