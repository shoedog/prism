# PR-B R10 fix-delta check (Opus) — source head `fc96688e`, delta `96817370..fc96688e`

Reviewer: Opus 5.5, read-only, 2026-10-10. One narrow round on the four product hunks, the tests, the cache pin and the two registries. Cap: 1 round, no extension.

**Goal this check serves:** whether `fc96688e` lands on main as PR-B. Contract: zero false Defs or edges; no LOST correct rows against main; Exact is a static-binding grade; call sites, non-JS output and symbol/call/module navigation byte-identical. Flow / type-annotated JavaScript is out of scope by owner decision.

**Materiality rule (unchanged from my R9 review):** MATERIAL = occurs in a measured corpus, or is triggered by a shape idiomatic in the code PR-B targets, or changes a number the owner decides on. IMMATERIAL = reproduced, but no measured and no realistic incidence. IMMATERIAL findings are recorded, never blockers. Under a strict reading of "zero false edges / no LOST rows" (the reading sol used last round) R10-1 to R10-4 would each be a contract instance; I state the incidence beside each so the owner can apply either reading.

## Verdict: APPROVE

- B1, B2, B4 and B5 are closed. B3 is closed for every form in the brief except one (enum declarations merged across `case` clauses of one `switch`), which is unchanged from the previous head.
- No MATERIAL finding. Four WRONG / IMMATERIAL findings, all in the enum and eval predicates. One of them (R10-1) is a new regression created by the B3 hunk; it needs two enums literally named `yield` and `await` in one scope. The other three are older rows the hunks did not reach.
- I recommend recording R10-1 to R10-4 in the SPEC follow-up list, correcting one sentence there (S1), and not opening another source round. Each fix is one or two lines, but any of them invalidates the binary-bound gates and the 579-root certificate for a shape with no measured or realistic incidence.

Convergence note: this round has 0 MATERIAL findings (last round 0 product / 2 evidence), and the one new regression is narrower than N1 and N2 were. The class is still open in the sense that each syntactic predicate has further boundary forms; the remaining ones are listed below so that nobody has to rediscover them.

## Status of B1–B5

| Item | Status | Evidence (all reproduced on the frozen binaries: main `da0604b3`, previous head r9, head r10) |
|---|---|---|
| B1 labelled body-level function declarations | **CLOSED** | `function q(y){ lbl:function y(){} use(y); }` and my `function N(y)` original: head equals main (2 rows, Exact) in JS, TS, TSX; single and chained labels; with a comment after the label; method, object method, function-valued property; use before the declaration; assignment after it (6 rows, equal to main, previous head had 2). Synthetic owners (function expression, arrow, async): formal → body read emitted again. F6 seam reproducer (`(y, d = 0)`, `(y, d = () => y)`, `(y = 1)`, `({y})`, `(...y)`): synthetic still 0 formal rows, named equals main. Labelled declaration in a nested block, in a labelled block, in a `switch` case, in a nested function or arrow: unchanged from the previous head; only the outer read keeps a row. Node control: body-level labelled function shares the formal's binding (`[9,9]` through a closure and the body read); nested-block form leaves the outer read on the formal (`typeof` `number` outside, `function` inside). |
| B2 JSX member tags | **CLOSED** | `<this.Comp />`, `<ui.box />`, `<props.icon>…</props.icon>`, `<a.b.c />`, `<ui.box<string> />`: path rows back, Exact, `.jsx` and `.tsx`, opening, closing, self-closing; kill by a second write is honoured. `<img />` with formal `img`, `<svg:path />` with formals `svg` and `path`, `<foo-bar />` with formals `foo` and `bar`, `<t<string> />`: no row at the tag. `<Item>`, `<_c />`, `<$d />`, `<UI.Box />`, `<ctx.Provider>`: unchanged. Named owners equal main in all 8 named cases. |
| B3 enum member names | **PARTIAL** (stated reproducers closed) | Fenced now, named and synthetic, TS and TSX: `"y"`, `'y'`, `"y"`, `"\u{79}"`, `'\x79'`; two and three merged declarations; reversed order; `const enum`; an escaped enum name (`enum E`); a nested arrow in the initializer; a local `let` Def instead of a formal. TypeScript 5.9.3 resolves every fenced read to the `EnumMember`. Negatives keep their rows and TypeScript resolves them to the `Parameter`: unrelated member, unrelated string member, `"yy"` / `"y "` / `" y"`, two different enums, same name in an inner or outer block, same name in a nested arrow, module-level enum of the same name, a read after the enum on the same line. Not fenced: R10-2 (switch clauses), R10-3 (computed literal keys). New over-fence: R10-1. |
| B4 comments inside a parenthesised `eval` | **CLOSED** | Refused now: `(/* c */ eval)`, `( // c ⏎ eval)`, nested and doubled comments, `(/* c */ eval as any)`, `(/* c */ eval!)`, `(/* c */ eval satisfies Function)`, `(/* c */ <any>eval)`, `(/* c */ eval)`, the same inside a nested arrow; local rows survive in every refused case. Still admitted: `(0, /* c */ eval)`, `(/* c */ 0, eval)`, `(/* c */ obj.eval)`, `(/* c */ obj).eval`, `(/* c */ eval)?.()`, `new (/* c */ eval)()`, `((0, eval) /* c */ as any)`, `(/* c */ evalx)`, `(/* eval */ g)`. Node control: block and line comment forms return 2 (direct), sequence and optional forms return 1. Named owners equal main in all 52 named cases. Residual: R10-4. |
| B5 test gaps | **CLOSED** | ERROR-ancestor walk has a test and mutant PD-115; `<svg:path />` now uses a colliding formal `svg`; bare enum member and `asserts v` have row assertions. My probes of the four fixtures match the asserted rows. S2 notes what is still unpinned. |
| CACHE_VERSION and registries | **consistent** | `src/cpg_cache.rs` 114 with its doc line and pin test; `PD-11` and `P2-M11` both mutate `114 → 113` against the same pin test; every new anchor (PD-110 to PD-115) and both cache anchors occur exactly once in the head source. |

