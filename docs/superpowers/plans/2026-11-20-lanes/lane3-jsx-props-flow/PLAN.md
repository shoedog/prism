# Lane 3: React value flow across JSX props — implementation plan

> **For agentic workers:** this plan is run by an orchestrator under `../ORCHESTRATION.md`. Each task is
> dispatched to an implementer (Sonnet-5.5, high) with its section as the brief. Steps use checkbox syntax.

**Goal:** a value passed as a JSX attribute reaches the component's use of that prop in prism's data-flow
graph, for Exact-resolved components with destructured or plain props, proven by a type-checker oracle,
end-to-end path counts and rated review examples.

**Architecture:** destructured formals get definitions from the data-flow producer through a new accessor
that leaves parameter occurrences and positional slots untouched; the #594 object-literal binder is split
into props sources (object literal, JSX attributes) and props targets (plain formal reads, pattern-formal
definitions), keeping its generic shape; JSX sites enter the argument-binding step through one new branch.

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
- Edges only for a site that resolves to exactly one Exact callee in the JavaScript family.
- **Not changed by this lane:** `function_parameter_occurrences`, `function_parameter_names`,
  `function_parameter_slots`, `function_parameter_slot_occurrences`, the slot builders in
  `src/parameter_slots.rs`, `compute_param_def_nodes` and its return type, the reasoning layer's seeds, any
  taint code, `src/cpg/trace.rs` (PR #841 is editing it).
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

## Review focus

1. A pattern local with the same name as an outer variable: the body's use is fed by the pattern Def and
   by nothing from the outer scope (already true of reference search; pinned so it stays true). (S1)
2. A pattern local reassigned in the body, and one captured by a nested callable: labels follow the rules
   that hold for identifier formals; no `exact` edge survives a write that kills it. (S1)
3. An attribute whose name equals a local variable's name but whose value is another variable
   (`const title = a; <D title={b} />`): the edge starts at `b`, never at `title`. (S3)
4. An element spanning several lines with the attribute on a later line than the tag: the edge exists and
   the trace engine crosses it. (S3)
5. A cold build, a full cache hit and a partial cache hit give the same props edges and the same
   `dfg_props` counters, at `RAYON_NUM_THREADS` 1 and 4. (S2, S3)

---

## Stage M: census and tooling (2 days)

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
      attribute with: site file and line; resolution outcome (exact singleton, name-only, several targets,
      unresolved with the drop reason); attribute name; value shape (identifier, member path, object
      literal, string, bare, call, conditional, template, function, element, other); whether a spread
      precedes or follows it; the target's first-formal shape (object pattern: flat, alias, default inert,
      default other, default another formal, rest, nested, computed, array inside; identifier: read by
      member only, read bare, destructured in the body; none); whether the target is a named function, a
      const-bound arrow, or an anonymous function inside an admitted wrapper; and whether the pattern or
      the member reads contain the attribute's key.
- [ ] A **predicted bindable pair** is one (attribute or object-literal leaf, target) such that: the site
      is an exact singleton; the value is an identifier or member path (each leaf of an object-literal value
      counts once); no spread follows the attribute; and the target's first formal is a pattern with an
      entry whose key path is comparable with the attribute's, or an identifier formal with a member read of
      that key or a bare read.
- [ ] Run on X and on the three candidates for R2 (`calcom-cal.com`, `apache-superset`, `apache-answer`).
      R2 is the candidate with the most predicted pairs (ties: the alphabetically first). Run on
      `private-corpus` with `--aggregate-only`.
- [ ] `CENSUS.md`: the tables per corpus, the predicted pairs, and the numbers DESIGN section 9 names for
      F1 to F5 and F7.

**Exit gate (census go):** predicted bindable pairs ≥ 1,000 on X plus R2. Below it: stop, post the tables on
#596, and wait for the owner.

### Task M-3: comparator and oracle

**Files (evidence directory):** `tools/compare_props.py`, `tools/oracle_props_edges.cjs`, `tools/paths.py`,
`tools/measure.sh`.

