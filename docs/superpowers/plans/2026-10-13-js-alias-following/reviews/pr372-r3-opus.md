# Review of PR #372, round 3 (final confirmation): follow `const` aliases to the callable they name

Reviewer: Opus 5.5 (independent). Head `568c284f` (`L4-prism --version` = `568c284f1a58`), main binary `38a599948771`. Worktree clean before and after (`git status --short` empty), no git writes, nothing posted.
Goal judged against: "the nearest binding decides, and an alias is followed to what it names", with the PR's rule: one predicate (`js_ts_alias_route_hit`), three uses, and main's records wherever it is false.

**Verdict: APPROVE.** Nothing material remains. All round-2 rows are closed. The rule is **not literally true by construction**, though. I found two places where it fails, both WRONG against the stated rule and both IMMATERIAL to the goal:
- W1: a lowercase JSX alias (`const h = Comp; <h />`). The predicate is true there, but the resolver returns `JsxIntrinsic` before the alias arm runs.
- W2: the stand-down un-suppresses a Level-4 textual record at a different site on the same line.

Both come from one root: the predicate is computed beside the code it gates, not from it. The two places are a closed list, established by reading the whole resolver preamble and the whole textual pass. A 4,680-fixture out-of-matrix sweep found nothing else. Before merge, the PR text needs to name these two exceptions, or the PR needs the two small fixes below. The GitHub PR description is also stale (S2).

## How I checked

