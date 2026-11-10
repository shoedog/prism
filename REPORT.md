# #630 slice 5 (Rust): rules U and D — report

Code branch `perf/630-ud-rust` @ 2c86edf (RED matrix 7ce774e, implementation 2c86edf), base origin/main 9f69debf. Cache v169.

## Step 0 grounding
1. **Owners.** The producer's owners for Rust are `function_item` only (`queries.rs`: `(function_item) @func`), including methods in `impl`/`trait` blocks. Closures, `async`/`gen` blocks are NOT owners: their defs/uses are lvalue/rvalue spans of the enclosing function (the query runs over the owner's byte range). A nested `fn` item is its own owner AND its lets/uses also appear as spans of the outer owner. All of these are therefore nested callables for U/D (closure/async/gen from `callable_boundary_node_types`, plus `function_item`), so defs inside them are always kept and uses inside them are never proven.
2. **Line anchoring.** Yes: uses are line-anchored as for Python/Go (`scoped_refs` per line; a zero-width anchor Use is minted when no rvalue span exists).
3. **Defs recorded.** `let` (any pattern, incl. tuple; `let x;` too) gives defs at the pattern identifiers; `=`/compound assignment give defs at the lhs path (`*p = ..` and index writes give a def of the base); `if let`/`while let`/`match`/`for`/closure-parameter bindings give NO def (not in the Assignments query); writes inside macro invocations are token trees, not assignments: no def, while identifiers there are reads (`println!("{}", x)` is a use).

## Design (Rust specifics, `src/data_flow/build/rust_prune.rs`)
- Enabled for `function_item` owners without parse errors. Region = `block` only. Write = direct-child `let x = v;` (identifier pattern, value, no `else`) or `expression_statement` holding `=`/compound assignment with lhs exactly the path (identifier or plain field chain). Statements preceded by an attribute (`#[cfg]`) are no write (may not exist).
- Hazards: `?`/`return`/`break`/`continue` leave the region (fine); labeled-block `break 'a` only leaves forward (no breaker, D applies); loops re-enter at the top; `unsafe` raw writes, interior mutability, `&mut` aliases and macros expanding to a write/`let` of the name are out of model and can only make the kept dominating def's own edge imprecise, never resurrect a dropped def (the dropped def is overwritten by the visible dominating statement on every path). Shadowing needs no binding facts: a dominating `let x` is the binding every later use in the region reads; inner-block lets are kept inside the (d*, S(u)] window or are in a block not containing the use. Rule S does not run for Rust.
- Tuple/struct patterns, `let else`, `let x;`, `if let`/`while let`/`match`/`for`/closure params, nested assignments, macro writes, alias twins: never d* (keep rows). Same-line pairs judged per raw def span.
- Isolation: Go/Python/JS untouched; `lexical_prune.rs` only got dispatch hooks, new code is in `rust_prune.rs` (~100 lines).

## Tests
- RED `7ce774e`: `rust_prune_matrix` (160 rows over `tests/fixtures/dfg_prune/rust/*.rs`, 84 drop rows, all fail on main) + `rust_prune_is_removal_only_over_fixture_tree` (edges_after ⊆ edges_before, labels unchanged, counter identity, over every `.rs` under `tests/fixtures`). Green at 2c86edf.
- Existing `lexical_prune_tests::prune_subset_and_labels_across_fixture_tree` extended: Rust is now allowed to change (removal-only + dominator invariant as Python).
- Targeted suites at RAYON_NUM_THREADS 1 and 4: `--lib cpg`, `--lib cpg_cache`, `--lib data_flow`, `--test ast|lang_rust|lang_go|lang_python|navigation|integration|algo_taxonomy` all pass. `-p prism-review --test cli`: 5 `owner_activation_test::owner_admission_*` fail (known, root). `cpg_cache::stream::tests::a_write_that_cannot_start_fails_and_leaves_the_previous_cache_usable` fails (in `--lib cpg` and `--lib cpg_cache`); it ALSO fails on origin/main 9f69debf in this environment (running as root), so unrelated. No goldens needed updating. `cargo fmt --all --check` clean; no clippy findings in touched files. No full suite, no mutgate.

## Measurement (release builds, 4 threads, `nav --no-cache dfg-stats --edges`, `call-stats`, `add2.py`)
Corpora: (1) prism's own `src/`+`crates/` at 9f69debf (git archive, outside the working tree); (2) tokio `tokio-1.40.0` — DECISION: `gh api .../tarball` was denied (no access to that repo from this session), so per controller instruction I used a parse-only `git clone --depth 1 --branch tokio-1.40.0`, commit `ea6d652a102dee3f22b490db70545b7f66a23fb7` (no tarball sha256 exists); never built or run. Controls from the release store, sha256 verified against SHA256SUMS: vitess `bd5fbe25…98a9`, django `462e6302…2bc`.

