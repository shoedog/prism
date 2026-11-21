# Lane 10: subscripts in the data-flow graph — implementation plan

> **For agentic workers:** this plan is run by an orchestrator under `../ORCHESTRATION.md`. Each task is
> dispatched to an implementer (Sonnet-5.5, high) with its section as the brief. Steps use checkbox syntax.

**Goal:** a value written through a string-literal subscript reaches later reads of the same element in
Python, JavaScript/TypeScript and Go; a write to one key neither reaches nor kills a read of another; dynamic
keys keep today's may-meaning; every rule is checked against executed truth and a labelled corpus sample.

**Architecture:** one node-based path builder (`src/ast/subscript_paths.rs`) answers unaffected, admitted or
refused for every subscript chain; producers use it for chains that hold a static key and keep today's text
path otherwise. Definitions, the member index, the definite-identity label rule and every consumer of the new
spellings switch together first, then real-span reads, then kills, then a wildcard reach rule for dynamic keys;
container reads and initialisers are conditional.

**Tech stack:** Rust (core crate: `src/access_path.rs`, `src/ast`, `src/data_flow/build`, `src/cpg`);
tree-sitter Python 0.23.6, JavaScript 0.23.1, TypeScript 0.23.2 and Go 0.25.0 + PR #193 (both vendored);
Python 3.12 (uv), Node 24.15 and Go 1.26.2 for the truth runner. The census runs on Prism's own parser.

**Spec:** `DESIGN.md` here (5.1 to 5.7 the rules: 5.3 identity and overlap fixed, the rest tentative until
stage D; 6 the census; 7 the gates; 9 the owner decisions, whose defaults this plan applies).

## Global constraints

- Worktree `~/code/prism-subscripts`; evidence `~/prism-evidence/subscripts/`.
- Corpora (`~/code/bench-repos`): P1 `apache-superset` at `7519fa8b6f`, P2 `saleor-saleor` at `cc4aea76e3`
  (Python); X Excalidraw at `0642e72c` (`~/prism-evidence/inputs/excalidraw-0642e72c/source`); T
  `TypeScript/src` at `7964e22f2b`; G1 `caddy` at `77e9ce7404`, G2 `etcd` at `61d518f55e` (Go); control R
  `ripgrep` at `82313cf958` (Rust, unchanged by the lane: zero edge and counter change on every slice).
- **Not changed:** `AccessPath::from_expr` and its tests; the path of any chain without a static-key subscript
  (byte-identical); integer indices and slices; languages other than Python, JS, TS, TSX, Go;
  `src/cpg/trace.rs` (PR #841, lane 8); review taint (`crates/prism-review/src/algorithms/taint*`,
  `taint_sinks/`); the Go field-recall pass (`src/cpg/reaching/contribution.rs`, `go_aggregate.rs`: S5's
  model, not edited); `src/reasoning/seeds.rs` (compares an argument's base only; S1 adds a test that proves it
  unaffected); `src/cfg/js_loops/for_init.rs` (spells every bracket `[]` from syntax, stays conservative).
- **Shared with other lanes (coordinate before the PR, record on both issues):** `src/cpg/object_literal_args.rs`
  (lane 3): S1 changes the one lookup at line ~497 to the new path; the Step 5b triplet (lane 9): S1 changes the
  argument path lookup only; the CPG cache version (every semantic lane). The orchestrator's section 11 table
  lists these.
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
  `cache_versions_are_pinned_for_cpg_semantics`; S0 bumps nothing. Tier-A applies to S0 to S6, the corpus
  differential to S1 to S6. Executed fixtures are lane-written only.
- Differentials (DESIGN 7): each slice is compared with the previous slice's binary (`bin/prism-S<n-1>`;
  `prism-base` for S0; when main moved under the slice, the binary of its actual merge parent) for attribution,
  and with `prism-base` for cumulative impact. Every gate names its comparison. Each receipt records both product
  SHAs, the corpus hashes and the class counts.
- The member index and its oracle change together (`Index::build` in `src/ast/member_reference_index.rs`;
  `collect_path_refs` and `collect_fenced_path_refs` in `member_reference_index/oracle.rs`), or
  `member_index_differential_edges_and_non_js` fails.

## Review focus

1. A key with an escape or interpolation (`d["\x61"]`, `o["\x78"]`, `d[f"{p}"]`, `` o[`${p}`] ``) is never a
   keyed component; it connects only through the dynamic rule, never `exact`. Rows K08, K09, K11 (S4).
2. `d["a"]` and `d.a` on one Python object (a `dict` subclass) stay apart both ways. Rows K01, K02 (control,
   confirmed in S1).
3. Augmented assignment through a subscript: `d["a"] = t; d["a"] += "x"; sink(d["a"])` reaches; so does
   `d["a"] = ""; d["a"] += t`. Rows P21, P31 (S1).
4. A subscript on a call result (`make()["a"]`) gets no path, definition or edge, and no definition of any name
   in it. Rows R01, R09 and the S0 test `refused_chains_are_refused`.
5. A nested subscript with a dynamic key (`d[k]["a"]`, `d["u"][k]`, `d[i]["a"]` against `d[j]["a"]`) never
   gives an `exact` edge or a kill. Rows Y05, Y06, Y10, Y11.
6. Definite identity (DESIGN 5.3): a rebound or uncertified-aliased base gives no `exact` keyed edge. Rows N01
   to N03.
7. Consumers move with producers: the existing `exact` literal-leaf transfer into `p.slot` survives. Row A01.

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
      `cross-key` and `spelling-leak` (definitions in S1); the classes each slice names; exit 1 on an
      unclassified change. Self-test `tools/test_compare_subscripts.py` on hand-made dumps: a cross-key edge
      into the second argument of `f(d["a"], d["b"])` fails; `o["x"] → o.x` passes; a wrong edge through a
      local, a respelled dynamic-only node on a static-key line and an unjoinable endpoint are each caught.

