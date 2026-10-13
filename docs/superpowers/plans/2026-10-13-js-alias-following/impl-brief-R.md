# Implementation brief — slice R: a bound name whose value is not proven gets no name search (JS/TS)

You are implementing one test-first slice in the prism repo (Rust; code slicing / call-graph tool). Work ONLY in the git worktree `{{WORKTREE}}` (branch `{{BRANCH}}`, cut from `{{BASE}}`). Read its `CLAUDE.md` first.

## Goal (what this serves)

prism resolves a JS/TS call `f()` or element `<F />` by asking what the name is bound to. The binder (`src/ast/js_binding*.rs`) finds the nearest binding and classifies it. The slice you are stacked on (slice L) follows `const` aliases to what they name. What is left: when the nearest binding is an in-file declaration whose value prism cannot prove (a `let`, a written binding, a call result, a destructured value, an alias L does not follow), the resolver "keeps base behavior", and base behavior searches by spelling. That produces wrong Exact edges in three ways, all reproduced on main and all pinned as `WRONG` rows in the variant matrix:

1. **Cross-file name search.** The alias is declared in an enclosing function and shares its name with the one function of that name anywhere in the repo: Exact `free_single` to that unrelated function (or several NameOnly `free_multi` candidates).
2. **Same-file name search.** The alias shadows a same-file function of the same name: Exact `local_def` to the shadowed function.
3. **The older textual alias path** ("Level 1: local variable function pointer", `src/call_graph.rs`, `crate::ast::resolve_fptr_assignment`): it reads `h = f` from source text and ignores scope, so it emits an Exact edge when the right-hand `f` is really a parameter or another local, or when `h` at the site is really a parameter or catch binding.

The owner's decision (2026-10-13): **a bound name whose value is unknown gets no name search.** This slice implements that as three rules. It removes edges; it adds none. A previous attempt at this (refusing by "import evidence" with a list of exemptions) was reviewed twice and each round found shapes it mishandled, so this slice uses the binder's answer and nothing else: no exemption lists, no per-shape special cases.

Measured on three corpora before this slice: these rules cover 109 wrong Exact edges and about 2,200 NameOnly candidates in a private React app, 19 in Excalidraw and about 200 in the TypeScript compiler through rule 1, plus about 70 same-file edges through rule 2. The controller measures your result; you do not run corpora.

## Grounding (verified; start here, then verify yourself)

- Binder states on a call site: `JsLocalBinding` in `src/call_graph.rs` (mapping from `JsBinding` near `js_local_binding_at`): `Callable` / the alias variants from slice L (proven; untouched here), `MayCall(reason)` with reason `"may_call"` (a written binding, or a call with a function argument) or `"alias"` (any other value that may hold a function), `Unproven(reason)` (`"unbound"`, `"import"`, `"not_callable"`, …), `Position(reason)` (the walk could not decide; keeps base by an earlier owner ruling), `Unchecked`.
- The resolver: `src/resolution.rs`, the `None`-qualifier arm of `resolve_call_site`: after the proven outcomes and the import-member route comes the same-file rung (`R4`, `local_def` by name; today it drops only for `Unproven(_)`), then a lexical guard, then the repo-wide rung (`R5`, `free_single` / `free_multi`; today it drops only for three `Unproven` reasons).
- A parameter-bound name is already `Unproven("not_callable")` and already dropped. An import-bound name never reaches the repo-wide rung. So the states that still reach the name searches with an in-file binding are exactly `MayCall(_)`.
- The textual path: `compute_indirect_call_sites` in `src/call_graph.rs`; slice L made it stand down where the binder followed the alias. It is language-generic (it exists for C function pointers); only its JS/TS behavior changes here.
- The 2026-09-27 "may-call" ruling that must survive: `const f = debounce(() => {…}); f()` and `const f = useMemo(() => …)`-style values bind, by name, the inline function written in that declaration (prism names an inline function after the declarator). The matrix control is `call-with-inline-fn` (CORRECT today, must stay CORRECT).

## Contract (this is the spec)

All three rules apply to JS/TS call and JSX element sites whose callee is a bare identifier. Records with origin other than `Source` are touched only by rule 3. Non-JS languages: byte-identical output.

