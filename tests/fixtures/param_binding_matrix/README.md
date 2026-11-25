# JavaScript parameter-binding matrix

`rows.jsonl` contains one case per line. A suffix splits a plan row into
independent cases. Group C uses the plan's `k` IDs and group D uses its `h`
IDs. Shared JS-family fixtures run in each declared language.

## Row schema

| Field | Meaning |
|---|---|
| `id` | Unique plan ID or plan ID with a sub-case suffix |
| `group` | `control`, `A`, `B`, `C`, `D` or `E` |
| `langs` | Nonempty list of `js`, `ts`, `tsx`, `go` or `py` |
| `files` | Fixture filename to source map. The build helper uses the filename stem and the extension for the current language. Multi-file fixtures with explicit extensions in imports use a single language. |
| `expect` | Nonempty list of assertions |
| `class` | Census metric and shape, or baseline control class |
| `truth` | `correct`, `wrong` or `ambiguous`. `wrong` means the row pins the existing unimported-export resolver defect, including rows that require later binding admission to refuse that resolution. |
| `issue` | Required for `wrong` rows |
| `unexecuted` | Reason the assertion needs a measurement or design decision beyond the fixture APIs. The test prints the ID and skips that row. |

An endpoint is `file:function:path@line:col`. Lines and columns are 1-based.
The column counts characters. The test converts it to a UTF-8 byte offset and
compares the filename, owner, access path, line and both byte offsets. A string
endpoint uses the literal path spelling to determine the expected end. For a
different source spelling, use `{"at":"file:function:path@line:col","text":"…"}`.
Use `"text":""` for a zero-width occurrence. An anonymous owner is
`<anon@L:C>` with its source start line and byte column.

## Assertion kinds

| Kind | Fields and check |
|---|---|
| `def` | `at` identifies exactly one variable Def. |
| `edge` | `from` identifies a Use and `to` identifies a Def or Use. Optional `label` is `exact` or `nameonly/<doubt>`. Optional `multiplicity` checks the number of matching edges, including parallel edges. The default requires at least one. |
| `no_edge` | `from` and optional `to` prohibit the same Use-to-Def or Use-to-Use edge. Without `to`, it prohibits transfers to other functions. Optional `label` limits the prohibition to that label. |
| `no_edge_into` | `to` prohibits incoming Use-to-Def or Use-to-Use transfers from other functions. Optional `label` limits the prohibition to that label. |
| `resolves` | `call` identifies one extracted site. `targets` is the full set of `file:function@start_line` targets, with `confidence` and `route` applied to each target. |
| `drop` | `call` has no targets and the given `reason` from `resolve_call_site_full`. |
| `reaches` | `from` is a prior Def. The occurrence at `to` must appear in the `dfg_forward_reachable` walk. The labeled API exposes visited node IDs so equal file/line/path entries stay distinct. |
| `not_reaches` | Same endpoints and walk as `reaches`, with the target absent. |
| `trace_boundary` | `taint_trace` from the line of the Def at `root` must record the Use at `from` to the Def at `to` as a boundary for that root. The target must be absent from that root's frontier. |
| `counter` | `key` is a dot-separated key in `call_stats`, `return_flow` or `dfg_labels`. `value` is an unsigned integer. An absent key is zero. External census and oracle keys appear only in rows marked `unexecuted`. |

A `call` object has `site` in `file@line:col` form at the start of the call
expression and the extracted `callee` name. Optional `end` is the exclusive
`line:col` position, and optional `owner` is the extracted caller name.

The ambiguous rest cases `b03` and `b09a` assert only the settled offsets.
`e02a` asserts the anonymous resolution and leaves the bare-arrow transfer
for stage D. `h09a`–`h09c` retain the desired edge assertion while stage D
decides the admissible receiver forms and their graph nodes. `e09a` uses a
receiver parameter with an existing single-owner name-search route.

For a baseline run, enable `control` and one feature group in
`src/cpg/param_binding_matrix_tests.rs`. Run the matrix through `cg test --lib
param_binding_matrix -- --nocapture` to display each row's result. The test
reports each row before failing and prints individual language/assertion
failures. Restore `ENABLED` to `&["control"]` afterward.

## Census classes by row