### Task M-2: inventory and counts C1 to C4

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
      Plus eligible container literals (DESIGN 5.5 layout) and whole-container value reads (DESIGN 5.3 roles).
- [ ] Golden: `census/golden/` small Python, JS and Go files with exact expected per-owner counts (mutually
      exclusive arms give no pair, a rebinding or an early return between W and R gives none, a JS dot read
      counts, a receiver-only read is no whole-container read); the census must match exactly.
- [ ] Validation, both directions: 200 sampled reported sites against `prism-base nav --cache-dir
      ~/prism-evidence/subscripts/navcache nodes-at --repo C --location f:l --format json` (a write needs a Def
      at the target bytes; a read is confirmed only by a node whose bytes lie in the subscript, never by a
      receiver Use alone); every subscript node of 30 sampled files by an independent tree walk against the
      census records; 100 sampled C1 same-key pairs read against the code for pair precision (Wilson lower
      bound). Agreement ≥ 95% on sites and files, else fix the census.
- [ ] `count.py`: C1 to C4 as DESIGN 6 defines them, each owner once per class.

### Task M-3: C5 and the memo

- [ ] `census/c5_dead.py <edges.jsonl> <sites.jsonl>`: per concrete subscript write (target bytes from the
      census), dead when the definition at those bytes has no outgoing DataFlow edge in the same build's edge
      dump; counted by bytes, never by the `"[]"` spelling. Control: a dot-field write that reaches its read is
      live, a census write with no Def at its bytes is `unobservable` and counted apart.
- [ ] `census/c5_bench.py`: in `~/prism-evidence/meas/secbench/repair-r1/final/entries.jsonl` (record the build
      in `binding.json`), cases with outcome `reached_function_only`, `partial` or `not_reached` whose recorded
      frontier holds the Def at a subscript write's bytes; per case, `writer-only` (every other frontier item is
      that write's own receiver or key read) or `unattributed`. This is a base-only classification: a head
      figure needs a benchmark run bound to the head build (stage G).
- [ ] `census/c5_diffs.py`: the 120 diffs of #602 (lane 11's `populations/602-m2-120.json` if merged, else
      `population.jsonl` from `~/prism-evidence/archive/prism-crossfile-docs-493ab5ee-receipts.tar.gz`, member
      `next-five-plan-fable-20261114/final-gate-campaign-preparation/`). Per diff: check out the head in a
      scratch worktree, run the edge dump, and apply `c5_dead.py` to the added string-key subscript writes (the
      first 20 per diff; report the selection and the uncovered count).
- [ ] `CENSUS.md`: C1 to C5 per corpus and language with the validation figures, the decision lines, posted on
      the issue.

**Exit gate:** scaled C1 same-key owners ≥ 500 across the six corpora (DESIGN 6); else post, stop, wait for the
owner. C3 ≥ 300 makes S5 a candidate, C4 ≥ 300 makes S6 one; stage D decides with the rows.

---

## Stage T and E: executed matrix (4 to 5 days)

### Task T-1: rows, truth runner, data-driven test

**Files:**
- Create under `tests/fixtures/subscript_matrix/`: `rows.jsonl`, `cases/<id>.{py,js,ts,go,rs,java,c}`,
  `README.md`, `truth/run_truth.py`, `truth/prelude.{py,mjs,go}`, `truth/test_run_truth.py`,
  `truth/truth.jsonl` (committed output), `truth/check.sh`. Create `tests/ast/subscript_matrix_test.rs`.
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
`inadmissible` row: such a row is repaired, never mapped to `not`. `test_run_truth.py` (`python3 -m unittest`):
reaching row → `reaches`; throwing row → `inadmissible`; two observations → `inadmissible`; nested marker
found; a case naming `os`, `subprocess`, `child_process`, `socket` or `net/http` is refused. Runs locally
through memguard.

`rows.jsonl`: `id` unique per program and language (`P01.py`), `group` (`control`, `S1` … `S6`), `lang`, `file`,
`class`, `census` (C1 to C4 class), optional `label` (`exact`, `killed`, `nonexact`: on the edge between the W
and R occurrences, joined by bytes), optional `pinned_main` (main's verdict and route: known-wrong rows and
accepted losses; an out-of-model row pins the answer its contract gives, DESIGN 5.7), optional `accepted_loss` (reason, issue). Truth comes only from `truth.jsonl`. A
`control` row is one where main's answer already agrees with truth; a row where they disagree is RED and
belongs to the first slice that can make it green.

`subscript_matrix_test.rs`: `const ENABLED: &[&str] = &["control"];` and `#[test] fn subscript_matrix_rows()`.
Per enabled row it writes the case to a temp repo and builds a `NavigationSession` (copy the `fixture` helper
of `tests/reasoning/taint_reaches_test.rs`) and a `DataFlowGraph` (labels via `crate::common::dfg_labels`).
Required (DESIGN 7), with the marked definition at the `t = src()` target bytes and the sink use at the sink
argument's bytes:
- truth `reaches` → `taint_reaches` from the `src()` line to the `sink(` line is `Reached`, and
  `route_through(dfg, marked, w_bytes, sink_use)` holds (any labels);
- truth `not` → `no_exact_route(dfg, marked, sink_use)`: no route over `exact` DataFlow and argument edges plus
  same-owner same-line use-to-definition transfers;
- `label` checked on the W→R edge pair; `pinned_main` takes precedence over truth for the assertion and the row
  is listed in the test output.

