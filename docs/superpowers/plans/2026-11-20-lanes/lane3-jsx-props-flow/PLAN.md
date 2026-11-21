# Lane 3: React value flow across JSX props — implementation plan

> **For agentic workers:** this plan is run by an orchestrator under `../ORCHESTRATION.md`. Each task is
> dispatched to an implementer (Sonnet-5.5, high) with its section as the brief. Steps use checkbox syntax.

**Goal:** a value passed as a JSX attribute reaches the component's use of that prop in prism's data-flow
graph, for Exact-resolved components with destructured or plain props, proven by a type-checker oracle,
end-to-end path counts and rated review examples.

**Architecture:** destructured formals get definitions through the existing parameter machinery; the #594
object-literal binder is split into props sources (object literal, JSX attributes) and props targets (plain
formal reads, pattern-formal definitions); JSX sites enter the argument-binding step through one new call.

**Tech stack:** Rust (core crate: `src/ast`, `src/parameter_slots.rs`, `src/cpg`), tree-sitter
JavaScript/TypeScript/TSX; Node with the pinned TypeScript 5.9.3 for the census and oracle; Python 3 for the
comparators.

**Spec:** `DESIGN.md` in this directory (sections 5.1 to 5.4 are the rules; section 7 the gates).

## Global constraints

- Worktree `~/code/prism-jsx-props`; evidence `~/prism-evidence/jsx-props/`.
- Corpora: X = Excalidraw at `0642e72c` (`~/prism-evidence/inputs/excalidraw-0642e72c/source`); T =
  `~/code/bench-repos/TypeScript/src` (regression only); R2 = the second React corpus chosen in Task M-2;
  `private-corpus` locally under ORCHESTRATION section 9.
