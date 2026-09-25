# Bounded CPG-entry and resolved-call proof over the twelve native-gap sites

**Status:** planning only. The spec review has a 2-round cap, with sol gating. This document authorizes no
implementation, public run, or merge. Implementation, if the owner approves it, gets its own 2-round review cap,
which the controller declares before dispatch.

**Predecessor:** the native positional-gap characterization, which concluded SPEC §8 (a), `next_action:
bounded_entry_and_call_proof`. Its authorities:

- `observation.json` SHA-256 `d7f7b1b8c8b107f3e666169c61dec507061db1e79c171a1b95cdb99163ff9355`
- `site-manifest.json` SHA-256 `789352a575d68ef672de7449299676de0c0aab8edd4abd8228e23efda65d326b`

Both are copied at `target/planner-context/` and are not yet on `main`.

**Base:** `origin/main` `7ecccd99`.

## 0. Owner decisions (before implementation)

| # | Decision | Options | Recommendation |
|---|---|---|---|
| D1 | Build the observer, or publish the probe readout? | **A.** Probe-receipt readout (Appendix A), 0 new lines: the controller re-runs the pinned PLANNING-PROBES commands twice, sol reconciles statically, and a readout records the decision. **B.** This spec: a byte-exact Rust observer, about 830–950 honest lines. | **A for this decision.** The probes already determine it (predicted yield: 1 tuple at 1 site, see §2). The refusals are structural and each has a cited mechanism. **B** is worth its cost only if the owner wants a reusable, byte-exact instrument, for example to re-measure on a second corpus before any positional-hole repair. |
| D2 | Pre-register the repair threshold (§8) | `Y≥3 ∧ S≥2` (proposed); a different pair; or "any Y≥1" | Confirm **before** implementation or run, so the rule cannot be fitted to the data. |
| D3 | Gate coverage for observer tests | Rust `#[cfg(test)]` in the example, run explicitly (proposed); or add a Node smoke module, which needs a `natives.json` row (env `PRISM_ENTRY_CALL_PROOF_EXAMPLE`, example `entry_call_proof`) | Rust only. A Node wrapper would duplicate the Rust tests and add custody surface. |

The rest of this spec is option B. It is written to be implementable as-is if the owner chooses it.

## 1. Question and deliverable

For exactly the 12 site identities selected by the predecessor, the question is: what does Prism's **current,
unmodified** CPG contain? Specifically:

- **(i)** an entry Def for the out-of-prefix parameter binding;
- **(ii)** resolved call sites to the callable;
- **(iii)** argument→parameter DataFlow at the selected ordinal.

Where one of these is absent, the proof must name which stage refuses and why.

The deliverable decides between two outcomes. Either **a targeted production repair is warranted**, in which case it
names the smallest one, but a separate plan implements it. Or the proof **defers, with a reason**.

This increment implements no parameter support, positional holes, export or JSX resolution, or ownership changes.

## 2. Grounding (measured; see PLANNING-PROBES.md)

- **Entry Defs.** Present with exact bytes for 11 sites. Absent for `getStateForZoom.appState`, because a parameter
  with only field-access use gets no Def by design (`src/data_flow.rs:602`).
- **Resolved callers.** Exist only for `prepareElementsForExport` (3) and `getStateForZoom` (5), all Exact
  `import_member`. The ten `forwardRef` components have zero. Their JSX uses drop `UnknownName`, because an exported
  `const X = wrapper(arrow)` records no export (`src/ast.rs:2884-2890`, deliberate). Some uses are also unowned
  (nested callback arrows) or aliased (`<Stack.Row>`).
- **Flow.** Zero inbound edges at all 12 selected ordinals. The in-prefix control holds: 3 of 3 edges into
  `elements`@0.
- **Counterfactual.** Only ImageExportDialog.tsx:108 → `exportSelectionOnly`@2 is blocked solely by the positional
  prefix.