The route helpers sit in `tests/ast/subscript_matrix_routes.rs` (declared in `tests/ast/main.rs`) with
self-tests on synthetic edge lists: `no_exact_route_rejects_an_indirect_all_exact_route` (the review's
`t → o.x → tmp → sink` shape), `route_through_rejects_a_route_that_skips_w`, `routes_bind_by_bytes_not_line`
(two uses on one line), `an_anchored_use_does_not_stand_in_for_the_sink_argument`,
`no_exact_route_finds_an_alternative_clean_route`. `#[test] fn subscript_matrix_rows_have_truth()`: every row
has a non-`inadmissible` truth. The RED record (rows failing when each group is enabled by hand on main) goes
in the ledger.

Seed rows (`/` separates lines):

| Row | Langs | Body | Truth | Group |
|---|---|---|---|---|
| P01 | py js go | `d["a"]=t W / d["b"]="safe" / sink(d["b"]) R` | not | control |
| P02 | py js go | same, `sink(d["a"])` | reaches | S1 |
| P03 | py js go | `d["a"]=t W / d["a"]="clean" / sink(d["a"]) R` | not | control |
| P04 | py js | `d["u"]={"n":"s0","m":"s0"} / d["u"]["n"]=t W / d["u"]["m"]="safe" / sink(d["u"]["m"]) R`; P05 `sink(d["u"]["n"])` | not; reaches | control; S1 |
| P06 | js ts | `o["x"]=t / sink(o.x)`; P07 `o.x=t / sink(o["x"])` | reaches | S1 |
| P08 | js | `o.x=t W / o["x"]="clean" / sink(o.x) R`; P09 `o.x=t W / o.y="safe" / sink(o.y) R` | not | control |
| P10 | go | `m["k"]=t W / sink(m["j"]) R`; P11 `sink(m["k"])` | not; reaches | control; S1 |
| P12 | py js | `k="q" / d[k]=t W / d["b"]="safe" / sink(d["b"]) R` | not | control |
| P13 | js | `k="q" / o[k]=t / sink(o.q)` | reaches | S4 (stage D item 6) |
| P14 | py js go | `d["a"]=t / sink(d)` | reaches | S5 |
| P15 | py js go | `d["a"]=t / x=d["a"] / sink(x)` | reaches | S1 |
| P16 | py | `d["a"]=t / e=d / sink(e["a"])` | reaches | S2; accepted-loss candidate (alias read) |
| P17 | py js go | `d={"a":t} / sink(d["a"])`; P20 `xs=[t] / sink(xs[0])` | reaches | control |
| P18 | py js go | `d={` / `"a":t,` / `"b":"safe"}` / `sink(d["b"])` | not | S6 |
| P19 | py js go | `d={"a":t,"b":"safe"} / sink(d["b"])` | not | S6; accepted-loss candidate (same-line propagation) |
| P21 | py js go | `d["a"]=t / d["a"]+="x" / sink(d["a"])`; P31 `d["a"]="" / d["a"]+=t / sink(d["a"])` | reaches | S1 |
| P22 | py js | `d["u"]=C() / d["u"].n=t W / d["u"].m="safe" / sink(d["u"].m) R` | not | S1 |
| P23 | go | `s.x=t / m["k"]=s / sink(m["k"].x)` (`map[string]S`) | reaches | control |
| P24 | py js | `d["u"]=C() / d["u"].n=t / sink(d["u"].n)` | reaches | control |
| P25 | py | `d["a"]=C() / d["b"]=C() / d["a"].c=t W / d["b"].c="x" / sink(d["a"].c) R`, label `exact` | reaches | S1 |
| P26 | py | `d["a"]=C() / d["a"].c=t W / d["a"].c="clean" / sink(d["a"].c) R` | not | control |
| P27 | py js go | `o.a=t / sink(o.a)` (Go struct); P30 py `o=C(t) / sink(o.a)` (`C(v)` stores `v` in `.a`) | reaches | control |
| P28 | py js | `o.a=t / sink(o)`; P29 py `o.a=t / p=o / sink(p.a)` | reaches | accepted loss, pinned (DESIGN 8) |
| K01 | py | `d=D()` (`class D(dict)`) `/ d["a"]=t W / d.a="safe" / sink(d.a) R`; K02 `d.a=t W / d["a"]="safe" / sink(d["a"]) R` | not | control |
| K04 | js | `o["a-b"]=t / sink(o["a-b"])`; K05 `o["a-b"]=t W / o["a b"]="s" / sink(o["a b"]) R` | reaches; not | S1; control |
| K06 | js | `o["1"]=t / sink(o[1])`; K07 `o[1]=t / sink(o["1"])` | reaches | S4 |
| K08 | js | `o["\x78"]=t / sink(o.x)` | reaches | S4 (item 6) |
| K09 | py go | `d["\x61"]=t / sink(d["a"])`; Go `m["\x6b"]=t / sink(m["k"])` | reaches | S4 |
| K10 | js | `` o[`x`]=t / sink(o.x) ``; K14 go `` m[`k`]=t / sink(m["k"]) ``; K13 py `d[r"a"]=t / sink(d["a"])`; K16 py `d["a" "b"]=t / sink(d["ab"])`; K15 js `o["__proto__"]=t / sink(o.__proto__)` | runner | key rule (S1 or S4) |
| K11 | py js | `p="a" / d[f"{p}"]=t / sink(d["a"])`; JS `` o[`${p}`] `` | reaches | S4 |
| K12 | py | `d[b"a"]=t W / d["a"]="s" / sink(d["a"]) R` | not | control |
| K18 | go | `const k = "a"` `/ m[k]=t / sink(m["a"])` | reaches | S4 |
| K19 | ts js | `o["x"]=t / sink(o?.["x"])`; K20 `o["$x"]=t / sink(o.$x)`; K21 `$o["x"]=t / sink($o.x)` | reaches | S1 (K21: refused root, accepted loss if DESIGN 5.1 refuses it) |
| L01 | py go; js | `d["a"]=t W / d["b"]="x" / sink(d["a"]) R`, label `exact` | reaches | S1; S3 |
| L02 | py js | `d["u"]={"n":"s0"} / d["u"]["n"]=t W / d["u"]={"n":"s0"} / sink(d["u"]["n"]) R` | not | S3 |
| L03 | js | `d["u"]={v:t} W / d["u"].w="x" / sink(d["u"]) R`, label `exact` | reaches | S3 |
| L04 | py js go | `k="a" / d["a"]=t W / d[k]="clean" / sink(d["a"]) R`, label `nonexact`; L05 `k="b"` | not; reaches | S1 |
| L06 | js | `o.x=t W / o["y"]="c" / sink(o.x) R`; L07 `o["a-b"]=t W / o.x="c" / sink(o["a-b"]) R`; label `exact` | reaches | S3 |
| L08 | js | `k="z" / o[k]=t W / o.x="c" / sink(o[k]) R` | reaches | item 4 |
| N01 | py | `d["a"]=t W / e=d / e["a"]="safe" / sink(d["a"]) R` | not | S1 (uncertified alias) |
| N02 | py go | `d["a"]=t W / d={"a":"s0"} / sink(d["a"]) R` | not | S1; S3 (root kill) |
| N03 | js | `const u=o / o["x"]=t W / u["x"]="safe" / sink(o["x"]) R` | not | S1 (certified alias, `alias_unstable`) |
| Y01 | py js go | `k="a" / d[k]=t W / sink(d["a"]) R`; Y03 `k="a" / d["a"]=t W / sink(d[k]) R`; label `nonexact`; Y02, Y04 the same with `k="b"` | reaches; not | S4; control |
| Y05 | py js | `k="x" / d[k]["a"]=t / sink(d["x"]["a"])`; Y06 `k="n" / d["u"][k]=t / sink(d["u"]["n"])` | reaches | S4 |
| Y10 | py js | `i="x" / j="y" / d[i]["a"]=t W / sink(d[j]["a"]) R`; Y11 `j="x"`, label `nonexact` | not; reaches | S1 |
| Y12 | py js | `k="b" / d["a"]=t / f(d["b"], d[k])`, `f(p, q)` calls `sink(q)`; Y13 js `k="x" / o.x=t / sink(o[k])` | reaches | S4 |
| Y07 | py | `d[0]=t W / sink(d["0"]) R` | not | control |
| Y08 | py js go | `xs=["s0","s0"] / xs[0]=t / sink(xs[1])`; Y09 `sink(xs[0])` | not; reaches | control; accepted loss, pinned |
| X01 | js; go | `o["k"]=t W / delete o["k"] / sink(o["k"]) R`; X03 Go `m["k"]=t W / delete(m, "k") / sink(m["k"]) R` | not | S3; accepted-loss candidate |
| X02 | py | `d["k"]=t / del d["k"] / sink(d)`; X04 `d["k"]=t W / del d["k"] / d["k"]="c" / sink(d["k"]) R` | not | S5; control |
| T01 | js | `for (o["a"] of [t]) {} / sink(o["a"])`; T02 `[o["a"]]=[t] / sink(o["a"])`; T03 `({v:o["a"]}={v:t}) / sink(o["a"])` | reaches | S1 |
| T04 | py | `for d["a"] in [t]: pass / sink(d["a"])`; T05 `d["a"], x = t, 1 / sink(d["a"])` | reaches | S1 |
| A01 | js | `d["u"]={n:"s0"} / d["u"].n=t W / consume({slot:d["u"].n})`, `consume(p)` calls `sink(p.slot)`; A02 `consume2(d["u"].n)` | reaches | control (must stay green from S1) |
| I01 | py js go | multi-line literal, `"a": t,` on its own line, `sink(d["a"])` | reaches | S6 (control if main passes) |
| I02 | py js go | same, `"b": "s",` on its own line, `sink(d["b"])`; I04 `"a": (` / `t` / `),` split value, `sink(d["a"])` | not; reaches | S6 |
| I03 | py | `base={"a":t} / d={**base, "b":"s"} / sink(d["a"])` | reaches | control |
| R01 | py js | `d=make() / d["a"]=t W / sink(make()["a"]) R` | not | control |
| R09 | py | `a="s0" / make()["a"]=t / sink(a)` | not | S1 (main: known wrong, `Def(a)`) |
| R02 | py js go | `d["a"]=t / f(d["a"])` with `f` calling `sink(p)`; R03 `d["b"]="s" / f(d["b"])` | reaches; not | S1; control |
| R04 | py js go | `self.d["a"]=t / sink(self.d["a"])` (JS `this.d`); R05 js ts `(d)["a"]=t`, `(d as any)["a"]=t`; R08 `if c: d["a"]=t / else: d["a"]="s" / sink(d["a"])`, `c` true | reaches | S1 |
| R06 | py js | `for i in range(2): / x=d["a"] / d["a"]=t W` then `sink(x) R` after the loop; R07 `d["a"]=t / def g(): sink(d["a"]) / g()` | reaches | S2 (R07: stage D may accept the loss) |
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
      access, maps versus slices, map aliasing). Each census class gets a row.

