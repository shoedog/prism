# Fold 3 report: PR #794 (#630 slice 5, Rust U and D), macro hygiene by census and format captures

Code: branch `perf/630-ud-rust`, plain pushes, no force-push. Base `5bc3fdb3`; fold commits **303f0c5** (code, docs, rows, tests) and **609b276** (test pins only: wire/oracle follow-ups, no source change). Cache **v172**.
Measured binaries: release builds of origin/main `72be3b73`, `5bc3fdb3` and `303f0c5` (the fold head's sources; 609b276 only changes tests and one fixture), `nav --no-cache dfg-stats --edges` / `dfg-stats` / `call-stats`. Corpora: **prism self** = `git archive 72be3b73 src crates` (the fold-2 convention; main 140204 edges, `return_input_edges` 8580, as in fold 2) with the whole origin/main tree minus `eval/secbench` (sparse checkout, never materialised) as a supplementary corpus; **tokio** `ea6d652a102dee3f` (parse only, main 32647 edges, as in fold 2); controls **vitess** `f1d90ff5` and **django** `9425a85c` (shallow clones of the current HEAD; the fold-1 `corpora-v1` pin was not reachable, controls are compared binary against binary). Corpus code was only parsed, never built or run. `add2.py` (sha256 verified) was run for main→fold and 5bc3fdb3→fold on every Rust corpus, plus my own `cmp3.py` (in `data/`) for restored/removed sets, causes and samples.

## Items done
1. **Option 3b census + per-name refusal + owner-off + item 3 retired** (303f0c5).
   - `src/call_graph/rust/macro_args/census.rs`: `RustMacroCensus` (verdict per `(qualifier, spelling)`; `generating`, `defined`, `ext_bound`, `macro_use_extern`; least fixpoint; `digest()`; `of_file` seam).
   - `rust_prune::scan_macros`: every `macro_invocation` of the owner (nested callables and items included); own path and every `ident ! (..)` in its soup are judged; one Refused spelling refuses every identifier token of the invocation (`LexicalFacts::rust_refused`: no U in `proof`, no D in `keep_def`); the identifier `macro_rules` in any token tree, or `paste!`/`concat_idents!`, turns the owner off. `in_macro_tokens` and `rust_macro_uses` are deleted.
   - Plumbing: `DataFlowGraph::build` / `build_subset` compute the census from the whole map; `build_with_census_files(subset, all_files)` for the scoped build (`cpg/scoped_build.rs`); `Producer::compact_file_refs`/`file_refs_with_census`/`LexicalFacts::new` take it; the test seam `file_refs(file, parsed)` keeps a one-file census.
   - Cache: digest persisted on `CompactDfg` (wire field `rust_macro_census_digest`, last); `build_incremental_with_scope_graph_inputs` recomputes it after the macro-shadow guard and falls back to `build_impl_inner` on mismatch; `CACHE_VERSION` 171→**172** (const, changelog line, in-file pin, `dfg_codec_tests` x3, CLAUDE.md).
2. **Format-capture reads** (303f0c5): `{name}`, `{name:spec}`, `{name:?}` and `name$` width/precision arguments in any macro's string literal token; `{{`, `{0}`, `{}`, `{:?}` skipped. A capture is an occurrence of the name on **every line of its enclosing statement** (the child of the innermost block, `statement_lines`), so it blocks U and is an occurrence D's windows must dominate; no Use node, no edge. **Multi-line decision: by the statement's lines**, so `x = format!(\n "{x}b"\n)` keeps the pair into the `x = format!(` line.
3. **Residuals disclosed** (not fixed) in CLAUDE.md and the `rust_prune.rs` module doc: attribute proc macros on the owner or an enclosing item; identifiers assembled by an external name-building macro under another name; the #772-style root collision. Also fixed the stale docs the Opus review named (`has_local_macro`, the `&mut` sentence).
4. **Rows and tests.** `05_fold3.rs` (21 rows: 16 keep, 5 controls), three flipped rows, census/two-file/scoped/cache tests (`rust_prune_fold3_tests.rs`).

## RED rows (every new keep row fails on 5bc3fdb3)
`05_fold3.rs` and the new `obligations.jsonl` were copied into a worktree at 5bc3fdb3 (log: `old-matrix-5bc3fdb3.log`, run as `cargo test -p prism --lib rust_prune_matrix`). It fails on **all 16 new keep rows**: `gen_u` (2→4 style, `generated_reader_u`), `gen_d`, `soup_macro_rules`, `metavariable_macro_rules` (`install!(macro_rules, x)`), `generator_transitive`, `metavariable_name` (`via!(install_get, x)`), `generator_nested_in_soup`, `external_refused`, `unknown_refused`, `conflicting_defs`, `name_builder` (`paste::paste!`), `expr_fragment`, and the four format rows `format_capture` (`x = format!("{x}b")`), `format_capture_spec` (`{x:?}`), `format_capture_write` (`write!(f, "{x}")` on the assignment line), `format_capture_multiline`. It also fails on `std_qualified` (a drop that 5bc3fdb3 kept) and on the three flipped rows. Controls that pass on both sides (as designed): `clean_in_repo`, `std_matches`, `format_escaped_control` (`format!("{{x}}")` still drops), `format_capture_dominated` (a capture read by the dominating def still drops).
Flipped rows: `02_hazards 114→116` and `03_fold1 81→83` flip back to **drop** (`println!("{}", x)` is Allowed, no capture), as the brief allows (every format row is still kept); `04_fold2 102→105` (`install_pick`) stays keep, now by the per-name rule; `03_fold1 111→114` (`shadow!(x)`, an undefined spelling) flips from drop to **keep** (unknown spelling, Refused). `03_fold1 hygiene` and `04_fold2 87→89` are unchanged (owner off).

## Measurement
Against **main** (removed only, 0 added, 0 kept-label transitions; add2.py and cmp3.py agree):

| corpus | main edges | 5bc3fdb3 | fold | removed main→fold | added | kept-label transitions |
|---|---|---|---|---|---|---|
| prism self (src, crates) | 140204 | 133503 | 132914 | **7290** | 0 | 0 |
| tokio ea6d652a | 32647 | 29953 | 30919 | **1728** | 0 | 0 |
| prism whole tree (supplementary) | 257298 | 249910 | 249018 | 8280 | 0 | 0 |
| vitess (control) | 1595019 | n/a | identical | 0 | 0 | 0 |
| django (control) | 191072 | n/a | identical | 0 | 0 | 0 |

Controls: edges, `dfg-stats` and `call-stats` are `cmp`-identical between main and the fold. `call-stats` outside `dfg_labels` main→fold: only `return_flow.return_input_edges`, **prism 8580→8567, tokio 493→487** (as before). 5bc3fdb3→fold: no difference outside `dfg_labels`.
Counters (expanded, pre-dedup), 5bc3fdb3 → fold: prism src write_only 4088→3878, dominated 2631→3430; tokio write_only 1604→849, dominated 1090→879.

**Against 5bc3fdb3** (both directions; `add2.py 5bc fold` flags every restored edge as "added" by construction):

| corpus | restored | by cause (ablation builds, below) | newly removed | newly removed on a use line with only Allowed invocations and no capture of the name |
|---|---|---|---|---|
| prism src | **228** | census refusal 225, format capture 3, owner-off belt 0 | **817** | 634; the other 183 see below |
| tokio | **966** | census refusal 966, capture 0, owner-off 0 | **0** | n/a |
| prism whole tree | 245 | census 242, capture 3 | 1137 | 944 / 193 |

Cause attribution is exact: a local-only ablation build of the fold (env switches `PRISM_F3_NO_CAPTURE`, `PRISM_F3_NO_CENSUS`, `PRISM_F3_NO_OWNEROFF`, not committed; edges present in the fold and absent when a cause is ablated). Every restored edge was attributed to exactly one cause (none overlapped, none unattributed).
**Format captures specifically:** main edges into lines with a `{name}` capture of the def's name that 5bc3fdb3 dropped and the fold restores: **3 on prism** (`member_only_param_test_support.rs 21→22` and `member_only_param_tests.rs 26→27` `formal`, which sit in a statement holding a `format!("{formal}")`, and `js_ts_facts_file_tests.rs 8→10` `body`, the literal itself), **0 on tokio**. The three also match a text search of the use line and the next three lines.
Restored by census: mostly names that appear in a Refused invocation elsewhere in the owner (on prism src 227 of 228 restored edges have no macro on their use line; on tokio 823 of 966 have none, the rest are `assert_ok!`/`assert_ready_ok!`/`poll`-style tokio test macros and `format!`).

**The 183 (prism src; 193 whole tree) newly removed edges that do see a format capture of the name.** My automated check ("no capture of the name in any string literal of the invocation") flags them; I read a sample (below). Each is a D drop where the capture is itself dominated: e.g. `let r = rows(..)` at 1239 is overwritten by a later `let r = rows(..)` and the use `assert!(.., "{language:?}: {r:?}")` reads the later one, so the capture reads the same def as the `r` token on the line. This follows item 2 as written (a capture is "a read occurrence for D's checks", not a keep). If the owner wants captures to keep D unconditionally, it is a one-line change (keep when any capture of the name exists) at a cost of those 183 edges; I did not take that reading because it is not what item 2 says.

**Census verdict table summary** (`data/<corpus>/census.json`; counts are invocations: own path, plus the ones spelled inside token trees marked "soup"):

| class | prism src allowed/refused | tokio allowed/refused |
|---|---|---|
| std list, unqualified | 17081 / 0 (+1118 / 0 soup) | **0 / 3879** (+0 / 309 soup) |
| std-qualified | 1 / 0 | 1 / 0 (soup) |
| in-repo `macro_rules!`, unqualified | 201 / 0 (+62 / 0 soup) | **0 / 2504** (+0 / 419 soup) |
| in-repo path (`crate::`, `$crate::`, ..) | none | 6 / 15 (soup) |
| qualified external (`serde_json::json`, `anyhow::bail`, `std::ptr::addr_of`..) | 0 / 149 (+0 / 39 soup) | 0 / 238 (+0 / 18 soup) |
| unknown unqualified (`json`, `ensure`, `anyhow`..) | 0 / 203 (+0 / 79 soup) | 0 / 336 (+0 / 33 soup) |

Prism src: 15 `macro_rules!` definitions, none generating, `macro_use_extern` false; top Refused spellings `json!`, `serde_json::json!`, the `anyhow` family. Census build cost 351 ms (single-threaded, 961 files). Tokio: 132 definitions; see the next paragraph.

### Finding: one `#[macro_use] extern crate` in an example file refuses every unqualified macro repo-wide
Tokio's census is all-Refused for unqualified spellings because `examples/tinyhttp.rs` has `#[macro_use] extern crate serde_derive;`, and the brief's rule (E2's `has_macro_use_extern_crate`, repo-wide, "unqualified spellings only") makes `macro_use_extern` a repo-wide flag. I kept that as specified. Its recall cost on tokio is large: an ablation build with only that flag ignored (`PRISM_F3_NO_EXTERN`, not committed) removes **2993** edges against main instead of **1728** (+1265), restores only 309 and newly removes 608 against 5bc3fdb3, and **655 of the 966 restored edges against 5bc3fdb3 are due to that one line**. The macro is that example crate's own (an example is a separate crate; `serde_derive` macros cannot reach tokio's library files). A sound refinement would scope the flag to the crate that contains the `extern crate` (crate root files under `examples/`, `tests/`, `benches/` are their own crates). This is a recall follow-up for the owner, not a soundness issue; I did not do it because the brief fixes the E2 rule.

## Samples (seed 7943)
`samples-fold3.jsonl` (60 rows with def and use source text): 20 random removed edges main→fold for each of prism src and tokio, plus the 20 newly removed 5bc3fdb3→fold (prism src; tokio has none). **60 RIGHT, 0 WRONG by my reading.** Limits: I read every def/use pair, opened the source for the ambiguous ones (`framework_params.rs` `target` 430→436, dominated by `let target = unwrap_parenthesized(target)` at 435; `broadcast.rs` `slot` 1060→1083, dominated by the `slot = ..` reassignment at 1064; `go_mod.rs` `index` 79→71, a backward edge with no loop around both and a dominating `index += 1`; `lvalues.rs` `inner` 598→601, shadowed by the next line's `let inner`). The `reason` field is a coarse class computed from the use line (U: the use line only writes or rebinds the name; D: a later dominating write overwrites the def, or a backward edge with no loop), not a per-edge proof. 60 samples support only the sampled edges.

## Timing (`nav --no-cache dfg-stats`, cold, interleaved; wall s, `data/timing.txt`)
tokio 5bc3fdb3 4.5, 4.6; fold 4.7, 4.4 (max RSS 375 vs 376 MB). prism src 32.5 vs 33.1 (1090 vs 1091 MB). prism whole tree 53.3 vs 53.0 (2171 vs 2142 MB). The census is one serial pass before the parallel per-file build (199 ms tokio, 351 ms prism src standalone); no measurable difference.

## Cache parity results (`rust_prune_fold3_tests.rs`, all pass)
- `macro_census_cache_parity_cold_full_partial`: v1 (`a.rs` owner, `b.rs` clean `install_get!`) prunes the pair; **full hit** replays the cold build (edges, census digest equal); `b.rs` flips non-generating→generating: the **partial hit**'s cached DFG carries the old digest (asserted unequal to the fresh one), `build_incremental` falls back to the full rebuild and agrees with a cold build on edges, `dfg_label_stats`, `rd_function_stats` and the digest, and `a.rs` (unchanged) is refused; a census-neutral edit of `b.rs` takes the incremental path and still agrees. **RED:** with the guard disabled (`if false && ..`, local only) the test fails at its "the partial hit refused x in the unchanged a.rs" assertion.
- `census_is_repo_wide_a_generator_in_another_file_keeps` (generator in `b.rs` keeps, the same owner with a clean `b.rs` prunes, alone the unknown spelling keeps), `scoped_build_reads_the_census_from_the_whole_repository` (`build_scoped_subset` over `a.rs` only equals the full build's `a.rs` edges), `census_verdicts_and_digest` (verdict table, cycles, conflicting definitions, std shadow by `use`, `#[macro_use] extern crate`, digest stability, empty digest), `keyword_spelled_macros_are_invocations_in_soup` (`gen!`/`try!` are anonymous tokens in soup and still count).

## Tests, and what I did not run
- `cargo test --workspace --no-fail-fast` at the fold head: all pass except **19** tests that fail identically on 5bc3fdb3 (rerun there by name: `freshness_*` and `capture_tests` in prism-mcp, `dir_chain`, both `owner_activation_test`s, `a_write_that_cannot_start...`, `unreadable_source_and_directory_skip`; this session runs as root, which ignores the mode bits they rely on). The three tests the fold did break were fixed in 609b276: `go_blank_test::{python,javascript}_underscore` (their pinned serialized bytes gain the 8-byte empty-digest tail; `tests/fixtures/go_blank/base.json`), the raw codec builders in `codec_review.rs`/`corruption.rs` (12 wire fields), and `dfg_incremental_test::partial_hit_multifile_edit_retain_delete` (its per-file singleton oracle now builds under the whole tree's census via `build_with_census_files`, as the assembled build does; Rust singletons otherwise saw an unknown spelling that a sibling defines).
- `cargo fmt --all --check` passes. Not run: **mutgate** (no mutant lane exists for the Rust prune; the pre-merge tier-1 `--since <merge-base> --scope fn --authoritative` is still owed, and a new lane for `census.rs`/`scan_macros` is worth adding), the **Tier-A accuracy harness** (the fold touches `src/cpg/build.rs` and `scoped_build.rs`; needs the eval stack), MCP golden transcripts beyond the workspace run.

## Decisions (conservative readings, recorded)
- Branch: the session preamble named `claude/630-ud-rust-fold3-gyt1uk`, the task and brief name `perf/630-ud-rust`; I pushed only to `perf/630-ud-rust` and this branch.
- In-repo qualifiers are only `crate`/`self`/`super`/`$crate`; a `mod`-rooted or workspace-crate-rooted macro path is Refused (no module/crate roots in the census, so adding a `mod` never moves the digest). `ext_bound` covers every defined or std-listed spelling brought under an external-rooted `use` (a superset of E2's listed-only set); `std::`/`core::`/`alloc::` and leading `::std` qualify the std list.
- A metavariable followed by `!` marks a definition generating whatever follows (`$k! name {..}`), and a keyword-spelled macro (`gen!`, `try!`) in soup is an invocation (fail closed).
- Captures count on every line of the enclosing statement (coarse: a closure or arm body without its own block takes the whole outer statement; recall cost only).
- A capture counts as an occurrence for D, so a dominated capture still drops (the 183 above); see that paragraph for the stricter alternative.
- `macro_use_extern` stays repo-wide (the brief's E2 rule); see the finding.

Progress: DONE