- **Prediction:** `defer / prefix_only_yield_below_threshold`, Y=1, S=1.

The observer's job is to make these facts byte-exact, reproducible, and independently reconcilable. It must not
assume them.

## 3. Seams relied on, and contracts left unchanged

Line numbers are navigation hints at `7ecccd99`.

| Seam | Role in this proof |
|---|---|
| `src/data_flow.rs:590-624` | Parameter Defs come from `function_parameter_occurrences`. They are pinned to `line = function start` with the token's exact bytes. `:602` skips params with no bare reference. This defines what counts as an entry Def for (i). |
| `src/ast.rs:10507-10531`, `:10537-10548` | Occurrence API versus slot API. These are distinct contracts. |
| `src/parameter_slots.rs:268-291`, `:437-449`, `:451-479` | The positional prefix stops at `object_pattern` (`:441`; TS `simple_pattern` returns `None` at `:470`). |
| `src/cpg/build.rs:152-239` `compute_param_def_nodes` | JS/TS branch: requires a unique function match by line range and name (`:184-198`). Takes slots from `function_parameter_slot_occurrences` (`:201`), a supported-occurrence check (`:210`), and an exact-byte Def at the start line (`:216-229`). |
| `src/cpg/build.rs:1433-1528` `step5b_edges_for_caller_with_exact` | The production order: R6 skip (`:1451`), empty args skip (`:1463`), param nodes (`:1471`), prefix loop (`:1476`), arg bound (`:1477`), bound Def (`:1511`), arg occurrence (`:1531-1640`). The observer's refusal order (§7.4) mirrors exactly this. |
| `src/resolution.rs:3683-3685` `resolve_call_site`; `:2294` `resolve_call_site_full`; `:1080-1085` `ResolutionOutcome.drop` | Callers use the same resolver Step 5b uses. Drop reasons come from `_full`. |
| `src/call_graph.rs:420-440` `CallSite`, `:704-708` `CallGraph.calls` | Recorded call sites (caller identity, `start_byte`, `line`). |
| `src/languages/mod.rs:641-654`, `:709-726`, `:835-850` | JSX elements are call nodes; the tag name is the callee; JSX has no `arguments` field, so there are no positional args. |
| `src/ast.rs:518-542` | Argument spans are the named, non-comment children of `call_arguments`. The observer re-derives this rule because `call_argument_texts_and_spans_at` (`:11520`) is `pub(crate)`. |
| `src/ast.rs:2874-2890` | Wrapped-initializer exports are skipped. This explains component `UnknownName` drops; it is reported, not changed. |
| `src/repo_loader.rs:64`, `src/cpg/context.rs:67-75`, `src/navigation/cache.rs:80+` | The observer builds exactly what the nav cold path builds, with no cache. |

**Unchanged:** every file under `src/`, plus `Cargo.toml`, `Cargo.lock`, `build.rs`, and all caches (CPG 97 and
nav 53 are neither read nor written). The observer never enumerates destructured leaves as arguments and never
synthesizes a slot or edge.

## 4. Trust boundary (explicit)

**In scope.** The operator's own account on their own machine is the trust boundary. Inputs are pinned by hash, and
the controller verifies them with the commands in §10. The observer is a pure reader:

- it takes a root and a targets file
- it builds one in-memory CPG
- it writes JSON to stdout, and diagnostics to stderr with a nonzero exit

**Out of scope, and not to be reintroduced by review:**

- launcher or child custody
- binary self-hashing
- staging or atomic new-only publication
- timeouts and output caps
- symlink or TOCTOU races
- hostile same-account processes
- re-verifying source hashes inside the tool

**Lesson this rests on.** The predecessor's launcher-custody layer produced five non-converging review rounds (WRONG
counts 6, 3, 1, 1, 3). The controller-run pipe plus independent static reconciliation converged. A finding that asks
for custody machinery is answered by citing this section. A reviewer who shows a threat *inside* this boundary raises
an owner-level spec change, not an implementation fix.

