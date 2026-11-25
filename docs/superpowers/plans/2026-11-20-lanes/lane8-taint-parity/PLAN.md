# Lane 8: one taint answer from both engines — implementation plan

> **For agentic workers:** this plan is run by an orchestrator under `../ORCHESTRATION.md`. Each task is
> dispatched to an implementer (Sonnet-5.5, high) with its section as the brief. Steps use checkbox syntax.

**Goal:** every difference between review taint and the trace engine behind `taint_reaches` disappears or
carries a named, tested reason; the trace engine crosses the hop classes of DESIGN 5.2 on evidence that
belongs to the hop and stops returning into callers it did not enter; both clients of the trace walker take
the same crossing knobs; `taint_reaches` gains a sink catalog mode, first in the CLI, then in MCP.

**Architecture:** the descent gate moves from `src/cpg/trace.rs` to `src/cpg/trace_descent.rs`, proves the
emitting site of each hop from a syntax view passed through the walk (DESIGN 5.2) and returns a reason instead
of a boolean; the reason rides on `BoundaryEdge` into the reasoning warnings. The walk state (entered sites,
remaining budget) lives in `src/cpg/trace_frames.rs`. A descent policy (ceiling, depth) is threaded beside PR
#841's crossing policy. A `SinkCatalog` trait in core is implemented in the review crate by review's own sink
loop over each root's fan, and passed in. For the MCP server (DESIGN decision 2, branch A2, owner
2026-11-24) that implementation and the taint module's closure then move into a new crate,
`crates/prism-taint`, below both binaries (slices I-9.0 to I-9.4).

**Tech stack:** Rust; Python 3 for census scripts; the SecBench harness (`eval/secbench`). **Spec:**
`DESIGN.md` in this directory (5.1 to 5.6 rules, 7 gates, 8 owner decisions). Review probes cited by name are
in `~/prism-evidence/lanes-2026-11-20/reviews/astra-r1/lane8/probes/`.

## Global constraints

- **Start:** PR #841 merged (DESIGN decision 1). First check that `src/cpg/taint_policy.rs` still exports
  `TaintCrossingPolicy` and `CrossingCeiling { Exact, NameOnly }`, and that `taint_trace_from_root` takes
  `policy: Option<&TaintCrossingPolicy<'_>>`, as at 493ab5ee. If not: I-5 defines its own
  `DescentCeiling { Exact, NameOnly }`, and `policy_ceiling` exists only if a crossing policy still reaches
  the descent test. Record the outcome in the ledger.
- Worktree `~/code/prism-taint-parity` (sparse, without `eval/secbench`). Measurement checkout
  `~/code/prism-taint-parity-bench` (full), used only by the census runner (a Claude seat: the benchmark is
  a security corpus, so no codex seat). Evidence `~/prism-evidence/taint-parity/`.
- **Defaults unchanged** unless a row says otherwise: default review taint findings byte-identical on
  population A after every slice (gate Defaults); `taint_reaches` with default knobs moves only where a row of
  the slice's group or an explicitly named cross-group CURRENT cell says so. Assert the change in the slice that first causes it.
- **Known wrong rows** (DESIGN 9): a row with `known_wrong` set keeps today's answer until the slice named
  there; no slice may move one to a further wrong answer.
- **File size** (weighted): `src/cpg/trace.rs` is 1,138.5 on this checkout and must not grow (new
  logic in `trace_descent.rs` and `trace_frames.rs`). `crates/prism-review/src/main.rs` is 674. I-5 extracts the existing
  `NavQuery::TaintReaches` arm from `cli_nav.rs:280` into `crates/prism-review/src/nav_taint.rs` (`mod nav_taint;`
  in `main.rs`) to keep seed parsing, knob handling and I-8 catalog dispatch together. `src/reasoning/taint_reaches.rs` is 583: catalog mode goes in `src/reasoning/sink_catalog.rs`.
  `src/cpg/tests.rs` is 2,636, over the hard ceiling: a slice may edit an expectation there, never add a test.
- **MCP goldens** (`tests/fixtures/mcp/golden/*.jsonl`) hold `taint_reaches`'s schema and description. Only
  I-5 and I-9.4 re-record them: `PRISM_MCP_BLESS_GOLDEN=1 cargo test -p prism-mcp --test mcp
  golden_transcript_test`, with the reason in the commit message.
- **Tier-A** (ORCHESTRATION 5 item 5) for every slice touching `src/cpg/` (I-1 to I-5). Its two taint cases
  that expect `BoundaryExited` (`eval/fixtures/python/taint_boundary_negative`, `taint_descent_depth_bound`)
  keep their answer under the defaults, and the additive `reason` field does not break their kind check. No
  slice changes stored CPG or navigation data. Stage D chose the syntax view; graph-only consumers refuse crossings without it. Core never names `prism_review`. Engine-parity assertions live in `tests/algo/taxonomy/`.

- **Rules are cited by DESIGN section** (5.2 "Hop evidence", "Binding", "Two call sites", "Spelling",
  "Returns", "Depth"; 5.3; 5.5; 5.6), as amended at stage D, so the implementer brief carries them.

## Review focus

1. `log(t); g(t)`: descends today; the witness's source occurrence must be the one inside `g(t)` (bytes
   asserted). `other.run(t); this.run('fixed')`: the edge carrying `t` is name-only and must stay refused.
   Rows `l01`, `l06`, `l07`; I-walk, I-same-line.
2. A binding edge into `p['clean']` from `pick({bad:t, clean:'fixed'})`, or into a read in a callable nested
   in the callee: refused, `target_not_binding`. Rows `b05`, `b06`; I-binding.
3. Returns go back only through the site entered (`w01`); the answer does not depend on exploration order
   (`w03`). I-walk.
4. An aliased or direct call with a spread before the bound slot never reaches the wrong formal (`w06`,
   `w07`) while `run(a0, t)` still reaches `b`; an aliased call reaches only its resolved target (`s05`); Go
   interface dispatch keeps descending (`d01`). I-walk, I-spelling.
5. Equal knobs give the same walk on `walk_comparable` rows under `(exact,0)`, `(exact,2)`, `(nameonly,2)`, `(exact,3)`;
   the finding and catalog levels hold their own expectations. I-5, `parity_knobs_agree`.
6. Catalog Sanitized only for a sanitizer of the sink's category (`c07`); the Python template keyword, the
   Def-only flat line and the multi-line call are reported as review reports them (`c08`, `c09`, `c12`);
   catalog lines equal trace-mode review's on real code. I-7, I-8, `parity_catalog`.
7. A sink one or more functions past a refused hop is BoundaryExited with that hop's reason (`f01` to `f03`);
   a walk cell aggregates every node that intersects the operand (`m01`). I-1, T-1.

## Stage M: issue and parity census (4 days)

### Task M-1: workspace, issue, binaries

- [ ] Worktrees and evidence directory (ORCHESTRATION 4). `memguard.sh 8 cargo build --release`; copy to
      `~/prism-evidence/taint-parity/bin/prism-base`; record `--version`.
- [ ] File the tracking issue "Lane 8: one taint answer from both engines (trace parity, sink catalog)",
      linking `DESIGN.md`, this plan, #602 (parent), #596 (lane 3's F5 moves here) and #376 item 1.
- [ ] Census binary (measurement apparatus, never pushed; local branch `eval/taint-parity-census`, applied to
      the base commit only, never ported to a slice head; DESIGN 6): patch `src/cpg/trace.rs` so
      `PRISM_CENSUS_DESCENT_LOG=<file>` appends a JSON line per refused crossing and per cut that records no
      boundary (depth-0 CFG filter, suppressed shortcut, sanitizer exclusion, a visited state reached again
      with more budget): root; `from`, `to` as file, function, line, path, bytes, access; relation; depth;
      first failing condition; the observable features (the callee's sites matching by line and by
      containment, with slot and grades; whether containment singles one out; whether a caller site resolves
      to the target under another spelling). `PRISM_CENSUS_RELAX=<class>[,<class>]` crosses the hop class
      unconditionally (an upper bound, no proof): `binding`, `same_line`, `spelling`, `not_exact` (descent;
      ascent exists only where an edge does), `depth` (64). Save as `census/census-relax.patch`; build
      `bin/prism-census`.
- [ ] Apparatus check before any population run: on synthetic fixtures (two roots and two sites reaching one
      sink line, one through a refused hop; a sink needing two relaxations jointly; a new false crossing to
      an already reported line) the census reports the right cell for each. Record in `CENSUS.md`.

### Task M-2: populations and runs

**Files (evidence directory):** `census/select_population.py`, `population.jsonl`, `run_census.py`,
`classify.py`, `ledger.py` (written here, run from I-1 on), `CENSUS.md`.

