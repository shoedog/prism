# Lane 12: routes as a first-class fact — design

Status: architecture final; the per-framework extraction rules are tentative until the census, the test
matrix and enumeration (PLAN stages M, T, E). Related: issue #600, the `nav_routes` gap in draft PR #606.
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

1. A route record: methods, the path as written, the composed path when every prefix on the mount chain is
   static, the handler's identity (named or anonymous) or why it is unresolved, the registration site, the
   framework.
2. A mount record, so prefixes compose across files: which router is mounted on which, with what prefix.
3. A listing, `prism nav routes`, with its own JSON shape.
4. Frameworks in waves, chosen by measurement: first the three already recognised, then Go, then the
   decorator and route-table families.
5. Every registration the extractor sees but cannot complete is listed with a reason; nothing is dropped
   silently.

Non-goals (this lane): changing the existing `framework_entry` navigation edges; an MCP tool; using routes
in taint or data flow; request-source changes (a separate slice with its own measurement, section 8);
runtime-registered routes, routes from configuration files, generated routers; Flask-AppBuilder.

Anti-goals: a composed path presented as certain when a prefix on the chain is dynamic; a second, diverging
handler-resolution rule; growing the two files that are over the size ceiling.

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
// src/framework_entries/routes.rs
pub struct RouteRecord {
    pub framework: String,                 // "flask", "fastapi", "express", "gin", "net/http", …
    pub methods: Vec<String>,              // upper-case, sorted; ["*"] when the registration accepts any
    pub path: String,                      // as written at the registration, parameters kept as written
    pub router: Option<RouterRef>,         // the router or app object the registration is made on
    pub full_path: PathStatus,             // Full(String) | Suffix(String) | Dynamic
    pub handler: RouteHandler,             // Resolved(FunctionId) | Anonymous(FunctionId) | Unresolved { text, reason }
    pub wrapper: Option<String>,           // name of a wrapper call around the handler (`asyncHandler`)
    pub site: FrameworkEntrySite,          // file, line, bytes of the registration
}
pub struct MountRecord {
    pub parent: Option<RouterRef>, pub child: RouterRef,
    pub prefix: Option<String>,            // None when not a string literal
    pub site: FrameworkEntrySite,
}
pub struct RouterRef { pub file: String, pub binding: String }   // the declaring file and name of the router object
```

- `path` and prefixes keep the framework's own parameter spelling (`<int:id>`, `{id}`, `:id`); a normalised
  form with every parameter written `{}` is added in the listing so consumers can match across frameworks.
- `full_path` is `Full` when the registration's router reaches an application root through mounts that all
  have literal prefixes and the chain is unique; `Suffix` when some prefix is missing or the router is
  mounted in more than one place; `Dynamic` when the registration's own path is not a literal.
- Handler resolution reuses what exists: a bare identifier goes through the same binding rules the entry
  records use. A handler argument is a value reference, not a call, so the call-site resolver does not
  apply to it as it stands; how imported names and members are resolved (which existing binding facts are
  read) is fixed at stage D. An inline JavaScript or TypeScript function gets its anonymous identity; Go
  function literals and Python lambdas have no anonymous identity today and are `Unresolved` with their own
  reason until one exists; a wrapper call with exactly one function argument
  (`asyncHandler(fn)`) resolves to that argument and records the wrapper's name; a member expression
  (`controller.method`) resolves when the call graph resolves that member, else it is `Unresolved` with
  reason `member_handler`. Reasons are a closed list fixed at stage D.

### 5.2 Composition

A mount graph per repository: nodes are router objects (`RouterRef`), edges are mounts with their prefix.
A router imported from another file is identified through the import binding (the declaring file and name),
using the resolution the call graph already has for imports. The composed path is the concatenation of
literal prefixes from the root to the registration, with exactly one `/` at each join.

### 5.3 Frameworks, in waves

| Wave | Frameworks | Shapes |
|---|---|---|
| 1 | Flask, FastAPI, Express | decorators and registration calls already recognised, now with method and path; `add_url_rule`; `app.route(p).get(h)`; inline, wrapped and imported handlers; mounts (`register_blueprint` with `url_prefix`, `include_router` with `prefix`, `app.use(prefix, router)`) |
| 2 | Go | gin (`r.GET`, `Group`, groups passed as parameters within one function chain where the argument is the group variable itself), `net/http` including method-in-pattern strings, chi; reusing the existing callback-registration facts for handler identity |
| 3 | decided at stage M by count | from: NestJS decorators, Fastify, Koa routers, Django `path`/`re_path`/`include`, DRF routers, object-literal route tables (`route([{ method, path, handler }])`) |

### 5.4 The listing

`prism nav routes --repo <dir> [--format json|text]`, CLI only, like `dfg-stats` and `call-stats`: its own
JSON, one object per line, sorted by (normalised full path or suffix, method, file, line):

```json
{"framework":"express","methods":["GET"],"path":"/:id","full_path":{"status":"full","value":"/users/:id","normalized":"/users/{}"},
 "handler":{"status":"anonymous","file":"routes/users.js","name":"<anon@12:24>","start_line":12,"end_line":20},
 "site":{"file":"routes/users.js","line":12},"router":{"file":"routes/users.js","binding":"router"}}