**Rule 1 — no cross-file name search for a bound name.** If the site's binding is `MayCall(_)`, the repo-wide rung is not consulted: no `free_single`, no `free_multi`. The site drops with `LocalBindingUnproven`. Unchanged: `Unproven("unbound")` and `Position(_)` keep the repo-wide rung exactly as today (script globals; that question is not decided), and so does every state that already drops.

**Rule 2 — a same-file by-name hit must belong to the binding.** If the site's binding is `MayCall(_)`, the same-file rung keeps a target only when that function is syntactically part of a value written to the binding: inside the declaration's initializer, or inside the right-hand side of an assignment to that binding in the binding's scope. Any other same-file function of that spelling is not the binding's value: the site drops with `LocalBindingUnproven`. This needs the binder to hand the resolver where those value expressions are (line spans are enough: function identities carry line ranges); it is persisted with the site, so it rides on a cache-version bump. A hit that is kept keeps its grade and kind.

**Rule 3 — the textual alias path respects bindings in JS/TS.** For a JS/TS site the path may emit its `IndirectResolution` record only when both hold: (i) the alias name at the call site is bound, by the binder's walk from that site, to the same declaration or assignment the text scan matched (not to a parameter, a catch binding, or a declaration in another block); and (ii) the right-hand identifier, looked up by the binder at its own position in that declaration or assignment, is bound to a function or to an import (not to a parameter or another local value). Otherwise it emits nothing. Where it does emit, its target and grade are as today. Other languages are unchanged.

**Must not change.** Sites whose binding is proven (including everything slice L follows). Sites with `Unproven("unbound")` or `Position(_)`. The import routes. Member-callee sites, `new`, `super`. The `call-with-inline-fn` behavior. Module export summaries (D4). Non-JS output.

**Drop reason.** Use the existing `LocalBindingUnproven`. Do not add a NameOnly demotion anywhere: an unrelated function with a lower grade still corrupts the graph.

## Test first