- [ ] Population A (#602 M2): `~/code/bench-repos/{directus-directus, calcom-cal.com, apache-superset,
      saleor-saleor, miniflux-v2, apache-answer}`, from each the 20 latest non-merge commits touching a route,
      handler or controller path with at most 15 files changed. Use the original list if
      `tar tzf ~/prism-evidence/archive/prism-602-*.tar.gz | grep -i m2` finds it; else select by the rule and
      say so in `CENSUS.md`.
- [ ] Per diff, in a detached scratch worktree at the commit with patch `git diff <commit>^ <commit>`,
      through memguard: default `prism-base --repo <wt> --diff <patch> --algorithm taint --resolution scoped
      --format sarif`; trace mode, the same plus `--taint-return-flow`. Freeze per run: commit, loaded-file
      manifest, options, roots, binary hash (`population.jsonl`). A finding is a result with `ruleId`
      `prism/taint/taint_sink`, matched across modes by (file, start line) for yield, graded by
      `properties.confidence`.
- [ ] Lost = default minus trace; gained = trace minus default (listed in full). Each diff with a lost
      finding reruns trace mode with `prism-census` per single relaxation, then per pair for findings no
      single one recovers. **Class** of a lost finding: the minimal relaxation set that recovers it
      (`binding+spelling` is a class); `other` with the log's non-boundary cause; `unknown` if neither. The
      boundary is the logged refusal on the recovered path, matched by root and from/to identity, not by
      function.
- [ ] Whole repository: per repository and census commit, `prism-census nav --no-cache taint-reaches` frontier
      mode on the full checkout for every frozen source (at most 50 per repository, fixed seed), with the log
      on and with each single relaxation; a class's size for the ordering rule is the larger of its lost
      findings on A and its refused hops here (DESIGN 6).
- [ ] `ledger.py`: reads the product's crossing trace (I-1's `PRISM_TAINT_CROSSING_TRACE`) from those frontier
      runs and from trace-mode review runs; every accepted crossing as a path (root occurrence, site span and
      slot, target, sink operand) with its grades; a slice's new crossings are the ledger difference against
      the parent build, whether or not the sink line is new. Tested on the apparatus fixtures once I-1 merges.
- [ ] Population B: in the bench checkout, `python3 -m eval.secbench --no-build --binary bin/prism-base
      --binary-sha <sha256> --sut-commit <commit> --out census/secbench-base` (the pin options come from lane
      4 Task M-2; if unmerged, do that task here as written there). Record traced counts per vulnerability
      class: the Benchmark base. The harness's `taint.json.gz` is trace-mode review (`--taint-return-flow`),
      so B supplies no default-review comparison and no classes.
- [ ] `CENSUS.md`: default sinks, trace survivors, lost and gained; lost and gained by class with three
      examples each; whole-repository refused hops by class; grades of default findings that survive in trace
      mode (the histogram of DESIGN 5.5).

**Ordering rule after M.** DESIGN §§6–8 record the override. The three descent groups have zero recoverable A findings. Binding alone has 68 latent Use targets behind depth. Dispatch of each of the three awaits owner decision 6 (matrix contract versus measured yield, three days each). If retained, order binding, same-line, spelling. Other slices can progress. Decisions 3 and 5 retain Exact/2 defaults.

## Stage T and E: the parity matrix (4 days)

### Task T-1: rows, fixtures, data-driven tests

**Files:** Create `tests/fixtures/taint_parity/rows.jsonl`, `tests/fixtures/taint_parity/README.md`, one
directory per fixture under `tests/fixtures/taint_parity/` (source and sink lines marked `@SRC` and `@SINK`
in comments, as `~/prism-evidence/lanes-2026-11-20/probes/lane8/make_fixtures.py` does),
`tests/algo/taxonomy/taint_parity_test.rs`; Modify `tests/algo/taxonomy/main.rs` (`mod` line),
`tests/integration/coverage_test.rs` (all three lists).

Row fields (DESIGN 5.1): `id`, `group`, `lang`, `fixture`, `axes` (`mechanism`, `layout`, `resolution`,
`depth`, `sanitizer`, `direction`, `distance`); `src` (`{file, line, bytes}`), `site` (`{bytes, slot}` or
null), `sink` (`{file, line, operand, nodes}`: operand bytes, and the spans of the Variable nodes on the line
that intersect it, DESIGN 5.1); `walk_comparable` (bool); `expect`: `walk` (`{reaches:
Reached|NotReached|BoundaryExited|Sanitized|SeedError, source, terminal, descent_depth}`, witness bytes or
null), `review` and `trace_review` (`exact|nameonly|unlabeled|none`), `catalog` (required list of `{file,
line, rule, operand, verdict}`, possibly empty); `reasons` (per level, DESIGN 5.3 list, or null); `knobs`
(`{edges, depth}` or null); `truth` (`flows|no_flow`); `known_wrong` (null, or the issue or group that fixes
it); `pending` (null, or the issue whose producer the row needs; the group test skips the row and prints the
issue, so no row is `#[ignore]`d); `class`; `note` (free text; it never waives a check). `expect` is the
target behaviour after all named admissions ship. For staged or deferred answers, assert the build's `current` cell as specified in STAGE_D_ROWS.md, including rows outside the enabled target group. Rows true on main are `control`; a slice that moves a control cell
edits the row in its PR and names it.

Harness: copy the fixture to a temp directory; `prism::repo_loader::load_repo`. **review** / **trace_review**:
`CpgContext::build_with_scope_graph_inputs(&repo.files, repo.type_db.as_ref(), repo.scope_graph_inputs.as_ref())`
(the inputs `NavigationIndex::build` uses), `prism_review::algorithms::taint::slice` with `TaintConfig { sources:
vec![src], taint_from_diff: false, extra_sinks: vec![], return_flow: false | true }` and a diff listing every
line; the cell is `prism::finding_confidence::classify_with_evidence("taint", ParseQuality::Clean, ev)` of the
`taint_sink` finding at the sink line (`result.evidence` is index-aligned), else `none`. `taint_from_diff:
false` does not stop framework and IPC sources (`taint/run.rs`); the README says so. **walk**:
`NavigationIndex::build`, `taint_reaches(&session, &[Loc src], Some(&[Loc sink]))` (as
`tests/reasoning/taint_reaches_test.rs` builds its sessions); the cell is DESIGN 5.1's aggregate over the
nodes that intersect the operand, plus the witness's source and terminal bytes; a seed error is `SeedError`.

```rust
// tests/algo/taxonomy/taint_parity_test.rs
const ENABLED: &[&str] = &["control"];   // "reasons": also check the reasons of every enabled row
#[test] fn parity_rows_are_well_formed()  // unique ids; fixture exists; one @SRC, one @SINK, each marker's file
                                         // named; src and operand bytes name an occurrence on the marked line;
                                         // sink.nodes equals the minted nodes intersecting the operand;
                                         // walk_comparable only if the source line holds one occurrence and no
                                         // framework source; a reason wherever two levels differ or truth differs
                                         // from expect without known_wrong; closed vocabularies; catalog present
#[test] fn parity_default_review()
#[test] fn parity_trace_review()
#[test] fn parity_taint_reaches()        // walk level; `reasons`, `knobs`, `catalog` are data until I-1, I-5, I-8
```