## Findings

### R10-1 — WRONG / IMMATERIAL — reproduced — the merge arm treats two different enums as one when neither name decodes

**Input → incorrect result:**

```ts
function q(y: number) {
 enum yield { y = 1 }
 enum await {
  z = y }
 use(y);
}
```

Main and the previous head: `q| y(param) → y@L4` and `→ y@L5`, both NameOnly(cfg_incomplete). Head: the L4 row is gone. TypeScript 5.9.3 accepts the file without a diagnostic on these lines and resolves the L4 read to the `Parameter`. This is a row main has and the head loses, in a named owner, created by this delta. The synthetic form loses the same row.

**Mechanism (static):** the sibling test compares `js_ts_decoded_identifier(sibling name) == js_ts_decoded_identifier(own name)` as two `Option`s. That decoder returns `None` for a name the JavaScript grammar does not parse as an identifier, so `None == None` makes `yield` and `await` the same enum. `enum let` decodes, and one undecodable name beside a normal one is handled correctly (both probed).

**Fix (recommended):** compare the raw name text when neither contains a backslash, as `js_ts_fence_pattern_binds` already does, and decode only escaped names; a twice-declared `enum yield` must stay merged (it is today, by the same accident). **Alternative:** record and disclose.

**When it would not apply:** it needs two enums named from {`yield`, `await`} in one statement list, one with a member that shadows a formal. I rate realistic incidence nil; the corpus head-to-head difference set is empty.

### R10-2 — WRONG / IMMATERIAL — reproduced — enum declarations merged across `case` clauses are not fenced

**Input → incorrect result:** `reg(function (y: number) { switch (k) { case 1: enum E { y = 1 } case 2: enum E {⏎ z = y } } use(y); });` → head emits `y(param) → y` at the initializer, **Exact**. TypeScript resolves that read to the `EnumMember` of the first declaration. Named owner: the same row exists on main (so not a loss). Unchanged from the previous head.

**Mechanism (static):** the merge scan looks at siblings under the declaration's parent node. Two clauses of one `switch` share a lexical scope but have different `switch_case` parents.

**Fix:** when the parent is `switch_case` / `switch_default`, scan every clause of the enclosing `switch_body`. **Alternative:** record. **When it would not apply:** no measured incidence; a local enum declared twice inside one `switch`.

### R10-3 — WRONG / IMMATERIAL — reproduced — computed literal enum keys are not fenced

