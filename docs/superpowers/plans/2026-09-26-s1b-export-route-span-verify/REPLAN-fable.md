# Re-plan after the spec review cap: S1b-2..4, the collector (Fable, 2026-09-26)

**Status:** planning only; builds on the reviewed r2 packet (`740b7c77`). Nothing under `src/`, `tests/`, `eval/`,
`Cargo.*` or `CLAUDE.md` changes. **S1b-1 (§3.5, F1–F3) is untouched**: it is being implemented in a separate clone
and this re-plan needs nothing from it (§6, OQ6). The sections of `SPEC.md`, `IMPLEMENTOR.md` and `REVIEWER.md` this
file changes are marked **pending owner choice**; the r2 text is preserved so the reviewed artifact stays intact.

**Evidence:** `~/prism-evidence/s1b/replan/` (`EXPECTATIONS-pre-run.md` written before every run, `RESULTS.md` (copied to `probes/replan/`),
`probes/`, `fixtures/`, `out/`, `census/`, `MANIFEST.sha256`). Base `prism-base-a6d853f5` and prototype
`prism-proto-v8e` were run on synthetic fixtures; the corpora were measured with an independent Python tree-sitter
census (never a private F path or name in this repo; F is aggregates only).

## 0. Summary

- **The class is genuinely open under the r2 design, and the reason is structural, not a run of bad luck.** The r1
  fold made the unit of proof the *node kind* ("is this kind classified?"). Round 2 showed that a kind's rule can be
  right for one child position and wrong for another: `method_definition` is a function environment for its body and
  parameters but **not** for its computed key; `internal_module` is a var scope for what it declares but its lookup
  also sees the merged namespace's exports. B0 cannot catch either, because B0 keys on kind membership and every
  kind is a member (Opus S1). A probe pass over ECMA-262 / TS *positions* found a **third** wrong-Exact position on
  v8 in an hour (enum member scope, RP1-d2) plus two right-edge losses (`with (f())`'s object, nested dotted
  namespaces). That is the signature of an open class: new instances of the same kind on demand.
- **What closes it.** Change the unit of proof from *kind* to *(kind, child position) → environment*, with the list
  of environment-forming kinds cited to ECMA-262 §9.1's environment creators plus TS's two containers, an explicit
  rule for every child field of those kinds (a finite table read off `node-types.json`, about 25 rows), and a
  **positional fail-safe** that can actually fire: the pinned grammars allow 319 `(kind, field)` pairs, corpus sites
  cross 103, and 216 are never crossed (RP3). TS declaration merging is not an environment rule but a lookup rule;
  it is closed by a *leave predicate* (leaving a namespace or enum body whose name has a same-file partner block, or
  in a script file) rather than by modelling merging.
- **Measured cost of closing it: zero rows on every corpus.** The only corpus with evaluation-context exposure is
  T (4,312 sites cross a namespace body). Of v8's 870 changed T rows, 42 cross one; 41 bind *inside* the body (already
  proven by T4) and the one that walks out binds lexically in the enclosing namespace (right). The merge-partner
  predicate fires on **0** T sites. Decorators, computed member keys, `with` and enum bodies are crossed by **0**
  sites on X, F, R and T. So every option below that keeps v8's yield does so at no measured cost; they differ in
  closure quality, review surface and budget, not in edges.
- **The bounded round-2 items fold cleanly:** Opus W2's delimited-child sealing keeps all 19 rows the narrow E6 rule
  kept (T 12, F 7) and moves none of the 45 it refuses (RP4); sol W2's structural-token rule restores the string-brace
  row; sol W3 and the B0 leaf SMELL are small folds; the auditor already carries the three auditor-side fixes.
- **Recommendation: Option K** (evaluation-context table, positional fail-safe, **keep base at unproven positions**),
  with S1b-2 split into 2a (collector at module scope, 0 rows) and 2b (D4 wiring, 0 rows), and S1b-3 re-capped to
  carry the site-walk table. One owner-approved spec round on the re-planned packet.

