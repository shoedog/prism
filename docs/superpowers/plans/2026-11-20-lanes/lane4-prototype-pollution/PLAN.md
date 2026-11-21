# Lane 4: prototype-pollution tracing — implementation plan

> **For agentic workers:** this plan is run by an orchestrator under `../ORCHESTRATION.md`. Each task is
> dispatched to an implementer (Sonnet-5.5, high) with its section as the brief. Steps use checkbox syntax.

**Goal:** first, an experiment that says how many benchmark cases a prototype-pollution sink would report
and how often it would fire on ordinary code; then, if the pre-stated gate passes, syntactic facts for
pollution-capable dynamic-key writes, a CLI listing of them, and a review-taint sink that fires when taint
reaches a selector key operand of an unguarded one.

**Architecture:** facts are computed per file from syntax in the core crate
(`src/ast/js_ts/proto_writes*`); `prism nav proto-writes` lists them; the review crate's sink check consumes
them from one call in the taint sink loop and decides by name which operand is tainted. No data-flow
producer or cache change.

**Tech stack:** Rust (core and prism-review crates, tree-sitter JavaScript/TypeScript); Python 3 for the
benchmark harness and joins; Node 24 for the truth runner (lane-written fixtures only).

**Spec:** `DESIGN.md` in this directory (facts and roles 5.1, guards 5.2, sink 5.3, listing 5.4,
experiment 6, truth 7, gates 8, owner decisions 10). Every rule is stated there; this plan only references it.

## Global constraints

- Two worktrees: `~/code/prism-pp` for product slices (sparse-checkout excludes `eval/secbench/`), and
  `~/code/prism-pp-meas` for everything under `eval/secbench/` and for measurement (full checkout). Changes
  to the harness are separate eval-only PRs made in the second worktree. Evidence `~/prism-evidence/pp/`.
- Benchmark inputs are read, never executed: `~/prism-evidence/inputs/secbench-js`, `…/secbench-pkgs`.
  Baseline rows: `~/prism-evidence/meas/secbench/repair-r1/final/entries.jsonl`.
- Only fixtures written for this lane run under Node, contained as DESIGN section 7 item 7 says (permission
  model, empty working directory, cleared environment, 5 second limit, refused network modules). The
  orchestrator reads every new row before its first run.
- Briefs for codex seats use neutral wording: "benchmark accounting", "entry point", "target write",
  "key operand". Product implementers work from `tests/fixtures/proto_writes/` and do not open the
  benchmark tree. Readers of benchmark functions (E3, E5) are Claude seats.
- The harness run and the real-diff run are measurements: host free, through memguard, one at a time
  (ORCHESTRATION section 4).
- No change to `src/ast/lvalues.rs`, `src/access_path.rs`, `src/data_flow/`, `src/cpg/` or the cache version.
  The core test `tests/ast/proto_key_use_test.rs` reads data-flow output and changes nothing.
- `src/navigation/queries.rs` is over the size ceiling: the listing's logic goes in a new module.
- PR #841 edits `taint/run.rs`: the sink slice rebases over it (DESIGN decision 8).
- Rule tag `js-proto-write`, CWE `CWE-1321`. Class names in output: `nested_keys`, `path_walk`,
  `recursive_merge`. Guard names: `none`, `partial`, `sufficient`. Operand roles: `selector`, `property`.
- The orchestrator files the data-flow issue for the receiver false positive (DESIGN 5.3, decision 10) at
  the end of stage M, with the probe of DESIGN section 2, and records its number in the ledger; rows and
  docs that mention it use that number.

## Review focus

1. TypeScript wrappers on the target or key (`(o as any)[k] = v`, `o![k] = v`, `o[k as string] = v`): the
   write is seen through them. (S1a)
2. Logical and destructuring assignment targets (`o[k] ??= {}`, `[o[k]] = [v]`, `({ a: o[k] } = src)`):
   counted as dynamic-key writes. (S1a)
3. A very large generated or minified file: analysis stays linear; a function with more than 10,000
   subscript writes is skipped and counted in a warning, never a hang. (S1a)
4. Two dynamic-key writes on one line with different guard states: a Use whose span lies inside one write's
   target is matched to that write alone; a Use outside every target (the zero-width anchor, a right-hand
   occurrence) is matched to every write on the line with a selector of its base, and the line fires when
   any of them is not sufficiently guarded (DESIGN 5.3). (S2)
5. A cursor captured by a nested callback (`path.split('.').forEach(p => { o = o[p]; })`): classified with
   the function that declares the cursor. (S1a)
6. Where the key's Use sits: multi-line targets, methods and arrow owners, TypeScript wrappers on the key,
   and a key that also occurs on the right-hand side; the sink matrix rows `skx-*` cross them. (S2)

---

## Stage M: the experiment (3 to 4 days)

### Task M-1: workspaces and a main build

- [ ] Create both worktrees and the evidence directory. In `prism-pp-meas`, build release through memguard
      and record `prism --version` and the binary's SHA-256.
- [ ] Comment on #603: the lane has started; the issue's premise is corrected in `DESIGN.md` section 2.

### Task M-2: harness class filter and recorded pin overrides (eval-only PR, in `prism-pp-meas`)

**Files:**
- Modify: `eval/secbench/run.py`, `eval/secbench/README.md`
- Test: `eval/secbench/test_cli.py` (it already drives `run.main()`); run with
  `python3 -m unittest discover -s eval/secbench -t .`

**Interfaces (produced):** command-line options
`--class <name>` (repeatable), `--binary-sha <hex>`, `--sut-commit <sha>`.

- `--sut-commit` replaces the built-in `SUT_COMMIT` everywhere `main()` uses it: the
  `git diff --exit-code <commit> -- src Cargo.toml Cargo.lock build.rs vendor` check in `--sut-repo`, the
  check that the binary's `--version` names the commit, and the `sut_head` recorded in `binding.json`.
  `--binary-sha` replaces `BINARY_SHA` in the binary authentication.
- `--class` restricts the **measured** population only. Inspection verification, the comparison with
  `input-pins.json` and the acquired-population count still run on all entries. The measurement loop, the
  progress counter, `entries.jsonl` and the summary cover only entries of the named classes.
