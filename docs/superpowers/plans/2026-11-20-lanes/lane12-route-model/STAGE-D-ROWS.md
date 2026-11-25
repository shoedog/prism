# Stage D row decisions

Authority is the 420-row `tests/fixtures/routes/rows.jsonl` at b924b349a63fd64750c944125bf5a2dafa36eedb, SHA-256 `4649d274a9342339e558b05c36e9a9cfac995e483db019523b2000967355af15`. The 158 executed receipts hash to `d133682d6cf1532e0f2ef31f8f7930a17cb7f7a588063dc65000e5f85838e4e4`. The remaining 262 rows are hand-derived. Of the 420, 222 carry `d:true` and 62 carry an accepted-loss proposal before this patch. The 83/8 counts in D-FACTS and the seed-only totals in RECONCILE belong to earlier snapshots. CLASS_MAP supplies the census-class mapping; group membership below includes later E rows.

## How to compare the result

1. Compare every static record and mount once by site/element/installation under DESIGN 5.1–5.4. Materialize all source anchors (including element_site and install_site) as UTF-8 byte ranges. A source occurrence counts from one across the named file. Partial nested objects constrain the named fields; arrays pin multiplicity, ordered paths and callbacks. Route and mount collections are matched one-to-one. Check deterministic wire order separately. For callback.delegate, null pins absence; an identity pins that exact target. The fixture's endpoint field is the endpoint name on an endpoint-only rule. Compare expect_diagnostics (default []) with **all** summary.diagnostics as an exact multiset of source spans and reasons; missing, extra, duplicate or coalesced failed items fail. A register-item span denotes the expression inside the call, including a spread's dots. Keep unlisted controls absent. No ignore or provenance change is authorized.
2. Match an executed declaration to the saved raw call by file, line, and ordinal on a shared line. Follow its call_ids and mount_call_ids into the exported root's served entries. A full literal alternative must be present there; compare all comparable alternatives, including multiplicity by **origin**. A suffix must match a linked full tail unless the branch is explicitly unattached, cyclic or truncated. When the static rule withholds proof, compare registration presence and the safe projected fields; report the withheld runtime exposure separately. fl21 is a static-only lookalike with zero native calls. et1-10 and es1-20 have native generated routes and es3-07 a saved decorator application, all with zero modeled declarations; reconcile each against its recorded source-site loss. A runtime-only site in any other row needs its own disposition.
3. Keep declared methods distinct from native effective methods. A missing declaration gives null (fl03); FastAPI's websocket kind and empty HTTP effective set compare to protocol websocket and null methods (fa06, es2-10). A literal [] compares as known empty, with the pathname retained (es3-10). Express all projects to Any over its framework table. An unknown methods expression can have a definite native set; record it without attributing the set to source literals. Compare raw expression spelling to the traced origin; compare a decoded scalar or folded literal to the **evaluated call argument** (es1-68, es3-02), a leaf array to individual served paths (et1-09), and a regex to the raw pattern/flags (ex13). The saved runner's older null literal/normalized helper fields do not change those observations. Build normalized token keys independently from native spelling and the adapter grammar; literal braces in es5-05 stay literal.
4. Compare resolved/anonymous callback identities at their native positions. Source-level unresolved reasons, a forwarded delegate and roles are separate assertions. For fl12/es2-07 compare the empty written callback array; retain the served endpoint map's null or later-bound callable as runtime evidence. For a wrapper compare position and origin expression, not the returned wrapper's name to its argument. An opaque spread stands for a variable-length native run with aligned known prefix/suffix; here es1-70's entire literal spread is verifiable. Python's decorated code starts at the first decorator for the runtime comparison; the route site is the individual decorator/call. es7-08 must pair the two installation epochs by call ids even when the served strings and Python callable are identical. Shadow status compares installation order, not a deduplicated path set.
5. A hand row remains hand even where a pinned runtime would likely agree. In particular fa12, ex30, es4-04/07 and es7-07 have runner coverage limits. Static suffix, root, host, version, guarded methods, diagnostics, normalized tokens, and the one-hop Go/Hapi proof require listing tests; no hand-written output can certify runtime execution. After approval the implementer updates the harness as well as the expectations and runs the same-environment base controls before enabling each group.

On the proposed pins the seed-era reconciler has five field differences across three rows. For es1-68 and es3-02 it compares four decoded path literals against the saved helper’s nulls. Compare the four raw rule/path arguments and their linked served strings instead. For es5-05 it compares a literal path key to the helper’s null; the saved served string is `/examples/:name/{draft-name}`, whose hyphenated text is outside the Starlette parameter-name grammar. These changes affect static projection only.

## Query roots

Pass the fixture's entry verbatim to --entry on 171 primary runs with a statically selectable entry (including hand rows). For every full exposure in those runs independently resolve the fixture entry's expected root from its source; assert equality even if an individual path pin omits root. Every full anywhere requires a non-null valid root. For the other 249 primary runs exercise inference (248 without an entry plus fl15); es4-11 pins both independent muxes and the nested factory call. es6-04 pins the configured URLconf and orphan side by side.

| Primary entry cohort | IDs |
|---|---|
| Python module binding (83) | fl01–fl14, fl16–fl17, fl19–fl21, fa01–fa14, cx02, cx05–cx06, cx19–cx28, et1-01–et1-02, et1-04–et1-05, et2-03, et3-02, et6-01, et8-01, es1-13–es1-16, es1-18, es1-20–es1-22, es1-68, es2-07–es2-12, es3-06–es3-10, es4-04–es4-07, es5-03–es5-05, es7-07–es7-08 |
| Python factory (4) | fl18, et1-03, es1-17, es1-19 |
| JS/TS module export (84) | ex01–ex30, cx01, cx03–cx04, cx11–cx18, et1-06–et1-10, et2-01–et2-02, et2-04–et2-05, et6-02, et8-02, es1-01–es1-12, es1-69–es1-70, es2-01–es2-06, es2-34, es3-01–es3-05, es4-01–es4-03, es5-01–es5-02, es5-16 |

entry_queries are additional **static** CLI runs with their own --entry. For each expect_paths compare the exact path collection of the named origin, including its root. et8-01/02 select the mounted child, es3-09 the source app of a copy; the primary receipt, where executed, remains scoped to the original entry. Factory controls fl18, et1-03, es1-17 and es1-19 pin the synthetic entry invocation frame. fl15's runtime export is the ungrounded instance member site.app. Its primary static run infers and retains the candidate; a separate explicit app:app query must report entry_unproven. Keep the raw runtime entry unchanged. For invalid/ambiguous entries require an explicit query error. hp08 and es1-34 pin a realm as the ancestry of register-call byte spans and per-call item ordinals (zero for a scalar item).

The harness checks §5.4 presence and nullability on **every** emitted variant, including absence of proof on strong/unproven receivers and of plugin on direct/unproven receivers. Independently locate each strong/weak plugin object at its defining literal, compare its association with the origin even if the formal/register bytes coincide, and materialize realm/root frames from setup source. Do not derive expected identity from the listing. On hp06 patch the two plugin identities; hp22/cx07 have a closed, vhost-free linked call set. Apply the same source-root equality check to inferred multi-root cases d1-03/12/13 and the d2 Go controls. Optional expect_failed_roots and expect_uncertain_roots arrays compare their summary collections as exact multisets by root, event span and reason. Check both on each interaction control, including empty sets on healthy controls. hp20 enters the realm then aborts at its first invalid row; hp10’s later dynamic GET may conflict, and cx48’s later opaque route item may reject after /ping was installed. Keep earlier established facts and suppress proven binding on the affected root.

An optional expect_bindings array drives a **test projection** for specified (selected root, method, value) probes. Resolve the named endpoint independently in the fixture; null requires no unique eligible terminal. Use the §5.2 selection and declared/default operation rule with §5.4 blocked methods. Compare production binding guards as separate fields. es4-01’s saved truth certifies all four installs. On separate /ledger layers the earlier GET can handle HEAD and readLedger sends the response. The conservative projection refuses HEAD (never checkLedger); on the shared /manifest Route the explicit HEAD suppresses GET fallback and checkManifest is eligible. ex28 is the shared-builder control. In et1-08 all and GET may participate; es1-10 and d1-05 admit continuation. Membership, winner and tested HTTP operation have separate assertions.

## Incidence of a partial result

| Rule family | Measured incidence or how to count it |
|---|---|
| Receiver/import, missing mount or outside a root | 236 Express lookalike flags are a review population. 150 fxa candidate rows require consumer proof. Ledger lacks reaching-definition, import-hop, root-membership and alias lifetime fields. Count per-site proof on the frozen source, including phased registrars and depth >1; match to the site ledger before comparing extractor output. |
| Prefix/regex/key, guards, cap and cycle | Prefix dimensions (Django 4, Express 3, FastAPI 1, Flask 2 variable) and 160 Django re_path syntax sites bound parts of the population. The ledger has seven dynamic ServeMux patterns. It does not encode regex language, host/method intersections, root reachability, copies, cycles or path multiplicity. Count chains per independently identified origin with a reference replay. |
| Receiver typed but handler withheld | M counts wrapper/member/inline/other forms, not their binding at use. See DESIGN 7 for totals. Inspect the source position for the exact forwarder import, class view, decorator, member, lookup and spread. E.g. the 31 Django inline forms and two Flask inline forms bound the Python anonymous loss; 12 Django other handlers bound the subscript case. The 14 Miniflux immediate method-value wrappers get no local-wrapper delegate. A correct middleware-only exclusion does not enter callback loss. |
| Go selectors | On Apache/Miniflux non-test terminals the M2 admitted totals are 217/150; 0/1 external interface-typed terminals give method_value_external. There are zero generic terminals in the census. Count qualified/local promotions by declaration path and profile; the 217/150 are all direct. Package-function positions are 3/0 (all non-terminal). No terminal method expressions, three-hop fields or type-parameter shapes counted there. The Go second look has one Prometheus call-result selector. M did not prove static lookup under every scope, build partition or embedding: count outcomes independently at each selector, by corpus and terminal/all positions. |
| Hapi plugin and table | Bare chain reaches 0/400 private entries and 0/48 public route calls. 134 factory elements in screwdriver are opaque route candidates, outside its 28 literal ledger rows. Private census: 0 pre, vhost, defaulted/written plain formal, legacy, or nested route installations observed; 4 wrappers, 2 members, 1 object. Starterkit has 24 config references among 29 rows. Trace direct imports/own properties, classify unresolved config as callback_unproven with exposure retained; count final weak eligibility and every handler location on the public corpora. |
| Unsupported registration API, data-generated rows, inactive or unknown installer | Independent census only counts its named syntax classes. Untallied APIs, implicit setup, translations, re-exports, indirect manifests, loops, request-selected configurations and custom composed decorators have unknown frequency. Inspect source at the pinned revisions for each construct (including sites *outside* the ledger), split real installed, inactive, pending/conditional and unsupported, and report numerator and eligible sites or positions. A zero ledger cell alone does not establish zero loss. |

The duplicate/refusal counts for Flask, FastAPI, Express, gin, ServeMux, chi, Django, Nest, native Map and Hapi remain unmeasured per corpus. Review Sol 6 identifies one Express test pair. Count by independently replaying writes or matching registrations on each dispatcher with its active constraints, including terminal continuation, rejection and superseded keys. For typed host and explicit-root cuts, join selector grammar, independently bound root and schema authority at the same revision. Report withheld keys and bindings per corpus alongside the positive results.

## Dispatcher interactions

**Question at the two-round cap.** What does the listing claim when registrations interact (shadow, conflict, overwrite, reject, fall through) in the framework's own dispatcher? The first round found Express continuation and Map overwrite. The second added distinct-key Go conflicts, HEAD across Express layers, Hapi association and sibling admission, and chi replacement. All use the same installed-event population. §5.2 now specifies the evidence required for an exposure and for a binding.

| Design-seat option | Benefit and limitation | Recall cost on the census | Incremental implementation estimate over round 1 |
|---|---|---|---:|
| A. Keep every record, withdraw both full exposure and binding on every potential overlap whose outcome declared facts cannot decide. Use disjointness and no detailed replay. | Small certificate surface. Also withholds genuine co-installations and proved replacements that a native table would distinguish. | No loss of origins. At least two established installed origins lose full credit in the reported Express test pair under a blanket overlapping-layer cut. Zero chi registration losses in Caddy and zero strong-path losses on the weak-only Hapi populations below. Counts elsewhere need the event join below. | +2 days |
| B. Port each pinned adapter’s native installer and request matcher for all represented static forms, including path precedence, tree insertion, effective methods and realm state. Run reference fixtures offline. | Recovers the most statically decidable installs and winners. Ten version-specific models to maintain. Even a native matcher cannot determine arbitrary handler continuation, opaque inputs or deployment choices. | Keeps both Express test installs. No new root or prefix for 400 pilot entries, 29 starterkit literals or the 28 screwdriver literal sites from the strict plugin chain. Caddy chi has no registration to recover. Other gains require paired replay and a certified endpoint. | +16–24 days |
| C. Bounded native certificates plus a common unknown outcome (§5.2), selected. | Proves co-installation separately from the winning endpoint, certifies writes, skip/reject and native disjointness/specificity for admitted facts. Unknown installation costs the full path; uncertain dispatch on an installed pattern costs the contested binding. Fits existing adapter checkpoints. | Keeps both installed Express test origins, withholds the one contested GET operation from unique binding. Same zero strong Hapi and chi deltas as above. Recovers the final chi/Map write when certified; the census has zero chi registration sites and 150 fxa table candidates of unknown installer eligibility. | +5 days |

The counted Express witness is in **tests**. It establishes no non-test rate. Caddy has one chi mount; Hapipal and Fastify/Koa/DRF have zero registration sites in scope. The Hapi counts describe root-chain recall, not absence of interaction. Pilot has 59 inline register functions, starterkit 17 and screwdriver 30 non-test plus 13 test inline functions: zero shared-reference losses in those observed plugin shapes. Starterkit has no register calls: any weak result among its 29 literals has unknown realm host and earns no configured binding from a prefix alone. The pilot aggregate does not establish per-plugin realm freedom. No per-corpus pair rate follows from the 232 Apache gin sites, 179 Miniflux mux sites, or seven dynamic ServeMux patterns. Before implementation acceptances, freeze per-dispatcher events, instance/epoch, effective request sets, startup outcome and consumer terminal; classify A/B/C counterfactuals by hand on the same independently labelled interaction sample (§7), count distinct retained origins, lost full keys and lost unique operations per corpus/split against the same opportunities. Publish unknown where the reference cannot decide. Material yield loss goes to the owner before merge.

**Enumeration.** Start with all ten adapter labels in the 420 row recipes, the 15 first-round additions and both rounds’ counterexamples. For each native operation (registration, copy, mount/delegation, metadata or table write, plugin invocation), cross earlier/later events on the same resource, then repeat on independent roots. Vary order (known/reversed/conditional), epoch, object identity and request sets (disjoint/equal/subset/crossing/unknown) across method including implicit methods, protocol, path grammar, host, version and guards. Compare the action at install time first: both, skip, replace some/all, reject/abort, or unknown. For two surviving entries compare first-match, specificity, continuation, or unknown on the intersection. Setup state is a sequence: add a third sentinel after a possible abort, and a second method/independent root to detect over-withdrawal. Test the product against independent case outcomes, not the adapter's own matcher. Unlisted native syntax and unknown order use §5.2's unknown result; a new path spelling never defaults to disjoint.