## Stage D: design of record (2 days)

The design seat amends DESIGN 5 from DESIGN, `rows.jsonl` with `truth.jsonl`, `CENSUS.md`, the research record,
the review and the accepted-loss candidates, and fixes against every row:

1. The keyed component's encoding under DESIGN 5.1's two conditions (injective with legacy field strings such
   as `["x"]` from `(o)?.["x"].f`; no existing output changes), its `Display`, `Debug` and JSON forms, and the
   key characters admitted (quotes, `]`, `.`, empty key, newline, control characters).
2. The per-language key rule: Python prefixes `r`, `u`, `b`, `f`, concatenation, triple quotes, non-ASCII; JS
   escapes, templates without substitution, numeric strings, `__proto__`; Go raw and interpreted literals; the
   refused roots (DESIGN 5.1, row K21).
3. The overlap label: `NameOnly(AliasUnstable)` or a new `FlowDoubt` (owner decision 4).
4. `replaces` and `may_overlap` for JavaScript (DESIGN 5.4, owner decision 3) and the label o28 takes. Pinned
   tests it changes: `semantic_tests.rs::ident_on_every_target_shape` ("any index is []"),
   `semantic_tests.rs::affects_and_eq_relations`, `census_defs_tests.rs::
   affects_is_prefix_with_a_wildcard_on_either_side`, `affects_tests.rs` (`the_trie_yields_exactly_the_affected_defs`,
   `bucketed_recorded_affects_matches_the_all_pairs_scan`, `writer_flags_by_receiver_and_binding`),
   `recorded_kill_tests.rs` with `variant-matrix.tsv` o27 and o28, `sites.tsv` rows whose source has a static
   key, and the 660 design's §4.4 note.