- [ ] `measure.sh <binary> <corpus> <outdir>`: runs, through memguard, `nav --no-cache dfg-stats`,
      `nav --no-cache call-stats`, and `PRISM_EDGES_BYTES=1 nav --no-cache dfg-stats --edges`.
- [ ] `compare_props.py <base dir> <head dir>`: keys each edge by (file, path, line, bytes) at both ends;
      classes: `pattern-def-intra` (a new edge that starts at a Def absent from the base, inside one
      function), `props-literal` (a new cross-function edge whose source lies in an object-literal
      argument), `props-jsx` (the same for a JSX attribute value), `removed` and `label-move` (each listed
      in full: none is expected); prints every counter delta of both stats files; exits 1 on an
      unclassified change. Model: `~/prism-evidence/jsts-lane/783/scripts/compare761.py`.
- [ ] `oracle_props_edges.cjs <corpus> <added-edges.jsonl>`: verdicts per DESIGN section 6 (tag symbol →
      declaration equals the target span, or a `const` initialised by `memo(…)`/`forwardRef(…)` around it;
      key equality from syntax). It loads the corpus's own files with no dependency installation. Self-test
      it on a fixture with one correct edge, one edge to the wrong component and one edge with the wrong
      key before using it.
- [ ] `paths.py <head edges> <oracle verdicts>`: the end-to-end path table of DESIGN section 6.

---

## Stage T and E: test matrix (3 days)

### Task T-1: matrix data and one data-driven test

**Files:**
- Create: `tests/fixtures/props_matrix/rows.jsonl`, `tests/fixtures/props_matrix/README.md`
- Create: `src/cpg/props_matrix_tests.rs` (declared with `#[cfg(test)] mod` in `src/cpg.rs`)

Each row: `id`, `group` (`control`, `S1`, `S2`, `S3`), `langs` (which of `js`, `ts`, `tsx`, `jsx`), `files`
(name → source text), `expect` (a list of assertions), `class`, `truth`. Assertion kinds, all evaluable on
main with the helpers `src/cpg/object_literal_arg_tests.rs` already has (`build`, node spelling
`function:path@line`): `def`, `no_def`, `edge` (optionally with a label), `no_edge`, `no_edge_into`,
`counter` (a key of the `dfg_props` map read from the serialised statistics; an absent key reads as 0),
`trace_reaches` and `trace_boundary` (through the trace engine, as the existing plain-call trace tests do).

`props_matrix_tests.rs` holds one test, `props_matrix_rows`, with a constant
`ENABLED: &[&str] = &["control"]`. It runs the rows of enabled groups and skips the rest. On main the
control rows pass. For every other row the implementer records, once, that it fails when its group is
enabled by hand (the RED record, in the ledger).

Rows asserting `exact` on an edge inside a function use bodies of at least two lines; a one-line owner is
labelled by the single-node rules.

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
| l05 | pattern `{ a, ...rest }`, argument `{ a: x, b: y }` | `x → a`, `y → rest` |
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
| l16 | `P({ a: x })` into `function P(props) { const { a } = props; … }` | `x → P:props` bare read, as #594 gives today; counter `edges_to_bare_use` = 1 |

**Group S3 (JSX attributes; TSX and JSX only):**

