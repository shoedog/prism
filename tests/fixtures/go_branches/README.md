# Go branch CFG / RD contract (#645)

These are static synthetic Go inputs, never executed. The population includes expression/type
switches, init and multi-value headers, empty/default-less arms, select communication forms,
blocking/empty/default-only selects, fallthrough (including an empty next case and comments),
else-if/plain/empty/no-else branches, nested callables, labels and loop-tail exits.

`case_value_loop.go` and `case_value_second.go` model K13/K23: a later definition cannot reach
next-iteration case evaluation without the unconditional loop write. `k21.go` models a continue
whose loop exits to a switch with an empty arm: the sibling default write is avoidable.
`multiline_init.go` prevents alias twins in an initializer from declaring their source binding;
`init_shadow.go` keeps real initializer shadowing. `select_dead_def.go` pins entry-unreachable doubt.

`base.jsonl` captures exact ea82911c production in the same worktree/environment. `expected.jsonl`
records the reviewed labels, counters, availability, CFG and statement universe. Each row retains
full occurrence-bearing endpoints. `go_branches_test` compares every label and strips labels to
compare the complete base endpoint multiset; it enumerates all mismatches before failing.
The rows also include the existing dfg_prune/go, go_jumps, go_jumps_round2 and go_loop_cycles
fixtures so historical receipt updates cannot hide an endpoint change.

All source fixtures are unchanged between the paired base/head captures. The evidence report
contains the full population, label/path and RD-availability tables; the boundary test separately
checks 4096 versus 4097 statement lines after dispatch anchors are added.

Fold r1 changes only row 00 x7→11 from killed10 to Exact (and its label counters): `var z` is not a declaration of x. Endpoints, availability, CFG and statement universe are unchanged.