5. Python and Go: the base predicates of DESIGN 5.3 confirmed on rows N01, N02 and M02; root and prefix kills of
   keyed definitions.
6. Whether, in JavaScript, `[]` matches field components in the reach rule (P13, K08, Y13).
7. Whether S5 and S6 run (C3, C4; P14, X02, P18, I01, I02, I04), their labels, S5's JS field-origin sources and
   S6's container-definition edges (DESIGN 5.3, 5.5).
8. Deletes (X01 to X03) and the accepted-loss list.
9. Dynamic-only `d[i] = t; sink(d[j])`: a promised may-flow or an accepted miss (DESIGN 5.3).

The exit is one revision that amends DESIGN, this plan's slice sections, gate predicates and differential
classes, and each row's `group` and `label`; implementer briefs are cut from that revision (ORCHESTRATION 6).
Both reviewers; cap 2 rounds.

---

## Stage I: slices

Order: S0 builds the tool with no behaviour change. S1 changes definitions, re-keys the member index, applies
the definite-identity label rule and migrates every consumer of the new spellings in one slice: keyed
definitions alone would lose edges main has (`m["k"].x` and `d["u"].n` reads, the literal-leaf transfer into
`p.slot`, plain arguments). S2 adds real-span reads on S1's paths. S3 changes kills and refines S1's
owner-wide overlap labels to routes. S4 adds wildcard reach beside exact matches. S5 and S6 wait for stage D's
go. If the lane stops before S4 merges, the owner accepts S1's `mixed-key-interim` losses by name or S1 is
reverted. **Gates of every slice (DESIGN 7):** Matrix (its group, every earlier group and `control`),
Differential (against the previous slice: its own classes only, every removed or relabelled edge read and
explained; against `prism-base`: cumulative classes listed), No flood (`cross-key` 0), Spellings (changed
fixture rows and golden files listed; `spelling-leak` 0). Sample and Engines run at stage G.

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

The JS identifier-like test copies the string arm of `literal_key` in `src/cpg/object_literal_args.rs` (ASCII,
no backslash, `plain_identifier`, not `__proto__`), as amended at stage D. With `PRISM_SUBSCRIPT_TRACE=1`,
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
      `at_requires_exact_span`; in `access_path.rs` `keyed_component_round_trips`,
      `keyed_key_is_none_for_legacy_strings` (`["x"]`, `[]`, `"[]"`, embedded dots, quotes and brackets),
      `display_debug_and_json_of_keyed_component`; `trace_lines_names_def_and_use_sites`,
      `trace_all_lists_isolated_nodes`, `trace_off_allocates_nothing`.
- [ ] Implement; GREEN. Differential against `prism-base` on the seven corpora: zero change in edges, counters
      and `nav nodes-at`/`taint-reaches`/MCP golden output. Tier-A; two reviewers.

### Slice S1: keyed definitions, the re-keyed index, definite identity, consumers (6 to 7 days)

**Files:**
- Modify `src/ast/lvalues.rs`: `extract_lvalue_spans_from_node`, before the field/index branch and the text
  fallback, matches `self.subscript_chain(node)`: `Admitted` pushes one `PathSpan` over the node and returns;
  `Refused` pushes nothing and returns (the target is consumed; its inputs stay reads of the rvalue walkers);
  `Unaffected` continues as today. `extract_for_in_lvalue_spans`, `extract_for_in_lvalues`,
  `extract_destructuring_def_spans` and `extract_destructuring_defs` gain a subscript arm that does the same.
  Both `assignment_lvalue_paths_on_lines` and `assignment_lvalue_spans_on_lines` (`src/ast/assignments.rs`)
  reach it (a test each).
- Modify `src/ast/rvalue_identifiers.rs` (field-access branch of the four walkers: the admitted path; no path use
  for `Refused`, whose inputs stay reads; `from_expr(text)` only for `Unaffected`), `src/ast/member_reference_index.rs` (`Index::build` keys
  field-access nodes by the same rule and registers every admitted subscript read; write-only targets are never
  registered as reads, in every language), `member_reference_index/oracle.rs` (same).