- TypeScript for scripts: `~/prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js`.
- Edges only for a site that resolves to exactly one Exact callee in the JavaScript family.
- No whole-object edge from an attribute value into a `props` formal.
- No taint code changes. No change to `src/cpg/trace.rs` (PR #841 is editing it).
- `src/cpg/build.rs` is at 1,777 weighted lines: add calls there, put logic in new modules.
- Every production change to the argument-binding step is mirrored in `src/cpg/callee_legacy.rs` and
  `src/cpg/step5b_reference.rs`; `object_literal_parity_tests.rs` must keep passing and gain the new shapes.
- Each slice that changes the graph bumps `CACHE_VERSION` in `src/cpg_cache.rs` with a list line, and
  updates the React and data-flow entries of `docs/features/language-coverage/js-ts-known-gaps.md` and the
  matching paragraph of `CLAUDE.md`.
- Tier-A applies to every slice (`src/ast`, `src/cpg` are touched): ORCHESTRATION section 5 item 5.
- Tests use the conventions of `src/cpg/object_literal_arg_tests.rs`: each case runs for JS, TS and TSX
  where the syntax exists; nodes are spelled `function:path@line`; expected edge sets are written by hand.

## Review focus

1. A pattern local that shadows an outer variable of the same name: after S1 the body's use is fed by the
   pattern Def and by nothing from the outer scope. (S1)
2. A pattern local reassigned in the body, and one captured by a nested callable: labels follow the rules
   that hold for identifier formals; no `exact` edge survives a write that kills it. (S1)
3. An attribute whose name equals a local variable's name but whose value is another variable
   (`const title = a; <D title={b} />`): the edge starts at `b`, never at `title`. (S3)
4. An element spanning several lines with the attribute on a later line than the tag: the edge exists and
   the trace engine crosses it. (S3)
5. A cold build, a full cache hit and a partial cache hit give the same props edges and counters, at
   `RAYON_NUM_THREADS` 1 and 4. (S2, S3)

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
      `prism-base nav --no-cache call-stats --repo <corpus> --dump-sites`, and for every record whose span
      is a JSX element, locate the element and (when resolved) the target function in the TypeScript
      syntax tree by byte span. Emit one JSON line per attribute with: site file and line; resolution
      outcome (exact singleton, name-only, several targets, unresolved with the drop reason); attribute
      name; value shape (identifier, member path, object literal, string, bare, call, conditional,
      template, function, element, other); whether a spread precedes or follows it; the target's
      first-formal shape (object pattern: flat, alias, default inert, default other, rest, nested, computed,
      array inside; identifier: read by member only, mixed, destructured in the body; none); and whether
      the pattern or the member reads contain the attribute's key.
- [ ] Run on X and on the three candidate second corpora (`calcom-cal.com`, `apache-superset`,
      `apache-answer`); choose R2 by DESIGN decision 1. Run on `private-corpus` with `--aggregate-only`.
- [ ] `CENSUS.md`: the tables per corpus, the predicted bindable pairs (identifier or member value, exact
      singleton target, key present), and the numbers that DESIGN section 9 names for F1 to F5.

**Exit gate (census go):** predicted bindable pairs ≥ 1,000 on X plus R2. Below it: stop, post the tables on
#596, and wait for the owner.

### Task M-3: comparator and oracle

**Files (evidence directory):** `tools/compare_props.py`, `tools/oracle_props_edges.cjs`, `tools/paths.py`,
`tools/measure.sh`.

- [ ] `measure.sh <binary> <corpus> <outdir>`: runs, through memguard, `nav --no-cache dfg-stats`,
      `nav --no-cache call-stats`, and `PRISM_EDGES_BYTES=1 nav --no-cache dfg-stats --edges`.
- [ ] `compare_props.py <base dir> <head dir>`: keys each edge by (file, path, line, bytes) at both ends;
      classes: `pattern-def-intra` (a new edge from a pattern local's Def inside its function),
      `shadow-removed` (a removed edge into a use that a pattern local now binds), `positional-after-pattern`,
      `props-literal`, `props-jsx`, `label-move`; prints every counter delta of both stats files; exits 1 on
      an unclassified change. Model: `~/prism-evidence/jsts-lane/783/scripts/compare761.py`.
- [ ] `oracle_props_edges.cjs <corpus> <added-edges.jsonl>`: for each added cross-function edge, verdict
      CORRECT, WRONG, UNDECIDED or INADMISSIBLE per DESIGN section 6. Self-test it on a ten-line fixture
      with one correct and one deliberately wrong edge before using it.
- [ ] `paths.py <head edges> <oracle verdicts>`: the end-to-end path table of DESIGN section 6.

---

## Stage T and E: test matrices (3 days)

Three matrices, one per slice, each as a test file that is RED on main where it states new behaviour. Stage
E (ORCHESTRATION section 6) runs on all three; the census's shapes and counts say which classes need rows.

### Task T-1: pattern formals (`src/cpg/pattern_param_tests.rs`)

| Row | Callee | Required |
|---|---|---|
| p01 | `function C({ html }) {` / `return use(html);` / `}` | Def `C:html@1` with the identifier's bytes; edge `C:html@1 → C:html@2` |
| p02 | `{ html: h }` | Def `h`; no Def `html` |
| p03 | `{ html = "" }` | Def `html` |
| p04 | `{ html = make() }` | no Def for `html` (tentative) |
| p05 | `{ a, ...rest }` | Defs `a` and `rest` |
| p06 | `{ style: { color } }` | Def `color` |
| p07 | `{ list: [first] }` | no Def for `first` |
| p08 | `{ [k]: v }` | Def `v` |
| p09 | `{ a, b }` with only `a` read | Def `a` only |
| p10 | `({ user }) => {` / `return user.name;` / `}` | base Def `user` and projection edge to `user.name@2` |
| p11 | `const C = ({ html }) => { … }`; `const C: React.FC<P> = ({ html }) => …`; `({ html }: P)` | as p01 |
| p12 | `({ a } = {})`; `({ a } = other)` | Def `a`; refused (tentative) |
| p13 | `(({ a }, ref) => use(a, ref))` called as `f({ a: x }, r)` | positional edge `r → ref`; none into `a` in this slice |
| p14 | outer `const title = 1;` then inner `({ title }) => use(title)` | no edge from the outer Def into the inner use |
| p15 | `({ a }) => { a = 1; use(a); }` | the edge from the pattern Def to `use(a)` is not `exact` |
| p16 | pattern local read inside a nested arrow | same label an identifier formal gets in that position |
| p17 | function uses `arguments` or `eval`; a nested callable rebinds the local's name | the refusals identifier formals get |
| p18 | method, class method, object method, async function, generator | as p01 |
| p19 | pattern at slot 1: `(a, { b })` | Defs `a` and `b`; slot 0 still binds positionally |
| p20 | the same sources as Python and Go functions with similar names | no change (control) |

### Task T-2: object-literal argument into a pattern formal (`src/cpg/pattern_literal_arg_tests.rs`)

| Row | Shape | Required edge(s) |
|---|---|---|
| l01 | `D({ html: t })` into `function D({ html })` | `A:t(use) → D:html@1(def)`, exact |
| l02 | shorthand `D({ html })` | same, from the shorthand's value |
| l03 | `D({ other: t })` | none; `props_refused_key_not_read` + 1 |
| l04 | pattern `{ html: h }` | into Def `h` |
| l05 | pattern `{ a, ...rest }`, argument `{ a: x, b: y }` | `x → a`, `y → rest` |
| l06 | nesting, three ways: literal `{ style: { color: c } }` with pattern `{ style: { color } }`; literal `{ style: s }` with the same pattern; literal `{ style: { color: c } }` with pattern `{ style }` | `c → color`; `s → color`; `c → style` |
| l07 | `D({ ...o, html: t })`; `D({ html: t, ...o })` | binds; refused (`props_refused_overwritten`) |
| l08 | duplicate key | only the last value binds |
| l09 | computed key in the literal | keys before it refused, keys after it bind |
| l10 | callee resolves name-only, or to two targets | none |
| l11 | defaulted element `{ html = "" }` | binds |
| l12 | l01 through the trace engine (`taint_trace`, as the existing plain-call tests do) | descends into `D`; the body use is reached |
| l13 | `f(a, { k: v })` into `function f(x, { k })` | `v → k`; `a → x` unchanged |
| l14 | `D(obj)` (an identifier, not a literal) into a pattern formal | none (accepted loss; DESIGN F1) |
| l15 | pattern entry with a computed key | receives nothing (`props_refused_key_unknown`) |

### Task T-3: JSX attributes (`src/cpg/jsx_props_tests.rs`, TSX and JSX only)

| Row | Shape | Required |
|---|---|---|
| j01 | the issue's example | `App:text(use) → Html:html@1(def)`, exact; default review taint reports `dangerouslySetInnerHTML`; `taint-reaches` from `text` to the sink line is Reached |
| j02 | `<HtmlP html={text} />` into `function HtmlP(props)` reading `props.html` | `text(use) → HtmlP:props.html(use)`; no edge into `props.other` |
| j03 | arrow component; named import; default import; `<UI.Html>` | as j01 |
| j04 | `memo(function M({ html }) {…})`; `forwardRef((props, ref) => …)` with `ref={r}` | binds to the inner function's slot 0; nothing into `ref` |
| j05 | `<Html {...p} html={text} />`; `<Html html={text} {...p} />` | binds; refused (`props_refused_overwritten`); `props_spread_sources_unbound` + 1 each |
| j06 | the same attribute twice | the last binds |
| j07 | `key={k}`, `ref={r}`; a pattern that names `key` | nothing bound |
| j08 | `title="x"`; bare `disabled` | none |
| j09 | value `user.name`; `(text)`; `text as string`; `text!` | bound (member value: the full path and its base, as arguments do) |
| j10 | value is a call, conditional, template, arrow, element | none; `props_refused_value_not_data` + 1 |
| j11 | `style={{ color: c }}` into `{ style: { color } }`, `{ style }`, `props.style.color` | `c → color`; `c → style`; `c → props.style.color` |
| j12 | `aria-label={l}` into `{ 'aria-label': a }` | `l → a` |
| j13 | `xlink:href={u}` | none |
| j14 | `const M2 = React.memo(Html)`; class component | none; `props_refused_not_exact_singleton` + 1 |
| j15 | `React.lazy` loader target; component with no formals | none; `props_refused_no_props_formal` + 1 |
| j16 | `<C a={x}>{y}</C>` | `x` binds; nothing from `y` |
| j17 | element over three lines, attribute on line 2 | edge exists; trace engine descends |
| j18 | two `<D a={x} />` on one line | both edges exist; trace engine reports a boundary (pinned as today's limit) |
| j19 | `<A icon={<B x={v} />} />` | `v → B`'s `x`; nothing into `A`'s `icon` |
| j20 | `const title = a; <D title={b} />` | from `b` only |
| j21 | `{items.map(i => <D a={i.x} />)}`; `{cond && <D a={x} />}` | bound from the enclosing callable's use |
| j22 | `.jsx` file; `<C<T> x={v} />` | as j01 |
| j23 | production, legacy twin, serial reference, cache round trip, incremental rebuild | identical edge multisets and counters (extend `object_literal_parity_tests.rs`) |

- [ ] For each task: write the test file with the rows above. Rows that state new behaviour are marked
      `#[ignore = "S1"]`, `"S2"` or `"S3"` after confirming once that they fail on main (the RED record goes
      in the ledger); the slice that implements them removes the attribute. Control rows that already hold
      on main (p20, l10, l14, j08, j13) are not ignored and must stay green throughout.
- [ ] Run stage E per matrix. Give enumerators the axes: pattern syntax (every destructuring form the
      grammar allows in a formal), formal position, function kind, default kinds, scoping and shadowing,
      body writes and captures; literal key forms; JSX attribute and value syntax, element forms (member,
      generic, fragment, nested), wrappers, component definition forms, file kind. Corpus shapes from the
      census are rows.

## Stage D: design of record (1 day)

- [ ] Amend DESIGN sections 5.1 to 5.3: remove "tentative" by stating each rule against the full row set;
      list accepted losses. Both reviewers; cap 2 rounds.

---

## Stage I: slices

### Slice S1: definitions for pattern formals (4 to 5 days)

**Files:**
- Create: `src/ast/js_ts/pattern_params.rs`, `src/ast/js_ts/pattern_params_tests.rs`
- Modify: `src/ast/js_ts/mod.rs` (module line), `src/parameter_slots.rs` (occurrences and slots),
  `src/ast/parameters.rs` (if the JavaScript path needs the pattern arm), `src/cpg/callee_params.rs`
  (`PatternFormal`), `src/cpg_cache.rs` (version), docs
- Test: `src/cpg/pattern_param_tests.rs`

**Interfaces (produced):**

```rust
// src/ast/js_ts/pattern_params.rs
pub(crate) struct PatternElement { pub key: Vec<String>, pub key_known: bool, pub local: String,
    pub local_start: usize, pub local_end: usize, pub defaulted: bool, pub default_inert: bool, pub rest: bool }
pub(crate) struct PatternParam { pub slot: usize, pub elements: Vec<PatternElement> }
impl ParsedFile { pub(crate) fn js_ts_pattern_params(&self, function: &tree_sitter::Node<'_>) -> Vec<PatternParam>; }

// src/cpg/callee_params.rs
pub(super) struct PatternEntry { pub key: Vec<String>, pub key_known: bool, pub target: Option<ParamTarget>,
    pub defaulted: bool, pub rest: bool }
pub(super) struct PatternFormal { pub slot: usize, pub entries: Vec<PatternEntry> }
/// What ParamCache hands out per callee from this slice on (today it is the positional vector alone).
pub(super) struct CalleeParamFacts { pub positional: Vec<Option<ParamTarget>>, pub patterns: Vec<PatternFormal> }
```

- [ ] Unit tests for `js_ts_pattern_params` first (one per row of the table in DESIGN 5.1), RED, implement,
      GREEN.
- [ ] Extend the parameter occurrences so admitted pattern locals are occurrences, and the slot builders so
      a pattern is a hole and numbering continues. Run `pattern_param_tests.rs`: GREEN.
- [ ] Write the differential prediction, then measure X, R2 and T with `measure.sh` and
      `compare_props.py`. Expected classes: `pattern-def-intra`, `shadow-removed`,
      `positional-after-pattern`, and no `label-move` without an attributed reason. Read every
      `shadow-removed` row (or a sample of 50 if more) and confirm the pattern local shadows the old source.
- [ ] Tier-A matrix and quick runs; cloud tests at 1 and 4 threads with a RED-on-base control; two reviewers.
- [ ] PR body: the prediction, the class counts per corpus, the Tier-A result, Review focus items 1 and 2.

**Gate:** DESIGN section 7 "Slice S1".

### Slice S2: props binder with pattern targets, for object-literal arguments (3 days)

**Files:**
- Create: `src/cpg/props_binding.rs`
- Modify: `src/cpg/object_literal_args.rs` (becomes the literal source and calls the binder),
  `src/cpg/mod.rs`, `src/cpg/build.rs`, `src/cpg/callee_legacy.rs`, `src/cpg/step5b_reference.rs` (pass the
  pattern formals through), the stats type that carries `dfg_label_*` (new counters), `src/navigation/`
  stats rendering for `dfg-stats` and `call-stats` (counters; `queries.rs` is over the size ceiling, so add
  the fields where the stats struct is serialised, not new logic there), `src/cpg_cache.rs`, docs
- Test: `src/cpg/pattern_literal_arg_tests.rs`; existing `object_literal_arg*_tests.rs` unchanged and green

**Interfaces (produced):**

```rust
// src/cpg/props_binding.rs
pub(super) enum PropsValue { Leaf(std::ops::Range<usize>), Constant, Opaque }
pub(super) enum PropsEntry { Key { keys: Vec<String>, value: PropsValue }, Spread, Wildcard }
pub(super) enum PropsTarget<'a> { Plain(&'a ParamTarget), Pattern(&'a PatternFormal) }
#[derive(Default)] pub(super) struct PropsCounters { /* the counters of DESIGN 5.4 */ }
pub(super) fn bind(out: &mut Vec<PendingArgumentFlow>, counters: &mut PropsCounters,
    site: &SiteInputs<'_>, entries: &[PropsEntry], target: PropsTarget<'_>);
```

- [ ] Move pairing from `object_literal_args.rs` into `bind` with no behaviour change: the existing
      object-literal tests stay green, and a base-versus-head differential on X and T shows zero changes.
      Commit this step alone.
- [ ] Add the pattern target and the counters. `pattern_literal_arg_tests.rs` GREEN, including l12.
- [ ] Differential on X, R2, T: only `props-literal` additions. Oracle on the additions: 0 WRONG.
- [ ] Optional, when the host is free: the SecBench harness refresh described in lane 4's plan gives the
      number of benchmark traces this slice converts; report it if lane 4 has already built that tooling.
- [ ] Review focus item 5. Tier-A; cloud tests; two reviewers.

### Slice S3: JSX attribute source (3 to 4 days)

**Files:**
- Create: `src/cpg/jsx_props.rs`
- Modify: `src/cpg/build.rs` (for a site with `jsx_element`, call the JSX source before the empty-argument
  skip, then continue), the two twins, `src/cpg/mod.rs`, `src/cpg_cache.rs`, docs
- Test: `src/cpg/jsx_props_tests.rs`, `src/cpg/object_literal_parity_tests.rs`

**Interfaces (produced):**

```rust
// src/cpg/jsx_props.rs
/// Entries of the JSX element whose opening tag starts at `site_start`, in source order.
/// None when the node at that offset is not a JSX opening or self-closing element.
pub(super) fn entries(caller: &ParsedFile, site_start: usize, site_end: usize) -> Option<Vec<PropsEntry>>;
pub(super) fn collect(out: &mut Vec<PendingArgumentFlow>, counters: &mut PropsCounters,
    site: &SiteInputs<'_>, facts: &CalleeParamFacts);     // target = slot 0, plain or pattern
```

- [ ] `jsx_props_tests.rs` GREEN row group by row group: sources (j05 to j13, j16, j19 to j22), targets
      (j01 to j04, j14, j15), engines (j01's taint assertions, j17, j18), parity (j23).
- [ ] Differential on X and R2: only `props-jsx` additions. Oracle: 0 WRONG; record UNDECIDED and read 50.
- [ ] Review focus items 3 and 4. Tier-A; cloud tests; two reviewers.

**Gate:** oracle and volume rows of DESIGN section 7 (≥ 1,000 edges from JSX attribute values on X plus R2,
0 WRONG).

### Slice S4: end-to-end measurement and close-out (2 to 3 days)

**Files (evidence directory):** `g/paths-{X,R2}.md`, `g/examples/`, `g/REPORT.md`; `tools/examples.py`.

- [ ] `paths.py` on X and R2: the path table.
- [ ] `examples.py`: select 30 commits per DESIGN section 6 from the corpus history (seeded choice, seed
      `20261120`, among qualifying commits); for each, check out the commit into a scratch worktree, write
      its patch, run `prism --repo … --diff … --algorithm leftflow,fullflow --format json` with
      `prism-base` and the head binary; write the lines only the head adds, with their source text, to
      `g/examples/NN.md`.
- [ ] Dispatch the reader (Opus-5.5, high) on the 30 files with the rubric of DESIGN section 6. Tally.
- [ ] Run the census, differential, oracle and path scripts on `private-corpus`; write aggregate numbers to
      the private evidence file and only counts to the report.
- [ ] `g/REPORT.md`: gates of DESIGN section 7 with results; the counters' refusal breakdown on X and R2;
      the follow-up table F1 to F6 with the census numbers beside each threshold.
- [ ] Post the report on #596 and close it if every gate holds. File issues for: the trace-engine stop at
      plain-formal edges (F5), the attribute-name uses, and each follow-up whose threshold is met.
- [ ] Refresh the handoff; offload raw dumps; delete `target/`.

## Acceptance for the lane

Every gate of DESIGN section 7 holds on the merged head; both reviewers approved S1, S2 and S3; the report
is posted. A gate that fails is reported with its numbers and the lane stops for the owner; thresholds are
not adjusted by the orchestrator.
