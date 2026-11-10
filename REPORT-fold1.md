# Fold 1 report: PR #794 (#630 slice 5, Rust U and D)

Fold code: branch `perf/630-ud-rust` @ **129ce64** (parent f85523ed). Measured binaries: release builds of origin/main 06ba0968, f85523ed and 129ce64, RAYON 4, `nav --no-cache dfg-stats --edges` / `call-stats`, `add2.py`. Corpora: prism self (`git archive 06ba0968 src crates`), tokio 1.40.0 shallow clone `ea6d652a` (parse only), vitess `bd5fbe25…98a9` and django `462e6302…bd1d8` from release `corpora-v1` (sha256 checked against SHA256SUMS). Never built or ran corpus code.

## Decisions (conservative readings)
- `self`: tree-sitter-rust `self` is not an `identifier` node and the occurrence collector never sees it, so `self.*` paths are **left unproven** (U and D keep them); documented in CLAUDE.md and the module doc, row `03_fold1` `self.f 67→69` keep.
- Stable base also refuses `&mut base` borrows (a borrow can re-point it, e.g. `mem::swap(&mut s, ..)`) and macro token trees containing a binding token (`let for match | => = mut ref` and compound assignment); both beyond the brief, both keep the pair. Method-call auto-ref re-pointing (`impl Reset for Box<S>`) is out of model, disclosed.
- `js_prune_tests.rs` is not on main (no JS PR merged in this tree), so I wrote the owner-keyed check myself in `rust_prune_tests.rs` (rows may carry an optional `owner`): keep row = every owner that had the pair before still has it; drop row = no owner has it after (or, with `owner`, none of that owner).
- Item 3 is docs only, as instructed.