## 5. Inputs (pinned)

1. **Targets:** `docs/eval/entry-call-proof/targets.json`, schema `prism.entry-call-targets/1`. It is copied
   byte-for-byte from this packet's `targets.json`, SHA-256
   `b584bb4abc8af77deae06044e633bbd58bd0f066a0c8e7660818cde3d7371ffd`. It holds 12 rows of
   `{path, callable{name,start_byte,end_byte}, binding{name,start_byte,end_byte}, ordinal}`. It is derived from the
   predecessor observation by this exact projection (jq 1.7.1):

   ```sh
   jq -S '{schema:"prism.entry-call-targets/1", source_observation_sha256:"d7f7b1b8c8b107f3e666169c61dec507061db1e79c171a1b95cdb99163ff9355", targets:[.sites[] | select(.next_proof_eligibility=="eligible") | . as $site | .candidates[0] as $c | ([$c.slots[].source_ordinal]) as $slotted | $c.bindings[] | select(.source_ordinal != null) | . as $b | select(($slotted | index($b.source_ordinal)) == null) | select($c.parameters[$b.source_ordinal].ordinary_required_identifier) | {path:$site.path, callable:{name:$c.name,start_byte:$site.start_byte,end_byte:$site.end_byte}, binding:{name:$b.name,start_byte:$b.start_byte,end_byte:$b.end_byte}, ordinal:$b.source_ordinal}]}' observation.json
   ```

   Sol reconciles each of the 12 rows against `observation.json` and the manifest. The site tuples must equal the
   manifest's 12 `(path,start_byte,end_byte)`.
2. **Source:** the whole pinned tree `/Users/wesleyjinks/prism-evidence/inputs/excalidraw-0642e72c/source`, bound by
   `source-file-manifest.sha256` (SHA-256 `1a6c4397bfacafc0dc2651d1f65fe4dd7afed60a0fa21f8f928061b9e20968f0`; 1,229
   files; the authority is `input-recovery.md`: the archive `0a5a136c…141d` matches the pin exactly).