- The summary JSON gains `"class_filter": [...]` and `"pins_overridden": {"binary_sha": …, "sut_commit": …}`
  (both absent when unused). Without any of the three options, behaviour and output are unchanged.
- A run with overrides is not comparable byte for byte with the baseline (`harness_sha256` in `binding.json`
  changes with the harness edit); the README says so.

- [ ] Tests first, in `test_cli.py`: `test_class_filter_measures_only_that_class`,
      `test_pin_overrides_replace_both_checks_and_are_recorded`, `test_default_run_is_unchanged`.
- [ ] Implement; tests pass; one reviewer (eval-only); merge.

### Task M-3: measure E1

- [ ] From the repository root of `prism-pp-meas`, through memguard, host free:
      `python3 -m eval.secbench --no-build --binary <main build> --binary-sha <its sha256> --sut-commit
      <the build's commit> --class prototype-pollution --inspection
      ~/prism-evidence/meas/secbench/repair-r1/inspection-final.jsonl --out ~/prism-evidence/pp/e1`.
      The worktree must be at the build's commit with no change under `src`. Keep `e1/raw/` whole: E2 reads
      `raw/prototype-pollution/<entry>/witness.json.gz`, and the yield gate later reads `taint.json.gz`.
- [ ] `e1/SUMMARY.md`: outcome counts beside the baseline's (41 traced of 95 with ground truth), and the
      list of case ids whose outcome changed, each with the baseline and current outcome.

### Task M-4: E2 to E5 scripts, labels and runs

**Files (evidence directory):** `m/key_reached.py`, `m/classify_proto.mjs`, `m/e3_labels.jsonl`,
`m/e4_labels.jsonl`, `m/e5_guards.jsonl`, outputs under `m/`.

- [ ] `key_reached.py <e1 out dir> <packages dir>` (DESIGN section 6, E2): for each traced case of
      `entries.jsonl`, read `raw/prototype-pollution/<entry>/witness.json.gz` (gunzip; the
      `reasoning.per_sink[]` records: each `sink` is a Use with file, line, path and bytes, each `sources[]`
      entry a root with its `reachability`) and the row's `sink.{file,line,column}` and `source`; parse the
      sink file (read, never run) to find the write at that line and column and the identifiers and member
      chains inside its index expressions, with their byte spans. Apply the harness's own `data_root` test
      (import it from `eval/secbench/run.py`; do not copy it) to pick data roots. Output per case: id,
      `by_name` yes or no, `strict` yes or no, the Reached Uses (path, bytes), the index identifiers. Unit
      test on two hand-made cases (a zero-width anchor that converts by name only; a byte-exact one).
- [ ] `classify_proto.mjs <file> <line>` and a directory mode: a prototype of the classes of DESIGN 5.1 in
      the strict reading, a guard-token report (does the function mention `__proto__`, `constructor`,
      `prototype`, `hasOwnProperty`, `Object.create(null)`), and an `aliased_receiver` flag (a `path_walk`
      cursor initialised from a parameter in a function whose formals share one line; DESIGN 5.3), written
      with the pinned TypeScript parser. It is a measuring tool, not the product rule; keep it under 400
      lines.
- [ ] E3: a Sonnet-5.5 reader (a Claude seat) reads each traced ground-truth function from the packages
      (read only) and writes `m/e3_labels.jsonl`: case id, strict class, shape label (DESIGN section 6,
      "E3 labels"; no other label values), guard form and placement, a `coincidental_credit` flag (the
      key is credited only through a name that belongs to another variable), and one sentence of reason.
      Run `classify_proto.mjs` on the same functions; list every disagreement between script and reader in
      the memo (the reader's label counts).
- [ ] E4: `classify_proto.mjs` in directory mode on Excalidraw, `~/code/bench-repos/TypeScript/src`,
      `~/code/bench-repos/express`, `~/code/bench-repos/directus-directus/api` and
      `~/code/bench-repos/calcom-cal.com/apps/api`. If they give fewer than 20 class hits, add
      `~/code/bench-repos/apache-superset/superset-frontend`, then `~/code/bench-repos/apache-answer/ui`,
      stopping once 20 is reached. Record each corpus's commit. A Sonnet-5.5 reader labels up to 40 hits
      (sort by file and line, then sample with `random.Random(20261120)`), reading each function: can
      pollute if the key is attacker-controlled / cannot / guarded.
- [ ] E5 (required: DESIGN 5.2 and decision 7): for up to 20 converted packages, find the fixed version in
      the benchmark entry's own metadata under `~/prism-evidence/inputs/secbench-js` (look at one entry to
      see which field names it; skip a package whose entry names none). Fetch each with
      `npm pack <name>@<version>` into `~/prism-evidence/pp/fixed/` (archives unpacked, never installed or
      run); a Claude reader records, per package, the guard form and its placement (which operand, before
      which step) in `m/e5_guards.jsonl`. Each distinct form becomes an input row for task T-2. If the
      network refuses, write "fixed versions owed: no guard suppresses" in the ledger; the fallback of
      DESIGN 5.2 then applies to every later slice.

### Task M-5: memo and gate

**Files:** `docs/superpowers/plans/2026-11-20-lanes/lane4-prototype-pollution/EXPERIMENT.md`

- [ ] Write the memo with the contents DESIGN section 6 lists, starting with the per-case list of traced
      cases that do not convert and their reasons.
- [ ] Check the contradicted-premise condition of DESIGN section 6 on that list. If it holds, do not apply
      the gate: write the problem for ORCHESTRATION section 8 and stop there.
- [ ] Otherwise apply the gate to C under the strict reading, with the liberal reading beside it. C ≥ 25:
      commit the memo, post the table on #603, continue. 15 ≤ C < 25: post and stop for the owner. C < 15:
      post, close the lane.
- [ ] File the data-flow issue for the receiver false positive (global constraints), with the aliased-receiver
      exposure counts from the memo.

---

## Stage T and E: executable matrix (4 to 5 days)

### Task T-1: fixtures, the truth runner and the core key-Use test

