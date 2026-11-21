# Lane 10: subscripts in the data-flow graph — implementation plan

> **For agentic workers:** this plan is run by an orchestrator under `../ORCHESTRATION.md`. Each task is
> dispatched to an implementer (Sonnet-5.5, high) with its section as the brief. Steps use checkbox syntax.

**Goal:** a value written through a string-literal subscript reaches later reads of the same element in
Python, JavaScript/TypeScript and Go; a write to one key neither reaches nor kills a read of another; dynamic
keys keep today's may-meaning; every rule is checked against executed truth and a labelled corpus sample.

**Architecture:** one node-based path builder (`src/ast/subscript_paths.rs`) answers unaffected, admitted or
refused for every subscript chain; producers use it for chains that hold a static key and keep today's text
path otherwise. First every producer and consumer moves behind one path function that still returns today's
paths (zero difference); then subscript reads become real-span uses (nearly inert); then keyed definitions,
the extension rule, the definite-identity label rule and the consumers' switch; then kills; then, behind its own
go, a wildcard reach rule for dynamic keys; container reads and initialisers are conditional. Every go is per
language (DESIGN 6).

**Tech stack:** Rust (core crate: `src/access_path.rs`, `src/ast`, `src/data_flow/build`, `src/cpg`);
tree-sitter Python 0.23.6, JavaScript 0.23.1, TypeScript 0.23.2 and Go 0.25.0 + PR #193 (both vendored);
Python 3.12 (uv), Node 24.15 and Go 1.26.2 for the truth runner. The census runs on Prism's own parser.

**Spec:** `DESIGN.md` here (5.1 to 5.7 the rules: 5.3 identity, overlap and the base predicates fixed, the
alias policy and the rest tentative until stage D; 6 the census; 7 the gates, the spelling pairs and the
containment; 9 the owner decisions, whose defaults this plan applies).

## Global constraints

- Worktree `~/code/prism-subscripts`; evidence `~/prism-evidence/subscripts/`.
- Corpora (`~/code/bench-repos`): P1 `apache-superset` at `7519fa8b6f`, P2 `saleor-saleor` at `cc4aea76e3`
  (Python); X Excalidraw at `0642e72c` (`~/prism-evidence/inputs/excalidraw-0642e72c/source`); T
  `TypeScript/src` at `7964e22f2b`; G1 `caddy` at `77e9ce7404`, G2 `etcd` at `61d518f55e` (Go); control R
  `ripgrep` at `82313cf958` (Rust, unchanged by the lane: zero edge and counter change on every slice).
- **Not changed:** `AccessPath::from_expr` and its tests; the path of any chain without a static-key subscript
  (byte-identical); integer indices and slices; languages other than Python, JS, TS, TSX, Go;
  `src/cpg/trace.rs` (PR #841, lane 8); review taint (`crates/prism-review/src/algorithms/taint*`,
  `taint_sinks/`); the Go field-recall pass (`src/cpg/reaching/contribution.rs`, `go_aggregate.rs`: S6's
  model, not edited); `src/reasoning/seeds.rs` (compares an argument's base only; S3 adds a test that proves it
  unaffected); `src/cfg/js_loops/for_init.rs` (spells every bracket `[]` from syntax, stays conservative; S3
  pins it); `src/ast/member_projection.rs` (stays on text, DESIGN 5.6; S3 pins it).
- **Per-language enablement:** the stage-M memo names the passing languages; the seam (`path_at`, S1) answers
  today's path for every chain of a language that did not pass, and S2 and S5 skip it, so its output never
  changes; its rows are regrouped as accepted misses with the count.
- **Shared with other lanes (coordinate before the PR, record on both issues):** `src/cpg/object_literal_args.rs`
  (lane 3): S1 routes the one lookup at line ~497 through the seam, S3 changes its answer; the Step 5b triplet
  (lane 9): the same for the argument path lookup only; the CPG cache version (every semantic lane). The
  orchestrator's section 11 table lists these.
- Weighted sizes (`python3 scripts/file_size.py`): `src/access_path.rs` 396, `src/ast/lvalues.rs` 564.5,
  `src/ast/rvalue_identifiers.rs` 594, `src/ast/member_reference_index.rs` 327.5, `src/ast.rs` 600.5,
  `src/ast/assignments.rs` 506, `src/ast/member_projection.rs` 291, `src/data_flow/build/file.rs` 850.5,
  `src/data_flow/build/aliases.rs` 96, `src/cpg/object_literal_args.rs` 469, `src/cpg/step5c.rs` 283,
  `src/cpg/step5c_reference.rs` 325, `src/cpg/reaching/capture_regions.rs` 524, `src/cpg/reaching/def_buckets.rs`
  319, `src/cpg/reaching.rs` 990, `src/cpg_cache.rs` 1,544, **`src/cpg/build.rs` 1,777** (call-site edits only).
  Logic goes in new modules.
- Module lines: `src/ast/` modules in `src/ast.rs` (tests `#[cfg(test)] mod x_tests;`), `src/data_flow/build/`
  modules in `src/data_flow/build/mod.rs`, `src/cpg/reaching/` modules in `src/cpg/reaching.rs`.
- A slice that changes stored DataFlow nodes, edges, labels or statistics bumps `CACHE_VERSION` (189 at
  planning) to the next free number at merge, with a `/// - vNNN:` line and the pin test
  `cache_versions_are_pinned_for_cpg_semantics`; S0 and S1 bump nothing. Tier-A applies to S0 to S7, the corpus
  differential to S1 to S7. Executed fixtures are lane-written only and run contained (DESIGN 7).
- Differentials (DESIGN 7): each slice is compared with the previous slice's binary (`bin/prism-S<n-1>`;
  `prism-base` for S0; when main moved under the slice, the binary of its actual merge parent) for attribution,
  and with `prism-base` for cumulative impact. Every gate names its comparison. Each receipt records both product
  SHAs, the corpus hashes and the class counts.
- The member index and its oracle change together (`Index::build` in `src/ast/member_reference_index.rs`;
  `collect_path_refs` and `collect_fenced_path_refs` in `member_reference_index/oracle.rs`), or
  `member_index_differential_edges_and_non_js` fails.

## Review focus

1. A key with an escape or interpolation (`d["\x61"]`, `o["\x78"]`, `d[f"{p}"]`, `` o[`${p}`] ``), or a JS
   non-ASCII key (`o["é"]`), is never a keyed component; it connects only through the dynamic rule, never
   `exact`. Rows K08, K09, K11, K22 (S5); the spelling-pair test (S0).
2. `d["a"]` and `d.a` on one Python object (a `dict` subclass) stay apart both ways. Rows K01, K02 (control,
   confirmed in S3).
3. Augmented assignment through a subscript: `d["a"] = t; d["a"] += "x"; sink(d["a"])` reaches; so does
   `d["a"] = ""; d["a"] += t`. Rows P21, P31 (S3).
4. A subscript on a call result (`make()["a"]`) gets no path, definition or edge, and no definition of any name
   in it. Rows R01, R09 and the S0 test `refused_chains_are_refused`.
5. A nested subscript with a dynamic key (`d[k]["a"]`, `d["u"][k]`, `d[i]["a"]` against `d[j]["a"]`) never
   gives an `exact` edge or a kill. Rows Y05, Y06, Y10, Y11.
6. Definite identity (DESIGN 5.3): a rebound, loop-bound or (under the stage-D alias policy) aliased base gives
   no `exact` keyed edge. Rows N01 to N03.
7. Consumers move with producers: the existing `exact` literal-leaf transfer into `p.slot` survives, and a
   prefix definition still reaches an extension read at its bytes across a call. Rows A01, P23, E01 to E07.

---

## Stage M: census (3 to 4 days)

### Task M-1: workspace, base binary, issue, baseline

- [ ] Worktree (ORCHESTRATION 4). `~/prism-evidence/tools/memguard.sh 8 cargo build --release`; copy to
      `~/prism-evidence/subscripts/bin/prism-base`; record `--version`.
