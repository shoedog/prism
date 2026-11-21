# Lane 12: routes as a first-class fact — design

Status: architecture final; the per-framework extraction rules are tentative until the census, the test
matrix and enumeration (PLAN stages M, T, E). Design review round 1 (2026-11-21) folded: one record per
registration with every composed path, router identity grounded at the use, every callback recorded,
gates reconciled against an independent site ledger. Owner decision 6 (2026-11-21) folded: Go method-value
handlers are in scope (section 5.6). Related: issue #600, the `nav_routes` gap in draft PR #606.
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
| Go method values (`r.GET("/x", a.questionController.List)`) have no fact. `go_registrations` records only a bare identifier that resolves to a free function (`resolve_go_bare_value_ref`). The Go receiver classifier types a selector chain of one or two fields only from a parameter, local or package `var` base (`classify_nested_selector`); the enclosing method's own receiver is gated out (`is_recv` in `classify_go_receiver_expanded_with_partition` and `classify_simple_ident_mode`), so even the call `a.questionController.List(c)` gets no receiver type. The parts a method value needs exist: declared field types (`CallGraph::go_field_types`, `resolve_go_struct_field_owner`), the receiver router (`CallGraph::go_concrete_receiver_route`) and the per-`(owner, name)` method index (`CallGraph::methods`) | `src/call_graph.rs:3984`, `src/resolution/free.rs:122`, `src/call_graph/go/receiver_index.rs:693`, `:870`, `src/resolution.rs:775`, `src/resolution/go/concrete_receiver.rs:332` |
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
runtime-registered routes, routes from configuration files, generated routers; Flask-AppBuilder; changing
how Go calls resolve (the method-value lookup of 5.6 is read by the route table only), and resolving
interface-typed Go handlers to candidate implementers.

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
  in its scope; Go: a bare identifier through the `CallGraph::go_registrations` record whose site bytes are
  the argument's own, a selector (method value, package function, method expression) by section 5.6.
  The entry records' file-and-name rule is not reused: at module scope it ignores imports, and a probe gave an
  imported `serve` handler's edge to an unrelated nested `serve`. An inline JS/TS function is `Anonymous`
  (`anon_owner::identity`); Go function literals and Python lambdas are `Unresolved` (no anonymous identity
  yet). A Python decorated function is the registered value only when every decorator between the route
  decorator and the `def` is itself a route decorator (those return the function unchanged); any other
  decorator below may replace it (`inner_decorator`). A call in a callback position (`asyncHandler(fn)`,
  `validate('r')`) is `Unresolved` (`wrapper_call`, its text kept); its `delegate` is set only when the callee
  is a forwarder on stage D's closed list, proven by its import binding, and the forwarded argument resolves
  by these rules. Arity and position never select a delegate or a handler. JS/TS and Python members
  (`controller.method`) are `member_handler` until stage D admits a resolution; Go method values follow 5.6
  (owner decision 6); class views (`as_view`) are `class_view`.

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
| 2 | Go | gin (`r.GET`, `Group`, groups passed as parameters within one function chain where the argument is the group variable itself), `net/http` including method-in-pattern strings, chi; reusing the existing callback-registration facts for bare-identifier handlers, and method-value handlers by 5.6 |
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
src/framework_entries/routes/go_method_value.rs  Go selector callbacks (5.6); reads call-graph facts, adds none
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

### 5.6 Go method-value handlers (owner decision 6)

A Go callback written as a selector `X.M` that is not a call is a value reference. Three kinds are told apart
by syntax: `pkg.F` with `pkg` an import name of the file is a function of another package; `T.M` and
`(*T).M` with `T` a type name are method expressions; anything else is a method value, which names the method
`M` of `X`'s static type. Go fixes a variable's static type at its declaration, and a method value of a
concrete type is bound when it is evaluated, not dispatched later: assigning a new value to the field,
parameter or local changes the receiver value, never the method. Only an interface-typed `X` selects the
method at run time. So the proof is about `X`'s declared type, and "nothing rebinds the receiver" means that
the spelling of `X`'s base resolves to one declaration in scope at the site; a shadowing local or closure
parameter of that spelling refuses.