| Adapter and controls | Native interaction and certificate | Remaining loss or control |
|---|---|---|
| Flask/Werkzeug · fl04, es4-04, es7-07, d1-10, d2-14 | Rule matching and endpoint-to-view installation are separate. Same endpoint/different callable can assert even at disjoint URLs. Reuse of the same callable is legal. Blueprint name conflicts, once-only replay and build-only rules affect admission. Separately compare host/subdomain, converters, slash policy, method defaults and implicit HEAD/OPTIONS. Both installed URL rules can survive a priority contest. | Block contested methods where specificity/tie or view metadata is unknown. No binding on an aborted root, even if a rule was inserted before the assertion. Omit implicit redirects. |
| FastAPI/Starlette · es2-10, es4-07, es7-08, d2-15 | Replay include copies at their epoch and live mount order. First FULL native match wins; an earlier PARTIAL method match yields to a later FULL match. Include wildcard methods, Route's HEAD fallback where applicable, protocol, typed/literal overlap and prefix mount delegation. An admitted mount may own the request even when the child later yields 404. | Retain both proved installed entries. Unknown converter/mount scope blocks the contested binding; no fabricated child callback or redirect. |
| Express 4 · es1-10, es4-01, et1-08, ex28, d1-05 | Track layer order and the native Route object shared by builder legs; separate app.get/app.head create separate layers. GET can take HEAD iff its Route has no explicit HEAD. Include all, implicit methods, path/prefix intersections, arrays, case/strict options, middleware and mounted router continuation. next(), next('route'), next('router') and response/error termination do not erase installations. | Keep proved layer installs. Without a terminal-flow certificate, block the effective overlapping methods even when just one layer **declares** that operation. Preserve GET on /ledger. No inferred HEAD schema key, no middleware-only endpoint. |
| gin · gn07, es2-15, d1-12, d2-11 | Compare insertions in each engine’s per-method radix tree after Group join and Match/Any expansion. Reject duplicate terminals, incompatible wildcard names/types at the same node, and catch-all vs static/parameter descendants in either insertion order when native insertion rejects. Ordinary static/parameter siblings can coexist with static priority. Compare tree slots, even for unequal path keys. | Preserve successful earlier legs of a multi-method write, record the failing leg, abort subsequent legs and dependent setup. Unknown tree/insert order gets no speculative full. No automatic slash redirects or NoRoute binding. |
| Go 1.22 ServeMux · nh06, es3-17/18, es7-01/02, d1-13, d2-10 | Compare native sets for method (GET includes HEAD; methodless includes all), host, decoded segments, one/many wildcards, subtree and exact end. Equal or overlapping incomparable patterns conflict unless the native host-qualified vs hostless exception orders them. Strict subset wins, disjoint sets coexist. Include mounts as handlers at their outer patterns before composing the child guard/strip. | Keep rejected source with dispatch_conflict, record failed mux; do not normalize schema tokens to test installation. Unknown pattern/version or mux membership cannot certify success against a possible rival. |
| chi · ch02/03, es3-19, es4-13, es5-10, d1-15, d2-13 | Replay straight-line writes to the same native leaf and effective method slot (With/Group can share the tree); last handler survives. Any/Method writes can affect several slots. Use native static/regexp/param/catch-all branch priority where certified. Route/Mount also installs delegation patterns and can reject a duplicate mount before a later registration. | Preserve partial survivors and superseded keys, distinguish unknown final write from known replacement. Complex regex overlap, opaque mounted handlers and unknown method tables lose affected winner credit. |
| Django · es3-23/24, es4-16, es5-13, d2-16 | Match installed URLconf sequence in order, including literal vs converter/regex, empty remainder, nested resolver success vs Resolver404 fall-through, active language and setup guard. Same-name reverse lookup has no dispatch effect. For a proved encompassing earlier success block the later endpoint on that intersection. | Custom converter behavior, request-selected URLconf and complex regex language lack a winner certificate. Keep proved patterns, block ambiguous bindings. |
| NestJS · es1-59, es2-32, es3-25, es7-09, d1-11, d2-17 | Resolve decorator overwrites/overrides and module/bootstrap reach before host/version filtering. Distinct context or disjoint literal versions can coexist. Compare wildcard verbs, underlying HTTP adapter's effective method/path and order including competing controller mounts. Default Express uses ordered installation; an explicit incompatible/opaque platform cannot borrow its acceptance. | Preserve metadata origins that contribute registrations. Unknown adapter admission withholds full on contested installs; uncertain host/version or request-time precedence withholds binding. Custom version selectors need independent scope evidence. |
| Certified object-table · es2-33, es3-27, es5-15, d1-06, d2-18 | JS own-property/spread evaluation determines a row’s values first. For the same native Map and actual dispatch string replay construction/set at each reachable installation. Last write wins per key; earlier record retains other live methods. Prefix alternatives, different Maps and passive reads have independent dispositions. | No certified final write after possible mutation/escape or uncertain order; report contested key and retain origins. User-defined equality or unmatched lookup is outside the Map certificate. |
| Hapi · hp08/20/22, cx07/48, es1-27/34/35, d1-03/04/09/14, d2-01–05/12 | Track server-wide effective plugin names, object-specific associations, parent realm and per-item repeat/skip/abort before route keys. One register function can install for two objects. Earlier opaque items can claim a name, install overlapping routes or abort before later items. Compare method arrays, literal/parameter/optional/wildcard path fingerprints and specificity, effective vhosts and fallback methods on the server across realms. Proven equivalent conflicting slots reject; known disjoint methods/vhosts retain their installations. | Opaque siblings, validation options or realm constraints get only common facts. Rejected/never-reached event yields no mount or served route; an independent weak association can remain. No loader/factory simulation, arbitrary hooks, or implicit operation binding. |

These losses are part of the decision: no handler-body/control-flow interpreter (Express middleware and continuation), arbitrary app execution or factory expansion (Hapi, Map), general regex/converter-language solver (chi, Django, Flask, JS), or inference of deployment/third-party adapter settings (Nest, Hapi, legacy mux mode). Existing proofs still apply to disjoint static portions. When a contested request subset cannot be expressed, withhold binding for the whole affected method/pattern; never narrow a pattern by guessed normalization. Pin the degraded case beside an independent survivor. Measure losses by distinct origin and operation as above.

**Mount controls independent of descendant paths.** The only mount-positive/registration-zero cell in the 53-corpus census.json ledger totals is Caddy/chi/non-test. At acmeserver.go:211 expect parent `r := chi.NewRouter()`, distinct child `func(r chi.Router)` formal, prefix {status:unevaluated,text:"ash.PathPrefix"}, no request guard, one alternative. Use d2-20 for independent single-field corruptions. The aggregate cannot identify all edges without scorable descendants in mixed cells. Before scoring, freeze a per-site list there, including Django include aliases, unproved child/excluded or end-blocked remainders, Hapi register items with zero admitted mounts, and dynamic-only branches. Check the **complete** edge multiset against a manually source-labelled oracle on each listed site. In the matrix fa13 (opaque include), es3-19 (opaque chi child beside a direct route), es3-24 (one blocked remainder) and hp20 (admitted realm, zero dispatchable rows) exercise different ways a route-based check could miss a bad mount.

## Go selector inventory for S4b

Read each row against the lexical base, explicit hops, promoted selector and visible declaration rules in DESIGN 5.6. Registration/handler facts retain their own source sites. These classes also govern new combinations of the same shapes.

| Selector or operand class | Baseline controls |
|---|---|
| Enclosing receiver, typed parameter, local or package var; direct pointer/value methods; named one/two-hop field and one local copy | gn03, gm01–07, gm18, nh01, cx35–40 |
| Concrete assertion (single/comma-ok), explicit or inferred composite var, constructor, imported or multi-result return slot | es1-36/37/65, es2-19, es3-13, gm04, cx36/37/39 |
| Rebinding same typed variable, block/closure/if shadow and expiry; package aliases, defined types, test/build visibility | gm02/11/12, es1-45/47/48, et4-02/03, es5-07/08, es7-03/04 |
| Implicit value/pointer/qualified promotion, minimum depth, named-field exclusion vs explicit embedded pointer/qualified hop | gm08/17, es1-39/40/41, es2-17. Only implicit promotion can cross the latter two embedded shapes. |
| Instantiated parameter/composite, implicit/explicit instantiated embedding, type parameter | et3-03, et4-01, es4-10, es1-46. Preserve slots as method_value_receiver_unproven. |
| Interface field/parameter/local/alias, promoted interface vs direct method, explicit interface over a concrete initializer, type-switch implicit variable | gm09/10, cx41, es1-38, es2-16/17, es3-16, es4-09. Imported external owner uses gm16 precedence. |
| Call result, index, deep chain, unary/parenthesized operand, multi-name/second local copy, range, conversion, anonymous struct | gm13, es1-42/43/44, es2-18, es3-14/15. No M1 proof. |
| Function-valued field or variable, bare alias, literal, package selector vs shadowed import, method expression, wrapper/conversion | gm14–16/19, cx09/42/43, es1-47, es5-06. Keep each callback position, applying the exact-fact/lexical fallback and closed delegate rules of 5.1. |

## Loss and reason crosswalk

The row patch fixes each accepted loss on its owning ID. The index below also includes partial outputs and correct negative controls that exercise the same wire reason. An unproven fake receiver is a negative control, not a missed route. A single row can exercise more than one rule. Frequencies refer to the independent populations below and in DESIGN 7/11, never to the number of rows in this index.

| Reason | Rows exercising the cut or diagnostic |
|---|---|
| anonymous_unindexed | gn02, nh02, cx26, cx43, cx57 |
| build_only | es7-07 |
| callback_unproven | cx59, es2-26, d1-02 |
| dispatch_conflict | hp10/cx48 (possible abort), d1-12/13/14, d2-10/11/13/14/17/18 |
| entry_unproven | fl15 explicit query, d1-07 missing export |
| superseded | d1-06, d1-15, d2-13 |
| class_view | fl13, cx05, es1-51 |
| expansion_truncated | es2-34 |
| func_field_value | gm14 |
| handler_object | nh03, es5-16 |
| hapi_handler_conflict | hp20 (failed root) |
| hapi_handler_object | hp11 |
| hapi_plugin_unproven | hp14, hp15, hp21, cx46 |
| hapi_plugin_unregistered | hp16, es1-25, es1-27, es1-32, es6-03, d2-06–09 |
| hapi_registration_unproven | hp22, cx07, es1-27, es1-32, d1-03/04/09, d2-04/12 |
| hapi_route_not_literal | cx47, cx48 |
| hapi_server_param_unproven | hp17, hp18, es1-33 |
| inner_decorator | fl20, es2-29, es3-06 |
| member_handler | hp11, cx27, cx50, cx58 |
| method_expression | gm15 |
| method_value_external | gm16 |
| method_value_interface | gm09, gm10, cx41, es1-38, es2-16, es2-17, es3-16, es4-09 |
| method_value_method_unproven | et4-02, d2-19 |
| method_value_receiver_shadowed | gm11, gm12, es5-07, es7-03 |
| method_value_receiver_unproven | et3-03, et4-01, es4-10, gm13, gm17, es1-40, es1-42, es1-43, es1-44, es1-46, es2-16, es2-18, es3-14, es3-15, d2-19 |
| mount_child_unproven | fa13, cx01, es1-05, es1-06, es1-15, es1-16, es3-19, es6-05 |
| mount_cycle | es1-02 |
| no_known_mount | ex29, gn05, hp22, fa14, cx01, cx07, es1-04, es1-05, es1-06, es1-15, es1-16, es1-17, es4-02, es6-04, es6-05, es6-09, d2-01/02/05/08/09 |
| non_literal_path | ex12, nh07, hp10, fl14, fa07, cx34, es1-68, es2-18, es2-28 |
| qualified_function_value | gm15, es1-47 |
| receiver_unproven | fl15, fl21 |
| regex_path | ex13, cx52, es5-13 |
| runtime_generated_registration | et1-10, es1-20 |
| table_consumer_unproven | ol05, hp12, es1-26 |
| unsupported_registration | fa12, et1-12, es2-30, es3-07, es3-08 |
| variable_prefix | ex23, gn06, nh08, hp10, fl08, fa10, cx55, es1-03, es2-05, es4-15, es4-17 |
| wrapper_call | ex03, ex04, ex05, ex08, ex09, gn08, gm19, hp11, cx09, cx28, cx60, es3-03, es3-04 |

## Patches by enabling group

One JSON object per input ID. The class identifies the fixture; apply only the RFC 6902 operations in `patch` to the matching baseline row in its original position. An empty patch ratifies the original pins. Removing `d` alone ratifies those exact arrays and fields. Explanatory `note` remains the original author’s provenance. All unpinned production fields still follow DESIGN.

### core (2)

```jsonl
{"id":"e01","class":"no_framework","patch":[]}
{"id":"e02","class":"framework_without_registration","patch":[]}
```

### w1-python (88)

