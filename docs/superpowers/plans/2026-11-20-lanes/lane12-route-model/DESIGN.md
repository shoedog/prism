# Lane 12: routes as a first-class fact — design

Stage D design of record for the 420-row matrix at b924b349. This revision is ready for the two-reviewer design gate. Owner decisions 6 and 7 admit Go method-value handlers and the Hapi plugin adapter. Related work includes #600 and draft PR #606. Research record:
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
identity is (framework, file, start byte, end byte, installation site and element ordinal where applicable). A copy
or second mount adds an exposure to its origin. Two declarations with identical arguments and handler remain two
records. Include test sources, preserving their package/build partition.

| Record | Fields and meaning |
|---|---|
| Route | framework, site, receiver, protocol (http or websocket), methods, path, paths, callbacks. Keep a literal endpoint name for endpoint-only rules. Table records also carry the installation site and element origin. A candidate keeps its source, receiver status and reason. |
| Receiver | router (file, binding, construction byte) or unproven(reason). Anonymous constructor expressions use their expression text and first byte. A framework parameter, controller decorator, URLconf list or default mux may supply the identity as specified below. |
| Methods | `{"known":[…]}` sorted unique, `"any"`, `{"status":"unknown","text":…}`, or `null` for an omitted declaration or a WebSocket. Preserve case for ServeMux. An explicit empty list remains known empty. |
| Path | `raw` is the expression as written, `literal` its single decoded path if known. Arrays retain raw spelling and null scalar literal. Regex retains its source and flags. A ServeMux pattern retains the original method/host in raw. |
| Exposure | root identity (nullable), status full(value), suffix(value,reason), or dynamic(reason), normalized path or null. Store effective mount constraints on the individual exposure (host, method guard, active version selector where present). Full describes the installed pattern in that root. It does not certify dispatch priority. |
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
anon_owner::identity. Go bare functions read go_registrations by the argument's bytes; an unwritten one-name
function alias may follow one explicit RHS to a unique same-package free declaration. Preserve ordered positions
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
and `runtime_generated_registration unsupported_registration build_only`. A diagnostic has a site and reason, not a
fabricated path. Accepted-loss descriptions in the matrix are audit notes; the enum is the wire contract.

### 5.2 Composition, ordering and comparison

Start from the selected application/URLconf/module root, or independent outermost application constructors when no
entry is selected. A mounted sub-app is seen through its parent; select it separately to query it as a root. Count a
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
repository (es3-09).

Apply mount semantics to a registration *event*, not just a pair of router names. Express live routers see later
declarations. Flask replays blueprint setup at registration (record_once once per blueprint identity per parent
app); FastAPI include copies the source's then-current entries with origin site and ordinal, including nested
copies. ASGI mount delegates live. Django include of a list captures that object; a module resolves its current
urlpatterns. Nest discovers metadata after bootstrap. Hapi registers a plugin in its realm and schedules only known
lifecycle hooks. Use source order inside a setup invocation, import initialization and uniquely ordered setup calls
across files. If order or membership cannot be proved, retain the source and only the known suffix. A copy can never
acquire later origin sites. Duplicate path/method declarations at different sites are listed even when first-match
dispatch shadows one (es4-16, es7-08).

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
coalesce only within one origin. Tables can be installed twice at different sites.

