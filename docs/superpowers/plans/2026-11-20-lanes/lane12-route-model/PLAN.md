# Lane 12: routes as a first-class fact — implementation plan

> **For agentic workers:** this plan is run by an orchestrator under `../ORCHESTRATION.md`. Each task is
> dispatched to an implementer (Sonnet-5.5, high) with its section as the brief. Steps use checkbox syntax.

**Goal:** `prism nav routes` lists a repository's web routes (methods, path as written, every composed path,
full where the chain is static and its installation is certified, the registered callbacks in order with their identities or
why each is unresolved, a delegate where proven, registration site, framework) and its mounts: Flask, FastAPI and Express first, then Go, then Django, NestJS, object-literal tables and Hapi. Nothing the
extractor sees is dropped silently.

**Architecture:** a route table computed on demand from parsed files and the call graph
(`src/framework_entries/routes*`), composed through a mount graph, listed by `src/navigation/routes.rs` and
one CLI arm. No cache change; `FrameworkEntryRecord` and the `framework_entry` navigation edges untouched.

**Tech stack:** Rust (core and prism-review crates); Python 3 (census, schema oracle); uv, Node and Go for
the truth runner.

**Spec:** `DESIGN.md` here (records 5.1, composition 5.2, waves 5.3, listing 5.4, layout 5.5, Go method-value
handlers 5.6, measurement 6, gates 7, follow-ups 8, owner decisions 9, Hapi plugin adapter 11).

## Global constraints

- Worktree `~/code/prism-routes` (ORCHESTRATION section 4). Evidence `~/prism-evidence/routes/` (`m/` census
  and oracle, `truth/` venv and runner logs, `g/` gate runs, `LEDGER.md`, `HANDOFF.md`). Branches
  `test/routes-matrix`, then `feat/routes-s1` … `feat/routes-s<n>`.
- Must not change: `src/call_graph.rs` (5,653 weighted lines: not one line), `src/resolution.rs` (4,171.5:
  not one line), everything under `src/resolution/` and `src/call_graph/go/` (DESIGN 5.6 M4: the Go
  method-value lookup reads their facts, all already `pub` or `pub(crate)`, and changes none),
  `src/navigation/queries.rs` (2,502.5), `src/navigation/types.rs`, `FrameworkEntryRecord`,
  `framework_entries::apply`, `CACHE_VERSION` (`src/cpg_cache.rs`), `NAV_CALL_EDGE_CACHE_VERSION` (`src/navigation/call_edge_cache.rs`), taint code,
  `crates/prism-mcp`, and the tests `tests/navigation/framework_entry_test.rs`,
  `src/call_graph/framework_entry_tests.rs` (the stale inline-arrow test stays: entry edges are a follow-up).
- Allowed edits to existing files: `pub(crate)` on the helpers each slice names (no behaviour change); one
  `pub mod routes;` line in `src/framework_entries/mod.rs` and in `src/navigation/mod.rs` (1,148.5 lines; the
  PR body says that line is the only addition); `NavQuery::Routes` in `crates/prism-review/src/cli.rs`; a
  `run_nav` arm of at most 10 lines in `crates/prism-review/src/cli_nav.rs` (the exhaustive match in run_nav; rendering is in core);
  one `mod` line in `tests/navigation/main.rs`, in `crates/prism-review/tests/cli/main.rs` and (S3) in
  `src/ast.rs` (600.5 lines); the review fixtures, route pins, validator and source-truth projection named in S1a and their owning checkpoints; the route modules and tests introduced earlier in this lane; the docs each task names (`CLAUDE.md`,
  `docs/features/query-layer/navigation/README.md`, `docs/features/language-coverage/js-ts-known-gaps.md`).
- Sizes: `python3 scripts/file_size.py <files>`; new files aim for 600 weighted lines. Only lane-written
  fixtures are executed, never a corpus (owner decision 5), under DESIGN 6's containment and pinned offline
  dependencies; fixture apps export their app and never listen.
- The census may count the object-literal table shape and the Hapi plugin shapes (DESIGN 11.4) on lane 5's
  `pilot-producer` locally, Claude seat, aggregate counts only (ORCHESTRATION section 9); slice SH's gate is
  measured there the same way. No row, fixture or text derives from it.

## Review focus

1. A router imported from another file is proven, not looked up by file (DESIGN 5.2): `export default
   router` imported under another name and `from . import auth` + `auth.bp` compose to `full`; `from
   .routers import admin as u` never takes `public`'s routes from the same file. Tests
   `package_import_mount_composes`, `imported_member_selects_its_own_router` (S2),
   `default_export_mount_composes` (S3).
2. Every certified surviving, compatible all-literal chain yields a full exposure within the 64-exposure bound. Snapshot order, method/host guards and inactive branches restrict reach; dynamic prefixes retain only the suffix. Tests `every_known_mount_prefix_is_a_full_path`,
   `dynamic_prefix_branch_is_a_suffix_beside_full_branches`, `mount_cycle_and_expansion_bound_terminate` (S1).
3. A route is never attached to a router it was not registered on: a rebound or function-local receiver is
   another `RouterId` or unproven. Tests `function_local_lookalike_receiver_is_unproven` (S2),
   `rebound_router_routes_stay_unmounted` (S3).
4. Every callback position is recorded in order; a wrapper call is unresolved and a delegate needs a proven
   forwarder; a handler resolves in its argument's own scope (an imported `serve` beside a nested `serve`).
   Tests `every_callback_position_is_recorded`, `wrapper_call_delegate_needs_a_proven_forwarder`,
   `handler_resolves_in_argument_scope` (S3).
5. Every builder verb leg and the pinned method table are registrations; `app.get('env')` is not. Test
   `builder_legs_and_pinned_methods_extract_and_settings_read_does_not` (S3).
