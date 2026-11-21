# Lane 12: routes as a first-class fact — implementation plan

> **For agentic workers:** this plan is run by an orchestrator under `../ORCHESTRATION.md`. Each task is
> dispatched to an implementer (Sonnet-5.5, high) with its section as the brief. Steps use checkbox syntax.

**Goal:** `prism nav routes` lists a repository's web routes (methods, path as written, composed path when
every prefix is static, handler identity or why it is unresolved, registration site, framework) and its
mounts: Flask, FastAPI and Express first, then Go, then the wave 3 frameworks the census picks. Nothing the
extractor sees is dropped silently.

**Architecture:** a route table computed on demand from parsed files and the call graph
(`src/framework_entries/routes*`), composed through a mount graph, listed by `src/navigation/routes.rs` and
one CLI arm. No cache change; `FrameworkEntryRecord` and the `framework_entry` navigation edges untouched.

**Tech stack:** Rust (core and prism-review crates); Python 3 (census, schema oracle); uv, Node and Go for
the truth runner.

**Spec:** `DESIGN.md` here (records 5.1, composition 5.2, waves 5.3, listing 5.4, layout 5.5, measurement 6,
gates 7, follow-ups 8, owner decisions 9).

## Global constraints

- Worktree `~/code/prism-routes` (ORCHESTRATION section 4). Evidence `~/prism-evidence/routes/` (`m/` census
  and oracle, `truth/` venv and runner logs, `g/` gate runs, `LEDGER.md`, `HANDOFF.md`). Branches
  `test/routes-matrix`, then `feat/routes-s1` … `feat/routes-s<n>`.
- Must not change: `src/call_graph.rs` (5,653 weighted lines: not one line), `src/navigation/queries.rs`
  (2,502.5), `src/navigation/types.rs`, `FrameworkEntryRecord`, `framework_entries::apply`, `CACHE_VERSION`
  (`src/cpg_cache.rs`), `NAV_CALL_EDGE_CACHE_VERSION` (`src/navigation/call_edge_cache.rs`), taint code,
  `crates/prism-mcp`, and the tests `tests/navigation/framework_entry_test.rs`,
  `src/call_graph/framework_entry_tests.rs` (the stale inline-arrow test stays: entry edges are a follow-up).
- Allowed edits to existing files: `pub(crate)` on the helpers each slice names (no behaviour change); one
  `pub mod routes;` line in `src/framework_entries/mod.rs` and in `src/navigation/mod.rs` (1,148.5 lines; the
  PR body says that line is the only addition); `NavQuery::Routes` in `crates/prism-review/src/cli.rs`; a
  `run_nav` arm of at most 10 lines in `crates/prism-review/src/main.rs` (990.5 lines; rendering is in core).
- Sizes: `python3 scripts/file_size.py <files>`; new files aim for 600 weighted lines. Only lane-written
  fixtures are executed, never a corpus (owner decision 5); fixture apps export their app and never listen.
- The census may count the object-literal table shape on lane 5's `pilot-producer` locally, Claude seat,
  aggregate counts only (ORCHESTRATION section 9). No row, fixture or text derives from it.

## Review focus

1. A router mounted from another file through a default export (`export default router`, imported under
   another name) or a package import (`from . import auth`, `auth.bp`) composes to `full`. Test
   `mount_through_default_export_and_package_import_composes` (S3).
2. The join (`"/items"+"/"`, `""+"/x"`, `"/a/"+"/b"`) keeps one `/` and agrees with executed truth. Test
   `join_keeps_one_slash_and_matches_rows` (S3).
3. A router mounted twice, or a variable prefix on the chain, never yields `full` (anti-goal). Test
   `multiply_mounted_or_dynamic_prefix_is_never_full` (S3).
4. Member, call, array, spread and missing handler arguments are listed with a reason; today
   `express_route_candidates` drops them (`src/framework_entries/express.rs:139`). Test
   `every_seen_registration_is_counted` (S2).