```

followed by a summary object: counts per framework, per `full_path` status, per handler status, and per
unresolved reason. Mount records are listed with `--mounts`.

### 5.5 Where the code goes

```
src/framework_entries/routes.rs            records, the entry point `routes(files, call_graph) -> RouteTable` (no dependency on navigation)
src/framework_entries/routes/handler.rs    handler resolution shared by the frameworks
src/framework_entries/routes/python.rs     Flask and FastAPI
src/framework_entries/routes/express.rs
src/framework_entries/routes/go.rs
src/framework_entries/routes/mounts.rs     mount graph and composition
src/navigation/routes.rs                   the listing (no logic in queries.rs)
crates/prism-review/src/cli.rs, main.rs    NavQuery::Routes and its arm
```

The existing extractors in `src/framework_entries/python.rs` and `express.rs` are not changed in behaviour.
Their matching helpers are reused, but registration sites are found by a new scan: the Express candidate
extractor drops any call whose arguments hold no identifier or function, so member, wrapped and `require`d
handlers never reach it. Gin registrations are recognised by the gin import, not by the presence of
`*gin.Context` in the file (router files take `*gin.RouterGroup`). No cache version change.

## 6. Measurement

**Census (stage M).** For each framework, on the corpora that use it: registrations by shape (an independent
regular-expression and syntax count), handler form (named, inline, wrapped, member, imported), where the
prefix comes from (none, literal on the router, literal at a mount in another file, variable,
configuration), and for the wave 3 candidates the counts that decide the order.

**Schema oracle.** Three corpora commit an API description next to their code (`apache-answer`, `directus`,
`cal.com`, whose API is NestJS and so is checked only if wave 3 admits that framework); FastAPI schemas
come from lane-written fixtures only, because corpus applications are never run. For these, the extracted set of (method, normalised full
path) is compared with the schema's operations: recall (operations the listing found) and precision (listed
routes that are operations), with every difference classified (dynamic prefix, unsupported shape, schema
drift, non-API route).

**Executed truth for the matrix.** Matrix rows are small applications. Where the framework is installable
offline on this machine, the row's truth is the framework's own route table printed by running the fixture
(`app.url_map`, `app.routes`, the router stack, `engine.Routes()`); otherwise the truth is written by hand
and marked so.

## 7. Gates

| Gate | Threshold (default) |
|---|---|
| Census go, per wave | at least 200 registrations of the wave's frameworks on the corpora; below that the wave is skipped and reported |
| Matrix | every row's records equal its truth; unsupported rows carry the stated reason |
| Schema oracle | on each schema-bearing corpus in a finished wave: precision at least 0.95 of `full` paths; recall reported with every miss classified; no `full` path that the schema contradicts without a classification |
| Completeness | registrations seen = records with a resolved or anonymous handler + records with an unresolved reason; nothing uncounted |
| No regression | `nav callers`, `call-stats` and the navigation goldens are unchanged; the cache version is unchanged |

## 8. Follow-ups, not started by this lane

| Follow-up | Evidence that would justify it |
|---|---|
| Request sources keyed on route records, with the handler's parameters taken by position | the naming gap already measured (84 handlers in one corpus); needs its own benchmark measurement and touches taint seeding |
| Entry edges derived from route records (inline and wrapped handlers visible to `nav callers`) | agent sessions that ask for callers of a handler and get nothing |
| An MCP tool `nav_routes` | a decision on the navigation specs in PR #606 |
| Lane 5's producer adapters reading route records | lane 5's pilot being funded; this lane's wave 2 gives it the Go registration adapter |

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
- **Handler wrappers.** Only the single-function-argument wrapper form resolves; decorators that replace the
  handler, dependency injection and class-based views are listed as unresolved with a reason.
- **Two registration facts for Go.** The existing callback-registration edges and the new records describe
  the same calls; the records reuse those facts for handler identity so the two cannot disagree.
- **Out of model:** routes added at run time in loops over data, plugin systems, reflection.
