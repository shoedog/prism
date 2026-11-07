# #630 slice 5: Python rules U and D — report

Branch `perf/630-ud-python` (base a2c9c9f4). Commits: RED matrix, implementation, test-oracle updates.

## What changed
- `src/data_flow/build/python_prune.rs` (new, 55 lines) + small edits in `lexical_prune.rs`: U and D enabled for Python `function_definition` owners (no parse errors). Go path byte-identical (go_prune_matrix and the all-fixture subset test pass unchanged for Go).
- Writes: whole `expression_statement` with one `assignment` (single/tuple/list/starred target, with value) or `augmented_assignment`, target exactly the path (identifier or attribute chain). Not writes: chained inner target, walrus, loop / `with as` / `except as` targets, imports, `x: int`, `del`.
- Regions: `block` only. Nested scopes (kept, never proven): `def`, `lambda` (existing boundaries), comprehensions, generator expressions, `class` bodies. Names declared `global`/`nonlocal` anywhere in the owner are never dropped by D.
- CACHE_VERSION 155 -> 156 (doc line, pins, CLAUDE.md sentence) and a CLAUDE.md Python paragraph beside the Go U/D text.
- Tests: `tests/fixtures/dfg_prune/python/` (4 fixtures, 70+ pair obligations, RED on main), `python_prune_tests.rs` (frozen-reference differential: edges_after ⊆ edges_before, labels unchanged, counters = removed edges). RAYON 1 and 4.

## Existing test updates (per-edge justification)
Straight-line Python witnesses of `NameOnly(Killed)` are now removed by D, which is the intended effect. Fixtures were given `global <name>` (D keeps global names) so the witnesses remain, lines shifted by +1: `tests/ast/dfg_label_test.rs` (x=source();x=clean();sink(x): edge 2->4 now dropped), `tests/integration/dfg_label_parity_test.rs` MIXED, `tests/common/mod.rs` provenance/taint/cross-file killed cases, `dfg_label_store_test.rs` same-line `a = 1; a = 2`. `lexical_prune_tests` subset oracle now allows Python to shrink (still subset, labels equal).

## Test results (RAYON 1 and 4)
`--lib data_flow` ok, `--test ast` ok, `lang_python` ok, `lang_go` ok, `navigation` ok, `algo_taxonomy` ok, `integration`: only failures that also fail on main — 5 `owner_activation_test` (checked on a clean main worktree) — plus `build_profile_round2::rss_intervals_exclude_cfg_in_both_owner_orders` which is a JS memory-sampling test that fails under the parallel load of the full target and passes alone (3/3). `--lib cpg_cache`: `a_write_that_cannot_start...` fails on main too (running as root). Clippy: no warnings in touched files. No full suite, no mutgate (per brief).

## Measurement (release main vs head, 4 threads, `nav --no-cache dfg-stats --edges`)
| corpus | removed | added | kept-label transitions | write_only / dominated counters |
|---|---|---|---|---|
| django e8cff292 | 44,756 | 0 | 0 | 20,556 / 24,236 (counters count duplicate emissions) |
| mypy 5ef09027 | 28,036 | 0 | 0 | 14,405 / 13,635 |
| vitess 16bcb943 (control) | 1,394, all `.py` files | 0 | 0 | |

Reverse direction (`add2.py HEAD MAIN`): main_only 0 on all three. Vitess: every non-`.py` edge identical (the corpus contains Python files, so the whole-output cmp differs only there); `dfg-stats` and `call-stats` differ only in the dfg counters/labels.
call-stats identical outside `dfg_labels`: django yes, mypy yes, vitess yes.

Removed by label — django: cfg_incomplete 24,439; killed 14,783; exact 4,869; alias_unstable 646; sameline 19. mypy: cfg_incomplete 19,294; killed 4,994; exact 2,886; alias_unstable 860; sameline 2.
Shape (heuristic on the use line): django write-only-like 21,204 / dominated-like 23,552; mypy 14,682 / 13,354.

Top-20 functions by drops: django — test_change_view (858), test_foreign_key (855), test_one_to_one (680), test_datefieldlistfilter (629), test_many_to_many_field (518), test_dwithin_gis_lookup_output_with_rasters (480), test_deconstructible (446), test_count (444), test_view_decorator (420), test_simplelistfilter (408), test_custom_pk (396), test_prepopulated_fields (380), test_truncate_html_words (352), regex_helper.normalize (351), test_validating_multiple_fields (351), test_login (346), create_big_data (329), test03_delslice (308), test_12_arithmetic (272), test_prevent_duplicates_from_with_the_same_formset (264). mypy — nativeparse.read_expression (1898), read_statement (1244), gen_blog_post_html.convert (621), emit_cast (594), _type_object_overlap region of meet.py (440), narrow_type_by_identity_equality (390), incompatible_argument (390), check_callable_call (328), infer_c_method_args (318), pretty_callable (314), visit_callable_type (255), test_find_sources_exclude (212), function_annotations (182), test_crawl_namespace_explicit_base (181), visit_instance (179), visit_sequence_pattern (158), test_executable_inference (153), visit_comparison_expr (151), join_instances (149), bool_value (148). (Enclosing function found by a text heuristic on indentation; nested defs may be attributed to the outer function.)

Sample audit: 40 edges per corpus, all RIGHT; no WRONG found. See `samples.md`. Not sampled: vitess's 1,394 Python edges (same code path).

Timing (django dfg-stats, wall, interleaved main/head/main/head): 23.10, 23.68, 23.31, 23.22 s — no measurable difference.

## Draft PR description
perf(dfg): Python rules U and D (#630 slice 5). Enables the removal-only lexical U and D proofs for Python function owners (see "What changed"). Removes 44,756 / 28,036 expanded edges on django / mypy with 0 added and 0 label transitions; Go and other languages unchanged; CACHE_VERSION 156. Tests: RED-first Python matrix, frozen differential; per-edge justified updates of 4 Python label-witness fixtures. Known accepted costs: `global`/`nonlocal` names, comprehensions and class bodies are conservatively kept; for/with/except targets never dominate.
