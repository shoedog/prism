# Lane 12: routes as a first-class fact — design

Stage D design of record for the 420-row matrix at b924b349. Both ordinary design rounds have run. This amendment closes their recurring dispatch question under ORCHESTRATION §8. Amendment approval is pending. STAGE-D-ROWS specifies the bounded additions requested in review. Owner decisions 6 and 7 admit Go method-value handlers and the Hapi plugin adapter. Related work includes #600 and draft PR #606. Research record:
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
   it is installed (full for an eligible literal chain, with its dispatch constraints), the registered callbacks in order with
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

## 5. Architecture

### 5.1 Records and proofs

The unit is a source registration, including each decorator, builder verb leg or consumed table element. Its
source origin is (framework, file, start byte, end byte, installation site and element ordinal where applicable). For a Hapi plugin route, qualify that origin by the defining plugin object (file, object_byte), even on a strong receiver. The effective name, register function and formal are never substitutes for the object. A copy
or second mount adds an exposure to its origin. Two declarations with identical arguments and handler remain two
records. Include test sources, preserving their package/build partition.

| Record | Fields and meaning |
|---|---|
| Route | framework, site, receiver, protocol (http or websocket), methods, path, paths, callbacks. Keep a literal endpoint name for endpoint-only rules. Table records also carry the installation site and element origin. A candidate keeps its source, receiver status and reason. |
| Receiver | router (file, binding, construction byte) or unproven(reason). Anonymous constructor expressions use their expression text and first byte. A framework parameter, controller decorator, URLconf list or default mux may supply the identity as specified below. |
| Methods | `{"known":[…]}` sorted unique, `"any"`, `{"status":"unknown","text":…}`, or `null` for an omitted declaration or a WebSocket. Preserve case for ServeMux. An explicit empty list remains known empty. |
| Path | `raw` is the expression as written, `literal` its single decoded path if known. Arrays retain raw spelling and null scalar literal. Regex retains its source and flags. A ServeMux pattern retains the original method/host in raw. |
| Exposure | root identity (nullable), status full(value), suffix(value,reason), or dynamic(reason), normalized path or null. Store effective mount constraints on the individual exposure (host, method guard, active version selector where present). Full describes a certified installation in that root at the listing epoch. Binding also checks root health and effective dispatch (§5.2). |
| Callback | position (zero-based), resolved or anonymous identity (file, name, span), or unresolved(text,reason). Optional delegate identifies a forwarded argument. Hapi prerequisites also have role and group. An endpoint-only registration has zero written callbacks. |
| Mount | parent, child, site, one prefix (literal, absent, unevaluated with text), and any request guard/strip transform. One site can supply several edges. Object alternatives, branches and child arguments have stable source ordinals. |

**Values at a use.** Walk the lexical scope, including captures, to the reaching declaration. A straight-line assignment of a fresh router starts a new lifetime (ex29). A use before or after that assignment retains the corresponding identity. Resolve a direct, unwritten alias to that object. Distinguish factories and scopes, imports and package partitions. A conditional merge of different identities, a write of unknown order, an unknown alias mutation or an escaping mutable value cannot prove a single target. Only propagate an object through a factory with a unique direct return of a grounded local construction and a distinguishable invocation. Do not merge separate instances at one allocation site: qualify by the invocation when distinguishable, otherwise refuse. Retain a recognisable but unproven site with empty paths and no binding credit. Proven-absent execution, a settings read and ordinary middleware emit nothing.

Use direct named/default/namespace ES imports, unshadowed literal CJS require (including destructuring, property and
default interop), Python from/qualified/relative imports including package initializers. Require one eligible target
module and the requested *member* in it. Validate the export's reaching value and callable or router type. Follow
local export specifiers, direct module.exports and its own properties, observing replacement/detachment of exports.
A second module forwarding the value, star import, ambiguous export, or shadowed require supplies no proof. If such
a value controls a conditional mount, retain the syntactic unproven mount unless the condition is independently
decidable (es1-06). Do not use a file basename as an owner. An initializer declaring a router differs from one
re-exporting it (cx02, es1-15).

For path and method arguments decode native literal nodes (JS literal/template without substitutions, Python
string/raw and adjacent tokens, Go raw/interpreted). Fold only literal string concatenation; preserve interpolation,
value references and configuration as expressions. Literal arrays/tuples/slices expand where the API accepts them. A
mixed method set is wholly unknown. Separately, table keys, explicit bootstrap metadata and known router/callback
references have the narrowly typed binding rules below. Never run application expressions.

Resolve a callback **value at the argument**, never a fabricated CallSite. Same-scope declaration, direct proven
import/member of a module, or a bound-once local alias of that value may resolve. Use
ParsedFile::js_ts_site_binding, CallGraph::import_bindings and the actual js_ts_exports/local CJS terminal. A class
or object instance member, getter, computed lookup or bound function supplies no unique callable by name. Python
routes record the def (qualified name, first decorator line for runtime comparison) unless a non-route decorator
*below* can replace it. An outer decorator cannot change the registered value. A lambda and a Go func literal retain
their entire source/argument site as unresolved(anonymous_unindexed); only JS/TS inline functions use
anon_owner::identity. For Go bare arguments prefer the go_registrations fact at those exact bytes. Where no fact exists for a bare element of an immutable expanded slice or a package-initializer argument (including inside a proved immediately invoked func literal, never a stored closure), resolve at the original identifier in its defining file and lexical interval. Require a unique same-package, profile-visible free function declaration with its FunctionId. A shadow, non-alias variable, multiple visible definitions or uncertain build partition gives callback_unproven. An unwritten one-name alias may follow one explicit RHS under the same checks. Never borrow a fact from another argument or call (es5-06). Preserve ordered positions
even for unresolved middleware and error callbacks. Array flattening uses source order. A proved immutable literal
list spread or variadic slice expands; otherwise emit one spread_callback slot and retain subsequent arguments
(static positions count slots).

Closed delegate list: unshadowed imported `net/http.HandlerFunc(f)` (conversion, argument 0), and the default
callable export of the package `express-async-handler` (argument 0). Require one argument and a resolved or
anonymous terminal. Even there the callback is the wrapper_call and the delegate a separate identity. A local file
named async-handler, withApiKeyAuth, bind, a discarding/branching/nested wrapper, an arbitrary Python decorator and
a forwarding literal earn no delegate. A unique local bound to a wrapper result retains the wrapper classification.
Preserve its argument spelling at the use. Do not follow another call through a wrapper. Missing or ambiguous free
names use callback_unproven (import failures use imported_unproven), and instance members use member_handler.
Unproven receivers suppress callback identity but retain the callback slots and the receiver's refusal reason. For
Go selector values see 5.6.

**Reason** is one closed snake_case enum shared by records, exposure cuts, diagnostics and matrix validation. Its variants are
`receiver_unproven imported_unproven mount_child_unproven table_consumer_unproven no_known_mount variable_prefix
non_literal_path regex_path mount_cycle expansion_truncated`,
`callback_unproven member_handler wrapper_call inner_decorator class_view anonymous_unindexed spread_callback handler_object`,
`method_value_receiver_unproven method_value_receiver_shadowed method_value_method_unproven method_value_interface
method_value_external func_field_value qualified_function_value method_expression`,
`hapi_registration_unproven hapi_plugin_unproven hapi_plugin_unregistered hapi_server_param_unproven
hapi_route_not_literal hapi_handler_object hapi_handler_conflict`,
and `runtime_generated_registration unsupported_registration build_only superseded dispatch_conflict entry_unproven`. All query errors, including malformed selectors, missing exports and ambiguous roots, use entry_unproven. A diagnostic has a site and reason, not a
fabricated path. Accepted-loss descriptions in the matrix are audit notes; the enum is the wire contract.

### 5.2 Composition, ordering and comparison

Start from the selected application/URLconf/module root, or independent outermost application constructors when no
entry is selected. The optional CLI/API entry in 5.4 selects one statically bound app, factory return or URLconf. A mounted sub-app is seen through its parent; select it separately to query it as a root. Count a
framework controller or plugin with only local route proof as such. A literal leaf on an unattached router yields a
suffix with null root and no_known_mount. Unknown prefix retains the longest literal tail below it; an unknown leaf
is dynamic. A determinate non-dispatching declaration (build_only) has no paths. Distinguish a known impossible
chain from an unknown one. Never promote a disconnected suffix to a served path. Keep the root when a known root has
an unknown prefix.

Use a literal exported app, bootstrap/factory return, selected URLconf or externally bound oracle root to select
setup context. A unique unmounted constructor can root a standalone registration fixture; several independently
evidenced roots stay distinct. Importing a callable as a callback never executes its body during setup. A literal
assignment to a framework settings key may establish a boolean setup guard (et3-02, es6-07). An unknown setup
condition on an otherwise grounded route can yield a possible registration. Carry its text as setup_guard and
exclude it from unconditional schema credit. This differs from unknown root or module membership, which yields only
a suffix. A copied router's source app is not another served root merely because the constructor appears in the
repository (es3-09). A root on the wire is {file,binding,construction_byte,invocations}. invocations is an ordered outer-to-inner list of {file,start_byte,end_byte} call sites; the selected factory's implicit call uses {entry:"module:factory()"} as the first frame. Module-scope allocations use []. A uniquely inferred setup function can be analyzed once as its declaration context; distinguish subsequent source calls by their frames. Ambiguous instantiation never earns a shared full root. Use the same identity on receivers and mount endpoints when instance qualification is needed.