## Disposition table
| finding | disposition | commit | RED row (fails on f85523ed) |
|---|---|---|---|
| sol 1 / Opus W1 field-path D, re-pointed base | fixed: `rust_prune::stable_bases` gate in `keep_def` | 129ce64 | `03_fold1 s.f 3→7` (re-point), `12→17` (Box move/restore), `22→25` (`if let` rebinding), `31→34` (second `let`), `38→41` (closure parameter), `59→62` (`&mut s`); `00_basic s.f 46→48` flipped to keep (base assigned) |
| Opus S1 pattern bindings record no def | fixed by the same gate (any pattern binding makes the base unstable) | 129ce64 | rows above |
| Opus S2 `self.a.b` claim | fixed: `self.*` documented as never proven; CLAUDE.md claim corrected | 129ce64 | `03_fold1 self.f 67→69` keep (pin, green both sides) |
| sol 2 hygienic macro reads | fixed: `has_local_macro` disables D for an owner with a `macro_definition` anywhere | 129ce64 | `03_fold1 x 74→77` |
| sol 3 survivor cap | disclosed (CLAUDE.md Rust paragraph and `rust_prune.rs` module doc, same sentence as Go/Python/JS), no code change | 129ce64 | n/a |
| sol 4 / Opus S3 return_input evidence | explained edge by edge below (U anchors, 0 added) | n/a | n/a |
| Opus S4 sample custody | fixed: `samples-fold1.jsonl` pushed here | n/a | n/a |
| Opus S5 missing rows | added: block expression, tail assignment, macro `let` between W and U, local macro with U | 129ce64 | `03_fold1` rows (green on both; they are pins) |
| Opus S6 invalid `closure_writer` | fixed: `c()` now before `x = 2` (valid Rust, still tests the captured writer) | 129ce64 | n/a |
| Opus S7 stale `lexical_prune.rs` module doc | fixed ("Go, Python and Rust") | 129ce64 | n/a |
| sol "nested defs always kept" wording | fixed in CLAUDE.md and module doc (U can drop a nested def's edge into an outer write-only line) | 129ce64 | `03_fold1` rows `owner=inner` drop / `owner=nested_fn_owner` keep |
| item 6 owner-keyed obligations | fixed (harness above) | 129ce64 | the nested-fn pair is dropped by owner `inner` but kept by the enclosing owner; the collapsed harness could not say that |
| Opus "U unaffected by macros" confirmation | row `03_fold1 x 88→89` (local macro_rules, macro read on the use line) keeps | 129ce64 | pin |

RED check: with `rust_prune.rs`/`lexical_prune.rs` restored to f85523ed the matrix fails exactly on 8 rows (`00_basic` 46→48 and `03_fold1` 3→7, 12→17, 22→25, 31→34, 38→41, 59→62, 74→77). Tests at the fold head: `--lib data_flow` 551 pass, `--lib cpg::` 838 pass, `cargo fmt --all --check` clean. Not run: full suite, mutgate, Tier-A.

## Item 4: ReturnInput edges (Step 5c), main vs fold, edge by edge
Method: throwaway example (`nav_session` CPG, every `CpgEdge::ReturnInput`, Use node file/function/line/path/bytes, whether the Use is zero-width, and its DataFlow in-degree) built per revision, outputs diffed on identity (in-degree excluded). prism 8580→8567: **13 removed, 0 added**; tokio 493→487: **6 removed, 0 added**; f85523ed and the fold head are identical to each other on both corpora. All 19 removed edges start at a **zero-width anchor Use** (no rvalue span) on a write-only line (`x = ..;`, `let x = ..` where the line has no read of the name) that lies inside a multi-line return/tail value: U no longer mints the anchor, so it is no longer a ReturnInput source. None carried a value path: each line only writes the name; the real reads of the same names keep their own ReturnInput edges. D cannot remove Use nodes, so the earlier D-based explanation was wrong; the mechanism is U, as the JS and Python slices disclose. No WRONG.

| corpus | use site | name / owner | return line | Use provenance | source line |
|---|---|---|---|---|---|
| prism | coordinator.rs:360 | `control` in `ensure_serving` | 331 | anchor (zero-width) | `control = self` |
| prism | idle_stdio.rs:130 | `response` in `serve_events` | 118 | anchor (zero-width) | `let response = if let Some(message) = message {` |
| prism | arbitrate.rs:308 | `owner` in `arbitrate` | 279 | anchor (zero-width) | `owner = Owner::Free;` |
| prism | arbitrate.rs:315 | `child` in `arbitrate` | 279 | anchor (zero-width) | `child = Some(starter.start(&dir).map_err(Failure::SpawnFaile` |
| prism | arbitrate.rs:319 | `owner` in `arbitrate` | 279 | anchor (zero-width) | `owner = Owner::Held;` |
| prism | alias_edges.rs:552 | `node` in `unwrap_js_ts_alias_rhs` | 549 | anchor (zero-width) | `node = unwrapped;` |
| prism | alias_edges.rs:556 | `next` in `unwrap_js_ts_alias_rhs` | 549 | anchor (zero-width) | `let next = match node.kind() {` |
| prism | js_binding_alias.rs:65 | `spelling` in `js_ts_follow_alias` | 52 | anchor (zero-width) | `spelling = self.node_text(&next).to_string();` |
| prism | js_binding_alias.rs:66 | `at` in `js_ts_follow_alias` | 52 | anchor (zero-width) | `at = next;` |
| prism | js_binding_alias.rs:215 | `value` in `js_ts_unwrap_value` | 206 | anchor (zero-width) | `value = inner?;` |
| prism | shadow_index.rs:128 | `visible` in `build` | 126 | anchor (zero-width) | `let mut visible = stack.last().copied().flatten();` |
| prism | shadow_index.rs:152 | `visible` in `build` | 126 | anchor (zero-width) | `visible = Some(scopes.len() as u32 - 1);` |
| prism | receiver_index.rs:249 | `operand` in `decompose_go_selector_chain` | 240 | anchor (zero-width) | `let operand = cur.child_by_field_name("operand")?;` |
| tokio | stream_panic.rs:17 | `panic_file` in `test_panic` | 15 | anchor (zero-width) | `let panic_file: Arc<Mutex<Option<String>>> = Arc::new(Mutex:` |
| tokio | panic.rs:26 | `panic_file` in `test_panic` | 24 | anchor (zero-width) | `let panic_file: Arc<Mutex<Option<String>>> = Arc::new(Mutex:` |
| tokio | block.rs:99 | `block` in `new` | 96 | anchor (zero-width) | `let block = std::alloc::alloc(Layout::new::<Block<T>>()) as ` |
| tokio | list.rs:171 | `try_updating_tail` in `find_block` | 124 | anchor (zero-width) | `try_updating_tail = false;` |
| tokio | list.rs:175 | `block_ptr` in `find_block` | 124 | anchor (zero-width) | `block_ptr = next_block.as_ptr();` |
| tokio | panic.rs:9 | `panic_file` in `test_panic` | 7 | anchor (zero-width) | `let panic_file: Arc<Mutex<Option<String>>> = Arc::new(Mutex:` |

The raw per-edge dumps (with byte offsets) are in `returninput-prism.json` and `returninput-tokio.json`.

## Measurement
| corpus | main edges | f85523ed | fold | removed (main→fold) | added | kept-label transitions |
|---|---|---|---|---|---|---|
| prism self | 139918 | 132041 | 132045 | 7873 | 0 | 0 |
| tokio 1.40.0 | 32647 | 28660 | 28660 | 3987 | 0 | 0 |
| vitess (control) | 1573063 | identical | identical | 0 | 0 | 0 |
| django (control) | 187316 | identical | identical | 0 | 0 | 0 |

Controls: edges and `call-stats` are `cmp`-identical across main, f85523ed and fold. `call-stats` outside `dfg_labels` differs only in `return_flow.return_input_edges` (prism 8580→8567, tokio 493→487), explained above; 8 `dfg_labels` counters move on each corpus, as before.

**Against f85523ed (fold may only restore):** prism: fold minus f85523ed = **4**, f85523ed minus fold = **0**; tokio: 0 and 0. The 4 restored edges are item-1 field paths, all `nameonly/killed`: `control.waiting` 274→365 and 280→365 (daemon/coordinator.rs), `cfg.first_call_wait` 51→56 (idle_fix_tests.rs), `evidence.conflict` 76→131 (go/owner_partition_s4.rs). Item 2 restored **0** edges on both corpora (no owner with a local `macro_rules!` lost a drop; identifier-path removals are 7765 on prism and 3952 on tokio in both f85523ed and fold). No newly removed edge: no stop-and-explain.

**Counters (fold):** prism write_only 4079, dominated 3812 (f85523ed 4079 / 3816); tokio write_only 1604, dominated 2383 (unchanged). Counters count expanded edges before dedup, so they exceed distinct removals.

**D/U split by path kind (distinct removed edges, main→fold):** prism 108 field-path, 7765 identifier (f85523ed: 112 / 7765); tokio 35 field-path, 3952 identifier (unchanged). The surviving field-path removals are U drops or D drops with a stable base.

## Samples (seed 794, 20 random removed edges per corpus, main minus fold)
`samples-fold1.jsonl` (40 rows: corpus, edge, verdict, one-line reason). Controls have no removals. **40 RIGHT, 0 WRONG** (prism 20, tokio 20). Categories: U into a write-only declaration or assignment line (22), D by a dominating `let` shadow or assignment (8; two are dominated through a compound `+=`, whose value path runs through the kept compound def), backward straight-line edges with no loop (10). One sample is a nested closure def into an outer write-only `let` (documented U-into-outer). 40 samples bound the wrong-drop rate below about 7% at 95%; they support only the sampled removals, not universal soundness.

## Timing (tokio, `nav --no-cache dfg-stats`, RAYON 4, 2+2 interleaved, wall s)
main 5.38, 5.54; fold 5.81, 5.87 (+~8%, 4 runs, noisy shared host; f85523ed was +3% in the earlier report). The added work is one owner walk (`stable_bases`, `has_local_macro`) per Rust owner.

Progress: DONE
