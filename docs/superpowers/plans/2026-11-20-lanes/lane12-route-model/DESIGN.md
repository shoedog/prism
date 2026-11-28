# Lane 12: routes as a first-class fact — design

Stage D design of record for the 420-row matrix at b924b349 and the additions in STAGE-D-ROWS.md. Owner decision of 2026-11-25 allows this targeted fold and one final review by two fresh reviewers. Either rejection stops lane 12.
The admitted lane is estimated at **59.5 working days** (original plan 34, prior draft 71.5). PLAN lists every slice. The common conflict rule removes five days of native dispatch work. A shared source sample and reuse of the Go and Hapi oracles remove seven evidence days.

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
source origin is (framework, file, start byte, end byte, installation site and element ordinal where applicable). For a Hapi plugin route, qualify that origin by the defining plugin object (file, object_byte), on strong, weak and refused associations alike. Serialize it as route.origin_plugin whenever the defining object is identified, including on retained object-table candidates. The effective name, register function and formal are never substitutes for the object. A copy
or second mount adds an exposure to its origin. Two declarations with identical arguments and handler remain two
records. Include test sources, preserving their package/build partition.

| Record | Fields and meaning |
|---|---|
| Route | framework, site, receiver, protocol (http or websocket), methods, path, paths, callbacks. Keep a literal endpoint name for endpoint-only rules. Table records also carry the installation site and element origin. A candidate keeps its source, receiver status and reason. |
| Receiver | router (file, binding, construction byte) or unproven(reason). Anonymous constructor expressions use their expression text and first byte. A framework parameter, controller decorator, URLconf list or default mux may supply the identity as specified below. |
| Methods | `{"known":[…]}` sorted unique, `"any"`, `{"status":"unknown","text":…}`, or `null` for an omitted declaration or a WebSocket. Preserve case for ServeMux. An explicit empty list remains known empty. |
| Path | `raw` is the expression as written, `literal` its single decoded path if known. Arrays retain raw spelling and null scalar literal. Regex retains its source and flags. A ServeMux pattern retains the original method/host in raw. |
| Exposure | root identity (nullable), status full(value), suffix(value,reason), or dynamic(reason), normalized path or null. Store effective mount constraints on the individual exposure (host, method guard, active version selector where present). Full certifies installation in that root at the listing epoch under the supported setup model and the adapter boundary in §5.3. Binding holds under the same conditions and also checks root health and effective dispatch (§5.2). |
| Callback | position (zero-based), resolved or anonymous identity (file, name, span), or unresolved(text,reason). Optional delegate identifies a forwarded argument. Hapi prerequisites also have role and group. An endpoint-only registration has zero written callbacks. |
| Mount | parent, child, site, one prefix (literal, absent, unevaluated with text), and any request guard/strip transform. One site can supply several edges. Object alternatives, branches and child arguments have stable source ordinals. |

**Values at a use.** Walk the lexical scope, including captures, to the reaching declaration. A straight-line assignment of a fresh router starts a new lifetime (ex29). A use before or after that assignment retains the corresponding identity. Resolve a direct, unwritten alias to that object. Distinguish factories and scopes, imports and package partitions. A conditional merge of different identities or a write of unknown order defeats a unique-target proof. For adapters with a stated value boundary in §5.3, apply its traced-form checks for uncertain alias mutations and mutable-value escapes. All other adapters apply the strict rule that an uncertain alias mutation or mutable-value escape defeats proof. Only propagate an object through a factory with a unique direct return of a grounded local construction and a distinguishable invocation. Qualify separate instances at one allocation site by their invocation when distinguishable, otherwise refuse. Retain a recognisable but unproven site with empty paths and no binding credit. Proven-absent execution, a settings read and ordinary middleware emit nothing.

Use direct named/default/namespace ES imports, unshadowed literal CJS require (including destructuring, property and
default interop), Python from/qualified/relative imports including package initializers. Require one eligible target
module and the requested *member* in it. Validate the export's reaching value and callable or router type. Follow
local export specifiers, direct module.exports and its own properties, observing replacement/detachment of exports.
A second ES/CJS module forwarding the value, a star import, an ambiguous export or a shadowed require supplies no
proof. For Python module members, apply the static import proof in §5.3, including syntactically established
package hops. A terminal binding that gets its router or callable from another module is an unsupported re-export. If an
unproved value controls a conditional mount, retain the syntactic unproven mount unless the condition is
independently decidable (es1-06). Do not use a file basename as an owner. An initializer declaring a router
differs from one re-exporting it (cx02, es1-15).

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
`receiver_unproven imported_unproven terminal_reexport mount_child_unproven table_consumer_unproven no_known_mount variable_prefix
non_literal_path regex_path mount_cycle expansion_truncated`,
`getattr_module star_import global_write multi_write nested_single submodule_shadow module_descriptor_name
class_private_name class_body_import` (Python import diagnostics),
`callback_unproven member_handler wrapper_call inner_decorator class_view anonymous_unindexed spread_callback handler_object`,
`method_value_receiver_unproven method_value_receiver_shadowed method_value_method_unproven method_value_interface
method_value_external func_field_value qualified_function_value method_expression`,
`hapi_registration_unproven hapi_plugin_unproven hapi_plugin_unregistered hapi_server_param_unproven
hapi_route_not_literal hapi_handler_object hapi_handler_conflict`,
and `runtime_generated_registration unsupported_registration build_only superseded dispatch_conflict entry_unproven`. All query errors, including malformed selectors, missing exports and ambiguous roots, use entry_unproven. A diagnostic has a site and reason, not a
fabricated path. Accepted-loss descriptions in the matrix are audit notes; the enum is the wire contract.
The public Flask summary carries counted import refusals as `{site,reason}` diagnostics in the
precedence specified in §5.3. Role-only reasons such as `terminal_reexport` stay on the affected record.

### 5.2 Composition, ordering and comparison

The **supported setup model** is the declared static setup subject to the adapter boundary in §5.3.
A runtime discrepancy is exempt from the §7 veto only if a frozen disposition ties it by evidence to
a named out-of-model form.

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

**Setup completes.** Exposures describe the declared setup conditional on completion. Full exposures and dispatch bindings hold under the supported setup model and the adapter boundary in §5.3. Keep every registration record, written callback and origin. Compare events on the same dispatcher at the same epoch using native installation resources (e.g. a Flask endpoint name) and effective request sets. A proved rejection removes the rejected key and adds the root, event site and reason to failed_roots. Preserve other installed keys as historical exposures. Every exposure on a failed root carries binding_blocked="any" and earns zero served-schema or binding credit. Independent setups retain their evidence.

An undecidable installation or overwrite withholds full and binding only on keys that could collide. Keep the source spelling and record conflicts on the affected origins. A known earlier install on an append-or-reject dispatcher survives a later attempted insert. If co-installation is proved but dispatch is undecidable, keep both paths and block only the contested methods. A proved overwrite removes only the superseded slot. Disjoint keys, roots and later events keep their own outcomes. Within the same config-prefixed gin group, compare known method and segment boundaries below the shared prefix before marking a potential collision. A proved skip or a sibling beyond a definite rejection in the same native call has no installation through that item. There is no speculative startup timeline, catch/await analysis or root-wide penalty for uncertainty.