| corpus | main edges | head edges | removed | added | kept-label transitions | write_only | dominated |
|---|---|---|---|---|---|---|---|
| prism self | 139536 | 131664 | 7872 | 0 | 0 | 4075 | 3815 |
| tokio 1.40.0 | 32647 | 28660 | 3987 | 0 | 0 | 1604 | 2383 |
| vitess (control) | 1559980 | identical | 0 | 0 | 0 | 0 | 0 |
| django (control) | 187279 | identical | 0 | 0 | 0 | 0 | 0 |

Controls: edges, `call-stats` and `dfg-stats` are byte-identical (`cmp`). (Counters exceed removed distinct edges because they count expanded edges before dedup.)

`call-stats` main vs head, outside `dfg_labels`/prune counters: only `return_flow.return_input_edges` moves — prism 8533→8520, tokio 493→487. Mechanism: removed edges whose use is a `return`/tail expression (66 such removed edges on prism; all sampled ones RIGHT, e.g. shadowing `let` dominating the returned name) shrink the Step 5c return-input set; I did not enumerate the 13/6 individual input edges (stated limitation). `dfg_labels` counters fall consistently with removals.

Removed by label — prism: nameonly/cfg_incomplete 3175, nameonly/killed 2880, exact 1542, alias_unstable 144, sameline 131. tokio: cfg_incomplete 1874, killed 1484, exact 451, sameline 173, alias_unstable 5.
Removed by shape (use line kind / direction) — prism: use-is-let backward 2221, other forward 1808, assign forward 1222, let forward 1139, other backward 1023, assign backward 459. tokio: let backward 1110, other forward 1089, other backward 759, let forward 702, assign forward 246, assign backward 81.
Top-20 by drops: the edge dump carries no function name, so the ledger groups by file; top prism files: navigation/queries.rs 421, go_scan.rs 310, js_write_census/semantic_tests.rs 278, paths_syntax.rs 273, callback_identity_tests.rs 171 …; top tokio files: fs/file/tests.rs 380, tokio-util/tests/codecs.rs 221, tests/io_buf_reader.rs 179, rt_unstable_metrics.rs 142, mpsc.rs 118 …. The 10 "top" samples are drawn from the top-20 files.

Timing (tokio, `dfg-stats`, 4 threads, 2+2 interleaved, wall s): main 5.28, 5.20; head 5.39, 5.50 (~+3%, within noise of this host).

## Samples (seed 750; 30 random removed + 10 from top-20 files per corpus): 80 reviewed, 80 RIGHT, 0 WRONG
Categories (each verified against source lines): (a) use line is a write-only `let x = ..`/`x = ..` (rule U; includes `i = next;` and `impls = &[];`); (b) shadowing `let` between def and use (D; e.g. transport.rs 424→458 `let id` at 429, codecs.rs chunk, tokio io_buf_reader.rs buf/nread); (c) backward edge with no loop (def after use, straight-line); (d) loop cases checked by hand and RIGHT: go_scan.rs `i += 2`(26)→`i += 1`(50) overwritten by `i += 4`(44) in the same arm block; paths_syntax.rs `i += 1`(68)→(123) after `i = 0`(101); paths_snapshot.rs `rest`(785)→757 overwritten at loop top by 748; notify.rs `old_waker = None`(1061)→`drop(old_waker)`(1076) overwritten at 1070 in the same if-block; (e) alias-twin defs (`idx`/`start`, `sanitized = result`, `old_dfg = dfg`) — not real writes, dropping is correct; (f) outer binding not yet in scope (pool.rs `let workers = { let mut workers = ..; workers.sort..}` 268→270); (g) `MaybeUninit::uninit()` path identifier coincidence (io_read_buf.rs). Retained by design: defs inside closures/async blocks (nested), uses inside closures. Sample files: `/tmp/out/{prism,tokio}-samples.json` (deleted; not pushed). No WRONG, so no fix/re-measure round was needed.

## Decisions recorded (controller defaults)
- tokio obtained by shallow clone, not tarball (above).
- Unsure forms kept: tail/match-arm assignment, tuple patterns, `ref`, field-path prefix writes, attributed statements, nested `fn` item lets (kept as nested callables even though they cannot reach the outer owner).
- A call-argument `if`-block write (`foo(if c { x = 5; 1 } else { 2 })`) keeps its use line: the rvalue collector reports a read there (matrix row pins keep).

## Draft PR description
**perf(dfg): Rust rules U and D (#630 slice 5)** — Enables the removal-only lexical rules for Rust `function_item` owners (`rust_prune.rs`, dispatch hooks in `lexical_prune.rs`). U drops pairs into write-only `let`/`=` lines; D drops pairs overwritten by a dominating direct-child `let`/assignment in a single-entry `block`. Closures, async/gen blocks and nested fns are nested callables (kept). Cache v169; CLAUDE.md paragraph. Evidence: prism self −7872 / tokio −3987 edges, 0 added, 0 label transitions, vitess/django byte-identical, 80/80 sampled removals RIGHT. `return_flow.return_input_edges` moves (−13 prism, −6 tokio) via removed edges into return expressions. Known unrelated test failures in a root environment: 5 `owner_admission_*`, `cpg_cache::stream::..a_write_that_cannot_start..` (fails on main too). Parallel slice also targets cache 169: whichever merges second bumps.

Progress: DONE
