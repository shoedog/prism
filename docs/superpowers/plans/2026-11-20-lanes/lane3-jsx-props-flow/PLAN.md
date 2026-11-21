# Lane 3: React value flow across JSX props — implementation plan

> **For agentic workers:** this plan is run by an orchestrator under `../ORCHESTRATION.md`. Each task is
> dispatched to an implementer (Sonnet-5.5, high) with its section as the brief. Steps use checkbox syntax.

**Goal:** a value passed as a JSX attribute reaches the component's use of that prop in prism's data-flow
graph, for Exact-resolved components with destructured or plain props, proven by a type-checker oracle,
end-to-end path counts and rated review examples.

**Architecture:** destructured formals get definitions from the data-flow producer through a new accessor
that leaves parameter occurrences and positional slots untouched; the #594 object-literal binder is split
into props sources (object literal, JSX attributes) and props targets (plain formal reads, pattern-formal
definitions), keeping its generic shape; JSX sites enter the argument-binding step through one new branch,
take their source from the line-keyed Use node their caller's def-use edges end at, and are admitted only
for a direct component, a React `memo`/`forwardRef` wrapper, and a namespace-import member tag.

**Tech stack:** Rust (core crate: `src/ast`, `src/data_flow`, `src/cpg`), tree-sitter
JavaScript/TypeScript/TSX; Node with the pinned TypeScript 5.9.3 for the census and oracle; Python 3 for the
comparators.

**Spec:** `DESIGN.md` in this directory (sections 5.1 to 5.4 are the rules; section 7 the gates).

## Global constraints

- Worktree `~/code/prism-jsx-props`; evidence `~/prism-evidence/jsx-props/`.
- Corpora: X = Excalidraw at `0642e72c` (`~/prism-evidence/inputs/excalidraw-0642e72c/source`, no git
  history); T = `~/code/bench-repos/TypeScript/src` (regression only); G = `~/code/bench-repos/gin` (control:
  a non-JavaScript corpus must show no change); R2 = the second React corpus chosen in Task M-2 (full
  history); `private-corpus` locally under ORCHESTRATION section 9.
- TypeScript for scripts: `~/prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js`.
- Edges only for a site that passes the target admission of DESIGN 5.3.
- **Not changed by this lane:** `function_parameter_occurrences`, `function_parameter_names`,
  `function_parameter_slots`, `function_parameter_slot_occurrences`, the slot builders in
  `src/parameter_slots.rs`, `compute_param_def_nodes` and its return type, the reasoning layer's seeds, any
  taint code, call resolution, `src/cpg/trace.rs` (PR #841 is editing it).
- `src/cpg/build.rs` is at 1,777 weighted lines: add module lines, calls and one field there; put logic in
  new modules.
- New `src/cpg/` modules that use the build's private items are declared in `src/cpg/build.rs` with
  `#[path = "…"]`, as `callee_params` and `object_literal_args` are. New test files get a
  `#[cfg(test)] mod` line in `src/cpg.rs`. New `src/ast/js_ts/` files are declared in `src/ast/js_ts.rs`.
- Every change to the argument-binding step is made at the same position in production
  (`step5b_edges_for_caller_with_exact` in `src/cpg/build.rs`), `src/cpg/callee_legacy.rs` and
  `src/cpg/step5b_reference.rs`. Emission order must be the same in all three: parity tests compare edge
  lists, and a parallel-equals-serial assertion compares order.
- Each slice that changes the graph or the statistics record bumps `CACHE_VERSION` in `src/cpg_cache.rs`
  with a list line, and updates the React and data-flow entries of
  `docs/features/language-coverage/js-ts-known-gaps.md` and the matching paragraph of `CLAUDE.md`.
- Tier-A applies to every slice (ORCHESTRATION section 5 item 5).
- Core tests must not name the review crate (the layering test forbids it). Assertions about review-taint
  findings live in the repository's `tests/algo/taxonomy/` directory.
- `private-corpus` runs (census, measurements, review examples) use Claude seats only, locally; outputs stay
  in the private evidence file; public text carries counts only (ORCHESTRATION section 9).

## Review focus

1. A pattern local with the same name as an outer variable: the body's use is fed by the pattern Def and
   by nothing from the outer scope (already true of reference search; pinned so it stays true). (S1)
2. A pattern local reassigned in the body, and one captured by a nested callable: labels follow the rules
   that hold for identifier formals; no `exact` edge survives a write that kills it. (S1)
3. An attribute whose name equals a local variable's name but whose value is another variable
   (`const title = a; <D title={b} />`): the edge starts at the line-keyed Use of `b`, never at a node of
   `title`. (S3)
4. An element spanning several lines with the attribute on a later line than the tag: the edge exists and
   the trace engine crosses it. (S3)
5. A cold build, a full cache hit and a partial cache hit give the same props edges and the same
   `dfg_props` counters, at `RAYON_NUM_THREADS` 1 and 4. (S2, S3)
6. Every bound JSX edge is reached from the value's definition: `title={title}`, guarded elements, one value
   in two attributes, `key` beside a bound attribute (rows j25 to j28, `path` assertions). (S3)
7. Admission: a member tag through an imported value and a component behind a non-React wrapper bind
   nothing and are counted (rows j34, j36). (S3)

---

## Stage M: census and tooling (2.5 days)

### Task M-1: workspace, binaries, issue

- [ ] Worktree and evidence directory (ORCHESTRATION section 4). Build main once:
      `~/prism-evidence/tools/memguard.sh 8 cargo build --release`; copy the binary to
      `~/prism-evidence/jsx-props/bin/prism-base`; record its `--version`.
- [ ] Comment on #596 that the lane has started, linking this directory.

### Task M-2: census

**Files (evidence directory):** `census/census_jsx.cjs`, `census/CENSUS.md`.

- [ ] Write `census_jsx.cjs` after the model `~/prism-evidence/js-args/census.cjs`: read
      `prism-base nav --no-cache call-stats --repo <corpus> --dump-sites`, and for every record whose byte
      span is a JSX opening or self-closing element in the TypeScript syntax tree (the dump has no JSX flag),
      locate the element and, when resolved, the target function by byte span. Emit one JSON line per
      attribute with: site file and line; the site's caller (name and start line, from the dump); resolution
      outcome (exact singleton, name-only, several targets, unresolved with the drop reason); for a member
      tag, the qualifier's kind (namespace import, named import, default import, local, other); the target's
      route by DESIGN 5.3 check 3 (direct, admitted React wrapper, other wrapper); whether the tag's spelling
      differs from the target function's name; attribute name; value shape (identifier, member path, object
      literal, string, bare, call, conditional, template, function, element, other); whether a spread
      precedes or follows it; the element's statement context and owner kind (the cells of Task T-3); the
      target's first-formal shape (object pattern: flat, alias, default inert, default other, default
      another formal or an outer variable, intermediate default, rest, nested, computed, array inside;
      identifier: read by member only, bare read with its own bytes, bare read in a declarator initialiser,
      condition, `for-in` head or `in` test, destructured in the body; none); whether the target is a named
      function, a const-bound arrow, or an anonymous function inside an admitted wrapper; whether it is a
      one-line function with two or more props; and whether the pattern or the member reads contain the
      attribute's key.