Use cheap native equality, disjoint method/host/literal checks and three priority proofs where applicable: static over parameter, known first successful match in order, and GET fallback on an Express Route lacking explicit HEAD. Keep declared operations separate from effective methods. Set shadowed=true only for an entire losing request set. If a contested subset cannot be represented, block that method over the affected pattern. The [adapter inventory](STAGE-D-ROWS.md#dispatcher-interactions) defines the closed list of implicit events, their scopes and the cheap checks. An internal event can reserve a key without producing a listing record. Withhold binding on overlap unless the native facts settle the winner.

For scorable paths emit path_tokens as a sequence of ["literal",text] and ["parameter"] entries, coalescing adjacent
literal text, or null for an unscorable grammar. Render that sequence as normalized for display (substitute {} for a
parameter). The oracle uses path_tokens and validates it against value with the adapter grammar. Schema tokens are
parsed with the schema grammar independently. Literal braces do not become parameters on a second parse. The
comparison tuple is (root, protocol, method, host selector, active version selector, path tokens). Path templates
that cannot be expressed as single segments have null normalized even when their spelling is full. For host
matching, represent absence as unconstrained, a literal host or list as exact alternatives, and a
parameterized/regex/unknown selector as constrained and unscorable by this oracle. For ServeMux compare the native host strings case-sensitively after native request-side port handling (ordinary requests strip the port, CONNECT retains the native lookup spelling). Preserve pattern spelling. A DNS-only authority cannot establish a host-qualified binding. Other adapters use their native host comparison.
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

##### Router and view value boundary (Python)

This boundary applies to Flask in S2a and FastAPI/Starlette in S2b. It governs decorator and call receivers,
children of `register_blueprint`, `include_router` and `mount`, and the view supplied by `view_func`, `endpoint` or a decorated def.

- **Router values.** Admit a bare name with a proved reaching binding, including a simple assignment or an
  alias that passes the existing write/use checks. Follow direct imports and `from` imports (including `as`
  aliases), and attribute chains through resolved modules to a proved definition or allocation. Admit calls to
  import-proved framework constructors (`Flask`, `Blueprint` here), and zero-argument calls to proved synchronous,
  parameterless factories without decorators whose sole return is the final direct statement and resolves to a
  named router in the invocation frames. Ordinary grouped value expressions are normalized. The following
  are disclosed residuals at `56a9a0d4`. The alias write/use check handles `alias = app` and skips
  `alias = (app)`. The Flask `record_once` setup-state matcher gets the owner of a bare
  `state.add_url_rule(...)` from the proved Blueprint. Comparing the receiver's written text to
  the formal leaves `(state).add_url_rule(...)` untraced.
  S2b retains the built-in FastAPI `.router` case below.
- **View values.** Admit the function definition at a route decorator with no opaque decorator beneath it, or a
  bare function name reached directly or through the same assignment, import/alias and module-member forms.
  A passed function's decorators must all be proved transparent. Flask's recognized `View`/`MethodView`
  subclass calls `C.as_view('n')` and `C.as_view(name='n')` can prove metadata under the same native
  class-shape checks. Their callbacks remain unresolved (`class_view`).

A subscript (including tuple, list or dict lookup), conditional, arbitrary call result, lambda, `self`/`cls`
member or value returned through an unproved helper never supplies a value proof. An unproved receiver is
`receiver_unproven`, `imported_unproven` or `terminal_reexport` (a terminal re-export) when the site is retained (Flask drops sites without a receiver hint).
An unproved mount child is `mount_child_unproven`. A view argument stays unresolved with its existing reason,
including `callback_unproven`, `wrapper_call`, `anonymous_unindexed`, `member_handler`, `imported_unproven`,
`inner_decorator` or `spread_callback` as applicable.

As with `Exact`, arbitrary post-creation replacement or deletion of router/app methods or attributes is out of
model. This includes `x.get = ...`, reflective replacement and method or attribute mutation through an untraced
rebind of a module attribute whose name matches a submodule. A direct store of a traced router such as
`pkg.routes = app` escapes the object and defeats its receiver proof.

View mutation is out of model when the argument path is untraced. This includes a subscript, a loop or
comprehension target, a container bound to a name (`vs = [f]` followed by `patch_all(vs)`), a class attribute
reached through a decorator, or an alias through an unproved form. For escape checks, trace a resolved view
supplied by a bare name, attribute or call, including one written inside an argument's tuple, list, set or dict
display, splat, conditional or boolean expression. Outside a proved native registration use, a view reaching a
call through a traced form withholds view-derived metadata unless the callee earns the no-effect exemption
below. Thus a traced `patch(f)` can keep its written callback identity while losing inferred
endpoint and method metadata.

Source-only pins still compare the declaration and written callback. Explicit setup checks still apply to
supported view metadata writes, endpoint decorators and Blueprint view-map merges. For mutation of the
`view_functions` mapping itself, d2-14 covers only direct `X.view_functions[key] = v`. App registry
`.update`, `.pop`, `del` on an entry and writes through a `view_functions` alias are out-of-model mutations
pinned as `KNOWN_LIMIT` in fk-26–29. The direct-store barrier applies only when lexical evidence traces
`X` to a router allocation. A parameter, `current_app` or `self.app` with no traced allocation can
hide a registry write. This is a known limit with zero runtime credit for affected dispatch claims.

**Static import binding (controller decision, Option 1).** `PROVEN` for an imported router, view or
no-effect callee means that the source import, in its lexical scope, identifies one eligible router
allocation or callable declaration under Prism's static rules for declaration, scope and import syntax.
An imported callee must also pass the existing body check for the no-effect exemption. Full exposure,
effective methods and dispatch binding each require their own proofs. The grade makes no claim about
the object a running Python process held at capture time.

The controller withdrew the closed-world rule and its eight reasons. The application census shows
that it withholds 100 of 115 import-reached registration and mount sites, including every Flask site
in that population, on evidence unrelated to module mutation. Option 1 applies the repository's
standing static-binding contract as written. §10 reports the measured change.

Require one eligible in-repo source module. Resolve the `P.N` module path in `from P.N import x`
through import syntax. Python loads that module without reading `P.N` as a package attribute. Apply
the hop predicates only where the capture reads a package member, including `import P.N as r`,
qualified access such as `P.N.x`, and selection of `N` in `from P import N`. Apply the terminal
predicates to the captured member `x`. Resolve a relative import from the capturing file's source
package.

Each attribute-path hop needs a syntactically established binding. A child-naming import
(`import P.N`, `import P.N as r`, or `from P import N` when it imports the child) must occur in
the capture's own lexical scope in the capture module, at or before the capture in that scope's
source text. The dotted aliased import itself supplies that evidence for its attribute reads.
`from P import N` imports the child under this rule when an eligible child exists, `P` has no
lexical value write to `N`, and `P` has no explicit lexical module-scope `__getattr__` binding. A lexical binding
of `N` in the package initializer, by import, definition or assignment, also establishes a member.
Follow it as a module only when the syntax identifies an eligible module, including an explicit
initializer alias such as `from . import real as routes`. The initializer's `from . import N`
is the explicit same-child binding in predicate 6. An empty initializer and `import P` leave
`P.N.x` unproved even if `P/N.py` exists or an unrelated module imports `P.N`. Keep the lexical
alias and shadow checks. Refuse a router or callable terminal re-export such as `from pkg import bp`
when the initializer obtains `bp` via `from .base import bp`. A terminal assignment alias whose
reaching value comes from an import from another module is also a re-export, for example
`from .base import bp as _bp; bp = _bp` (fk-80). A simple assignment alias in the capturing scope
can still name a directly imported declaration under the value rules above. Recognized native Flask imports continue to supply `Flask`,
`Blueprint`, `View` and `MethodView` under the existing framework checks.

For a direct factory call, the written capture is the callee. For class-view metadata, it
is the class operand of `as_view`. A refused factory receiver or mount child leaves the written
registration fields available. A refused class operand leaves the written callback and explicit
methods available and withholds view-derived metadata.

**Contract and certificate (PR #928 part 2; owner decision 2026-11-27: "we cover what is, not what
may be").** A Python route proof is a static-binding proof, in the words CLAUDE.md uses for `Exact`:
the name the proof relies on resolves, by Python's declaration, scope, import and `global`/`nonlocal`
syntax as prism models it, to exactly one binding. It does not assert runtime truth. Run-time mutation
(monkey-patching, reflective writes through `globals()`, `setattr`, `exec` or `sys.modules`, descriptor
and metaclass behaviour) and changes made through ANOTHER HOLDER of the same object (a view passed to
a call, an attribute store on a view or router reached under another name, an extension handed the app)
are outside the claim. The patterns the contract covers are the ones real applications use (the census
in section 10), not every constructed shape.

The decision lives in ONE function, `PythonBindings::certify(file, scope, key) -> Result<(), Clause>`
(`src/framework_entries/routes/receivers/certificate.rs`), cached per (file, scope, key). The key is the
NFKC-normalised name with class-private mangling (`writes::effective_name`). `Clause` is a closed enum
whose variants map to existing labels, so no output format, `Reason` or schema changes:
`ParseError` and `Count` to `multi_write`, `Redirect` to `global_write`, `Star` to `star_import`, `Kind`
to `nested_single`. For a module scope the clauses are, first failing names the refusal (the order the
code tests them):

- (e) The module has a parse error (any ERROR or MISSING node in the file): every key of the module
  fails. Measured: it withholds 0 rows over 1,945 files, so it is not narrowed to the binding's
  statement.
- (a, redirect) A nested scope declares `global K` and binds it: `global_write`.
- (a, count) The module scope holds exactly ONE binding occurrence of K, counting occurrences at any
  nesting of the module scope AND occurrences in a def or class that declares `global K`. The
  module-system names (`__doc__ __annotations__ __annotate__ __package__ __name__ __file__ __spec__ __loader__
  __builtins__ __cached__ __path__ __warningregistry__`) count as one more occurrence, so a def of one
  fails. Zero or two-plus fails (`multi_write`).
- (c) A `from m import *` counts as a binding of K exactly when `star_could_supply` says the star can
  bind K, that is when `m` resolves to an in-repo module that can. A star from an external or
  unresolved module is not seen (the contract sees what the repository declares): `star_import`. This
  holds for a star NESTED in an in-repo star's source as well (`star_could_supply(.., external = false)`
  on the capture side; the source side and the candidate search keep `external = true`). A default star
  (no `__all__`) also copies the child module that the package initializer's own import syntax installs
  (`from .child import x`, `from . import child`, `import pkg.child`); a literal `__all__` clears it.
- (b) The single binding is an import (not star), def, async def, class, or `K = <expr>` / `K: T = <expr>`
  with one bare-name target, and it is a DIRECT statement of the scope body (not under `if`, `try`,
  `with`, `for`, `while` or `match`): else `nested_single`. A tuple, `for`, `with`, `except`, walrus,
  match, `del`, augmented or `type` binding is never admitted.

A function scope is certified only against redirects: a nested scope that declares `nonlocal K` and
binds it fails (`global_write`). A `global K` at MODULE level redirects nothing and is dropped at
indexing, so the module still binds K. Reads of the key resolve through `LexicalWrites::read_binding_scope`
(class scopes skipped, directives honoured). Nothing is traced through a rebinding and no position
filter is used: a rebinding fails (a), wherever it sits and whenever it runs, so a module-scope router
or view name bound twice is not proved at either read (the position-sensitive lifetime pins were
re-labelled; function-local names keep the flow-sensitive lifetime walk below).

*Binding forms.* The binding occurrences of a key are the closed syntactic list that
`LexicalWrites::written_names` collects: import, import-as, from-import, from-import-as; def, async
def, class; parameters (lambda included); assignment targets (plain, augmented, annotated with a
value, tuple, list and starred leaves); walrus (binds the enclosing non-comprehension scope); `for` and
comprehension targets; `with ... as`; `except ... as` and `except* ... as`; match captures; `del K`;
`type K = ...`. `global K` and `nonlocal K` are directives that redirect a write in their scope to the
module scope or the nearest enclosing function binding. PEP 695 type parameters bind an implicit
annotation scope of their own, so by scope awareness they are another scope's binding of the same
spelling and neither count nor refuse (fk-95 stays a known limit).

*Scope awareness.* The SAME SPELLING in another scope that does not declare it `global` or `nonlocal`
is a DIFFERENT binding and neither counts nor refuses: a parameter, a function local, a class-body
name, a comprehension target, a loop target in a function (census S04, S05, S09, S14e, S14g).

*Where the code differs from the contract file.* The package-child rule is not in `certify`: a package
`__init__` that imports a child module spelled like the key stays the source-side `submodule_shadow`
predicate (6) with its `same_child_import` exception (the formspree `from .create_app import create_app`
shape: no proof existed before, none is lost). A star from an external module in a SOURCE module stays
`star_import` of predicate 2. PEP 695 type parameters are not counted (above). Clause (e) is
whole-file. Function scopes get the `nonlocal` redirect check only; a full local count passed every
test but was not priced on the corpora, so the flow-sensitive `reaching` stays for function-local
names (setup and factory functions). Round-1 review (F1) asked whether function and class scope must
also refuse a binding that a loop back edge runs before the read. No proof reaches such a read: a
registration whose read sits in a `for` or `while` body is a `runtime_generated_registration`
diagnostic and never a rule (setup function, factory and module alike), a registration in a class body
belongs to no active scope, and the mutation index reads every binding occurrence with no position
filter, so the full count was not adopted (pinned by the `f1_*` rows). Two flow-sensitive rules close
the neighbouring wrong proofs: in an import lookup a LATER write in the same non-module scope displaces
an earlier import (the reaching occurrence wins, S1), and in the mutation index a definite write (an
import, def, class or `K = <expr>` directly in the function body) before a store supersedes every
occurrence before it, later occurrences and nested `nonlocal` writes staying (S3).

*Replaced by the certificate (code deleted).* `competing_import_paths` and `has_competing_import`
(candidate search shared by the escape gate and the candidate traversal); the rule that a census
refusal of the written callee is authoritative, with its fallback in `refused_factory` (a refused
callee has no eligible-factory fallback because `certify` fails first); the position-sensitive
`reaching_sources` walk and `may_resolve` in the mutation values code, now the position-free
`LexicalWrites::binding_sites`/`occurrences` and `import_candidates` (every binding occurrence of the
name in its resolved binding scope, used when `certify` fails so the conservative router and view
taint is kept); the `capture_sites` consumer in `prove_import` (`competing_global`) and the
`has_global_write` early return in `named` (clause (a) covers both); the default-methods withholding
of a class view for a direct write to `methods` or `required_methods` (a store on a view class is a
holder change: outside the claim, a known limit; `view_metadata_writes` stays for the supported view
metadata writes of the setup model).

*Stays, and why.* The source-side predicates below that ask about OTHER modules' exports and package
children, not whether the name is one binding: 1 `getattr_module`, 2 `star_import` (a wildcard of a
checked source module that could supply the member), 6 `submodule_shadow` and 7
`module_descriptor_name`. Predicates 3 to 5 (`global_write`, `multi_write`, `nested_single`) are now
the certificate's clauses (a) and (b), applied to the capturing module and to each source module and
hop package alike.
`view_metadata_escape` and `no_effect_body`, `escaped_mount_child` and the router-store barrier
(`router_mutated`, converters, config): holder barriers on objects the proof reads, which the
certificate does not model. The local flow-sensitive `reaching` for function-local names in a setup
function or factory (reason above). The price of the round-3 rule that preceded this paragraph is
recorded in section 10; the census that bounds the contract is its own entry there.

Require one unconditional eligible terminal binding. The capturing module must also contain
exactly one binding of the captured import's local name, the import that creates it.
Find the candidate import by lexical lookup of the written capture's root name, whether bare or
qualified, including an earlier import hidden by a later write to the same name. The capture-side
certificate counts these sites for the import's effective local key in the scope the read resolves to,
including conditional sites and sites after the read, and the `global`/`nonlocal` redirects from nested
scopes. The same spelling bound in another scope without such a directive is a different binding and does
not count: fk-87 (a method-local import of the spelling) is `PROVEN` at its own local binding and fk-92
(a comprehension target) is `PROVEN`; the former file-wide census withheld both.

- Explicit `import` and `from ... import` names, including `as` aliases and the leading name of an
  unaliased dotted import.
- Function and class declaration names, function and lambda parameters, and Python 3.12+
  `type` alias declaration names, including the alias name in a generic declaration.
- Bare-name targets of ordinary and augmented assignments, annotated assignments with a value,
  walrus expressions, `for` statements and comprehension or generator iteration. Include explicit
  name targets in destructuring and the `as` names of `with`, `except` and `except*` clauses.
- Structural match capture names and explicit name deletions.

Read `global` and `nonlocal` as scope declarations and use annotation-only declarations for local
lookup. These sites define the complete capture-side contract. The census counts assignment targets
under their own names. It starts from the written root, so following `callback = handler` back to
`handler` to count competing bindings is outside its scope; a star import in the importing module counts
only when it can bind the name (clause c). The source-side checks cover assignment-alias chains for
terminal re-export detection and wildcard imports that could supply a source member. The capture-side
count is lexical and requires no execution model. Use Python's NFKC-normalised identifiers as
binding and predicate keys. NFKC-normalise module-path and hop segments in the source text and
match the resulting keys byte-for-byte against filesystem candidate names (fk-74–75).
A capturing import is eligible at module or function scope, subject to the predicates below.
An import whose nearest enclosing scope is the class body triggers `class_body_import`,
including inside an `if`, `try` or `with` in that body. Keep original UTF-8 byte spans and
spellings in diagnostics. Apply the first seven predicates in order across the terminal and
applicable attribute-path hops, with predicates 3 and 4 also covering the capturing name.
Then apply the root-chain spelling check and the import-scope check, in that order.

1. `getattr_module` applies to any explicit module-scope lexical binding of `__getattr__`,
   conditional or unconditional. Count named imports and aliases that bind that spelling,
   defs, classes, `type __getattr__` statements, assignment targets including tuples, `for`,
   `with`, `except` and `except*` targets, match captures and walrus targets. A binding through
   `global __getattr__` in a nested scope also counts. A nested scope's own local binding or an
   unwritten `global` declaration does not trigger the veto. A wildcard-supplied hook is a
   `KNOWN_LIMIT` (fk-72 and fk-c72). Predicate 2 still checks whether the star could supply the
   captured member itself.
2. `star_import` applies when a wildcard import in a checked source module could supply the name,
   including an unknown source or export set.
3. `global_write` applies when a class or function body writes a module binding through
   `global`, including a competing write to the captured local name in the importing module
   (fk-86 and fk-90). Use the effective identifier after NFKC and Python's class-private mangling for
   both `global_write` and the write keys of `multi_write`. Within an enclosing class's lexical
   context, including its methods, a name with at least two leading underscores and fewer
   than two trailing underscores gets the class name as a prefix. Strip the class name's leading underscores
   and prepend one underscore. An all-underscore class leaves the name as written, and a
   nested class supplies its own prefix. All five pinned CPython versions (3.9.6, 3.11.15,
   3.12.13, 3.13.13 and 3.14.4) disassemble `global __f` followed by `def __f` in
   `class Install` to `STORE_GLOBAL _Install__f` (fk-69–70). An ordinary class attribute binds
   in the class namespace. [Python's private-name mangling rules](https://docs.python.org/3.12/reference/expressions.html#private-name-mangling)
4. `multi_write` applies to competing lexical binding or deletion sites for the same effective
   name in the same binding scope (module scope, counting `global` redirects), including later writes, writes through `global`, imports, `del`, `match` captures,
   `for` targets, `with ... as`,
   `except ... as`, `except* ... as`, walrus writes to the enclosing scope and Python 3.12
   `type` statement names (including generic aliases). Count an annotated assignment with a
   value as a write. An annotation without a value (`f: object`) writes no value and can still
   affect function-local name resolution. A comprehension or generator target binds the
   comprehension's own scope, so it is another binding of the spelling and counts for neither
   `multi_write` nor `nested_single` (the certificate is scope-aware; census S14e). In the importing file, use the capture-side forms
   listed above. Same-value repeated bindings are an **accepted over-refusal**. This includes
   `import pkg` followed by `import pkg.routes` (fk-91), the same named import twice, a successful
   named import with an `except ImportError` fallback binding, and a duplicate guarded by
   `TYPE_CHECKING`. The reshaped fk-c55 uses only `import pkg.routes` and remains `PROVEN`.
5. `nested_single` applies when the sole counted write, including a `type` statement, is nested
   in conditional or control-flow syntax.
6. `submodule_shadow` applies when the hop or terminal module binds `N` by a lexical write
   other than an import of that same child (for example, the initializer's `from . import N`),
   while an eligible same-named in-repo child `N` exists (`P/N.py`, `P/N/__init__.py` or
   a source namespace directory `P/N/`). A capture-side import of the child does not clear
   the veto. Use source candidates independently of installation order.
7. `module_descriptor_name` is a spelling-only veto for the closed list `__dict__` and
   `__class__`. On CPython 3.9.6, 3.11.15, 3.12.13, 3.13.13 and 3.14.4, from-import and a
   first qualified lookup return the module dictionary or type in place of a same-named
   def. On 3.11+ a repeated qualified lookup may return the def after specialization. The veto
   applies in both cases. The tested `__annotations__` and `__annotate__` lookups preserve an
   explicit declaration, including on versions exposing the corresponding descriptors.
8. `class_private_name` withholds a capture if any NFKC-normalised identifier in the root
   chain is mangle-eligible. A mangle-eligible name has at least two leading underscores
   and fewer than two trailing underscores. Check the imported name, import path segments,
   the local or alias name at the read, alias sources and attribute segments read to reach a
   router, view or callee, wherever the import or read stands. CPython decides mangling from
   the spelling and the enclosing class at compile time. Prism withholds on spelling
   because the capture proof does not model which class encloses the use. A module-scope
   read of `views.__f` is an accepted over-refusal with zero measured cost (§10, fk-85).
   See fk-70 and fk-82–85 for the spelling boundary.
9. `class_body_import` applies when the capturing import's nearest enclosing scope is the
   class body, including under `if`, `try` and `with` in that body. An ordinary name
   imported there remains withheld because it binds in the class namespace (fk-81 and its
   module-scope control fk-c81).

Keep the existing decorator transparency checks. Flask accepts a proved `@bp.get('/a')` (fk-c10).
For the following stack, keep the admitted route and mark its written callback unresolved with
`inner_decorator`.

```python
@bp.get('/a')
@skip('/a')
def f():
    pass
```

A no-effect callee must bind to its direct, undecorated def. Assignment aliases of that def cannot
earn the exemption. Comments, `pass`, a bare return and a return of `None`
(including grouped `None`) pass. A docstring statement fails.

Evaluate imported captures used by registration or mount proofs, by the no-effect check, or reached
during escape indexing (fk-34). On refusal preserve independently established literals, the written
callback and the role-specific receiver, callback or mount behavior. A failed callee check withholds
dependent inferred metadata. Emit only the first applicable counted reason per distinct refused
capture span. Use the candidate import's name before `as` for a bare from-import, including an
earlier import hidden by a competing write. Use the full written expression for a qualified capture.
For a counted import refusal of a no-effect callee, emit once at each distinct whole-call span
(fk-65). Emit the diagnostic even when Flask drops a site for lack of a receiver hint. An unsupported
form with no counted reason, including an unestablished child hop or terminal re-export, keeps
only its role-specific refusal. Use `mount_child_unproven` for a mount child,
`terminal_reexport` for a receiver whose import is a terminal re-export (the Rule is retained with that receiver refusal and the callback slot carries the same reason) and
`imported_unproven` for any other imported receiver or view retained by the adapter. For an unsupported
imported callee, the no-effect exemption is unavailable and dependent fields are withheld.
`unestablished_hop` names a census outcome only; `terminal_reexport` is also the wire receiver reason above and is not a counted import refusal.

During escape indexing, keep possible targets when an import proof fails and withhold their
view-derived metadata if the escape cannot be cleared. When the certificate refuses the
name, include every import or alias occurrence of it in the scope the read resolves to, wherever it sits
or runs (no position filter), including guarded imports and writes redirected by `global` or `nonlocal`.
Resolve a qualified argument's member path against each such module root. A proved no-effect
callee still clears the escape. In fk-34, the helper captures and mutates
one of two definitions of `f`. `f` is bound twice in `views.py`, so it is not one static binding: the
written local callback is `callback_unproven` (it was resolved at the second definition under the
position-sensitive walk), and inferred endpoint and effective methods are withheld. In fk-c44 this amendment deliberately recovers `setup.patch`:
at `56a9a0d4`, `setup.unused = _mut` invalidated the entire module for the no-effect exemption.

**Standing static-binding contract.** The following paragraph defines `Exact`. [CLAUDE.md](../../../../../CLAUDE.md)

> `Exact` is a static-binding grade: the site's name resolves, by the language's declaration, scope, import
> and export syntax as prism models it, to exactly one callable. It does not assert runtime truth. Runtime
> mutation of module objects (monkey-patching, reflective writes, `eval`, host globals, re-acquisition through
> `require`/`import()`), from the same file or another, is out of model for every rung and every language
> (replan probes, 2026-09-25). Producer-side mutation is modeled only where mutation is the construction
> idiom (CJS `module.exports`, `src/ast/js_cjs_export_barriers.rs`). A slice that wants to model runtime
> mutation proposes it as a new contract, not as a precision fix to an existing rung.

For these Python imports, out-of-model behavior includes runtime module-object mutation (reflective
writes and direct attribute stores, `globals()`, `vars()`, `setattr` and `__dict__` stores,
`importlib.reload`, `sys.modules` replacement (fk-66) and `exec`), dynamic lookup without an
explicit lexical module `__getattr__` binding, dynamic import machinery changes and non-source
children, initialization order and timing, and native descriptor precedence. The controller withdrew
the import-machinery veto after the rule produced three of the four distinct material findings in
review round 2. The five attributes `__path__`, `__name__`, `__spec__`, `__package__` and
`__loader__` are inputs to runtime import machinery. Static name binding follows the source import path.
Module-scope lexical writes to these attributes, including writes through `global`, are out of model
in the terminal module, hop packages, source initializers and the capturing module. Mutations such
as `__path__.insert(...)`, `__path__.append(...)` and `__path__[:] = ...` are also out of model.
A wildcard import can supply one of these attributes or install a `__getattr__` hook through its
export set. Pins fk-68, fk-71–73 and fk-76–77 record these import boundaries. fk-c68 is the
`PROVEN` source-import control. The closed descriptor veto
is a conservative belt based on source spelling. Supported view and router escape checks and
explicit view metadata writes remain part of the setup model. The [Stage D pins](STAGE-D-ROWS.md#python-router-and-view-value-boundary)
record expected adapter fields beside native observations across 163 cases, with 56 `WITHHELD`,
32 `KNOWN_LIMIT` and 75 `PROVEN`. Each `KNOWN_LIMIT` pin earns zero runtime credit for the
affected fields.

#### FastAPI

FastAPI/APIRouter and the app's built-in .router. GET, HEAD, POST, PUT, DELETE, PATCH, OPTIONS and TRACE decorators,
api_route/add_api_route, websocket/add_api_websocket_route, compatibility route/add_route, include_router and mount
of a grounded ASGI app. Known empty method set has a registered path and zero HTTP operations. Omitted defaults are
reported separately from declaration. WebSocket protocol and null methods. Dependencies and include_in_schema do not
add/remove declarations. StaticFiles mounts, constructor routes and enabled documentation URLs contribute internal claims (§5.2).

#### Express 4

express() and Router() (including direct imports). Use the 36 spellings from the Stage M method table: acl, all,
bind, checkout, connect, copy, delete, get, head, link, lock, m-search, merge, mkactivity, mkcalendar, mkcol, move,
notify, options, patch, post, propfind, proppatch, purge, put, query, rebind, report, search, source, subscribe,
trace, unbind, unlink, unlock, unsubscribe. Accept literal bracket spellings and the del→DELETE alias.

Represent all as framework Any. Every verb leg, repeated/saved Route, router-returning verb chain, bound verb with
same proven receiver and optional call on non-null receiver. A definitely null receiver, empty builder, get setting
read, param, static and middleware-only use add no route. use overload inspects its first flattened item, supports
pathless lists, path arrays and several children mixed with middleware. Function use layers are non-competing pre-handlers. A terminal function use (including a static responder) can intercept a listed binding and is a disclosed coverage loss. Verb registrations including all, and mounted routers/apps, compete. The binding names the route terminal conditional on its pre-handlers continuing. Count observed interception separately in actual request yield. At router exhaustion implicit OPTIONS can intercept a later parent OPTIONS.

Emit one mount per (child, prefix alternative) at the same site. A router as a verb callback is a handler_object;
secondary dispatch through it is an accepted loss.

#### gin

gin.New/Default, their embedded RouterGroup and chained Group expressions. Uppercase GET, POST, PUT, PATCH, DELETE,
HEAD, OPTIONS; Handle(method,path,…), Match(method slice,path,…). Any expands gin's finite
GET/POST/PUT/PATCH/HEAD/OPTIONS/DELETE/CONNECT/TRACE set. Group is a mount. Use, NoRoute, NoMethod and static-file
helper internals add no written verb registration.

#### net/http

An unevaluated pattern has unknown methods and a dynamic leaf. A parsed pattern with no method is Any.

Package Handle/HandleFunc, DefaultServeMux, or a proven ServeMux, tracking straight-line replacements at use. DefaultServeMux has one stable repo-local anchor per program, shared by package helper calls and implicit `init` claims (stdlib packages that register on it); replacement muxes are constructor-anchored.
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

Includes per-method array iteration and Map(entries). Require a matching dispatch lookup. An unmodelled mutation, reset or escape invalidates the keys it may change, including all keys for an unbounded clear. This is a small syntax matcher, not general interprocedural flow. Read one local or direct-import table
literal at its definition scope. Replay static own properties/spreads in JS evaluation order. An opaque write
poisons only properties it may override. A passive read alongside a certified installation does not duplicate it.

For a recognised route-shaped call (bare or member route/install with method, path and handler fields), failure to
prove the consumer yields an object-table candidate. Unused data, client request descriptors and unrelated count
calls are silent. An adapter may claim a site only once: Hapi first if grounded, otherwise the generic candidate.
For object-table the public site is the installer call plus element_site and element_ordinal.

For Hapi the site is the element plus install_site and element ordinal. This also specifies the ledger crosswalk for imported rows.

#### Hapi

Direct server rows share the object reader. Plugin realm/admission, callbacks and lifecycle are specified in 11.

S1b exercises the event algebra on synthetic relations with two origins, epochs, guards and independent roots. Each adapter owns its native relation and corresponding rows. Vary live/copy/deferred installation, order and identity against an independent reference. Retain each grounded origin even where it has no eligible exposure.

### 5.4 The listing

`prism nav routes --repo <dir> [--entry <entry>] [--mounts] [--format json|text]`. Accept the fixture entry forms (Python module:binding or module:factory(), JS/TS repo-relative file selecting its default/app export, or file#export for a named app) and a URLconf module:binding. An entry may also be a serialized 5.2 root identity for a statically evidenced app/dispatcher instance in any adapter. Resolve within the repository without executing setup. Invalid or uncertifiable explicit selections return a query error (entry_unproven for a binding without an app proof). Selection cannot ground an unregistered plugin formal or fabricate an invocation. The selection fixes served-root context while independently known orphan declarations remain reportable. One JSON line per record, optional wrapped mount
lines and a final wrapped summary. Explicit selection failure returns one query error and no successful listing. S1a gives lane 5 the typed operation and callback fields and lane 9 the listing, origin and mount identities for navigation. At S1a and S1b the enabled adapter set is empty (core e01/e02 only). An empty listing carries no evidence of route absence. Expose the enabled adapter names in the listing summary at every checkpoint.

**Wire fields.** Every row below gives the JSON type and its absence rule. Required arrays may be empty. A site is {file,line,start_byte,end_byte} in the repository (one-based line, zero-based half-open UTF-8 bytes).
A router identity is {file,binding,construction_byte,invocations}, with 5.2's ordered call-site frames or initial {entry} frame. An identity for a callable is {file,name,start_line,end_line,start_byte,end_byte} at its declaration or anonymous expression. Byte spans distinguish same-line functions.
A method set is `{"known":[strings]}` (sorted, unique), `"any"` or `{"status":"unknown","text":string}`. Reason always uses 5.1's closed enum.

| Field | Type and values | Absent or null | Emitter |
|---|---|---|---|
| route.framework, protocol, site | adapter name from 5.3, http/websocket, Site | required | all |
| route.element_site, install_site, element_ordinal | Site, Site, uint (zero-based). Object-table site is install, Hapi site is element. | include ordinal and complementary site for table elements, omit elsewhere | object-table, Hapi |
| receiver.status, receiver identity or reason | router + router identity, or unproven + Reason | disjoint required variants | all, also mount endpoints |
| route.origin_plugin | {file:string,object_byte:uint,name:string or null} | required for each identified plugin-object association, even with receiver.status=unproven. Omit when no defining object is identified. Null name means unknown effective name. | Hapi and its retained candidates |
| receiver.proof | plugin_contract | weak only, omit on strong and unproven receivers | Hapi |
| methods, default_methods, endpoint | method set or null declaration, known default set, endpoint name string | methods required on a known row shape, null for omitted/WS. Omit default unless independently proved and endpoint unless written. | all, defaults on defaulting HTTP APIs, endpoint on Flask |
| path.raw, path.literal | expression string (regex includes spelling/flags), decoded string or null | required on a known row shape, null literal for array/regex/dynamic. Omit path and methods on opaque items with no path/method fields. | all |
| candidate, reason | boolean, Reason | candidate omitted means false. Reason present on invalid, withheld or fully superseded records as applicable. | all |
| superseded_keys, conflicts | arrays of {root,method_guard,path,site,reason}. Root identity or null, method set, native pattern string or null if unknown, competing write/event Site, superseded or dispatch_conflict. | omit empty arrays. Preserve partial cuts alongside surviving paths. | Map/chi writes, all contested installs respectively |
| paths, callbacks | ordered arrays of Exposure and Callback | required, including [] | all |
| exposure.root, status, value, reason | router identity or null, full/suffix/dynamic, string, Reason | root required (null for an orphan), value on full/suffix, omit on dynamic. Reason required for partial/dynamic, omit on full. | all |
| exposure.path_tokens, normalized | array of literal/parameter tokens (5.2), display string | both required, both null for unscorable path grammar | all |
| exposure.host_kind, hosts | unconstrained/literal/parameterized/regex/unknown, array of written host strings | required. [] means no known strings. Only unconstrained proves no host restriction. | all, constrained on Flask/Hapi/Nest/ServeMux |
| exposure.method_guard, binding_blocked | method set for effective mount/write restriction, known method set or "any" ineligible for binding by 5.2 | method_guard required, "any" when unrestricted. Use "any" on every exposure of a failed root. Otherwise omit for no contest and use "any" for an unrepresentable blocked subset. | all |
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
| summary.enabled_adapters | sorted unique array of adapter names available in this build | required, [] at S1a and S1b | all |
| summary.registrations, proven, unproven, candidates, mounts | uint counts. registrations = proven + unproven = route lines. candidates is the candidate:true subset, mounts counts edges regardless of display. | required, zero allowed | all |
| summary.diagnostics, failed_roots | arrays of {site,reason} and {root,site,reason}. Each failed entry identifies a proved native rejection at its event on that router instance. | required, [] when clear | all |
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
paths and effective methods, linked to those calls. A function's first line is never used as a site. Historical executed fixtures
disable framework-added routes (Flask `static_folder=None`, FastAPI `docs_url`, `redoc_url`, `openapi_url` set to
`None`); any route without an instrumented call fails the row. Truth is kept as raw observations; the
comparison projects each field explicitly (a static `suffix` agrees with a runtime full path ending in it; a
static `full` must equal it; declared and effective methods are kept apart). Containment: the staged copy holds
only the row's directory (a real directory under `tests/routes_matrix/`, no links) and the pinned dependency
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
| Schema | Frozen scope reports: Directus 136/136 operations, Cal.com 121/121, Apache Answer 191/199 (seven installer keys and one reviewed drift). Use validated path_tokens and effective per-exposure constraints under the bound root/base. Precision ≥0.95 per scope on scorable full method/path pairs; never use ambiguous suffixes, Any, unknown, WebSocket, or unbound host/version for exact credit. Require exact-full recall ≥0.80 of independently full-scorable keys per positive scope (including a positive Directus literal-chain denominator), and ≥0.80 of independently full-or-unique-suffix-scorable keys recovered in the appropriate class per scope. Freeze denominators before extraction, report raw recall on all 136/121/191 as well, and classify every exclusion/miss/extra. A suffix never satisfies a full requirement. Where all prefixes are unknown, zero full precision is undefined and needs independent evidence of that fact plus positive unique-suffix yield; a missing positive control fails. A false full under the supported setup model fails even above the precision floor. |
| Go | Non-test Apache Answer ≥207/217 (95%) and Miniflux ≥143/150 (95%) admitted terminals with the **exact census method.declaration** in the frozen go_callback_shapes.json. Join by pinned revision, registration/selector site and terminal position; compare FunctionId's repo-relative declaring file + declaration line (and method name). Join all 217/150, including schema-unmatched sites. Report equal / unresolved / contradicted / missing per corpus, list contradictions and apply the false-certainty veto below. The other 15 Miniflux concrete values are wrapper arguments. Report its separate external http.Handler refusal outside the 150. On Apache, ≥90% of distinct matched in-scope operations (full or unique suffix) must agree with their annotated **terminal** callback or certified terminal delegate; matched denominator must be positive. The annotation check compares terminal callbacks and ignores `binding_blocked`. "Same config-prefixed gin group" means the same prefix expression, so the separate `Group(APIBaseURL+"/answer/api/v1")` calls count as one. A wrong binding, even at the correct pathname, cannot earn credit; any observed false binding must be corrected or withdrawn and rescored. |
| Hapi | 11.4; report the private aggregate separately. |
| No regression | Unchanged callers, call-stats, navigation goldens and cache versions. Record cold session time, table time and peak RSS per gate corpus. |

**Accuracy gate.** Freeze eligibility, comparators and inclusion probabilities before the product run. Exhaustively compare a field when its independent oracle exists: ledger for site membership, frozen schema scopes for operation keys, Go declarations for selector identity, saved executed truth for fixture observations, and the authorized entry oracle in §11.4. An oracle for a span says nothing about its parent, path or callback.
Audit the following dimensions separately: origin/receiver, root, path status/value/tokens with effective guards, installation/dispatch outcome, and terminal or certified delegate identity. Compare complete expected and emitted sets on selected sites (additional, missing, downgraded and refused claims included). A wrong positive is an FP and leaves the correct expected fact missed. Independently justified refusals get their own disposition and never count as positive endpoints.

Score exposure and binding claims against the supported setup model (§5.2). Each `KNOWN_LIMIT`
pin records the emitted proof or withhold beside native observations and identifies the named
out-of-model form. For example, fk-38 pins the emitted binding to the source `views.__doc__` def
beside native capture of None and a GET KeyError. fk-53 pins emitted endpoint f and GET/HEAD/OPTIONS
beside native endpoint renamed and POST/OPTIONS after a function-local reflective write. Award zero
runtime credit for affected claims. A ledger exemption requires evidence for that site and the named
form. Apply ordinary checks to source-only pins, explicit metadata writes and supported view-map
operations, including d2-14.

Compare §5.3 diagnostics at their exact capture or call spans. The descriptor and unestablished
child-hop pins (fk-52 and fk-54) have scored refusals with distinct controls. The former keeps the
explicit endpoint and methods with an unresolved callback and one counted diagnostic. The latter
keeps the parent and independent child declarations with the role-specific mount refusal.
Record withheld fields and independently justified partial facts, including at unlisted receiver
sites and refused no-effect callees. Count losses in the applicable recall denominators. §10 gives
the two scanner populations and the measured import-binding cost.

Without a field oracle, freeze one stratified source sample per corpus across all dimensions. Label every applicable field at each selected site, including implicit claims in its setup. Rank by SHA-256 of the UTF-8 compact JSON array ["lane12-D3",corpus,revision,split,framework,kind,file,start_byte,end_byte] (no ASCII escaping or whitespace). Break ties by the tuple in byte order.
Draw up to 48 non-test and 16 test sites per corpus, splitting by adapter, registration/mount, and source form (direct, composed, dynamic/guarded, opaque or interacting). Take at least one per occupied stratum, expanding the budget if needed, then allocate remaining places proportionally by largest remainder, breaking ties by stratum name. Audit the whole stratum when its allocation reaches N, then redistribute unused places. Audit every member of a frame below 25.
Freeze source-derived eligibility and labels blind to output. Deduplicate shared reads, record N, n, seed, spans and labels, and weight sampled totals by N/n within each stratum. Give independently classified out-of-ledger extras their own certainty stratum (weight 1), so each false positive is counted once. Publish the selected IDs, unsampled counts and weighted estimates with stratified finite-population uncertainty (enumerated strata have zero sampling variance).
Full-population site coverage and sampled field correctness have separate denominators. Classify every ledger disposition, every extra site and **every** discrepancy in each scored population, with reason and evidence. Inspect extras lacking a ledger key independently so false positives also enter precision.

Gate each corpus and claimed dimension on precision TP/(TP+FP) ≥0.95, both overall and for the non-test split when defined. Gate non-test recall TP/(TP+FN) ≥0.80 per eligible dimension with N≥25. Also report tests separately. Every instance in the scored joins/samples needs a disposition. Unexplained discrepancies fail; a source-backed unknown has the stated refusal cost.
An empty positive prediction has undefined precision, still owes recall and the positive control. Schema and Go keep their dedicated floors. Any independently observed false claim of certainty under the supported setup model, inside or outside the sample (including a false full, root, terminal, mount or binding), vetoes the gate even if explained and above both rate floors. Fix or withdraw the claim in the listing and rerun the same frozen comparison. Preserve the original discrepancy and count any resulting miss/refusal in recall. Freeze full-scorable schema keys by root, native method, static chain, comparable tokens and proved compatible constraints; a suffix-scorable key needs a unique matching tail. Demonstrate any zero denominator independently. Mutate enough roots/handlers or downgrade enough true full paths to cross each threshold; 47 correct full paths and one known false full must fail despite 47/48 precision.

**Mount semantics.** Before scoring enumerate every (corpus, adapter, split) with mounts only and every edge with zero independently scorable descendants, including opaque children, blocked/empty subtrees and register calls with no grounded chain. Freeze a small independent oracle for **every** such site and its zero-or-more edges. The census totals have exactly one mount-only (corpus, adapter, split) cell: Caddy/chi non-test, one Route and zero route rows; screwdriver has six non-test and 75 test register sites to classify.
For other mounts use existing truth or the stratified protocol. Compare the exact alternative multiset at each selected site by parent and child identity (including instance/realm), prefix status and value/text, guard/transform and alternative count, independently of the span or downstream route. Missing/extra edges enter recall/precision. Mount-only controls require exact agreement on every field and edge count, including zero. Replacing Caddy's child with its parent, changing only its prefix, dropping a guard or duplicating an edge must fail the corresponding control.

**Independent exclusions and costs.** Before comparison a separate reader freezes a source-based classification at the ledger revisions. Exclude a middleware-only use or settings read by its actual arguments, a client or non-framework receiver by tracing its import/construction, a descriptor outside an installed dispatcher or inactive metadata by the framework's lookup semantics. Exclude the redundant inner Django include from the mount denominator only after pairing it to the enclosing path/re_path span (88 include spans: 74 lie inside one outer mount span, two inside two, and 12 inside none. Use the nearest enclosing path/re_path after checking the syntax, and disposition the 12 independently). Never exclude by absence from the listing, by `lookalike_possible`, by test status, or by a low schema score. A dynamic route remains eligible. Record the evidence span and reason; if there is insufficient evidence leave the site eligible and investigate.

* 695 Express use.middleware entries (124 non-test) need the argument test; 236 receiver flags (159 Express, 52 Directus, 25 Cal.com) need actual source classification. The 25 include client calls; their true exclusion count is unmeasured. In fxa 150 literal objects are candidates before checking their consumers. Prometheus's 80 own-router sites are in the Go second look, not 80 deletions from the frozen net/http ledger. Classify gin's 13 non-test internal delegations by receiver/use and seven zero-callback test calls by signature, not by directory.
* M's variable-prefix observations: Django 4, Express 3, FastAPI 1, Flask 2. A method/path can remain useful when a binding refuses. Callback-position populations exposed to the rules: 468 Express, 277 Django, 18 Flask wrapper forms; 10, 337, 2 member forms respectively; Django 31 inline and Flask 2. These are shape counts before member/import/forwarder proof. Non-test Go literal losses on the two primary application corpora are 3 and 1; the Miniflux wrappers include 14 immediate method values. See 11 for Hapi.
* Directus has 295 non-test asyncHandler call sites through its local forwarder (Opus S5); each such route callback remains wrapper_call without delegate. These calls are not 295 distinct schema operations. Census non-test Express wrapped positions excluding use.middleware: Directus 353, Express 2, Cal.com 0 (separate middleware 43/32/2). Before S3 gate join each terminal/delegate to its registration and operation, and report bound/eligible and wrapper losses **per corpus**. Other local-forwarder counts and final binding yield are unmeasured. Keep a consumer-visible binding opportunity denominator alongside the scorable-proof denominator.
* Before dispatching S4/S5 classify immutable slice expansions and package-initializer bare arguments by their syntax and exact-fact availability per corpus; M's bare terminal tally (9/2 on Apache/Miniflux) does not identify these positions. Use the shared sample to identify actual collision, blocked-remainder and uncertain Hapi item/config losses. Record the affected sites, paths and binding losses per corpus; M did not join those dimensions.
* M has no per-site result for reaching assignments, one-helper overflow, snapshot/lifecycle order, unsupported generators, language-specific template cuts, object installer proof, constraint conflicts, max-expansion cuts, or the other fine-grained refusal predicates. Their frequencies are **unknown**. Use the §7 source audit to tag each sampled site, callback and edge by predicate, estimate distinct losses / eligible sites by corpus and report lost full keys and bindings for the selected rule. For an independently enumerated population report exact counts. Zero observed in a sample is a sample result; reserve a population zero for an exhaustive oracle or source proof. Use the row-linked loss inventory, never infer incidence from fixture frequency. Failure of the floor or a material yield drop returns to design; do not rebaseline.

The frozen populations, binding floors and ledger dispositions above govern each wave. The fixture-level changes and the limits of the census evidence are in STAGE-D-ROWS.md.

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
| 7 | Hapi plugin adapter | **In scope.** Without decision 8, SH (8 days including preflight and corpus gates) buys lane 5 plugin-local routes, methods, ordered callbacks and typed suffixes. Pilot: **0/400 operations bindable from a proven root chain**. Under decision 11 a per-pair declared mount can yield separately labelled configured bindings (§11.5); their count is unmeasured. | Authorized seat must measure the per-file nearest-manifest join, weighted by entries, **before any SH dispatch**. If the ≥380/400 gate is infeasible, return to owner before building. | pre-SH |
| 8 | Indirect Hapi loading | Open after SH. Keep the strict root-chain rule. The owner can fund loader analysis. A proved loader edge still requires a static prefix for a full path. Configured joins do not require decision 8. | Mutable aggregators require separate data-flow proof. | SH gate |
| 9 | Dispatch across overlapping patterns/middleware/deployments | Use §5.2’s key-local conflict rule, closed implicit claims and named priority checks. Revisit only on measured loss. | A general dispatch simulator needs separate truth. | lane 5 integration |
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
- **Python router/view boundary (§5.3).** Section 1 of `~/prism-evidence/routes/m/UNTRACED-FORMS-CENSUS.md`
  used Flask `36e4a824f340`, FastAPI `0cb4a8e284b4` and Apache Superset `7519fa8b6f5a` for a separate set
  of syntax-only raw tallies. Section 4 instead used Superset `ad7fcac1` for the import-binding census.
  Section 1 counted 2,206 candidate registrations and mounts, including two
  receiver calls and four child-router calls with non-constructor spellings. It counted zero subscript or
  conditional shapes among the classified receivers and child arguments, and three lambdas among 58 explicit
  view arguments. The mutation predicates returned eight method-attribute write hits, zero router/app
  `setattr` or `delattr` hits and eleven functions flagged for writes to route attributes on parameters.
  The no-effect predicate found zero duplicates among 168 candidates using direct defs and simple assignments
  for the write count. Its body filter also admits docstrings. Runtime module-attribute rebinding and indirect
  view-map writes are unmeasured.
- **Closed capture and escape rule: price (PR #928 round 3; superseded by the certificate).** The
  round-3 rule that section 5.3 replaced was priced
  with a shadow evaluator (`~/prism-evidence/routes/price-b928/`) over the five listing corpora before
  it was implemented. Constructs withheld, of 107 receiver-proved / 48 handler-identity / 30 strict /
  42 default-method rows: a `global` or `nonlocal` naming the captured name 0 / 0 / 0 / 0; a module-scope
  competing binding of the name 0 / 0 / 0 / 0; the same checks along the import chain 0 / 0 / 0 / 0;
  an assignment to `.methods` outside a class body in a module the proof reads 0 / 0 / 0 / 0. Combined: 0
  withheld in every group. Two looser readings were rejected: counting a binding of the same
  spelling in any scope (a function-local variable is another binding) and counting any call that
  receives the name withhold 10 / 1 / 3 / 0 rows (CTFd `users` and `teams`, a local variable in
  a view function); a hand-read of all of them found a local shadow, not a redirect, so those
  withholdings were lost recall. Incidence of the four round-3 shapes over 4,840 modules
  of the corpora and the framework and application populations: `global` redirect through an import
  in the function body 0; direct factory call through a competing import and local definition 0;
  `Name.methods = ...` 4 (Flask and FastAPI library internals: `view.methods = cls.methods`, `cls.methods = methods`, `route.methods = ...`; none on a proof the listing makes);
  `global` declaration of a module-bound name 19 (none naming a listed receiver or handler).
- **Capture-side limits (§5.3).** The fixtures record the scanner status and native Flask result
  beside each control. With the certificate (PR #928 part 2) the pins moved: fk-92 (a comprehension
  target) and fk-87 (a method-local import of the spelling) are `PROVEN`, because another scope's
  binding is a different binding; fk-93 (an alias after a `global` rebind) is `WITHHELD`
  (`global_write`, the aliased name has a second binding); fk-94 (an in-repo star that can bind the
  name) is `WITHHELD` (`star_import`); fk-88 is `WITHHELD` by the private-name rule
  (`class_private_name`), no longer `multi_write`. fk-95 stays a known limit. Under native Flask
  fk-95 appears as an HTTP 500 unless `app.testing` is set. Job 101 measured zero registrations and
  zero mounts for each form in both populations (3,503 framework/Superset sites and 340
  application sites).
  - **Python 3.12 generic type parameter (fk-95).** Generic parameter bindings are an implicit
    annotation scope, so they are another scope's binding by scope awareness and not counted. The
    scanner selects `views.f` with no counted predicate flagged. Flask installs a `TypeVar` and
    dispatch raises `TypeError`.
  - **Rebound native Flask imports.** Native constructor and class-base recognition uses
    the framework checks. A lexical rebind of a native import can leave a source router or
    class-view metadata proof attached to the original framework binding.
  - **Parser recovery.** The lexical census counts binding nodes exposed by the parser. Clause (e)
    of the certificate refuses every key of a module whose tree holds an error or missing node,
    which closes the hidden competing write; it withheld 0 rows over 1,945 files. Tree-sitter and
    Python's `ast` can still disagree on a file that parses cleanly under one of them.
  - **Opaque Flask mounts with untraced children.** For an opaque parent, the escape index
    widens a traced Blueprint child to competing allocations at that binding. When the child
    is reachable only via an unproved read, such as a sole import inside a `with` block, the
    mount can hide a child escape. Independently proved source registrations keep their
    receiver proof. The opaque mount and paths through it earn zero installation or dispatch credit.

  **Reason-label note (IMMATERIAL).** When a later assignment hides the import in fk-89 and
  the source also declares `__getattr__`, the scanner reports `multi_write` where the
  specified precedence selects `getattr_module`. Both reasons withhold dispatch proof.
- **Census of real usage and what is out of scope (2026-11-27).** The certificate and its contract
  (5.3) cover the patterns real code uses. Evidence: `~/prism-evidence/routes/census-used/CENSUS.md`
  (sections 1, 2, 4, 5), run against the product at `cffd488d`. Public code only; the Express and Go
  frameworks were not censused.
  - **Population.** 17 repositories: the five listing corpora (CTFd, flask, microblog, redash,
    superset), two FastAPI applications (fastapi-realworld, full-stack-fastapi) and 10 applications
    chosen mechanically by `POPULATION-RULE.md` (written before any count): ihatemoney, simple-login,
    RedInk, nyaa, DockFlare, trape, FractalCore, sqlite-web, formspree, kamiFaka. Flask itself is
    framework source: shown, never counted as "used". Only one class-view-heavy application
    (ihatemoney) qualified, a recorded shortfall. The 16 applications hold 5 with proved routes
    (CTFd 77, FractalCore 33, kamiFaka 53, redash 4, simple-login 3).
  - **Patterns.** 63 patterns (57 detector patterns, H18 the static-binding baseline, plus 6 idioms),
    merged from 47 reviewer members and three enumeration passes. Counts: (i) occurrences anywhere
    in application code, (ii) occurrences touching a name a route proof relies on, by resolved
    identity. The detectors use Python `ast`, not the product parser, and over-count by design. Readers:
    every (ii) hit read blind by an Opus-5.5 reader (261 items plus 99 S08 decorator groups), 66 re-read
    by the author (59 of 66 agree; the 7 differences move no pattern between used and not observed);
    (i) sampled (424 items, seed 20261127, Sonnet-5.5). Of the 56 non-baseline patterns, 20 are used and
    36 are not observed. Used, pinned by `used::*` rows: S01+S07 a
    name bound on two branches of an `if` (trape; no proof before or after the certificate, the router
    has two bindings). Used and harmless (the proof is kept): S04/S14e/S14g loop and comprehension targets
    (simple-login, sqlite-web), S05 the same spelling in another scope (41 hits, 9 applications), S06
    star import, S08 non-router decorators (436 sites), S09 class-body names, S11 a package `__init__`
    importing its own same-named child (formspree), S22/S23/H04/H19 list, loop and factory-driven
    mounts, R01, R02, H10.
  - **Out of scope: the 36 patterns not observed on any name a route proof relies on.** (ii) is zero
    in applications for each except where noted; existing rows that pin them stay as documentation,
    re-labelled known-limit where the code no longer withholds them. Columns: (i) all repositories /
    applications, (ii) all / applications.

  | id | meaning | (i) | (ii) |
  |---|---|---|---|
  | S02 | a function declares `global X` for a module name | 105 / 104 | 0 / 0 |
  | S03 | a nested function declares `nonlocal X` | 9 / 9 | 0 / 0 |
  | S10 | a module imports from itself | 44 / 41 | 0 / 0 |
  | S12 | `import pkg` and `import pkg.sub` in one module | 31 / 31 | 0 / 0 |
  | S13 | a `type X = ...` statement | 0 / 0 | 0 / 0 |
  | S14a | a walrus target spelled like a relied name | 0 / 0 | 0 / 0 |
  | S14b | `except E as X` spelled like another module-scope binding | 28 / 28 | 0 / 0 |
  | S14c | `with .. as X` spelled like another module-scope binding | 8 / 8 | 0 / 0 |
  | S14d | `del X` of a relied name | 0 / 0 | 0 / 0 |
  | S14f | a match capture spelled like a relied name | 0 / 0 | 0 / 0 |
  | S15 | an augmented or annotated assignment of a module-scope name | 551 / 545 | 0 / 0 |
  | S16 | a tuple, list or starred unpacking target at module scope | 21 / 21 | 0 / 0 |
  | S17 | a binding of a module-system name (`__doc__`, `__path__`, ...) | 0 / 0 | 0 / 0 |
  | S18 | a non-ASCII (NFKC) identifier | 0 / 0 | 0 / 0 |
  | S19 | a file that does not parse | 0 / 0 | 0 / 0 |
  | S20 | a registration whose receiver is a call result | 344 / 344 | 0 / 0 |
  | S21 | a registration inside a def body (idiom, not a hazard; see below) | 159 / 148 | 154 / 145 |
  | R03 | `setattr` or `delattr` | 53 / 51 | 0 / 0 |
  | R04 | an attribute store or delete on an imported module name | 256 / 249 | 0 / 0 |
  | R05 | frame or globals access (`__globals__`, `_getframe`, ...) | 12 / 10 | 0 / 0 |
  | R07 | a module-level `__getattr__` | 3 / 1 | 0 / 0 |
  | R08 | a store to `__path__` or `__dict__` | 1 / 1 | 0 / 0 |
  | R09 | a subscript store through `view_functions`, `url_map` or `blueprints` | 7 / 0 | 3 / 0 |
  | H01 | a relied view passed to a call that is not a registration | 20 / 17 | 0 / 0 |
  | H02 | an attribute or subscript store on a relied view | 0 / 0 | 0 / 0 |
  | H05 | the result of a route or hook decorator call is kept | 1 / 0 | 0 / 0 |
  | H06 | a hook registration method called with a bare callable | 19 / 17 | 0 / 0 |
  | H07 | the result of `as_view(..)` is kept | 0 / 0 | 0 / 0 |
  | H08 | a view class defines `__init__`, `methods`, `decorators`, `super()` and similar | 5 / 3 | 0 / 0 |
  | H09 | a class defines a registration-like method | 15 / 5 | 0 / 0 |
  | H11a | a class subclasses `Flask`, `Blueprint` or `APIRouter` | 5 / 4 | 0 / 0 |
  | H11b | `__subclasses__`, `__bases__`, `__mro__`, `__class__` or `type(router)` | 40 / 36 | 0 / 0 |
  | H12 | a registration call with a starred argument | 5 / 1 | 2 / 0 |
  | H13 | `.record(..)`, `.record_once(..)` or `blueprint.register(..)` | 13 / 1 | 0 / 0 |
  | H15 | an `except .. as e` handler reads `e.obj` | 0 / 0 | 0 / 0 |
  | H17 | a bound name aliased by a plain `a = b` | 375 / 363 | 0 / 0 |

  S21 is a setup-function idiom, not a hazard (its (ii) is the idiom's frequency, 154 / 145, and it
  is listed here because no reader found a route change). R09 and H12 have (ii) hits in framework
  source only (3 and 2), none in applications. Several of the 36 occur elsewhere in the files (105
  `global` declarations, 256 stores on imported modules, 375 plain aliases, all repositories) but never on a relied
  name; the round-3 and round-4 review shapes (a view passed to a call H01, a store on a view H02,
  a later alias H17) are among them.
  - **Used and route-bearing: limits, not diagnostics.** R06 run-time plugin loading (CTFd
    `import_module` + `load(app)`, 1 site); H03b an extension handed the app or blueprint (flask_restx
    `Api(bp)` in CTFd and ihatemoney, Flask-SocketIO `SocketIO(app)` in trape); wsgi and URL rewriting
    (sqlite-web `PrefixMiddleware` on `app.wsgi_app`, redash `add_org_resource` rewriting URLs
    under `/<org_slug>/`); S23 blueprints mounted in a loop behind a settings-driven factory (redash
    authentication). Decision: no application-level diagnostic. The closed `Reason` enum has no value for
    them and adding one is a format change; the mount listing already reports unproven mount
    children. Each is a holder or run-time change outside the contract in 5.3.
  - **Caveats.** Detectors over-count, so "not observed" is solid for the code read but only as good as
    the relied-name resolution (a router reached through an attribute chain, a dict or a factory
    return is not tracked); registration forms the detectors do not model (`@expose`, Django-style
    tables) are outside the count; 17 repositories is a small population. The two FastAPI applications
    count for patterns only: the product lists no FastAPI routes at this commit.
- **Static import cost (§5.3).** DESIGN specifies the rule. Section 4 of
  `~/prism-evidence/routes/m/UNTRACED-FORMS-CENSUS.md` estimates its costs and supersedes the earlier
  file-wide counts. Twenty source checks trace the import and destination, ten per population.

  R and M denote registrations and mounts. For the frameworks and Superset, the source screen
  excludes 1,545 client-like sites from 3,503 scanner sites. The scanner excludes 413 Superset mock
  decorators before that site count. The five-app denominator includes all 340 scanner sites.
  Review round 2 reproduced the preceding site keys, import-reached keys and cost table.
  The outcome columns use the first counted reason. The `class_body_import` column counts
  sites with at least one such capture. The `class_private_name` column counts sites with
  a mangle-eligible name anywhere in a scanner root chain, across all scanner sites.
  The capturing-module column counts sites with a competing write to a capture's effective
  key under `global_write` or `multi_write`. These columns include sites where an earlier reason
  takes precedence. Across 3,503 framework/Superset sites and 340 application sites, the symmetric
  rule flagged 0 registrations and 0 mounts in each population. The scan found zero
  mangle-eligible root names across 3,503 framework/Superset sites and 340 application sites.
  The accepted module-scope over-refusal therefore costs zero measured registrations or mounts.

  | Framework or corpus | Revision | Scanner sites | R | M | Import-reached R / M | No counted predicate flagged R / M | `terminal_reexport` R / M | Unestablished hop R / M | Counted reasons R / M | `class_body_import` R / M | `class_private_name` R / M | Capturing-module writes R / M | Former rule flagged R / M |
  |---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
  | Flask | `36e4a824` | 683 | 354 | 74 | 3 / 5 | 3 / 5 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 3 / 3 |
  | FastAPI | `0cb4a8e2` | 2,142 | 1,358 | 152 | 1 / 5 | 1 / 5 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 2 |
  | Apache Superset | `ad7fcac1` | 678 | 14 | 6 | 0 / 1 | 0 / 1 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 1 |
  | Total |  | 3,503 | 1,726 | 232 | 4 / 11 | 4 / 11 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 3 / 6 |

  | Application | Revision | Scanner sites | R | M | Import-reached R / M | No counted predicate flagged R / M | `terminal_reexport` R / M | Unestablished hop R / M | Counted reasons R / M | `class_body_import` R / M | `class_private_name` R / M | Capturing-module writes R / M | Former rule flagged R / M |
  |---|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|---:|
  | full-stack-fastapi-template | `77e550a7` | 29 | 23 | 6 | 0 / 6 | 0 / 6 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 |
  | fastapi-realworld-example-app | `029eb778` | 29 | 20 | 9 | 0 / 9 | 0 / 9 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 |
  | microblog | `a975ef64` | 32 | 27 | 5 | 27 / 5 | 27 / 5 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 27 / 5 |
  | redash | `f561a53d` | 66 | 60 | 6 | 21 / 1 | 4 / 1 | 17 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 21 / 1 |
  | CTFd | `b88e0bf7` | 184 | 171 | 13 | 34 / 12 | 34 / 12 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 34 / 12 |
  | Total |  | 340 | 301 | 39 | 82 / 33 | 65 / 33 | 17 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | 82 / 18 |

  Every first-reason count and pairwise overlap is zero for both populations and both site kinds
  (`getattr_module`, `star_import`, `global_write`, `multi_write`, `nested_single`, `submodule_shadow`,
  `module_descriptor_name`, `class_private_name`, `class_body_import`). Other unresolved cases
  among the import-reached sites are also zero.
  Review round 1 measured 18 framework/Superset and 157 app check entries removed by skipping
  attribute-hop predicates on a from-import's module path, with the same site classifications.

  The withdrawn rule flagged 9 of 15 import-reached sites across the frameworks and Superset
  (9 of 1,958 source-screened sites). The former mutation check was first on three mounts and the
  former initialization check was first on three registrations and three mounts. Those checks
  overlapped on one Superset mount. The withdrawn rule flagged 100 of 115 in the applications,
  including every import-reached Flask site. Its mutation check was first at all 100 sites, and
  the former initialization check overlapped at 99. The application flags came from `setattr`
  on ordinary objects or `exec` on private dictionaries.

  The estimator still records 17 role-only refusals, all Redash registrations that import the
  Blueprint through a terminal re-export. Redash has four import-reached registrations and one mount
  with no counted predicate flagged. A follow-up could measure one terminal re-export hop, including
  assignment aliases of imported values.

  The scanner estimates flags from Tree-sitter syntax. Its lexical inventory now normalises
  NFKC identifiers, uses effective class-private keys for global writes and capture-side bindings,
  distinguishes annotation-only declarations from value writes and counts Python 3.12 type
  statements. It preserves raw source text
  and records lexical import-machinery writes as informational evidence. It can skip opaque ordinary
  verb calls, computed or subscripted roots, splatted arguments and module-valued assignments it cannot
  trace. It flags explicit terminal imports and simple assignment aliases rooted in a reaching import,
  uses source-only module candidates and does not model transitive wildcard
  exports, arbitrary import lookup, dynamic hooks, reflective writes, or runtime branch and
  initialization order. Static hook bindings are screened by name. The wildcard screen is broader
  than the rule when a known export set excludes the name. Imported no-effect callees, decorator and
  escape proofs, exact adapter yield and downstream paths lost per mount remain unmeasured.
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
- **Strict receiver and import proof costs recall** (factory apps, re-exported routers); include each refusal in the
  applicable recall denominator, including role-specific refusals with no counted diagnostic.
- **Out of model:** routes added at run time in loops over data, plugin systems other than the Hapi plugin
  adapter (section 11), reflection.

## 11. Hapi plugin adapter (owner decision 7)

### 11.1 Why and what is proven

A direct register-chain certificate and a plugin-contract certificate answer different questions. On the private aggregate, 58 calls contain 400 literal rows, all inside register, with 59 plain unwritten first formals in 59 plugin objects. The original root-chain rule reaches **0/400**. One mutable factory list accounts for 115 entries/27 plugins and an aggregator list for 285/32; the final root-to-factory link is unresolved. Prefixes on 57/59 plugins come from variables. The adapter can identify the server formal, entries and callbacks without inventing a deployed base.

### 11.2 Rules

1. Root: bound @hapi/hapi or legacy hapi namespace/default/require, including an unshadowed destructured and renamed constructor, with server(), Server() or new Server(). A grounded server's register accepts one item, a literal array, or one direct local/imported binding; unwrap {plugin,options,routes} and a module's plugin property.
   A mount records entry into one proved register invocation. Use item routes.prefix before call options and inherit an absent prefix. An unvalidated prefix can fail before entry (hp10). Refuse that mount, retain the object-qualified origins and diagnose the item. Apply weak proof only if independently eligible.
   On a given server use the effective name (own name or pkg.name). Admit an ordinary first registration or a legal multiple=true repeat. A proved once skip produces no mount or diagnostic. A proved disallowed repeat fails at that item. Unknown repeat, realm or option interactions withhold the disputed mount. A preceding opaque item can claim the literal sibling’s name (d1-09). Ground a spread of an unwritten literal plugin list by the same bounded item reader, otherwise treat it as opaque. An unrelated later event has no possible-failure penalty. Key an admitted realm by its parent and register span/item ordinal (hp08, es1-34). Retain the defining plugin object on every associated origin, including a refused or skipped one.
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
   Read exactly one handler location after this step: handler, options.handler, or config.handler. Missing/competing handlers give hapi_handler_conflict and no dispatch credit for a provably invalid literal. For either options or config follow one unwritten local binding or direct import/require to its object literal, including a statically named own-property chain on that object; evaluate properties and pre in their definition scope. If exactly one valid handler location is proved, its identity may remain callback_unproven on an installed path (e.g. a conditional choosing between two proved functions in the same handler field). An opaque options/config value can leave presence or uniqueness undecided. Retain the raw path, candidate and one callback_unproven slot, with paths=[] and no root failure. A proved missing or conflicting handler rejects that key.
   An object handler yields hapi_handler_object. Keep options/config.pre before the handler, role=pre and zero-based group (nested parallel entries share the stage; source order is display order); final role=handler. No pres counted as a terminal handler. Non-Hapi route-shaped candidates use object-table and no credit; the Fastify url-shaped lookalike is silent.
6. On a grounded register call diagnose each failed item exactly once at the argument expression (array element/spread including its dots). Use hapi_registration_unproven for opaque items/imports, unvalidated prefixes or repeat refusals, hapi_plugin_unproven for unsupported contract shape, hapi_server_param_unproven for an inadmissible formal.
   A failure inside an already-entered plugin is attributed once at the failing native event in the root summary. No extra item-resolution diagnostic is due on that admitted call. A valid formal shadowed only at a use retains the mount; that route alone refuses. A non-Hapi .register emits no register diagnostic. Keep the route-site candidate separately. Missing, extra and duplicate diagnostics fail the matrix.
   Nonliteral table elements do not stand for a known number of routes. Treat the legacy API, general push/factory/aggregator mount proof, Glue manifests, opaque option factories and arbitrary escaped callbacks as measured limits. Row ids and individual dispositions are in STAGE-D-ROWS.md.

### 11.3 Matrix

Enable all w3-hapi rows after the literal reader (PLAN SH). Include direct, weak and unregistered receivers, overwrite/conflict, same-plugin registrations, direct import and re-export controls, nested helper and dormant callbacks, lifecycle installation and host/pre metadata. Use the hand labels in the frozen matrix. The security/custody boundary for measurement is aggregate-only.

### 11.4 Measurement and gate

**Before any SH dispatch**, the authorized seat joins each pilot plugin file to its nearest package.json, checks the runtime/peer dependency, formal and export/use, and joins the plugin to its entries. Freeze plugin and entry counts by strong/weak/ineligible and reason, plus attainable literal-path and inline-handler counts. Return only aggregates. Run the same independent classification on the public corpora. An infeasible floor returns to the owner before SHa.

| Corpus | Pass condition |
|---|---|
| Starterkit | All 11 calls are inside plugin register, with 29 non-test literal rows and zero grounded chains. Require ≥24/29 correct weak records and ≥95% of independently weak-eligible rows, with a positive denominator. Reconcile all 29. Trace the 24 referenced configs and five other member forms. Report object hits and opaque references. Keep an installed path only when valid handler presence is independently proved. |
| Screwdriver | Reconcile 28 literal rows, 37 calls (26 inside register) and 81 register sites. Account for all 134 opaque elements individually. Require a candidate per grounded element and independent evidence for any ungrounded site. Match each failed register item to its diagnostic. The §7 registration floor counts the 11 non-test ledger literals (17 test literals reported separately). Below N=25 classify each miss. Separately report factory-element exposure recall as correctly exposed elements / 134 non-test call-shaped elements, with grounded candidates and unknown factory expansion counts shown apart. The proposed reader can expose 0/134 factory elements; even 11/11 literal recovery covers only 11/145 (7.6%) of these site/element opportunities. Report this design loss to the owner for acceptance at SH preflight. The factory output cardinalities remain unknown. |
| Hapipal | The zero-call control yields zero Hapi registrations. |
| Pilot (private aggregates) | ≥380/400 with the independently correct receiver, root and exposure. Require ≥365/384 correct literal-path exposures and ≥374/393 correct inline identities at their source span. Each numerator counts distinct correct **entries** in the frozen population. |

The pilot has 384 literal paths, 15 templates and one identifier. Compare status, value and typed tokens per entry.
Of 400 handlers, 399 are in config.handler and one is top-level. Beyond the 393 inline handlers, four are wrappers, two members and one an object. The aggregate counts zero route-level pre/vhost forms; register-level realm constraints still require proof.
The authorized seat uses the independently labelled entry oracle to join all 400 positions before aggregation and reports each disposition, with zero unexplained.
Apply the §7 false-certainty veto to each observed false identity, wrong root and false full. Report strong/weak and full/suffix/dynamic separately.
A certified surviving strong all-literal chain requires full. The weak-only population has no positive full-path minimum.

### 11.5 Lane 5 configured mounts

Under question 11’s proposed opt-in, lane 5 may join a plugin_contract record to a per-pair declaration naming a root, valid literal Hapi mount and the same origin_plugin (defining file and object byte). Require a null-root static suffix with validated path_tokens and mount_refusal, a unique compatible protocol/method and known realm host/version, setup and dispatch constraints. Binding needs one proved terminal (resolved or anonymous) or certified delegate.
Lane 5 owns composition and its tests. Use §5.2’s Hapi join (/api plus / gives /api), preserve token boundaries, and refuse ambiguity or unknown constraints. Label the result **configured** with configuration provenance, plugin, original suffix and refusal. Producer gates count only producer evidence.

Without a configured match the record remains plugin-local. Decision 7 alone gives the pilot zero producer-proven bindings. The controller's prototype bound 236/316 client calls with zero wrong in 50 checked. That measurement does not establish SH's configured yield.