| Framework | Join and comparison grammar |
|---|---|
| Flask | Registration prefix overrides blueprint default, None/absent inherits, empty string overrides. Compose nested prefixes, strip boundary slashes and insert one unless leaf empty (prefix alone); slash leaf retains slash. Subdomain override composes child.parent under the configured server name. Keep host and strict-slash metadata, only the written pattern. Single-segment Werkzeug converters normalize to {}, tail/custom grammar gets null. Colons and braces are literal. |
| FastAPI/Starlette | Concatenate include prefix + router prefix + leaf verbatim, honoring the empty leaf and final slash. Root path is ASGI deployment metadata. Only valid named Starlette parameters, including built-in single-segment converters, normalize to {}; path tails give null. |
| Express/Nest | Join at the mount boundary, keep a slash leaf for Express. Nest inserts one slash between nonempty global, URI version, module, controller and method segments, defaults to /, strips terminal slash as Nest does. Keep raw patterns. A simple :name maps to {}; optional, repetition, wildcard, inline regex or Express string-pattern metacharacters give null. Regex leaf is dynamic(regex_path); regex mount retains the child suffix. Never collapse interior slashes to score. |
| gin/chi | Gin uses path.Join cleaning with the framework's terminal-slash preservation; empty relative path leaves the base. :name is one segment, *tail is unkeyed. Chi composes Route/Mount at a boundary, Group/With add no segment. {name} keys as {}, {name:regex} and * have null keys. |
| net/http | Parse Go 1.22 [method SP/HTAB] [host]/path, split segments before unescaping; preserve encoded slash/braces and CONNECT path spelling. {id} → {}, {tail...} and trailing subtree / → null, final {$} → exact trailing slash. Intersect host and method conditions through mux delegation; invert StripPrefix with its actual first argument and then apply the outer pattern (es5-09, es7-01). A direct mux child receives the original URL. Store the outer pattern as guard_pattern. A router delegated without stripping has absent prefix; StripPrefix stores the strip operand as prefix, independently of the outer subtree guard. If two dynamic pieces cannot be compared, retain a suffix with variable_prefix. |
| Django | Consume parent match, append child remainder verbatim, prepend the request's leading /. Preserve trailing slash; str/int/slug/uuid placeholders → {}, path and custom converters → null. Anchored regex made solely of literal path characters (escapes decoded as regex literals) may contribute an exact prefix; an end-anchored include permits only an empty-matching remainder. Only an exact anchored literal terminal gets an exact key. Other regex leaves are dynamic(regex_path), unanchored/complex regex mounts keep a suffix. |
| Hapi / object-table | Hapi concatenates realm prefixes with the route; a slash-only child under /api is /api. {id} keys as {}, optional, wildcard and counted wildcard are unkeyed. Object-table's certified dispatcher has literal key semantics; concatenate its proven prefix as its code spells it. |

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

`prism nav routes --repo <dir> [--mounts] [--format json|text]`. One JSON line per record, optional wrapped mount
lines and a final wrapped summary. For example, the two exposures of the same Any-method registration in es7-01
include:

```json
[{"status":"full","value":"/browse/ping","normalized":"/browse/ping","method_guard":{"known":["GET","HEAD"]}},{"status":"full","value":"/manage/ping","normalized":"/manage/ping","method_guard":"any"}]
```

The listing also emits root per exposure; literal host arrays use `hosts`, header version uses
`version:{kind,header,value}`. Guards intersect with the leaf's native dispatch methods; use only explicitly
declared/default HTTP operations for schema scoring (never credit an extra implicit HEAD/OPTIONS). For omitted HTTP
methods, compute a separate default_methods={"known":["GET"]} only when the API defaults to GET and no view metadata
overrides it. On Flask read a directly declared view's literal methods attribute and literal required_methods, and
the automatic-options switch, before certifying a default. A class-dispatched or decorated callable of unknown
metadata has no certified default. Implicit HEAD/OPTIONS never manufacture explicit schema keys. Score only section
7's eligible HTTP methods. A full exposure with `shadowed:true` is installed but preceded by an identical
unconditional first-match entry in the same root; exclude it from single-handler binding. Mark only when identical
protocol, active constraints, pattern and relevant method set/order prove it. Build-only has `reason:"build_only"`,
paths []. Opaque Hapi elements are `candidate:true`, at their syntax site with a record-level reason and
paths/callbacks []; count these apart from proved registrations in yield.

Rows sort by first path key or spelling, method, file, both site offsets, installation site and element ordinal;
mount lines by source site then alternative/child ordinal. Order paths full before suffix before dynamic, then
spelling, root and constraints. Callback arrays are ordered, route/mount matrix expectations match by origin or
resolved site anchors (5.1), not by arbitrary authors' array order. Test output sorting independently. Record counts
include candidates; summary.registrations = proven-receiver records + unproven-receiver records, with separate
candidate, diagnostic, mount, status and reason counts. Failed registration items without literal row sites emit
counted site diagnostics in summary.diagnostics, each with its site and reason. A missing delegate serializes as
null. A detector can report an unsupported source site without treating it as a registered route.

For example, a GET registration with one named callback has the following JSON line. The byte offsets illustrate its shape.

```json
{"framework":"express","protocol":"http","site":{"file":"a.js","line":3,"start_byte":60,"end_byte":76},"receiver":{"status":"router","file":"a.js","binding":"app","construction_byte":24},"methods":{"known":["GET"]},"path":{"raw":"'/x'","literal":"/x"},"paths":[{"status":"full","value":"/x","normalized":"/x","path_tokens":[["literal","/x"]],"root":{"file":"a.js","binding":"app","construction_byte":24},"method_guard":"any"}],"callbacks":[{"position":0,"status":"resolved","file":"a.js","name":"h","start_line":2,"end_line":2,"delegate":null}]}
```

