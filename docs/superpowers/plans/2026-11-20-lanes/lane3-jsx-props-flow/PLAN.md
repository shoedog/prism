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
take one connected, binding-correct caller Use per leaf, and require an existing Exact singleton. A member site also proves the namespace export and each JSX or literal-pattern position needs the callable and argument-mapping proof for that site kind.

**Tech stack:** Rust (core crate: `src/ast`, `src/data_flow`, `src/cpg`), tree-sitter
JavaScript/TypeScript/TSX; Node with the pinned TypeScript 5.9.3 for the census and oracle; Python 3 for the
comparators.

**Spec:** `DESIGN.md` sections 5.1–5.4 define the rules and counters, 5.7–5.9 record losses, generators and amendments, and section 7 defines the gates. [ROW-DISPOSITIONS.md](ROW-DISPOSITIONS.md) maps the pinned 311 rows plus the 56 source fixtures in [NEW-ROWS.jsonl](NEW-ROWS.jsonl).

## Global constraints

- Worktree `~/code/prism-jsx-props`; evidence `~/prism-evidence/jsx-props/`.
- Corpora: X = Excalidraw at `0642e72c` (`~/prism-evidence/inputs/excalidraw-0642e72c/source`, no git
  history); T = `~/code/bench-repos/TypeScript/src` (regression only); G = `~/code/bench-repos/gin` (control:
  a non-JavaScript corpus must show no change); R2 = cal.com, chosen in Task M-2 (full
  history); `private-corpus` locally under ORCHESTRATION section 9.
- TypeScript for scripts: `~/prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js`.
- Edges only for a site that passes the target admission of DESIGN 5.3.
- **Not changed by this lane:** `function_parameter_occurrences`, `function_parameter_names`,
  `function_parameter_slots`, `function_parameter_slot_occurrences`, the slot builders in
  `src/parameter_slots.rs`, `compute_param_def_nodes` and its return type, the reasoning layer's seeds, any
  taint code, call resolution outcomes (DESIGN 5.5's read-only export query for admission check 2 is the
  one addition to `src/resolution/`), `src/cpg/trace.rs` (PR #841 is editing it).
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
  with a list line and updates all four independent numeric pins (three in `src/cpg_cache/dfg_codec_tests.rs`, one in `src/cpg_cache.rs::cache_versions_are_pinned_for_cpg_semantics`). S1, S2b and S3 each run `cargo test --lib cache_versions_are_pinned_for_cpg_semantics` and `cargo test --lib dfg_codec_tests` for owned/streamed byte equality, activation and stale-version misses. S1, S2b and S3 each verify cold/partial Step 5b assembly against serialized full-hit stats and edges at 1 and 4 threads. Each also updates the React and data-flow entries of
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
6. Verify final connectivity separately from source admission. In the following positive controls the source Def still reaches the edge: `title={title}`, guarded elements, one value
   in two attributes, `key` beside a bound attribute (rows j25 to j28, `path` assertions). (S3)
7. Admission: a member tag through an imported value, a namespace member the module does not export, and a
   component behind a non-React wrapper bind nothing and are counted (rows j34, j39, j36). (S3)

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
      tag, the qualifier's kind (namespace import whose module exports the member as the target, namespace
      import without that export, named import, default import, local, other); the target's
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
- [ ] Stage M source outcome per Leaf value, using its original first-wins line-index and incoming-endpoint lookup (DESIGN §6 Census): with a warmed nav cache for the corpus (one
      `prism-base nav call-stats` run without `--no-cache` first), run `prism-base nav nodes-at --repo
      <corpus> --location <file>:<line> --format json` once per distinct value line and look for a Use node
      of the value's path whose function is the site's caller (by name, and by start line where the output
      gives it) (`connected` when a `to`
      endpoint of `PRISM_EDGES_BYTES=1 prism-base nav dfg-stats --edges` has that node's file, line, path,
      access and bytes; otherwise `unconnected`); failing that, a Use node of that path in a function whose
      span encloses the caller (`in_enclosing_owner`); otherwise `not_found`.
- [ ] A **predicted connected bindable pair** is one (attribute or object-literal leaf, target) such that:
      the site is an exact singleton; a member tag's qualifier is a namespace import whose module exports
      the member as the target; the route is direct or an admitted React wrapper; the value is an
      identifier or member path (each leaf of an object-literal value counts once); its source outcome is
      `connected`; the attribute or leaf is the last writer of its key under DESIGN 5.2 (no later spread,
      computed key or attribute of the same name, the parent attribute included for a nested leaf; not
      `key` or `ref`; a `children` attribute only on an element with no child content); and the target's first formal is a pattern (not alias-skipped) with an entry whose key path
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
`tools/oracle_pattern_defs.cjs`, `tools/undecided_sample.py`, `tools/paths.py`, `tools/measure.sh` and `tools/selftest/` (the independent TypeScript model, fixtures and poison controls).

- [ ] `measure.sh <binary> <corpus> <outdir>`: runs, through memguard, `nav --no-cache dfg-stats`,
      `nav --no-cache call-stats`, and `PRISM_EDGES_BYTES=1 nav --no-cache dfg-stats --edges`.
- [ ] Supply the byte-keyed input to oracle check 3. In the measurement build, capture the immutable variable table, index insertion order, concrete incoming DataFlow edges/labels and owner IDs immediately before `collect_step5b_edges_with_exact` in `src/cpg/build.rs`. Extend the measurement-only exporter in both base and head. Emit the node dictionary with run ID and concrete index, access, owner ID and canonical declaration bytes, structured path, line and occurrence bytes. Snapshot additions to the dictionary through Step 5c. Export pre-guard and **final** DataFlow edge multisets as `(from_id,to_id,edge_kind,label,doubt,kill_line)` from the same graph, with pre-guard edge occurrence IDs, guard seeds, refused formal IDs and removed occurrences. Retain earlier IDs on surviving nodes. Bind the sidecar, phase records and `call-stats --dump-sites` by capture ID, product/build identity and corpus digest; require a unique dictionary row for every endpoint. The public geometry-only dump alone cannot reconstitute those identities. Compare instrumented and ordinary builds’ final edges and stats. Use the same observation point in the test adapter. Do not infer pre-step connectivity by deleting cross-owner edges from the finished graph; that would retain recursive argument edges.
- [ ] `compare_props.py <base dir> <head dir>`: joins final node IDs to the same-run sidecar/dictionary first. Across builds key each endpoint by corpus-relative file, canonical owner (kind/name and callable bytes, including synthetic identity), Def/Use access, structured path, line and occurrence bytes. Compare labeled multisets, preserving parallel edges; reject unresolved owners or duplicate canonical identities as inadmissible. Self-test same-line zero-width nodes of the same spelling in different owners, Def/Use at identical geometry, and renumbered IDs in the second build. An owner/access swap must fail comparison and an ID renumber alone must pass;
      classes: `pattern-def-intra` (a new edge that starts at a Def absent from the base, inside one
      function), `props-literal` (an added edge with an object-literal argument witness, including a recursive site in the same owner), `props-jsx` (the same for a JSX element site), `removed` and
      `label-move` (unexpected removals and moves listed in full), separately `legacy-unproved-read`, `guard-removed` and `guard-restored` only with the stage-specific witnesses below, and `s1-one-node-relabel` (an existing edge whose
      label moves from `exact` to `cfg_incomplete` in a one-node owner that gains a pattern Def written or
      updated in that owner: the expected change class of DESIGN section 7 "Slice S1", listed in full and
      counted; any other label move stays `label-move`); prints every counter delta of both stats files;
      exits 1 on an unclassified change. Model: `~/prism-evidence/jsts-lane/783/scripts/compare761.py`.
- [ ] `oracle_props_edges.cjs <corpus> <added-edges.jsonl>` and its fixture suite implement DESIGN §6 checks 1–3 on actual TS source and byte-keyed endpoints. Recover candidates from the syntax and dump, including same-owner calls. Preserve every valid witness with site/position, actual operand and selected bytes, source path, target binding and key restriction. Test the following **positive and deliberately WRONG** controls before S2b or S3 uses the oracle. A fixture expectation cannot become a corpus witness.