- [ ] Source outcome per Leaf value, by the DESIGN 5.3 rule: with a warmed nav cache for the corpus (one
      `prism-base nav call-stats` run without `--no-cache` first), run `prism-base nav nodes-at --repo
      <corpus> --location <file>:<line> --format json` once per distinct value line and look for a Use node
      of the value's path whose function is the site's caller (by name, and by start line where the output
      gives it) (`connected` when a `to`
      endpoint of `PRISM_EDGES_BYTES=1 prism-base nav dfg-stats --edges` has that node's file, line, path,
      access and bytes; otherwise `unconnected`); failing that, a Use node of that path in a function whose
      span encloses the caller (`in_enclosing_owner`); otherwise `not_found`.
- [ ] A **predicted connected bindable pair** is one (attribute or object-literal leaf, target) such that:
      the site is an exact singleton; a member tag's qualifier is a namespace import; the route is direct
      or an admitted React wrapper; the value is an identifier or member path (each leaf of an
      object-literal value counts once); its source outcome is `connected`; no spread follows the
      attribute; and the target's first formal is a pattern (not alias-skipped) with an entry whose key path
      is comparable with the attribute's, or an identifier formal with a member read of that key or a bare
      read with its own bytes.
- [ ] Run on X and on the three candidates for R2 (`calcom-cal.com`, `apache-superset`, `apache-answer`).
      R2 is the candidate with the most predicted connected bindable pairs (ties: the alphabetically
      first). Run on `private-corpus` with `--aggregate-only`.
- [ ] `CENSUS.md`: the tables per corpus, the predicted connected bindable pairs, the census-go rate of
      DESIGN section 7, the source-outcome table by statement context and owner kind, and the numbers
      DESIGN section 9 names for F1 to F5, F7 and F9.

**Exit gate (census go):** DESIGN section 7 "Census go" (≥ 1,000 predicted connected bindable pairs on X
plus R2, and a rate ≥ 30% on each). Below it: stop, post the tables on #596, and wait for the owner.

### Task M-3: comparator, oracles, paths

**Files (evidence directory):** `tools/compare_props.py`, `tools/oracle_props_edges.cjs`,
`tools/oracle_pattern_defs.cjs`, `tools/undecided_sample.py`, `tools/paths.py`, `tools/measure.sh`.

- [ ] `measure.sh <binary> <corpus> <outdir>`: runs, through memguard, `nav --no-cache dfg-stats`,
      `nav --no-cache call-stats`, and `PRISM_EDGES_BYTES=1 nav --no-cache dfg-stats --edges`.
- [ ] `compare_props.py <base dir> <head dir>`: keys each edge by (file, path, line, bytes) at both ends;
      classes: `pattern-def-intra` (a new edge that starts at a Def absent from the base, inside one
      function), `props-literal` (a new cross-function edge whose source is in the caller of an
      object-literal argument site), `props-jsx` (the same for a JSX element site), `removed` and
      `label-move` (each listed in full: none is expected); prints every counter delta of both stats files;
      exits 1 on an unclassified change. Model: `~/prism-evidence/jsts-lane/783/scripts/compare761.py`.
- [ ] `oracle_props_edges.cjs <corpus> <added-edges.jsonl>`: verdicts per DESIGN section 6 "Oracle",
      checks 1 to 3 (component identity with aliases and the one-step follow; key match; attribution from
      the TypeScript tree). It loads the corpus's own files with no dependency installation. Self-test it
      before use on a fixture with one correct edge and one edge for each WRONG kind: wrong component, wrong
      key, an attribute overwritten by a later spread, the first of two duplicate attributes, a `key`
      attribute, a `children` attribute on an element with child content, a source that belongs to another
      attribute, a member tag through a named import, plus one correct expando component (`Tabs.Tab = X`
      through a namespace import).
- [ ] `oracle_pattern_defs.cjs <corpus> <added-edges.jsonl>`: the slice S1 binding-identity oracle of
      DESIGN section 6, and a list of added `exact` edges whose local has a write reference in its function.
      Self-test on a fixture with one correct edge and one edge from a shadowed outer binding.
