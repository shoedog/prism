# Fold 4 report: PR #794 (#630 slice 5, Rust U and D), the fold-3 confirmation findings

Code: branch `perf/630-ud-rust`, plain push, no force-push. Base `69b8da37`; fold commit **0c77bd9** (code, docs, rows). Cache stays **v172** (the digest format is unchanged; the census now holds more names, so a digest differs where soup-defined macros exist).
Measured binaries: release builds of origin/main `8cc4985` (the main merged into the branch), `69b8da3` and `0c77bd9`; `nav --no-cache dfg-stats --edges`. Corpora: prism self = `git archive 72be3b73 src crates` (fold-2/3 convention, main 140204 edges as before), tokio `ea6d652a` (main 32647 edges as before). Controls: see below.

## Items (all in 0c77bd9)
1. **Format captures in nested blocks and closures.** `rust_prune::statement_lines` now returns the lines of EVERY ancestor statement up to the owner (each ancestor that is a child of a `block`), instead of only the innermost block's child. Monotone toward keeping.
   - RED rows (fail on 69b8da37): `label_if_let` (`name = if let Some(p) = prefix {..format!("{p}::{name}")..}`), `label_closure` (block-bodied closure `.map(|i| {..format!("{name}{i}")..})`), `label_match` (braced match arm). Control `label_dominated` (a later direct write dominates the capture) still drops.
2. **`macro_rules!` in token trees outside owners / in definition bodies.** `census.rs::walk` scans every `token_tree` (module level, `cfg_if!` bodies, owners) and every definition RHS; each `macro_rules ! NAME` adds NAME to `defined` AND `generating` (fail closed); both sets are in the digest already.
   - RED rows: `soup_def_clean_elsewhere` (`identity! { macro_rules! install_soup {..} }` with a clean same-spelling definition in a module, then `install_soup!(x); x = get!() + 1;`), `soup_def_std_spelling` (soup-defined `assert` used as `assert!(x)`), `soup_def_cfg_if` (the same under `cfg_if::cfg_if!`).
3. **Comments inside token soup.** `scan_soup` drops `line_comment`/`block_comment` children before invocation, path and metavariable recognition (the same function serves the definition-body generating check).
   - RED rows: `comment_in_owner_soup` (`identity! { gen_cmt /* c */ !(x); }`), `comment_in_definition_body` (transitive: body `gen_cmt /* c */ !($v)`), `comment_after_bang` (comment between `!` and the tree, in a definition body), `comment_after_metavariable` (`$m /* c */ ! ($v)`), `comment_between_bang_and_tree_in_soup` (owner soup).
4. **Format grammar.** `format_captures` decodes cooked literals first (`\n \r \t \\ \0 \' \"`, `\xHH`, `\u{..}`, line continuation; raw literals stay raw) and parses the spec by `[[fill]align][sign]['#']['0'][width]['.' precision][type]` (a named width/precision argument is a capture), unioned with the old loose `name$` scan so malformed specs stay conservative.
   - RED rows: `format_zero_width` (`{:0x$}`), `format_all_flags` (`{:>+#0x$.y$}`), `format_hex_escape` (`"\x7bx}b"`), `format_unicode_escape` (`"\u{7b}x}b"`). Controls still drop: `{{x}}` (`escaped_brace_is_no_capture`, fold 3), raw `r"{{x}}"` (`format_raw_double_brace`), `{:5}` (`format_plain_width`).
5. **Raw identifiers and comments around attributes.** `ext_bound` strips `r#` from import spellings before relevance and alias checks; `lexical_prune` strips `r#` from the DFG path name before matching refused names (and, my conservative extension, the mutably-borrowed set, whose scan now stores stripped spellings); `#[macro_use]` on an `extern crate` skips comment siblings.
   - RED rows: `raw_alias_import` (`use ext::install_get as r#assert;` then `assert!(x)`, file `07_fold4_import.rs`), `macro_use_comment` (`#[macro_use]` `/* c */` `extern crate ext;`, file `08_fold4_extern.rs`).
   - **Not RED:** `raw_local` (a `r#type` formal passed to `ext::gen!(r#type)`) PASSES on 69b8da37 too. Raw-identifier locals are never pruned by U or D at all (a plain `r#type = 5;` after a `mut r#type` formal keeps its pair even with no macro on 69b8da37: `LexicalFacts` and the DFG spell the name differently upstream), and a raw `let` has no DFG edges in the baseline. The `r#` strip is therefore defensive (the mismatch cannot lose an edge today); the row is kept as a pin, not as a RED row. Recorded, not hidden.
6. **Disclosed, not fixed:** external glob imports (`use ext::*;`) under a spelling with an unrelated clean in-repo definition out of textual scope, added to CLAUDE.md, the `rust_prune.rs` module doc and the `census.rs` module doc beside the #772 family. Also added the proc-macro sentence to the name-builder disclosure (a proc macro can emit a call-site identifier from nothing, `quote!(x)`), and fixed the `src/cpg_cache.rs` v172 changelog wording (a capture adds an occurrence D may dominate; it does not unconditionally keep the pair).