```jsonl
{"id":"fl01","class":"explicit_methods","patch":[]}
{"id":"fl02","class":"verb_decorator","patch":[]}
{"id":"fl03","class":"implicit_methods","patch":[{"op":"add","path":"/expect/0/methods","value":null},{"op":"remove","path":"/d"}]}
{"id":"fl04","class":"stacked_decorators","patch":[]}
{"id":"fl05","class":"blueprint_constructor_prefix","patch":[{"op":"add","path":"/expect/0/paths/0/path_tokens","value":[["literal","/auth/"],["parameter"],["literal","/u"]]},{"op":"add","path":"/expect/0/methods","value":null}]}
{"id":"fl06","class":"blueprint_mount_prefix","patch":[{"op":"add","path":"/expect/0/methods","value":null}]}
{"id":"fl07","class":"blueprint_prefix_override","patch":[{"op":"add","path":"/expect/0/methods","value":null}]}
{"id":"fl08","class":"blueprint_variable_prefix","patch":[{"op":"add","path":"/expect/0/methods","value":null}]}
{"id":"fl09","class":"blueprint_repeated_mount","patch":[{"op":"add","path":"/expect/0/methods","value":null}]}
{"id":"fl10","class":"blueprint_nested_mount","patch":[]}
{"id":"fl11","class":"add_url_rule_callback","patch":[{"op":"add","path":"/expect/0/methods","value":null}]}
{"id":"fl12","class":"add_url_rule_endpoint_only","patch":[{"op":"add","path":"/expect/0/methods","value":null},{"op":"add","path":"/expect/0/callbacks","value":[]},{"op":"add","path":"/expect/0/endpoint","value":"i"},{"op":"remove","path":"/d"}]}
{"id":"fl13","class":"class_view","patch":[{"op":"add","path":"/expect/0/methods","value":null},{"op":"remove","path":"/d"},{"op":"add","path":"/accepted_loss","value":"class_view"}]}
{"id":"fl14","class":"non_literal_path","patch":[{"op":"add","path":"/expect/0/methods","value":null}]}
{"id":"fl15","class":"member_receiver","patch":[{"op":"add","path":"/expect/0/methods","value":null},{"op":"add","path":"/expect/0/candidate","value":true},{"op":"remove","path":"/d"},{"op":"add","path":"/accepted_loss","value":"receiver_unproven"},{"op":"add","path":"/query_entry","value":null},{"op":"add","path":"/entry_queries","value":[{"entry":"app:app","expect_error":"entry_unproven"}]}]}
{"id":"fl16","class":"concatenated_prefix","patch":[{"op":"replace","path":"/expect/0/paths/0/status","value":"full"},{"op":"replace","path":"/expect/0/paths/0/value","value":"/a/x"},{"op":"remove","path":"/expect/0/paths/0/reason"},{"op":"replace","path":"/expect_mounts/0/prefix/status","value":"literal"},{"op":"remove","path":"/expect_mounts/0/prefix/text"},{"op":"add","path":"/expect_mounts/0/prefix/value","value":"/a/"},{"op":"remove","path":"/d"}]}
{"id":"fl17","class":"non_literal_methods","patch":[{"op":"remove","path":"/d"}]}
{"id":"fl18","class":"factory_local_router","patch":[{"op":"add","path":"/expect/0/paths/0/root","value":{"file":"app.py","binding":"app","construction_byte":54,"invocations":[{"entry":"app:create_app()"}]}}]}
{"id":"fl19","class":"qualified_constructor","patch":[]}
{"id":"fl20","class":"inner_replacing_decorator","patch":[]}
{"id":"fl21","class":"shadowed_receiver","patch":[{"op":"add","path":"/expect/0/candidate","value":true}]}
{"id":"fa01","class":"verb_decorator","patch":[]}
{"id":"fa02","class":"router_constructor_prefix","patch":[]}
{"id":"fa03","class":"router_composed_prefix","patch":[]}
{"id":"fa04","class":"api_route_methods","patch":[]}
{"id":"fa05","class":"add_api_route_callback","patch":[]}
{"id":"fa06","class":"websocket_decorator","patch":[{"op":"add","path":"/expect/0/protocol","value":"websocket"},{"op":"add","path":"/expect/0/methods","value":null},{"op":"remove","path":"/d"}]}
{"id":"fa07","class":"non_literal_path","patch":[]}
{"id":"fa08","class":"router_repeated_include","patch":[]}
{"id":"fa09","class":"router_nested_include","patch":[]}
{"id":"fa10","class":"router_variable_prefix","patch":[]}
{"id":"fa11","class":"aliased_submodule_router","patch":[]}
{"id":"fa12","class":"static_files_mount","patch":[{"op":"remove","path":"/d"},{"op":"add","path":"/accepted_loss","value":"unsupported_registration"},{"op":"add","path":"/expect_diagnostics","value":[{"reason":"unsupported_registration","site":{"file":"app.py","line":7,"text":"app.mount(\"/s\", StaticFiles(directory=Path(__file__).with_name(\"assets\")))"}}]}]}
{"id":"fa13","class":"unproven_include_child","patch":[{"op":"remove","path":"/d"}]}
{"id":"fa14","class":"aliased_member_selects_router","patch":[]}
{"id":"cx02","class":"flask_blueprint_package_init","patch":[]}
{"id":"cx05","class":"flask_method_view","patch":[{"op":"add","path":"/expect/0/methods","value":null},{"op":"remove","path":"/d"},{"op":"add","path":"/accepted_loss","value":"class_view"}]}
{"id":"cx06","class":"fastapi_route_dependency","patch":[]}
{"id":"cx19","class":"fastapi_decorator_delete","patch":[]}
{"id":"cx20","class":"fastapi_decorator_head","patch":[]}
{"id":"cx21","class":"fastapi_decorator_options","patch":[]}
{"id":"cx22","class":"fastapi_decorator_patch","patch":[]}
{"id":"cx23","class":"fastapi_post_cross_file_literal_prefix","patch":[]}
{"id":"cx24","class":"fastapi_decorator_put","patch":[]}
{"id":"cx25","class":"fastapi_decorator_trace","patch":[]}
{"id":"cx26","class":"flask_inline_lambda_view","patch":[{"op":"add","path":"/expect/0/callbacks","value":[{"position":0,"status":"unresolved","text":"lambda: \"ok\"","reason":"anonymous_unindexed"}]},{"op":"remove","path":"/d"},{"op":"add","path":"/accepted_loss","value":"anonymous_unindexed"}]}
{"id":"cx27","class":"flask_member_view","patch":[]}
{"id":"cx28","class":"flask_wrapped_view","patch":[{"op":"add","path":"/expect/0/callbacks/0/delegate","value":null},{"op":"remove","path":"/d"}]}
{"id":"et1-01","class":"flask_add_url_rule_tuple_methods","patch":[]}
{"id":"et1-02","class":"flask_host_and_slash_options","patch":[{"op":"add","path":"/expect/0/paths/0/hosts","value":["api.example.test"]},{"op":"add","path":"/expect/0/paths/0/strict_slashes","value":false},{"op":"remove","path":"/d"},{"op":"add","path":"/expect/0/paths/0/host_kind","value":"literal"}]}
{"id":"et1-03","class":"flask_blueprint_registered_in_factory","patch":[{"op":"add","path":"/expect/0/paths/0/root","value":{"file":"app.py","binding":"app","construction_byte":169,"invocations":[{"entry":"app:create_app()"}]}}]}
{"id":"et1-04","class":"fastapi_router_dependency","patch":[]}
{"id":"et1-05","class":"fastapi_app_router_registration","patch":[{"op":"add","path":"/expect/0/paths","value":[{"status":"full","value":"/health"}]},{"op":"add","path":"/expect/0/receiver","value":{"status":"router","binding":"app","file":"app.py","construction_text":"FastAPI("}},{"op":"remove","path":"/d"}]}
{"id":"et2-03","class":"flask_outer_replacing_decorator","patch":[]}
{"id":"et3-02","class":"flask_config_conditional_registration","patch":[]}
{"id":"et6-01","class":"fastapi_include_before_route_snapshot","patch":[{"op":"remove","path":"/d"}]}
{"id":"et8-01","class":"fastapi_mounted_subapplication","patch":[{"op":"remove","path":"/d"},{"op":"replace","path":"/expect/0/callbacks/0/start_line","value":6},{"op":"add","path":"/expect/0/paths/0/root","value":{"file":"app.py","binding":"app","construction_byte":35,"invocations":[]}},{"op":"add","path":"/entry_queries","value":[{"entry":"app:child","expect_paths":[{"site":{"file":"app.py","line":6,"text":"@child.get(\"/items\")","occurrence":1},"paths":[{"status":"full","value":"/items","root":{"file":"app.py","binding":"child","construction_byte":100,"invocations":[]}}]}]}]}]}
{"id":"es1-13","class":"flask_blueprint_diamond_nested_overrides","patch":[]}
{"id":"es1-14","class":"flask_none_and_empty_mount_prefix","patch":[{"op":"remove","path":"/d"}]}
{"id":"es1-15","class":"flask_package_initializer_router_reexport","patch":[]}
{"id":"es1-16","class":"flask_star_import_overwrites_same_name","patch":[]}
{"id":"es1-17","class":"flask_same_named_routers_in_two_factories","patch":[{"op":"remove","path":"/d"},{"op":"add","path":"/expect/1/paths/0/root","value":{"file":"app.py","binding":"app","construction_byte":369,"invocations":[{"entry":"app:create_app()"}]}},{"op":"add","path":"/expect/0/paths/0/root","value":null}]}
{"id":"es1-18","class":"fastapi_reciprocal_includes_are_snapshots","patch":[{"op":"remove","path":"/d"}]}
{"id":"es1-19","class":"fastapi_nested_scope_captured_router","patch":[{"op":"remove","path":"/d"},{"op":"add","path":"/expect/0/paths/0/root","value":{"file":"app.py","binding":"app","construction_byte":57,"invocations":[{"entry":"app:create_app()"}]}}]}
{"id":"es1-20","class":"flask_generated_loop_registrations","patch":[{"op":"replace","path":"/accepted_loss","value":"runtime_generated_registration"},{"op":"remove","path":"/d"},{"op":"add","path":"/expect_diagnostics","value":[{"reason":"runtime_generated_registration","site":{"file":"app.py","line":9,"text":"app.add_url_rule(page, view_func=status, methods=[\"GET\"])"}}]}]}
{"id":"es1-21","class":"flask_empty_and_slash_blueprint_leaves","patch":[]}
{"id":"es1-22","class":"fastapi_python_package_parent_relative_mount","patch":[]}
{"id":"es1-68","class":"flask_path_escape_concatenation_and_fstring","patch":[{"op":"replace","path":"/expect/0/path/literal","value":"/notes"},{"op":"replace","path":"/expect/0/paths/0/status","value":"full"},{"op":"remove","path":"/expect/0/paths/0/reason"},{"op":"add","path":"/expect/0/paths/0/value","value":"/notes"},{"op":"replace","path":"/expect/1/path/literal","value":"/items/latest"},{"op":"replace","path":"/expect/1/paths/0/status","value":"full"},{"op":"remove","path":"/expect/1/paths/0/reason"},{"op":"add","path":"/expect/1/paths/0/value","value":"/items/latest"},{"op":"remove","path":"/d"}]}
{"id":"es2-07","class":"flask_endpoint_decorator_late_binding","patch":[{"op":"add","path":"/expect/0/callbacks","value":[]},{"op":"add","path":"/expect/0/endpoint","value":"stock-index"},{"op":"remove","path":"/d"}]}
{"id":"es2-08","class":"flask_callable_required_methods","patch":[]}
{"id":"es2-09","class":"flask_literal_punctuation_and_path_tail","patch":[{"op":"add","path":"/expect/0/paths/0/path_tokens","value":[["literal","/literal/:id/{draft}"]]}]}
{"id":"es2-10","class":"fastapi_direct_websocket_and_http_same_path","patch":[{"op":"add","path":"/expect/0/protocol","value":"websocket"},{"op":"add","path":"/expect/0/methods","value":null},{"op":"add","path":"/expect/1/protocol","value":"http"},{"op":"remove","path":"/d"}]}
{"id":"es2-11","class":"fastapi_mixed_literal_and_unknown_methods","patch":[{"op":"remove","path":"/d"}]}
{"id":"es2-12","class":"fastapi_empty_leaf_uses_constructor_prefix","patch":[]}
{"id":"es3-06","class":"flask_replacing_decorator_between_route_decorators","patch":[]}
{"id":"es3-07","class":"flask_saved_decorator_applied_in_later_call","patch":[{"op":"replace","path":"/accepted_loss","value":"unsupported_registration"},{"op":"remove","path":"/d"},{"op":"add","path":"/expect_diagnostics","value":[{"reason":"unsupported_registration","site":{"file":"app.py","line":11,"text":"saved(inventory)"}}]}]}
{"id":"es3-08","class":"fastapi_constructor_supplied_route_table","patch":[{"op":"replace","path":"/accepted_loss","value":"unsupported_registration"},{"op":"remove","path":"/d"},{"op":"add","path":"/expect_diagnostics","value":[{"reason":"unsupported_registration","site":{"file":"app.py","line":10,"text":"APIRoute('/catalog', catalog, methods=['GET'])"}}]}]}
{"id":"es3-09","class":"fastapi_include_other_application_router_snapshot","patch":[{"op":"remove","path":"/d"},{"op":"add","path":"/expect/0/paths/0/root","value":{"file":"app.py","binding":"app","construction_byte":185,"invocations":[]}},{"op":"add","path":"/entry_queries","value":[{"entry":"app:source","expect_paths":[{"site":{"file":"app.py","line":6,"text":"@source.get('/recent')","occurrence":1},"paths":[{"status":"full","value":"/recent","root":{"file":"app.py","binding":"source","construction_byte":38,"invocations":[]}}]}]}]}]}
{"id":"es3-10","class":"fastapi_explicit_empty_method_set","patch":[{"op":"add","path":"/expect/1/methods","value":null},{"op":"remove","path":"/d"}]}
{"id":"es4-04","class":"flask_blueprint_record_once_limits_deferred_rule_exposure","patch":[{"op":"remove","path":"/accepted_loss"},{"op":"remove","path":"/d"}]}
{"id":"es4-05","class":"flask_blueprint_nested_subdomain_override","patch":[{"op":"add","path":"/expect/0/paths/0/hosts","value":["events.team.example.test"]},{"op":"remove","path":"/d"},{"op":"add","path":"/expect/0/paths/0/host_kind","value":"literal"}]}
{"id":"es4-06","class":"flask_put_patch_delete_shortcut_decorators","patch":[]}
{"id":"es4-07","class":"fastapi_starlette_compatibility_registration_apis","patch":[{"op":"remove","path":"/accepted_loss"},{"op":"remove","path":"/d"}]}
{"id":"es5-03","class":"flask_keyword_blueprint_rule_and_mount","patch":[]}
{"id":"es5-04","class":"fastapi_schema_hidden_route_and_asgi_root_path","patch":[]}
{"id":"es5-05","class":"fastapi_typed_tail_and_literal_parameter_syntax","patch":[{"op":"add","path":"/expect/2/paths/0/normalized","value":"/examples/:name/{draft-name}"},{"op":"add","path":"/expect/2/paths/0/path_tokens","value":[["literal","/examples/:name/{draft-name}"]]},{"op":"remove","path":"/d"}]}
{"id":"es7-07","class":"flask_build_only_rule_never_dispatches","patch":[{"op":"add","path":"/expect/0/reason","value":"build_only"},{"op":"remove","path":"/d"},{"op":"remove","path":"/accepted_loss"}]}
{"id":"es7-08","class":"fastapi_identical_declarations_keep_distinct_snapshot_origins","patch":[{"op":"add","path":"/expect/0/paths/0/shadowed","value":false},{"op":"add","path":"/expect/0/paths/1/shadowed","value":false},{"op":"add","path":"/expect/1/paths/0/shadowed","value":true},{"op":"remove","path":"/d"}]}
```

### w1-express (84)