**Why the whole tree, not a caller subset.** Callers live outside the 11 site files, in App.tsx, actions/*, and
ImageExportDialog.tsx. Resolution is whole-program: import and export facts, re-export chains, and name collisions.
A subset could change resolution; for example, removing a same-name definition could turn a dropped collision into a
false Exact. The production nav build is whole-root. The tool loads what `load_repo` loads, and the packet records
`files_loaded`, `files_skipped`, and a digest over `(hash, path)` of the loaded set. Sol checks this digest against
the pinned manifest, restricted to the loaded files.

## 6. Architecture and owned paths

Owned paths:

- `examples/entry_call_proof/main.rs` holds the observer and its `#[cfg(test)]` module. Cargo auto-discovers it; no
  `Cargo.toml` edit is needed. It uses only public API, with the imports shown in `skeleton/main.rs.txt`.
- `examples/entry_call_proof/fixtures/{calls,jsx,identity}/{repo/*,targets.json,expected.json}` holds the synthetic
  fixture data. It does not count toward the budget, but its size is reported.
- `docs/eval/entry-call-proof/{targets.json,observation.json,receipt.json,readout.md}`.

The example builds warning-free in default `cargo test` (examples compile but their tests don't run), so the
default suite count is unchanged. Before the implementer moves fixtures in-repo, they must confirm that `load_repo`
on an in-repo fixture directory loads exactly the fixture files. The parent `.gitignore` must not add or drop any.
If it does, relocate the fixtures and report.

CLI: `entry_call_proof <repo-root> <targets.json>`. It takes exactly two positional arguments; anything else is a
usage error with exit 2.

Flow:

1. Parse the targets strictly: `deny_unknown_fields`, the exact schema, a `.js/.jsx/.ts/.tsx` path, a binding inside
   the callable span with nonzero width, and no duplicate `(path, callable, ordinal)`. Sort by that key.
2. `load_repo(root)`, then `CpgContext::build_with_scope_graph_inputs(files, type_db, scope_graph_inputs)`, exactly as
   the nav cold path does. No cache is read or written.
3. Build three indexes once:
   - recorded sites by `(caller.file, start_byte)`;
   - `resolve_call_site` for **each site once**, indexed by target `FunctionId`;
   - a syntactic name-equal call-node scan over JS/TS files, restricted to the 12 callable names.
4. Emit one row per target (§7). Then emit totals and the decision (§8). Output is pretty JSON with one trailing
   newline, deterministic: BTree ordering, and no timestamps, paths outside the root, or NodeIndex values.

The skeleton (`skeleton/main.rs.txt`, unreviewed, `bfff8c48…`) shows one sound shape. The implementer may start
from it but owns every line. **This spec, not the skeleton, is normative.** Additions the skeleton lacks:

- resolve each site once;
- `resolved_targets_at_site`;
- pure `reconcile` and `prefix_only_blocked` functions;
- `callable_not_unique → inconclusive`.

## 7. Output schema `prism.entry-call-proof/1` and taxonomy

Top-level fields, in order:

1. `schema`
2. `measurement: "cpg_entry_and_resolved_call_observation"`
3. `authorizes_production_change: false`
4. `targets_sha256` (of the exact bytes read)
5. `source_observation_sha256` (copied from the targets file)
6. `repo {files_loaded, files_skipped, loaded_files_digest}`
7. `sites []`
8. `totals {…}`
9. `decision {…}`

### 7.1 Site row

A site row has these fields:

- `path`, `callable`, `binding`, `ordinal`
- `callable_status`
- `slots` (null, or `[{name,start_byte,end_byte}]`)
- `entry` (null, or `{status, bare_reference}`)
- `callers []`
- `candidates []`
- `site_status`

**`callable_status`:**

- `unique`: exactly one `all_functions` node has the exact byte span and `function_name` text, **and** exactly one CPG
  `Function` node has the same file, name, and bytes.
- `missing`: either count is zero.
- `ambiguous`: otherwise.

A non-unique row keeps `slots` and `entry` null and its arrays empty.

**`slots`:** the native `function_parameter_slot_occurrences` prefix. It is null if the API returns `None`, **or** if
the production unique-function rule (`build.rs:184-198`: line range plus name) matches other than exactly one node.
Null is never `[]`.

**`entry.status`:** count the CPG `Variable` nodes with all of these:

- file, function name, and `function_start_line` equal to the callee's;
- `line == start_line`;
- `path == AccessPath::simple(binding.name)`;
- `access == Def`;
- `(start_byte,end_byte)` equal to the binding's.

A count of 1 is `present`, 0 is `absent`, and more than 1 is `ambiguous`.

**`entry.bare_reference`:** `has_bare_references(fn, name)`. This is a diagnostic only; it explains `absent`.

### 7.2 Caller rows (stage ii)

There is one row per `(recorded CallSite, resolved hit)` where the hit's target equals the callee `FunctionId`. Rows
are sorted by `(file, start_byte)`. Fields:

- `file`, `start_byte`, `line`
- `caller`, `caller_start_line`
- `confidence` (`Exact` | `NameOnly`)
- `kind` (the serialized `ResolutionKind`)
- `resolved_targets_at_site`: the number of hits at this site. A value above 1 is the "ambiguous caller" signal;
  Step 5b binds to every hit.
- `arg_count`, `argument_variables`, `flow_edges`
- `production_refusal`, `prefix_only_blocked`, `prefix_controls`

### 7.3 Candidate rows (why callers are missing)

A candidate is a JS/TS call node (per `is_call_node`) whose `call_function_name` text equals the callable name, and
whose `(file,start_byte)` is **not** a caller row for this callee. Its fields are `file`, `start_byte`, `line`,
`disposition`, `drop`, and `resolved_targets`. The disposition is one of:

- `unowned`: no recorded CallSite exists at `(file,start_byte)` whose effective source callee name equals the callable
  name.
- `dropped`: a recorded site resolved to nothing. `drop` holds the `DropReason` Debug name.
- `resolved_elsewhere`: a recorded site resolved only to other targets.

The scan is a name-equal **lower bound**. Aliased uses (`import {X as Y}`, `<Stack.Row>`) are not found. The readout
must say so.

### 7.4 Stage (iii): flow at the ordinal, per caller row

Argument spans come from the first pre-order call node at `site.start_byte` whose callee text equals
`effective_source_callee_name()`. The spans are the named, non-comment children of `call_arguments`. `arg_count` is
null when the call has no argument list (JSX).

- **`argument_variables`:** the number of CPG `Variable` nodes of the **caller** identity (file, name,
  `function_start_line`) contained in the ordinal's argument span. It is null if that argument is absent. Caller
  identity mirrors Step 5b's lookup key.
- **`flow_edges`:** the number of incoming `DataFlow` edges into the unique entry node whose source is such a
  variable. It is 0 when the entry is not unique.
- **`prefix_controls`:** one entry `{ordinal, slot_def_present, flow_present}` for each `j < min(|slots|, arg_count)`.
  Its Def uses the same exact-byte rule as the entry, applied to slot `j`. This is the in-situ positive control.

**`production_refusal`:** the **first** refusing stage, in Step-5b order. It is a pure `classify(Facts)`:

1. `r6_candidate_excluded`: `kind == R6MultiOwnerCandidate`
2. `no_positional_arguments`: `arg_count` is null or 0
3. `callee_slots_unavailable`: `slots` is null
4. `slot_prefix_excludes_ordinal`: `|slots| ≤ ordinal`
5. `argument_absent_at_ordinal`: `ordinal ≥ arg_count`
6. `entry_def_unbound`: the entry is not unique, or `slots[ordinal] ≠ binding`
7. `argument_variable_unresolved`: `argument_variables == 0`
8. `none`

Then a pure `reconcile(classify, flow_edges>0)` compares that result against the graph:

| `classify` | `flow_edges > 0` | Result |
|---|---|---|
| `none` | true | `none` |
| `none` | false | `unexplained_absence` |
| any refusal | true | `unexplained_presence` |
| any refusal | false | that refusal |

The graph edge is ground truth. `unexplained_*` flags a divergence between the observer's model and production. It
is never silently resolved.

**`prefix_only_blocked`** is a pure predicate, a necessary-condition upper bound. It is not a proof that a repair
would emit an edge. It is true iff all of these hold:

- the refusal is `slot_prefix_excludes_ordinal`;
- `entry.status == present`;
- `argument_variables ≥ 1`;
- `confidence == Exact`;
- the site is not R6.

**`site_status`,** in priority order:

1. `callable_not_unique`
2. `no_resolved_callers`
3. `flow_present` (any caller has `flow_edges > 0`)
4. `flow_absent`

`totals` counts each `site_status:*`, `entry:*`, `caller_refusal:*`, and `candidate:*`, plus `sites`.

## 8. Decision rule (pre-registered; owner confirms D2 before implementation)

Definitions:

- **Y** is the number of caller rows with `prefix_only_blocked`.
- **S** is the number of sites with at least one such row.
- **M** is the number of `unexplained_*` rows.
- **U** is the number of sites whose `callable_status ≠ unique` or whose `entry.status == ambiguous`.

The pure function `decide(yields, M, U)` returns the first matching case:

1. `M>0` → `inconclusive / observer_model_mismatch`
2. `U>0` → `inconclusive / target_identity_unresolved`
3. `Y≥3 ∧ S≥2` → `targeted_repair_warranted / exact_callers_blocked_only_by_positional_prefix`
4. `Y=0` → `defer / no_exact_caller_blocked_only_by_positional_prefix`
5. otherwise → `defer / prefix_only_yield_below_threshold`

**Rationale for the thresholds.** The smallest repair changes CPG construction for every JS/TS callable with an object
or array pattern before a later parameter. That means:

- a CPG cache-version bump;
- parity tests;
- a Tier-A trigger (`src/cpg/`);
- review of hole semantics.

Repeated demand (at least 3 exact tuples across at least 2 callables) is the minimum signal that justifies that cost
on this corpus. The 12 sites are the **complete** object-before-later-required population of the pinned 414-file
census, so Y and S are corpus totals, not a sample.

### Readout obligations

The readout must do all of the following:

1. State the disposition and list every blocked tuple.
2. **Name the smallest repair** if the rule warrants one. The name is R1, "JS/TS positional holes for Step 5b": a
   hole-bearing slot variant consumed only by `compute_param_def_nodes` (`build.rs:201`). It leaves
   `function_parameter_slots` and its other consumers unchanged (`call_graph.rs:2075`, `peer_consistency_slice`,
   `primitive_slice`), and it needs its own plan.
3. **List the non-positional refusals as separate lanes, with counts, and not as repairs from this proof:**
   - wrapped-export JSX drops (`ast.rs:2884-2890`)
   - JSX has no positional args (`ref` is framework-routed)
   - field-only entry absence (`data_flow.rs:602`)
   - unowned nested-callback call sites
   - member-alias uses
4. Report candidate-scan limits and the complete 12-site denominator.

## 9. Tests, RED, controls, and budget

### 9.1 Behavioral RED

The RED is required and must be recorded before the observation logic is final. A test-only adapter switch
(`exact_entry: bool`, threaded as in the skeleton, `true` in `run()`) keys the entry by name, without bytes. Running
`PRISM_CAPTURE_RED=1 cargo test --offline --example entry_call_proof calls_fixture_complete_record` must fail on the
complete-record assertion **at the concrete value** `sites[bare].entry.status: "ambiguous" ≠ "present"`. The
consequences that follow also appear: `decision.exact_prefix_only_blocked` drops from 3 to 1, and the disposition
changes from `targeted_repair_warranted` to `inconclusive / target_identity_unresolved`. The test is admissible
because the adapter compiles and runs.

These are inadmissible as RED:

- a missing module or example, or a compile error
- a missing `expected.json`
- a fixture or setup refusal
- zero tests selected
- a passing characterization

Record the RED log excerpt in the handback.

### 9.2 Control groups (complete-record `assert_eq!` against hand-derived `expected.json`)

Before freezing the expected literals, characterize the native results; that characterization is not RED. Each
expected value must be justified from source text and the seams in §3, never pasted from observer output without
review.

1. **`calls`** (sources as in `skeleton/fixtures/calls`, including the `u` and `v` lines):
   - `both`: flow present at ordinal 1; prefix controls for 0 and 1 present; the comment-argument call
     `both(/* c */ n, later)` also gives `none`.
   - `mid` (variable): `slot_prefix_excludes_ordinal` and blocked. Prefix control 0 present.
   - `mid(…, true)` (literal): same refusal, not blocked (`argument_variables: 0`).
   - `fieldOnly`: `entry: absent, bare_reference:false`, not blocked.
   - `bare` ×2: entry `present`, exact bytes despite the same-line twin; blocked.
   - Decision: `targeted_repair_warranted` (Y=3, S=2).
2. **`jsx`:**
   - `Island` (forwardRef): no callers; one candidate `dropped / UnknownName`.
   - `PlainRef` JSX site: Exact, `arg_count:null`, `no_positional_arguments`.
   - `PlainRef({…}, r)`: blocked.
   - Class-method `setState` callback: owned, `argument_variables: 0`, not blocked.
   - Wrapped class-field arrow: candidate `unowned`.
   - Decision: `defer / prefix_only_yield_below_threshold`.
3. **`identity`:** wrong bytes, wrong name, and a missing file are each `missing`, with null or empty stages.
   Decision: `inconclusive / target_identity_unresolved`.
4. **Pure tables:**
   - `classify`: one row per refusal, with rows carrying several simultaneous failures so the order is pinned;
   - `reconcile`: all 4 cells;
   - `prefix_only_blocked`: each conjunct false once, including `NameOnly` and R6;
   - `decide`: each of the 5 branches, plus both threshold edges (`Y=3,S=1` and `Y=2,S=2` defer; `Y=3,S=2` warranted).
5. **Target parsing:** unknown field, wrong schema, non-JS extension, binding outside the callable, zero-width binding,
   and duplicate row each refuse. The canonical row parses.
6. **Determinism:** two in-process runs of `calls` serialize byte-identically.

The `ambiguous` callable and entry arms are fail-closed branches that no natural fixture reaches, and
`callee_slots_unavailable` / `r6_candidate_excluded` are reached only through the pure tables. State this; do not
fabricate a parser duplicate.

### 9.3 Required killed mutants

Run each mutant and record it as killed, with the killing test named:

| Mutant | Killed by |
|---|---|
| M1: swap `classify` rules 2 and 4 | `classify` table; `jsx` fixture |
| M2: drop entry byte equality | `calls` fixture (the RED) |
| M3: argument containment → same line | `calls`: the literal `true` shares a line with `n` |
| M4: drop `argument_variables ≥ 1` | `calls` literal; `jsx` `setState` |
| M5: drop `entry present` | `calls` `fieldOnly` |
| M6: treat an unrecorded site as `dropped` | `jsx` unowned |
| M7: `Y≥3` → `Y>3`, or `∧` → `∨` | `decide` table; `calls` |
| M8: count comment children as arguments | `calls` `v` line, which becomes `unexplained_absence` |
| M9: syntactic scan skips JSX nodes | `jsx` `Island` candidate |
| M10: `reconcile(none,false)` → `none` | `reconcile` table |

### 9.4 Honest budget

Executable lines are non-blank and not `//`-only. Rust is counted after `cargo fmt`, and `#[cfg(test)]` counts as
tests. There is no JavaScript. Fixture and expected JSON and fixture sources are not counted, but their sizes are
reported.