- [ ] File the issue "Data flow through subscripts: keyed paths (lane 10)", linking DESIGN and PLAN.
- [ ] `tools/measure.sh <binary> <corpus> <outdir>` (lane 11's `scripts/measure/` instead if merged): through
      memguard, `prism nav --no-cache dfg-stats --repo C`, `prism nav --no-cache call-stats --repo C` and
      `PRISM_EDGES_BYTES=1 prism nav --no-cache dfg-stats --repo C --edges`; from S0 on also the node inventory
      `PRISM_SUBSCRIPT_TRACE=all` (S0). Run on the seven corpora with `prism-base` into `base/<corpus>/`.
- [ ] `tools/compare_subscripts.py <parent dir> <head dir> <corpus>` (model
      `~/prism-evidence/jsts-lane/783/scripts/compare761.py`): edges keyed by file, path and bytes at both ends;
      the node inventories when both sides have one; every counter delta of both stats files; the zero-gates
      `cross-key`, `spelling-leak` and `orphan-anchor` (definitions in S2 and S3); the `dynamic-edge` bound
      (S5); the classes each slice names, each a mechanical predicate over the joined records (no class needs
      reading); exit 1 on an unclassified change. Self-test `tools/test_compare_subscripts.py` on hand-made
      dumps: a cross-key edge into the second argument of `f(d["a"], d["b"])` fails; `o["x"] → o.x` passes; a
      wrong edge through a local, a respelled dynamic-only node on a static-key line, an unjoinable endpoint and
      a zero-width lane-path use are each caught; a removed `exact` edge whose endpoints get different static
      keys is counted as `cross-key-removed`, not failed; a `dynamic-edge` count over its bound fails.

### Task M-2: inventory and counts C1 to C4, C6, C7

**Files (evidence):** `census/subscript_census.rs` (copied into the worktree as `examples/subscript_census.rs`
for the run and never committed, like `examples/dfg_census.rs`), `census/count.py`, `census/golden/`,
`census/CENSUS.md`. The census uses Prism's own parser (`ParsedFile::parse`, the pinned and vendored grammars),
not PyPI grammars.

- [ ] Grammar classes: from `vendor/tree-sitter-go/src/node-types.json`,
      `vendor/tree-sitter-typescript/{typescript,tsx}/src/node-types.json` and the locked registry sources of
      `tree-sitter-python-0.23.6` and `tree-sitter-javascript-0.23.1`, list every node kind that can hold a
      subscript as write, augmented, update, read, delete, pattern leaf or loop head; the census dispatches on
      each listed kind and counts any other parent as `unknown`.
- [ ] `cargo run --release --example subscript_census -- <corpus> <out.jsonl>`. File filter: the `TESTPATH`
      regex copied from `~/prism-evidence/lanes-2026-11-20/probes/lane10/sizing/size.py` (do not import it: it
      runs its corpus loop and writes `sizing_results.json` on import). One record per subscript node (Python
      `subscript`, JS/TS/TSX `subscript_expression`, Go `index_expression`): file, owner, line, bytes, role,
      full base chain, index kind (`str_ident`, `str_other`, `str_escape`, `str_interp`, `int`, `name`,
      `go_const`, `other`), key text, enclosing loop, enclosing exclusive arm, and for each pair candidate the
      feasibility facts of DESIGN 6 (exclusive arms, early exit, base rebinding, definite overwrite, Go binding).
      Plus eligible container literals (DESIGN 5.5 layout), whole-container value reads (DESIGN 5.3 roles), JS
      dot writes paired with identifier-key bracket reads, and per base the facts of C6: the language predicate
      of DESIGN 5.3 (Python bound-once by the `is_binding_occurrence` forms, JS `stable_bases` shape, Go one
      declaration, no other plain write, not address-taken), an alias-map input (`ParsedFile::
      collect_alias_assignments` gives `x = b` and the owner writes through `x`) for policy C, and a bare
      occurrence of `b` other than its binding or as a subscript, attribute or call object for policy A.
- [ ] `census/bracket_edges.py <edges.jsonl> <sites.jsonl>` (C7): every edge of the base edge dump with `[]` in
      either endpoint's path, joined by bytes to the census's subscript records, classified by the paths it will
      have under the census's key classification: equal, prefix → extension, different static key or tail,
      static ↔ dynamic, refused chain (definitions and uses), dynamic-only. Reclassified with S0's trace before
      S3's prediction.
- [ ] Golden: `census/golden/` small Python, JS and Go files with exact expected per-owner counts (mutually
      exclusive arms give no pair, a rebinding or an early return between W and R gives none, a JS dot read
      counts, a receiver-only read is no whole-container read); the census must match exactly.
- [ ] Validation, both directions: 200 sampled reported sites against `prism-base nav --cache-dir
      ~/prism-evidence/subscripts/navcache nodes-at --repo C --location f:l --format json` (a write needs a Def
      at the target bytes; a read is confirmed only by a node whose bytes lie in the subscript, never by a
      receiver Use alone); every subscript node of 30 sampled files by an independent tree walk against the
      census records; 100 sampled C1 same-key pairs read against the code for pair precision (Wilson lower
      bound). Agreement ≥ 95% on sites and files, else fix the census.
- [ ] `count.py`: C1 to C4 and C6 as DESIGN 6 defines them, per language, each owner once per class; C6's
      `exact` yield under policies C, A and B.

### Task M-3: the memo

- [ ] `CENSUS.md`: C1 to C4, C6 and C7 per corpus and language with the validation figures and one decision
      line per language and per conditional slice, posted on the issue.

**Exit gate (per language, DESIGN 6):** a language whose scaled C1 same-key owners are below 150 is dropped
(builder and seam only, its count stated); JS's part of S4 needs its different-key-kill count ≥ 50; S5 needs, per
passing language, dynamic pairs ≤ same-key pairs and a per-owner 99th percentile ≤ 50; C3 ≥ 300 makes S6 a
candidate, C4 ≥ 300 makes S7 one; stage D decides with the rows. No language passing: post, stop, wait for the
owner.

---

## Stage T and E: executed matrix (4.5 to 5.5 days; one language 3 to 3.5)

### Task T-1: rows, truth runner, data-driven test

**Files:**
- Create under `tests/fixtures/subscript_matrix/`: `rows.jsonl`, `cases/<id>.{py,js,ts,go,rs,java,c}`,
  `README.md`, `truth/run_truth.py`, `truth/prelude.{py,mjs,go}`, `truth/test_run_truth.py`,
  `truth/truth.jsonl` (committed output), `truth/check.sh`, `spellings/{py,js,go}.txt` (one key spelling per
  line, DESIGN 7), `truth/spelling_truth.jsonl` (committed output: `lang`, `s1`, `s2`, `same`). Create
  `tests/ast/subscript_matrix_test.rs`.
- Modify: `tests/ast/main.rs` (`mod subscript_matrix_test;`)

Cases: Python `def case():`, JS/TS `function caseFn() { … }`, Go `package main` with `func caseFn()`. Each starts
with `t = src()` (`const t = src();`, `t := src()`) on its own line, calls `sink(…)` exactly once (one
observation), one statement per line; a trailing comment `W` marks the subscript write that matters and `R` the
read line. TS uses erasable syntax only. Containers start with every key the row reads set to `"s0"`
(`d = {"a": "s0", "b": "s0"}`; nested `{"u": {"n": "s0", "m": "s0"}}`), Go maps are typed
(`map[string]string`, `map[string]map[string]string`, `map[string]S` for a struct value), so no read raises and
`+=` compiles. `C` is a small class.

`run_truth.py <cases> <rows.jsonl>`: per row, a fresh process (10 s timeout) runs prelude, case and a call of
the case function: `uv run --python 3.12 python`, `node` (24, type stripping on) for `.js`/`.ts`, `go run case.go
prelude.go`. `src()` returns `"TAINT-7f3a"`; `sink(v)` prints whether any string reachable from `v` contains it
(deep and cycle-safe: Python dict, list, tuple, set, `__dict__`; JS own properties, arrays, `Map`, `Set`; Go
`reflect` over map, slice, array, struct, pointer, interface). Truth: `reaches`, `not`, or `inadmissible`
(exception, timeout, no output, more than one observation). First line: the three interpreter versions.
`check.sh` fails on an empty, truncated or duplicated `truth.jsonl`, on a row with no truth and on any
`inadmissible` row: such a row is repaired, never mapped to `not`. Containment is DESIGN 7's, checked by the
runner at launch. `run_truth.py --spellings` runs DESIGN 7's spelling pairs from `spellings/<lang>.txt` under the
same containment and writes `spelling_truth.jsonl`. `test_run_truth.py` (`python3 -m unittest`): reaching row → `reaches`; throwing row →
`inadmissible`; two observations → `inadmissible`; nested marker found; a case with an `import`, `require`,
`__import__`, `open`, `process` or `fetch` is refused before any case runs; a prelude whose import set differs is
refused; `'a'` and `"a"` are one key in Python; `"é"` and `.é` are one key in JS. Runs locally through memguard.

`rows.jsonl`: `id` unique per program and language (`P01.py`), `group` (`control`, `S2` … `S7`), `lang`, `file`,
`class`, `census` (C1 to C4 class), `expect` for truth `not` (`no_route` when W and R name different static
keys, else `no_exact_route`), `base_verdict` (the base build's `taint-reaches` verdict, required on `control`
rows), optional `label` (`exact`, `killed`, `nonexact`: on the edge between the W and R occurrences, joined by
bytes; a row whose label depends on the alias policy gives it per policy, `{"C": .., "A": .., "B": ..}`, until
stage D), optional `pinned_main` (main's verdict and route: known-wrong rows and accepted losses; an
out-of-model row pins the answer its contract gives, DESIGN 5.7), optional `accepted_loss` (reason, issue).
Truth comes only from `truth.jsonl`. A `control` row is one where main's answer already agrees with truth; a row
where they disagree is RED and belongs to the first slice that can make it green.

`subscript_matrix_test.rs`: `const ENABLED: &[&str] = &["control"];` and `#[test] fn subscript_matrix_rows()`.
Per enabled row it writes the case to a temp repo and builds a `NavigationSession` (copy the `fixture` helper
of `tests/reasoning/taint_reaches_test.rs`) and a `DataFlowGraph` (labels via `crate::common::dfg_labels`).
Required (DESIGN 7), with the marked definition at the `t = src()` target bytes and the sink use at the sink
argument's bytes; a row whose sink use is not found at those bytes fails, whatever its truth:
- truth `reaches` → `taint_reaches` from the `src()` line to the `sink(` line is `Reached`, and
  `route_through(dfg, marked, w_bytes, sink_use)` holds (any labels);
- truth `not` → `no_exact_route(dfg, marked, sink_use)` (no route over `exact` DataFlow and argument edges plus
  same-owner same-line use-to-definition transfers) or, for `expect: no_route`, `no_route(dfg, marked,
  sink_use)` over every edge;
- `control` rows: `taint_reaches` equals `base_verdict`;
- `label` checked on the W→R edge pair (per the active policy from stage D on); `pinned_main` takes precedence
  over truth for the assertion and the row is listed in the test output.

The route helpers sit in `tests/ast/subscript_matrix_routes.rs` (declared in `tests/ast/main.rs`) with
self-tests on synthetic edge lists: `no_exact_route_rejects_an_indirect_all_exact_route` (the review's
`t → o.x → tmp → sink` shape), `route_through_rejects_a_route_that_skips_w`, `routes_bind_by_bytes_not_line`
(two uses on one line), `an_anchored_use_does_not_stand_in_for_the_sink_argument`,
`no_exact_route_finds_an_alternative_clean_route`, `no_route_rejects_a_nonexact_route`,
`a_missing_sink_use_fails_the_row` (a `not` row with no node at the sink bytes fails, it is not vacuously
green). `#[test] fn subscript_matrix_rows_have_truth()`: every row has a non-`inadmissible` truth, and every
`control` row a `base_verdict`. The RED record (rows failing when each group is enabled by hand on main) goes in
the ledger.

Seed rows (`/` separates lines):

| Row | Langs | Body | Truth | Group |
|---|---|---|---|---|
| P01 | py js go | `d["a"]=t W / d["b"]="safe" / sink(d["b"]) R` | not (`no_route`) | control |
| P02 | py js go | same, `sink(d["a"])` | reaches | S3 |
| P03 | py js go | `d["a"]=t W / d["a"]="clean" / sink(d["a"]) R` | not | control |
| P04 | py js | `d["u"]={"n":"s0","m":"s0"} / d["u"]["n"]=t W / d["u"]["m"]="safe" / sink(d["u"]["m"]) R`; P05 `sink(d["u"]["n"])` | not (`no_route`); reaches | control; S3 |
| P06 | js ts | `o["x"]=t / sink(o.x)` | reaches | S3 |
| P07 | js ts | `o.x=t / sink(o["x"])` | reaches | S2 |
| P08 | js | `o.x=t W / o["x"]="clean" / sink(o.x) R`; P09 `o.x=t W / o.y="safe" / sink(o.y) R` | not (P09 `no_route`) | control |
| P10 | go | `m["k"]=t W / sink(m["j"]) R`; P11 `sink(m["k"])` | not (`no_route`); reaches | control; S3 |
| P12 | py js | `k="q" / d[k]=t W / d["b"]="safe" / sink(d["b"]) R` | not (verdict may change in S5 only under `dynamic-edge`) | control |
| P13 | js | `k="q" / o[k]=t / sink(o.q)` | reaches | S5 (stage D item 6) |
| P14 | py js go | `d["a"]=t / sink(d)` | reaches | S6 |
| P15 | py js go | `d["a"]=t / x=d["a"] / sink(x)` | reaches | S3 |
| P16 | py | `d["a"]=t / e=d / sink(e["a"])` | reaches | S3; accepted-loss candidate (alias read) |
| P17 | py js go | `d={"a":t} / sink(d["a"])`; P20 `xs=[t] / sink(xs[0])` | reaches | control |
| P18 | py js go | `d={` / `"a":t,` / `"b":"safe"}` / `sink(d["b"])` | not | S7 |
| P19 | py js go | `d={"a":t,"b":"safe"} / sink(d["b"])` | not | S7; accepted-loss candidate (same-line propagation) |
| P21 | py js go | `d["a"]=t / d["a"]+="x" / sink(d["a"])`; P31 `d["a"]="" / d["a"]+=t / sink(d["a"])` | reaches | S3 |
| P22 | py js | `d["u"]=C() / d["u"].n=t W / d["u"].m="safe" / sink(d["u"].m) R` | not (`no_route`) | S3 |
| P23 | go | `s.x=t / m["k"]=s / sink(m["k"].x)` (`map[string]S`) | reaches | control |
| P24 | py js | `d["u"]=C() / d["u"].n=t / sink(d["u"].n)` | reaches | control |
| P25 | py | `d["a"]=C() / d["b"]=C() / d["a"].c=t W / d["b"].c="x" / sink(d["a"].c) R`, label `exact` | reaches | S3 |
| P26 | py | `d["a"]=C() / d["a"].c=t W / d["a"].c="clean" / sink(d["a"].c) R` | not | control |
| P27 | py js go | `o.a=t / sink(o.a)` (Go struct); P30 py `o=C(t) / sink(o.a)` (`C(v)` stores `v` in `.a`) | reaches | control |
| P28 | py js | `o.a=t / sink(o)`; P29 py `o.a=t / p=o / sink(p.a)` | reaches | accepted loss, pinned (DESIGN 8) |
| K01 | py | `d=D()` (`class D(dict)`) `/ d["a"]=t W / d.a="safe" / sink(d.a) R`; K02 `d.a=t W / d["a"]="safe" / sink(d["a"]) R` | not | control |
| K04 | js | `o["a-b"]=t / sink(o["a-b"])`; K05 `o["a-b"]=t W / o["a b"]="s" / sink(o["a b"]) R` | reaches; not (`no_route`) | S3; control |
| K06 | js | `o["1"]=t / sink(o[1])`; K07 `o[1]=t / sink(o["1"])` | reaches | S5 |
| K08 | js | `o["\x78"]=t / sink(o.x)` | reaches | S5 (item 6) |
| K09 | py go | `d["\x61"]=t / sink(d["a"])`; Go `m["\x6b"]=t / sink(m["k"])` | reaches | S5 |
| K10 | js | `` o[`x`]=t / sink(o.x) ``; K14 go `` m[`k`]=t / sink(m["k"]) ``; K13 py `d[r"a"]=t / sink(d["a"])`; K16 py `d["a" "b"]=t / sink(d["ab"])`; K15 js `o["__proto__"]=t / sink(o.__proto__)` | runner | key rule (S3 or S5) |
| K11 | py js | `p="a" / d[f"{p}"]=t / sink(d["a"])`; JS `` o[`${p}`] `` | reaches | S5 |
| K12 | py | `d[b"a"]=t W / d["a"]="s" / sink(d["a"]) R` | not | control |
| K18 | go | `const k = "a"` `/ m[k]=t / sink(m["a"])` | reaches | S5 |
| K19 | ts js | `o["x"]=t / sink(o?.["x"])`; K20 `o["$x"]=t / sink(o.$x)`; K21 `$o["x"]=t / sink($o.x)` | reaches | S3 (K21: refused root, accepted loss if DESIGN 5.1 refuses it) |
| L01 | py go; js | `d["a"]=t W / d["b"]="x" / sink(d["a"]) R`, label `exact` | reaches | S3; S4 |
| L02 | py js | `d["u"]={"n":"s0"} / d["u"]["n"]=t W / d["u"]={"n":"s0"} / sink(d["u"]["n"]) R` | not | S4 |
| L03 | js | `d["u"]={v:t} W / d["u"].w="x" / sink(d["u"]) R`, label `exact` | reaches | S4 |
| L04 | py js go | `k="a" / d["a"]=t W / d[k]="clean" / sink(d["a"]) R`, label `nonexact`; L05 `k="b"` | not; reaches | S3 |
| L06 | js | `o.x=t W / o["y"]="c" / sink(o.x) R`; L07 `o["a-b"]=t W / o.x="c" / sink(o["a-b"]) R`; label `exact` | reaches | S4 |
| L08 | js | `k="z" / o[k]=t W / o.x="c" / sink(o[k]) R` | reaches | item 4 |
| N01 | py | `d["a"]=t W / e=d / e["a"]="safe" / sink(d["a"]) R` | not | S3 (alias policy, DESIGN 5.3: `no_exact_route` under C and A; under B a pinned accepted loss) |
| N02 | py go | `d["a"]=t W / d={"a":"s0"} / sink(d["a"]) R` | not | S3; S4 (root kill) |
| N03 | js | `const u=o / o["x"]=t W / u["x"]="safe" / sink(o["x"]) R` | not | S3 (certified alias, `alias_unstable`) |
| Y01 | py js go | `k="a" / d[k]=t W / sink(d["a"]) R`; Y03 `k="a" / d["a"]=t W / sink(d[k]) R`; label `nonexact`; Y02, Y04 the same with `k="b"` | reaches; not | S5; control |
| Y05 | py js | `k="x" / d[k]["a"]=t / sink(d["x"]["a"])`; Y06 `k="n" / d["u"][k]=t / sink(d["u"]["n"])` | reaches | S5 |
| Y10 | py js | `i="x" / j="y" / d[i]["a"]=t W / sink(d[j]["a"]) R`; Y11 `j="x"`, label `nonexact` | not; reaches | S3 |
| Y12 | py js | `k="b" / d["a"]=t / f(d["b"], d[k])`, `f(p, q)` calls `sink(q)`; Y13 js `k="x" / o.x=t / sink(o[k])` | reaches | S5 |
| Y07 | py | `d[0]=t W / sink(d["0"]) R` | not | control |
| Y08 | py js go | `xs=["s0","s0"] / xs[0]=t / sink(xs[1])`; Y09 `sink(xs[0])` | not; reaches | control; accepted loss, pinned |
| X01 | js; go | `o["k"]=t W / delete o["k"] / sink(o["k"]) R`; X03 Go `m["k"]=t W / delete(m, "k") / sink(m["k"]) R` | not | S4; accepted-loss candidate |
| X02 | py | `d["k"]=t / del d["k"] / sink(d)`; X04 `d["k"]=t W / del d["k"] / d["k"]="c" / sink(d["k"]) R` | not | S6; control |
| T01 | js | `for (o["a"] of [t]) {} / sink(o["a"])`; T02 `[o["a"]]=[t] / sink(o["a"])`; T03 `({v:o["a"]}={v:t}) / sink(o["a"])` | reaches | S3 |
| T04 | py | `for d["a"] in [t]: pass / sink(d["a"])`; T05 `d["a"], x = t, 1 / sink(d["a"])` | reaches | S3 |
| A01 | js | `d["u"]={n:"s0"} / d["u"].n=t W / consume({slot:d["u"].n})`, `consume(p)` calls `sink(p.slot)`; A02 `consume2(d["u"].n)` | reaches | control (must stay green from S3) |
| E01 | py js go | extension rule (DESIGN 5.3), each with `d["u"]=C(t) W` (`C(v)` stores `v` in `.n`; JS `{n:t}`; Go `map[string]S`, `S{n: t}`): `f(d["u"].n)`, `f` calls `sink(p)`; E02 `x=d["u"].n / sink(x)`; E03 `h(t)` writes and `return d["u"].n`, `sink(h(t))`; E04 `sink(d["u"].get("n"))` (Python `{"n":t}`, JS a `Map`, Go a method on `S`) | reaches | S3 |
| E05 | js | `d["u"]={n:t} W / consume({slot:d["u"].n})`; E06 py js go `d["u"]={"n":t} W / sink(d["u"]["n"])`; E07 py js go `d["u"]=C(t) W / sink(d["w"].n)` (`d["w"]` starts as `C("s0")`) | reaches; reaches; not (`no_route`) | S3 |
| K22 | js | `o.é=t W / o["é"]="clean" / sink(o.é) R` (main `killed`); K23 `o["é"]=t / sink(o.é)` | not; reaches | control; S5 (stage D item 6) |
| I01 | py js go | multi-line literal, `"a": t,` on its own line, `sink(d["a"])` | reaches | S7 (control if main passes) |
| I02 | py js go | same, `"b": "s",` on its own line, `sink(d["b"])`; I04 `"a": (` / `t` / `),` split value, `sink(d["a"])` | not; reaches | S7 |
| I03 | py | `base={"a":t} / d={**base, "b":"s"} / sink(d["a"])` | reaches | control |
| R01 | py js | `d=make() / d["a"]=t W / sink(make()["a"]) R` | not | control |
| R09 | py | `a="s0" / make()["a"]=t / sink(a)` | not | S3 (main: known wrong, `Def(a)`) |
| R02 | py js go | `d["a"]=t / f(d["a"])` with `f` calling `sink(p)`; R03 `d["b"]="s" / f(d["b"])` | reaches; not (`no_route`) | S3; control |
| R04 | py js go | `self.d["a"]=t / sink(self.d["a"])` (JS `this.d`); R05 js ts `(d)["a"]=t`, `(d as any)["a"]=t`; R08 `if c: d["a"]=t / else: d["a"]="s" / sink(d["a"])`, `c` true | reaches | S3 |
| R06 | py js | `for i in range(2): / x=d["a"] / d["a"]=t W` then `sink(x) R` after the loop; R07 `d["a"]=t / def g(): sink(d["a"]) / g()` | reaches | S3 (R07: stage D may accept the loss) |
| M01 | py | `o={"x":C(),"y":C()} / i="x" / j="y" / o[j].z="safe" / o[i].z=t / sink(o[j].z)` | not | pinned known wrong (main `exact`), not widened |
| M02 | py | `o.x=t / o=C() / sink(o.x)`; M03 `p=o / o.x=t / p.x="safe" / sink(o.x)` | not | pinned known wrong (dot fields) |
| M04 | js | `put({a:t,b:"safe"})`, `put(q)` calls `sink(q["b"])` | not | pinned known wrong (lane 3's binder) |
| O01 | py js | `d["a"]=t / mutate(d) / sink(d["a"])` (`mutate` overwrites `"a"`) | not | out of model, pinned |
| O02 | py | `class R(dict)` whose `__setitem__` and `__getitem__` use one slot; `d["a"]=t / sink(d["b"])` | runner | out of model, pinned |
| O03 | js | setter `x`, Proxy, array `length=0`; `o["x"]=t / sink(o.x)` each | runner | out of model, pinned |
| O04 | py | `v={} / d={"a":v,"b":v} / d["a"]["x"]=t / sink(d["b"]["x"])` | reaches | out of model, pinned miss |
| C01 | rust java c | `v[0].x=t; sink(v[0].x)`; `a[0]=t; sink(a[0])`; `buf[i].x=t; sink(buf[i].x)`; Rust `m["k"]` read | n/a | control, pinned |

- [ ] Write rows, cases, runner, preludes, README (schema, markers, rules), route helpers and test. Run the
      preflight: every row executes once with one observation (`check.sh` green); every `control` row is green
      on main, else it is regrouped. RED record for the rest; commit; CI green with only `control` enabled. One
      reviewer (test-only).

### Task T-2: enumeration

- [ ] Tiers per ORCHESTRATION 6. Axes: language; role (write, augmented, logical, update, read, delete,
      destructuring and loop-head targets, receiver, key evaluation); key form by grammar (string kinds,
      prefixes, quote delimiters, escapes, interpolation, concatenation, templates, numbers, names, constants,
      expressions, slices, tuples, special JS keys); base (identifier, `self`/`this`, member chain, call,
      parentheses, TS wrappers, optional chain, `$` roots, comments inside the chain); order and position of
      field, keyed and dynamic components, paired static and dynamic at the same level; flow (direct, local,
      alias, augmented, loop-carried, branch, closure, whole read, argument, literal leaf, return); kills (same,
      other, prefix, root rebinding, deeper, dynamic, delete, alias write, captured write); identity (rebound,
      aliased, unproven base); multiplicity (many keys on one base with one wildcard, two reads on one line);
      initialisers (layouts, duplicates, spreads); semantics (ToPropertyKey, item versus attribute, overloaded
      access, maps versus slices, map aliasing); extension reads of a prefix definition (argument, local,
      return, receiver, literal leaf, nested, other key). Each census class gets a row. Two classes are not
      enumerated by hand: key spellings (closed by the spelling-pair table, DESIGN 7) and alias forms (enumerated
      after stage D chooses the alias policy, as rows O05 onward).

## Stage D: design of record (2 days)

The design seat amends DESIGN 5 from DESIGN, `rows.jsonl` with `truth.jsonl`, `CENSUS.md`, the research record,
the review and the accepted-loss candidates, and fixes against every row:

1. The keyed component's encoding under DESIGN 5.1's two conditions (injective with legacy field strings such
   as `["x"]` from `(o)?.["x"].f`; no existing output changes), its `Display`, `Debug` and JSON forms, and the
   key characters admitted (quotes, `]`, `.`, empty key, newline, control characters).
2. The per-language key rule: Python prefixes `r`, `u`, `b`, `f`, concatenation, triple quotes, non-ASCII; JS
   escapes, templates without substitution, numeric strings, `__proto__` (inside DESIGN 5.1's three classes);
   Go raw and interpreted literals; the refused roots (DESIGN 5.1, row K21). Every choice must keep
   `spelling_truth.jsonl` and the builder in agreement (S0's pair test).
3. The overlap label: `NameOnly(AliasUnstable)` or a new `FlowDoubt` (owner decision 4).
4. `replaces` and `may_overlap` for JavaScript (DESIGN 5.4, owner decision 3) and the label o28 takes. Pinned
   tests it changes: `semantic_tests.rs::ident_on_every_target_shape` ("any index is []"),
   `semantic_tests.rs::affects_and_eq_relations`, `census_defs_tests.rs::
   affects_is_prefix_with_a_wildcard_on_either_side`, `affects_tests.rs` (`the_trie_yields_exactly_the_affected_defs`,
   `bucketed_recorded_affects_matches_the_all_pairs_scan`, `writer_flags_by_receiver_and_binding`),
   `recorded_kill_tests.rs` with `variant-matrix.tsv` o27 and o28, `sites.tsv` rows whose source has a static
   key, and the 660 design's §4.4 note.
5. The alias policy (DESIGN 5.3: C default, A, B) on C6's `exact` yield under each, with rows N01 to N03 and M02;
   the base predicates of DESIGN 5.3 confirmed on N01, N02 and M02; root and prefix kills of keyed definitions;
   each policy-dependent `label` collapsed to the chosen policy, and the alias-form enumeration briefed.
6. Whether, in JavaScript, `[]` matches field components in the reach rule (P13, K08, K23, Y13), with C1's count.
7. Whether S6 and S7 run (C3, C4; P14, X02, P18, I01, I02, I04), their labels, S6's JS field-origin sources and
   S7's container-definition edges (DESIGN 5.3, 5.5). The passing languages and S5's go are the stage-M memo's.
8. Deletes (X01 to X03); whether a closed list of container mutators (Python `d.pop`, `d.popitem`, `d.clear`,
   `d.update`, `d.setdefault`; Go `clear(m)`) is a possible overwrite (review r2 I8); the accepted-loss list.
9. Dynamic-only `d[i] = t; sink(d[j])`: a promised may-flow or an accepted miss (DESIGN 5.3).

The exit is one revision that amends DESIGN, this plan's slice sections, gate predicates and differential
classes, and each row's `group` and `label`; implementer briefs are cut from that revision (ORCHESTRATION 6).
Both reviewers; cap 2 rounds.

---

## Stage I: slices

Order: S0 builds the tool with no behaviour change. S1 routes every producer and consumer through one path
function that still returns today's paths (zero difference). S2 adds real-span uses for admitted subscript
reads (isolated nodes, except JS dot definitions reaching identifier-key reads). S3 flips the path function:
keyed definitions, the respelled field-access reads, the extension rule, the definite-identity label rule and
every consumer change together, onto S2's real spans, so the matrix and the no-flood gate bind by bytes from the
first keyed edge. S4 changes kills and refines S3's owner-wide overlap labels to routes. S5 (its own go, stage
M) adds wildcard reach for dynamic keys. S6 and S7 wait for stage D's go. Each slice applies only to the passing
languages. If the lane stops before S5 merges, the owner accepts S3's `mixed-key-interim` and
`prefix-overwrite-interim` losses by name or S3 is reverted. **Gates of every slice (DESIGN 7):** Matrix (its
group, every earlier group and `control`, with `control` verdicts pinned), Differential (against the previous
slice: its own mechanical classes only; against `prism-base`: cumulative classes listed), No flood (`cross-key`
0; `orphan-anchor` 0 from S2), Spellings (changed fixture rows and golden files listed; `spelling-leak` 0),
Sample on each slice that adds or relabels edges (S2, S3, S4, S5, S6, S7), before merge. Engines run at
stage G.

### Slice S0: node-based path builder, used by a trace only (1 to 2 days)

**Files:** create `src/ast/subscript_paths.rs`, `src/ast/subscript_paths_tests.rs`,
`src/data_flow/build/subscript_trace.rs`, `src/data_flow/build/subscript_trace_tests.rs`; modify `src/ast.rs`
and `src/data_flow/build/mod.rs` (module lines), `src/access_path.rs` (helpers and unit tests),
`src/data_flow/build/file.rs` (one `subscript_trace::trace` call per owner beside `param_alias_trace::trace`,
gated before any traversal when the variable is unset).

```rust
// src/access_path.rs — encoding per stage D item 1
pub fn keyed_component(key: &str) -> String;
pub fn keyed_key(component: &str) -> Option<&str>;   // None for a field, for `[]` and for every legacy string

// src/ast/subscript_paths.rs
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum IndexComponent { Field(String), Keyed(String), Dynamic }
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChainPath { pub path: AccessPath, pub static_keys: usize, pub dynamic: usize }
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Chain { Unaffected, Admitted(ChainPath), Refused(&'static str) }
impl ParsedFile {
    /// Python `subscript`, JS/TS/TSX `subscript_expression`, Go `index_expression`, or a field access over one.
    /// Unaffected without a static key; Refused with one when the root is not a plain identifier, `self` or
    /// `this` (parentheses, TS `!`/`as`/`satisfies` peeled) or the chain is one DESIGN 5.1 refuses.
    pub(crate) fn subscript_chain(&self, node: tree_sitter::Node<'_>) -> Chain;
    pub(crate) fn subscript_chain_at(&self, start: usize, end: usize) -> Chain;
    /// The admitted path, else None (callers keep today's text path only for Unaffected).
    pub(crate) fn static_key_path_at(&self, start: usize, end: usize) -> Option<AccessPath>;
    pub(crate) fn index_component(&self, subscript: tree_sitter::Node<'_>) -> IndexComponent;
}
```

The JS key rule copies the three classes of `literal_key` in `src/cpg/object_literal_args.rs` (DESIGN 5.1:
ASCII `plain_identifier` other than `__proto__` → field; ASCII without backslash that fails the identifier test →
keyed; non-ASCII, backslash, template, `__proto__` → `Dynamic`), as amended at stage D. With `PRISM_SUBSCRIPT_TRACE=1`,
`trace` prints one stderr line per subscript-bearing def or use span:
`subscript-site\t<file>\t<owner>\t<line>\t<start>\t<end>\t<def|use>\t<recorded path>\t<chain path|refused|->`;
with `PRISM_SUBSCRIPT_TRACE=all` it prints every def and use span (the node inventory the gates diff, isolated
nodes included). The logic sits in a pure `trace_lines(…) -> Vec<String>`.

- [ ] Tests first, against a skeleton returning `Unaffected` (RED): `key_rule_table` (one row per language and
      key form of item 2); `chains_keep_every_component` (`d["u"]["n"]`, `d["a"].c`, `o.items["k"]`,
      `d[k]["a"]`, JS `o["x"].y` → `o.x.y`, Go `m["k"].x`); `no_static_key_is_unaffected` (`d[k]`, `d[0]`,
      `d[k].c`, `o.x`, Rust, Java and C subscripts); `refused_chains_are_refused` (`make()["a"]`,
      `make()["b"].field`, `(a or b)["k"]`, `$o["x"]`, a comment inside the chain);
      `chain_outcome_matches_an_independent_check` (every field-access and subscript node of every
      `.py .js .ts .tsx .go` file under `tests/fixtures/`: Admitted exactly when an independent check finds a
      string-literal index and an admitted root, Refused when it finds the index and a refused root);
      `at_requires_exact_span`; `js_non_ascii_key_is_dynamic` (`o["é"]` → `Dynamic`, `o["a-b"]` → keyed,
      `o["x"]` → field); `spelling_pairs_agree_with_runtime` (reads
      `tests/fixtures/subscript_matrix/truth/spelling_truth.jsonl` through `CARGO_MANIFEST_DIR`: for every pair,
      the two components are equal exactly when `same`, or one is `Dynamic`; a missing or empty file fails);
      in `access_path.rs` `keyed_component_round_trips`,
      `keyed_key_is_none_for_legacy_strings` (`["x"]`, `[]`, `"[]"`, embedded dots, quotes and brackets),
      `display_debug_and_json_of_keyed_component`; `trace_lines_names_def_and_use_sites`,
      `trace_all_lists_isolated_nodes`, `trace_off_allocates_nothing`.
- [ ] Implement; GREEN. Differential against `prism-base` on the seven corpora: zero change in edges, counters
      and `nav nodes-at`/`taint-reaches`/MCP golden output. Tier-A; two reviewers.

### Slice S1: one path function for every producer and consumer, zero difference (2 to 3 days)

**Files:** create `src/ast/path_seam.rs` and `src/ast/path_seam_tests.rs` (module lines in `src/ast.rs`); modify
the producers `src/ast/lvalues.rs` (`extract_lvalue_spans_from_node`, `extract_for_in_lvalue_spans`,
`extract_for_in_lvalues`, `extract_destructuring_def_spans`, `extract_destructuring_defs`: the field/index
branch), `src/ast/rvalue_identifiers.rs` (the field-access branch of the four walkers),
`src/ast/member_reference_index.rs` (`Index::build`) and `member_reference_index/oracle.rs` (`collect_path_refs`,
`collect_fenced_path_refs`), and the consumers `src/cpg/build.rs` ~1639 (`full_arg_path`, call-site edit only),
`src/cpg/callee_legacy.rs` ~105, `src/cpg/step5b_reference.rs` ~81, `src/cpg/object_literal_args.rs` ~497 (lane
3, one lookup), `src/cpg/step5c.rs` ~172, `src/cpg/step5c_reference.rs` ~174, `src/cpg/reaching/capture.rs`
~169 (`collect_reference_identities`), `src/cpg/reaching/capture_regions.rs` ~446. Each site keeps its own
fallback chain inside the `today` closure. No cache bump.

```rust
// src/ast/path_seam.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) enum PathSite { Lvalue, ForHead, Destructuring, Rvalue, MemberIndex, MemberIndexOracle, Step5b,
    Step5bLegacy, Step5bReference, ObjectLiteralLeaf, Step5c, Step5cReference, CaptureIdentity, CaptureRegion }
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum PathAnswer { Path(AccessPath), Refused, Absent }
impl ParsedFile {
    /// The path recorded or looked up for the expression at exactly `start..end`. S1: `today()` as is
    /// (`Absent` for `None`). S3: the admitted chain path in a passing language, `Refused` for a refused chain,
    /// `today()` otherwise.
    pub(crate) fn path_at(&self, site: PathSite, start: usize, end: usize,
        today: impl FnOnce() -> Option<AccessPath>) -> PathAnswer;
}
```

Under `cfg(test)` the seam logs `(PathSite, start, end)` in a thread-local. Alias resolution composes paths and
looks nothing up at bytes; it changes structurally in S3.

- [ ] Tests first (RED against a skeleton that is never called): `every_site_calls_the_seam` (one fixture per
      language with a subscript and a field chain in every position: assignment target, augmented target, `for`
      head, destructuring leaf, read, argument, literal leaf, return, capture; building the DFG and the CPG logs
      every `PathSite` variant at least once); `seam_returns_today_for_every_site` (each logged call's answer
      equals its `today()`).
- [ ] Implement; GREEN. Differential against `prism-S0` on the seven corpora: zero change in edges, counters,
      the node inventory and `nav nodes-at`/`taint-reaches`/MCP golden output. Tier-A; two reviewers.

### Slice S2: subscript reads as real-span uses (2 to 2.5 days, sample included)

**Files:** create `src/ast/subscript_reads.rs` (declared in `src/ast.rs`) with `fn push_static_key_read(&self,
node, out)`: for the outermost admitted node of a chain that is a plain subscript read (a field access over a
subscript keeps today's recorded path until S3) it pushes the path span over the node with the builder's path,
and the walker then recurses into the children as today (base and index uses stay); inner nodes of the same chain
are not pushed; a write-only target is never registered as a read, in every language. Modify
`src/ast/rvalue_identifiers.rs` (one call per walker), `src/ast/member_reference_index.rs` and
`member_reference_index/oracle.rs` (register these reads), matrix `"S2"`, `src/cpg_cache.rs`, docs. No definition
is respelled, so a new use is isolated except where an existing JS dot definition has the same path (P07).

- [ ] Tests first (RED): matrix `S2`; in `src/ast/subscript_paths_tests.rs` `outermost_chain_node_is_the_read`
      (`d["u"]["n"]` gives one use, `d.["u"].["n"]`, at the outer bytes), `write_only_subscript_target_is_not_a_read`
      (py, js, go), `field_access_over_subscript_keeps_today_until_s3` (`d["a"].c` still `d.[]`).
- [ ] Implement. GREEN. `cargo test -p prism-mcp --test mcp golden_transcript_test` must not change; if it does,
      re-record with `PRISM_MCP_BLESS_GOLDEN=1` and name each changed assertion in the PR.
- [ ] Differential against `prism-S1`, classes: `subscript-use-added` (a new real-span use with a lane-built path
      and no edge), `js-field-read-edge` (an edge from an existing JS dot-field definition into a new
      identifier-key read use, labelled by today's rule); zero-gates `cross-key`, `spelling-leak`,
      `orphan-anchor`. Sample on the `js-field-read-edge` edges. Tier-A; cloud tests; two reviewers.

### Slice S3: keyed definitions, the extension rule, definite identity, consumers (4 to 5 days, sample included)

**Files:**
- Modify `src/ast/path_seam.rs`: `path_at` answers DESIGN 5.1 for the passing languages. The producers act on
  the answer: `src/ast/lvalues.rs` pushes one `PathSpan` over the node for `Path`, nothing for `Refused` (the
  target is consumed; its inputs stay reads of the rvalue walkers), today's branch for the `today` fallback, in
  every entry point S1 routed (both `assignment_lvalue_paths_on_lines` and `assignment_lvalue_spans_on_lines` in
  `src/ast/assignments.rs` reach it, a test each); `src/ast/rvalue_identifiers.rs` gives a field access over a
  subscript its admitted path and no path use for `Refused`; the index and its oracle key field-access nodes by
  the same answer. Every S1 consumer follows through the seam.
- Modify `src/data_flow/build/aliases.rs` (`resolve_path`, `resolve_on_alias_span`: append keyed components
  structurally, never by reparsing).
- Create `src/data_flow/build/subscript_occurrences.rs` (declared in `src/data_flow/build/mod.rs`) with
  `subscript_occurrences_tests.rs`: per owner, the admitted read occurrences (path, start, end) indexed by
  `(base binding, depth)` and per position by static component, with a prefix lookup; the extension rule of
  DESIGN 5.3 emits, for each lane-built definition of `p`, an edge to every admitted read occurrence that strictly
  extends `p`, through the same use lookup and prune checks as exact ones, deduplicated per edge pair. S5 adds the
  wildcard bucket to the same table.
- Create `src/data_flow/build/subscript_overlap.rs` (declared in `src/data_flow/build/mod.rs`) with
  `subscript_overlap_tests.rs`: the per-language base predicates of DESIGN 5.3 (Python a bound-once census over
  `python_prune::is_binding_occurrence`, made `pub(super)`, plus `stable_receivers`; JS `js_prune::stable_bases`
  and the `affects_writers.rs` eligibility; Go `BindingFacts`, `scope_prune::is_plain_write_site` and
  `scope_prune::address_taken_names`), the alias policy (stage D; default C: an alias-derived definition,
  `push_rd_def(.., true)`, whose resolved path is identical or may overlap is a possible overwrite; JS
  `alias_cert.rs`), and the S3 overlap rule, flow-insensitive: an edge from or to a lane-built path is joined
  (`worst`) with the overlap label (stage D item 3) when either endpoint fails definite identity or the owner holds
  any write in possible overlap with the definition's path, a possible overwrite of the alias policy, or (for an
  extension edge `p` → `q`) a write of a path `r` with `p` a prefix of `r` and `r` a prefix of `q`, or a strict
  static prefix of `p`. Applied after either RD arm in the producer's label maps (`src/data_flow/build/file.rs`,
  one call). S4 refines it to routes.
- `src/cpg/multiline_call_arg_parity_tests.rs` (a keyed-argument case: production, legacy twin and serial
  reference give identical edges in identical order); `tests/ast/subscript_matrix_test.rs` (`ENABLED` gains
  `"S3"`); `src/cpg_cache.rs`; docs.

JS `o["x"] = v` now records `o.x`; the census still says `o.[]`, so the site stays uncovered and its kill-only
`o.[]` def keeps today's wildcard kills until S4 (o27 and L01-js unchanged here).

- [ ] Tests first (RED): matrix `S3`; `subscript_overlap_tests.rs`: `dynamic_selector_pair_is_never_exact`
      (Y10, Y11), `rebound_base_is_never_exact` (N02), `python_bound_once_census` (a local bound once by `=`
      outside loops is definite; a second binding, a `for`, `with`, walrus or `except` target, a `global` or
      `nonlocal`, a binding inside a loop or comprehension is not), `go_definite_base_needs_one_declaration`
      (a reassignment, a flat-fallback binding or `&m` fails), `alias_map_twin_is_a_possible_overwrite` (N01 under
      policy C), `certified_js_alias_stays_alias_unstable` (N03), `owner_dynamic_write_downgrades_keyed_edge`
      (L04), `owner_prefix_write_downgrades_keyed_edge` (L02's shape, labelled overlap until S4);
      `subscript_occurrences_tests.rs`: `prefix_definition_reaches_extension_read_at_its_bytes` (E01 to E06,
      py, js, go: the edge ends at the argument's, local's, return value's, receiver's, leaf's and nested read's
      own bytes, never at a zero-width use), `prefix_definition_skips_other_key_extension` (E07);
      in `src/ast/subscript_paths_tests.rs`: `refused_call_root_records_no_definition` (R09: no `Def(a)`, no
      `Def(make)`, `Use(make)` kept; and no whole-text use for `make(a)["k"].c`),
      `for_of_head_subscript_target_defines_path` (T01), `array_pattern_subscript_leaf_defines_path` (T02),
      `python_for_and_tuple_targets_define_path` (T04, T05); consumer tests:
      `object_literal_leaf_keeps_exact_edge_after_rekey` (A01: `d.["u"].n` → `p.slot` `exact`, from the leaf's
      bytes), `plain_argument_keeps_exact_edge_after_rekey` (A02), `go_map_value_field_crosses_the_call` (P23),
      `keyed_return_keeps_its_return_value_shape` (Step 5c and its reference agree, return-flow counters equal);
      pins of readers left on text: `seed_base_comparison_unaffected_by_keyed_fields` (`seeds.rs`),
      `member_projection_unchanged_by_keyed_reads` (a formal read only as `p["x"]` stays mixed, `p["x"] = v` stays
      a computed write, M04 unchanged), `lexical_prune_never_drops_a_cross_key_pair` (`d["a"] = t; d["b"] = 1;
      sink(d["a"])`: rules U, D, S and O keep the `d.["a"]` pair, py, js, go),
      `for_init_hazard_unchanged_by_keyed_recorded_paths` (a `for (o["a"] = 0; …)` initializer keeps today's
      hazard verdict).
- [ ] Implement the seam's answer and the producers, then index and oracle, then walkers and consumers, then
      aliases, then the occurrence table and the overlap rule. GREEN.
- [ ] Prediction from C7 (reclassified with S0's trace), then differential on the seven corpora (`measure.sh`,
      `compare_subscripts.py`, against `prism-S2`). Classes, each mechanical: `keyed-def-respelled` (same bytes,
      `x.[]` → keyed or field path), `consumer-respelled` (argument, literal-leaf, return and capture endpoints,
      same bytes, label unchanged), `prefix-extension` (an edge main had, same bytes at both ends, now from a
      prefix definition to an extension read), `anchor-to-real` (an edge main had into a zero-width `x.[]` use now
      ends at the real span of that line's admitted occurrence), `keyed-edge-added` (each `exact` one under
      definite identity, every other one with the overlap label), `keyed-label-move` (Python/Go `killed` →
      `exact` where the killer wrote another static key, under definite identity), `cross-key-removed` (a removed
      edge whose endpoints are both admitted occurrences by bytes and whose component sequences differ at a static
      position), `identity-downgrade` (an `exact` edge at the same bytes now labelled overlap because its base
      fails the DESIGN 5.3 predicate or the alias policy; the trace names the failing clause),
      `refused-node-removed` (definitions and uses of whole-text or unrelated names on refused chains, R09),
      `into-overwrite-line` (as dot fields today), `mixed-key-interim` (edges main had between a keyed write and a
      dynamic read or the reverse through `x.[]`: listed in full, restored as overlap edges by S5, a row each),
      `prefix-overwrite-interim` (edges labelled overlap by the owner-wide prefix-write rule, `killed` by S4),
      `census-cap-interim` (JS owners that cross the completed-definition cap, 2,048, because a static-key write
      keeps a kill-only `o.[]` def beside its recorded def), `prune-counter`, `loop-plan`. Zero-gates (DESIGN 7):
      `cross-key`, `spelling-leak` (a changed path at bytes that hold no static-key subscript occurrence, on the
      node inventory) and `orphan-anchor`. Anything else exits 1. R: no change. Sample on the slice's added and
      relabelled edges.
- [ ] Rows K01, K02, P21, P23, R01, R09, A01, E01 to E07 (review focus 2 to 4, 7) pass. Tier-A; cloud tests at 1
      and 4 threads with a RED-on-base control; two reviewers. PR body: prediction against C7, class counts,
      `mixed-key-interim` and `prefix-overwrite-interim` lists, changed pinned rows, the lane 3 and lane 9
      coordination notes, Tier-A, the sample.

### Slice S4: kills and route-based overlap (3 to 4 days; Python and Go only 2 to 2.5)

JS's part (the census identity and `kill-narrowed`) runs only when the stage-M memo passes JS and its
different-key-kill count (DESIGN 6); otherwise S4 changes the non-JS kills and the route rule only.

**Files:** modify `src/ast/js_write_census/identity.rs` (`Comp` gains a keyed variant; `ident` gives a
static-key subscript `Comp::Name` or the keyed variant by the S0 rule, `Comp::Index` otherwise; `render` and the
two relations per item 4), `src/ast/js_write_census/completion.rs` (`access_path` maps the keyed variant with
`keyed_component`), `src/cpg/reaching/def_buckets.rs` (`affects` splits into `replaces` and `may_overlap`
(DESIGN 5.4); `Trie::affected` and `legacy_kill_sets_with` kill by `replaces`; in non-JS owners a recorded def
with a keyed component is killed by a `replaces` write of its path or any static prefix, the root included, and
an extension edge also by a write between its two paths (DESIGN 5.3), through a per-base trie built only when the
owner has one), the interference consumers `d3.rs`, `d2c.rs`, `refusal_classes.rs`, `alias_kill.rs`,
`census_defs.rs` (switch to `may_overlap`), `subscript_overlap.rs` (the owner-wide rule becomes the route rule of
DESIGN 5.3, reusing the may-kill route check of `alias_kill.rs`), the pinned tests of item 4, matrix `"S4"`,
`src/cpg_cache.rs`, docs (`CLAUDE.md` #674 and write-census paragraphs).

- [ ] Tests first (RED): matrix `S4`; `src/cpg/reaching/keyed_relation_tests.rs` (declared in
      `src/cpg/reaching.rs`): `replaces_and_may_overlap_tables` (must, may and unrelated pairs for every
      consumer above, same and different bindings, equal dynamic selectors), `trie_matches_the_all_pairs_scan`
      for both relations, `possible_overwrite_on_every_route_downgrades` and `one_clean_route_keeps_exact`,
      `root_rebinding_kills_keyed_def` (N02), `write_between_prefix_and_extension_kills` (L02, E-shapes),
      `o27_still_kills`, `o28_never_exact`, `js_non_ascii_bracket_write_still_kills_the_dot_def` (K22).
- [ ] Implement. GREEN. Pinned-test updates in one commit that lists every changed assertion and fixture row.
- [ ] Differential against `prism-S3`: labels only, edge multiset identical. Classes `kill-narrowed` (JS
      `killed` → `exact`, each killer on the route a static-key write of another key; under definite identity
      when an endpoint is lane-built, unconditionally for a dot-def to dot-use edge, whose kill came only from the
      wildcard), `kill-prefix` (Python/Go `exact` or overlap → `killed` by a prefix, root or in-between write,
      `prefix-overwrite-interim` resolved), `overlap-route` (S3's owner-wide overlap label lifted to `exact` where
      no route crosses a possible overwrite), `killed-to-overlap` (a former kill by a possible-overlap write: o28,
      a `[]` write over a keyed def), `census-covered` (trace `PRISM_RD_CENSUS_TRACE=1`, `kill:affects`),
      `census-shift` (labels moved by census coverage, refusal, same-line, captured or alias doubt, or the
      completed-definition cap after site changes, each traced). Sample on the relabelled edges. Tier-A; cloud
      tests; two reviewers.

### Slice S5 (stage M go): dynamic-key reach (4 to 4.5 days, sample included)

Runs per passing language whose C1 dynamic class meets DESIGN 6's go. The rule is DESIGN 5.3's third row and the
dynamic-read paragraph: a dynamic-only write reaches admitted keyed reads of its base binding at its depth with
equal static components before the dynamic position (dot fields only if stage D item 6 admits them), only in an
owner where the base has a lane-built occurrence; no edge into a lane-registered dynamic read is `exact`.

**Files:** extend `src/data_flow/build/subscript_occurrences.rs` with the wildcard bucket, so a definition is
compared only with candidates of its own keys and of the wildcard bucket (JS members too if item 6 says so), and
`find_wildcard(owner, path, def_line, def_byte, reads_only) -> Vec<(AccessPath, usize, usize)>`: every
admitted occurrence (path, start, end) in possible overlap (DESIGN 5.3; never two different static components
at one position), whether or not an exact match exists on the same line. Create
`src/data_flow/build/subscript_wildcard.rs` (declared in `src/data_flow/build/mod.rs`): for a def span whose
`subscript_chain_at` is admitted with a dynamic component, or a dynamic-only def (`Unaffected`) whose base has a
lane-built occurrence in the owner, the occurrences become edges through the same use lookup and prune checks as
exact ones, deduplicated per edge pair and labelled by `subscript_overlap.rs`, never `exact`; a dynamic subscript
read of a base that has a lane-built definition gets a real-span use, and every edge into it, an existing
dynamic-only definition's included, takes the overlap label. Bound: at most 1,000,000 compared pairs per owner;
an owner over it gets no wildcard edges and is counted in persisted `RdFileStats` counters
`dfg_subscript_wildcard_pairs` and `dfg_subscript_wildcard_owners_capped` (in `nav dfg-stats` and
`call-stats.dfg_labels`). Modify `src/data_flow/build/file.rs` (two calls), matrix
`"S5"`, `src/cpg_cache.rs`. If
item 3 adds a `FlowDoubt`: `src/cpg/flow_confidence.rs` and every file `rg -l
dfg_label_nameonly_alias_unstable src crates tests` lists (counter, `--edges` name, stats goldens).

- [ ] Tests first (RED): matrix `S5`; `src/data_flow/build/subscript_wildcard_tests.rs`:
      `exact_and_overlap_reads_on_one_line_both_bind` (Y12), `many_keys_one_wildcard_is_linear` (1,000 keys and
      one wildcard on one base: compared pairs ≤ 2,000 by a work counter), `over_budget_owner_gets_no_wildcard_edges`,
      `wildcard_matches_the_all_pairs_scan`, `dynamic_read_gets_a_use_only_beside_a_keyed_def`,
      `dynamic_write_reaches_keyed_reads_of_its_depth_only`
      (Y01, Y05; `d[k] = t` reaches neither `d["a"]["b"]` at another depth nor `d.f`),
      `dynamic_write_without_lane_occurrence_is_unchanged` (an owner with `arr[i] = v` and no static key: today's
      nodes and edges), `edge_into_registered_dynamic_read_is_never_exact` (an existing `d[i] = v` def into
      `d[k]` beside a keyed def). Rows Y05, Y06, K08, K09, K11 (review focus 1, 5) and one row per S3
      `mixed-key-interim` shape pass.
- [ ] Implement. GREEN.
- [ ] Differential against `prism-S4`: `dynamic-edge` (added, never `exact`; count per corpus at most 1.5 times
      the C1 prediction and no owner over the cap, else the slice stops for the owner), `mixed-key-restored`
      (each S3 interim loss back as an overlap edge), `control-verdict-dynamic` (a `control` row's verdict moved
      only through a `dynamic-edge`, P12), `wildcard-counter`; `cross-key` 0; owner fan-out distribution of added
      edges in the receipt. Sample on the added edges. Tier-A; cloud tests; two reviewers.

### Slice S6 (stage D go): element write to whole-container read (3 to 4 days)

Create `src/cpg/reaching/element_recall.rs` (declared in `src/cpg/reaching.rs`, called beside
`contribution::analyze` at line ~605); emit beside `recall.emit` in `src/data_flow/build/file.rs` (~811); add
persisted `RdFileStats` counters `dfg_element_recall_*` (in `nav dfg-stats` and `call-stats.dfg_labels`);
matrix `"S6"`; cache version; docs. Modelled on `contribution.rs`: Python, JS/TS/TSX and Go owners with RD
(passing languages only); sources and consumers as DESIGN 5.3 (container reads) and stage D item 7 define them:
the source is the definition occurrence, the consumer a maximal value operand (the whole container or an
enclosing subcontainer), never a receiver, key, membership, length, identity, method or nested projection; a
write of the source path or a prefix, a delete or a root rebinding kills the contribution, a descendant write
weakens it; the 1,000,000-candidate budget and skip counters; cold, full-hit and partial-hit builds agree (a cache
test). Label per item 7. Tests first in `src/cpg/reaching/element_recall_tests.rs` (declared in
`src/cpg/reaching.rs`): `element_write_reaches_later_bare_read`, `receiver_of_sibling_read_is_not_a_consumer`
(`sink(d["b"])`), `enclosing_subcontainer_is_a_consumer`, `killed_element_gives_no_recall`,
`over_budget_owner_is_skipped`, `cold_full_and_partial_hits_agree`; then `"S6"` RED → GREEN. Class
`element-recall` against `prism-S5` (or the last merged slice); sample; Tier-A; cloud tests; two reviewers; PR
body as S3.

### Slice S7 (stage D go): static-key initialisers (3 to 4 days)

Create `src/ast/static_key_literals.rs` (declared in `src/ast.rs`); one call in `src/ast/lvalues.rs`; matrix
`"S7"`; cache version; docs. Eligibility and binding as DESIGN 5.5: per eligible entry a def of the entry's
keyed (JS: field) path whose input is the value expression's span, located from the assignment's initializer;
duplicate keys, last wins; the container's def and its edges as stage D item 7 decides; single-line literals
stay an accepted loss unless stage D says otherwise. Tests first in `src/ast/static_key_literals_tests.rs`:
`own_line_entries_get_keyed_defs`, `split_value_entry_is_ineligible` (I04), `entry_on_opening_line_refuses`,
`duplicate_key_last_wins`, `spread_or_computed_key_refuses`, `single_line_literal_unchanged`; then `"S7"` RED →
GREEN, with P18 or I02 a pre-change false positive removed. Class `initialiser-def` against the previous slice;
sample; Tier-A; cloud tests; two reviewers; PR body as S3.

---

## Stage G and H: gates and close-out (2 to 3 days)

- [ ] Final head: full matrix; differential against `prism-base` (cumulative) on the seven corpora, with
      intervening main changes isolated by the merge-base binary; `cross-key` 0; R unchanged.
- [ ] Sample: 200 added or relabelled edges of the cumulative change by `tools/sample_edges.py` (seed 10,
      stratified by rule, language, label, fan-out and owner; the same tool draws each slice's 100), five lines of
      context each, read by Opus-5.5 (high) under DESIGN 7's per-stratum criteria; unknowns and denominators
      reported. Gate ≥ 95% per stratum.
- [ ] Engines: per row, `taint_reaches` verdict and the route checks beside truth; `tools/review_taint_rows.py`
      records review taint per row (`prism --repo <row dir> --diff <one-line unrelated diff> --algorithm taint
      --format json`, research caveat 10), reporting applicability (whether any source and sink applied); no
      change required.
- [ ] Dead writes, base and head (DESIGN 6): `g/dead_writes.py <edges.jsonl> <sites.jsonl>` per concrete
      subscript write (target bytes from the census), dead when the definition at those bytes has no outgoing
      DataFlow edge in the same build's edge dump; counted by bytes, never by spelling; a census write with no
      Def at its bytes is `unobservable` and counted apart. The benchmark leg only from a benchmark run bound to
      the head build (lane 11's route, or one the controller produces), classified as the research's
      `writer-only`/`unattributed`, else reported as not measured. A change of spelling is never counted as a
      repair. `g/REPORT.md`: every gate with its numbers, per language, and the dropped languages with their
      census counts.
- [ ] Docs: `CLAUDE.md` (a keyed-subscript paragraph; #674 and write-census paragraphs),
      `docs/features/cpg/architecture.md:313` (resolved question 3), `js-ts-known-gaps.md` (§6 `p.x[k] = 1`,
      ~245, and a subscript entry), `docs/features/language-coverage/language-analysis-gaps.md`.
- [ ] Follow-up issues: DESIGN 8's items; the main defects of DESIGN 2 with the review's probes (rows R09, M01
      to M04): call-root targets defining unrelated names (fixed by S3 for static keys), dynamic-only `exact`
      across different runtime keys, Python dot fields `exact` across root rebinding and uncertified alias
      writes, the object-literal binder's `q["b"]` edge (lane 3); statement-scoped same-line propagation in
      `trace.rs` (P19); alias reads (P16, P29); one static-key policy shared with `object_literal_args::literal_key`.
      Report on the issue (close it if every gate holds); refresh `HANDOFF.md` (`~/.claude/handoff-template.md`)
      and `LEDGER.md`; offload raw dumps; delete `target/`.

## Acceptance for the lane

Every gate of DESIGN 7 holds on the merged head (matrix, per-slice differentials, no flood, sample ≥ 95%,
engines recorded, spellings listed); both reviewers approved every slice that ran; the
report is posted. A failing gate is reported with its numbers and the lane stops for the owner.

## Estimates (working days, review rounds included)

| Languages | M | T and E | D | S0 | S1 | S2 | S3 | S4 | G and H | Total | S5 (stage-M go) | S6, S7 (each, stage-D go) |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| all three pass | 3–4 | 4.5–5.5 | 2 | 1–2 | 2–3 | 2–2.5 | 4–5 | 3–4 | 2–3 | **23.5–31** | 4–4.5 | 3–4 |
| Python only | 3–4 | 3–3.5 | 1.5–2 | 1–2 | 2–3 | 1.5 | 3–4 | 2–2.5 | 2–2.5 | **19–25** | 3 | 2.5–3 |

No language passing stops the lane after M (3–4 days). S0 and S1 are the same work whatever passes (the builder
and the seam cover all three languages).
