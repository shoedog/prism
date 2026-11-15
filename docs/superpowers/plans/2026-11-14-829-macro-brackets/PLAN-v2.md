# #829 implementation plan (v2)

Design: `DESIGN-v2.md` in this directory (read sections 1, 5 and 6 first; `PLAN.md` and `DESIGN.md` are history).
Worktree: `/Users/wesleyjinks/code/prism-829`, branch `fix/829-macro-brackets`. Work only there. Do not push without the controller.

**Goal of the task (state it in every review brief):** rules U and D for `.c` and C++ owners must not drop a def→use pair that the same rules keep on the compiler's token stream. The mechanism is a certificate: an owner keeps U and D only when every macro invocation in it provably expands to a well-nested token string in place, and the owner is not inside an argument list that starts outside it. Refusing an owner is always sound; its cost is recall, which is reported, not gated.

**Owner decisions (2026-11-14, `DECISIONS.md`, final):** OD-A candidate 10 with shielded parentheses; OD-B the 31-name list; OD-C close the two-file form of A-span in this PR (the hidden-opener half of G10: `paren_open` in tasks 4 and 5, the test in task 8, the rows in task 1); OD-D the interval cascade is accepted. Design review by gpt-6.1-sol before implementation (cap 2 rounds); then this plan.

**Shape: one PR, four commits**, reviewed commit by commit. The must-keep rows only turn green when all four checks exist, and CI must be green to merge, so the gates cannot be split across merged PRs without ignoring rows in between.
- C1 (tests and harness only): tasks 1 and 2.
- C2 (census facts; no edge changes): tasks 3 to 5.
- C3 (G7, G8, G9): tasks 6 and 7.
- C4 (G10, cache version, docs, gates): tasks 8 to 10.
Fallback, decided by the controller: if the review of C4 has not converged after two rounds, move C4 to a stacked PR; the enclosing-group rows (`enc*` generator families, the round-3 enclosing tests, inline cases 29 and 31, `hull2`) are then `#[ignore = "#829 G10 (stacked PR)"]` in the first PR and un-ignored in the second.

**Size (guidance, not a cap).** Product code about 700 to 800 lines: `macro_census.rs` +75, new `macro_census/brackets.rs` about 245, new `data_flow/build/c_file_parens.rs` about 270, `c_prune.rs` +60, `lexical_prune.rs`/`file.rs`/`cpg_cache.rs` about 20. New tests about 900 lines, harness about 200. `macro_census.rs` is at 1,458 weighted lines: over 1,000 and under the 2,000 ceiling; the PR says so, and its split is a separate move-only PR. No file may cross 2,000.