Measured shapes (research run of 2026-11-21 with a tree-sitter script written independently of prism,
`~/prism-evidence/lanes-2026-11-20/research/scratch/lane12-go/shapes.py`; PLAN M-2 re-counts them; terminal
callback of each non-test registration):

| Shape of the terminal callback | `apache-answer` 6744295fa888 (gin, 232) | `miniflux-v2` faecfa2fac61 (net/http, 179) |
|---|---|---|
| method value on a field of the enclosing method's receiver (`a.questionController.List`) | 211 (90.9%) | 0 |
| method value on a local bound once to such a field (`c := pr.connectorController; c.Info`) | 6 (2.6%) | 0 |
| method value on a local with a composite-literal or constructor initialiser (`handler := &handler{…}; handler.show`) | 0 | 150 (83.8%) |
| method value on a local initialised by an in-repo call (`h := googlereader.NewHandler(…); h.ServeHTTP`) | 0 | 1 |
| method value on a typed parameter, a package `var`, an interface-typed value, a call result; method expression | 0 | 0 |
| method value inside a wrapper call (`withApiKeyAuth(h.tokenHandler)`) | 0 | 14 (7.8%) |
| bare function or function variable | 9 | 8 |
| function literal (forwarding or not) | 3 | 1 |
| other wrapper call (`gin.WrapH(…)`) | 3 | 5 |

On `apache-answer`, 192 of the 199 swag `@Router` comments sit on methods that are registered as method
values, so without this section the wave 2 binding comparison can cover at most 7 operations. Non-terminal
positions there are four middleware arguments (three `middleware.BanAPIForUserCenter`, one
`authUserMiddleware.Auth()`). A second look: `prometheus` (its own `route.Router`, not a wave 2 framework)
has 3 method values on the enclosing method's receiver, 5 on composite locals and 51 inside `wrapAgent(…)`
of 80 registrations; `caddy` registers 3 handlers, none a method value.

Rules (owner decision 6 puts these shapes in scope; stage D may rename a reason, or narrow a rule only with a
matrix row that shows a wrong binding):

- **M1, receiver.** `X` is one of:
  (a) the enclosing method's own receiver, typed by the receiver parameter of its `method_declaration`
  (`Language::go_receiver_var` names it; the type is read from the same parameter, pointer peeled) and
  placed by `resolve_go_receiver_owner_identity`; refused when `ParsedFile::receiver_type_in_fn` counts a
  local binding of that spelling before the site or `ParsedFile::go_func_literal_parameter_binds_receiver`
  finds a function-literal parameter of that spelling around it (new: the existing classifier gates the
  receiver out, section 2);
  (b) a parameter, a local or a package-level `var` typed by the existing classifier,
  `classify_go_receiver_expanded_with_partition` followed by `screen_go_receiver_prerequisites`, called with a
  `GoReceiverCtx` at the selector's own bytes and `ReceiverRecoveryConfig::default()` (typed parameters,
  composite-literal and `NewX` constructor locals, `var x T`, type assertions, call-initialised locals
  through `go_return_types`, package `var`s through `go_package_vars`; a recovery it marks shadowed refuses);
  (c) one or two field hops `b.f`, `b.f.g` from a base admitted by (a) or (b), each hop typed by the field's
  declared type through `resolve_go_struct_field_owner` over `CallGraph::go_field_types` (for a (b) base the
  classifier's own nested-selector walk does this; for an (a) base the new module runs the same per-hop
  call). The route table passes an empty field-target map, so a hop through an embedded `*T` or `pkg.T` field,
  which needs that proof, refuses;
  (d) a local declared exactly once (the count `receiver_type_in_fn` reports) by a one-name
  `x := <(a), (b) or (c) form>`, typed as that form; one level, never a chain of locals (new: the classifier
  recovers a local only from a constructor, `var` or call initialiser, so a field-chain initialiser has no
  fact today; the new module reads that one `short_var_declaration` from the syntax).
  (a) and (d) are the only new typing steps; both read declarations the code spells.
  Anything else (a call result `f().M`, an index `m[k].M`, three or more hops, a local declared by another
  form or more than once, a type parameter, a prerequisite drop) is `method_value_receiver_unproven`, and a
  refused shadow is `method_value_receiver_shadowed`.