**Seed rows, research probes** (today's values observed on a0f2c427):

| Row | Mechanism | review | trace today | reaches today | Group: after |
|---|---|---|---|---|---|
| js00, js10 | source and sink in one function; `exec(f(t))` | exact | exact | Reached | control |
| js01, js02, js08 | argument, same file / ESM import; `this.run(t)` | exact | unlabeled | Reached d1 | control |
| js03, js04 | `P({html:t})` into `props.html` / bare `props` | exact | none | BoundaryExited | binding: unlabeled, Reached d1 |
| js05, py03, go03 | `y = f(t); sink(y)` | unlabeled | unlabeled | Reached d1 | control |
| js09 | `o.x=t; exec(o.x)` | unlabeled | unlabeled | Reached | control |
| js06 | capture read in nested function | none | none | NotReached | control; truth flows, accepted loss |
| js07 | `arr.forEach(v => exec(v))` | none | none | line Reached at `arr`; operand `v` NotReached | control; truth flows; reasons `unmodelled` (lane 9) |
| js11 | `run(t); run(u);` on one line | exact | none | BoundaryExited | same_line: unlabeled, Reached d1 |
| js11b | the same on two lines | exact | unlabeled | Reached d1 | control |
| js12 | `handler.run(t)`, name-only single owner | nameonly | none | BoundaryExited | control, reason `not_exact` |
| js13 | a → b → c, sink in c | exact | none | BoundaryExited | control, reason `depth_cap` |
| js13b | a → b | exact | unlabeled | Reached d2 | control |
| js14, py04 | sanitizer, then sink | none | none | Sanitized | control |
| js14c | js14 without sanitizer | unlabeled | unlabeled | Reached | control |
| py01, py02 | argument, same file / import | nameonly | unlabeled | Reached d1 | control |
| go01, go02 | argument, same file / sibling file | exact | unlabeled | Reached d1 | control |
| go04 | `exec.Command(c).Run()` | none | none | NotReached | control; truth flows, accepted loss |

**Further seed rows:**

| Row | Shape | Group: required |
|---|---|---|
| l01 | `log(t); g(t)`, one site of `g`; the witness's source occurrence is the `t` inside `g(t)` | control: Reached d1, source bytes asserted |
| l02, l03 | js11 in Python and Go | same_line: Reached d1 |
| l04 | `g(g(t))` | same_line: BoundaryExited, `ambiguous_site` |
| l05 | `g(user); g(user)` (pinned by `test_two_same_line_same_name_calls_do_not_descend`, `src/cpg/tests.rs`) | same_line: Reached d1 through the first occurrence [74,78) only |
| s01 to s04 | `import { run as go }`; default import under another name; `const go = run; go(t)` (#376 item 1); Python `from b import run as go` | spelling: Reached d1 |
| s05 | alias to one of two `run` functions in different files | spelling: Reached, resolved target only |
| b01 | `<D a={x} />` into `props.a` (needs lane 3 S3's producer, else `pending: "#596"`) | binding: Reached d1 |
| b02, b03 | descent, then an object literal in the callee; one level deeper | binding: Reached d2; `depth_cap` |
| b05 | literal leaf into a read inside an arrow nested in the callee | binding: `target_not_binding` |
| k01 to k03 | js12 under `nameonly,2`; js13 under `exact,3`; return from a name-only call under `nameonly,2` | knobs: Reached d1; d3; NotReached, `no_graph_edge` |
| c01, c02 | `document.write(v)` (flat); `exec(t)` (structured, operand 0) | catalog: one sink each, with its rule |
| c03, c04 | `exec(y, x)`, only `x` tainted; `log(x); exec(x)` on one line | catalog: as review |
| c05 | sanitizer on the path before an HTML sink | catalog: Sanitized; review: none |
| c06 | sanitizer in the source function, off the path | catalog: Reached; review: none; reason `sanitizer_model` |

**Rows from the design review** (review probes named; today's values on a0f2c427):

| Row | Shape (probe) | Today | Group: required |
|---|---|---|---|
| w01 | `entry` passes `t` to `id`; `separate` reads `safe = id('fixed')` (`return_one`) | Reached; truth no_flow; known wrong | walk: NotReached, `return_not_entered_site` |
| w02 | w01 with `id(t); id('other')` on one line (`return_two`) | BoundaryExited | same_line: NotReached (returns only into the discarded `id(t)`) |
| w03 | a depth-2 route reaches `join` before a depth-1 route (`diamond`) | BoundaryExited; truth flows; known wrong | walk: Reached d2 |
| w04 | w03 without the deeper route (`diamond_short`) | Reached d2 | control |
| w05 | non-simple `return v + '!'` seeded inside the callee (`return_expr`) | NotReached, no reason; known wrong | walk: Reached |
| w06 | `run(...[], t)` into `run(a, b)`, direct import (`spread_direct`) | Reached at `b`; truth no_flow; known wrong (lane 9 slice A) | walk: BoundaryExited, `unproven_slot` |
| w07 | w06 through `import { run as go }` (`spread_alias`) | BoundaryExited | spelling: BoundaryExited, `unproven_slot` |
| w08 | `run(t)` (`spread_absent`) | NotReached | control |
| w09 | a call resolving Exact to two function expressions (`multi_exact`) | Reached | walk: BoundaryExited, `not_exact` (two `local_def` targets) |
| n01 | `cb(x); handler(0);` on one line, the edge into `cb`'s target name-only with doubt `call_nameonly`, `handler` Exact (lane 9 row h13) | — | guard: BoundaryExited, `not_exact` |
| n02 | n01 with doubt `builtin_callback` (lane 9 row e14; the kind lane 9 adds) | NotReached, `no_graph_edge`, until lane 9 adds the edge | guard: BoundaryExited, `not_exact` |
| w10 | competing shallow and deep routes under `exact,3`; mutual recursion; zero budget; descent then ascent at the cap | — | knobs: w10 Reached d2; w10b Reached d1; w10c and w10d BoundaryExited, `depth_cap` |
| b04 | `f({a:x}, false)` recursive, read after the call (`self_literal_after`) | Reached d0 over DataFlow (wrong witness) | binding: BoundaryExited, `self_param` (DESIGN 5.2 "Binding") |
| b04b | the read before the recursive call (`self_literal`) | NotReached | control |
| b06 | `pick({bad:t, clean:'fixed'})` into `observe(p['clean'])` (`literal_index`) | review exact (wrong edge; filed); BoundaryExited | binding: BoundaryExited, `target_not_binding` |
| b07, b08 | the same into `p.clean` / `p.bad` (`literal_dot`, `literal_tainted`) | NotReached / BoundaryExited | control / binding: Reached d1 |
| l06 | `other.run(t); this.run('fixed')` (`method_mixed`) | BoundaryExited | guard: BoundaryExited, `not_exact` |
| l07 | `first('fixed', t); t = 'fixed'; first(t, 'fixed')` (`rebound_site`) | review nameonly (line granularity; truth no_flow); BoundaryExited | same_line: BoundaryExited, `no_matching_site` once the version proof refutes the remaining candidate |
| x01 | `z = forget(t)` returning a literal; `document.write(z)` (`constant_return_flat`) | review reports (known wrong; filed); NotReached | control, reason `shortcut_suppressed` |
| c07 | `x = escapeHtml(t); exec(x)` (`sanitizer_domain`) | line query Sanitized (documented any-category) | catalog: Reached, XSS sanitizer advisory |
| c08 | multiline `render_template_string("{{ name \| safe }}", other="fixed", name=t)` (`template_operand`) | review reports | catalog: one sink, operand the `name=` value |
| c09 | `document.write('fixed')` on the signature line of a callee reached at its parameter Def (`flat_def_line`) | review reports (known wrong; filed) | catalog: as review |
| c10, c11 | two calls of one rule on one line, one tainted; the same sink line shape in two files | — | catalog: one entry; two entries |
| g01 | `f({a})` into `sinkA(p.a)`, binding labelled `NameOnly(SameLine)` (`object_literal_arg_tests.rs`) | trace unlabeled | binding: admit the literal pairing, preserve the weaker read evidence |

**Rows from the second review** (r2 probes named, in `~/prism-evidence/lanes-2026-11-20/reviews/fable-r2/lane8/probes/`;
today's values on a0f2c427):

| Row | Shape (probe) | Today | Group: required |
|---|---|---|---|
| f01 | `main -> a -> b -> c -> d`, sink in `d` (`b5`) | review exact; NotReached, warning in `c` | reasons: BoundaryExited, `depth_cap` |
| f02 | `handler.run(t)` name-only, `run` calls `inner(x)`, sink in `inner` (`b4`) | review nameonly; NotReached | reasons: BoundaryExited, `not_exact` |
| f03 | f02 with an object-literal hop first | — | reasons: BoundaryExited, `target_not_binding` until binding, then `not_exact` |
| m01 | `pick({a: t, b: 'fixed'})` into `observe(p.a); exec(p.b)` on one line, mixed formal (`a1`) | review exact at `exec(p.b)` (known wrong; filed); operand `p.a` has two disagreeing nodes | binding: `p.a` Reached d1, `p.b` NotReached |
| m02 | m01 with the reads swapped (`a3`) | no binding edge; both miss | control; truth flows; reason `no_graph_edge` (filed) |
| x02 | `const y = id(\n t\n); exec(y)` (`b3`) | review none; trace review unlabeled; Reached d1 | control; reason `review_no_return_flow` |
| c12 | `exec(\n u,\n t\n)` with `u = t` (`c2`) | review reports lines 6 and 7 | catalog: as review, both lines |
| d01 | Go `r.Run(t)`, interface with two implementations, both Exact (`d1`) | review exact; Reached d1 | control: Reached d1 under DESIGN 5.2 (iii) |
| d02, d03 | Go embedding promotion; Python instance of typed `Base` | both Reached d1 | walk: both stay Reached d1. d03 certifies the instance offset to `Base.run` only |
| p01 | Python `g(b=t, a='fixed')` keyword arguments (`b2`) | no edge; NotReached | control; truth flows; reason `no_graph_edge` |

- [ ] Write fixtures, rows, README (schema, vocabularies) and test. Control rows GREEN on main; every other
      row RED once with its group enabled by hand (ledger). Commit; CI green; one reviewer (test-only).

### Task T-2: enumeration

E is closed. Its historical assignments follow. The remaining classes and the end-of-tier coverage boundary are recorded in DESIGN §9. Dispatch the design rules by class; further enumeration is outside Stage D.

- [ ] Tiers per ORCHESTRATION 6 on `rows.jsonl`. Axes: mechanism of flow (argument, object literal, JSX
      attribute, return, field write, capture, callback, receiver, callee alias); call-site layout (one or
      several per line, nested, multi-line argument, occurrence outside the site, chained call, same-line
      rebinding, zero-width and continuation nodes); producer and site identity (same target with mixed
      grades, Exact multi-target pools, method and alias combinations, earlier same-name JSX attributes);
      formal read (dotted, bare, mixed, literal and dynamic subscript, prefix and sibling keys, duplicate keys
      and spreads, member writes, nested callable timing); slot (spread before or after, Python starred and
      keyword arguments, keyword, default, `*args` and `**kwargs` formals, `self`/`cls` offsets and unbound
      method calls, signature-changing decorators, JS rest and destructured formals, `call`/`apply`/`bind`, Go
      variadic and method expressions); returns (same or different caller, used or
      discarded result, recursion, wrapper); resolution grade (Exact, name-only single owner, several targets,
      one-dispatch pools, unresolved); depth (0 to 3, descent and ascent mixed, competing budgets); sink distance
      past the refused hop (the hop target, same function, one function further, two or more); direction of the
      difference (trace loses, trace gains); sanitizer (none, on path, off
      path, in callee, on one of two paths, wrong category, two categories on one line); public projection
      (several roots or uses, source equals sink, framework sources, seed errors); language (JS, TS, TSX,
      Python, Go). Census `other` and `unknown` examples become rows.

### Task E-2: catalog emitter inventory

- [ ] One enumerator (Opus-5.5, high) lists every family that adds a line to review's sink lines in
      `crates/prism-review/src/algorithms/taint/run.rs` and the structured dispatch: flat AST hits, legacy
      substring rules, C/C++ format wrappers and configured extra sinks, structured positional and semantic
      checks, JSX attributes, Python template and keyword cases, multi-line calls, JS request-source ranges and
      inline YAML, Python inline Flask, Go framework tables, inline and no-path fallbacks, synthesized
      target-seed paths, path-specific suppressions, Bash unquoted expansion. Use the matrix rows and cross explicit and diff-derived sources where representable. Pin each unsupported route's diagnostic. The inventory and row dispositions are recorded in DESIGN 5.6 and the T-2 shards. Families 9/10 need native matcher tests and 11 a configured positive in I-8.

## Stage D: design of record (2 days)

**Design delivered.** DESIGN §§5.1–5.7, 6–9 and [STAGE_D_ROWS.md](../../../../../tests/fixtures/taint_parity/STAGE_D_ROWS.md) are the implementation brief. The ledger resolves every `tbd` row and records the other changed contracts. Apply the row edits in the owning product slices, including the affected shards, witness assertions, `walk_comparable` flags for genuinely identical single roots, and documented level-specific dependencies. Designate the slice that first changes each control or intermediate reason in its receipt. E closed with open JSX ownership/formal reads and route-handler classes; the generators and exclusions in DESIGN §9 apply to future members. The review cap is two rounds. Design delivery does not attest those reviews or owner decision 6.

- [x] Brief the design seat with DESIGN 5.2, 5.3, 5.5, 5.6, the rows, `CENSUS.md`, `catalog-inventory.md`,
      the accepted-loss candidates (js06, js07, go04, census `other`/`unknown`), the known wrong rows, and these facts:
      `descend_target` requires a `Def` target and counts sites by the callee's spelling before resolving;
      it accepts one Exact target among several; its cache key omits the target node and the occurrence's end
      byte; ascent accepts any singleton-Exact call in the destination function; the visited set is
      first-come; object-literal edges end at the Use one hop from the formal's `Def`, and a computed read
      has the empty chain (`src/cpg/object_literal_args.rs`); Step 5c builds no return edge for a name-only
      call; the depth test runs before `descend_target`.
- [x] The amendment states, each in the DESIGN subsection the slices cite: 5.2 Hop evidence (i) to (v), with
      one-dispatch pools (rows d01 to d03, w09) and lane 3's line-keyed source nodes; where the proof's facts
      come from (the syntax view, or facts stored on edges: DESIGN 5.2; the second re-plans I-walk), and what
      an entry point without a view does (a required argument, or every crossing refused; never the old rule);
      5.2 Binding (key path, nested callables, row b04; or the cheaper rule: trust the producer's pairing in
      `object_literal_args::collect` and refuse only what it admits wrongly, so I-binding duplicates no
      key-path logic and edits no lane-3 file); 5.2 Two call sites (rows l01, l04 to l07); 5.2 Spelling and
      the slot certificate; 5.2 Returns and Depth (rows w01 to w10; the parent store per state and boundary
      retraction, with the two-site row `const a = id(t); const b = id(\n t\n);` asserting source and return
      bytes); 5.3 names and precedence (the closed reason list, one reason per lost or gained sink; `unexplained` must reach zero); 5.6 operand kinds and the unsupported families (the five per-root-inexpressible families). The implementer
      updates rows. Both reviewers; cap 2. DESIGN decision 2 is settled (branch A2, owner 2026-11-24); stage D
      does not reopen it.

## Stage I: slices

Order: I-1 first (every later row asserts a reason; gate Explained reads them); I-guard (lane 9 waits for
it); I-walk (repairs the engine's own wrong answers before any slice widens what it crosses); the owner-retained descent
slices by the ordering rule; I-5 after the retained set, so `parity_knobs_agree` runs on the final rules; I-7 after I-5
(it takes `TaintReachesOptions`); I-8; then I-9.0 to I-9.4 in that order (branch A2, DESIGN decision 2). A slice starts only
when the DESIGN sections it cites are approved and the types it names have merged.

**Gates per slice** (DESIGN 7): Matrix and Defaults (all); Benchmark (I-guard); Sound and Benchmark (I-walk
and the descent slices); Catalog (I-8, I-9.4); Recovered and Explained (close-out). ORCHESTRATION 5 items 1 to 4, 8 to 10; two
reviewers. No slice adds or relabels graph edges, so item 6 does not apply; the census rerun replaces it.

### Slice I-1: reasons and the crossing trace (3.5 days)

**Files:** Create `src/cpg/trace_descent.rs`, `src/cpg/trace_descent_tests.rs` (`mod trace_descent;` and
`#[cfg(test)] mod trace_descent_tests;` in `src/cpg.rs`, which also re-exports `LossReason`); Modify
`src/cpg/trace.rs`, `src/reasoning/types.rs`, `src/reasoning/taint_reaches.rs` (`boundary_warnings`),
`src/reasoning/shape.rs` (`reachability_for_node_from_ordered`: the boundary-target test of DESIGN 5.3),
`src/navigation/render.rs`, `tests/algo/taxonomy/taint_parity_test.rs` (`ENABLED` gains `"reasons"`),
the affected `tests/fixtures/taint_parity/rows*.jsonl` shards (including f01–f03), `CLAUDE.md`. Rule: DESIGN 5.3; the crossing trace of
DESIGN 6.

```rust
// src/cpg/trace_descent.rs — serialized snake_case names are fixed by DESIGN 5.3
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "snake_case")] #[non_exhaustive]
pub enum LossReason { TargetNotBinding, NoMatchingSite, AmbiguousSite, NotExact, UnprovenSlot,
    ReturnNotEnteredSite, DepthCap, PolicyCeiling, SelfParam, SelfReturn,
    NoGraphEdge, ShortcutSuppressed, CfgPruned, SanitizerModel, OperandGranularity, Unmodelled,
    ReviewNoReturnFlow, ReviewNoRecoveredDefUse, ReviewCfgFilter, ReviewContextPolicy }   // census and rows only
impl CodePropertyGraph {
    pub(super) fn descend_target(&self, from: NodeIndex, to: NodeIndex, rel: Relation,
        cache: &mut BTreeMap<DescentGateKey, Result<(), LossReason>>) -> Result<(), LossReason>;
}
// src/cpg/trace.rs: BoundaryEdge gains `pub reason: LossReason`; Trace gains
//   Per-root boundaries/cuts retain from, to, relation, certified site/slot and primary reason.
//   I-walk qualifies them by expansion version before projection.
// src/reasoning/types.rs: InterproceduralBoundary { sink: String, reason: LossReason }   (additive)
// src/cpg/trace.rs: under PRISM_TAINT_CROSSING_TRACE=1 the walk prints to stderr one `taint-crossing` line per
//   accepted CallDescent or ReturnFlow (root, from, to, relation, site span where known, grades) and one
//   `taint-refusal` line per refused hop or cut (the same, plus the reason); output otherwise unchanged
```

- [ ] Commit 1, move only: `DescentGateKey`, `descend_target`, `call_site_matches_source_occurrence`,
      `is_parameter_binding_from`, `return_flow_ascent_allowed` to `trace_descent.rs`.
- [ ] Unit tests first (builders `build`, `spell` of `object_literal_arg_tests.rs` are `pub(super)`):
      `use_target_reports_target_not_binding`, `no_site_reports_no_matching_site`,
      `two_sites_report_ambiguous_site`, `nameonly_site_reports_not_exact`, `third_descent_reports_depth_cap`,
      `context_policy_refusal_reports_policy_ceiling`, `self_call_reports_self_param`,
      `self_return_reports_self_return`, `depth_zero_cfg_cut_is_recorded` (row w05's node is recorded with
      `CfgPruned`, verdict unchanged), `suppressed_shortcut_is_recorded` (x01),
      `return_not_entered_site_is_a_cut_not_a_boundary`, `sink_past_refused_hop_is_boundary_exited_with_its_reason`
      (f01, f02: verdict BoundaryExited, reason `depth_cap` / `not_exact`), `boundary_target_test_cap_warns`,
      `crossing_trace_lists_accepted_and_refused_hops` (one line each, fields asserted; nothing printed with the
      switch off); the enum and fields land with a placeholder in the same commit so they compile and fail.
      Implement DESIGN 5.3's ordered checks and sink tie-break. Test the 128th distinct continuation pair and a required 129th for one root/operand, with cold and warm caches. The latter yields `incomplete` and counts as `unexplained`. Trace conditional candidates and joints separately. A same-function warning, exhausted attribution cap or unbound relaxation cannot explain a sink. Exercise twin roots/sites, unrelated/distant sinks and reverse traversal; one primary per direction and sink, unresolved identities fail Explained. Until I-walk lands, exercise wrong-return cuts with constructed inputs. With live paths, a reason is attached to the requested sink through the hop's
      identity; message `taint reached a {kind} boundary at {path} ({reason})`.
- [ ] Enable `"reasons"`: RED, GREEN. `test_self_call_param_stays_boundary` and
      `test_two_same_line_same_name_calls_do_not_descend` pass unchanged. Run `ledger.py` on the apparatus
      fixtures. PR body: reasons, move commit, Tier-A, the rows whose verdict moved to BoundaryExited.

### Slice I-guard: name-only binding edges (1 day)

Under Exact, inspect the binding edge’s own label before site lookup or a proof-cache hit. A `call_nameonly` or `builtin_callback` doubt refuses `not_exact` even when an Exact sibling names the target. A read-side `SameLine` doubt alone does not trip the guard. Retain the existing spelling and single-site requirements. For ordinary byte-addressed occurrences, also require the executing caller and containment in that site, including on a single line. A producer line key can only nominate candidates. Leave uncertified line-keyed widenings refused.

Report the following default answers in the I-guard PR. `n01` and `n02` are NotReached on the base fixtures, which lack the lane-9 edges. With those edges present the guard returns `BoundaryExited / not_exact`. Pin this on constructed edges until the producers land. `s2-js-alias-same-line`, `s2-js-alias-multiline-overlap` and `s3-ts-optional-default-competitor` move from Reached d1 to `BoundaryExited / no_matching_site` until I-spelling. In each, the alias is the only site for the target that contains the tainted actual. Assert these intermediate cells in I-guard. Keep l01’s positive witness at its second `t` and l06’s refusal. The guard needs only the edge label and existing site bytes, so lane 9 can depend on it before I-walk.

**Files:** Modify `src/cpg/trace_descent.rs`, `src/cpg/trace_descent_tests.rs`,
the guard and alias rows in `tests/fixtures/taint_parity/{rows,rows-t2b,rows-t2c}.jsonl`, `tests/algo/taxonomy/taint_parity_test.rs` (`ENABLED` gains
`"guard"`), `CLAUDE.md`.

- [ ] Tests first: `nameonly_callback_edge_not_crossed_through_same_line_exact_site` (n01),
      `byte_carrying_occurrence_matches_by_containment_only`, `log_then_g_still_descends_from_inner_occurrence`
      (l01, source bytes asserted); the same test for `builtin_callback` lands with lane 9's kind (n02).
      Include a cache warmed by an Exact sibling. Enable `"guard"` and assert the listed alias CURRENT cells even though spelling is disabled: RED, implement, GREEN.
- [ ] I-guard cannot merge before the comparable population-B base exists, unless the owner explicitly accepts Benchmark as unmeasured for the named slices. The orchestrator’s 2026-11-25 census receipt records the base. Compare the head per class and read newly traced cases for the gate.
- [ ] Census (default byte-identical, trace losses listed and explained), Benchmark, Tier-A, cloud tests as in
      the descent slices; two reviewers. PR body: moved rows, census, the note on lane 9's issue.

### Slice I-walk: hop evidence and walk state (4.5 days)

**Pre-dispatch gate.** Use I-1’s accepted-crossing ledger with base crossing behaviour on the census’s frozen frontier sources in all six repositories (same revisions, roots, graph inputs and Exact/2). Reconcile against the pinned base’s frontier results. Compare every base-accepted crossing with rule (iv)’s refusal list, retaining caller/site/actual, target/formal, receiver, default and expansion evidence. Retrieve missing facts at the recorded bytes. Also classify rule (iii)’s non-dispatch pools. Record counts, implicated rows, and the recall cost of each deliberate refusal. For unmatched or indeterminate cases, record the missing evidence and settle the rule and row before proceeding. The orchestrator must read this comparison before dispatching I-walk. A first-failure census or a post-implementation diff cannot substitute for it.

Rules: DESIGN 5.2 (i)–(v), Returns and Depth. Bind the shared syntax view to the graph's parsed files. Implement proof/refusal and frame repairs first; leave optional Use-target, colliding-site and renamed-site admissions gated until the owning slice is dispatched. For each seam test positive and refused proofs directly even when public admission is still gated. The full matrix ledger specifies the affected rows. No edge/schema change.

**Files:** Create `src/cpg/trace_frames.rs`, `src/cpg/trace_frames_tests.rs` (`mod` lines in `src/cpg.rs`);
Modify `src/cpg/trace_descent.rs`, `src/cpg/trace_descent_tests.rs`, `src/cpg/trace.rs` (the walk keys its
states through `trace_frames`; weighted size not above main), `src/reasoning/order.rs` (`AstSiteView` beside
`AstOrderView`), and every walk entry and client the view threads through: the entries `taint_trace`,
`taint_trace_with_mode`, `taint_trace_with_mode_policy` (#841), `taint_trace_nodes`,
`taint_trace_nodes_with_mode`, `taint_trace_nodes_excluding`, `taint_trace_nodes_excluding_with_mode`,
`taint_forward_cfg_with_return_flow`, `taint_forward_cfg_with_return_flow_labeled` and its #841 `_policy` form
(`src/cpg/trace.rs`); the clients `src/reasoning/taint_reaches.rs` (frontier, witness mode, the sanitizer bypass
re-walk), `src/reasoning/shape.rs`, `src/reasoning/sanitizer_walk.rs`, review's
`crates/prism-review/src/algorithms/taint/run.rs`, and the test callers (`src/cpg/tests.rs` expectations only,
`src/cpg/multiline_call_arg_tests.rs`, `src/data_flow/build/field_recall*_tests.rs`,
`tests/integration/dfg_label_parity_test.rs`, `tests/reasoning/{smoke,scope_honesty}_test.rs`);
`tests/algo/taxonomy/taint_parity_test.rs` (`ENABLED` gains `"walk"`), the affected `tests/fixtures/taint_parity/rows*.jsonl` shards,
`CLAUDE.md`.

```rust
// Schematic interfaces; all bytes are half-open file offsets.
pub trait SiteView {
    // Facts include value occurrences, syntactic roles, effective arguments/openings,
    // lexical owner/version, formal or paired read, and assignment result component.
    fn facts_for_edge(&self, edge: &EdgeEndpoints, candidate: &CallSite) -> Option<SiteFacts>;
}
pub struct AstSiteView<'a> { files: &'a BTreeMap<String, ParsedFile> }
pub(super) struct Frame { caller: FunctionId, site: FileSpan, callee: FunctionId }
pub(super) struct State { node: NodeIndex, entered: Vec<Frame>, initial_cfg: bool }
// Keep remaining budget, canonical immutable parent per version, and that expansion's cuts.
// The proof cache includes endpoints, relation, label, full site/value spans and slot.
// Existing no-view entry points retain local traversal and refuse every crossing.
```

- [ ] Assert both clients and sanitizer bypass use the same view. Probe missing/partial view (including root-in-callee ascent and recursive Use), mixed-grade and unrelated all-Exact vectors, d01/d02, d03’s bound slot and the two plain instance forms (`self.run(t)`, `r = Runner(); r.run(t)`), s1-py-cls through the class, and w09. Check the actual → formal edge in each instance/classmethod witness. Exercise supplied versus omitted defaults (including JS `undefined`), static/other decorators, explicit unbound receiver, preceding expansion, fixed-prefix/rest controls, per-result slots and caches with equal starts/different ends and different targets. Assert boundary retraction on cheaper revisit, preservation for a different live stack and identical final witnesses/warnings in reversed traversal order. Keep internal frame bytes in the test oracle.

- [ ] Tests first: `return_after_descent_only_through_entered_site` (w01),
      `root_in_callee_returns_to_any_exact_caller`, `deeper_first_route_does_not_block_cheaper_route` (w03),
      `ascent_and_descent_share_the_budget`, `synthetic_return_value_survives_depth_zero_filter` (w05),
      `spread_before_slot_refuses_unproven_slot` (w06), `plain_positional_slot_still_descends` (`run(a0, t)`),
      `exact_multi_target_site_follows_stage_d` (w09), `go_interface_pool_descends` (d01),
      `two_site_witness_returns_through_its_own_site` (DESIGN 5.2's literal fixture and exact interior bytes; also instrument `a1-js-equal-budget-return-sites`),
      `site_cache_key_includes_target_and_slot`, and in `taint_parity_test.rs` `both_clients_pass_ast_site_view`.
      Enable `"walk"`: RED.
- [ ] Implement `prove_site` for `descend_target` and ascent. Assert l07’s version-refuted CURRENT reason at I-walk even if I-same-line is deferred. Run all enabled groups and `test_self_call_param_stays_boundary`.
- [ ] Census, Benchmark, Sound, wall time, Tier-A, cloud tests and PR body as in the descent slices below.

### Slices I-binding, I-same-line, I-spelling (3 days each)

Dispatch only on owner decision 6. For each deferred group retain the target expectations and reasons as data. Materialise and assert CURRENT on the chosen build, for the group and every dependent cell, under the STAGE_D_ROWS rule. This includes f03’s `target_not_binding` until I-binding (then `not_exact`) and the alias rows’ `no_matching_site` until I-spelling. If retained, one slice per change of DESIGN 5.2: they edit one function, but each has its own group, census class and
sample, and merging them would make the Sound sample unattributable. Each needs I-walk's `prove_site`.

**Files:** Modify `src/cpg/trace_descent.rs`, `src/cpg/trace_descent_tests.rs`,
`tests/algo/taxonomy/taint_parity_test.rs` (`ENABLED`),
the affected `tests/fixtures/taint_parity/rows*.jsonl` shards (STAGE_D_ROWS), `CLAUDE.md`; I-same-line also `src/cpg/tests.rs`
(split the graph-only refusal from the view-equipped first-site witness on l05; no test is
added to that file, which is over the size ceiling);
I-binding covers computed/written/nested/recursive reads and unique versus ambiguous line-key reconstruction, including nonzero token and zero-width anchors. I-spelling also `docs/features/language-coverage/js-ts-known-gaps.md` (section 3, taint descent entry).

| Slice | Group | Rule | Unit tests first |
|---|---|---|---|
| I-binding | `binding` | Reuse the actual producer pairing and check the formal's one-hop read, AST read kind and lexical owner. No second key-path implementation or lane-3 edit. Reject recursive Use as `self_param`. | `objlit_edge_into_formal_member_read_descends`, `objlit_edge_into_bare_formal_read_descends`, `objlit_edge_into_literal_subscript_read_refuses` (b06), `objlit_edge_into_disjoint_dotted_key_absent` (b07), `use_in_callable_nested_in_callee_refuses` (b05), `two_indistinguishable_leaf_emissions_refuse`, `recursive_use_target_refuses_self_param` (b04), `mixed_formal_operand_aggregates_its_nodes` (m01) |
| I-same-line | `same_line` | DESIGN 5.2 "Two call sites" | `two_sites_one_line_choose_containing_site`, `second_occurrence_inside_g_descends` (l01), `python_first_repeated_argument_has_its_own_site` (l05), `nested_same_callee_refuses_ambiguous`, `nameonly_edge_not_crossed_through_other_exact_site` (l06), `same_line_rebinding_refuses` (l07), `entered_site_return_discarded` (w02) |
| I-spelling | `spelling` | DESIGN 5.2 "Spelling" | `aliased_import_site_descends`, `const_alias_site_descends`, `aliased_site_descends_only_into_resolved_target`, `aliased_spread_before_slot_refuses` (w07) |

- [ ] Enable the group: RED. Implement. Assert each active cell, including f03’s post-binding reason and the alias rows’ post-spelling witnesses. Run every enabled group.
- [ ] Write the prediction; rerun `run_census.py` on A with the head build: default byte-identical; trace
      gains listed by class; any trace loss is a WRONG to explain or fix; `ledger.py` against the parent build,
      whole-repository frontier runs and review runs.
- [ ] Benchmark: harness run with the head build; traced count per class not below base; every case whose
      outcome becomes traced is read against its ground truth (a false witness is a WRONG).
- [ ] Wall time: `taint-reaches` frontier mode over the frozen sources of population A's largest repository,
      head against parent; over 2× goes to the controller before merge.
- [ ] Sound: a reader (Opus-5.5, high, blind to slice, shown the new hop and its code) labels a random
      min(100, all) of the slice's new crossing paths from the whole-repository ledger (and the review ledger
      beside it), each "new hop real" and "whole path real"; DESIGN 7 Sound; adjudications kept.
- [ ] Tier-A; cloud tests at 1 and 4 threads (spec: `--test algo_taxonomy taint_parity_test::`,
      `-p prism --lib trace_descent_tests`, `-p prism --lib trace_frames_tests`, `--test reasoning
      taint_reaches_test::`); two reviewers.
- [ ] PR body: prediction and result, census against parent and baseline, sample tally (both labels),
      benchmark cases read, wall time, Tier-A, moved rows.
      I-spelling comments on #376 (item 1). I-binding comments on #596 that F5 is closed only if row b01 runs
      un-ignored on lane 3's producer; otherwise it says what is still pending.

### Slice I-5: knobs on the walk both clients call (1.5 days)

PR #841's review options govern context files (import depth, the label of an edge entering a context file);
the default review walk has no descent. So this slice adds descent knobs to the trace engine's core entry
points, exposed on `taint-reaches` and the MCP tool; trace-mode review keeps its CLI and passes the default,
and tests drive it with other knobs through the core API. Rule: DESIGN 5.4 (the ceiling governs the resolution
grade on an existing proved edge; nameonly permits the site's singleton name-only target only. Step 5c supplies no name-only return edge. Budget counts ascent and descent; review finding columns always use default knobs).

**Files:** Modify `src/cpg/trace_descent.rs`, `src/cpg/trace.rs` (thread `descent` beside `policy`;
`MAX_TAINT_DESCENT_DEPTH` becomes the default), `src/cpg.rs`, `src/reasoning/taint_reaches.rs`,
`crates/prism-review/src/{cli.rs, cli_nav.rs, main.rs}`, create `crates/prism-review/src/nav_taint.rs`,
`crates/prism-mcp/src/{input.rs, tools_reasoning.rs}` (schema; description now covers knobs and reasons),
goldens, `tests/algo/taxonomy/taint_parity_test.rs`, `docs/MCP.md`, `CLAUDE.md`.

```rust
// src/cpg/trace_descent.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaintDescentPolicy { pub ceiling: CrossingCeiling, pub max_depth: usize }   // Default: Exact, 2
// src/cpg/trace.rs (existing entry points delegate with the default)
pub fn taint_trace_nodes_with_descent(&self, roots: &[NodeIndex], order: Option<&dyn SameLineOrderView>,
    mode: ReturnFlowMode, view: Option<&dyn SiteView>, descent: TaintDescentPolicy) -> Trace;
pub fn taint_trace_nodes_excluding_with_descent(&self, roots: &[NodeIndex], order: Option<&dyn SameLineOrderView>,
    excluded: &BTreeSet<(NodeIndex, NodeIndex)>, mode: ReturnFlowMode, view: Option<&dyn SiteView>, descent: TaintDescentPolicy) -> Trace;
pub fn taint_forward_cfg_with_return_flow_labeled_with_descent(&self, sources: &[(String, usize)],
    view: Option<&dyn SiteView>, descent: TaintDescentPolicy) -> (Vec<FlowPath>, BTreeMap<(VarLocation, VarLocation), EvidencePath>);   // trace-mode review's walk
// src/reasoning/taint_reaches.rs (taint_reaches delegates with the default)
#[derive(Clone, Default)] #[non_exhaustive] pub struct TaintReachesOptions { pub descent: TaintDescentPolicy }
pub fn taint_reaches_with(session: &NavigationSession, sources: &[SeedSpec], sinks: Option<&[SeedSpec]>,
    options: &TaintReachesOptions) -> Result<Evidence, QueryError>;
```

The syntax view parameter of I-walk is threaded through each new entry as through the old ones. CLI: `nav
taint-reaches --edges exact|nameonly` (default `exact`) `--depth <0..=3>` (default 2). MCP: optional `edges`
and `depth` (0 to 3). The sanitizer bypass re-walk in `witness_mode` uses the same policy.

- [ ] Tests first: `parity_knobs_agree` (each enabled `walk_comparable` row under `(exact,0)`, `(exact,2)`,
      `(nameonly,2)`, `(exact,3)` and its own `knobs`: the trace-mode review walk
      (add a node-rooted, preprojection adapter to the same walk; a line-seeded finding API cannot select an occurrence) from the row's source occurrence and
      `taint_reaches_with` give the same verdict, source and terminal at the sink operand; the finding level is
      checked against the row's own expectation, not inferred); enable `knobs`; CLI parse tests in `cli.rs`
      (values, range); MCP input tests (accepted, out of range, unknown value). RED, implement, GREEN.
- [ ] Defaults gate; goldens re-recorded with the reason; Tier-A; cloud tests; two reviewers.
- [ ] PR body: `not_exact` and `depth` census sizes, for owner decisions 3 and 5.

The trace-mode grade slice is dropped (DESIGN 5.5, 9).

### Slice I-7: catalog trait and catalog mode in core (3.5 days)

Needs I-5 merged (`TaintReachesOptions`). Rule: DESIGN 5.6.

**Files:** Create `src/reasoning/sink_catalog.rs` (in `src/reasoning/mod.rs`), `tests/reasoning/sink_catalog_test.rs`
(in `tests/reasoning/main.rs`; a fixed in-test catalog); Modify `src/reasoning/types.rs`,
`src/reasoning/taint_reaches.rs` (`witness_mode` becomes `pub(crate)` and takes an optional sink category),
`CLAUDE.md`.

Implement DESIGN 5.6's root-tagged batch trait and result. Carry the root, projected line, occurrence span/kind, rule, optional CWE/category, strength, tagged operand, review cleansing and coverage diagnostics. Share the complete-fan builder with trace review. For line-only decisions aggregate reached line targets; for selected subtrees evaluate that root's intersecting nodes at the operand's bytes, including multiline cases. Preserve matcher decisions with NotReached or BoundaryExited. Keep a missing operand witness absent. Apply the matching-category sanitizer/bypass proof from the same root and policy. Group per-source verdicts only after computing them and keep projected lines distinct.

- [ ] Test with a fixed catalog. Cover one batch per query, exact fan equality including Def-only lines, weak hits, categoryless/advisory and matching-category hits, raw bypass and uncertain bypass, body-only cleansing, independent fans on the same line and an unrelated unselected root. Skip grammar trivia when indexing recognized sanitizer arguments (a1-js-sanitizer-comment-slot, shared with the line walk). For a paired check require the selected operand, dominating success branch and no raw bypass in that fan. Require c04's decision with no operand witness, c09's line-only verdict, c12's two projected rows, same-rule multiple calls, all five unsupported diagnostics and a query with no decisions. Reject a purported supported decision outside its fan's admitted rows. CLI/MCP line-query output stays compatible. Two reviewers.



### Slice I-8: review implementation and CLI catalog mode (5 days)

Rule: DESIGN 5.6, families 1–22, 26 and 28. Return per-root coverage diagnostics for 23, 24, 25, 27 and 29. Test each route with its matched control, including provenance when a supported route can hit the same line. Reuse actual framework discovery and seed membership for the route-handler class in §9. Do not seed the explicit catalog walk from discoveries.

**Files:** Create `crates/prism-review/src/algorithms/taint/catalog.rs` (`pub mod catalog;` in
`algorithms/taint.rs`; inside `taint` it reaches the `pub(super)` matchers); Modify `algorithms/taint/run.rs`
(extract, behaviour unchanged, the loop over paths into `sink_decisions(cpg, files, facts: &SinkRunFacts,
rooted_fans) -> CatalogDecisions` (root, projected line, occurrence span/kind, inventory family, rule, CWE, category, strength, tagged operand, cleansing and coverage), which `slice` then folds into `sink_lines` as today; `SinkRunFacts` holds the framework
source set and ranges, wrapper and extra sinks), `algorithms/taint/seeds.rs` (`detect_framework_sources` takes
the graph and files instead of `&CpgContext`, which it reads only for those), `algorithms/taint/sink_match.rs`
and the per-language outcome files only if the decision needs the call span or operand they compute,
`crates/prism-review/src/api/` (re-export),
`crates/prism-review/src/{cli.rs, nav_taint.rs}`, `tests/algo/taxonomy/taint_parity_test.rs`, `CLAUDE.md`;
Create `crates/prism-review/tests/cli/nav_taint_catalog_test.rs` (in that directory's `main.rs`).

```rust
pub struct ReviewSinkCatalog;   // Sync: per-call state only
impl prism::reasoning::sink_catalog::SinkCatalog for ReviewSinkCatalog { /* SinkRunFacts, then sink_decisions */ }
```

Compute run facts once and instantiate one classifier per call. Forward the matcher's actual decision, with family-qualified IDs for configured/discovered names pending registry IDs. Test whole argument, JSX attribute, template keyword value, flat identifier/assignment/row and continued call projections. Preserve range masks, flat/structured precedence and review's weak/cleansed behavior. Exercise legacy and discovered wrapper families in native fixtures and configured-only via API. Use `nav taint-reaches --source … --sinks catalog`, conflicting with `--sink`.

- [ ] Step 1: the extraction alone; `cargo test --test algo_taxonomy taint_sink` and `cargo test --test cli`
      unchanged, Defaults gate on population A byte-identical. Commit.
- [ ] Tests first: `parity_catalog` (per enabled row: the reported multiset of (file, line, rule, operand,
      verdict) equals `expect.catalog`; every (file, line) of the review column's `taint_sink` findings is
      accounted for at matched roots/options, or the row identifies a different root or a specific unsupported emitter); enable `catalog`; CLI tests: `--sinks catalog` on c02
      prints the rule; on c08 it reports the keyword operand; on c12 both lines; with `--sink` it is rejected.
- [ ] Real-code check (DESIGN 7 Catalog): an ignored test `catalog_matches_trace_review_on_repo` in
      `nav_taint_catalog_test.rs` (repository and sources file from `PRISM_CATALOG_CHECK_REPO` and
      `PRISM_CATALOG_CHECK_SOURCES`) loads the repository once, runs catalog mode and trace-mode review through
      the API (`taint_from_diff: false`, the same whole-repository files) and compares the (file, line) set of
      decisions not `review_cleansed` with the `taint_sink` lines; the census seat runs it per repository of
      population A at its census commit with its frozen sources. Compare eligible projections before clipping at the same floor, on identical rooted fans. Attribute both directions, subtract only lines exclusively supplied by the five unsupported routes. A shared line with a missing supported decision fails. A missing occurrence on an already matching line is caught by the matrix. Result in the PR body.
- [ ] Catalog-mode wall time and file-scan counts against frontier mode on A's largest JS repository and one
      Python and one Go repository, in the PR body (over 3× is a follow-up). Two reviewers.

### Slices I-9.0 to I-9.4: the catalog in MCP through a crate below both binaries (DESIGN decision 2, 7.5 days)

The owner chose branch A2 on 2026-11-24 (DESIGN section 4 and decision 2): a new crate `crates/prism-taint`
(package `prism-taint`, library `prism_taint`) holds the taint module's closure and the catalog
implementation; `prism-review` and `prism-mcp` depend on it; core is unchanged. The slices run in order after
I-8 merges, one PR each. Move the root identity, projected lines, tagged operand handling and diagnostics with the matcher; the core contract stays below the adapters. The closure cannot be moved in parts: its modules form one cycle of references
(verified at 57f7b8cf: `taint_sinks/` names `matches_sink`, `line_matches_structured_sink`,
`SinkMatchOutcome` and the Go sink tables through `taint.rs`; `taint/js_ts/` names `taint_sinks`; and
`cleansing.rs`, `sink_match.rs`, `seeds.rs` and the `js_ts/`, `python/` and `go/` directories name each
other), and a crate below review cannot name review, so any subset left behind breaks the build. So the
preparation that needs judgement (I-9.1) lands in the review crate first, in small reviewable commits, and the
move itself (I-9.3) is one generated PR that changes only `use` paths, checked byte for byte. No moved file
gains a line, and every new file stays under 600 weighted lines, so no file approaches the 2,000 ceiling.

**Stop rule (every I-9 slice).** Stop and ask the owner when (a) a closure module names an item of the review
crate outside `algorithms/taint*` and `taint_sinks/` (none today; lane 7's registry lives in core, `prism::rule_registry`, and review only re-exports it, so the closure imports it from `prism` and `crates/prism-review/src/output/` is not named; `SinkHit.rule`/`cwe` are private with accessors, which I-8's catalog uses); (b) the untangling needs an edit other than moving an item, rewriting
visibility (`pub(super)`, `pub(in crate::algorithms::taint)`, `pub(crate)`), rewriting a `use` path, or I-8's
signature change of `detect_framework_sources`; (c) the closure exceeds 20,000 raw lines or 55 files; (d)
another lane has an open PR on a closure file (ORCHESTRATION's shared table: lanes 4, 6 and 7 finish their
taint slices first or rebase after; the move is announced on their issues as a move-only window that lasts
from I-9.1's dispatch to I-9.3's merge).

#### Slice I-9.0: closure measurement (0.5 day)

The numbers below were measured before PR #841 merged. On `main` at 9d1a7d06 the taint module is about twice
as many files, `taint/seeds.rs` names `source_scope::admission`, and `source_scope/admission.rs` calls
`run::slice_attributed_impl`, so the closure may now reach `run.rs`, which this plan keeps in the review crate.
Include I-8's native legacy/wrapper/configuration routes and the source admission and suppression helpers it actually calls. I-9.0 re-measures on its own head and decides whether that module joins the closure and whether `run`'s entry
points move or the call is inverted; if the closure no longer fits the 7.5-day estimate, the stop rules below
apply and the orchestrator reports to the controller.

- [ ] `census/closure.py` (kept in the evidence directory) computes the module closure mechanically from the
      catalog implementation and the sink loop (`use` trees, qualified and child-module paths, re-exports; a
      module's private children move with it, since `impl` blocks need no import) on the head I-9.1 will start
      from (I-8 and any taint slice of lanes 4, 6 and 7 merged), and records files, raw lines, external crates
      and every item imported from a module outside the closure. Measured 2026-11-21 at 9ba4249d (before
      I-8): `taint_sinks/` 23 files, 7,322 lines; from `taint/sink_match.rs` 25 files, 9,919 lines
      (`cleansing.rs`, all of `js_ts/`, `python/`, `go/`); 12 items of `taint.rs` (`TaintSeed`, `JsTsAliasDef`,
      `JsTsAliasDefs`, `JsTsAliasKill`, `JsTsRequestDataSource`, `JsTsParam`, `UrlSanitizerBinding`,
      `PathSanitizerBinding`, `SafeFormatHtmlBinding`, `JsTsPathSanitizerBinding`,
      `TargetSeedSynthesisContext`, `matches_sink`), `seeds.rs::reference_line_cfg_reachable`; with I-8's sink
      loop also `seeds.rs::detect_framework_sources` and `flat.rs::detect_format_string_wrappers`: about 18,300
      of 19,146 lines, 51 of 53 files. A reading of the code at 57f7b8cf adds three `taint.rs` items that
      `taint/python/sources.rs` names (`PythonDjangoRequestDataSources`, `PythonFlaskRequestDataSources`,
      `PythonParam`), so the item list is taken from the script, not from this paragraph. The only external
      crates the closure names at 57f7b8cf are `prism` and `tree_sitter` (`anyhow` only in `run.rs`, which
      stays). Left in review: `run.rs` (orchestration), `report.rs` (the one file naming `crate::output`),
      `flat.rs`'s IPC detection, and the `taint.rs` items the closure does not name (`TaintConfig`, `slice`'s
      re-export, its inline tests).
- [ ] Apply the stop rule; record the closure list (files, items, the rewrite counts I-9.3 will expect) in
      `LEDGER.md` and in I-9.1's brief.

#### Slice I-9.1: untangle the closure inside the review crate (1.5 days)

Freeze the I-8 decision multiset and unsupported-family diagnostics before moving helpers. Review-only, behaviour unchanged. Every edit is one the stop rule allows. After it, the closure is a set of
whole files in which every item another module uses is `pub`, so the move needs no edit but its paths.

**Files:** Create `crates/prism-review/src/algorithms/taint/model.rs` (the `taint.rs` items the closure names,
moved verbatim), `taint/format_wrappers.rs` (`detect_format_string_wrappers` and the helpers only it uses, out
of `flat.rs`), `taint/sink_loop.rs` (I-8's `sink_decisions` and `SinkRunFacts`, out of `run.rs`); Modify
`algorithms/taint.rs` (`mod` lines for the three; `pub(crate) use model::{…}` so every `crate::algorithms::taint::X`
path still resolves), `taint/flat.rs`, `taint/run.rs`, `taint/catalog.rs` (its `use` of the sink loop), and the
visibility of every closure file.

- [ ] Commit 1: move the closure items of `taint.rs` into `model.rs`, with the re-export. Commit 2: move the
      format-wrapper detection into `format_wrappers.rs`. Commit 3: move the sink loop into `sink_loop.rs`.
      Each commit compiles; moved lines byte-identical except their own `use` lines, which the PR body lists.
- [ ] Commit 4: in every closure file, `pub(super)`, `pub(in crate::algorithms::taint)` and `pub(crate)` become
      `pub`, and every child `mod` line of a closure module becomes `pub mod` (a crate boundary needs both).
      `grep -rn "pub(" <closure files>` returns nothing afterwards; the PR body gives the counts (57f7b8cf:
      257 `pub(super)`, 46 `pub(in crate::algorithms::taint)`, 12 `pub(crate)`).
- [ ] `cargo test --test algo_taxonomy taint`, `cargo test -p prism-review --lib algorithms::taint` (the
      inline `taint_sinks/` tests included) and `cargo test --test cli` unchanged; Defaults gate on
      population A byte-identical; `python3 scripts/file_size.py` on every touched file (the three new files
      under 600; `taint.rs`, `flat.rs` and `run.rs` shrink). Two reviewers: the review is of the item lists and
      the visibility diff, not of behaviour.

#### Slice I-9.2: the crate and the layering test (0.5 day)

**Files:** Create `crates/prism-taint/Cargo.toml` (`name = "prism-taint"`, `publish = false`, workspace
version, edition and license; dependencies `prism = { path = "../.." }` and `tree-sitter = "0.25"`, plus any
crate I-9.0 lists), `crates/prism-taint/src/lib.rs` (module doc only: what the crate holds and that it never
names `prism_review`); Modify the root `Cargo.toml` (`members` and `default-members` gain
`crates/prism-taint`, so a root `cargo test` runs the moved unit tests), `crates/prism-review/Cargo.toml` and
`crates/prism-mcp/Cargo.toml` (`prism-taint = { path = "../prism-taint" }`), `tests/integration/layering_test.rs`.

Keep the catalog contracts in core and the inventory-specific implementation in prism-taint. Layering test: a third scan, over `crates/prism-taint/src/`, reports any non-comment line naming
`prism_review`, `crate::algorithms`, `algorithms::` or `prism::algorithms` (test files get no exemption, as for
core); the core scan of `src/` also reports `prism_taint`; the MCP patterns (`MCP_REVIEW_PATTERNS`) are
unchanged, and the moved code's paths (`prism_taint::taint::…`, `prism_taint::taint_sinks::…`) contain none of
them, so the MCP crate may name the new crate without an exception. The module doc says so.

- [ ] Tests first, beside the existing ones: `the_taint_crate_naming_the_review_crate_is_reported`,
      `the_taint_crate_naming_review_algorithms_is_reported`, `core_naming_the_taint_crate_is_reported`,
      `the_mcp_crate_may_name_the_taint_crate`, and the walk over `crates/prism-taint/src` (non-empty once
      `lib.rs` exists). RED, then GREEN.
- [ ] `cargo build --workspace`; `cargo test --test integration layering_test::`. Reviewers as ORCHESTRATION
      section 2 assigns. No product behaviour changes.

#### Slice I-9.3: the generated move (3 days)

One move-only PR, generated and checked by a script. Compare the extracted I-8 CLI decisions (including projected lines, per-root verdicts and unsupported diagnostics) before and after the move.

**Files:** Create `scripts/move/taint_crate.py` (the generator and its check, with a test file beside it that
runs under `python3 -m unittest`), `crates/prism-taint/src/taint.rs` (glue: `pub mod` lines for the moved
children of `taint/`, and the re-exports the moved code names through `crate::taint::…`: the `model` items,
`line_matches_structured_sink`, `SinkMatchOutcome` and the sink tables that review's `taint.rs` re-exported);
Move every closure file from `crates/prism-review/src/algorithms/taint/…` to `crates/prism-taint/src/taint/…`
and from `algorithms/taint_sinks/…` to `crates/prism-taint/src/taint_sinks/…`, keeping the directory shape so
every `super::` path holds; Modify `crates/prism-taint/src/lib.rs` (`pub mod taint; pub mod taint_sinks;`),
`crates/prism-review/src/algorithms/taint.rs` (the `mod` lines of moved modules become
`pub(crate) use prism_taint::taint::{…};`, so `run.rs`, `report.rs`, `flat.rs` and the tests keep their
`crate::algorithms::taint::…` paths), `crates/prism-review/src/algorithms/mod.rs` (`pub(crate) mod taint_sinks;`
becomes `pub(crate) use prism_taint::taint_sinks;`), `CLAUDE.md`.

- [ ] The check, run by the script (its own unit tests first, on a toy tree with one planted extra byte that
      must fail): for every moved file, the new bytes
      equal the old bytes after the rewrite map `crate::algorithms::taint_sinks` → `crate::taint_sinks`, then
      `crate::algorithms::taint` → `crate::taint`, and nothing else; the number of rewrites per file equals
      I-9.0's expected counts (the whole taint module before PR #841 had 161 `crate::algorithms::taint` and 2
      `taint_sinks` occurrences; the counts for the moved files come from I-9.0); no closure file is left under
      `crates/prism-review/src/algorithms/` and no other file moves; the glue files are listed and are the only
      new bytes. The script's report (files, rewrites, glue) goes in the PR body. `git diff -M` shows every
      moved file as a rename.
- [ ] `cargo build --workspace`; `cargo test --test algo_taxonomy taint`, `cargo test --test cli`,
      `cargo test -p prism-taint` (the moved unit tests, the same number that `cargo test -p prism-review --lib
      algorithms::taint` ran before the move) and `cargo test --test integration layering_test::`, all unchanged; Defaults gate on population A
      byte-identical; Tier-A does not apply (nothing under `src/` changes).
- [ ] `CLAUDE.md`: the workspace becomes four crates (the code-organisation and "Layering and file layout"
      sections, the dependency direction `prism` ← `prism-taint` ← `prism-review`, `prism-mcp`), the layering
      test's third scan, and every path in the `taint_sinks/` paragraph.
- [ ] Footprint: the `macos-footprint` idle guard (`cargo test --release -p prism-mcp --test mcp
      footprint_guard -- --ignored --nocapture --test-threads=1`, `docs/MCP.md`); release binary sizes of
      `prism` and `prism-mcp` against main, in the PR body. Two reviewers: the review covers the generator, the rewrite map, the
      glue and the check's report, not the moved lines.

#### Slice I-9.4: the catalog in the MCP tool (2 days)

- [ ] MCP input `sink_catalog: true` (rejected with `sinks`); `crates/prism-mcp/src/tools_reasoning.rs` calls
      `taint_reaches_catalog` with `prism_taint`'s implementation (the type I-8 named `ReviewSinkCatalog`, now
      exported from `prism_taint`; review re-exports it from `prism_review::api` under the same name);
      `max_results` clips `per_sink` and reports the omitted count; the tool description covers catalog mode;
      goldens re-recorded with the reason in the commit; tests `taint_reaches_catalog_mode_reports_rules`
      over c01, c02, c04, c08, c12, independent-root and unsupported-emitter fixtures (assert the same root, occurrence/operand bytes, projected lines, all verdicts and coverage diagnostics as the CLI, even for an empty hit list) and an input test (accepted, combined with `sinks`, wrong type). Catalog gate
      (DESIGN 7) on the MCP path for the matrix rows. Footprint guard and MCP binary size against main in the
      PR body. `docs/MCP.md` (the `taint_reaches` row). Two reviewers.

**Alternatives not run** (DESIGN section 4, recorded): branch A, the same closure into a core module
`src/taint_rules/` with review keeping re-exports (9 days; the owner's earlier choice, made before the closure
was measured); branch B, `prism-mcp` depends on `prism-review` and only a new `crates/prism-mcp/src/catalog.rs`
names `prism_review::api::ReviewSinkCatalog` under a narrow layering exception (2.5 days); branch C, catalog
mode stays CLI-only and the MCP description says so (0.5 day).

## Close-out (2 days)

- [ ] Census rerun on A with the census apparatus (same frozen roots, manifests and options), final head:
      Explained (every lost default sink and every gained trace-mode finding has a reason from DESIGN 5.3;
      `unexplained` zero); Recovered (share by class). Benchmark on B.
- [ ] Docs: `CLAUDE.md` (one paragraph on crossings, reasons, knobs, catalog mode, beside the reasoning-layer
      line); `docs/MCP.md` (`taint_reaches` row); `skills/prism-code-navigation/SKILL.md` (catalog mode,
      reasons). Comments on #602 (census table, what remains by design), #596 (F5, as I-binding states it),
      and lane 9's spread issue (row w06's trace reproduction).
- [ ] File issues for wrong results the lane does not fix, each with its review probe as reproduction and its
      row: the object-literal leaf's Exact edge into a computed read of a disjoint literal key (b06,
      `literal_index`); review's flat sink with a fixed operand on a line reached only at a parameter Def
      (c09, `flat_def_line`); review's finding through the same-line shortcut past a certified constant
      return (x01, `constant_return_flat`); the pre-existing defects of DESIGN 9 found by the r2 probes (rows
      m01, m02, p01; `b1p`). Follow-ups: name-only return edges (graph change, DESIGN 5.4); the dropped work of
      DESIGN 9, each with its decision; lane 7 registry ids if not done; catalog
      performance if flagged.
- [ ] Refresh `HANDOFF.md` and `LEDGER.md`; offload raw census output; delete `target/`.

## Acceptance for the lane

Every gate of DESIGN 7 holds on the merged head (numbers in the close-out report); both reviewers approved
every shipped product slice; census and matrix are on the tracking issue. Matrix asserts the active cells at every level, including CURRENT for deferred targets. Report the empty default Sound sample on descent slices as unmeasured. Apply the Recovered threshold actually recorded by the owner under decision 6 (proposed floor 53/56 plus all named recoveries), and report Benchmark against the comparable B base or cite the owner’s explicit unmeasured exception. A failing gate stops the lane for the owner. Only the owner may change a threshold.

## Estimate

| Stage or slice | Days |
|---|---|
| M, T and E, D | 4, 4, 2 |
| I-1 reasons and crossing trace; I-guard; I-walk | 3.5; 1; 4.5 |
| I-binding, I-same-line, I-spelling | 3 each (9) |
| I-5 knobs | 1.5 |
| I-7 catalog core; I-8 review catalog and CLI | 3.5; 5 |
| I-9.0 closure; I-9.1 untangle; I-9.2 crate and layering; I-9.3 move; I-9.4 MCP (branch A2) | 0.5; 1.5; 0.5; 3; 2 (7.5) |
| Close-out | 2 |
| **Total** | **47.5** (40 before the MCP slices, plus 7.5) |

The alternatives not run would have cost 9 days (A), 2.5 (B) or 0.5 (C) for the MCP slices, totals 49, 42.5
and 40.5.
