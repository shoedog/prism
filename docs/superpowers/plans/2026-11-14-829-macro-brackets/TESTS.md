# #829 test matrix: base outcomes

Base = head `680c7438` plus the compile stubs of this commit (`unbalanced` / `loose_comma` always false; `Off::MacroBrackets`, `Off::MacroComma`, `Off::MacroArgSplit` never produced; empty `BLOCK_MACROS`).
Command: `~/prism-evidence/tools/memguard.sh 8 cargo test -p prism --lib -- macro_census_brackets c_prune_brackets cz1 c_prune_matrix cpp_prune_matrix`

Kinds: **S** soundness (a live pair is dropped), **C** conservative, **K** control (pruning must survive), **F** census fact (`true` rows are red; `false` rows are green guards against over-approximation), `-` infrastructure.

## Totals

- 75 plan rows: **59 RED, 16 GREEN** on base. The 16 green rows are the controls and `false` fact rows the plan predicted: U09, U10, U11, U12, U19, U21, U25, L03, L12, b10, b11, x5, a7, a8, y3, y4. Every outcome matches the plan's expectation.
- Extras beyond the 75: `block_macros_list_is_the_design_list` (RED), three mf controls (GREEN: `mf1/mf2/mf3 ctl`, the "same owner, no define: prunes" controls the plan asks for, kept as separate tests so the red row does not mask them).
- 81 distinct tests: 62 RED, 19 GREEN. The aggregate matrices `c_prune_matrix` and `cpp_prune_matrix` read the new obligation rows and are RED on base too (they go green with the implementation). All other `c_prune`, `cpp_prune`, `macro_census`, `c_macro_census`, `cpg_cache`, `dfg_codec` tests pass on base (233 passed in that run; the 64 failures are the 62 new red tests and the two matrices).
- Row count by table: M1 26, M2 19, M3 6 (mf1 to mf4, cz1, E01), M4 12, M5 12 (a1 is gate-only, see below).

## Deviations from PLAN.md (found by running the rows on base)

1. **b1, b2, b5, b12, b13 and a1 as specified are inadmissible: the baseline has no pair to keep.** All six use the scope-shift shape `{ int *x = 0; M_CLOSE; r = x; M_OPEN; }` with the pair "parameter `x` -> `r = x`". The matrix harness reports `inadmissible missing baseline pair` for them, and so does an unpruned run of the release binary (`c1.c` with `__asm__("")` added so the owner is off: the only edge is `x@5 -> x@7`, the inner declaration). The parser binds `r = x` to the inner `x`, the baseline never draws the parameter edge, and a removal-only rule cannot drop an edge that does not exist. The compiled program does read the parameter (`42`), so the shape is a baseline recall gap, not a drop; no pair row can be red or green on it, so the gate on these functions is pinned by E01 only. The DESIGN section 2 table's "pair dropped on head" entries for the C scope-shift probes (`c1.c` checked directly; `t1.c`, `e.c`, `p.c`, `a1.c` share the shape and the matrix harness shows the same for their rows) name a pair that is not in the baseline; the C++ `b1s`/`f1s` rows (a `x = 5` -> `x = 1` write-only drop) are unaffected.
   Fix applied, per the plan's own instruction ("fix the fixture"): each original function stays in the case, markers stripped, so E01 still pins its gate (`Off::MacroBrackets`, or `Off::MacroArgSplit` for `arg_split_scope`). For b1, b2, b5, b12 and b13 a **guard-shape twin** carries the row id: `int x = src(); if (cond) BEGIN; x = 2; END; sink(x);` (and `ENTER`/`LEAVE`, `LBD`/`RBD`, `pthread_cleanup_push`/`pop`, `Py_BEGIN_CRITICAL_SECTION`/`END`). The parser reads `if (cond) M;` as an expression-statement body, so `x = 2` dominates and the pair is dropped; the compiler reads `if (cond) { ...; x = 2; ... }`, so the pair is live when `cond` is false (the same mechanism as the plan's b3). These twins have a baseline pair and are RED on base. Their liveness is argued from the b3 mechanism and the `clang -E` expansions of DESIGN section 3 (C4, C6, C8); I did not compile them.
   **a1 stays gate-only.** I found no balanced-body argument-split shape with a baseline pair and a guard; the argument-split class has pair coverage in a2, a3 and the C++ row y1 (a live `x = 5` -> `x = 1` drop).
2. The extra test `block_macros_list_is_the_design_list` pins the exact 16 names of DESIGN 6.3 (MB-12 in the plan would otherwise only be caught through b12 and x6).
3. Row b6/b7 (paren and square macros) pass the admissibility check on base (the owner is on and the pair is dropped), so they stay as specified.

## Per-row outcomes

