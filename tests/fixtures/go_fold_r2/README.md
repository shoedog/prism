# Go fold r2 lexical binding population

Forty synthetic sources are parsed and analyzed by Rust tests; none are executed.
`population.jsonl` identifies each grammar form and its semantic control. All files have loop re-entry.
`base.jsonl` records exact f0bbc0a91e8908e21e6abfc9e69788673ade614d in the same worktree at threads1/4.
`expected.jsonl` pins every final label and its counters. The receipt test compares the complete stripped endpoint multiset, CFG, statements and RD availability to base.

Receive/range :=, per-clause type-switch aliases, grouped variable/constant declarations, literal formals/results, and local types are lexical bindings even where the producer has no def endpoint. Declarations cannot make values or endpoints. Assignment =, same-scope short reuse, captured variables and labels are negative controls. Type-switch aliases are distinct in every clause including default; no-outer and cross-arm controls are explicit. Secondary names in two-name receive/range are covered.

Forty-seven stored labels change in this population, all individually RED on f0 at1/4 and GREEN after. The r1 control_receive fixture supplies two more changed labels. Literal reads remain cfg_incomplete because their CFG is not modeled; direct BindingFacts tests distinguish lexical ownership without claiming an RD precision gain for those labels. Same-line declaration ordering and for-header initializer kill behavior are outside this fold.

See REPORT-fold-r2.md and r2-label-deltas.json in the lane evidence for all49 identities and the complete Go0.25.0 binding census. No test rewrites these inputs.
