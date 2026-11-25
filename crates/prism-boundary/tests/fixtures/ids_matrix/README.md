# Element identity matrix

Each fixture path is relative to this directory. A route-table configuration or other source evidence
for that row lives beside the named fixture. The two server-order fixtures, two envelope-order fixtures,
and two definition-rename fixtures are paired comparisons.

The expected IDs cover operations and named fields in their request and success-response bodies.
Warnings are the tentative reader's warning codes, including on gap rows. A diagnostic input is called
out where it deliberately uses a construct outside its format.

For host-only, header-only, open-method, and shared-pointer gaps the columns list any identities that
can be spelled without conflating distinct elements. The comments identify the remaining semantic
requirements. Optional and catch-all path rows give the literal route templates as concrete examples
of the matching behaviour. Server-order rows list each callable path for comparison across revisions.

| Row | Comment |
| --- | --- |
| `allof-envelope-first` | Reversing the two envelope members preserves the same four fields. |
| `allof-envelope-last` | Reversing the two envelope members preserves the same four fields. |
| `definition-pointer-punctuation` | The reference addresses a punctuated Swagger definition name. Only the wire property enters the ID. |
| `field-exact-star-vs-map` | The literal star property and a dictionary value each carry a distinct tone field. |
| `gin-clean-slash-join` | Gin cleans the registered group and path when it joins them. |
| `gin-mixed-segment` | Gin treats the wildcard after the static prefix as a capture. The Python normaliser only matches colons at segment starts. |
| `oas-body-fallback-sorted` | XML sorts before YAML even though the YAML content member appears first. |
| `oas-body-fallback-sorted-control` | The sole named field is from XML, which sorts before the opaque YAML representation written first. |
| `oas-body-json-preferred` | The opaque CBOR representation sorts first and the JSON representation carries the only named field. |
| `oas-colon-literal` | A colon is literal in a schema path. The Python normaliser currently applies its route colon regex here. |
| `oas-default-only` | There is no explicitly declared success response to root a field. |
| `oas-lowercase-range` | Deliberately invalid lowercase range accompanies a valid empty 200 response. |
| `oas-range-and-exact-status` | The exact 201 owns its status and the 2XX entry describes other successes. |
| `oas-response-media-same-pointer` | JSON mark and XML mark require separate field identities. Only the operation and its unambiguous XML child fit the current syntax. |
| `oas-server-order-north-first` | The two order fixtures describe the same operation and both deployment paths. |
| `oas-server-order-south-first` | The two order fixtures describe the same operation and both deployment paths. |
| `oas-server-trailing-join` | The server URL ends with a slash and the declared relative path starts with one. |
| `oas-server-variable-collapse` | Both allowed deployments reach the same declaration at distinct paths. |
| `oas-server-variable-distinct-operations` | The companion schemas document independent operations with identical path items and different required server paths. |
| `oas-uppercase-key` | Deliberately invalid operation key. An uppercase path-item member declares no operation. |
| `route-express4-optional` | The Express 4 registrar accepts both /mugs and /mugs/large. |
| `route-gateway-stage` | Only requests entering the declared stage are forwarded to the internal table. |
| `route-hapi-catchall` | A single capture matches /mugs/rim and /mugs/rim/top. |
| `route-hapi-fixed-catchall` | The route accepts /mugs/rim/top and rejects /mugs/rim. |
| `route-hapi-insensitive` | The lowercase accepted spelling is the witness for the same registration reached through /Mugs and /mugs. |
| `route-hapi-optional` | One registration accepts both /mugs and /mugs/large. |
| `route-hapi-strip-trailing` | The router accepts /mugs and /mugs/ through one declared route. |
| `route-hapi-vhosts` | The two handlers have distinct response bodies and require distinct operation IDs. |
| `route-hapi-wildcard-method` | The fallback handles every applicable incoming method, including methods outside any fixed list. |
| `route-header-dispatch` | The x-finish header selects one of two independent handlers with the same method and path. |
| `route-method-head-options` | The generic route table explicitly registers both methods. This is distinct from Hapi's automatic HEAD handling. |
| `route-method-list` | The prototype already expands string arrays into separate routes. |
| `route-proxy-rewrite` | The upstream is internal and the ingress path is the public identity. |
| `swag-detached-annotation` | The blank line detaches the annotation from the function. |
| `swag-method-drift` | The registration gives the served operation while the annotation supplies stale identity evidence. |
| `swag-path-drift` | The registration gives the served operation while the annotation supplies stale identity evidence. |
| `swag-two-annotations` | Both annotation lines attach to the same function. |
| `swagger-body-path-inheritance` | The operation overrides the inherited parameter with the same name and location. |
| `swagger-disjoint-collision` | The two equivalent path templates have disjoint methods and both message roots survive. |
| `swagger-duplicate-collision` | Ambiguous input from Review focus 1 lists the losing path first in the file. |
| `swagger-range-key` | Deliberately invalid Swagger wildcard status accompanies a valid empty 200 response. |
| `swagger-repeated-name` | One parameter declaration is used at both template positions. |
| `swagger-root-with-base` | Appending the Swagger path / to the base /studio yields /studio/. |
| `swagger-trace-key` | Deliberately invalid Swagger Path Item probes the difference from OpenAPI 3 TRACE. |
| `wire-container-cycle` | Both container properties lead back to Coil and stay present without recursive descendants. |
| `wire-depth-17` | Layer 17 is a real finite field. The tentative reader warns after layer 16. |
| `wire-direct-cycle` | The next field holds the back-reference to Coil. |
| `wire-neutral-rename-clayreceipt` | The two rename fixtures change only the definition key and its reference. |
| `wire-neutral-rename-pressedreceipt` | The two rename fixtures change only the definition key and its reference. |
| `wire-sibling-ref-reuse` | Two sibling expansions reach the same definition independently. |

