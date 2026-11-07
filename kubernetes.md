# #694 slice A - kubernetes (go, 17240 files) main c639385 vs head

Edges: main 2385313, head 2486557 (4 threads, no OOM; `nav --no-cache dfg-stats --edges`; one run each: main 296 s, head 312 s).

## Added: 111282
range 99128, type-switch 1623, receive 1281, var/const 9250 (def -> use of the new def; the guard->def edge is the existing one).
Labels: [['confidence=exact/doubt=None', 61887], ['confidence=nameonly/doubt=cfg_incomplete', 34751], ['confidence=nameonly/doubt=sameline', 10125], ['confidence=nameonly/doubt=killed', 4519]].

## Removed: 10038
into a declare-only header (rule U) 3329; outer def -> later use / twin def edges (rule S, categorical keep lifted; no alias twin at new defs) 6709.
Prune counters (raw): dfg_pruned_scope 2592245 -> 2622753, dfg_pruned_write_only 1304260 -> 1370872, dfg_pruned_dominated 761293 -> 758312, dfg_pruned_order 260771 -> 262202.

## Kept-label transitions: 155
{'confidence=exact/doubt=None -> confidence=nameonly/doubt=killed/kill_line=N': 137, 'confidence=nameonly/doubt=killed/kill_line=N -> confidence=nameonly/doubt=killed/kill_line=N': 16, 'confidence=nameonly/doubt=alias_unstable -> confidence=nameonly/doubt=sameline': 2}
Same three classes as vitess: exact -> killed (the new range/receive def kills on loop re-entry, or a nested range var `c` shadows an outer `c := ...` whose pair S keeps because the use is a storage access `c.Message`); killed -> killed (nearer kill line); alias_unstable -> sameline (twin def and a real def of the name now share the declaration line). No NameOnly -> Exact upgrade.

## Samples (seed 694)
Removed 20/20 RIGHT (different declaration - closure/if-scoped err, sa, v - or declare-only header/var-group line, or a twin def into a header).
Added 18 RIGHT / 2 WRONG (#2 `ep@100` range target -> use@75, the formal of a func literal `func(ep Endpoint)` earlier in the same owner; #8 `val@220` -> use@203, another loop's `val`). Both names are function-literal formals/other-loop names S cannot prove in that owner (slice B). (#0, a var-group zero value, is RIGHT: same scope, used later in a closure).

## call-stats (4 threads)
Outside dfg_labels only return_flow.return_input_edges 76262 -> 76269 (+7, Step 5c return-expression inputs reading new defs). dfg_labels: exact 1405830 -> 1466320, loop_carried 132864 -> 132479, alias_unstable 40190 -> 39718, cfg_incomplete 367470 -> 397936, killed 236808 -> 239089, sameline 277180 -> 285659.