## 1. Diagnosis

### 1.1 The findings are one class, and the r2 fold did not change its unit of proof

| Round | Findings on the collector | Where the rule was wrong |
|---|---|---|
| r1 (15) | static-block/namespace `var`, `import_alias`, TS assertion writes, catch/`with` hoisting, type space, implicit `arguments`, labelled functions, destructuring defaults, decorators | **what a scope declares** (the D/P/W rows) and **which kinds open a scope** (Σ) |
| r2 (2 + 3 bounded) | computed method keys (sol W1), namespace merging (Opus W1) | **which environment a child position is evaluated in** (E), and a TS lookup rule that is not lexical |
| this re-plan (RP1) | enum member scope (wrong Exact), `with` object position and dotted namespaces (right edges lost) | the same E class |

The r1 fold answered r1's class properly: the D-table and Σ are keyed to the spec, and no r2 finding touched them.
It answered the *next* class with B0, a fail-safe over kinds. But B0's runtime allowlist is the set of every non-leaf
kind of the pinned grammars (154 kinds; Opus S1), so it can only fire on a grammar upgrade. Every r2 miss is inside a
classified kind: the kind was right, a *position* of it was not. Sol named the missing dimension exactly: "make
evaluation context an explicit category in the closure table; grammar shape alone cannot establish it."

### 1.2 Probes (RP1; `probes/replan/RESULTS.md`)

Expectation before the run: if H1 ("the r2 misses are position errors") holds, reading ECMA-262 and TS scoping for
*positions* finds further instances without any new kind. Falsifier: every spec-derived position is already right on
v8. Result: H1 holds.

| Probe | Correct static denotation | v8 | Class |
|---|---|---|---|
| RP1-a/b/g computed member key (class and object literal), JS and TS | the key is evaluated in the class scope / enclosing environment, not the method's | Exact → the method-body `f` | **wrong Exact** |
| RP1-c namespace merge (TS) | `N.f` (the merged block's export) | Exact → outer `f` | **wrong Exact** |
| **RP1-d2 enum member scope (TS, new)** | `E.f` (a member, not callable) | Exact → outer `f` | **wrong Exact** |
| RP1-e `with (f()) {}` object position (JS, new) | outer `f` (ECMA §14.11.2 evaluates the object outside the object environment) | drop | right edge lost |
| RP1-f `namespace A {…} namespace A.B { f() }` (TS, new) | `A.f` | drop | right edge lost |
| RP1-h2 `class f extends f()` inside a function | the inner class binding (TDZ) | drop | v8 right |
| RP1-i `h(g, a = g())` | the parameter | drop | v8 right |

Two controls (h2, i) show the table is right where a *scope* handles the position; the misses are where the
structural parent chain and the evaluation environment part ways.

### 1.3 Why the class is closable (unlike S1's)

S1's re-plan class (runtime mutation) was open because JavaScript has ambient authority: no finite list closes it,
so closure had to be *declared*. This class is about **static** semantics, which is finite:

1. **Environment creators are spec-closed.** ECMA-262 creates environments in exactly these places: function
   instantiation (§10.2.11, including the separate parameter environment), Block and CaseBlock (§14.2.3, §14.12),
   `for` heads and per-iteration copies (§14.7.4–5), `catch` (§14.15.2), class definition (§15.7.14, the classScope
   and each field initializer / static block as a function), `with` (§14.11.2), Script/Module (§16). TypeScript adds
   two value containers, `namespace`/`module` bodies and `enum` bodies, whose lookup also sees the merged declarations
   of the same name. That list is Σ (r2 T1–T9) plus `enum`. It can be reviewed against the spec's table of contents,
   not against a reviewer's imagination.
2. **Positions are finite and mechanically enumerable.** For every kind in Σ ∪ {namespace, enum, decorator holders},
   `node-types.json` lists its fields. Each `(kind, field)` gets one rule (§3 below): *Inside* (the child is evaluated
   in the environment the kind opens), *Outside* (evaluated in the enclosing environment: computed keys, decorators,
   the `with` object, `extends` in the class scope but outside the members), *Parameter* (J1), or *Leave-check*
   (namespace/enum bodies). Kinds not in that set have no environment of their own, so all their positions are
   structural by the spec's evaluation rules (an operand is evaluated in the running context's current environment).