```jsonl
{"id":"ex01","class":"named_callback","patch":[]}
{"id":"ex02","class":"inline_arrow","patch":[]}
{"id":"ex03","class":"wrapped_inline_callback","patch":[{"op":"add","path":"/expect/0/callbacks/0/delegate","value":null},{"op":"remove","path":"/d"}]}
{"id":"ex04","class":"wrapped_named_callback","patch":[{"op":"add","path":"/expect/0/callbacks/0/delegate","value":null},{"op":"remove","path":"/d"}]}
{"id":"ex05","class":"unproven_wrapper_delegate","patch":[{"op":"add","path":"/expect/0/callbacks/0/delegate","value":null}]}
{"id":"ex06","class":"commonjs_member_callback","patch":[{"op":"replace","path":"/expect/0/callbacks/0/status","value":"resolved"},{"op":"remove","path":"/expect/0/callbacks/0/text"},{"op":"remove","path":"/expect/0/callbacks/0/reason"},{"op":"add","path":"/expect/0/callbacks/0/end_line","value":1},{"op":"add","path":"/expect/0/callbacks/0/file","value":"site.js"},{"op":"add","path":"/expect/0/callbacks/0/name","value":"index"},{"op":"add","path":"/expect/0/callbacks/0/start_line","value":1},{"op":"remove","path":"/d"}]}
{"id":"ex07","class":"esm_imported_callback","patch":[]}
{"id":"ex08","class":"wrapper_bound_to_local","patch":[{"op":"add","path":"/expect/0/callbacks/0/delegate","value":null},{"op":"remove","path":"/d"}]}
{"id":"ex09","class":"ordered_callbacks","patch":[]}
{"id":"ex10","class":"route_builder_verbs","patch":[]}
{"id":"ex11","class":"all_methods","patch":[{"op":"remove","path":"/d"}]}
{"id":"ex12","class":"template_literal_paths","patch":[]}
{"id":"ex13","class":"regexp_path","patch":[{"op":"replace","path":"/expect/0/paths/0/reason","value":"regex_path"},{"op":"add","path":"/expect/0/paths/0/normalized","value":null},{"op":"remove","path":"/d"}]}
{"id":"ex14","class":"router_parameter_unproven","patch":[{"op":"replace","path":"/expect/0/paths","value":[{"status":"full","value":"/x"}]},{"op":"replace","path":"/expect/0/receiver/status","value":"router"},{"op":"remove","path":"/expect/0/receiver/reason"},{"op":"add","path":"/expect/0/receiver/file","value":"app.js"},{"op":"add","path":"/expect/0/receiver/binding","value":"app"},{"op":"add","path":"/expect/0/receiver/construction_text","value":"express()"},{"op":"add","path":"/expect/0/callbacks","value":[{"end_line":4,"file":"app.js","name":"named","position":0,"start_line":4,"status":"resolved"}]},{"op":"remove","path":"/d"}]}
{"id":"ex15","class":"optional_and_wildcard_paths","patch":[{"op":"add","path":"/expect/0/paths/0/path_tokens","value":null},{"op":"add","path":"/expect/1/paths/0/path_tokens","value":null},{"op":"remove","path":"/d"}]}
{"id":"ex16","class":"middleware_without_route","patch":[]}
{"id":"ex17","class":"explicit_head_and_options","patch":[{"op":"remove","path":"/d"}]}
{"id":"ex18","class":"typescript_named_callback","patch":[]}
{"id":"ex19","class":"local_router_mount","patch":[]}
{"id":"ex20","class":"direct_require_mount","patch":[]}
{"id":"ex21","class":"commonjs_bound_router_mount","patch":[]}
{"id":"ex22","class":"esm_default_router_mount","patch":[]}
{"id":"ex23","class":"variable_mount_prefix","patch":[]}
{"id":"ex24","class":"same_router_mounted_twice","patch":[]}
{"id":"ex25","class":"nested_router_mount","patch":[]}
{"id":"ex26","class":"mounted_slash_path","patch":[]}
{"id":"ex27","class":"imported_callback_over_nested_namesake","patch":[]}
{"id":"ex28","class":"settings_read_and_builder_legs","patch":[{"op":"add","path":"/expect_bindings","value":[{"method":"HEAD","value":"/c","endpoint":"c"}]}]}
{"id":"ex29","class":"router_rebinding","patch":[]}
{"id":"ex30","class":"array_mount_prefixes","patch":[{"op":"remove","path":"/d"}]}
{"id":"cx01","class":"express_index_barrel_mount","patch":[]}
{"id":"cx03","class":"express_router_use_array","patch":[{"op":"remove","path":"/d"}]}
{"id":"cx04","class":"express_path_scoped_middleware","patch":[]}
{"id":"cx11","class":"express_builder_all","patch":[{"op":"remove","path":"/d"}]}
{"id":"cx12","class":"express_verb_bind","patch":[]}
{"id":"cx13","class":"express_verb_delete","patch":[]}
{"id":"cx14","class":"express_verb_patch","patch":[]}
{"id":"cx15","class":"express_verb_post","patch":[]}
{"id":"cx16","class":"express_verb_put","patch":[]}
{"id":"cx17","class":"express_verb_search","patch":[]}
{"id":"cx18","class":"express_callback_array","patch":[]}
{"id":"et1-06","class":"express_param_is_not_route","patch":[]}
{"id":"et1-07","class":"express_static_middleware","patch":[]}
{"id":"et1-08","class":"express_builder_all_then_get","patch":[{"op":"add","path":"/expect/0/paths/0/binding_blocked","value":{"known":["GET","HEAD"]}},{"op":"add","path":"/expect/1/paths/0/binding_blocked","value":{"known":["GET","HEAD"]}}]}
{"id":"et1-09","class":"express_route_path_array","patch":[{"op":"replace","path":"/expect/0/paths","value":[{"status":"full","value":"/a"},{"status":"full","value":"/b"}]},{"op":"remove","path":"/d"}]}
{"id":"et1-10","class":"express_dynamic_bracket_verb_loop","patch":[{"op":"remove","path":"/d"},{"op":"add","path":"/expect_diagnostics","value":[{"reason":"runtime_generated_registration","site":{"file":"app.js","line":5,"text":"app[method]('/items', show)"}}]}]}
{"id":"et2-01","class":"express_saved_route_builder","patch":[{"op":"remove","path":"/d"}]}
{"id":"et2-02","class":"express_multiple_routers_single_use","patch":[{"op":"remove","path":"/d"}]}
{"id":"et2-04","class":"express_two_calls_same_line","patch":[]}
{"id":"et2-05","class":"express_pathless_router_mount","patch":[]}
{"id":"et6-02","class":"express_mount_before_route_live_router","patch":[]}
{"id":"et8-02","class":"express_mounted_exported_subapplication","patch":[{"op":"remove","path":"/d"},{"op":"add","path":"/expect/0/paths/0/root","value":{"file":"app.js","binding":"app","construction_byte":48,"invocations":[]}},{"op":"add","path":"/entry_queries","value":[{"entry":"app.js#child","expect_paths":[{"site":{"file":"app.js","line":6,"text":"child.get('/items', items)","occurrence":1},"paths":[{"status":"full","value":"/items","root":{"file":"app.js","binding":"child","construction_byte":73,"invocations":[]}}]}]}]}]}
{"id":"es1-01","class":"express_cross_file_mount_diamond","patch":[]}
{"id":"es1-02","class":"express_reachable_mount_cycle","patch":[{"op":"remove","path":"/d"}]}
{"id":"es1-03","class":"express_parallel_static_dynamic_mounts","patch":[]}
{"id":"es1-04","class":"express_esm_named_export_selects_router","patch":[]}
{"id":"es1-05","class":"express_esm_explicit_router_reexport","patch":[]}
{"id":"es1-06","class":"express_esm_star_namespace_collision","patch":[{"op":"remove","path":"/accepted_loss"},{"op":"remove","path":"/d"}]}
{"id":"es1-07","class":"express_commonjs_member_and_destructured_router_import","patch":[{"op":"remove","path":"/d"}]}
{"id":"es1-08","class":"express_esm_default_import_commonjs_router","patch":[{"op":"remove","path":"/d"}]}
{"id":"es1-09","class":"express_static_bracket_verb","patch":[{"op":"remove","path":"/d"}]}
{"id":"es1-10","class":"express_repeated_same_builder_verb","patch":[{"op":"remove","path":"/d"},{"op":"add","path":"/expect/0/paths/0/shadowed","value":null},{"op":"add","path":"/expect/1/paths/0/shadowed","value":null},{"op":"add","path":"/expect/0/paths/0/binding_blocked","value":{"known":["GET","HEAD"]}},{"op":"add","path":"/expect/1/paths/0/binding_blocked","value":{"known":["GET","HEAD"]}}]}
{"id":"es1-11","class":"express_route_error_middleware_position","patch":[]}
{"id":"es1-12","class":"express_nonstandard_hyphenated_verb","patch":[{"op":"remove","path":"/d"}]}
{"id":"es1-69","class":"express_nested_mount_aliases_with_middleware","patch":[{"op":"remove","path":"/d"}]}
{"id":"es1-70","class":"express_spread_callback_before_resolved_terminal","patch":[{"op":"add","path":"/expect/0/callbacks","value":[{"end_line":3,"file":"app.js","name":"auth","position":0,"start_line":3,"status":"resolved"},{"end_line":4,"file":"app.js","name":"audit","position":1,"start_line":4,"status":"resolved"},{"end_line":5,"file":"app.js","name":"finish","position":2,"start_line":5,"status":"resolved"}]},{"op":"remove","path":"/accepted_loss"},{"op":"remove","path":"/d"}]}
{"id":"es2-01","class":"express_remaining_pinned_verbs","patch":[]}
{"id":"es2-02","class":"express_deprecated_delete_alias","patch":[{"op":"remove","path":"/d"}]}
{"id":"es2-03","class":"express_default_callable_exports","patch":[{"op":"remove","path":"/d"}]}
{"id":"es2-04","class":"express_pathless_nested_use_array","patch":[{"op":"remove","path":"/d"}]}
{"id":"es2-05","class":"express_regex_mount_prefix","patch":[{"op":"remove","path":"/d"},{"op":"replace","path":"/accepted_loss","value":"variable_prefix"}]}
{"id":"es2-06","class":"express_identical_anonymous_source_distinct_sites","patch":[]}
{"id":"es2-34","class":"composition_exposure_cap_retains_first_paths","patch":[{"op":"replace","path":"/group","value":"w1-express"},{"op":"remove","path":"/d"},{"op":"replace","path":"/accepted_loss","value":"expansion_truncated"}]}
{"id":"es3-01","class":"express_builder_without_verb","patch":[]}
{"id":"es3-02","class":"express_escaped_and_concatenated_strings","patch":[{"op":"add","path":"/expect/0/path/literal","value":"/notes"},{"op":"replace","path":"/expect/0/paths/0/status","value":"full"},{"op":"remove","path":"/expect/0/paths/0/reason"},{"op":"add","path":"/expect/0/paths/0/value","value":"/notes"},{"op":"add","path":"/expect/1/path/literal","value":"/reports/current"},{"op":"replace","path":"/expect/1/paths/0/status","value":"full"},{"op":"remove","path":"/expect/1/paths/0/reason"},{"op":"add","path":"/expect/1/paths/0/value","value":"/reports/current"},{"op":"remove","path":"/d"}]}
{"id":"es3-03","class":"express_discarding_and_nested_wrappers","patch":[{"op":"add","path":"/expect/0/callbacks/0/delegate","value":null},{"op":"add","path":"/expect/1/callbacks/0/delegate","value":null},{"op":"remove","path":"/d"}]}
{"id":"es3-04","class":"express_bound_class_method_callback","patch":[{"op":"add","path":"/expect/0/callbacks/0/delegate","value":null},{"op":"replace","path":"/accepted_loss","value":"wrapper_call"}]}
{"id":"es3-05","class":"express_chained_router_verb_receiver","patch":[{"op":"replace","path":"/expect/1/paths","value":[{"status":"full","value":"/staff/invite"}]},{"op":"replace","path":"/expect/1/receiver/status","value":"router"},{"op":"remove","path":"/expect/1/receiver/reason"},{"op":"add","path":"/expect/1/receiver/file","value":"app.cjs"},{"op":"add","path":"/expect/1/receiver/binding","value":"staff"},{"op":"add","path":"/expect/1/receiver/construction_text","value":"express.Router()"},{"op":"add","path":"/expect/1/callbacks","value":[{"end_line":6,"file":"app.cjs","name":"invite","position":0,"start_line":6,"status":"resolved"}]},{"op":"remove","path":"/accepted_loss"},{"op":"remove","path":"/d"}]}
{"id":"es4-01","class":"express_head_fallback_across_separate_and_shared_layers","patch":[{"op":"add","path":"/expect/0/paths/0/binding_blocked","value":{"known":["HEAD"]}},{"op":"add","path":"/expect/1/paths/0/binding_blocked","value":{"known":["HEAD"]}},{"op":"add","path":"/expect_bindings","value":[{"method":"GET","value":"/ledger","endpoint":"readLedger"},{"method":"HEAD","value":"/ledger","endpoint":null},{"method":"GET","value":"/manifest","endpoint":"readManifest"},{"method":"HEAD","value":"/manifest","endpoint":"checkManifest"}]}]}
{"id":"es4-02","class":"express_replaced_module_exports_detaches_exports_alias","patch":[{"op":"remove","path":"/d"}]}
{"id":"es4-03","class":"express_optional_chain_grounded_and_null_receivers","patch":[{"op":"remove","path":"/d"}]}
{"id":"es5-01","class":"express_bound_verb_registration_alias","patch":[{"op":"remove","path":"/d"}]}
{"id":"es5-02","class":"express_nested_literal_callback_array_order","patch":[]}
{"id":"es5-16","class":"express_router_as_verb_callback_preserves_request_path","patch":[{"op":"add","path":"/expect/1/callbacks","value":[{"position":0,"status":"unresolved","text":"catalog","reason":"handler_object"}]},{"op":"replace","path":"/accepted_loss","value":"handler_object"},{"op":"remove","path":"/d"}]}
```

### w2-gin (25)

```jsonl
{"id":"gn01","class":"named_callback","patch":[]}
{"id":"gn02","class":"inline_handler","patch":[{"op":"add","path":"/expect/0/callbacks","value":[{"position":0,"status":"unresolved","text":"func(c *gin.Context) {}","reason":"anonymous_unindexed"}]},{"op":"remove","path":"/d"},{"op":"add","path":"/accepted_loss","value":"anonymous_unindexed"}]}
{"id":"gn04","class":"group_one_helper","patch":[{"op":"add","path":"/expect/0/callbacks","value":[{"position":0,"status":"resolved","name":"list","file":"app.go","start_line":5}]},{"op":"remove","path":"/d"}]}
{"id":"gn05","class":"group_two_helpers","patch":[{"op":"add","path":"/expect/0/callbacks","value":[{"position":0,"status":"resolved","name":"list","file":"app.go","start_line":5}]},{"op":"remove","path":"/d"},{"op":"add","path":"/accepted_loss","value":"no_known_mount"}]}
{"id":"gn06","class":"configuration_prefix","patch":[]}
{"id":"gn07","class":"handle_and_any","patch":[{"op":"replace","path":"/expect/1/methods","value":{"known":["CONNECT","DELETE","GET","HEAD","OPTIONS","PATCH","POST","PUT","TRACE"]}},{"op":"remove","path":"/d"}]}
{"id":"gn08","class":"middleware_call_and_handler","patch":[]}
{"id":"gn09","class":"gin_import_without_context_type","patch":[{"op":"add","path":"/expect/0/paths","value":[{"status":"full","value":"/api/health"}]}]}
{"id":"gn10","class":"empty_group_prefix","patch":[]}
{"id":"cx29","class":"gin_delete","patch":[]}
{"id":"cx30","class":"gin_head","patch":[]}
{"id":"cx31","class":"gin_options","patch":[]}
{"id":"cx32","class":"gin_patch","patch":[]}
{"id":"cx33","class":"gin_put","patch":[]}
{"id":"et1-11","class":"gin_no_route_handler","patch":[]}
{"id":"et1-12","class":"gin_static_helpers","patch":[{"op":"remove","path":"/d"},{"op":"replace","path":"/accepted_loss","value":"unsupported_registration"},{"op":"add","path":"/expect_diagnostics","value":[{"reason":"unsupported_registration","site":{"file":"app.go","line":11,"text":"r.Static(\"/assets\", \"public\")"}},{"reason":"unsupported_registration","site":{"file":"app.go","line":12,"text":"r.StaticFS(\"/files\", http.Dir(\"public\"))"}}]}]}
{"id":"et4-03","class":"go_external_test_package_same_named_handler","patch":[{"op":"remove","path":"/d"}]}
{"id":"es1-67","class":"gin_match_method_slice","patch":[{"op":"remove","path":"/d"}]}
{"id":"es2-13","class":"gin_handle_dynamic_method_static_leaf","patch":[]}
{"id":"es2-14","class":"gin_use_and_group_middleware_outside_route_arguments","patch":[]}
{"id":"es2-15","class":"gin_relative_leaf_join_cleans_and_retains_final_slash","patch":[]}
{"id":"es3-11","class":"gin_inline_chained_group_receivers","patch":[{"op":"replace","path":"/expect/0/paths","value":[{"status":"full","value":"/ops/v2/status"}]},{"op":"replace","path":"/expect/0/receiver/status","value":"router"},{"op":"remove","path":"/expect/0/receiver/reason"},{"op":"add","path":"/expect/0/receiver/file","value":"app.go"},{"op":"add","path":"/expect/0/receiver/binding","value":"r.Group(\"/ops\").Group(\"/v2\")"},{"op":"add","path":"/expect/0/receiver/construction_text","value":"r.Group(\"/ops\").Group(\"/v2\")"},{"op":"add","path":"/expect/0/callbacks","value":[{"position":0,"status":"resolved","file":"app.go","name":"status","start_line":5}]},{"op":"replace","path":"/expect_mounts","value":[{"parent":{"status":"router","file":"app.go","binding":"r","construction_text":"gin.New()"},"child":{"status":"router","file":"app.go","binding":"r.Group(\"/ops\")","construction_text":"r.Group(\"/ops\")"},"site":{"file":"app.go","line":9,"text":"r.Group(\"/ops\")","occurrence":1},"prefix":{"status":"literal","value":"/ops"}},{"parent":{"status":"router","file":"app.go","binding":"r.Group(\"/ops\")","construction_text":"r.Group(\"/ops\")"},"child":{"status":"router","file":"app.go","binding":"r.Group(\"/ops\").Group(\"/v2\")","construction_text":"r.Group(\"/ops\").Group(\"/v2\")"},"site":{"file":"app.go","line":9,"text":"r.Group(\"/ops\").Group(\"/v2\")","occurrence":1},"prefix":{"status":"literal","value":"/v2"}}]},{"op":"remove","path":"/accepted_loss"},{"op":"remove","path":"/d"}]}
{"id":"es3-12","class":"gin_no_method_fallback_with_explicit_route","patch":[]}
{"id":"es4-08","class":"gin_explicit_embedded_root_router_group","patch":[{"op":"remove","path":"/d"}]}
{"id":"es5-06","class":"gin_variadic_callback_slice","patch":[{"op":"add","path":"/expect/0/callbacks","value":[{"position":0,"status":"resolved","file":"app.go","name":"check","start_line":5},{"position":1,"status":"resolved","file":"app.go","name":"show","start_line":6}]},{"op":"remove","path":"/accepted_loss"},{"op":"remove","path":"/d"}]}
```

### w2-nethttp (29)