- Read the predicate and its three uses, every reader of `local_binding` and of the `Alias*` variants in `src`, all of `resolve_call_site_full`'s pre-arm returns, all of `compute_indirect_call_sites`, `build_resolved_call_edges`, `to_callers_output`, both build paths' ordering, the binder (`js_binding_alias.rs`) and the rule test.
- Two-binary comparisons on synthetic fixtures under `r3-opus-scratch/fx/` (`run.py <dir>`). Each fixture ran under both binaries in the same environment with fresh caches, so main is the attribution control.
- An out-of-matrix sweep, `sweep_gen.py` / `sweep_run.py` / `sweep_cmp.py`: 4,680 fixtures, 6,480 site keys. The dimensions are 10 terminals × 3 placements × 27 site forms (JSX upper/lower/pair/member, `new`, tagged, `?.()`, `.call`, generic, `!`, spread, same-line Level-2/Level-4 neighbours, two aliases on a line, callback argument and more) × 3 decoys × JS/TS.
- The matrix, re-run on both binaries (`matrix/`, `matrix_cmp.py`): 1,825 site keys. 1,522 identical, 303 proven (6 of them with main's textual record absent), **0 other**. 28 fixtures have no records on either binary.
- **The rule test's own logic, run outside the matrix.** `ruleprobe/` is a scratch crate that depends on the worktree by path and is built into scratch (`target/` is in scratch; nothing in the worktree changed). It copies `check_fixture` verbatim and adds one check the in-repo test lacks: at a predicate-true site, the resolver must return exactly the route's hit. I ran it on the sweep and on the fixtures below.
- In the worktree, with the bless variable unset: `cargo test --test integration js_alias` gave 82 passed, 0 failed. `cargo test --lib js_binding_alias` gave 19 passed, 0 failed. The rule test prints `1349 fixtures, 1825 Source sites, 348 alias-bound, 303 predicate-true, 45 compared, 15 held, 6 suppressed`.
- Binary SHA-256 values are in `r3-opus-scratch/binaries.sha256`.

---

## WRONG findings

### W1. A lowercase JSX alias makes the predicate true while the resolver returns `JsxIntrinsic`: main's textual record is removed, `--format callers` credits a call that does not exist, and the counter says "proven" — WRONG (against the rule), IMMATERIAL

**Input** (`fx/p1/app.jsx`):

```jsx
function Comp() { return null; }
export function sameFn() {
  const h = Comp;
  return <h />;
}
export function outer() {
  const k = Comp;
  return function inner() { return <k />; };
}
export function upper() {
  const H = Comp;
  return <H />;
}
```

| | main | head |
|---|---|---|
| `<h />` (line 4), records | Source dropped `JsxIntrinsic`; **IndirectResolution Exact `local_def` → `Comp`** | Source dropped `JsxIntrinsic` only |
| `<k />` (line 8), records | Source dropped `JsxIntrinsic` | same |
| `--format callers` for `Comp` (diff `fx/p1.diff`) | `sameFn`, `upper` | **`inner`**, `sameFn`, `upper` |
| `nav callers --symbol Comp` | `sameFn`, `upper` | `upper` |
| `call-stats` | `local_binding_may_call.alias: 3` | `js_alias_follow.callable_proven: 3` (one edge exists) |

**Mechanism.**
1. `resolve_call_site_full` returns `JsxIntrinsic` for a lowercase unqualified JSX tag (`src/resolution.rs:2309-2317`). That happens before the `None`-qualifier arm reaches `js_ts_alias_route` (`:3411`).
2. `js_ts_alias_route` does not know about that gate. The binder follows `h` (`AliasCallable`), so the predicate is true.
3. The predicate then drives all three uses, not the resolver's answer:
   - The stand-down (`call_graph.rs:2779`) removes main's textual record.
   - `to_callers_output` collects the route's target through `js_ts_alias_route_outcome` (`review.rs:362-375`) and credits it in every placement.
   - The counter (`queries.rs:409-424`) records "proven".

`nav callers` reads the resolver's targets, so it does not list these sites. On the head, `nav callers` and `--format callers` therefore disagree.

By reading, there is a second instance with the same mechanism. An installed `executable_owner` (`resolution.rs:2318-2329`, the JS/TS census owner sessions) also answers before the alias arm. I did not execute it.

**Measured population.** Sweep: 144 predicate-true lowercase-JSX sites across all placements and proven terminals, and the extra check flags all of them as `JsxIntrinsic`. Six site keys (same function, where main's textual path fired) differ in records. The rest differ only in `--format callers` and in the counter.

**Why the structural test passes anyway.** `js_alias_rule_test.rs:103-107` does `continue` at every predicate-true site, so half (i) of the rule is never asserted. A suppressed record at a predicate-true site is allowed by the textual check. Running the test's own logic on `fx/p1` gives 0 violations; the added predicate-true check gives 2 (`ruleprobe_run.log`).

**Why IMMATERIAL.** The removed record is main's wrong edge: `<h>` is an intrinsic DOM element and never calls `Comp`. The new caller in `--format callers` exists only for a lowercase const alias used as a JSX tag, which React itself renders as an unknown element and warns about. The shape is in none of the matrix or corpus evidence.

**Recommended fix (closes the family, not just this instance).** Make the predicate the resolver's own answer:
- add `via_alias_route: bool` to `ResolutionOutcome` (false by default, set only on the `None`-arm return at `:3411`);
- define `js_ts_alias_route_hit(site)` as `self.resolve_call_site_full(site).via_alias_route`, and have `js_ts_alias_route_outcome` return the resolved targets only then;
- add the predicate-true assertion to the rule test (the outcome is exactly one Exact target equal to the route's), plus the matrix shape `jsx-lowercase-alias` for the same function and for a nested one.

This covers the intrinsic gate, the executable owner and any future rung placed before the arm. Cost: one field on a widely built struct. The stand-down calls the resolver for alias-bound Level-1 candidates; gate that on `matches!(binding, Alias*)` first so the cost stays negligible.

**Alternative (this instance only).** Move the intrinsic test into a shared helper and return `None` from `js_ts_alias_route` when it holds. That is two lines and restores exactly main's records, including main's wrong textual edge. Cost: the executable-owner instance stays open, and the next pre-arm rung can reopen it.

**Would not apply** if the owner prefers to keep removing main's wrong edge at intrinsic tags. In that case, disclose it as a second knowing difference, and make `to_callers_output` and the counter read the resolver's outcome.

### W2. The stand-down adds a Level-4 textual record at a different site on the same line — WRONG (against the rule), IMMATERIAL

**Input** (`fx/p2`; `aux`/`lib` export `cbImpl` and `target`):

```js
import { target, cbImpl } from './lib';
const obj = {};
obj.field = cbImpl;
export function sameLine() {
  const h = target;
  return h(obj.field());
}
export function directSameLine() {
  return target(obj.field());
}
```

| Site `obj.field()` (predicate false: not an alias site) | main | head |
|---|---|---|
| in `sameLine` (line 6) | Source dropped `UnknownName` | Source dropped `UnknownName`; **IndirectResolution Exact → `cbImpl`** |
| in `directSameLine`, and the same call on its own line | both records | both records |

**Mechanism.** Level 4 skips a qualified site when any Level-1/2 record already exists for the same caller and line (`already_resolved`, `call_graph.rs:2844-2851`). The stand-down withholds the alias site's Level-1 record, so the neighbour is no longer suppressed.

I checked the alternative explanation, that the newly resolving Source record causes this. The rule test's own seam rules it out: on one graph, the record is in `audit(stand_down = true)` and not in `audit(false)`. It is also present on main for the direct twin.

**Measured population.** Sweep: 30 site keys (`h(obj.field())`, `h(); obj.field();`, `obj.field(h())`, a JSX attribute, a sibling argument × 3 terminals × JS/TS). All are in the same-function placement, where main's textual path fires. The test's own logic reports all 30 as "stand-down added a textual record main does not have". So the test would catch this; the matrix just has no such shape.

**Why IMMATERIAL.** The added record is exactly what main emits for the same call on any other line or beside a direct call. It needs both a proven alias whose main textual path fired and a Level-4-indexed member call on the same line. Per the brief, no record was added or removed on X, T or F, so the stand-down never fired there.

**Recommended fix.** Always compute the Level-1 record as main does. When `alias_followed`, push it to a `stood_down` vector instead of `extra_sites`, and scan `extra_sites.iter().chain(&stood_down)` in `already_resolved`. That is about 5 lines and gives exact main parity. Add the matrix shape `same-line-level4-neighbour`.

**Alternative.** Disclose it as an exception. Cost: the rule's statement needs a clause, which is arguably the more consistent behavior but is not "exactly main".

**Would not apply** if the owner adopts the alternative.

### W3. Early-error declarations are still followed: a catch-parameter redeclaration and reserved-word binding names — WRONG, IMMATERIAL (question 4; open-class by nature)

Each row is head vs main in `fx/q4`; `node --check` (parse only) on an `.mjs` copy in `syntax/`:

| Declaration | head | main | `node --check` |
|---|---|---|---|
| `try {…} catch (h) { const h = f; return h(); }` | Exact → `m.js:f` | dropped | `Identifier 'h' has already been declared` |
| `const { static } = N; static()` (also `let`, `package`, `enum`, `eval`) | Exact → `m.js:impl` | dropped | `Unexpected strict mode reserved word` / `Unexpected reserved word` / `Unexpected eval or arguments` |
| controls: parameter + `const` of the same name, `{ var h }`, `const`+`function` duplicate, two declarators, escaped duplicate, type-only import | equal to main | | |

"Every entry flat" plus "all bound names unique" closes the duplicate-name class. It cannot close early errors in general, because prism reads tree-sitter trees, and those accept programs the language rejects.

Main already binds the same early-error shapes Exact when the value is a callable directly (`fx/q4b`: `catch (h) { const h = () => 1; h() }`, `const static = () => 2; static()`, `const eval = function(){}; eval()`). This is the base binder's existing contract, inherited, not something the alias follow adds. Such programs cannot run or bundle, so IMMATERIAL.

**Recommended closure (family, not instances).** One clause in the `CLAUDE.md` `Exact` paragraph: the binding grade assumes a program free of early errors, and guards refuse what they detect cheaply. Do not add more guards in this PR. **Alternative:** a reserved-word list plus a catch-parameter check in the binder for both `Callable` and `Alias*`. Cost: binder-wide semantics and new review surface for zero real-code yield. **Would not apply** if prism is ever asked to grade invalid programs.

---

## SMELL findings

**S1. The structural test checks only the predicate-false half of the rule — SMELL, IMMATERIAL (but take it before the returned-value slice builds on this rule).** See W1. Its inputs are only the matrix, so it cannot see shapes like W2 either; the check logic itself caught W2 when given the input. Fix: the predicate-true assertion and the two matrix shapes listed under W1 and W2. With these, the replica flags exactly the 144 and 30 sites.

**S2. The GitHub PR description is the pre-round-2 text — SMELL, IMMATERIAL, fix before merge (disclosure).** It still says:
- "It never refuses a name search";
- the stand-down applies "where the binder followed the alias";
- "For one shape the head has fewer records than main without a proven hit … the stand-down removes it" (this exception no longer exists);
- 41 mutants, 1,335 fixtures, "Round 2 is in progress";
- corpus measurements are from the `d22f8349` binary.

It never states the one-predicate rule the brief describes. Replace it with the rule, its two exceptions (W1, W2) or their fixes, and the current counts.

**S3. The brief's structural-test tallies (1,811 / 338 / 35 / 14) are from the 1,335-fixture matrix — SMELL, IMMATERIAL.** On this head the test prints 1,825 / 348 / 45 / 15 (303 and 6 are unchanged). The 14 fixtures added in `568c284f` account for the difference. My matrix sweep counts 1,825 site keys, not 1,846; my keying is (file, start byte, end byte), and the controller's may differ.

**S4. `to_callers_output` small items — SMELL, IMMATERIAL.**
- `listed` is computed for every alias site before the identity test, so each BFS pop costs |alias sites| × |bucket|. Test identity first, or index alias sites by target `FunctionId`.
- The queue's `Option<FunctionId>` is always `Some`.
- `listed` is redundant with `visited` (an equivalent-mutant candidate). This was carried from round 2.

---

## Answers to the five questions

**1. Is the rule true by construction?** Not literally, but the gap is small, closed and IMMATERIAL. The complete list of exceptions, by reading:

(i) **Resolver.** For a JS identifier site, `resolve_call_site_full` has two returns before the alias arm: the JSX intrinsic gate (W1) and an installed executable owner (same mechanism, by reading). The others (pre-resolved target, Rust scope graph, `::`, C/C++ static linkage, a qualifier arm) cannot apply to an `Alias*` site, because the binder assigns those only to unqualified identifier callees.

At a predicate-false site, the rest of the arm reads the binding in exactly three places:
- `import_proven`, false off the route;
- R4 `Unproven(_)` and R5 `Unproven(r)`, false for `Alias*` and `MayCall`.

All three treat `Alias*` as `MayCall("alias")`. The `namespace_refused` match sits in the qualifier arm, which `Alias*` never reaches. Empirically, 1,017 sweep sites plus 45 matrix sites were compared with their `MayCall("alias")` twin: 0 differences.

(ii) **Textual path.** Level 1 is the only reader of the predicate. The only cross-site read of Level-1 output is Level 4's `already_resolved` (W2). The stand-down's input state equals query-time state on both build paths: `recompute_indirect_calls` is the last mutation in each. Warm, partial-hit and cold caches give identical dumps (`fx/p3`, `fx/p5`). The seam's "held equals the audit" check had 0 failures over 4,680 sweep fixtures and the matrix.

(iii) **Callers.** The nav index reads the resolver's targets, so it stays consistent. `--format callers` reads the route's targets, so it is inconsistent under W1. The counter reads the predicate.

Everywhere else the `Alias*` variants are matched (the binder, the cache round-trip, the counters), they are treated as `MayCall("alias")` or only counted.

Beyond W1, W2 and the disclosed `function f() { function f() {}` shape, I found no predicate-true site with anything other than one `Source` record carrying the one Exact target. All 2,709 proven sweep sites match the constructed truth, and every removed textual record named the same target as the proven hit.

**2. Is the structural test sound?**
- The predicate-false half is sound. The rewrite matches main's real binding by construction: `Alias*` arises only when the base binding is `Alias`, `js_binding.rs` is unchanged, and the binding cache is a pure memo. The two-binary sweeps on the matrix and on my 4,680 fixtures agree.
- The seam matches the build (checked by the test itself, and 0 failures in my replica).
- The vacuity guards bite (303 true, 45 compared, 6 suppressed).
- It is unsound for half (i): predicate-true sites are skipped (W1, S1).
- Like any input-driven test, it visits only matrix shapes. W2 is a shape the matrix lacks, and the test's logic does catch it once given the input.
- It does not cover (iii), which `js_alias_callers_test.rs` and the follow tests cover on named inputs.

The controller's two-binary sweep is the independent check, and my rerun of it agrees: 0 other.

**3. Closure of the round-2 rows.**

| Row | State | Evidence |
|---|---|---|
| Pattern binding another name twice (sol W1) | Closed | `fx/c1`: `{ f, x: a, f: a }`, `{ f, f }` and `{ f: a, x: a, x }` equal main; the control `{ f: p, f: q }` follows both |
| Callable terminal returned its own drop (sol W2, Opus W2) | Closed | `fx/c2` (with decoy: main's `local_def → h@3`; without: `UnknownName`), `fx/c2b` wrapped nested, `fx/c5` one-line two functions: all equal main |
| Stand-down by binding kind (sol W3, Opus W1) | Closed | `fx/c3`: CJS own-name call and JSX keep main's two records; `fx/c3b` and the one-line collision `fx/c3c` equal main; `fx/c3e` is the disclosed shape. New instance in the same family: W2 |
| `import_proven` leak (Opus W3) | Closed | `fx/c3d` (paths re-entrant `F()`): `UnknownName` on both; the paths alias and a wrapped paths JSX alias still follow |
| `--format callers` by name (sol W4) | Closed | `fx/c4`: `run` credited to `m.js:f` only, not `b.js:f`. New instance via a different mechanism: W1 |
| Counters, comment, `CLAUDE.md` | Closed | The `CLAUDE.md` sentence and module comment are accurate. `*_proven` overcounts W1 sites. The PR description is stale (S2) |
| Taint descent, CommonJS proof | Owner follow-ons, not raised | |

**4. Pattern admission.** Yes, some early-error declarations are still followed: W3. The flat-and-unique admission is correct for what it targets (more than 20 shapes in `fx/q4` and `fx/c1`). The remaining early errors are the base binder's contract and open-class, so close them by contract wording.

**5. New and material from the rework.** Nothing material.
- `stand_down` is internal; its only caller passes `true`.
- `audit_indirect_records` is a `#[doc(hidden)] pub` seam that clones the graph, used only by tests.
- `js_ts_alias_route_outcome`/`_hit` are `#[doc(hidden)] pub`.
- The formatter's identity tracking fixes sol W4. Its one defect is W1, and S4 covers its cost.

---

## Probe log

| Hypothesis | Expected if true / falsifier | Result |
|---|---|---|
| The intrinsic gate pre-empts the alias arm while the predicate stays true | head drops `JsxIntrinsic` with no textual record, and `--format callers` lists the nested site / head equal to main | Confirmed (`fx/p1`). Alternative "the binder does not follow lowercase tags" refuted: `callable_proven: 3`, and the replica reports the predicate true |
| The stand-down un-suppresses same-line Level 4 | an extra head record at `obj.field()` / equal records | Confirmed (`fx/p2`). The direct twin and the other-line control have the record on both binaries; the seam on one graph isolates the stand-down |
| The rule test misses W1 and catches W2 | replica: 0 on `p1` plus 2 from the extra check; 1 on `p2` | Confirmed; sweep: 30 in-repo-logic violations, 144 extra-check violations, nothing else |
| Cache state can make the stand-down disagree with query-time predicate | warm or partial dump ≠ cold | Refuted (`fx/p3`, `fx/p5`) |
| Early errors are still followed | head Exact, main drops, `node --check` errors | Confirmed for the catch parameter and reserved words; main binds the direct-callable analogs |
| A require/import local collision misroutes the alias | alias ≠ direct twin | Refuted (`fx/p6`); TS overloads also equal the direct twin |
| First `c3d` probe | | Inadmissible: my `tsconfig` was not picked up, so the direct call dropped on both. Fixed by using the test helper's config shape, then confirmed closed |

## Follow-ups (identical on main, not made more reachable)

- Level 4's line-level dedupe (`already_resolved`) is itself a heuristic: on main, a qualified call's record depends on whether an unrelated Level-1 record shares its line.

---

### Checked and found sound

- All round-2 rows (table above), by two-binary fixtures.
- The predicate-false half of the rule, by reading the full arm and by 1,062 twin comparisons. The matrix re-run: 1,825 keys, 0 other. The out-of-matrix sweep: 6,480 keys, with every non-identical, non-proven key being W1 or W2.
- Proven targets: 2,709 sweep sites, all equal to the constructed truth. Every removed textual record named the same target as the hit.
- The seam equals the build (4,680 fixtures plus the matrix). Both build paths order export resolution before `recompute_indirect_calls`. Warm, partial-hit and cold caches are equal.
- The nav callers index counts each alias site once per exact target. `--format callers` identity holds across files.
- Pattern admission: duplicates, rest, nested, defaults, computed keys, escaped duplicates, cross-declarator duplicates.
- The diff-scope skeleton is unaffected: `compute_scope` is name-only (`resolve_callees`).
- The 82 `js_alias` integration tests and 19 binder tests pass. The mutation lane's 45 entries were reviewed by reading.

### Not checked

- Corpora: F (opened nothing private); X and T not re-run (no known path without searching outside allowed directories). The brief's corpus table is taken on trust.
- The full test suite. I ran 101 tests and rely on CI (`gh pr checks` not re-run).
- The mutation lane was not executed.
- The executable-owner instance of W1: by reading only.
- MCP tools, `nav ego`, the Tier-A harness.
- `--format callers` at depth greater than 1 through an alias.
- The recommended fixes: not built or run.
- Truth for my fixtures is by construction and by the direct-call twin, not by the TypeScript checker.
- The later stacked slices.

**Verdict: APPROVE** (nothing material remains; before merge, refresh the PR description (S2) and either name the two exceptions W1/W2 in the rule's statement or take their fixes; W1's family fix and S1's predicate-true assertion should land before the returned-value slice relies on this rule).