| Row | Shape | Required |
|---|---|---|
| j01 | the issue's example, element and component on separate lines | `App:text(use) → Html:html@1(def)`, exact; `trace_reaches` from `text` to the `dangerouslySetInnerHTML` line |
| j02 | `<HtmlP html={text} />` into `function HtmlP(props)` reading `props.html` and `props.other` | edge into `HtmlP:props.html(use)`; no edge into `props.other` |
| j03 | arrow component; named import; default import; `<UI.Html>` | as j01 |
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
| j16 | `React.lazy` loader target; component with no formals | none; `refused_no_props_formal` = 1 |
| j17 | `<C a={x}>{y}</C>` | `x` binds; nothing from `y` |
| j18 | element over three lines, attribute on line 2 | edge exists; `trace_reaches` |
| j19 | two `<D a={x} />` on one line | both edges exist; `trace_boundary` (today's limit, pinned) |
| j20 | `<A icon={<B x={v} />} />` | `v → B`'s `x`; nothing into `A`'s `icon` |
| j21 | `const title = a; <D title={b} />` | from `b` only |
| j22 | `{items.map(i => <D a={i.x} />)}`; `{cond && <D a={x} />}` | bound; in the first, the source is the use in the arrow passed to `map` (an anonymous owner, spelled as the harness prints it) |
| j23 | `.jsx` file; `<C<T> x={v} />` | as j01 |
| j24 | `<P a={x} />` into `function P(props) { const { a } = props; … }` | `x → P:props` bare read; counter `edges_to_bare_use` = 1 |

- [ ] Write the rows file, the README (row schema and assertion kinds) and the test. Confirm: control rows
      green on main; record the RED run for the other groups.
- [ ] Commit; CI green (only `control` is enabled).

### Task T-2: enumeration

- [ ] Run the tiers (ORCHESTRATION section 6) on `rows.jsonl`. Axes: pattern syntax (every destructuring
      form the grammar allows in a formal), formal position, function kind and how the function is named,
      default kinds, scoping and shadowing, body writes and captures, TypeScript annotations; literal key
      forms; JSX attribute and value syntax, element forms (member, generic, fragment, nested), wrappers,
      component definition forms, file kind. The census's shapes are rows.

## Stage D: design of record (1 day)

- [ ] The design seat amends DESIGN sections 5.1 to 5.3 against the full row set: removes "tentative",
      states each rule, lists accepted losses (including the outcome for j05). The implementer updates the
      rows' expectations to match. Both reviewers; cap 2 rounds.

---

## Stage I: slices

### Slice S1: definitions for pattern formals (4 to 5 days)

**Files:**
- Create: `src/ast/js_ts/pattern_params.rs`, `src/ast/js_ts/pattern_params_tests.rs`
- Modify: `src/ast/js_ts.rs` (module lines), `src/data_flow/build/file.rs` (register Defs for admitted
  pattern locals in the loop that handles `function_parameter_occurrences`, with the same classification
  and refusals), `src/cpg/props_matrix_tests.rs` (`ENABLED` gains `"S1"`), `src/cpg_cache.rs` (version), docs
- Tests to re-run and, where they compare the live producer with the test-only legacy producer
  (`src/data_flow/build/legacy_producer.rs`), to adjust by excluding owners that have a pattern formal, each
  exclusion named in the PR: find them with `rg -l "legacy_producer|with_legacy"` under `src/`. Also run
  `parameter_slots_parity_tests`, `js_param_defs_tests`, `inert_default_parameter_tests`,
  `member_only_param_tests`; none should change.

**Interfaces (produced):**

```rust
// src/ast/js_ts/pattern_params.rs
pub(crate) struct PatternElement { pub key: Vec<String>, pub key_known: bool, pub local: String,
    pub local_start: usize, pub local_end: usize,       // bytes of the binding identifier token
    pub defaulted: bool, pub rest: bool, pub admitted: bool }
pub(crate) struct PatternParam { pub position: usize, pub elements: Vec<PatternElement> }
impl ParsedFile {
    /// Object-pattern formals of a JavaScript, TypeScript or TSX callable. Empty for other languages,
    /// for a parameter list with recovery nodes, and for patterns the parameter-default alias code handles.
    pub(crate) fn js_ts_pattern_params(&self, function: &tree_sitter::Node<'_>) -> Vec<PatternParam>;
    /// The admitted locals as (name, start, end): the shape `function_parameter_occurrences` returns.
    pub(crate) fn js_ts_pattern_local_occurrences(&self, function: &tree_sitter::Node<'_>) -> Vec<ParameterOccurrence>;
}
```

`admitted` is DESIGN 5.1's admission. `position` counts the formals before the pattern in the syntactic
list. The local's bytes are the identifier token's (`shorthand_property_identifier_pattern` for `{ a }`,
the `value` identifier of a `pair_pattern`, the left identifier of a default, the identifier of a rest
pattern). Reuse the existing destructuring and default helpers where they fit
(`extract_destructuring_def_spans`, `is_inert_default_value`) and say in the PR which were reused.

- [ ] Unit tests for `js_ts_pattern_params` first, in `pattern_params_tests.rs`: one per row of DESIGN 5.1's
      element table, one per admission rule, one for the alias-default exclusion (p13). They do not compile
      until the module exists: add the module skeleton returning empty lists in the same commit, so the
      tests are RED, not broken.
- [ ] Implement the accessor. Unit tests GREEN.
- [ ] Enable group `S1` in `props_matrix_tests.rs`: RED. Extend the producer loop in `file.rs`. GREEN.
- [ ] Review focus items 1 and 2 are rows p14, p15, p16: confirm they pass.
- [ ] Write the differential prediction (DESIGN section 7, "Slice S1"), then measure X, R2, T and G with
      `measure.sh` and `compare_props.py`. Read every `removed` and `label-move` row; each needs an
      explanation in the PR or a fix. Counters outside the data-flow statistics must not move; G must show
      no change.
- [ ] Tier-A matrix and quick runs; cloud tests at 1 and 4 threads with a RED-on-base control; two reviewers.
- [ ] PR body: the prediction, the class counts per corpus, the reaching-definition availability counter
      deltas, the Tier-A result, the list of excluded parity owners.

**Gate:** DESIGN section 7 "Slice S1".

### Slice S2: props binder with pattern targets, for object-literal arguments (3 to 4 days)

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
pub(super) struct PatternFormal<T> { pub position: usize, pub entries: Vec<PatternEntry<T>> }
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
    resolve: impl Fn(&T) -> Option<NodeIndex>);