Text output carries the same statuses, constraints and reasons.

Recognised unsupported API or data-generated sites produce a summary diagnostic at the source expression.
This counts the written site once even when runtime produces several routes. STAGE-D-ROWS.md pins the
empty-list controls and their diagnostic anchors.

### 5.5 Where the code goes

```
src/framework_entries/routes.rs            records, `routes(files, call_graph, manifests) -> RouteTable` (no dependency on navigation)
src/framework_entries/routes/receivers.rs  receiver grounding and `RouterId`
src/framework_entries/routes/handler.rs    callback resolution shared by the frameworks
src/framework_entries/routes/python.rs     Flask and FastAPI
src/framework_entries/routes/express.rs
src/framework_entries/routes/go.rs
src/framework_entries/routes/go_method_value.rs  Go selector callbacks (5.6); reads call-graph facts, adds none
src/framework_entries/routes/mounts.rs     mount graph and composition
src/framework_entries/routes/{django,nestjs,tables}.rs   wave 3 adapters and shared literal row reader
src/framework_entries/routes/hapi.rs       Hapi plugin adapter (section 11, owner decision 7)
src/navigation/routes.rs                   the listing (no logic in queries.rs)
crates/prism-review/src/cli.rs, main.rs    NavQuery::Routes and its arm
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

**Scope refinements at D.** Confirm M1–M4. First determine which declaration Go binds *at the selector*: check package path/declaring directory, receiver, block, closure, if/switch initializer lifetime and package/test visibility. A package qualifier must actually denote the import at this use (es1-47), an expired if shadow cannot poison the outer declaration (es7-03), and two identical package basenames cannot substitute for the import-path owner (es7-04). Same-block short redeclaration preserves the old variable; a new inner declaration triggers the shadow refusal (es5-07). The route-local scope check qualifies the classifier's evidence; a whole-function count alone cannot establish lifetime. Prefer explicit static type to initializer (es4-09): an interface never acquires the concrete initializer's method. An alias follows the type it denotes; a distinct defined type has its own method set (es5-08). Use declared generic owner identity for a concrete instantiation and its promoted members. Do not dispatch through a type parameter. Consult exactly one visible method at minimum embedding depth, or refuse. Production, same-package test and external test partitions stay distinct; incomparable build-tag alternatives give method_value_method_unproven (et4-01/02/03, es1-48). Local proofs may read the exact Go declaration, return slot or asserted type from syntax where the summarized fact omits that detail (es1-37, es2-19, es3-13); a multi-type switch case keeps its interface type (es3-16). Read inferred var-composite types and direct concrete return slots at their declarations. Parenthesized/unary operands, arbitrary conversions, anonymous-struct field bases, range-variable inference and a concrete type-switch implicit variable remain outside M1 (es1-42, es3-14/15, es2-18/16). No general Go inference is introduced.

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
  call). The route table passes an empty field-target map, so a hop through an embedded `*T` or `pkg.T` field,
  which needs that proof, refuses;
  (d) a local declared exactly once (the count `receiver_type_in_fn` reports) by a one-name
  `x := <(a), (b) or (c) form>`, typed as that form; one level, never a chain of locals (new: the classifier
  recovers a local only from a constructor, `var` or call initialiser, so a field-chain initialiser has no
  fact today; the new module reads that one `short_var_declaration` from the syntax).
  These steps and the local scope/declared-slot checks above read only declarations the source spells.
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
as method values, 5.6), so each matched operation’s annotation is checked against its terminal callback or certified terminal delegate under section 7. FastAPI fixtures need no
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
| Site recall | For each finished wave, ≥0.80 of eligible **registration sites**, separately reporting corpus, adapter and test split. Numerator counts each site once with a proven receiver and at least one exposure. Count documented losses, candidates and unproven receivers in the denominator if they are actual registrations; never trim the denominator to admitted shapes. Report gross yield against all frozen registrations too. Mounts have their own reconciliation. Nonempty reviewed full-path and binding yield on each eligible corpus when independently possible; if only mounts, weak plugins, dynamic prefixes or opaque callbacks exist, demonstrate that for the entire relevant population before declaring that particular yield inapplicable. An empty listing on a positive control fails. |
| Schema | Frozen scope reports: Directus 136/136 operations, Cal.com 121/121, Apache Answer 191/199 (seven installer keys and one reviewed drift). Use exact normalized tokens and effective per-exposure constraints under the bound root/base. Precision ≥0.95 per scope on scorable full method/path pairs; never use ambiguous suffixes, Any, unknown, WebSocket, or unbound host/version for exact credit. Report exact recall and (exact + unique suffix) distinct-key recall on frozen in-scope denominators with every miss/extra classified. Where all prefixes are unknown, zero full precision is undefined and needs independent evidence of that fact plus positive unique-suffix yield; a missing positive control fails. No blanket schema-drift exemption. |
| Go | Non-test Apache Answer ≥207/217 (95%) and Miniflux ≥143/150 (95%) admitted terminal method values resolved; no selector lacks a disposition. Count and review the one Miniflux interface site separately. On Apache, ≥90% of distinct matched in-scope operations (full or unique suffix) must agree with their annotated **terminal** callback or certified terminal delegate; matched denominator must be positive. A wrong binding, even at the correct pathname, cannot earn credit; investigate every contradiction and fail unexplained ones. |
| Hapi | 11.4; report the private aggregate separately. |
| No regression | Unchanged callers, call-stats, navigation goldens and cache versions. Record cold session time, table time and peak RSS per gate corpus. |

**Independent exclusions and costs.** Before comparison a separate reader freezes a source-based classification at the ledger revisions. Exclude a middleware-only use or settings read by its actual arguments, a client or non-framework receiver by tracing its import/construction, a descriptor outside an installed dispatcher or inactive metadata by the framework's lookup semantics. Exclude the redundant inner Django include from the mount denominator only after pairing it to the enclosing path/re_path span (88 include spans: 74 lie inside one outer mount span, two inside two, and 12 inside none. Use the nearest enclosing path/re_path after checking the syntax, and disposition the 12 independently). Never exclude by absence from the listing, by `lookalike_possible`, by test status, or by a low schema score. A dynamic route remains eligible. Record the evidence span and reason; if there is insufficient evidence leave the site eligible and investigate.

* 695 Express use.middleware entries (124 non-test) need the argument test; 236 receiver flags (159 Express, 52 Directus, 25 Cal.com) need actual source classification. The 25 include client calls; their true exclusion count is unmeasured. In fxa 150 literal objects are candidates before checking their consumers. Prometheus's 80 own-router sites are in the Go second look, not 80 deletions from the frozen net/http ledger. Classify gin's 13 non-test internal delegations by receiver/use and seven zero-callback test calls by signature, not by directory.
* M's variable-prefix observations: Django 4, Express 3, FastAPI 1, Flask 2. A method/path can remain useful when a binding refuses. Callback-position populations exposed to the rules: 468 Express, 277 Django, 18 Flask wrapper forms; 10, 337, 2 member forms respectively; Django 31 inline and Flask 2. These are shape counts before member/import/forwarder proof. Non-test Go literal losses on the two primary application corpora are 3 and 1; the Miniflux wrappers include 14 immediate method values. See 11 for Hapi.
* M has no per-site result for reaching assignments, one-helper overflow, snapshot/lifecycle order, unsupported generators, language-specific template cuts, object installer proof, constraint conflicts, max-expansion cuts, or the other fine-grained refusal predicates. Their frequencies are **unknown**. Before accepting a cut, independently tag each affected source site and callback/edge on its pinned corpus with the predicate, count distinct losses / eligible sites by corpus and reason, and compare exact full-path and binding yield with and without the cut. Record zero only after that count. Use the row-linked loss inventory, never infer incidence from fixture frequency. Failure of the floor or a material yield drop returns to design; do not rebaseline.

### Stage D change log

| Observation | Parameter fixed at D |
|---|---|
| Wave populations 3,020 / 681; Caddy has one chi Route and zero chi registrations | Build waves 1 and 2 including chi. Require a positive mount control for a mount-only corpus. |
| Django 996 sites and NestJS 297 including mounts; fxa has 150 table candidates; Fastify/Koa/DRF zero | Build S6 Django, S7 NestJS, S8 forced tables, then forced SH. Gate membership uses syntax sites for wave 3, not a non-test-only threshold. |
| 695 middleware forms, 236 Express lookalike flags, nested Django include spans, unverified table consumers | Independent exclusions, registration-only 0.80 floor, separate mount and gross yield. Candidate counts cannot certify eligible installations. |
| 136 Directus, 121 Cal.com, 199 Apache schema keys with 191 in scope | Freeze root/base/constraint-aware scoring. Keep 0.95 precision and 0.90 terminal binding agreement. |
| The expected 151st concrete Miniflux terminal has static type http.Handler | 150 admitted there and 217 on Apache. Retain 0.95 resolution per corpus, classify the interface site separately. |
| Direct-chain reach 0/400 private and 0/48 public calls; 57/59 variable plugin prefixes | Adopt weak plugin proof with separate full/suffix yield and ≥380/400 private gate. Use starterkit for public plugin concentration and screwdriver for mixed shapes. |
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
| Go interface-typed handlers as candidate lists | a nonzero `method_value_interface` count on a corpus a consumer cares about |
| Lane 5's producer adapters reading route records | lane 5's pilot being funded; this lane gives it registration records (adapter input, not a wired adapter) for the waves that ship, with the binding rule: bind only a uniquely compatible full exposure with its constraints satisfied and a resolved endpoint or certified delegate; a middleware or Hapi prerequisite alone earns no endpoint credit, and the callback list is ordered (#600 gets the same position semantics) |

## 9. Owner decisions

| # | Question | Default applied | Cost of changing later | Latest cheap point |
|---|---|---|---|---|
| 1 | Where routes live | a separate on-demand table; entry records and navigation edges untouched | caching the table later is a cache bump | before S1 |
| 2 | Exposure | CLI listing only | an MCP tool is a later slice with golden changes | after wave 1 |
| 3 | Wave 3 membership and order | by census count; object-literal route tables are included regardless of count because the private producer of lane 5 uses that shape | — | stage M |
| 4 | Flask-AppBuilder and similar class-based frameworks | out of scope | each is its own extractor | — |
| 5 | Executing fixture applications for truth | yes where the framework is already installed or installable offline; never anything from a corpus | hand-written truths only | stage T |
| 6 | Go method-value handlers (`r.GET("/x", a.questionController.List)`) | **decided by the owner, 2026-11-21: in scope.** Resolved by 5.6 from declared types (receiver, parameter, local, package `var`, up to two field hops) and the existing owner index; interface-typed receivers are counted, never a single handler; slice S4b. Replaces "unresolved until stage D decides otherwise" | resolving interface receivers to candidate lists is a later slice with its own consumer | — |
| 7 | A Hapi plugin adapter: routes registered with `server.route` inside plugin `register(server, options)` functions become proven records (the shape of lane 5's `pilot-producer`) | **decided by the owner, 2026-11-24: in scope**, 3.5 days in the Stage D plan (SH). Section 11 settles the plugin proof and its aggregate gate | none if it is built; without it those routes stay candidates (`table_consumer_unproven`) | — |
| 8 | Should indirect Hapi loading warrant a whole-program loader analysis? | Open for owner after SH. Ship the dependency-backed plugin suffix rule (11); require a proven root chain for full exposure and lane 5 binding. | Data flow through mutable aggregators is a separate slice. | SH gate |
| 9 | Should consumers require a dispatch winner across arbitrary overlapping patterns, request middleware and deployment configurations? | Open for owner before downstream binding. Publish installed records with per-exposure constraints and certified identical shadowing; ambiguous operation bindings get no credit. | A dispatch simulator needs separate truth. | lane 5 integration |

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
  counted reason. The independent Miniflux census found one such terminal on an interface-typed local.
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

1. Root: bound @hapi/hapi or legacy hapi namespace/default/require, including an unshadowed destructured and renamed constructor, with server(), Server() or new Server(). A grounded server's register accepts one item, a literal array, or one direct local/imported binding; unwrap {plugin,options,routes} and a module's plugin property. Each proven item yields a mount to its register formal. Item routes.prefix overrides call options; inherit absent, preserve dynamic text. Respect once per server/plugin identity in registration order. Compose nested plugin realms. Do not infer a mount from a factory, spread, loop or Glue.compose manifest. Account for the failed item even when a literal sibling succeeds.
2. Object contract: own name or pkg, and a unique inline method/arrow/function or a directly resolved function reference for register (includes shorthand, local export list, one direct imported function, es7-06). Accept CJS exports.plugin, module.exports.plugin or whole module, named/default ESM export, or an in-place object. First formal is a single identifier, unwritten in its binding scope; for proven strong invocation a default on a plain identifier is harmless because Hapi supplies the actual server (et7-02). Refuse destructured/written formals and shadowed uses. A stable alias retains the same realm. A dynamic register property and legacy register.attributes are accepted losses.
3. **Weaker rule adopted.** Without a strong mount, admit the plain, non-defaulted formal of a contract-shaped plugin when the nearest package.json declares @hapi/hapi or hapi as a runtime dependency (dependencies or peerDependencies), and the object is exported as a plugin or flows into a grounded server's registration argument via local collection/factory syntax. For the latter require the actual plugin object in the literal list or factory return; this is evidence for parameter type only. A demonstrated non-Hapi invocation or contradictory binding defeats the weak proof. Never inherit a dependency from an enclosing package past the nearest manifest. Emit receiver.proof=plugin_contract, a null root and no_known_mount suffix for a static leaf, or dynamic for an unknown leaf; never a synthetic mount. This admits hp22/cx07 and preserves hp16/es1-25 as negative controls. A later strong proof replaces the orphan exposure.
4. Registration phase: accept the register body, a uniquely invoked captured local installer or one direct helper transfer (5.2). Admit captured onPreStart and dependency-after closures only on the proven realm, with an observed initialize/start setup call and, for dependency-after, all named dependencies installed. Preserve the parent realm. Merely exposing/storing a function does not invoke it (es4-14); an external manifest loader supplies no direct registration evidence (es6-03).
5. Route: grounded server.route accepts a literal object/list or one direct binding/import to such an object/list; resolve values where defined. Literal object site + install site/ordinal identifies the row. A static spread of a visible, immutable row array expands in order (es1-28). An opaque call/spread element becomes one candidate, at the argument/element bytes, reason hapi_route_not_literal, no exposures/callbacks (cx47/48); continue siblings. String or string-array methods uppercase/deduplicate, '*' is Any. Keep vhost as exposure hosts. Evaluate known own property names (literal/bracket or unique string-constant key), same-key last write wins. Read exactly one handler location after this step: handler, options.handler, or config.handler. Missing/competing handlers give hapi_handler_conflict and no dispatch credit for a provably invalid literal. If configuration is a function, the value is unknown (callback_unproven), not a proven missing handler. An object handler yields hapi_handler_object. Keep options/config.pre before the handler, role=pre and zero-based group (nested parallel entries share the stage; source order is display order); final role=handler. No pres counted as a terminal handler. Non-Hapi route-shaped candidates use object-table and no credit; the Fastify url-shaped lookalike is silent.
6. Unexpanded registration items produce hapi_registration_unproven diagnostics. Nonliteral table elements do not stand for a known number of routes. Treat the legacy API, general push/factory/aggregator mount proof, Glue manifests, opaque option factories and arbitrary escaped callbacks as measured limits. Row ids and individual dispositions are in STAGE-D-ROWS.md.

### 11.3 Matrix

Enable all w3-hapi rows after the literal reader (PLAN SH). Include direct, weak and unregistered receivers, overwrite/conflict, same-plugin registrations, direct import and re-export controls, nested helper and dormant callbacks, lifecycle installation and host/pre metadata. Use the hand labels in the frozen matrix. The security/custody boundary for measurement is aggregate-only.

### 11.4 Measurement and gate

Use hapijs-starterkit as primary public plugin corpus (11/11 calls inside register, 29 literal rows, zero register calls), screwdriver as the complementary mixed corpus (26/37 inside, 28 literals, 134 call-shaped items, 81 register sites), hapipal-realworld-example-app as zero-call control. Direct reach 0/11 and 0/37. Cover both positive corpora in ledger reconciliation, separately counting all opaque item diagnostics. Starterkit has 24/29 missing **literal** handler locations and five member forms; a syntax count cannot determine the outcome of option evaluation. Count weak-proof successes, negative evidence and losses separately before gate disposition. Public weak-eligible exact counts need a per-object manifest/use join, absent in M.

Private gate: ≥380/400 entries (95% of **all** entries) with a proven receiver and exposure; report strong/weak, full/suffix/dynamic, callback statuses, refusals and every unaccounted entry (zero unexplained). 384 literal paths, 15 templates, one identifier; 399 config.handler and one top-level, 393 inline, four wrappers, two members, one object, zero pre/vhost or legacy forms. Those counts do not certify 400 successful inferences. A weak receiver identifies the formal in the plugin contract. Deployment remains unproven and its exposure cannot be used to bind an operation. No positive full-path minimum on the weak-only population. A proven all-literal strong chain requires full; a weak one cannot score full or a schema binding. Aggregate-only counting stays with the authorized measurement seat.
