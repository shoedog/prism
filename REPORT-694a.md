# REPORT-694a: Go producer defs for range/receive `:=`, type-switch aliases, later var/const names (#694 slice A)

Branch `fix/694a-go-nodef-defs` (pushed, plain push; 5 commits on origin/main c639385). Cache 154 -> 155.
Measurements: `vitess.md`, `kubernetes.md` (+ sample files), django control below. Receipt diffs: `receipts.txt`, `receipts-blank.txt`.

## What changed
- `src/ast/go_header_defs.rs` (new): `collect_go_header_def_spans` adds, to the Go lvalue spans, a def at the identifier of every range/receive `:=` target, every type-switch alias and every `var`/`const` name the Assignments query skipped (all names of all direct and grouped specs; the first name of the first direct spec was already a def). One 2-line hook in `ast.rs::assignment_lvalue_spans_on_lines` (ast.rs does not grow otherwise). The right-hand side's reads sit on the same line, so `x := e` feeding applies unchanged: range expression, channel, guard operand, positional var/const initializers, one multi-value call feeding every name.
- **Type-switch alias representation (contract 2): ONE def at the header identifier (so guard reads and the def share a line, which is how taint/queries propagate), grouped with the alias's per-clause declarations.** `BindingFacts` attaches it to the first clause's declaration and records the group keyed by the DEF INDEX (`alias_groups`, `def_reaches_declaration`); `relation_at` (RD) and rule S treat a use resolving to any clause declaration (including `default`) as reached by it, and a use outside the switch as a different declaration. The group hangs off the header def only: an ordinary write inside one clause (`default: v = 1`) reaches just that clause's declaration (found and fixed during the receipt re-baseline: `type_alias_arm_write.go` went Exact when the group was keyed by declaration).
- Rule S: `address_taken_names` no longer keeps the four forms (function-literal formals and named results keep, slice B). `is_declaring_name` / `is_plain_write_site` know range/receive `:=` targets and the alias.
- Rule U (`lexical_prune.rs`): range/receive `:=` targets and aliases are plain writes, so a header that only declares the name is write-only; a header that also reads (`for _, v := range v`) is not. Inside nested callables U keeps its caution for ordinary writes, but a declare-only header is write-only there too (the rvalue collector records a read at the target's own identifier inside closures; ignored for header targets).
- Rule D: header targets are never dominating writes (`window()` skips them), so a use after the loop or select still sees the outer def; they are clause-local through S/RD. Rule O is unchanged (the new defs are plain writes of their declaration by S's own check; O re-entry requires the same declaration).
- No alias twin at any new def: the legacy flow-insensitive alias map (`b = a` anywhere) otherwise minted a twin of `a` at `var a, b`, `for _, b := range` and so on (`go_header_target_bytes`, both twin sites in `file.rs`, mirrored in the frozen oracle `legacy_producer.rs`).
- RD (contract 5): `go_header_writes` HeaderWrites feed the field-write contribution lane only; the new defs are ordinary RD defs, so the two lanes do not double kill. Kills/labels: a new header def kills earlier defs of the same declaration only; across declarations the S/RD binding relation keeps them apart. All receipts/goldens that exercise HeaderWrite still pass.

## Tests
RED first (`b154c19`, fails on main: 5 of 6 rows): `go_nodef_defs_tests.rs` (range, two-name range/receive, `for i := range n`, receive, per-clause/default alias, `var a, b = 1, x`, grouped var, later `var`, implicit const, `var a, b = g()`, self-reading `switch v := v.(type)` / `for _, v := range v` / `case v := <-v`, indirect copy, independent guard, `var _, v = 0, v`, literal formals and named results still exempt) plus rows added with the fixes: declare-only headers (rule U), arm write, no alias twin (range and later var names), D/U scoping after loop/select/switch, closures. The no-twin rows were mutation-checked by hand (fail without the guard). `scope_prune_tests` rows that pinned the old categorical keep now pin the drop; literal formal and named result rows kept.

Results (RAYON 4 and 1): `--lib data_flow` 409 pass; `--test ast` 584, `--test lang_go` 324, `--test navigation` 281, `--test algo_taxonomy` 422 pass; `--lib cpg`, `--lib cpg_cache`, `--test integration` pass except the pre-existing failures below. No mutgate, no full suite (per brief).
Pre-existing (also on origin/main in a worktree, same failures): `cpg_cache::stream::..a_write_that_cannot_start..` (root ignores 0555 dirs), 5x `owner_activation_test` ("trailing characters" parsing a fixture), `build_profile_round2_test::rss_intervals_exclude_cfg_in_both_owner_orders` (fails inside the full integration run on main as well, passes alone on both).

## Goldens (per-edge justification: `receipts.txt`, `receipts-blank.txt`)
- go_fold_r2 (16 files): +28/-99 edge rows (multiset, duplicates counted), 4 label moves. Removed rows are all (a) outer or twin def -> use of a different declaration the body now gets through the new def, or (b) def -> declare-only header line (rule U); added rows are the new def -> body uses (`v@6 -> v@7` etc.). The 4 label moves are one fixture, `type_alias_same_init.go` (`switch v := y; v := v.(type)`: the initializer def and the alias def share line 6): `NameOnly(Killed)` -> `NameOnly(SameLine)` on init-def -> clause uses (both non-Exact, never an upgrade). Test change: `go_fold_r2_outer_writes..` threshold 15 -> 8 (25 -> 10 observed pairs: 15 `distinct` rows no longer produce a pair at all), `type_switch_visibility` expects the outer-constant -> body pair dropped when the alias is present (was a non-Exact label).
- go_fold_r1 (2 files, +3/-6): `control_receive.go` (the old killed(6) outer-v -> inner-v pair is dropped), `control_type_alias.go`. `go_fold_r1_real_binding_controls` now asserts the pair is absent.
- go_branches (3 files, +3/-4): `select_receive_declaration`, `type_alias`, `edge_cases`: new def -> body use, old use -> header-line anchors dropped.
- go_jumps `edge_cases.go` `rangeBreak` (+1/-2): `v@26 -> v@27` added (range target), the two twin-def edges into the range header line (`v@27 -> v@26`, `v@29 -> v@26`) dropped by U; stats updated.
- go_blank `base.json` (5 cases: const_iota, range_map, range_value, select_declaration, var_grouped): new defs/edges only (a@5, b@7 const; k@3; v@3; ok@4/6 with their body edges; a@5); select_declaration lost its two cross-clause header edges.
No Exact label changed to another label in any receipt.

## Measurement (main c639385 release vs head, 4 threads; corpora verified against SHA256SUMS asset 715288388)
| corpus | main edges | head edges | added | removed | kept-label transitions |
|---|---|---|---|---|---|
| vitess | 1535434 | 1566588 | 49327 | 18173 | 83 |
| kubernetes | 2385313 | 2486557 | 111282 | 10038 | 189 |
| django (control) | 232036 | 232036 | 0 | 0 | 0 |
add2.py run in both directions: `main_only` equals the other run's `added` exactly (vitess 18173/49327, kubernetes 10038/111282, django 0/0). Django edges, dfg-stats and call-stats are byte-identical.
- Added by form (def -> use): vitess range 38082, type-switch 6450, receive 601, var/const 4194; kubernetes range 99128, type-switch 1623, receive 1281, var/const 9250. The guard -> def edge is the pre-existing Def(guard) -> Use(guard) on the header line: unchanged.
- Removed by reason: into a declare-only header (rule U) vitess 2878, kubernetes 3329; outer def / twin def -> later use of a different declaration (rule S, categorical keep lifted, no new twins) vitess 15295, kubernetes 6709.
- Kept-label transitions, all classes explained: exact -> nameonly killed (vitess 71, kubernetes 137: the new def kills on loop re-entry; the old Exact was a false carry across the range re-binding; or a pair S keeps for a storage access `c.Message` now sees the shadowing range `c`), killed -> killed (6/16, nearer kill line), alias_unstable -> sameline (4/2, twin def and real def share the declaration line). No upgrade to Exact.
- call-stats outside `dfg_labels`: only `return_flow.return_input_edges` +1 (vitess 35112 -> 35113) and +7 (kubernetes 76262 -> 76269): Step 5c inputs of return expressions that read a new def (more Variable nodes carry defs); I did not isolate the individual edges. Everything else equal. `dfg_labels` shifts as above (exact +26727/+60490, cfg_incomplete, sameline up, killed down on vitess).
- Timing, vitess dfg-stats 4 threads, interleaved 2+2: main 230.8 / 226.5 s, head 227.6 / 234.5 s (noise). Kubernetes single runs 296 s / 312 s (with `--edges` output).
- Samples (seed 694, 20 added + 20 removed per corpus, sources in the sample files): **removed 40/40 RIGHT** (each is a use of a different declaration - closure/if/clause-scoped `err`, `sa`, `t1uuid`, `in`, `node`, loop `i` - a declare-only header or var-group line, or a def textually after the use with no loop around both: `limit` 258 -> 243); **added 36 RIGHT / 3 WRONG / 1 RIGHT-by-zero-value**: vitess #4 (`row@1652 -> row@2625`), kubernetes #2 (`ep@100 -> ep@75`, formal of an earlier func literal), #8 (`val@220 -> val@203`). The three wrong adds are name-collision pairs (different loops / literal formals of the same name inside one big owner) that S cannot prove because the name is also a function-literal formal or named result in the owner (categorical keep, slice B) or the use is a storage access; they carry doubt labels (killed / cfg_incomplete), the same imprecision `x := e` defs already have.

## Known limits (not fixed here)
- Implicit const repetition (`const ( A = iota; B )`) gets the def for `B` but no read of the repeated expression at `B`'s line; a constant expression reads only constants, so the lost value flow is constant-to-constant.
- Function-literal formals and named results: slice B (still exempt from S).
- Name-collision false pairs where S cannot prove different declarations (see above) persist and now include pairs from the new defs.
- The frozen oracle `legacy_producer.rs` mirrors the new def set and twin suppression (header comment says so); it is no longer an exact main replica for those forms.

## Draft PR description
**fix(dfg): Go producer defs for range/receive `:=`, type-switch aliases and later var/const names (#694 slice A)**

The Go producer emitted no def for range `:=` targets, receive `:=` targets, type-switch aliases and `var`/`const` names beyond the first name of the first direct spec, although `BindingFacts` declares them; rule S kept every outer-def -> body-use pair for those names categorically. This emits a def at each identifier, lifts S's keep for exactly these forms (function-literal formals and named results stay exempt, slice B), and adjusts the neighbouring rules.

- One def per target at its identifier; the right-hand side's reads share the line. A type-switch alias is one header def grouped, in `BindingFacts`, with its per-clause declarations (keyed by def index, so an arm write reaches only its own clause).
- U: declare-only headers are write-only (also inside closures); D: header targets are never dominating writes; no alias twin at any new def.
- Cache v155. Receipts re-baselined with per-edge justification (REPORT-694a, receipts*.txt).

Measured against main on vitess and kubernetes (4 threads): edges +49327/-18173 and +111282/-10038; 40/40 sampled removals right; 36/40 sampled additions plain right, 3 wrong (name collisions S cannot prove, slice B); django identical; vitess timing unchanged. Kept-label transitions are exact -> killed (loop re-entry kills), killed -> killed, alias_unstable -> sameline only.

Not done: implicit const repetition reads; slice B.
