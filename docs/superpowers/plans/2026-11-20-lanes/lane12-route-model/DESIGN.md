# Lane 12: routes as a first-class fact — design

Status: architecture final; the per-framework extraction rules are tentative until the census, the test
matrix and enumeration (PLAN stages M, T, E). Design review round 1 (2026-11-21) folded: one record per
registration with every composed path, router identity grounded at the use, every callback recorded,
gates reconciled against an independent site ledger. Related: issue #600, the `nav_routes` gap in draft PR #606.
The plan files a tracking issue. Research record:
`~/prism-evidence/lanes-2026-11-20/research/lane12-route-model.md`.

## 1. Intent

Prism should be able to list a repository's web routes: which HTTP method and path lead to which handler
function, with the mount prefix where it is static. Today it only knows that some registration exists for a
handler in three frameworks, and it throws away the method and the path. A route list serves several
features: an agent's entry-point question ("what handles `POST /users`"), binding schema operations to
handlers (lane 5), and request sources keyed on registration instead of on a per-file framework import
(#600).

## 2. What exists today (verified on `origin/main` at 1179055f)

| Fact | Reference |
|---|---|
| Route registrations are recognised for Flask and FastAPI decorators and Express registration calls only, as `FrameworkEntryRecord { caller, handler, site, framework }`. The method name is checked and dropped; the path argument, `methods=`, prefixes and the router's identity are never read. | `src/framework_entries/mod.rs:82`, `python.rs`, `express.rs`; `src/frameworks/mod.rs:160` |
| Not recognised: `add_url_rule`, `register_blueprint`, `include_router`, Flask-AppBuilder `@expose`, Django `path()`, DRF routers, and every Go, Fastify, Koa and NestJS route. | probes |
| Express handlers resolve only when the argument is a bare identifier bound to exactly one function in the same file. Inline arrow functions are counted as unresolved, although they now have graph identities (`<anon@L:C>` from `anon_owner::identity`); the comment and test that say otherwise are stale. Handlers written as a call (`asyncHandler(async …)`) or a member (`site.index`) are dropped without being counted. A mount (`app.use('/api', router)`) is counted as an unresolved handler. | `src/framework_entries/express.rs:132`, `src/call_graph/framework_entry_tests.rs:199` |
| Go has partial handler edges of another kind: `r.GET("/x", h)` appears as a `callback_registration` edge from the registering function, with no method or path. | probe |
| The records feed navigation only (edge kind `framework_entry`, name-only confidence, score 0.6). The record's documentation states they are never used by call edges, data flow or taint. | `src/navigation/mod.rs:536-575` |
| Request sources are attached per file: the file must import the framework, and for Express the handler's parameters must be named `req` then `res`. Issue #600's census found no instance of the registration-in-another-file shape on five corpora; the larger measured gap is naming (84 handlers in one corpus written `(req, _res, next)` get no source). | `crates/prism-review/src/algorithms/taint/js_ts/framework_params.rs:16`; issue #600 |
| How real code composes paths (from `bench-repos`): Flask blueprints take `url_prefix` and are registered in another file; FastAPI composes `APIRouter(prefix=…)` with `include_router(…, prefix=…)`, literals in the examples seen; Express mounts routers with a literal prefix in another file (5 of 5) and writes most handlers inline (39 inline to 8 named in one repository; about 254 wrapped in `asyncHandler(…)` in another); NestJS uses decorators with literal paths plus a version constant and an environment prefix; gin passes a group into registration functions as a parameter with a prefix built from configuration; Go 1.22 `net/http` patterns carry the method in the string (`"POST /v1/users"`, 155 of 159 registrations in one repository). | research section 4 |
| `src/call_graph.rs` and `src/navigation/queries.rs` are over the 2,000-line ceiling. The navigation `Evidence` model is closed (no route item, no method or path field). CPG cache v189, navigation sidecar v83. | research sections 1 and 5 |

## 3. Goals, non-goals, anti-goals

Goals:

1. A route record per registration site: methods, the path as written, every composed path through which
   it is served (full when every prefix on that chain is static), the registered callbacks in order with
   their identities (named or anonymous) or why each is unresolved, a delegate where one is proven, the
   registration site, the framework.
2. A mount record, so prefixes compose across files: which router is mounted on which, with what prefix.
3. A listing, `prism nav routes`, with its own JSON shape.
4. Frameworks in waves, chosen by measurement: first the three already recognised, then Go, then the
   decorator and route-table families.
5. Every registration the extractor sees but cannot complete is listed with a reason; nothing is dropped
   silently.

Non-goals (this lane): changing the existing `framework_entry` navigation edges; an MCP tool; using routes
in taint or data flow; request-source changes (a separate slice with its own measurement, section 8);
runtime-registered routes, routes from configuration files, generated routers; Flask-AppBuilder.

Anti-goals: a composed path presented as certain when a prefix on its chain is dynamic; a known static path
given up because another chain is unknown; a route attached to a router it was not registered on; one
callback presented as "the" handler by arity or position; a handler identity that disagrees with the entry
records where both rest on one unique, visible, unwritten binding; growing the two files that are over the
size ceiling.

## 4. Approaches considered

**A. Extend `FrameworkEntryRecord` with method and path.** Smallest diff, but the record is cached, feeds
navigation edges whose output is pinned, and has no place for mounts, unresolved handlers or anonymous
handlers. Every wave would bump the cache and risk the navigation goldens. Rejected.

**B. A separate route table computed on demand from parsed files and the call graph** (chosen). Extraction is
syntactic and cheap; computing it when the listing is asked for needs no cache change and no change to
navigation edges. The existing entry records stay as they are. If a later consumer needs routes inside the
graph build, the table can be cached then.

**C. A framework-agnostic "registration call" heuristic** (any call with a string that looks like a path and
a function argument). Broad, but it cannot know methods or prefixes and would list non-routes. Rejected in
favour of per-framework extractors behind one record type.

## 5. Architecture — *extraction rules tentative*

### 5.1 Records

```rust
// src/framework_entries/routes.rs; `Reason` is one closed enum for every reason, fixed at stage D
pub struct RouteRecord {                   // one per registration site (decorator node, call node, builder verb leg)
    pub framework: String,                 // "flask", "fastapi", "express", "gin", "net/http", …
    pub methods: Methods,                  // Known(Vec<String>) upper-case, sorted, deduplicated | Any | Unknown { text }
    pub path: RoutePath,                   // { raw: String, literal: Option<String> }: spelling, and the evaluated literal
    pub receiver: Receiver,                // Router(RouterId) | Unproven(Reason)
    pub paths: Vec<Exposure>,              // every composed path (5.2); empty only for an unproven receiver
    pub callbacks: Vec<Callback>,          // every callback position as written, in order
    pub site: FrameworkEntrySite,          // file, line, start_byte, end_byte of the registration
}
pub struct Exposure { pub root: Option<RouterId>, pub status: PathStatus }
pub enum PathStatus { Full(String), Suffix { known: String, reason: Reason }, Dynamic(Reason) }
pub struct Callback { pub position: usize, pub value: CallbackValue, pub delegate: Option<FunctionId> }
pub enum CallbackValue { Resolved(FunctionId), Anonymous(FunctionId), Unresolved { text: String, reason: Reason } }
pub struct MountRecord {
    pub parent: Receiver, pub child: Receiver,
    pub prefix: MountPrefix,               // Literal(String) | Absent (framework default) | Unevaluated { text }
    pub site: FrameworkEntrySite,
}
pub struct RouterId { pub file: String, pub binding: String, pub construction_byte: usize }
```

- `path.raw` and prefixes keep the framework's own spelling (`<int:id>`, `{id}`, `:id`). `path.literal` is
  set only for an evaluated literal node: a string without escapes or interpolation (decoding escapes and
  concatenating literals are stage D choices); text that merely starts with a quote is not a literal
  (`js_ts_literal_string_value` is not used: it strips quotes from `"/a" + "/b"` and leaves
  `"/\u0078"` undecoded). The listing adds a comparison key, `normalize_path(framework, path) -> Option<String>`,
  which writes each parameter `{}` by that framework's grammar and is `None` for a template it cannot
  classify (a typed tail such as `<path:p>` or `{p...}`, a regex, an optional); a Flask `:id` is literal text.
- `methods`: `Known` when every method is a literal; `Any` for registrations that accept every method
  (`all`, `Any`, a net/http pattern without a method); `Unknown` otherwise. Framework-added HEAD and OPTIONS
  are not listed (stage D may add an effective set).
- Receiver identity is grounded at the use: the receiver name must resolve in its own lexical scope
  (innermost function, else module) to exactly one binding, never written afterwards, whose value is a
  recognised constructor (`Flask(…)`, `Blueprint(…)`, `FastAPI(…)`, `APIRouter(…)`, `express()`,
  `express.Router()`, `gin.Default()`, `gin.New()`, a `Group` result), a proven import of one (5.2), or a Go
  parameter within the stated bound. `RouterId` is that binding with its constructor's start byte, so two
  bindings of one name, or a rebinding to a fresh router, are two routers. Anything else is
  `Unproven(reason)`: the record is listed and counted, with no path and no binding credit. (A
  file-and-name key would attach an unmounted router's routes to a mounted one of the same name, and
  today's Python receiver set is file-wide: a function-local `app = Sink()` decorator gets a Flask entry.)
- Callbacks: every callback position, in order (Express arguments after the path, arrays flattened in order,
  a spread unresolved; gin handler arguments; a decorator's function; `view_func=`; `endpoint`). Each
  value resolves in its own lexical scope: JS/TS through `ParsedFile::js_ts_site_binding` (a callable
  terminal is `Resolved`; an import is followed through `CallGraph::import_bindings` to the target file's
  `js_ts_exports` entry and accepted only for a proven callable terminal); Python through the name's binding
  in its scope; Go through the `CallGraph::go_registrations` record whose site bytes are the argument's own.
  The entry records' file-and-name rule is not reused: at module scope it ignores imports, and a probe gave an
  imported `serve` handler's edge to an unrelated nested `serve`. An inline JS/TS function is `Anonymous`
  (`anon_owner::identity`); Go function literals and Python lambdas are `Unresolved` (no anonymous identity
  yet). A Python decorated function is the registered value only when every decorator between the route
  decorator and the `def` is itself a route decorator (those return the function unchanged); any other
  decorator below may replace it (`inner_decorator`). A call in a callback position (`asyncHandler(fn)`,
  `validate('r')`) is `Unresolved` (`wrapper_call`, its text kept); its `delegate` is set only when the callee
  is a forwarder on stage D's closed list, proven by its import binding, and the forwarded argument resolves
  by these rules. Arity and position never select a delegate or a handler. Members (`controller.method`)
  are `member_handler` until stage D admits a resolution; class views (`as_view`) are `class_view`.

### 5.2 Composition

A mount graph per repository: nodes are routers (`RouterId`), edges are mounts with their effective prefix,
computed by the framework's rule (Flask: a `url_prefix` at `register_blueprint` replaces the constructor's;
FastAPI: the include's prefix is prepended to the router's own; Express and gin: the mount's path). Roots are
application objects (a Flask or FastAPI app, an `express()` result, a gin engine, a `ServeMux` or the
default mux). Each registration gets one exposure per chain from a root to its router: `Full` when every
prefix on that chain is literal, `Suffix { known, reason }` when one is not (`known` is the literal part
below it), so a router mounted at `/v1` and `/v2` yields both full paths, and one literal plus one variable
mount yields one full path and one suffix. A router no mount reaches gives one `Suffix` (`no_known_mount`);
a cycle closes its branch as `Suffix` (`mount_cycle`); past 64 exposures per registration the rest is one
`Suffix` (`expansion_truncated`). `Dynamic` is the registration's own non-literal path. Joins follow each
framework's rule, settled at stage D against executed truth (seeds: Flask `/api` with rule `""` is `/api`;
FastAPI `/items` with `/` is `/items/`).

A router imported from another file is proven, not looked up by file: the importing file's
`CallGraph::import_bindings` entry (local name, module, imported member, eligibility) leads to one target
file, and the imported member (JS/TS: the local spelling its `js_ts_exports` entry names, `default`
included, or a top-level `module.exports = r` read from the syntax, since the CJS export facts hold
callables only; an entry is a spelling, never proof; Python: `from m import x as u`, or
`from . import auth` then `auth.bp` through the submodule) must be a binding that the table grounded as a router in that file. An ineligible binding, a
second candidate file, a re-export chain or a star import leaves the child `Unproven` (`mount_child_unproven`).

### 5.3 Frameworks, in waves

| Wave | Frameworks | Shapes |
|---|---|---|
| 1 | Flask, FastAPI, Express | decorators and registration calls already recognised, now with method and path; `add_url_rule`, `add_api_route`; every verb leg of `app.route(p).get(h).post(h2)`; Express verbs from the pinned Express's own method table (HEAD and OPTIONS included), not the entry matcher's seven names; `app.get('env')` with no callback is a settings read, not a registration; `use` split into middleware and mounts; inline, wrapped and imported callbacks; mounts (`register_blueprint` with `url_prefix`, `include_router` with `prefix`, `app.use(prefix, router)`) |
| 2 | Go | gin (`r.GET`, `Group`, groups passed as parameters within one function chain where the argument is the group variable itself), `net/http` including method-in-pattern strings, chi; reusing the existing callback-registration facts for handler identity |
| 3 | decided at stage M by count | from: NestJS decorators, Fastify, Koa routers, Django `path`/`re_path`/`include`, DRF routers, object-literal route tables (`route([{ method, path, handler }])`). A table is a route only when its consumer is grounded: a framework API or a stage D adapter, proven by its import binding, that installs the rows it is passed; an ungrounded table is listed as a candidate with reason `table_consumer_unproven` and earns no path or binding credit |

### 5.4 The listing

`prism nav routes --repo <dir> [--format json|text]`, CLI only, like `dfg-stats` and `call-stats`: its own
JSON, one object per registration per line, sorted by (first path's comparison key or spelling, first
method, file, start byte); a consumer that wants one row per served path expands `paths`:

```json
{"framework":"express","methods":{"known":["GET"]},"path":{"raw":"'/:id'","literal":"/:id"},
 "paths":[{"status":"full","value":"/v1/users/:id","normalized":"/v1/users/{}","root":{"file":"app.js","binding":"app","construction_byte":40}},
          {"status":"suffix","value":"/users/:id","reason":"variable_prefix","root":{"file":"app.js","binding":"app","construction_byte":40}}],
 "callbacks":[{"position":0,"status":"resolved","file":"auth.js","name":"auth","start_line":3,"end_line":9},
              {"position":1,"status":"anonymous","file":"routes/users.js","name":"<anon@12:24>","start_line":12,"end_line":20}],
 "receiver":{"status":"router","file":"routes/users.js","binding":"router","construction_byte":61},
 "site":{"file":"routes/users.js","line":12,"start_byte":310,"end_byte":420}}
```

Every enum is a discriminated `status` field; an unresolved callback adds `text` and `reason`, a proven
delegate adds `delegate`, an unproven receiver adds `reason` and has `"paths":[]`. A summary object
follows: registrations, routes with a proven receiver, counts per framework, per path status, per callback
status and per reason. Mount records are listed with `--mounts`; the user page (PLAN S1) shows one example line per
record kind and status.

### 5.5 Where the code goes

```
src/framework_entries/routes.rs            records, the entry point `routes(files, call_graph) -> RouteTable` (no dependency on navigation)
src/framework_entries/routes/receivers.rs  receiver grounding and `RouterId`
src/framework_entries/routes/handler.rs    callback resolution shared by the frameworks
src/framework_entries/routes/python.rs     Flask and FastAPI
src/framework_entries/routes/express.rs
src/framework_entries/routes/go.rs
src/framework_entries/routes/mounts.rs     mount graph and composition
src/navigation/routes.rs                   the listing (no logic in queries.rs)
crates/prism-review/src/cli.rs, main.rs    NavQuery::Routes and its arm
```

The existing extractors in `src/framework_entries/python.rs` and `express.rs` are not changed in behaviour,
and their site and receiver logic is not reused for routes: the Express candidate extractor drops any call
whose arguments hold no identifier or function, and its matcher accepts only a builder's first verb; the
Python receiver sets are file-wide. Small parsing helpers (decorator receiver and method) may be reused.
Gin registrations are recognised by the gin import, not by the presence of
`*gin.Context` in the file (router files take `*gin.RouterGroup`). No cache version change.

## 6. Measurement

**Census (stage M).** For each framework, on the corpora that use it: registrations by shape (an independent
regular-expression and syntax count), handler form (named, inline, wrapped, member, imported), where the
prefix comes from (none, literal on the router, literal at a mount in another file, variable,
configuration), and for the wave 3 candidates the counts that decide the order. The syntax pass also writes
the **site ledger**: one line per registration or mount site (corpus revision, file, start and end byte of
the decorator, call or builder-leg node, kind, shape), frozen at stage M. It is written independently of
prism and is the completeness denominator (section 7).

**Schema oracle.** Three corpora commit an API description next to their code (`apache-answer`, `directus`,
`cal.com`, whose API is NestJS and so is checked only if wave 3 admits that framework). Each schema is bound
before scoring to its application root, revision, API base and inclusion rule; only exposures under that
root and base are scored. Operations are (method, exact path) keys from the expanded method sets; a `full`
exposure matches only an identical path (no trailing-slash or `//` collapsing, unless the framework is
independently documented as slash-insensitive there); a `suffix` matching exactly one operation is a unique
candidate, one matching several is ambiguous and credits none; `Any` methods are reported apart. The oracle
reports each numerator and denominator (exact full, unique suffix, ambiguous, unsupported) and classifies
every difference (dynamic prefix, unsupported shape, schema drift, outside the bound scope). Schema keys
cannot see a handler swap, so where the schema names its handlers the bindings are compared too:
`apache-answer`'s swag `@Router` comments (199) sit on the handler functions, so each operation's annotated
function is checked against the listing's callbacks and delegate for that operation. FastAPI fixtures need no
generated schema: their executed truth already binds each route to its endpoint.

**Executed truth for the matrix.** Matrix rows are small lane-written applications, run only with the pinned
offline dependencies (owner decision 5; never a corpus application). The runner instruments the
registration calls themselves (Flask's and `Blueprint`'s `add_url_rule`, through which every Flask decorator
passes; FastAPI `APIRouter.add_api_route`; Express verb, `route` and `use` methods) before loading the row,
recording per call its site (the first stack frame in the row's files, with its ordinal on the line), the
method and path arguments, whether a mount made the call (`register_blueprint` and `include_router` re-register
copies, linked to the original by callback identity), and each callback object's source identity
(Python: code object file, first line and qualified name; JS: the function's source text located in the row's
files, which maps an anonymous callback to its `<anon@L:C>`); the framework's own table then gives the served
paths and effective methods, linked to those calls. A function's first line is never used as a site. Fixtures
disable framework-added routes (Flask `static_folder=None`, FastAPI `docs_url`, `redoc_url`, `openapi_url` set to
`None`); any route without an instrumented call fails the row. Truth is kept as raw observations; the
comparison projects each field explicitly (a static `suffix` agrees with a runtime full path ending in it; a
static `full` must equal it; declared and effective methods are kept apart). Containment: the staged copy holds
only the row's directory (a real directory under `tests/fixtures/routes/`, no links) and the pinned dependency
tree; a static check before each run requires every relative import to stay inside the row and every bare
import to be in the pinned set; Python children run with `-I` and no `PYTHONPATH`, and after loading every
module in `sys.modules` must come from the row, the venv or the standard library; Node children run with no
`NODE_PATH` and the dependency tree copied into the staged root, so ESM and CJS both resolve to it; Go is never fetched
(`GOPROXY=off GOFLAGS=-mod=mod GOWORK=off`). A framework absent from the pinned caches
(gin and chi on 2026-11-21) and net/http, which has no route table, are hand-labelled; a row's `executed` or
`hand` provenance changes only by a reviewed commit.

## 7. Gates

| Gate | Threshold (default) |
|---|---|
| Census go | waves 1 and 2: at least 200 ledger registrations of the wave's frameworks together (syntax count, test and non-test), and then every framework of the wave with any registration is built; wave 3: per framework, at least 200, in descending order; object-literal tables are built whatever their count (owner decision 3). Below the threshold, or zero: skipped and reported. The shared slice (PLAN S1) is always built |
| Matrix | every row's records equal its truth under the stated projection, sites and callback identities included, so a swapped handler or a wrong site fails; unsupported rows carry the stated reason |
| Schema oracle | on each schema-bearing corpus in a finished wave: precision at least 0.95 of in-scope `full` exposures, each matching an exact operation; ambiguous suffixes credit nothing; recall reported with every miss classified; every listed binding that contradicts a schema-named handler is classified by a reader, and an unexplained contradiction fails |
| Completeness (reconciled) | every ledger site of a built framework is extracted (by start and end byte), excluded by a rule stated independently of the extractor, an accepted stage-D loss, or unexplained; unexplained sites fail the gate, as does a supported shape class with no extracted site; extracted sites missing from the ledger are listed apart and classified. Recall over the ledger (sites extracted with a proven receiver and at least one exposure) is reported against a floor fixed at stage D. Each eligible corpus needs a nonempty, reviewed yield of `full` exposures and resolved bindings; a zero denominator fails unless the ledger has no applicable site. The listing's own invariant still holds: registrations = records with a proven receiver + records with an unproven one, and every callback has a status |
| No regression | `nav callers`, `call-stats` and the navigation goldens are unchanged; the cache version is unchanged. Each wave gate also records cold session time, table time and peak RSS on its corpora |

## 8. Follow-ups, not started by this lane

| Follow-up | Evidence that would justify it |
|---|---|
| Request sources keyed on route records, with the handler's parameters taken by position | the naming gap already measured (84 handlers in one corpus); needs its own benchmark measurement and touches taint seeding |
| Entry edges derived from route records (inline and wrapped handlers visible to `nav callers`) | agent sessions that ask for callers of a handler and get nothing; also the probe in which a module-level registration's entry edge goes to an unrelated nested function instead of the imported handler |
| An MCP tool `nav_routes` | a decision on the navigation specs in PR #606 |
| Lane 5's producer adapters reading route records | lane 5's pilot being funded; this lane gives it registration records (adapter input, not a wired adapter) for the waves that ship, with the binding rule: bind an operation only to a `full` exposure with a resolved callback or delegate, and treat the callback list as ordered (#600 gets the same position semantics) |

## 9. Owner decisions

| # | Question | Default applied | Cost of changing later | Latest cheap point |
|---|---|---|---|---|
| 1 | Where routes live | a separate on-demand table; entry records and navigation edges untouched | caching the table later is a cache bump | before S1 |
| 2 | Exposure | CLI listing only | an MCP tool is a later slice with golden changes | after wave 1 |
| 3 | Wave 3 membership and order | by census count; object-literal route tables are included regardless of count because the private producer of lane 5 uses that shape | — | stage M |
| 4 | Flask-AppBuilder and similar class-based frameworks | out of scope | each is its own extractor | — |
| 5 | Executing fixture applications for truth | yes where the framework is already installed or installable offline; never anything from a corpus | hand-written truths only | stage T |

## 10. Risks and accepted limits

- **Prefixes are often dynamic** (configuration values, version constants, environment prefixes). The
  listing says `suffix` and shows what it knows; it never guesses.
- **Routers passed through functions.** A group handed to a registration function as a parameter is followed
  only within the stated bound; beyond it the path is a suffix.
- **Handler wrappers.** A wrapper call is an unresolved callback; a delegate is recorded only through stage
  D's closed forwarder list. Decorators that may replace the handler, dependency injection and class-based
  views are unresolved with a reason. Request-source coverage built on the listing pays for this.
- **Two registration facts for Go.** The existing callback-registration edges and the new records describe
  the same calls; the records read those facts at the argument's own bytes, so the two cannot disagree. A Go
  method value (`c.List`) has no such fact and is unresolved until stage D decides otherwise.
- **Strict receiver and import proof costs recall** (factory apps, re-exported routers); each refusal is a
  counted reason, and the stage D recall floor is set knowing it.
- **Out of model:** routes added at run time in loops over data, plugin systems, reflection.
