# Lane 8: one taint answer from both engines — implementation plan

> **For agentic workers:** this plan is run by an orchestrator under `../ORCHESTRATION.md`. Each task is
> dispatched to an implementer (Sonnet-5.5, high) with its section as the brief. Steps use checkbox syntax.

**Goal:** every difference between review taint and the trace engine behind `taint_reaches` disappears or
carries a named, tested reason; the trace engine crosses the hop classes of DESIGN 5.2 on evidence that
belongs to the hop and stops returning into callers it did not enter; both clients of the trace walker take
the same crossing knobs; `taint_reaches` gains a sink catalog mode, first in the CLI, then in MCP.

**Architecture:** the descent gate moves from `src/cpg/trace.rs` to `src/cpg/trace_descent.rs`, proves the
emitting site of each hop and returns a reason instead of a boolean; the reason rides on `BoundaryEdge` into
the reasoning warnings. The walk state (entered sites, remaining budget) lives in `src/cpg/trace_frames.rs`.
A descent policy (ceiling, depth) is threaded beside PR #841's crossing policy. A `SinkCatalog` trait in core
is implemented in the review crate over review's own sink decisions, and passed in.

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
  the slice's group says so.
- **Known wrong rows** (DESIGN 9): a row with `known_wrong` set keeps today's answer until the slice named
  there; no slice may move one to a further wrong answer.
- **File size** (weighted): `src/cpg/trace.rs` is 1,113.5 on main plus #841's lines and must not grow (new
  logic in `trace_descent.rs` and `trace_frames.rs`). `crates/prism-review/src/main.rs` is 990.5: I-5 moves
  the `nav taint-reaches` handler to a new `crates/prism-review/src/nav_taint.rs` (`mod nav_taint;` in
  `main.rs`). `src/reasoning/taint_reaches.rs` is 583: catalog mode goes in `src/reasoning/sink_catalog.rs`.
- **MCP goldens** (`tests/fixtures/mcp/golden/*.jsonl`) hold `taint_reaches`'s schema and description. Only
  I-5 and I-9 re-record them: `PRISM_MCP_BLESS_GOLDEN=1 cargo test -p prism-mcp --test mcp
  golden_transcript_test`, with the reason in the commit message.
- **Tier-A** (ORCHESTRATION 5 item 5) for every slice touching `src/cpg/` (I-1 to I-5; I-6 if it does). No
  slice changes stored CPG or nav data: hop evidence is reconstructed at query time. If stage D finds that
  site identity must be stored on edges, that is a graph change (`CACHE_VERSION` bump, edge differential of
  ORCHESTRATION 5 item 6) and the slice is re-planned at stage D. Core never names `prism_review`;
  engine-parity assertions live in `tests/algo/taxonomy/`.
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
   `w07`); an aliased call reaches only its resolved target (`s05`). I-walk, I-spelling.
5. Equal knobs give the same walk on `walk_comparable` rows under `(exact,2)`, `(nameonly,2)`, `(exact,3)`;
   the finding and catalog levels hold their own expectations. I-5, `parity_knobs_agree`.
6. Catalog Sanitized only for a sanitizer of the sink's category (`c07`); the Python template keyword and the
   Def-only flat line are reported as review reports them (`c08`, `c09`). I-7, I-8, `parity_catalog`.

## Stage M: issue and parity census (4 days)

### Task M-1: workspace, issue, binaries

- [ ] Worktrees and evidence directory (ORCHESTRATION 4). `memguard.sh 8 cargo build --release`; copy to
      `~/prism-evidence/taint-parity/bin/prism-base`; record `--version`.
- [ ] File the tracking issue "Lane 8: one taint answer from both engines (trace parity, sink catalog)",
      linking `DESIGN.md`, this plan, #602 (parent), #596 (lane 3's F5 moves here) and #376 item 1.
- [ ] Census binary (measurement apparatus, never pushed; local branch `eval/taint-parity-census`): patch
      `src/cpg/trace.rs` so `PRISM_CENSUS_DESCENT_LOG=<file>` appends a JSON line per refused crossing and
      per cut that records no boundary (depth-0 CFG filter, suppressed shortcut, sanitizer exclusion, a
      visited state reached again with more budget): root; `from`, `to` as file, function, line, path,
      bytes, access; relation; depth and remaining budget; the entered-site stack; first failing condition;
      the callee's sites matching by line and by containment, with slot and grades; whether a caller site
      resolves to the target under another spelling. `PRISM_CENSUS_RELAX=<class>[,<class>]` relaxes one or
      more conditions, each applying the tentative proof of DESIGN 5.2 (hop evidence, key path, slot):
      `binding`, `same_line`, `spelling`, `not_exact` (descent; ascent exists only where an edge does),
      `depth` (64). Save as `census/census-relax.patch`; build `bin/prism-census`.
- [ ] Apparatus check before any population run: on synthetic fixtures (two roots and two sites reaching one
      sink line, one through a refused hop; a sink needing two relaxations jointly; a new false crossing to
      an already reported line) the census reports the right cell for each. Record in `CENSUS.md`.

### Task M-2: populations and runs

