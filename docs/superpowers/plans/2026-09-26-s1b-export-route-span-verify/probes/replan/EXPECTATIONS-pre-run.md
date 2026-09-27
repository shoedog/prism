# S1b re-plan probes: expectations written before each run (Fable, 2026-09-26)

Binaries: `../planning/bin/prism-base-a6d853f5` (B0, `df22fbcf…`) and `../planning/bin/prism-proto-v8e` (P8,
`346eddef…`, identical to v8d with no switch set, Q33). Every nav command uses `--no-cache`. Outputs under `out/`.
Corpus F: raw outputs stay here; only aggregates go into the packet.

## RP1 Evaluation-context probes (synthetic, both grammars where the syntax exists)

Hypothesis H1: the round-2 misses are *position* errors: a classified kind has a child position that is evaluated
in an environment other than the one the structural parent chain implies (or than the walk's J1/J2 jumps give). If
H1 holds, I expect to find further instances by reading ECMA-262 / TS scoping for *positions*, without touching any
new kind. Falsifier: every position I derive from the spec is already right on v8 (then the class is two one-off
kind rules, and the controller's open-class call is too strong).

| Probe | Input | Correct static denotation | Base expected | v8 expected | Class |
|---|---|---|---|---|---|
| RP1-a (sol W1) | `class C { [f()]() { function f(){} return f() } }`, no outer `f` | key call: unbound; body call: inner `f@3` | both Exact → inner | both Exact → inner (key call WRONG) | wrong Exact |
| RP1-b (sol W1 + outer) | as a, plus module `function f(){return 2}` | key → outer `f@1`; body → inner | both: 2 targets | both → inner (key WRONG) | wrong Exact |
| RP1-c (Opus W1) | `function f(){}`; `namespace N { export function f(){} }`; `namespace N { export function run(){ return f() } }` (TS only) | `N.f` (merged namespace export) | 2 targets | Exact → outer `f@1` (WRONG) | wrong Exact |
| RP1-d (new: enum member scope) | `function f(){return 1}`; `enum E { f = 1, g = f() }` (TS only) | `E.f` (enum member, not callable) → refuse | Exact → `f@1` | Exact → `f@1` (WRONG) | wrong Exact |
| RP1-e (new: `with` object position) | `function f(){return {}}`; `function run(){ with (f()) { return 1 } }` (JS only) | outer `f@1` (the object expression is evaluated outside the object environment, ECMA §14.11.2) | Exact → `f@1` | drop `with` (recall loss) | right edge lost |
| RP1-f (new: nested-namespace visibility) | `namespace A { export function f(){} }`; `namespace A.B { export function g(){ return f() } }` (TS only) | `A.f` | Exact → `A.f` | drop unbound (recall loss) | right edge lost |
| RP1-g (sol W1, object literal) | `function f(){return 2}`; `const o = { [f()]() { function f(){} return f() } }` | key → outer; body → inner | 2 targets each | both → inner (key WRONG) | wrong Exact |
| RP1-h (class heritage, control) | `function f(){ return class {} }`; `class f extends f() {}` | inner class binding (TDZ) → not callable → refuse | Exact → `f@1` | drop (T8) — v8 right | v8 right |
| RP1-i (parameter default, control) | `function g(){}`; `function h(g, a = g()){}` | parameter → refuse | Exact → `g@1` | drop — v8 right (C122) | v8 right |

## RP2 Bounded round-2 items (synthetic)

| Probe | Input | Correct | Base expected | v8 expected |
|---|---|---|---|---|
| RP2-a (Opus W2 header error) | `function f(){return 1}` / `function run(){ let f = 3 ) \` const g = () => { return 2; }; f(); }` | `let f` shadows → refuse | Exact → `f@1` (base wrong too) | Exact → `f@1` (WRONG, B1 hole) |
| RP2-b (sol W2 string brace) | `function f(){return 1}`; `function broken(){ const x = "{" "y"; return x }`; `export function run(){ return f() }` | `f@1` | Exact | drop `LocalBindingUnproven` (recall loss) |
| RP2-c (sol W3 string export name) | lib `function f(){} export { f as "g" }`; app `import * as ns from "./lib"; ns.g()` | Exact `import_qualified` → `f` | `UnknownName` | `UnknownName` (recall gap) |

## RP3 Position census (Python tree-sitter, independent of prism)

For every unqualified call (`call_expression` whose `function` is an `identifier`) and every plain JSX tag in X, R, T
and F, record the `(parent kind, child field)` pairs the identifier's ancestor path crosses. Expectations:

- The set of distinct pairs crossed is small (I expect under 150 on the union) against the grammar's possible
  pairs (hundreds), so a positive `(kind, field)` allowlist is a *real* fail-safe (it can fire), unlike B0.
- The "evaluation-context" positions (parameter lists, computed keys, decorators, heritage, namespace bodies,
  enum bodies, `with`) are crossed by **under 2%** of sites on every corpus. If so, an option that refuses (or
  keeps base) at those positions costs little recall.
- Joined with the v8d row-diff: **under 5%** of v8's changed rows sit on such positions. Falsifier: a large share
  of T's 868 changed rows cross a namespace body (T is the TypeScript compiler; if the pin still uses
  `namespace ts`, the cost of refusing namespace bodies is large and Option B is expensive).

## RP4 B1 delimited-child re-measure (Python)

For the rows the narrow E6 kept and the broad rule refused (F 7, T 12; `planning/proto-runs/v8d-broadE6/*-vs-default`),
re-check with Opus's delimited-child rule (an error is sealed only inside a child whose first and last tokens are
paired delimiters). Expectation: **most of the 19 stay kept** (their errors are grammar gaps inside bodies, Q13);
any that fall are a recall cost of the Opus W2 fold, reported per corpus.

## Addendum 1 (after the first RP1 run)

RP1-d and RP1-h produced **no call-site row on either binary**: prism extracts sites only inside functions
(`all_functions()`), and both fixtures placed the site at module scope. Inadmissible for the hypothesis, not evidence
against it. Re-run as RP1-d2 / RP1-h2 with the enum / class nested inside `export function host() { … }`. Expectations
unchanged: d2 base Exact → `f@1`, v8 Exact → `f@1` (WRONG: `E.f`); h2 base Exact → `f@1`, v8 drop (right).