RED check: with `src/` stashed back to 69b8da37 and the new fixtures and `obligations.jsonl`, `rust_prune_matrix` fails on **17 of the 18 keep rows** (every keep row above except `raw_local`); with the fix all rows pass (`cargo test --lib rust_prune`: 7 pass). Fixtures: `tests/fixtures/dfg_prune/rust/06_fold4.rs`, `07_fold4_import.rs`, `08_fold4_extern.rs`; rows appended to `obligations.jsonl`.

## Measurement
Against **main** (add2.py; removed only, 0 added, 0 kept-label transitions):

| corpus | main edges | 69b8da37 | fold 4 | removed main to fold 4 | added | kept-label transitions |
|---|---|---|---|---|---|---|
| prism self (src, crates) | 140204 | 132914 | 133128 | **7076** | 0 | 0 |
| tokio ea6d652a | 32647 | 30919 | 30919 | **1728** | 0 | 0 |

Controls: the repo's non-Rust fixture tree (`git ls-files tests/fixtures` without `.rs` and secbench: 1145 files, Go/Python/JS/TS/C/...; vitess and django were not available offline with their pinned shas, so this substitutes for them): main, 69b8da37 and fold 4 edge outputs are `cmp`-identical (7362 edges each). The diff touches only Rust-gated code.

Against **69b8da37** (this fold may only restore):

| corpus | restored | newly removed |
|---|---|---|
| prism src | **214** | **0** |
| tokio | 0 | 0 |

Restored by item (ablation build `wt-abl` of the fold head with env switches `PRISM_F4_NO_NEST|SOUPDEF|COMMENT|FMT|RAW`, not committed; restored = edges in the fold and absent with that item ablated): **item 1 (nested captures) 214**, items 2, 3, 4, 5: **0** each; all five switched off reproduces 69b8da37's edge multiset exactly (0 and 0), and no ablation yields an edge the fold lacks. Items 2 to 5 close real holes in the verdict logic but no prism-src or tokio use line exercises them (no soup-defined macro, no comment in a soup, no escaped format string, no raw alias). Tokio restores nothing because its census is all-Refused for unqualified macros (the repo-wide `#[macro_use] extern crate` in `examples/tinyhttp.rs`, fold 3's finding, unchanged).

## Samples (seed 7944)
`samples-fold4.jsonl`: 20 random removed edges main to fold 4 per corpus (prism src, tokio), with def and use source text. **40 RIGHT, 0 WRONG by my reading.** Limits: I read the use line of all 40 (21 are a plain write or `let` rebind of the name, rule U) and opened the surrounding source for 8 of the others (`contract_slice.rs`, `go_scan.rs`, `sqla.rs`, tokio `time/tests/mod.rs`, `process/mod.rs`, `net_named_pipe.rs`, `idle_process_test.rs`, `c_jumps_tests.rs`), all a later write or `let` rebind that overwrites the def before the use (rule D); the remaining D samples are the same shape (a parameter or `let` followed by `let n = ..`, a later `x = ..` before the use) and were judged from the def/use lines without opening the source. The "reason" field is by pattern, not by a per-edge trace.

## Timing (`nav --no-cache dfg-stats`, cold, interleaved, wall s, `data/fold4/timing.txt`)
tokio 69b8da37: 4.39, 4.46; fold 4: 4.65, 4.24. No measurable difference (the new work is one extra token-tree scan in the census pass and a decoded string per format literal).

## Tests, and what I did not run
- `cargo test --lib`: 3076 pass, 2 fail (`cpg_cache::stream::tests::a_write_that_cannot_start_fails_and_leaves_the_previous_cache_usable`, `resolution::js_ts::paths_snapshot::boundary::tests::unreadable_source_and_directory_skip`), both file-mode tests that fail identically before this fold (this session runs as root, as fold 3 recorded). `cargo test --test ast dfg_prune` passes; `cargo fmt --all --check` passes. I did not rerun the whole workspace (the diff is in the prism core library only; `prism-review` and `prism-mcp` were not changed).
- Not run: mutgate (no lane for the Rust prune or census exists; the pre-merge tier-1 gate is still owed), the Tier-A accuracy harness (needs the eval stack; the fold touches no resolution or CPG construction code), MCP golden transcripts.

## Decisions (conservative readings, recorded)
- Item 2 counts a soup-defined name whose position holds a metavariable (`macro_rules! $n`) as nothing: it can only occur in a definition body, whose definition is already generating. Every `macro_rules ! NAME` in any token tree of any file, owners included, adds NAME (broader than "outside owners", fail closed; recall cost only on a same-spelling collision).
- Item 5's `r#` strip also covers the mutably-borrowed set (same class, keep direction).
- Item 4 keeps the old loose scan as a union with the grammar parser.
- The cache number stays v172; a cached DFG built before this fold carries the older census digest where a soup-defined macro exists, and a partial hit rebuilds on mismatch as designed. A full hit with unchanged file hashes would replay a v172 cache written by 69b8da37 (not released; the branch is unmerged).

Progress: DONE