**Files (evidence directory):** `census/select_population.py`, `population.jsonl`, `run_census.py`,
`classify.py`, `ledger.py`, `CENSUS.md`.

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
- [ ] `ledger.py`: from the head build's trace with `PRISM_CENSUS_DESCENT_LOG`, every accepted crossing as a
      path (root occurrence, site span and slot, target, sink operand) with its grades; a slice's new
      crossings are the ledger difference against the parent build, whether or not the sink line is new.
- [ ] Population B: in the bench checkout, `python3 -m eval.secbench --no-build --binary bin/prism-base
      --binary-sha <sha256> --sut-commit <commit> --out census/secbench-base` (the pin options come from lane
      4 Task M-2; if unmerged, do that task here as written there). Record traced counts per vulnerability
      class: the Benchmark base. The harness's `taint.json.gz` is trace-mode review (`--taint-return-flow`),
      so B supplies no default-review comparison and no classes.
- [ ] `CENSUS.md`: default sinks, trace survivors, lost and gained; lost by class with three examples each;
      grades of default findings that survive in trace mode (input to stage D's grade).

**Ordering rule.** I-binding, I-same-line and I-spelling run in descending order of lost findings in their
class on A (joint classes count for each member); ties keep DESIGN 5.2's order. A class with zero members
becomes an owner question (skip, or keep for the matrix); other slices continue. If no class has a member,
DESIGN decision 6 is asked before any descent slice. `not_exact` and `depth` sizes go to the owner with I-5
(decisions 3 and 5).

## Stage T and E: the parity matrix (4 days)

### Task T-1: rows, fixtures, data-driven tests

**Files:** Create `tests/fixtures/taint_parity/rows.jsonl`, `tests/fixtures/taint_parity/README.md`, one
directory per fixture under `tests/fixtures/taint_parity/` (source and sink lines marked `@SRC` and `@SINK`
in comments, as `~/prism-evidence/lanes-2026-11-20/probes/lane8/make_fixtures.py` does),
`tests/algo/taxonomy/taint_parity_test.rs`; Modify `tests/algo/taxonomy/main.rs` (`mod` line),
`tests/integration/coverage_test.rs` (all three lists).

Row fields (DESIGN 5.1): `id`, `group`, `lang`, `fixture`, `axes` (`mechanism`, `layout`, `resolution`,
`depth`, `sanitizer`); `src` (`{file, line, bytes}`), `site` (`{bytes, slot}` or null), `sink` (`{file, line,
operand}`, operand bytes); `walk_comparable` (bool); `expect`: `walk` (`{reaches:
Reached|NotReached|BoundaryExited|Sanitized|SeedError, source, terminal, descent_depth}`, witness bytes or
null), `review` and `trace_review` (`exact|nameonly|unlabeled|none`), `catalog` (required list of `{file,
line, rule, operand, verdict}`, possibly empty); `reasons` (per level, DESIGN 5.3 list, or null); `knobs`
(`{edges, depth}` or null); `truth` (`flows|no_flow`); `known_wrong` (null, or the issue or group that fixes
it); `class`; `note` (free text; it never waives a check). `expect` is the behaviour after the row's group
ships. Rows true on main are `control`; a slice that moves a control cell edits the row in its PR and names it.

Harness: copy the fixture to a temp directory; `prism::repo_loader::load_repo`. **review** / **trace_review**:
`CpgContext::build(&repo.files, None)`, `prism_review::algorithms::taint::slice` with `TaintConfig { sources:
vec![src], taint_from_diff: false, extra_sinks: vec![], return_flow: false | true }` and a diff listing every
line; the cell is `prism::finding_confidence::classify_with_evidence("taint", ParseQuality::Clean, ev)` of the
`taint_sink` finding at the sink line (`result.evidence` is index-aligned), else `none`. **walk**:
`NavigationIndex::build`, `taint_reaches(&session, &[Loc src], Some(&[Loc sink]))` (as
`tests/reasoning/taint_reaches_test.rs` builds its sessions); the cell is the verdict of the sink operand's
node (not the line aggregate), plus the witness's source and terminal bytes; a seed error is `SeedError`.

```rust
// tests/algo/taxonomy/taint_parity_test.rs
const ENABLED: &[&str] = &["control"];   // "reasons": also check the reasons of every enabled row
#[test] fn parity_rows_are_well_formed()  // unique ids; fixture exists; one @SRC, one @SINK, each marker's file
                                         // named; src and operand bytes name an occurrence on the marked line;
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
| js01, js02, js08 | argument, same file / ESM import; `this.run(t)` | exact | unlabeled | Reached d1 | control (grade per D) |
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
| go01, go02 | argument, same file / sibling file | exact | unlabeled | Reached d1 | control (grade per D) |
| go04 | `exec.Command(c).Run()` | none | none | NotReached | control; truth flows, accepted loss |

**Further seed rows:**

| Row | Shape | Group: required |
|---|---|---|
| l01 | `log(t); g(t)`, one site of `g`; the witness's source occurrence is the `t` inside `g(t)` | control: Reached d1, source bytes asserted |
| l02, l03 | js11 in Python and Go | same_line: Reached d1 |
| l04 | `g(g(t))` | same_line: BoundaryExited, `ambiguous_site` |
| l05 | `g(user); g(user)` (pinned by `test_two_same_line_same_name_calls_do_not_descend`, `src/cpg/tests.rs`) | same_line: set at stage D |
| s01 to s04 | `import { run as go }`; default import under another name; `const go = run; go(t)` (#376 item 1); Python `from b import run as go` | spelling: Reached d1 |
| s05 | alias to one of two `run` functions in different files | spelling: Reached, resolved target only |
| b01 | `<D a={x} />` into `props.a` (needs lane 3 S3's producer, else `#[ignore = "#596"]`) | binding: Reached d1 |
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
| w09 | a call resolving Exact to two function expressions (`multi_exact`) | Reached | walk: set at stage D |
| n01 | `cb(x); handler(0);` on one line, the edge into `cb`'s target name-only with doubt `call_nameonly`, `handler` Exact (lane 9 row h13) | — | walk: BoundaryExited, `not_exact` |
| n02 | n01 with doubt `builtin_callback` (lane 9 row e14; the kind lane 9 adds) | NotReached, `no_graph_edge`, until lane 9 adds the edge | walk: BoundaryExited, `not_exact` |
| w10 | competing shallow and deep routes under `exact,3`; mutual recursion; zero budget; descent then ascent at the cap | — | walk: set at stage D, order-independent |
| b04 | `f({a:x}, false)` recursive, read after the call (`self_literal_after`) | Reached d0 over DataFlow (wrong witness) | binding: set at stage D, a counted self-parameter crossing |
| b04b | the read before the recursive call (`self_literal`) | NotReached | control |
| b06 | `pick({bad:t, clean:'fixed'})` into `observe(p['clean'])` (`literal_index`) | review exact (wrong edge; filed); BoundaryExited | binding: BoundaryExited, `target_not_binding` |
| b07, b08 | the same into `p.clean` / `p.bad` (`literal_dot`, `literal_tainted`) | NotReached / BoundaryExited | control / binding: Reached d1 |
| l06 | `other.run(t); this.run('fixed')` (`method_mixed`) | BoundaryExited | same_line: BoundaryExited, `not_exact` |
| l07 | `first('fixed', t); t = 'fixed'; first(t, 'fixed')` (`rebound_site`) | review nameonly (line granularity; truth no_flow); BoundaryExited | same_line: BoundaryExited, `ambiguous_site` |
| x01 | `z = forget(t)` returning a literal; `document.write(z)` (`constant_return_flat`) | review reports (known wrong; filed); NotReached | control, reason `shortcut_suppressed` |
| c07 | `x = escapeHtml(t); exec(x)` (`sanitizer_domain`) | line query Sanitized (documented any-category) | catalog: Reached, XSS sanitizer advisory |
| c08 | multiline `render_template_string("{{ name \| safe }}", other="fixed", name=t)` (`template_operand`) | review reports | catalog: one sink, operand the `name=` value |
| c09 | `document.write('fixed')` on the signature line of a callee reached at its parameter Def (`flat_def_line`) | review reports (known wrong; filed) | catalog: as review |
| c10, c11 | two calls of one rule on one line, one tainted; the same sink line shape in two files | — | catalog: one entry; two entries |
| g01 | `f({a})` into `sinkA(p.a)`, binding labelled `NameOnly(SameLine)` (`object_literal_arg_tests.rs`) | trace unlabeled | grade: never `exact` |

- [ ] Write fixtures, rows, README (schema, vocabularies) and test. Control rows GREEN on main; every other
      row RED once with its group enabled by hand (ledger). Commit; CI green; one reviewer (test-only).

### Task T-2: enumeration

- [ ] Tiers per ORCHESTRATION 6 on `rows.jsonl`. Axes: mechanism of flow (argument, object literal, JSX
      attribute, return, field write, capture, callback, receiver, callee alias); call-site layout (one or
      several per line, nested, multi-line argument, occurrence outside the site, chained call, same-line
      rebinding, zero-width and continuation nodes); producer and site identity (same target with mixed
      grades, Exact multi-target pools, method and alias combinations, earlier same-name JSX attributes);
      formal read (dotted, bare, mixed, literal and dynamic subscript, prefix and sibling keys, duplicate keys
      and spreads, member writes, nested callable timing); slot (spread before or after, Python starred and
      keyword, receiver offset, bound or shifted arguments); returns (same or different caller, used or
      discarded result, recursion, wrapper); resolution grade (Exact, name-only single owner, several targets,
      unresolved); depth (0 to 3, descent and ascent mixed, competing budgets); sanitizer (none, on path, off
      path, in callee, on one of two paths, wrong category, two categories on one line); public projection
      (several roots or uses, source equals sink, framework sources, seed errors); language (JS, TS, TSX,
      Python, Go). Census `other` and `unknown` examples become rows.

### Task E-2: catalog emitter inventory

- [ ] One enumerator (Opus-5.5, high) lists every family that adds a line to review's sink lines in
      `crates/prism-review/src/algorithms/taint/run.rs` and the structured dispatch: flat AST hits, legacy
      substring rules, C/C++ format wrappers and configured extra sinks, structured positional and semantic
      checks, JSX attributes, Python template and keyword cases, multi-line calls, JS request-source ranges and
      inline YAML, Python inline Flask, Go framework tables, inline and no-path fallbacks, synthesized
      target-seed paths, path-specific suppressions, Bash unquoted expansion. Each family gets at least one
      `catalog` row (or rows pinning its absence, if stage D declares it unsupported), crossed with explicit and
      diff-derived sources. Output `catalog-inventory.md` in the evidence directory; rows in `rows.jsonl`.

## Stage D: design of record (1.5 days)

- [ ] Brief the design seat with DESIGN 5.2, 5.3, 5.5, 5.6, the rows, `CENSUS.md`, `catalog-inventory.md`,
      the accepted-loss candidates (js06, js07, go04, census `other`), the known wrong rows, and these facts:
      `descend_target` requires a `Def` target and counts sites by the callee's spelling before resolving;
      it accepts one Exact target among several; its cache key omits the target node and the occurrence's end
      byte; ascent accepts any singleton-Exact call in the destination function; the visited set is
      first-come; object-literal edges end at the Use one hop from the formal's `Def`, and a computed read
      has the empty chain (`src/cpg/object_literal_args.rs`); Step 5c builds no return edge for a name-only
      call; the depth test runs before `descend_target`.
- [ ] The amendment states, each in the DESIGN subsection the slices cite: 5.2 Hop evidence (i) to (v), with
      multi-target sites and lane 3's line-keyed source nodes; 5.2 Binding (key path, nested callables,
      recursion, row b04); 5.2 Two call sites (rows l01, l04 to l07); 5.2 Spelling and the slot proof; 5.2
      Returns and Depth (rows w01 to w10); 5.3 names and precedence; 5.5 grade; 5.6 operand kinds and the
      unsupported families. The implementer updates rows. Both reviewers; cap 2. The orchestrator asks the
      owner DESIGN decision 2 now, with the measured import closure.