| Row | Group | Census class |
|---|---|---|
| `c01` | control | `control/fixed-slot` |
| `c02` | control | `control/parenthesized-arrow` |
| `c03a` | control | `control/go-spread` |
| `c03b` | control | `control/python-splat` |
| `c04` | control | `control/literal-after-spread` |
| `c05` | control | `control/builtin-drop` |
| `c06` | control | `control/arguments` |
| `c07` | control | `control/unimported-export` |
| `c08` | control | `control/unimported-export` |
| `a01` | A | `M1/unknown-offset` |
| `a02a` | A | `M1/spread-source` |
| `a02b` | A | `M1/known-prefix` |
| `a03a` | A | `M1/unknown-offset` |
| `a03b` | A | `M1/unknown-offset` |
| `a03c` | A | `M1/unknown-offset` |
| `a03d` | A | `M1/unknown-offset` |
| `a04a` | A | `M1/constructor` |
| `a04b` | A | `M1/receiver` |
| `a04c` | A | `M1/optional` |
| `a05a` | A | `M1/chain-identity` |
| `a05b` | A | `M1/chain-identity` |
| `a05c` | A | `M1/chain-identity` |
| `a05d` | A | `M1/call-result` |
| `a05e` | A | `M1/constructor-chain` |
| `a06a` | A | `M1/literal-length` |
| `a06b` | A | `M1/literal-length` |
| `a06c` | A | `M1/literal-length` |
| `a06d` | A | `M1/literal-length` |
| `a06e` | A | `M1/unknown-offset` |
| `a06f` | A | `M1/unknown-offset` |
| `a06g` | A | `M1/literal-length` |
| `a06h` | A | `M1/overload` |
| `a07a` | A | `M1/two-rows-one-site` |
| `a07b` | A | `M1/two-rows-one-site` |
| `a08a` | A | `M1/excluded-call-apply` |
| `a08b` | A | `M1/excluded-call-apply` |
| `a09a` | A | `M1/refusal-reachability` |
| `a09b` | A | `M1/refusal-reachability` |
| `b01a` | B | `M2/rest` |
| `b01b` | B | `M2/arrow-rest` |
| `b01c` | B | `M2/method-rest` |
| `b01d` | B | `M2/rest` |
| `b02a` | B | `M2/rest` |
| `b02b` | B | `M2/rest` |
| `b02c` | B | `M2/rest` |
| `b03` | B | `M2/ambiguous-offset` |
| `b04` | B | `M2/ts-this` |
| `b05a` | B | `M2/rest-pattern` |
| `b05b` | B | `M2/rebound-rest` |
| `b06` | B | `M2/nameonly` |
| `b07` | B | `M2/rest-object-literal` |
| `b08` | B | `M2/refusal-reachability` |
| `b09a` | B | `M2/least-offset` |
| `b09b` | B | `M2/literal-length` |
| `b09c` | B | `M2/literal-length` |
| `b10a` | B | `M2/no-variable-source` |
| `b10b` | B | `M2/no-variable-source` |
| `b10c` | B | `M2/no-variable-source` |
| `b10d` | B | `M2/no-variable-source` |
| `k01a` | C | `M3/bare-arrow` |
| `k01b` | C | `M3/bare-arrow` |
| `k02` | C | `M3/inferred-name` |
| `k03` | C | `M3/inferred-name` |
| `k04` | C | `M3/inferred-name` |
| `k05a` | C | `M3/literal-leaf` |
| `k05b` | C | `M3/rebound-formal` |
| `k06a` | C | `M2/cross-file-free-single` |
| `k06b` | C | `M3/cross-file-free-single` |
| `k06c` | C | `M3/cross-file-free-single` |
| `k07a` | C | `M2+M3/imported` |
| `k07b` | C | `M2+M3/same-file` |
| `k08a` | C | `M3/return-flow` |
| `k08b` | C | `M3/return-flow` |
| `h01a` | D | `M4/array-method` |
| `h01b` | D | `M4/array-method` |
| `h01c` | D | `M4/array-method` |
| `h01d` | D | `M4/array-method` |
| `h01e` | D | `M4/array-method` |
| `h01f` | D | `M4/array-method` |
| `h01g` | D | `M4/array-method` |
| `h01h` | D | `M4/array-method` |
| `h02a` | D | `M4/reducer` |
| `h02b` | D | `M4/reducer` |
| `h03a` | D | `M4/promise` |
| `h03b` | D | `M4/promise` |
| `h04` | D | `M4/named-local` |
| `h05a` | D | `M4/named-imported` |
| `h05b` | D | `M4/shadowed` |
| `h05c` | D | `M4/local-let` |
| `h06a` | D | `M4/callback-role` |
| `h06b` | D | `M4/no-formal` |
| `h06c` | D | `M4/destructured-formal` |
| `h07a` | D | `M4/member-only-formal` |
| `h07b` | D | `M4/mixed-formal` |
| `h07c` | D | `M4/member-receiver` |
| `h07d` | D | `M4/optional-receiver` |
| `h07e` | D | `M4/this-receiver` |
| `h07f` | D | `M4/shared-line` |
| `h07g` | D | `M4/no-receiver-node` |
| `h08` | D | `M4/in-repo-receiver` |
| `h09a` | D | `M4/call-result-receiver` |
| `h09b` | D | `M4/call-result-receiver` |
| `h09c` | D | `M4/call-result-receiver` |
| `h10a` | D | `M4/excluded-method` |
| `h10b` | D | `M4/excluded-method` |
| `h10c` | D | `M4/excluded-method` |
| `h11` | D | `M4/array-method` |
| `h12a` | D | `M4/instance-override` |
| `h12b` | D | `M4/instance-override` |
| `h13` | D | `M4/same-line-exact-call` |
| `h14a` | D | `M4/refusal-reachability` |
| `h14b` | D | `M4/refusal-reachability` |
| `e01` | E | `M5/named-target` |
| `e02a` | E | `M5/anonymous-target` |
| `e02b` | E | `M5/anonymous-target` |
| `e03a` | E | `M5/mixed-targets` |
| `e03b` | E | `M5/mixed-targets` |
| `e04a` | E | `M5/payload-arity` |
| `e04b` | E | `M5/payload-arity` |
| `e05` | E | `M5/fanout` |
| `e06a` | E | `M5/written-formal` |
| `e06b` | E | `M5/nameonly-caller` |
| `e06c` | E | `M5/spread-caller` |
| `e06d` | E | `M5/excluded-call-apply` |
| `e07` | E | `M5/imported-holder` |
| `e08a` | E | `M5/forward-depth` |
| `e08b` | E | `M5/forward-depth` |
| `e09a` | E | `M5/existing-name-search` |
| `e09b` | E | `M5/default-formal` |
| `e10` | E | `M5/engine-boundary` |
| `e11` | E | `M5/call-result` |
| `e12` | E | `M5/cross-file-free-single` |
| `e13` | E | `M5/anonymous-caller` |
| `e14` | E | `M5/same-line-exact-call` |
| `e15` | E | `M5/destructured-slot` |
| `e16a` | E | `M5/refusal-reachability` |
| `e16b` | E | `M5/refusal-reachability` |