```jsonl
{"id":"nh02","class":"default_mux_and_function_literal","patch":[{"op":"add","path":"/expect/0/callbacks","value":[{"position":0,"status":"unresolved","text":"func(w http.ResponseWriter, r *http.Request) {}","reason":"anonymous_unindexed"}]},{"op":"remove","path":"/d"},{"op":"add","path":"/accepted_loss","value":"anonymous_unindexed"}]}
{"id":"nh03","class":"handler_object","patch":[{"op":"add","path":"/expect/0/callbacks","value":[{"position":0,"status":"unresolved","text":"v","reason":"handler_object"}]},{"op":"remove","path":"/d"},{"op":"add","path":"/accepted_loss","value":"handler_object"}]}
{"id":"nh04","class":"declared_get_and_effective_head","patch":[{"op":"remove","path":"/d"}]}
{"id":"nh05","class":"host_pattern","patch":[{"op":"add","path":"/expect/0/paths/0/hosts","value":["example.com"]},{"op":"add","path":"/expect/0/methods","value":"any"},{"op":"remove","path":"/d"},{"op":"add","path":"/expect/0/paths/0/host_kind","value":"literal"}]}
{"id":"nh06","class":"tail_wildcard_and_end_anchor","patch":[{"op":"add","path":"/expect/0/paths/0/path_tokens","value":null},{"op":"remove","path":"/d"}]}
{"id":"nh07","class":"dynamic_pattern","patch":[{"op":"add","path":"/expect/0/methods","value":{"status":"unknown","text":"versionPath"}}]}
{"id":"nh08","class":"strip_prefix_mount","patch":[{"op":"remove","path":"/d"}]}
{"id":"ch01","class":"chi_get","patch":[]}
{"id":"ch02","class":"chi_route_closure","patch":[]}
{"id":"ch03","class":"chi_factory_mount","patch":[{"op":"remove","path":"/d"}]}
{"id":"ch04","class":"chi_inline_middleware_router","patch":[]}
{"id":"cx09","class":"nethttp_handle_handlerfunc_conversion","patch":[{"op":"add","path":"/expect/0/callbacks/0/delegate","value":{"file":"app.go","name":"report","start_line":5,"end_line":7}},{"op":"remove","path":"/d"}]}
{"id":"cx10","class":"chi_nested_route_closures","patch":[]}
{"id":"cx34","class":"nethttp_handle_dynamic_pattern","patch":[{"op":"add","path":"/expect/0/methods","value":{"status":"unknown","text":"pattern"}}]}
{"id":"es1-66","class":"nethttp_mux_mount_without_strip_prefix","patch":[{"op":"replace","path":"/expect","value":[{"framework":"net/http","site":{"file":"app.go","line":11,"text":"child.HandleFunc(\"GET /api/items\", items)","occurrence":1},"methods":{"known":["GET"]},"path":{"raw":"\"GET /api/items\"","literal":"/api/items"},"paths":[{"status":"full","value":"/api/items"}],"callbacks":[{"position":0,"status":"resolved","file":"app.go","name":"items","start_line":5}]}]},{"op":"replace","path":"/expect_mounts","value":[{"site":{"file":"app.go","line":13,"text":"app.Handle(\"/api/\", child)","occurrence":1},"parent":{"status":"router","binding":"app"},"child":{"status":"router","binding":"child"},"prefix":{"status":"absent"},"guard_pattern":"/api/"}]},{"op":"remove","path":"/d"}]}
{"id":"es2-20","class":"nethttp_explicit_default_mux_and_package_helper","patch":[{"op":"remove","path":"/d"}]}
{"id":"es2-21","class":"nethttp_method_host_and_encoded_literal_braces","patch":[{"op":"add","path":"/expect/0/paths/0/hosts","value":["warehouse.example.test"]},{"op":"remove","path":"/d"},{"op":"add","path":"/expect/0/paths/0/host_kind","value":"literal"}]}
{"id":"es2-22","class":"chi_methodfunc_and_handlefunc_methods","patch":[{"op":"remove","path":"/d"}]}
{"id":"es2-23","class":"chi_inline_group_without_path_prefix","patch":[{"op":"remove","path":"/d"}]}
{"id":"es3-17","class":"nethttp_method_pattern_case_sensitive","patch":[{"op":"add","path":"/expect/1/methods","value":{"known":["get"]}},{"op":"remove","path":"/accepted_loss"},{"op":"remove","path":"/d"}]}
{"id":"es3-18","class":"nethttp_subtree_and_exact_slash_anchor","patch":[{"op":"remove","path":"/d"}]}
{"id":"es3-19","class":"chi_mount_opaque_http_handler","patch":[{"op":"replace","path":"/accepted_loss","value":"mount_child_unproven"},{"op":"remove","path":"/d"}]}
{"id":"es4-11","class":"nethttp_package_helper_tracks_replaced_default_mux","patch":[{"op":"remove","path":"/d"},{"op":"add","path":"/expect/0/paths/0/root","value":{"file":"app.go","binding":"before","construction_byte":285,"invocations":[]}},{"op":"add","path":"/expect/1/paths/0/root","value":{"file":"current.go","binding":"after","construction_byte":175,"invocations":[{"file":"app.go","start_byte":386,"end_byte":398}]}}]}
{"id":"es4-12","class":"nethttp_connect_preserves_unclean_path","patch":[]}
{"id":"es4-13","class":"chi_regex_mount_parameter_and_leaf_wildcard","patch":[{"op":"remove","path":"/d"}]}
{"id":"es5-09","class":"nethttp_strip_prefix_shorter_than_mux_pattern","patch":[{"op":"add","path":"/expect_mounts/0/guard_pattern","value":"/edge/api/"},{"op":"remove","path":"/d"}]}
{"id":"es5-10","class":"chi_registered_extension_method","patch":[]}
{"id":"es7-01","class":"nethttp_method_guard_filters_child_mount_exposures","patch":[{"op":"add","path":"/expect/0/paths/0/method_guard","value":{"known":["GET","HEAD"]}},{"op":"add","path":"/expect/0/paths/1/method_guard","value":"any"},{"op":"add","path":"/expect/1/paths/0/method_guard","value":{"known":["GET","HEAD"]}},{"op":"add","path":"/expect/1/paths/1/method_guard","value":"any"},{"op":"add","path":"/expect/2/paths/0/method_guard","value":"any"},{"op":"add","path":"/expect_mounts/0/method_guard","value":{"known":["GET","HEAD"]}},{"op":"add","path":"/expect_mounts/1/method_guard","value":"any"},{"op":"remove","path":"/d"},{"op":"remove","path":"/accepted_loss"}]}
{"id":"es7-02","class":"nethttp_encoded_slash_preserves_segment_boundary","patch":[]}
```

### w2-method-value (61)

```jsonl
{"id":"gn03","class":"receiver_field_method","patch":[]}
{"id":"gm01","class":"qualified_receiver_field","patch":[]}
{"id":"gm02","class":"same_named_types_across_packages","patch":[]}
{"id":"gm03","class":"local_bound_to_receiver_field","patch":[]}
{"id":"gm04","class":"composite_local_shadows_type_name","patch":[]}
{"id":"gm05","class":"typed_parameter_method","patch":[]}
{"id":"gm06","class":"package_var_method","patch":[]}
{"id":"gm07","class":"pointer_and_value_methods","patch":[]}
{"id":"gm08","class":"promoted_value_embed_method","patch":[{"op":"remove","path":"/d"}]}
{"id":"gm09","class":"interface_field_method","patch":[]}
{"id":"gm10","class":"interface_parameter_method","patch":[]}
{"id":"gm11","class":"local_shadows_enclosing_receiver","patch":[]}
{"id":"gm12","class":"closure_parameter_shadows_receiver","patch":[]}
{"id":"gm13","class":"unsupported_method_operands","patch":[]}
{"id":"gm14","class":"func_typed_field_value","patch":[]}
{"id":"gm15","class":"package_function_and_method_expression","patch":[]}
{"id":"gm16","class":"external_type_method","patch":[]}
{"id":"gm17","class":"embedded_pointer_field_hop","patch":[]}
{"id":"gm18","class":"method_value_and_call_target_agree","patch":[]}
{"id":"gm19","class":"wrapped_method_value","patch":[{"op":"add","path":"/expect/0/callbacks/0/delegate","value":null},{"op":"remove","path":"/d"}]}
{"id":"nh01","class":"method_in_pattern_and_method_value","patch":[{"op":"replace","path":"/group","value":"w2-method-value"}]}
{"id":"cx35","class":"go_two_hop_enclosing_receiver","patch":[]}
{"id":"cx36","class":"go_constructor_initialized_local","patch":[]}
{"id":"cx37","class":"go_call_initialized_local","patch":[]}
{"id":"cx38","class":"go_method_value_on_enclosing_receiver","patch":[]}
{"id":"cx39","class":"go_explicitly_typed_local","patch":[]}
{"id":"cx40","class":"go_two_hop_parameter_field","patch":[]}
{"id":"cx41","class":"go_interface_typed_local","patch":[]}
{"id":"cx42","class":"go_bare_function_variable","patch":[{"op":"remove","path":"/d"}]}
{"id":"cx43","class":"go_forwarding_function_literal","patch":[{"op":"add","path":"/expect/0/callbacks","value":[{"position":0,"status":"unresolved","text":"func(c *gin.Context) { list(c) }","reason":"anonymous_unindexed","delegate":null}]},{"op":"remove","path":"/d"},{"op":"add","path":"/accepted_loss","value":"anonymous_unindexed"}]}
{"id":"et3-03","class":"go_instantiated_generic_receiver_method","patch":[{"op":"remove","path":"/d"},{"op":"replace","path":"/expect/0/callbacks/0","value":{"position":0,"status":"unresolved","text":"c.List","reason":"method_value_receiver_unproven","delegate":null}},{"op":"add","path":"/expect/0/paths","value":[{"status":"full","value":"/items"}]},{"op":"add","path":"/expect/0/methods","value":{"known":["GET"]}},{"op":"add","path":"/accepted_loss","value":"method_value_receiver_unproven"}]}
{"id":"et4-01","class":"go_generic_method_in_separate_file","patch":[{"op":"remove","path":"/d"},{"op":"replace","path":"/expect/0/callbacks/0","value":{"position":0,"status":"unresolved","text":"c.List","reason":"method_value_receiver_unproven","delegate":null}},{"op":"add","path":"/expect/0/paths","value":[{"status":"full","value":"/items"}]},{"op":"add","path":"/expect/0/methods","value":{"known":["GET"]}},{"op":"add","path":"/accepted_loss","value":"method_value_receiver_unproven"}]}
{"id":"et4-02","class":"go_method_value_mutually_exclusive_build_tags","patch":[{"op":"remove","path":"/d"}]}
{"id":"es1-36","class":"go_concrete_single_result_type_assertion","patch":[]}
{"id":"es1-37","class":"go_concrete_comma_ok_type_assertion","patch":[{"op":"remove","path":"/d"}]}
{"id":"es1-38","class":"go_cross_package_alias_of_interface_receiver","patch":[]}
{"id":"es1-39","class":"go_promoted_method_via_embedded_pointer","patch":[{"op":"remove","path":"/d"}]}
{"id":"es1-40","class":"go_embedded_qualified_hop_vs_promoted_method","patch":[{"op":"remove","path":"/d"},{"op":"add","path":"/expect/0/methods","value":{"known":["GET"]}},{"op":"add","path":"/expect/0/paths","value":[{"status":"full","value":"/explicit"}]},{"op":"add","path":"/expect/0/callbacks/0/delegate","value":null},{"op":"add","path":"/expect/1/methods","value":{"known":["GET"]}},{"op":"add","path":"/expect/1/paths","value":[{"status":"full","value":"/promoted"}]},{"op":"add","path":"/expect/1/callbacks/0/delegate","value":null}]}
{"id":"es1-41","class":"go_promoted_selector_nearest_embedding_depth","patch":[{"op":"remove","path":"/d"}]}
{"id":"es1-42","class":"go_parenthesized_and_unary_method_operands","patch":[{"op":"remove","path":"/d"}]}
{"id":"es1-43","class":"go_multi_name_short_declaration_field_initializer","patch":[]}
{"id":"es1-44","class":"go_chained_local_copies_of_receiver_field","patch":[]}
{"id":"es1-45","class":"go_receiver_assignment_preserves_declaration","patch":[]}
{"id":"es1-46","class":"go_type_parameter_method_value","patch":[]}
{"id":"es1-47","class":"go_import_name_shadowed_by_typed_parameter","patch":[{"op":"remove","path":"/d"}]}
{"id":"es1-48","class":"go_same_package_test_only_method_value","patch":[{"op":"remove","path":"/d"}]}
{"id":"es1-65","class":"go_imported_call_result_initializes_local","patch":[]}
{"id":"es2-16","class":"go_type_switch_case_binds_concrete_method_operand","patch":[{"op":"remove","path":"/d"},{"op":"replace","path":"/accepted_loss","value":"method_value_receiver_unproven"}]}
{"id":"es2-17","class":"go_promoted_interface_vs_direct_method","patch":[]}
{"id":"es2-18","class":"go_range_bound_method_operand","patch":[{"op":"remove","path":"/d"},{"op":"replace","path":"/accepted_loss","value":"method_value_receiver_unproven"}]}
{"id":"es2-19","class":"go_type_inferred_var_composite_method_operand","patch":[{"op":"remove","path":"/d"}]}
{"id":"es3-13","class":"go_multi_result_call_initializes_method_receiver","patch":[{"op":"remove","path":"/d"}]}
{"id":"es3-14","class":"go_conversion_changes_method_value_owner","patch":[{"op":"replace","path":"/accepted_loss","value":"method_value_receiver_unproven"},{"op":"remove","path":"/d"}]}
{"id":"es3-15","class":"go_method_value_field_of_anonymous_struct","patch":[{"op":"replace","path":"/accepted_loss","value":"method_value_receiver_unproven"},{"op":"remove","path":"/d"}]}
{"id":"es3-16","class":"go_multi_type_switch_case_keeps_interface","patch":[{"op":"remove","path":"/d"}]}
{"id":"es4-09","class":"go_explicit_interface_type_over_concrete_initializer","patch":[]}
{"id":"es4-10","class":"go_promoted_method_through_instantiated_generic_embedding","patch":[{"op":"remove","path":"/d"},{"op":"replace","path":"/expect/0/callbacks/0","value":{"position":0,"status":"unresolved","text":"view.Render","reason":"method_value_receiver_unproven","delegate":null}},{"op":"replace","path":"/expect/1/callbacks/0","value":{"position":0,"status":"unresolved","text":"view.renderer.Render","reason":"method_value_receiver_unproven","delegate":null}},{"op":"add","path":"/accepted_loss","value":"method_value_receiver_unproven"}]}
{"id":"es5-07","class":"go_short_redeclaration_preserves_method_value_owner","patch":[{"op":"remove","path":"/d"}]}
{"id":"es5-08","class":"go_concrete_alias_vs_distinct_defined_receiver","patch":[{"op":"remove","path":"/d"}]}
{"id":"es7-03","class":"go_if_initializer_shadow_ends_before_method_value","patch":[{"op":"remove","path":"/d"}]}
{"id":"es7-04","class":"go_method_values_on_packages_with_identical_basenames","patch":[]}
```

### w3-django (32)