**Input → incorrect result:** `enum E { ["y"] = 1,⏎ z = y }` and ``enum E { [`y`] = 1,⏎ z = y }`` inside a callable with formal `y` → head keeps `y(param) → y` at the initializer, NameOnly. TypeScript accepts both without a diagnostic and resolves the read to the `EnumMember`. Unchanged from the previous head and from main.

**Mechanism (static):** the key match handles `string` and identifier kinds; `computed_property_name` falls to `None`.

**Fix:** peel a `computed_property_name` whose only child is a string or a substitution-free template. **Alternative:** record. **When it would not apply:** no measured incidence.

### R10-4 — WRONG / IMMATERIAL — reproduced — an HTML-like comment inside the parentheses still hides direct `eval`

**Input → incorrect result:** `reg(function (f) { (<!-- c⏎ eval)("f = 2"); use(f); });` in a `.js` script → head emits `f(param) → f`, Exact. Node runs this as direct eval (the formal becomes 2). The file parses without ERROR nodes. Unchanged from the previous head.

**Mechanism (static, inferred from the grammar; I did not dump the tree):** tree-sitter-javascript has a separate `html_comment` extra; the new filter drops only `comment`.

**Fix:** skip every extra (`n.is_extra()`) rather than the one kind. **Alternative:** record. **When it would not apply:** sloppy scripts only; not TypeScript; no measured incidence.

### S1 — SMELL / IMMATERIAL — static — the SPEC follow-up sentence is broader than the code

SPEC "R10 follow-ups" says "Same-name enum declarations in the same lexical scope and file share member fences." The code implements "in the same statement list" (R10-2), for string and identifier keys only (R10-3). **Fix:** reword and list R10-1 to R10-4 there. Docs only.

### S2 — SMELL / IMMATERIAL — static plus probe — the enum-name equality in the merge arm has no negative control

The R10 enum negatives are an unrelated member, an unrelated member in a merged declaration, and the same name in a nested block. None has two differently named enums in one scope, so a mutant that replaces the name comparison with `true` survives. The behaviour is right today (`enum A { y = 1 } enum B { z = y }` keeps its row on the head binary); it is the predicate R10-1 lives in. **Fix:** add that fixture to the frozen controls. Also unchanged from last round: `node.is_missing()` in the ERROR-ancestor walk cannot be true for an ancestor.

### Recorded, out of scope

- A label hides a nested function declaration from the CfgIncomplete downgrade: the new synthetic row for `lbl: function y(){}` is Exact where the unlabelled twin is NameOnly(cfg_incomplete). Main has the same split for named owners; the binding is the same in both.
- Labelled function declarations are an early error in strict code (modules, `"use strict"`, class bodies); main and head give them rows alike.
- `lbl: function* y(){}` is always a SyntaxError; the head now gives the synthetic owner a row for it.
- ``(/* c */ eval)`f = 2` `` (tagged template) is refused as direct eval; Node shows it is not. Conservative, and the uncommented form was already refused.
- A formal read only as the base of a member (`return props.icon`, `<props.icon />`) gets no row on main or head.

## Tests and mutants (priority 3)

| Fix | Row-level test | Fails on `96817370` | Negative control | Mutant |
|---|---|---|---|---|
| B1 | `r10_labelled_callable_body_rows_survive_without_a_seam` (full byte/grade rows, named and synthetic, 3 grammars, 2 label shapes) | yes | seam, nested block, unrelated name (frozen rows) | PD-110 disables the label peel |
| B2 | `r10_jsx_member_tags_keep_complete_path_rows` | yes | `<svg:path />`, `<img />`, `<foo-bar />` with colliding formals | PD-111 removes the identifier-kind test |
| B3 | `r10_enum_string_and_merged_members_fence_rows` | yes | unrelated member, merged unrelated member, nested-block same name | PD-112 (string decode off), PD-113 (merge off); name equality unpinned (S2) |
| B4 | `r10_eval_comments_refuse_formals_and_keep_indirect_rows` | yes | sequence, optional, member, `new`; local rows survive | PD-114 removes the comment filter |
| B5 | `r10_error_ancestor_walk_preserves_local_rows`, `r10_bare_enum_and_asserts_name_arms_have_rows` | no, by design (existing arms) | clean duplicate arrow refuses | PD-115 disables the ancestor proof |

