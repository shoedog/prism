# #694 slice A (PR #709) review fold 1 (round 1 of 2) — targeted fixes on the existing branch
Repository shoedog/prism. Check out `fix/694a-go-nodef-defs` (head 29e04a93 = slice + main merged, cache 156). Read the branch diff vs
main, `src/ast/go_header_defs.rs`, the `BindingFacts` alias-group code (`src/cpg/binding_facts.rs`, `go_bindings.rs`), RD kill
construction (`src/cpg/reaching/def_buckets.rs`, `same_def_binding` / `proven_same_def_binding`), `scope_prune.rs`, `lexical_prune.rs`.
Each fix gets a test that fails on 29e04a93 (RED) plus a control.

## Fix
1. **WRONG (both reviewers) — later-clause writes do not kill the grouped alias header def → false Exact.**
   ```go
   func f(x any, c bool) {
       switch v := x.(type) {
       case string:
           println(v)
       default:
           if c { v = 1 } else { v = 2 }
           println(v)
       }
   }
   ```
   The header def H is attached to clause 1's declaration; `relation_at`/S treat every clause as reached via `alias_groups`, but RD kills
   use plain binding equality, so a write in clause k >= 2 never kills H and H → the final `println(v)` is labelled Exact. Fix: make the
   header def's kill relation group-aware in both directions (a write binding ANY member declaration kills H; H kills defs of any member
   on re-entry), while ordinary arm writes stay clause-local (a write in clause 2 must not reach clause 3). Options: group-aware
   `same_def_binding`/`proven_same_def_binding`, or private per-clause RD identities mapped to the one public endpoint — say which. Tests:
   the example above (H → final use labelled Killed, never Exact), the same in clause 2 (non-default), clause-1 control, nested same-name
   switch, loop re-entry (a clause-k write carried into the next iteration's clause-k use must be killed by H).
2. **WRONG (sol; Opus called it a known limit) — multiline headers lose the replacement path.**
   ```go
   func f(xs []int) {
       v := xs
       for _, v := range
           v {
           println(v)
       }
   }
   ```
   Main kept outer `v@3` → body use (S's categorical keep); head drops it via S, and the replacement path is broken because queries and
   taint connect Use→Def only on the same line (`src/cpg/query.rs` ~767, `src/cpg/trace.rs` ~953) and the new def is on line 4 while the
   outer read is on line 5. Fix (conservative): keep S's exemption for a range/receive/type-switch/var-const declaration whose target
   identifier's line does not contain every right-hand-side read (multiline header or spec); S may drop only when the replacement path
   is connected on one line. Optional (only if small and clean): connect RHS reads to the introduced defs across the whole header/spec.
   Tests: multiline range, receive, type-switch alias and later-var controls (pair kept), single-line controls still dropped.
3. **Ledger.** The PR reports kept-label transitions 83 (vitess) / 189 (kubernetes), but the class breakdowns sum to 81 / 155. After the
   fixes, re-measure and attach the COMPLETE transition ledger (every class with counts that sum to the total; state the counting
   convention: endpoint pairs vs multiset rows). Also identify the `return_input_edges` additions (+1 vitess / +7 kubernetes): which
   return expressions and why (new Use nodes from the new defs).
4. **Known limit to document (Opus W3):** none needed if fix 2 keeps the exemption for split headers; otherwise list it.

## Measure (after tests green)
Release binaries main (current origin/main), prev 29e04a93, head. Corpora via REST (`gh api repos/shoedog/prism/releases/assets/<id> -H
'Accept: application/octet-stream'`), verify against SHA256SUMS asset 715288388: vitess 699857148, kubernetes 699857228, django 688449716.
One corpus/process at a time; push the report after each corpus. `nav --no-cache dfg-stats --edges` and `call-stats` (4 threads; kubernetes
at 1 if 4 OOMs). `add2.py` (beside this brief) head vs main and head vs prev. Report: head vs main added/removed/transitions (complete
ledger), head vs prev (given back / changed, by fix), call-stats outside `dfg_labels` explained, django identical, 10 samples per Go corpus of
head-vs-prev changes with verdicts. Delete raw dumps; never push them.

## Do
`cargo fmt`, clippy clean on touched code; file-size rule (`src/ast.rs` must not grow). Targeted at RAYON 1 and 4: `--lib data_flow`, `--lib cpg`,
`--lib cpg_cache`, `--test ast`, `--test lang_go`, `--test navigation`, `--test integration`, `--test algo_taxonomy`. Goldens: update only with
per-edge justification. Commit, push to `fix/694a-go-nodef-defs` (plain fast-forward; if your session cannot, push your own branch and say so).
Write `REPORT-694a-fold1.md` and push it incrementally to a new branch `test-results/694a-fold1`. If you stop early push `FAILED.md` there.

## Rules
Do not open PRs, comment on GitHub, or touch other branches. Never read or print dotfiles or credentials. Do not open the `eval/secbench`
directory and exclude it from every recursive search. Static analysis only: never execute Go code or packages; tests only parse fixtures.