6. A Go file importing gin without `*gin.Context` (`parsed.framework()` is not gin there,
   `src/frameworks/gin.rs:16`) still extracts; each bare gin callback equals the `CallGraph::go_registrations`
   target at its own bytes. Tests `gin_router_file_without_context_type_extracts`,
   `gin_callback_identity_matches_callback_registration` (S4).
7. A Go method value resolves only from a declared type and one method of the owner index (DESIGN 5.6): the
   field's own package decides between two same-named controllers, a shadowed receiver and an interface-typed
   one never resolve, and a resolved method value agrees with the call resolver wherever both type the same
   receiver. Tests `same_named_controllers_in_two_packages_stay_apart`, `shadowed_receiver_is_refused`,
   `interface_typed_receiver_is_counted_never_resolved`, `method_value_agrees_with_call_resolution` (S4b).

## Stage M: measure first (2.75 days, closed)

The Stage M tasks below preserve the measurement protocol and prior predictions. DESIGN 7 records the frozen results.

### Task M-1: tracking issue and workspace

- [ ] File "Routes as a first-class fact: `prism nav routes` (lane 12)", linking `DESIGN.md`, this plan,
      #600 and PR #606 (the stage T PR writes its number into `DESIGN.md`). Create the worktree and the
      evidence directory; record these corpus commits in `LEDGER.md`:

| Corpus (`~/code/bench-repos/`) | Commit | Wave | Committed API description |
|---|---|---|---|
| `flask` | 36e4a824f340 | 1 | none |
| `fastapi` | 0cb4a8e284b4 | 1 | none (corpus apps are never run to print one; FastAPI schemas come from lane fixtures) |
| `express` | 18e5985b8a9d | 1 | none |
| `directus-directus` | c40ab79f2c4a | 1 | `packages/specs/src/openapi.yaml` (OAS 3.0.1, path items are `$ref`s into `paths/`) |
| `calcom-cal.com` | f972feefa137 | 1 (small), 3 (NestJS) | `docs/api-reference/v2/openapi.json` (OAS 3.0.0, 82 paths, 121 operations) |
| `apache-answer` | 6744295fa888 | 2 (gin) | `docs/swagger.json` (Swagger 2.0, basePath `/`, 161 paths, 199 operations) |
| `miniflux-v2` | faecfa2fac61 | 2 (net/http) | none |
| `prometheus` | 505095b64b43 | 2 (net/http) | `web/api/v1/testdata/openapi_3.1_golden.yaml` (not a gate input: own `route.Router`) |
| `gin` | d75fcd4c9ab2 | 2 | none |
| `saleor-saleor` | cc4aea76e397 | 3 (Django) | none |
| `screwdriver`, `hapijs-starterkit` (primary) and `hapipal-realworld-example-app` (zero-call control) | recorded at M-2 | 3 (Hapi plugin adapter, slice SH) | as found |

### Task M-2: census

**Files (evidence):** `m/census.py`, `m/census.json`, `m/ledger.jsonl`, `m/CENSUS.md`; census seat (Sonnet-5.5, medium),
prediction first. `census.py <corpus>...` skips `node_modules`, `vendor`, `dist`, `build`, `.git` and counts
each shape twice, by regular expression and by syntax (the installed Python tree-sitter bindings
`tree_sitter_python`, `_javascript`, `_typescript`, `_go`, written independently of prism), split test /
non-test (`test`, `tests`, `__tests__`, `*_test.go`, `*.test.*`, `*.spec.*`). Per framework (DESIGN 6):
shapes (Flask `@x.route`, `@x.<verb>`, `add_url_rule`; FastAPI verbs, `api_route`, `add_api_route`,
`websocket`; Express verbs, `.route(p).<verb>`, `use`; gin verbs, `Handle`, `Any`, `Group`; net/http with
and without a method in the pattern; chi verbs, `Route`, `Mount`; wave 3: NestJS decorators, Fastify, Koa,
Django `path`/`re_path`/`include`, DRF routers, `[{ method, path, handler }]`; Hapi: `server.route` calls and
row objects by form and handler spelling, plugin objects by shape and export form, `server.register` calls by
argument form and prefix source, DESIGN 11.4); handler form (named, inline,
wrapped, member, imported); prefix source (none, literal on the router, literal at a mount in another file,
variable, configuration); and each shape whose two counts differ by over 5%, with five examples. The syntax
pass writes `ledger.jsonl` (DESIGN 6: corpus, revision, file, start and end byte of the decorator, call or
builder-leg node, line, `registration` or `mount`, shape, test or non-test). It counts every Express builder
verb leg and every verb of the pinned Express method table (read from the installed `express@4.22.3`'s
`methods` dependency), and excludes settings reads (`app.get` with one string and no callback) by a stated rule. Self-checks first:
toy files with a two-leg builder, a settings read, a lookalike receiver and two registrations on one line give
the predicted ledger lines. The ledger is frozen (sha256 in `LEDGER.md`) before stage T.

Go callback shapes (DESIGN 5.6; owner decision 6): for every Go registration on the wave 2 corpora (and, as a
second look, `prometheus` and `caddy`), each callback position and the terminal one apart, classified from
the syntax and a per-package table of the corpus's own declarations (functions, methods per type, struct
field types, package `var`s, interface types), written independently of prism: bare function, function
variable, function literal (forwarding or not), wrapper call (with or without a method value among its
arguments), package function `pkg.F`, method expression, and method values by receiver form: the enclosing
method's receiver, one or two field hops from a receiver, parameter or local, a typed parameter, a local with a
composite-literal or constructor initialiser, a call-initialised local, a local bound once to a field chain, a
package `var`, an interface-typed field or parameter, a call result, three or more hops. For each method value
the table also records whether the corpus declares the method exactly once on the receiver's type (directly,
or promoted through a non-pointer embedded field). Output: `m/go_callback_shapes.json` and a table in
`CENSUS.md`, per corpus with counts and shares of non-test registrations, in the shape of DESIGN 5.6's table.
Prediction (DESIGN 5.6, research run of 2026-11-21): `apache-answer` 211 enclosing-receiver field values, 6
locals bound to a field, 9 bare functions, 3 literals, 3 wrapper calls of 232; `miniflux-v2` 150 composite
locals, 1 call-initialised local, 14 wrapped method values of 179. The shares become the denominators of
DESIGN 7's Go handler binding row and each 5.6 shape gets a ledger example for stage T.

