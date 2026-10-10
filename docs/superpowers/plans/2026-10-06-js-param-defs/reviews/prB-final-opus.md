# PR-B "callback identity" — final review (Opus), candidate `2431cb10` (PR #351)

Reviewer: Opus 5.5, read-only. Date 2026-10-09.
Head built from `git archive origin/wip/js-param-defs-prB-r3` (`2431cb10`), main from `origin/main`, both `cargo build --release` with `probes/byte_dump.rs` as an example. Every "reproduced" row below comes from those two fresh binaries, not from `repair-r7/bin/`.

Goal this review serves (owner contract): zero false Defs or edges; no LOST correct rows against main; Exact stays a static-binding grade; call sites, non-JS output and symbol/call/module navigation byte-identical. E13 (Flow) and option (b) are decided and are not re-litigated here.

**Materiality rule used.** MATERIAL = the defect either occurs in a measured corpus or is triggered by a shape that is idiomatic in the code PR-B targets. IMMATERIAL = reproduced, but only on shapes with no measured incidence and no realistic incidence. IMMATERIAL findings are recorded, not blockers.

## Verdict: FIX (closed, enumerable)

The design holds. The fence, own-scope containment, kill-only nested writes, `this` rebinding, SEAM / EVAL / ARGS on their core shapes, the R7 parameter kinds and navigation containment all behaved correctly under attack (section "What held"). Two product defects and one evidence defect need an owner decision or a bounded fix before landing; the rest is recorded.

1. **F1** — lowercase JSX tag names become Uses in synthetic passes, including a false **Exact** edge. Idiomatic React; zero measured incidence. ~10-line fix.
2. **F2** — PR-B's early-error refusal fires on parse-recovered parameter lists and drops whole named functions: **595 oracle-CORRECT LOST rows** in two SecBench roots, one of them in a file with no `@flow` pragma. A one-line guard restores 589 of them (measured what-if).
3. **F3** — the certificate's "SyntaxError" class is not proof of an early error; it is what hid F2. E13's stated extent is understated.

The owner can alternatively re-affirm E13 with the corrected extent (F2/F3 become disclosure-only) and accept F1 as a disclosed non-goal. That is an owner call, not mine: F2 sits on the boundary of a decision the owner made on incomplete numbers.

Convergence note: F5–F8 are four more instances of "syntactic predicate boundary" — the kind every round since R3 has produced. That class is open in principle. None is material; I recommend recording them and not looping on them.

---

## WRONG findings

### F1 — WRONG / MATERIAL — intrinsic JSX tag names are admitted as Uses in synthetic passes

**Input → incorrect result (reproduced, `.tsx` and `.jsx`):**

```tsx
registry.add((img) => {
  const cls = pick(img);
  return (
    <img
      className={cls}
    />
  );
});
```

Head emits `<cb@1:14>|img@L1:14-17 (param) -> img@L4:65-68  exact`. Bytes 65–68 are the tag name in `<img`; line 4 contains no read of `img`. `<img>` is an intrinsic element, not a reference to the formal. Main emits nothing here (no pass), so this is an ADDED false Exact edge.

Same mechanism, NameOnly label:
- `options.map((option) => (\n <option value={option.value}>…` → Use endpoint is the tag name `option`, not `option.value`.
- `links.map((a) => (\n <a href={a}>\n {a}\n </a>\n))` → a row to line 4, whose only token is the closing tag `</a>`.
- `(i) => … <i className="fa" />` → row to the `<i` line.
- Control: `items.map((Item) => (\n <Item />\n))` → row to `<Item` is correct (capitalised = value reference).

**Mechanism (static):** tree-sitter gives JSX element names kind `identifier`. `js_ts_span_is_non_reference` (`src/ast_callback_identity.rs:830-844`) filters only `property_identifier | private_property_identifier | statement_identifier | type_identifier`, and `js_ts_identifier_is_not_a_read` (`:930-1000`) has no JSX arm, so `collect_fenced_simple_refs` (`:1165-1176`) admits the tag name as the line's first occurrence. B-D9 lists "JSX attribute names" as filtered; tag names were missed.

**Incidence (measured):** zero. I dumped head on X and scanned every row endpoint preceded by `<` or `</` with a lowercase name: 0 of 75,619 rows (scanner positive-controlled on the fixture: 4 hits). Named passes on main have the same legacy defect (`function E(label){ return (\n <label>{label}</label> ) }`), also 0 on X.

