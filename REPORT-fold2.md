# Fold 2 report: PR #794 (#630 slice 5, Rust U and D), structural closure

Code: branch `perf/630-ud-rust` @ **5bc3fdb** (merge commit f1b2112 of origin/main 72be3b73 into 129ce64, then the fold commit). Plain pushes, no force-push. Measured: release builds of origin/main 72be3b73, 129ce64 and 5bc3fdb, `RAYON_NUM_THREADS=4`, `nav --no-cache dfg-stats --edges` / `call-stats`. Corpora: prism self (`git archive 72be3b73 src crates`), tokio 1.40.0 `ea6d652a` (parse only), controls vitess `14349b4d` and django `6a8248f1` (**shallow clones of current HEAD, not the fold-1 `corpora-v1` release**, which was not reachable from this session; controls are compared binary against binary, so the pin does not matter for the identity check). Never built or ran corpus code. `add2.py` was not used: its checks (added edges, kept-label transitions, removed counts) were redone with an equivalent script on the same identity key (edge minus label fields) and label fields.

## Step 0 (merge)
Conflicts in `src/cpg_cache.rs`, `lexical_prune.rs`, `lexical_prune_tests.rs` resolved keeping both JS and Rust hooks (CLAUDE.md merged cleanly, both paragraphs kept). Cache bumped to **v171**: doc line (v170 JS then v171 Rust), const, in-file pin test, `dfg_codec_tests.rs` (x3 + `owned.version`), CLAUDE.md sentence. After the merge `--lib data_flow` passed.

## Changes (items 1-6)
1. Rust D proves identifier paths only: `rust_prune::stable_bases`, `BINDING_TOKENS`, `in_binding_macro` and the `rust_stable_bases` gate are deleted; `keep_def` keeps every Rust field path. U for field paths is unchanged.
2. U and D are both off for an owner with a `macro_definition` anywhere (`enabled` is false; the `rust_local_macro` flag is gone).
3. D keeps a pair whose use line has an occurrence of the name inside a macro token tree (`rust_macro_uses`, collected per owner).
4. D keeps a name mutably borrowed anywhere in the owner (`rust_prune::mutably_borrowed`): `&mut`/`&raw mut` on a place rooted at the name (field, index, deref, parentheses, `?`), `ref mut` patterns (`ref_pattern`/`field_pattern` holding `mut`) in `let`/`if let`/`match`/`for` mark the scrutinee root, and in a macro token tree every identifier beside a direct `mutable_specifier` child. Nested callables are included (whole-owner scan).
5. CLAUDE.md and the `rust_prune.rs` module doc: the sentence "never resurrecting a dropped def" is replaced by the accepted unseen-writer contract (auto-ref `&mut self` method, `clone_from`, interior mutability, `unsafe`, macro-expanded writes; Go design §2.2 analogue). The token list now matches the code (the dead `mut`/`ref` entries are gone).
6. Harness: drop rows now require `!had.is_empty()` after owner filtering; `03_fold1.rs` `second_let` and `swap_base` are valid Rust (`let mut`, `mut t`); new fixture `04_fold2.rs` (24 rows).

## Disposition (round 2)
All "RED" rows were run against 129ce64 sources with the fold-2 fixtures and harness copied in: `rust_prune_matrix` failed on exactly the rows below (every new keep row plus the flipped rows); the two controls (`plain_use_control`, `unborrowed_control`, both drop rows) pass on both sides.

| finding | disposition | commit | RED row (fails on 129ce64) |
|---|---|---|---|
| sol r2 #1 field path, deref/prefix restore, auto-ref | fixed (item 1: field paths never get D) | 5bc3fdb | `04_fold2 s.f 17→21` (deref restore), `s.p.f 26→31` (prefix restore), `s.f 37→41` (auto-ref `Exchange`) |
| sol r2 #2 generated local macro | fixed (item 3, plus item 2 for explicit ones) | 5bc3fdb | `04_fold2 x 102→105` (`install_pick!`) |
| sol r2 #3 / Opus S2 vacuous drop rows | fixed (item 6 harness) | 5bc3fdb | no row: it is a harness change; the two owner rows (`inner`, `nested_fn_owner`) are non-vacuous |
| Opus W1 strict prefix re-pointed | fixed (item 1) | 5bc3fdb | `04_fold2 s.g.h 45→49` |
| Opus W2 `&mut &mut S` formal | fixed (item 1) | 5bc3fdb | `04_fold2 s.f 53→57` |
| Opus W3 `&mut s` in a macro | fixed (item 1; item 4 for the identifier case) | 5bc3fdb | `04_fold2 s.f 63→67`, `x 141→146` (`dbg!(swap(&mut x,..))`) |
| Opus W4 `ref mut` borrow | fixed (item 1 for fields, item 4 for identifiers) | 5bc3fdb | `04_fold2 s.f 73→77`, `x 150→154` |
| Opus W5 U beside a hygienic `macro_rules!` | fixed (item 2) | 5bc3fdb | `04_fold2 x 87→89` |
| Opus S1 unseen writers / false "never resurrecting" | explicit forms fixed (item 4); the rest disclosed (item 5) | 5bc3fdb | `x 116→121` (swap before and after W), `125→129` (before), `133→137` (after), `158→161` (`&raw mut`); auto-ref on identifiers, `clone_from` and interior mutability are disclosed, not fixed |
| Opus S3 docs vs code token list | fixed (item 1 deleted the list; docs rewritten) | 5bc3fdb | n/a |
| Opus S4 non-compiling fixtures | fixed | 5bc3fdb | n/a (syntactic rows unchanged) |
| Opus S5 / sol round-1 sample custody | `samples-fold2.jsonl` pushed here | n/a | n/a |
| rows flipped by the fold (recall cost, report items 1 and 3) | `00_basic`-style `03_fold1 s.f 46→48`, `52→54` (field-path D off), `03_fold1 x 81→83` (println use line), `02_hazards x 114→116` (macro read use line) now keep | 5bc3fdb | all four fail on 129ce64 (they dropped there) |

