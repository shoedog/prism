# #694 slice A (PR #709) review fold 1 — report

Branch `fix/694a-go-nodef-defs`, fold commits `9640094`, `18feaea`, `d4c27d4` (on top of `29e04a93`).
Binaries: release builds of origin/main (`921f07e`), prev (`29e04a93`), head (`d4c27d4`). Corpora from the release
assets, sha256 verified against SHA256SUMS (vitess, kubernetes, django all match). One corpus per process, 4 rayon threads
(kubernetes ran at 4 without OOM). Static analysis only; nothing executed. Raw dumps deleted, not pushed.

## Fixes
1. **Alias header kills (WRONG, both reviewers).** Chosen option: group-aware `same_def_binding` /
   `proven_same_def_binding` (new `BindingFacts::alias_group_related`), not private per-clause RD identities. The header def H
   kills, and is killed by, a def of any member clause declaration; ordinary clause writes are not group heads, so they stay
   clause-local. Tests (`tests/ast/go_fold694a_test.rs`, RED on 29e04a93, 3 of 5 fail there): default clause, non-default clause
   (+ clause 3 untouched, clause-local writes do not reach clause 3), clause-1 control, nested same-name switch, loop re-entry
   (carried clause-2 write must not be Exact at the next iteration's clause-2 use).
2. **Split headers (WRONG, sol).** `scope_split.rs`: rule S keeps the outer pair when a range/receive `:=`, type-switch alias or
   var/const spec has a right-hand-side identifier read on a different line than the declared name AND the outer def is the one
   that read resolves to (the def that feeds the header). Optional RHS-to-def connection not done.
   Tests (RED on 29e04a93): multiline range, receive, type-switch alias, var spec (label before the use turns rule D off so only
   S decides), the feeding-def rule (unrelated `if`-local `v` still dropped), single-line controls still dropped, a multiline
   literal with no identifier reads is not split.
   Two earlier versions were measured and rejected: (a) "RHS spans lines" kept +54 (vitess) / +376 (kubernetes) rows over prev,
   mostly multiline `[]string{...}` literals and unrelated same-named defs in sibling subtests; (b) "RHS reads an identifier on
   another line" still kept +20 / unrelated defs. The final rule changes nothing on either Go corpus.
4. Known limit: none beyond the documented conservative rule; split headers keep the exemption.

## Ledger (fix 3), head vs main
Counting convention: **multiset rows** of the `--edges` JSONL, keyed by the whole edge object minus
`confidence/doubt/kill_line/loop_carried`. For each key the label multisets are paired: paired rows with a different label are
*relabelled*, unpaired main rows are *removed*, unpaired head rows *added*. `A = unchanged + relabelled + removed`,
`B = unchanged + relabelled + added` (asserted). The PR's 83/189 are the relabelled rows; its class breakdowns (81/155) were short
of that total (they listed only the top classes). Every key here is relabel-only, wholly new or wholly gone.

### vitess  (main 1,535,434 rows -> head 1,566,588)
unchanged 1,517,178; relabelled 83; removed 18,173; added 49,327.
- relabelled 83: exact -> nameonly/killed 73; nameonly/killed -> nameonly/killed (kill_line changed) 6; nameonly/alias_unstable -> nameonly/sameline 4.
- removed 18,173: nameonly/killed 8,430; cfg_incomplete 8,057; exact 688; sameline 505; alias_unstable 493.
- added 49,327: exact 27,488; cfg_incomplete 13,848; sameline 4,292; killed 3,699.

### kubernetes  (main 2,385,313 rows -> head 2,486,557)
unchanged 2,375,086; relabelled 189; removed 10,038; added 111,282.
- relabelled 189: exact -> nameonly/killed 166; killed -> killed (kill_line) 21; alias_unstable -> sameline 2.
- removed 10,038: cfg_incomplete 4,285; killed 2,404; sameline 1,648; exact 1,231; alias_unstable 470.
- added 111,282: exact 61,887; cfg_incomplete 34,751; sameline 10,125; killed 4,519.

### django: edges and call-stats byte-identical main vs head.

## Head vs prev (given back / changed, by fix)
- vitess: 0 rows changed (edges identical, call-stats identical).
- kubernetes: 0 rows changed (edges identical, call-stats identical).
- Fix 1 relabels nothing on either corpus (no later-clause write after a type-switch header with a reaching use occurs); fix 2 in its final
  form keeps nothing extra. Both are test-level protections. Consequently there are **no head-vs-prev changes to sample** (10 samples
  per corpus: none exist). The rejected intermediate variants' samples were all NameOnly pairs between sibling loops' same-named
  `doc`/`sa` declarations, i.e. the over-keeping that motivated the final rule.

## call-stats outside `dfg_labels` (head vs main)
Identical except `return_input_edges`: vitess 35,112 -> 35,113 (+1), kubernetes 76,262 -> 76,269 (+7). Identified with an env-gated trace
(local, not committed; main vs head list of ReturnInput edges diffed): every added input is a **new Use node of a loop variable
read inside a return expression**, a Use that exists now because the range target has a def.
- vitess: `vstreamer/lastpk_validation.go` `validateLastPK`, `return nil, vterrors.Errorf(..., fields[pkCol].Name, err)` (line 57-58): `pkCol` is the `for i, pkCol := range pkColumns` target.
- kubernetes (7): `pkg/kubelet/nodestatus/setters.go` loop index `i[]` in the returns of `DiskPressureCondition` (l.690), `MemoryPressureCondition` (566), `PIDPressureCondition` (628), `ReadyCondition` (525, 526, 527); `staging/.../impersonation.go` `WithImpersonation` `impersonationRequest[]` (73).
Nothing was removed. The `dfg_label_*`/`dfg_pruned_*` counters move with the edge ledger above.

## Tests (d4c27d4)
Targeted at RAYON_NUM_THREADS 4 and 1: `--lib data_flow`, `--lib cpg`, `--lib cpg_cache`, `--test ast`, `--test lang_go`,
`--test navigation`, `--test algo_taxonomy` all pass except `cpg_cache::stream::tests::a_write_that_cannot_start_fails_and_leaves_the_previous_cache_usable`.
`--test integration`: 1475 pass, 6 fail: 5 `owner_activation_test::owner_admission_*` and `build_profile_round2_test::rss_intervals_exclude_cfg_in_both_owner_orders`.
- The cpg_cache test and the 5 owner_activation tests fail identically on the unmodified `29e04a93` (sandbox runs as root; owner_activation fails parsing child output, "trailing characters").
- `rss_intervals...` passes when run alone at head (4 of 4 runs) and failed only inside the full parallel integration run (it measures RSS, load-sensitive); it passed on the unmodified branch in the one run made there, so this one is not proven pre-existing, only load-flaky.
- No golden/fixture changed: every Go fixture receipt passes untouched. cargo fmt clean; clippy clean on touched code; `src/ast.rs` unchanged; new `scope_split.rs` 61 weighted lines.
Mutation gate not run (no mutants anchored in the changed fns were checked; flagged for the pre-merge gate).