5. A Go file importing gin without `*gin.Context` (`parsed.framework()` is not gin there,
   `src/frameworks/gin.rs:16`) still extracts; gin handler identity equals `CallGraph::go_registrations`.
   Tests `gin_router_file_without_context_type_extracts`, `gin_handler_identity_matches_callback_registration` (S4).

## Stage M: measure first (2 days)

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

### Task M-2: census

**Files (evidence):** `m/census.py`, `m/census.json`, `m/CENSUS.md`; census seat (Sonnet-5.5, medium),
prediction first. `census.py <corpus>...` skips `node_modules`, `vendor`, `dist`, `build`, `.git` and counts
each shape twice, by regular expression and by syntax (the installed Python tree-sitter bindings
`tree_sitter_python`, `_javascript`, `_typescript`, `_go`, written independently of prism), split test /
non-test (`test`, `tests`, `__tests__`, `*_test.go`, `*.test.*`, `*.spec.*`). Per framework (DESIGN 6):
shapes (Flask `@x.route`, `@x.<verb>`, `add_url_rule`; FastAPI verbs, `api_route`, `add_api_route`,
`websocket`; Express verbs, `.route(p).<verb>`, `use`; gin verbs, `Handle`, `Any`, `Group`; net/http with
and without a method in the pattern; chi verbs, `Route`, `Mount`; wave 3: NestJS decorators, Fastify, Koa,
Django `path`/`re_path`/`include`, DRF routers, `[{ method, path, handler }]`); handler form (named, inline,
wrapped, member, imported); prefix source (none, literal on the router, literal at a mount in another file,
variable, configuration); and each shape whose two counts differ by over 5%, with five examples.

### Task M-3: wave decisions

- [ ] Wave go (DESIGN section 7): a wave proceeds with at least 200 registrations of its frameworks (syntax
      count, test and non-test together) on the corpora; otherwise it is skipped and reported.
- [ ] Wave 3: each candidate with at least 200 joins, in descending count order; object-literal tables join
      regardless (owner decision 3), last unless their count is higher. Candidates may also be counted on
      other `bench-repos` corpora that use them (name and commit recorded). Below 200: recorded, not built.
- [ ] Post counts and decisions on the issue; the stage T PR adds the wave 3 list to `DESIGN.md`.

### Task M-4: schema oracle

**Files (evidence):** `m/schema_oracle.py`, `m/test_schema_oracle.py` (`python3 -m unittest discover -s m`).

`schema_oracle.py --schema <file> --listing <routes.jsonl> --out <dir>`. Operations: the method keys
(`get put post delete options head patch trace`) of each `paths` entry, a relative `$ref` path item loaded
from its file; base path from Swagger `basePath` or the path of a relative `servers[0].url` (absolute URLs
ignored, recorded). Normalisation, both sides: upper-case method; schema `{name}` → `{}` (the listing's
`normalized` already is); collapse `//`; drop a trailing `/` except `/`; drop a query. `full` routes match
exactly; `suffix` routes match a schema path ending with them, reported apart. Precision = matched `full` /
all `full`; recall = operations matched by either / all. Differences go to `misses.jsonl`/`extras.jsonl`,
pre-classed `dynamic_prefix` (a suffix match exists), `unsupported_shape` (the literal occurs in code, no
record lists it), `schema_drift` (no occurrence), `non_api_route` (static, UI, health, metrics), else
`unclassified`; a Sonnet-5.5 reader confirms each from the code, and no gate report keeps `unclassified`.
Unit tests: one toy row per class, a base path, a `$ref` path item. Then record the denominators by running
it with an empty listing on the three schema files.

## Stage T and E: the matrix (3.5 days)

### Task T-1: fixtures, rows, truth runner, data-driven test