## Stage I: slices

Order: I-1 first (every later row asserts a reason; gate Explained reads them); I-walk (repairs the engine's
own wrong answers before any slice widens what it crosses); the descent slices by the ordering rule; I-5
after them, so `parity_knobs_agree` runs on the final rules; I-6 (rewrites trace grade cells); I-7 after I-5
(it takes `TaintReachesOptions`); I-8; I-9 only with the owner's answer. A slice starts only when the DESIGN
sections it cites are approved and the types it names have merged.

**Gates per slice** (DESIGN 7): Matrix and Defaults (all); Sound and Benchmark (I-walk and the descent
slices); Catalog (I-8, I-9); Recovered and Explained (close-out). ORCHESTRATION 5 items 1 to 4, 8 to 10; two
reviewers. No slice adds or relabels graph edges, so item 6 does not apply; the census rerun replaces it.

### Slice I-1: reasons (2.5 days)

**Files:** Create `src/cpg/trace_descent.rs`, `src/cpg/trace_descent_tests.rs` (`mod trace_descent;` and
`#[cfg(test)] mod trace_descent_tests;` in `src/cpg.rs`, which also re-exports `LossReason`); Modify
`src/cpg/trace.rs`, `src/reasoning/types.rs`, `src/reasoning/taint_reaches.rs` (`boundary_warnings`),
`src/navigation/render.rs`, `tests/algo/taxonomy/taint_parity_test.rs` (`ENABLED` gains `"reasons"`),
`CLAUDE.md`. Rule: DESIGN 5.3.

