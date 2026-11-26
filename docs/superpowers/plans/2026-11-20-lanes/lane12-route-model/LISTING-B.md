# Lane 12 · Option B · What a route record claims

Proposed consumer-only amendment under the existing listing contract. Evaluate registration, installation and handler identity separately. List every retained declaration and its refusals. Credit a registration on a router when its receiver is proved, including when its callback is unresolved. [DESIGN §5.4](DESIGN.md#54-the-listing)

## Claims

| ID | Claim | Evidence required | Consumer conclusion |
|---|---|---|---|
| B-01 | Registration | `site`, a proved `receiver`, `path.raw`, `path.literal` where decoded, and declared `methods` where known. A shortcut declares its verb. | The source declares this path expression and these method facts on this router under the adapter boundary. A nonliteral path or unknown method stays unresolved. A receiver refusal supports source navigation only. |
| B-02 | Installation | An independently proved root and effective mount chain or direct install, with composed path and per-exposure constraints. Apply the setup and root-health rules in [§5.2](DESIGN.md#52-composition-ordering-and-comparison). | A rooted `full` exposure establishes the composed path on that root at the listing epoch, conditional on supported setup completing. A `suffix` or local leaf supports search only. |
| B-03 | Handler identity | A resolved or anonymous `callbacks` slot proves the registered value at that site. Only a certified `delegate` identifies a forwarded terminal. A function join also needs proved dispatch for the matched operation. | Name the proved callable at its position. Follow `site` to the written `def` for source navigation. An opaque decorator below the route decorator may replace the registered value. A decorator above it cannot retroactively replace it. |
| B-04 | Body execution | The listing supplies no execution proof. A separate wrapper analysis would have to establish forwarding under stated conditions. | A callback identity, `functools.wraps` or a source link confers no guarantee that a body runs. A wrapper or request hook may return early, and Flask can handle automatic `OPTIONS` itself. |

## Flask listing rows

Each retained Flask HTTP rule emits `site` at the decorator or registration call, `path.raw` as written, `path.literal` as the decoded local leaf or null, and `methods` as the declaration. Preserve the explicit set, including an empty set, or the unknown expression. Use null for an omitted declaration. A string argument inside a path helper supplies no decoded leaf. Include a written literal `endpoint` and any independently proved `default_methods` under §5.4. `paths` and `callbacks` are ordered arrays and may be empty. For a receiver-withheld Flask rule, the route's `reason` repeats the receiver refusal. Every row adds one to `summary.registrations`. The summary also counts mount edges separately in `mounts` and carries `enabled_adapters`, `diagnostics` and `failed_roots`. The measured rules each have one callback slot and a null `delegate`.

| ID | Class | Listing fields in addition to the shared fields above | Consumer MAY conclude | Consumer MAY NOT conclude |
|---|---|---|---|---|
| B-05 | Decorated view, proved receiver | `receiver=router` with identity. Unresolved callback with written text and `inner_decorator` when an opaque inner decorator intervenes, `delegate=null`. `paths` contains only independently composed exposures. Adds one to `summary.proven`. | Credit registration and navigate to the written `def`. Join an operation if its separate exposure and method proofs qualify. | Identify the decorated `def` as the registered callable or dispatched body. Infer installation from the decorator. |
| B-06 | Mount escape through an imported router | `receiver=unproven(receiver_unproven)`, `candidate=true`, local path and declared method evidence retained. Callback slot retains text and receiver refusal, `delegate=null`, `paths=[]`. Adds one each to `summary.unproven` and `summary.candidates`. | Display the candidate and any independently retained written method or prefix. An omitted prefix on the escaped mount leaves the effective prefix unknown. | Name a proved router allocation, construct a root URL, or credit a handler. |
| B-07 | Terminal re-export | `receiver=unproven(terminal_reexport)`, `candidate=true`. Callback slot retains text with `terminal_reexport`, `delegate=null`, `paths=[]`. Adds one each to `summary.unproven` and `summary.candidates`. No counted import refusal for this reason. | Display the candidate, its local spelling and the role refusal. | Treat the forwarding binding as receiver, installation or handler proof. |
| B-08 | Class-based `as_view` with proved receiver | `receiver=router` with identity. Unresolved callback with argument text and `class_view`, `delegate=null`. `paths` contains only independently composed exposures. Adds one to `summary.proven`. | Credit registration. Use independently proved method evidence under CR2 and join an operation if its exposure qualifies. | Infer that an `as_view` call resolves to `dispatch_request`, an HTTP verb method or any other body. |
| B-09 | Other unproven receiver, if retained | `receiver=unproven` with its specific reason, `candidate=true`. Any callback slots retain text with that reason, `delegate=null`, `paths=[]`. Adds one each to `summary.unproven` and `summary.candidates`. | Display the source facts and refusal at the site. | Credit a registration on a known router, an exposure or a handler. |

- **B-05** An inner decorator can replace the callable without invalidating the receiver or the declaration.
- **B-06** A traceable child can escape through an unproved parent, so the written prefix and effective prefix have separate proof.
- **B-07** The eligible forwarding target justifies retention at the source site under the current extractor's rule.
- **B-08** `as_view` produces a callable and may supply view metadata, while the callable's body remains unresolved.
- **B-09** Retention requires adapter evidence such as a router hint. A method spelling alone establishes no receiver.

The 63 measured receiver-withheld rows fall under B-06 or B-07. B-09 pins the general retention rule. [Measurement addendum](/Users/wesleyjinks/prism-evidence/routes/s2-static/MEASURE.md:124) · [Mount diagnosis](/Users/wesleyjinks/prism-evidence/routes/s2-diag/DIAG.md:59) · [Gap report](/Users/wesleyjinks/prism-evidence/routes/s2-static/GAPS-REPORT.md:18)

## Consumer pins

| ID | Rule | Positive | Control |
|---|---|---|---|
| CR1 | Source provenance is not installation. An unproven receiver yields `paths=[]`. Keep distinct source origins and router identities even when suffixes match. Never complete a local leaf or suffix with an assumed empty prefix. | Blueprints `a` and `b` each declare `/items` and mount on a proved app at `/a` and `/b`. Keep both origins and use the independently proved `/a/items` and `/b/items` exposures. | Replace only `a`'s mount with one on an opaque parent and withhold its receiver. Keep its local `/items` and empty `paths`, separate from `b`. Preserve `b`'s independently proved `/b/items`. With both mounts removed, the identical `suffix` exposures on proved receivers remain distinct and yield no rooted `/items`. |
| CR2 | Omitted `methods` asserts no verb. Require explicit known method evidence for the strict predicate. A default-inclusive consumer may use a verb only when `default_methods` independently proves it. | On an otherwise eligible `full` exposure, `.post('/items')` or `methods=['POST']` supports `POST`. A separately proved `default_methods={"known":["GET"]}` supports `GET` only under the default-inclusive predicate. | Keep the eligible path and replace the explicit verb with `route('/items')` whose `methods` is null and whose `default_methods` is absent, for example when view metadata is opaque. Withhold `GET`. An unrestricted `method_guard` or Flask's usual fallback supplies no missing declaration. |

For a known Flask rule with no shortcut or method list, `methods` is null on the §5.4 wire. Treat an absent declaration as unknown as well. `flask.rs` stores `default_methods` separately on `Route`. `measure.rs` omits it from the measurement JSONL, and `table.rs` assembles the route table without serializing it. §5.4 already specifies the optional wire field when independently proved. The inspected files therefore establish the contract and the internal value, while actual listing serialization remains unverified in the permitted source set. B's strict figure needs no defaults. Defer default-inclusive credit. If the deployed wire lacks field support, enabling it costs an emitted field, a wire/schema update and a listing version change. A producer already emitting the §5.4 field needs proof and measurement for each admitted default. [Flask method computation](/Users/wesleyjinks/code/prism-routes-b/src/framework_entries/routes/python/flask.rs:32) · [Measurement serializer](/Users/wesleyjinks/code/prism-routes-b/src/framework_entries/routes/tests/flask/measure.rs:125) · [Route table](/Users/wesleyjinks/code/prism-routes-b/src/framework_entries/routes/table.rs:10)

## Lane 5 joins

| ID | Predicate |
|---|---|
| B-10 | **Operation join.** Require exactly one eligible `full` exposure for the scoped operation. Match the bound root, HTTP protocol, validated `path_tokens` and a compatible known explicit method. Admit independently proved `default_methods` only in the default-inclusive predicate. Apply method guards, host and active version selectors, and exclude failed roots, unproved setup guards, shadows and blocked or unresolved dispatch for the requested key under §5.2. A local registration, suffix or dynamic exposure never supplies an operation join. Callback proof is immaterial to this join. |
| B-11 | **Function join.** Require B-10 plus a proved terminal identity or certified terminal delegate for the dispatched slot. A source `def`, middleware or prerequisite alone earns no function credit. |

## Summary and compatibility

| ID | Rule |
|---|---|
| B-12 | Keep `summary.proven = summary.registrations - summary.unproven`. `RouteTable::new` counts proved receivers regardless of callback status. The measured 48 uses the stricter predicate of a proved receiver and a nonempty callback list with every slot resolved or anonymous. |
| B-13 | In a separate versioned §5.4 listing-schema amendment, add two optional unsigned counts. `handler_proved` counts rows with a proved receiver and a resolved or anonymous registered handler slot under the adapter's role rules. `installation_proved` counts rows with at least one rooted, unguarded `full` exposure on a nonfailed root. Count each qualifying row once per field. `proven` supplies the registration-receiver count. |

B can derive these counts locally. Adding them to the wrapped summary changes the wire. Keep `registrations = proven + unproven` for existing readers. Readers that ignore unknown keys can consume the additive shape. Strict §5.4 validators and matrix expectations need the new listing schema version. Treat absent counts on older listings as unavailable. [Summary construction](/Users/wesleyjinks/code/prism-routes-b/src/framework_entries/routes/table.rs:73) · [Measurement predicate](/Users/wesleyjinks/code/prism-routes-b/src/framework_entries/routes/tests/flask/measure.rs:101)

## Scope of B, C and D

**B** changes the consumer predicates only. Existing receiver vetoes, mount proof, callback resolution and delegate certification govern every claim. Source-router attribution for withheld receivers and body execution are outside B. Consumers need the public listing to carry the Flask facts. ASTRA reports an empty adapter set at PR #928's cited checkpoint. [ASTRA implementation prerequisite](/Users/wesleyjinks/prism-evidence/routes/s2-diag/ASTRA-ANSWER.md:65)

**C** adds optional source-router identity distinct from `receiver` for a Blueprint allocation independently traced before a mount-escape veto. The row keeps `receiver_unproven`, `paths=[]` and an unknown effective prefix for the refused mount. An ambiguous or untraced allocation has no source-router identity.

**D** adds one bounded static terminal re-export hop on top of C. The same optional field identifies the source Blueprint only when the hop reaches a unique eligible allocation and carries the forwarding site as provenance. Keep `terminal_reexport` as the receiver refusal and retain independent vetoes. Unsupported chains and missing or ambiguous targets earn no attribution. [ASTRA options](/Users/wesleyjinks/prism-evidence/routes/s2-diag/ASTRA-ANSWER.md#options)

## Predicted yields

**PREDICTED for B.** Replace with end-to-end listing measurements on the same five corpora. Count Rule origins once and exclude Mount events. A strict local key requires a proved receiver, a decoded literal leaf and at least one explicit verb. The default-inclusive column is a bound for adding independently proved defaults. The function column measures source registration identity. The receipts certify zero rooted method/path joins and zero rooted function bindings. Their actual yields remain unmeasured. [ASTRA yield derivation](/Users/wesleyjinks/prism-evidence/routes/s2-diag/ASTRA-ANSWER.md#measured-yields) · [Measurement scope](/Users/wesleyjinks/prism-evidence/routes/s2-static/MEASURE.md#measurement-and-custody)

| Corpus | Rules | Receiver-proved registration | Strict local path + method | Local path + method with defaults | Source function identity |
|---|---:|---:|---:|---:|---:|
| CTFd | 77 | 77 | 29 | 29–44 | 19 |
| Microblog | 27 | 0 | 0 | 0 | 0 |
| Redash | 25 | 4 | 0 | 0 | 4 |
| Flask | 37 | 26 | 1 | 1–26 | 25 |
| Superset | 4 | 0 | 0 | 0 | 0 |
| **Total** | **170** | **107** | **30** | **30–70** | **48** |

## Corrections

- ASTRA's wording about omitted methods can read as filling `methods` from `default_methods`. `flask.rs` computes separate declaration, default and effective method values. The declaration stays null when omitted, even with a proved default.
- ASTRA's `terminal_reexport` label agrees with the Flask extractor and measurement. DESIGN §5.3 calls it census-only and specifies `imported_unproven` for the retained role. The closed §5.1 reason enum omits `terminal_reexport`. B records the observed Flask role refusal and leaves the wire discrepancy for owner disposition. [DESIGN §5.3](DESIGN.md#router-and-view-value-boundary-python) · [Extractor](/Users/wesleyjinks/code/prism-routes-b/src/framework_entries/routes/python/flask.rs:296) · [Gap 3](/Users/wesleyjinks/prism-evidence/routes/s2-static/GAPS-REPORT.md:30)