- **M2, method.** The owner from M1 goes to `CallGraph::go_concrete_receiver_route(type, Some(&owner), false,
  false, M, file)`, the router the call resolver uses:
  `ConcreteDirect` resolves to the one `FunctionId` of `CallGraph::methods[(owner.name, M)]` whose file is in
  `owner.package_dir` and passes `go::owner_partition::exact_declaration_visibility` (the filter the call
  resolver's private `go_field_target_lookup` applies, restated in the new module because `resolution.rs`
  does not change); `ConcretePromoted` resolves to the one such entry of `CallGraph::promoted_aliases`, and
  `ConcretePromotedDeferred` to a `go::promoted_snapshot::consult_promoted_snapshot` hit; zero or several
  targets, `ConcreteNoSelector` and `Unproven` are `method_value_method_unproven`; `InterfaceDispatch`,
  `EmbeddedInterfaceDispatch` and `R2OnDemandNameCollisionBail` are `method_value_interface` (no handler and
  no candidates); `FuncValueField` is `func_field_value` (`X.F` reads a function-typed field; its value is not
  followed); `ExternalNewRecoveryDrop` is `method_value_external`, and so is an M1 type (receiver
  declaration, field type or classifier result) qualified by an import whose directory key
  (`resolution::go_import_path_dir_key`) is absent from `CallGraph::go_package_basenames` (gin's own types),
  checked before the hop or route that would otherwise refuse it as unproven. Pointer and value receivers
  need no case: the method index keys both under `T`, and the compiler accepts `x.M` only where it is legal.
- **M3, the other selectors.** `pkg.F` is `qualified_function_value` and a method expression is
  `method_expression`: both are listed and counted, never resolved by this lane (a method expression takes
  the receiver as its first argument, so it is not a gin or `http.HandlerFunc` handler in compiling code).
- **M4, scope.** The lookup is read only by the route table: it creates no `CallSite`, no edge and no cached
  fact, leaves `go_registrations` and call resolution as they are, and adds nothing to `src/call_graph.rs`
  or `src/resolution.rs` (every item it reads is already `pub` or `pub(crate)`). A forwarded argument of a
  delegate (5.1) resolves by M1 to M3 as well, so `withApiKeyAuth(h.tokenHandler)` gains a delegate only if
  stage D lists the forwarder.

Left out, each counted by its reason: interface-typed receivers (resolving them to their implementers is a
candidate list, not a handler; follow-up only if a consumer asks), call-result and deeper receivers, embedded
pointer or qualified hops, function-typed fields, package function values and method expressions. On the two
census corpora no terminal method value has a left-out shape (all 217 and 151 are admitted shapes); outside
5.6 the census puts only the 3 package-function middleware arguments on `apache-answer` (none terminal) and
the 14 wrapped method values on `miniflux-v2` (7.8% of its registrations), which wait on stage D's forwarder
list. Whether each admitted site resolves is measured by the gate (section 7), not assumed from its shape.

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
`apache-answer`'s swag `@Router` comments (199) sit on the handler functions (192 of them methods registered
as method values, 5.6), so each operation's annotated function is checked against the listing's callbacks
and delegate for that operation. FastAPI fixtures need no
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
| Go handler binding (wave 2) | on `apache-answer` and `miniflux-v2`: every ledger registration whose terminal callback is a selector carries a resolved handler or one of 5.6's reasons, none unexplained; at least 0.95 of the terminal method values of 5.6's admitted shapes resolve (census: 217 of 232 registrations on `apache-answer`, 151 of 179 on `miniflux-v2`), and each refused one is classified by a reader; on `apache-answer`, of the schema operations matched by a `full` exposure or a unique suffix, at least 0.90 have a resolved terminal callback that agrees with the operation's `@Router` function, and no contradiction is unexplained (the schema oracle row still applies). Stage D may lower a threshold only with the census rows that explain the loss |
| No regression | `nav callers`, `call-stats` and the navigation goldens are unchanged; the cache version is unchanged. Each wave gate also records cold session time, table time and peak RSS on its corpora |