```rust
// src/cpg/trace_descent.rs — variants as stage D names them (DESIGN 5.3)
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
#[serde(rename_all = "snake_case")] #[non_exhaustive]
pub enum LossReason { TargetNotBinding, NoMatchingSite, AmbiguousSite, NotExact, UnprovenSlot,
    ReturnNotEnteredSite, DepthCap, PolicyCeiling, SelfParam, SelfReturn,
    NoGraphEdge, ShortcutSuppressed, CfgPruned, /* … */ }
impl CodePropertyGraph {
    pub(super) fn descend_target(&self, from: NodeIndex, to: NodeIndex, rel: Relation,
        cache: &mut BTreeMap<DescentGateKey, Result<(), LossReason>>) -> Result<(), LossReason>;
}
// src/cpg/trace.rs: BoundaryEdge gains `pub reason: LossReason`; Trace gains
//   `pub cuts: BTreeSet<(NodeIndex, NodeIndex, NodeIndex, LossReason)>` (root, from, to) for losses with no boundary
// src/reasoning/types.rs: InterproceduralBoundary { sink: String, reason: LossReason }   (additive)
```

- [ ] Commit 1, move only: `DescentGateKey`, `descend_target`, `call_site_matches_source_occurrence`,
      `is_parameter_binding_from`, `return_flow_ascent_allowed` to `trace_descent.rs`.