| Witness | CORRECT control | WRONG control |
|---|---|---|
| Target identity / position | Direct, stable renamed import/alias, proven namespace export (including an exported expando if its value is the selected callable), JSX wrapper operand zero, local literal-call React hook | Foreign unbound singleton, inaccessible declaration, shadowed hook, rewriting or forwarding factory, comparator, unexported or unrelated same-spelled member, wrong runtime argument, prior argument spread |
| Pattern | Equal key and renamed local; shorter value path into nested local; deeper value into container | Wrong local bytes, divergent segment, dotted spelling split into two segments, deeper leaf sent to a sibling container. In `paths.py` separately reject a chain leaving the correct container through a sibling |
| Rest | Unconsumed top-level and nested residual, sibling outside nested wildcard | Consumed key (including a refused/unused sibling), parent-only value, unknown sibling, wrong residual suffix |
| Plain target | Reached matching member at exact bytes, mixed base Use with full dot syntax, genuine nonzero bare Use, legacy literal bracket/odd-key projection compared with base | Wrong sibling/member, suppressed member write, a label stronger than the reached read, zero-width bare, JSX bracket or exposing a rich-only key in the legacy projection; a selected real shadow beside a genuine bare or member read in the same environment/line inventory |
| Literal source | Shorthand and explicit value, both full and base endpoints in the bytes of `D({a:user.name})`, a disconnected exact Use | Wrong byte occurrence elsewhere on the same line, path from a different root or intermediate prefix |
| JSX source | Own full, proved base with dead full, zero-width or earlier representative (including a name token), transparent/optional path, recursive edge | Disconnected/outer source, wrong shadow/version, base despite live or conflicting full, wrong attribute or wrong operand line |
| Writer and metadata | `key/ref` in a literal and inside a nested JSX object, explicit JSX children without content, final duplicate, independent nested site | Top-level JSX `key/ref`, children with content, displaced duplicate/subtree/leaf or foreign-site operand, later spread |
| Dedup and verdict | One edge with two valid restrictions, plus a discarded invalid candidate; unresolved external symbol → UNDECIDED | No viable witness for the endpoint → WRONG; malformed probe → INADMISSIBLE |

- [ ] For S2b/S3 replay the post-Step-5c guard before and after toggling only the identified slice hops, preserving all other edges, seed flags and parallel witnesses. Restore a suppressed legacy edge’s own seed flag with that occurrence where applicable. Reconcile each newly exposed D12 formal and outgoing removal (or restoration for a lost last seed path), confidence tallies and affected paths to the final export per DESIGN §6. Exercise all four `d2-guard-*` rows and their imported controls. A pre-existing exposure, a non-seed or a missing counterfactual proof cannot classify a removal.
- [ ] Test the §10 React replacement as an `OUT_OF_MODEL` runtime probe for memo, forwardRef and literal useCallback. Cover direct/computed/aliased/reflective writes. Keep the runtime disposition distinct from the static edge verdict; incomplete static provenance is UNDECIDED, tool failure is INADMISSIBLE. A shadowed import or wrong wrapper operand is still an in-model negative.
- [ ] `oracle_pattern_defs.cjs` checks the checker symbol at Def bytes and the selected destination bytes (or the complete represented-read set for a line anchor). Manufacture a wrong first endpoint beside a valid same-line outer read. Pair a uniformly bound zero-width/name-token representative with mixed anchor and distinct-block/loop/switch/catch/default controls. Check projection roots and captured reads; retain the 30 written-local exact-edge sample. Exercise each wrong control by poisoning only that endpoint, binding, writer or position.
- [ ] For the B-2 endpoint audit cross the five `d1-selected-endpoint` lexical environments (block, loop, switch, catch, parameter/body) with pattern Def/selected reference and plain projected-member, mixed-base-member and nonzero bare Use endpoints. For both literal and JSX transport mark the formal and shadow bytes, alternate which is selected, verify the actual reached node and poison the oracle by swapping only the selected endpoint. Keep #594’s bracket-as-bare control for a proved literal root. Record non-materialized product shapes explicitly and exercise the binding classifier on the parsed occurrences separately. For `d2-{literal,jsx}-selected-read`, first witness the inherited pre-Step-5b `P:p@1(def)` edge to the shadow Use marked by `to_bytes` in each cross-owner `no_edge` check. Report a missing inherited edge as an unmet fixture precondition. Grade the props edge, refusal and independently valid reads on the resulting graph.
- [ ] `undecided_sample.py <verdicts> <n> <seed>`: the seeded sample of UNDECIDED rows for the hand read.
- [ ] `paths.py <head edges> <oracle verdicts> <corpus>`: the end-to-end path table and the connected share
      of DESIGN section 6 from the final post-guard edge IDs (resolve the value's root binding with the checker; no credit for a chain that
      changes key inside a container or rest local, DESIGN 5.3).

---

## Stage T and E: test matrix (3.5 to 4 days)

### Task T-1: matrix data and one data-driven test

**Files:**
- Create: `tests/fixtures/props_matrix/rows.jsonl`, `tests/fixtures/props_matrix/README.md`
- Create: `src/cpg/props_matrix_tests.rs` (declared with `#[cfg(test)] mod` in `src/cpg.rs`)

Each row: `id`, `group` (`control`, `S1`, `S2`, `S3`), `langs` (which of `js`, `ts`, `tsx`, `jsx`), `files`
(name → source text), `expect` (a list of assertions), `class`, `truth`, `base` (`red` when the row states
behaviour main does not have, `green` when its expectations already hold on main, as for p04, p07, l10, l14
and j09). Evaluate assertions using the helpers in `src/cpg/object_literal_arg_tests.rs` (`build`, node spelling
`function:path@line`) plus a traversal of the built graph's data-flow edges: `def`, `no_def`, `edge`
(optionally with a label), `no_edge`, `no_edge_into`, `path` and `no_path` (a chain of data-flow edges from
one named node to another, both spelled `function:path@line` with `(def)` or `(use)`), `counter` (a key of
the `dfg_props` map read from the serialised statistics, with absent keys read as 0), `s1_counter` (a
registered S1 evidence key with an observed integer count, including zero), `s1_ledger` (a count and
terminal disposition for a byte-keyed S1 candidate), `trace_reaches`,
`trace_misses` and `trace_boundary` (through the trace engine, as the existing plain-call trace tests do;
each names its seed line and its target line).

An `edge` with `count` requires exactly that many DataFlow edge instances matching the endpoints and
any specified byte spans. With no `label` filter, count across all labels, including parallel edges.
With an explicit `label`, count only that label. Pair `count: 1` without a label with a separate
labelled assertion to pin one edge of the required label, as in `d2-default-capture`.
For `s1_counter`, a missing registration or a missing or noninteger observed count fails even when the
expectation is zero.
For `s1_ledger`, match `binding_bytes` and `operand_bytes` in the selected source, case and grammar
against the candidate ledger before graph deduplication. Require the exact `count` of matching entries
across all dispositions and require each matched entry's terminal `disposition` to equal the specified value.

**Binding rows assert reachability from the definition.** Join the path through the concrete connected caller Use. For a pattern target assert descent only where DESIGN 5.3.5 admits it. Pin cross-function spelling/site boundaries and the same-owner self-parameter boundary separately. Test a refused leaf with no incoming props edge and a negative path from an existing seed.