Stage T produces data and one core test of existing behaviour, not tests of new code: the modules those tests
would name do not exist yet. Each slice adds its Rust test in its first commit and shows it RED before
implementing.

**Files:**
- Create: `tests/fixtures/proto_writes/cases/<row>.js` (one exported function per file; `.ts` for TypeScript
  rows), `tests/fixtures/proto_writes/rows.jsonl`, `tests/fixtures/proto_writes/sink_rows.md` (the sink rows
  below, as source text with `// SEED` and `// SINK` markers, for slices S2 and S3),
  `tests/fixtures/proto_writes/truth/run_truth.mjs`, `tests/fixtures/proto_writes/truth/families.json`,
  `tests/fixtures/proto_writes/truth/truth.jsonl` (committed output, after task T-1b),
  `tests/fixtures/proto_writes/truth/check.sh`, `tests/fixtures/proto_writes/README.md`,
  `tests/ast/proto_key_use_test.rs`
- Modify: `tests/ast/main.rs` (its `mod proto_key_use_test;` line)

`rows.jsonl` fields: `id`, `file`, `write_line`, `call`, `attacker_names` (DESIGN section 7: the names at the
write that carry attacker data), `truth` (`pollutes`, `safe`; copied from `truth.jsonl`), `want_class` (a
class name or `none`), `want_guard`, `class` (what the row is an instance of), `baseline_cases` (optional:
benchmark case ids of the same shape), `accepted_loss` (optional reason). After stage D the implementer
updates `want_class` and `want_guard` from the amended design in the same commit as the design; `truth`
never changes by hand.

`call` fields (DESIGN section 7): `arg` (the position of the attacker argument); `template` (one of
`{"kind":"path_string","delimiter":".","leading":""}`, `{"kind":"path_array"}`,
`{"kind":"flat_object","delimiter":"."}`, `{"kind":"nested_object"}`, `{"kind":"text","syntax":"argv"|"ini"}`);
`array_segments` (whether the array-wrapped families apply); `root` (optional: `{"arg": <position>,
"kinds": [...]}` from `plain`, `null_proto`, `class_instance`, `array`); `args` (JSON values for the other
positions); `control` (optional: segments and the expected effect as a JavaScript expression over `root`
and the return value; default segments `a`, `b`, value 1).

`run_truth.mjs <cases dir> <rows.jsonl> <families.json>`: implements DESIGN section 7 items 1 to 8 exactly:
for each row, family, root kind and module mode, a fresh contained child process runs a generated driver that
loads the case (a CommonJS wrapper and an ES-module wrapper of the same text), calls it as `call` says, and
prints the prototype snapshot difference; the control runs the same way. Writes `truth.jsonl` (Node version
first, then one record per row with every result, the control's result and the row's truth: `pollutes`,
`safe` or `inadmissible`). A unit test of the runner itself (`truth/run_truth.test.mjs`, run with
`node --test`): a row that pollutes only as sloppy CommonJS is `pollutes`; a row whose control throws is
`inadmissible`; a case file that names `http` is refused.