- Migrate every consumer of the respelled endpoints (DESIGN 5.6) to `static_key_path_at` at the same bytes:
  the three argument-binding positions (`step5b_edges_for_caller_with_exact`, `full_arg_path` in
  `src/cpg/build.rs` ~1639, `src/cpg/callee_legacy.rs` ~105, `src/cpg/step5b_reference.rs` ~81: then
  `bounded_argument_member_path_at`, then `static_key_path_at`, then `from_expr`), `src/cpg/object_literal_args.rs`
  ~497 (lane 3, one lookup), `src/cpg/step5c.rs` ~172 and `src/cpg/step5c_reference.rs` ~174,
  `src/cpg/reaching/capture.rs` ~169 (`collect_reference_identities`), `src/cpg/reaching/capture_regions.rs` ~446,
  `src/ast/member_projection.rs` ~245, `src/data_flow/build/aliases.rs` (`resolve_path`,
  `resolve_on_alias_span`: append keyed components structurally, never by reparsing).
- Create `src/data_flow/build/subscript_overlap.rs` (declared in `src/data_flow/build/mod.rs`) with
  `subscript_overlap_tests.rs`: per owner, the definite-identity check of DESIGN 5.3 (Go `BindingFacts`;
  Python the `strip_rebound` census of `python_prune.rs` applied to the base; JS `js_prune::stable_bases` and
  `alias_cert.rs`), and the S1 overlap rule, flow-insensitive: an edge from or to a lane-built path is joined
  (`worst`) with the overlap label (stage D item 3) when either endpoint fails definite identity or the owner
  holds any write in possible overlap with the definition's path. Applied after either RD arm in the producer's
  label maps (`src/data_flow/build/file.rs`, one call). S3 refines it to routes.
- `src/cpg/multiline_call_arg_parity_tests.rs` (a keyed-argument case: production, legacy twin and serial
  reference give identical edges in identical order); `tests/ast/subscript_matrix_test.rs` (`ENABLED` gains
  `"S1"`); `src/cpg_cache.rs`; docs.

JS `o["x"] = v` now records `o.x`; the census still says `o.[]`, so the site stays uncovered and its kill-only
`o.[]` def keeps today's wildcard kills until S3 (o27 and L01-js unchanged here).

- [ ] Tests first (RED): matrix `S1`; `subscript_overlap_tests.rs`: `dynamic_selector_pair_is_never_exact`
      (Y10, Y11), `rebound_base_is_never_exact` (N02), `uncertified_alias_is_never_exact` (N01),
      `certified_js_alias_stays_alias_unstable` (N03), `owner_dynamic_write_downgrades_keyed_edge` (L04);
      in `src/ast/subscript_paths_tests.rs`: `refused_call_root_records_no_definition` (R09: no `Def(a)`, no
      `Def(make)`, `Use(make)` kept), `for_of_head_subscript_target_defines_path` (T01),
      `array_pattern_subscript_leaf_defines_path` (T02), `python_for_and_tuple_targets_define_path` (T04, T05),
      `write_only_subscript_target_is_not_a_read` (py, js, go); consumer tests:
      `object_literal_leaf_keeps_exact_edge_after_rekey` (A01: `d.["u"].n` → `p.slot` `exact`, from the leaf's
      bytes), `plain_argument_keeps_exact_edge_after_rekey` (A02),
      `keyed_return_keeps_its_return_value_shape` (Step 5c and its reference agree, return-flow counters equal),
      `seed_base_comparison_unaffected_by_keyed_fields` (`seeds.rs`).
- [ ] Implement lvalues and entry points, then index and oracle, then walkers and consumers, then the overlap
      rule. GREEN.
- [ ] Prediction, then differential on the seven corpora (`measure.sh`, `compare_subscripts.py`, against
      `prism-S0`). Classes: `keyed-def-respelled` (same bytes, `x.[]` → keyed or field path),
      `consumer-respelled` (argument, literal-leaf, return and capture endpoints, same bytes, label unchanged),
      `keyed-edge-added` (each `exact` one under definite identity, every other one with the overlap label),
      `keyed-label-move` (Python/Go `killed` → `exact` where the killer wrote another static key, under
      definite identity), `refused-def-removed` (definitions of unrelated names on refused targets, R09),
      `into-overwrite-line` (as dot fields today), `mixed-key-interim` (edges main had between a keyed write and
      a dynamic read or the reverse through `x.[]`: listed in full, restored as overlap edges by S4, a row each),
      `prune-counter`, `loop-plan`. No other loss of an `exact` edge is allowed. Zero-gates (DESIGN 7):
      `cross-key` and `spelling-leak` (a changed path at bytes that hold no static-key subscript occurrence, on
      the node inventory). Anything else exits 1. R: no change.
- [ ] Rows K01, K02, P21, R01, R09, A01 (review focus 2 to 4, 7) pass. Tier-A; cloud tests at 1 and 4 threads
      with a RED-on-base control; two reviewers. PR body: prediction, class counts, `mixed-key-interim` list,
      changed pinned rows, the lane 3 and lane 9 coordination notes, Tier-A.

### Slice S2: subscript reads as real-span uses (2 days)

