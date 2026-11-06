# Go review-fold r1 static population

These61 synthetic Go files are parsed and analyzed by Rust tests; they are never executed.
`population.jsonl` enumerates semantic label controls, required initializer edges and forbidden bypasses.
`base.jsonl` records exact4db85d5b production in the same worktree/environment; `expected.jsonl` records the inspected fixed labels.
Every endpoint, statement universe and availability object is compared against base; every label, including unchanged labels, is compared against expected. No test rewrites these inputs.

-10 alias fixtures: six loop shapes (`:=`, initialized `var`, uninitialized `var` followed by assignment, each with switch or if/else), CR#4's later assignment shape, and all three header forms.
-11 negative controls: real declarations/shadows, parameters, genuine overwrite (producer-pruned pair), multi-name swap, header scopes, receive declaration syntax and type-switch alias syntax.
-10 case-scope fixtures: expression/type/communication/default clauses, default before and after ordinary arms, plus a receive-declaration communication header with a body-local shadow.
-18 assignment/short initializer fixtures: if/expression switch/type switch, alternative/default present or absent, plus a separate condition/tag line.
-12 other simple initializer controls: call, send, increment and decrement for each construct. Inc/dec retain the existing header projection and definition limitation.

`absent` checks identify producer-pruned or unmodeled declaration endpoints; they are not RD labels. In these sources the producer does not emit defs for receive `:=` or type-switch aliases. Fold r2 now represents these lexical bindings independently of endpoints. The two outer-v→inner-v receive labels in expected.jsonl become killed(6), and an explicit v@9→v@7 assertion rejects Exact. The other r1 labels stay unchanged; no producer endpoint is added.

Separate initializer and condition/tag share a dispatch anchor, so initializer→condition can remain SameLine. The existing within-header ordering limitation (including outer-parameter→condition false exact after an assignment) remains; the fold corrects initializer-before-body/case ordering and reachability, not within-anchor occurrence order. The evidence report lists these boundaries explicitly.

Full population, all163 changed occurrence identities, paths, RED/GREEN logs and guard-reversion evidence are in the lane's REPORT-fold-r1.md and r1-label-table.md. The historical163rd label is `dfg_prune/go/00_write_only_lines.go` x7→11 and is tested in its existing receipt/pruning suites.

Six inline terminal-initializer controls cover empty final if/expression/type constructs with assignment and short declarations. Paired main/base/fixed captures preserve9 expanded entries and6 cfg_incomplete labels; all6 NoCfgEdges refusals clear versus4db. terminal-base.jsonl records the actual4db inputs and outputs.

#662 hand-edited eight records in base.jsonl/expected.jsonl (as #652 did).