`tests/ast/proto_key_use_test.rs` (DESIGN section 2, the key's Use on the write line): builds the CPG for small
in-test sources and asserts that an identifier inside the index of an assignment target has a Use on that
line (any byte span, zero width included), for a function declaration, a method, an arrow bound to a `const`
and an anonymous callback, and for the write forms `o[k] = v`, `a[k1][k2] = v`, `t[key] = s[key]` and
`o[keys[keys.length - 1]] = v`. It names only existing items, passes on `main`, and needs one reviewer
(test-only). Run with `cargo test --test ast proto_key_use_test::` through memguard.

Seed rows (truth is whatever the runner prints; the expectation column is the hypothesis to check):

| Row | Function | Expect |
|---|---|---|
| pw01 | the path setter of DESIGN section 1; roots `plain`, `null_proto`, `class_instance`, `array` | pollutes; `path_walk`; guard none |
| pw02 | `while (parts.length > 1) { const k = parts.shift(); obj = obj[k] = obj[k] \|\| {}; } obj[parts[0]] = v;` | pollutes; `path_walk` |
| pw03 | `path.split('.').forEach(function (p, i, a) { if (i === a.length - 1) obj[p] = v; else obj = obj[p] = obj[p] \|\| {}; });` | pollutes; `path_walk` |
| pw04 | `keys.slice(0, -1).reduce((o, k) => (o[k] = o[k] \|\| {}), obj)[keys[keys.length - 1]] = v;` | pollutes; `path_walk` |
| pw05 | recursive setter: `function set(o, keys, v) { if (keys.length === 1) o[keys[0]] = v; else set(o[keys[0]] = o[keys[0]] \|\| {}, keys.slice(1), v); }` | pollutes; `recursive_merge` or `path_walk` (stage D names it) |
| pw06 | JSON-pointer setter over `pointer.split('/')`; template `path_string`, delimiter `/`, leading `/` | pollutes; `path_walk` |
| pw07 | unflatten: keys such as `"a.b"` expanded into nested objects; template `flat_object` | pollutes; `path_walk` |
| pw08 | `function setIn(o, path, last, v) { for (const k of path) o = o[k] = o[k] \|\| {}; o[last] = v; }`, attacker argument `path` (array), attacker names `k` | pollutes; `path_walk` |
| pw09 | pw08's function, attacker argument `last` only (`path` a benign constant), attacker names `last` | safe; fact present, `last` only a property operand |
| pw10 | read-only walk then a fixed-name write: `for (const s of path.split('.')) o = o[s]; o.enabled = true;` | pollutes; class decided by stage D (no fact under 5.1 as written) |
| pw11 | pw10 with `o[KEY] = v` for a module constant `KEY` | pollutes; class decided by stage D |
| pw12 | walk over the characters of a word: `for (const ch of word) cur = cur[ch] = cur[ch] \|\| {};`, template `path_array` for a list of strings and `path_string` with an empty delimiter for a string | by the runner (expected: safe for a string, pollutes for an array of strings) |
| nk01 | `function put(a, k1, k2, v) { a[k1][k2] = v; }` | pollutes; `nested_keys` |
| nk02 | `for (let r = 0; r < n; r++) for (let c = 0; c < m; c++) grid[r][c] = 0;` | safe; no fact |
| nk03 | `a["fixed"][k] = v` | safe; no fact |
| nk04 | `function put(store, kind, id, v) { store[kind][id] = v; }`, attacker argument `id` only, attacker names `id` | safe; fact present, `id` only a property operand |
| nk05 | `var i, j; for (i = 0; i < rows.length; i++) for (j = 0; j < rows[i].length; j++) m[i][j] = rows[i][j];` (baseline: `libnested`, `nodee-utils`, where such a counter is the only index identifier reached) | safe; no fact |
| nk06 | `rows[rows.length - 1][k] = v` | safe; no fact |
| nk07 | `for (let i = 0; i < rows.length; i++) { const row = rows[i]; row[col] = v; }` | safe; no fact |
| nk08 | `list.forEach((x, i) => { out[i][k] = x; })`; and one row each for an index from `entries()` and from `indexOf` | by the runner (expected safe); stage D decides which index sources count as numeric |
| nk09 | `out[parseInt(a, 10)][k] = v`; and one row each for `out[+a][k]` and `out[a \| 0][k]` | by the runner (expected safe) |
| nk10 | `` cache[`user:${id}`][f] = v `` | safe; no fact expected after stage D |
| nk11 | `obj['__' + k + '__'][f] = v` | pollutes (`k = "proto"`); `nested_keys` |
| nk12 | `a[c ? 'x' : 'y'][k] = v` | safe; no fact expected after stage D |
| nk13 | `handlers[EventType.Click][phase] = v` with `EventType` a module constant object | safe; no fact expected after stage D |
| nk14 | TypeScript: `function put(grid: Cell[][], y: number, x: number, v: Cell) { grid[y][x] = v; }`, `y` from JSON text | pollutes; `nested_keys` (a `number` annotation is no exclusion) |
| nk15 | `map.get(k1)[k2] = v` with `map` a `Map` | safe; no fact |
| rm01 | the recursive merge of issue #603; roots `plain`, `null_proto` | pollutes; `recursive_merge` |
| rm02 | the same with `Object.keys(source).forEach(key => { … })` | pollutes; `recursive_merge` |
| rm03 | merge that recurses through a same-file helper (`mergeObject` ↔ `mergeValue`) | pollutes; `recursive_merge` |
| rm04 | one-level copy: `for (const k in s) t[k] = s[k];` | safe; no fact |
| rm05 | deep clone that builds a new object and never writes into an argument | by the runner |
| np01 | `Object.defineProperty(o, '__proto__', { value: v })` | safe; no fact |
| np02 | `Object.assign(o, { ['__proto__']: v })` | safe; no fact |
| np03 | `Object.assign({}, JSON.parse(text))` | safe; no fact |

Guard rows (DESIGN 5.2; placements are stated against selectors):

| Row | Function | Expect |
|---|---|---|
| g01 | rm01 with `if (key === '__proto__' \|\| key === 'constructor') continue;` | safe; guard sufficient |
| g02 | rm01 with `if (key === '__proto__') continue;` | pollutes; guard partial |
| g03 | rm01 with `if (!source.hasOwnProperty(key)) continue;` | pollutes; guard none |
| g04 | pw01 with `if (!Object.prototype.hasOwnProperty.call(o, k)) o[k] = {};` before each step | by the runner |
| g05 | pw01 with the root from `Object.create(null)` | pollutes (`a.__proto__.polluted`); guard none |
| g06 | pw01 with `if (['__proto__', 'constructor', 'prototype'].includes(k)) return;` on each walk key | safe; guard sufficient |
| g07 | pw01 with a regular-expression test of the key | safe; expected accepted loss (fact not sufficient) unless stage D admits the form |
| g08 | guard written after the write | pollutes; guard none |
| g09 | guard on a different variable than the key | pollutes; guard none |
| g10 | guard inside a helper `isSafeKey(k)` | safe; expected accepted loss |
| g11 | pw01 with `if (k === '__proto__') return;` on each walk key | pollutes as sloppy CommonJS (`constructor.prototype`), safe as an ES module; guard partial |
| g12 | a setter that accepts `Array.isArray(path) ? path : path.split('.')`, with the three-name `===` guard on each walk key | pollutes (`[["__proto__"], "polluted"]`); guard not sufficient |
| g13 | g12 with the guard written as `includes` on a literal array | pollutes; guard not sufficient |
| g14 | g12 with loose `==` | by the runner |
| g15 | pw01 with the positive form `if (k !== '__proto__' && k !== 'constructor' && k !== 'prototype') { step }` | safe; guard sufficient |
| g16 | pw01 with a module-level `const UNSAFE = new Set([...]); if (UNSAFE.has(k)) return;` | safe; guard sufficient |
| g17 | pw01 with a module-level array and `if (BLOCKED.indexOf(k) !== -1) return;` | safe; guard sufficient |
| g18 | pw01 with `if (k in Object.prototype) return;` | by the runner |
| g19 | pw01 with `if (Object.prototype.hasOwnProperty.call(Object.prototype, k)) return;` | by the runner |
| g20 | pw01 with a test of the whole path before splitting (`if (path.includes('__proto__')) return;`) | by the runner (expected: pollutes through `constructor.prototype`) |
| g21 | pw01 with a regular expression on the whole path before splitting | by the runner |
| g22 | pw01 whose segments pass a `filter` that drops the three names | by the runner |
| g23 | pw01 with a sanitiser that renames a forbidden segment before the step | by the runner |
| g24 | rm01 with `if (!Object.prototype.hasOwnProperty.call(target, key))` guarding the recursive descent | safe; guard by stage D |
| g25 | rm01 with a composite predicate (`Object.hasOwn(target, key) && Object.prototype.propertyIsEnumerable.call(target, key)`) before the descent | by the runner |
| g26 | a setter with `typeof k === 'string' && k !== '__proto__' && k !== 'constructor' && k !== 'prototype'` on each walk key, array paths accepted | safe; guard sufficient |
| g27 | pw01 whose final write is `Object.defineProperty(o, last, { value: v, enumerable: true, writable: true, configurable: true })` | by the runner |
| g28 | an ini-style section writer that redirects to a scratch object when the section name is `__proto__` | by the runner |
| g29 | pw01 with a cursor identity check `if (o === Object.prototype) return;` after each step | by the runner |
| g30 | pw01 with null-prototype intermediates (`o = o[k] = o[k] \|\| Object.create(null)`) | by the runner |
| g31 | pw01 with the three-name guard on the final key only | pollutes; guard none (a property guard does not count) |
| g32 | pw01 with the three-name guard on the walk keys only | safe; guard sufficient |
| g33 | nk01 with `if (k1 === '__proto__') return;` | safe; expected accepted loss (fact `partial`) |
| g34 | pw01 with the three-name guard, then `k = k.toLowerCase()` (and one row with `trim()`) before the step | pollutes (`"__PROTO__"`, `" __proto__ "`); guard none |

Shapes outside the classes as written (DESIGN 5.1, E3 shape labels):

| Row | Function | Expect | Baseline cases |
|---|---|---|---|
| hw01 | walk in a helper, write through its result: `const parent = walk(obj, keys); parent[last] = v` | pollutes; no class (`helper_walk`, a known miss) | `convict`, `decal`, `getobject` |
| at01 | `const u = users[id]; u[field] = v` | pollutes; class decided by stage D (`alias_two_step`) | `component-flatten` |
| at02 | `p = out[section] = out[section] \|\| {}; p[key] = v` | pollutes; class decided by stage D | the ini-parser shape |
| rp01 | merge recursing through a property: `exports.merge(target[key], source[key])` | pollutes; class decided by stage D (`property_recursion`) | `hoek`, `merge-change`, `safe-obj` |
| rp02 | the same through `this.merge(…)` in an object's method | pollutes; class decided by stage D | — |
| ra01 | `clone = target[name]; … extend(deep, clone, copy)` | pollutes; class decided by stage D (`alias_arg_recursion`) | `putil-merge` |
| ra02 | `target[key] = merge(target[key] \|\| {}, source[key])` | pollutes; class decided by stage D | — |
| rr01 | return-style recursion: `obj[key] = setPath(obj[key], rest, v)` | pollutes; `recursive_merge` (or `path_walk` per pw05's decision) | `keyget`, `object-path-set`, `json8-merge-patch`, `paypal-adaptive` |
| ic01 | a cursor moved through an intermediate variable (`const next = o[k]; o = next;`) | pollutes; class decided by stage D (`indirect_cursor`) | `objection`, `strikeentco-set`, `undefsafe` |
| ic02 | a cursor moved through a ternary (`o = o[k] ? o[k] : (o[k] = {})`) | pollutes; class decided by stage D | — |
| ic03 | a write through the parent of the cursor (`parent = o; o = o[k]; … parent[k2] = v`) | pollutes; class decided by stage D | — |
| fc01 | a cursor held in an object field: `st.cur = st.cur[k]` | pollutes; no class (`field_cursor`, a known miss) | — |
| dc01 | a destructured cursor: `({ [k]: o } = o)` | pollutes; class decided by stage D | — |
| im01 | an iterative merge with an explicit stack | pollutes; class decided by stage D | — |
| oa01 | `Object.assign(target[key], source[key])` as the leaf of a recursive merge | pollutes; class decided by stage D | — |
| rs01 | `Reflect.set(a[k1], k2, v)` | by the runner; class decided by stage D | — |
| rs02 | `Object.defineProperty(cursor, last, { value: v })` after a walk | by the runner; class decided by stage D | one minified case |

- [ ] Write the cases, the runner and its unit test, `families.json` with the initial family set of DESIGN
      section 7 item 1, and `rows.jsonl`. Run the runner. Every `inadmissible` row is fixed (its `call` or
      control corrected) and re-run until none is left; a row is never relabelled `safe` to clear it. Do not
      commit `truth.jsonl` yet: task T-1b first.
- [ ] Write the core key-Use test; it passes on `main`.
- [ ] Write `sink_rows.md` (all in one file per row, so the existing single-file sink test support can run
      them):
      - `sk01` pw01 called with a seeded path: fires at the final write's line and at the walk step's line
        (both asserted).
      - `sk02` only the value seeded: silent.
      - `sk03a` only the receiver seeded, `obj[k1][k2] = v` with no alias: silent. `sk03c` the same for
        `const u = obj[k1]; u[k2] = v`: silent.
      - `sk03b` only the receiver seeded on pw01 (cursor alias `let o = obj`, formals on one line): fires
        today; asserted as the known false positive of DESIGN 5.3 with the issue number in a comment, so a
        data-flow fix turns it red.
      - `sk04` sufficient guard: silent. `sk05` partial guard: fires and the description says so.
      - `sk06` `cache[id] = v` with seeded `id`: silent.
      - `sk07` a caller function in the same file passes the seeded value to the path setter: fires at the
        setter's write line.
      - `sk08` the key is a member expression (`o[opts.key][name] = v` with `opts.key` seeded): fires.
      - `sk09` a recursive merge whose key comes from `for…in` over a seeded object: fires (slice S3).
        `sk10` the same with a sufficient guard: silent (S3). `sk11` a recursive merge whose key comes from
        `Object.keys(source).forEach(key => …)`: silent today because callback parameters are not bound
        (#434), recorded as an accepted loss (S3).
      - `sk12` nk04's function with only `id` seeded: silent (property only). `sk13` pw08's function with a
        constant path and seeded `last`: silent.
      - `sk14` an Express handler in the file writes `store[req.params.kind][req.params.id] = 1`: fires (the
        framework-source selector of DESIGN 5.3).
      - `sk15` `out[k1][k2] = eval(code)` with `k1` and `code` seeded: one finding at that line, with the
        "also holds another taint sink" suffix.
      - `sk16a` two writes on one line sharing the seeded key `k`, only one guarded: fires. `sk16b` two writes
        on one line with keys `k` (seeded, its write sufficiently guarded) and `m` (not seeded): silent.
      - `sk17` a weak SQL call on the same line as a hit: severity `warning`.
      - `skx-<form>-<owner>-<rhs>`: the cross product of write form (plain statement, inside an `if`,
        compound `+=`, logical `??=`, walk step, a target spread over two lines, `as string` on the key) ×
        owner kind (function declaration, method, arrow bound to a `const`, function expression assigned to
        a property) × key also on the right-hand side (yes, no), on `a[k1][k2] = …` with `k1` seeded: fires.
      A helper in another file is not a row: review taint keeps findings in the diff's file until PR #841
      lands.

### Task T-1b: payload families and execution contexts (enumeration)

Truth depends on which payloads and contexts are tried, so this set is closed before any truth is committed.

- [ ] Run the enumerator tiers (ORCHESTRATION section 6, serially, each reading every earlier list) on
      `families.json` and the seed rows. The question: which key-segment families, path syntaxes and
      execution contexts make a function of this kind write onto a shared base object where the current set
      does not. Axes: custom delimiters and path syntaxes (`/`, brackets, `:`, argument vectors, ini text);
      flat delimited keys; prefix segments before the base name; doubled base names; non-string segments
      (arrays, numbers, objects with `toString`); names changed by a normalising step (case, surrounding
      whitespace) before `toLowerCase` or `trim`; root kinds; sloppy and strict module modes. Each entry
      names the family or context, an example function where it matters, and the seed rows whose truth it
      could change. Codex seats get the neutral wording of the global constraints.
- [ ] Add accepted entries to `families.json` (and to `run_truth.mjs` when a context is new); re-run the
      runner. The task ends when a full tier round adds nothing.
- [ ] Commit `truth.jsonl`. Where the runner contradicts the "Expect" column, the runner wins: correct the
      row's expectation and note it in the ledger.
- [ ] `truth/check.sh` re-runs the runner and diffs against the committed `truth.jsonl`. The orchestrator
      runs it before stage D and before each slice merges; it is not part of `cargo test`.

### Task T-2: enumeration

- [ ] Run the tiers on `rows.jsonl`. Axes: how the cursor moves (assignment forms, loops, array methods,
      recursion, helper calls, intermediate variables, fields, destructuring); where keys come from (`split`,
      `for…in`, `Object.keys/entries`, parameters, destructuring, `shift`/`pop`, regular-expression
      matches); which operand is attacker-controlled (selector levels, property, receiver, value); root kind
      (plain, null-prototype, class instance, array); numeric index provenance (counters, `forEach` indices,
      `entries()`, `indexOf`, coercions, last-element indices); write forms (plain, compound, logical,
      destructuring targets, `Object.defineProperty`, `Reflect.set`, `Object.assign` leaves); guard forms and
      placements (the operand guarded, string-ness of the key, a rewrite after the guard); module and
      function kinds; TypeScript syntax. The memo's no-class cases and the E5 guard forms are rows. Every
      accepted row gets its truth from the runner before it is committed.

## Stage D: design of record (1 to 2 days)

- [ ] The design seat (ORCHESTRATION section 3, stage D) amends DESIGN 5.1 and 5.2 against the full row
      set: exact class definitions, including where recursive path setters belong (decided first: it sizes
      slice S1a), whether a read-only walk followed by a fixed-name write is a fact, and the readings left
      open in 5.1; the selector and property rule, and whether property-only taint is silent or a
      `suggestion`; which index sources are numeric; the guard forms and placements that are sufficient (each
      backed by `safe` rows; strict-equality forms only on provably string keys); and the accepted-loss list
      in both directions. The implementer updates `rows.jsonl` to match. Both reviewers; cap 2 rounds.
- [ ] If stage D puts recursive path setters in `path_walk`, recursion detection for them is part of S1a and
      S1a's estimate grows by one day; otherwise it is S3's. Record which in the ledger.

---

## Stage I: slices

### Slice S1a: facts for `nested_keys` and `path_walk`, operand roles, the listing (3 to 4 days)

**Files:**
- Create: `src/ast/js_ts/proto_writes.rs`, `src/ast/js_ts/proto_writes/classes.rs`,
  `src/ast/js_ts/proto_writes_tests.rs`, `src/navigation/proto_writes.rs`,
  `crates/prism-review/tests/cli/nav_proto_writes_test.rs`
- Modify: `src/ast/js_ts.rs` (the `mod proto_writes;` and `#[cfg(test)] mod proto_writes_tests;` lines),
  `src/navigation/mod.rs`, `crates/prism-review/src/cli.rs` (`NavQuery`),
  `crates/prism-review/src/main.rs` (`run_nav` arm), `crates/prism-review/tests/cli/main.rs`, `CLAUDE.md`,
  `docs/features/language-coverage/js-ts-known-gaps.md`
- Separate eval-only PR from `prism-pp-meas`, after this slice merges: `eval/secbench/pp_join.py` and
  `eval/secbench/test_pp_join.py`

**Interfaces (produced):**

```rust
// src/ast/js_ts/proto_writes.rs
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProtoWriteClass { NestedKeys, PathWalk, RecursiveMerge }
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum GuardState { None, Partial, Sufficient }
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum KeyRole { Selector, Property }
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct KeyOperand { pub name: String, pub base: String, pub role: KeyRole,
    pub line: usize, pub start_byte: usize, pub end_byte: usize }
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
pub struct ProtoWrite { pub line: usize, pub end_line: usize, pub start_byte: usize, pub end_byte: usize,
    pub function: String, pub function_start_line: usize, pub function_end_line: usize,
    pub class: ProtoWriteClass, pub key_operands: Vec<KeyOperand>, pub guard: GuardState }
impl ParsedFile {
    /// Pollution-capable dynamic-key writes of this file, sorted by (line, start_byte).
    /// Empty for languages other than JavaScript, TypeScript and TSX.
    pub fn js_ts_proto_writes(&self) -> Vec<ProtoWrite>;
}

// src/navigation/proto_writes.rs
#[derive(serde::Serialize)]
pub struct ProtoWriteRow { pub file: String, #[serde(flatten)] pub write: ProtoWrite }
pub fn proto_writes(session: &NavigationSession) -> Vec<ProtoWriteRow>;   // sorted by (file, line, start_byte)
```

`line`..`end_line` and `start_byte`..`end_byte` are the write target's lines and span. `KeyOperand.name` is
the identifier or member chain as the data-flow graph spells it (`keys`, `opts.key`); `base` is its first
identifier; `role` and the selector/property rule are DESIGN 5.1; `line` is the operand's own line (a
selector can lie on another line than the write). In S1a `guard` is always `None`. CLI:
`prism nav proto-writes --repo <dir> [--format json|text]` (JSON: one object per line); its help text and
the `CLAUDE.md` paragraph say the subcommand is unstable. `eval/secbench/pp_join.py <harness out dir>
<packages dir> <prism binary>` runs the listing per package and prints, per case: traced, key reached by name
and strict (the rule of `m/key_reached.py`, DESIGN 6 E2, moved here with its unit test), fact class, guard,
`finding_equivalent` (DESIGN 5.4), the first failing condition when not, and `sink_finding` (whether
`raw/<class>/<entry>/taint.json.gz` holds a `taint_sink` finding at the ground-truth file and line whose
description contains `js-proto-write`; always `false` before S2).

- [ ] Module skeleton returning no facts; `proto_writes_tests.rs` is one data-driven test that reads
      `rows.jsonl` and asserts, for every enabled row, the required behaviour of DESIGN section 7 (fact
      class, guard, selector bases against `attacker_names`). A row is skipped when its `want_class` is not
      in the constant `ENABLED_CLASSES` (S1a: `nested_keys`, `path_walk`), or when its `want_guard` is not
      `none` and the constant `GUARDS_ENABLED` is `false` (S1a). RED for the rows that want a fact.
- [ ] Implement `classes.rs` (the two classes and the operand roles). GREEN for the enabled rows.
- [ ] Review focus items 1, 2, 3, 5: one test each.
- [ ] The listing and its CLI test (`lists_facts_sorted_as_json_lines`, `other_languages_list_nothing`).
- [ ] Docs: the `CLAUDE.md` paragraph; in the known-gaps doc, a line that the `reduce` shape (pw04) records a
      definition whose access path is a fragment of source text (`keys.slice(0.-1).reduce((o.k) => (o.[]`),
      harmless for this rule.
- [ ] Measure before review, host free: the listing on the E4 corpora (fact count per class; the full list
      for non-test directories goes in the PR body or an attached file).
- [ ] After the merge: the `pp_join.py` PR (with a unit test on a two-case fixture), then `pp_join.py` on the
      E1 output with the merged binary (finding-equivalents by class and shape label).
- [ ] Gates of ORCHESTRATION section 5: items 1 to 5 (Tier-A matrix only: `src/ast` is touched but no
      resolution or graph behaviour changes; say so in the PR), 8, 9, 10. Two reviewers.

**Gate:** finding-equivalents ≥ 0.9 × the converted cases (C, strict reading) whose E3 class is `nested_keys`
or `path_walk` as stage D defined them (DESIGN section 8, benchmark yield by class); the E4 sample re-labelled
against the product facts shows at least 80% able to pollute.

### Slice S1b: guards (2 to 3 days)

**Files:** create `src/ast/js_ts/proto_writes/guards.rs`; modify `src/ast/js_ts/proto_writes.rs`,
`src/ast/js_ts/proto_writes_tests.rs` (`GUARDS_ENABLED`), docs.

- [ ] Set `GUARDS_ENABLED` to `true`: RED for the guard rows. Implement the guard forms and placements of the
      amended DESIGN 5.2 (selector operands, string-ness, per-class placement, rewrites after the guard).
      GREEN.
- [ ] If the ledger says "fixed versions owed" (task M-4, E5), the sink fallback of DESIGN 5.2 applies:
      record that in the PR body; the guard state is still computed and listed.
- [ ] If slice S2 has merged, enable sink rows `sk04`, `sk05` and `sk16a`/`sk16b` in its test in this slice.
- [ ] Measure: the listing on the E4 corpora (guard state counts); `pp_join.py` on E1 (cases newly
      `sufficient`, listed for a human check).
- [ ] Gates of ORCHESTRATION section 5 as for S1a. Two reviewers.

**Gate:** every enabled row GREEN; no converted case loses its finding-equivalent except cases listed as
guarded for the human check.

### Slice S2: the sink (3 to 4 days; may start once S1a has merged)

**Files:**
- Create: `crates/prism-review/src/algorithms/taint_sinks/proto_write.rs`,
  `tests/algo/taxonomy/taint_sink_proto_write_test.rs` (and its `mod` line in `tests/algo/taxonomy/main.rs`)
- Modify: `crates/prism-review/src/algorithms/taint_sinks/mod.rs` (module line),
  `crates/prism-review/src/algorithms/taint/run.rs` (the per-edge check, the source-line check, the side set,
  the emission block's description and strength), `CLAUDE.md` (the `taint_sinks/` paragraph), the
  known-gaps doc

**Interfaces (produced):**

```rust
// crates/prism-review/src/algorithms/taint_sinks/proto_write.rs
pub(crate) const RULE: &str = "js-proto-write";
pub(crate) const CWE: &str = "CWE-1321";
pub(crate) struct ProtoWriteIndex { /* built once per file from js_ts_proto_writes() */ }
pub(crate) struct ProtoHit { pub partial_guard: bool }
impl ProtoWriteIndex {
    /// `guards_suppress` is false when the fixed-version harvest is owed (DESIGN 5.2): `Sufficient` then
    /// counts as `Partial`.
    pub(crate) fn for_file(parsed: &ParsedFile, guards_suppress: bool) -> ProtoWriteIndex;
    /// The key-hit rule of DESIGN 5.3 for one flow-path edge end. Reads `at.kind` (only a Use can hit),
    /// `at.function_start_line`, `at.line`, `at.path` (base and fields, matched by name against selector
    /// operands) and `at.start_byte`/`at.end_byte` (only to tell two writes on one line apart).
    pub(crate) fn key_hit(&self, at: &VarLocation) -> Option<ProtoHit>;
    /// The framework-source selector rule of DESIGN 5.3: a hit when a selector operand of a write on `line`
    /// overlaps one of the line's framework source-access byte ranges.
    pub(crate) fn source_key_hit(&self, line: usize, ranges: &[(usize, usize)]) -> Option<ProtoHit>;
}
```

In `run.rs` (DESIGN 5.3): `key_hit` runs once per flow-path edge on `edge.to`, outside the per-identifier
loop and before the framework-source-line skip; `source_key_hit` runs once per JS-family framework source line
with the ranges `run.rs` already computes (`framework_source_access_ranges`). A hit records the line with its
`ProtoHit` in a side set. The emission block, for a line in that set, writes the description of DESIGN 5.3
(with the "also holds another taint sink" suffix when the line is also in the existing sink set) and forces
Strong (severity `warning`) whatever `line_strength` returns. Category stays `taint_sink` (DESIGN decision 9).
`guards_suppress` is a constant in `proto_write.rs`, set from the ledger's E5 state when the slice is written.

- [ ] Rebase on `main`; if PR #841 has merged, read the new shape of the sink loop first.
- [ ] Write `taint_sink_proto_write_test.rs` from `sink_rows.md` rows `sk01` to `sk08`, `sk12` to `sk17` and
      every `skx-*` row, with the existing single-file support (`taint_sink_rules_support.rs`:
      `Case { label, source }`, `// SEED`, `// SINK`); add a helper there only for rows that read the
      finding's description (`sk05`, `sk15`). Rows `sk04`, `sk05`, `sk16a`, `sk16b` are enabled only if S1b
      has merged (otherwise S1b enables them). RED.
- [ ] Implement the index, both checks and the emission changes. GREEN.
- [ ] Review focus items 4 and 6.
- [ ] No-regression check: run the existing taint test targets (`--test algo_taxonomy`); the only changed
      expectations are new findings with the DESIGN 5.3 description, each listed in the PR body. Note that
      `test_taint_js_prototype_pollution` in `taint_lang_test.rs` passes through a `res.send` sink and is
      not evidence for this rule; leave it as it is. The core key-Use test stays green.
- [ ] Measure before review, host free: (a) re-run E1 with the slice's build (task M-3's command) and
      `pp_join.py`: `sink_finding` against the converted cases of the enabled classes (DESIGN section 8);
      (b) the real-diff run of slice S4's first step, written once to `s/diffs.jsonl` and reused there.
      Both results go in the PR body.
- [ ] Two reviewers.

### Slice S3: `recursive_merge` (3 to 4 days)

**Files:** `src/ast/js_ts/proto_writes/classes.rs` (or a new `merge.rs` beside it if `classes.rs` would pass
600 weighted lines), `src/ast/js_ts/proto_writes_tests.rs` (`ENABLED_CLASSES`),
`tests/algo/taxonomy/taint_sink_proto_write_test.rs` (rows `sk09` to `sk11`), docs.

- [ ] Add `recursive_merge` to `ENABLED_CLASSES`: RED. Implement direct recursion and one same-file helper
      hop, parameter-position matching of the dynamic read, single-alias bases, and the selector of DESIGN
      5.1 (the descent read's key); plus recursive path setters if stage D put them here. GREEN.
- [ ] Measure: listing on the E4 corpora (new facts listed), re-run E1 and `pp_join.py` (merge-class
      findings; the potential set P re-counted with the product facts).
- [ ] Sink rows `sk09` (fires), `sk10` (silent) and `sk11` (silent, the accepted loss of #434, asserted so
      a later fix shows up). Two reviewers.

### Slice S4: acceptance and close-out (2 to 3 days)

- [ ] Real-diff sink measure (DESIGN section 8, "Sink on real diffs"; first run in S2, re-run here with the
      final build). The diff set, fixed once in `~/prism-evidence/pp/s/diffs.jsonl` (repository, commit,
      parent): every diff of the #602 population (`population.jsonl` at
      `next-five-plan-fable-20261114/final-gate-campaign-preparation/` inside
      `~/prism-evidence/archive/prism-crossfile-docs-493ab5ee-receipts.tar.gz`; 6 repositories × 20 commits)
      that touches a `.js`, `.jsx`, `.mjs`, `.cjs`, `.ts` or `.tsx` file; if fewer than 60, add non-merge
      commits that touch such files from `~/code/bench-repos/calcom-cal.com` and
      `~/code/bench-repos/directus-directus`, sampled with `random.Random(20261121)`, until 60. For each,
      check the commit out in a scratch worktree of the bench-repos clone (never move the clone's own
      checkout), write `git diff <parent> <commit>` to a patch, and run
      `prism --repo <scratch> --diff <patch> --algorithm taint --format json` through memguard, host free.
      List every finding whose description contains `js-proto-write`; a Sonnet-5.5 reader labels up to 40
      (sampled with `random.Random(20261121)` when more): real / helper capable but the key is not
      attacker-controlled / cannot pollute / receiver-only through an aliased cursor (DESIGN 5.3). Report
      the tally; state percentages only on 20 or more findings.
- [ ] Re-run E1 with the final build and `pp_join.py`: the **benchmark yield** gate of DESIGN section 8 on
      `sink_finding`, by class. List converted cases the final rules call guarded, for a human check.
- [ ] Re-label the E4 sample against the final facts: the **ordinary code** gate.
- [ ] Run `truth/check.sh`.
- [ ] `g/REPORT.md` in the evidence directory and a summary on #603: C (strict and liberal), sink findings by
      class and shape, potential P by first break (linking #595 and #434), ordinary-code counts, the
      real-diff tally, the receiver false-positive counts (benchmark exposure from E3, corpora from E4, the
      real-diff label) with the data-flow issue's number, accepted losses.
- [ ] Correct `docs/features/language-coverage/js-ts-known-gaps.md` (the statement that the write is not a
      definition) and close #603. File the follow-ups of DESIGN section 9 that the numbers justify.
- [ ] Refresh the handoff; offload raw harness output and the real-diff outputs; delete both worktrees'
      `target/` and the scratch worktrees.

## Acceptance for the lane

The experiment memo is merged; if the gate passed: every gate of DESIGN section 8 holds on the merged head,
both reviewers approved S1a, S1b, S2 and S3, and the report is posted on #603.