**Files:**
- Create: `tests/fixtures/routes/<row>/` (one small application per row; `go.mod` in Go rows),
  `tests/fixtures/routes/rows.jsonl`, `tests/fixtures/routes/README.md`,
  `tests/fixtures/routes/truth/{run_truth.py,flask_routes.py,fastapi_routes.py,express_routes.mjs,gin_routes.go,truth.jsonl,check.sh}`,
  `crates/prism-review/tests/cli/routes_matrix_test.rs`
- Modify: `crates/prism-review/tests/cli/main.rs` (`mod routes_matrix_test;`)

`rows.jsonl` fields: `id`; `dir`; `group` (`w1-decorators`, `w1-express`, `w1-mounts`, `w2-gin`,
`w2-nethttp`, `w3-<framework>`); `class`; `truth_source` (`executed` or `hand`); `entry` (executed rows:
`module:attr`, `module:factory()`, or the JS file exporting the app); `expect` (route objects in DESIGN 5.4's
JSON shape, only the fields the row pins: `methods`, `path`, `full_path`, `handler`, `wrapper`, `site`,
`router`); `expect_mounts`; `accepted_loss` (optional reason); `ignore` (optional issue number). After stage
D the implementer updates `expect` in the same commit as the design; truth never changes by hand.

```rust
// crates/prism-review/tests/cli/routes_matrix_test.rs: drives the binary, so it names no Rust item that
// does not exist yet, and compares nothing while no group is enabled
const ENABLED_GROUPS: &[&str] = &[];  // each slice adds its groups in its first commit
#[test] fn routes_matrix_rows_are_well_formed();  // unique ids, dirs exist, closed value sets; an executed
                                                  // row's truth.jsonl entry agrees with its `expect`
#[test] fn routes_matrix_matches_rows();  // per enabled, non-ignored row: `crate::prism_bin()` runs
    // `nav --no-cache routes --repo tests/fixtures/routes/<dir> --mounts --format json`; projected route and
    // mount lines equal `expect`/`expect_mounts` exactly; the summary keeps the completeness invariant
```

Truth runner (orchestrator only, never `cargo test`): one child process per row on a temporary copy, 10 s
limit; per route it records the framework's methods, composed path, registration line, handler name or
`anonymous`. Flask and FastAPI: `uv venv ~/prism-evidence/routes/truth/venv; uv pip install --offline flask
fastapi` (both cached; versions recorded); `app.url_map.iter_rules()` with `view_functions`, and `app.routes`
with `endpoint.__name__`, `__code__.co_firstlineno`. Express: `npm install --offline --prefix
~/prism-evidence/routes/truth/node express@4.22.3` (cached); walk `app._router.stack`, taking a mount prefix
only from Express 4's literal-prefix layer pattern (otherwise the row is `hand`); lines from wrapping the
verb, `route` and `use` methods before loading `entry`; `.ts` rows use erasable TypeScript under Node 24's
type stripping. gin and chi: `engine.Routes()` / `chi.Walk` if the module can be fetched (it is not in the
module cache; offline the rows are `hand`); net/http has no route table: `hand`. `check.sh` re-runs and
diffs `truth.jsonl`, before stage D and before each merge. The runner beats a seed expectation; note it.

Seed rows ("Expect" is the hypothesis; "D" is settled at stage D; other wave 3 rows are written only for
the frameworks the census admitted):