- [ ] Unit tests first (builders `build`, `spell` of `object_literal_arg_tests.rs` are `pub(super)`):
      `use_target_reports_target_not_binding`, `no_site_reports_no_matching_site`,
      `two_sites_report_ambiguous_site`, `nameonly_site_reports_not_exact`, `third_descent_reports_depth_cap`,
      `context_policy_refusal_reports_policy_ceiling`, `self_call_reports_self_param`,
      `self_return_reports_self_return`, `depth_zero_cfg_cut_is_recorded` (row w05's node is recorded with
      `CfgPruned`, verdict unchanged), `suppressed_shortcut_is_recorded` (x01); the enum and fields land with a
      placeholder in the same commit so they compile and fail. Implement in stage D's precedence; a reason is
      attached to the requested sink through the hop's identity; message `taint reached a {kind} boundary at
      {path} ({reason})`.
- [ ] Enable `"reasons"`: RED, GREEN. `test_self_call_param_stays_boundary` and
      `test_two_same_line_same_name_calls_do_not_descend` pass unchanged. PR body: reasons, move commit, Tier-A.

### Slice I-walk: hop evidence and walk state (3.5 days)

Rules: DESIGN 5.2 "Hop evidence", "Returns", "Depth". It changes today's answers only on rows `w01`, `w03`,
`w05`, `w06`, `w09` and rows stage D names. Lane 9's slices D and E2 merge only after this slice (the
name-only check of rows n01 and n02); the PR body says so on lane 9's issue.

**Files:** Create `src/cpg/trace_frames.rs`, `src/cpg/trace_frames_tests.rs` (`mod` lines in `src/cpg.rs`);
Modify `src/cpg/trace_descent.rs`, `src/cpg/trace_descent_tests.rs`, `src/cpg/trace.rs` (the walk keys its
states through `trace_frames`; weighted size not above main), `tests/algo/taxonomy/taint_parity_test.rs`
(`ENABLED` gains `"walk"`), `tests/fixtures/taint_parity/rows.jsonl`, `CLAUDE.md`.

```rust
// src/cpg/trace_descent.rs
pub(super) enum Slot { Arg(usize), Receiver, Return }
pub(super) struct SiteProof { pub caller: crate::call_graph::FunctionId, pub span: (usize, usize),
                              pub slot: Slot, pub target: crate::call_graph::FunctionId }
