# Lane 12: routes as a first-class fact — implementation plan

> **For agentic workers:** this plan is run by an orchestrator under `../ORCHESTRATION.md`. Each task is
> dispatched to an implementer (Sonnet-5.5, high) with its section as the brief. Steps use checkbox syntax.

**Goal:** `prism nav routes` lists a repository's web routes (methods, path as written, every composed path,
full where every prefix on its chain is static, the registered callbacks in order with their identities or
why each is unresolved, a delegate where proven, registration site, framework) and its mounts: Flask, FastAPI and Express first, then Go, then the wave 3 frameworks the census picks. Nothing the
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
  `run_nav` arm of at most 10 lines in `crates/prism-review/src/main.rs` (990.5 lines; rendering is in core);
  one `mod` line in `tests/navigation/main.rs`, in `crates/prism-review/tests/cli/main.rs` and (S3) in
  `src/ast.rs` (600.5 lines); the docs each task names (`CLAUDE.md`,
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
2. Every known mount prefix yields a `full` exposure; only a chain with an unknown prefix is a suffix, and no
   chain with a dynamic prefix is `full` (anti-goals). Tests `every_known_mount_prefix_is_a_full_path`,
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

## Stage M: measure first (2.75 days)

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
| `screwdriver` and `hapipal-realworld-example-app` (already under `bench-repos`; the gate corpus is chosen at M-2) | recorded at M-2 | 3 (Hapi plugin adapter, slice SH) | as found |

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
| wave 2 ≥ 200 | S4 (gin), S4b (Go method values, owner decision 6), S5 (net/http, chi: each built only if it has a ledger site) | — |
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
`expect_mounts`; `accepted_loss` (optional reason); `ignore` (optional issue number). After stage D the
implementer updates `expect` in the same commit as the design; truth never changes by hand.

```rust
// crates/prism-review/tests/cli/routes_matrix_test.rs: drives the binary, so it names no Rust item that
// does not exist yet, and compares nothing while no group is enabled
const ENABLED_GROUPS: &[&str] = &[];  // each slice adds its groups in its first commit
#[test] fn routes_matrix_rows_are_well_formed();  // unique ids, dirs are real directories under the fixture
                                                  // root, closed value sets; an executed row's truth.jsonl
                                                  // entry agrees with its `expect` under DESIGN 6's projection
#[test] fn routes_matrix_matches_rows();  // per enabled, non-ignored row: `crate::prism_bin()` runs
    // `nav --no-cache routes --repo tests/fixtures/routes/<dir> --mounts --format json`; projected route and
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

Seed rows ("Expect" is the hypothesis; "D" is settled at stage D; other wave 3 rows are written only for
the frameworks the census admitted):

| Rows | Input | Expect |
|---|---|---|
| e01–e02 | no web framework; a Flask import and no registration | empty listing, zero summary |
| fl01–fl04 | `@app.route("/items", methods=["GET","POST"])`; `@app.get("/x")`; `@app.route("/x")`; two decorators on one def | full; methods as written; fl03 implicit methods D; two records |
| fl05–fl10 | `Blueprint(url_prefix="/auth")` + `@bp.route("/<int:id>/u")` registered in another file; prefix only at `register_blueprint(bp, url_prefix="/v1")`; both; `url_prefix=prefix`; registered twice (distinct `name=`); nested `/p` + `/c` | `/auth/<int:id>/u` (normalised `/auth/{}/u`); `/v1/…`; override per truth; suffix; both full paths; `/p/c/x` |
| fl11–fl13 | `add_url_rule("/r", view_func=h)`; `add_url_rule("/", endpoint="i")`; `view_func=V.as_view("i")` | resolved; unresolved, D; unresolved, D |
| fl14–fl19 | `@app.route(PATH)`; `@self.app.route("/x")`; `url_prefix="/a"` + `"/"`; `methods=M`; routes in `create_app()`; `app = flask.Flask(…)` | dynamic; D (seen or loss); per truth; D; full; full |
| fa01–fa06 | `@app.get("/items/{id}")`; `APIRouter(prefix="/items")` + `include_router(items.router)`; both prefixes; `@app.api_route(…, methods=[…])`; `add_api_route("/x", h, methods=["PUT"])`; `@app.websocket("/ws")` | full; `/items/` per truth; `/admin/items/…`; as written; resolved; D |
| fa07–fa13 | `@app.get(app.url)`; included twice; nested include; `include_router(r, prefix=P)`; `from .routers import users as u` + `include_router(u.router)`; `app.mount("/s", StaticFiles(…))`; a non-router included | dynamic; both full paths; full; suffix; full; D; D |
| ex01–ex08 | `app.get('/x', named)`; inline arrow; `asyncHandler(async (req, res) => {})`; `asyncHandler(named)`; `wrap(a, b)`; `site.index` with `const site = require('./site')`; `import { list } from './users'`; `const h = asyncHandler(…)` then `get('/', h)` | resolved; `<anon@L:C>`; callback `wrapper_call`, delegate the anonymous function only if D lists the forwarder; same with a resolved delegate; `wrapper_call`, no delegate; resolved or `member_handler`; resolved; D |
| ex09–ex13 | `get('/', validate('r'), h, respond)`; `app.route('/r').get(h1).post(h2)`; `app.all`; template literal with/without `${}`; regex or array path | callbacks `wrapper_call`, `h`, `respond` in order; two records `/r`; D; literal / dynamic; D |
| ex14–ex17 | `(app) => { app.get(…) }`; `:id?` and `*`; `app.use(mw)`; `app.options`/`app.head` | unresolved, D; normalisation D; not a route; D |
| ex18–ex26 | TypeScript `import express`; same-file `app.use('/api', r)`; `app.use('/v1', require('./v1'))`; `module.exports = router` + `require`; `export default router` imported elsewhere; `app.use(PREFIX, r)`; mounted twice; nested `router.use('/sub', sub)`; `app.use('/u', r)` + `r.get('/', h)` | as JS; full; full; full; full; suffix; both full paths; full; per truth |
| gn01–gn06 | `r := gin.Default(); r.GET("/x", list)`; `g.POST("/y", func(c *gin.Context){})`; `a.ctrl.Get`; `register(r.Group("/api"))` with `g *gin.RouterGroup`; two function levels; `r.Group(conf.Base + "/api")` | resolved (callback-registration id); D (no Go anonymous identity); resolved by DESIGN 5.6 (gn03 is in group `w2-method-value`); full within the D bound; suffix beyond; suffix |
| gn07–gn10 | `r.Handle("GET", "/x", h)`, `r.Any`; `r.GET("/x", auth(), h)`; gin import without `*gin.Context`; `r.Group("", mw)` | D; callbacks `wrapper_call`, `h`; extracted; full |
| nh01–nh08 | `mux.HandleFunc("POST /v1/u", h.create)` with `h := &handler{…}`; `http.HandleFunc("/w", func(w, r){})`; `mux.Handle("/x", v)`; `"GET /x"` (serves HEAD too); `"example.com/x"`; `{p...}`, `{$}`; `HandleFunc(versionPath, h)`; `root.Handle(b+"/", http.StripPrefix(b, a))` | POST, callback resolved by DESIGN 5.6; any, handler D; D; D; D; normalisation D; dynamic; D (mount?) |
| gm01–gm04 | `a.questionController.List` in a method of `*AnswerAPIRouter` whose field is `questionController *ctrl.QuestionController`, `ctrl` another package of the row (the corpus shape); `a.siteInfo.Get` and `a.adminSiteInfo.Get` with fields `*ctrl.SiteInfoController` and `*admin.SiteInfoController`; `c := pr.connector` then `r.GET("/c", c.Info)`; `handler := &handler{…}` then `mux.HandleFunc("GET /x", handler.show)` (a variable named like its type) | resolved to `ctrl`'s `List`; each to its own package's `Get`; resolved; resolved |
| gm05–gm08 | `func reg(r *gin.RouterGroup, c *Ctrl) { r.GET("/x", c.List) }`; a package `var defaultCtrl *Ctrl` and `defaultCtrl.List`; value-receiver `func (c Ctrl) List` and pointer-receiver `func (c *Ctrl) Show` both registered through a `*Ctrl` field; `Ctrl` embeds `Base` (non-pointer, in-repo), `Base` declares `Health`, `a.ctrl.Health` | resolved; resolved; both resolved; resolved through the existing promotion facts (D confirms the route) |
| gm09–gm12 | a field `svc Lister` with `type Lister interface{ List(*gin.Context) }` and `a.svc.List`; a parameter `l Lister` and `l.List`; `a := other` in an inner block, then `a.ctrl.List`; a closure `func(a *Other) { r.GET("/x", a.ctrl.List) }` inside a method whose receiver is `a` | `method_value_interface` twice, no callback identity; `method_value_receiver_shadowed` twice |
| gm13–gm17 | `newCtrl().List`, `a.x.y.z.List` (three hops), `m["k"].List`; a field `onList func(*gin.Context)` and `a.onList`; `middleware.Ban` (package function) and `(*Ctrl).List` (method expression); `a.engine.HandleContext` with `engine *gin.Engine`; `a.Base.ctrl.List` through an embedded `*Base` field named explicitly | `method_value_receiver_unproven` three times; `func_field_value`; `qualified_function_value`, `method_expression`; `method_value_external`; `method_value_receiver_unproven` (no field-target proof) |
| gm18–gm19 | `r.GET("/x", c.List)` and the call `c.List(ctx)` in one function, `c` a typed parameter, then a field chain `p.f.List` from a parameter; `withApiKeyAuth(h.tokenHandler)` | each method value's target equals the call's Exact target; callback `wrapper_call`, a delegate only if stage D lists the forwarder, then resolved by 5.6 |
| ch01–ch04 | `r.Get`; `r.Route("/api", func(r chi.Router){…})`; `r.Mount("/adm", adm())`; `r.With(mw).Get` | full; full; D; full |
| ol01–ol04 | `route([{ method: 'GET', path: '/x', handler: h }])` with `route` a grounded consumer; exported array used in another file; inline handler; `method: ['GET','POST']` | listed; consumer list and framework label D |
| ol05–ol07 | the same array passed to `function route(rows) { return rows.length; }`; an unused exported array; a client request descriptor of the same shape | candidate `table_consumer_unproven`, no credit; nothing; nothing |
| fl20–fl21, fa14 | `@app.get("/w")` above `@replace` whose function returns another function; a function-local `app = Sink()` whose `get` returns an identity decorator, in a file importing Flask; `from .routers import admin as u` with `admin` and `public` in one file | callback `inner_decorator`; receiver unproven, no path; only `admin`'s routes |
| hp01–hp22 | the Hapi plugin rows of slice SH (DESIGN 11.3) | as listed there |
| ex27–ex30 | `import { serve }` beside an unrelated nested `function serve`, `app.get('/users', serve)` at module scope; `app.get('env')` and `app.route('/c').get(a).post(b).head(c)`; `let r = express.Router()`, `r.get('/live')`, `app.use('/api', r)`, then `r = express.Router()`, `r.get('/ghost')`; `app.use(['/v1', '/v2'], r)` | resolved to the export; no record, then three records; `/api/live` full, `/ghost` suffix `no_known_mount`; two full paths (D confirms arrays) |

- [ ] Write fixtures, rows, runner and test; run the runner's self-checks, then the runner; commit
      `truth.jsonl`. One reviewer (test-only). CI green. With no group enabled the matrix compares nothing:
      base RED for behaviour is shown per slice (stage I), not here.

### Task T-2: enumeration

- [ ] Tiers on `rows.jsonl` (ORCHESTRATION section 6). Axes: registration form (pinned verb table, builder
      legs saved in a variable or repeated, settings reads, middleware-only `use`, pathless and multi-child
      mounts, bracket verbs); receiver (bare, attribute chain, call result, parameter, imported, aliased,
      rebound, shadowed, live and unmounted routers of one name, two factory scopes, a sub-app both mounted
      and a root); path (literal, escaped literal, concatenation, f-string or template, constant, variable,
      regex, array, empty leaf versus `/`, trailing slash, literal colons and braces, parameter syntaxes,
      typed and tail converters, host patterns); methods (list, tuple, duplicates, mixed literal and unknown,
      variable, any versus unknown, websocket, implicit, framework-added HEAD/OPTIONS); callbacks (named,
      imported, renamed or re-exported, default, namespace member, class method, bound method, inline,
      nested decoy of the same name, wrappers, nested and branch-selecting wrappers, a discarding wrapper,
      arrays and spreads, terminal versus continuing middleware, error middleware, replacing and stacked
      decorators on either side, method-dispatched views, one callable at two sites on one line, swapped
      anonymous callbacks); Go selector callbacks (DESIGN 5.6: method values on the enclosing receiver, field
      chains of one, two and three hops, typed parameters, constructor, composite and call-initialised locals,
      a local bound to a field chain, package `var`s, interface fields, parameters and aliases to interfaces,
      embedded value and pointer hops, promoted methods, a shadowing local or closure parameter, same-named
      types in two packages, value and pointer receivers, methods split by build tags, generic receiver types,
      external types, function-typed fields, package functions, method expressions, method values inside
      wrappers); mounts (constructor, mount, both, override, variable, configuration, nested,
      twice, static plus unknown, a diamond, a cycle, cross-file by import, `require` or package import,
      barrels and star conflicts, non-router child); scope (module, factory, nested callable, `if DEBUG:`,
      loop over data); module forms (CJS, ESM, TypeScript, Python packages, multi-file Go packages,
      `_test.go`). Out of model shapes are `accepted_loss` rows. Runner truth first.

## Stage D: design of record (1.5 days)

- [ ] Inputs: DESIGN 5's tentative rules, `rows.jsonl` with `truth.jsonl`, `CENSUS.md`, the frozen ledger,
      the oracle denominators, the D cells as accepted-loss candidates. The design seat amends DESIGN 5.1 to
      5.3 and fixes, per framework: which calls and decorators are registrations or mounts; method
      derivation; the forwarder list for delegates; how imported and member callbacks resolve (a callback is
      a value reference, not a call site); Go function literals (Go method values are settled by owner
      decision 6: DESIGN 5.6's rules and reasons are confirmed, renamed, or narrowed by a row that shows a wrong
      binding, never reopened); per-framework joins and
      comparison keys for optional, wildcard, regex and host forms; the closed `Reason` list; the bound for
      groups passed as parameters; accepted losses; the ledger exclusion rules and the recall floor (DESIGN
      7). In the same change it amends every slice section below that the amendment contradicts (rules, test
      names and assertions, rows to enable, files), as ORCHESTRATION section 3 requires; a slice brief is cut
      only from the approved revision. The implementer updates `rows.jsonl`. Both reviewers; cap 2 rounds.

## Stage I: slices

The orchestrator pastes this paragraph and the no-regression gate into every slice brief. Each slice: its
first commit adds its groups to `ENABLED_GROUPS` (`crates/prism-review/tests/cli/routes_matrix_test.rs`) and
shows `routes_matrix_matches_rows` RED. S1's RED is bringing up the command (base has no `routes`); from S2
on, RED is the previous slice's merged binary on the newly enabled rows, row by row (the command exists, so
each row's failure is behavioural). ORCHESTRATION section 5 items 1 to 5 (item 5 because
`src/navigation/` is touched: Tier-A matrix and quick, regressions in the PR body), 8, 9, 10; two reviewers,
cap 3 rounds. Commands, through memguard:
`cargo test -p prism-review --test cli routes_matrix_test::`, `cargo test -p prism --lib framework_entries::routes`,
`cargo test --test navigation routes_test::`.

**No-regression gate, every slice, final head:** release builds of merge base and head; on the slice's
corpora `prism nav --no-cache call-stats --repo <c>` identical, and `nav --no-cache callers --format json`
identical for the head listing's first 20 resolved callbacks per corpus; `cargo test -p prism-mcp --test mcp
golden_transcript_test` passes unre-recorded; `git diff <base>..HEAD -- src/call_graph.rs src/cpg_cache.rs
src/resolution.rs src/resolution/ src/call_graph/go/ src/navigation/{queries,types,call_edge_cache}.rs` empty.

### Slice S1: records, listing and composer, shared by every wave (3 days)

First and always built (M-3), because every later slice extends its records, listing and composer; it holds
no framework extractor, so skipping a wave never removes what another wave needs.

**Files:** create `src/framework_entries/routes.rs`, `src/framework_entries/routes/{mounts,tests}.rs`,
`src/navigation/routes.rs`, `tests/navigation/routes_test.rs`, `docs/features/query-layer/navigation/routes.md`
(one example line per record kind and status, DESIGN 5.4); modify `src/framework_entries/mod.rs`,
`src/navigation/mod.rs`, `tests/navigation/main.rs` (`mod routes_test;`), `crates/prism-review/src/{cli,main}.rs`,
the navigation docs `README.md` (a row), `CLAUDE.md`.

```rust
// src/framework_entries/routes.rs: declares `mod mounts;` (S1), `mod receivers; mod handler;` (created by
// the first framework slice that runs), `mod python;` (S2), `mod express;` (S3), `mod go;` (S4),
// `mod go_method_value;` (S4b),
// `#[cfg(test)] mod tests;`. DESIGN 5.1's records verbatim (RouteRecord, Methods, RoutePath, Receiver,
// Exposure, PathStatus, Callback, CallbackValue, MountRecord, MountPrefix, RouterId); all derive Debug,
// Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, enums with a snake_case `status` tag.
pub enum Reason { ReceiverUnproven, MemberHandler, WrapperCall, InnerDecorator, ClassView,
    ImportedUnproven, MountChildUnproven, VariablePrefix, NoKnownMount, MountCycle, ExpansionTruncated,
    NonLiteralPath, TableConsumerUnproven,
    // DESIGN 5.6 (owner decision 6), produced from S4b on
    MethodValueReceiverUnproven, MethodValueReceiverShadowed, MethodValueMethodUnproven,
    MethodValueInterface, MethodValueExternal, FuncFieldValue, QualifiedFunctionValue, MethodExpression
    /* , stage D's additions */ }
#[derive(Default)] pub struct RouteTable { pub routes: Vec<RouteRecord>, pub mounts: Vec<MountRecord> }
/// Takes the session's parts: this module sits below `navigation`.
pub fn routes(files: &BTreeMap<String, ParsedFile>, call_graph: &CallGraph) -> RouteTable;
/// DESIGN 5.1's comparison key; `None` for a template the framework's grammar entry cannot classify.
pub fn normalize_path(framework: &str, path: &str) -> Option<String>;

// src/framework_entries/routes/mounts.rs: DESIGN 5.2 (one exposure per chain, cycles, the 64 bound)
pub(super) fn compose(routes: &mut [RouteRecord], mounts: &[MountRecord], roots: &BTreeSet<RouterId>);
pub(super) fn join(framework: &str, prefix: &str, path: &str) -> String;   // per-framework table, extended per slice

// src/navigation/routes.rs (serde::Serialize rows in DESIGN 5.4's shape; RouteSummary = registrations,
// proven, mounts and the BTreeMap<String, usize> counts by_framework, by_path_status, by_callback_status,
// by_reason)
pub struct RouteListing { pub routes: Vec<RouteRow>, pub mounts: Vec<MountRow>, pub summary: RouteSummary }
pub fn routes_listing(session: &NavigationSession) -> RouteListing; // routes(&session.repo.files, session.index.call_graph())
pub fn render(listing: &RouteListing, format: &str, include_mounts: bool) -> String;
```

JSON: DESIGN 5.4 lines and order; with `--mounts` one `{"mount":{…}}` line each; last `{"summary":{…}}`.
Text: one line per registration, `GET,POST /v1/users/:id | /users/:id (suffix) -> auth, <anon@12:24>
(routes/users.js:12) [express]`. CLI: `NavQuery::Routes { repo: PathBuf, format: String (default "json";
json|text), mounts: bool }`.

- [ ] Enable `core` (rows `e01` no web framework, `e02` a framework import and no registration); RED (the
      command is new). Tests in `routes/tests.rs`, on synthetic records: `every_known_mount_prefix_is_a_full_path`
      (one router under one root at `/v1` and `/v2`: two `Full`), `dynamic_prefix_branch_is_a_suffix_beside_full_branches`
      (one literal and one `Unevaluated` mount: one `Full`, one `Suffix`), `mount_cycle_and_expansion_bound_terminate`
      (a cycle closes as `mount_cycle`; 65 chains give 64 exposures and one `expansion_truncated`),
      `unmounted_router_is_a_suffix_no_known_mount`, `unproven_receiver_has_no_paths`,
      `normalize_path_is_framework_specific` (Express `/p/:id` → `/p/{}`; Flask `/p/:id` unchanged; Flask
      `<path:p>` → `None`); in `routes_test.rs`: `listing_is_sorted_and_ends_with_summary`,
      `json_lines_carry_status_discriminants`, `text_format_has_one_line_per_registration`,
      `other_languages_list_nothing`. GREEN. No-regression gate. PR body: rows enabled, no-regression
      result, sizes of new files and of `main.rs`.

### Slice S2: Flask and FastAPI registrations, callbacks and mounts (3.5 days)

Second: Python callbacks are mostly the decorated function, so review is about receiver identity, imports and
composition. Built only if wave 1 goes.

**Files:** create `src/framework_entries/routes/{python,receivers,handler}.rs`; modify `routes.rs`,
`routes/mounts.rs` (Flask and FastAPI joins), `routes/tests.rs`, `src/frameworks/python/{flask,fastapi}.rs`
(`decorator_receiver_and_method`, `is_flask_module`, `is_fastapi_module` → `pub(crate)`), the user page,
`CLAUDE.md`.

Sites: a new scan over the decorators and calls of Python files whose imports name Flask or FastAPI
(decorators, `add_url_rule`, `add_api_route`, `api_route`, `websocket`, `register_blueprint`,
`include_router`); neither `python::python_route_candidates` nor the file-wide `route_receivers` sets are
used. Receivers are grounded by DESIGN 5.1's rule (`receivers.rs`; `review_context/import_targets/shadow.rs`
shows the scope walk). Callbacks by DESIGN 5.1 (`handler.rs`): the decorated function unless a non-route
decorator sits below the route decorator; `view_func=`, `endpoint`. Mounts carry the effective prefix
(DESIGN 5.2); imported routers through `CallGraph::import_bindings` and `call_graph::file_matches_module`
(exactly one file), then the grounded binding there. The Python string-literal helper is new (none exists).

- [ ] Enable `w1-python`; RED (S1's binary on these rows). Tests in `routes/tests.rs`:
      `flask_decorator_reads_methods_and_path`, `python_registration_without_literal_path_is_dynamic`,
      `function_local_lookalike_receiver_is_unproven` (fl21: a function-local `app = Sink()` in a Flask file
      gives an unproven receiver and no path), `replacing_decorator_below_the_route_is_inner_decorator`
      (fl20), `stacked_route_decorators_give_two_records_for_one_function`, `package_import_mount_composes`
      (`from . import auth`, `register_blueprint(auth.bp, url_prefix="/a")`: `full`),
      `imported_member_selects_its_own_router` (fa14: only `admin`'s routes under the mount),
      `registered_twice_yields_both_full_paths` (fl09, fa08),
      `flask_registration_prefix_replaces_and_fastapi_prefixes_add` (fl07, fa03),
      `flask_empty_rule_under_prefix_is_the_prefix`. GREEN. No-regression gate on `flask` and `fastapi`;
      PR body: the summary on both beside the ledger's counts.

### Slice S3: Express registrations, callbacks and mounts, closing wave 1 (4.5 days)

Third: the riskiest callback rules (scope, imports, wrappers) and the builder matcher. Built only if wave 1
goes.

**Files:** create `src/framework_entries/routes/express.rs`, `src/ast/js_router_binding.rs` (declared by one
`mod` line in `src/ast.rs`); modify `routes/{receivers,handler}.rs` (JS/TS parts), `routes/mounts.rs` (Express join), `routes.rs`, `routes/tests.rs`, `CLAUDE.md`, the user page,
`docs/features/language-coverage/js-ts-known-gaps.md`.

Sites: a route-specific matcher over `collect_js_ts_call_nodes` (`express_call_match_kind` and the entry
extractor stay as they are): verb calls on a grounded receiver, the pinned Express method table (M-2), every
verb leg of a builder walked back to its grounded `.route(p)`, `app.get` with one string and no callback
excluded as a settings read, `use` split into middleware and mounts (a path or array of paths and a router
child). Receivers by DESIGN 5.1: `js_router_binding.rs` adds `pub(crate) fn js_ts_unique_declarator(&self,
ident: Node<'_>, name: &str, cache: &mut JsBindingCache<'_>) -> Option<Node<'_>>` on `ParsedFile`, the
nearest declaring scope's single, unwritten declarator, built on `js_ts_binding_walk`, `js_ts_scope_lookup`
and `js_ts_scoped_written` (the walk is not exported today); its value must be a recognised constructor.
Callbacks by DESIGN 5.1: every argument after the path in order, `ParsedFile::js_ts_site_binding` at each
identifier, imports through `CallGraph::import_bindings` and the target's `js_ts_exports`, inline functions
by `anon_owner::identity`; path literals by DESIGN 5.1 (not `js_ts_literal_string_value`). Imported routers
through `call_graph::js_ts_relative_module_candidates` and the target file's export entry or `module.exports`,
then the grounded binding there.

- [ ] Enable `w1-express`; RED (S2's binary on these rows). Tests: `every_callback_position_is_recorded`
      (member, call, array, spread and missing callbacks each recorded in order with a status; records =
      proven + unproven receivers), `inline_handler_gets_its_anonymous_identity`,
      `wrapper_call_delegate_needs_a_proven_forwarder` (ex03 to ex05: `wrapper_call` callbacks; a delegate only
      for a listed forwarder; `choose(save)` with `choose = ignored => maintenance` gets none),
      `handler_resolves_in_argument_scope` (ex27: the imported `serve`, not the nested one), and
      `handler_matches_the_entry_record_on_overlap` (equal to the `CallGraph::framework_entries` handler for the
      same site where the binding is unique, visible and unwritten),
      `builder_legs_and_pinned_methods_extract_and_settings_read_does_not` (ex10, ex17, ex28),
      `rebound_router_routes_stay_unmounted` (ex29), `default_export_mount_composes` (ex22),
      `array_mount_yields_each_path` (ex30), `escaped_or_concatenated_path_is_not_a_literal`. GREEN.
- [ ] Wave 1 gate (DESIGN 7): schema oracle on `directus-directus`; reconciled completeness against the
      frozen ledger on the five wave 1 corpora, with the positive-yield check; no-regression gate with cold
      session time, table time and peak RSS. Results in the PR body, raw in `g/`.

### Slice S4: Go gin (3 days)

Fourth: gin is the larger Go family (237 registrations in `apache-answer`) and reuses S1's composer; it needs
no wave 1 code. Built only if wave 2 goes.

**Files:** create `src/framework_entries/routes/go.rs`, and `routes/{receivers,handler}.rs` if wave 1 was
skipped (else modify them); modify `routes.rs`, `routes/mounts.rs` (gin join), `routes/tests.rs`, docs.

Routers: `gin.Default()`/`gin.New()` results, `Group` results (a mount), parameters typed `*gin.Engine`,
`*gin.RouterGroup` or `gin.IRoutes` within stage D's bound. A file qualifies by importing
`github.com/gin-gonic/gin` (`parsed.extract_imports()`), never by `parsed.framework()`. Every handler argument
is a callback, in order (DESIGN 5.1); a bare identifier takes the target of the `CallGraph::go_registrations`
record whose `site.file` and `site.start_byte` equal that argument's own; a call is `wrapper_call`; a selector
is `member_handler` in this slice (DESIGN 5.1's default), replaced by S4b. The `w2-method-value` rows are not
enabled here, so no S4 row or test pins a selector's reason.

- [ ] Enable `w2-gin`; RED. Tests: `gin_router_file_without_context_type_extracts` (a file that takes
      `r *gin.RouterGroup` and never names `*gin.Context`); `gin_callback_identity_matches_callback_registration`
      (`r.GET("/x", auth, endpoint)`: callbacks `auth` then `endpoint`, each equal to the `go_registrations`
      target at its own bytes); `group_passed_beyond_the_bound_is_suffix`. GREEN. No-regression gate on
      `apache-answer` and `gin`; PR body: the summary on both (on `apache-answer` about 217 `member_handler`
      callbacks are expected here, DESIGN 5.6's census).

### Slice S4b: Go method-value handlers (2.5 days)

Between S4 and S5 (owner decision 6): S4 gives the Go extractor whose selector callbacks this slice resolves,
and S5's net/http rows (`nh01`) and the wave 2 gate need it. Its own slice because it is the only Go typing
logic in the lane: review is about DESIGN 5.6's receiver proof and nothing else. Built only if wave 2 goes.

**Files:** create `src/framework_entries/routes/go_method_value.rs` (DESIGN 5.6; aim under 600 weighted
lines) and `src/framework_entries/routes/go_method_value_tests.rs` (declared `#[cfg(test)] mod` from
`go_method_value.rs`); modify `routes.rs` (`mod go_method_value;`), `routes/go.rs` (a selector callback, and
a selector forwarded through a delegate, call `go_selector_callback`), the user page (one example line per
5.6 reason), `CLAUDE.md` (one sentence in the route paragraph). Nothing under `src/call_graph*`,
`src/resolution*` or `src/ast*` changes (global constraints; every item below is already `pub` or
`pub(crate)`).

```rust
// src/framework_entries/routes/go_method_value.rs: DESIGN 5.6, read only by the route table
/// M3 first (`pkg.F`, method expressions), then M1 and M2. `fn_node` is the enclosing
/// `function_declaration` or `method_declaration`; `selector` is the callback argument.
pub(super) fn go_selector_callback(parsed: &ParsedFile, fn_node: Node<'_>, selector: Node<'_>,
    file: &str, call_graph: &CallGraph) -> CallbackValue;    // Resolved(FunctionId) | Unresolved { text, reason }
/// M1: (a) enclosing receiver via `Language::go_receiver_var` and the receiver parameter's type,
/// (b) `call_graph::go::receiver_index::classify_go_receiver_expanded_with_partition` +
/// `screen_go_receiver_prerequisites` with a `GoReceiverCtx` at the selector's bytes, a `GoReceiverFacts`
/// borrowed from `CallGraph` fields and an empty field-target map, `ReceiverRecoveryConfig::default()`,
/// (c) hops by `call_graph::go::receiver_index_visibility::resolve_go_struct_field_owner`,
/// (d) one `short_var_declaration` read from the syntax.
fn receiver_owner(parsed: &ParsedFile, fn_node: Node<'_>, operand: Node<'_>, file: &str,
    call_graph: &CallGraph) -> Result<(String, GoOwnerIdentity), Reason>;
/// M2: `CallGraph::go_concrete_receiver_route`, then the package-dir and
/// `owner_partition::exact_declaration_visibility` filter over `CallGraph::methods` (or
/// `promoted_aliases`, or `promoted_snapshot::consult_promoted_snapshot`); exactly one target.
fn method_target(call_graph: &CallGraph, static_type: &str, owner: &GoOwnerIdentity, method: &str,
    file: &str) -> Result<FunctionId, Reason>;
```

- [ ] Enable `w2-method-value` (gm01 to gm19 and gn03); RED (S4's binary gives `member_handler` on every
      row but gm19, whose `wrapper_call` already holds there; gm19 pins that a wrapped method value gains a
      delegate only through stage D's forwarder list). Tests in `go_method_value_tests.rs`, written first,
      each with a negative twin, building a `CallGraph` from in-memory Go files the way
      `src/call_graph/go_receiver_typing_tests.rs` does:
      `method_value_on_receiver_field_resolves` (gm01; twin: the same field typed by an interface),
      `same_named_controllers_in_two_packages_stay_apart` (gm02), `method_value_on_local_bound_to_a_field_resolves`
      (gm03; twin: the local declared twice), `method_value_on_composite_local_resolves` (gm04),
      `method_value_on_typed_parameter_and_package_var_resolves` (gm05, gm06),
      `value_and_pointer_receiver_methods_both_resolve` (gm07), `promoted_method_resolves_only_through_the_index`
      (gm08; twin: the name supplied by an embedded interface field, `method_value_interface`),
      `interface_typed_receiver_is_counted_never_resolved`
      (gm09, gm10), `shadowed_receiver_is_refused` (gm11, gm12), `call_result_and_deep_receivers_are_unproven`
      (gm13), `func_typed_field_is_not_a_method` (gm14), `package_function_and_method_expression_are_counted` (gm15),
      `external_receiver_type_is_method_value_external` (gm16), `embedded_pointer_hop_refuses` (gm17),
      `method_value_agrees_with_call_resolution` (gm18: for receiver forms (b) and (c), the method value's
      target equals `CallGraph::resolve_call_site_full`'s single Exact target for `X.M(…)` at the same place),
      `method_declared_in_two_build_profiles_is_method_unproven` (the outcome of the visibility filter for
      `//go:build linux` and `windows` twins; stage D confirms it). RED, on the first commit (rows enabled,
      tests written, `go_selector_callback` returning `member_handler`), and GREEN, through memguard:
      `cargo test -p prism-review --test cli routes_matrix_test::` and
      `cargo test -p prism --lib framework_entries::routes::go_method_value`, then the other Stage I commands.
- [ ] Measure on `apache-answer` (and `gin`): terminal callbacks by status and reason beside DESIGN 5.6's
      census table; every refused method value of an admitted shape is classified by a reader (`g/s4b/`).
      No-regression gate (in particular `call-stats` identical: this slice must not move call resolution).
      PR body: the counts, the classified refusals, sizes of the two new files.

### Slice S5: net/http and chi, closing wave 2 (2.5 days)

Fifth: it reuses S4's Go module, S4b's method-value lookup and the handler identity; chi's `Route` closure
reuses the mount graph.

**Files:** modify `routes/go.rs`, or create `src/framework_entries/routes/go_nethttp.rs` (declared in
`routes.rs`) if `go.rs` would pass 600 weighted lines; `routes/tests.rs`; docs.

- [ ] Enable `w2-nethttp`; RED. Tests: `method_in_pattern_is_split_from_the_path`,
      `plain_pattern_accepts_any_method`, `chi_route_closure_composes`,
      `nethttp_method_value_handler_resolves` (nh01: `h.create` on a composite local, through
      `go_selector_callback`). GREEN.
- [ ] Wave 2 gate (DESIGN 7): schema oracle on `apache-answer`, with its `@Router` bindings; the Go handler
      binding row on `apache-answer` and `miniflux-v2`; reconciled completeness against the ledger on
      `apache-answer`, `miniflux-v2`, `prometheus`, `gin`, chi's ledger sites included; no-regression gate
      with session time, table time and peak RSS.

### Slices S6 onward: wave 3, one framework per slice (2 to 3 days each)

In stage M's order. Each: enable `w3-<framework>`; RED; `src/framework_entries/routes/<framework>.rs`
declared in `routes.rs` (and `receivers.rs`/`handler.rs` parts for its language if no earlier slice made
them); one test per rule stage D fixed; GREEN; reconciled completeness on its corpora; schema oracle on
`calcom-cal.com` when NestJS is built; no-regression gate. Object-literal tables (DESIGN 5.3): tests
`grounded_table_consumer_lists_rows` and `ungrounded_table_is_a_candidate_without_credit` (ol05 to ol07,
lane-written positives and same-shape negatives).

### Slice SH: Hapi plugin adapter (2.5 days)

Owner decision 7 (2026-11-24): built whatever the census count, right after the object-literal table slice,
whose row reader it uses; added after the design review, its rules (DESIGN section 11) are settled at stage D
with the rest. Its goal is that routes registered with `server.route` inside plugin `register(server, options)`
functions, the shape of lane 5's `pilot-producer`, are proven records rather than `table_consumer_unproven`
candidates.

**Files:** create `src/framework_entries/routes/hapi.rs` (DESIGN 11.2; aim under 600 weighted lines) and
`src/framework_entries/routes/hapi_tests.rs` (declared `#[cfg(test)] mod` from `hapi.rs`); modify
`routes.rs` (`mod hapi;`, and the table entry point calls `hapi_routes`), `routes/mounts.rs` (the Hapi join),
`routes/receivers.rs` (a plugin's server parameter as a router), the object-literal table module (its row
reader made `pub(super)` if it is not), `tests/fixtures/routes/rows.jsonl` (hp01 to hp22),
`crates/prism-review/tests/cli/routes_matrix_test.rs` (`ENABLED_GROUPS` gains `w3-hapi`), the user page (one
example line per Hapi reason), `CLAUDE.md` (one sentence in the route paragraph),
`docs/features/language-coverage/js-ts-known-gaps.md`. If wave 1 was skipped, this slice also adds
`src/ast/js_router_binding.rs` (`js_ts_unique_declarator`) as S3 describes. Nothing under `src/call_graph*` or
`src/resolution*` changes; the items it reads (`ParsedFile::js_ts_site_binding`, `CallGraph::import_bindings`,
`CallGraph::js_ts_exports`, `call_graph::js_ts_relative_module_candidates`,
`frameworks::js_ts::call_classify::collect_js_ts_call_nodes`, `anon_owner::identity`) are already `pub` or
`pub(crate)`.

```rust
// src/framework_entries/routes/hapi.rs: DESIGN section 11, read only by the route table
/// Roots, registrations and mounts, plugin objects, server parameters, route rows and candidates, in that
/// order; records carry framework "hapi" and are composed by `mounts::compose` with the other frameworks'.
pub(super) fn hapi_routes(files: &BTreeMap<String, ParsedFile>, call_graph: &CallGraph, table: &mut RouteTable);
/// Root: `Hapi.server(…)`, `new Hapi.Server(…)`, `Hapi.Server(…)` on a proven `@hapi/hapi` (or `hapi`) import.
fn hapi_roots(parsed: &ParsedFile, file: &str, call_graph: &CallGraph) -> Vec<RouterId>;
/// Plugin object: an object literal with `register` and `name` or `pkg`, in place or named by an export.
fn plugin_object(files: &BTreeMap<String, ParsedFile>, call_graph: &CallGraph, file: &str,
    item: Node<'_>) -> Result<PluginRef, Reason>;
/// The server parameter: the plain, unwritten first formal of `register`.
fn server_parameter(parsed: &ParsedFile, file: &str, register: Node<'_>) -> Result<RouterId, Reason>;
// Reason gains (stage D may rename): HapiPluginUnproven, HapiPluginUnregistered, HapiRegistrationUnproven,
// HapiServerParamUnproven, HapiRouteNotLiteral, HapiHandlerObject, HapiHandlerConflict
```

Rows (group `w3-hapi`; "D" is settled at stage D):

| Rows | Input | Expect |
|---|---|---|
| hp01–hp02 | plugin `{ name: 'users', register: async (server, options) => { server.route([{ method: 'GET', path: '/users', handler: list }, { method: 'POST', path: '/users', config: { handler: create } }]) } }` registered by `await server.register(plugin)` on `const server = Hapi.server()`, `const Hapi = require('@hapi/hapi')`; the single-object form `server.route({ method: 'GET', path: '/x', options: { handler: h } })` | two records, full `/users`, `list` and `create` resolved; one record, full `/x`, `h` resolved |
| hp03 | hp01 registered with `server.register(plugin, { routes: { prefix: '/api' } })` | full `/api/users` |
| hp04–hp06 | `exports.plugin = { name, register }` in `users.js` and `server.register(require('./users'))`; `module.exports = { name: 'u', register }` with a module-level `async function register(server, options)` and `server.register({ plugin: require('./users'), routes: { prefix: '/v1' } })`; ESM `export const plugin = {…}` and `import { plugin as users } from './users.js'` | full paths through each export form |
| hp07 | `method: '*'`; `method: ['GET', 'POST']` | `Any`; `Known` GET, POST |
| hp08 | one plugin registered under `/v1` and `/v2` | two full paths on one record |
| hp09 | plugin A registered under `/a` calls `server.register(B, { routes: { prefix: '/b' } })` on its own parameter | B's `/x` at `/a/b/x` per truth |
| hp10 | `routes: { prefix: base }` with `base` a variable; `path: PATH` | suffix `variable_prefix`; dynamic |
| hp11 | handlers: an inline arrow; `controller.list`; `wrap(h)`; `{ directory: { path: '.' } }`; `options.pre: [{ method: check }]` | anonymous; `member_handler`; `wrapper_call`; `hapi_handler_object`; D |
| hp12 | `const server = makeFake(); server.route([{ method: 'GET', path: '/x', handler: h }])` in a file with no Hapi import | candidate `table_consumer_unproven`, no credit |
| hp13 | `fastify.register(async (server, opts) => { server.route({ method: 'GET', url: '/x', handler: h }) }, { prefix: '/p' })` with `fastify = require('fastify')()` | no `hapi` record |
| hp14–hp15 | `module.exports = { register(server) { server.route(…) } }` (no `name` or `pkg`); `module.exports = function register(server, options) { server.route(…) }` passed to `container.register(…)` | candidates, `hapi_plugin_unproven` |
| hp16 | a plugin object passed only to `registry.register(plugin)`, `registry` not a Hapi server | candidates, `hapi_plugin_unregistered` |
| hp17–hp18 | inside `register`, `const server = other; server.route(…)` in an inner block, and a nested callback `(server) => server.route(…)`; `register({ route }, options)` and `register(server) { server = wrap(server); … }` | `hapi_server_param_unproven` |
| hp19 | `server.route(require('./routes'))` with `routes.js` exporting an array literal of route objects | D (one level through a proven binding, as ol02) |
| hp20 | a row with both `handler` and `config.handler`; a row with no handler | `hapi_handler_conflict` twice |
| hp21 | legacy `exports.register = function (server, options, next) {…}` with `exports.register.attributes = { name }` | D (admitted, or an accepted loss) |
| hp22 | plugins registered only by `for (const p of plugins) await server.register(p)` | candidates, `hapi_plugin_unregistered`; the count goes to stage D (DESIGN 11.2, the weaker rule) |

- [ ] Rows: hp01 to hp22 are written at stage T with the other rows, so that stage D sees them (the slice's
      2.5 days include them; `truth_source` `hand` unless an offline install of a pinned `@hapi/hapi`
      succeeds, then `executed` through a `truth/hapi_routes.mjs` adapter that instruments `server.route` and
      `server.register` and reads `server.table()`; DESIGN 11.3). Enable `w3-hapi`; RED (the previous slice's
      binary lists these rows as candidates or not at all). Tests in `hapi_tests.rs`, written first, each with
      a negative twin: `plugin_route_table_rows_are_records` (hp01, hp02), `registration_prefix_composes`
      (hp03), `exported_plugin_shapes_ground_through_imports` (hp04 to hp06), `star_and_array_methods` (hp07),
      `plugin_registered_twice_yields_both_paths` (hp08), `nested_plugin_prefixes_compose` (hp09),
      `dynamic_prefix_and_path` (hp10), `handler_forms_follow_the_callback_rules` (hp11),
      `lookalike_route_on_a_non_hapi_object_is_a_candidate` (hp12), `fastify_register_is_not_a_hapi_plugin`
      (hp13), `register_function_that_is_not_a_plugin_is_a_candidate` (hp14, hp15),
      `plugin_not_registered_on_a_hapi_server_is_unregistered` (hp16, hp22),
      `shadowed_or_written_server_parameter_is_refused` (hp17, hp18),
      `handler_spelled_twice_or_missing_refuses` (hp20). GREEN, through memguard: `cargo test -p prism-review
      --test cli routes_matrix_test::` and `cargo test -p prism --lib framework_entries::routes::hapi`, then
      the other Stage I commands.
- [ ] Gate (DESIGN 11.4): reconciled completeness over the public Hapi corpus's ledger sites; on
      `pilot-producer`, locally by a Claude seat, the share of route entries in plugin `server.route` calls
      that are records with a proven receiver and at least one exposure, against stage D's floor, every other
      entry with a reason classified by a reader, aggregate counts only (outputs under `~/prism-evidence/`,
      never in this repository, an issue or a PR body beyond the aggregates); no-regression gate. PR body:
      the public corpus summary, the `pilot-producer` aggregates, sizes of the two new files.

## Close-out (1 day)

- [ ] `CLAUDE.md` Architecture paragraph (route table, listing, statuses, reasons, out of model) and the
      user page final. Comments on #600 (records exist; the request-source follow-up and its evidence) and
      PR #606 (the CLI listing exists; `nav_routes` is a DESIGN section 8 follow-up).
- [ ] Note on lane 5's issue, for the waves that shipped: the JSON line shape, `normalized` for matching
      operations, DESIGN 8's binding rule (a `full` exposure with a resolved callback or delegate; callbacks
      ordered), the measured bindings and losses, Go registration records if wave 2 shipped (adapter input,
      not a wired adapter). The same position semantics on #600. File DESIGN section 8's six follow-ups with
      their evidence, and the stage D accepted losses that have a fix.
- [ ] Run `truth/check.sh`; refresh `HANDOFF.md` and `LEDGER.md`; offload raw evidence; delete `target/`.

## Acceptance for the lane

Each built wave passed its census gate; the matrix holds for every enabled group; schema-oracle precision on
in-scope `full` exposures ≥ 0.95 on each schema-bearing corpus of a finished wave, every difference and
binding contradiction classified; reconciled completeness (no unexplained ledger site, recall at or above the
stage D floor, nonempty yield), the Go handler binding row when wave 2 is built, and no-regression hold on
each slice's final head; both reviewers approved
each slice; follow-ups filed.

## Estimates (working days)

| Stage or slice | Days |
|---|---|
| M: issue, census and ledger (with Go callback shapes), wave decisions, oracle | 2.75 |
| T and E: fixtures (with the `w2-method-value` rows), instrumented runner, test, enumeration | 4.75 |
| D | 1.5 |
| S1 records, listing, composer | 3 |
| S2 Flask and FastAPI; S3 Express and wave 1 gate | 3.5 + 4.5 |
| S4 gin; S4b Go method values; S5 net/http, chi and wave 2 gate | 3 + 2.5 + 2.5 |
| S6+ wave 3: object-literal tables, then each admitted framework | 2.5, then 2.5 each |
| SH Hapi plugin adapter (owner decision 7) | 2.5 |
| Close-out | 1 |
| Total, waves 1 and 2 plus object-literal tables and the Hapi plugin adapter | 34 |