**Forecast basis.** The concrete, compiled skeleton measured **helper 618 / tests 158**. Implementation adds:

- resolve-once indexing
- `resolved_targets_at_site`
- the pure `reconcile` and `prefix_only_blocked` functions, and `decide(U)`
- tests: the `reconcile` and predicate tables, threshold edges, split long literals, and 2 more parse rows

| Bucket | Forecast | Hard cap (~15% margin) | 95% early stop |
|---|---:|---:|---:|
| helper | 630–700 | **800** | 760 |
| tests | 200–250 | **290** | 275 |
| combined | 830–950 | **1,090** | — |

Fixture data (not counted) is about 45 source lines, three targets files, and three `expected.json` files of roughly
250–450 pretty lines each.

**One slice.** The work is far below a multi-thousand-line big-bang, and it has a single owner path. A measured or
forecast breach of a hard cap is a STOP that returns to the owner, with no silent inflation and no restart.

## 10. Acceptance (controller-run; after implementation approval)

1. **Freeze the commit.** `git diff --stat origin/main -- src Cargo.toml Cargo.lock build.rs` must be empty; if it is
   not, STOP.
2. **Verify the input.** Run `cd <root> && shasum -a 256 -c ../source-file-manifest.sha256 --quiet`. Then check that
   the `find -type f` set equals the manifest set and that there are 0 symlinks. Record SHA-256 for the manifest
   (`1a6c4397…`), `targets.json` (`b584bb4a…`), and `observation.json` (`d7f7b1b8…`).