3. **The fail-safe can fire.** A positive `(kind, field)` allowlist is a real fail-safe: 319 possible pairs, 103
   crossed on four corpora, 216 never crossed. A future grammar kind, or a Σ-kind field the table does not list,
   hits it. B0 stays for grammar upgrades (extended to leaf kinds, sol S4).
4. **TS merging is a lookup rule, closed by a predicate rather than a model.** Inside a namespace or enum body, an
   unqualified name that is not declared in the body may denote a merged partner's export (RP1-c, RP1-f) before it
   denotes anything lexically outer. Modelling merging means modelling namespace+function, namespace+class,
   namespace+enum, `export import`, dotted names and, for scripts, cross-file merges: an open vocabulary again. The
   closed alternative: when the walk *leaves* a namespace or enum body, and the body's namespace has another
   declaration in the file sharing its first name segment, or the file is a script (no top-level import/export), the
   binding is **unproven**. Measured: 0 sites on T, and no namespace-body sites elsewhere (RP3b).

### 1.4 What "closed" then means, and what remains a judgement

A round-3 WRONG on the collector has to be one of: a Σ-kind field row whose rule mis-cites the spec; an environment
creator missing from Σ (must be cited); a D-row miss (two rounds found none in r2); or a hole in the leave predicate.
Each is a bounded fix on a listed row. Under the recommended policy (§2, K), any position **not** listed gives the
row base behavior, so a newly noticed position can cost at most "S1b did not fix this base row", never a new wrong
Exact. That is the property the r2 design lacked.

The remaining judgement is the *policy* at unproven positions (keep base vs refuse), an owner call (OQ1), and the
usual B1 tokenization assumption.

## 2. Options for S1b-2..4

All yields are **projections** from v8d's audited row-diffs joined with the position census (RP3); no new prototype
was built. Where a projection could over-estimate, it is said. Budgets are v8 function attributions (SPEC §9, Q34)
plus estimated deltas; the implementer recounts and the cap governs.