// src/cpg/build.rs
pub struct DfgLabelStats { /* existing fields */ pub dfg_props: BTreeMap<String, usize> }
```

`object_literal_args::collect<T>` keeps its generic signature and gains two parameters: the callee's
pattern formals in the same representation `T`, and `&mut PropsCounters`. For the argument at position `i`
it binds to the positional target `i` as today, or, when a pattern formal has `position == i`, to that
pattern. The step returns the counters beside the flows per caller; assembly merges them in caller order
into `dfg_props`.

- [ ] Step 1, no behaviour change: move the pairing from `object_literal_args.rs` into `bind` (plain targets
      only). The existing object-literal tests stay green; a base-versus-head differential on X and T shows
      zero edge changes and only the new `dfg_props` keys in the statistics. Commit this step alone.
- [ ] Step 2: pattern facts (`compute_pattern_formal_nodes`, the cache method) and the pattern target in
      `bind`. Enable group `S2`: RED, implement, GREEN.
- [ ] Update the three tests named under Files; say in the PR which assertions moved or changed.
- [ ] Differential on X, R2, T: only `props-literal` additions. Oracle on the additions: 0 WRONG.
- [ ] Review focus item 5: extend `object_literal_parity_tests.rs` with a pattern-formal case (production,
      legacy twin, serial reference, cache round trip, incremental rebuild: identical edges and counters).
- [ ] Tier-A; cloud tests; two reviewers.

### Slice S3: JSX attribute source (3 to 4 days)

**Files:**
- Create: `src/cpg/jsx_props.rs`, `tests/algo/taxonomy/taint_jsx_props_test.rs` (with its `mod` line in
  `tests/algo/taxonomy/main.rs`)
- Modify: `src/cpg/build.rs` (module line and the JSX branch), `src/cpg/callee_legacy.rs`,
  `src/cpg/step5b_reference.rs`, `src/cpg/props_matrix_tests.rs` (`ENABLED` gains `"S3"`),
  `src/cpg/object_literal_parity_tests.rs`, `src/cpg_cache.rs`, docs

**Interfaces (produced):**

```rust
// src/cpg/jsx_props.rs
/// Entries of the JSX element whose opening tag spans [site_start, site_end), in source order.
/// None when the node at that span is not a JSX opening or self-closing element.
pub(super) fn entries(caller: &ParsedFile, site_start: usize, site_end: usize) -> Option<Vec<PropsEntry>>;
/// Binds the element's entries to the callee's first formal: the positional target 0, or the pattern
/// formal at position 0.
pub(super) fn collect<T>(out: &mut Vec<PendingArgumentFlow>, counters: &mut PropsCounters,
    site: &SiteInputs<'_>, site_span: (usize, usize), positional: &[Option<T>], patterns: &[PatternFormal<T>],
    resolve: impl Fn(&T) -> Option<NodeIndex>);
