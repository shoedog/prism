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