## 8. Follow-ups, not started by this lane

| Follow-up | Evidence that would justify it |
|---|---|
| Request sources keyed on route records, with the handler's parameters taken by position | the naming gap already measured (84 handlers in one corpus); needs its own benchmark measurement and touches taint seeding |
| Entry edges derived from route records (inline and wrapped handlers visible to `nav callers`) | agent sessions that ask for callers of a handler and get nothing; also the probe in which a module-level registration's entry edge goes to an unrelated nested function instead of the imported handler |
| An MCP tool `nav_routes` | a decision on the navigation specs in PR #606 |
| Go receiver typing from a method's own receiver (`a.f.M()`) in call resolution, reusing 5.6 M1 (a) | the wave 2 counts of method values the route table binds and `nav callees` misses; needs a Tier-A and call-stats measurement of its own |
| Go interface-typed handlers as candidate lists | a nonzero `method_value_interface` count on a corpus a consumer cares about |
| Lane 5's producer adapters reading route records | lane 5's pilot being funded; this lane gives it registration records (adapter input, not a wired adapter) for the waves that ship, with the binding rule: bind an operation only to a `full` exposure with a resolved callback or delegate, and treat the callback list as ordered (#600 gets the same position semantics) |

## 9. Owner decisions

| # | Question | Default applied | Cost of changing later | Latest cheap point |
|---|---|---|---|---|
| 1 | Where routes live | a separate on-demand table; entry records and navigation edges untouched | caching the table later is a cache bump | before S1 |
| 2 | Exposure | CLI listing only | an MCP tool is a later slice with golden changes | after wave 1 |
| 3 | Wave 3 membership and order | by census count; object-literal route tables are included regardless of count because the private producer of lane 5 uses that shape | — | stage M |
| 4 | Flask-AppBuilder and similar class-based frameworks | out of scope | each is its own extractor | — |
| 5 | Executing fixture applications for truth | yes where the framework is already installed or installable offline; never anything from a corpus | hand-written truths only | stage T |
| 6 | Go method-value handlers (`r.GET("/x", a.questionController.List)`) | **decided by the owner, 2026-11-21: in scope.** Resolved by 5.6 from declared types (receiver, parameter, local, package `var`, up to two field hops) and the existing owner index; interface-typed receivers are counted, never a single handler; slice S4b. Replaces "unresolved until stage D decides otherwise" | resolving interface receivers to candidate lists is a later slice with its own consumer | — |

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
  method value (`c.List`) has no such fact; the records resolve it by 5.6 (owner decision 6), and nothing
  else in prism sees that result: `nav callers` still shows no edge into a method registered only as a
  method value (entry edges from route records are a section 8 follow-up).
- **Go method values left out** (5.6): interface-typed receivers, call-result and deeper receivers, embedded
  pointer or qualified hops, function-typed fields, package function values and method expressions, each a
  counted reason. None of them is a terminal callback on the census corpora; a corpus written against
  interfaces (handlers held as `http.Handler` fields) would show its cost in `method_value_interface`.
- **Typing the enclosing receiver is new** (5.6 M1 (a)): the call resolver does not type `a.f.M()` from a
  method's own receiver `a` either, so the route table can bind handlers that `nav callees` cannot. The rule
  reads only the method's declaration; carrying it into call resolution is outside this lane (a resolution
  change with its own corpus measurement).
- **Strict receiver and import proof costs recall** (factory apps, re-exported routers); each refusal is a
  counted reason, and the stage D recall floor is set knowing it.
- **Out of model:** routes added at run time in loops over data, plugin systems, reflection.