```jsonl
{"id":"cx51","class":"django_path_named_view","patch":[{"op":"remove","path":"/d"}]}
{"id":"cx52","class":"django_re_path_named_view","patch":[{"op":"add","path":"/expect/0/paths","value":[{"status":"dynamic","reason":"regex_path","normalized":null}]},{"op":"remove","path":"/d"}]}
{"id":"cx53","class":"django_path_include","patch":[{"op":"remove","path":"/d"}]}
{"id":"cx54","class":"django_re_path_include","patch":[{"op":"remove","path":"/d"},{"op":"replace","path":"/expect_mounts/0/prefix","value":{"status":"literal","value":"ops/"}},{"op":"add","path":"/expect_mounts/0/guard_pattern","value":"^ops/"},{"op":"add","path":"/expect_mounts/0/end_guard","value":false}]}
{"id":"cx55","class":"django_variable_mount_prefix","patch":[{"op":"remove","path":"/d"}]}
{"id":"cx56","class":"django_imported_view","patch":[{"op":"remove","path":"/d"}]}
{"id":"cx57","class":"django_inline_view","patch":[{"op":"add","path":"/expect/0/callbacks","value":[{"position":0,"status":"unresolved","text":"lambda request: HttpResponse(\"ok\")","reason":"anonymous_unindexed"}]},{"op":"remove","path":"/d"},{"op":"add","path":"/accepted_loss","value":"anonymous_unindexed"}]}
{"id":"cx58","class":"django_member_view","patch":[{"op":"remove","path":"/d"}]}
{"id":"cx59","class":"django_lookup_view","patch":[{"op":"add","path":"/expect/0/callbacks","value":[{"position":0,"status":"unresolved","text":"views[\"report\"]","reason":"callback_unproven"}]},{"op":"remove","path":"/d"},{"op":"add","path":"/accepted_loss","value":"callback_unproven"}]}
{"id":"cx60","class":"django_wrapped_view","patch":[{"op":"add","path":"/expect/0/callbacks/0/delegate","value":null},{"op":"remove","path":"/d"}]}
{"id":"et1-13","class":"django_namespaced_include_augmented_urlpatterns","patch":[{"op":"remove","path":"/d"}]}
{"id":"es1-49","class":"django_inline_patterns_inside_include","patch":[{"op":"remove","path":"/d"}]}
{"id":"es1-50","class":"django_cross_file_module_and_pattern_list_includes","patch":[{"op":"remove","path":"/d"}]}
{"id":"es1-51","class":"django_class_based_view_dispatch","patch":[{"op":"remove","path":"/d"}]}
{"id":"es1-52","class":"django_unattached_patterns_from_real_constructor","patch":[{"op":"remove","path":"/d"}]}
{"id":"es1-53","class":"django_tail_and_custom_path_converters","patch":[{"op":"add","path":"/expect/0/paths/0/normalized","value":null},{"op":"add","path":"/expect/1/paths/0/normalized","value":null},{"op":"remove","path":"/d"}]}
{"id":"es1-54","class":"django_localized_urlpatterns_prefix","patch":[{"op":"replace","path":"/expect/0/paths","value":[{"status":"full","value":"/fr/help/"},{"status":"full","value":"/help/"}]},{"op":"remove","path":"/accepted_loss"},{"op":"remove","path":"/d"}]}
{"id":"es1-55","class":"django_include_exported_resolver_tuple_directly","patch":[{"op":"remove","path":"/d"}]}
{"id":"es1-56","class":"django_nested_includes_share_one_leaf","patch":[{"op":"remove","path":"/d"}]}
{"id":"es2-27","class":"django_aliased_path_and_local_constructor_lookalike","patch":[{"op":"remove","path":"/d"}]}
{"id":"es2-28","class":"django_locale_translated_leaf_without_locale_mount","patch":[{"op":"remove","path":"/d"},{"op":"replace","path":"/accepted_loss","value":"non_literal_path"}]}
{"id":"es2-29","class":"django_view_method_guard_does_not_change_urlpattern","patch":[{"op":"remove","path":"/d"}]}
{"id":"es3-23","class":"django_include_list_snapshot_vs_module_lookup","patch":[{"op":"remove","path":"/d"}]}
{"id":"es3-24","class":"django_end_anchored_regex_include_empty_remainder","patch":[{"op":"remove","path":"/accepted_loss"},{"op":"remove","path":"/d"},{"op":"replace","path":"/expect_mounts/0/prefix","value":{"status":"literal","value":"desk/"}},{"op":"add","path":"/expect_mounts/0/guard_pattern","value":"^desk/$"},{"op":"add","path":"/expect_mounts/0/end_guard","value":true},{"op":"add","path":"/expect/-","value":{"framework":"django","site":{"file":"desk_urls.py","line":13,"text":"path('hours/', hours)","occurrence":1},"methods":"any","path":{"raw":"'hours/'","literal":"hours/"},"paths":[],"callbacks":[{"position":0,"status":"resolved","file":"desk_urls.py","name":"hours","start_line":9}],"receiver":{"status":"router","file":"desk_urls.py","binding":"urlpatterns","construction_text":"[path('', overview), path('hours/', hours)]"},"candidate":false}},{"op":"remove","path":"/expect_unlisted"}]}
{"id":"es4-15","class":"django_unanchored_regex_include_matches_at_nonzero_offset","patch":[{"op":"remove","path":"/d"},{"op":"replace","path":"/expect_mounts/0/prefix","value":{"status":"unevaluated","text":"r'desk/'"}},{"op":"add","path":"/expect_mounts/0/guard_pattern","value":"desk/"},{"op":"add","path":"/expect_mounts/0/end_guard","value":false}]}
{"id":"es4-16","class":"django_duplicate_urlpatterns_preserve_registration_sites","patch":[{"op":"add","path":"/expect/0/paths/0/shadowed","value":false},{"op":"add","path":"/expect/1/paths/0/shadowed","value":true},{"op":"remove","path":"/d"}]}
{"id":"es5-13","class":"django_terminal_regex_without_end_anchor_matches_remainder","patch":[{"op":"replace","path":"/expect/1/paths/0/reason","value":"regex_path"},{"op":"replace","path":"/accepted_loss","value":"regex_path"},{"op":"remove","path":"/d"}]}
{"id":"es6-04","class":"django_request_urlconf_override_selects_view","patch":[{"op":"remove","path":"/d"},{"op":"replace","path":"/accepted_loss","value":"no_known_mount"},{"op":"add","path":"/expect/0/paths/0/root","value":null},{"op":"add","path":"/expect/1/paths/0/root","value":{"file":"urls.py","binding":"urlpatterns","construction_byte":159,"invocations":[]}}]}
{"id":"es6-05","class":"django_settings_select_include_urlconf","patch":[{"op":"remove","path":"/d"},{"op":"replace","path":"/accepted_loss","value":"mount_child_unproven"}]}
{"id":"es6-06","class":"django_i18n_omits_default_language_prefix","patch":[{"op":"remove","path":"/d"}]}
{"id":"es6-07","class":"django_debug_false_excludes_conditional_urlpattern","patch":[{"op":"remove","path":"/d"}]}
{"id":"es7-05","class":"django_append_slash_redirect_and_view_opt_out","patch":[{"op":"remove","path":"/d"}]}
```

### w3-nestjs (26)

```jsonl
{"id":"cx61","class":"nestjs_controller_prefix_get","patch":[{"op":"remove","path":"/d"}]}
{"id":"cx62","class":"nestjs_delete","patch":[{"op":"remove","path":"/d"}]}
{"id":"cx63","class":"nestjs_patch","patch":[{"op":"remove","path":"/d"}]}
{"id":"cx64","class":"nestjs_post","patch":[{"op":"remove","path":"/d"}]}
{"id":"cx65","class":"nestjs_put","patch":[{"op":"remove","path":"/d"}]}
{"id":"cx66","class":"nestjs_controller_without_prefix","patch":[{"op":"remove","path":"/d"}]}
{"id":"et1-14","class":"nestjs_all_and_uri_method_version","patch":[{"op":"remove","path":"/d"}]}
{"id":"es1-57","class":"nestjs_controller_method_path_array_product","patch":[{"op":"remove","path":"/d"}]}
{"id":"es1-58","class":"nestjs_uri_controller_versions_and_method_override","patch":[{"op":"remove","path":"/d"}]}
{"id":"es1-59","class":"nestjs_header_versions_share_uri","patch":[{"op":"add","path":"/expect/0/paths/0/version","value":{"kind":"header","header":"X-Revision","value":"1"}},{"op":"add","path":"/expect/1/paths/0/version","value":{"kind":"header","header":"X-Revision","value":"2"}},{"op":"remove","path":"/d"}]}
{"id":"es1-60","class":"nestjs_global_prefix_and_method_specific_exclusion","patch":[{"op":"remove","path":"/d"}]}
{"id":"es1-61","class":"nestjs_cross_file_module_graph_and_unregistered_controller","patch":[{"op":"remove","path":"/d"}]}
{"id":"es1-62","class":"nestjs_inherited_route_and_undecorated_override","patch":[{"op":"remove","path":"/d"}]}
{"id":"es1-63","class":"nestjs_request_mapping_options_and_aliased_head","patch":[{"op":"remove","path":"/d"}]}
{"id":"es1-64","class":"nestjs_module_router_prefix","patch":[{"op":"remove","path":"/d"}]}
{"id":"es2-30","class":"nestjs_custom_composed_route_decorator","patch":[{"op":"remove","path":"/d"},{"op":"replace","path":"/accepted_loss","value":"unsupported_registration"},{"op":"add","path":"/expect_diagnostics","value":[{"reason":"unsupported_registration","site":{"file":"app.ts","line":15,"text":"@LibraryGet('catalog')"}}]}]}
{"id":"es2-31","class":"nestjs_default_uri_version_without_v_prefix","patch":[{"op":"remove","path":"/d"}]}
{"id":"es2-32","class":"nestjs_stacked_verb_decorator_metadata_overwrite","patch":[{"op":"remove","path":"/d"}]}
{"id":"es3-25","class":"nestjs_host_constrained_controllers_share_path","patch":[{"op":"add","path":"/expect/0/paths/0/hosts","value":["staff.example.test"]},{"op":"add","path":"/expect/0/paths/0/normalized","value":"/reports/summary"},{"op":"add","path":"/expect/1/paths/0/hosts","value":[":tenant.accounts.example.test"]},{"op":"add","path":"/expect/1/paths/0/normalized","value":"/reports/summary"},{"op":"remove","path":"/accepted_loss"},{"op":"remove","path":"/d"},{"op":"add","path":"/expect/0/paths/0/host_kind","value":"literal"},{"op":"add","path":"/expect/1/paths/0/host_kind","value":"parameterized"}]}
{"id":"es3-26","class":"nestjs_sse_route_and_redirect_metadata","patch":[{"op":"remove","path":"/d"}]}
{"id":"es4-17","class":"nestjs_configuration_global_prefix_retains_controller_suffix","patch":[{"op":"remove","path":"/d"}]}
{"id":"es5-14","class":"nestjs_router_module_child_prefix_composition","patch":[{"op":"remove","path":"/d"}]}
{"id":"es6-08","class":"nestjs_async_dynamic_module_declares_controller","patch":[{"op":"remove","path":"/d"}]}
{"id":"es6-09","class":"nestjs_dynamic_module_option_controls_controller_registration","patch":[{"op":"remove","path":"/d"},{"op":"replace","path":"/accepted_loss","value":"no_known_mount"}]}
{"id":"es6-10","class":"nestjs_forward_ref_module_cycle_reaches_controller_once","patch":[{"op":"remove","path":"/d"}]}
{"id":"es7-09","class":"nestjs_method_version_metadata_without_enabled_versioning","patch":[{"op":"add","path":"/expect/0/paths/0/version","value":null},{"op":"add","path":"/expect/1/paths/0/version","value":null},{"op":"remove","path":"/d"}]}
```

### w3-object-table (11)

```jsonl
{"id":"ol01","class":"grounded_table_consumer","patch":[{"op":"add","path":"/expect/0/element_site","value":{"file":"app.js","line":5,"text":"{ method: 'GET', path: '/x', handler: status }","occurrence":1}},{"op":"add","path":"/expect/0/element_ordinal","value":0},{"op":"remove","path":"/d"}]}
{"id":"ol02","class":"exported_route_table","patch":[{"op":"add","path":"/expect/0/element_site","value":{"file":"routes.js","line":3,"text":"{ method: 'GET', path: '/x', handler: status }","occurrence":1}},{"op":"add","path":"/expect/0/element_ordinal","value":0},{"op":"remove","path":"/d"}]}
{"id":"ol03","class":"inline_table_callback","patch":[{"op":"add","path":"/expect/0/element_site","value":{"file":"app.js","line":3,"text":"{ method: 'GET', path: '/x', handler: (_request, response) => response.end('ready') }","occurrence":1}},{"op":"add","path":"/expect/0/element_ordinal","value":0},{"op":"remove","path":"/d"}]}
{"id":"ol04","class":"table_method_array","patch":[{"op":"add","path":"/expect/0/element_site","value":{"file":"app.js","line":5,"text":"{ method: ['GET', 'POST'], path: '/x', handler: status }","occurrence":1}},{"op":"add","path":"/expect/0/element_ordinal","value":0},{"op":"remove","path":"/d"}]}
{"id":"ol05","class":"ungrounded_table_consumer","patch":[{"op":"add","path":"/expect/0/framework","value":"object-table"},{"op":"add","path":"/expect/0/candidate","value":true}]}
{"id":"ol06","class":"unused_exported_table","patch":[]}
{"id":"ol07","class":"client_request_descriptor","patch":[]}
{"id":"cx50","class":"route_table_member_handler","patch":[{"op":"add","path":"/expect/0/element_site","value":{"file":"app.js","line":4,"text":"{ method: 'GET', path: '/export', handler: reports.csv }","occurrence":1}},{"op":"add","path":"/expect/0/element_ordinal","value":0},{"op":"remove","path":"/d"}]}
{"id":"es2-33","class":"object_table_spread_overwrites_explicit_handler","patch":[{"op":"add","path":"/expect/0/element_site","value":{"file":"app.cjs","line":9,"text":"{ method: 'GET', path: '/health', handler: health }","occurrence":1}},{"op":"add","path":"/expect/0/element_ordinal","value":0},{"op":"add","path":"/expect/1/element_site","value":{"file":"app.cjs","line":10,"text":"{ method: 'GET', path: '/latest', handler: stale, ...overrides }","occurrence":1}},{"op":"add","path":"/expect/1/element_ordinal","value":1},{"op":"remove","path":"/d"}]}
{"id":"es3-27","class":"object_table_two_installs_with_distinct_prefixes","patch":[{"op":"add","path":"/expect/0/element_site","value":{"file":"app.cjs","line":4,"text":"{ method: 'GET', path: '/summary', handler: summary }","occurrence":1}},{"op":"add","path":"/expect/0/element_ordinal","value":0},{"op":"add","path":"/expect/1/element_site","value":{"file":"app.cjs","line":4,"text":"{ method: 'GET', path: '/summary', handler: summary }","occurrence":1}},{"op":"add","path":"/expect/1/element_ordinal","value":0},{"op":"remove","path":"/d"}]}
{"id":"es5-15","class":"object_table_passive_read_and_installer_share_one_literal","patch":[{"op":"add","path":"/expect/0/element_site","value":{"file":"app.cjs","line":6,"text":"{ method: 'GET', path: '/health', handler: health }","occurrence":1}},{"op":"add","path":"/expect/0/element_ordinal","value":0},{"op":"remove","path":"/d"}]}
```

### w3-hapi (62)