| row | test | file | kind | base | note |
|---|---|---|---|---|---|
| U01 | `u01_open_brace` | `src/call_graph/c/macro_census_brackets_tests.rs` | F | RED |  |
| U02 | `u02_close_brace` | `src/call_graph/c/macro_census_brackets_tests.rs` | F | RED |  |
| U03 | `u03_open_paren` | `src/call_graph/c/macro_census_brackets_tests.rs` | F | RED |  |
| U04 | `u04_close_paren` | `src/call_graph/c/macro_census_brackets_tests.rs` | F | RED |  |
| U05 | `u05_square_brackets` | `src/call_graph/c/macro_census_brackets_tests.rs` | F | RED |  |
| U06 | `u06_crossing_kinds_with_equal_counts` | `src/call_graph/c/macro_census_brackets_tests.rs` | F | RED |  |
| U07 | `u07_closer_first_with_equal_counts` | `src/call_graph/c/macro_census_brackets_tests.rs` | F | RED |  |
| U08 | `u08_function_like_opens_a_block` | `src/call_graph/c/macro_census_brackets_tests.rs` | F | RED |  |
| U09 | `u09_well_nested_bodies_are_balanced` | `src/call_graph/c/macro_census_brackets_tests.rs` | F | GREEN |  |
| U10 | `u10_brackets_in_literals_do_not_count` | `src/call_graph/c/macro_census_brackets_tests.rs` | F | GREEN |  |
| U11 | `u11_brackets_in_comments_do_not_count` | `src/call_graph/c/macro_census_brackets_tests.rs` | F | GREEN |  |
| U12 | `u12_cpp_raw_string_and_digit_separator` | `src/call_graph/c/macro_census_brackets_tests.rs` | F | GREEN |  |
| U13 | `u13_digraph_brackets` | `src/call_graph/c/macro_census_brackets_tests.rs` | F | RED |  |
| U14 | `u14_closure_over_referenced_macros` | `src/call_graph/c/macro_census_brackets_tests.rs` | F | RED |  |
| U15 | `u15_any_definition_taints_the_spelling` | `src/call_graph/c/macro_census_brackets_tests.rs` | F | RED |  |
| U16 | `u16_line_continuation_in_a_body` | `src/call_graph/c/macro_census_brackets_tests.rs` | F | RED |  |
| U17 | `u17_multi_line_comment_in_a_body` | `src/call_graph/c/macro_census_brackets_tests.rs` | F | RED |  |
| U18 | `u18_va_opt_parentheses` | `src/call_graph/c/macro_census_brackets_tests.rs` | F | RED |  |
| U19 | `u19_unterminated_literal_is_poisoned` | `src/call_graph/c/macro_census_brackets_tests.rs` | F | GREEN |  |
| U20 | `u20_define_inside_a_block_comment_counts_pinned` | `src/call_graph/c/macro_census_brackets_tests.rs` | F | RED |  |
| U21 | `u21_trigraph_body_is_the_existing_incomplete_belt` | `src/call_graph/c/macro_census_brackets_tests.rs` | F | GREEN |  |
| U22 | `u22_permissive_census_text` | `src/call_graph/c/macro_census_brackets_tests.rs` | F | RED |  |
| U23 | `u23_undef_does_not_clear` | `src/call_graph/c/macro_census_brackets_tests.rs` | F | RED |  |
| U24 | `u24_digest_covers_the_bracket_fact` | `src/call_graph/c/macro_census_brackets_tests.rs` | F | RED |  |
| U25 | `u25_stringizing_holds_no_bracket` | `src/call_graph/c/macro_census_brackets_tests.rs` | F | GREEN |  |
| U26 | `u26_function_like_body_only` | `src/call_graph/c/macro_census_brackets_tests.rs` | F | RED |  |
| L01 | `l01_top_level_comma` | `src/call_graph/c/macro_census_brackets_tests.rs` | F | RED |  |
| L02 | `l02_brace_held_comma` | `src/call_graph/c/macro_census_brackets_tests.rs` | F | RED |  |
| L03 | `l03_commas_directly_inside_parentheses` | `src/call_graph/c/macro_census_brackets_tests.rs` | F | GREEN |  |
| L04 | `l04_va_args_position` | `src/call_graph/c/macro_census_brackets_tests.rs` | F | RED |  |
| L05 | `l05_va_opt_parenthesis_is_transparent` | `src/call_graph/c/macro_census_brackets_tests.rs` | F | RED |  |
| L06 | `l06_gnu_named_variadic_fails_closed` | `src/call_graph/c/macro_census_brackets_tests.rs` | F | RED |  |
| L07 | `l07_statement_expression_declarators` | `src/call_graph/c/macro_census_brackets_tests.rs` | F | RED |  |
| L08 | `l08_comma_inside_square_brackets` | `src/call_graph/c/macro_census_brackets_tests.rs` | F | RED |  |
| L09 | `l09_closure_is_blanket_pinned` | `src/call_graph/c/macro_census_brackets_tests.rs` | F | RED |  |
| L10 | `l10_cpp_template_arguments` | `src/call_graph/c/macro_census_brackets_tests.rs` | F | RED |  |
| L11 | `l11_permissive_census_text` | `src/call_graph/c/macro_census_brackets_tests.rs` | F | RED |  |
| L12 | `l12_commas_in_literals_do_not_count` | `src/call_graph/c/macro_census_brackets_tests.rs` | F | GREEN |  |
| b1 | `row_b1` | `src/data_flow/build/c_prune_brackets_tests.rs` | S | RED | guard shape (see Deviations) |
| b2 | `row_b2` | `src/data_flow/build/c_prune_brackets_tests.rs` | S | RED | guard shape |
| b3 | `row_b3` | `src/data_flow/build/c_prune_brackets_tests.rs` | S | RED |  |
| b4 | `row_b4` | `src/data_flow/build/c_prune_brackets_tests.rs` | S | RED |  |
| b5 | `row_b5` | `src/data_flow/build/c_prune_brackets_tests.rs` | S | RED | guard shape |
| b6 | `row_b6` | `src/data_flow/build/c_prune_brackets_tests.rs` | C | RED |  |
| b7 | `row_b7` | `src/data_flow/build/c_prune_brackets_tests.rs` | C | RED |  |
| b8 | `row_b8` | `src/data_flow/build/c_prune_brackets_tests.rs` | C | RED |  |
| b9 | `row_b9` | `src/data_flow/build/c_prune_brackets_tests.rs` | C | RED |  |
| b10 | `row_b10` | `src/data_flow/build/c_prune_brackets_tests.rs` | K | GREEN |  |
| b11 | `row_b11` | `src/data_flow/build/c_prune_brackets_tests.rs` | K | GREEN |  |
| b12 | `row_b12` | `src/data_flow/build/c_prune_brackets_tests.rs` | S | RED | guard shape |
| b13 | `row_b13` | `src/data_flow/build/c_prune_brackets_tests.rs` | C | RED | guard shape |
| x1 | `row_x1` | `src/data_flow/build/c_prune_brackets_tests.rs` | S | RED |  |
| x2 | `row_x2` | `src/data_flow/build/c_prune_brackets_tests.rs` | S | RED |  |
| x3 | `row_x3` | `src/data_flow/build/c_prune_brackets_tests.rs` | S | RED |  |
| x4 | `row_x4` | `src/data_flow/build/c_prune_brackets_tests.rs` | S | RED |  |
| x5 | `row_x5` | `src/data_flow/build/c_prune_brackets_tests.rs` | K | GREEN |  |
| x6 | `row_x6` | `src/data_flow/build/c_prune_brackets_tests.rs` | C | RED |  |
| a1 | `e01_21_macro_args_c` | `src/data_flow/build/c_prune_brackets_tests.rs` | S | RED | gate-only: E01 `e01_21_macro_args_c` (see Deviations) |
| a2 | `row_a2` | `src/data_flow/build/c_prune_brackets_tests.rs` | C | RED |  |
| a3 | `row_a3` | `src/data_flow/build/c_prune_brackets_tests.rs` | C | RED |  |
| a4 | `row_a4` | `src/data_flow/build/c_prune_brackets_tests.rs` | C | RED |  |
| a5 | `row_a5` | `src/data_flow/build/c_prune_brackets_tests.rs` | C | RED |  |
| a6 | `row_a6` | `src/data_flow/build/c_prune_brackets_tests.rs` | C | RED |  |
| a7 | `row_a7` | `src/data_flow/build/c_prune_brackets_tests.rs` | K | GREEN |  |
| a8 | `row_a8` | `src/data_flow/build/c_prune_brackets_tests.rs` | K | GREEN |  |
| y1 | `row_y1` | `src/data_flow/build/c_prune_brackets_tests.rs` | S | RED |  |
| y2 | `row_y2` | `src/data_flow/build/c_prune_brackets_tests.rs` | C | RED |  |
| y3 | `row_y3` | `src/data_flow/build/c_prune_brackets_tests.rs` | K | GREEN |  |
| y4 | `row_y4` | `src/data_flow/build/c_prune_brackets_tests.rs` | K | GREEN |  |
| mf1 | `mf1_header_defines_reach_the_owner` | `src/data_flow/build/c_prune_brackets_tests.rs` | S | RED | control `mf1_control_without_defines_prunes` |
| mf2 | `mf2_defines_arrive_as_a_census_text` | `src/data_flow/build/c_prune_brackets_tests.rs` | S | RED | control `mf2_control_balanced_census_text_prunes` |
| mf3 | `mf3_one_of_two_definitions_in_two_files` | `src/data_flow/build/c_prune_brackets_tests.rs` | C | RED | control `mf3_control_only_the_balanced_definition_prunes` |
| mf4 | `mf4_the_issue_files` | `src/data_flow/build/c_prune_brackets_tests.rs` | S | RED |  |
| mf1 ctl | `mf1_control_without_defines_prunes` | `src/data_flow/build/c_prune_brackets_tests.rs` | K | GREEN |  |
| mf2 ctl | `mf2_control_balanced_census_text_prunes` | `src/data_flow/build/c_prune_brackets_tests.rs` | K | GREEN |  |
| mf3 ctl | `mf3_control_only_the_balanced_definition_prunes` | `src/data_flow/build/c_prune_brackets_tests.rs` | K | GREEN |  |
| cz1 | `cz1_cache_parity_when_a_header_define_becomes_unbalanced` | `src/data_flow/build/c_macro_census_tests.rs` | - | RED |  |
| E01 | `e01_20_macro_brackets_c` | `src/data_flow/build/c_prune_brackets_tests.rs` | - | RED | owner verdicts through `c_prune::scan` |
| E01 | `e01_20_macro_brackets_cc` | `src/data_flow/build/c_prune_brackets_tests.rs` | - | RED | owner verdicts through `c_prune::scan` |
| E01 | `e01_21_macro_args_c` | `src/data_flow/build/c_prune_brackets_tests.rs` | - | RED | owner verdicts through `c_prune::scan` |
| E01 | `e01_21_macro_args_cc` | `src/data_flow/build/c_prune_brackets_tests.rs` | - | RED | owner verdicts through `c_prune::scan` |
| OD-3 | `block_macros_list_is_the_design_list` | `src/data_flow/build/c_prune_brackets_tests.rs` | - | RED | extra row: the 16 names, exactly |