3. **Build.** `cargo build --offline --locked --release --example entry_call_proof`. Record the SHA-256 of
   `target/release/examples/entry_call_proof`.
4. **Run twice.** `target/release/examples/entry_call_proof <root> docs/eval/entry-call-proof/targets.json >
   <evidence>/out-N.json` for N=1,2. Both must exit 0, and the outputs must be byte-identical. Every run is cold,
   because the tool has no cache.
5. **Cross-check with the existing tools** (planning-probe commands on the same binary base). Compare:
   - the caller `(file,line)` sets for the two helpers against `nav call-stats --dump-sites`;
   - each `entry.status` against `nav nodes-at`;
   - "0 flow" against `nav dfg-stats --edges`.

   Any disagreement is a STOP, reported with both outputs.
6. **Independent static reconciliation (sol).** Re-derive every row from source text and the recorded fields:
   callable identity, entry, each caller's argument at the ordinal, refusal, predicates, totals, and decision. Check
   the loaded-files digest against the pinned manifest restricted to loaded files.
7. **Publish.** Write `docs/eval/entry-call-proof/{targets.json, observation.json (out-1), receipt.json, readout.md}`.
   The receipt records:
   - commit, binary, and input hashes
   - commands and exit codes
   - both output hashes
   - the cross-check result
   - the reconciliation verdict
