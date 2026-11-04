# Go join fold r0 (#652)

Static synthetic sources; never execute these Go packages. `population.jsonl` names every
fixture, shape, inserted syntax, position, exact expected edges and x def/use label.
The pinned Go0.25.0 grammar's complete non-anchor statement population is block,
inc_statement, dec_statement, empty_statement and type_declaration. Grouped types,
comments/chains, empty/nested blocks and anchored controls are included.

526 rows cover after-construct joins, terminal arm/body positions and arm entry.
17 constructs include if/else/chains, expression/type switches with and without
default, select with and without default, loops/range, labels/blocks, nesting,
break and fallthrough. Loop bodies resume the header; only the header exits.

`base.jsonl` binds the complete occurrence-bearing endpoint multiset, statement
universe and RD availability of those526 fixtures to391313bc, measured in the
same worktree/environment at both1/4 threads. `extra-base.jsonl` extends that
contract to the16 class/dead/controller fixtures in the sibling go_join_* directories.
`labels.jsonl` records every expected label across all542 inputs; the receipt test
enumerates every mismatch. All139 changed labels fail on391313bc at1/4 threads.
No endpoint, statement-universe or availability change is expected in this fold.

The sibling directories pin three reachable labels restored by the fix, all three
main→cfg_incomplete classes after blocking select, fully terminating if/switch
controls and the controller's four static reproductions. The controller examples
retain pre-existing doubt on standalone unanchored inc/dec lines; this fix adds
no anchors and does not change increment definition modeling.

The detailed pre-edit population, root-cause log, paired controls, complete label
deltas and corpus remeasurement limitations are in REPORT-fold-r0.md in the lane
evidence directory. Historical Go branch expectations are unchanged by this fold.
