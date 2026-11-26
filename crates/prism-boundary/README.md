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

- Swagger 2 and OpenAPI 3.0/3.1 JSON with admitted entry bytes and static pointer dependencies.
- Dialect-eligible request bodies and explicit success keys with exact-over-range status selection.
- Exact `application/json`, effective Swagger media lists, or admitted root-scoped Swagger omission proof.
- Named property and required edges, homogeneous items, and schema-valued map remainders with typed pointers.
- Feasible scalar, object, array, `allOf`, `anyOf`, and provably disjoint `oneOf` transitions within 16 edges.
- Comparable fields use a supported schema dialect. Sibling comparison requires a feasible witness for each required value.
- Union facts reflect agreement across every feasible arm within the 128-alternative budget. The signature uses `branch_dependent` for facts that vary by arm.
- JSON admits exact numeric tokens and preserves object token kinds. Repeated members count as `duplicate_member` at the governing field or message boundary.
- An empty enum retains its declaration. A 3.1 type array or a 3.0 `nullable` declaration with an effective null and non-null type uses `branch_dependent` in the signature's type slot.
- Stopped references preserve edge requiredness and use `unknown` in the type, format, enum, and item type slots.
- The reader checks object feasibility on the seventeenth edge and counts a depth cut when a feasible edge reaches the inventory boundary.
- Swagger request omission proof establishes Content-Type selection. Explicit success keys use the canonical JSON string encoder in status records.

The public `index_schema` inventory keeps comparable fields separate from
structural observations, with typed pointers, declaration sites and counted
message limits. `definitions_of` and `operations_reaching` return sorted
relationships and their coverage. The source byte SHA-256 and canonical
inventory hash are independent.

Known limits include fallback success bodies, non-JSON representations,
unproved Swagger media, nested schema resources and anchors, recurring
references, tuples and evaluated tails, arbitrary assertion vocabularies,
overlapping `oneOf` arms, and unknown scalar or container feasibility. The
`known_limits` rows also document these comparison boundaries.

- Changing a `const` discriminant in a supported `oneOf` or `anyOf` can leave `content_hash` unchanged. The signature design is open. Options include a singleton enum encoding, a counted limit for union tags, and retaining the current encoding with disclosure.
- Authored request-body `required: true` and `required: false` share the same comparison output and the standing `body_presence_unchecked` limit. Requiredness within the selected body is tracked separately.
- The identity census exercises the internal operation reader. The `wire` and public golden targets exercise `index_schema`.

Other inputs have counted partial or withheld coverage at the affected scope. The
`wire` target reads the pinned ID matrix and the separate P1b enumeration. The
`known_limits` test pairs conservative dispositions with supported neighbors.
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