| Rows | Input | Expect |
|---|---|---|
| fl01–fl04 | `@app.route("/items", methods=["GET","POST"])`; `@app.get("/x")`; `@app.route("/x")`; two decorators on one def | full; methods as written; fl03 implicit methods D; two records |
| fl05–fl10 | `Blueprint(url_prefix="/auth")` + `@bp.route("/<int:id>/u")` registered in another file; prefix only at `register_blueprint(bp, url_prefix="/v1")`; both; `url_prefix=prefix`; registered twice; nested `/p` + `/c` | `/auth/<int:id>/u` (normalised `/auth/{}/u`); `/v1/…`; override per truth; suffix; suffix; `/p/c/x` |
| fl11–fl13 | `add_url_rule("/r", view_func=h)`; `add_url_rule("/", endpoint="i")`; `view_func=V.as_view("i")` | resolved; unresolved, D; unresolved, D |
| fl14–fl19 | `@app.route(PATH)`; `@self.app.route("/x")`; `url_prefix="/a"` + `"/"`; `methods=M`; routes in `create_app()`; `app = flask.Flask(…)` | dynamic; D (seen or loss); per truth; D; full; full |
| fa01–fa06 | `@app.get("/items/{id}")`; `APIRouter(prefix="/items")` + `include_router(items.router)`; both prefixes; `@app.api_route(…, methods=[…])`; `add_api_route("/x", h, methods=["PUT"])`; `@app.websocket("/ws")` | full; `/items/` per truth; `/admin/items/…`; as written; resolved; D |
| fa07–fa13 | `@app.get(app.url)`; included twice; nested include; `include_router(r, prefix=P)`; `from .routers import users as u` + `include_router(u.router)`; `app.mount("/s", StaticFiles(…))`; a non-router included | dynamic; suffix; full; suffix; full; D; D |
| ex01–ex08 | `app.get('/x', named)`; inline arrow; `asyncHandler(async (req, res) => {})`; `asyncHandler(named)`; `wrap(a, b)`; `site.index` with `const site = require('./site')`; `import { list } from './users'`; `const h = asyncHandler(…)` then `get('/', h)` | resolved; `<anon@L:C>`; anonymous + wrapper; resolved + wrapper; D; resolved or `member_handler`; resolved; D |
| ex09–ex13 | `get('/', validate('r'), h, respond)`; `app.route('/r').get(h1).post(h2)`; `app.all`; template literal with/without `${}`; regex or array path | D; two records `/r`; D; literal / dynamic; D |
| ex14–ex17 | `(app) => { app.get(…) }`; `:id?` and `*`; `app.use(mw)`; `app.options`/`app.head` | unresolved, D; normalisation D; not a route; D |
| ex18–ex26 | TypeScript `import express`; same-file `app.use('/api', r)`; `app.use('/v1', require('./v1'))`; `module.exports = router` + `require`; `export default router` imported elsewhere; `app.use(PREFIX, r)`; mounted twice; nested `router.use('/sub', sub)`; `app.use('/u', r)` + `r.get('/', h)` | as JS; full; full; full; full; suffix; suffix; full; per truth |
| gn01–gn06 | `r := gin.Default(); r.GET("/x", list)`; `g.POST("/y", func(c *gin.Context){})`; `a.ctrl.Get`; `register(r.Group("/api"))` with `g *gin.RouterGroup`; two function levels; `r.Group(conf.Base + "/api")` | resolved (callback-registration id); D (no Go anonymous identity); D; full within the D bound; suffix beyond; suffix |
| gn07–gn10 | `r.Handle("GET", "/x", h)`, `r.Any`; `r.GET("/x", auth(), h)`; gin import without `*gin.Context`; `r.Group("", mw)` | D; D; extracted; full |
| nh01–nh08 | `mux.HandleFunc("POST /v1/u", h.create)`; `http.HandleFunc("/w", func(w, r){})`; `mux.Handle("/x", v)`; `"GET /x"` (serves HEAD too); `"example.com/x"`; `{p...}`, `{$}`; `HandleFunc(versionPath, h)`; `root.Handle(b+"/", http.StripPrefix(b, a))` | POST; any, handler D; D; D; D; normalisation D; dynamic; D (mount?) |
| ch01–ch04 | `r.Get`; `r.Route("/api", func(r chi.Router){…})`; `r.Mount("/adm", adm())`; `r.With(mw).Get` | full; full; D; full |
| ol01–ol04 | `route([{ method: 'GET', path: '/x', handler: h }])`; exported array used in another file; inline handler; `method: ['GET','POST']` | listed; framework label D |