Apply mount semantics to a registration *event*, not just a pair of router names. Express live routers see later
declarations. Flask replays blueprint setup at registration (record_once once per blueprint identity per parent
app); FastAPI include copies the source's then-current entries with origin site and ordinal, including nested
copies. ASGI mount delegates live. Django include of a list captures that object; a module resolves its current
urlpatterns. Nest discovers metadata after bootstrap. Hapi registers a plugin in its realm and schedules only known
lifecycle hooks. Use source order inside a setup invocation, import initialization and uniquely ordered setup calls
across files. If order or membership affects installation and cannot be proved, retain the source and only independently justified partial facts. A copy can never
acquire later origin sites. Each declared origin survives even when the installation policy below removes its last exposure.

The **one-call bound** transfers the exact router argument at a direct, statically selected installer call to its
plain unwritten formal, for a named router or inline grounded construction (gn04 also admits the plan's inline
variant). Alternatives from distinct proven callers compose independently. A second helper does not transfer a
prefix. A typed gin group then retains the local suffix (gn05); an untyped argument needs its own receiver proof.
API-defined registration callbacks (chi.Route, Hapi register, Nest module imports) are framework edges, separate
from the single user helper. An indefinitely deferred closure cannot install a route merely by capturing the server.

Compose every supported literal alternative, retaining parallel static and dynamic branches. Visit live mounts
backward per chain, stopping a repeated identity with mount_cycle and the accumulated suffix; snapshot edges visit
earlier event versions. Retain at most 64 distinct exposures plus one expansion_truncated suffix, with all mounts
retained. Traverse in stable root/file/site/alternative order, exact literal branches first, breaking ties by byte
range; avoid materializing the Cartesian product past the bound. Equal exposures (root, status, value, constraints)
coalesce only within one object-qualified origin, and only when their setup, dispatch and binding restrictions agree. Tables can be installed twice at different sites.

| Framework | Join and comparison grammar |
|---|---|
| Flask | Registration prefix overrides blueprint default, None/absent inherits, empty string overrides. Compose nested prefixes, strip boundary slashes and insert one unless leaf empty (prefix alone); slash leaf retains slash. Subdomain override composes child.parent under the configured server name. Keep host and strict-slash metadata, only the written pattern. Single-segment Werkzeug converters normalize to {}, tail/custom grammar gets null. Colons and braces are literal. |
| FastAPI/Starlette | Concatenate include prefix + router prefix + leaf verbatim, honoring the empty leaf and final slash. Root path is ASGI deployment metadata. Only valid named Starlette parameters, including built-in single-segment converters, normalize to {}; path tails give null. |
| Express/Nest | Join at the mount boundary, keep a slash leaf for Express. Nest inserts one slash between nonempty global, URI version, module, controller and method segments, defaults to /, strips terminal slash as Nest does. Keep raw patterns. A simple :name maps to {}; optional, repetition, wildcard, inline regex or Express string-pattern metacharacters give null. Regex leaf is dynamic(regex_path); regex mount retains the child suffix. Never collapse interior slashes to score. |
| gin/chi | Gin uses path.Join cleaning with the framework's terminal-slash preservation; empty relative path leaves the base. :name is one segment, *tail is unkeyed. Chi composes Route/Mount at a boundary, Group/With add no segment. {name} keys as {}, {name:regex} and * have null keys. |
| net/http | Parse Go 1.22 [method SP/HTAB] [host]/path, split segments before unescaping; preserve encoded slash/braces and CONNECT path spelling. {id} → {}, {tail...} and trailing subtree / → null, final {$} → exact trailing slash. Intersect host and method conditions through mux delegation; invert StripPrefix with its actual first argument and then apply the outer pattern (es5-09, es7-01). A direct mux child receives the original URL. Store the outer pattern as guard_pattern. A router delegated without stripping has absent prefix; StripPrefix stores the strip operand as prefix, independently of the outer subtree guard. If two dynamic pieces cannot be compared, retain a suffix with variable_prefix. |
| Django | Consume parent match, append child remainder verbatim, prepend the request's leading /. Preserve trailing slash; str/int/slug/uuid placeholders → {}, path and custom converters → null. Anchored regex made solely of literal path characters (escapes decoded as regex literals) may contribute an exact prefix; an end-anchored include permits only an empty-matching remainder. Only an exact anchored literal terminal gets an exact key. Record the regex source in guard_pattern, decoded literal prefix in prefix.value, and end_guard=true only for a terminal anchor. A known nonmatching remainder yields zero exposures along that edge; retain its declared URLPattern and callbacks (es3-24). Other regex leaves are dynamic(regex_path), unanchored/complex regex mounts keep a suffix with an unevaluated prefix. |
| Hapi / object-table | With a valid literal realm prefix (empty or a Hapi prefix beginning with / and without a trailing slash), concatenate prefix and route verbatim, except a slash-only child under a nonempty prefix gives the prefix. Empty prefix preserves the leaf including /. {id} keys as {}, optional, wildcard and counted wildcard are unkeyed. Object-table's certified dispatcher has literal key semantics; concatenate its proven prefix as its code spells it. |

