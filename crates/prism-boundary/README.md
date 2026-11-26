# Prism Boundary

The `prism-boundary` package owns the `prism_boundary` library and the
`prism-boundary` binary. It is an opt-in workspace member.

P1a implements typed operation and field IDs, literal route declaration
projections, and internal operation and status inventory for Swagger 2.0 and
OpenAPI 3.0 and 3.1 JSON. The reader consumes admitted bytes and explicit
reference dependencies. Convert YAML at the caller boundary. The binary
provides `--help` and `--version`.

Wire-field discovery, source extraction, binding and impact documents, and
schema diffs arrive in later slices. Adapters will add the `prism` dependency
when they use `prism::api` and will list any non-facade calls here.

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
- OpenAPI message fields use a single unambiguous `application/json` representation with a readable schema.
- Swagger message fields use an effective media list containing JSON or admitted omission evidence for that operation and message root. Request evidence also establishes Content-Type selection.
- Schema objects use the governing dialect. OpenAPI 3.1 accepts boolean schemas, the implicit and explicit 3.1 base dialect, and the supported JSON Schema 2020-12 dialect.
- Named object properties contribute fields when capacity and required-value witnesses support their presence. Readable directional flags refine requiredness where the dialect defines it.
- Homogeneous array items contribute fields where an item can occur. The parent item-type slot summarizes a single readable item type.
- Schema-valued object remainders contribute map-value fields when the object has capacity and the remainder is feasible.
- Inventory paths retain typed property, item and map edges through depth 16. The reader checks object feasibility at edge 17 before recording a depth cut.
- Recognized types, readable format labels, exact scalar enum members and established field requiredness contribute signature facts. An empty enum declaration can be retained as an observation where the edge survives capacity checks.
- A known effective nullable type set in OpenAPI 3.1 or same-object OpenAPI 3.0 `nullable: true` signs as `["known", <sorted type set>]` when it has multiple members. A single type keeps the scalar encoding, such as `["known","null"]`.
- Supported scalar, object and array bounds contribute feasibility evidence where the reader can establish a witness or contradiction.
- Nonempty `allOf` contributes the conjunction of readable arms within the composition budget.
- Nonempty `anyOf` contributes feasible arms within the 128-alternative budget.
- Nonempty `oneOf` contributes feasible arms within the same budget when effective types or required scalar tags establish disjointness.
- Union slots sign facts shared across every feasible arm. The `branch_dependent` tag represents differences in a slot or its presence across arms.

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
- `reference_unresolved` stops unreadable or missing schema or message targets. A stopped schema reference preserves edge requiredness and signs unreadable slots `unknown`.
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
