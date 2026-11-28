# Prism Boundary

**Status: PROVISIONAL.** The admission claim of this crate is scoped to what the
real-pair replay measures; see [P1d admission claim](#p1d-admission-claim-provisional).

The `prism-boundary` package owns the `prism_boundary` library and the
`prism-boundary` binary. It is an opt-in workspace member.

P1a implements typed operation and field IDs, literal route declaration
projections, and internal operation and status inventory for Swagger 2.0 and
OpenAPI 3.0 and 3.1 JSON. The reader consumes admitted bytes and explicit
reference dependencies. Convert YAML at the caller boundary. The binary
provides `--help` and `--version`.

The library indexes wire fields, compares admitted schema revisions, and
publishes bindings and impact contracts. Source extractors and fresh matching
land with the adapters. The first adapter (P2 producer bindings, below) added the
`prism` dependency and lists its non-facade calls there.

```sh
cargo test -p prism-boundary
python3 crates/prism-boundary/tests/fixtures/check_matrices.py
```

The identity target pairs literal source projections with the pinned
operation ID expectations. It reads the pinned schema bytes and compares
operation ID and response declaration inventories. The structural matrix
checker runs once in CI and checks both censuses and fixture references.

## Server defaults

An empty operation or Path Item `servers` array inherits the next enclosing
level. The document default is `/`. The Python prototype uses `/` immediately
when the selected list is empty. The Rust reader diagnoses a nonempty list
that produces zero deployments.

## Known limits

- An inline OpenAPI path parameter qualifies with a `schema` object or a
  3.1 boolean schema, `content` absent, and `explode` omitted or boolean.
  Default or explicit `simple` spells `{}`, including for arrays.
  A determinably scalar `label` spells `.{}` and requires a reference-free
  schema in the operation's document, readable parameter lists, and the
  implicit schema dialect without schema resolution keywords. Readable lists
  with no matching parameter still give `{}`.
- Every other parameter object is withheld. The diagnostics inventory counts
  `path_style_unsupported` for content-only parameters, unsupported styles,
  and malformed `explode` on an otherwise supported declaration. Malformed
  content-only values use the same `path_style_unsupported` family.
  Path parameter `required` is unchecked, including false, missing and non-boolean values. It counts
  `path_parameter_style_unchecked` for parameters with both `schema` and
  `content` or neither, invalid schema forms, and undeterminable effective
  declarations or label types.
- An operation id identifies the path template and the capture positions with
  their serialization form. It makes no claim about which values a capture
  admits, the empty string included. Value admissibility belongs to consumer
  matching in P3, which must not assume a capture is non-empty.
- Compared with `c8708313`, **0 of 1,165 pinned operations** and **0 of 199
  public-pair operations** were withheld or changed.
- Route projections record `raw_path`, `mount` and `api` without consulting them.
- Policy values from `route-table.toml` carry no spec hash.
- A literal `{}` in a schema path prints the same way as a capture and can
  trigger a false `duplicate_operation` diagnostic.
- `ascii_fold` lowercases the hex digits in percent escapes.

## P1b wire accept-list

P1b identifies operations and the wire fields a client can depend on.
Runtime or semantic equality of schemas, including value-level validation,
is outside the model. `index_schema` records comparable fields, structural
observations, declaration sites and counted limits. `definitions_of` and
`operations_reaching` report sorted relationships and coverage. The source
byte SHA-256 and the canonical inventory hash are independent.

- Operation IDs cover readable literal methods and path templates in admitted Swagger 2.0 and OpenAPI 3.0/3.1 JSON with unambiguous declarations.
- Static JSON Pointer references contribute readable target facts when the target has a unique admitted blob and the reference chain is in scope.
- OpenAPI request fields come from a readable JSON request body on a method eligible in that dialect.
- Swagger request fields come from one effective body parameter with JSON media evidence and a readable schema.
- Response fields come from an explicit success key that owns at least one body-eligible status, with exact status keys taking precedence over ranges in OpenAPI.
- Response and request-body reference terminals must match the role's key and value-type description. Each role allows `x-` extensions.
  - OpenAPI 3.x Response requires a string `description`. Optional `headers`, `content`, and `links` are objects.
  - Swagger 2.0 Response requires a string `description`. Optional `headers` and `examples` are objects. An optional `schema` is an object or boolean at terminal admission.
  - OpenAPI Request Body requires an object `content`. Optional `description` is a string and optional `required` is a boolean.
  - A terminal outside this description withholds projection with `reference_unresolved`. Other malformed-document shapes, including violations in unchecked roles, are outside the model.
- OpenAPI message fields use a single unambiguous `application/json` representation with a readable schema.
- Swagger message fields use an effective media list containing JSON or admitted omission evidence for that operation and message root. Request evidence also establishes Content-Type selection.
- Schema objects use the governing dialect. OpenAPI 3.1 accepts boolean schemas, the implicit and explicit 3.1 base dialect, and the supported JSON Schema 2020-12 dialect.
- Named object properties contribute fields when capacity and required-value witnesses support their presence. Readable directional flags refine requiredness where the dialect defines it.
- Homogeneous array items contribute descendant fields where an item can occur. A named array field signs its known item type or type set in the item slot.
- The item type slot signs the surviving type set for a multi-type item schema. The reader drops types it proves impossible, including strings with contradictory length bounds, even when feasibility of a surviving type is unknown. If an item-level stop blocks pruning, the slot signs the readable declared set. The item limit remains counted in both cases. [Item arm](src/schema/wire.rs) · [Feasibility rules](src/schema/wire/constraints.rs) · [Fold-5 hash test](tests/hashes.rs)
- Schema-valued object remainders contribute map-value fields when the object has capacity and the remainder is feasible.
- Inventory paths retain typed property, item and map edges through depth 16. The reader checks object feasibility at edge 17 before recording a depth cut.
- Recognized types, readable format labels, exact scalar enum members and established field requiredness contribute signature facts. An empty enum declaration can be retained as an observation where the edge survives capacity checks.
- Known effective type sets from OpenAPI 3.1 and Swagger 2.0 type arrays or same-object OpenAPI 3.0 `nullable: true` sign as `["known", <sorted type set>]` in both type slots. A single member keeps the scalar encoding, such as `["known","null"]`.
- Supported scalar, object and array bounds contribute feasibility evidence where the reader can establish a witness or contradiction.
- Nonempty `allOf` contributes the conjunction of readable arms within the composition budget.
- Nonempty `anyOf` contributes feasible arms within the 128-alternative budget.
- Nonempty `oneOf` contributes feasible arms within the same budget when effective types or required scalar tags establish disjointness.
- Union slots sign facts shared across every feasible arm. The `branch_dependent` tag represents differences in a slot or its presence across arms.

### Wire-field signature and value-level limits

The signed field record includes the field name and its presence in the
inventory through the field ID, which binds the operation, request or response
root, and typed path. The signature tuple records five facts about the field.

- Effective field type or type set
- Readable format label
- Established requiredness
- Effective exact scalar enum members
- Immediate homogeneous list-item type or type set

Each slot records a known fact, missing fact, branch dependence, or unknown
fact. In a type slot, `Missing` means the schema was declared without a type
keyword and leaves the type unconstrained. `Known` → `Missing` records a loss
of typing evidence. The value-level constraints outside the signature are `minimum`,
`maximum`, `exclusiveMinimum`, `exclusiveMaximum`, `multipleOf`, `minLength`,
`maxLength`, `pattern`, `minItems`, `maxItems`, `uniqueItems`, `minProperties`,
`maxProperties`, and `const`.
`enum` and `format` are unsigned on unnamed list items and map values, and
where a named field's slot cannot carry the effective value, such as
branch-specific values. The reader can use constraints to establish
feasibility and prune impossible types. A change that only narrows or widens
the admitted values is outside this slice's claim.

`fold6-item-value-bounds` pins the same signature and hash for narrow and
wide items on the request field `/x`. The wide schema admits `{"x":[1]}`
and the narrow schema caps its items at `0.9`. Both sign the item type
`number` after feasibility pruning. `control-fold6-item-value-bounds` pins
the different signature and hash when the wide schema's item type changes
to `integer`.

A temporary reader probe collected keywords after reference and `allOf` expansion for each public golden field ID, attributing unnamed item and map-value keywords to the enclosing named field and counting only unsigned `enum` and `format` payloads.
The same probe counted each (fixture file, field ID) once per keyword across 510 OpenAPI 3.x JSON documents in `p1b_rows` and `ids_matrix`, including comparable and observed fields and the fold-6 pair and control.
The tables retain the original keyword counts and distinct-field subtotals. `minProperties` and `maxProperties` were excluded from that probe and have not been recounted.

**Public Swagger 2.0 golden**

| Unsigned keyword | Fields |
| --- | ---: |
| `minimum` | 19 |
| `maximum` | 10 |
| `exclusiveMinimum` | 0 |
| `exclusiveMaximum` | 0 |
| `multipleOf` | 0 |
| `minLength` | 28 |
| `maxLength` | 152 |
| `pattern` | 0 |
| `minItems` | 0 |
| `maxItems` | 0 |
| `uniqueItems` | 0 |
| `const` | 0 |
| `enum` | 0 |
| `format` | 0 |
| **Distinct fields carrying at least one** | **171** |
| Total fields | 2,421 |

**OpenAPI 3.x enumeration fixtures**

| Unsigned keyword | Fields |
| --- | ---: |
| `minimum` | 5 |
| `maximum` | 4 |
| `exclusiveMinimum` | 1 |
| `exclusiveMaximum` | 1 |
| `multipleOf` | 10 |
| `minLength` | 3 |
| `maxLength` | 3 |
| `pattern` | 10 |
| `minItems` | 0 |
| `maxItems` | 2 |
| `uniqueItems` | 0 |
| `const` | 16 |
| `enum` | 13 |
| `format` | 4 |
| **Distinct fields carrying at least one** | **57** |
| Total fields | 600 |

### Withheld or pinned limits

Counted limits identify a scope where comparison loses authority or
coverage. A readable neighboring field or root may still be comparable.

- `array_uniqueness_unchecked` withholds uniqueness-dependent feasibility because item witnesses do not establish distinctness.
- `body_form_conflict` withholds Swagger request fields when body and form parameters compete.
- `body_parameters_unreadable` withholds Swagger request selection when effective parameters are unreadable.
- `body_presence_unchecked` counts authored request-body presence separately. Authored true and false share a comparison output and required fields inside the body still have their own requiredness.
- `composition_budget` caps alternative expansion and reference/composition steps at 128.
- `composition_unreadable` withholds malformed or empty compositions because the arms cannot establish a readable domain.
- `const_signature_unchecked` withholds an unrepresented scalar `const` outside the supported union or enum context.
- `declaration_unreadable` stops a Path Item or parameter reference chain at a non-object declaration.
- `depth_cut` caps the field inventory at 16 wire edges and counts a descendant beyond the cap.
- `direction_flags_unchecked` withholds malformed, misplaced or dialect-ineligible direction flags.
- `direction_policy_unchecked` withholds required 3.1 direction combinations whose effect is application-dependent.
- `discriminator_inheritance` counts discriminator-driven inheritance outside the supported composition proof.
- `duplicate_member` withholds affected declarations when a repeated JSON member removes unique authority.
- `duplicate_operation` withholds message comparison when multiple declarations project to the same operation ID.
- `enum_unreadable` withholds malformed enums and dialect-ineligible empty enums.
- `fallback_success` withholds bodies selected through `default` because success ownership is implicit.
- `indexed_tuple` withholds tuple and evaluated-tail traversal because the supported item edge is homogeneous.
- `json_media_ambiguous` withholds a representation when several media declarations compete for JSON.
- `json_media_range_only` withholds selection based only on a wildcard media range.
- `json_media_variant_unchecked` withholds parameterized or otherwise unsupported JSON media variants.
- `media_encoding_unchecked` limits fields affected by authored encoding rules beyond the schema shape.
- `media_schema_missing` withholds fields when the selected media lacks a schema.
- `media_schema_unreadable` withholds fields when the media entry or schema has an unreadable form.
- `media_unreadable` withholds malformed content maps or effective media lists.
- `media_unverified` withholds comparison for an omitted Swagger media declaration without admitted root evidence.
- `multiple_request_bodies` withholds selection among competing Swagger body parameters.
- `non_json_media` limits non-JSON representations and form bodies because the wire inventory concerns JSON.
- `nullable_composition_unchecked` withholds nullable compositions without an established interpretation or disjointness proof.
- `nullable_scope_unchecked` withholds 3.0 nullability without a same-object explicit type.
- `operation_unreadable` withholds a non-object operation declaration.
- `parameter_unreadable` counts an unreadable parameter name or location before path serialization.
- `parameters_unreadable` counts a parameter list whose shape prevents reading declarations.
- `path_item_field_unchecked` withholds an overlapping field across a Path Item reference chain.
- `path_parameter_style_unchecked` withholds capture spelling when its effective declaration or label type is undetermined.
- `path_style_unsupported` withholds path captures with serialization outside the supported simple and scalar label forms.
- `paths_unreadable` counts a malformed paths collection because operations cannot be enumerated from it.
- `recursive` stops recurring schema or message reference expansion.
- `recursive_reference` stops recurring Path Item or parameter declaration references.
- `ref_siblings_unchecked` withholds Swagger schema references with meaningful siblings because the sibling semantics are unchecked.
- `reference_blob_ambiguous` withholds a target with competing admitted blobs.
- `reference_blob_unavailable` counts a schema or message reference whose target blob is unavailable.
- `reference_blob_unchecked` stops a Path Item or parameter reference whose target blob lacks admission.
- `reference_pointer_unchecked` stops declaration references with unparseable pointers or missing targets.
- `reference_scope` stops traversal through resource boundaries, anchors and dynamic or recursive reference keywords.
- `reference_siblings_unchecked` withholds OpenAPI 3.1 message references with siblings beyond summary or description.
- `reference_unresolved` stops unreadable or missing targets and targets outside the referenced message's object shape. A stopped schema reference preserves edge requiredness and signs unreadable slots `unknown`.
- `reference_uri_unchecked` counts an unparseable declaration reference address.
- `request_body_unreadable` withholds unreadable request body or content declarations.
- `required_unreadable` withholds requiredness from malformed lists under the governing dialect.
- `response_key_ambiguous` withholds a repeated explicit response key.
- `response_link_unchecked` counts response links because link semantics fall outside the wire field projection.
- `response_unreadable` withholds a non-object response declaration.
- `responses_unreadable` counts an unreadable or dialect-required missing response collection.
- `retrieval_base_unchecked` stops relative references and server URLs whose retrieval base is unavailable.
- `scalar_feasibility_unchecked` withholds facts that require a scalar witness the reader cannot establish.
- `schema_dialect_unchecked` withholds schema facts under an unsupported dialect or vocabulary.
- `schema_extension_unchecked` withholds assertions that may depend on an unknown extension.
- `schema_form_unchecked` withholds schema forms outside the dialect or a recognized schema slot.
- `schema_keyword_unreadable` withholds recognized keywords with unreadable values.
- `server_deployments_unchecked` counts a nonempty server list that produces zero deployments.
- `server_list_unchecked` counts a malformed server list.
- `server_url_unchecked` counts missing or unparseable server URLs.
- `server_variable_unchecked` counts template variables with unchecked choices.
- `unsupported_schema_keyword` withholds unsupported assertions and compositions without an adequate disjointness proof.

The matrix pins the outputs below beside supported controls. The eleven
fold 2 cases require uncommon or contradictory shapes. The pinned public
Swagger contains none of them, so excluding each would withhold zero of
its 199 operations and zero of its 2,421 fields.
The price comes from scanning the pinned public Swagger 2.0 for each shape. The price for OpenAPI 3.x producers is unmeasured.

- `fold1-union-const` pins equal hashes for edited union `const` discriminants. Union consts, enum tags and per-arm type, format, enum or item-type changes can leave `content_hash` unchanged because the union signature records shared facts and payload-free branch dependence.
- `fold1-body-presence` pins authored true and false request presence under the standing counted limit because P1b compares the fields inside the selected body.
- `fold1-census-scope` pins the scope distinction between the internal identity census and the public `index_schema` wire projection.
- KNOWN-LIMIT `fold2-ref-sibling-one-of` retains checked fields when disjointness depends on ignored 3.0 reference siblings. The accepted form uses effective inline tags.
- KNOWN-LIMIT `fold2-inferred-object-enum` retains the inferred object slot despite a numeric enum. The accepted form supplies a compatible explicit type.
- KNOWN-LIMIT `fold2-boolean-dialect-root` retains checked infeasibility for a boolean under an unsupported dialect. The accepted form uses a 3.1 dialect that admits boolean schemas.
- KNOWN-LIMIT `fold2-saturated-empty-enum` omits an empty-enum declaration behind a saturated object. The accepted form has spare property capacity for the declaration observation.
- KNOWN-LIMIT `fold2-depth-cut-array` retains the sixteenth field and `depth_cut` for a required, impossible array at edge 17. The accepted control permits an empty array beyond the depth boundary.
- KNOWN-LIMIT `fold2-dynamic-ref` signs unreadable slots as missing under `reference_scope`. The accepted form uses an admitted static pointer.
- KNOWN-LIMIT `fold2-media-proof-tokens` admits a Swagger request catalog with a duplicate selection flag or a wrapped number token. The accepted proof has one boolean flag and numeric span tokens.
- KNOWN-LIMIT `fold2-unrelated-union-const` retains an equal hash after a sibling const edit beside `anyOf: [true]`. Removing the unrelated union exposes the counted const limit.
- KNOWN-LIMIT `fold2-any-of-arm-only-type` retains an equal hash after a type edit in a single feasible arm. Removing that field changes the projected inventory and hash.
- KNOWN-LIMIT `fold2-one-of-enum-tag` retains an equal hash after changing one of two disjoint enum tags. Removing an arm-only field changes the projected inventory and hash.
- KNOWN-LIMIT `fold2-items-dialect` retains the parent item type despite an unsupported items dialect. The accepted form supplies the item schema under the supported default dialect.
- KNOWN-LIMIT Facts below a list-item or map-value edge with no named field can leave the hash unchanged. This includes item enum, item format, arrays of arrays, and maps of arrays. `fold3-root-scalar-slots` pins equal hashes for scalar map-value and root scalar-array item type edits. `fold4-item-enum-slot` and `fold4-item-format-slot` pin checked coverage, zero limits, and equal hashes for item enum widening and item format changes. The controls sign the corresponding facts when a named field has a slot for them.
- KNOWN-LIMIT `kl_p2_1_01` (KL1, P2-1, `tests/producer_fold2.rs`) a keyless gap from a read file (a malformed or block-comment `@Router` directive, an ambiguous base) withholds uniqueness (`candidate`, `competitor_unread`) for every operation of the document, although the visible directive may concern one other operation (shapes of rows `kl_p2_1_01` to `_04`: block comments, a split directive, spelling variants, invalid path spellings). The accepted control is the valid edge without that sibling, or a sibling whose gap is keyed to another operation. Incidence on the public pair (Apache Answer pinned checkout: 199 directives, 446 files, 0 entry gaps, root base path, single server): 0. CONSTRUCTED.
- KNOWN-LIMIT `kl_p2_1_05`, `kl_p2_1_06`, `kl_p2_1_07`, `kl_p2_1_09` (KL2, P2-1) a gap key is built by joining the schema's base to the directive's path. Row 05: a detached directive that already spells the base (`/api/x` under `basePath /api`) is keyed `/api/api/x` and does not block `/api/x`; that edge keeps `Bound`, which is correct under swaggo's base-relative `@Router` semantics (a detached comment generates nothing), so row 05 is the one case that keeps a claim the crate's own attached-directive rule would not. Row 06: an attached `/north/x` under servers `/north` and `/south` blocks only `/north/x`. Row 07: a prefix learned from an operation-level server override is applied to every operation (a spurious key `/fixed/y`). Row 09 (the row-07 consequence): the spurious key withholds a real edge, so with a detached `/x` and an operation `/z` overriding its servers with `/fixed`, an independently annotated `GET /fixed/x` drops from `Bound` to `Candidate`. The accepted controls are the authored path under a base path, a single server prefix, and a schema without the override. Incidence on the public pair: 0 (root base path, single server). CONSTRUCTED.
- KNOWN-LIMIT `kl_p2_1_08` (KL3, P2-1) the `annotation_deployments_unprojected` region comes from any server-scope diagnostic of the document, so a hit on an operation that overrides the servers with a finite list still records it. The accepted control is a schema with no server diagnostic. Incidence on the public pair: 0 (no server diagnostic). CONSTRUCTED.

Open questions for the design seat

- How should signatures represent edited union const discriminants?
- Should the signature tuple gain a slot for facts on unnamed list items and map values?

The `wire` target reads the pinned ID matrix and the P1b enumeration.
The enumeration materializes M3 as separate codec and producer-source rows
and T6 as separate response-collection and duplicate-key rows.

Stage the pinned public schema in `PRISM_BOUNDARY_PUBLIC_SWAGGER`. The byte
gate uses `PRISM_BOUNDARY_PUBLIC_IDS_TSV` or the default path in the pin manifest
and checks both SHA-256 pins before comparing the ordered output and final LF.
For local runs with missing staged input, `PRISM_BOUNDARY_PUBLIC_PENDING_OK=1`
reports the gate as PENDING and continues.
The test is `#[ignore]`d so that CI, which has no staged inputs, reports it as ignored rather than
failing or passing silently; run it with `cargo test -p prism-boundary --test public_ids -- --ignored`.
With the inputs missing an explicit `--ignored` run fails unless the override above is set.

## P1c structural diff

`diff_schemas(&before, &after)` compares admitted schema inventories. It
delegates to `diff_boundary(&before, &after)`, which also accepts revision-local
typed producer inventories and an explicit `DiffBasis`. Both sides need a
common API, operation authority, and logical scope. An API-wide scope can be
restricted to a typed path prefix. A contradictory source slot or an
unestablished comparison basis returns `DiffError`.

- Whole-operation additions and removals carry their side-local status and
  field detail. A typed collision can establish operation presence. A status
  claim from a collision requires agreement across all declarations.
- Explicit success-key changes are independent of body eligibility. Unmatched
  status rows carry their own field detail, including a retained range's
  fields when it loses or regains its last body-eligible code.
- A unique outgoing and incoming JSON response root can pair by typed relative
  field path, retaining both original IDs. Unknown candidates, overlapping
  status ownership, and incomparable media leave a scoped limit. Exact keys
  own eligible responses before `2XX`, then `default`. Schema-bearing Swagger
  responses with unproved media remain possible body candidates even when
  their field inventories are empty. Each affected root carries standing
  `media_unverified` coverage, and named field absence remains unknown at
  retained roots. A replacement that cannot be paired carries a pairing limit.
- Checked named fields compare presence, type, format, requiredness, scalar
  enum, and immediate list-item type. A readable empty enum retains its
  declaration even when the field has no feasible value. Member deltas
  require two readable lists. Checked empty or absent bodies can establish
  field presence changes at retained roots. A type or item-type change is
  claimed only when both sides have no reader limit at the slot's typed
  pointer. The diff withholds every other type or item-type comparison and
  counts a side-local `Type` or `ItemType` limit. Independently comparable
  slots still report edits.
- An authored request excluded by its dialect carries a side-local
  `request_dialect_excluded` coverage limit, including when both revisions
  have no projected request message.
- Typed producer facts compare route declarations, validation, resolved
  handler references, and corresponding endpoint body digests. Moving a
  site or changing an alias to the same endpoint leaves those facts equal.
- Producer-only operation presence requires declaration sites bound to the
  inventory tree. Under a path prefix, an unavailable declaration path
  leaves a method-wide `producer_operation_unscoped` gap for producer-backed
  claims. Checked schema facts remain comparable. At API scope, a
  bound `Checked(None)` declaration counts as operation presence, and a
  transition between `Checked(None)` and `Checked(Some(...))` emits
  `RouteDeclarationChanged`. Changing only a capture name also emits
  `RouteDeclarationChanged` at the same operation ID.

Each side carries its own counted limits and evidence. A missing schema,
dialect-excluded request root, unproved media selection, partial subtree,
or unreadable shape slot limits the claims that require it. Checked sibling
fields and independent producer facts can still compare. The limit ledger
persists across equal hashes. Formatting, definition renames, and equivalent
projections leave the structural changes empty. Value-only narrowing or
widening, unsigned union `const` edits, and unsigned item or map facts stay
silent when the signed projection is unchanged.

Known limit `fold1-server-document-gap` records that a malformed document-level
server list gaps every method, including operations with their own server
lists. This only costs recall by withholding operation absence or suppressing
otherwise supported status and field edits.

Depth cuts, unsupported assertions, unproved scalar feasibility, and stopped
references still bound reader coverage. A whole-message refusal or an opaque
ancestor may leave descendants without a signed slot. The enclosing coverage
or field-presence limit records those gaps.

The following P1c rows use `subset_limits`. They assert the listed limits
and leave extra limits unasserted.

- `c1`, `c2`, `c4`, `c5`, `c6`, `c8`, `d1`, `d1-common-status`,
  `d1-withheld-inventory`, `d2`, `d2-checked-absence`, `d2-literal-capture`,
  `definition-rename`, `e6`, `evidence-withheld`, `f4-dialect`, `o2`, `o4`.
- `control-ambiguous-status-root-replacement`, `control-range-status-root-overlap`,
  `control-retained-route-handler-edit`, `p1`, `p2`, `p2-checked-absence`, `p3`,
  `p4`, `p5`, `p5-partial`, `p6`, `p6-alias`, `p6-inline-reference`, `p6-withheld`.
- `r10`, `r6`, `r6-ambiguous`, `r6-proof-withheld`, `r6-wildcard`, `r8`, `r9`,
  `r9-retained-default`, `r9-unknown-peer`, `s6`, `s7`, `s7-repeated-key`,
  `s7-repeated-key-added`, `s7-repeated-map`, `u2`, `u3-unknown`.

## P1d document contracts

`BindingsDocument` freezes one producer or consumer snapshot. It carries the
snapshot and schema provenance, raw observations, expected files, and extraction
coverage. `ImpactDocument` freezes a review with slot assignments, side-local
changes, comparison-owned match maps, and saved-binding status. The bundled
Draft 2020-12 schemas in `schemas/` use version `"0.1"`. Register both schemas
by their `$id` to resolve the impact schema's shared records. The Rust
`from_json` entry points check the wire contract and refuse duplicate members.
`ImpactDocument::validate` checks references, slots, and comparison ownership
on decoded records.

Admission accepts commit and computed-tree snapshots. Producer authorities are
schema, route table, registration and annotation. The consumer authority is
calls. The schema reader projects JSON Swagger 2.0 and OpenAPI 3.0/3.1.
Blob, inventory and media receipts bind a named tree. Config, mapping and
source receipts can bind a named tree or independent cutoff evidence. An
external reference can use an independent cutoff blob receipt with a retrieval
URI and repository/tree/path/blob identity. Derived
records carry a version and explicit input dependencies. Byte-backed schema and
media receipts require SHA-256 over the raw bytes. Live ref, historical pin, historical snapshot, ref history,
explicit pin and first-parent date-rule receipts bind selection to the requested
time. Computed trees require merge evidence. Historical computed trees also
require independent evidence of the exact tree by cutoff.

Admission counts each dependent claim once per refusal code and retains the
record evidence. Identical receipts coalesce. Conflicts, cycles, missing
dependencies and unsupported kinds withhold their dependent claims. Tracked
dirtiness stays attached as checkout context. The schema reader uses the
admitted named-tree blob and explicitly linked references.

Coverage stores each basis/side/scope/pointer/claim/reason once. Each limit
retains its origin and can link to multiple changed or unchecked claims.
Keyless and unlinked limits remain in the ledger. Saved scans and membership
negatives require the relevant inventory and coverage. Unmeasured totals are
null and carry a reason. Repository reads, fresh matching and rendering
belong to later slices. The binary exposes `--help` and `--version`.


The schema tests use Python with `jsonschema`. Set
`PRISM_BOUNDARY_SCHEMA_PYTHON` to that interpreter when it differs from
`python3`.

## P1d admission claim (PROVISIONAL)

This crate is PROVISIONAL. The admission claim is certified only for the request
shape that the real-pair replay exercises, and for nothing else.

**Certified shape.** Admission is certified for a historical review (`historical_as_of`) with report time at or after the cutoff, whose subject head is the service after input at the cutoff; two inputs of one API and repository, each a commit snapshot with one named-tree blob receipt that has no dependencies, whose commit, tree, path, blob id and bytes the caller has checked against one another in Git (admission checks each receipt's declared repository, tree, nonempty path and blob, and SHA-256 over the supplied bytes, not that the commit's tree holds that blob at that path or that the blob id names those bytes); two receipt-backed schema claims; no consumer side, no computed tree, no merge, no held alias, no selection pin, no media, config or reference receipts, and no frozen consumer records. On the 37 usable revision pairs of the public pair this shape admitted 74 of 74 claims, with no refusal of any kind. A read-only code census of the settlement paths (code reading, not executed, about plus or minus 3) leaves in this shape nine census rows, none classified material and one uncertain.

`ImpactDocument::validate()` is accepted but not certified in any shape, this one included: the 37 replay documents all passed, which shows nothing about refusals, and known limits 14 and 15 show frozen checks that pass while an input is unchecked. The library builds no document inputs, so every document's inputs are caller-built.

**Accepted but not certified.** Every other review mode and request shape
(live, version comparison, other historical shapes, joint reports, computed
trees, merges, media decisions, saved status, bindings, membership and every
frozen-document check on a hand-edited or foreign document) is accepted by the
code but NOT CERTIFIED. A claim marked admitted there can rest on a dependency
that was not itself admitted. The census found about 105 such cases in 38
settlement paths and no reviewer calls the list complete (backlog counts: issue
#1002; related: #960).
The census was read from code and not executed. Almost every case needs a state
the replay never builds (a refused or unchecked input, a pin, a merge, a consumer
side), so the replay shows nothing about incidence there. For the 12 cases that
depend on schema contents rather than on the request, incidence on the public
pair is not measured.

**Where the status is carried.** Neither document has a document-level admission
status. `ImpactDocument` has `warnings`, which hold unchecked claims (status
`unchecked`, tied to a claim), and `coverage`, which holds limits keyed to a
comparison, an input and a claim kind; `BindingsDocument` has neither. Adding a document-level status is a format change, so
it is not made here and documents produced outside the certified shape do not
carry the status themselves. The status is carried by this README and by the
report slice, which shows the standing limits both as one note beside each
change it blocks and as a count in the appendix (owner decision, 2026-11-27).

**Fix.** A redesign of the settlement path is its own slice and lands before P4.
Until then the known-limit rows below pin the cases reviewers found, beside
controls, as expected-until-redesign.

## P1d supported forms and known limits

The supported source of document claims is this library's admitted projection.
Read schema indexes through `SchemaAdmission::index`, use those indexes for
bindings and diffs, and keep each attachment tied to the same admitted entry
bytes and dependency closure. Supported live and historical comparisons use
the recorded subject snapshots and a mode-appropriate selection receipt for
each held or cross-repository counterpart relative to one subject head.
A joint report defaults to the producer after slot when it supplies no subject
head. A frozen historical review records the subject commit at cutoff. If the
effective after input is a computed tree, the merge receipt binds the separate
topic head and independent cutoff evidence covers the exact after-tree. A version comparison
records its selection time and a pin or first-parent date-rule receipt for
each selected input in `change.selection.receipts`. Branch-based receipts must
match the frozen input branch. The singular `change.selection.input_id` and
`receipt` fields can record one input's proof.
The design is that a claim is admitted only when every required dependency in its closure is admitted
(certified only in the shape above; known limits 12 to 15 show where the code does not yet enforce it),
including the separately declared topic head for a historical computed after-tree.
The dependency edges are derived: a claim depends on the settled selection claim of every
input it reads. In a historical review that includes the subject (topic) head, because the
cutoff and the merge head come from it; live and version reviews take their cutoff from the
request and do not read the topic head, so a failed topic-head selection does not refuse
their claims (the narrowing was forced by `fold2_version_admission_needs_proof_and_clock`
and the D14 and A36/A38 document rows, which pin independent sibling claims). The frozen cutoff
takes subject After candidates only from slots of the matching role and depends on every input
holding the selected merge receipt. `ClaimStatus` is built only at the gate in
`revision/admission/gate.rs`.

`ImpactDocument::validate()` attempts these internal checks on frozen records. It is not certified for any of them in any shape, and known limits 14 and 15 show where it does not yet enforce them.

1. Input, dependency, slot, reference and selection IDs resolve within the
   document. Recorded snapshot, role, cutoff and receipt facts agree where
   required by those slots.
2. Supplied comparison maps belong to their slots and cover their recorded
   client occurrence inventory. Each reported usage has the applicable match
   links. Limit and warning links resolve to the recorded claims.
3. Recorded status and presence agree with the selected side, required
   evidence references and local status invariants.
4. Metric counts fit the portable integer range. A known total is at least
   the observed count and has a null reason. An unknown total has a reason.
5. The impact DTO conforms to the closed wire schema. Use the matching
   `from_json` entry point to check the original JSON, including duplicate
   members and unknown keys, before validating the typed impact document.

The checks apply equally to hand-edited or foreign documents, with the same exclusion. Authenticating
source bytes, re-deriving changes and comparisons, and recomputing saved scans
require the original inputs. Use `saved_binding_status` with the
saved and selected inventories derived from their admitted dependencies for
saved-status claims. Use `unobserved_elements` with admitted bindings and
complete evidence for every applicable query region for membership claims.
Each complete region must cover the API or the queried operation.
Use the original scan and query region for these checks, even when a frozen
document already records a negative.

Count each refused or withheld claim under its refusal reason, including claims
withheld at the accept-list boundary. At ingress, count a decoder error as one
envelope refusal. The twelve [known-limit rows](tests/fixtures/p1d_rows/fixtures/known-limits.json)
record where the current APIs can accept or refuse an unsupported input and
pair each case with a supported control. Rows 12 to 15 pin the four round-6
findings (live exemption, historical media decision, duplicate merge holders and
an unchecked Before input in the frozen cutoff) and are expected until the
redesign, which deletes them. The standalone Draft 2020-12 schema
checks count bounds and nullability. The typed validator checks count ordering.

## P2 producer bindings

**Scope of this section.** It describes the swaggo annotation adapter (P2-1). Struct-field bindings,
the registration reader, drift and stale comparison and the `bind` subcommand are later P2 slices;
`ProducerAdapter::GoStructField` exists but `bind_producer` refuses it with
`ProducerError::NotImplemented`, and the default options select the annotation adapter only.
**PROVISIONAL inheritance.** Everything here is built on the provisional admission above and inherits its scope:
it is certified for nothing beyond what the P1d admission claim certifies, and a bindings document produced here
is not an admission claim of its own.

**Entry point.** `bind_producer(&ProducerInputs, &SchemaIndex, &ProducerOptions) -> Result<BindingsDocument, ProducerError>`.
`ProducerInputs` is a `DocumentInput` (producer role, authority `annotation`, API scope, a commit or computed-tree
snapshot), the expected file inventory (`path`, blob id) and the files read from the named tree (`tree`, `path`,
blob id, bytes). The core reads no filesystem and no Git. A `NavigationSession` wrapper is not part of this slice.
The schema index is admitted for a document only when it is the schema the document declares (path, blob, repo
and tree, or the named-tree entry when the document carries no schema evidence), has the schema authority, covers
the same scope region as the document, and, for an explicit file receipt, agrees with the receipt on the SHA-256 of
the bytes, on the content hash and on the info version (each compared whenever the receipt declares a value, an
empty string included), and with the document's own snapshot on the revision, commit date and subject (a commit
receipt must name the snapshot's revision, and its date and subject, when declared, must equal the snapshot's;
a computed-tree receipt must carry none of the three); otherwise the files are `partial` (`schema_not_admitted`)
and nothing binds. One function owns that list (`schema_receipt_agrees`, `src/model/receipt.rs`): the P1d input
validator calls it without an index and this admission calls it with one, and it destructures the schema origin and
evidence exhaustively, so a new receipt field cannot be added without being classified. `format` is classified as
location, not identity (the wire contract pins it to `json`). The named-tree fallback declares none of the digest
fields and keeps its own contract.

**Input contract (P2-1).** P2-1 binds inputs that the lane's own acquisition code produced from a pinned tree. A
caller-built input whose declared fields disagree with one another or with the index is refused where the shared
check sees it, and is otherwise outside the contract, under the PROVISIONAL admission scope P1d states (see
[P1d admission claim](#p1d-admission-claim-provisional), unchanged).
`ProducerError` covers only caller mistakes (no adapter, an unimplemented adapter, a wrong role, authority, scope or
snapshot kind); every input-data problem becomes a coverage record instead.

**Annotation grammar (pinned, Swagger-oriented swaggo form).** A comment block is the maximal run of `//` line
comments, each alone on its line (Go whitespace before it is fine, a trailing comment is not), whose last line ends
on the row directly above a `func` or method declaration with a readable name and an error-free header (everything
but the body). A bare `//` stays in the run; a block comment ends it. The canonical directive line is `//`
(exactly two slashes), optional blanks, the literal `@Router` (case-sensitive), blanks, a path, blanks,
`[method]`, optional blanks. The blank before `[` is required: the path is the run up to the next blank, as in
the prototype's `(\S+)\s+\[` pattern, so `@Router /x[get]` has no bracket after its path and is recorded
`malformed` (`malformed_directive`, the path kept in `raw_path`), never split into `/x` and GET and never guessed
into a binding. `tests/producer_fold2.rs` pins it. The path is an absolute literal: it starts with `/` and holds no `?` or `#`. The method
is ASCII letters, one of GET POST PUT DELETE PATCH HEAD OPTIONS compared case-insensitively; the record keeps the
raw spelling (`original_method`, `raw_method`). A line comment that names `@Router` in any other shape (other case,
`///`, trailing text, no bracket, a list, `*`, a split directive) is `malformed` with reason `malformed_directive`,
`invalid_path` or `unsupported_method`. A block comment that mentions `@Router` is `unsupported_form`
(`block_comment`). A canonical directive that is not in an attached block is `unattached` (`trailing_comment`,
`inside_function_body` or `not_attached_to_function`). A `@Router` inside a string is data. None of these is
guessed into a binding; each is a recorded gap. Adjacent `@Param`, `@Success` and similar lines are not read: no
status, media or field claim comes from this adapter.

**Address basis.** An annotation `METHOD path` selects every schema operation with a declaration whose method equals
it (case-insensitively) and whose **authored** schema path has the same typed path as the annotation path
(`schema_path(path, "", [])`, including capture names and constraints). There is no slash collapsing (`/x//y/` and
`/x/y` differ, a trailing slash is significant), no case folding of the path, no colon-to-capture conversion
(`/x/:id` is a literal colon path) and no base-path stripping or appending: the schema's own server base is joined
once by the schema reader, so `@Router /x` under `basePath /api` binds `op:GET /api/x` and `@Router /api/x` is not
repaired. Every server projection of a matching declaration is bound. One annotation can bind several operations.

**Outcome per directive (`annotations[]`).** `bound`, `candidate` (the same operation annotated on two or more
distinct declarations in the admitted input; both edges carry the evidence note `competing_handlers`, and no
unique-binding credit is given; the same grade, with the note `competitor_unread`, withholds uniqueness when an
expected file's text was never read, or an entry gap is keyless or names the operation, since either could hide a
competitor. A read `parse_error` file counts only through its keyed gaps. The observed site is always kept), `annotation_without_operation` (a complete operation inventory has no match;
there is no `schema_stale_against_code` here, it needs registrations), `not_checked` (the schema index is not
admitted for this input: `schema_not_admitted`; or the match is unprovable: `typed_path_differs` when only the
printed path agrees, `annotation_path_includes_base`, `schema_operations_incomplete` when an unprojected operation
could be the target), `unattached`, `malformed`, `unsupported_form`. A default or foreign `SchemaIndex` (no source, a
different API, tree, commit, path, blob, authority or scope than the document declares, or a declared digest or
revision that contradicts the index) is never read as zero bindings: every file region becomes `partial` with
reason `schema_not_admitted`.

**Records.** A `Binding` per (operation, declaration) edge: `sites` holds the annotation line (path, blob, physical
one-based line, half-open byte span of the admitted bytes, trailing `\r` excluded), `handler` the declaration
(package, name, receiver as written and receiver type name, declaration site and span). The schema declaration is not
copied into `sites`. Edge notes in `evidence`: `generated_file` (a `// Code generated ... DO NOT EDIT.` line before
the package clause; bindings are kept), `bodyless_declaration`, `competing_handlers`, `competitor_unread`. Multiple identical annotations
on one declaration are one edge with several sites. Edges are sorted by operation, file and declaration offset, so
the document does not depend on input order. `//line` directives are ignored: locations are physical.

**Coverage.** Each expected file gets one `file_status` region: `complete`; `partial` (`parse_error`, or
`schema_not_admitted`); `unreadable` (`invalid_utf8`, `parse_failed`); `unknown` (`file_not_supplied`,
`duplicate_file_input`, `tree_mismatch`, `blob_mismatch`: a supplied file whose tree or blob id does not match the
snapshot and inventory is refused for that file only, and its text is never read); `skipped`
(`not_in_expected_inventory`, for a supplied file the caller did not list). A file with parse errors stays `partial`
with an empty key list, which blocks every dependent negative (the membership check treats an unaccounted region with
no keys as covering nothing). Two entries of one path in the expected inventory with different blobs are `unknown`
(`duplicate_expected_entry`) whatever the input order and bind nothing; identical duplicates coalesce. Each gap
directive adds an `entries` region carrying the blob and the directive span. `annotation_unattached` and
`annotation_not_checked` are keyed by the projected operation ids (the ids the directive would name under every
server and base path projection of the schema), or keyless when those cannot be listed (an incomplete operation
domain, nothing to learn a projection from, an unadmitted schema); `annotation_path_includes_base` keeps the
projected id it spells. `annotation_malformed` and `annotation_unsupported_form` have no key. A hit on a known
projection also adds one keyless `annotation_deployments_unprojected` region per document (at the first hit) when
the schema reports server diagnostics (a server variable with a default but no finite enum, an unreadable server list): other deployments may
exist that are not projected. That region does not withhold uniqueness, since it hides no competing handler. A
malformed `@Router /x []` has `raw_method: null` (the wire schema forbids an empty string; `raw` keeps the text). A file with a broken body but an intact header keeps its
attachment and is `partial`; an error between the comment and the declaration confirms no attachment. The caller
chooses the Go file population (this slice does not skip `_test.go`, vendored or build-tagged files); an API-wide
negative needs that population to be the one it claims.

**Document extensions (all optional on the wire, so P1d documents decode and serialize unchanged).**
`Binding.grade` (`bound` or `candidate`), `Binding.handler`, `Binding.evidence[]`, and a top-level
`annotations[]` of `AnnotationRecord` (raw text, raw method and path, disposition, reason, handler, selected
operation ids, notes). `schemas/boundary-bindings.schema.json` gained the `handler` and `annotation` definitions and
these properties; `tests/producer_inputs.rs` pins their closed values and that a P1d document still decodes.
`BindingsDocument::with_schema_inventory` was not changed: called on a document with the annotation authority it
returns `InvalidDocument` (D09), because the schema index is a schema-authority input and the document is not;
`tests/producer_fold1.rs` pins the refusal. A schema-authority document with matching schema provenance can still
use it.

**Non-facade `prism` use.** None of `prism::api` is used. The adapter calls `prism::ast::ParsedFile::parse(path,
source, prism::languages::Language::Go)` and reads its public `tree` and `parse_error_count` fields, and walks the
tree with the `tree_sitter` crate (a direct dependency pinned to the version the core uses). A parser facade can
replace these when the core gains one.

**Public-pair result.** The pinned Apache Answer checkout (6744295f, 446 non-test Go files) binds 199 of 199
schema operations (gate 196/199), 199 `bound` directives on 198 declarations, 0 candidates, 0
`annotation_without_operation`, 0 parse-error files. This is the annotation adapter on the public pair only; the
private pair has no annotations and is not used. Run it with
`PRISM_BOUNDARY_PUBLIC_SWAGGER=<swagger.json> PRISM_BOUNDARY_PUBLIC_CORPUS=<checkout>
cargo test -p prism-boundary --test producer_public -- --ignored --nocapture`.

**Design notes.** (a) `with_schema_inventory` keeps its refusal on an annotation-authority document. Schema
inventory and annotation edges are distinct authorities. If P4 needs schema facts per bound operation, add a method
that only fills rows that already exist and never creates one. Membership treats a missing row as Absent, so P4 must
not feed annotation documents to membership unchanged (an unannotated schema operation would read as absent).
(b) A `bound` edge is unique only when its competitors were accounted for: an unread expected file or a gap that
could hide one yields `candidate` with `competitor_unread` (see Outcome per directive). (c) Whether the producer fields
move to a versioned producer payload is decided at P2-5, when the `bind` CLI first publishes documents.