# Round-1 enumeration fold (terra + sol, 2026-11-14)

Two reviewers enumerated gaps after the first matrix (`.scratch829/enum-r1/enum-r1-{terra,sol}.out`, rows `G-terra-01..16`, `G-sol-01..15`; probes in `terra-probes/`, `sol-probes/`). One WRONG/MATERIAL finding was folded into DESIGN 6.3 and PLAN task 4 (G-terra-01 = G-sol-01: an in-repo macro wrapping a listed external block macro; the owner is refused through `MacroFacts::free`, reason stays `Off::MacroBrackets`, no new variant). Everything else is tests.

Base for these rows = HEAD of this branch (same stubs). Files: `src/call_graph/c/macro_census_brackets_r1_tests.rs` (census facts, 21 tests), `src/data_flow/build/c_prune_brackets_r1_tests.rs` (owner rows, 58 tests), `c_macro_census_tests.rs` (cz2), inline cases `22_macro_wrappers`, `23_macro_args_live`, `24_macro_splice`, `24_macro_splice_crlf` (`.c` and `.cc`; 33 new obligation rows, additions only). Helpers of `c_prune_brackets_tests.rs` became `pub(super)` (tests only).

## Totals (new tests only)

- **80 new tests: 63 RED, 17 GREEN on base.** Every GREEN is a control the reports predicted (K) or a premise/contract the design relies on and base already satisfies (free-set closure, token-paste G5, tight `__VA_OPT__`, parameter shadow, held-comma controls). Every RED fails on the assertion it was written for; the probes that could fail for another reason carry a premise assertion (census holds the payload macro: `assert_read`, owner exists on base with verdict `None`, every pair row passes the admissibility check, `dropped` is the failure).
- Pair rows (obligations): 33 (27 keep rows RED, 6 drop rows GREEN: w6 C and C++, k1, k2, k3, y11). Aggregate matrices `c_prune_matrix` and `cpp_prune_matrix` read them and stay RED on base.
- Kinds of the 27 RED pair rows: **18 S** (live pair dropped on base: w1, w2, w3, w8 and the C++ w1 to w3 compiled with real pthread.h or the reports' receipts; w5 by `clang -E` expansion with CPython 3.12 headers; a1, a1t, a9, a10, a11, a12; s1 to s4 compiled) and **9 C** (conservative, no liveness claim: w4 C and C++ because no CPython 3.13 headers are installed here, w7, y5 to y10). The other tests are census facts (F), owner verdicts, multi-file/loader/cache rows and K controls.

## Design amendment tests (WRONG, MATERIAL)

| report ids | test | file | kind | base | note |
|---|---|---|---|---|---|
| G-terra-01, G-sol-01 | `row_g_terra_01_w1`, `row_cpp_g_terra_01_w1` | c_prune_brackets_r1 (inline `22_macro_wrappers`) | S | RED | object-like alias `P`/`Q` of pthread push/pop, C and C++; terra compiled `1 2` |
| G-sol-01 | `row_g_sol_01_w2`, `row_cpp_g_sol_01_w2` | same | S | RED | function-like `ENTER`/`LEAVE`; sol compiled `41 2` |
| G-terra-01 (multi-hop) | `row_g_sol_01_w3`, `row_cpp_g_sol_01_w3` | same | S | RED | `E2 -> E1 -> ENTER`; compiled here with real pthread.h: `41 41` (cond false: the original 41 is live), C and C++ |
| G-sol-01 (callable name) | `row_g_sol_01_w8` | same | S | RED | `WHICH()(h, 0)`; compiled here, `41 41` |
| G-sol-01 (CPython) | `row_g_sol_01_w4`, `row_cpp_g_sol_01_w4`, `row_g_sol_01_w5` | same | C, C, S | RED | `PY_IN`/`PY_OUT` (Critical Section: no 3.13 headers here, so C); `PY_NT`/`PY_NT_END` (allow-threads: `clang -E` with 3.12 headers shows `if (cond) { PyThreadState *_save; ... x = 2; ...; }`) |
| G-sol-01 | `e01_22_wrappers_c`, `e01_22_wrappers_cc` | same | - | RED | verdict `MacroBrackets` for each owner (7 C, 4 C++) |
| G-sol-01 (all 16) | `g_sol_01_every_listed_name_is_refused_through_wrappers` | c_prune_brackets_r1 | - | RED | each of the 16 names via a function-like 2-hop wrapper and an object-like 2-hop alias, `.c` and `.cc` |
| G-sol-01 (all 16) | `g_sol_01_every_listed_name_is_refused_when_spelled_directly` | same | - | RED | extra beyond the reports: direct spelling of each name (the matrix pins two) |
| G-sol-01 | `g_sol_01_test_names_are_the_production_list` | same | - | RED | the loops iterate the same 16 names as `BLOCK_MACROS` |
| G-sol-01 (closure) | `g_sol_01_free_set_holds_the_listed_name_through_every_wrapper_shape` | macro_census_brackets_r1 | K | GREEN | the premise the amendment reads: `MacroFacts::free` holds the name through direct, alias and multi-hop wrappers for all 16 names, parsed `.c`/`.cc` and census text |
| G-terra-01 (header, text) | `g_terra_01_wrapper_in_a_header_c_and_cpp`, `g_terra_01_wrapper_in_a_census_text_c_and_cpp`, `g_terra_01_alias_chain_across_two_headers` | c_prune_brackets_r1 | S | RED | multi-file; each has the balanced-wrapper control inside the test (pruning must survive) |
| G-sol-08 | `row_g_sol_08_w6`, `row_cpp_g_sol_08_w6`, `e01_22_wrappers_c_control`, `e01_22_wrappers_cc_control`, `g_sol_08_a_parameter_spelled_like_a_hazard_is_not_free` | same / macro_census_brackets_r1 | K | GREEN | `PASS(pthread_cleanup_push)` and `ID(BEGIN)` stay clean: a parsed definition's parameter is not free. Not imposed on permissive texts (over-approximation accepted) |
| G-terra-07 | `row_g_terra_07_w7` | c_prune_brackets_r1 | C | RED | `(OPENV)(0)` with `#define OPENV(v) {`: spelled, not invoked; refused by spelling |

## Live argument-split rows

| report ids | test | kind | base | note |
|---|---|---|---|---|
| G-sol-02 = G-terra-02 | `row_a1_g_sol_02` | S | RED | **the live `a1` row** (sol source, `F(p,q) q if (c) p` / `G(p,q) q p`; compiled `41 2`). The gate-only original (`e01_21_macro_args_c`) stays |
| G-terra-02 | `row_a1t_g_terra_02` | S | RED | terra's source kept as a second independent source (`1 2`); same class, different shape (nested blocks, `if` inside the argument) |
| G-sol-03 | `row_a9_g_sol_03`, `g_sol_03_rescan_split_through_a_header` | S | RED | the split exists only after rescan (`H(INIT)`); verdict `MacroComma` through `INIT` (`e01_23_live_c`); the header-borne form carries a tight-`INIT` control |
| G-sol-04 | `row_a10_g_sol_04_alias`, `row_a11_g_sol_04_comment`, `row_a12_g_sol_04_chain` | S | RED | object alias `AF`/`AG`, comment before the group, chained group `ID(F)(...)`; one test each as the report lists them; verdict `MacroArgSplit` (`e01_23_live_c`) |
| G-sol-05, G-terra-05 | `row_y5_g_sol_05` to `row_y8_g_sol_05`, `row_y9_g_terra_05`, `row_y10_g_sol_05_subscript` | C | RED | square-held comma in a lambda capture list: plain, reference, init, mixed (the report's enumeration); terra's alias + comment form; subscript comma in C++. Verdict `MacroArgSplit` (`e01_23_live_cc`) |
| G-sol-06 | `row_k1_g_sol_06`, `row_k2_g_sol_06`, `e01_23_live_c_controls` | K | GREEN | `SZ((char[]){','})` and `SZ((int[]){(1,2)})`: no gate, pruning survives |
| G-terra-06 | `row_y11_g_terra_06`, `e01_23_live_cc_control` | K | GREEN | `ONE([a=(b,c)]{})`: the comma's innermost bracket is a parenthesis |
| G-sol-07 | `row_k3_g_sol_07` | K | GREEN | `(M)((struct P){1,2})` with a function-like `M(p,q) q p`: not an invocation, no G9. **Risk for the implementer:** `macro_call_arguments` only starts a group when the name is directly followed by `(`; `(M)(...)` has `)` after `M`, so the existing walk already treats it as a non-invocation. If the implementation adds a different recognizer for G9, this row must stay green |

## Lexical, census and loader rows

| report ids | test | kind | base | note |
|---|---|---|---|---|
| G-terra-08, G-sol-10 | `g_terra_08_raw_string_c_reading_exposes_a_closer`, `g_sol_10_raw_string_c_reading_exposes_brace_and_comma` | C (sol-10 IMMATERIAL) | RED | C++ raw string whose C reading exposes `}` / `{ ,`: the union of readable passes counts (accepted over-refusal, pinned) |
| G-terra-09 | `g_terra_09_cpp_digraph_exception_fails_closed` | C | RED | `V<::S>`: any `<:` spelling counts |
| G-terra-10 | `g_terra_10_name_split_by_a_splice` (census), `row_s1_g_terra_10_crlf`, `row_s2_g_terra_10`, `row_s3_g_terra_10_cpp`, `row_s4_g_terra_10_cpp_crlf`, `e01_24_splice_names`, `g_terra_10_spliced_name_from_a_header_with_lf_and_crlf` | S | RED | `#define OP\<nl>EN {` with a comment before the `;`; LF/CRLF, C/C++, same-file and header-borne. Compiled here (LF, CRLF, C, C++): `41 2` |
| G-terra-11 | `g_terra_11_va_opt_with_a_comment_before_its_parenthesis`, `g_terra_11_va_opt_comment_inside_tight_parentheses_is_tight` | F | RED / GREEN | comment and splice between `__VA_OPT__` and `(`; the comma-only form is added because `__VA_ARGS__` at top level would mask the transparent entry in terra's own source |
| G-sol-09 | `g_sol_09_va_opt_under_a_brace_or_bracket_is_loose`, `g_sol_09_va_opt_under_parentheses_stays_tight_and_balanced` | F | RED / GREEN | nesting under transparent entries incl. square counterparts and `__VA_OPT__()` empty |
| G-terra-12, G-sol-12 | `g_terra_12_loose_comma_union_across_files_after_undef_and_alias`, `g_sol_12_loose_comma_survives_undef_and_a_tight_redefinition`, `g_sol_12_loose_and_tight_definitions_in_separate_files`, `g_sol_12_loose_comma_union_in_permissive_texts` | F | RED | union over definitions, `#undef`, both orders, separate files, transitive alias (closure), permissive texts |
| G-terra-13, G-sol-11 | `g_terra_13_permissive_unreadable_line_sets_both_facts`, `g_sol_11_permissive_unreadable_line_with_only_a_comma` | F | RED | raw fallback: brace + comma after a balanced parameter group; comma only |
| G-terra-15, G-sol-13 | `g_terra_15_digest_changes_when_only_the_comma_fact_changes`, `g_sol_13_text_aggregate_digest_changes_when_only_the_comma_fact_changes`, `cz2_cache_parity_when_a_header_define_gains_a_loose_comma`, `g_sol_13_aggregate_identity_moves_on_a_loose_only_edit` | F | RED | census digest; text aggregate digest; partial-hit cache parity (cz1 now shares `cz_parity`); loader aggregate identity (`c-census:aggregate`) moves and returns |
| G-terra-14, G-sol-14 | `g_sol_14_transitive_non_c_include_reaches_the_owner_gates`, `g_terra_14_quoted_and_angle_transitive_includes`, `g_sol_14_net_file_without_an_include_reaches_the_owner_gates`, `g_sol_14_control_balanced_payload_prunes` | S/C, K | RED, GREEN | real files in a tempdir (`first.payload` -> `defs.payload`; `outer.inc` -> `defs.txt`, quoted and angle; the C-like-extension net `defs.inc` with no include); `.c` and `.cc` owners; `MacroBrackets` for the guard owner and `MacroComma` for the comma owner; each test asserts the loader read the payload first |
| G-sol-15 | `g_sol_15_split_names_and_digraph_directives_in_a_parsed_file`, `g_sol_15_split_names_and_digraph_directives_in_a_census_text` | F | RED | `%:define OP\<nl>EN /* ) , */ {` etc.; variants LF, CRLF, CR, BOM, continued comment; parsed `.c`/`.cc` and census text. All 45 (variant, path, macro) cases exist in the census on base (verified), so the failure is the fact |
| G-terra-16 | `g_terra_16_token_paste_poisons_whatever_the_new_facts_say`, `g_terra_16_digraph_and_plain_paste_owners_are_g5` | K | GREEN | `%:%:` and `##` poison; owners verdict `G5`, both dialects |

## Dedupe map

| pair | handled as |
|---|---|
| G-terra-01 = G-sol-01 | one design amendment; rows w1 (terra alias), w2 (sol wrapper), w3 (multi-hop), w8 (callable name), w4/w5 (CPython), header, census-text, two-header and loader forms, all 16 names, both dialects |
| G-terra-02 = G-sol-02 | `a1` is sol's source; terra's source kept as `a1t` |
| G-terra-05, G-sol-05 | square split: y5 to y10 (sol's members as separate tests, terra's alias+comment form as y9) |
| G-terra-08, G-sol-10 | two tests (the closer-only and the brace-and-comma sources) |
| G-terra-11, G-sol-09 | `__VA_OPT__` comment/splice adjacency and nesting kept as separate tests |
| G-terra-12, G-sol-12 | four tests (single-file undef both orders, two files, terra's three-file alias, permissive) |
| G-terra-13, G-sol-11 | two tests (both facts, comma only) |
| G-terra-14, G-sol-14 | loader tests; the net-file counterpart and the control added |
| G-terra-15, G-sol-13 | digest (parsed and text) + cz2 + loader aggregate |
| G-terra-10, G-sol-15 | splice-built names (terra) and normalization variants (sol) kept as separate rows |
| G-terra-03, G-terra-04, sol deviation audit | no test: confirmations recorded below |

## Confirmations and discrepancies

- **Round-1 confirmation of the guard twins' liveness.** Both reviewers compiled b1, b2, b5 and b12 twins (`1 1 1 1 / 2 2 2 2`, terra; `41 2`, sol): live. b13 (CPython critical section): sol confirmed it with the local CPython header (default GIL configuration, `41 2`); terra could not (no real headers) and kept it C. The b1/b2/b5/b12/b13 twins therefore stand as S (b13 as S on sol's evidence only). Both reviewers also confirmed the builder's diagnosis that the original scope-shift shapes (b1, b2, b5, b12, b13, a1) have no baseline parameter pair: gate-only (E01) is right; a1 now has the live row `a1`.
- **G-sol-05, C member (`STR2(a[0,1])` in a `.c` file) cannot be built:** tree-sitter-c does not parse a comma expression as a subscript index, so the owner is `G1Error` on base and head alike (the pair is kept; no hole, no row). The C++ form (y10) parses and is a row. Not a flipped expectation: the report listed it as a member to enumerate without a base claim.
- **C++ owners must not pass the local to a call as a bare argument.** `sink(x)` escapes `x` in C++ (E3), so the first draft of the C++ rows passed on base without exercising the gate. All C++ cases use `sink(x + 1)` (the convention of the existing C++ rows).
- w4 and the `Py_BEGIN_CRITICAL_SECTION*` family are C, not S, in this fold: no CPython 3.13 headers were available here. Listing the names can only refuse more.
- cz1 now calls `cz_parity` (same assertions, messages gained a label).

# Chain A (round 2) (terra, sonnet, sol2, opus; 2026-11-14)

Four enumerators (`.scratch829/chain2/{terra,sonnet,sol,opus}/LIST.md`; ids `G2-terra-01..04`, `G2-sonnet-01..07`, `G2-sol2-01..04`, `G2-opus-01..09`). **The rule of DESIGN.md is tentative and will be redesigned against the complete scenario set, so none of these tests names an `Off::` reason and none depends on a proposed fix**: each asserts the behaviour the goal requires (the live pair is kept; the control pruning survives; the census fact or input is seen). The enumerators' proposed fixes were ignored; only the scenarios count.

Base = HEAD `26f003be` (same stubs). Files:
- `src/data_flow/build/c_prune_brackets_r2_tests.rs` (owner rows through `check_row`, the real-header excerpts) and `c_prune_brackets_r2_loader_tests.rs` (tempdir loader rows);
- `src/call_graph/c/macro_census_brackets_r2_tests.rs` (census facts);
- `src/data_flow/build/c_macro_census_tests.rs` (`cz_parity` became `cz_parity_in`: owner file name and direction are parameters, `parse` picks C++ for `.cc`; cz1/cz2 are unchanged in meaning);
- inline cases `25_r2_external_alias` to `34_r2_control_unbalanced` (`.c`/`.cc`) appended to the two inline-case files (C 966, C++ 729 weighted lines); 28 C and 14 C++ new obligation rows (additions only, regenerated with both `gen_obligations.py`). `c_prune_matrix` and `cpp_prune_matrix` read them and stay red on base.

**BOUNDARY** (an input or macro the model cannot see: external macros, includes, build flags) is tagged twice: the test name starts `boundary_`, and the table marks it. A later step decides per row whether the fix goes in this PR, in #813, or splits; a row deferred to #813 will be marked `#[ignore = "#813"]`. Nothing is ignored yet.

Source: **obs** = the enumerators compiled the program (value live, `41 2` or equivalent) and ran prism on base with the asm control; **obs+** = the same, and the twin or variant was compiled again here (Apple clang 21.0.0); **rules** = derived from the rules and the expansion, no liveness run (kind C unless noted).

## Totals (new tests only)

- **62 new tests: 53 RED, 9 GREEN on base.** Every RED fails on the assertion it was written for: pair rows report `dropped` (never `inadmissible`: every row's pair exists in the baseline), loader rows report `the live pair ... was dropped`, fact rows fail on the fact (the macro is in the census), cache rows fail on the post-edit pair. The 9 GREEN are controls: 8 pair-row controls (the two variadic-forwarding drops, K3, K1, K2, K5a, K5b, K6) and the free-name premise of cz3.
- Pair-row tests: 41 (33 RED, 8 GREEN) over 42 new obligation rows (36 keep rows: 33 RED and 3 GREEN controls; 6 drop rows, all GREEN); census facts: 7 RED; cache: 2 RED + 1 GREEN premise; loader: 9 RED; real-header excerpts: 2 tests, RED (see below).
- Kinds of the 33 RED pair rows: S (live pair dropped, compiled): g2t01 (.c .cc), g2o06 (.c .cc), g2o06c (.c .cc), g2s01 (.c; .cc rules-derived), g2s02 (.c; .cc rules-derived), g2s03a, g2o01 (.c .cc), g2o01b (.c .cc), g2o02 (.c .cc), g2o03 (.c .cc), g2o04, g2o07; C (rules-derived): g2s01a-d, g2s03b-e2, g2o09, g2s07 (.c .cc).

## Pair rows (inline cases, `c_prune_brackets_r2_tests.rs`)

| row | test | case file | kind | BOUNDARY | source | base |
|---|---|---|---|---|---|---|
| G2-terra-01 | `boundary_row_g2_terra_01`, `boundary_row_cpp_g2_terra_01` | `25_r2_external_alias.c/.cc` | S | BOUNDARY | obs (terra; `-E` and run, exit 0 both dialects) | RED |
| G2-opus-06 | `boundary_row_g2_opus_06`, `boundary_row_cpp_g2_opus_06` | same | S | BOUNDARY | obs (opus, c) + obs+ (C++ and C compiled here: `41 2`) | RED |
| G2-opus-06 control | `row_g2_opus_06_in_repo_apply_control_keeps_the_pair`, `row_cpp_...` | same | S | | obs+ (the in-repo `APPLY` form compiled here, c and c++: `41 2`) | RED (the stub gate is absent; required: kept) |
| G2-sonnet-01 | `row_g2_sonnet_01`, `row_cpp_g2_sonnet_01` | `26_r2_external_names.c/.cc` | S (.c) / C (.cc) | | obs (CPython 3.12 headers, prints `41`); `.cc` twin rules | RED |
| G2-sonnet-01 members | `row_g2_sonnet_01a_critical_section_mut`, `01b_..._section2_mut`, `01c_..._sequence_fast`, `01d_suppress_iph` | same | C | | rules (3.13 internal headers and MSVC-only names; no compile evidence) | RED |
| G2-sonnet-02 | `boundary_row_g2_sonnet_02`, `boundary_row_cpp_g2_sonnet_02` | same | S (.c) / C (.cc) | BOUNDARY | obs (NumPy 2.x headers, `41`); `.cc` rules | RED |
| G2-sonnet-03 (1) | `row_g2_sonnet_03a` | same | S | | obs (`-DIFC=` and `-DIFC='if (c)'`, prints `2`) | RED |
| G2-sonnet-03 (2) | `row_g2_sonnet_03b` (`M **pp`), `03c` (`M(c) *p`), `03d` (`M *p = *q`), `03e1`/`03e2` (`M *p = 2, *q = 3`, both params) | same | C | | rules (same mechanism; the expansion shown by `clang -E`, drops by prism, no per-row run) | RED |
| G2-sonnet-03 (3) | `row_cpp_g2_sonnet_03k3_control` | `26_r2_external_names.cc` | K | | obs (C++ escapes the name already) | GREEN |
| G2-sonnet-03 (4) | `row_g2_sonnet_03k1_control` (`M x = 2;`), `03k2_control` (`M x;`) | `26_r2_external_names.c` | K | | obs (write-only drops on base) | GREEN |
| G2-sonnet-03 (5) | `row_g2_sonnet_03k5a_control`, `03k5b_control` | `27_r2_inrepo_prefix.c` | K | | obs (in-repo `IFC`/`NOTHING` keep the pair: the macro-typed declaration is ineligible) | GREEN |
| G2-sonnet-03 (6) | `row_g2_sonnet_03k6_control` | `26_r2_external_names.c` | K | | obs (a real inner `T *p = 0; p = 0; use(*p);` prunes on base: two drop obligations) | GREEN |
| G2-sonnet-07 | `row_g2_sonnet_07`, `row_cpp_g2_sonnet_07` | `28_r2_listed_name_shim.c`, `27_r2_listed_name_shim.cc` | C | | rules. **Borderline**: the pair is live only if the system macro, not the repository's balanced shim, is in effect (include order, `#ifdef`); the row asserts kept because that is undecidable from the repository. If the redesign proves the shim is the definition in effect, this row's expectation is the design's call (it is a sonnet "kind C") | RED |
| G2-opus-09 | `boundary_row_g2_opus_09` | `26_r2_external_names.c` | C | BOUNDARY | rules (PostgreSQL `PG_TRY`/`PG_CATCH`/`PG_END_TRY` unbraced, external; no Postgres headers here) | RED |
| G2-opus-01 | `row_g2_opus_01`, `row_cpp_g2_opus_01` | `29_r2_enclosing.c/.cc` | S | | obs (opus, both dialects, `41 2`) | RED |
| G2-opus-01 control | `row_g2_opus_01_variadic_control`, `row_cpp_...` | `30_r2_enclosing_variadic.c/.cc` | K | | obs+ (`K(...) __VA_ARGS__` rejoins the pieces; compiled here `2 2`: `x = 41` is dead, so the drop is correct) | GREEN |
| G2-opus-01b | `row_g2_opus_01b`, `row_cpp_g2_opus_01b` | `31_r2_enclosing_file_scope.c/.cc` | S | | obs (opus, `41 2`) | RED |
| G2-opus-02 | `row_g2_opus_02`, `row_cpp_g2_opus_02` | `32_r2_strip_parenthesis.c/.cc` | S | | obs (opus, c, `41 2`) + obs+ (C++ compiled here, `-std=gnu++17`, `41 2`) | RED |
| G2-opus-03 | `row_g2_opus_03`, `row_cpp_g2_opus_03` | `33_r2_digit_separator.c/.cc` | S | | obs (opus; `-std=c2x` and `-std=c++17`, `41 2`; recompiled here) | RED |
| G2-opus-04 | `row_cpp_g2_opus_04_owner_side_hide` | `33_r2_digit_separator.cc` | S | | obs (opus, prints `451`) | RED |
| G2-opus-07 | `row_cpp_g2_opus_07` | `34_r2_control_unbalanced.cc` | S | | obs (opus, prints `451`; base verdict `on-no-d:g6`, U drops the pair) | RED |

## Loader rows (`c_prune_brackets_r2_loader_tests.rs`; real files in a tempdir, never under tests/fixtures)

Each asserts `x = 41` to `return x` exists in the baseline and in the pruned result, for `.c` and `.cc`. Controls assert first that the loader's census holds `OPEN`/`CLOSE`.

| row | test | kind | BOUNDARY | source | base |
|---|---|---|---|---|---|
| G2-terra-02 | `boundary_g2_terra_02_computed_include_of_a_non_net_extension` | S | BOUNDARY (documented D2 exception) | obs (terra; `read=0`) | RED |
| G2-terra-03 | `g2_terra_03_computed_include_of_a_net_file` | C (control: the net reads `defs.inc`) | | obs (terra; `read=1`) | RED (stub facts) |
| G2-sol2-01 | `boundary_g2_sol2_01_absolute_include_of_an_in_repo_file` | S | BOUNDARY | obs (sol2); the tempdir's canonical absolute path is written into the include | RED |
| G2-sol2-02 | `boundary_g2_sol2_02_parent_after_a_directory_symlink` | S | BOUNDARY | obs (sol2); layout `a/link -> ../actual/deep`, `actual/defs.payload`; capability check: symlink creation (else prints SKIP and returns) and that the filesystem resolves `link/..` through the target | RED (ran, not skipped, on this macOS host) |
| G2-sol2-03 | `boundary_g2_sol2_03_include_name_differs_in_case` | S | BOUNDARY | obs (sol2); capability check: `DEFS.PAYLOAD` must resolve to `defs.payload` (else prints SKIP and returns) | RED (ran, not skipped: this host is case-insensitive) |
| G2-sol2-03 control | `g2_sol2_03_control_exact_case_include` | C | | obs (sol2 `rel` control) | RED (stub facts) |
| G2-sol2-04 | `boundary_g2_sol2_04_header_injected_by_the_build` | S | BOUNDARY | obs (sol2). **Not expressible as a build input**: prism takes no forced include (the loader and CLI carry none; `--compile-commands` feeds type enrichment only). The row documents the input prism does receive: the source names no header and `defs.payload` sits in the repository. Fix placement is the astra step's call | RED |
| G2-sol2-04 companion | `g2_sol2_04_companion_net_file_is_collected_without_an_include` | C | | sol2 proposed it, no receipt; the net should collect `defs.inc` | RED (stub facts) |
| sol2 relative control | `g2_sol2_relative_literal_include_control` | C | | obs (sol2 `rel_c/rel_cc`, `read=1`) | RED (stub facts) |

## Census facts and cache

| row | test | kind | source | base |
|---|---|---|---|---|
| G2-sonnet-06 | `g2_sonnet_06_double_quote_in_a_char_literal_does_not_hide_a_brace` (Q1), `..._single_quote_in_a_string_literal_...` (Q2), `..._escaped_backslash_char_literal_...` (Q3), `..._escaped_backslash_string_literal_...` (Q4), `..._quote_characters_do_not_hide_a_loose_comma` (Q5, Q6); each in a parsed `.c`, a parsed `.cpp` and a census text | F, IMMATERIAL unless a second scanner is written | rules | RED (each macro is read; the fact is false) |
| G2-opus-03 | `g2_opus_03_digit_separator_pairs_do_not_hide_a_bracket`, `g2_opus_03_digit_separator_pairs_do_not_hide_a_loose_comma` (the member: `#define L 1'0, 2'0`) | F | rules (the owner row `row_g2_opus_03` is the compiled S) | RED |
| G2-terra-04 | `cz3_cache_parity_when_a_header_alias_starts_naming_a_listed_macro` (forward: pruned before the header edit, kept after, incremental = fresh, digest moved; `.c` and `.cc`), `cz3_cache_parity_when_a_header_alias_stops_naming_a_listed_macro` (reverse, from a newly saved cache, against a clean build) | F | rules (no compiler claim) | RED |
| G2-terra-04 premise | `cz3_premise_only_the_free_names_differ` | K | the two header states differ in no bracket or comma fact, only in `MARK`'s transitive free set | GREEN |
| G2-sonnet-04 | `g2_sonnet_04_real_block_macro_excerpts_in_a_parsed_header`, `..._as_a_census_text` | S/C | real definitions: glibc and macOS pthread_cleanup, CPython allow-threads, trashcan (condition and wrapper), 3.13 and 3.14 critical sections (default and free-threading spellings), NumPy aliases. macOS, CPython 3.12 and 3.14 texts compared with the local headers; glibc and NumPy from memory. The pair `x@5 -> x@9` is judged (not an edge total: see Discrepancies 2) | RED: 7 of 8 excerpts drop the pair (glibc is a green control: `__sigsetjmp_cancel` makes the macro `control`, so rule D is already off) |

## Design inputs (no assertion)

Not built: the required behaviour is a design choice, or the row exists to record measured numbers. Sources are the lists.

1. **G2-opus-08, a header-position unbalanced pair** (`TEST_BEGIN(f)` = `static void f(void) { p_test_init(#f);` and `TEST_END` = `goto done; done: p_test_fini(); }`, used as `TEST_BEGIN(test_x) { body } TEST_END`, jemalloc `test.h:544-553`). tree-sitter reads it as one function whose body equals the compiler's inner block, so U and D inside the body agree with the compiler; the macro's `{` and `}` lie outside the body. Refusing is acceptable and costs recall; pruning inside may survive. Synthetic minimum: `#define TB(f) static void f(void) { prep();` / `#define TE goto done; done: fini(); }` / `TB(t1) { int x = src(); x = 2; sink(x); } TE`. Yield on base (opus, token-level scan, not the implementation): redis 128 `on` plus 200 `on-no-d` owners spell `TEST_BEGIN` (328 of the 530 below); cpython `BEGIN_CODECS_LIST`/`END_CODECS_LIST` (`Modules/cjkcodecs/cjkcodecs.h:256`); libcheck `START_TEST`/`END_TEST` (external). Opus found no header-position shape that changes the scopes inside a clean body (class-scope wrapper, GNU nested functions opened before the owner, a header macro hiding parameters: unresolved names are kept).
2. **G2-opus-05, C89/C90 `//` is not a comment** (IMMATERIAL; owner decision: keep the pair, or record C89 as an out-of-model dialect). Source, `.c`:
   ```c
   #define OPEN 4 //* c */ 2; if (c) {
   #define CLOSE 4 //* c */ 2; }
   int probe(int c) {
       int x = 41;
       OPEN;
       x = 2;
       CLOSE;
       return x;
   }
   ```
   `clang -std=c89 -E -P` prints `4 / 2; if (c) {;` / `x = 2;` / `4 / 2; };`; `clang -std=c89 -w` runs and prints `41 2`; `-std=c99` prints `4;`. Base: owner on, `x@4->x@8` absent; the asm twin restores it.
3. **G2-sonnet-05, a list edit does not invalidate caches by itself** (SMELL, IMMATERIAL). The block-macro list is a code constant, so a list edit moves neither the census digest nor the cached DFG; only the manual `CACHE_VERSION` bump protects a cached graph (the convention of `RETURNS_TWICE`, `KEYWORDS`). Optional row if the list survives the redesign: the census digest depends on the list.
4. **Yield of the tentative rule (measured, not by the implementation).** opus (token-level scan, owners that are `on` today and spell a flagged name): redis 530 of 8,465 owners (6.3%: `serverPanic` 167, `TEST_BEGIN` 328, `GETKEYS_RESULT_INIT` 13); cpython 362 of 10,815 (3.3%: the critical-section family 108, `EXTRA` 58, `_PyStatus_ERR` 46); protobuf 6 of 7,748 (`PROTOBUF_TC_PARAM_DECL`, a loose-comma parameter list used in function headers). Flagged macro names, unbalanced/loose-comma: redis 26/164, cpython 30/3,101, protobuf 1/32, re2 0/1, leveldb 0, snappy 0. Most loose-comma hits are argument-list or initializer fragments (`EXTRA`, `PySlot_DATA`, `*_INIT`) whose real uses cannot split. sonnet (same automaton over redis): 170 macro names would be flagged (17 unbalanced, the rest loose-comma), 479 `.c` files; `serverPanic` (a top-level comma operator) is spelled in 31 files, `TEST_BEGIN`/`TEST_END` in 118, `ERR` in 42; leveldb 0. The standing "measure cut yield first" rule applies before any of this is folded.
5. **Corpus observations (opus; no row).** About 80 cpython owners that are `on` and spell `Py_BEGIN_CRITICAL_SECTION`/`2`/`_MUTEX`/`Py_BEGIN_ALLOW_THREADS`/`_Py_BEGIN_SUPPRESS_IPH` drop 0 to 90 pairs each; every drop inspected crosses an unconditional `{ ...; }` wrapper (for example `Objects/exceptions.c:503`: `res@506->res@510`, overwritten inside the critical section), so none is live: the live risk needs a guard (the b rows) or shadowing (b1s), and neither was found in the corpora. A wrapper without a semicolon followed by a call is read as a prototype (`Py_END_ALLOW_THREADS` then `PyObject_Free(buffer);`, `Modules/_testcapi/mem.c:577-580`, owner `on`): the baseline itself never has the pair, so it is a recall gap, not a drop. Conditional and loop openers without a semicolon are off on base: `FOR_EACH_HOOK_BEGIN` (redis `src/hook.c:159`) and `mi_commit_mask_foreach` (cpython `Objects/mimalloc/segment.c:615`) are `G1Error`; Linux `for_nexthops(fi) { } endfor_nexthops(fi);` is `G1Nested`; in `log_do_begin` (`deps/jemalloc/test/unit/log.c:21`) the whole file loses its owners after line 5. The nav call-edge cache (v83) holds no DFG edges, so the `CACHE_VERSION` bump is the only cache that matters.
6. **Header audit (sonnet).** `scan.py` over CPython 3.11 to 3.14, the cpython checkout, the macOS SDK, Homebrew, Node 24, NumPy, PyTorch and pyarrow headers on that machine. Function-scope unbalanced macros: CPython 3.11/3.12 `Py_BEGIN/END_ALLOW_THREADS`, `Py_TRASHCAN_BEGIN_CONDITION`, `Py_TRASHCAN_END`, `Py_TRASHCAN_SAFE_BEGIN/END`, `_Py_BEGIN/END_SUPPRESS_IPH`; 3.13/3.14 `Py_BEGIN/END_CRITICAL_SECTION[2]`, `_MUTEX`/`_MUT`, `_SEQUENCE_FAST`; macOS SDK only `pthread_cleanup_push/pop` (plus file-scope `*_DECLS`/`*_NAMESPACE`); Homebrew `_XFUNCPROTOBEGIN/END`; NumPy `NPY_BEGIN/END_SUPPRESS_IPH`. Note: `Py_BEGIN_CRITICAL_SECTION_MUTEX` is the 3.14 public spelling; 3.13 spells it `_MUT` (internal). Other unlisted public-library members, header-scan only, no liveness claim (kind C, IMMATERIAL unless the owner wants a wider belt): OpenSSL `OSSL_TRACE_BEGIN`/`OSSL_TRACE_END`, PyTorch `HANDLE_TH_ERRORS`/`END_HANDLE_TH_ERRORS[_RET|_PYBIND]`, pyarrow `BEGIN_PARQUET_CATCH_EXCEPTIONS`/`END_...`; their usual spelling (`BEGIN(x) { ... } END`) parses as a nested function or errors, so the owner is off today. PostgreSQL: `PG_TRY` family in extension code is braced everywhere in 50 GitHub hits (`PG_TRY(); { ... } PG_CATCH(); { ... } PG_END_TRY();`), which keeps every write in its own block; only the unbraced form (row G2-opus-09) is at risk.
7. **Pre-existing, not macro-specific (sonnet, IMMATERIAL for #829).** Nested external calls `g(g(...(y)...))` cost 0.06 s at depth 250, 2.3 s at 500, 17.7 s at 1000 on base (the in-repo-macro form `F(F(...))` is the same), and 20,000 `y = F(y);` lines exceed 100 s. Not touched by the new gates. GCC versus clang for a directive between a macro name and its group, and `-std=c89`, could not be checked there (no GCC).
8. **Checked, no row (opus, reasoned from the gate rules).** Dangling `else` bound to a macro's `if`; a macro-made label or `?:`; `_Pragma`/OpenMP in a macro body; C++ `<,>` template-argument splits; a hidden reference declarator `int AMP(r) = x;`; `#if` that splits an `if` and its body; `$` in identifiers. sonnet: census lexing of quote characters, splice before `#`/between `#` and `define`/between name and parameter list, `/* */ #define` (all read correctly on base; `#\<nl>define` gives `off:Incomplete`); in-repo statement-prefix macros over 34 shapes (only legitimate write-only drops); a directive between a macro name and its group (clang does not expand, so "name directly followed by `(`" is right); scoped review builds read the census from `all_files`. sol2: embedded NULs (clang did not join identifier fragments), enclosing macros, template owners, another external wrapper.

## Chain A (round 2): not built, with reasons

- G2-opus-01 members "a positional macro that drops or reorders a piece", "an owner that is a C++ member function inside a class body" (opus: not built there either). The drop shape needs a source whose pieces still parse after dropping (`p r` of the encl2 source gives `int a e;`); not worth a bespoke fixture before the redesign.
- G2-opus-07 in C: the C scope-shift shapes have no baseline pair (Deviations 1 above), and a guard-shape twin with a `goto` macro already has D off and no U-droppable pair. The C++ row is the compiled one.
- G2-sol2 `_off` companions and the asm controls of every row: the enumerators' same-environment controls establish removal rather than a missing baseline edge; here the equivalent is the `before` assertion of each row (inadmissible if the baseline has no pair).

## Discrepancies and notes

1. No row's base outcome differs from the list's claim. Every S row's pair exists in the baseline and is dropped on base; every K control is green.
2. **Edge totals are the wrong oracle when an owner stays `on-no-d`.** The rules legitimately drop other pairs (the one into a write-only line such as `x = 2;`), so `a == b` on edge counts fails for a correct implementation even when the live pair is kept. The glibc excerpt showed it (`on-no-d:g6`: 7 edges, the live pair x@6 -> x@10 kept, three other pairs dropped by U). The round-2 tests judge the specific pair (`has_pair`, `check_row`). The round-1 rows that use `counts(...)` and compare totals (`mf1`, `mf2`, `mf3`, `g_terra_01_*`, `g_sol_03_*`, `g_terra_10_*`, `g_sol_14_*`) are exact only while the redesign turns the owner fully off; if the redesign keeps U on for some of them, those assertions must become pair checks.
3. G2-sonnet-03 K2 (`M x;`) is kept as the list gives it, but the macro could expand to `if (c)`, in which case `x;` reads the parameter and discards the value; the drop is a write-only drop only in the sense that no value flows anywhere. If the redesign refuses it, K2 goes.
4. G2-sonnet-07 (`row_g2_sonnet_07`) is borderline between a conservative row and a design choice; see its table row.
5. The forced include of G2-sol2-04 cannot be expressed to prism (no input carries it); the row documents the input prism receives.
6. The `.cc` twins of G2-sonnet-01 and G2-sonnet-02 are rules-derived (the enumerators compiled the C probes only); the `.cc` twin of G2-opus-02 was compiled here. `.c` and `.cc` of G2-opus-01k were compiled here only as `.cc`.
7. `cz_parity` now takes an owner file and a direction (`cz_parity_in`); cz1 and cz2 still call it with `a.c` and the forward direction and are still red on base (their digest assertion).