```jsonl
{"id":"hp01","class":"plugin_route_array","patch":[]}
{"id":"hp02","class":"plugin_single_route","patch":[]}
{"id":"hp03","class":"plugin_options_prefix","patch":[]}
{"id":"hp04","class":"commonjs_plugin_property","patch":[]}
{"id":"hp05","class":"commonjs_plugin_object","patch":[]}
{"id":"hp06","class":"esm_plugin_exports","patch":[{"op":"add","path":"/expect/0/receiver/plugin","value":{"file":"health.js","object_byte":61,"name":"health"}},{"op":"add","path":"/expect/1/receiver/plugin","value":{"file":"users.js","object_byte":54,"name":"users"}}]}
{"id":"hp07","class":"plugin_method_forms","patch":[]}
{"id":"hp08","class":"plugin_two_mounts","patch":[{"op":"add","path":"/expect_mounts/0/child/realm","value":[{"file":"app.js","start_byte":309,"end_byte":360,"item_ordinal":0}]},{"op":"add","path":"/expect/0/paths/0/root","value":{"file":"app.js","binding":"app","construction_byte":254,"invocations":[]}},{"op":"add","path":"/expect_mounts/1/child/realm","value":[{"file":"app.js","start_byte":372,"end_byte":423,"item_ordinal":0}]},{"op":"add","path":"/expect/0/paths/1/root","value":{"file":"app.js","binding":"app","construction_byte":254,"invocations":[]}}]}
{"id":"hp09","class":"nested_plugin_mounts","patch":[]}
{"id":"hp10","class":"plugin_dynamic_prefix_and_path","patch":[{"op":"add","path":"/expect_failed_roots","value":[]},{"op":"add","path":"/expect_uncertain_roots","value":[{"root":{"file":"app.js","binding":"app","construction_byte":468,"invocations":[]},"site":{"file":"app.js","line":14,"text":"{ method: 'GET', path: PATH, handler: reports }","occurrence":1},"reason":"dispatch_conflict"}]}]}
{"id":"hp11","class":"plugin_handler_forms","patch":[{"op":"add","path":"/expect/0/callbacks/0/role","value":"handler"},{"op":"add","path":"/expect/1/callbacks/0/role","value":"handler"},{"op":"add","path":"/expect/2/callbacks/0/role","value":"handler"},{"op":"add","path":"/expect/3/callbacks/0/role","value":"pre"},{"op":"add","path":"/expect/3/callbacks/0/group","value":0},{"op":"add","path":"/expect/3/callbacks/1/role","value":"handler"},{"op":"add","path":"/expect/4/callbacks/0/role","value":"handler"},{"op":"remove","path":"/d"}]}
{"id":"hp12","class":"lookalike_route_consumer","patch":[{"op":"add","path":"/expect/0/framework","value":"object-table"},{"op":"add","path":"/expect/0/candidate","value":true}]}
{"id":"hp13","class":"fastify_lookalike_plugin","patch":[]}
{"id":"hp14","class":"plugin_missing_name_or_pkg","patch":[{"op":"add","path":"/expect/0/framework","value":"object-table"},{"op":"add","path":"/expect/0/candidate","value":true},{"op":"add","path":"/expect_diagnostics","value":[{"site":{"file":"app.js","line":6,"text":"require('./users')","occurrence":1},"reason":"hapi_plugin_unproven"}]}]}
{"id":"hp15","class":"bare_register_function","patch":[{"op":"add","path":"/expect/0/framework","value":"object-table"},{"op":"add","path":"/expect/0/candidate","value":true}]}
{"id":"hp16","class":"unregistered_plugin_object","patch":[{"op":"add","path":"/expect/0/framework","value":"object-table"},{"op":"add","path":"/expect/0/candidate","value":true}]}
{"id":"hp17","class":"shadowed_plugin_server_parameter","patch":[{"op":"add","path":"/expect/0/framework","value":"object-table"},{"op":"add","path":"/expect/0/candidate","value":true},{"op":"add","path":"/expect/1/framework","value":"object-table"},{"op":"add","path":"/expect/1/candidate","value":true}]}
{"id":"hp18","class":"invalid_plugin_server_parameters","patch":[{"op":"add","path":"/expect/0/framework","value":"object-table"},{"op":"add","path":"/expect/0/candidate","value":true},{"op":"add","path":"/expect/1/framework","value":"object-table"},{"op":"add","path":"/expect/1/candidate","value":true},{"op":"add","path":"/expect_diagnostics","value":[{"site":{"file":"app.js","line":24,"text":"destructured","occurrence":4},"reason":"hapi_server_param_unproven"},{"site":{"file":"app.js","line":24,"text":"reassigned","occurrence":3},"reason":"hapi_server_param_unproven"}]}]}
{"id":"hp19","class":"imported_hapi_route_array","patch":[{"op":"add","path":"/expect/0/install_site","value":{"file":"app.js","line":4,"text":"server.route(require('./routes'))","occurrence":1}},{"op":"remove","path":"/d"}]}
{"id":"hp20","class":"conflicting_or_missing_handler","patch":[{"op":"add","path":"/expect/0/paths","value":[]},{"op":"add","path":"/expect/0/candidate","value":true},{"op":"add","path":"/expect/0/reason","value":"hapi_handler_conflict"},{"op":"add","path":"/expect/1/paths","value":[]},{"op":"add","path":"/expect/1/candidate","value":true},{"op":"add","path":"/expect/1/reason","value":"hapi_handler_conflict"},{"op":"add","path":"/expect_failed_roots","value":[{"root":{"file":"app.js","binding":"app","construction_byte":386,"invocations":[]},"site":{"file":"app.js","line":10,"text":"{ method: 'GET', path: '/duplicate', handler: list, config: { handler: fallback } }","occurrence":1},"reason":"hapi_handler_conflict"}]},{"op":"add","path":"/expect_uncertain_roots","value":[]}]}
{"id":"hp21","class":"legacy_plugin_attributes","patch":[{"op":"add","path":"/expect/0/framework","value":"object-table"},{"op":"add","path":"/expect/0/candidate","value":true},{"op":"replace","path":"/accepted_loss","value":"hapi_plugin_unproven"},{"op":"remove","path":"/d"},{"op":"add","path":"/expect_diagnostics","value":[{"site":{"file":"app.js","line":6,"text":"require('./users')","occurrence":1},"reason":"hapi_plugin_unproven"}]}]}
{"id":"hp22","class":"plugin_registration_loop","patch":[{"op":"replace","path":"/expect/0/paths","value":[{"status":"suffix","value":"/health","reason":"no_known_mount","root":null}]},{"op":"replace","path":"/expect/0/callbacks/0/status","value":"resolved"},{"op":"remove","path":"/expect/0/callbacks/0/text"},{"op":"remove","path":"/expect/0/callbacks/0/reason"},{"op":"add","path":"/expect/0/callbacks/0/file","value":"app.js"},{"op":"add","path":"/expect/0/callbacks/0/name","value":"status"},{"op":"add","path":"/expect/0/callbacks/0/start_line","value":4},{"op":"replace","path":"/expect/0/receiver/status","value":"router"},{"op":"remove","path":"/expect/0/receiver/reason"},{"op":"add","path":"/expect/0/receiver/file","value":"app.js"},{"op":"add","path":"/expect/0/receiver/binding","value":"server"},{"op":"add","path":"/expect/0/receiver/construction_text","value":"server"},{"op":"add","path":"/expect/0/receiver/proof","value":"plugin_contract"},{"op":"add","path":"/expect/0/receiver/construction_byte","value":317},{"op":"add","path":"/expect/0/framework","value":"hapi"},{"op":"replace","path":"/expect/1/paths","value":[{"status":"suffix","value":"/users","reason":"no_known_mount","root":null}]},{"op":"replace","path":"/expect/1/callbacks/0/status","value":"resolved"},{"op":"remove","path":"/expect/1/callbacks/0/text"},{"op":"remove","path":"/expect/1/callbacks/0/reason"},{"op":"add","path":"/expect/1/callbacks/0/file","value":"app.js"},{"op":"add","path":"/expect/1/callbacks/0/name","value":"list"},{"op":"add","path":"/expect/1/callbacks/0/start_line","value":3},{"op":"replace","path":"/expect/1/receiver/status","value":"router"},{"op":"remove","path":"/expect/1/receiver/reason"},{"op":"add","path":"/expect/1/receiver/file","value":"app.js"},{"op":"add","path":"/expect/1/receiver/binding","value":"server"},{"op":"add","path":"/expect/1/receiver/construction_text","value":"server"},{"op":"add","path":"/expect/1/receiver/proof","value":"plugin_contract"},{"op":"add","path":"/expect/1/receiver/construction_byte","value":163},{"op":"add","path":"/expect/1/framework","value":"hapi"},{"op":"remove","path":"/d"},{"op":"add","path":"/accepted_loss","value":"hapi_registration_unproven"},{"op":"add","path":"/expect_diagnostics","value":[{"site":{"file":"app.js","line":25,"text":"p","occurrence":16},"reason":"hapi_registration_unproven"}]},{"op":"add","path":"/expect/0/receiver/plugin","value":{"file":"app.js","object_byte":282,"name":"health"}},{"op":"add","path":"/expect/0/paths/0/path_tokens","value":[["literal","/health"]]},{"op":"add","path":"/expect/1/receiver/plugin","value":{"file":"app.js","object_byte":129,"name":"users"}},{"op":"add","path":"/expect/1/paths/0/path_tokens","value":[["literal","/users"]]},{"op":"add","path":"/expect/0/paths/0/mount_refusal","value":"hapi_registration_unproven"},{"op":"add","path":"/expect/1/paths/0/mount_refusal","value":"hapi_registration_unproven"},{"op":"add","path":"/expect/0/paths/0/host_kind","value":"unconstrained"},{"op":"add","path":"/expect/0/paths/0/hosts","value":[]},{"op":"add","path":"/expect/1/paths/0/host_kind","value":"unconstrained"},{"op":"add","path":"/expect/1/paths/0/hosts","value":[]}]}
{"id":"cx07","class":"hapi_factory_built_register_list","patch":[{"op":"replace","path":"/expect/0/paths","value":[{"status":"suffix","value":"/checks","reason":"no_known_mount","root":null}]},{"op":"replace","path":"/expect/0/callbacks/0/status","value":"resolved"},{"op":"remove","path":"/expect/0/callbacks/0/text"},{"op":"remove","path":"/expect/0/callbacks/0/reason"},{"op":"add","path":"/expect/0/callbacks/0/file","value":"app.js"},{"op":"add","path":"/expect/0/callbacks/0/name","value":"ping"},{"op":"add","path":"/expect/0/callbacks/0/start_line","value":5},{"op":"replace","path":"/expect/0/receiver/status","value":"router"},{"op":"remove","path":"/expect/0/receiver/reason"},{"op":"add","path":"/expect/0/receiver/file","value":"app.js"},{"op":"add","path":"/expect/0/receiver/binding","value":"pluginServer"},{"op":"add","path":"/expect/0/receiver/construction_text","value":"pluginServer"},{"op":"add","path":"/expect/0/receiver/proof","value":"plugin_contract"},{"op":"add","path":"/expect/0/receiver/construction_byte","value":157},{"op":"add","path":"/expect/0/framework","value":"hapi"},{"op":"remove","path":"/d"},{"op":"add","path":"/expect_diagnostics","value":[{"site":{"file":"app.js","line":17,"text":"makePlugins()","occurrence":2},"reason":"hapi_registration_unproven"}]},{"op":"add","path":"/expect/0/receiver/plugin","value":{"file":"app.js","object_byte":122,"name":"checks"}},{"op":"add","path":"/expect/0/paths/0/path_tokens","value":[["literal","/checks"]]},{"op":"add","path":"/expect/0/paths/0/mount_refusal","value":"hapi_registration_unproven"},{"op":"add","path":"/expect/0/paths/0/host_kind","value":"unconstrained"},{"op":"add","path":"/expect/0/paths/0/hosts","value":[]}]}
{"id":"cx08","class":"hapi_register_literal_array","patch":[]}
{"id":"cx44","class":"hapi_name_and_pkg_plugin","patch":[]}
{"id":"cx45","class":"hapi_pkg_only_plugin","patch":[]}
{"id":"cx46","class":"hapi_wrapped_register_property","patch":[{"op":"add","path":"/expect/0/framework","value":"object-table"},{"op":"add","path":"/expect/0/candidate","value":true},{"op":"add","path":"/expect_diagnostics","value":[{"site":{"file":"app.js","line":16,"text":"plugin","occurrence":4},"reason":"hapi_plugin_unproven"}]}]}
{"id":"cx47","class":"hapi_route_call_argument","patch":[{"op":"replace","path":"/expect/0/paths","value":[]},{"op":"replace","path":"/expect/0/site/text","value":"buildRoute()"},{"op":"replace","path":"/expect/0/site/occurrence","value":2},{"op":"add","path":"/expect/0/callbacks","value":[]},{"op":"add","path":"/expect/0/candidate","value":true},{"op":"add","path":"/expect/0/reason","value":"hapi_route_not_literal"},{"op":"remove","path":"/d"}]}
{"id":"cx48","class":"hapi_route_call_array_item","patch":[{"op":"replace","path":"/expect/0/paths","value":[]},{"op":"add","path":"/expect/0/callbacks","value":[]},{"op":"add","path":"/expect/0/candidate","value":true},{"op":"add","path":"/expect/0/reason","value":"hapi_route_not_literal"},{"op":"remove","path":"/d"},{"op":"add","path":"/expect_failed_roots","value":[]},{"op":"add","path":"/expect_uncertain_roots","value":[{"root":{"file":"app.js","binding":"app","construction_byte":48,"invocations":[]},"site":{"file":"app.js","line":9,"text":"buildRoute()","occurrence":2},"reason":"dispatch_conflict"}]}]}
{"id":"cx49","class":"hapi_root_direct_single_route","patch":[]}
{"id":"et3-01","class":"hapi_captured_server_in_invoked_nested_function","patch":[{"op":"remove","path":"/d"}]}
{"id":"et5-01","class":"hapi_namespace_import_plugin_member","patch":[]}
{"id":"et5-02","class":"hapi_require_property_module_exports_plugin","patch":[]}
{"id":"et5-03","class":"hapi_nested_cross_file_plugins","patch":[]}
{"id":"et7-01","class":"hapi_destructured_server_factory_root","patch":[]}
{"id":"et7-02","class":"hapi_defaulted_server_parameter","patch":[{"op":"replace","path":"/expect/0/paths","value":[{"status":"full","value":"/health"}]},{"op":"replace","path":"/expect/0/callbacks/0/status","value":"resolved"},{"op":"remove","path":"/expect/0/callbacks/0/text"},{"op":"remove","path":"/expect/0/callbacks/0/reason"},{"op":"add","path":"/expect/0/callbacks/0/file","value":"app.js"},{"op":"add","path":"/expect/0/callbacks/0/name","value":"health"},{"op":"add","path":"/expect/0/callbacks/0/start_line","value":3},{"op":"replace","path":"/expect/0/receiver/status","value":"router"},{"op":"remove","path":"/expect/0/receiver/reason"},{"op":"add","path":"/expect/0/receiver/file","value":"app.js"},{"op":"add","path":"/expect/0/receiver/binding","value":"server"},{"op":"add","path":"/expect/0/receiver/construction_text","value":"server = app"},{"op":"add","path":"/expect/0/framework","value":"hapi"},{"op":"replace","path":"/expect_mounts","value":[{"site":{"file":"app.js","line":10,"text":"app.register(plugin)","occurrence":1},"parent":{"status":"router","file":"app.js","binding":"app","construction_text":"Hapi.server()"},"child":{"status":"router","file":"app.js","binding":"server","construction_text":"server = app"},"prefix":{"status":"absent"}}]},{"op":"remove","path":"/accepted_loss"}]}
{"id":"es1-23","class":"hapi_inline_plugin_with_aliased_constructor","patch":[{"op":"remove","path":"/d"}]}
{"id":"es1-24","class":"hapi_bound_registration_object_and_const_register","patch":[]}
{"id":"es1-25","class":"hapi_non_hapi_register_does_not_ground_plugin","patch":[{"op":"add","path":"/expect/0/framework","value":"object-table"},{"op":"add","path":"/expect/0/candidate","value":true},{"op":"remove","path":"/accepted_loss"}]}
{"id":"es1-26","class":"hapi_shadowed_require_keeps_real_root_distinct","patch":[{"op":"add","path":"/expect/0/framework","value":"object-table"},{"op":"add","path":"/expect/0/candidate","value":true},{"op":"remove","path":"/accepted_loss"}]}
{"id":"es1-27","class":"hapi_plugin_array_spread_and_literal_sibling","patch":[{"op":"add","path":"/expect/0/framework","value":"object-table"},{"op":"add","path":"/expect/0/candidate","value":true},{"op":"add","path":"/expect_diagnostics","value":[{"site":{"file":"app.js","line":18,"text":"...extraPlugins","occurrence":1},"reason":"hapi_registration_unproven"}]},{"op":"add","path":"/expect/1/receiver/plugin","value":{"file":"app.js","object_byte":244,"name":"public"}}]}
{"id":"es1-28","class":"hapi_local_route_array_and_spread_item","patch":[{"op":"replace","path":"/expect/2/site/line","value":5},{"op":"replace","path":"/expect/2/site/text","value":"{ method: 'GET', path: '/views', handler: ready }"},{"op":"replace","path":"/expect/2/paths/0/status","value":"full"},{"op":"remove","path":"/expect/2/paths/0/reason"},{"op":"add","path":"/expect/2/paths/0/value","value":"/views"},{"op":"add","path":"/expect/2/path","value":{"raw":"'/views'","literal":"/views"}},{"op":"add","path":"/expect/2/methods","value":{"known":["GET"]}},{"op":"add","path":"/expect/2/callbacks","value":[{"position":0,"status":"resolved","file":"app.js","name":"ready","start_line":3}]},{"op":"remove","path":"/accepted_loss"},{"op":"remove","path":"/d"}]}
{"id":"es1-29","class":"hapi_vhost_and_namespace_constructor","patch":[{"op":"add","path":"/expect/0/paths/0/hosts","value":["staff.example.com"]},{"op":"remove","path":"/d"},{"op":"add","path":"/expect/0/paths/0/host_kind","value":"literal"}]}
{"id":"es1-30","class":"hapi_new_server_and_root_leaf_under_prefix","patch":[{"op":"remove","path":"/d"}]}
{"id":"es1-31","class":"hapi_helper_received_server_argument","patch":[{"op":"remove","path":"/d"}]}
{"id":"es1-32","class":"hapi_plugin_reexport_barrel","patch":[{"op":"add","path":"/expect/0/framework","value":"object-table"},{"op":"add","path":"/expect/0/candidate","value":true},{"op":"add","path":"/expect_diagnostics","value":[{"site":{"file":"app.mjs","line":6,"text":"rates","occurrence":2},"reason":"hapi_registration_unproven"}]}]}
{"id":"es1-33","class":"hapi_destructuring_write_to_server_parameter","patch":[{"op":"add","path":"/expect/0/framework","value":"object-table"},{"op":"add","path":"/expect/0/candidate","value":true},{"op":"remove","path":"/accepted_loss"},{"op":"add","path":"/expect_diagnostics","value":[{"site":{"file":"app.js","line":13,"text":"plugin","occurrence":2},"reason":"hapi_server_param_unproven"}]}]}
{"id":"es1-34","class":"hapi_once_skips_second_prefix","patch":[{"op":"remove","path":"/d"},{"op":"add","path":"/expect_mounts/0/child/realm","value":[{"file":"app.js","start_byte":259,"end_byte":313,"item_ordinal":0}]},{"op":"add","path":"/expect/0/paths/0/root","value":{"file":"app.js","binding":"app","construction_byte":48,"invocations":[]}}]}
{"id":"es1-35","class":"hapi_registration_item_prefix_overrides_options","patch":[{"op":"remove","path":"/d"}]}
{"id":"es2-24","class":"hapi_esm_export_specifier_names_local_plugin","patch":[{"op":"remove","path":"/d"}]}
{"id":"es2-25","class":"hapi_imported_single_route_object","patch":[{"op":"replace","path":"/expect/0/site/file","value":"snapshot.cjs"},{"op":"replace","path":"/expect/0/site/line","value":3},{"op":"replace","path":"/expect/0/site/text","value":"{ method: 'GET', path: '/snapshot', handler: current }"},{"op":"replace","path":"/expect/0/paths/0/status","value":"full"},{"op":"remove","path":"/expect/0/paths/0/reason"},{"op":"add","path":"/expect/0/paths/0/value","value":"/snapshot"},{"op":"add","path":"/expect/0/install_site","value":{"file":"app.cjs","line":5,"text":"server.route(snapshot)","occurrence":1}},{"op":"add","path":"/expect/0/methods","value":{"known":["GET"]}},{"op":"add","path":"/expect/0/path","value":{"raw":"'/snapshot'","literal":"/snapshot"}},{"op":"add","path":"/expect/0/callbacks","value":[{"position":0,"status":"resolved","file":"snapshot.cjs","name":"current","start_line":1}]},{"op":"remove","path":"/d"},{"op":"remove","path":"/accepted_loss"}]}
{"id":"es2-26","class":"hapi_options_factory_supplies_handler","patch":[{"op":"replace","path":"/expect/0/callbacks/0/text","value":"configuration"},{"op":"replace","path":"/expect/0/callbacks/0/reason","value":"callback_unproven"},{"op":"remove","path":"/d"},{"op":"replace","path":"/accepted_loss","value":"callback_unproven"}]}
{"id":"es3-20","class":"hapi_alias_of_grounded_plugin_server","patch":[{"op":"replace","path":"/expect/0/paths","value":[{"status":"full","value":"/ops/settings"}]},{"op":"replace","path":"/expect/0/callbacks/0/status","value":"resolved"},{"op":"remove","path":"/expect/0/callbacks/0/text"},{"op":"remove","path":"/expect/0/callbacks/0/reason"},{"op":"add","path":"/expect/0/callbacks/0/file","value":"app.cjs"},{"op":"add","path":"/expect/0/callbacks/0/name","value":"settings"},{"op":"add","path":"/expect/0/callbacks/0/start_line","value":4},{"op":"replace","path":"/expect/0/receiver/status","value":"router"},{"op":"remove","path":"/expect/0/receiver/reason"},{"op":"add","path":"/expect/0/receiver/file","value":"app.cjs"},{"op":"add","path":"/expect/0/receiver/binding","value":"server"},{"op":"add","path":"/expect/0/receiver/construction_text","value":"server"},{"op":"add","path":"/expect/0/framework","value":"hapi"},{"op":"remove","path":"/accepted_loss"},{"op":"remove","path":"/d"}]}
{"id":"es3-21","class":"hapi_literal_and_indirect_computed_route_keys","patch":[{"op":"replace","path":"/expect/1/callbacks/0/status","value":"resolved"},{"op":"remove","path":"/expect/1/callbacks/0/text"},{"op":"remove","path":"/expect/1/callbacks/0/reason"},{"op":"add","path":"/expect/1/callbacks/0/file","value":"app.cjs"},{"op":"add","path":"/expect/1/callbacks/0/name","value":"older"},{"op":"add","path":"/expect/1/callbacks/0/start_line","value":5},{"op":"remove","path":"/d"}]}
{"id":"es3-22","class":"hapi_required_optional_and_counted_wildcard_parameters","patch":[{"op":"add","path":"/expect/1/paths/0/path_tokens","value":null},{"op":"add","path":"/expect/2/paths/0/path_tokens","value":null},{"op":"remove","path":"/d"}]}
{"id":"es4-14","class":"hapi_escaped_nested_installer_does_not_run_at_registration","patch":[{"op":"remove","path":"/d"}]}
{"id":"es5-11","class":"hapi_duplicate_keys_single_handler_location","patch":[{"op":"remove","path":"/d"}]}
{"id":"es5-12","class":"hapi_parallel_pre_handlers_then_serial_handler","patch":[{"op":"add","path":"/expect/0/callbacks/0/role","value":"pre"},{"op":"add","path":"/expect/0/callbacks/0/group","value":0},{"op":"add","path":"/expect/0/callbacks/1/role","value":"pre"},{"op":"add","path":"/expect/0/callbacks/1/group","value":0},{"op":"add","path":"/expect/0/callbacks/2/role","value":"pre"},{"op":"add","path":"/expect/0/callbacks/2/group","value":1},{"op":"add","path":"/expect/0/callbacks/3/role","value":"handler"},{"op":"remove","path":"/accepted_loss"},{"op":"remove","path":"/d"}]}
{"id":"es6-01","class":"hapi_prestart_extension_adds_route_during_initialize","patch":[{"op":"remove","path":"/d"}]}
{"id":"es6-02","class":"hapi_dependency_after_callback_adds_route_at_initialize","patch":[{"op":"remove","path":"/d"}]}
{"id":"es6-03","class":"hapi_glue_manifest_registers_plugin","patch":[{"op":"add","path":"/expect/0/framework","value":"object-table"},{"op":"add","path":"/expect/0/candidate","value":true},{"op":"remove","path":"/d"},{"op":"replace","path":"/accepted_loss","value":"hapi_plugin_unregistered"}]}
{"id":"es7-06","class":"hapi_plugin_register_imported_from_separate_module","patch":[{"op":"remove","path":"/d"}]}
```