## Measurement
| corpus | main edges | 129ce64 | fold | removed main→fold | added | kept-label transitions |
|---|---|---|---|---|---|---|
| prism self | 140204 | 132301 | 133503 | 6701 | 0 | 0 |
| tokio 1.40.0 | 32647 | 28660 | 29953 | 2694 | 0 | 0 |
| vitess (control) | 1593876 | not comparable | identical to main | 0 | 0 | 0 |
| django (control) | 191061 | not comparable | identical to main | 0 | 0 | 0 |

Controls: edges and `call-stats` are `cmp`-identical between main and the fold. 129ce64 differs from main on vitess and django only because it predates the merged JS/TS U/D (the cache and JS pruning came with main), so those two columns are not a control for it.

`call-stats` outside `dfg_labels`, main→fold: only `return_flow.return_input_edges`, **prism 8580→8567, tokio 493→487**, the same numbers as fold 1 (U anchors, 0 added); no new ReturnInput difference. 129ce64→fold: no difference outside `dfg_labels` on either corpus. `dfg_labels` counters move (8 on each corpus against main, 5 and 6 against 129ce64), as before.

**Against 129ce64 (fold may only restore):** prism: fold minus 129ce64 = **1202**, 129ce64 minus fold = **0**; tokio: **1293** and **0**. No newly removed edge: no stop-and-explain. All restored edges were `exact`/`nameonly` as emitted by main (no label changes anywhere).

**Cost of item 3 and the rest (restored edges against 129ce64, by item).** Heuristic attribution from the source text of the owner and use line (first match in order 1, 2, 3, 4), because the binary has no per-item switches; the unattributed rest are continuation lines of multi-line macro invocations (`assert_eq!(` arguments) and owners that borrow the name elsewhere:

| corpus | restored | 1 field path | 2 local macro_rules | 3 macro token use line | 4 mutably borrowed | unattributed |
|---|---|---|---|---|---|---|
| prism | 1202 | 2 | 5 | 612 | 213 | 370 |
| tokio | 1293 | 1 | 1 | 927 | 182 | 182 |

Item 3 is the dominant recall cost (`println!`/`assert!`/`vec!` use lines lose D). Counters (`dfg_pruned_*`, expanded edges before dedup): prism write_only 4088, dominated **2631** (129ce64: 3833); tokio write_only 1604, dominated **1090** (129ce64: 2383). U is unchanged.

**Identifier / field split of removed edges main→fold:** prism 6594 identifier, 107 field path (129ce64: 7765 / ~108 by fold 1); tokio 2660 identifier, 34 field path. The surviving field-path removals are U drops only.

## Samples (seed 7942, 20 random removed edges per corpus, main minus fold)
`samples-fold2.jsonl`, 40 rows with def and use source text. **40 RIGHT, 0 WRONG by my reading.** Honest limits: I read every def/use line pair, and opened the source for the two least obvious (call_classify.rs 213→215: a parameter shadowed by `let node = unwrap_parenthesized(*node)` on the next line, RIGHT; lexical_prune.rs `node` 180→266: the def is an anchor on a multi-line `let` inside a returning `if` branch, RIGHT but unusual). The `reason` field is a coarse one-line class, not a per-edge proof. Categories: use line only writes or rebinds the name (U), shadowing `let` or dominating write (D), backward straight-line edge with no enclosing loop. 40 samples support only the sampled removals.

## Timing (tokio, `nav --no-cache dfg-stats`, RAYON 4, 2+2 interleaved, wall s)
main 5.94, 5.67; fold 5.73, 5.87. No measurable difference (noise on a shared host).

## Tests and not run
`cargo test --lib -- data_flow cpg_cache` at the fold head: 628 passed, 1 failed, `cpg_cache::stream::tests::a_write_that_cannot_start_fails_and_leaves_the_previous_cache_usable`. It expects a write into a mode-0555 directory to fail; this session runs as root (uid 0), which ignores the mode, so I read it as environmental, but I did **not** confirm it on main. `cargo fmt --all` was run. Not run: the full workspace suite, mutgate, Tier-A harness, MCP tests. The PR still owes the mutgate pre-merge tier (`--since <merge-base> --scope fn --authoritative`, plus the Rust lane if any).

## Decisions (conservative readings)
- Item 4's token-tree rule is broader than the brief: every identifier directly beside a `mut` token in the same tree counts, not only the one after `&mut`/`ref mut`. Recall cost only.
- `ref mut` marks the scrutinee root of the nearest `let`/`let_condition`/`match`/`for`; a `ref mut` pattern with no such holder marks nothing.
- `has_local_macro` also disables U and D for a nested `fn` that defines a macro, and for its enclosing owner (whole-owner scan).
- Controls are fresh shallow clones, see above.

Progress: DONE