- [ ] Write fixtures, rows, runner and test; run the runner; commit `truth.jsonl`. One reviewer
      (test-only). CI green.

### Task T-2: enumeration

- [ ] Tiers on `rows.jsonl` (ORCHESTRATION section 6). Axes: registration form; receiver (bare, attribute
      chain, call result, parameter, imported, aliased, rebound, shadowed); path (literal, concatenation,
      f-string or template, constant, variable, regex, array, empty, trailing slash, parameter syntaxes);
      methods (list, tuple, variable, any, websocket, implicit); handler (named, imported, default, namespace
      member, class method, inline, wrappers, nested wrappers, middleware chain, wrapper result, class-based
      view, missing); mounts (constructor, mount, both, override, variable, configuration, nested, twice,
      cross-file by import, `require` or package import, cycle, non-router child); scope (module, factory,
      nested callable, `if DEBUG:`, loop over data); module forms (CJS, ESM, TypeScript, Python packages,
      multi-file Go packages, `_test.go`). Out of model shapes are `accepted_loss` rows. Runner truth first.

## Stage D: design of record (1 day)

- [ ] Inputs: DESIGN 5's tentative rules, `rows.jsonl` with `truth.jsonl`, `CENSUS.md`, the oracle
      denominators, the D cells as accepted-loss candidates. The design seat amends DESIGN 5.1 to 5.3 and
      fixes, per framework: which calls and decorators are registrations or mounts; method derivation; which
      argument is the handler among several; admitted wrappers; how imported and member handlers resolve (a
      handler argument is a value reference, not a call site); Go function literals and method values;
      normalisation of optional, wildcard, regex and host forms; the closed `UnresolvedReason` list (with
      `member_handler`); the bound for groups passed as parameters; accepted losses. The implementer updates
      `rows.jsonl`. Both reviewers; cap 2 rounds.

## Stage I: slices

The orchestrator pastes this paragraph and the no-regression gate into every slice brief. Each slice: its
first commit adds its groups to `ENABLED_GROUPS` (`crates/prism-review/tests/cli/routes_matrix_test.rs`) and
shows `routes_matrix_matches_rows` RED; ORCHESTRATION section 5 items 1 to 5 (item 5 because
`src/navigation/` is touched: Tier-A matrix and quick, regressions in the PR body), 8, 9, 10; two reviewers,
cap 3 rounds. Commands, through memguard:
`cargo test -p prism-review --test cli routes_matrix_test::`, `cargo test -p prism --lib framework_entries::routes`,
`cargo test --test navigation routes_test::`.

**No-regression gate, every slice, final head:** release builds of merge base and head; on the slice's
corpora `prism nav --no-cache call-stats --repo <c>` identical, and `nav --no-cache callers --format json`
identical for the head listing's first 20 resolved handlers per corpus; `cargo test -p prism-mcp --test mcp
golden_transcript_test` passes unre-recorded; `git diff <base>..HEAD -- src/call_graph.rs src/cpg_cache.rs
src/navigation/{queries,types,call_edge_cache}.rs` empty.

### Slice S1: records, listing, Flask and FastAPI decorators (3 days)

First because the handler is the decorated function: review is about the record and listing, not resolution.

**Files:** create `src/framework_entries/routes.rs`, `src/framework_entries/routes/{python,tests}.rs`,
`src/navigation/routes.rs`, `tests/navigation/routes_test.rs`, `docs/features/query-layer/navigation/routes.md`;
modify `src/framework_entries/mod.rs`, `src/navigation/mod.rs`, `tests/navigation/main.rs`
(`mod routes_test;`), `crates/prism-review/src/{cli,main}.rs`, `src/frameworks/python/{flask,fastapi}.rs`
(`decorator_receiver_and_method`, `first_decorator_target` → `pub(crate)`), the navigation docs
`README.md` (a row), `CLAUDE.md`.

```rust
// src/framework_entries/routes.rs: declares `mod python;` (S1), `mod express; mod handler;` (S2),
// `mod mounts;` (S3), `mod go;` (S4), `#[cfg(test)] mod tests;`. All records derive
// Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize.
pub struct RouterRef { pub file: String, pub binding: String }
pub enum PathStatus { Full(String), Suffix(String), Dynamic }
#[serde(rename_all = "snake_case")]
pub enum UnresolvedReason { MemberHandler /* , the rest of stage D's closed list */ }
pub enum RouteHandler { Resolved(FunctionId), Anonymous(FunctionId),
    Unresolved { text: String, reason: UnresolvedReason } }