### Task M-3: wave decisions

- [ ] Apply DESIGN 7's census-go row; every branch, written out:

| Result | Built | Not built |
|---|---|---|
| always (object tables are forced) | S1 (shared records, listing, composer) | — |
| wave 1 ≥ 200 (exactly 200 goes) | S2, S3; wave 1 gate | — |
| wave 1 < 200 | — | S2, S3; the wave 1 schema and completeness gates; directus rows not run |
| wave 2 ≥ 200 | S4 (gin), S5 (net/http and chi), then S4b (Go selector values and wave gate) | — |
| wave 2 < 200 | — | S4, S4b, S5; close-out promises lane 5 no Go records |
| wave 3 candidate ≥ 200 | one slice each, descending count, ties by name | — |
| object-literal tables | always (owner decision 3), last unless their count is higher; zero public sites: positive and negative rows are lane-written | — |
| Hapi plugin adapter | always (owner decision 7), slice SH right after the object-literal table slice | — |

- [ ] Candidates may also be counted on other `bench-repos` corpora that use them (name and commit
      recorded). A wave whose corpora carry no schema runs its gates without the schema oracle, and says so.
- [ ] Post counts and decisions on the issue; the stage T PR adds the wave 3 list to `DESIGN.md`.

### Task M-4: schema oracle

**Files (evidence):** `m/schema_oracle.py`, `m/test_schema_oracle.py` (`python3 -m unittest discover -s m`).

