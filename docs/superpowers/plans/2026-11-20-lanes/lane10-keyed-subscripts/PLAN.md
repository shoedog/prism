# Lane 10: subscripts in the data-flow graph — implementation plan

> **For agentic workers:** this plan is run by an orchestrator under `../ORCHESTRATION.md`. Each task is
> dispatched to an implementer (Sonnet-5.5, high) with its section as the brief. Steps use checkbox syntax.

**Goal:** a value written through a string-literal subscript reaches later reads of the same element in
Python, JavaScript/TypeScript and Go; a write to one key neither reaches nor kills a read of another; dynamic
keys keep today's may-meaning; every rule is checked against executed truth and a labelled corpus sample.

**Architecture:** one node-based path builder (`src/ast/subscript_paths.rs`) gives every subscript chain that
holds a static key a full path; producers use it only for such chains and keep today's text path otherwise.
Definitions and the member index switch first, then reads and argument binding, kills, and a wildcard reach
rule for dynamic keys; container reads and initialisers are conditional.

**Tech stack:** Rust (core crate: `src/access_path.rs`, `src/ast`, `src/data_flow/build`, `src/cpg`);
tree-sitter Python 0.23.6, JavaScript 0.23.1, TypeScript 0.23.2 and Go 0.25.0 + PR #193 (both vendored);
Python 3.12 (uv), Node 24.15 and Go 1.26.2 for the truth runner; py-tree-sitter for the census.

**Spec:** `DESIGN.md` here (5.1 to 5.5 the rules, tentative until stage D; 6 the census; 7 the gates; 9 the
owner decisions, whose defaults this plan applies).

## Global constraints

- Worktree `~/code/prism-subscripts`; evidence `~/prism-evidence/subscripts/`.
- Corpora (`~/code/bench-repos`): P1 `apache-superset` at `7519fa8b6f`, P2 `saleor-saleor` at `cc4aea76e3`
  (Python); X Excalidraw at `0642e72c` (`~/prism-evidence/inputs/excalidraw-0642e72c/source`); T
  `TypeScript/src` at `7964e22f2b`; G1 `caddy` at `77e9ce7404`, G2 `etcd` at `61d518f55e` (Go); control R
  `ripgrep` at `82313cf958` (Rust, unchanged by the lane: zero edge and counter change on every slice).