- [ ] `undecided_sample.py <verdicts> <n> <seed>`: the seeded sample of UNDECIDED rows for the hand read.
- [ ] `paths.py <head edges> <oracle verdicts> <corpus>`: the end-to-end path table and the connected share
      of DESIGN section 6 (the value's root binding decided with the checker).

---

## Stage T and E: test matrix (3.5 to 4 days)

### Task T-1: matrix data and one data-driven test

**Files:**
- Create: `tests/fixtures/props_matrix/rows.jsonl`, `tests/fixtures/props_matrix/README.md`
- Create: `src/cpg/props_matrix_tests.rs` (declared with `#[cfg(test)] mod` in `src/cpg.rs`)

Each row: `id`, `group` (`control`, `S1`, `S2`, `S3`), `langs` (which of `js`, `ts`, `tsx`, `jsx`), `files`
(name → source text), `expect` (a list of assertions), `class`, `truth`. Assertion kinds, all evaluable on
main with the helpers `src/cpg/object_literal_arg_tests.rs` already has (`build`, node spelling
`function:path@line`) plus a traversal of the built graph's data-flow edges: `def`, `no_def`, `edge`
(optionally with a label), `no_edge`, `no_edge_into`, `path` and `no_path` (a chain of data-flow edges from
one named node to another, both spelled `function:path@line` with `(def)` or `(use)`), `counter` (a key of
the `dfg_props` map read from the serialised statistics; an absent key reads as 0), `trace_reaches`,
`trace_misses` and `trace_boundary` (through the trace engine, as the existing plain-call trace tests do;
each names its seed line and its target line).

**Binding rows assert reachability from the definition.** Every S3 row that expects a binding also asserts
a `path` from the value's definition (a local's Def, or the caller's own formal Def) to the edge's target,
and, when the target is a pattern formal, a `trace_reaches` seeded at that definition's line. An `edge`
assertion alone cannot see a source node that the caller's definitions do not reach (DESIGN 5.3).

`props_matrix_tests.rs` holds one test, `props_matrix_rows`, with a constant
`ENABLED: &[&str] = &["control"]`. It runs the rows of enabled groups and skips the rest. On main the
control rows pass. For every other row the implementer records, once, that it fails when its group is
enabled by hand (the RED record, in the ledger). A seeded row whose stated premise about main proves false in
the RED run (for example that a read has its own bytes on main) is listed in the RED record with the
observation and goes to stage D unchanged.

Rows asserting `exact` on an edge inside a function use bodies of at least two lines; a one-line owner is
labelled by the single-node rules. Rows that use `memo` or `forwardRef` import it from `"react"` (named, or
as a member of the default or namespace import) unless the row says otherwise, because DESIGN 5.3 check 3
admits only React's wrappers.

**Group S1 (pattern formals):**

| Row | Callee | Required |
|---|---|---|
| p01 | `function C({ html }) {` / `return use(html);` / `}` | Def `C:html@1` with the identifier's bytes; edge `C:html@1 → C:html@2` |
| p02 | `{ html: h }` | Def `h`; no Def `html` |
| p03 | `{ html = "" }` | Def `html` |
| p04 | `{ html = make() }` | no Def for `html` |
| p05 | `{ a, ...rest }` | Defs `a` and `rest` |
| p06 | `{ style: { color } }` | Def `color` |
| p07 | `{ list: [first] }` | no Def for `first` |
| p08 | `{ [k]: v }` | Def `v` |
| p09 | `{ a, b }` with only `a` read | Def `a` only |
| p10 | `({ user }) => {` / `return user.name;` / `}` | base Def `user` and a projection edge to `user.name@2` |
| p11 | `const C = ({ html }) => { … }`; `const C: React.FC<P> = ({ html }) => { … }`; `function C({ html }: P) { … }` | as p01 |
| p12 | `({ a } = {})` | Def `a` |
| p13 | `({ a } = other)` with `other` an earlier formal | exactly the Defs main has today (the parameter-default alias code); no second Def at the same bytes |
| p14 | outer `const title = 1;` then inner `({ title }) => { return use(title); }` | no edge from the outer Def into the inner use; an edge from the pattern Def |
| p15 | `({ a }) => { a = 1; return use(a); }` | the edge from the pattern Def to `use(a)` is not `exact` |
| p16 | pattern local read inside a nested arrow | the label an identifier formal gets in that position |
| p17 | the function uses `arguments`; the function calls `eval` | the refusals an identifier formal gets in that function |
| p18 | method, class method, object method, async function, generator | as p01 |
| p19 | `function f(a, { b }, c) { … }` | Defs `a` and `b`; `a` and `c` exactly as on main |
| p20 | `(x = 1, { b }) => { … }` | `b` gets a Def; `x` exactly as on main |
| p21 | one-line owner `const C = ({ a, b }) => use(a, b);` | Defs `a` and `b`; edges present, label not asserted |
| p22 | `function C({ html }) {` / `function html() {}` / `return use(html);` / `}` (a hoisted function replaces the formal) | the edge from `C:html@1` into `use(html)` is `killed`, as the identifier twin `function C(html)` gets |
| p23 | `function C({ a }, b = (a = 1)) {` / `return use(a);` / `}` | the edge from `C:a@1` is labelled as the identifier twin `(a, b = (a = 1))` gets (`killed`) |
| p24 | `function C({ o }, x = (o.k = 1)) {` / `return use(o.k);` / `}` | the edge from `C:o@1` to `o.k` gets what the identifier twin `(o, x = (o.k = 1))` gets (the own-parameter member-write refusal) |
| p25 | `function C({ a = 1 }) {` / `var a;` / `return use(a);` / `}`; the same as an anonymous function passed to a call | as the identifier twin `(a = 1)` with `var a`, named and anonymous |
| p26 | `function L({ id }) {` / `return xs.map(({ id }) => {` / `return use(id);` / `});` / `}` | no edge from `L:id@1` into the inner use; the inner arrow's pattern Def feeds it |
| p27 | `function C({ a }) {` / `run(() => { a = 2; });` / `return use(a);` / `}` | the edge from `C:a@1` to `use(a)` is labelled as the identifier twin gets (`cfg_incomplete`, the captured-write rule) |
| p28 | `{ a: { c } = make() }`; `{ a: { c } = other }`; `{ a: { c } = {} }` | no Def for `c`; no Def for `c`; Def `c` |
| p29 | `function f({ a } = outer) {` / `return use(a);` / `}` with `outer` an outer variable, not a formal | exactly the Defs main has today (the alias code's `a` and its twin); no second Def at the same bytes |
| p30 | `({ a } = make())` | no Def for `a` |
| p31 | `function C({` / `a,` / `b = a,` / `}) {` / `return use(a, b);` / `}` | Def `C:a@1` with the bytes on line 2; no Def for `b`; the read of `a` on line 3 gives what it gives for the identifier twin `function C(` / `a,` / `b = a,` / `) {…}` |
| p32 | `const html = ({ html }) => {` / `return use(html);` / `};` | Def of the pattern local `html` in the arrow owner named `html`, and its edge into `use(html)` |
| p33 | `const One = ({ title }) => <D title={title} />;` | Def `One:title@1`; outgoing edges not asserted here (row j31) |
| p34 | `function C({ a }: P = {} as P) {` / `return use(a);` / `}` | Def `a` (assertion wrappers around the default are removed before the inert test) |

**Group control:** the p01, p05 and p19 sources rewritten as Python and Go functions (no Def changes); a call
`f({ a: x }, r)` into `const f = ({ a }, ref) => { … }` has no edge into `ref` (positional slots still stop
at a pattern); `D({ other: t })`-style rows that already hold.

**Group S2 (object-literal argument into a pattern formal):**

| Row | Shape | Required |
|---|---|---|
| l01 | `D({ html: t })` into `function D({ html }) { … }` | `A:t(use) → D:html@1(def)`, exact |
| l02 | shorthand `D({ html })` | the same, from the shorthand's value |
| l03 | `D({ other: t })` | none; counter `refused_key_not_read` = 1 |
| l04 | pattern `{ html: h }` | into Def `h` |
| l05 | pattern `{ a, ...rest }`, argument `{ a: x, b: y }` | `x → a`, `y → rest`; counters `edges_to_def` = 1, `edges_to_rest` = 1 |
| l06 | literal `{ style: { color: c } }` with pattern `{ style: { color } }`; literal `{ style: s }` with the same pattern; literal `{ style: { color: c } }` with pattern `{ style }` | `c → color`; `s → color`; `c → style` |
| l07 | `D({ ...o, html: t })`; `D({ html: t, ...o })` | binds; none, counter `refused_overwritten` = 1 |
| l08 | duplicate key | only the last value binds |
| l09 | computed key in the literal | keys before it refused, keys after it bind |
| l10 | callee resolves name-only, or to two targets | none (control on main too) |
| l11 | defaulted element `{ html = "" }` | binds |
| l12 | l01 through the trace engine | `trace_reaches` from `t` to the body use in `D` |
| l13 | `f(a, { k: v })` into `function f(x, { k }) { … }` | `v → k`; `a → x` as on main |
| l14 | `D(obj)` (an identifier) into a pattern formal | none (accepted loss; DESIGN F1) |
| l15 | pattern entry with a computed key | receives nothing; counter `refused_key_unknown` = 1 |
| l16 | `P({ a: x })` into `function P(props) {` / `const { a } = props;` / `return use(a);` / `}` | no edge into `P`; counter `refused_zero_width_read` = 1 (the read in the declarator initialiser is zero-width; DESIGN F3) |
| l17 | `P({ a: x })` into `function P(props) {` / `return wrap(props);` / `}` | `x → P:props@2` bare read, as #594 gives today; counter `edges_to_bare_use` = 1 |
| l18 | `P({ a: x })` into `function P(props) {` / `return <Inner {...props} />;` / `}` | `x → P:props@2` bare read; counter `edges_to_bare_use` = 1 |
| l19 | `D({ a: x })` into `function D({ a } = defaults) {` / `return use(a);` / `}` (`defaults` an outer variable) | no edge from `x`; counter `refused_alias_pattern` = 1 |
| l20 | `D({ 'aria-label': l })` into `function D({ 'aria-label': a }) { … }`; the same literal into `function D(p) { … }` reading `p.label` | `l → a`; none, as on main |
| l21 | `import { api } from "./ctx"; api.send({ a: x })` where `ctx` has an unrelated `export function send({ a })`; the same call with `import * as api from "./ctx"` and `export function send({ a })` | none, counter `refused_member_qualifier` = 1; `x → send:a@…(def)` |

**Group S3 (JSX attributes; TSX and JSX only):**

| Row | Shape | Required |
|---|---|---|
| j01 | the issue's example, element and component on separate lines | `App:text(use) → Html:html@1(def)`, exact; `trace_reaches` from `text`'s definition to the `dangerouslySetInnerHTML` line |
| j02 | `<HtmlP html={text} />` into `function HtmlP(props)` reading `props.html` and `props.other` | edge into `HtmlP:props.html(use)`; no edge into `props.other` |
| j03 | arrow component; named import `{ Html }`; default import `Html` of a file whose default export is `function Html`; `<UI.Html>` with `import * as UI` | as j01 (in each form the site's recorded callee name equals the function's name; forms where they differ are j35) |
| j04 | `memo(function M({ html }) {…})`; `forwardRef((props, ref) => …)` with `ref={r}` | binds to the inner function's first formal; nothing into `ref` |
| j05 | `const C = memo(({ html }) => {…})` (anonymous inner function) | binds like j04 (if stage D records it as an accepted loss, the row asserts no edge and counter `refused_no_props_formal` = 1) |
| j06 | `<Html {...p} html={text} />`; `<Html html={text} {...p} />` | binds; none with `refused_overwritten` = 1; `spread_sources_unbound` = 1 each |
| j07 | the same attribute twice | the last binds |
| j08 | `key={k}`, `ref={r}`; a pattern that names `key` | nothing bound |
| j09 | `title="x"`; bare `disabled` | none (control) |
| j10 | value `user.name`; `(text)`; `text as string`; `text!` | bound (member value: the full path and its base, as arguments do) |
| j11 | value is a call, conditional, template, arrow, element | none; `refused_value_not_data` = 1 each |
| j12 | `style={{ color: c }}` into `{ style: { color } }`, `{ style }`, `props.style.color` | `c → color`; `c → style`; `c → props.style.color` |
| j13 | `aria-label={l}` into `{ 'aria-label': a }` | `l → a` |
| j14 | `xlink:href={u}` | none |
| j15 | `const M2 = React.memo(Html)`; class component | none; `refused_not_exact_singleton` = 1 |
| j16 | `React.lazy` loader target (same file); component with no formals | none, `refused_unknown_wrapper` = 1; none, `refused_no_props_formal` = 1 |
| j17 | `<C a={x}>{y}</C>` | `x` binds; nothing from `y` |
| j18 | element over three lines, attribute on line 2 | edge exists; `trace_reaches` |
| j19 | two `<D a={x} />` on one line | the edge `x → D:a` exists (both sites share the line-keyed source and dedupe to one edge); `trace_boundary` (today's limit, pinned) |
| j20 | `<A icon={<B x={v} />} />` | `v → B`'s `x`; nothing into `A`'s `icon` |
| j21 | `const title = a; <D title={b} />` | from the line-keyed Use of `b` only; no edge leaves a node of path `title` |
| j22 | `{items.map(i => <D a={i.x} />)}`; `{cond && <D a={x} />}` | bound; in the first, the source is the use in the arrow passed to `map` (an anonymous owner, spelled as the harness prints it) and the `path` starts at that arrow's formal `i` |
| j23 | `.jsx` file; `<C<T> x={v} />` | as j01 |
| j24 | `<P a={x} />` into `function P(props) {` / `const { a } = props;` / `return use(a);` / `}` | no edge into `P`; counter `refused_zero_width_read` = 1 |
| j25 | attribute named like its value, in a function declaration, a const-bound block arrow and a method: `function Outer({ title }) {` / `return <Inner title={title} />;` / `}`; the same with `const title = get();` in the body; the same with the element over several lines | `path` from the value's definition (`Outer:title@1(def)` or the local's Def) to `Inner:title@1(def)`; `trace_reaches` from that definition's line to `Inner`'s use |
| j26 | in an owner with a branch: `{user && <Avatar user={user} />}`; `cond ? <D a={x} /> : <E a={x} />` under `return` | each binding has a `path` from the value's definition; both `D` and `E` are bound |
| j27 | in an owner with a branch: `<D a={x} b={x} />` | `x → D:a`, `x → D:b`; a `path` from `x`'s definition to each |
| j28 | `{items.map((it) => <Row key={it.id} item={it} />)}`; the same with `item={it}` before `key` | `it → Row:item`, with a `path` from the arrow's formal `it` to `Row:item@…(def)`; nothing from `key` |
| j29 | element in a declarator: `const el = <D title={title} />;`; the multi-line `const a = (` / `<D` / `title={title}` / `/>` / `);`; an assignment `el = <D title={title} />;` | bound, with a `path` from `title`'s definition |
| j30 | multi-line expression-bodied arrow: `const Card = (item) => (` / `<div>` / `<Row item={item} />` / `</div>` / `);` | bound, with a `path` from `Card:item@1(def)` |
| j31 | one-line expression-bodied arrow: `const One = ({ title }) => <D title={title} />;` | bound, with a `path` from `One:title@1(def)` (stage D: mint the signature-line use, or accept the loss; if accepted, no edge and `refused_source_not_found` = 1) |
| j32 | element as an object-literal value, a conditional branch and a JSX child, each in a declarator: `const o = { icon: <D title={title} /> };`; `const g = c ? <D title={title} /> : null;`; `const h = <div><D title={title} /></div>;` | bound, with a `path` from `title`'s definition |
| j33 | a value captured from the enclosing component inside a callback: `{items.map((it) => <Row onPick={onPick} />)}`; the block-bodied `{items.map((it) => {` / `return <Row onPick={onPick} />;` / `})}`, with `onPick` a local or prop of the component | bound, with a `path` from the component's `onPick` definition (stage D: bind from the enclosing owner's node, or accept the loss; if accepted, no edge and `refused_source_in_enclosing_owner` = 1 for the expression-bodied form, and the stage D outcome for the block-bodied form) |
| j34 | member tags through an imported value: `import { Ctx } from "./ctx"; <Ctx.Provider value={a}>` with `ctx` declaring `export function Provider({ value })`; `import { Menu } from "./table"; <Menu.Item html={a} />` with `table` holding `export const Menu = { Item: Impl }` and an unrelated function `Item`; the same through a default import; `<Tabs.Tab html={a} />` with `Tabs.Tab = function TabImpl(…)` and an unrelated `export function Tab` | no edge; `refused_member_qualifier` = 1 each |
| j35 | tag spelling differs from the function's name: `import { Html as RichText } from "./ui"; <RichText html={text} />`; `import View from "./ui"` with `export default function Html`; `const Button = memo(function ButtonInner({ label }) {…}); <Button label={t} />` | the edge into the target's pattern Def exists with a `path` from the value's definition; `trace_boundary` (DESIGN 5.6 limit) |
| j36 | wrappers: `const C = observer(({ a }) => {…})` with `observer` imported from a package; `styled(({ className }) => …)`; `withRouter(…)`; `memo(observer(fn))`; `export default observer(function X({ a }) {…})`; `const M = React.memo(function M2({ html }) {…})` with `import React from "react"` | none, `refused_unknown_wrapper` = 1 each, for the first five; the last binds like j04 |
| j37 | one-line component: `function Card({ title, body }) { return <div title={title} dangerouslySetInnerHTML={{ __html: body }} />; }` with `<Card title={t} body={safe} />`, `t` from a source; the same component over several lines | one-line: `trace_reaches` from `t`'s definition to the sink line (accepted limit, DESIGN 5.6); several lines: `trace_misses` for the sink line |
| j38 | `<C children={x}>{y}</C>`; `<C children={x} />` | nothing from `x`, `refused_overwritten` = 1; `x → C:children` |

- [ ] Write the rows file, the README (row schema and assertion kinds) and the test. Confirm: control rows
      green on main; record the RED run for the other groups.
- [ ] Commit; CI green (only `control` is enabled).

### Task T-2: enumeration

- [ ] Run the tiers (ORCHESTRATION section 6) on `rows.jsonl`. Axes: pattern syntax (every destructuring
      form the grammar allows in a formal, defaults at every level), formal position, function kind and how
      the function is named, default kinds, scoping and shadowing, body writes and captures, TypeScript
      annotations; literal key forms; JSX attribute and value syntax, element forms (member with each
      qualifier kind, generic, fragment, nested), wrappers (React's and others), component definition forms,
      tag spelling against the function's name, file kind. The census's shapes are rows.

### Task T-3: element-context enumeration

The source rule of DESIGN 5.3 depends on where an element sits and which function holds the value's node,
so this axis gets its own pass, run with the same tiers and rules as Task T-2.

- [ ] Brief: for each statement context of a JSX element crossed with each owner kind, a row with the input,
      the required behaviour (the attribute binds and its source is reached from the value's definition),
      the truth label and the class. Seed cells (rows j25 to j33 cover the probed ones): statement contexts
      `return`, declarator, assignment, expression statement, arrow expression body (one line and several),
      call argument, object-literal value, conditional, logical guard, JSX child, JSX attribute value of
      another element; owner kinds named function, const-bound arrow, method, anonymous callback, callback
      inside JSX (expression-bodied and block-bodied). Cells not yet probed, to be covered: class members,
      getters and setters, generators, `switch` arms, `try`, `catch` and `finally` blocks, default-parameter
      values, elements nested in template literals, and a value whose spelling occurs earlier on its line in
      any of these.
- [ ] For each new row, record in the row's `class` whether the DESIGN 5.3 rule is expected to bind it
      (a node of the caller under the line key exists on main, by `nav nodes-at`) or not. Rows the rule does
      not cover go to stage D as decisions (fix in this lane, or accepted loss with its census count).

## Stage D: design of record (1 to 1.5 days)

- [ ] The design seat amends DESIGN sections 5.1 to 5.3 against the full row set: removes "tentative",
      states each rule, lists accepted losses (including the outcomes for j05, j31, j33 and every Task T-3
      row the 5.3 rule does not cover, each with its census count). The implementer updates the rows'
      expectations to match. Both reviewers; cap 2 rounds.

---

## Stage I: slices

### Slice S1: definitions for pattern formals (4.5 to 5 days)

**Files:**
- Create: `src/ast/js_ts/pattern_params.rs`, `src/ast/js_ts/pattern_params_tests.rs`
- Modify: `src/ast/js_ts.rs` (module lines), `src/data_flow/build/file.rs` (register Defs for admitted
  pattern locals in the loop that handles `function_parameter_occurrences`, with the same classification
  and refusals), `src/cpg/props_matrix_tests.rs` (`ENABLED` gains `"S1"`), `src/cpg_cache.rs` (version), docs
- Tests to re-run and, where they compare the live producer with the test-only legacy producer
  (`src/data_flow/build/legacy_producer.rs`), to adjust by excluding owners that have a pattern formal, each
  exclusion named in the PR: find them with `rg -l "file_refs_frozen" src/` (the command's list is
  authoritative; at 1179055f it names 15 test files under `src/data_flow/build/`, among them
  `js_prune_tests.rs`, `prune_cap_tests.rs`, `lexical_prune_tests.rs`, `scope_prune_tests.rs`,
  `order_prune_tests.rs` and the C, C++, Rust and Python prune tests). Also run
  `parameter_slots_parity_tests`, `js_param_defs_tests`, `inert_default_parameter_tests`,
  `member_only_param_tests`; none should change.

**Interfaces (produced):**

```rust
// src/ast/js_ts/pattern_params.rs
pub(crate) struct PatternElement { pub key: Vec<String>, pub key_known: bool, pub local: String,
    pub local_start: usize, pub local_end: usize,       // bytes of the binding identifier token
    pub defaulted: bool, pub rest: bool, pub admitted: bool }
pub(crate) struct PatternParam { pub position: usize, pub alias_skipped: bool,
    pub elements: Vec<PatternElement> }                  // no elements when alias_skipped
impl ParsedFile {
    /// Object-pattern formals of a JavaScript, TypeScript or TSX callable. Empty for other languages and
    /// for a parameter list with recovery nodes. A pattern the parameter-default alias code defines is
    /// returned with `alias_skipped` and no elements.
    pub(crate) fn js_ts_pattern_params(&self, function: &tree_sitter::Node<'_>) -> Vec<PatternParam>;
    /// The admitted locals as (name, start, end): the shape `function_parameter_occurrences` returns.
    pub(crate) fn js_ts_pattern_local_occurrences(&self, function: &tree_sitter::Node<'_>) -> Vec<ParameterOccurrence>;
}
```

`admitted` is DESIGN 5.1's admission. `alias_skipped` is the alias code's own predicate
(`param_alias_of_entry`, reached through a `pub(crate)` wrapper in `src/ast/js_ts/param_alias.rs` if
needed). `position` counts the formals before the pattern in the syntactic list. The local's bytes are the
identifier token's (`shorthand_property_identifier_pattern` for `{ a }`, the `value` identifier of a
`pair_pattern`, the left identifier of a default, the identifier of a rest pattern). Reuse the existing
destructuring and default helpers where they fit (`extract_destructuring_def_spans`,
`is_inert_default_value`) and say in the PR which were reused.

- [ ] Unit tests for `js_ts_pattern_params` first, in `pattern_params_tests.rs`: one per row of DESIGN 5.1's
      element table, one per admission rule (the intermediate-default rule and the assertion-wrapper removal
      included), and the alias-default exclusion for an earlier formal and for an outer variable (p13,
      p29). They do not compile until the module exists: add the module skeleton returning empty lists in
      the same commit, so the tests are RED, not broken.
- [ ] Implement the accessor. Unit tests GREEN.
- [ ] Enable group `S1` in `props_matrix_tests.rs`: RED. Extend the producer loop in `file.rs`. GREEN.
- [ ] Review focus items 1 and 2 are rows p14, p15, p16, p26, p27: confirm they pass.
- [ ] Write the differential prediction (DESIGN section 7, "Slice S1"), then measure X, R2, T and G with
      `measure.sh` and `compare_props.py`. Read every `removed` and `label-move` row; each needs an
      explanation in the PR or a fix. Counters outside the data-flow statistics must not move; G must show
      no change. Run `oracle_pattern_defs.cjs` on the `pattern-def-intra` additions of X and R2 and the hand
      read of 30 listed `exact` edges (a Claude seat).
- [ ] Tier-A matrix and quick runs; cloud tests at 1 and 4 threads with a RED-on-base control; two reviewers.
- [ ] PR body: the prediction, the class counts per corpus, the reaching-definition availability counter
      deltas, the binding-identity oracle and hand-read results, the Tier-A result, the list of excluded
      parity owners.

**Gate:** DESIGN section 7 "Slice S1".

### Slice S2: props binder with pattern targets, for object-literal arguments (3.5 to 4.5 days)

Two PRs: S2a is a pure move and is reviewed as one; S2b adds behaviour.

**Files:**
- Create: `src/cpg/props_binding.rs`
- Modify: `src/cpg/object_literal_args.rs` (becomes the literal source and calls the binder),
  `src/cpg/callee_params.rs` (pattern facts), `src/cpg/build.rs` (module line; pass pattern facts; merge
  counters; the `dfg_props` field on `DfgLabelStats`), `src/cpg/callee_legacy.rs`,
  `src/cpg/step5b_reference.rs`, `src/cpg/props_matrix_tests.rs` (`ENABLED` gains `"S2"`),
  `src/cpg/object_literal_arg_tests.rs` (`destructured_rest_and_bare_arrow_formals_are_out_of_scope`: its
  destructured case moves to the matrix with the new expectation; the rest and bare-arrow cases stay),
  `src/cpg/callee_param_cache_tests.rs` and any test that names the cache's types, if they change,
  `crates/prism-review/tests/cli/min_confidence_test.rs` (the statistics golden lists every field),
  `src/cpg_cache.rs` and `src/cpg_cache/stream.rs` (version; the statistics record if it is serialised),
  `src/cpg.rs` (test module lines), docs

**Interfaces (produced):**

```rust
// src/cpg/callee_params.rs  (compute_param_def_nodes and its return type are unchanged)
pub(super) struct PatternEntry<T> { pub key: Vec<String>, pub key_known: bool, pub target: Option<T>,
                                    pub defaulted: bool, pub rest: bool }
pub(super) struct PatternFormal<T> { pub position: usize, pub alias_skipped: bool,
                                     pub entries: Vec<PatternEntry<T>> }
/// Graph nodes of the callee's admitted pattern locals. The twins call this directly.
pub(super) fn compute_pattern_formal_nodes(parsed: &ParsedFile, callee: &FunctionId,
    var_index: &VarIndex, graph: &DiGraph<CpgNode, CpgEdge>) -> Vec<PatternFormal<NodeIndex>>;
impl<'a> ParamCache<'a> {
    /// The same facts as cached `ParamTarget`s, memoised per callee beside the positional cell.
    pub(super) fn patterns(&self, callee: &FunctionId, parsed: &ParsedFile, index: &VarIndex,
        graph: &DiGraph<CpgNode, CpgEdge>) -> std::sync::Arc<Vec<PatternFormal<ParamTarget>>>;
}

// src/cpg/props_binding.rs
pub(super) enum PropsValue { Leaf(std::ops::Range<usize>), Constant, Opaque }
pub(super) enum PropsEntry { Key { keys: Vec<String>, value: PropsValue }, Spread, Wildcard }
pub(super) enum PropsTarget<'a, T> { Plain(&'a T), Pattern(&'a PatternFormal<T>) }
#[derive(Default, Clone, PartialEq, Eq, Debug)]
pub(super) struct PropsCounters(pub BTreeMap<&'static str, usize>);      // keys of DESIGN 5.4
pub(super) fn bind<T>(out: &mut Vec<PendingArgumentFlow>, counters: &mut PropsCounters,
    site: &SiteInputs<'_>, entries: &[PropsEntry], target: PropsTarget<'_, T>,
    source: impl Fn(&AccessPath, &std::ops::Range<usize>) -> Result<NodeIndex, &'static str>,
    resolve: impl Fn(&T) -> Option<NodeIndex>);

// src/cpg/build.rs
pub struct DfgLabelStats { /* existing fields */ pub dfg_props: BTreeMap<String, usize> }
```

`source` is the source-node lookup of DESIGN 5.3 for the site kind, for one path of a Leaf value and the
value's span: the literal's byte-span lookup here (its `Err` is never counted, as #594 counts nothing), the
JSX line-keyed lookup `jsx_props::source_node` in S3 (its `Err` is the refusal key to count). `object_literal_args::collect<T>` keeps its generic signature and gains
two parameters: the callee's pattern formals in the same representation `T`, and `&mut PropsCounters`. For
the argument at position `i` it binds to the positional target `i` as today, or, when a pattern formal has
`position == i`, to that pattern. The step returns the counters beside the flows per caller; assembly merges
them in caller order into `dfg_props`.

- [ ] **S2a (its own PR), no behaviour change:** move the pairing from `object_literal_args.rs` into `bind`
      (plain targets only, no counters, no statistics field). The existing object-literal tests stay green;
      a base-versus-head differential on X and T shows zero changes in edges and statistics. No cache bump.
      Both reviewers, briefed that it is a pure move; merge before S2b branches.
- [ ] **S2b, step 1:** the counters, the `dfg_props` field and the string-key representation change of
      DESIGN 5.2 (plain targets unchanged). A differential on X and T shows zero edge changes and only the
      new `dfg_props` keys.
- [ ] **S2b, step 2:** pattern facts (`compute_pattern_formal_nodes`, the cache method), admission checks 1,
      2 and 4 of DESIGN 5.3 for literal sites, and the pattern target in `bind`. Enable group `S2`: RED,
      implement, GREEN.
- [ ] Update the three tests named under Files; say in the PR which assertions moved or changed.
- [ ] Differential on X, R2, T: only `props-literal` additions. Oracle on the additions: 0 WRONG.
- [ ] Review focus item 5: extend `object_literal_parity_tests.rs` with a pattern-formal case (production,
      legacy twin, serial reference, cache round trip, incremental rebuild: identical edges and counters).
- [ ] Tier-A; cloud tests; two reviewers.

### Slice S3: JSX attribute source (4 to 5 days)

**Files:**
- Create: `src/cpg/jsx_props.rs`, `src/ast/js_ts/component_route.rs`, `src/ast/js_ts/component_route_tests.rs`,
  `tests/algo/taxonomy/taint_jsx_props_test.rs` (with its `mod` line in `tests/algo/taxonomy/main.rs`)
- Modify: `src/ast/js_ts.rs` (module lines), `src/cpg/build.rs` (module line and the JSX branch),
  `src/cpg/callee_legacy.rs`, `src/cpg/step5b_reference.rs`, `src/cpg/props_matrix_tests.rs` (`ENABLED`
  gains `"S3"`), `src/cpg/object_literal_parity_tests.rs`, `src/cpg_cache.rs`, docs

**Interfaces (produced):**

```rust
// src/ast/js_ts/component_route.rs
pub(crate) enum ComponentRoute { Direct, ReactWrapper, OtherWrapper }
impl ParsedFile {
    /// DESIGN 5.3 check 3 for the function node of a JSX target.
    pub(crate) fn js_ts_component_route(&self, function: &tree_sitter::Node<'_>) -> ComponentRoute;
}

// src/cpg/jsx_props.rs
/// Entries of the JSX element whose opening tag spans [site_start, site_end), in source order.
/// None when the node at that span is not a JSX opening or self-closing element.
pub(super) fn entries(caller: &ParsedFile, site_start: usize, site_end: usize) -> Option<Vec<PropsEntry>>;
/// The DESIGN 5.3 source node of one JSX Leaf value: the line-keyed Use in the site's caller, with the
/// enclosing-owner check that picks the refusal counter (`Err` is the counter key). `site` already carries
/// the caller's id, parsed file, `var_index` and graph.
pub(super) fn source_node(site: &SiteInputs<'_>, path: &AccessPath, value_span: &std::ops::Range<usize>)
    -> Result<NodeIndex, &'static str>;
/// Binds the element's entries to the callee's first formal: the positional target 0, or the pattern
/// formal at position 0.
pub(super) fn collect<T>(out: &mut Vec<PendingArgumentFlow>, counters: &mut PropsCounters,
    site: &SiteInputs<'_>, site_span: (usize, usize), positional: &[Option<T>], patterns: &[PatternFormal<T>],
    resolve: impl Fn(&T) -> Option<NodeIndex>);
```

**The branch.** In the loop over a site's resolved targets, after the skip for `R6MultiOwnerCandidate` and
after `callee_id` and `caller_parsed` are known, and **before** the site's arguments are fetched (a JSX site
has none, and the code after that point skips it): if `site.jsx_element`, then apply the target admission of
DESIGN 5.3 in its order (check 2 reads `site.local_binding`; check 3 calls `js_ts_component_route` on the
callee's function node), and for an admitted site whose callee file is loaded, obtain the callee's
positional and pattern facts exactly as the call path does and call `jsx_props::collect`; in every case
`continue`. The source node of each Leaf comes from `jsx_props::source_node`, never from the call path's
byte-span lookup. The same branch goes at the same position in the two twins (each with its own index
representation). `SiteInputs.args` is empty for a JSX site.

- [ ] Unit tests for `js_ts_component_route` first (a direct function, both admitted wrapper forms, each
      wrapper of row j36, the `lazy` loader, a wrapper declared inside a function); skeleton returning
      `Direct` in the same commit so they are RED. Implement; GREEN.
- [ ] Enable group `S3`: RED. Implement `entries`, `source_node`, then `collect`, then the branch in the
      three copies. GREEN row group by row group: sources (j06 to j14, j17, j20 to j23, j38), source nodes
      (j25 to j33, with the stage D outcomes), admission (j15, j16, j34, j36), targets (j01 to j05, j24),
      engines (j01's and j18's `trace_reaches`, j19's and j35's `trace_boundary`, j37).
- [ ] `taint_jsx_props_test.rs`: the issue's example in one TSX file with an explicit source line; default
      review taint reports a `taint_sink` finding on the `dangerouslySetInnerHTML` line. RED before the
      branch exists, GREEN after.
- [ ] Parity: add a JSX case to `object_literal_parity_tests.rs` (Review focus item 5).
- [ ] Differential on X and R2: only `props-jsx` additions. Oracle with checks 1 to 3: 0 WRONG; UNDECIDED
      share within the gate; hand read of the `undecided_sample.py` sample (50 per corpus, all when fewer)
      by a Claude seat; `paths.py` connected share.
- [ ] Review focus items 3, 4, 6 and 7 are rows j21, j18, j25 to j28, j34 and j36: confirm they pass.
      Tier-A; cloud tests; two reviewers.

**Gate:** the oracle, volume and end-to-end rows of DESIGN section 7 (on X plus R2: ≥ 1,000 CORRECT edges
from JSX attribute values, 0 WRONG, UNDECIDED ≤ 25% per corpus, connected share ≥ 90% per corpus, ≥ 25
chains with two or more component hops).

### Slice S4: end-to-end measurement and close-out (3 to 4 days)

**Files (evidence directory):** `g/paths-{X,R2}.md`, `g/examples/`, `g/REPORT.md`; `tools/examples.py`.

- [ ] `paths.py` on X and R2: the path table, the multi-hop chain count and the connected share.
- [ ] `examples.py <repo with history> <base binary> <head binary> <direction> <n> <out dir>`: list
      first-parent commits, newest first; parse each candidate file at the commit with the pinned
      TypeScript; take the first `n` qualifying commits in non-test `.tsx` or `.jsx` files. For each, check
      out the commit into a scratch worktree and write its patch.
      - `forward`: a commit qualifies when its diff changes a line inside a JSX attribute's value expression
        on a capitalised element. Run `prism nav --no-cache taint-reaches --repo … --source <file>:<line>
        --format json` (frontier mode) for each changed attribute line with both binaries; write the
        frontier lines only the head reaches, with their source text, to `g/examples/fwd-NN.md`.
      - `backward`: a commit qualifies when its diff changes a line in a component body that reads one of
        its props (a pattern local of its first formal, or a member read of a plain first formal). Run
        `prism --repo … --diff … --algorithm fullflow --format json` with both binaries; write the lines
        only the head adds to `g/examples/bwd-NN.md`.
      Run 15 per direction on R2. If a full clone of Excalidraw can be fetched
      (`git clone https://github.com/excalidraw/excalidraw`), take 8 per direction from R2 and 7 from
      Excalidraw instead; record which.
- [ ] Dispatch the two readers (Opus-5.5 high and Sonnet-5.5 high, separate sessions) on every example with
      the rubric of DESIGN section 6. Tally per direction with the combination rule; report agreement and
      list every disagreement.
- [ ] Run the census, differential, oracle and path scripts on `private-corpus`; if it has git history, run
      `examples.py` there with up to 10 per direction and the same two Claude readers. Write aggregate
      numbers to the private evidence file and only counts to the report.
- [ ] `g/REPORT.md`: gates of DESIGN section 7 with results; the `dfg_props` breakdown on X and R2
      (including the shares of `edges_to_bare_use` and `edges_to_rest`, and every refusal key); the
      follow-up table F1 to F9 with the census numbers beside each threshold.
- [ ] Post the report on #596 and close it if every gate holds. File issues for: the trace-engine stop at
      edges that end at a member read and at name-mismatched tags (F5); the attribute-name uses in the
      caller; the resolver defect of DESIGN 10 (a wrong Exact target for member calls and member elements
      whose qualifier is an imported value), with the inputs of rows j34 and l21 and the review's
      reproductions in `~/prism-evidence/lanes-2026-11-20/reviews/lane3-fable-r1-probes/` (`res2`, `res3`),
      copied into the issue as code; and each follow-up whose threshold is met.
- [ ] Refresh the handoff; offload raw dumps; delete `target/`.

## Acceptance for the lane

Every gate of DESIGN section 7 holds on the merged head; both reviewers approved S1, S2a, S2b and S3; the
report is posted. A gate that fails is reported with its numbers and the lane stops for the owner;
thresholds are not adjusted by the orchestrator.

An oracle WRONG (or a hand-read WRONG in an UNDECIDED sample) is never overridden by the orchestrator. If the
orchestrator believes a verdict is an artifact of the oracle, it sends the raw row to both reviewers; a
verdict is changed only on per-row proof, and a change for a whole class of rows needs that proof for every
row in it.