8. **Project verification.** Report totals from logs:
   - `cargo fmt --all -- --check`
   - `cargo clippy --offline --all-targets --features mcp -- -W clippy::all` must be clean for the new file
   - default `cargo test --offline --no-fail-fast` must stay at **4,559 passed / 0 failed / 1 ignored**
   - `cargo test --offline --example entry_call_proof`: own totals
   - the full Node gate: `node scripts/gate-inputs/acquire.mjs && node scripts/gate-inputs/gate.mjs --out
     target/gate-runs/entry-call-proof`, which must pass with the population unchanged (no new `.test.mjs`, so no
     `natives.json` row)

   Report any out-of-scope failure; do not re-baseline or silently fix it.
9. **Tier-A is not triggered.** No file under `src/call_graph.rs`, `src/navigation/`, `src/cpg/`, or `src/ast.rs`
   changes, and the observer only reads. Touching any of them is a scope STOP, not permission to add Tier-A.

## 11. Review plan and STOP conditions

- **Spec review:** at most 2 rounds (sol gates; the standing parallel reviewer runs alongside). Disagreements go to the
  owner.
- **Implementation review:** at most 2 rounds, on the frozen diff, using `REVIEWER.md`. At the cap, the controller
  classifies before acting. Converging findings are folded with a one-line disclosure. Open-class findings park and
  escalate. There is no restart without owner approval.