- **Not changed:** `AccessPath::from_expr` and its tests; the path of any chain without a static-key subscript
  (byte-identical); integer indices and slices; languages other than Python, JS, TS, TSX, Go;
  `src/cpg/object_literal_args.rs` (lane 3); `src/cpg/trace.rs` (PR #841); review taint
  (`crates/prism-review/src/algorithms/taint*`, `taint_sinks/`); the Go field-recall pass
  (`src/cpg/reaching/contribution.rs`, `go_aggregate.rs`: S5's model, not edited); the text paths in
  `src/ast/member_projection.rs`, `src/cpg/step5c.rs`, `src/reasoning/seeds.rs` (follow-ups).
- Weighted sizes (`python3 scripts/file_size.py`): `src/access_path.rs` 396, `src/ast/lvalues.rs` 564.5,
  `src/ast/rvalue_identifiers.rs` 594, `src/ast/member_reference_index.rs` 327.5, `src/ast.rs` 600.5,
  `src/data_flow/build/file.rs` 850.5, `src/cpg/reaching/def_buckets.rs` 319, `src/cpg/reaching.rs` 990,
  `src/cpg_cache.rs` 1,544, **`src/cpg/build.rs` 1,777** (call-site edits only). Logic goes in new modules.
- Module lines: `src/ast/` modules in `src/ast.rs` (tests `#[cfg(test)] mod x_tests;`), `src/data_flow/build/`
  modules in `src/data_flow/build/mod.rs`, `src/cpg/reaching/` modules in `src/cpg/reaching.rs`.
- A slice that changes stored DataFlow nodes, edges, labels or statistics bumps `CACHE_VERSION` (189 at
  planning) to the next free number at merge, with a `/// - vNNN:` line and the pin test
  `cache_versions_are_pinned_for_cpg_semantics`; S0 bumps nothing. Tier-A applies to S0 to S6, the corpus
  differential to S1 to S6. Executed fixtures are lane-written only.
- Argument binding changes at the same position in `src/cpg/build.rs` (`step5b_edges_for_caller_with_exact`,
  `full_arg_path`, line ~1641), `src/cpg/callee_legacy.rs` (~105) and `src/cpg/step5b_reference.rs` (~81).
- The member index and its oracle change together (`Index::build` in `src/ast/member_reference_index.rs`;
  `collect_path_refs` and `collect_fenced_path_refs` in `member_reference_index/oracle.rs`), or
  `member_index_differential_edges_and_non_js` fails.

## Review focus

1. A key with an escape or interpolation (`d["\x61"]`, `o["\x78"]`, `d[f"{p}"]`, `` o[`${p}`] ``) is never a
   keyed component; it connects only through the dynamic rule, never `exact`. Rows K08, K09, K11 (S4).
2. `d["a"]` and `d.a` on one Python object (a `dict` subclass) stay apart both ways. Rows K01, K02 (control,
   confirmed in S1).
3. Augmented assignment through a subscript: `d["a"] = t; d["a"] += "x"; sink(d["a"])` reaches; so does
   `d["a"] = ""; d["a"] += t`. Row P21 (S1).
4. A subscript on a call result (`make()["a"]`) gets no path, definition or edge. Row R01 and the S0 test
   `no_static_key_means_none`.
5. A nested subscript with a dynamic inner key (`d[k]["a"]`, `d["u"][k]`) keeps both components, never matches
   a static-key chain exactly, and connects only through the dynamic rule. Rows Y05, Y06 (S4).

---

## Stage M: census (2 to 3 days)

### Task M-1: workspace, base binary, issue, baseline

- [ ] Worktree (ORCHESTRATION 4). `~/prism-evidence/tools/memguard.sh 8 cargo build --release`; copy to
      `~/prism-evidence/subscripts/bin/prism-base`; record `--version`.
- [ ] File the issue "Data flow through subscripts: keyed paths (lane 10)", linking DESIGN and PLAN.
- [ ] `tools/measure.sh <binary> <corpus> <outdir>` (lane 11's `scripts/measure/` instead if merged): through
      memguard, `prism nav --no-cache dfg-stats --repo C`, `prism nav --no-cache call-stats --repo C` and
      `PRISM_EDGES_BYTES=1 prism nav --no-cache dfg-stats --repo C --edges`. Run on the seven corpora with
      `prism-base` into `base/<corpus>/`: the baseline of every differential.
- [ ] `tools/compare_subscripts.py <base dir> <head dir> <corpus>` (model
      `~/prism-evidence/jsts-lane/783/scripts/compare761.py`): edges keyed by file, path, line and bytes at both
      ends; every counter delta of both stats files; the zero-gates `cross-key` and `spelling-leak` (S1); the
      classes each slice names, added by that slice; exit 1 on an unclassified change.

### Task M-2: inventory and counts C1 to C4

**Files (evidence):** `census/sites.py`, `census/confirm.py`, `census/count.py`, `census/CENSUS.md`. The CLI has
no whole-repository node dump (only `nav nodes-at` per line): sites come from the same grammars, prism confirms
a sample.

- [ ] `sites.py <corpus> <out.jsonl>`, run as `uv run --with tree-sitter==0.23.2 --with
      tree-sitter-python==0.23.6 --with tree-sitter-javascript==0.23.1 --with tree-sitter-typescript==0.23.2
      --with tree-sitter-go==0.25.0 python census/sites.py …` (record any version PyPI lacks and its
      substitute). File filter: `TESTPATH` imported from `~/prism-evidence/lanes-2026-11-20/probes/lane10/
      sizing/size.py`. One record per subscript node (Python `subscript`, JS/TS/TSX `subscript_expression`, Go
      `index_expression`): file, owner (innermost callable), line, bytes, role (`write`, `augmented`, `read`,
      `delete`, `pattern`), base chain up to the first subscript (identifiers and dot members, else `none`),
      index kind (`str_ident`, `str_other`, `str_escape` backslash or non-ASCII, `str_interp`, `int`, `name`,
      `go_const`, `other`), key text, enclosing loop. Plus container literals assigned to a plain name
      (Python `dictionary`, JS `object`, Go map `composite_literal`) with their static keys, and bare reads of
      a base (`sink(d)`, `f(d)`, `return d`).
- [ ] `confirm.py <sites.jsonl> <corpus> --sample 200 --seed 10`: per sampled site, `prism-base nav --cache-dir
      ~/prism-evidence/subscripts/navcache nodes-at --repo C --location f:l --format json` (model
      `probes/lane10/nodes_dump.py`): a write has a Def whose `fields` end in `"[]"` at the target bytes, a read
      a Use of the base. Agreement ≥ 95%, else fix `sites.py`.
- [ ] `count.py`: **C1** owners with a write W then a read R of the same base chain (R after W in byte order, or
      both in one loop), by key relation (same literal, different literals only, either dynamic), each owner
      once per class; **C2** sites by index kind, role, language; **C3** owners with an element write then a
      bare read of the base; **C4** static-key literals whose name is later read by one of their keys
      (`d["k"]`, or `o.k` in JS).

### Task M-3: C5 and the memo

- [ ] `census/c5_bench.py`: in `~/prism-evidence/meas/secbench/repair-r1/final/entries.jsonl` (record the build
      in `binding.json`), cases with outcome `reached_function_only`, `partial` or `not_reached` whose
      `trace_detail.payload_frontier` holds a Def with a `"[]"` field at bytes that are a subscript write in
      `~/prism-evidence/inputs/secbench-pkgs`: count and ids.
- [ ] `census/c5_diffs.py`: the 120 diffs of #602 (lane 11's `populations/602-m2-120.json` if merged, else
      `population.jsonl` from `~/prism-evidence/archive/prism-crossfile-docs-493ab5ee-receipts.tar.gz`, member
      `next-five-plan-fable-20261114/final-gate-campaign-preparation/`). Per diff: check out the head in a
      scratch worktree; for each added line with a string-key subscript write (≤ 20 per diff) run `prism-base
      nav --cache-dir … taint-reaches --repo <checkout> --source f:l --format json`; count seeds whose frontier
      is only the write's own Def.
- [ ] `CENSUS.md`: C1 to C5 per corpus and language, the decision lines, posted on the issue.

**Exit gate:** C1 same-key owners ≥ 500 across the six corpora; else post, stop, wait for the owner. C3 ≥ 300
makes S5 a candidate, C4 ≥ 300 makes S6 one; stage D decides with the rows.

---

## Stage T and E: executed matrix (3 to 4 days)

### Task T-1: rows, truth runner, data-driven test

**Files:**
- Create under `tests/fixtures/subscript_matrix/`: `rows.jsonl`, `cases/<id>.{py,js,ts,go,rs,java,c}`,
  `README.md`, `truth/run_truth.py`, `truth/prelude.{py,mjs,go}`, `truth/test_run_truth.py`,
  `truth/truth.jsonl` (committed output), `truth/check.sh`. Create `tests/ast/subscript_matrix_test.rs`.
- Modify: `tests/ast/main.rs` (`mod subscript_matrix_test;`)

Cases: Python `def case():`, JS/TS `function caseFn() { … }`, Go `package main` with `func caseFn()`. Each starts
with `t = src()` (`const t = src();`, `t := src()`) on its own line, calls `sink(…)` once, one statement per
line; a trailing comment `W` marks the subscript write that matters and `R` the read line. TS uses erasable
syntax only.

`run_truth.py <cases> <rows.jsonl>`: per executable row, a fresh process (10 s timeout) runs prelude, case and
a call of the case function: `uv run --python 3.12 python`, `node` (24, type stripping on) for `.js`/`.ts`,
`go run case.go prelude.go`. `src()` returns `"TAINT-7f3a"`; `sink(v)` prints whether any string reachable
from `v` contains it (deep and cycle-safe: Python dict, list, tuple, set, `__dict__`; JS own properties,
arrays, `Map`, `Set`; Go `reflect` over map, slice, array, struct, pointer, interface). Truth: `reaches`,
`not`, or `inadmissible` (exception, timeout, no output). First line: the three interpreter versions.
`test_run_truth.py` (`python3 -m unittest`): reaching row → `reaches`; throwing row → `inadmissible`; nested
marker found; a case naming `os`, `subprocess`, `child_process`, `socket` or `net/http` is refused. Runs
locally through memguard.

`rows.jsonl`: `id`, `group` (`control`, `S1` … `S6`), `lang`, `file`, `class`, `census` (C1 to C4 class),
optional `label` (`exact`, `killed`, `nonexact`: the W→R edge), optional `pinned_main` (main's verdict, for
control-language rows and accepted losses), optional `accepted_loss` (reason, issue). Truth comes only from
`truth.jsonl`.

`subscript_matrix_test.rs`: `const ENABLED: &[&str] = &["control"];` and `#[test] fn subscript_matrix_rows()`.
Per enabled row it writes the case to a temp repo and builds a `NavigationSession` (copy the `fixture` helper
of `tests/reasoning/taint_reaches_test.rs`) and a `DataFlowGraph` (labels via `crate::common::dfg_labels`).
Required (DESIGN 7): truth `reaches` → `taint_reaches` from the `src()` line to the `sink(` line is `Reached`;
truth `not` → no `Exact` DataFlow edge whose `from` has the W target's byte span and whose `to` is on the R
line; `label` checked on that edge; `pinned_main` rows keep main's verdict. `#[test] fn
subscript_matrix_rows_have_truth()`: every executable row has a non-`inadmissible` truth. The RED record (rows
failing when each group is enabled by hand on main) goes in the ledger.

Seed rows (`/` separates lines; `d` starts as `{}`, Go `map[string]any{}`; `C` is a small class):

| Row | Langs | Body | Truth | Group |
|---|---|---|---|---|
| P01 | py js go | `d["a"]=t W / d["b"]="safe" / sink(d["b"]) R` | not | control |
| P02 | py js go | same, `sink(d["a"])` | reaches | S1 |
| P03 | py js go | `d["a"]=t W / d["a"]="clean" / sink(d["a"]) R` | not | control |
| P04 | py js | `d["u"]={} / d["u"]["n"]=t W / d["u"]["m"]="safe" / sink(d["u"]["m"]) R`; P05 `sink(d["u"]["n"])` | not; reaches | control; S1 |
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
| P21 | py js go | `d["a"]=t / d["a"]+="x" / sink(d["a"])`; `d["a"]="" / d["a"]+=t / sink(d["a"])` | reaches | S1 |
| P22 | py js | `d["u"]=C() / d["u"].n=t W / d["u"].m="safe" / sink(d["u"].m) R` | not | S1 |
| P23 | go | `s.x=t / m["k"]=s / sink(m["k"].x)` | reaches | control |
| P24 | py js | `d["u"]=C() / d["u"].n=t / sink(d["u"].n)` | reaches | control |
| P25 | py | `d["a"]=C() / d["b"]=C() / d["a"].c=t W / d["b"].c="x" / sink(d["a"].c) R`, label `exact` | reaches | S1 |
| P26 | py | `d["a"]=C() / d["a"].c=t W / d["a"].c="clean" / sink(d["a"].c) R` | not | control |
| P27 | py js go | `o.a=t / sink(o.a)` (Go struct); P30 py `o=t / sink(o.a)` | reaches | control |
| P28 | py js | `o.a=t / sink(o)`; P29 py `o.a=t / p=o / sink(p.a)` | reaches | accepted loss, pinned (DESIGN 8) |
| K01 | py | `d=D()` (`class D(dict)`) `/ d["a"]=t W / d.a="safe" / sink(d.a) R`; K02 `d.a=t W / d["a"]="safe" / sink(d["a"]) R` | not | control |
| K04 | js | `o["a-b"]=t / sink(o["a-b"])`; K05 `o["a-b"]=t W / o["a b"]="s" / sink(o["a b"]) R` | reaches; not | S1; control |
| K06 | js | `o["1"]=t / sink(o[1])`; K07 `o[1]=t / sink(o["1"])` | reaches | S4 |
| K08 | js | `o["\x78"]=t / sink(o.x)` | reaches | S4 (item 6) |
| K09 | py go | `d["\x61"]=t / sink(d["a"])`; Go `m["\x6b"]=t / sink(m["k"])` | reaches | S4 |
| K10 | js go py | `` o[`x`]=t / sink(o.x) ``; Go `` m[`k`]=t / sink(m["k"]) ``; K13 `d[r"a"]=t / sink(d["a"])`, `d["a" "b"]=t / sink(d["ab"])`; K15 `o["__proto__"]=t / sink(o.__proto__)` | runner | key rule (S1 or S4) |
| K11 | py js | `p="a" / d[f"{p}"]=t / sink(d["a"])`; JS `` o[`${p}`] `` | reaches | S4 |
| K12 | py | `d[b"a"]=t W / d["a"]="s" / sink(d["a"]) R` | not | control |
| K18 | go | `const k = "a"` `/ m[k]=t / sink(m["a"])` | reaches | S4 |
| K19 | ts js | `o["x"]=t / sink(o?.["x"])`; `o["$x"]=t / sink(o.$x)` | reaches | S1 |
| L01 | py go; js | `d["a"]=t W / d["b"]="x" / sink(d["a"]) R`, label `exact` | reaches | S1; S3 |
| L02 | py js | `d["u"]={} / d["u"]["n"]=t W / d["u"]={} / sink(d["u"]["n"]) R` | not | S3 |
| L03 | js | `d["u"]={v:t} W / d["u"].w="x" / sink(d["u"]) R`, label `exact` | reaches | S3 |
| L04 | py js go | `k="a" / d["a"]=t W / d[k]="clean" / sink(d["a"]) R`; L05 `k="b"` | not; reaches | item 5; control |
| L06 | js | `o.x=t W / o["y"]="c" / sink(o.x) R`; L07 `o["a-b"]=t W / o.x="c" / sink(o["a-b"]) R`; label `exact` | reaches | S3 |
| L08 | js | `k="z" / o[k]=t W / o.x="c" / sink(o[k]) R` | reaches | item 4 |
| Y01 | py js go | `k="a" / d[k]=t W / sink(d["a"]) R`; Y03 `k="a" / d["a"]=t W / sink(d[k]) R`; label `nonexact`; Y02, Y04 the same with `k="b"` | reaches; not | S4; control |
| Y05 | py js | `d["x"]={} / k="x" / d[k]["a"]=t / sink(d["x"]["a"])`; Y06 `d["u"]={} / k="n" / d["u"][k]=t / sink(d["u"]["n"])` | reaches | S4 |
| Y07 | py | `d[0]=t W / sink(d["0"]) R` | not | control |
| Y08 | py js go | `xs=[0,0] / xs[0]=t / sink(xs[1])`; Y09 `sink(xs[0])` | not; reaches | control; accepted loss, pinned |
| X01 | js; go | `o["k"]=t W / delete o["k"] / sink(o["k"]) R`; X03 Go `m["k"]=t W / delete(m, "k") / sink(m["k"]) R` | not | S3; accepted-loss candidate |
| X02 | py | `d["k"]=t / del d["k"] / sink(d)`; X04 `d["k"]=t W / del d["k"] / d["k"]="c" / sink(d["k"]) R` | not | S5; control |
| I01 | py js go | multi-line literal, `"a": t,` on its own line, `sink(d["a"])` | reaches | S6 (control if main passes) |
| I02 | py js go | same, `"b": "s",` on its own line, `sink(d["b"])` | not | S6 |
| I03 | py | `base={"a":t} / d={**base, "b":"s"} / sink(d["a"])` | reaches | control |
| R01 | py js | `d=make() / d["a"]=t W / sink(make()["a"]) R` | not | control |
| R02 | py js go | `d["a"]=t / f(d["a"])` with `f` calling `sink(p)`; R03 `d["b"]="s" / f(d["b"])` | reaches; not | S2; control |
| R04 | py js go | `self.d["a"]=t / sink(self.d["a"])` (JS `this.d`); R05 js ts `(d)["a"]=t`, `(d as any)["a"]=t`; R08 `if c: d["a"]=t / else: d["a"]="s" / sink(d["a"])`, `c` true | reaches | S1 |
| R06 | py js | `for i in range(2): / sink(d["a"]) / d["a"]=t`; R07 `d["a"]=t / def g(): sink(d["a"]) / g()` | reaches | S2 (R07: stage D may accept the loss) |
| C01 | rust java c | `v[0].x=t; sink(v[0].x)`; `a[0]=t; sink(a[0])`; `buf[i].x=t; sink(buf[i].x)`; Rust `m["k"]` read | n/a | control, pinned |

- [ ] Write rows, cases, runner, preludes, README (schema, markers, rules) and test. Control green on main;
      RED record for the rest; commit; CI green with only `control` enabled. One reviewer (test-only).

### Task T-2: enumeration

- [ ] Tiers per ORCHESTRATION 6. Axes: language; role (write, augmented, read, delete, destructuring and `for`
      targets); key form by grammar (string kinds, prefixes, escapes, interpolation, concatenation, templates,
      numbers, names, constants, expressions, slices, tuples, special JS keys); base (identifier, `self`/`this`,
      member chain, call, parentheses, TS wrappers, optional chain); order of field, keyed and dynamic
      components; flow (direct, local, alias, augmented, loop-carried, branch, closure, whole read, argument,
      return); kills (same, other, prefix, deeper, dynamic, delete); initialisers; semantics (ToPropertyKey,
      item versus attribute, overloaded `__getitem__`, maps versus slices, map aliasing). Each census class
      gets a row.

## Stage D: design of record (1 to 2 days)

The design seat amends DESIGN 5 from DESIGN, `rows.jsonl` with `truth.jsonl`, `CENSUS.md`, the research record
and the accepted-loss candidates, and fixes against every row:

1. The keyed component's spelling in `fields`, its `Display`, `Debug` and JSON forms, and the key characters
   admitted (`"`, `]`, newline must not be ambiguous).
2. The per-language key rule: Python prefixes `r`, `u`, `b`, `f`, concatenation, triple quotes, non-ASCII; JS
   escapes, templates without substitution, numeric strings, `__proto__`; Go raw and interpreted literals.
3. The label of dynamic-key flows: `NameOnly(AliasUnstable)` or a new `FlowDoubt` (owner decision 4).
4. The narrowed JavaScript kill rule (owner decision 3): what a `[]` writer and a `[]` def match; whether a
   static-key write kills an earlier `[]` def. Pinned tests it changes: `semantic_tests.rs::
   ident_on_every_target_shape` ("any index is []"), `census_defs_tests.rs::
   affects_is_prefix_with_a_wildcard_on_either_side`, `affects_tests.rs` (`the_trie_yields_exactly_the_affected_defs`,
   `bucketed_recorded_affects_matches_the_all_pairs_scan`, `writer_flags_by_receiver_and_binding`),
   `recorded_kill_tests.rs` with `variant-matrix.tsv` o27 and o28, `sites.tsv` rows whose source has a static
   key, and the 660 design's §4.4 note.
5. Python and Go kills: whether a keyed write kills paths below it; whether a `[]` write keeps killing equal
   `[]` paths (main does, `def_buckets.rs:154`; DESIGN 5.4 says it kills nothing with certainty; DESIGN 3 says
   `[]` keeps its meaning); the label of a keyed edge crossed by a dynamic write (L04).
6. Whether, in JavaScript, `[]` matches field components in the reach rule (P13, K08).
7. Whether S5 and S6 run (C3, C4; P14, X02, P18, I01, I02), their labels, and S6's multi-line limit.
8. Deletes (X01 to X03) and the accepted-loss list.

The implementer then sets each row's `group` and `label` to the amended design. Both reviewers; cap 2 rounds.

---

## Stage I: slices

Order: S0 builds the tool with no behaviour change. S1 changes definitions **and** re-keys the member index in
one slice: keyed definitions alone would lose edges main has (`m["k"].x` and `d["u"].n` reads meet main's
`x.[]` definitions only through the shared text spelling). S2 adds real-span reads and argument binding on
S1's paths. S3 changes kills once the paths exist. S4 adds wildcard reach beside exact matches. S5 and S6 wait
for stage D's go. **Gates of every slice (DESIGN 7):** Matrix (its group, every earlier group and `control`),
Differential (its own classes only; every removed or relabelled edge read and explained), No flood
(`cross-key` 0), Spellings (changed fixture rows and golden files listed; `spelling-leak` 0). Sample and
Engines run at stage G.

### Slice S0: node-based path builder, used by a trace only (1 to 2 days)

**Files:** create `src/ast/subscript_paths.rs`, `src/ast/subscript_paths_tests.rs`,
`src/data_flow/build/subscript_trace.rs`, `src/data_flow/build/subscript_trace_tests.rs`; modify `src/ast.rs`
and `src/data_flow/build/mod.rs` (module lines), `src/access_path.rs` (helpers and unit tests),
`src/data_flow/build/file.rs` (one `subscript_trace::trace` call per owner beside `param_alias_trace::trace`).

```rust
// src/access_path.rs — spelling per stage D item 1
pub fn keyed_component(key: &str) -> String;
pub fn keyed_key(component: &str) -> Option<&str>;   // None for a field and for `[]`

// src/ast/subscript_paths.rs
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum IndexComponent { Field(String), Keyed(String), Dynamic }
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChainPath { pub path: AccessPath, pub static_keys: usize, pub dynamic: usize }
impl ParsedFile {
    /// Python `subscript`, JS/TS/TSX `subscript_expression`, Go `index_expression`, or a field access over one;
    /// root a plain identifier, `self` or `this` (parentheses, TS `!`/`as`/`satisfies` peeled); else None.
    pub(crate) fn subscript_chain_path(&self, node: tree_sitter::Node<'_>) -> Option<ChainPath>;
    /// The chain path when `static_keys > 0`, else None (callers keep today's text path).
    pub(crate) fn static_key_path(&self, node: tree_sitter::Node<'_>) -> Option<AccessPath>;
    pub(crate) fn static_key_path_at(&self, start: usize, end: usize) -> Option<AccessPath>;
    pub(crate) fn subscript_chain_path_at(&self, start: usize, end: usize) -> Option<ChainPath>;
    pub(crate) fn index_component(&self, subscript: tree_sitter::Node<'_>) -> IndexComponent;
}
```

The JS identifier-like test copies the string arm of `literal_key` in `src/cpg/object_literal_args.rs` (ASCII,
no backslash, `plain_identifier`), as amended at stage D; that file is not edited. With
`PRISM_SUBSCRIPT_TRACE=1`, `trace` prints one stderr line per subscript-bearing def or use span:
`subscript-site\t<file>\t<owner>\t<line>\t<start>\t<end>\t<def|use>\t<recorded path>\t<chain path or ->`;
the logic sits in a pure `trace_lines(…) -> Vec<String>`.

- [ ] Tests first, against a skeleton returning `None` (RED): `key_rule_table` (one row per language and key
      form of item 2); `chains_keep_every_component` (`d["u"]["n"]`, `d["a"].c`, `o.items["k"]`, `d[k]["a"]`,
      JS `o["x"].y` → `o.x.y`, Go `m["k"].x`); `no_static_key_means_none` (`d[k]`, `d[0]`, `d[k].c`, `o.x`,
      `make()["a"]`, `(a or b)["k"]`, Rust, Java and C subscripts); `static_key_path_is_some_only_for_static_key_chains`
      (every field-access and subscript node of every `.py .js .ts .tsx .go` file under `tests/fixtures/`:
      `Some` exactly when an independent check finds a string-literal index on the receiver chain — the
      guarantee that existing paths stay unchanged); `at_requires_exact_span`; in `access_path.rs`
      `keyed_component_round_trips`, `display_of_keyed_component`; `trace_lines_names_def_and_use_sites`.
- [ ] Implement; GREEN. Differential on the seven corpora: zero change. Tier-A; two reviewers.

### Slice S1: keyed definitions and the re-keyed index (4 to 5 days)

**Files:** modify `src/ast/lvalues.rs` (`extract_lvalue_spans_from_node`: before the field/index branch and the
text fallback, `if let Some(path) = self.static_key_path(node)` pushes one `PathSpan` over the node and
returns; Python `subscript` takes this branch), `src/ast/rvalue_identifiers.rs` (field-access branch of the
four walkers: `static_key_path(node).unwrap_or_else(|| AccessPath::from_expr(text))`),
`src/ast/member_reference_index.rs` (`Index::build` keys field-access nodes by the same rule and also
registers every subscript node whose `static_key_path` is `Some`), `member_reference_index/oracle.rs` (same),
`src/cpg/reaching/capture.rs` (`collect_reference_identities`, same), `tests/ast/subscript_matrix_test.rs`
(`ENABLED` gains `"S1"`), `src/cpg_cache.rs`, docs. Re-run, unchanged expected: `member_reference_index/`
tests, `for_init_tests.rs`, `member_only_param_*_tests.rs`, `recorded_kill_tests.rs`, `alias_occurrence`,
`js_write_census` and `js_alias_matrix` tests.

JS `o["x"] = v` now records `o.x`; the census still says `o.[]`, so the site stays uncovered and its kill-only
`o.[]` def keeps today's wildcard kills until S3 (o27 and L01-js unchanged here). Check that
`js_ts_identifier_is_not_a_read` refuses a subscript node in write position on the fenced JS path; if not,
refuse it in the index walk and add a row.

- [ ] Enable `S1`: RED. Implement lvalues, then index and oracle, then walkers. GREEN.
- [ ] Prediction, then differential on the seven corpora (`measure.sh`, `compare_subscripts.py`). Classes:
      `keyed-def-respelled` (same bytes, `x.[]` → keyed or field path), `keyed-edge-added`,
      `keyed-label-move` (Python/Go `killed` → `exact` where the killer wrote another key),
      `into-overwrite-line` (as dot fields today), `mixed-key-interim` (edges main had between a keyed write
      and a dynamic read or the reverse through `x.[]`: listed in full, restored by S4, a row each),
      `prune-counter`, `loop-plan`. Zero-gates: `cross-key` (an added keyed edge whose `to` line holds no
      subscript of that base with the def's key) and `spelling-leak` (a changed path on a line without a
      static-key subscript). Anything else exits 1. R: no change.
- [ ] Rows K01, K02, P21, R01 (review focus 2 to 4) pass. Tier-A; cloud tests at 1 and 4 threads with a
      RED-on-base control; two reviewers. PR body: prediction, class counts, `mixed-key-interim` list,
      changed pinned rows (none expected), Tier-A.

### Slice S2: subscript reads as real-span uses; argument binding (2 to 3 days)

**Files:** create `src/ast/subscript_reads.rs` (declared in `src/ast.rs`) with `fn push_static_key_read(&self,
node, out)`: for a subscript node with `Some(path)` it pushes the path span over the node, and the walker
then recurses into the children as today (base and index uses stay). Modify `src/ast/rvalue_identifiers.rs`
(one call per walker), the three argument-binding positions (`bounded_argument_member_path_at`, then
`static_key_path_at`, then `from_expr`), `src/cpg/multiline_call_arg_parity_tests.rs` (a keyed-argument case:
production, legacy twin and serial reference give identical edges in identical order), matrix `"S2"`,
`src/cpg_cache.rs`, docs.

- [ ] Enable `S2`: RED. Implement. GREEN. `cargo test -p prism-mcp --test mcp golden_transcript_test` must
      not change; if it does, re-record with `PRISM_MCP_BLESS_GOLDEN=1` and name each file in the PR.
- [ ] Differential classes: `subscript-use-added`, `anchor-to-real` (a `to` moves from a zero-width anchor to
      a real span, same line and path), `arg-binding-keyed`; zero-gates as in S1. Tier-A; cloud tests; two
      reviewers.

### Slice S3: kills (3 days)

**Files:** modify `src/ast/js_write_census/identity.rs` (`Comp` gains a keyed variant; `ident` gives a
static-key subscript `Comp::Name` or the keyed variant by the S0 rule, `Comp::Index` otherwise; `render` and
`affects` per item 4), `src/ast/js_write_census/completion.rs` (`access_path` maps the keyed variant with
`keyed_component`), `src/cpg/reaching/def_buckets.rs` (`affects`, `Trie::affected`, `legacy_kill_sets_with`
per item 4; in non-JS owners a recorded def with a keyed component kills per item 5, through a per-base trie
built only when the owner has one), the pinned tests of item 4, matrix `"S3"`, `src/cpg_cache.rs`, docs
(`CLAUDE.md` #674 and write-census paragraphs).

- [ ] Enable `S3`: RED. Census identity, relation, non-JS kills. GREEN. Pinned-test updates in one commit that
      lists every changed assertion and fixture row.
- [ ] Differential: labels only, edge multiset identical. Classes `kill-narrowed` (JS `killed` → `exact`, each
      killer on the route a static-key write of another key), `kill-prefix` (Python/Go `exact` → `killed`),
      `census-covered` (trace `PRISM_RD_CENSUS_TRACE=1`, `kill:affects`). Tier-A; cloud tests; two reviewers.

### Slice S4: dynamic-key reach (3 to 4 days)

**Files:** create `src/ast/member_reference_index/wildcard.rs` (declared in `member_reference_index.rs`): a map
from `(base, component count)` to candidates whose `subscript_chain_path` has a keyed or `[]` component (JS
members too if item 6 says so), and `MemberReferenceSearch::find_wildcard_reference_spans(owner, path,
def_line, def_byte, within, reads_only) -> BTreeMap<usize, (usize, usize)>`: lines with a compatible candidate
(same length; components equal except at least one position pairing `[]` with a keyed component; never two
different keys) absent from the exact search. Create `src/data_flow/build/subscript_wildcard.rs` (declared in
`src/data_flow/build/mod.rs`): for a def span whose `subscript_chain_path_at` has a keyed or `[]` component,
the wildcard lines become edges through the same `get_use` and prune checks as exact ones, kept in a pair set;
after either RD arm each pair's label is joined (`worst`) with item 3's label in the producer's label maps.
Modify `src/data_flow/build/file.rs` (two calls), `oracle.rs` (a legacy twin of the wildcard search), matrix
`"S4"`, `src/cpg_cache.rs`. If item 3 adds a `FlowDoubt`: `src/cpg/flow_confidence.rs` and every file
`rg -l dfg_label_nameonly_alias_unstable src crates tests` lists (counter, `--edges` name, stats goldens).

- [ ] Enable `S4`: RED. Implement. GREEN. Rows Y05, Y06, K08, K09, K11 (review focus 1, 5) and one row per
      S1 `mixed-key-interim` shape pass.
- [ ] Differential: `dynamic-edge` (added, never `exact`); `cross-key` 0. Tier-A; cloud tests; two reviewers.

### Slice S5 (stage D go): element write to whole-container read (3 to 4 days)

Create `src/cpg/reaching/element_recall.rs` (declared in `src/cpg/reaching.rs`, called beside
`contribution::analyze` at line ~605); emit beside `recall.emit` in `src/data_flow/build/file.rs` (~811); add
persisted `RdFileStats` counters `dfg_element_recall_*` (in `nav dfg-stats` and `call-stats.dfg_labels`);
matrix `"S5"`; cache version; docs. Modelled on `contribution.rs`: Python, JS/TS/TSX and Go owners with RD;
sources are non-alias recorded defs with a keyed component; consumers are real-span uses of a strict prefix
ending before the first subscript component that the source reaches with no kill of its path or a prefix; the
1,000,000-candidate budget and skip counters; cold, full-hit and partial-hit builds agree (a cache test). Label
per item 7. Tests first in `src/cpg/reaching/element_recall_tests.rs` (declared in `src/cpg/reaching.rs`):
`element_write_reaches_later_bare_read`, `killed_element_gives_no_recall`, `over_budget_owner_is_skipped`,
`cold_full_and_partial_hits_agree`; then `"S5"` RED → GREEN. Class `element-recall`; Tier-A; cloud tests; two
reviewers; PR body as S1.

### Slice S6 (stage D go): static-key initialisers (3 to 4 days)

Create `src/ast/static_key_literals.rs` (declared in `src/ast.rs`); one call in `src/ast/lvalues.rs`; matrix
`"S6"`; cache version; docs. A literal assigned to a plain name whose keys all pass the key rule (no spread,
no computed key) adds per entry a def of the entry's keyed (JS: field) path over the entry's bytes; the
container's def stays. The trace engine joins every use and def on a line, so only entries on their own line
separate; single-line literals stay an accepted loss unless stage D says otherwise. Tests first in
`src/ast/static_key_literals_tests.rs`: `multi_line_entries_get_keyed_defs`, `spread_or_computed_key_refuses`,
`single_line_literal_unchanged`; then `"S6"` RED → GREEN. Class `initialiser-def` (existing edges of the
container's def unchanged); Tier-A; cloud tests; two reviewers; PR body as S1.

---

## Stage G and H: gates and close-out (2 days)

- [ ] Final head: full matrix; differential against `base/` on the seven corpora; `cross-key` 0; R unchanged.
- [ ] Sample: 200 added edges by `tools/sample_edges.py` (seed 10, stratified by corpus and class), five lines
      of context each, read by Opus-5.5 (high): real flow or not. Gate ≥ 95%.
- [ ] Engines: per row, `taint_reaches` verdict beside truth; a `not` row reached only through a non-`exact`
      W→R edge agrees under the label rule and is listed. `tools/review_taint_rows.py` records review taint
      per executable row (`prism --repo <row dir> --diff <one-line unrelated diff> --algorithm taint --format
      json`, research caveat 10); no change required.
- [ ] C5 with the head build. `g/REPORT.md`: every gate with its numbers.
- [ ] Docs: `CLAUDE.md` (a keyed-subscript paragraph; #674 and write-census paragraphs),
      `docs/features/cpg/architecture.md:313` (resolved question 3), `js-ts-known-gaps.md` (§6 `p.x[k] = 1`,
      ~245, and a subscript entry), `docs/features/language-coverage/language-analysis-gaps.md`.
- [ ] Follow-up issues: DESIGN 8's five; statement-scoped same-line propagation in `trace.rs` (P19); alias
      reads (P16, P29); subscript paths in `member_projection.rs` and `step5c.rs`; one static-key policy shared
      with `object_literal_args::literal_key`. Report on the issue (close it if every gate holds); refresh
      `HANDOFF.md` (`~/.claude/handoff-template.md`) and `LEDGER.md`; offload raw dumps; delete `target/`.

## Acceptance for the lane

Every gate of DESIGN 7 holds on the merged head (matrix, per-slice differentials, no flood, sample ≥ 95%,
engines recorded, spellings listed); both reviewers approved S0 to S4 and each conditional slice that ran; the
report is posted. A failing gate is reported with its numbers and the lane stops for the owner.

## Estimates (working days, review rounds included)

| M | T and E | D | S0 | S1 | S2 | S3 | S4 | G and H | Total | S5, S6 (each, conditional) |
|---|---|---|---|---|---|---|---|---|---|---|
| 2–3 | 3–4 | 1–2 | 1–2 | 4–5 | 2–3 | 3 | 3–4 | 2 | **21–28** | 3–4 |