**Caps (declare before dispatch).** Implementer: at most 2 repair attempts per failing task before reporting back. Review: 3 rounds for the PR (the budget belongs to blocker #829); at the cap classify (converging: fold and continue with one line of disclosure; open-class: park and escalate). A must-keep row that candidate 10's rule admits is not a repair item: stop and report (the design is wrong there).

## Rules for the implementer

- Tests first. For each task write the tests, run them, and keep the failing output (RED) in your notes before writing product code. Every behaviour gets a test that fails on the pre-change code and a negative case per new code path.
- Run cargo through the memory guard, targeted tests only: `~/prism-evidence/tools/memguard.sh 8 cargo test -p prism --lib -- <filter>`. The full suite and the mutation gate run in GitHub CI or a cloud session, not locally. Never `pkill -f cargo`.
- No file containing `#define` goes under `tests/fixtures/` or under any C-like extension in the repository (the loader would read it into the census of prism's own tree). Macro inputs live as Rust strings; oracle data files are `.jsonl`.
- Do not change the set `spelled_identifiers` returns, nor the set `macro_call_arguments` returns. The new tokens must not consume or split identifier text.
- Do not touch `eval/secbench` (sparse-checkout it out if the worktree has it).
- clang is an authoring tool only: the Rust tests read frozen data (`scripts/dfg_prune/XARM.md`, "Workflow").
- A row or test that fails for a reason this plan does not name is reported, never re-baselined.

## Task 0. Keep the base binary

`target/release/prism` is the base binary of the saved L3 sweeps (tag `base`) and of every frozen verdict. Before any product change: `mkdir -p .scratch829/bin && cp target/release/prism .scratch829/bin/prism-base`. Tasks 1 and 2 label rows with this binary (`PRISM=.scratch829/bin/prism-base`); task 9 compares against it.

## Task 1 (C1). New rows from the design's probes

Sources: DESIGN-v2 section 11 (originals in `.scratch829/redesign/probes/`).
1. Inline cases, C in `c_prune_inline_cases.rs` and C++ twins in `cpp_prune_inline_cases.rs` (next free case numbers, one census per case, every statement of an owner on its own line; the C++ twins read `sink(x + 1)` or `return x + 1`): `bodysplit`, `bodyfwd`, `deferred`, `shield_nested`, `outer_shield` (hand: kept) and `shield_safe` (hand: pruned).
2. Pair rows in a new `c_prune_brackets_v2_tests.rs` (through `c_prune_brackets_r3_support.rs`): `hull2` (C and C++); the hidden-opener rows `aspan` (three files `defs.h`, `impl.c`, `unity.c`; target `impl.c`, owner `f`, pair `x@4 -> x@9`; hand: kept), its C++ twin (the owner of DESIGN-v2 section 11), `aspan_sib` (hand: kept), a control with `BEGIN_IMPL` spelled only after `f` (hand: pruned) and `aspan3` (four files; hand: kept, assumption `A-span`). Each declares its translation unit: `Declared` gains `tu: Option<String>` (`unity.c`).
3. Harness (`xarm.py`): when a row declares a `tu` and its target is another file, the verdict is `UNJUDGED` with detail `target-not-a-translation-unit` (the file is never compiled alone, so judging it alone has no basis: DESIGN-v2 section 10); `overrides.json` then gives `aspan`, its twin and `aspan_sib` `assert: kept`, and `aspan3` `assert: pinned_hole` with `assumption: A-span` (`derive` honours an `assumption` in an override), each with its receipt name. `xarm_receipts.py` gets the four drivers (each `unity` program returns 42 exactly when the first def of `x` is live). Update XARM.md's "Receipt overrides" paragraph.
4. Run the XARM workflow (export, `label`, both `gen_obligations.py`, `oracle_report.py`).
Expected on base: the ten must-keep inline rows and both `hull2` rows CONTRADICTED with head dropping (red); `shield_safe` and the hidden-opener control CONFIRMED (guide); `aspan`, its twin and `aspan_sib` kept by receipt with head dropping (red); `aspan3` pinned (green: it asserts today's drop). `python3 scripts/dfg_prune/xscore.py run --pops l1 --l1-export <the fresh export directory>` must then show candidate B5 with at least one L1 violation (`bodysplit`) and B6 with at least one (`bodyfwd`, `deferred`), B8 with at least one (the `aspan` rows), and candidate 10 with none. If a new row is not CONTRADICTED on base, report it with the oracle output instead of editing the fixture until it is.

## Task 2 (C1). Generator family `nest` (offline)

The grammar cannot derive a body that holds an invocation (DESIGN-v2 section 2). In `xgen.py` add a family `nest` at the statement and guard sites: two use sites `A;` and `B;` around `x = 2;` as in `stmt`/`guard`, with macros
- inner `K(p, q)` and `K2(p, q)` whose bodies come from the enclosing-macro fragment set (parameters, `p;`, `q;`, `if (c)`, braces);
- outer object-like `A`, `B` whose body is one of: `K(<fragments>)`, `ID(K <N>)`, `K(<N>)`, `one(K(<fragments>))`, `ID(one(<N>))`, where `<N>` is a third macro with a fragment body and `ID(v) v`, `one` a declared function.
Effect classes for the key: add `NEST_HELD` (a comma inside braces inside the inner group), `NEST_FWD` (a macro with a comma outside parentheses inside the group), `NEST_SHIELD` (the group is inside `one(..)`).
Run bound 3 for the family (`xgen.py run --families nest --bound 3`), then `keys`, `freeze`, `verify`; add the family to `c_prune_generated_rows.rs`. Budget: two hours of machine time, one run; report the candidate, compile and verdict counts as in `L3-BASE.md`.
Gate: `xscore.py run --pops l2 --l2dirs <the nest run>` must show 0 MUSTKEEP admitted for candidate 10. If not, stop and report the key and its minimal program: that is a design finding.

## Task 3 (C2). Tokens

`macro_census.rs`: `Tok` gains `LBrace`, `RBrace`, `LBracket`, `RBracket`, `Comma` and `Digraph` (any of `<%`, `%>`, `<:`, `:>`); `tokenize_seq` pushes them only when `seq` is `Some`. `macro_call_arguments` ignores the new tokens (its returned set must be byte-identical: assert it on the existing fixtures before and after). `%:` and `%:%:` keep their meaning.
Tests: token sequences for `{ } [ ] ,` in code, inside string, character and raw-string literals (no token), inside comments (no token), digit separators in both dialects, each digraph.

## Task 4 (C2). Body scan and raw facts

New `src/call_graph/c/macro_census/brackets.rs`: `scan_body(seq: &[Tok], params: Params) -> BodyFacts { unbalanced, loose_comma, held_param, held_after, fwd_param, fwd_after, paren_open }`, exactly the automaton of DESIGN-v2 5.1 (`Params::Known(&set)` for a parsed definition, `Params::Unknown` for a census text: every identifier is then a parameter root and no identifier is excluded from the forwarded names). Points that are easy to get wrong:
- The root of a `(`: `Param` after a parameter, `__VA_ARGS__` or the `)` of a `VaOpt`; `Name(t)` after any other identifier `t` except `__VA_OPT__`; the root of the paren just closed when the previous token is that `)`; `None` for the first token of the body, an operator, a literal.
- A held comma (the innermost non-`VaOpt` entry is a brace or bracket) marks every paren on the stack by its root, at any depth; a comma whose innermost entry is a paren marks nothing.
- Forwarded names: identifiers that are not parameters, recorded for every rooted paren on the stack; `__VA_ARGS__` and `__VA_OPT__` are not names.
- A digraph sets `unbalanced` and nothing else.
- `paren_open` comes from a second, parentheses-only stack (`(` and the `(` of `__VA_OPT__` push; `)` pops when the stack is not empty; braces and square brackets are ignored): set when it is not empty at the end. `) (` has it, `if (c) {` does not, `f((T[]) {` has it.
`RawDef` gains the seven fields; `entry()` fills them from a pass whose body is `ok` (a pass that is not `ok` contributes none); `merge` ORs the booleans and unions the sets; `Parsed` records a named variadic, which sets `loose_comma` and `held_param`. `collect_permissive`: tokenize the text after the name in both dialects with `Params::Unknown` and OR the results; a line no dialect reads sets `unbalanced`, `loose_comma`, `held_param` and `paren_open`; `ident...` in a leading parenthesis group sets `loose_comma` and `held_param`.
Tests (`macro_census_brackets_v2_tests.rs`), each with its negative, asserted on the closed facts of task 5 where the root must be resolved: `arg_split` for `K({ a, b })` with `K` a macro, `p({ a, b })` with `p` a parameter, `K(x)({ a, b })`, `K((  { a, b } ))`, `K(g(1), { __VA_ARGS__ })`; not for `f((T){ a, b })` and `foo_impl((struct o){ __VA_ARGS__ })` with `f`, `foo_impl` no macros (the named-argument idiom stays usable), `((T){ a, b })`, `= { a, b }`, `K(a, b)`, `K((a, b))`; `loose_comma` unchanged from the tentative tests; `H(INIT)` and `ID(F BR)` forward `INIT` / `BR`, `INIT` outside a group and a parameter do not; `__VA_OPT__(,)` and `__VA_OPT__(K)(a, { b })`; named variadic; permissive twins (`fn({ a, b })` in a census text is split whatever `fn` is); an unreadable permissive line; `paren_open` for `W3(`, `for (`, `) (`, `f((T[]) {`, and not for `if (c) {`, `}`, `)`, `g(a)(b)`, `__VA_OPT__(,)`.

## Task 5 (C2). Closure, exported facts, digest

`close()`: `unbalanced*`, `loose*`, `split`, `arg_split*` as in DESIGN-v2 5.1 (compute `loose*` first; `split(M)` resolves the roots in `held_after` and `fwd_after` against the macro set and reads `loose*` of the forwarded names that are macros). `paren_open*` is the plain OR over reach. `MacroFacts { unbalanced, loose_comma, arg_split, paren_open }`; `arg_split_count()`, `paren_open_count()`. Digest tag `c-macro-census-v3`, flag bytes `[control, poisoned, unbalanced, loose_comma, arg_split, paren_open]`.
Un-ignore the 78 quarantined census tests (DESIGN-v2 section 10 table). Named changes only: `g_sol_11_permissive_unreadable_line_with_only_a_comma` expects `(true, true)`; digest tag `v3`. Add: `arg_split` through reach (`#define W HI`), through a forwarded name (`#define HI H(INIT)` with `INIT` loose two hops away), not set by a loose macro outside any group (`#define B { INIT }`), union over two definitions and across an `#undef`, digest and text-aggregate digest change when only `arg_split` changes, and when only `paren_open` changes (`#define X )` against `#define X (`: both `unbalanced`); `paren_open` through an alias chain and across an `#undef`.
After C2 no edge changes: `nav dfg-stats --edges` digests of redis and leveldb equal the base digests of `L3-BASE.md` (D4).

## Task 6 (C3). G7, G8 and the name list

`c_prune.rs`: `Off::MacroBrackets`, `Off::MacroBodySplit`, `Off::MacroArgSplit`, `Off::MacroArgLoose`, `Off::OwnerExposed` (remove the `MacroComma` stub); `BLOCK_MACROS` = the decided list (31 names: the 16 of `xscore_lex.BLOCK_MACROS` plus `xscore_candidates.BLOCK_MACROS_ADDED`). In `note_name`: a spelled name in `BLOCK_MACROS` -> `MacroBrackets`; in the census branch `facts.unbalanced` or `facts.free` holding a `BLOCK_MACROS` name -> `MacroBrackets`; `facts.arg_split` -> `MacroBodySplit`. The trace prints `off:<variant>`.
Un-ignore and update the two list pins (`block_macros_list_is_the_design_list`, `g_sol_01_test_names_are_the_production_list`).
Tests through `scan`: each reason with its control; the parameter-shadow control of the round-1 amendment (`#define PASS(pthread_cleanup_push) (pthread_cleanup_push)` stays on); a listed name that the repository also defines balanced is still off (the list is by name).

## Task 7 (C3). G9: the owner pass

`brackets.rs`: `owner_group_hazard(text: &str, macro_loose: &dyn Fn(&str) -> Option<bool>) -> Option<Option<GroupHazard>>` (`macro_loose(name)`: `None` no in-repo macro, `Some(loose_comma)`); outer `None` when a variant cannot be tokenized (the caller returns `Off::G2`, as for `macro_call_arguments`). It runs the pass of DESIGN-v2 5.2 over every text of `normalized_texts` in both dialects and returns the first hazard: `Split` (a checked held comma, a mismatched closer or a digraph inside a callee paren) or `Loose`.
Paren kinds: callee = after an in-repo macro name, or after the `)` of a callee paren; shielded = after any other identifier **and only when a callee paren is already open**; plain otherwise. Checked = inside a callee paren and inside no shielded paren.
Called from `scan` after the name loop; maps to `MacroArgSplit` / `MacroArgLoose`.
Tests, each a pair of refuse and admit: `F({ a, b })` / `F((a, b))`; `SF(({ a, b }))`; `AF({ a, b })` with `#define AF F`; `ID(F)({ a, b })` (chain); `ID(x)(y)({ a, b })`; `M(a[i, j])`; `H(INIT)` / `H(TIGHT)`; `H(BLK(AB))`; `ID(two(PAIR))` admitted and `ID(PAIR)` refused; `ID(one(K({ a, b })))` admitted and `one(K({ a, b }))` refused (a paren outside every callee paren shields nothing); `f({ a, b })` with `f` no macro admitted; the digit-separator row (`33 g2o03`) refused in one dialect.
Pin the parser guarantees with two tests: an owner with a digraph bracket, and one with `#if` inside a macro's argument list, are `G1Error`.
After C3, expected: every must-keep row green except the enclosing-group ones.

## Task 8 (C4). G10: the exposed-owner test

New `src/data_flow/build/c_file_parens.rs`, DESIGN-v2 5.3: `CFileParens::build(parsed, cuts, opens: &dyn Fn(&str) -> bool)` (`opens(name)`: the census has the name with `paren_open`) and `exposed(start_byte, end_byte) -> bool`.
- `cuts`: start and end byte of every node of `parsed.all_functions()`, sorted, deduplicated. Pieces between cuts are normalized separately (`translate`, `splice`, `strip_comments`: make the three `pub(crate)` or add a `pub(crate) fn` in `macro_census.rs` that returns the stripped variants of a piece) and scanned by a lenient scanner in this module: comments are already stripped; a string or character literal ends at its quote or at the line end; raw strings through `raw_string_end` (C++ dialect); a line whose first token is `#` or `%:` is a directive: it yields `If`/`Elif`/`Else`/`Endif` by its first word and no parenthesis. Pass `at_line_start` for a piece that starts mid-line (its first line is then no directive).
- Variants: trigraphs on/off and lenient/strict splices only when they change the text; dialects: `[cxx]` for `Language::Cpp`, `[c, cxx]` otherwise. The event list of a variant is the concatenation over pieces; record the event index of every cut.
- Hidden opener (DESIGN-v2 5.3 step 5): the scanner also reads identifiers of code (the census's identifier rule; not in directive lines, comments or literals); per variant keep the ordinal of the first piece that holds an identifier with `opens(name)`. Pieces are numbered from 0 and piece `k` ends at cut `k` (the last piece ends at the end of the file). `exposed` is also true when, in some variant, the first such piece has `k` at most the ordinal of the owner's start cut, that is, the spelling lies before the owner's start (a spelling inside the owner is in a later piece: G7 has it).
- Intervals and the test exactly as DESIGN-v2 5.3 (unclamped balance; hull at `Endif`, plus the pushed interval unless a plain `#else` was seen; `exposed = hi(a) > min(0, least lo before a) || least lo after b < hi(b)` in any variant; an unknown cut is exposed).
Wiring: a `OnceCell<CFileParens>` in `data_flow/build/file.rs` beside `declared_types`, passed through `LexicalFacts::new` to `c_prune::scan`; `scan` asks it last, after G9, and returns `Off::OwnerExposed`. Test helpers that call `scan` or `LexicalFacts::new` directly build the cell from the file's functions.
Tests: owner after `K(` at file scope; owner whose group was opened in a sibling owner; opened in a declaration initializer; closer after the owner with the opener hidden in a macro (`BEGIN_FNS .. )`); both halves under `#else` branches (`hull2`); balanced `#if`/`#else` branches that each open one parenthesis closed after `#endif` (not exposed); `#if 0` prose with an apostrophe and an unbalanced parenthesis (the owners after it are exposed: pinned cost); parentheses in `#define` bodies, `#if` conditions, `#error` text, comments, strings, a C++ multi-line raw string (not counted); a nested owner (local class method) at depth 0 inside its outer function; a method inside a lambda inside a call (exposed); the cascade of DESIGN-v2 5.3 (pinned). Hidden opener: a `paren_open` macro at file scope before the owner (exposed), through an alias, inside a sibling owner before it, under `#if 0` before it, closed by a literal `)` before the owner (still exposed: pinned cost); spelled only after the owner, only in a `#define` body, a comment or a string of the file (not exposed); a brace-only block macro before the owner (not exposed); the name-union cost pinned with a local balanced redefinition (`#define CASE(op) ..` in one function while another census input holds a `paren_open` `CASE`: the owners after the use are exposed).
After C4: every must-keep row green; the generator family tests green.

## Task 9 (C4). Cache version, relabel, corpus gates, docs

1. `CACHE_VERSION` 188 -> 189 with the changelog line and the pinned asserts (rebase first: main may have moved the number).
2. Re-run the XARM labelling with the head binary so `head_drops` and the derived assertions are current. Expected: `head_drops` false for every `kept` row; the pinned A-ext witness moves to a row head still drops (an unlisted external name); A-dialect has no pinned row left (`33 g2o03` is kept); the 21 A-inputs pins and the A-span pin (`aspan3`) unchanged; the three hidden-opener rows kept, `head_drops` false. Regenerate the TESTS.md block.
3. L3 with the head binary (expansion text is reused; about two minutes of prism runs per large corpus): `xarm_l3.py sweep <corpus> --out .scratch829/l3/<corpus> --tag fix --prism target/release/prism` for the six corpora, then `xarm_l3.py compare .scratch829/l3/<corpus> --base base --head fix`. Gates: D1 0 monotonicity violations (every base edge is in the head's output); D2 0 CONTRADICTED pairs still dropped (the 50 cpython pairs are restored); (a pair the head still drops that the oracle calls CONTRADICTED is triaged first for validity: the oracle's clang action only lexes, and lemma V of the design speaks of translation units the compiler accepts under the sweep's configuration; an owner admitted through a shielded position in a unit that does not compile there is no basis, anything else is a finding that stops the merge); D3 ledger reported in the PR by `Off::` reason, to be compared with the scorer's prediction (399 CONFIRMED pairs restored, 214 owners, of which 67 pairs and 106 owners by the hidden opener in three cpython files; a larger number is a finding to explain, not to tune away). D4: `parity-langs` for click, cobra, express, ripgrep (`--base-bin .scratch829/bin/prism-base --head-bin target/release/prism`): identical digests; `cache-modes` on redis `src/`: all four parity checks true.
4. Cost of G10: wall time of `nav --no-cache dfg-stats` on redis and cpython, base against head, three runs each, paired; report the delta. No threshold; a delta above the run-to-run spread is reported to the controller.
5. Docs: CLAUDE.md (text below); `scripts/dfg_prune/XARM.md` (the `nest` family, the `tu` declaration, the override `assumption`); `python3 scripts/file_size.py` on every touched file, numbers in the PR.
6. `mutants/c-macro-structure.json` (table below).

## Task 10. PR

Description: the goal sentence; the rule in five lines; the owner decisions taken; scores before (L4, DESIGN-v2 section 9) and after (task 9.3); the recall ledger; the file-size note for `macro_census.rs`; what was verified where and what was not (GCC, PostgreSQL headers, corpora beyond six); the disclosed limits (A-span as narrowed, the interval cascade, the name-union cost of the hidden opener, #831). Mark it "Refs #829, #813, #831".
Gates, in GitHub CI or a cloud session: full suite (`cargo test --workspace`), `cargo fmt --all --check`; mutation gate tier 1 from the merge base (`python3 scripts/mutgate/mutgate.py --since <merge-base> --scope fn --authoritative`) plus the new lane (`--lane mutants/c-macro-structure.json`). No tier-2 trigger: the change is in C/C++ pruning and the C census only. Tier-A is not triggered (no call-resolution, navigation or CPG-construction path changes).

## Mutation lane `mutants/c-macro-structure.json`

`lib_test_prefixes`: `["call_graph::c::", "data_flow::build::"]`. One mutant per premise or branch; the anchors are written against the final code. Each selector must name exactly one test.

| id | mutation | killed by |
|---|---|---|
| BR-unbalanced-off | `scan_body` never sets `unbalanced` on a mismatched closer | `u02_close_brace` |
| BR-end-stack | a non-empty stack at the end is not `unbalanced` | `u01_open_brace` |
| BR-digraph | a digraph token does not set `unbalanced` | `u13_digraph_brackets` |
| BR-closure | `unbalanced` not propagated over reach | `u14_closure_over_referenced_macros` |
| LC-held-is-tight | a comma under a brace does not set `loose_comma` | `l02_brace_held_comma` |
| LC-vaopt-opaque | the `__VA_OPT__` paren counts as a paren for tightness | `l05_va_opt_parenthesis_is_transparent` |
| AS-held-off | a held comma marks no paren | the `K({ a, b })` fact test of task 4 |
| AS-root-param | a paren after a parameter gets no root | the `p({ a, b })` fact test |
| AS-root-chain | a paren after the `)` of a rooted paren gets no root | the `K(x)({ a, b })` fact test |
| AS-root-any | `split` takes every name in `held_after` as a macro | the `foo_impl((struct o){ __VA_ARGS__ })` negative |
| AS-fwd-off | `split` ignores the forwarded names | the `H(INIT)` closure test of task 5 |
| AS-fwd-outside | names outside rooted parens are forwarded too | the `#define B { INIT }` negative of task 5 |
| AS-permissive-param | a census text's identifiers are not parameter roots | the permissive `fn({ a, b })` test |
| G7-list | the `BLOCK_MACROS` spelling check removed | the listed-name `scan` test of task 6 |
| G7-wrapper | the `free` ∩ `BLOCK_MACROS` check removed | `g_terra_01` wrapper row test |
| G8-off | `arg_split` not consulted in `note_name` | the `bodysplit` inline row |
| G9-held-off | the checked held comma does not refuse | the `F({ a, b })` owner-pass test |
| G9-loose-off | the checked loose macro does not refuse | the `H(INIT)` owner-pass test |
| G9-shield-outside | a paren after an identifier shields even outside a callee paren | the `one(K({ a, b }))` owner-pass test |
| G9-chain | a chained group is not a callee paren | the `ID(F)({ a, b })` owner-pass test |
| G10-open-before | the first half of the exposed test removed | the file-scope `K(` test of task 8 |
| G10-close-after | the second half removed | the hidden-opener test of task 8 |
| G10-hull-first | `Endif` takes the first branch's end instead of the hull | the `hull2` test |
| G10-directive | parentheses of directive lines are counted | the `#define`-body negative of task 8 |
| G10-hidden-off | the hidden-opener half removed | the `aspan` row |
| G10-hidden-after | a spelling after the owner counts too | the spelled-after control of task 8 |
| G10-hidden-directive | identifiers of directive lines count | the `#define`-body hidden-opener negative of task 8 |
| PO-brace | braces take part in the parentheses-only stack | the `if (c) {` negative of task 4 |
| PO-closure | `paren_open` not propagated over reach | the alias-chain test of task 5 |
| DG-flags | the digest omits `arg_split` | the digest twin of task 5 |
| DG-flags-open | the digest omits `paren_open` | the `)` against `(` digest test of task 5 |

## CLAUDE.md text (task 9.5)

Replace the sentence about external macros in the C rules paragraph ("Out of model (disclosed, owner D2) ...") by the extended D2 wording, and add after the G5/G6 gates:

> #829 (design `docs/superpowers/plans/2026-11-14-829-macro-brackets/DESIGN-v2.md`): an owner keeps U and D only when every macro invocation in it provably expands in place to a well-nested token string. The census records per macro, closed over the macros a body names, `unbalanced` (the replacement list does not nest in `()`, `{}`, `[]`; any digraph bracket counts), `loose_comma` (a comma or `__VA_ARGS__` outside parentheses or held by `{}`/`[]`), `paren_open` (a `(` that no later `)` of the list closes, parentheses alone) and `arg_split` (a body paren group that follows a parameter, an in-repo macro name or a chained `)` holds a comma held by `{}`/`[]`, or names a macro with `loose_comma`; in a census text that is not parsed C/C++ every identifier counts as a parameter); a named variadic and an unreadable line of a census text fail closed. An owner is off (`Off::MacroBrackets`) when it spells an in-repo macro with `unbalanced`, a name of the `BLOCK_MACROS` list (31 well-known block macros of POSIX threads, CPython, NumPy and PostgreSQL) or an in-repo macro that names one; (`Off::MacroBodySplit`) when it spells a macro with `arg_split`; (`Off::MacroArgSplit`, `Off::MacroArgLoose`) when, inside the argument list of an in-repo macro in its own text (chained groups included) and outside a function-call parenthesis opened inside that list, a comma is held by `{}`/`[]` or a macro with `loose_comma` is named; (`Off::OwnerExposed`) when the file's own parentheses, read over every branch of conditional compilation, can leave a `(` open at the owner's start or a `)` after it unmatched, or when the file spells in code before the owner a macro with `paren_open` (the list it opens may be closed in the file that includes this one; the fact is by name over every definition, so a balanced local redefinition of such a name still refuses the owners after its use) (`c_file_parens.rs`; two conditionals with the same condition are not correlated, so a parenthesis opened under one and closed under the other refuses the owners around them). Out of model: a macro defined outside the repository acts as a function (its expansion nests and has no comma outside its own parentheses, it passes each argument on whole, it does not forward to an in-repo macro), except the listed names; a macro argument list that an external macro opens, or that is still open at the end of a file the owner's file includes, is assumed to close before the end of the owner's file (A-span); a top-level `;` in a body used inside a declaration is #831; census inputs prism does not read are #813.

Census paragraph: add `unbalanced`, `loose_comma`, `arg_split`, `paren_open` to the recorded facts and the digest tag `v3`; cache version 189.

## What is verified where

| check | where | when |
|---|---|---|
| targeted tests (`macro_census`, `c_prune`, `cpp_prune`, `c_file_parens`, `c_macro_census`) | local, memguard | every task |
| `xscore.py selftest`, `xscore.py run` | local (no prism, no clang) | tasks 1, 2 |
| oracle labelling, receipts, generator | local, clang, offline | tasks 1, 2, 9.2 |
| L3 sweep and compare, D4 parity, G10 cost | local, release binary, no measurement running in parallel | task 9 |
| full suite, fmt | GitHub CI | PR |
| mutation tier 1 + the new lane | cloud session | PR, final head |

Done means: every must-keep row green in Rust; the pinned and guide rows as task 9.2 expects; D1 and D2 at 0 on six corpora; the D3 ledger and the G10 cost in the PR; CI green; mutation verdicts all KILLED; the handoff refreshed.
