# REPORT fold 1b: PR #811 (#630 slice 6b)

Head: `perf/630-s6b-c-rules` at 5a2b4b1 (merge of main ef7274e8 plus the freshness commit). Base build: main ef7274e8. Both release builds.

## Merge resolution
- `git merge origin/main`; conflicts in the 6a files were resolved by taking main's version (`macro_census.rs`, `macro_census_fold3_tests.rs`, `repo_loader/c_census.rs`, `repo_loader.rs`, the 6a-era DESIGN.md), then re-applying my 6b addition (`normalized_texts`, `spelled_identifiers` in `macro_census.rs`, applied as a patch).
- `lexical_prune.rs`: kept my `c_census` parameter (main's `_c_census` placeholder dropped). `data_flow/build/mod.rs`: kept my `c_prune*` modules.
- CLAUDE.md: kept main's census paragraph (with its narrowed exotic-lexing wording, "slice 6b consumes it" replacing "exists for slice 6 ... no output yet") and my slice-6b rules paragraph. Cache is v175 everywhere (`CACHE_VERSION`, its pinned tests, the history comment).
- `cargo test --lib -- c_prune macro_census f3_ cpg_cache lexical_prune`: 151 passed, 1 failed. The failure, `cpg_cache::stream::tests::a_write_that_cannot_start_fails_and_leaves_the_previous_cache_usable`, relies on a 0555 directory refusing writes; this sandbox runs as root, which ignores it. Environmental, not caused by the merge.

## Freshness item
- `repo_loader::current_c_census_identity` recomputes the `c-census:` identity from disk with the loaded repo's own skip list (`c_census::gather`); `c_census_text_paths` names the text files of the loaded identity.
- `freshness_capture::validate` compares the loaded `c-census:*` entries of `LoadedRepo.manifest_hashes` with that recomputation (like with like; `scope_graph_inputs` is untouched, so main's every-load-stale bug is not reintroduced). A changed, added or removed text include (or a text-failed entry) is reported as the file's path.
- `FreshnessProbe::from_loaded_repo` now stamps the census text files as real tracked paths (the `c-census:` keys were logical and never changed), so a later in-session edit is caught by `check()` too.
- Tests (`freshness_capture_tests.rs`): `freshness_census_text_edit_marks_stale` (editing `.inl`, also through a probe's stamps), `freshness_census_text_removal_marks_stale` (removing `.inc`), `freshness_census_unchanged_with_unreadable_directory_is_not_stale`. All 3 pass.
- `cargo test -p prism-mcp freshness_`: 19 passed, 10 failed. The same 10 fail on the unmodified merge base (checked with stash): they use mode-0 files and directories, which root can still read. Environmental. Consequence: the unreadable-directory test cannot exercise a real unreadable directory in this sandbox; it should be re-run as a non-root user.

## Measurement
Edges from `prism nav --no-cache dfg-stats --repo <dir> --edges`, compared by (from, to) pair, base vs head.

| corpus | base pairs | head pairs | removed | added | label changed |
|---|---|---|---|---|---|
| redis 7.2.4 `src/` (d2c8a4b9, unmodified) | 290,566 | 255,853 | 34,713 | 0 | 0 |
| cpython v3.12.7 (whole repo) | 3,481,002 | 3,481,002 | 0 | 0 | 0 |
| prism `src/` (Rust, non-C control) | 98,734 | 98,734 | 0 | 0 | 0 |
| prism `tests/fixtures` (mixed, has C fixtures) | 8,458 | 8,149 | 309 | 0 | 0 |

Raw edge lines (with duplicates): redis 290,580 to 255,867, a difference of 34,713, equal to the earlier redis-ascii fold-1 count of 34,713.

Census state:
- redis: not `incomplete` (no `off:Incomplete` owner). About 2,027 `#define` lines in `src/*.c|h` (a textual grep, not a census readout; no census API was used).
- cpython: `incomplete`; every C owner is off (13,177 `off:Incomplete`), hence 0 removed. I did not investigate which input trips it.

## Ledger (owners by verdict, `PRISM_C_PRUNE_TRACE=1`)
| verdict | redis | cpython | fixtures |
|---|---|---|---|
| on | 3,986 | 0 | 117 |
| on-no-d:g6 | 5 | 0 | 1 |
| off:G1Error | 47 | 1,319 | 2 |
| off:G1File | 57 | 2,643 | 0 |
| off:G1Nested | 0 | 0 | 1 |
| off:G2 | 10 | 0 | 5 |
| off:G3 | 2 | 0 | 2 |
| off:G4 | 0 | 0 | 1 |
| off:G5 | 69 | 0 | 1 |
| off:Incomplete | 0 | 13,177 | 0 |

## Samples
`samples-fold1b.jsonl`: 20 random removed redis edges (seed 8111, from the sorted removed set), each with both source lines and the base labels. I read every one in source with surrounding code. All 20 are correct removals:
- write-only use line (rule U): samples 0, 1, 4, 6, 9, 13, 14, 15, 17, 18;
- an overwritten or dominated def (rule D): 2, 3, 5, 7, 8, 10, 11, 12, 16, 19 (each use is dominated by a direct-child write earlier in its region, e.g. sample 3 `t = lua_type(..)` at script_lua.c:634 before the read at 635; sample 10 `o = createObject(..)` at rdb.c:2252 before the use at 2340).
0 of 20 are wrong. Judgement from source excerpts, not an independent tool.

## Timing (redis `src/`, `dfg-stats` without `--edges`, interleaved 3+3, wall seconds)
- base: 10.10, 10.21, 9.81 (mean 10.04)
- head: 10.47, 10.89, 11.02 (mean 10.80), about +7.6%.

Progress: DONE