**When registrations interact.** Replay setup events (including mounts) through the requested epoch for each native dispatcher instance. The replay starts at entry to a framework call with evaluated arguments; arbitrary exceptions or side effects while computing those arguments are outside this model. Compare native request sets and installation resources, including endpoint names, tree slots and plugin names. Distinct comparison tokens neither prove disjointness nor define native equality. Apply the bounded certificates in [the adapter decision table](STAGE-D-ROWS.md#dispatcher-interactions).

Retain every origin, callback and plugin association. Give a key a full exposure only if its installation survives in every setup consistent with the declared facts on that branch. An independently certified setup guard stays attached. A known rejection or overwrite removes that key. An undecidable install, including a later event behind a possible setup abort, withholds full and binding on the affected branches.
A proved abort stops that setup sequence; do not mark a never-attempted sentinel as a second failure. Keep independently proved suffixes or dynamic facts with their reasons and record the cut. Preserve an earlier certain install as a historical installation on its root.
A failed or possibly aborted setup has no proven bindings on that root. Isolated roots and independently completed alternatives keep their own evidence.

When native semantics establish co-installation (ordered layers, for example), full describes the installed pattern. For each request method, unique binding additionally requires a proved choice among **all** effective native matches, including mounts, fall-through and default methods. Mark contested methods on the exposure even if only one contender declares the schema operation. Set shadowed=true only for exclusion over the whole represented request set. When only part of a pattern loses and the overlap cannot be encoded, block that method on the affected exposure; keep the winning operation where independently proved. Exclude failed or uncertain roots from served-schema credit as well as bindings. Unmodelled native relations take the undecidable branch. The table gives the alternatives, measurements, residual losses and pair-generation procedure used at the review cap.

For scorable paths emit path_tokens as a sequence of ["literal",text] and ["parameter"] entries, coalescing adjacent
literal text, or null for an unscorable grammar. Render that sequence as normalized for display (substitute {} for a
parameter). The oracle uses path_tokens and validates it against value with the adapter grammar. Schema tokens are
parsed with the schema grammar independently. Literal braces do not become parameters on a second parse. The
comparison tuple is (root, protocol, method, host selector, active version selector, path tokens). Path templates
that cannot be expressed as single segments have null normalized even when their spelling is full. For host
matching, represent absence as unconstrained, a literal host or list as exact alternatives, and a
parameterized/regex/unknown selector as constrained and unscorable by this oracle. Compare DNS names in ASCII
lowercase, after applying the framework's port handling (ServeMux strips the request port for ordinary methods).
Restrict Flask explicit host/subdomain, Hapi vhost and Nest @Controller host to a matching authority in the frozen
schema scope. ServeMux's host comes from the pattern, never from splitting a slash inside an escaped segment. Other
adapters have no intrinsic host selector in these rows. A header version requires explicit matching header/value
evidence; media/custom or dynamic selectors without scope evidence are unscorable. Inactive Nest metadata serializes
as null version. Without matching scope for a constraint, report unsupported; do not strip it to get credit.
Comparison is of registered patterns, not generated redirects, fallback handlers or language equivalence between
regexes.

### 5.3 Registration and mount adapters

#### Flask

Import-bound Flask/Blueprint. route, get/post/put/patch/delete decorators, add_url_rule positional or keyword
rule/view_func, register_blueprint including keyword child and record_once setup-state add_url_rule. Shortcut
declares its verb. Omitted methods remain null (usual effective GET/HEAD/OPTIONS); as_view, required_methods and
provide_automatic_options may alter dispatch. Endpoint text alone adds no callback; a later @endpoint is no second
registration. Preserve a build_only declaration with empty exposures. Stored decorator application and
data-generated registrations are accepted losses.

#### FastAPI

FastAPI/APIRouter and the app's built-in .router. GET, HEAD, POST, PUT, DELETE, PATCH, OPTIONS and TRACE decorators,
api_route/add_api_route, websocket/add_api_websocket_route, compatibility route/add_route, include_router and mount
of a grounded ASGI app. Known empty method set has a registered path and zero HTTP operations. Omitted defaults are
reported separately from declaration. WebSocket protocol and null methods. Dependencies and include_in_schema do not
add/remove declarations. Opaque StaticFiles and constructor-seeded APIRoute are outside this source API.

#### Express 4

express() and Router() (including direct imports). Use the 36 spellings from the Stage M method table: acl, all,
bind, checkout, connect, copy, delete, get, head, link, lock, m-search, merge, mkactivity, mkcalendar, mkcol, move,
notify, options, patch, post, propfind, proppatch, purge, put, query, rebind, report, search, source, subscribe,
trace, unbind, unlink, unlock, unsubscribe. Accept literal bracket spellings and the del→DELETE alias.

Represent all as framework Any. Every verb leg, repeated/saved Route, router-returning verb chain, bound verb with
same proven receiver and optional call on non-null receiver. A definitely null receiver, empty builder, get setting
read, param, static and middleware-only use add no route. use overload inspects its first flattened item, supports
pathless lists, path arrays and several children mixed with middleware.

Emit one mount per (child, prefix alternative) at the same site. A router as a verb callback is a handler_object;
secondary dispatch through it is an accepted loss.

#### gin

gin.New/Default, their embedded RouterGroup and chained Group expressions. Uppercase GET, POST, PUT, PATCH, DELETE,
HEAD, OPTIONS; Handle(method,path,…), Match(method slice,path,…). Any expands gin's finite
GET/POST/PUT/PATCH/HEAD/OPTIONS/DELETE/CONNECT/TRACE set. Group is a mount. Use, NoRoute, NoMethod and static-file
helper internals add no written verb registration.

#### net/http

An unevaluated pattern has unknown methods and a dynamic leaf. A parsed pattern with no method is Any.

Package Handle/HandleFunc, DefaultServeMux, or a proven ServeMux, tracking straight-line replacements at use.
Handle(func object) keeps one handler_object slot; explicit HandlerFunc conversion follows 5.1. A grounded mux child
(direct or through StripPrefix) emits a guarded mount only. GET on a mount admits GET and HEAD; path guards with
empty intersection yield no child exposure. Omit implicit redirects.

#### chi

github.com/go-chi/chi including versioned imports. NewRouter/NewMux,
Connect/Delete/Get/Head/Options/Patch/Post/Put/Trace, Handle/HandleFunc (Any), Method/MethodFunc (literal method,
including registered extensions), Route, Mount, Group and With. Group/Route closure parameters ground to their
lexical child, With retains the receiver. A single-return mux factory may ground a child. Opaque http.Handler under
Mount retains an unproven child, no invented subtree.

#### Django

Proven django.urls path/re_path (including aliases) selected into urlpatterns; a nested include is a mount at its
enclosing path/re_path. Follow literal module names/imports, actual list identities including +=, inline lists,
(patterns,app_name) include and resolver triples. Namespace has no path effect. Each URLPattern accepts Any at this
layer, including guarded views and as_view. Omit discarded descriptors.

Take explicit ROOT_URLCONF (settings.configure or bound settings module), else the unique outer urls.py as a URLconf
root in a standalone fixture. Ambiguous outer roots give suffixes. Evaluate literal DEBUG guards and i18n_patterns
with fixed USE_I18N, LANGUAGES, LANGUAGE_CODE and prefix_default_language; enumerate distinct locale paths, no
synthetic mount. Unknown locale prefix gives suffix; translated leaf dynamic.

Request-time URLconf selection and settings chosen by environment have no certified additional root/mount.
APPEND_SLASH and view opt-out add no records.

#### NestJS

Import-bound @nestjs/common and @nestjs/core. NestFactory.create(module) reaches @Module imports/controllers,
literal DynamicModule result (also async direct return), and forwardRef(()=>M); visit each module/context once. A
conditional controllers array retains possible controller as suffix, a definitely unused controller is omitted.
RouterModule.register paths/children compose per module context. Effective @Controller string, array or
{path,host,version}, nearest inherited method unless overridden, @Get/Post/Put/Delete/Patch/Head/Options/All/Sse
(GET) and @RequestMapping using literal RequestMethod.

Last metadata write in decorator application order wins. A local decorator factory and redirect metadata create no
separate route. Cross-product of literal paths, setGlobalPrefix and method-sensitive exclusions; apply active URI
version (method > controller > default, neutral omits, explicit prefix or false), or active header/media/custom
constraint without a path segment. Inactive @Version adds nothing.

Dynamic global prefix gives suffix. Controller mounts carry effective module/controller path; expansion for
versions/global exclusions belongs to the exposure.

#### Object tables

Label object-table. Recognize a literal row only at a consumer: Hapi via 11, or an in-repo direct installer with a
structural certificate. Follow the installer binding, verify rows iterated/destructured or mapped into a native Map
of (method + space + optional literal call prefix + path) → that row's handler, and verify lookup on that same map
with the corresponding method/path key invokes the retrieved value in its dispatch function.

Includes per-method array iteration and Map(entries). Reject coercing getters, arbitrary writes/escape or different
keys/map. This is a small syntax matcher, not general interprocedural flow. Read one local or direct-import table
literal at its definition scope. Replay static own properties/spreads in JS evaluation order. An opaque write
poisons only properties it may override. A passive read alongside a certified installation does not duplicate it.

For a recognised route-shaped call (bare or member route/install with method, path and handler fields), failure to
prove the consumer yields an object-table candidate. Unused data, client request descriptors and unrelated count
calls are silent. An adapter may claim a site only once: Hapi first if grounded, otherwise the generic candidate.
For object-table the public site is the installer call plus element_site and element_ordinal.

For Hapi the site is the element plus install_site and element ordinal. This also specifies the ledger crosswalk for imported rows.

#### Hapi

Direct server rows share the object reader. Plugin realm/admission, callbacks and lifecycle are specified in 11.

**Open class from E.** Sol was still adding 9–10 rows per round. The recurring variable is whether a source site contributes a registration in a particular setup state. Use the product of origin identity, active lexical binding, installation phase, known/unknown guard, root/realm and request constraints. Transfer only reference-defined operations in the table. For unknown inputs keep the common proven binding/path or a partial record; definitely inactive branches produce no exposure.

On an installer role outside the adapter's closed setup operations, retain only evidence established by an
independently grounded origin. It earns neither a new mount nor unconditional root credit. Do not treat nested
source containment as execution. No user code is executed by extraction. Test the rule with a deterministic
Cartesian generator in S1 and adapter-specific instantiations: vary two origins (including equal text), two roots
and two setup epochs, true/false/unknown gate, and live/snapshot/deferred installation.

Compute expected events by replaying small finite lists per root, independently of the product graph traversal.
Check origin multiplicity, visibility after each epoch, constrained methods, no cross-root credit, and stability
under alpha-renaming and reversing independent events. Source matrix E stays closed.

Examples: es6-01/02 deferred registration versus escaped es4-14; es6-03 an external loader; es6-04/05/06/07 request
root, settings, locale and DEBUG; es6-08/09/10 awaited, conditional and cyclic modules. es7-01/02 distinguish outer
method guards and encoded segments; es7-03/04 separate lexical interval and package identity; es7-05 excludes
redirects; es7-06 binds imported register; es7-07 distinguishes building a URL from dispatch; es7-08 preserves
duplicate snapshot origins; es7-09 applies only active versioning. STAGE-D-ROWS.md fixes each of the 420 row
projections and identifies accepted losses.

### 5.4 The listing

`prism nav routes --repo <dir> [--entry <entry>] [--mounts] [--format json|text]`. Accept the fixture entry forms (Python module:binding or module:factory(), JS/TS repo-relative file selecting its default/app export, or file#export for a named app) and a URLconf module:binding. An entry may also be a serialized 5.2 root identity for a statically evidenced app/dispatcher instance in any adapter. Resolve within the repository without executing setup. Invalid or uncertifiable explicit selections return a query error (entry_unproven for a binding without an app proof). Selection cannot ground an unregistered plugin formal or fabricate an invocation. The selection fixes served-root context while independently known orphan declarations remain reportable. One JSON line per record, optional wrapped mount
lines and a final wrapped summary. Explicit selection failure returns one query error and no successful listing.

**Wire fields.** Every row below gives the JSON type and its absence rule. Required arrays may be empty. A site is {file,line,start_byte,end_byte} in the repository (one-based line, zero-based half-open UTF-8 bytes).
A router identity is {file,binding,construction_byte,invocations}, with 5.2's ordered call-site frames or initial {entry} frame. An identity for a callable is {file,name,start_line,end_line,start_byte,end_byte} at its declaration or anonymous expression. Byte spans distinguish same-line functions.
A method set is `{"known":[strings]}` (sorted, unique), `"any"` or `{"status":"unknown","text":string}`. Reason always uses 5.1's closed enum.

| Field | Type and values | Absent or null | Emitter |
|---|---|---|---|
| route.framework, protocol, site | adapter name from 5.3, http/websocket, Site | required | all |
| route.element_site, install_site, element_ordinal | Site, Site, uint (zero-based). Object-table site is install, Hapi site is element. | include ordinal and complementary site for table elements, omit elsewhere | object-table, Hapi |
| receiver.status, receiver identity or reason | router + router identity, or unproven + Reason | disjoint required variants | all, also mount endpoints |
| receiver.plugin | {file:string,object_byte:uint,name:string or null} | required on strong and weak plugin associations, omit on direct and unproven receivers. Null name means unproved effective name. | Hapi |
| receiver.proof | plugin_contract | weak only, omit on strong and unproven receivers | Hapi |
| methods, default_methods, endpoint | method set or null declaration, known default set, endpoint name string | methods required on a known row shape, null for omitted/WS. Omit default unless independently proved and endpoint unless written. | all, defaults on defaulting HTTP APIs, endpoint on Flask |
| path.raw, path.literal | expression string (regex includes spelling/flags), decoded string or null | required on a known row shape, null literal for array/regex/dynamic. Omit path and methods on opaque items with no path/method fields. | all |
| candidate, reason | boolean, Reason | candidate omitted means false. Reason present on invalid, withheld or fully superseded records as applicable. | all |
| superseded_keys, conflicts | arrays of {root,method_guard,path,site,reason}. Root identity or null, method set, native pattern string or null if unknown, competing write/event Site, superseded or dispatch_conflict. | omit empty arrays. Preserve partial cuts alongside surviving paths. | Map/chi writes, all contested installs respectively |
| paths, callbacks | ordered arrays of Exposure and Callback | required, including [] | all |
| exposure.root, status, value, reason | router identity or null, full/suffix/dynamic, string, Reason | root required (null for an orphan), value on full/suffix, omit on dynamic. Reason required for partial/dynamic, omit on full. | all |
| exposure.path_tokens, normalized | array of literal/parameter tokens (5.2), display string | both required, both null for unscorable path grammar | all |
| exposure.host_kind, hosts | unconstrained/literal/parameterized/regex/unknown, array of written host strings | required. [] means no known strings. Only unconstrained proves no host restriction. | all, constrained on Flask/Hapi/Nest/ServeMux |
| exposure.method_guard, binding_blocked | method set for effective mount/write restriction, known method set or "any" ineligible for binding by 5.2 | method_guard required, "any" when unrestricted. Omit binding_blocked for no contested methods, use "any" if the blocked subset cannot be represented safely. | all |
| exposure.version | {kind,header,value} for header (strings), {kind,value} for media/custom/unknown (text or null) | null when no active non-URI selector, URI version already in path | Nest, null elsewhere |
| exposure.strict_slashes, setup_guard | boolean or null for unknown policy, source expression string for a conditional install | strict_slashes on Flask, setup_guard only when conditional. Guarded exposures earn no unconditional credit. | Flask, any conditional adapter |
| exposure.shadowed | true/false for proved first-match loser/winner, null when priority unknown | omit when no priority comparison applies | ordered adapters |
| exposure.mount_refusal | hapi_registration_unproven for linked failed item, no_known_mount for no known chain | required on weak exposures only, distinct from path reason | Hapi (§11.5) |
| callback.position, status, identity, text, reason, delegate | uint, resolved/anonymous/unresolved, callable identity fields, source string, Reason, callable identity or null | position/status/delegate required. Identity on resolved/anonymous, text/reason on unresolved. Null delegate means no certified forwarding. | all |
| callback.role, group | pre/handler, zero-based stage uint | roles on Hapi callbacks, group only on pre | Hapi |
| mount.parent, child, site, prefix | receiver, receiver, Site, {status:literal,value:string}, {status:absent} or {status:unevaluated,text:string} | required on each wrapped mount | all mounting adapters |
| mount.child.realm | ordered array of {file,start_byte,end_byte,item_ordinal}, outermost first | required for admitted plugin realm, omit otherwise | Hapi |
| mount.guard_pattern, end_guard, method_guard | original pattern string, boolean for terminal anchor, method set | pattern on guarded mount, end_guard on Django regex, method_guard on method-filtered delegation | Django, ServeMux |
| mount.host_kind, hosts, setup_guard, transform | same host classification and conditional expression as on an exposure; transform is identity or strip_prefix | emit host pair on host-aware mounts and setup_guard on conditional edges. Emit transform on ServeMux delegations; with strip_prefix, prefix is its actual argument. | Hapi, ServeMux and Flask host-aware edges; conditional mounts across adapters |
| mount.alternative_ordinal, child_ordinal | uint, zero-based source order | use 0 for singleton | mounting adapters |
| summary.registrations, proven, unproven, candidates, mounts | uint counts. registrations = proven + unproven = route lines. candidates is the candidate:true subset, mounts counts edges regardless of display. | required, zero allowed | all |
| summary.diagnostics, failed_roots, uncertain_roots | arrays of {site,reason}, {root,site,reason}, {root,site,reason}. Root is a router identity. Distinguish a proved setup abort from a possible one. | required, [] when clear | all |
| error.reason, error.entry | entry_unproven, original selector | required on query error only | CLI/API |

Methods on the route are declarations. Compute a GET default only on a defaulting API when view metadata allows it. For Flask inspect literal methods, required_methods and automatic-options settings. Native fallback affects selection even where it creates no schema operation. Count only independently declared/default operations. Section 11.5 specifies downstream use of weak evidence.

Sort routes by first path key/spelling, method, file, site offsets, install site, element ordinal and plugin object identity. Sort mounts by source site and alternative/child ordinal, paths by full/suffix/dynamic then spelling, root and constraints. Compare collections by origin and test sorting independently. Count every unproven receiver as a candidate, along with grounded invalid/opaque elements. Each failed register item has a distinct diagnostic. Unsupported source shapes may have only a diagnostic. Text output preserves the same statuses, reasons and constraints.

### 5.5 Where the code goes

```
src/framework_entries/routes.rs            records, `routes(files, call_graph, manifests, entry: Option<RootSelector>) -> Result<RouteTable>` (no dependency on navigation)
src/framework_entries/routes/receivers.rs  receiver grounding and `RouterId`
src/framework_entries/routes/handler.rs    callback resolution shared by the frameworks
src/framework_entries/routes/python.rs     Flask and FastAPI
src/framework_entries/routes/express.rs
src/framework_entries/routes/go.rs          gin and shared Go arguments
src/framework_entries/routes/{go_nethttp,chi}.rs   ServeMux and chi installation
src/ast/js_router_binding.rs                route-specific JS/TS declarator lookup (S3)
src/framework_entries/routes/go_method_value.rs  Go selector callbacks (5.6); reads call-graph facts, adds none
src/framework_entries/routes/mounts.rs     mount graph and composition
src/framework_entries/routes/{django,nestjs,tables}.rs   wave 3 adapters and shared literal row reader
src/framework_entries/routes/hapi.rs       Hapi plugin adapter (section 11, owner decision 7)
src/navigation/routes.rs                   the listing (no logic in queries.rs)
crates/prism-review/src/cli.rs, cli_nav.rs    NavQuery::Routes and its arm
```

The navigation adapter has the repository root in `NavigationSession.repo.root` (LoadedRepo.root). For a prospective weak Hapi plugin it walks from the source directory up to that root to read the nearest package.json, once per directory per listing. Supply immutable manifest facts to the route-table entry point. A missing, malformed or out-of-root manifest cannot certify that dependency. LoadedRepo.manifest_snapshot presently admits Cargo.toml, go.mod and go.work. The SH reader supplies the missing package.json evidence without expanding the loader snapshot or changing its cache key. Keep this input local to the query and out of the CPG/cache.

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

Stage M counted the following non-test terminals. Apache Answer 232 (211 enclosing-receiver field, six locals bound to a field, nine bare functions, three literals, three wrappers). Miniflux 179 (150 composite locals, one interface-typed call-initialized local, 14 immediate wrapped method values, five other wrappers including one nested method value, two bare functions, three function variables/fields, one literal, three other handler values). All 217 and 150 admitted terminal selectors have a unique direct declaration in the independent census. Three package functions and one wrapper also occur in non-terminal Apache positions. The five-corpus second look includes 80 registrations on Prometheus's own router. The 199 Apache annotations have distinct schema keys and spans. These are shape counts, not binding receipts.

**Scope refinements at D.** Confirm M1–M4. First determine which declaration Go binds *at the selector*: check package path/declaring directory, receiver, block, closure, if/switch initializer lifetime and package/test visibility.
A package qualifier must actually denote the import at this use (es1-47), an expired if shadow cannot poison the outer declaration (es7-03), and two identical package basenames cannot substitute for the import-path owner (es7-04). Same-block short redeclaration preserves the old variable; a new inner declaration triggers the shadow refusal (es5-07). The route-local scope check qualifies the classifier's evidence; a whole-function count alone cannot establish lifetime.
Prefer explicit static type to initializer (es4-09): an interface never acquires the concrete initializer's method. An alias follows the type it denotes. A distinct defined type has its own method set (es5-08).
Refuse instantiated owners at the base, an explicit field, or a potentially supplying embedding (et3-03, et4-01, es4-10), and type parameters (es1-46), with method_value_receiver_unproven. Raw declaration keys retain type arguments. The census found zero generic terminals. Consult exactly one visible method at minimum embedding depth, or refuse.
Production, same-package test and external test partitions stay distinct; incomparable build-tag alternatives give method_value_method_unproven (et4-02/03, es1-48).
Local proofs may read the exact Go declaration, return slot or asserted type from syntax where the summarized fact omits that detail (es1-37, es2-19, es3-13); a multi-type switch case keeps its interface type (es3-16). Read inferred var-composite types and direct concrete return slots at their declarations.
Parenthesized/unary operands, arbitrary conversions, anonymous-struct field bases, range-variable inference and a concrete type-switch implicit variable remain outside M1 (es1-42, es3-14/15, es2-18/16). No general Go inference is introduced.

- **M1, receiver.** `X` is one of:
  (a) the enclosing method's own receiver, typed by the receiver parameter of its `method_declaration`
  (`Language::go_receiver_var` names it; the type is read from the same parameter, pointer peeled) and
  placed by `resolve_go_receiver_owner_identity`; screened for a live shadow as above using declaration spans and the literal-parameter check (the general classifier gates the enclosing receiver out, section 2);
  (b) a parameter, a local or a package-level `var` typed by the existing classifier,
  `classify_go_receiver_expanded_with_partition` followed by `screen_go_receiver_prerequisites`, called with a
  `GoReceiverCtx` at the selector's own bytes and `ReceiverRecoveryConfig::default()` (typed parameters,
  composite-literal and `NewX` constructor locals, `var x T`, type assertions, call-initialised locals
  through `go_return_types`, package `var`s through `go_package_vars`; a recovery it marks shadowed refuses);
  (c) one or two field hops `b.f`, `b.f.g` from a base admitted by (a) or (b), each hop typed by the field's
  declared type through `resolve_go_struct_field_owner` over `CallGraph::go_field_types` (for a (b) base the
  classifier's own nested-selector walk does this; for an (a) base the new module runs the same per-hop
  call). Explicitly test the selected field on **both** base paths: embedded && (pointer spelling || qualified spelling) refuses method_value_receiver_unproven before advancing. The field-owner helper alone does not enforce the classifier's requires_local_struct_proof check (gm17, es1-40). Ordinary named pointer/qualified fields remain eligible; implicit promotion is M2;
  (d) a local declared exactly once (the count `receiver_type_in_fn` reports) by a one-name
  `x := <(a), (b) or (c) form>`, typed as that form; one level, never a chain of locals (new: the classifier
  recovers a local only from a constructor, `var` or call initialiser, so a field-chain initialiser has no
  fact today; the new module reads that one `short_var_declaration` from the syntax).
  These steps and the local scope/declared-slot checks above read only declarations the source spells.
  Anything else (a call result `f().M`, an index `m[k].M`, three or more hops, a local declared by another
  form or more than once, a type parameter, a prerequisite drop) is `method_value_receiver_unproven`, and a
  refused shadow is `method_value_receiver_shadowed`.
- **M2, method.** Resolve direct methods through go_method_declarations[GoOwnerIdentity] by name, declaring file, exact_declaration_visibility, pointer/value method-set applicability and addressability. Return the unique declaration's function_id. Read field, struct and interface declarations under the same package clause, import path and file profile. Require exported access when crossing package boundaries, including the selected member. Consult go_concrete_receiver_route for supported direct lanes; a bare-name promotion bucket, a snapshot hit or a missing router lane cannot prove the declaring owner.
- For promotion, use a route-local breadth-first walk of visible declared embedded fields. Resolve each type in the embedding declaration's file/imports; look up the method in the **embedded owner's** package/profile. Admit local value (gm08), local pointer (es1-39) and qualified non-generic embedding (es1-40); ordinary named fields never promote. Preserve distinct embedding paths at each depth, cut cycles per path. A visible direct member ends the search at depth zero. A unique shallowest selector wins over deeper names (es1-41). Consider fields and methods together, including interface suppliers. Two paths at the winning depth, uncertain visibility, an unknown competitor at that or a shallower depth, or no selector cannot certify a function. Never skip an unknown branch in favor of a known sibling. Use method_value_method_unproven for ambiguity/missing method and the receiver refusal above for an unresolved instantiated type.
- Refusals in order after lexical selection: a proven external package owner (import directory absent locally) gives method_value_external, including Miniflux's http.Handler; a local interface or promoted interface lane gives method_value_interface, and a function-valued field gives func_field_value. No implementer candidates. Apply external precedence also to declared return slots/field hops. A qualified type imported from another local directory uses that directory's declarations (gm01/02, es1-38, es7-04). Build/test alternatives must resolve uniquely in the caller's partition.
  If the router skipped a qualified or instantiated embedding, run the complete minimum-depth declaration walk **before** accepting any router refusal, including EmbeddedInterfaceDispatch. A shallower qualified concrete supplier wins; an unresolved generic competitor at or above the winning depth refuses. Where the router considered all possible suppliers preserve its refusal lanes: InterfaceDispatch, EmbeddedInterfaceDispatch and R2OnDemandNameCollisionBail withhold identity with method_value_interface; FuncValueField gives func_field_value and ExternalNewRecoveryDrop gives method_value_external. ConcretePromoted/Deferred are hints. A missing promotion lane can be recovered only by the complete declaration walk above; otherwise ConcreteNoSelector/Unproven gives method_value_method_unproven.
- **M3, the other selectors.** `pkg.F` is `qualified_function_value` and a method expression is
  `method_expression`: both are listed unresolved by M3. A method expression takes an explicit receiver and **can** have a valid handler signature (gm15, (*Ctrl).Abort with Ctrl an alias of gin.Context). The refusal describes the proof boundary, not a compile error.
- **M4, scope.** The lookup is read only by the route table: it creates no `CallSite`, no edge and no cached
  fact, leaves `go_registrations` and call resolution as they are, and adds nothing to `src/call_graph.rs`
  or `src/resolution.rs` (every item it reads is already `pub` or `pub(crate)`). A forwarded argument of 5.1's closed list resolves by M1–M3; gm19's local wrapper has no delegate.

The exclusion counts and admitted-shape gates are in 7. Comparison to a real call on the same concrete receiver is a differential test, never the method-value lookup itself.

## 6. Measurement

**Census (stage M).** For each framework, on the corpora that use it: registrations by shape (an independent
regular-expression and syntax count), handler form (named, inline, wrapped, member, imported), where the
prefix comes from (none, literal on the router, literal at a mount in another file, variable,
configuration), and for the wave 3 candidates the counts that decide the order. The syntax pass also writes
the **site ledger**: one line per registration or mount site (corpus revision, file, start and end byte of
the decorator, call or builder-leg node, kind, shape), frozen at stage M. It is written independently of
prism and is the completeness denominator (section 7).

**Schema oracle.** Three corpora commit an API description next to their code (`apache-answer`, `directus`,
`cal.com`, checked by the admitted NestJS slice). Each schema is bound
before scoring to its application root, revision, API base and inclusion rule; only exposures under that
root and base are scored. Operations are (method, exact path) keys from the expanded method sets; a `full`
exposure matches only an identical path (no trailing-slash or `//` collapsing, unless the framework is
independently documented as slash-insensitive there); a `suffix` matching exactly one operation is a unique
candidate, one matching several is ambiguous and credits none; `Any` methods are reported apart. The oracle
reports each numerator and denominator (exact full, unique suffix, ambiguous, unsupported) and classifies
every difference (dynamic prefix, unsupported shape, schema drift, outside the bound scope). Schema keys
cannot see a handler swap, so where the schema names its handlers the bindings are compared too:
`apache-answer`'s swag `@Router` comments (199) sit on the handler functions (192 of them methods registered
as method values, 5.6), so each matched operation’s annotation is checked against its terminal callback or certified terminal delegate, independently of the all-selector declaration join in section 7. FastAPI fixtures need no
generated schema: their executed truth already binds each route to its endpoint.