impl CodePropertyGraph {
    pub(super) fn prove_site(&self, from: NodeIndex, to: NodeIndex, rel: Relation,
        cache: &mut BTreeMap<SiteKey, Result<SiteProof, LossReason>>) -> Result<SiteProof, LossReason>;
}   // SiteKey: source and target node, slot, site span
// src/cpg/trace_frames.rs
pub(super) struct WalkState { pub node: NodeIndex, pub entered: Vec<(usize, usize)> }   // entered site spans, at most max_depth
```

- [ ] Tests first: `return_after_descent_only_through_entered_site` (w01),
      `root_in_callee_returns_to_any_exact_caller`, `deeper_first_route_does_not_block_cheaper_route` (w03),
      `ascent_and_descent_share_the_budget`, `synthetic_return_value_survives_depth_zero_filter` (w05),
      `spread_before_slot_refuses_unproven_slot` (w06), `exact_multi_target_site_follows_stage_d` (w09),
      `nameonly_callback_edge_not_crossed_through_same_line_exact_site` (n01; the same test for
      `builtin_callback` lands with lane 9's kind, n02),
      `site_cache_key_includes_target_and_slot`. Enable `"walk"`: RED.
- [ ] Implement; `descend_target` and ascent decide through `prove_site`; GREEN, every other enabled group
      unchanged; `test_self_call_param_stays_boundary` unchanged.
- [ ] Census, Benchmark, Sound, Tier-A, cloud tests and PR body as in the descent slices below.

### Slices I-binding, I-same-line, I-spelling (3 days each)

One slice per change of DESIGN 5.2: they edit one function, but each has its own group, census class and
sample, and merging them would make the Sound sample unattributable. Each needs I-walk's `prove_site`.

**Files:** Modify `src/cpg/trace_descent.rs`, `src/cpg/trace_descent_tests.rs`,
`tests/algo/taxonomy/taint_parity_test.rs` (`ENABLED`),
`tests/fixtures/taint_parity/rows.jsonl` (moved control cells), `CLAUDE.md`; I-same-line also `src/cpg/tests.rs`
(`test_two_same_line_same_name_calls_do_not_descend` takes stage D's expectation, named in the PR);
I-spelling also `docs/features/language-coverage/js-ts-known-gaps.md` (section 3, taint descent entry).

| Slice | Group | Rule | Unit tests first |
|---|---|---|---|
| I-binding | `binding` | DESIGN 5.2 "Binding" | `objlit_edge_into_formal_member_read_descends`, `objlit_edge_into_bare_formal_read_descends`, `objlit_edge_into_literal_subscript_read_refuses` (b06), `objlit_edge_into_disjoint_dotted_key_absent` (b07), `use_in_callable_nested_in_callee_refuses` (b05), `use_target_with_two_matching_sites_refuses`, `recursive_use_target_is_counted_self_param` (b04) |
| I-same-line | `same_line` | DESIGN 5.2 "Two call sites" | `two_sites_one_line_choose_containing_site`, `single_site_outside_occurrence_still_descends` (l01, source bytes asserted), `nested_same_callee_refuses_ambiguous`, `nameonly_edge_not_crossed_through_other_exact_site` (l06), `same_line_rebinding_refuses` (l07), `entered_site_return_discarded` (w02) |
| I-spelling | `spelling` | DESIGN 5.2 "Spelling" | `aliased_import_site_descends`, `const_alias_site_descends`, `aliased_site_descends_only_into_resolved_target`, `aliased_spread_before_slot_refuses` (w07) |

- [ ] Enable the group: RED. Implement. GREEN, every other enabled group unchanged.
- [ ] Write the prediction; rerun `run_census.py` on A with the head build: default byte-identical; trace
      gains listed by class; any trace loss is a WRONG to explain or fix; `ledger.py` against the parent build.
- [ ] Benchmark: harness run with the head build; traced count per class not below base.
- [ ] Sound: a reader (Opus-5.5, high, blind to slice, shown the new hop and its code) labels a random
      min(100, all) of the slice's new crossing paths from the ledger; DESIGN 7 Sound; adjudications kept.
- [ ] Tier-A; cloud tests at 1 and 4 threads (spec: `--test algo_taxonomy taint_parity_test::`,
      `-p prism --lib trace_descent_tests`, `-p prism --lib trace_frames_tests`, `--test reasoning
      taint_reaches_test::`); two reviewers.
- [ ] PR body: prediction and result, census against parent and baseline, sample tally, Tier-A, moved rows.
      I-spelling comments on #376 (item 1). I-binding comments on #596 that F5 is closed only if row b01 runs
      un-ignored on lane 3's producer; otherwise it says what is still pending.

### Slice I-5: knobs on both clients of the walker (2.5 days)

PR #841's review options govern context files (import depth, the label of an edge entering a context file);
the default review walk has no descent. So this slice adds descent knobs to the trace engine, exposed on
`taint-reaches`, the MCP tool and trace-mode review. Rule: DESIGN 5.4 (the ceiling governs the resolution
grade the hop evidence accepts at descent and ascent, where an edge exists; the depth counts every crossing).

**Files:** Modify `src/cpg/trace_descent.rs`, `src/cpg/trace.rs` (thread `descent` beside `policy`;
`MAX_TAINT_DESCENT_DEPTH` becomes the default), `src/cpg.rs`, `src/reasoning/taint_reaches.rs`,
`crates/prism-review/src/{cli.rs, main.rs}`, create `crates/prism-review/src/nav_taint.rs`,
`crates/prism-review/src/algorithms/taint.rs` (`TaintConfig.descent`), `algorithms/taint/run.rs`,
`crates/prism-review/src/api/run.rs` (`AlgorithmParams.taint_descent`), every `TaintConfig {` literal
(`rg -n "TaintConfig \{" crates tests`), `crates/prism-mcp/src/{input.rs, tools_reasoning.rs}` (schema;
description now covers knobs and reasons), goldens, `tests/algo/taxonomy/taint_parity_test.rs`,
`docs/MCP.md`, `CLAUDE.md`.

```rust
// src/cpg/trace_descent.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TaintDescentPolicy { pub ceiling: CrossingCeiling, pub max_depth: usize }   // Default: Exact, 2
// src/cpg/trace.rs (existing entry points delegate with the default)
pub fn taint_trace_nodes_with_descent(&self, roots: &[NodeIndex], order: Option<&dyn SameLineOrderView>,
    mode: ReturnFlowMode, descent: TaintDescentPolicy) -> Trace;
pub fn taint_trace_nodes_excluding_with_descent(&self, roots: &[NodeIndex], order: Option<&dyn SameLineOrderView>,
    excluded: &BTreeSet<(NodeIndex, NodeIndex)>, mode: ReturnFlowMode, descent: TaintDescentPolicy) -> Trace;
// src/reasoning/taint_reaches.rs (taint_reaches delegates with the default)
#[derive(Clone, Default)] #[non_exhaustive] pub struct TaintReachesOptions { pub descent: TaintDescentPolicy }
pub fn taint_reaches_with(session: &NavigationSession, sources: &[SeedSpec], sinks: Option<&[SeedSpec]>,
    options: &TaintReachesOptions) -> Result<Evidence, QueryError>;
```

CLI: `nav taint-reaches --edges exact|nameonly` (default `exact`) `--depth <0..=8>` (default 2). Review:
`--taint-descent-edges`, `--taint-descent-depth`, each `requires = "taint_return_flow"`. MCP: optional
`edges` and `depth` (0 to 8). The sanitizer bypass re-walk in `witness_mode` uses the same policy.

- [ ] Tests first: `parity_knobs_agree` (each enabled `walk_comparable` row under `(exact,2)`,
      `(nameonly,2)`, `(exact,3)` and its own `knobs`: trace-mode review's walk from the row's source
      occurrence and `taint_reaches_with`'s give the same verdict, source and terminal at the sink operand; the
      finding level is checked against the row's own expectation, not inferred); enable `knobs`; CLI parse
      tests in `cli.rs` (values, range, review flags without `--taint-return-flow` rejected); MCP input tests
      (accepted, out of range, unknown value). RED, implement, GREEN.
- [ ] Defaults gate; goldens re-recorded with the reason; Tier-A; cloud tests; two reviewers.
- [ ] PR body: `not_exact` and `depth` census sizes, and a labelled sample of 50 new crossing paths under each
      non-default knob (information, no threshold), for owner decisions 3 and 5.

### Slice I-6: trace-mode grade (1 day; only if stage D changes it)

If stage D keeps `unlabeled`, there is no slice. Otherwise, rule DESIGN 5.5: Modify `src/finding_confidence.rs`
(`EvidencePath::from_trace`: a graded `CallDescent` or `ReturnFlow` window becomes an `EvidenceHop::Call`
whose confidence is the weaker of the site's resolution and the crossed edge's own label, instead of setting
`crossed_unlabeled`), the trace (record the resolved edge and the crossed edge's label per accepted
crossing in `Trace`), `src/finding_confidence/tests.rs`, `tests/fixtures/taint_parity/rows.jsonl`,
`crates/prism-review/tests/cli/min_confidence_test.rs`, `CLAUDE.md`.

- [ ] Tests first: `justified_descent_window_grades_as_its_call`, `objlit_sameline_read_label_keeps_nameonly`
      (g01), `assignment_propagation_still_unlabeled`, and in `min_confidence_test.rs` a g01-shaped finding
      excluded by `--min-confidence exact`; edit affected trace_review cells. RED, GREEN. Census grade
      histogram before and after in the PR body.

### Slice I-7: catalog trait and catalog mode in core (3.5 days)

Needs I-5 merged (`TaintReachesOptions`). Rule: DESIGN 5.6.

**Files:** Create `src/reasoning/sink_catalog.rs` (in `src/reasoning/mod.rs`), `tests/reasoning/sink_catalog_test.rs`
(in `tests/reasoning/main.rs`; a fixed in-test catalog); Modify `src/reasoning/types.rs`,
`src/reasoning/taint_reaches.rs` (`witness_mode` becomes `pub(crate)` and takes an optional sink category),
`CLAUDE.md`.

DESIGN 5.6's `CatalogSink` and `SinkCatalog` verbatim, plus:

```rust
// src/reasoning/types.rs
#[derive(Debug, Clone, serde::Serialize, PartialEq)]
pub struct CatalogRule { pub rule: String, pub cwe: Option<String>, pub operand: Option<(usize, usize)> }
// SinkResult gains: #[serde(skip_serializing_if = "Option::is_none")] pub catalog: Option<CatalogRule>
// src/reasoning/sink_catalog.rs
pub fn taint_reaches_catalog(session: &NavigationSession, sources: &[SeedSpec], catalog: &dyn SinkCatalog,
    options: &TaintReachesOptions) -> Result<Evidence, QueryError>;
```

Rule: trace as frontier mode does; for each line holding a reached occurrence (Def or Use), build the witness
to each such occurrence as a `prism::data_flow::FlowPath` (one `FlowEdge` per hop) and ask `sinks_for_path`;
merge results by (file, sink span, rule, operand); each goes through `witness_mode` with the sink's category:
Sanitized only for a sanitizer of that category, in the witness hits and the bypass exclusions; any other
sanitizer is an advisory warning beside Reached. Each `SinkResult`'s `sink` is a `SymbolRef::Statement` of kind
`CatalogSink` with the sink's bytes; `catalog` holds rule, CWE and operand. Only `Reached` and `Sanitized`
sinks are reported.

- [ ] Tests first: `catalog_reports_reached_sink_with_rule`, `catalog_asks_def_only_lines`,
      `catalog_passes_witness_path_to_implementation`, `catalog_sanitized_needs_matching_category`,
      `catalog_other_category_sanitizer_is_advisory` (c07), `catalog_sanitizer_on_categoryless_rule_is_advisory`,
      `catalog_two_sinks_same_rule_one_line_are_two_entries`, `catalog_unreached_line_is_not_asked`. RED, GREEN.
      Existing wire output unchanged. Two reviewers.

### Slice I-8: review implementation and CLI catalog mode (4 days)

Rule: DESIGN 5.6, with the families `catalog-inventory.md` lists as supported.

**Files:** Create `crates/prism-review/src/algorithms/taint/catalog.rs` (`pub mod catalog;` in
`algorithms/taint.rs`; inside `taint` it reaches the `pub(super)` matchers); Modify
`algorithms/taint/sink_match.rs` (extract, behaviour unchanged, `structured_sink_decisions_on_line(parsed, line,
path: Option<&FlowPath>) -> Vec<StructuredDecision>` (call span, pattern, decided operand span, outcome), which
`line_matches_structured_sink` then folds), `crates/prism-review/src/api/` (re-export),
`crates/prism-review/src/{cli.rs, nav_taint.rs}`, `tests/algo/taxonomy/taint_parity_test.rs`, `CLAUDE.md`;
Create `crates/prism-review/tests/cli/nav_taint_catalog_test.rs` (in that directory's `main.rs`).

```rust
pub struct ReviewSinkCatalog { wrapper_sinks: Vec<String> }   // Sync: no RefCell inside
impl ReviewSinkCatalog { pub fn for_files(files: &BTreeMap<String, ParsedFile>) -> Self; }
impl prism::reasoning::sink_catalog::SinkCatalog for ReviewSinkCatalog { /* … */ }
```

Each call builds a fresh `SinkClassifier` (it holds `RefCell`/`Rc`). Flat: `identifiers_on_line` →
`classify(&parsed.path, …, SINK_PATTERNS)`; rule `SinkHit.rule`, CWE `SinkHit.cwe`, no category, operand
`None`, as review's flat matcher (any reached target on the line). Structured: the decisions above with the
witness path, so review's own operand, semantic and per-path cleansing predicates decide (Python template
keywords included); rule `structured-<category>` with lane 7 DESIGN 5.2's CWEs (79, 89, 918, 502, 78, 22),
the pattern's category. Once lane 7 merges, ids come from its registry. CLI: `nav taint-reaches --source …
--sinks catalog`, `conflicts_with = "sink"`.

- [ ] Step 1: the extraction alone; `cargo test --test algo_taxonomy taint_sink` unchanged. Commit.
- [ ] Tests first: `parity_catalog` (per enabled row: the reported multiset of (file, line, rule, operand,
      verdict) equals `expect.catalog`; every (file, line) of the review column's `taint_sink` findings is
      reported, or the row's catalog reason names why); enable `catalog`; CLI tests: `--sinks catalog` on c02
      prints the rule; on c08 it reports the keyword operand; with `--sink` it is rejected.
- [ ] Catalog-mode wall time and file-scan counts against frontier mode on A's largest JS repository and one
      Python and one Go repository, in the PR body (over 3× is a follow-up). Two reviewers.

### Slice I-9: the catalog in MCP — owner answer required (DESIGN decision 2; no default)

The lane stops before this slice until the owner has answered decision 2 (asked at stage D).

**Branch A: shared functions into core (I-9a move and factoring, 5 days; I-9b exposure, 2 days).**
- [ ] I-9a step 0: diagram the function-level closure of the catalog implementation (start from
      `taint/catalog.rs`; follow `use`, qualified paths such as `crate::algorithms::taint::matches_sink`,
      relative imports and re-exports). Factor the occurrence-matching and table functions out of source
      discovery, scoring and emission in review-only commits (behaviour unchanged, `taint_sink` tests
      unchanged); then move the factored closure into `src/taint_sinks/` and `src/reasoning/catalog_impl.rs`
      by move-only commits, review keeping re-exports, moved bytes checked identical. Rule ids: if lane 7 has
      merged, its registry mapping moves with them or is injected through the trait's return value. Update
      the `CLAUDE.md` code-organisation section.
- [ ] I-9b: MCP input `sink_catalog: true` (rejected with `sinks`); `tools_reasoning.rs` calls
      `taint_reaches_catalog`; `max_results` clips `per_sink` and reports the omitted count; goldens
      re-recorded; tests `taint_reaches_catalog_mode_reports_rules` over c01, c02, c08 and an input test.

**Branch B: the MCP crate depends on the review crate (2.5 days).**
- [ ] `crates/prism-mcp/Cargo.toml` gains `prism-review = { path = "../prism-review" }`; only a new
      `crates/prism-mcp/src/catalog.rs` names it (`prism_review::api::ReviewSinkCatalog`).
      `tests/integration/layering_test.rs`: `MCP_REVIEW_PATTERNS` stay for every other file; in
      `catalog.rs` only that path is allowed; add `mcp_catalog_file_may_name_only_the_catalog`; update its
      module doc and the `CLAUDE.md` layering paragraph. Then I-9b's input, call, goldens and tests.
- [ ] Run the `macos-footprint` idle guard locally (command in `docs/MCP.md`); report binary size and idle
      footprint against main.

**Branch C: CLI only (0.5 day).** `docs/MCP.md` and the tool description say catalog mode is CLI-only; a
follow-up issue records branches A and B with this lane's numbers.

## Close-out (2 days)

- [ ] Census rerun on A with the census apparatus (same frozen roots, manifests and options), final head:
      Explained (every lost default sink has a reason from DESIGN 5.3; `unexplained` zero); Recovered (share
      by class). Benchmark on B.
- [ ] Docs: `CLAUDE.md` (one paragraph on crossings, reasons, knobs, catalog mode, beside the reasoning-layer
      line); `docs/MCP.md` (`taint_reaches` row); `skills/prism-code-navigation/SKILL.md` (catalog mode,
      reasons). Comments on #602 (census table, what remains by design), #596 (F5, as I-binding states it),
      and lane 9's spread issue (row w06's trace reproduction).
- [ ] File issues for wrong results the lane does not fix, each with its review probe as reproduction and its
      row: the object-literal leaf's Exact edge into a computed read of a disjoint literal key (b06,
      `literal_index`); review's flat sink with a fixed operand on a line reached only at a parameter Def
      (c09, `flat_def_line`); review's finding through the same-line shortcut past a certified constant
      return (x01, `constant_return_flat`). Follow-ups: name-only return edges (graph change, DESIGN 5.4);
      lane 7 registry ids if not done; unexplained classes; catalog performance if flagged.
- [ ] Refresh `HANDOFF.md` and `LEDGER.md`; offload raw census output; delete `target/`.

## Acceptance for the lane

Every gate of DESIGN 7 holds on the merged head (numbers in the close-out report); both reviewers approved
every product slice; census and matrix are on the tracking issue. A failing gate stops the lane for the
owner; the orchestrator does not adjust thresholds.

## Estimate

| Stage or slice | Days |
|---|---|
| M, T and E, D | 4, 4, 1.5 |
| I-1 reasons; I-walk | 2.5; 3.5 |
| I-binding, I-same-line, I-spelling | 3 each (9) |
| I-5 knobs; I-6 grade (conditional) | 2.5; 0 or 1 |
| I-7 catalog core; I-8 review catalog and CLI | 3.5; 4 |
| I-9 branch A / B / C | 7 / 2.5 / 0.5 |
| Close-out | 2 |
| **Total** | **43.5 to 44.5** (A), **39 to 40** (B), **37 to 38** (C) |