| Option | Design | Changed rows X / F / R / T (v8: 153 / 35 / 0 / 868) | Right edges lost beyond E6 (F 4, T 41) | src estimate (2a / 2b / 3 / 4) | Residual risk | Convergence outlook |
|---|---|---|---|---|---|---|
| **K** keep-base at unproven positions (recommended) | E-table over Σ-kind fields; positional fail-safe → `Unchecked` (base) with a counter; leave predicate → base; computed keys, `with` object, enum body modelled (§3) | **153 / 35 / 0 / 868** (0 rows at unproven positions on any corpus) | **0** | ~450 / ~120 / ~440 / ~170 = ~1,180 | base's own wrong Exacts at unproven positions stay (0 measured; RP1-c-shaped inputs keep base's 2-target row); B1 tokenization | **converging**: review surface = ~25 table rows + Σ citation + leave predicate; an unlisted position can only preserve base |
| **A** refuse at unproven positions | as K, but the fail-safe and the leave predicate drop the edge (`LocalBindingUnproven`) | 153 / 35 / 0 / 868 (identical on the corpora: the refusals fire on 0 sites) | 0 measured; synthetic RP1-f-shaped code loses its right edge | as K (+~5) | a future unlisted position removes right edges silently (counter-visible); contradicts the owner's "defer rather than refuse" | converging as K; reviewers may argue each refusal's recall cost |
| **B-keep** conservative allowlist, keep base elsewhere | E-table limited to structural + parameter env + function/block/class/catch/for/switch bodies; namespace bodies, computed keys, decorators, heritage, `with`, enum bodies all → base | 153 / 35 / 0 / **826** (T's 42 namespace-body rows stay base: 13 wrong-removals and 28 right re-targets forgone) | 0 | ~450 / ~120 / ~370 / ~170 = ~1,110 | gives up T4's already-reviewed namespace rows; base's multi-target rows inside namespaces remain | highest: the smallest table; but forgoes rows two rounds already proved |
| **B-refuse** conservative allowlist, refuse elsewhere | as B-keep but unproven → drop | 153 / 35 / 0 / 868 + **3,201 T removals** | **T −3,195 `local_def` edges** (every site under a namespace body) | ~1,110 | unacceptable recall on T; measured to show what "provably closed by refusal" costs | n/a |
| **D** park S1b-3; ship 2a/2b + 4 only | module-scope collector and R3 only; the site walk deferred | **0 / 4 (Exact→NameOnly) / 0 / 0** | 0 | ~450 / ~120 / – / ~170 | forfeits S1b's measured value (the R4 rows: X 153, F 31, T 817 wrong edges) | trivially converging; not worth the slice |

Not on the table: patching the r2 table probe by probe (the path that did not converge), and modelling TS merging
now (§1.3.4; a follow-up lane if OQ2 asks for it).

**Why K and A tie on the corpora and differ in principle.** The census shows the refusal set is empty on X, F, R, T
after the E-table is in place; the owner's stated preference (maximize correct edges, defer rather than refuse)
decides between them, and `Unchecked` already exists in v8 as "base behavior" (`JsLocalBinding::Unchecked`), so K
costs no new mechanism. K also matches the E5 precedent: rows S1b cannot prove keep base and are counted.

## 3. The evaluation-context table (design text for SPEC §3.1a; pending owner choice)

**Environment creators (Σ′).** Σ as in r2 T1–T9, plus **T10** `enum_body` (a TS value container: its member names,
non-callable, for the members of *this* block). Cited: ECMA-262 §10.2.11, §14.2.3, §14.7.4–5, §14.11.2, §14.12,
§14.15.2, §15.7.14, §16; TypeScript namespaces and enums (declaration merging).

**Positions.** The site walk moves from a node to its parent and, at every step, consults `E(parent kind, field of
the child it came from)`. For a parent kind **not** in Σ′ ∪ {`decorator`}, the position is *Structural* (the child is
evaluated in the parent's environment) and the walk continues. For a parent in Σ′ ∪ {`decorator`}, the field must be
listed:

| Kind | Field | Rule | Why |
|---|---|---|---|
| function-like (T2) | `body` | Inside | the function environment |
| function-like | `parameters`, `parameter` | Parameter (J1: the parameter environment, then above the function) | §10.2.11 steps 19–28 |
| function-like | `name`, `return_type`, `type_parameters` | never a site position (type space / a binding); fail-safe if reached | – |
| `method_definition` | `name` (a `computed_property_name`) | **Outside** (J3): skip this method's environment; continue at the member's owner (the class scope for a class member, the enclosing environment for an object-literal member) | §15.7.14 ClassElementEvaluation evaluates keys in classScope; §13.2.5 for object literals |
| `public_field_definition` / `field_definition` | `name` (computed) | Structural (the field is not a scope; the walk reaches the class, T8) | §15.7.14 |
| `public_field_definition` / `field_definition` | `value` | Structural (an initializer is a method whose lexical environment is the class scope; `arguments` is an early error there) | §15.7.10 |
| class kinds (T8) | `body`, `class_heritage` child | Inside (classScope: the inner name binding is visible, TDZ at runtime) | §15.7.14 steps 3–8 |
| class kinds | `decorator` child | **Outside** (J2): above the class; for a **named class expression** the ECMA and TS-legacy readings differ, so the position is *unproven* (K: base) | decorators proposal / TS legacy emit |
| `decorator` (any holder) | all | Outside (J2) as r2 | as above |
| `class_static_block` | `body` | Inside (T4 var scope) | §15.7.11 |
| `statement_block`, `switch_body`, `program` | children | Inside | §14.2.3, §14.12, §16 |
| `for_statement` | `initializer`, `condition`, `increment`, `body` | Inside (the loop environment; TDZ for the head's names) | §14.7.4 |
| `for_in_statement` | `right`, `body` | Inside (TDZ environment for `let`/`const` heads; the head names are declared by T5, so a callee naming one refuses as not callable) | §14.7.5.6 |
| `catch_clause` | `parameter`, `body` | Inside | §14.15.2 |
| `with_statement` | `object` | **Outside** (continue above the `with`) | §14.11.2 step 1 |
| `with_statement` | `body` | Refuse `with` (as r2) | object environment |
| `internal_module` / `module` | `body` | Inside (T4 var scope) while the name is declared in the body; on **leaving** the body: the **leave predicate** | TS declaration merging |
| `enum_body` | children | Inside (T10: the block's member names); on leaving: the leave predicate | TS enum merging |
| anything else in Σ′ | any field not listed | **fail-safe** `unproven_position` → K: base + counter; A: refuse | – |

**Leave predicate.** When the walk leaves a namespace or enum body without having bound the name, the binding is
*unproven* iff (a) another `internal_module`/`module`/`enum_declaration` in the file has the same first name segment
(a merge partner, including dotted `A.B` against `A`), or (b) the file is a script (no top-level `import_statement`
or `export_statement`; cross-file global merging is possible). Otherwise the walk continues lexically (the enclosing
namespace body is an ordinary lexical scope: T's one leaving row, RP3b). Measured: fires on 0 corpus sites.

**Fail-safe policy (OQ1).** K: `JsLocalBinding::Unchecked` (base behavior), counted per reason in
`local_binding_unchecked_position: {reason: n}`. A: `Unproven`.

**Derivation and check.** `probes/grammar_closure.py` gains a second table: for every kind in Σ′ ∪ {`decorator`},
every field from `node-types.json` must have a rule above, else the probe exits non-zero; the runtime `E` table is the
same data. B0 is extended to leaf kinds (sol S4): every *named* kind, leaf or not, is in the allowlist, with the leaf
allowlist derived by the same probe.

**Controls to add (both grammars unless TS-only):** RP1-a/b/g (computed keys: key → Outside, body → inner), RP1-c
(merge partner → base under K / drop under A), RP1-d2 (enum member → not callable), RP1-e (`with` object → Exact),
RP1-f (dotted partner → base / drop), RP1-h2 and RP1-i (unchanged), a script-file namespace twin of RP1-c, a named
class expression with a decorator (TS), and one fail-safe mutant (remove a table row → the counter appears, rows go
base). Each row is a RED on v8's behavior as recorded in `RESULTS.md` (copied to `probes/replan/`).

## 4. Fold plan for the bounded round-2 items

| Item | K / A / B-keep | D |
|---|---|---|
| **Opus W2** header errors escape B1 | **Fold** in S1b-2a's sealed-error rule: an error is sealed only inside a *delimited child* of the sealing node (`body` with paired braces, `parameters` with paired parentheses, `class_body`); header errors refuse. Measured cost **0 rows** (RP4: T 12 / 12 and F 7 / 7 kept; the 45 E6 refusals unchanged), so **E6 stands as chosen** | same |
| **sol W2** raw-text braces | **Fold** in S1b-2a: only anonymous structural `{`/`}` tokens under an `ERROR`, or `MISSING` braces, break condition (i); string, template, regex and comment text never do. Controls: both quote forms, JSX and TSX. The auditor is already folded (`probes/jsscope.py` r3: RP2-b → `callable`) | same |
| **sol W3** string-literal export names | **Fold** in S1b-2b: a `ModuleExportName` helper computes the ECMAScript StringValue (both quote forms, escapes) for string-literal specifiers on the export **and** import sides, so `export { f as "g" }` / `import { "g" as h }` reach R3 and R4c. Prevalence: **0** real occurrences on X, F, R, T (T's three grep hits are comments), so it is a correctness fold with no corpus row; pin with JSX/TSX controls. Alternative: pin as a non-goal (OQ5) | fold in 2b |
| **sol S4** B0 leaf kinds | **Fold** in S1b-2a: classify every named kind including leaves (§3 "Derivation and check"); ~10 lines | same |
| **Opus S1** closure claim overstated | **Fold** by this re-plan: the SPEC's closure claim becomes the §3 statement (creators cited, positions listed, fail-safe can fire), and the TS merge row is the leave predicate | same |
| **Opus S2** auditor `with` tuple | **Folded now** in `probes/jsscope.py` (returns `[n]`) | same |
| **Opus S3** 2a/2b split | **Adopted** (§5) | n/a |

Nothing here needs S1b-1 to change: B1 lives in S1b-2a, the string-name helper in 2b, and S1b-1's F1–F3 are the
shared pattern/parameter collectors, which no fold touches.

## 5. Recommendation (principal-engineer stance)

**Option K**, for four reasons.

1. **Consistency of the confidence contract.** Exact is a static-binding grade "as prism models it" (`CLAUDE.md`).
   K states the model at the granularity where the r2 findings lived (positions), cites the environment creators to
   the spec, and makes the fail-safe an actual fail-safe. It does not reintroduce refusal as a substitute for proof:
   an unproven position is *base*, counted, and visible in `call-stats`, exactly as E5's may-call rows are.
2. **Correct edges.** K keeps all of v8's audited yield (X 153, F 35, T 868 rows; T 1,799 extra targets removed) at
   zero measured cost. A ties on the corpora but would silently remove right edges on RP1-f-shaped code, against
   the owner's preference. B-keep forgoes 41 T rows two rounds have already proved. D forgoes the slice's value.
3. **Reviewer convergence.** The round-3 surface is enumerable: ~25 table rows with citations, one Σ′ list, one
   predicate, and a derivation probe. Under K a reviewer who finds an unlisted position finds a *base-preserved row*
   and files it as a SMELL with a bounded fix (add the row); a WRONG needs a listed row to be mis-cited. That is a
   closed, converging shape, and it is what both reviewers asked for (sol: "an explicit evaluation-context table";
   Opus: "closure rests on each classified kind's rule being semantically right", which the table now states per
   position).
4. **Long-term health.** The E-table is the seam the next JS/TS slices need anyway (S2 qualifiers, `paths`,
   Python parity): a site → environment walk that is data-driven and checked against the grammar, instead of a
   hand-rolled `next_up`.

**Slicing.** Split S1b-2 into **S1b-2a** (the collector: scope index, D/P/W rows, classification, B0 with leaves, B1
with structural tokens and delimited-child sealing, the module terminal; **0 corpus rows**, reviewed by table, controls
and the closure probes) and **S1b-2b** (D4 wiring: `VerifiedLocal`, `ResolvedJsExport.wrapped`, the barrel key, the
R4c gate, the `ModuleExportName` helper; **0 corpus rows** on X/R/T and F). Reason: 2a is the largest review surface
and the one that must converge on the table; 2b is mechanism fidelity and serde. **S1b-3** owns the site walk with the
E-table, the leave predicate, the positional fail-safe, the scoped write index, `CallSite.local_binding` and
`useCallback`; it is re-capped (below) because the table moves in. **S1b-4** is unchanged.

**Budget (estimates from the v8 attribution; caps ≈ estimate + 10%; tests from S1's ratio and the table rows):**

| Sub-slice | v8 src attribution | Delta (K) | src estimate | src cap / early stop | tests cap / report point |
|---|---|---|---|---|---|
| S1b-1 | 37 | 0 | 37 | **60 / 54** (unchanged) | **200 / 180** |
| S1b-2a | 60 + 127 + 48 + 93 + 29 + 48 + 20 + 15 = 440 | +8 leaf allowlist, +6 structural tokens, +12 delimited child | ~466 | **510 / 465** | **560 / 500** |
| S1b-2b | 48 + 30 + 15 = 93 | +30 `ModuleExportName` (export + import sides) | ~123 | **140 / 125** | **200 / 180** |
| S1b-3 | 125 + 44 + 70 + 40 + 10 = 289 | +35 E-table data, +30 table-driven walk, +10 J3, +35 leave predicate (partner index, script check), +15 enum scope, +5 `with` object, +12 counters | ~431 | **480 / 430** | **700 / 630** |
| S1b-4 | 167 | 0 | 167 | **185 / 165** (unchanged) | **330 / 300** |

Combined src cap 1,375 against r2's 1,195; the growth is the table and the predicate, which is the closure. Under
B-keep, S1b-3 is ~370 (cap 410); under D, S1b-3 does not dispatch.

## 6. Owner questions

| # | Question | Options and tradeoffs | Recommendation |
|---|---|---|---|
| **OQ1** | Policy at an unproven position (fail-safe and leave predicate) | (a) **keep base** and count (`Unchecked`): 0 right edges lost, base's own wrong Exacts at such positions stay as today (0 measured); (b) refuse (`Unproven`): same corpus rows, but synthetic RP1-f-shaped code loses right edges and a future unlisted position removes edges silently | (a) |
| **OQ2** | TS declaration merging | (a) **leave predicate** (partner in file or script file → OQ1's policy), 0 corpus rows, closed; (b) model merging now (~40 src; namespace+function/class/enum, `export import`, dotted, cross-file scripts: an open vocabulary), recorded as a follow-up lane instead; (c) unproven on *any* leave: T 1 changed row reverts under (a)-policy, T 257 base rows lost under refuse | (a), with (b) as a follow-up lane if a corpus ever shows partners |
| **OQ3** | Slicing and caps | (a) **five sub-slices** S1b-1 / 2a / 2b / 3 / 4 with the §5 caps (2a 510, 2b 140, 3 480); (b) keep four with S1b-2 at 650 and S1b-3 at 480 | (a) |
| **OQ4** | Round-3 authority | the 2-round spec cap is spent; one **owner-approved** spec round on the re-planned packet (both reviewers, the §7 addendum), judged against the fixed model; an APPROVE dispatches 2a | approve one round |
| **OQ5** | sol W3 string-literal export names | (a) **fold** in 2b (~30 src, 0 corpus rows, both sides); (b) pin as a non-goal with a test | (a) |
| **OQ6** | S1b-1 | **no change needed**: B1's fixes live in 2a, the string-name helper in 2b, no fold touches F1–F3 or §3.5. Stated for the record so the S1b-1 implementer is not blocked | – |
| **OQ7** | Named class expressions with decorators | (a) **unproven** (OQ1 policy; 0 corpus sites), (b) J2 as r2 (ECMA classScope reading disagrees for the inner name) | (a) |

## 7. Review brief addendum (round 3, owner-approved; for `REVIEWER.md`)

> ## Round 3 (owner-approved, final): the evaluation-context model is fixed
>
> The owner has set the collector's model for this round (`REPLAN-fable.md` §3, SPEC §3.1a). Judge the packet
> **against this model**, not against a stronger one:
>
> > A JS/TS name denotes a callable when the site's walk, moving from node to parent and consulting the
> > evaluation-context table `E(kind, field)` at every step, reaches the nearest environment (Σ′: ECMA-262's
> > environment creators plus TS namespace and enum bodies) that declares the name, and that environment holds
> > exactly one declaration whose value is a function (or an admitted React wrapper), passes B0, B1 and the
> > leave predicate, and is not in the may-call class (E5). A position the table does not list, or a namespace/enum
> > body left with a merge partner or in a script file, gives the row **base behavior**, counted (owner OQ1). Runtime
> > mutation, `eval`, host globals and `require`/`import()` re-acquisition are out of model.
>
> **What is a WRONG under this model.** Give the input, the incorrect result, the mechanism, the spec clause and a
> file:line.
> - An Exact edge from a **listed** position whose rule mis-cites the spec (for example a field of a Σ′ kind marked
>   *Inside* that ECMA-262 evaluates outside), or from an environment creator **missing from Σ′** (cite the clause
>   that creates it).
> - A declaration form a scope holds that the D/P/W rows miss or misclassify (r2's class; none found in round 2).
> - A leave-predicate hole: a same-file merge partner, or a script file, that the predicate does not detect.
> - A parse-recovery shape that escapes B1 as folded (structural brace tokens; delimited-child sealing).
> - A removed or changed edge that was right and is not an accepted cost (SPEC §0, §8: E6's refusals).
> - Any changed row in the may-call class; a refusal or base-fallback not counted, or counted twice; a counter that
>   disagrees with the row-diff; a cap breach.
>
> **What is not a WRONG.** A base row that S1b leaves at base because its position is unlisted or its walk left a
> namespace/enum body under the predicate: that is the owner's policy, counted in
> `local_binding_unchecked_position`; file it as a **SMELL** with the row to add if you think it should be proven.
> Runtime mutation and the SPEC §2 non-goals, as before. If you think the policy or the model is wrong, say so under
> **Convergence view** with the edge cost, not as a finding.
>
> **Per finding**, keep the shape: tag (WRONG / SMELL); a recommended fix that is bounded and specific; at least one
> alternative with its cost, risk, scope and maintenance tradeoff, including "accept and document" where reasonable;
> and **when this finding would not apply**: the assumptions it rests on (the model, the reachable inputs, SPEC §2,
> an owner decision) and how confident you are that they hold. Report WRONG items first; a finding without a concrete
> failure scenario is a SMELL.
>
> **Prior-round closure.** For r2 Opus W1, W2, S1–S3 and sol W1–W3, S4: CLOSED, NOT CLOSED, OUT OF MODEL (name the
> test) or DEFERRED (name the owner decision).
>
> **Also check:** the derivation probe (`grammar_closure.py`, both tables) exits 0 and its data equals the runtime
> tables; the RP1 controls and the fail-safe mutant are pinned in both grammars; the census join (RP3) reproduces
> from the recorded commands.
>
> Review as a senior or principal engineer whose goal is prism's long-term health: rigorous in finding real defects,
> never inventing them. A clean result is a valid result.

## 8. Not settled here

- **The policy fork (OQ1) and the merge question (OQ2)** are the owner's; the yields tie on the corpora, so the
  choice is about principle and future behavior, and §5 says which way I would go.
- **No v9 prototype was built.** Yields are projections from v8d's audited row-diffs and an independent census
  (RP3); the census shows the E-table's refusal/keep-base set is empty on all four corpora, so the projection is
  exact for changed rows, but an implementation must re-measure (`probes/rowdiff.py` against v8d's dumps must be
  byte-identical on X, R, T and F before acceptance).
- **The auditor still shares v8's position blindness** for computed keys, enum bodies and the `with` object (its
  r3 header says so). The implementer's RED rows for RP1 are the independent check; a second-author auditor was not
  written.
- **Budget lines are estimates** from the v8 function attribution plus per-item deltas, not a recount.
- **`with` in TS files**: T's grammar accepts `with`; the census found 0 sites crossing one; no rule changes.
- **Cross-file namespace merging in script files** is handled by the predicate's script clause only; a script
  corpus was not measured (none of X, F, R, T has namespace-bearing scripts).
