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
  and malformed `explode` on an otherwise supported declaration. It counts
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
- `tests/identity.rs` includes `schema/mod.rs` through `#[path]` until P1b
  publishes `index_schema`.
