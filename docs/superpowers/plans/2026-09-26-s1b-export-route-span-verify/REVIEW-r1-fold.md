# Spec round 1 (Opus-5.5 FIX 9 WRONG / 4 SMELL; sol FIX 6 WRONG / 2 SMELL): fold record

Both reviews are saved outside the repo (`~/prism-evidence/s1b/reviews/spec-r1-{opus,sol}.md`). Both reproduced the
controls, the row-diff hashes and the budget. The WRONGs are one family, as both reviewers said: binding and write
forms the collector missed, several of which the auditor missed too. Following both reviewers' convergence advice
(and the controller's structural fold), this fold does not patch probe by probe. It (1) rewrites SPEC §3.1 as an
enumerated table keyed to ECMA-262 static semantics and TypeScript's declaration spaces, (2) adds a fail-safe that
refuses any binding scope holding a node kind the table does not classify, with the classification derived from the
pinned grammars by a probe (`probes/grammar_closure.py`), and (3) rewrites the auditor against the same table and
adds one pinned control per table entry and per reviewer input, in both grammars (C115–C151).

The owner's r1 decisions (SPEC §0) change several dispositions: **E5** keeps may-call rows at base (so a written
binding is no longer refused), **E6** takes the narrower sealed-error rule, and **E7** splits the namespace fallback.

Every row below is re-measured on prototype v8 (PLANNING-PROBES Q24–Q35).

## Opus

| # | Finding | Disposition | Where | Evidence |
|---|---|---|---|---|
| W1 | `var` in class static blocks and TS namespace bodies treated as block-lexical | **Fixed.** The body block of `class_static_block`, `internal_module` and `module` is a var scope (hoisting stops at it; its own `var`s hoist to it) | SPEC §3.1 T2, T4 | C115, C116, C117: Exact → the static-block / namespace arrow; drop for the non-callable `var` |
| W2 | `import f = M.g` (`import_alias`) never a declaration | **Fixed.** `import_alias` declares its first named child | §3.1 D5 | C141: Exact to a nested decoy → drop |
| W3 | Write scans miss TS assertion-wrapped targets | **Fixed in the shared pattern walker** (`as`, `satisfies`, `!`, `<T>` unwrap), so both scans see them. Under E5 such a binding is may-call, so its rows now stay base (the fix changes classification, not edges) | §3.1 W1; shared fix F1 | C142: base (Exact ×2) kept. Shared-fix blast radius on base: X 0, F 7, R 0, T 2 rows, all wrong R5 edges removed (Q27) |
| W4 | R4 filter span-only; `JsTerminal` drops `local` | **Fixed as option (a).** `JsTerminal` carries the registered name; R4 matches `(name, span)` before any same-name lookup, so a proven binding whose callable has another name (`const f = function g(){}`) now binds | §3.6 | C118: Exact → `g` (was the Pattern-2 `o.f`). Adds 10 right T edges (import shadowed by an enclosing function), audited 10 / 10 (Q31) |
| W5 | Hoisted `var` initialized inside `catch (f)` or `with` | **Fixed.** Such a declarator records its catch clause or `with` body as a non-callable marker (Annex B.3.4 for catch; `with` for any name) | §3.1 D7 | C119, C120: Exact ×2 → drop |
| W6 | TS type-space declarations shadow values | **Fixed.** Type-space forms (`interface`, `type`, `import type`, type-only specifiers) declare nothing; value forms (`enum`, `namespace`, `declare`) still do | §3.1 D6 | C143, C144, C146: base Exact kept (v7 dropped them) |
| W7 | Parameter initializers ignore implicit `arguments` | **Fixed.** A non-arrow function's parameter scope and body scope both declare `arguments` (non-callable) | §3.1 T2, T3 | C121: drop inside a function, Exact from a module-scope arrow |
| W8 | Removing `formal_parameters` survives the controls | **Fixed.** Added B-10b (C122: `function f(g, a = g())`) and split B-M2 into B-M2a (no parameter scope) and B-M2b (no jump over the function) | §7 | C110 kills B-M2b; C122 kills B-M2a |
| W9 | No-parentheses B3 removed 16 right X rows; taxonomy counted them may-call | **Fixed.** B3 unwraps parentheses, TS assertions and assignment chains; `taxonomy.py` marks M only from the auditor's may-call verdict (a written binding or a call with a direct function argument), never from the declaration keyword. The r1 README headline ("0 right edges removed outside parse recovery") was false and is corrected | §3.1 B3; `probes/taxonomy.py` | C123 keeps its edge; the 21 X `woff2` rows v1 removed for this reason are unchanged from base in v8 (Q29) |
| S1 | Naming S1b-a/b vs S1b-1/2/3 | **Fixed.** One naming, S1b-1…S1b-4 (§0 E1 now has four sub-slices) | all files | – |
| S2 | S1b-1 budget headroom | **Re-capped with evidence** (SPEC §9): the v8 prototype is 1,079 src lines; the split moves the collector into its own sub-slice | §9 | Q34 |
| S3 | §8 cited missing `S1b-2-E4a-{R,T}.json` | **Fixed.** E4 is decided (admit), so the expected row-diffs are regenerated for the owner's answers only | `probes/expected/` | Q33 |
| S4 | Labelled function declarations became Annex-B markers | **Fixed** (with sol W5): `labeled_statement` is transparent | §3.1 D3 | C127 |
| E7 view | Stem match is not module identity | **Adopted by the owner (E7 split).** Sibling-extension match is Exact; any other stem fallback is NameOnly | §3.4 | C131, C132 NameOnly; C133 drop; C148 Exact; T's 24 rows stay Exact, F's 4 become NameOnly (Q30) |