| `t1-names-ascii-keys` | Numeric-looking keys, case variants, and __proto__ are distinct literal JSON member names. |
| `t1-ref-inline-pointer` | The reference points to a nested property schema, not a named definition. |
| `t1-ref-external-relative` | The external JSON Schema is supplied alongside the OpenAPI document and resolves relative to it. |
| `t1-ref-siblings-31` | Both the referenced schema and the sibling properties apply under the OpenAPI 3.1 dialect. |
| `t1-ref-siblings-30` | In OpenAPI 3.0 this position contains a Reference Object; its sibling properties member is ignored. |
| `t1-tuple-prefix-items` | Both tuple positions admit an object; the tentative item pointer does not distinguish positions. |
| `t1-required-allof` | The required constraint and the property declaration are on different allOf members. |
| `t1-not-required-field` | Any object containing forbidden fails the negated required constraint; kept remains possible. |
| `t1-enum-format-before` | The paired fixture changes enum and format while keeping the wire field name and ID. |
| `t1-enum-format-after` | The paired fixture changes enum and format while keeping the wire field name and ID. |

| `t1-fragment-percent-collision` | The literal names # and %23 identify separate properties. |
| `t1-unicode-normalization` | The visually similar names use composed and decomposed Unicode sequences. |
| `t1-defs-reference` | The reference resolves against the containing OpenAPI document using the absolute JSON Pointer in the fixture. |
| `t1-discriminator-mapping` | The discriminator values map to named alternatives; the wire property kind is shared. |
| `t1-directional-annotations` | OpenAPI 3.0 says opposite-direction transmission SHOULD NOT occur, not MUST NOT; required applies in the indicated direction. |
| `t1-path-item-reference` | Both paths apply the same Path Item under different path keys. |
| `t1-express-implicit-head` | Express 4 GET registration also handles HEAD in the absence of a preceding explicit HEAD registration. |

| `t1-conditional-branches` | Both branches are satisfiable: kind a permits warp, and kind b permits weft. |
| `t1-pattern-exact-property` | The anchored pattern matches exactly bolt; additionalProperties false rejects other names. |
| `t1-property-names-uri` | The two property names contain URI characters requiring distinct fragment representations. |
| `t1-allof-closed-intersection` | Each member prohibits the other member’s property; the empty object is valid but neither named field can occur. |
| `t1-ref-id-base` | The schema $id changes the base to shapes/root.json; leaf.json resolves beside that resource, not beside the OpenAPI file. |
| `t1-relative-server-document-base` | The companion context gives the document retrieval URL; ../api resolves to /catalog/api before the path is appended. |
| `t1-express-array-paths` | One Express 4 registration matches either literal path; POST avoids implicit HEAD handling. |

| `t1-array-zero-maximum` | The only valid array is empty, so the item object cannot occur. |
| `t1-false-property` | A false schema rejects every value of blocked, while open can occur. |
| `t1-oneof-overlap-exclusion` | Every object with marked matches both alternatives and fails oneOf; an object with only plain matches the second. |
| `t1-dependent-schema` | An object containing trigger and bobbin satisfies the dependent schema. |
| `t1-contains-items` | An array containing an object with weave is valid and satisfies contains. |
| `t1-unevaluated-properties` | Both allOf members evaluate their own properties before the outer closure applies. |

