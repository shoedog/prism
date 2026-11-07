# #694 slice A: Go producer defs for range/receive `:=` targets, type-switch aliases and later var/const names
Repository shoedog/prism, base origin/main (c6393854; contains #630 rules S #691 and O #698, cache 154). Create branch
`fix/694a-go-nodef-defs`. Read issue #694 and its comment (`gh issue view 694 --repo shoedog/prism --comments` if gh works; text
summarized below), CLAUDE.md's Go DFG paragraphs (U/D/S/O, the rule S paragraph's no-def exemption sentence), the producer
(`src/data_flow/build/file.rs`, Go lvalue/declaration extraction in `src/languages/mod.rs` ~line 411 and `src/ast.rs` lvalue
extraction, `src/queries.rs` Go assignment query), `src/data_flow/build/scope_prune.rs` (`address_taken_names`: the categorical
no-def keep set), `src/cpg/binding_facts/go_bindings.rs` (which declarations exist, per-clause type-switch aliases) and
`src/cpg/reaching/go_header_writes.rs` (RD's existing HeaderWrite kills for these forms).

## Problem (#694)
The Go producer emits no def for these binding forms, although `BindingFacts` declares them:
- range `:=` targets: `for k, v := range xs { use(v) }` (also `for i := range n`, `for range` without targets has none)
- receive `:=` targets: `case v := <-ch:`, `case v, ok := <-ch:`
- type-switch aliases: `switch v := x.(type) { case T: use(v) }` — declared once PER CLAUSE (including `default`)
- `var`/`const` names other than the first name of the first direct spec: `var a, b = 1, x`, grouped `var ( a = 1; b = x )`, later
  const specs (incl. implicit repetition `const ( A = iota; B )`).
So no edge connects the right-hand side to body uses; the only edge into such a use is a name-coincidence edge from an outer def,
and rule S keeps those pairs categorically. Function-literal formals and named results are slice B (do NOT change them here).

## Contract
1. Emit a producer def for each such declaration at its identifier, with the right-hand side's reads feeding it the way `x := e`
   does (same-line propagation; for range, the range expression; for receive, the channel; for type switches, the guard operand;
   for var/const, positional pairing of names and values, and every name reading the call when one multi-value call initializes
   several names; implicit const repetition reads the repeated expression's names).
2. **Type-switch aliases (trap):** the alias is a distinct declaration in every clause. A single def bound to one clause's declaration
   would let rule S drop alias-def → use pairs in the OTHER clauses (real flows). Choose a representation in which every clause's
   uses are reached by a def of THAT clause's declaration (e.g. one def per clause, anchored so `binding_of_def` returns that clause's
   declaration), or make S treat the alias def as matching every clause declaration. State the choice; test multi-clause and default.
3. Lift rule S's categorical keep for exactly these four forms (keep it for function-literal formals and named results). With the new
   defs, S may drop outer-def → body-use pairs for them because the value now reaches the body through the new def. Keep every other S
   safety rule (address-taken/sliced names, storage-access uses, plain-write defs, same-line coalescing).
4. Rules U/D/O interplay: the new defs are plain writes of their declaration; check D (a range/receive target def is not an
   unconditional dominating write for code after the loop/select; a type-switch alias def is clause-local) and O. U must not treat the
   new def's line as write-only when it also reads.
5. RD: these forms already have HeaderWrite kills; make sure the new defs and those kills agree (no double kill, labels sensible).
   Label transitions on kept edges are now possible (new defs kill earlier ones) — report every transition class and justify it.

## Test-first
RED tests (fail on main): range/receive/type-switch/var/const forms each get a guard→def→use edge; self-reading (`switch v := v.(type)`,
`for _, v := range v`, `case v := <-v`), indirect copy (`c := (v); switch v := c.(type)`), independent guard
(`v := 1; switch v := c.(type)` → the outer v → clause use pair is now dropped by S), multi-clause and default aliases (every clause
reached), two-name range/receive, `for i := range n`, `var _, v = 0, v`, grouped var, implicit const. Keep rows for function-literal
formals and named results (still exempt). `edges` changes may add AND remove — report both.
CACHE_VERSION: main's value + 1 (doc line, pins, CLAUDE.md cache sentence). CLAUDE.md: update the S paragraph's exemption sentence and
the producer description; one line in the #694 sense.

## Measure (release main vs head; corpora via REST `gh api repos/shoedog/prism/releases/assets/<id> -H 'Accept: application/octet-stream'`,
verify against SHA256SUMS asset 715288388; vitess 699857148, kubernetes 699857228, django 688449716 control; one at a time; push the
report after each corpus)
`nav --no-cache dfg-stats --edges` (4 threads; kubernetes at 1 if 4 OOMs) and `call-stats` (4). `add2.py` both directions.
Report: added (by form: range / receive / type-switch / var-const; guard→def and def→use), removed (by reason: S now drops outer pairs,
other), kept-label transitions (every class explained), call-stats outside `dfg_labels` (explain any change: Variable node counts may
rise with new defs), django identical. Samples: 20 random ADDED and 20 random REMOVED per Go corpus (fixed seed) with source lines and
RIGHT/WRONG (added: is it a real flow? removed: can any execution carry the def's value to the use?). One WRONG removal blocks.
Timing: vitess main vs head 2+2 interleaved. Delete raw dumps; never push them.

## Do
`cargo fmt`, clippy clean on touched code; file-size rule (new module if a touched file would pass 1,000 weighted lines; `src/ast.rs` is
over 2,000 and must not grow: put new code in new modules). Targeted at RAYON 1 and 4: `--lib data_flow`, `--lib cpg`, `--lib cpg_cache`,
`--test ast`, `--test lang_go`, `--test navigation`, `--test integration`, `--test algo_taxonomy`. Goldens: update only with a per-edge
justification in the report (as #698 did). No full suite (CI), no mutgate. Commit in reviewable steps (RED first); push
`fix/694a-go-nodef-defs` (plain push; if your session cannot push that branch, push your own and say so). Write `REPORT-694a.md` (with a
draft PR description) and push it incrementally with small files to `test-results/694a`. If you stop early push `FAILED.md` there with
exactly what is done.

## Rules
Do not open PRs, comment on GitHub, or touch other branches. Never read or print dotfiles or credentials. Do not open the `eval/secbench`
directory and exclude it from every recursive search. Static analysis only: never execute Go code or packages; tests only parse fixtures.