1. **Matrix** (`tests/fixtures/js_alias_matrix/`, read its README; the bless guard compares with the committed baseline and needs ledger entries for regressions). Run it green on your base first. After implementing, list the differing rows, then bless.
   Expected:
   - Every `WRONG` row whose wrong edge comes from `free_single` on a locally bound name, from `local_def` on a shadowed same-file function (`shadows-same-file-fn-props`, `shadows-same-file-fn-let`), or from the textual path (`ident-rhs-is-outer-const`, `ident-rhs-is-outer-param`, `ident-alias-rebound-by-catch`, `alias-in-sibling-block`) loses that edge and becomes `MISSING`, `MAY-NONE` or `NONE-OK` according to its truth.
   - Rows carrying NameOnly `free_multi` candidates on a bound name (naming `trap2`) lose the candidates.
   - `WRONG` rows that must remain, with the reason: `react-lazy` (the loader arrow is inside the declaration; a contract question, not this slice), `lib-global-trap` and `module-unbound-module-target` (unbound names; the script-global question is not decided).
   - **Rows that leave `CORRECT`.** Some rows are correct today only by coincidence: the alias has the same name as its true target, sits in an enclosing function, and its shape is one slice L does not follow (`let`, `var`, object or array members, wrappers, hook results, …), so the name search happens to land on the right function. Rule 1 removes those edges; that is the decision, and later slices (following returned values, following arguments) are what restore them with proof. Each such row needs a ledger entry `{"id", "from":"CORRECT", "to":"MISSING", "reason"}`; use one reason text: "name-search coincidence: the binding's value is not proven (owner decision 2026-10-13); to be restored by value following". Report the list grouped by shape. If a row leaves `CORRECT` and is NOT such a coincidence (its edge came from a proven route, or from the declaration's own inline function), stop and report: that is a defect.
   - `call-with-inline-fn`, `ident-own-name-import`, `ident-same-file-fn`, every row slice L made `CORRECT`, the `static-property-*` indirect controls, and all import, import-path, same-file, parameter and global rows not named above: byte-identical.
   Report the diff as a table: shape → count by (old verdict → new verdict), and separately the rows whose only change is losing NameOnly candidates.
2. **Focused tests**, written first and shown RED on your base for each rule: resolver-level tests asserting the full record set (origin, target, grade, kind, drop reason) for: rule 1 with a unique same-named decoy elsewhere (Exact today) and with two decoys (NameOnly today), in an enclosing function; rule 2 with a shadowed same-file function for an alias, a destructured value and a reassigned `let`; rule 2's keep cases (inline function in the initializer; inline function assigned later `let f; f = () => {}`; a function expression nested inside a call argument in the initializer); rule 3 for each of: right-hand name is an outer parameter, an outer const holding a non-function, the caller's own parameter; alias name rebound by a catch parameter; alias declared in a sibling block while the site reads a parameter; and rule 3's keep case `let h = f; h()` with `f` an import or a same-file function. Negative controls: an unbound name still reaches the repo-wide rung exactly as today (module → script global, script → script global, the `Position` cases: a call in a merged-namespace body, an enum-merged namespace, a decorator on a class expression); a parameter-bound name still drops as today; a C (or other non-JS) function-pointer test for the textual path is unchanged (find the existing ones and run them; do not write new ones).
3. Persisted state: bump the CPG cache version and the nav call-edge cache version, their pins, the three anchored cache-version mutants in the older lanes, and cover the changed binding shape in the existing serialization round-trip test.
4. Add a mutation lane for this slice in the existing format (`mutants/`), with at least: rule 1 skipped for `"alias"`; rule 1 skipped for `"may_call"`; rule 1 applied to `Unproven("unbound")`; rule 1 applied to `Position`; rule 2 skipped; rule 2 without the "inside a written value" exception; rule 2's exception widened to the whole function; rule 3 condition (i) removed; rule 3 condition (ii) removed; rule 3 applied to non-JS. Every mutant KILLED (authoritative).
5. Keep every Rust file under 600 lines; put new logic in new modules beside the slice L ones rather than growing `resolution.rs` or `call_graph.rs`.

## Verify (report exact totals and verbatim summary lines)

`cargo fmt --check`; the clippy invocation CI uses, `--features mcp`, all targets; full suite `cargo nextest run --all-targets --features mcp --no-fail-fast` and `cargo test --doc --features mcp` (two known unrelated failures may appear: `example/parameter_slot_characterization …d7…`, and load-sensitive `mcp::…::lazy_*` tests; report, do not fix); the matrix test once with `RAYON_NUM_THREADS=4`; `cargo build --release && cd eval && uv run tier-a --matrix-only --allow-stale-sut` (report regressions and flip candidates, never re-baseline); `python3 scripts/mutgate/mutgate.py --plan-only` exit 0 and your lane authoritative. State what you did not verify.

## Boundaries

- Static work only inside this worktree. Do not read or open any directory named `frontend-portal`, anything under `~/prism-evidence/`, or any other checkout. No corpus runs.
- Git: commit locally in small logical commits (tests first). Do NOT push, open PRs, or touch other branches or worktrees. End each commit message with these two lines exactly:
  `Co-Authored-By: Claude Opus 5.5 (1M context) <noreply@anthropic.com>`
  `Claude-Session: https://claude.ai/code/session_01CeCpr7mLfpQeq9vEEKBFhQ`
- No planning or analysis documents. Add to the call-resolution paragraph of `CLAUDE.md`: a JS/TS callee whose nearest binding is an in-file declaration with an unproven value is not resolved by spelling: no repo-wide name search, a same-file function only when it is written into that binding, and the textual alias path only when the binder agrees on both names.
- No scope growth. Not in this slice: script globals and other unbound names; following more value shapes; parameters and arguments; `lazy()`; anything that adds an edge. If a rule cannot be met as written, or meeting it changes a row the brief says must not change, stop and report with the evidence instead of adjusting the rule.

## Report back (concise, facts only)

1. Commits and `git diff --stat {{BASE_REF}}`.
2. RED evidence per rule.
3. The matrix tables (verdict moves; NameOnly-only changes; the ledger list grouped by shape) and any row outside the expected set.
4. The design in a few lines: what the binder now hands the resolver for rule 2, where each rule lives, how rule 3 decides (i) and (ii).
5. Verification totals; lane results.
6. Anything in the contract you could not meet, anything in the grounding that turned out wrong, and what you did not verify.