**Executed truth for the matrix.** Matrix rows are small lane-written applications, run only with the pinned
offline dependencies (owner decision 5; never a corpus application). The runner instruments the
registration calls themselves (Flask's and `Blueprint`'s `add_url_rule`, through which every Flask decorator
passes; FastAPI `APIRouter.add_api_route`; Express verb, `route` and `use` methods) before loading the row,
recording per call its site (the first stack frame in the row's files, with its ordinal on the line), the
method and path arguments, whether a mount made the call (`register_blueprint` and `include_router` re-register
copies, linked by the captured source call, ordered copy event and callback identity), and each callback object's source identity
(Python: code object file, first line and qualified name; JS: the function's source text located in the row's
files, disambiguated by argument span/column for identical function text on one line); the framework's own table then gives the served
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

| Gate | Population and pass condition |
|---|---|
| Admission | Frozen M: 3,020 wave 1 and 681 wave 2 registration sites in the primary sets, both ≥200. Include chi (one mount in Caddy). Wave 3 uses all syntax sites incl. mounts: Django 996 (832 registrations), NestJS 297 (241), then forced table 150, forced Hapi. Fastify/Koa/DRF zero. |
| Matrix | Every enabled row after the STAGE-D-ROWS patch. Compare site spans, per-origin multiplicity, mount alternatives, callbacks and reasons, including controls with an empty listing. Preserve all 158 executed observations. Hand truths and the generator's synthetic cases are separate evidence. |
| Ledger reconciliation | Freeze SHA-256 867a441dc2c127c5f8428b7b52fc1f45c13dc524a2df98c969cba18bb5966d58: 5,607 sites, 4,984 registrations + 623 mounts. Match (corpus,revision,framework,kind,file,start,end); both ends distinguish builder legs. A table object can be matched through its element origin. A many-to-one mount alias needs its own ledger disposition. For every site record extracted, independently excluded, accepted loss with rule/row, or unexplained. Zero unexplained; classify all extras separately and fail a supported nonempty class with zero representatives. Mount-only classes can pass by a mount. |
| Site recall | Population is actual non-test registrations **in the frozen ledger**, per corpus for the enabled wave, including actual registrations missed or listed only as candidates. Require ≥0.80 site recall for N≥25. A site hit needs a proved receiver, candidate:false, and an exposure. Reconcile those spans exhaustively with the ledger. For recall credit the exposure must be independently correct; estimate that numerator by the weighted source audit where no field oracle exists. Report test, non-test and gross yield separately. For 1–24 eligible sites audit all, report exact numerator and denominator, and classify each miss without a percentage recall veto (one site changes the rate by more than four points). Before eligibility decisions Prometheus has 6, gin 13 and Flask 22. Zero needs independent evidence. Retain positive registration/mount controls and report binding yield wherever independently possible. Opaque out-of-ledger elements get a separate opportunity denominator (§11.4). |

| Schema | Frozen scope reports: Directus 136/136 operations, Cal.com 121/121, Apache Answer 191/199 (seven installer keys and one reviewed drift). Use validated path_tokens and effective per-exposure constraints under the bound root/base. Precision ≥0.95 per scope on scorable full method/path pairs; never use ambiguous suffixes, Any, unknown, WebSocket, or unbound host/version for exact credit. Require exact-full recall ≥0.80 of independently full-scorable keys per positive scope (including a positive Directus literal-chain denominator), and ≥0.80 of independently full-or-unique-suffix-scorable keys recovered in the appropriate class per scope. Freeze denominators before extraction, report raw recall on all 136/121/191 as well, and classify every exclusion/miss/extra. A suffix never satisfies a full requirement. Where all prefixes are unknown, zero full precision is undefined and needs independent evidence of that fact plus positive unique-suffix yield; a missing positive control fails. No blanket schema-drift exemption. |
| Go | Non-test Apache Answer ≥207/217 (95%) and Miniflux ≥143/150 (95%) admitted terminals with the **exact census method.declaration** in the frozen go_callback_shapes.json. Join by pinned revision, registration/selector site and terminal position; compare FunctionId's repo-relative declaring file + declaration line (and method name). Join all 217/150, including schema-unmatched sites. Report equal / unresolved / contradicted / missing per corpus, list contradictions and fail unexplained ones. The other 15 Miniflux concrete values are wrapper arguments. Report its separate external http.Handler refusal outside the 150. On Apache, ≥90% of distinct matched in-scope operations (full or unique suffix) must agree with their annotated **terminal** callback or certified terminal delegate; matched denominator must be positive. A wrong binding, even at the correct pathname, cannot earn credit; investigate every contradiction and fail unexplained ones. |
| Hapi | 11.4; report the private aggregate separately. |
| No regression | Unchanged callers, call-stats, navigation goldens and cache versions. Record cold session time, table time and peak RSS per gate corpus. |

**Accuracy gate.** Freeze eligibility, comparators and inclusion probabilities before the product run. Exhaustively compare a field when its independent oracle exists: ledger for site membership, frozen schema scopes for operation keys, Go declarations for selector identity, saved executed truth for fixture observations, and the authorized entry oracle in §11.4. An oracle for a span says nothing about its parent, path or callback.
Audit the following dimensions separately: origin/receiver, root, path status/value/tokens with effective guards, installation/dispatch outcome, and terminal or certified delegate identity. Compare complete expected and emitted sets on selected sites (additional, missing, downgraded and refused claims included). A wrong positive is an FP and leaves the correct expected fact missed. Independently justified refusals get their own disposition and never count as positive endpoints.

Without a field oracle, freeze a stratified source sample **per corpus and dimension**. Rank by SHA-256 of the UTF-8 compact JSON array ["lane12-D2",corpus,revision,split,dimension,framework,kind,file,start_byte,end_byte] (no ASCII escaping or whitespace). Break ties by the tuple in byte order.
Draw up to 48 non-test and 16 test sites per dimension, splitting by adapter, registration/mount, and source form (direct, composed, dynamic/guarded, opaque or interacting). Take at least one per occupied stratum, expanding the budget if needed, then allocate remaining places proportionally by largest remainder, breaking ties by stratum name. Audit the whole stratum when its allocation reaches N, then redistribute unused places. Audit every member of a frame below 25.
Freeze source-derived eligibility and labels blind to output. Deduplicate shared reads, record N, n, seed, spans and labels, and weight sampled totals by N/n within each stratum. Give independently classified out-of-ledger extras their own certainty stratum (weight 1), so each false positive is counted once. Publish the selected IDs, unsampled counts and weighted estimates with stratified finite-population uncertainty (enumerated strata have zero sampling variance).
Full-population site coverage and sampled field correctness have separate denominators. Classify every ledger disposition, every extra site and **every** discrepancy in each scored population, with reason and evidence. Inspect extras lacking a ledger key independently so false positives also enter precision.

Gate each corpus and claimed dimension on precision TP/(TP+FP) ≥0.95, both overall and for the non-test split when defined. Gate non-test recall TP/(TP+FN) ≥0.80 per eligible dimension with N≥25. Also report tests separately. Every instance in the scored joins/samples needs a disposition. Unexplained discrepancies fail; a source-backed unknown has the stated refusal cost.
An empty positive prediction has undefined precision, still owes recall and the positive control. Schema and Go keep their dedicated floors. An individual explained error gets zero credit and remains in the rates. Freeze full-scorable schema keys by root, native method, static chain, comparable tokens and proved compatible constraints; a suffix-scorable key needs a unique matching tail. Demonstrate any zero denominator independently. Mutate enough roots/handlers or downgrade enough true full paths to cross each threshold; also assert a single swap increments FP and FN even where the rate still passes.

**Mount semantics.** Before scoring enumerate every (corpus, adapter, split) with mounts only and every edge with zero independently scorable descendants, including opaque children, blocked/empty subtrees and register calls with no grounded chain. Freeze a small independent oracle for **every** such site and its zero-or-more edges. The census totals have exactly one mount-only (corpus, adapter, split) cell: Caddy/chi non-test, one Route and zero route rows; screwdriver has six non-test and 75 test register sites to classify.
For other mounts use existing truth or the stratified protocol. Compare the exact alternative multiset at each selected site by parent and child identity (including instance/realm), prefix status and value/text, guard/transform and alternative count, independently of the span or downstream route. Missing/extra edges enter recall/precision. Mount-only controls require exact agreement on every field and edge count, including zero. Replacing Caddy's child with its parent, changing only its prefix, dropping a guard or duplicating an edge must fail the corresponding control.

**Independent exclusions and costs.** Before comparison a separate reader freezes a source-based classification at the ledger revisions. Exclude a middleware-only use or settings read by its actual arguments, a client or non-framework receiver by tracing its import/construction, a descriptor outside an installed dispatcher or inactive metadata by the framework's lookup semantics. Exclude the redundant inner Django include from the mount denominator only after pairing it to the enclosing path/re_path span (88 include spans: 74 lie inside one outer mount span, two inside two, and 12 inside none. Use the nearest enclosing path/re_path after checking the syntax, and disposition the 12 independently). Never exclude by absence from the listing, by `lookalike_possible`, by test status, or by a low schema score. A dynamic route remains eligible. Record the evidence span and reason; if there is insufficient evidence leave the site eligible and investigate.

* 695 Express use.middleware entries (124 non-test) need the argument test; 236 receiver flags (159 Express, 52 Directus, 25 Cal.com) need actual source classification. The 25 include client calls; their true exclusion count is unmeasured. In fxa 150 literal objects are candidates before checking their consumers. Prometheus's 80 own-router sites are in the Go second look, not 80 deletions from the frozen net/http ledger. Classify gin's 13 non-test internal delegations by receiver/use and seven zero-callback test calls by signature, not by directory.
* M's variable-prefix observations: Django 4, Express 3, FastAPI 1, Flask 2. A method/path can remain useful when a binding refuses. Callback-position populations exposed to the rules: 468 Express, 277 Django, 18 Flask wrapper forms; 10, 337, 2 member forms respectively; Django 31 inline and Flask 2. These are shape counts before member/import/forwarder proof. Non-test Go literal losses on the two primary application corpora are 3 and 1; the Miniflux wrappers include 14 immediate method values. See 11 for Hapi.
* Directus has 295 non-test asyncHandler call sites through its local forwarder (Opus S5); each such route callback remains wrapper_call without delegate. These calls are not 295 distinct schema operations. Census non-test Express wrapped positions excluding use.middleware: Directus 353, Express 2, Cal.com 0 (separate middleware 43/32/2). Before S3 gate join each terminal/delegate to its registration and operation, and report bound/eligible and wrapper losses **per corpus**. Other local-forwarder counts and final binding yield are unmeasured. Keep a consumer-visible binding opportunity denominator alongside the scorable-proof denominator.
* Before dispatching S4/S5 classify immutable slice expansions and package-initializer bare arguments by their syntax and exact-fact availability per corpus; M's bare terminal tally (9/2 on Apache/Miniflux) does not identify these positions. Duplicate dispatch, blocked Django remainders and Hapi repeat/config cuts likewise need per-dispatcher event replay, regex/remainder pairs and per-item name/metadata/config resolution, respectively. Record the affected sites, paths and binding losses per corpus; M did not join those dimensions.
* M has no per-site result for reaching assignments, one-helper overflow, snapshot/lifecycle order, unsupported generators, language-specific template cuts, object installer proof, constraint conflicts, max-expansion cuts, or the other fine-grained refusal predicates. Their frequencies are **unknown**. Use the §7 source audit to tag each sampled site, callback and edge by predicate, estimate distinct losses / eligible sites by corpus and compare full-path and binding yield with and without the cut. For an independently enumerated population report exact counts. Zero observed in a sample is a sample result; reserve a population zero for an exhaustive oracle or source proof. Use the row-linked loss inventory, never infer incidence from fixture frequency. Failure of the floor or a material yield drop returns to design; do not rebaseline.

### Stage D change log

| Observation | Parameter fixed at D |
|---|---|
| Wave populations 3,020 / 681; Caddy has one chi Route and zero chi registrations | Build waves 1 and 2 including chi. Require a positive mount control for a mount-only corpus. |
| Django 996 sites and NestJS 297 including mounts; fxa has 150 table candidates; Fastify/Koa/DRF zero | Build S6 Django, S7 NestJS, S8 forced tables, then forced SH. Gate membership uses syntax sites for wave 3, not a non-test-only threshold. |
| 695 middleware forms, 236 Express lookalike flags, nested Django include spans, unverified table consumers | Independent exclusions, per-corpus non-test registration floor of 0.80 at N≥25, separate mount and gross yield. Candidate counts cannot certify eligible installations. |
| 136 Directus, 121 Cal.com, 199 Apache schema keys with 191 in scope | Freeze root/base/constraint-aware scoring. Keep 0.95 precision and 0.90 terminal binding agreement. |
| The expected 151st concrete Miniflux terminal has static type http.Handler | 150 admitted there and 217 on Apache. Retain 0.95 resolution per corpus, classify the external http.Handler site separately. |
| Direct-chain reach 0/400 private and 0/48 public calls; 57/59 variable plugin prefixes | Adopt weak plugin proof with separate full/suffix yield. Apply the manifest preflight and private path/identity gates in 11.4. Use starterkit for public plugin concentration and screwdriver for mixed shapes. |
| Screwdriver has 134 factory-shaped elements outside its 28 literal rows | Emit opaque candidates with site-level reason, count once per item, and retain literal siblings. |
| Matrix ex29/fa14, ex30/cx03, et6-01/es7-08, es7-01/07 | Lifetime-scoped receiver, alternatives per scalar mount, ordered copies, per-exposure guards and proven non-dispatch declarations. |
| gm15, gn04/05, gm04/gm19, es2-34, es4-14 | Method expressions can fit a handler signature. Accept named or inline group for one helper. Move Go protocol sites before the selector gate, the Express cap fixture after S1, and require activation for captured plugin callbacks. |

## 8. Follow-ups, not started by this lane

| Follow-up | Evidence that would justify it |
|---|---|
| Request sources keyed on route records, with the handler's parameters taken by position | the naming gap already measured (84 handlers in one corpus); needs its own benchmark measurement and touches taint seeding |
| Entry edges derived from route records (inline and wrapped handlers visible to `nav callers`) | agent sessions that ask for callers of a handler and get nothing; also the probe in which a module-level registration's entry edge goes to an unrelated nested function instead of the imported handler |
| An MCP tool `nav_routes` | a decision on the navigation specs in PR #606 |
| Go receiver typing from a method's own receiver (`a.f.M()`) in call resolution, reusing 5.6 M1 (a) | the wave 2 counts of method values the route table binds and `nav callees` misses; needs a Tier-A and call-stats measurement of its own |
| Go interface-typed handlers as candidate lists | an independently verified interface population on a consumer corpus, including externally declared interfaces reported as method_value_external |
| Lane 5's producer adapters reading route records | lane 5's pilot being funded; this lane gives it registration records (adapter input, not a wired adapter) for the waves that ship, with the binding rule: bind only a uniquely compatible full exposure with its constraints satisfied and a proved named or anonymous endpoint identity or certified terminal delegate; a middleware or Hapi prerequisite alone earns no endpoint credit, and the callback list is ordered (#600 gets the same position semantics). Section 11.5 defines the separate configured join |

## 9. Owner decisions

| # | Question | Default applied | Cost of changing later | Latest cheap point |
|---|---|---|---|---|
| 1 | Where routes live | a separate on-demand table; entry records and navigation edges untouched | caching the table later is a cache bump | before S1 |
| 2 | Exposure | CLI listing only | an MCP tool is a later slice with golden changes | after wave 1 |
| 3 | Wave 3 membership and order | by census count; object-literal route tables are included regardless of count because the private producer of lane 5 uses that shape | — | stage M |
| 4 | Flask-AppBuilder and similar class-based frameworks | out of scope | each is its own extractor | — |
| 5 | Executing fixture applications for truth | yes where the framework is already installed or installable offline; never anything from a corpus | hand-written truths only | stage T |
| 6 | Go method-value handlers (`r.GET("/x", a.questionController.List)`) | **decided by the owner, 2026-11-21: in scope.** Resolved by 5.6 from declared types (receiver, parameter, local, package `var`, up to two field hops) and package/profile-qualified declaration and promotion proof. Instantiated generic owners are an accepted zero-measured-terminal loss. Interface-typed receivers keep a refusal; slice S4b. Replaces "unresolved until stage D decides otherwise" | resolving interface receivers to candidate lists is a later slice with its own consumer | — |
| 7 | Hapi plugin adapter | **In scope.** Without decision 8, SH (9.5 days including preflight and corpus gates) buys lane 5 plugin-local routes, methods, ordered callbacks and typed suffixes. Pilot: **0/400 operations bindable from a proven root chain**. Under decision 11 a per-pair declared mount can yield separately labelled configured bindings (§11.5); their count is unmeasured. | Authorized seat must measure the per-file nearest-manifest join, weighted by entries, **before any SH dispatch**. If the ≥380/400 gate is infeasible, return to owner before building. | pre-SH |
| 8 | Indirect Hapi loading | Open after SH. Keep the strict root-chain rule. The owner can fund loader analysis. A proved loader edge still requires a static prefix for a full path. Configured joins do not require decision 8. | Mutable aggregators require separate data-flow proof. | SH gate |
| 9 | Dispatch across overlapping patterns/middleware/deployments | Stage D selects the bounded native certificates in §5.2 and the row inventory. Broader native replay is an optional future investment after the loss measurement. | A general dispatch simulator needs separate truth. | lane 5 integration |
| 10 | Admit a structural local Express forwarder? | Keep the closed delegate list. Owner may fund a one-formal wrapper returning a function that forwards its own arguments to that formal, with lexical, return and escape checks. Directus's 295 calls motivate measuring the gain. | New positive/negative adapter evidence. | S3 gate, before promising binding yield |
| 11 | May lane 5 supplement the owner’s full-only binding rule with a per-pair declared plugin mount? | Proposed default: permit an explicit opt-in **configured** result under §11.5; ordinary producer-proven bindings still require full. Alternative: keep lane 5 full-only, exposing weak rows for inspection. | Consumer policy and separate yield accounting; zero producer-proven pilot operations under either choice. | owner confirmation before lane 5 enables configured binding |

## 10. Risks and accepted limits

- **Prefixes are often dynamic** (configuration values, version constants, environment prefixes). The
  listing says `suffix` and shows what it knows; it never guesses.
- **Routers passed through functions.** A group handed to a registration function as a parameter is followed
  only within the stated bound; beyond it the path is a suffix.
- **Handler wrappers.** A wrapper call is an unresolved callback; a delegate is recorded only through stage
  D's closed forwarder list. Decorators that may replace the handler, dependency injection and class-based
  views are unresolved with a reason. Request-source coverage built on the listing pays for this.
- **Two registration facts for Go.** The existing callback-registration edges and the new records describe
  the same calls; the records prefer those facts at the argument's own bytes. Only the two unindexed bare-function positions in 5.1 use the declaration fallback. A Go
  method value (`c.List`) has no such fact; the records resolve it by 5.6 (owner decision 6), and nothing
  else in prism sees that result: `nav callers` still shows no edge into a method registered only as a
  method value (entry edges from route records are a section 8 follow-up).
- **Go method values left out** (5.6): interface-typed receivers, call-result and deeper receivers, embedded
  pointer or qualified explicit hops, instantiated generic owners, function-typed fields, package function values and method expressions, each a
  counted reason. The independent Miniflux census found one such terminal on an external http.Handler local, reported as method_value_external.
- **Typing the enclosing receiver is new** (5.6 M1 (a)): the call resolver does not type `a.f.M()` from a
  method's own receiver `a` either, so the route table can bind handlers that `nav callees` cannot. The rule
  reads only the method's declaration; carrying it into call resolution is outside this lane (a resolution
  change with its own corpus measurement).
- **Strict receiver and import proof costs recall** (factory apps, re-exported routers); each refusal is a
  counted reason, and the stage D recall floor is set knowing it.
- **Out of model:** routes added at run time in loops over data, plugin systems other than the Hapi plugin
  adapter (section 11), reflection.

## 11. Hapi plugin adapter (owner decision 7)

### 11.1 Why and what is proven

A direct register-chain certificate and a plugin-contract certificate answer different questions. On the private aggregate, 58 calls contain 400 literal rows, all inside register, with 59 plain unwritten first formals in 59 plugin objects. The original root-chain rule reaches **0/400**. One mutable factory list accounts for 115 entries/27 plugins and an aggregator list for 285/32; the final root-to-factory link is unresolved. Prefixes on 57/59 plugins come from variables. The adapter can identify the server formal, entries and callbacks without inventing a deployed base.

### 11.2 Rules

1. Root: bound @hapi/hapi or legacy hapi namespace/default/require, including an unshadowed destructured and renamed constructor, with server(), Server() or new Server(). A grounded server's register accepts one item, a literal array, or one direct local/imported binding; unwrap {plugin,options,routes} and a module's plugin property.
   A mount to the register formal records entry into that invocation. A later route validation error can abort the root after the realm was entered. Item routes.prefix overrides call options; inherit absent, preserve dynamic text. Replay events on the owning root server and compose nested plugin realms. Do not infer a mount from a factory, spread, loop or Glue.compose manifest. Carry possible failure and possible installed names through subsequent siblings and awaited calls. Emit a later mount only if it is reached and admitted in all possibilities. A visible immutable spread can prove noninterference without grounding its own mount (es1-27).
   Keep the server's registration-name set across realms. Use the effective plugin name (own name, otherwise pkg.name); distinct objects can share it. The first admissible registration installs. On a repeat, a proved plugin/item/call once skips the item; otherwise plugin.multiple=true admits another invocation, and a proved disallowed repeat fails setup. Respect the native validation of option combinations. For unknown name, ordering or repeat metadata, admit only transitions common to all possibilities. Diagnose the undecidable/rejected item, withhold its mount and dependent setup credit. A known once skip needs no diagnostic. Key each admitted realm by server, parent realm and register call span/item ordinal, even when object/name repeat (hp08 versus es1-34). On mount.child serialize realm as the ordered ancestry of {file,start_byte,end_byte,item_ordinal} register events. The registration origin keeps its plugin object association across invocations (§5.1). Retain a rejected or unreachable sibling’s written route, with its supported receiver facts and no exposure through that event.
2. Object contract: own name or pkg, and a unique inline method/arrow/function or a directly resolved function reference for register (includes shorthand, local export list, one direct imported function, es7-06). Accept CJS exports.plugin, module.exports.plugin or whole module, named/default ESM export, or an in-place object.
   First formal is a single identifier, unwritten in its binding scope; for proven strong invocation a default on a plain identifier is harmless because Hapi supplies the actual server (et7-02). Refuse destructured/written formals and shadowed uses. A stable alias retains the same realm. A dynamic register property and legacy register.attributes are accepted losses.
3. **Weak receiver proof.** Without a strong mount, admit the plain, non-defaulted formal of a contract-shaped plugin when the nearest package.json declares @hapi/hapi or hapi as a runtime dependency (dependencies or peerDependencies), and the object is exported as a plugin or flows into a grounded server's registration argument via local collection/factory syntax. For the latter require the actual plugin object in the literal list or factory return; this is evidence for parameter type only.
   A demonstrated non-Hapi invocation or contradictory binding defeats the weak proof. Never inherit a dependency from an enclosing package past the nearest manifest.
   Emit receiver.proof=plugin_contract, a null root and no_known_mount suffix for a static leaf, or dynamic for an unknown leaf; never a synthetic mount. hp22/cx07 qualify. d2-06–09 isolate the export/flow, contradictory invocation and manifest boundaries. A strong proof replaces the orphan only for that same plugin object. Its sibling object can still have a weak suffix from the same function. A provably skipped or rejected registration does not create a weak exposure for that event.
4. Registration phase: accept the register body, a uniquely invoked captured local installer or one direct helper transfer (5.2). Admit captured onPreStart and dependency-after closures only on the proven realm, with an observed initialize/start setup call and, for dependency-after, all named dependencies installed. Preserve the parent realm.
   Merely exposing/storing a function does not invoke it (es4-14); an external manifest loader supplies no direct registration evidence (es6-03).
5. Route: grounded server.route accepts a literal object/list or one direct binding/import to such an object/list; resolve values where defined. Literal site, install site/ordinal and the owning plugin object identify the row (§5.1). A static spread of a visible, immutable row array expands in order (es1-28). An opaque call/spread element becomes one candidate, at the argument/element bytes, reason hapi_route_not_literal, no exposures/callbacks (cx47/48). Inspect each sibling independently for source facts and apply §5.2 before claiming its installation.
   String or string-array methods uppercase/deduplicate, '*' is Any. Carry effective route and enclosing register/item routes.vhost through every strong realm. A single literal source with no rival setting is exact; differing or dynamic settings without a certified precedence result are unknown.
   A weak association has host_kind=unknown unless the linked call set is nonempty, complete for that association and every item, call and ancestor realm is proved vhost-free. Only then use route-level vhost or unconstrained. An absent link gives unknown, including on an exported orphan. Evaluate known own property names (literal/bracket or unique string-constant key), same-key last write wins.
   Read exactly one handler location after this step: handler, options.handler, or config.handler. Missing/competing handlers give hapi_handler_conflict and no dispatch credit for a provably invalid literal. For either options or config follow one unwritten local binding or direct import/require to its object literal, including a statically named own-property chain on that object; evaluate properties and pre in their definition scope. An opaque reference, getter, factory or further re-export leaves handler presence unknown. Preserve the grounded route and path with one callback_unproven handler slot carrying the expression; reserve hapi_handler_conflict for proved invalidity.
   An object handler yields hapi_handler_object. Keep options/config.pre before the handler, role=pre and zero-based group (nested parallel entries share the stage; source order is display order); final role=handler. No pres counted as a terminal handler. Non-Hapi route-shaped candidates use object-table and no credit; the Fastify url-shaped lookalike is silent.
6. On a grounded register call diagnose each failed item exactly once at the argument expression (array element/spread including its dots). Use hapi_registration_unproven for opaque items/imports or repeat refusals, hapi_plugin_unproven for unsupported contract shape, hapi_server_param_unproven for an inadmissible formal.
   A failure inside an already-entered plugin is attributed once at the failing native event in the root summary. No extra item-resolution diagnostic is due on that admitted call. A valid formal shadowed only at a use retains the mount; that route alone refuses. A non-Hapi .register emits no register diagnostic. Keep the route-site candidate separately. Missing, extra and duplicate diagnostics fail the matrix.
   Nonliteral table elements do not stand for a known number of routes. Treat the legacy API, general push/factory/aggregator mount proof, Glue manifests, opaque option factories and arbitrary escaped callbacks as measured limits. Row ids and individual dispositions are in STAGE-D-ROWS.md.

### 11.3 Matrix

Enable all w3-hapi rows after the literal reader (PLAN SH). Include direct, weak and unregistered receivers, overwrite/conflict, same-plugin registrations, direct import and re-export controls, nested helper and dormant callbacks, lifecycle installation and host/pre metadata. Use the hand labels in the frozen matrix. The security/custody boundary for measurement is aggregate-only.

### 11.4 Measurement and gate

**Before any SH dispatch**, the authorized seat joins each pilot plugin file to its nearest package.json, checks the runtime/peer dependency, formal and export/use, and joins the plugin to its entries. Freeze plugin and entry counts by strong/weak/ineligible and reason, plus attainable literal-path and inline-handler counts. Return only aggregates. Run the same independent classification on the public corpora. An infeasible floor returns to the owner before SHa.

| Corpus | Pass condition |
|---|---|
| Starterkit | All 11 calls are inside plugin register, with 29 non-test literal rows and zero grounded chains. Require ≥24/29 correct weak records and ≥95% of independently weak-eligible rows, with a positive denominator. Reconcile all 29. Trace the 24 referenced configs and five other member forms. Report object hits and opaque references. Preserve the path when only the callback is unknown. |
| Screwdriver | Reconcile 28 literal rows, 37 calls (26 inside register) and 81 register sites. Account for all 134 opaque elements individually. Require a candidate per grounded element and independent evidence for any ungrounded site. Match each failed register item to its diagnostic. The §7 registration floor counts the 11 non-test ledger literals (17 test literals reported separately). Below N=25 classify each miss. Separately report factory-element exposure recall as correctly exposed elements / 134 non-test call-shaped elements, with grounded candidates and unknown factory expansion counts shown apart. The proposed reader can expose 0/134 factory elements; even 11/11 literal recovery covers only 11/145 (7.6%) of these site/element opportunities. Report this design loss to the owner for acceptance at SH preflight. The factory output cardinalities remain unknown. |
| Hapipal | The zero-call control yields zero Hapi registrations. |
| Pilot (private aggregates) | ≥380/400 with the independently correct receiver, root and exposure. Require ≥365/384 correct literal-path exposures and ≥374/393 correct inline identities at their source span. Each numerator counts distinct correct **entries** in the frozen population. |

The pilot has 384 literal paths, 15 templates and one identifier. Compare status, value and typed tokens per entry.
Of 400 handlers, 399 are in config.handler and one is top-level. Beyond the 393 inline handlers, four are wrappers, two members and one an object. The aggregate counts zero route-level pre/vhost forms; register-level realm constraints still require proof.
The authorized seat uses the independently labelled entry oracle to join all 400 positions before aggregation and reports each disposition, with zero unexplained.
Classify each false identity, wrong root and false full as an incorrect positive under §7, with no numerator credit. Report strong/weak and full/suffix/dynamic separately.
A certified surviving strong all-literal chain requires full. The weak-only population has no positive full-path minimum.

### 11.5 Lane 5 configured mounts

Decision 11 proposes a separate consumer result when the pair configuration names one producer plugin by (defining file, object byte), a root and a literal mount. Match the weak receiver and refusal fields in §5.4, preserve the original source origin, and require a null-root static suffix with validated tokens. A shared name, register function or suffix alone cannot identify the plugin.

Compose **tokens** in the declared root using the §5.2 Hapi join: concatenate the certified literal prefix and the suffix, elide a slash-only leaf under a nonempty prefix (`/api` + `/` → `/api`), preserve parameter tokens and coalesce adjacent literals. Only a valid literal Hapi prefix is eligible; leave all other literal text, including interior slashes, unchanged. Refuse an unscorable path, unsupported prefix or ambiguous root.

Match protocol, declared/default method and effective host/version, setup and dispatch constraints. Unknown realm constraints (including unknown vhost), a competing endpoint or an unproved terminal role prevent binding. A configured prefix alone provides no host or dispatch proof.

Require one endpoint with status resolved **or anonymous** and a proved callable identity, or a certified terminal delegate; pre/middleware alone earns zero. Label the result **configured**, carrying configuration source, configured root and mount, plugin identity, original suffix and mount_refusal. Count it separately from producer-proven operations; never promote the listing or feed configured results into the producer gates.

Without a configured match the record remains plugin-local. Decision 7 alone gives the pilot zero producer-proven bindings. The controller's prototype bound 236/316 client calls with zero wrong in 50 checked. That measurement does not establish SH's configured yield.