**Why the tables cannot bound it:** in the oracle's `noLib` program, `getSymbolAtLocation` on an intrinsic tag name returns no symbol (I ran it: `<option>`, `<a>`, `<path>`, `<label>` → NONE; `<Item>` → Parameter). `adjudicate.cjs` then returns UNDECIDED (`no_symbol_declaration`), never WRONG. ADDED UNDECIDED is 0 on X/T/SecBench, which agrees with zero incidence; F's "all CORRECT" implies the same but I could not check F.

**Fix (recommended):** in the reads-only path, refuse an `identifier` that is the `name` field of `jsx_opening_element` / `jsx_closing_element` / `jsx_self_closing_element` when its text starts with a lowercase ASCII letter or contains `-`; refuse `jsx_namespace_name` parts. Add the same arm to `js_ts_span_is_non_reference`. One test per element kind plus the capitalised and `<a.B>` controls. Rows can only shrink, so re-tabling X and F is a diff of removed rows.
**Alternatives:** (a) disclose as a non-goal alongside legacy E7 and ship; (b) fix it for legacy named passes too (changes main rows — out of PR-B's scope).

**Self-critique — when it would not apply:** if the owner reads "zero false edges" as "zero on measured corpora", this is IMMATERIAL today (0 rows on X, and by inference F). Single-line callbacks get a SameLine label and a line that does contain a real read, so only the byte endpoint is wrong there. I rate it MATERIAL because the shape is the canonical React list idiom, the multi-line form yields a false Exact, and the oracle is blind to it.

**Classification:** CLOSED INSTANCE of E7 (Use at a non-reference token; B-D9 filter list incomplete).

---

### F2 — WRONG / MATERIAL — the early-error refusal fires on parse-recovered parameter lists and drops named functions main gets right

**Input → incorrect result (reproduced, fresh head vs fresh main):**

```js
// f1.js (also reproduces without `export`, in a sloppy script, and for a class constructor)
export function j(a: string, b: string): string {
  let x = a;
  return x + b;
}
```

Main: `j|x@L2 -> j|x@L3 exact` (+1 twin row). Head: no rows; the pass is gone.
`function t(ary: string[]): void { let part = ary[0]; return part; }` → same: main has `part@L2 -> L3 exact`, head has nothing.
Control `function k(a: string, b: number)` → head equals main.

**Mechanism (reproduced with a parse probe):**
- `(a: string, b: string)` parses as `(formal_parameters (ERROR (identifier)) (identifier) (ERROR (identifier)) (identifier))`. The real names sit inside ERROR nodes; the two `string` tokens become the formals. `js_ts_parameter_early_error` (`src/ast_callback_identity.rs:164-173`) sees duplicate BoundNames, and `simple` is false because of the ERROR children, so duplicates are refused even in sloppy code.
- `(ary: string[])` parses with `(subscript_expression … (MISSING identifier))`; the R7 invalid-BindingElement clause (`:133`, `:347-368`) fires.
- `dfg_owner_name` (`:29-32`) then returns `None` for the **named** function, so every row of that function is lost, locals included.

This contradicts the R7 owner rule that the refusal needs *positive proof*: a recovered parameter list is treated as proof.

**Measured extent (R7 `final/` tables, re-derived by me):**

| Root | File(s) | LOST rows in refused functions | of which oracle-CORRECT |
|---|---|---:|---:|
| `prototype-pollution/aurelia-path_1.1.7` | `src/index.js` — **no `@flow` pragma** (Babel flow-strip-types source) | 97 | **87** |
| `redos/react-native_0.63.0-rc.0` | 20 files; `Libraries/Blob/URL.js` has **no `@flow` pragma** (53 of its 72 rows oracle-CORRECT) | 917 | **508** |

Fresh reproduction on aurelia-path: head LOST 97 rows vs main, all in `relativeToFile` (7), `join` (67), `trimDots` (23). Example: `normalizedBaseParts` local Def → Use inside `relativeToFile`, Exact on main, absent on head.

**Complete firing population (my census, independent of the certificate):** I ran a parse-only probe over 585 roots (X, T, all 583 SecBench roots including the 7 excluded; 226,721 callables). The early-error refusal fires on **75 callables, all in those two roots**. 35 have ERROR/MISSING inside the parameter list (real callables; the named ones among them are the functions that lose rows above). 40 are parser artefacts in `@flow` files (38 "methods" named `if`, 2 named `Error`). **It fires on zero callables in valid JavaScript or TypeScript anywhere in the corpus.**

**Fix (recommended), with measured yield:** in `js_ts_parameter_early_error`, return `false` when the parameter node `has_error()`. I built this as a scratch what-if (5 lines, not product code, deleted with `target/`):

| Root | head LOST | what-if LOST | oracle-CORRECT overrides restored | new rows not in main |
|---|---:|---:|---:|---:|
| aurelia-path 1.1.7 | 97 | 10 | 87 / 87 | 0 |
| react-native 0.63.0-rc.0 | 35,257 | 34,587 | 502 / 508 | 2 (unadjudicated, Flow file) |

The 10 rows still lost on aurelia are exactly the 10 the oracle calls WRONG. The what-if is not adjudicated and no test suite was run on it; it shows yield, not correctness. A real fix needs a regression test (the three fixtures above), the two roots re-tabled, and the matrix re-run.
**Alternatives:** (a) owner re-affirms E13 with the corrected extent and ships as is; (b) drop the duplicate/invalid-target clauses for named owners only (named passes then equal main by construction); (c) remove the early-error refusal entirely — on measured data it has no true positive — but that reopens the R4 matrix cells and is a larger change.

**Self-critique — when it would not apply:** both files contain Flow-style type annotations, so if E13's "Flow-annotated" means "contains Flow syntax" rather than "carries the `@flow` pragma", these rows are inside the accepted class and this is a disclosure defect only (see F3). The owner said they do not use Flow. I still rate it MATERIAL: the brief asks for this class outside `@flow` files, the LOST direction is ~100× the 6 rows the owner was shown, and the cause is PR-B's own predicate, not the misparse artefacts E13 describes ("a type name treated as a variable Def").

**Classification:** CLOSED INSTANCE of the R7 family (early-error refusal without positive proof), overlapping E13.

---

### F3 — WRONG / MATERIAL — the certificate's "SyntaxError" class is not evidence of an early error, and E13's extent is understated

**State → incorrect result (static, from `repair-r6/certificate.py`, `repair-r7/final/*/capture/syntax-exceptions.json`, `final/*/tables/overrides.json`):**

- The class test is `new Function('return (' + source + ')')` and `'return ({' + source + '})'` both throwing SyntaxError. Any type-annotated callable fails that, as does any valid callable that needs context (`static m(){}`, a constructor calling `super()`, `#private` methods). The test cannot tell "ES early error" from "Node cannot parse this text".
- **All 765** certificate SyntaxError rows (87 aurelia-path, 678 react-native) and **all 1,014** table overrides (97 + 917) carry Node messages of that second kind: `Unexpected token ':'` (608 + 90 + …), `Unexpected identifier 'join'`, `Unexpected strict mode reserved word` (a `static` method), `Unexpected token 'if'`. None is "Duplicate parameter name", "Illegal 'use strict' directive", or any other early-error message.
- The overrides relabel **595 oracle-CORRECT LOST rows to WRONG** (87 + 508). React-native's raw table is LOST CORRECT 514; the reported 6 is what remains after the override.
- The "census" is not independent: `repair-r5/seam_census.rs` opens with "Static certificate census using the product predicates" and calls `js_ts_parameter_refusals` and `dfg_owner_name`. So "every differing row is in a class" means "head differs from `f8c768b3` only where head's own predicates fire". That is a sound containment check; it is not a check that the predicates are right, and it passes trivially on predicate false negatives (F5, F6).

**Consequences for E13 as written in SPEC §8:** "one admitted SecBench root", "82 ADDED WRONG … and 6 LOST", "all 22 files involved carry the `@flow` pragma", "the other 578 admitted SecBench roots have none". The 88 STOP rows are indeed confined to 22 `@flow` files (I checked every file). But the type-annotation class also covers aurelia-path (second root, no pragma) and 595 further oracle-CORRECT LOST rows that the override removed before the count.

**Fix (recommended):** correct SPEC §8 E13 and the R7 report: state the raw LOST CORRECT figures (514 + 87), name aurelia-path, define the boundary as "files containing type-annotation syntax in `.js`" (the pragma does not work as a boundary), and rename the class to what it measures. Tighten the override to require a recognised early-error message. If F2's guard lands, the class should become empty on the corpus — that is the cleanest proof.
**Alternatives:** keep the override but report its rows separately as "type-annotated source, oracle-CORRECT, not counted".

**Self-critique — when it would not apply:** REPORT.md does say "917 SyntaxError overrides were applied before this population was counted", and the raw partitions are preserved in `final/`, so nothing was concealed; the summary numbers the owner decided on are what is misleading. The containment half of the certificate (named SEAM bindings equal main; no synthetic row with a refused formal; no non-JS delta) is unaffected and I found nothing against it.

**Classification:** NEW FAMILY (evidence admissibility: an override proof that does not discriminate its claimed mechanism). It is an evidence family, not a product family.

---

### F5 — WRONG / IMMATERIAL — TypeScript wrappers around `eval` defeat the EVAL refusal

**Input → incorrect result (reproduced, `.ts`):** `reg(function (f: string) {\n (eval as any)("f = 5");\n use(f);\n});` → head emits `f (param) -> use(f)  exact`. Same for `eval!("f = 5")` and `(<any>eval)("f = 5")`. Plain `eval("f = 5")` is refused as designed. TS 5.9.3 `transpileModule` emits a direct `eval("f = 5")` for all three (and for `satisfies`), and Node returns 5.
**Mechanism (static):** `js_ts_direct_eval_anywhere` (`:507-519`) peels only `parenthesized_expression`.
**Fix:** also peel `as_expression`, `satisfies_expression`, `non_null_expression`, `type_assertion` (the same five wrappers the read classifier already climbs). Alternative: disclose.
**Self-critique:** the usual TS idiom is `(0, eval)(…)`, which is indirect and correctly admitted; I know of no measured instance. The binding is still the formal's, so under "Exact = static binding" this is wrong only by PR-B's own refusal rule.
**Classification:** CLOSED INSTANCE of the EVAL predicate boundary.

### F6 — WRONG / IMMATERIAL — a labelled body-level function declaration defeats SEAM

**Input → incorrect result (reproduced):** `reg(function (y, d = 0) {\n lbl: function y() {}\n use(y);\n});` → head emits `y (param) -> use(y)  exact`. Node: `(function(y, d = () => y){ lbl: function y(){}; return [typeof y, typeof d()] })(1)` → `['function','number']`: two environments; `use(y)` reads the body binding.
**Mechanism (static):** `js_ts_seam_binding` (`:484-491`) tests only direct `function_declaration` children of the body; `js_ts_block_statement_binds` (`:666-685`) has no `labeled_statement` arm.
**Fix:** unwrap `labeled_statement` chains in both places. Alternative: disclose.
**Self-critique:** labelled function declarations are sloppy-mode Annex B syntax that essentially does not occur; the named control equals main.
**Classification:** CLOSED INSTANCE of the SEAM predicate boundary.

### F7 — WRONG / IMMATERIAL — sloppy `async function` / `function*` with duplicate simple parameters is refused as an early error

**Input → incorrect result (reproduced):** `async function q(a, a) { use(a); }` and `function* g(a, a) {\n use(a);\n}` → main emits the formal→use rows; head emits none. Node accepts both (`new Function` compile); only methods and arrows reject duplicates in sloppy code.
**Mechanism (static):** `permits_duplicates` (`:164-167`) requires kind `function_declaration | function_expression` and no `async` child.
**Fix:** allow `generator_function_declaration` and drop the `async` exclusion (duplicates are illegal only for arrows, methods, strict code and non-simple lists). Alternative: disclose.
**Self-critique:** zero firings on valid code in 226,721 callables (F2 census).
**Classification:** CLOSED INSTANCE of the R7 early-error family.

### F8 — WRONG / IMMATERIAL — a TS enum member shadows a formal inside the enum body; no fence

**Input → incorrect result (reproduced, `.ts`):** `reg(function (y: number) {\n enum E { y = 1, z = y }\n use(y);\n});` → head emits `y (param) -> y` at the `y` in `z = y`, NameOnly(CfgIncomplete). TS emit: `E["z"] = 1` — the initializer reads member `E.y`, not the formal (run with formal = 7 → `[1, 7]`).
**Mechanism (static):** `js_ts_scope_binds` (`:622-663`) has no enum-body scope. Named passes have the same row on main (zero-width).
**Fix:** treat `enum_body` as a scope binding its member names. Alternative: disclose.
**Self-critique:** a local enum inside a callback whose member name equals a formal and is read by a sibling initializer; NameOnly.
**Classification:** CLOSED INSTANCE of the E3 fence (missing binder kind).

---

## SMELL findings

### F4 — SMELL / MATERIAL (to dispatch, priority 6) — `IMPLEMENTOR-prB.md` cannot be used to land `2431cb10`

**Evidence (static):** line 1 says "No completion or adoption is claimed"; line 4 says "STOP; PR-B PARKED"; the only dispatch text is the "Historical R2" one — source `cb996630` plus `R2-src.patch`, cache version 110, gates "LOST CORRECT and ADDED WRONG must each be zero". It never mentions `2431cb10`, R7, E13, cache 112 or the "outside Flow-annotated files" acceptance rule. `SPEC-prB.md`'s status line is the R6 one ("certificate … in progress; no acceptance"), and B-D8 says `CACHE_VERSION 107 → 108 (one transition)` while the code and its pin are at 112 with five history lines.
**Fix:** replace the header with a current dispatch: source `2431cb10`, comparison base, the acceptance rule as amended by E13, the cache version to land (108 or 112 — pick one and re-pin PD-11 / P2-M11 to match), the gate list from R7, and "everything below is historical". Update the SPEC status line. Alternative: add a short `LANDING.md` and mark both files historical.
**Self-critique:** if the controller lands the branch by merging PR #351 as it stands and nobody reads the dispatch, this changes nothing in the product.
**Classification:** CLOSED INSTANCE (stale-authority docs; same kind as the S-d / S2 custody-pin findings in R2).

### F9 — SMELL / IMMATERIAL — type-predicate parameter names are admitted as synthetic Uses

**Evidence (reproduced):** `reg((v: unknown):\n v is string => {\n check(v);\n});` → row to the `v` in `v is string`; same for `asserts v is string`. `typeof v` in a return type is correctly erased. On X: 7 synthetic rows end on a predicate name (e.g. `.filter((i): i is Point => !!i)`), all NameOnly; in `App.tsx:5141` the predicate line has no value read. Binding is correct and the oracle scores them CORRECT.
**Mechanism (static):** `is_erased_type_boundary` (`src/languages/mod.rs:663-670`) lists `type_annotation` but not the predicate/asserts annotation kinds. B-D12 says synthetic Uses must read values.
**Fix:** add the type-predicate and asserts annotation kinds for the synthetic read filter only (the comment at `ast_callback_identity.rs:592-593` wants legacy rows preserved). Alternative: leave and disclose.
**Self-critique:** no false binding, no Exact; purely a read-role imprecision.
**Classification:** CLOSED INSTANCE of B-D12 / E4 (read role).

### F10 — SMELL / IMMATERIAL — `RdResult.reaching_edges` is dead

**Evidence (static):** `src/cpg/reaching.rs:71-73, 244-256, 311`. The doc comment says "Used only to admit implicit parameter-to-body entry copies"; the copy model is deleted and nothing reads the field (4 occurrences, all in that file). It is still computed for every DFG edge of every pass in every language. The rest of the copy machinery (`implicit_entry`, `parameter_reaching_sources`, `copy_edges`, …) is gone — I grepped.
**Fix:** delete the field and its loop. Alternative: leave; output is unaffected.
**Self-critique:** cost is whatever the perf gate already measured.
**Classification:** CLOSED INSTANCE (option-(b) deletion completeness, Fable R4 §4.1).

### F11 — SMELL / IMMATERIAL — synthetic alias twins reach earlier lines

**Evidence (reproduced):** `http.createServer(function (req, res) {\n exec(req);\n var x = req;\n res.end(x);\n});` → `<cb@1:19>|req@L3[x] -> req@L2  nameonly/cfg_incomplete`: the twin created on line 3 reaches the read on line 2 in straight-line code. The named control has the same row on main, and `(f, g, d = g) { var f = g; … }` sends a body twin to the default expression. Binding-correct by the oracle's twin rule; never Exact.
**Fix:** drop twin references that start before the alias lvalue unless a loop encloses both. Alternative: leave as E11 parity.
**Self-critique:** SPEC promises only that E4/E7 do not grow in new passes; E11 is explicitly "not generally repaired".
**Classification:** CLOSED INSTANCE of E11 (flow-insensitive alias twins).

---

## What held under attack (reproduced unless marked)

- **SEAM:** named owner equals main byte-for-byte on six shapes (plain, arrow with closure default, method with destructured default + body function, non-seam sibling formal, class method with `for (var f of d)`); the only named differences were fence removals of rows into nested binders. Synthetic owner: formal refused; `var f = 2 → use(f)` kept and scoped to the body; `var f;` alone yields nothing; body-level `function f`, nested-block `var`, `for (var f of …)`, `[d = 1]` and `{[k]: d}` all trigger; `(f, {d})` correctly does not.
- **EVAL:** `eval(…)`, `(eval)(…)`, eval inside a nested arrow → refused. `(0, eval)(…)`, `o.eval(…)` → admitted.
- **ARGS:** sloppy `arguments[0] = 9`, `arguments` in a nested arrow, `var a = arguments` → refused. Strict body, `arguments` only in a nested non-arrow function → admitted.
- **Fence, both directions:** outer Def into `catch (y)`, block `let`, `for (let …)`, `for (const … of)`, class-expression and function-expression self names, `switch` lexicals, destructured catch → all fenced. Inner `var y` no longer reaches the outer `use(y)` (main's Exact row removed). Class static block, getters/setters, generator methods and anonymous `function*` behave as boundaries.
- **Kill-only nested writes:** nested `t = 2` and `a = 9` produce `nameonly/killed` on the outer rows and no emitted Def; a nested rebinding `function (t) { t = 5 }` is not treated as the same binding.
- **`this`:** rows into a nested `function`, an object method and a class field are refused; rows into a nested arrow are kept as CfgIncomplete; an arrow owner works.
- **R7 kinds:** strict UMD `function (window, undefined) { "use strict"; … }` is admitted; `(a, b = 1) { "use strict" }` and `(a, a) =>` are refused; rest, destructured sibling, TS `this` parameter, optional parameter, async function/arrow all behave.
- **Identity containment:** `dfg_owner_name` has exactly two callers (`data_flow.rs:558, 564`); the Step-5b callee match (`cpg/build.rs:232`) compares against `FunctionId` names, which cannot start with `<`. On a fixture repo, `callers`, `callees`, `symbol-spans`, `functions`, `nodes-at`, `ego`, `call-stats`, `repo-map` were byte-identical between head and main and contained no `<cb@`.
- **E13 boundary, ADDED direction:** the 88 STOP rows are in 22 files, each with `@flow`. I found no type-syntax-as-arrow misread in the TS grammar (function types, `asserts`, `accessor`, `<const T>`, abstract signatures, `declare function`) or in plain JS (JSDoc, regex, template literals). The oracle's `synthetic_owner_scope` rule would flag such rows as WRONG anywhere, and it reports none outside react-native.
- **UNDECIDED rows:** X 2, SecBench 25 LOST (cejs 16, react-native 7, lodash 1, mithril 1) + 1 RELABELLED — counts match the report. Sampled 8: all NameOnly, collapsed or alias-twin rows in named owners. The two on X are a twin from an inner `(event) =>` to a read of the outer `event` — correctly removed.

## Probe log (admissibility)

- My first navigation comparison passed `--file/--line` to `nodes-at`; both binaries returned the same usage error. Inadmissible; re-run with `--location`.
- JSX scan on X returned 0; before believing it I ran the scanner on the fixture (4 hits).
- Early-error census: 585 probe runs, 0 failures, 29 roots with no JS/TS callables.

## What I did not check

- **F** — never opened. Everything about F is the controller's aggregate.
- Gates: nextest, mutants, clippy parity, fmt, Tier-A matrix/quick, perf, O1, MCP — none re-run.
- Byte tables: not re-run or re-adjudicated. I dumped head on X (no main diff), and head/main on aurelia-path and react-native only. T and the other 581 SecBench roots: parse-only census.
- The 763 + 200 matrix cells and their Node proofs; `MATRIX-param-env-r7.md` (12k lines) not read.
- The 7 excluded roots' DataFlow behaviour (parsed only).
- `rowdiff.py`, `oracle-environments.cjs`, `oracle-read-role.cjs`, and the retained-capture / reuse logic in `r7_final_batch.py` beyond a read-through.
- Navigation identity on X/T (one fixture repo only); non-JS identity; call-site identity.
- `with` bodies, alias-twin lvalue identity (B-D11) beyond one fixture, RD label logic in `reaching.rs` / `scope.rs` beyond the diff, the cache-version mutant coupling, and the test file / goldens beyond grepping for coverage.
- The F2 what-if guard: yield measured on two roots only; its 2 new react-native rows are unadjudicated; no tests run on it.
