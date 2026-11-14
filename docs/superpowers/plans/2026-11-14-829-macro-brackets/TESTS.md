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