## New matrix rows required by round 1

These IDs are additions for the implementation seat after design approval. Each row names its input, required observations, truth label and census/behavior class. Capture any new executed receipt independently on its fixture; the 420 historical rows and their raw truth remain the control.

| ID (group) | Input and required behavior | Truth | Class |
|---|---|---|---|
| d1-01 (w2-nethttp) | A package var initializer immediately calls a func literal containing http.HandleFunc("GET /init", show); a stored, uncalled literal registers /later. Resolve /init to the same-package show at its declaration; no installation of /later. | hand | go_package_initializer_bare_handler |
| d1-02 (w3-hapi) | Two direct server rows use config or options. One requires an object and selects its static nested own property with pre and handler; resolve both in that module. The second reads an opaque config reference; retain full path and one callback_unproven slot. | hand | hapi_referenced_options_object |
| d1-03 (w3-hapi) | Distinct objects share effective name (explicit name vs pkg.name), with different prefixes. Second call once:true skips even with a different object. On an independent server a disallowed same-name repeat yields an item diagnostic and no second mount; pin roots and retained first origins, the failing root/site and no later credit on that failed setup. | hand | hapi_same_name_repeat |
| d1-04 (w3-hapi) | Repeat a named plugin at /b after /a with multiple set to an unknown boolean expression, then vary effective once. Admit /a where first registration is valid, no guaranteed /b mount, exactly one item diagnostic on the undecidable repeat; a known once skip has none. | hand | hapi_unknown_repeat_metadata |
| d1-05 (w1-express) | Two GET /foo layers, first calls next('route'), second sends. Keep both full, correct root/order/identities, shadowed=null and binding_blocked={known:[GET,HEAD]} on both; zero unique winner credit. | executed (native registrations; dispatch policy static) | express_duplicate_continuation |
| d1-06 (w3-object-table) | Install GET+POST /x for a then GET /x for b in the same certified Map. GET belongs only to b, POST remains on a, a records the superseded GET key. Overwrite its final POST too in a second control; retain a with callbacks and paths []. | hand | native_map_last_write |
| d1-07 (w1-express) | Invoke the same app factory at two distinct source sites and export left/right. Select each export separately at the CLI; same leaf/allocation, different invocation frames. A missing export must return error.reason=entry_unproven and the requested selector. | hand | selected_root_factory_instances |
| d1-08 (w2-method-value) | Two imported packages with the same basename declare like-named owners and different embedded method declarations. Resolve each promoted selector to its own declaring file/line; explicitly traversing its qualified embedded field refuses. | hand | go_package_qualified_promotion_collision |
| d1-09 (w3-hapi) | On one grounded server put two distinct opaque spreads before a local named literal plugin (no manifest). Either spread may install that name or abort. Require three distinct hapi_registration_unproven item diagnostics, one per spread and one for the blocked literal, no guaranteed mount, and the literal’s route origin with empty paths, no weak or binding credit. Mark setup possible-abort, not known failed. es1-27 supplies the independently noninterfering control. | hand | hapi_failed_item_multiplicity |
| d1-10 (w1-python) | Distinct Flask endpoints install the same URL and GET method; retain installed origins and callbacks, block the overlapping effective methods, and assert no unique GET binding. | executed (registrations; ambiguity static) | flask_duplicate_dispatch |
| d1-11 (w3-nestjs) | Two active controllers install the same method/path on the default Express adapter under the same unconstrained root. Preserve both installed records, block the contested method and refuse unique binding. | hand | nest_duplicate_dispatch |
| d1-12 (w2-gin) | Register identical GET /x twice on one engine; preserve second origin with dispatch_conflict and no new exposure. Use a second engine as disjoint control. Pin both inferred roots to their distinct engine constructor bytes and invocation frames, rejected event site/failed root and the healthy root’s GET handler; a root swap must fail. | hand | gin_duplicate_install |
| d1-13 (w2-nethttp) | Two HandleFunc calls install the identical pattern on one mux; retain failed second origin, no new exposure or binding on failed setup. On a second independent mux install the same pattern successfully. Pin both inferred root identities at their distinct constructors and invocation frames, exact failed root/site and surviving handler; swapping roots must fail. | hand | servemux_duplicate_install |
| d1-14 (w3-hapi) | Two identical direct method/path/vhost installs on one server conflict; retain failed second origin, no path for it. On a fresh server use distinct literal vhosts; both install. Assert exact failed site/root on the first server and both host-specific full exposures only under the healthy root. | hand | hapi_duplicate_install |
| d1-15 (w2-nethttp) | Straight-line a then b write GET /x on one chi tree, with POST /x to c as a control. Keep a with callbacks, paths [], superseded GET key and superseded reason; b alone has full GET /x, c full POST /x. Pin their common mux identity and both surviving handlers. | hand | chi_duplicate_endpoint |

## New matrix rows required by round 2

The implementation seat authors these 20 rows after approval. “Pin root” means compare the independent router identity including invocation frames. For split controls use isolated dispatcher instances and assert the exact outcomes on each. Hand labels assert static behavior; native fixture receipts, when called for, have their own provenance.

| ID (group) | Input and required behavior | Truth | Class |
|---|---|---|---|
| d2-01 (w3-hapi) | Two exported runtime-dependency plugin objects in one module have the same name and reference the same local register(server) containing GET /x. No grounded register site. Require two plugin_contract records with the **same** route, install, ordinal, callback and formal bytes, distinct plugin.object_byte, both null-root suffixes, mount_refusal=no_known_mount and unknown host. No mount. A name-only join fails. | hand | shared_register_weak_weak |
| d2-02 (w3-hapi) | Same function on exported A and B; a grounded call registers only A at /a. A has one strong full /a/x with its plugin identity and no proof field, B its own weak /x with null root and no_known_mount. Exact one mount to A’s realm. Swap A/B in the output and require failure. | hand | shared_register_strong_weak |
| d2-03 (w3-hapi) | Strong server.register(plugin,{routes:{prefix:'/api',vhost:'admin.test'}}) with GET / and no route vhost. Pin parent, child event, prefix, full /api, literal host and hosts=['admin.test']; no unconstrained or other-host credit. Second server with no vhost pins unconstrained. | hand | strong_register_realm_vhost |
| d2-04 (w3-hapi) | Two independent hp22-style loops with a closed object list and qualified manifest. Both plugins declare GET / with an indexed inline terminal. One register call supplies routes.vhost='admin.test', one has no item/call/ancestor vhost. Both weak /, null root and linked hapi_registration_unproven. First host_kind=unknown, no configured binding even with /api; second unconstrained, validated tokens for configured /api + / give /api and its anonymous identity. Test /api/ mismatch, ambiguous plugin and pre-only negative. | hand | weak_realm_and_configured_root_leaf |
| d2-05 (w3-hapi) | Export a contract object with an owned, nonliteral name (unknown effective name) and no observed register chain; nearest peerDependencies declares hapi. Pin plugin.name=null, defining byte, proof, null-root typed suffix, reason and mount_refusal=no_known_mount, unknown host and no mount/binding. | hand | unlinked_weak_plugin |
| d2-06 (w3-hapi) | hp16 with a nearest runtime Hapi manifest. A private contract stored by the non-Hapi registry has neither an export nor flow into a grounded register. Preserve its object-table candidate with hapi_plugin_unregistered and paths [], no weak proof. Export-only positive is d2-05. | hand | weak_requires_export_or_flow |
| d2-07 (w3-hapi) | es1-25 with a runtime Hapi manifest and an explicit plugin export. Fake catalog invokes its register with a descriptor collector. Require object-table candidate, hapi_plugin_unregistered, paths [], no plugin_contract or mount. | hand | weak_defeated_by_non_hapi_call |
| d2-08 (w3-hapi) | Export-only plugin, ancestor package declares Hapi, nearer package has no such dependency. Keep unproven hapi_plugin_unregistered candidate with no paths or mount. Moving the runtime dependency to the nearest manifest admits the suffix. | hand | nearest_manifest_boundary |
| d2-09 (w3-hapi) | Valid exported contract; only nearest devDependencies declares Hapi. Require the same unproven candidate. Changing that entry to peerDependencies admits a weak suffix with no mount. | hand | runtime_dependency_only |
| d2-10 (w2-nethttp) | Fresh mux for each ordered pair: GET /foo then HEAD /; /a/{x} then /{x}/b. Later install panics on each, origin/callback retained, paths [], dispatch_conflict, exact failed root/site, later sentinel never admitted. Reverse pair order on another mux and pin the new rejected origin. Host-qualified vs hostless crossing, GET /foo vs HEAD /foo (subset), and disjoint literals on healthy muxes have both installs; exact roots and method/host-appropriate winners. | hand | servemux_crossing_and_specificity |
| d2-11 (w2-gin) | Same engine/method: /who/are/*you then /who/are/foo; repeat reversed, and /users/:id then /users/:name. Pin rejected origin with empty paths and exact failed engine/site in each. Fresh engines: different methods on the catch-all/static pair coexist; ordinary /users/me vs /users/:id gives static priority on /users/me. Assert inferred root for each survivor. | hand | gin_radix_slot_conflict |
| d2-12 (w3-hapi) | Grounded register array with same-name A and B (once=false, multiple=false), then fresh C. Pin admitted A realm, failed B item/root and both B/C source records with no mount through that call. C was never attempted. On another server once:true skips B, C installs. Repeat with unknown earlier spread as d1-09, distinguish possible from proved abort. | hand | hapi_abort_and_skip_before_sibling |
| d2-13 (w2-nethttp) | chi: two mounts at the identical literal mount path on the same mux reject the second; retain its child’s origins, no second edge, failed root. Separately overwrite one exact verb via With on the same tree; last verb handler survives, a different verb survives. Conditional final write to the same slot has no certified winner/full on that slot. | hand | chi_mount_vs_method_write |
| d2-14 (w1-python) | Flask add_url_rule on different paths with the same endpoint string but different view functions. Catch the native assertion **only in the fixture observation harness** so the partial table is inspectable. Pin failed app/site and both written origins; the second view never replaces the first endpoint mapping, even if the URL map already holds its rule. Give that second view no full exposure or binding on that root. On another app reuse the same endpoint and callable legally for both paths. | hand | flask_endpoint_namespace_conflict |
| d2-15 (w1-python) | Starlette Route on an earlier literal GET-only route and later POST on the same path: later FULL gets POST despite earlier PARTIAL. Reverse ordered literal vs matching parameter GET routes in independent apps: first FULL handles that request. Both installed, other-method and websocket controls independent. | hand | starlette_partial_then_full |
| d2-16 (w3-django) | Earlier included URLconf matches a prefix but has no matching child for /p/item/, followed by a matching literal sibling; latter serves it. Add a matching child in a second URLconf to reverse the winner. Keep all installed origin sites and the exact include edge/guard. | hand | django_resolver_fallthrough |
| d2-17 (w3-nestjs) | In a grounded Nest bootstrap choose an opaque HTTP adapter and declare two active controllers with the same GET path. Keep both origins and callbacks, withhold contested full and binding (dispatch_conflict); default Express control d1-11 has two installed records. Distinct active header versions on the default adapter are independent (es1-59). | hand | nest_platform_admission |
| d2-18 (w3-object-table) | Two certified installers write one GET key on the same Map, final write under an unknown guard. Both origins retained, no unconditional surviving GET key or binding; independently written POST survives. Unconditional final write recovers d1-06. | hand | map_unknown_final_write |
| d2-19 (w2-method-value) | Embed a qualified concrete owner supplying exported Show at depth one and a local wrapper embedding an interface supplying Show at depth two. Even if the router reports EmbeddedInterfaceDispatch, resolve to the concrete declaration in its package. Move interface to depth one for ambiguity; substitute a potentially supplying generic embedding for the qualified owner and require receiver refusal. | hand | skipped_embedding_precedes_router_refusal |
| d2-20 (w2-nethttp) | chi parent := NewRouter(); parent.Route(config.PathPrefix, func(child chi.Router){ external.Install(child) }). Pin one edge, parent constructor and distinct closure child, unevaluated prefix text, no guard, zero alternative/child ordinals, no invented route. Comparator mutations swap/alias endpoints, change prefix, add guard, drop or duplicate edge; each fails individually. | hand | mount_without_scorable_descendants |