`props_matrix_tests.rs` holds one test, `props_matrix_rows`, with a constant
`ENABLED: &[&str] = &["control"]`. It runs the rows of enabled groups and skips the rest. On main the
control rows pass. For every row with `base: red` the implementer records, once, that it fails when its
group is enabled by hand (the RED record, in the ledger); a `base: green` row is recorded passing in the same
run, as a preservation row. The branch stays green throughout: only enabled groups run in CI. A seeded row whose stated premise about main proves false in
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
| p15 | `({ a }) => { a = 1; return use(a); }` | no entry edge to the final read; line-2 assignment Def → final Use, exact (identifier twin) |
| p16 | pattern local read inside a nested arrow | the label an identifier formal gets in that position |
| p17 | the function reads its own `arguments`; the function calls `eval` | a read of the owner's `arguments` makes the local mixed, with identifier-twin base admission and labels and no projection; retain the identifier twin's `eval` behavior |
| p18 | method, class method, object method, async function, generator | as p01 |
| p19 | `function f(a, { b }, c) { … }` | Defs `a` and `b`; `a` and `c` exactly as on main |
| p20 | `(x = 1, { b }) => { … }` | `b` gets a Def; `x` exactly as on main |
| p21 | one-line owner `const C = ({ a, b }) => use(a, b);` | Defs `a` and `b`; edges present, label not asserted |
| p22 | `function C({ html }) {` / `function html() {}` / `return use(html);` / `}` (a hoisted function replaces the formal) | the edge from `C:html@1` into `use(html)` is `killed`, as the identifier twin `function C(html)` gets |
| p23 | `function C({ a }, b = (a = 1)) {` / `return use(a);` / `}` | formal-bytes edge into the body read is the identifier twin’s `cfg_incomplete` in js, ts and tsx; locate the two line-1 Defs by bytes |
| p24 | `function C({ o }, x = (o.k = 1)) {` / `return use(o.k);` / `}` | the edge from `C:o@1` to `o.k` gets what the identifier twin `(o, x = (o.k = 1))` gets (the own-parameter member-write refusal) |
| p25 | `function C({ a = 1 }) {` / `var a;` / `return use(a);` / `}`; the same as an anonymous function passed to a call | as the identifier twin `(a = 1)` with `var a`, named and anonymous |
| p26 | `function L({ id }) {` / `return xs.map(({ id }) => {` / `return use(id);` / `});` / `}` | no edge from `L:id@1` into the inner use; the inner arrow's pattern Def feeds it |
| p27 | `function C({ a }) {` / `run(() => { a = 2; });` / `return use(a);` / `}` | entry Def → final Use `killed`; captured-write Def → final Use `cfg_incomplete` in js, ts and tsx |
| p28 | `{ a: { c } = make() }`; `{ a: { c } = other }`; `{ a: { c } = {} }` | no Def for `c`; no Def for `c`; Def `c` |
| p29 | `function f({ a } = outer) {` / `return use(a);` / `}` with `outer` an outer variable, not a formal | exactly the Defs main has today (the alias code's `a` and its twin); no second Def at the same bytes |
| p30 | `({ a } = make())` | no Def for `a` |
| p31 | `function C({` / `a,` / `b = a,` / `}) {` / `return use(a, b);` / `}` | Def `C:a@1` with the bytes on line 2; no Def for `b`; the read of `a` on line 3 gives what it gives for the identifier twin `function C(` / `a,` / `b = a,` / `) {…}` |
| p32 | `const html = ({ html }) => {` / `return use(html);` / `};` | Def of the pattern local `html` in the arrow owner named `html`, and its edge into `use(html)` |
| p33 | `const One = ({ title }) => <D title={title} />;` | Def `One:title@1`; outgoing edges not asserted here (row j31) |
| p34 | `function C({ a }: P = {} as P) {` / `return use(a);` / `}` | Def `a` (assertion wrappers around the default are removed before the inert test) |
| p35 | `function C({ p }, safe) {` / `return report(++p, safe, p);` / `}` (one-node owner) | Def `C:p@1`; the existing sibling edge `C:safe@1 → C:safe@2` is `cfg_incomplete` (`exact` on main), as the identifier twin `function C(p, safe)` gets today; the differential classes it `s1-one-node-relabel` (DESIGN 10) |

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
| l22 | `import * as api from "./ctx"; api.send({ a: x })` where `ctx` has no export `send` but holds `export const Other = { send: ({ a }) => use(a) }`; the same with an unexported `function send({ a })` in `ctx` | none, `refused_member_not_export` = 1 each |
| l23 | rest residuals: `D({ title: t, b: y })` into `{ [k]: v, ...rest }`; `D({ list: x, b: y })` into `{ list: [first], ...rest }`; `D({ a: x, b: y })` into `{ a: aa, ...rest }`; `D({ style: { color: c, size: s }, title: t })` into `{ style: { color, ...r }, title }` | nothing into `v` or `rest`; `y → rest` only; `x → aa` and `y → rest` only; `c → color`, `s → r`, `t → title`, nothing else into `r` |
| l24 | `D({ style: { color: c, size: s } })` into `function D({ style }) {` / `return draw(style.size);` / `}` | `c → D:style@1(def)` and `s → D:style@1(def)`; `edges_to_container` = 2; the `path` from `c` to `D:style.size@2(use)` exists (accepted limit, DESIGN 5.3, pinned) |

**Group S3 (JSX attributes in TSX and JavaScript with JSX syntax, including .js):**

| Row | Shape | Required |
|---|---|---|
| j01 | the issue's example, element and component on separate lines | `App:text(use) → Html:html@1(def)`, exact; `trace_reaches` from `text`'s definition to the `dangerouslySetInnerHTML` line |
| j02 | `<HtmlP html={text} />` into `function HtmlP(props)` reading `props.html` and `props.other` | edge into `HtmlP:props.html(use)`; no edge into `props.other` |
| j03 | arrow component; named import `{ Html }`; default import `Html` of a file whose default export is `function Html`; `<UI.Html>` with `import * as UI` | as j01 (in each form the site's recorded callee name equals the function's name; forms where they differ are j35) |
| j04 | `memo(function M({ html }) {…})`; `forwardRef((props, ref) => …)` with `ref={r}` | binds to the inner function's first formal; nothing into `ref` |
| j05 | `const C = memo(({ html }) => {…})` (anonymous inner function) | bind and trace through the admitted anonymous first-argument function’s pattern Def, located by callable and local bytes |
| j06 | `<Html {...p} html={text} />`; `<Html html={text} {...p} />` | binds; none with `refused_overwritten` = 1; `spread_sources_unbound` = 1 each |
| j07 | the same attribute twice | the last binds |
| j08 | `key={k}`, `ref={r}`; a pattern that names `key` | nothing bound |
| j09 | `title="x"`; bare `disabled` | none (control) |
| j10 | value `user.name`; `(text)`; `text as string`; `text!` | member: use the connected full `user.name` node once, with a path from `user`’s Def; no extra base edge. Parentheses, `as` and `!` bind their operand |
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
| j25 | attribute named like its value, in a function declaration, a const-bound block arrow and a method: `function Outer({ title }) {` / `return <Inner title={title} />;` / `}`; the same with `const title = get();` in the body; the same with the element over several lines | `path` from the value's definition (`Outer:title@1(def)` or the local's Def) to `Inner:title@1(def)`; `trace_reaches` from that definition's line to `Inner`'s use; the props edge leaves the very graph node at which the def-use edge from that definition ends (on main the controls `function Named(html)` and `function Alias({ html } = fallback)` returning `<Other html={html} />` show, under `PRISM_EDGES_BYTES=1`, that this node has the attribute name's bytes), so the row joins by node, not by `function:path@line` spelling |
| j26 | in an owner with a branch: `{user && <Avatar user={user} />}`; `cond ? <D a={x} /> : <E a={x} />` under `return` | each binding has a `path` from the value's definition; both `D` and `E` are bound |
| j27 | in an owner with a branch: `<D a={x} b={x} />` | `x → D:a`, `x → D:b`; a `path` from `x`'s definition to each |
| j28 | `{items.map((it) => <Row key={it.id} item={it} />)}`; the same with `item={it}` before `key` | `it → Row:item`, with a `path` from the arrow's formal `it` to `Row:item@…(def)`; nothing from `key` |
| j29 | element in a declarator: `const el = <D title={title} />;`; the multi-line `const a = (` / `<D` / `title={title}` / `/>` / `);`; an assignment `el = <D title={title} />;` | bound, with a `path` from `title`'s definition |
| j30 | multi-line expression-bodied arrow: `const Card = (item) => (` / `<div>` / `<Row item={item} />` / `</div>` / `);` | bound, with a `path` from `Card:item@1(def)` |
| j31 | one-line expression-bodied arrow: `const One = ({ title }) => <D title={title} />;` | keep the S1 Def, no props edge/path/descent; `refused_source_not_found` = 1 |
| j32 | element as an object-literal value, a conditional branch and a JSX child, each in a declarator: `const o = { icon: <D title={title} /> };`; `const g = c ? <D title={title} /> : null;`; `const h = <div><D title={title} /></div>;` | bound, with a `path` from `title`'s definition |
| j33 | a value captured from the enclosing component inside a callback: `{items.map((it) => <Row onPick={onPick} />)}`; the block-bodied `{items.map((it) => {` / `return <Row onPick={onPick} />;` / `})}`, with `onPick` a local or prop of the component | no props edge/path/descent in either case; `refused_source_in_enclosing_owner` = 1 in expression, `refused_source_unconnected` = 1 in block |
| j34 | member tags through an imported value: `import { Ctx } from "./ctx"; <Ctx.Provider value={a}>` with `ctx` declaring `export function Provider({ value })`; `import { Menu } from "./table"; <Menu.Item html={a} />` with `table` holding `export const Menu = { Item: Impl }` and an unrelated function `Item`; the same through a default import; `<Tabs.Tab html={a} />` with `Tabs.Tab = function TabImpl(…)` and an unrelated `export function Tab` | no edge; `refused_member_qualifier` = 1 each |
| j35 | tag spelling differs from the function's name: `import { Html as RichText } from "./ui"; <RichText html={text} />`; `import View from "./ui"` with `export default function Html`; `const Button = memo(function ButtonInner({ label }) {…}); <Button label={t} />` | the edge into the target's pattern Def exists with a `path` from the value's definition; `trace_boundary` (DESIGN 5.6 limit) |
| j36 | wrappers: `const C = observer(({ a }) => {…})` with `observer` imported from a package; `styled(({ className }) => …)`; `withRouter(…)`; `memo(observer(fn))`; `export default observer(function X({ a }) {…})`; `const M = React.memo(function M2({ html }) {…})` with `import React from "react"` | observer, styled and withRouter Exact fallbacks count `refused_unknown_wrapper` = 1 each; nested and default-export have no target and count `refused_not_exact_singleton` = 1 each. The admitted React default-import wrapper binds with the name-mismatch trace boundary |
| j37 | one-line component: `function Card({ title, body }) { return <div title={title} dangerouslySetInnerHTML={{ __html: body }} />; }` with `<Card title={t} body={safe} />`, `t` from a source; the same component over several lines | one-line: `trace_reaches` from `t`'s definition to the sink line (accepted limit, DESIGN 5.6); several lines: `trace_misses` for the sink line |
| j38 | `<C children={x}>{y}</C>`; `<C children={x} />` | nothing from `x`, `refused_overwritten` = 1; `x → C:children` |
| j39 | `import * as UI from "./table"; <UI.Item html={a} />` where `table` has no export `Item` but holds `export const Other = { Item: ({ html }) => … }`; the same with an unexported `function Item({ html })` in `table` | no edge; `refused_member_not_export` = 1 each (contrast: j03's `<UI.Html>` binds) |
| j40 | `<C style={{ color: x }} style={{ size: y }} />` and `<C style={{ color: x }} style={s()} />` into `function C({ style: { color, size } })` and into `function C({ style })` | only `y → size` and `y → style` in the first; nothing from `x` in either, `refused_overwritten` = 1 for `x` each |

The tables above describe the seed cases. Every enumerator row and amended expectation, including mixed cases and counters, is in [ROW-DISPOSITIONS.md](ROW-DISPOSITIONS.md). The pin contains 11 control, 62 S1, 69 S2 and 169 S3 rows. Stage D round 1 added 1 S1, 9 S2 and 3 S3. Stage D round 2 added 1 S1, 6 S2 and 7 S3. The Stage D subtotal is 338 (11 control / 64 S1 / 84 S2 / 179 S3). The first S1 rule amendment added nine rows. The entry-projection amendment added eleven, and its round-1 and round-2 folds added four and five respectively, for 367 total (11 control / 93 S1 / 84 S2 / 179 S3). The §5.1.2 primary-rule count is 75.

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
- [ ] Deliver the applicability table and cell manifest in DESIGN §5.8. Expand each of the eight owner columns explicitly for each slot. Record generated family/dimensions, exact pinned ID/variant or exclusion and reason. Report per-family potential, inapplicable, applicable, covered, missing and failed counts. The direct context matrix has 261/328 applicable; the region panel covers specified class/accessor/generator contexts separately. Mark deeper ungenerated interactions as exclusions. Empty-pattern neighbors with no designated local are inapplicable. A pinned case has only its recorded owner. Do not count a parse-only program as covered.
- [ ] For each new row, record in the row's `class` the pre-D prediction for whether the original line-index lookup finds a caller Use
      (a node of the caller under the line key exists on main, by `nav nodes-at`) or not. Rows the rule does
      not cover go to stage D as decisions (fix in this lane, or accepted loss with its census count).

## Stage D: design of record (1 to 1.5 days)

- [x] Record the decisions in DESIGN 5.1–5.9 and the dispositions of all 311 pinned rows and 56 additions. E closed with two open classes; use the generators in 5.8.
- [ ] Before each enabling slice, import its `add` lines verbatim from `NEW-ROWS.jsonl`, apply the pinned `change` instructions, validate unique IDs/cases and record fresh observed RED/GREEN receipts on the same base (ORCHESTRATION 7). Preserve the prior receipts as historical evidence. The supplied 06157400a run reports 272 failures and 39 passes. p13, p29, p30, j08, j14, terra-1-09 and terra-1-10 passed despite red seeds. Correct their base metadata independently of behavioral disposition. A passed negative is a preservation check.
- [x] In round 3 at `fa6212af`, reviewer A approved and reviewer B rejected the same-line field-edge precondition. The amended fixtures require the lookup, hidden-full and mutant checks in S3.

**PR #925 S1 rule fold verification.** The 347 ledger IDs match the 311 pinned rows and 36 additions in order. The nine new rows passed 74 TypeScript 5.9.3 source/grammar parses and 12 JavaScript runtime controls. The reviewed generator at `0085a4e4` reproduced its committed fixture byte for byte with `node tests/props_matrix_gen/props_generated.cjs --check`. All 576 marked reads and 232 expected sinks were inside the target bodies. A byte-bearing `nav --no-cache dfg-stats --edges` export from pre-S1 `4138d8b9f4af` confirmed 12 mixed-line legacy edges at the signature point and 12 wholly body-line preservation edges across JS, JSX, TS and TSX. Amended producer graphs and corpus acceptance await S1.

**Round-3 fixture verification.** TypeScript 5.9.3 parsed all 44 source/extension instances of the six amended rows and all 24 multiline panel cells. Its checker distinguished the selected shadow from the formal in all ten selected-read shadow instances. The eight multiline negatives had the marked declaration or intervening write, and all eight twins moved the competing read beyond the operand lines. With input `{a: 17}`, Node returned `[0,17]` for the separate body `var` and `0` for the write to the captured parameter. Native lookup, graph labels, mutant edges and corpus counts await the implementation slices.

**Round-2 design verification at `fa6212af` (before the round-3 fixture edits).** The 338 ledger IDs match the pinned 311 followed by all 27 additions in order. The 14 new rows passed 99 tree-sitter source-file parses across 79 case/grammar instances (JavaScript grammar 0.23.1, TypeScript/TSX 0.23.2). All 76 explicit endpoint byte checks across those instances matched spelling and line. The 3,840 core, 4,000 collision and 24 multiline programs also parsed in memory. These are syntax receipts. The native graph preconditions and the independent TypeScript 5.9.3 outcomes still require slice execution. Record fresh receipts for the amended multiline, selected-read and default-capture cases, the paired baselines and all additions on their slice bases. The 9,788 comparisons in §5.8 remain planned.

**Supplied round-1 fold receipt at c4d90d0f (original d1 sources).** All 324 ledger IDs matched pin order followed by addition order. A read-only adaptation of the pinned row evaluator used the existing lane `libprism` archive (SHA-256 `50fdcaa4b62c9dedd0973106d8948fd0480c09bc04b8274649d908022866796a`). The 11 original controls passed. All 13 new rows parsed and built in all 73 case/extension instances and failed their planned edge or counter expectations on that pre-implementation graph. The same library produced the p23/p27 labels in all six identifier twins. The existing lane test binary passed all six `cpg_cache::dfg_codec_tests`, including owned/streamed parity and stale-version miss. The future cache versions still require these checks. Sources for all seven §5.8 panels parsed and produced nonempty native graphs (1,920 + 176 + 16 + 2,048 + 96 + 576 + 80). These baseline checks used an existing archive without a fresh build at the reviewed revision. Exact-base slice REDs, independent model comparisons on implementation heads, oracle verdicts and corpus gates require the planned slice runs.

---

## Stage I: slices

### Slice S1: definitions for pattern formals (4.5 to 5 days)

**Files:**
- Reuse the reviewed `src/cpg/props_generated_tests.rs` registration in `src/cpg.rs` and the §5.8 generator/fixture pair at `tests/props_matrix_gen/{props_generated.cjs,generated-cases.jsonl}` on `0085a4e4`.
- Create: `src/ast/js_ts/pattern_params.rs`, `src/ast/js_ts/pattern_params_tests.rs` and the shared AST key/wrapper helper (`src/ast/js_ts/props_keys.rs`). Expose `is_inert_default_value` in `src/parameter_slots.rs` with its existing callers’ behavior.
- Add focused `pattern_*` modules under `src/data_flow/build/` for the new walks and `get_use` narrowing, including the replay routing. Keep registration and calls in `file.rs`, already at 1,000 weighted lines on the reviewed code. Split the new responsibilities away from `pattern_formals.rs` and its tests, and account for any file left above 1,000 weighted lines.
- Modify: `src/ast/js_ts.rs` (module lines), `src/data_flow/build/file.rs` (register Defs for admitted
  pattern locals in the loop that handles `function_parameter_occurrences` and pass admitted binding and occurrence bytes through endpoint selection, both count/emit passes and member projection per 5.1.2),
  `src/cpg/props_matrix_tests.rs` (`ENABLED` gains `"S1"`), `src/cpg_cache.rs` (version, history and independent semantic pin), `src/cpg_cache/dfg_codec_tests.rs` (three version assertions), docs
- Tests to re-run and, where they compare the live producer with the test-only legacy producer
  (`src/data_flow/build/legacy_producer.rs`), to adjust by excluding owners that have a pattern formal, each
  exclusion named in the PR: find them with `rg -l "file_refs_frozen" src/` (the command's list is
  authoritative; on the reviewed product it names 16 files, including the producer and test support, under `src/data_flow/build/`, among them
  `js_prune_tests.rs`, `prune_cap_tests.rs`, `lexical_prune_tests.rs`, `scope_prune_tests.rs`,
  `order_prune_tests.rs` and the C, C++, Rust and Python prune tests). Also run
  `parameter_slots_parity_tests`, `js_param_defs_tests`, `inert_default_parameter_tests`,
  `member_only_param_tests`; none should change.
- Re-run `cpg::required_parameter_tests::unsupported_parameters_are_not_introduced_and_do_not_compress_slots`. Apply and report the `optional` and `field` Def assertion changes and the `third` → `c` slot control specified in DESIGN §7.
- In `src/cpg/step5c.rs` and `src/navigation/queries.rs`, add an env-gated trace in the style of existing `PRISM_*` traces or a test-only exporter. Emit one byte-keyed line per Step 5c return input and one per return-flow suppression transition with counter name and old and new reason. In `src/cpg/build.rs` Step 5b, add an env-gated trace at ordinary argument-slot selection where arguments are paired with `offsets.slot`. Emit the line for the TypeScript argument-binding edge there with the source Use, selected argument slot and callee Def, before enqueueing the edge. Keep the output outside the stored graph. The instrumentation requires no graph or cache change.

**Interfaces (produced):**

Produce per-position pattern facts under DESIGN 5.1. Preserve empty/alias-skipped positions. Keep the per-object consumed-key set and unknown-sibling flag even for empty, unused, excluded or array-valued children. Each element carries its structured path or unknown status, a rest-level exclusion predicate if applicable, local name and identifier bytes, default state and admission. Index positions excluding TypeScript's `this`. Put shared key and erased-wrapper classification in the AST layer. Expose the existing inert-default predicate and alias accessor internally where needed. Pair the facts with the exact callable syntax node.

`ParsedFile::js_ts_pattern_params(&Node) -> Vec<PatternParam>` returns these facts. In the data-flow adapter classify each operand by the owner child its path enters. Fence descendant parameter lists and parameter decorators at any depth on body and formals paths. For the owner's own list implement §5.1.2's whole-list proof, ordered bare defaults/keys and final literal reader. Pin both forms to `cfg_incomplete` when emitted. Only the reader can materialize a capture Use on a named signature with no preferred Use. Inventory other owner-child paths as other-binding, including associated TS and TSX sibling decorators. Preserve ordinary parameter/slot producers and identifier twins. Carry exact binding, operand, environment and path through mixed-line selection, both cap passes and graph interning.

Implement M1 with a binding-aware occurrence walk over the own list and body, including all nested callables and refused occurrences. Treat runtime value roles other than static member reads as bare, including dynamic subscripts. Attribute `arguments` reads through arrows to their lexical owner, stopping at an ordinary function or a shadow. A mixed local uses identifier-twin base behavior only. Use DESIGN §5.1.2's W1 test for every owner member/element store, update or delete, including nested callables, pattern and loop targets and compound/logical writes. Accept an empty census. Otherwise every store must have the same root by binding identity and a proved same, prefix or descendant path relative to the projected read. Apply the existing overlap treatment there. Withhold projections for any other root or non-comparable path. A `with` statement withholds every projection in its owner. For S, check evaluated signature initializers and computed keys from the binding's initialization point, including coercion of computed object-literal keys. Require a proved declarative binding for each identifier under §5.1.2, with the three built-in exceptions, and withhold for any lookup through a `with` scope or in an owner nested in a `with` body. Preserve the existing `spread_element` veto for every spread, including literals. Account for implicit effects even without an expression. Apply S2 for an array binding pattern, object rest or rest formal anywhere in the owner's list, including the post-initialization sequence of iterator acquisition, stepping and close, property `Get`, and rest enumeration and copying. W1, `with`, S and S2 use `write_effect` and emit the base read a mixed local gets, at that label, when admitted. Keep the established p23/p24 handling, including p24's `cfg_incomplete` for the same-root signature store. These walks are independent of signature-read admission and endpoint availability.

Narrow `get_use` in the augmented producer so only a witnessed mixed-line signature-closure point relocates to the proved body operand at the same full label. Apply the test to preferred and fallback selections for raw writes and alias twins. Preserve wholly-body, version-only and scope-only endpoints. Invoke pre-S1 routing on both cap-replay paths.

**Refusal accounting.** Produce an S1 evidence ledger on the actual decision path. Inventory candidate value operands by normalized spelling against each proposed eligible binding, including candidates that produce no Def or edge. Key an attempt by revision, file, grammar, binding owner span, binding bytes, operand bytes and normalized path. Non-value occurrences, including write-only targets, are outside the read denominator. Assign one terminal disposition per candidate. Outside the owner's formals and body use other-binding. On the two applicable paths, apply ancestry and syntax before binding. A syntactically refused candidate gets that refusal even if its independent binding verdict proves a shadow. Attach the verdict separately. For the owner's signature, the first failing whole-list check wins, including a sibling failure. Record whether the read justifies a Def separately. A read refused at admission cannot seed a Def. An admitted read with a later edge refusal can still seed one.

| Counter, in decision order | Evidence withheld |
|---|---|
| `refused_param_list_decorator` | Operand under a parameter decorator on the owner-to-read path, including a decorator in a nested decorator argument. |
| `refused_nested_param_list` | Operand reached through another callable's formals, including a method in the owner's body. |
| `refused_param_list_shape` | For a read through the owner's own formals, the list or the read falls outside the permitted syntax. A nonfinal reader, member reader, invoked arrow or opaque sibling fails the whole-list check. |
| `refused_s1_binding` | Runtime binding or owner identity is unresolved, or the list fails the allowed-binder check (for example, an outer name in a bare default or an ordinary formal in the final reader). A proved differently bound candidate that survives the preceding checks is other-binding unless the whole list fails the allowed-binder check. |
| `refused_param_list_order` | With syntax and binding checks passed, initialization order fails anywhere in the list or remains unproved, including in a sibling. Check each bare read at evaluation and the final reader at closure creation. A definitely abrupt prior step refuses downstream reads. |
| `refused_param_list_write_effect` | W1, `with`, S or S2 withholds an otherwise considered entry projection, or an explicit or implicit signature effect leaves the reaching binding or label of a body pair unproved. Key coercion and destructuring Get are included. |
| `refused_s1_replaced` | A proved replacement suppresses the proposed entry pair under the identifier/projection rule. An emitted `killed` edge is recorded with its full label. |
| `refused_s1_value_version` | Reaching value or label at the actual operand remains unproved, including overlapping destructuring writes whose order is unknown. |
| `refused_s1_endpoint` | The admitted read has no eligible endpoint in its environment after preferred and permitted fallback lookups. |

Before graph deduplication reconcile candidate reads to emitted, other-binding, the sum of the per-attempt refusals above, and otherwise-emittable candidates withheld by a cap. A projection attempt withheld under W1, `with`, S or S2 has one `write_effect` disposition, with any emitted base replacement recorded alongside it. Count the replacement in the differential without adding a second read to the ledger. Record all applicable causes separately for the overlap report. Count each attempt once across count/replay and cache modes. Record uncertain signature effects and escapes as `unresolved_param_list_write_sites`, once per owner and site span, even with zero affected body pairs. Record a cap refusal once per affected owner as `refused_s1_rd_cap` or `refused_s1_pair_cap` and list the incremental Def/pair identities withheld. These site and owner counters have separate denominators. Corpus measurements and the matrix's `s1_counter` checks use the same registered S1 evidence counters. Existing §5.4 props counters retain their units.

**Fixture decisions.** Apply the S1 lines in `ROW-DISPOSITIONS.md` and the 31 S1 records in `NEW-ROWS.jsonl`, for 93 S1 rows in all. Keep the body and stored-reader positives in `sol-4-03` and `d2-default-capture` at their selected bytes. Re-decide the forward, member and call-valued defaults, the descendant-list capture, and the parameter/body collision. Pin the ordered sibling defaults (p31, astra-2-03), computed key (sol-3-01) and `d1-selected-endpoint/parameter-bare` to `cfg_incomplete`. Retain p10, p17, p23, p24 and p35. Verify p23/p24's signature writes against their **body** reads in each recorded grammar. Keep p27's two source Defs, j25's uniform endpoint and j31's named-line miss.

Retain all 576 §5.8 default/uniqueness cell IDs from `0085a4e4`. All 576 marked reads and 232 expected edge sinks in the 528 applicable programs lie in the target body. **Zero signature-capture cells need dropping.** The serialized outcomes are 128 admitted, 68 legacy alias, 160 early error and 172 no Defs. The other 48 IDs lack a bound name for one of three neighbor shapes, 16 each. Keep their exclusion reasons and the S2b reuse. The §5.8 JSX owner/region panels belong to S3. The signature cases below are focused S1 controls outside the 528.

- [ ] Unit tests for `js_ts_pattern_params` first, in `pattern_params_tests.rs`: one per binding/default/rest form in DESIGN 5.1 and 5.8, one per admission rule (the intermediate-default rule and the assertion-wrapper removal
      included), and the alias-default exclusion for an earlier formal and for an outer variable (p13,
      p29). They do not compile until the module exists: add the module skeleton returning empty lists in
      the same commit, so the tests are RED, not broken.
- [ ] Implement the accessor. Unit tests GREEN.
- [ ] Re-run the registered graph-builder adapter under the §5.8 two-stage contract. The TS generator must assert `ts.version === '5.9.3'` and emit each cell’s required evidence state and expected outcome without product input. Run `node tests/props_matrix_gen/props_generated.cjs --check` against the committed sibling JSONL and require the Rust adapter to consume that same file. Require each observed precondition before comparing the actual graph to the serialized outcome. Record generator/version, command, digest and per-family compared counts. Verify full/partial/cold cache and codec controls per Global constraints.
- [ ] Enable group `S1` in `props_matrix_tests.rs`: RED. Extend the producer loop in `file.rs`. GREEN.
- [ ] Exercise the assertion runner with `d1-selected-endpoint/parameter-bare`'s expected zero against a missing counter and a registered zero. Only the registered zero passes. Exercise `d2-default-capture` with a parallel edge at the same endpoint bytes under a second label. The required label still matches and the unfiltered `count: 1` fails.
- [ ] Check the exact terminal ledger entries in `d3-refusal-precedence`, `d3-abrupt-key-order`, `d3-sibling-order` and both `d3-method-*` rows, including the independent binding verdict for the nested shadow. In `d3-method-decorator`, require exactly one `other-binding` entry for [15,16) in each of TS and TSX. Assert the denominator reconciliation. For `d3-implicit-key-effects`, exercise coercion and a destructuring getter that can mutate the reachable field, and confirm the disjoint bare binding survives. In both `d3-signature-alias-*` rows verify mixed classification and base-read parity with the identifier twin, including when the named signature has no materialized default Use. Require the legacy alias twin to keep `nameonly/alias_unstable` in all three pattern variants. In each `shape-refused` case verify the exact shape refusal, retained body base read and absent entry projection. Check the runtime results with `p` omitted and supplied.
- [ ] Run the alias and projection rows against the reviewed #929 head and the amended product. Keep p17's assertions. The direct, arrow, intervening ordinary-function and shadowed `arguments` coverage lives in `owner-arguments-and-arrow`. Include static optional and quoted-key reads beside a dynamic subscript, and a refused nested capture. Exercise W1 member targets in assignment patterns and `for…in/of`, the existing same-root overlap handling and the non-comparable same-root witnesses. Run the sloppy-JavaScript `with` witness and its control. Exercise S with an effect before initialization, an effect after initialization, evaluated children and computed keys in a literal and an uninvoked closure. Run S2's array pattern, object rest and rest-formal witnesses with their control. Check the base read and full label a mixed local gets at the emitted receiver bytes for each demotion witness.
- [ ] Pin B1's attribute-own-expression containment. An attribute name may stand for a certified read only when that read is inside the attribute's own value expression. Exercise the own-read control beside sibling and spread reads. Pin B5 on the witnessed shared line in the CPG and in the byte-aware DFG export and lookup: require `cfg_incomplete` at the signature occurrence and `exact` at the body occurrence. Keep the existing line-group lookup for callers without occurrence bytes. Pin B6's compound, logical and indexed updates as value reads, with plain write-only targets as controls. Count guard-refused reads in the value-read ledger even when they cannot justify a Def. The code branch (`adb5e49e`) fixes round-1 A3's unconditional computed-key replacement by suppressing the entry edge. Verify the replacement edge at `cfg_incomplete`, matching the identifier twin.
- [ ] Review focus items 1 and 2 are rows p14, p15, p16, p26, p27: confirm they pass.
- [ ] Pin both admitted signature forms against near misses. Pair an earlier and a forward bare default/key, a stored scalar reader and a member reader, a final and a nonfinal reader, a stored and an invoked reader, and the same eligible operand with an inert and an opaque sibling. Check operand and binding bytes, sole-use Def presence/absence, label, and the reason for refusal. Pair a signature-only loss with the same local read in the body. Use the two environments of `d1-selected-endpoint/parameter-body` and its bare-default control.
- [ ] Re-decide `default_execution_regions_preserve_parameter_order` as counted refusals for its iterator, async, class, accessor and invocation forms. Retain its independent body `keep` in every case. Exercise stored generators, immediate consumption by spread or `.next()`, calls through `f.call(null)`, `f.apply(null, [])` and `f.bind(null)()`, suspension inside a call argument, inherited instance fields and computed getter selection. In `projection_writes_respect_the_parameter_body_seam` refuse the signature-only `use(o.k)` in both writer variants. Check R2's stored/invoked writer pair, and an affected body projection beside a disjoint body read. In `only_runtime_references_in_the_formals_environment_supply_defs` refuse single/nested parameter decorators and the call-valued sibling default. Keep the ordinary/arrow `arguments`, erased/live computed key and disjoint escaped-name controls. Include erased `declare`/`abstract` field keys beside a runtime field key.
- [ ] Test plain and nested destructuring writes independently in a body JSX attribute and in a signature. Use array and object targets, defaults and computed keys. Verify the actual value version or the counted omission. Keep astra-5-02's proved first key. Check bare-base replacement and escaped overlapping writes against disjoint and shadowed controls. For `d3-mixed-line-fallback` use focused producer cells for preferred and fallback Uses, raw and twin, mixed-line and wholly-body anchors, and version-only and scope-only invalidity. Include a refused signature-only read with no pattern Def. Require a positive relocation and each excluded case's actual legacy endpoint.
- [ ] At, below and above both producer caps compare the legacy owner's full edges/labels and count/replay results with S1 enabled. Pin B7's completed Def census at 2,047 → 2,049 across the 2,048-Def cap. Use p35 with a small test pair cap below its fallback count. Require the legacy one-node owner to have RD with distinct bases, the S1 increment to introduce the repeated base, and the capped replay to keep the legacy `safe` edge and label with one `refused_s1_pair_cap`. The uncapped p35 relabel and an unaffected owner are controls. Re-establish changed-row base colors in the same environment. Use the pre-S1 base for legacy obligations and `0085a4e4` for the review regressions. Require each of the 13 findings to have a byte-specific regression or an explicit refusal with a surviving control.
- [ ] For `legacy-relocation-cap-replay` add companion generated or producer cells using the lowered test caps. Exercise actual Def-cap and pair-cap replay at the mixed-line seam and compare the full owner against pre-S1 with a control that reaches the augmented route. Keep the fixture prefix through the seam and use the uncapped row's same alias-twin Def identity, the `a` declarator at [64,65), in both cap cells. Require that Def's `cfg_incomplete` edge to the legacy [31,31) point and exclude its relocated edge to [40,41). Use the ordinary matrix row for the uncapped result. For `step5c-pattern-input` and `ordinary-argument-new-source` use the measurement exporter for the Step 5c input/counter and the ordinary call-slot join. Compare against pre-S1 and the constant controls. The row schema covers the byte-specific graph edges. These companion cells are outside the existing §5.8 grid.
- [ ] Census signature candidates on pinned X, R2 and T, including those with no reviewed-head edge. Per corpus report the eligible locals, candidate reads, admissions, refusals by reason, distinct signature-only Def losses and retained body-callback captures. Split proved genuine losses, proved impossible/differently bound reads and unresolved cases. Compare the amended graph with `0085a4e4` by both endpoint spans, path and full label. Check the symbol **and** its evaluation environment. TypeScript 5.9.3 returns one symbol for the four `a` tokens at 13, 31, 42 and 59 in `d2-default-capture/m`. Require the separate parameter/body environments and the paired runtime results in §5.1.2. Use runtime or structural ordering witnesses for timing and writes.
- [ ] Write the differential prediction (DESIGN §7), then measure X, T, R2 and G with `measure.sh` and `compare_props.py`. Use pre-S1 for legacy parity and the reviewed #929 head for gross projection demotions. Run the binding oracle on added pattern edges and check each retained projection against M1, W1, `with`, S and S2. Hand-read 30 listed `exact` edges (a Claude seat) and the §7 retained projection stratum (X 30, T 27, cal.com 30) against the witness classes, including `with`, setters and shared sub-objects. Record the retained-stratum result in DESIGN §5.9. Read every legacy removal and full-label move. Count the p35 class separately and verify the mechanism for 20 examples per corpus, or all when fewer. Require exact Go-control parity and apply the one-third escalation trigger.
- [ ] Report gross projection demotions per corpus, with explicit zero counts. Give distinct owners and distinct projections withheld by each cause (bare occurrence, owner `arguments`, `with`, other-root store including an unproved root, same-root non-comparable store, signature effect and S2), their overlaps and deduplicated unions, distinct locals, base replacements after endpoint selection and deduplication, and reads left without an edge. Reconcile each §5.9 exception. List every R2.6 old/new byte pair with full label, grouped by fixture, grammar, corpus and raw/twin producer, with zero wholly-body, version-only, scope-only and replay moves. Require zero D3 alias-twin relabels for the omitted, trailing-formal and supplied pattern variants. Count p35 separately. Surface any unapproved label delta. Use the S1 measurement exporter to give one byte-keyed join per added `return_input_edges` input, the exact counter name and old and new suppression reason per transition, and the source-Use / call-slot / callee-Def join for each added ordinary argument-binding edge. Report gross counts and zeros per corpus.
- [ ] Tier-A matrix and quick runs; cloud tests at 1 and 4 threads with a RED-on-base control; two reviewers.
- [ ] PR body: the prediction, class counts and causal joins required above, the reaching-definition availability counter deltas, the binding and retained-projection checks, the hand-read and Tier-A results, and the list of excluded parity owners.

**Gate:** DESIGN section 7 "Slice S1".

### Slice S2: props binder with pattern targets, for object-literal arguments (3.5 to 4.5 days)

Two PRs: S2a is a pure move and is reviewed as one; S2b adds behaviour.

**Files:**
- S2b updates `tests/props_matrix_gen/generated-cases.jsonl` and reruns the TypeScript 5.9.3 regeneration comparison.
- Create in S2b: `src/ast/js_ts/component_route.rs`, `src/ast/js_ts/component_route_tests.rs` with declarations in `src/ast/js_ts.rs`. Extend `src/cpg/props_generated_tests.rs`, `tests/props_matrix_gen/props_generated.cjs`, and `src/cpg/object_literal_parity_tests.rs`.
- Create: `src/cpg/props_binding.rs` (move in S2a; extend in S2b). Use the S1 AST facts and shared key helpers. Add literal writer/matcher generated cases to the existing props tests in S2b.
- Modify: `src/cpg/object_literal_args.rs` (becomes the literal source and calls the binder),
  `src/cpg/callee_params.rs` (pattern facts), `src/cpg/build.rs` (module line; pass pattern facts; merge
  counters; the `dfg_props` field on `DfgLabelStats`), `src/cpg/callee_legacy.rs`,
  `src/cpg/step5b_reference.rs`, `src/cpg/props_matrix_tests.rs` (`ENABLED` gains `"S2"`),
  `src/resolution/js_ts/namespace.rs` and `src/resolution/go/satisfiers.rs` (the read-only export query),
  `src/cpg/object_literal_arg_tests.rs` (`destructured_rest_and_bare_arrow_formals_are_out_of_scope`: its
  destructured case moves to the matrix with the new expectation; the rest and bare-arrow cases stay),
  `src/cpg/callee_param_cache_tests.rs` and any test that names the cache's types, if they change,
  `crates/prism-review/tests/cli/min_confidence_test.rs` (the statistics golden lists every field),
  `src/cpg_cache.rs`, `src/cpg_cache/dfg_codec_tests.rs` and `src/cpg_cache/stream.rs` (version pins, owned/streamed statistics transport),
  `src/cpg.rs` (test module lines), docs

**Interfaces (produced):**

Attach the 84 S2 disposition lines, their pinned data and additions to the S2b brief. S2a moves the existing plain-target pairing code verbatim where possible. Keep public/internal callers, representation, traversal, labels and byte-keyed source behavior identical. Introduce the following behavior-bearing interfaces in **S2b**, after the S2a differential.

- `compute_pattern_formal_nodes` joins the unique callable syntax span and each binding's exact bytes to a Def. Cache per-callee pattern positions beside the existing positional cell. Retain empty and alias-skipped positions and all levels' exclusions. Cached targets carry owner and binding geometry; on a line with multiple same-name Defs resolve the correct one by bytes (use the exact occurrence index). Direct twins use concrete node indices for the same facts. Keep `compute_param_def_nodes` and the positional cache's clients intact.
- `PropsEntry` represents one ordered writer **at one object level**: named leaf (operand bytes), named object (ordered children, possibly empty), constant/opaque named terminal, numeric opaque writer, unknown writer or spread. Keep legacy #594 visibility on the same parsed writes. `PropsTarget<T>` is a plain target or a pattern position with per-level exclusions and optional local targets.
- `collect<T>` takes the existing positional targets, pattern positions, resolver/query, caller site and per-caller counters. `bind<T>` joins surviving leaves to eligible destinations. Its source callback resolves a **whole leaf**, returning concrete source node(s) or one refusal. S2b keeps #594's full/base byte-contained candidates; S3 returns one connected caller node or one reason for the entire leaf. A missing literal byte lookup supplies no JSX source counter. Resolve generic targets with the existing cache/twin adapters.
- Pre-loop, count a direct-literal site once and apply the Exact-singleton gate to the whole resolution. Preserve ordinary argument flow. At Exact sites skip each literal after an argument spread, counting `refused_argument_after_spread` per position. For each safe pattern position run checks 2 **and 3**. `js_ts_component_route(site_kind, lexical_site, selected_callable, export_proof)` returns direct, proven local hook operand, JSX render operand, unadmitted wrapper or unproved identity. Require binding/file/body bytes and position mapping. Check hook provenance at its initializer. Count `refused_unknown_wrapper` or `refused_target_identity` in §5.4 order and preserve the old plain position at a mixed site. Merge admission reasons in DESIGN 5.4 order. Share a stable per-caller dedupe by concrete endpoints for props counters and emissions; recursive edges qualify by site/position even when both owners coincide.
- Production, legacy and reference traversals return counters beside pending edges. Merge in caller order into `DfgLabelStats.dfg_props`. Do not serialize graph indices. DESIGN 5.1–5.4 and 5.8 define all states and generated cases for this slice.

- [ ] **S2a (its own PR), no behaviour change:** move the pairing from `object_literal_args.rs` into `bind`
      (plain targets only, no counters, no statistics field). The existing object-literal tests stay green;
      a base-versus-head differential on X and T shows zero changes in edges and statistics. No cache bump.
      Both reviewers, briefed that it is a pure move; merge before S2b branches.
- [ ] **S2b, step 1:** introduce the counters, statistics field and ordered writer tree. Apply the original projection to the same old plain-literal targets, including bare reads of odd string keys. X and T must show zero edge changes at this step. Add the selected-binding filter in step 2. Exercise empty parent replacement, scoped wildcard, and byte-distinct literal sources.
- [ ] **S2b, step 2:** implement pattern facts and position-local admission (DESIGN 5.3.1–5.3.3, 5.4). Expose the read-only namespace/export equality query through `CpgCallResolver` for both resolvers; do not extend the candidate set. Apply the §5.3.2 selected binding test to pattern Defs, legacy plain member/base and bare Uses before projecting keys. For `d2-literal-selected-read` check the M-3 inherited-edge preconditions at the actual shadow bytes, then check the refusal and the independently valid reads. Preserve all proved #594 bracket controls and record any removed legacy witnesses as `legacy-unproved-read`. Check same-region TDZ vs the deferred `d2-forward-literal` positive, CommonJS/returned-value refusal and F9’s site-kind filter. Exercise rewrite, delayed/pass-through and shadowed hooks, unbound foreign `FreeSingle`, the local React hook and a stable import alias. Check exact resolver preconditions before counting the failure. Run all 84 S2 dispositions, including mixed-target calls, recursive literal sites, empty patterns, residuals and unsafe spread positions. Run §5.8's collision, literal replacement and default panels through the real builder against the independent model, including the legacy plain projection with literal and bracket compatibility controls. Check disconnected literal sources and both byte-contained full/base edges. Pin `d2-unknown-writer` and `d2-unknown-target` to the collision mutations. Execute both literal guard rows with verified foreign/imported resolver routes. Run both cache-pin tests in Global constraints. Enable the group after recording its RED controls.
- [ ] Update the affected tests listed above. Report which object-literal, cache-version, parity and statistics assertions changed.
- [ ] Differential on X, R2, T: require `props-literal` additions, individually proved `legacy-unproved-read` suppressions and their witnessed post-guard consequences (including new exposures through added hops). Fail on any unexplained change; report the new counters. Extend `tools/compare_props.py` and `tools/oracle_props_edges.cjs` in the evidence workspace as needed for recursive same-owner sites. Use site/argument evidence before owner equality and self-check a recursive hop beside an ordinary intra-owner edge. Oracle on the additions: 0 WRONG.
- [ ] Review focus item 5: extend `object_literal_parity_tests.rs` with a pattern-formal case (production,
      legacy twin, serial reference, cache round trip, incremental rebuild: identical edges and counters).
- [ ] Tier-A; cloud tests; two reviewers.

### Slice S3: JSX attribute source (4 to 5 days)

**Files:**
- Update `tests/props_matrix_gen/generated-cases.jsonl` and `tests/props_matrix_gen/props_generated.cjs`. Byte-compare regeneration under TypeScript 5.9.3 and check evidence/outcome in Rust as in S1.
- Create: `src/cpg/jsx_props.rs`,
  `tests/algo/taxonomy/taint_jsx_props_test.rs` (with its `mod` line in `tests/algo/taxonomy/main.rs`), plus JSX source/owner generator cases in the props tests
- Modify: `src/ast/js_ts.rs` (module lines), `src/cpg/build.rs` (module line and the JSX branch),
  `src/cpg/callee_legacy.rs`, `src/cpg/step5b_reference.rs`, `src/cpg/props_matrix_tests.rs` (`ENABLED`
  gains `"S3"`), `src/cpg/object_literal_parity_tests.rs`, `src/cpg/props_generated_tests.rs`, `src/cpg/props_binding.rs`, `src/cpg/object_literal_args.rs` (expose reached read geometry), `src/ast/js_ts/component_route{,_tests}.rs`, `src/cpg_cache.rs`, `src/cpg_cache/dfg_codec_tests.rs`, matrix README and docs

**Interfaces (produced):**

- Extend S2b's `js_ts_component_route` to classify the actual function node and JSX tag binding. Return direct, admitted React first operand, other wrapper, or unproved identity. Compare the selected body with the unique syntax span; an anonymous first operand is supported (j05). Use the existing wrapper proof and namespace query from S2b.
- `jsx_props::entries(caller, opening_span)` produces the S2b writer tree for this opening, including emitted-child overwrite. Extract nested sites independently. Match namespace tags, erased wrappers, reserved top-level names and child trivia per DESIGN 5.2.
- `jsx_plain_read(parsed, endpoint, formal)` returns a proved whole read, complete dot chain, bracket refusal, zero-width bare refusal or unproved read. First prove the actual selected root/representative under §5.3.2 for all three representations. A genuine use elsewhere on the line cannot certify the shadow at these bytes. Count each failed site/Use once. Walk receiver parents even for projected endpoints; account for all §5.3.2 forms. Record the per-site/read counters without suppressing valid reads. Keep the legacy literal adapter for sol-3-07.
- `jsx_props::source_node(site, leaf) -> Result<NodeIndex, SourceRefusal>` consumes the operand's structured path, bytes and value-token lines. Use the immutable pre-Step-5b graph and first-wins VarIndex. Scan all full-path candidate lines for a conflict before any selection, then use the §5.3.4 root-line priority and conflict rules for base fallback. Try full path then base and choose one connected caller Use. Exercise `d1-multiline-selection` against the real pre-Step-5b inputs. Use the §5.3.4 test observer at the Step 5b call in `src/cpg/build.rs` to check each indexed full Use's bytes and binding/version conflict before testing connectivity. Require the incoming definition/projection and binding/version proof for the emitted source. Report ambiguous, unconnected, enclosing-only or missing in DESIGN 5.3.4 priority. Do not require a nonzero-width source. Do not create a Use or use an ancestor as the source. Reuse the AST scope walk and write facts in the JSX module; no DFG producer or trace changes in S3.
- `jsx_props::collect<T>` calls the binder with one JSX target at runtime position zero, the ordered entries, the whole-leaf source callback and the same target adapter and counters as S2b. There are no positional arguments at a JSX site.

**Site admission and branch.** Count `sites_jsx` once per recorded attributed opening, before target iteration; check 1 may return no callee. For the Exact singleton, branch before fetching the site's arguments. Apply member qualifier/export checks, wrapper/target identity, then formal presence/alias state in that order. Count the first site refusal once and continue; otherwise pair surviving entries in DESIGN 5.4 order. Insert the same branch and stable emission order in all three traversals.

- [ ] **Entry gate before S3 development.** Produce the X/cal.com non-test receipts for A-S1 items 1, 2 and 5 per DESIGN §5.7. Join census `attributes.jsonl`, the pinned TS syntax/symbols and the S1/S2 pre-binding evidence by exact bytes. Add the two §5.7 counts on S2b: predicted JSX pairs rooted at a bare-arrow or rest-identifier formal reached backwards from an existing cross-file FreeSingle seed (separate existing reach from exposure through predicted hops), and predicted plain targets whose first-wins reached Use resolves to a different binding. Report eligible denominators, individual and deduplicated union losses, pair overlap, unresolved joins and remaining edge margin against 1,000. Report guard-related loss of caller connectivity separately. The public aggregates alone cannot settle these cuts.
- [ ] Extend the S2b route tests to JSX (a direct function, both admitted wrapper forms, each
      wrapper of row j36, the `lazy` loader, a wrapper declared inside a function); record RED before adding the JSX arm, then verify GREEN.
- [ ] Attach the 179 S3 disposition lines, pinned data and additions to the brief. Record updated controls and enable the group. Implement entries, source proof and the three collection paths. Exercise DESIGN 5.7's mixed outcomes for j31, j33 and the remaining callback, catch, signature and collision cases. Run §5.8's source-generated core, region, multiline path and attribute-name panels plus the JSX half of the replacement panel through the real builder. Retain checked site, owner, endpoint and trace/graph witnesses; require the family counts and discriminating evidence states. Include the direct T-3 grid, both declaration orders of the core const-arrow target, both JSX guard rows and the `d2-jsx-selected-read` preconditions from M-3. Test both bracket rows and the B-7 j22/j31 and hook call/JSX controls.
- [ ] In `props_generated_tests.rs` run the §5.3.4 lookup observer for `d1-multiline-selection/{shadow,stale}`, `d2-multiline-{shadow,version}/conflict` and the eight generated negatives (two conflicts × two layouts × two grammars). Match the first-wins full Use ID and bytes to the marked read and prove the shadow declaration or intervening write even if it has no incoming edge. With only full entries hidden, prove the outer Def → root-line base Use and emit that base → target edge. On the untouched lookup require one ambiguity refusal and no edge/path to the target. On the same input the broken fall-through selector must emit that exact base edge. Run the unfiltered paired source baselines and both full/base positives (§5.8).
- [ ] In the matrix harness and README extend `trace_boundary` with optional `boundary_kind` (`cross_function` by default, or `self_function_param`), `from` and `from_bytes`. Check the same boundary record's kind, root, source node/bytes and target; the target stays out of the frontier. Older rows retain their cross-function behavior. Assert `SelfFunctionParam` from the intended Use to the formal Def and `trace_misses` at the body for astra-1-07 and astra-3-13/direct. Keep their paths and astra-3-13/wrapped. No trace product edit. Repeat cache restore, codec and parity checks.
- [ ] `taint_jsx_props_test.rs`: the issue's example in one TSX file with an explicit source line; default
      review taint reports a `taint_sink` finding on the `dangerouslySetInnerHTML` line. RED before the
      branch exists, GREEN after.
- [ ] Parity: add a JSX case to `object_literal_parity_tests.rs` (Review focus item 5).
- [ ] Before the differential, remeasure the 5.7 source/refusal cells on the S1/S2 graph and split earlier-spelling cases by true binding/version conflict. Compare predicted pairs, edge deduplication and realized connected edges with stage M. Extend the evidence oracle's current spelling/line check to verify binding/version at a borrowed representative independently. Self-check a safe earlier read, an attribute-name token, a shadow collision, a same-line write and a zero-width continuation. Cover recursive props edges and per-key rest/container witnesses in `tools/paths.py` and the comparator as needed. Differential on X and R2: require `props-jsx` additions and only causally proved `guard-removed` consequences under §6. Check final connected share and Def reachability separately from pre-binding admission, including any deleted props occurrences. Reject unexplained removals/label moves. Oracle with checks 1 to 3: 0 WRONG; UNDECIDED
      share within the gate; hand read of the `undecided_sample.py` sample (50 per corpus, all when fewer)
      by a Claude seat; `paths.py` connected share.
- [ ] Review focus items 3, 4, 6 and 7 are rows j21, j18, j25 to j28, j34, j36 and j39: confirm they pass.
      Tier-A; cloud tests; two reviewers.

**Gate:** the oracle, volume and end-to-end rows of DESIGN section 7 (on X plus R2: ≥ 1,000 CORRECT edges
from JSX attribute values, 0 WRONG, UNDECIDED ≤ 25% per corpus, connected share ≥ 90% per corpus, ≥ 25
chains with two or more component hops).

### Slice S4: end-to-end measurement and close-out (3 to 4 days)

**Files (evidence directory):** `g/paths-{X,R2}.md`, `g/examples/`, `g/REPORT.md`, `tools/examples.py`, `tools/paths.py` and `tools/props_residuals.cjs`.

- [ ] Materialize the §5.7 attempt ledger per corpus/partition. Join syntax, complete site inventory, pre-Step-5b evidence, final post-guard edge and node IDs, full-cache and fresh stats, verdicts and trace boundaries. Reconcile before-dedup pairs against distinct edges. Publish the three ambiguity buckets, unjoined sites, S1 endpoint omissions, S2 literal-position failures, S3 read refusals, the F3 only-candidate proxy versus all reached zero-width sites, and F5 boundary causes with the specified denominators. An unavailable measurement is `unmeasured` and its follow-up decision stays pending.

- [ ] `paths.py` on X and R2: the path table, the multi-hop chain count and the connected share. Report first failures and post-S1 recoveries separately from baseline predictions. For recursive props require the same site proof as for cross-owner props; for each credited container/rest hop retain the incoming key restriction.
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
      In both directions keep only head-only lines reached across a props edge (the head's witness or
      slice crosses an edge that `compare_props.py` classes `props-jsx` or `props-literal`) and list the
      others apart, unrated (DESIGN section 6).
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
      whose qualifier is an imported value, or a namespace import whose module has no export of the
      member's name), with the inputs of rows j34, j39, l21 and l22 and the reviews'
      reproductions in `~/prism-evidence/lanes-2026-11-20/reviews/lane3-fable-r1-probes/` (`res2`, `res3`) and
      `~/prism-evidence/lanes-2026-11-20/reviews/astra-cal/lane3/cmp-scratch/k1`,
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
