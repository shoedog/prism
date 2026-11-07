# #694 slice A - vitess (go, 3263 files) main c639385 vs head

Edges: main 1535434, head 1566588 (4 threads, `nav --no-cache dfg-stats --edges`).

## Added: 49327
By def form (heuristic on the def's source line; `other` is grouped `var (..)`/`const (..)` specs, checked by sampling the top 25 lines):
range 38082, type-switch 6450, receive 601, var/const 4194.
Labels: [['confidence=exact/doubt=None', 27488], ['confidence=nameonly/doubt=cfg_incomplete', 13848], ['confidence=nameonly/doubt=sameline', 4292], ['confidence=nameonly/doubt=killed', 3699]].
The guard->def edge (the existing Def(guard) -> Use(guard) on the header line) is unchanged; every added edge is def -> use of the new def.

## Removed: 18173
by reason (use-line heuristic): into a declare-only header (rule U) 2878; outer def -> later use / twin def edges (rule S now, categorical keep lifted, plus no alias twin at new defs) 15295.
Producer prune counters (raw candidates incl. duplicates): dfg_pruned_scope 6806241 -> 6840124, dfg_pruned_write_only 15475347 -> 15518223, dfg_pruned_dominated 3344968 -> 3342257, dfg_pruned_order 636827 -> 638333.

## Kept-label transitions: 81
{'confidence=exact/doubt=None -> confidence=nameonly/doubt=killed/kill_line=N': 71, 'confidence=nameonly/doubt=alias_unstable -> confidence=nameonly/doubt=sameline': 4, 'confidence=nameonly/doubt=killed/kill_line=N -> confidence=nameonly/doubt=killed/kill_line=N': 6}
- exact -> nameonly killed (71): the new range/receive/alias def kills the earlier def on loop re-entry / clause entry; the old Exact was a false carry (`for _, expr := range xs { use(expr); expr = f(expr) }`: the def at the end of the body does not reach the next iteration's use, the range re-binds it).
- killed -> killed (6): the kill line moves to the nearer header line of the new def.
- alias_unstable -> sameline (4): `var a, b []T` + `a = b` elsewhere: the old edge started at the alias twin def at the declaration line; a real def of the same name now sits on the line, so the pair is a collapsed same-line group.

## Samples (seed 694; sources and RIGHT/WRONG in the two sample files)
Removed 20/20 RIGHT (every one is a use of a different declaration - closure/clause-local err, t1uuid, in, node - or a declare-only header, or a def textually after the use with no loop around both).
Added 19 RIGHT / 1 WRONG (#4: `for _, row := range ...` at 1652 -> use at 2625, a different `row`; killed label; S cannot prove it because `row` is a function-literal formal elsewhere in the owner, slice B).

## call-stats (4 threads)
Outside dfg_labels only return_flow.return_input_edges 35112 -> 35113 (+1, a return expression reading a new def). dfg_labels: exact 1071201 -> 1097928, loop_carried 211609 -> 211453, alias_unstable 23018 -> 22521, cfg_incomplete 261688 -> 267479, killed 91251 -> 86593, sameline 71986 -> 75777.

## Timing (dfg-stats, 4 threads, interleaved): main 230.8 / 226.5 s, head 227.6 / 234.5 s.