```

**The branch.** In the loop over a site's resolved targets, after the skip for `R6MultiOwnerCandidate` and
after `callee_id` and `caller_parsed` are known, and **before** the site's arguments are fetched (a JSX site
has none, and the code after that point skips it): if `site.jsx_element`, then, when the site is an exact
singleton and the callee's file is loaded, obtain the callee's positional and pattern facts exactly as the
call path does and call `jsx_props::collect`; in every case `continue`. The same branch goes at the same
position in the two twins. `SiteInputs.args` is empty for a JSX site.

- [ ] Enable group `S3`: RED. Implement `entries`, then `collect`, then the branch in the three copies.
      GREEN row group by row group: sources (j06 to j14, j17, j20 to j23), targets (j01 to j05, j15, j16,
      j24), engines (j01's and j18's `trace_reaches`, j19's `trace_boundary`).
- [ ] `taint_jsx_props_test.rs`: the issue's example in one TSX file with an explicit source line; default
      review taint reports a `taint_sink` finding on the `dangerouslySetInnerHTML` line. RED before the
      branch exists, GREEN after.
- [ ] Parity: add a JSX case to `object_literal_parity_tests.rs` (Review focus item 5).
- [ ] Differential on X and R2: only `props-jsx` additions. Oracle: 0 WRONG; record UNDECIDED and read 50.
- [ ] Review focus items 3 and 4 are rows j21 and j18: confirm they pass. Tier-A; cloud tests; two reviewers.

**Gate:** the oracle and volume rows of DESIGN section 7 (≥ 1,000 edges from JSX attribute values on X plus
R2, 0 WRONG).

### Slice S4: end-to-end measurement and close-out (2 to 3 days)

**Files (evidence directory):** `g/paths-{X,R2}.md`, `g/examples/`, `g/REPORT.md`; `tools/examples.py`.

- [ ] `paths.py` on X and R2: the path table.
- [ ] `examples.py <repo with history> <base binary> <head binary> <out dir>`: list first-parent commits,
      newest first; a commit qualifies when its diff changes a line inside a JSX attribute's value
      expression on a capitalised element in a non-test `.tsx` or `.jsx` file (parse the commit's version of
      the file with the pinned TypeScript); take the first 30 qualifying commits. For each, check out the
      commit into a scratch worktree, write its patch, run `prism --repo … --diff … --algorithm
      leftflow,fullflow --format json` with both binaries, and write the lines only the head adds, with
      their source text, to `g/examples/NN.md`. Run it on R2. If a full clone of Excalidraw can be fetched
      (`git clone https://github.com/excalidraw/excalidraw`), take 15 from each instead; record which.
- [ ] Dispatch the reader (Opus-5.5, high) on the 30 files with the rubric of DESIGN section 6. Tally.
- [ ] Run the census, differential, oracle and path scripts on `private-corpus`; write aggregate numbers to
      the private evidence file and only counts to the report.
- [ ] `g/REPORT.md`: gates of DESIGN section 7 with results; the `dfg_props` breakdown on X and R2
      (including the share of `edges_to_bare_use`); the follow-up table F1 to F8 with the census numbers
      beside each threshold.
- [ ] Post the report on #596 and close it if every gate holds. File issues for: the trace-engine stop at
      edges that end at a member read (F5), the attribute-name uses in the caller, and each follow-up whose
      threshold is met.
- [ ] Refresh the handoff; offload raw dumps; delete `target/`.

## Acceptance for the lane

Every gate of DESIGN section 7 holds on the merged head; both reviewers approved S1, S2 and S3; the report
is posted. A gate that fails is reported with its numbers and the lane stops for the owner; thresholds are
not adjusted by the orchestrator.