`schema_oracle.py --schema <file> --scope <scope.json> --listing <routes.jsonl> --out <dir>` implements
DESIGN 6's schema oracle. `scope.json` (written per corpus at M, before any listing exists) names the
application root file and binding, the revision, the API base (Swagger `basePath`, or the path of
`servers[0].url`, absolute URLs included) and the inclusion rule. Operations: the method keys (`get put post
delete options head patch trace`) of each `paths` entry, a relative `$ref` path item loaded from its file,
one key per method; schema `{name}` → `{}` for the key only, paths otherwise exact. Unsupported schema
constructs are reported as such, never matched. Binding input: for `apache-answer`, a self-contained reader
maps each `@Router <path> [<method>]` comment to the function it documents (file, name, line span), written
to `bindings.jsonl`; the oracle compares it with each matched operation's callbacks and delegate. Outputs:
`report.json` with numerator and denominator per class (exact full, unique suffix, ambiguous suffix, `Any`,
unsupported, binding agree / contradict / unresolved), `misses.jsonl`, `extras.jsonl`, pre-classed
`dynamic_prefix`, `unsupported_shape` (the literal occurs in code, no record lists it), `schema_drift` (no
occurrence, confirmed by a reader, since a composed path's literals can sit in several files), `out_of_scope`
(outside the bound root or base), else `unclassified`; a Sonnet-5.5 reader confirms each from the code, and
no gate report keeps `unclassified`. Unit tests (negative controls first): `/exact/` never matches `/exact`;
one suffix against `/a/u` and `/b/u` is ambiguous and credits neither; two handlers swapped on correct paths
give two binding contradictions; one operation dropped and one lookalike added changes the class counts; a
100-operation schema with 10 out-of-scope UI routes keeps precision 1.0; `Any` against explicit methods;
repeated operations across two roots; a base path; a `$ref` path item. Then record the denominators by
running it with an empty listing on the three schema files (precision is reported as undefined there, not 0
or 1).

## Stage T and E: the matrix (4.75 days)

### Task T-1: fixtures, rows, truth runner, data-driven test

**Files:**
- Create: `tests/fixtures/routes/<row>/` (one small application per row; `go.mod` in Go rows),
  `tests/fixtures/routes/rows.jsonl`, `tests/fixtures/routes/README.md`,
  `tests/fixtures/routes/truth/{run_truth.py,stage.py,flask_routes.py,fastapi_routes.py,express_routes.mjs,truth.jsonl,ENV.md,check.sh}`,
  `crates/prism-review/tests/cli/routes_matrix_test.rs`
- Modify: `crates/prism-review/tests/cli/main.rs` (`mod routes_matrix_test;`)

`rows.jsonl` fields: `id`; `dir`; `group` (`core`, `w1-python`, `w1-express`, `w2-gin`, `w2-method-value`,
`w2-nethttp`, `w3-<framework>`, `w3-hapi`); `class`; `truth_source` (`executed` or `hand`); `entry` (executed rows:
`module:attr`, `module:factory()`, or the JS file exporting the app); `expect` (route objects in DESIGN 5.4's
JSON shape, only the fields the row pins: `methods`, `path`, `paths`, `callbacks`, `receiver`, `site`);
`expect_mounts`; `accepted_loss` (optional reason); `ignore` (optional issue number). After Stage D approval apply STAGE-D-ROWS.md to this frozen input, author its explicitly listed new cases, and refresh baseline receipts. Preserve the raw receipts for the original cohort. Raw truth never changes by hand.

```rust
// crates/prism-review/tests/cli/routes_matrix_test.rs: drives the binary, so it names no Rust item that
// does not exist yet, and compares nothing while no group is enabled
const ENABLED_GROUPS: &[&str] = &[];  // each slice enables its assigned rows before implementing them
#[test] fn routes_matrix_rows_are_well_formed();  // unique ids, dirs are real directories under the fixture
                                                  // root, closed value sets; an executed row's truth.jsonl
                                                  // entry agrees with its `expect` under DESIGN 6's projection
#[test] fn routes_matrix_matches_rows();  // per enabled, non-ignored row: `crate::prism_bin()` runs
    // `nav --no-cache routes --repo tests/fixtures/routes/<dir> --mounts --format json`. Pass query_entry
    // (default: row.entry; null: infer), then run entry_queries separately. Projected route and
    // mount lines equal `expect`/`expect_mounts` exactly; the summary keeps the completeness invariant
```

Truth runner (orchestrator only, never `cargo test`), implementing DESIGN 6's executed truth and its
containment: `stage.py` validates each row (canonical path under `tests/fixtures/routes/`, no links, relative
imports inside the row, bare imports in the pinned set), stages the row and the pinned dependency tree into a
fresh temporary directory, and runs one child per row with a 10 s limit, a temporary `HOME`, and no
`PYTHONPATH`, `NODE_PATH` or `GOPATH` inherited. Environments, built once with versions written to
`truth/ENV.md`: `uv venv ~/prism-evidence/routes/truth/venv` then `uv pip install --python
~/prism-evidence/routes/truth/venv/bin/python --offline flask==3.1.3 fastapi==0.115.12` (both in the uv cache on
2026-11-21; a missing transitive wheel fails the install, and that group is hand-labelled and recorded),
children run as `venv/bin/python -I`; `npm install --offline --prefix ~/prism-evidence/routes/truth/node
express@4.22.3` (cached), its `node_modules` copied into each staged root, Node 24 (`.ts` rows use erasable
TypeScript under type stripping). The instrumented adapters (`flask_routes.py`, `fastapi_routes.py`,
`express_routes.mjs`) record per registration call its site, ordinal, arguments and callback identities and
per served route its path, declared and effective methods and the calls it came from (DESIGN 6); Express
mount prefixes come only from Express 4's literal-prefix layer pattern, otherwise the row is `hand`. gin, chi
(not in the Go module cache) and net/http rows are `hand`; Go never fetches. Runner self-checks, before the
first row: a valid multi-file row passes; an out-of-root `dir`, an `entry` through a link, an undeclared
transitive helper and an ambient `PYTHONPATH` module each refuse; two anonymous Express callbacks swapped in a
copy change the truth. `check.sh` re-runs, diffs `truth.jsonl` and fails if a row's `truth_source` no longer
matches what the runner can do, before stage D and before each merge. The runner beats a seed expectation;
note it.

Stages M, T and E are closed. The 420 fixture IDs and their data, including all E rounds, supersede the historical seed sketches. CLASS_MAP.md crosswalks the 171 census classes; RECONCILE.md describes the seed reconciliation. There are now 158 executed and 262 hand rows. Add only the bounded review cases listed at the end of STAGE-D-ROWS.md. See STAGE-D-ROWS.md for current expectations, including cases where a hand row has no observable from the runner.

## Stage D: design of record (2 days)

DESIGN 5.1–5.6, 7, 9 and 11 and STAGE-D-ROWS.md are one revision. The two ordinary design reviews have run. At the cap, the new cross-adapter interaction cases repeated a class; STAGE-D-ROWS records the §8 options and DESIGN 5.2 the chosen amendment. Both reviewers assess the amendment once under §8; rejection goes to the owner. After approval, apply the row patches and update the fixture validator and independent truth projections before enabling groups. Keep runtime truth bytes and provenance fixed. For each changed expectation refresh the row's base RED or GREEN on the same checkout/environment before its enabling slice. Preserve honest baseline GREENs, including empty controls. The generative checks below implement the open-class rule without a further E round.

## Stage I: slices

Cut each brief from the approved DESIGN and this PLAN at the same revision. Include the slice's cited sections, its rows from STAGE-D-ROWS.md, Stage I commands and the no-regression gate. Each slice owns all IDs in its named group after the two membership patches. Keep all earlier groups green. At each checkpoint enable its row IDs (incremental allowlists within groups); run its pins on the actual predecessor binary. Record row-level RED, initial GREEN or command-missing; a missing command is rechecked at S1, and a previously satisfied control is not claimed as behavioral RED. S1 also supplies pure tests on synthetic records. Use fail-on-pre-change and negative/edge coverage. Each product checkpoint below is a separate reviewable change with its own affected-row gate and two reviewers, cap three rounds; use ORCHESTRATION 5, 6 and 7. Through memguard run `cargo test -p prism-review --test cli routes_matrix_test::`, `cargo test -p prism --lib framework_entries::routes` and `cargo test --test navigation routes_test::`. Apply the project's Tier-A checks when applicable. Record commands, totals, gaps and final-head corpus results. Per-slice affected checks; full suite pending before merging to main.

Planning ranges for the previously oversized product changes, in handwritten weighted lines including tests but excluding mechanical row data: S1a 500–900, S1b 400–700, S2a/b and S3a/b 400–800 each, SHa/b/c 350–700 each. Estimate again against the actual dispatch diff and split at a named responsibility before review if it outgrows the checkpoint. Corpus classification and oracle adaptation have separate receipts.

At brief creation list each enabled ID, primary query (explicit or inferred), each secondary query, selector implementation and adapter dependencies. Enable both query modes through the actual CLI. S4/S5 own inferred Go roots and selection of their evidenced identity; S4b reuses both. S6 owns URLconf binding/inference, S7 the Nest bootstrap, S8 the Map installer and SH the grounded server/realm plus weak orphan projection. Rows needing a later checkpoint move by **ID** to that checkpoint before enabling; account for every ID at group close. For controls with two roots verify expected constructor byte and frames even in inference mode.

**No-regression gate, every slice, final head:** release builds of merge base and head; on the slice's
corpora `prism nav --no-cache call-stats --repo <c>` identical, and `nav --no-cache callers --format json`
identical for the head listing's first 20 resolved callbacks per corpus; `cargo test -p prism-mcp --test mcp
golden_transcript_test` passes unre-recorded; `git diff <base>..HEAD -- src/call_graph.rs src/cpg_cache.rs
src/resolution.rs src/resolution/ src/call_graph/go/ src/navigation/{queries,types,call_edge_cache}.rs` empty.

### S1a — wire contract, projection and CLI (3 days)

Implement DESIGN 5.1, 5.4–5.5 in new routes.rs, navigation/routes.rs, their tests, the module declarations, cli.rs and the short cli_nav.rs arm. Expose routes(files,call_graph,manifests,entry), optional --entry, text/JSON renderers, typed host/root and invocation fields, reason enum and summary partition. Document the CLI. Use empty inputs for the core rows; validate explicit selectors and preserve inference when omitted. Use synthetic identities to check present, absent, ambiguous and invalid selectors. S1a enables only e01/e02; the real fl15 candidate and error run in S2a. Compare actual candidate counts as a subset of route lines.

- [ ] Apply the approved patches to the 420 original rows. Implementers add the 35 specified review fixtures (d1-01–15 and d2-01–20) in their owning checkpoints, with their own baselines and provenance. Update routes_matrix_test.rs, its truth.rs and the offline reconcile.py projection. Validate the consolidated §5.4 wire table and all Reason values, presence vs null, binding guards and failed/uncertain root sets. Distinguish a Hapi plugin object association from its shared source site. Add the expect_bindings test projection and exact expect_failed_roots/expect_uncertain_roots comparisons. Pin exact route/mount multiplicity by origin with one-to-one partial matching, ordered callback slots, typed tokens/guards, optional delegate, plugin identity, and diagnostics as an exact multiset (absent expectation means empty). Materialize expected roots independently from fixture entry/declaration and call sites, compare --entry secondary queries separately from saved primary receipts. Check invalid selectors and corrupt/swapped root, origin, role and diagnostic controls. Remove the blanket empty-loss skip. Preserve the 158 raw executed observations. Enable core e01/e02, test sorted output and summary through the CLI.

### S1b — composition (3 days)

Implement DESIGN 5.2–5.3 in routes/mounts.rs and its synthetic tests. Cover live, snapshot and deferred installations, origin/epoch/root/guard, impossible chains, orphan suffixes, stable truncation at 64 plus a marker, grammar-specific path tokens and adapter-specific duplicate transitions. Use installation_event_product_matches_reference_replay and cross the install and request outcomes in the STAGE-D-ROWS dispatcher table. Mutate roots, order, guards and origins and add a third event after a possible abort; preserve both origins and independent method/root controls. Keep the projection and CLI gates green; adapters supply native fixtures in their own slices.

### Oracle adaptation — independent evidence task (1.5 days)

Outside both S1 product PRs, the evidence seat updates m/schema_oracle.py and its own tests. Before scoring freeze independent populations, roots, typed schema keys, selector/fixture joins and per-corpus sampled dimensions under DESIGN 6–7. Record seed, immutable sorted sampling frames, strata sizes, selected IDs, weights and blind source labels. Test a same-span wrong mount, wrong prefix/guard or cardinality, and failure to count an unlisted expected endpoint. Keep original scope denominators. Reject a literal-brace/placeholder collision, parameterized/literal host confusion, a wrong selected root, guarded GET scored as POST, swapped handlers, 1 correct key with the rest downgraded, and test-only aggregate recall. Record control numerators and denominators.

### Slice S2: Flask then FastAPI (3 + 2.5 days)

Implement DESIGN 5.1–5.5 and 7 in new `routes/{python,receivers,handler}.rs` (split Python setup/copy helpers at a responsibility boundary), `routes/{mounts,tests}.rs` and module declarations; existing flask/fastapi parsing helpers may become pub(crate). Update user page and CLAUDE.md. S2a enables the 49 Flask rows (47 with a Flask record plus es1-20 and es3-07). S2b enables the 39 FastAPI/Starlette rows (37 with a record plus fa12 and fa13). Gate each separately, including its new review rows. S2a owns Python module binding, import and factory selector resolution for its Flask rows, including fl15 inference plus explicit entry_unproven, blueprint replay and d2-14. S2b enables FastAPI module/factory and selected mounted or copied app roots with its include/API rows, including d2-15. Enable a mixed case only after its primary and secondary selectors work. All 88 original rows must be enabled by S2b. Scan the whole imported router binding surface, including files using an imported router without their own framework constructor. Use argument-scope value lookup, receiver lifetimes, positional/keyword APIs, Python literals and only certified captures/factory returns. See DESIGN 5.3 for default, omitted and WebSocket methods, compatibility calls, dependencies, build-only and scope limits.

- [ ] `function_local_lookalike_receiver_is_unproven` (fl21), `package_import_mount_composes`, `imported_member_selects_its_own_router` (fa14), `inner_but_not_outer_decorator_can_replace_endpoint`, `python_literal_expression_preserves_spelling`, `endpoint_only_has_no_registered_callback`, `websocket_and_http_share_path_without_method_collision`.
- [ ] `blueprint_override_empty_none_and_root_leaf`, `record_once_runs_only_on_first_registration_per_app`, `include_snapshots_keep_origin_multiplicity` (et6-01, es1-18, es7-08; move an include across identical calls), `build_only_has_no_dispatch_exposure` (es7-07), `subapp_mount_and_static_files_have_distinct_outcomes`. Contrast live mounts with copies in the event generator. Register two different factory-local routers with the same spelling; reject the wrong member. Reconcile test and non-test yield on flask and fastapi.

### Slice S3: Express adapter and composition (3 + 2 days)

Implement DESIGN 5.1–5.5 and 7 in new `routes/express.rs` (separate builder/mount or value-walk module if needed), `src/ast/js_router_binding.rs` plus its ast.rs declaration, routes/{receivers,handler,mounts,tests}.rs, user page, JS/TS known gaps and CLAUDE.md. S3a implements JS file → single app export selection for every direct row from ex01 onward, argument resolution, native Route/layer effective methods, competing GET/HEAD vs shared-builder control es4-01, all and continuation. S3b implements named/multiple-instance, factory and mounted-child selection, installer, mount/import composition and the cap. The 53 original rows with empty expect_mounts all select a single exported app by file and have no secondary query; enable those and d1-05 in S3a. S3b enables the 31 originals with mount edges (including the additional child query in et8-02), plus the multi-instance export selectors in d1-07. Gate all 84 original w1-express rows including es2-34 at S3b. Use the exact census verb table, source-scoped js_ts_site_binding, module/export terminal value and anon_owner::identity. A route-specific declarator helper returns the reaching value at the use, with new identity for an ordered rebinding. Do not reuse the old file-wide/single unwritten router test.

- [ ] `rebound_router_routes_stay_unmounted` (ex29), `default_export_mount_composes`, `replaced_module_exports_detaches_old_alias`, `handler_resolves_in_argument_scope` (ex06/ex27), `every_callback_position_is_recorded` (ordered flattened arrays, proved spread plus terminal, error middleware), `wrapper_call_delegate_needs_a_proven_forwarder` (local async-handler delegates absent; add a synthetic exact-package positive and a rebound-name negative).
- [ ] `builder_legs_and_pinned_methods_extract_and_settings_read_does_not`, `bound_bracket_and_chained_verbs_keep_the_receiver`, `literal_concatenation_and_route_arrays_expand`, `use_arrays_expand_each_router_and_prefix`, `live_mount_sees_later_registration`, `sixty_five_alternatives_keep_all_mounts`, `handler_matches_the_entry_record_on_overlap`. Verify site ordinals and duplicate-looking anonymous functions.

### Wave 1 gate — independent evidence task (6 days)

On final S3b head, allow 2 days for all 236 receiver flags, 695 middleware calls and ledger eligibility, 2.5 for independent stratified source labels and mount controls, 1.5 for joins, classification, mutation controls and receipts. Freeze labels before viewing output. Use §7 precision ≥0.95 per corpus/dimension, ≥0.80 eligible non-test recall at N≥25, classify every instance observed and fail unexplained discrepancies. Ledger joins cover sites; full field joins require an independent oracle. For mounts freeze separate parent/child, prefix, guard and multiplicity evidence, exhaustively on mount-only controls and sites without scorable descendants. Test a duplicated edge with unchanged span as well as a missing one. Directus: ≥0.95 full precision and ≥0.80 scorable full recall, report all 136 keys and terminal/delegate yield including the 295 local-forwarder observations. Record the independent sampled interaction loss by option, limits, time/RSS and no-regression; later waves reuse the same gate.

### Slice S4: gin sites and Go callback arguments (4 days)

Implement DESIGN 5.1–5.5 and 7 in new `routes/go.rs`, shared receiver/handler, mount join, tests and docs. Enable `w2-gin`. Ground gin by import/type and engine/group construction even without a *gin.Context parameter. Group may be passed inline or by variable into one installer. Count second-hop typed group as suffix. Apply the native per-method insertion certificates, d1-12 and d2-11, with failed root/site and independent engine pins. Include Match, Handle and finite Any, anonymous chained Group and explicit engine.RouterGroup. Before S4 dispatch the evidence seat freezes a per-corpus count of expanded bare elements and package-initializer arguments lacking byte-exact facts. Prefer the exact fact; for these two positions resolve the original lexical identifier to one same-package/profile-visible free function. Test a same-named shadow and an ambiguous build partition. Initial selector fallback is internal until S4b; no enabled row may require its final value prematurely.

- [ ] `gin_router_file_without_context_type_extracts`, `gin_callback_identity_matches_callback_registration` (each bare argument's byte range), `function_literal_has_text_and_no_graph_id`, `literal_variadic_slice_preserves_order`, `inline_and_named_group_share_one_helper_bound` (gn04, gn05 plus inline control), `gin_join_cleans_but_preserves_final_slash`, `gin_any_is_finite`. Gate on apache-answer and gin; record selector population still awaiting S4b.

### Slice S5: net/http and chi sites (5 days, before S4b)

Implement DESIGN 5.1–5.5 and 7 in `routes/{go_nethttp,chi}.rs`, declared under routes.rs, plus Go handler helpers, mounts/tests and docs. Enable `w2-nethttp` with nh01 moved to the selector group. Supply the sites needed by gm04, gm19 and the remaining net/http selector rows before enabling S4b. Include chi despite its zero registration-site count (Caddy has one Route mount). Implement native ServeMux method/host/path set relations, Go mux setup-failure propagation, chi per-method last writes and duplicate mounts (d1-13/15, d2-10/13). Include the mount-only d2-20, check independent inferred mux identities and every mount field.

- [ ] `serve_mux_parses_method_host_and_encoded_segments` (GET versus get, encoded slash, CONNECT, {$} and subtree), `package_helper_reads_current_default_mux`, `strip_prefix_uses_actual_strip_and_outer_guard` (es5-09, es7-01; GET→GET+HEAD, POST excluded only on that branch), `direct_mux_preserves_original_url`. Add the package-initializer bare-function row, inert literal control and net/http/chi duplicate-install controls. Test `handlerfunc_conversion_has_one_delegate` against an ordinary wrapper. `chi_route_group_with_and_opaque_mount` covers pathless inline child, factory return, regex, wildcard and extension verbs. No-regression on Miniflux, Prometheus and Caddy. The wave gate runs after S4b.

### Slice S4b: Go selector values (3 days)

Implement DESIGN 5.1, 5.6 and 7 in new `routes/{go_method_value,go_method_value_tests}.rs`, with call sites only under the Go route adapter; docs/user page. All `w2-method-value` rows, including nh01, gn03 and gm04/gm19, now have predecessors that recognize their registrations. Read GoReceiverCtx/classifier, field types, owner partition and concrete receiver routing; distinguish scope-local type/return-slot proof from a call-site fact. Keep src/call_graph*, src/resolution* unchanged.

- [ ] `same_named_controllers_in_two_packages_stay_apart`, `shadowed_receiver_is_refused`, `expired_if_shadow_and_same_block_redeclaration`, `interface_typed_receiver_is_counted_never_resolved`, `alias_and_new_defined_type_have_distinct_method_sets`, `package_expression_and_shadowed_qualifier` (gm15, es1-47), `method_value_agrees_with_call_resolution` (gm18; compare existing real call's Exact target). Exercise every M1 receiver form, promoted minimum-depth and prohibited explicit embedded hop, exact package paths, refused instantiated owners (direct, explicit and promoted), and build/test partitions. Cross-package promotion follows embedded declarations and their own visibility, breadth-first across distinct paths. When a router lane skipped a qualified/generic embedding, run the full walk first even if the lane refused. d2-19 checks a shallow concrete supplier vs deeper interface and the converse ambiguity. Never certify from a bare-name bucket. Contrast gm17 with es1-39/40; include collisions, field/interface and nearest-depth competitors. Unordered tagged alternatives refuse. A Go wrapper stays wrapper_call and gm19 gets no delegate.

### Wave 2 gate — independent evidence task (2.5 days)

Apply §7’s corpus and dimension floors, full oracle joins and frozen stratified samples. Exhaustively compare the Caddy chi edge: outer NewRouter r, distinct Route closure formal r, unevaluated ash.PathPrefix, absent request guard, exactly one child/prefix alternative. A right-span clone of the parent as child must fail. Enumerate other mounts with no scorable descendants before scoring.

- [ ] Wave 2 gate on apache-answer, miniflux-v2, prometheus, gin and Caddy chi control. Credit ≥207/217 Apache and ≥143/150 Miniflux only when the registration/terminal-site join returns the same declaring file/line and method as the frozen census method.declaration. Check every selector including schema-unmatched sites; report equal, unresolved, missing and contradictions, fail unexplained contradictions. Keep the external http.Handler terminal and wrapper arguments outside those denominators. Swap two targets in each census-join control and require two contradictions, even when the schema keys still match. Apache schema 191 in-scope, terminal binding ≥90% on matched operations, zero unexplained contradictions. Run no-regression on final wave head, time/RSS, class and ledger accounting.

### Slice S6: Django URLconf (4 days)

Enable all `w3-django`. Implement DESIGN 5.1–5.5, 7 in `routes/django.rs`, a dedicated URLconf composition helper if needed, shared Python callback/mount modules, tests and docs. Freeze independently selected ROOT_URLCONF and module/list binding identity before computing exposures. Support include tuples, +=, direct module/list/string references, conditional settings and i18n as specified. Include resolver failure/fall-through d2-16 and keep each installed URLPattern, including duplicate paths; check first-match shadowing separately. Keep every admitted child declaration and callback when an end-guard blocks its only exposure. Pin raw regex, decoded prefix and end guard separately.

- [ ] `urlpatterns_membership_and_imported_view` (cx51–60, es1-52); `captured_list_and_live_module_diverge` (es3-23); `regex_consumption_restricts_child_remainder` (es3-24, es4-15, es5-13); `locale_and_debug_select_setup_events` (es1-54, es6-04–07); `append_slash_is_no_registration` (es7-05). Perturb include order and false flag in the generator. Census reconciliation (996 sites across django, saleor-saleor and cockroach); independently reconcile both mount syntaxes. No schema corpus for this adapter.

### Slice S7: NestJS metadata and application roots (4.5 days)

Enable all `w3-nestjs`. Implement DESIGN 5.1–5.5, 7 in `routes/nestjs.rs` and separate module/bootstrap reader, reuse JS imports/value helpers; mount/tests/docs. Check platform-specific installation uncertainty against d1-11 and d2-17. Import-selected decorators and effective inherited metadata, module closure/context and RouterModule children; host/header constraints and URI version/defaults, global-prefix exclusions at the method, active configuration only.

- [ ] `only_reachable_controllers_and_effective_methods` (cx61–66, es1-61/62, es2-32); `version_host_and_prefix_constrain_exposures` (es1-57–60, es3-25, es7-09); `nested_module_paths_keep_receivers_distinct`; `dynamic_module_and_forward_ref_replay` (es6-08–10; definite, optional, cycle visited once). Cal.com schema 121 operations; reconcile 297 sites on Cal.com and fxa.

### Slice S8: object-literal tables (3 days)

Enable `w3-object-table`. Implement DESIGN 5.1–5.5, 7 in `routes/tables.rs`, JS handler/value helpers, tests and docs. Read own object fields, shorthand, static spread and one direct imported object/array at the element's definition site. Use the Map installation/dispatch proof and label object-table; pin both table origin and installation identity. No Hapi plugin credit in this slice.

- [ ] `grounded_table_consumer_lists_rows` (ol01–04), `ungrounded_table_is_a_candidate_without_credit` (ol05–07), `post_spread_value_is_the_installed_handler`, `one_literal_two_installs_and_one_passive_read` (es3-27/es5-15); check d2-18 conditional overwrites and replay exact keys on the same Map, pin superseded origins with empty paths and live keys on partial overwrites. A mismatched Map, key or discarded callback cannot certify an installer. Inspect the 150 fxa candidate objects independently; report gross candidate counts even where zero installations qualify.

### Wave 3 public gate — independent evidence task (1.5 additional days)

Complete the §7 samples and mount controls at the S6–S8 heads, in addition to the schema/ledger joins already budgeted in those slices. Report Django, Nest and table yield by corpus. Freeze the same independent event evidence used for the interaction option comparison.

### SH pre-dispatch measurement (0.5 day, evidence only)

Before **SHa**, an authorized seat performs the aggregate-only nearest-package manifest/use join of DESIGN 11.4, weighted by routes, literal paths and inline handler positions. Freeze private and public attainable counts and independent classifications. An infeasible public or private threshold returns to owner before SH dispatch. Keep the 400-entry denominator.

### Slice SH: direct rows, plugin realms, weak proof and lifecycle (1.5 + 2.5 + 2.5 days)

Use routes/hapi.rs and separate realm/lifecycle modules, the S8 object reader, adapter tests and user docs. Each checkpoint enables its rows by ID and reruns its predecessors.

- [ ] SHa implements grounded server constructors and direct route objects/lists, property overwrite, direct config/options binding/import and opaque handler fallback, pre groups/vhosts and nonliteral elements. Gate direct rows (hp12/13/19, cx47–49, et7-01, es1-26/28/29, es2-25/26, es3-21/22, es5-11/12) plus the new config and direct duplicate controls.
- [ ] SHb implements strong plugin contract/registration, exact item diagnostics, effective name/once/multiple and per-event nested realms. Exercise distinct objects sharing a name, unknown repeat metadata, conflicting/missing handler, valid and invalid formals, imported register, and multiple opaque/blocked items in one array. Qualify every plugin record by defining object even when register is shared; propagate possible aborts and register-level hosts. Gate hp08, es1-27/32/34, revised d1-09, d2-03/12 and other strong rows. The mixed strong/weak row d2-02 runs in SHc.
- [ ] SHc implements query-local nearest-manifest weak proof and plugin identity, negative use evidence and active lifecycle hooks. Gate hp16/22, cx07, es1-25, es4-14, es6-01–03, d2-01/02 and d2-04–09, including all four manifest-bearing negative controls. Then run all 62 original Hapi rows and additions. Test malformed/nested manifests, absent dependency, no invented mount, root:null suffix/typed tokens, and the listing fields for a configured consumer join.

### SH corpus gates (2.5 days, evidence only)

Require starterkit ≥24/29 correct plugin-contract records **and** ≥95% independently eligible, reconcile all 29 rows, including 24 referenced configs. On screwdriver reconcile 28 rows, all 134 opaque items by their independently classified receiver, and register diagnostics; the non-test site denominator is the 11 ledger literals, 17 test literals separate. Classify every small-N miss; report 0/134 expected factory-element exposure recall and gross 11/145 ceiling separately for owner acceptance. Apply §7 precision, mount and eligible controls; use hapipal as zero control. Authorized seat alone runs the private gate: ≥380/400 correctly grounded/exposed, ≥365/384 correct literal paths and ≥374/393 correct inline identities, all joined per entry before aggregation, zero unexplained. As negative controls, relabel the literal paths dynamic, then drop the inline identities. Each must fail independently of the 380/400 coverage check. Separate strong/weak and full/suffix/dynamic yield. Run final no-regression and timing/RSS. Configured consumer results never enter these gates.

## Close-out (1 day)

- [ ] `CLAUDE.md` Architecture paragraph (route table, listing, statuses, reasons, out of model) and the
      user page final. Comments on #600 (records exist; the request-source follow-up and its evidence) and
      PR #606 (the CLI listing exists; `nav_routes` is a DESIGN section 8 follow-up).
- [ ] Give lane 5 the shipped wire shape and measured binding losses. Match **validated path_tokens, root, protocol/method and effective constraints**, including host/version; normalized is display text. Exclude failed or possibly aborted setup roots, shadowed operations and blocked effective methods. Require a uniquely compatible endpoint with a resolved or anonymous identity, or a certified terminal delegate. Subject to owner question 11, document DESIGN 11.5: join a plugin_contract suffix to an explicitly configured per-pair plugin mount only as **configured**. Use the Hapi token join for /, require compatible known realm constraints and retain plugin identity, refusal and config provenance; count separately from proven full bindings. Include Go records and callback position semantics for #600. File DESIGN 8 follow-ups with their evidence.
- [ ] Run `truth/check.sh`; refresh `HANDOFF.md` and `LEDGER.md`; offload raw evidence; delete `target/`.

## Acceptance for the lane

Each built wave passed its census gate; the matrix holds for every enabled group; schema-oracle precision on
in-scope `full` exposures ≥ 0.95 on each schema-bearing corpus of a finished wave, every difference and
binding contradiction classified; reconciled completeness (no unexplained ledger site, recall at or above the
per-corpus non-test floor, independently verified path/root and endpoint accuracy, exact-full scorable recall and nonempty yield), the Go handler binding row when wave 2 is built, and no-regression hold on
each slice's final head; both reviewers approved
each slice; follow-ups filed.

## Estimates (working days)

| Phase in execution order | Days |
|---|---:|
| M, T/E, D (including review fold) | 2.75 + 4.75 + 2 |
| S1a projection/CLI, S1b composition, independent oracle | 3 + 3 + 1.5 |
| S2a Flask, S2b FastAPI | 3 + 2.5 |
| S3a Express, S3b composition, independent wave 1 gate | 3 + 2 + 6 |
| S4 gin, S5 net/http and chi, S4b selectors and wave 2 gate | 4 + 5 + 3 + 2.5 |
| S6 Django, S7 NestJS, S8 tables, additional public evidence | 4 + 4.5 + 3 + 1.5 |
| SH preflight, direct rows, plugin realms, weak/lifecycle, corpus gates | 0.5 + 1.5 + 2.5 + 2.5 + 2.5 |
| Close-out | 1 |
| Total for the admitted lane | 71.5 (round 1: 58.5, +13) |

Of the 13 additional days, 5 implement the §5.2 certificates (S1b, S2a, S3a, S4, S6, S7 and SHb +0.5 each, S5 +1.5). S1a wire and SHc weak controls add 1 day together. Evidence adds 7 days: wave 1 +3, wave 2 +1.5 over its earlier 1-day share of S4b, wave 3 public +1.5 and SH +1. The aggregate-only preflight remains a prerequisite for SH.