"Fails on `96817370`" rests on the worker's `red-final-tests-corrected.log` (2 pass / 4 fail) and on my own probes: for each of the four repair groups the previous-head binary lacks the asserted rows or has the extra row. Mutant results are the worker's `pd-mutants.log` (109/109 KILLED, PD-110 to PD-115 each KILLED) and `cache-mutant.log` (P2-M11 KILLED); I read them and did not re-run them. `green-final.log` in the evidence directory is an intermediate run (5 pass / 1 fail); the passing run is `green-complete.log` (6/6) and the full suite `nextest.log.stderr` (5247 passed, 1 skipped).

## Probe log

Expectation before each batch, then the result. 361 fixture files (14 of them the two reviews' original reproducers verbatim), each run on main, the previous head and the head; rows compared by owner, path, bytes, grade and doubt.

| Batch | Expected if the fix is right | Would falsify | Result |
|---|---|---|---|
| B1 (65 + 17 + 20 files, 4 Node cells) | named non-seam equals main; synthetic non-seam has formal → body read; seam synthetic 0 formal rows; nested-block only the outer read | a named case where head ≠ main; a formal row to a read bound elsewhere | as expected; no falsifier |
| B2 (50 + 12 files) | member-tag path rows return; intrinsic, namespaced, hyphenated tags no row; named equals main | a tag row for an intrinsic; a named difference | as expected |
| B3 (65 + 10 files, 19 TypeScript checker cells) | string and merged members fenced; every fenced read is an `EnumMember`; every kept read a `Parameter` | a fenced read that TypeScript binds to the parameter (over-fence); a kept read it binds to a member | over-fence found: R10-1; kept member reads: R10-2, R10-3 |
| B4 (104 files, 6 Node cells) | comment forms refused; indirect forms admitted; named equals main | an admitted form Node runs as direct eval | R10-4 |
| B5 (4 files) | rows as asserted in the two gap tests | a different row | as asserted |
| Navigation (25-file fixture repo) | `functions`, `repo-map`, `callers`, `callees`, `ego`, `nodes-at`, `symbol-spans`, `module-deps` byte-identical on main, previous head, head; no `<cb@` | any difference | identical; `call-stats` differs only in `dfg_labels` |

Inadmissible probes, fixed before any conclusion: my first navigation pass used unresolvable symbol seeds and a wrong `nodes-at` flag, so all three binaries printed the same error; re-run with location seeds and a definitions file. One fixture generator had a quoting error and produced nothing; re-run.

Alternative mechanisms I ruled out: a stale head binary (the worker's 728-file source manifest equals `git show fc96688e:<path>` for every entry, and the head binary shows the R10 behaviour on every original reproducer); for R10-1, "TypeScript also merges them" (it resolves the read to the parameter) and "any undecodable name triggers it" (one undecodable name beside a normal one keeps its row).

## What I did not check

- **F** — never opened. No `frontend-portal` access.
- No cargo build and no gate re-run: nextest, the six R10 tests, mutants, clippy, fmt, Tier-A matrix and quick, perf, MCP. I read the worker's receipts for the R10 tests, the suite total and the mutants only.
- That the frozen head binaries were built from the manifest source: I checked the manifest against the commit and the binaries' behaviour, not the build itself.
- The corpus certificate (579 admitted roots, empty head-to-head difference set) and the three-bucket Part A tables: read in MEASUREMENTS-prB-R10.md, not regenerated or re-scored this round.
- The evidence-rule changes (Part A, allowlist, bucket boundary, controller consumer) and the docs patch beyond the R10 follow-up paragraph: outside this delta.
- The 48 frozen control rows beyond their source list and row counts; the other 103 PD mutants and 120 lane-P mutants.
- Legacy-octal string keys, namespace and cross-file enum merging (declared out of model), and performance of the sibling scan (it runs only for a reference inside an enum body within an owner).
- Whether main's current CACHE_VERSION or another open branch collides with 114 at merge time.

Scratch fixtures in the clone are deleted; the clone is clean on `review-r9` at `5ddbf1d6`. The only git write was `git fetch origin`. Node ran only my own snippets and the pinned TypeScript 5.9.3 library; no corpus package was executed.