Specifications: [OpenAPI 3.1](https://spec.openapis.org/oas/v3.1.0.html), [JSON Schema 2020-12 core](https://json-schema.org/draft/2020-12/json-schema-core), and [JSON Schema 2020-12 validation](https://json-schema.org/draft/2020-12/json-schema-validation).

| `t1-anchor-ref` | The reference uses a JSON Schema anchor rather than a JSON Pointer. |
| `t1-max-properties-zero` | The empty object is valid; grain cannot occur in any valid object. |
| `t1-array-prefix-index-collision` | Both positions may occur together and have separate mark fields; the tentative [] pointer cannot distinguish them. |
| `t1-request-media-same-pointer` | JSON mark and XML mark need distinct identities; only the XML child has an unambiguous current spelling. |
| `t1-external-pointer-escaped` | The external resource is local to the fixture and its property name contains a literal slash. |
| `t1-pattern-excluded-by-property` | Both schemas apply to bolt, and false makes both bolt and its descendant unreachable. |

| Row | Comment |
| --- | --- |
| `s1-express4-constrained-captures` | The listed patterns witness separate capture domains. /fletch/19 and /fletch/wax reach different POST handlers. Express also answers OPTIONS over both domains. [Express 4 route parameters](https://expressjs.com/en/4x/api.html#app.METHOD) [Express 4.21.2 router](https://github.com/expressjs/express/blob/4.21.2/lib/router/index.js) |
| `s1-express4-generated-options` | The automatic OPTIONS response advertises POST and PUT at the same path. [Express 4.21.2 router](https://github.com/expressjs/express/blob/4.21.2/lib/router/index.js) |
| `s1-express4-query-dispatch` | POST /fletch?cut=round uses the wax handler. POST /fletch?cut=square uses the pith handler. Their IDs need to differ. The generated OPTIONS remains unambiguous. [Express route handlers](https://expressjs.com/en/guide/routing/#route-handlers) [Express 4.21.2 router](https://github.com/expressjs/express/blob/4.21.2/lib/router/index.js) |
| `s1-express5-mount-aliases` | The path alternatives belong to the router mount. The child registers only /proof. [Express 5 app.use](https://expressjs.com/en/5x/api.html#app.use) [Express 5 router](https://github.com/pillarjs/router/blob/v2.2.0/index.js) |
| `s1-express5-optional-literal-group` | The registration accepts /fletch/proof and /fletch/draft/proof. The literal pattern preserves that optionality. [Express 5 path syntax](https://expressjs.com/en/guide/migrating-5/#path-syntax) [Express 5 router](https://github.com/pillarjs/router/blob/v2.2.0/index.js) |
| `s1-express5-quoted-parameter` | The hyphen belongs to the capture name. [Express 5 path syntax](https://expressjs.com/en/guide/migrating-5/#path-syntax) [Express 5 router](https://github.com/pillarjs/router/blob/v2.2.0/index.js) |
| `s1-express5-root-inclusive-wildcard` | POST / and POST /fletch/wax both hit the named wildcard registration. The ungrouped named wildcard excludes /. [Express 5 path syntax](https://expressjs.com/en/guide/migrating-5/#path-syntax) [Express 5 router](https://github.com/pillarjs/router/blob/v2.2.0/index.js) |
| `s1-field-control-characters` | The literal tab, newline, and null character name three separate members. [RFC 6901 §6](https://www.rfc-editor.org/rfc/rfc6901.html#section-6) |
| `s1-gin-raw-encoded-slash` | When RawPath is supplied for the escaped slash, Gin searches it before decoding capture values. The two requests reach different handlers. [Gin 1.11.0 request dispatch](https://github.com/gin-gonic/gin/blob/v1.11.0/gin.go#L605-L625) |
| `s1-gin-static-file-methods` | The registration supplies neither method explicitly. [Gin 1.11.0 RouterGroup](https://github.com/gin-gonic/gin/blob/v1.11.0/routergroup.go) |
| `s1-hapi-internal-route` | The fixture includes one ordinary public route as a control. [Hapi internal route option](https://github.com/hapijs/hapi/blob/v21.3.12/API.md#route.options.isInternal) |
| `s1-hapi-optional-mixed-segment` | Both /fletch/wax-/proof and /fletch/wax-ochre/proof reach this route. [Hapi 21.3.12 route API](https://github.com/hapijs/hapi/blob/v21.3.12/API.md#L2070-L2081) |
| `s1-hapi-prefix-on-plugin-root` | Hapi joins a plugin prefix and a child root by using the prefix alone. [Hapi 21.3.12 route construction](https://github.com/hapijs/hapi/blob/v21.3.12/lib/route.js#L33-L36) |
| `s1-oas30-get-request-body` | Paired with the 3.1 row. Only the OpenAPI version changes. [OpenAPI 3.0.3](https://spec.openapis.org/oas/v3.0.3.html) |
| `s1-oas31-204-response-body` | The 204 schema is intentionally diagnostic. The 200 is the body-bearing control. [HTTP Semantics](https://www.rfc-editor.org/rfc/rfc9110.html) |
| `s1-oas31-anyof-impossible-branch` | The first alternative both requires and forbids gate. The second accepts an object containing wax and excludes ghostWax. [JSON Schema Core](https://json-schema.org/draft/2020-12/json-schema-core) |
| `s1-oas31-authority-dispatch` | The companion describes another service. Wax and pith handlers have separate operations and response roots, but the current syntax erases both authorities. [OpenAPI 3.1](https://spec.openapis.org/oas/v3.1.1.html) |
| `s1-oas31-default-success-witness` | The companion synthetic exchange fixes the otherwise unspecified status at 202. The exact 200 still uses its own response object. [OpenAPI 3.1](https://spec.openapis.org/oas/v3.1.1.html) |
| `s1-oas31-dynamic-ref-override` | base.json supplies a fallback anchor. The outer resource overrides it, so only outerWax is declared on the resolved object. [JSON Schema Core](https://json-schema.org/draft/2020-12/json-schema-core) |
| `s1-oas31-enum-object-shape` | Exactly two response objects are admissible. One carries wax and its sheen child while the other carries pith. [JSON Schema Validation](https://json-schema.org/draft/2020-12/json-schema-validation) |
| `s1-oas31-external-path-item-server` | The referring path is /reed. The relative server rev belongs to branch/parts.json. The external document's /outside path contributes no URL to the entry description. [OpenAPI 3.1](https://spec.openapis.org/oas/v3.1.1.html) |
| `s1-oas31-get-request-body` | OpenAPI 3.1 permits the explicitly documented GET body. [OpenAPI 3.1](https://spec.openapis.org/oas/v3.1.1.html) |
| `s1-oas31-head-response-body` | The HEAD schema is intentionally diagnostic. The same schema can describe a GET 200 body. [HTTP Semantics](https://www.rfc-editor.org/rfc/rfc9110.html) |
| `s1-oas31-label-path-parameter` | The two documented values produce /reed/.wax and /reed/.pith. [OpenAPI 3.1 style examples](https://spec.openapis.org/oas/v3.1.1.html#style-examples) |
| `s1-oas31-network-path-server` | The leading double slash introduces the authority in a relative URI reference. Only /amber belongs to the path. [RFC 3986 §4.2](https://www.rfc-editor.org/rfc/rfc3986.html#section-4.2) |
| `s1-oas31-property-names-finite-map` | The map permits only indigo and saffron, and its value schema describes glint beneath each. [JSON Schema Core](https://json-schema.org/draft/2020-12/json-schema-core) |
| `s1-oas31-request-media-case-range` | A request with Content-Type application/json uses the more specific Application/JSON entry. Other media types have no named fields. [HTTP Semantics](https://www.rfc-editor.org/rfc/rfc9110.html) [OpenAPI 3.1](https://spec.openapis.org/oas/v3.1.1.html) |
| `s1-oas31-required-only-name` | An object must contain wax. Its value may be any JSON value. [JSON Schema Validation](https://json-schema.org/draft/2020-12/json-schema-validation) |
| `s1-oas31-response-media-range` | application/json selects cerulean and application/yaml selects umber. The range remains applicable where the exact media type does not match. [OpenAPI 3.1](https://spec.openapis.org/oas/v3.1.1.html) |
| `s1-oas31-tuple-unevaluated-tail` | An array with one wax object is valid. Any second item fails because no other keyword evaluates it. [JSON Schema Core](https://json-schema.org/draft/2020-12/json-schema-core) |
| `s1-oas31-yaml-status-keys` | The quoted YAML scalar 201 is a string key and overrides the 2XX range for that status. [OpenAPI 3.1](https://spec.openapis.org/oas/v3.1.1.html) |
| `s1-swagger-body-parameter-ref-override` | POST inherits SapEnvelope. PUT substitutes PithEnvelope because both references resolve to the same parameter name and location. [Swagger 2.0](https://spec.openapis.org/oas/v2.0.html) |
| `s1-swagger-discriminator-inheritance` | A response with family CopperReed and ridge selects the inheriting schema. [Swagger 2.0](https://spec.openapis.org/oas/v2.0.html) |
| `s1-swagger-xml-wire-name` | The sole advertised representation names the child element clay-cut. [Swagger XML Object](https://spec.openapis.org/oas/v2.0.html#xml-object) |