pub struct RouteRecord { pub framework: String, pub methods: Vec<String>, pub path: String,
    pub router: Option<RouterRef>, pub full_path: PathStatus, pub handler: RouteHandler,
    pub wrapper: Option<String>,   // DESIGN 5.1: "records the wrapper's name"
    pub site: FrameworkEntrySite }
pub struct MountRecord { pub parent: Option<RouterRef>, pub child: RouterRef,
    pub prefix: Option<String>, pub site: FrameworkEntrySite }
#[derive(Default)] pub struct RouteTable { pub routes: Vec<RouteRecord>, pub mounts: Vec<MountRecord>,
    pub registrations_seen: usize }
/// Takes the session's parts: this module sits below `navigation`.
pub fn routes(files: &BTreeMap<String, ParsedFile>, call_graph: &CallGraph) -> RouteTable;
/// `<int:id>`, `<id>`, `{id}`, `{id:path}`, `:id`, `{id...}` → `{}`; other forms per stage D.
pub fn normalize_path(path: &str) -> String;

// src/navigation/routes.rs (serde::Serialize rows: RouteRow = DESIGN 5.4 fields, `wrapper` skipped when
// None; MountRow = parent, child, prefix, site; RouteSummary = registrations_seen, routes, mounts and the
// BTreeMap<String, usize> counts by_framework, by_full_path, by_handler, by_reason)
pub struct RouteListing { pub routes: Vec<RouteRow>, pub mounts: Vec<MountRow>, pub summary: RouteSummary }
pub fn routes_listing(session: &NavigationSession) -> RouteListing; // routes(&session.repo.files, session.index.call_graph())
pub fn render(listing: &RouteListing, format: &str, include_mounts: bool) -> String;
```

JSON: DESIGN 5.4 route lines sorted by (normalised full path or suffix, first method, file, line); with
`--mounts` one `{"mount":{…}}` line each; last `{"summary":{…}}`. Text: `GET,POST /users/:id -> handler
(routes/users.js:12) [express] full`. CLI: `NavQuery::Routes { repo: PathBuf, format: String (default
"json"; json|text), mounts: bool }`.

In S1, `full_path` is `Full` on an application object (`Flask(…)`, `FastAPI(…)`) and `Suffix` on a
`Blueprint`/`APIRouter` (S3 composes). Decorator sites come from `python::python_route_candidates` (site
bytes, handler identity, framework); the decorator node is found from the site bytes and re-read for
receiver, method and arguments. `add_url_rule` and `add_api_route` are found by a scan over calls on
`flask::route_receivers` / `fastapi::route_receivers`. The Python string-literal helper is new (none exists).

- [ ] Enable `w1-decorators`; RED. Tests in `routes/tests.rs`: `normalize_path_rewrites_each_parameter_form`,
      `flask_decorator_reads_methods_and_path`, `python_registration_without_literal_path_is_dynamic`,
      `blueprint_route_is_suffix_before_composition`; in `routes_test.rs`:
      `listing_is_sorted_and_ends_with_summary`, `text_format_has_one_line_per_route`,
      `other_languages_list_nothing`. GREEN. No-regression gate. PR body: rows enabled, no-regression
      result, sizes of new files and of `main.rs`.

### Slice S2: Express registrations and handler forms (3.5 days)

Second because handler resolution is the riskiest rule and must reuse the entry records' binding rules.

**Files:** create `src/framework_entries/routes/express.rs`, `src/framework_entries/routes/handler.rs`
(shared handler-form classifier: bare name, inline, wrapper, member, imported); modify
`src/framework_entries/express.rs` (`express_call_match_kind`, `express_call_method_name`,
`is_js_ts_string_like_node`, `express_receiver_is_shadowed`, `enclosing_named_function_facts` →
`pub(crate)`), `routes.rs`, `routes/tests.rs`, `CLAUDE.md`, the user page,
`docs/features/language-coverage/js-ts-known-gaps.md`.

The site scan is new: every call from `collect_js_ts_call_nodes` that `express_call_match_kind` accepts,
whatever its arguments. A bare same-file name resolves by the entry records' rule
(`express::is_bare_binding_function`, exactly one match, local-shadow check); an inline function gets
`anon_owner::identity`; imported and member handlers follow stage D; a `.route(p)` chain takes `p` from the
inner call (`js_ts_literal_string_value`).

- [ ] Enable `w1-express`; RED. Tests: `every_seen_registration_is_counted` (member, call, array, spread
      and missing handler arguments each give a record; `registrations_seen == resolved + anonymous +
      unresolved`), `inline_handler_gets_its_anonymous_identity`,
      `wrapper_with_one_function_argument_records_wrapper`, `same_file_handler_matches_the_entry_record`
      (equal to the `CallGraph::framework_entries` handler for the same site),
      `shadowed_receiver_is_unresolved_with_reason`. GREEN. No-regression gate. PR body: the summary on
      `express` and `directus-directus` beside the census's handler forms.

### Slice S3: mounts and composition, closing wave 1 (3 days)

Third because composition needs both extractors and holds the anti-goal (no false `full`).

**Files:** create `src/framework_entries/routes/mounts.rs`; modify `routes/python.rs` (`register_blueprint`,
`include_router`, constructor `url_prefix=`/`prefix=`), `routes/express.rs` (`use(prefix, router)`),
`routes.rs`, `routes/tests.rs`, docs.

```rust
// src/framework_entries/routes/mounts.rs
pub(super) fn compose(routes: &mut [RouteRecord], mounts: &[MountRecord], roots: &BTreeSet<RouterRef>);
pub(super) fn join(prefix: &str, path: &str) -> String;   // exactly one `/` at the join
```

An imported router is identified through `CallGraph::imports` (file → local → module): JS/TS by
`call_graph::js_ts_relative_module_candidates` then `CallGraph::js_ts_exports[file].named` (`"default"` or
the name); Python by `call_graph::file_matches_module` over the indexed Python files, exactly one match. A
child that cannot be identified keeps the mounting file and its text and connects to nothing.

- [ ] Enable `w1-mounts`; RED. Tests: `mount_through_default_export_and_package_import_composes` (an
      `export default router` imported under another name and mounted with `app.use('/users', …)`, and a
      blueprint registered as `auth.bp` after `from . import auth`, both `full`);
      `join_keeps_one_slash_and_matches_rows` (`"/items"+"/"`, `""+"/x"`, `"/a/"+"/b"`);
      `multiply_mounted_or_dynamic_prefix_is_never_full`; `mount_cycle_terminates_as_suffix`. GREEN.
- [ ] Wave 1 gate: schema oracle on `directus-directus` (precision ≥ 0.95 of `full`, recall with every miss
      classified); completeness on the five wave 1 corpora (the invariant, and the census syntax count beside
      `registrations_seen`, differences classified); no-regression gate. Results in the PR body, raw in `g/`.

### Slice S4: Go gin (3 days)

Fourth: gin is the larger Go family (237 registrations in `apache-answer`) and reuses the mount graph.

**Files:** create `src/framework_entries/routes/go.rs`; modify `routes.rs`, `routes/tests.rs`, docs.

Routers: `gin.Default()`/`gin.New()` results, `Group` results (a mount), parameters typed `*gin.Engine`,
`*gin.RouterGroup` or `gin.IRoutes` within stage D's bound. A file qualifies by importing
`github.com/gin-gonic/gin` (`parsed.extract_imports()`), never by `parsed.framework()`. A bare identifier
handler takes the target of the `CallGraph::go_registrations` record whose `site.file` and
`site.start_byte` equal the argument's; none: unresolved with stage D's reason.

- [ ] Enable `w2-gin`; RED. Tests: `gin_router_file_without_context_type_extracts` (a file that takes
      `r *gin.RouterGroup` and never names `*gin.Context`); `gin_handler_identity_matches_callback_registration`
      (the route's `FunctionId` equals the `go_registrations` target for the same bytes);
      `group_passed_beyond_the_bound_is_suffix`. GREEN. No-regression gate on `apache-answer` and `gin`; PR
      body: the summary on both.

### Slice S5: net/http and chi, closing wave 2 (2.5 days)

Fifth: it reuses S4's Go module and handler identity; chi's `Route` closure reuses the mount graph.

**Files:** modify `routes/go.rs`, or create `src/framework_entries/routes/go_nethttp.rs` (declared in
`routes.rs`) if `go.rs` would pass 600 weighted lines; `routes/tests.rs`; docs.

- [ ] Enable `w2-nethttp`; RED. Tests: `method_in_pattern_is_split_from_the_path`,
      `plain_pattern_accepts_any_method`, `chi_route_closure_composes`. GREEN.
- [ ] Wave 2 gate: schema oracle on `apache-answer`; completeness on `apache-answer`, `miniflux-v2`,
      `prometheus`, `gin`; no-regression gate.

### Slices S6 onward: wave 3, one framework per slice (2 to 3 days each)

In stage M's order. Each: enable `w3-<framework>`; RED; `src/framework_entries/routes/<framework>.rs`
declared in `routes.rs`; one test per rule stage D fixed; GREEN; completeness on its corpora; schema oracle
on `calcom-cal.com` when NestJS is built; no-regression gate.

## Close-out (1 day)

- [ ] `CLAUDE.md` Architecture paragraph (route table, listing, statuses, reasons, out of model) and the
      user page final. Comments on #600 (records exist; the request-source follow-up and its evidence) and
      PR #606 (the CLI listing exists; `nav_routes` is a DESIGN section 8 follow-up).
- [ ] Note on lane 5's issue: the JSON line shape, `normalized` for matching operations, bind only on `full`,
      the Go registration adapter from wave 2. File DESIGN section 8's four follow-ups with their evidence,
      and the stage D accepted losses that have a fix.
- [ ] Run `truth/check.sh`; refresh `HANDOFF.md` and `LEDGER.md`; offload raw evidence; delete `target/`.

## Acceptance for the lane

Each built wave passed its census gate; the matrix holds for every enabled group; schema-oracle precision on
`full` ≥ 0.95 on each schema-bearing corpus of a finished wave, every difference classified; completeness
and no-regression hold on each slice's final head; both reviewers approved each slice; follow-ups filed.

## Estimates (working days)

| Stage or slice | Days |
|---|---|
| M: issue, census, wave decisions, oracle | 2 |
| T and E: fixtures, runner, test, enumeration | 3.5 |
| D | 1 |
| S1 records, listing, Python decorators; S2 Express handlers; S3 mounts and wave 1 gate | 3 + 3.5 + 3 |
| S4 gin; S5 net/http, chi and wave 2 gate | 3 + 2.5 |
| S6+ wave 3: object-literal tables, then each admitted framework | 2, then 2.5 each |
| Close-out | 1 |
| Total, waves 1 and 2 plus object-literal tables | 24.5 |