- **STOP conditions:**
  - any `src/`, Cargo, or lockfile change
  - custody machinery (§4)
  - a public run before implementation approval
  - a cap breach
  - `unexplained_*` or a cross-check disagreement on the public run (report it; do not tune the observer to the data)
  - adding parameter or JSX or export support

## Appendix A: option A procedure (probe-receipt readout, 0 new lines)

1. **Pin.** Record the `origin/main` commit and the release `prism` binary SHA-256, and complete the input
   verification in §10 step 2.
2. **Run twice, cold.** Run each of these with `--no-cache`:
   - `nav call-stats --dump-sites`
   - `nav dfg-stats --edges`
   - `nav nodes-at` at each of the 12 `(path:start_line)`
   - `nav callers --symbol --file` for the 12 callables

   Hash every output; the two runs must be byte-identical.
3. **Derive the extracts** with the jq filters in PLANNING-PROBES (P2–P4), and write the per-caller argument table
   (P5) by reading the 8 resolved caller sites.
4. **Reconcile.** Sol statically re-derives P1–P5 from the hashed outputs and source.
5. **Write the readout,** applying the §8 rule by hand with the same readout obligations.

This option's weakness: `dfg-stats` edges lack byte and function identity. Absence claims stay robust; the only
presence claims are the ordinal-0 control. Per-argument attribution is manual, over 8 call sites.