## sol

| # | Finding | Disposition | Where | Evidence |
|---|---|---|---|---|
| W1 | Destructuring defaults escape declaration and write detection | **Fixed in the shared pattern walker** (`object_assignment_pattern` binds its `left`). The declaration case now refuses; the write case is may-call under E5 and stays base | §3.1 P1; shared fix F2 | C124: drop; C125: base kept |
| W2 | S1b-1's base write scan removes a correct export edge (arrow-parameter miss) | **Fixed in the shared parameter collector** (the single-arrow `parameter` field), the cheaper of sol's two options; the module scope keeps one scan. Measured: the scoped scan and the fixed base scan give identical rows on all four corpora (Q28) | shared fix F3 | C126: Exact kept |
| W3 | `var` in class static blocks omitted | Same fix as Opus W1 | §3.1 T4 | C116 |
| W4 | `formal_parameters` treated as one environment: implicit `arguments` and parameter decorators | **Fixed.** Implicit `arguments` as Opus W7; a decorator is resolved where its class is defined (the walk jumps from the decorator to the class's parent) | §3.1 T3, J2 | C121, C145 |
| W5 | Annex-B marker rejects labelled function declarations | **Fixed** (labelled statements are transparent) | §3.1 D3 | C127 |
| W6 | R3 keeps an incomplete namespace-qualifier guard | **Fixed in S1b-4.** The qualifier's visible binding must be exactly the file's one namespace `import_statement` at module scope, clean, sealed and unwritten; otherwise R3 does not apply to it | §3.4 | C128, C129, C130: no `import_qualified` edge |
| S7 | Naming | As Opus S1 | – | – |
| S8 | Generator function expressions neither supported nor excluded | **Pinned non-goal.** They are not indexed (`function_node_types`), so a binding to one refuses as `unindexed` and base behavior (no edge) is unchanged | §2, §7 | C134 |
| E7 view | Bare and non-sibling stem matches are false identity | Adopted (owner E7) | §3.4 | as above |

## Found during the fold (not raised by a reviewer)

| # | Issue | Disposition | Evidence |
|---|---|---|---|
| N1 | TS `using x = e` parses as an assignment (anonymous `using` token), so the v7 collector read it as a write | `using` / `await using` are lexical declarations and not writes | C114 (both rows now as the table says) |
| N2 | E5's "call-wrapped" class must require a direct function argument; otherwise `using h = res()` or `const x = make()` would keep a wrong base target | M1 is "a declarator whose value is a call with a direct function argument (a Pattern-3 over-named argument)"; other calls hold no callable | SPEC §3.1 M1 |
| N3 | Escaped identifiers (`var \u0066`) are invisible to raw-spelling comparison | B0 refuses a scope holding any escaped identifier | C137 |
| N4 | The first narrow-E6 draft also required the erroneous function's text not to spell the name; C151 showed that refuses on mere references, and the file-level brace condition already fixes every sealing node's extent | The mention check was removed before any corpus run | addendum 6; C149–C151 |
| N5 | The v8 namespace route relabelled 1,176 T drop reasons (`ImportExternal` ↔ `UnknownName`) with no edge change | Zero candidates keep base's reasons | Q30 |

## Where the planner and the reviewers differ

- **E5 and C21.** S1 SPEC §12 item 1 listed C21 (`export let A = …; A = Other`) as a D4 false Exact for S1b. The owner's E5
  keeps written bindings at base, and a closed syntactic rule cannot exempt the export route without a second rule.
  The packet applies E5 on every route, so C21, C77 and C73 stay base. This is flagged for the owner as **E5b**
  (§0), with 0 corpus prevalence.
- **Parse recovery (E6).** Both reviewers endorsed refusing on any scope error; the owner chose the narrower rule.
  The packet implements it as a closed predicate (SPEC §3.1 B1) and measures it: it keeps F 7 and T 12 right rows,
  risks 0 (every kept row audited right), and still refuses F 4 and T 41 right rows whose error is not inside a
  braced function or class. A further option (E6b, sealing type-space containers too) is measured in Q32 and left to
  the owner, because angle-bracketed type nodes do not have the brace guarantee the closure argument rests on.