**Files:** create `src/ast/subscript_reads.rs` (declared in `src/ast.rs`) with `fn push_static_key_read(&self,
node, out)`: for an admitted subscript read it pushes the path span over the node, and the walker then recurses
into the children as today (base and index uses stay); a write-only target is skipped (S1's role rule).
Modify `src/ast/rvalue_identifiers.rs` (one call per walker), matrix `"S2"`, `src/cpg_cache.rs`, docs.

- [ ] Enable `S2`: RED. Implement. GREEN. `cargo test -p prism-mcp --test mcp golden_transcript_test` must not
      change; if it does, re-record with `PRISM_MCP_BLESS_GOLDEN=1` and name each changed assertion in the PR.
- [ ] Differential against `prism-S1`, classes: `subscript-use-added`, `anchor-to-real` (a `to` moves from a
      zero-width anchor to a real span, same line and path); zero-gates as in S1. Tier-A; cloud tests; two
      reviewers.

### Slice S3: kills and route-based overlap (4 days)

**Files:** modify `src/ast/js_write_census/identity.rs` (`Comp` gains a keyed variant; `ident` gives a
static-key subscript `Comp::Name` or the keyed variant by the S0 rule, `Comp::Index` otherwise; `render` and the
two relations per item 4), `src/ast/js_write_census/completion.rs` (`access_path` maps the keyed variant with
`keyed_component`), `src/cpg/reaching/def_buckets.rs` (`affects` splits into `replaces` and `may_overlap`
(DESIGN 5.4); `Trie::affected` and `legacy_kill_sets_with` kill by `replaces`; in non-JS owners a recorded def
with a keyed component is killed by a `replaces` write of its path or any static prefix, the root included,
through a per-base trie built only when the owner has one), the interference consumers `d3.rs`, `d2c.rs`,
`refusal_classes.rs`, `alias_kill.rs`, `census_defs.rs` (switch to `may_overlap`), `subscript_overlap.rs`
(the owner-wide rule becomes the route rule of DESIGN 5.3, reusing the may-kill route check of
`alias_kill.rs`), the pinned tests of item 4, matrix `"S3"`, `src/cpg_cache.rs`, docs (`CLAUDE.md` #674 and
write-census paragraphs).

- [ ] Tests first (RED): matrix `S3`; `src/cpg/reaching/keyed_relation_tests.rs` (declared in
      `src/cpg/reaching.rs`): `replaces_and_may_overlap_tables` (must, may and unrelated pairs for every
      consumer above, same and different bindings, equal dynamic selectors), `trie_matches_the_all_pairs_scan`
      for both relations, `possible_overwrite_on_every_route_downgrades` and `one_clean_route_keeps_exact`,
      `root_rebinding_kills_keyed_def` (N02), `o27_still_kills`, `o28_never_exact`.
- [ ] Implement. GREEN. Pinned-test updates in one commit that lists every changed assertion and fixture row.
- [ ] Differential against `prism-S2`: labels only, edge multiset identical. Classes `kill-narrowed` (JS
      `killed` → `exact`, each killer on the route a static-key write of another key, under definite identity),
      `kill-prefix` (Python/Go `exact` or overlap → `killed` by a prefix or root write), `overlap-route` (S1's
      owner-wide overlap label lifted to `exact` where no route crosses a possible overwrite), `killed-to-overlap`
      (a former kill by a possible-overlap write: o28, a `[]` write over a keyed def), `census-covered` (trace
      `PRISM_RD_CENSUS_TRACE=1`, `kill:affects`), `census-shift` (labels moved by census coverage, refusal,
      same-line, captured or alias doubt, or the completed-definition cap after site changes, each traced).
      Tier-A; cloud tests; two reviewers.

### Slice S4: dynamic-key reach (4 days)

**Files:** create `src/ast/member_reference_index/wildcard.rs` (declared in `member_reference_index.rs`): an
index per owner by `(base binding, depth)`, then per position by static component with one wildcard bucket, so
a definition is compared only with candidates of its own keys and of the wildcard bucket (JS members too if
item 6 says so); `MemberReferenceSearch::find_wildcard_references(owner, path, def_line, def_byte, within,
reads_only) -> Vec<(AccessPath, usize, usize)>`: every admitted occurrence (path, start, end) in possible
overlap (DESIGN 5.3; never two different static components at one position), whether or not an exact match
exists on the same line. Create `src/data_flow/build/subscript_wildcard.rs` (declared in
`src/data_flow/build/mod.rs`): for a def span whose `subscript_chain_at` is admitted, the occurrences become
edges through the same use lookup and prune checks as exact ones, deduplicated per edge pair and labelled by
`subscript_overlap.rs`; a dynamic subscript read of a base that has a lane-built definition gets a real-span use
(DESIGN 5.3). Bound: at most 1,000,000 compared pairs per owner; an owner over it gets no wildcard edges and is
counted in persisted `RdFileStats` counters `dfg_subscript_wildcard_pairs` and
`dfg_subscript_wildcard_owners_capped` (in `nav dfg-stats` and `call-stats.dfg_labels`). Modify
`src/data_flow/build/file.rs` (two calls), `oracle.rs` (a legacy twin of the wildcard search), matrix `"S4"`,
`src/cpg_cache.rs`. If item 3 adds a `FlowDoubt`: `src/cpg/flow_confidence.rs` and every file `rg -l
dfg_label_nameonly_alias_unstable src crates tests` lists (counter, `--edges` name, stats goldens).

- [ ] Tests first (RED): matrix `S4`; `src/data_flow/build/subscript_wildcard_tests.rs`:
      `exact_and_overlap_reads_on_one_line_both_bind` (Y12), `many_keys_one_wildcard_is_linear` (1,000 keys and
      one wildcard on one base: compared pairs ≤ 2,000 by a work counter), `over_budget_owner_gets_no_wildcard_edges`,
      `dynamic_read_gets_a_use_only_beside_a_keyed_def`. Rows Y05, Y06, K08, K09, K11 (review focus 1, 5) and
      one row per S1 `mixed-key-interim` shape pass.
- [ ] Implement. GREEN.
- [ ] Differential against `prism-S3`: `dynamic-edge` (added, never `exact`), `mixed-key-restored` (each S1
      interim loss back as an overlap edge), `wildcard-counter`; `cross-key` 0; owner fan-out distribution of
      added edges in the receipt. Tier-A; cloud tests; two reviewers.

### Slice S5 (stage D go): element write to whole-container read (3 to 4 days)

Create `src/cpg/reaching/element_recall.rs` (declared in `src/cpg/reaching.rs`, called beside
`contribution::analyze` at line ~605); emit beside `recall.emit` in `src/data_flow/build/file.rs` (~811); add
persisted `RdFileStats` counters `dfg_element_recall_*` (in `nav dfg-stats` and `call-stats.dfg_labels`);
matrix `"S5"`; cache version; docs. Modelled on `contribution.rs`: Python, JS/TS/TSX and Go owners with RD;
sources and consumers as DESIGN 5.3 (container reads) and stage D item 7 define them: the source is the
definition occurrence, the consumer a maximal value operand (the whole container or an enclosing subcontainer),
never a receiver, key, membership, length, identity, method or nested projection; a write of the source path
or a prefix, a delete or a root rebinding kills the contribution, a descendant write weakens it; the
1,000,000-candidate budget and skip counters; cold, full-hit and partial-hit builds agree (a cache test). Label
per item 7. Tests first in `src/cpg/reaching/element_recall_tests.rs` (declared in `src/cpg/reaching.rs`):
`element_write_reaches_later_bare_read`, `receiver_of_sibling_read_is_not_a_consumer` (`sink(d["b"])`),
`enclosing_subcontainer_is_a_consumer`, `killed_element_gives_no_recall`, `over_budget_owner_is_skipped`,
`cold_full_and_partial_hits_agree`; then `"S5"` RED → GREEN. Class `element-recall` against `prism-S4`; Tier-A;
cloud tests; two reviewers; PR body as S1.

### Slice S6 (stage D go): static-key initialisers (3 to 4 days)

Create `src/ast/static_key_literals.rs` (declared in `src/ast.rs`); one call in `src/ast/lvalues.rs`; matrix
`"S6"`; cache version; docs. Eligibility and binding as DESIGN 5.5: per eligible entry a def of the entry's
keyed (JS: field) path whose input is the value expression's span, located from the assignment's initializer;
duplicate keys, last wins; the container's def and its edges as stage D item 7 decides; single-line literals
stay an accepted loss unless stage D says otherwise. Tests first in `src/ast/static_key_literals_tests.rs`:
`own_line_entries_get_keyed_defs`, `split_value_entry_is_ineligible` (I04), `entry_on_opening_line_refuses`,
`duplicate_key_last_wins`, `spread_or_computed_key_refuses`, `single_line_literal_unchanged`; then `"S6"` RED →
GREEN, with P18 or I02 a pre-change false positive removed. Class `initialiser-def` against the previous slice;
Tier-A; cloud tests; two reviewers; PR body as S1.

---

## Stage G and H: gates and close-out (2 to 3 days)

- [ ] Final head: full matrix; differential against `prism-base` (cumulative) on the seven corpora, with
      intervening main changes isolated by the merge-base binary; `cross-key` 0; R unchanged.
- [ ] Sample: 200 added or relabelled edges by `tools/sample_edges.py` (seed 10, stratified by rule, language,
      label, fan-out and owner), five lines of context each, read by Opus-5.5 (high): real flow or not; unknowns
      and denominators reported. Gate ≥ 95%.
- [ ] Engines: per row, `taint_reaches` verdict and the route checks beside truth; `tools/review_taint_rows.py`
      records review taint per row (`prism --repo <row dir> --diff <one-line unrelated diff> --algorithm taint
      --format json`, research caveat 10), reporting applicability (whether any source and sink applied); no
      change required.
- [ ] C5 with the head build: `c5_dead.py` on the corpora and the 120 diffs; the benchmark leg only from a
      benchmark run bound to the head build (lane 11's route, or one the controller produces), else reported as
      not measured. A change of spelling is never counted as a repair. `g/REPORT.md`: every gate with its
      numbers.
- [ ] Docs: `CLAUDE.md` (a keyed-subscript paragraph; #674 and write-census paragraphs),
      `docs/features/cpg/architecture.md:313` (resolved question 3), `js-ts-known-gaps.md` (§6 `p.x[k] = 1`,
      ~245, and a subscript entry), `docs/features/language-coverage/language-analysis-gaps.md`.
- [ ] Follow-up issues: DESIGN 8's items; the main defects of DESIGN 2 with the review's probes (rows R09, M01
      to M04): call-root targets defining unrelated names (fixed by S1 for static keys), dynamic-only `exact`
      across different runtime keys, Python dot fields `exact` across root rebinding and uncertified alias
      writes, the object-literal binder's `q["b"]` edge (lane 3); statement-scoped same-line propagation in
      `trace.rs` (P19); alias reads (P16, P29); one static-key policy shared with `object_literal_args::literal_key`.
      Report on the issue (close it if every gate holds); refresh `HANDOFF.md` (`~/.claude/handoff-template.md`)
      and `LEDGER.md`; offload raw dumps; delete `target/`.

## Acceptance for the lane

Every gate of DESIGN 7 holds on the merged head (matrix, per-slice differentials, no flood, sample ≥ 95%,
engines recorded, spellings listed); both reviewers approved S0 to S4 and each conditional slice that ran; the
report is posted. A failing gate is reported with its numbers and the lane stops for the owner.

## Estimates (working days, review rounds included)

| M | T and E | D | S0 | S1 | S2 | S3 | S4 | G and H | Total | S5, S6 (each, conditional) |
|---|---|---|---|---|---|---|---|---|---|---|
| 3–4 | 4–5 | 2 | 1–2 | 6–7 | 2 | 4 | 4 | 2–3 | **28–33** | 3–4 |
