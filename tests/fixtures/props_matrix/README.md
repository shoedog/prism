# JSX props matrix

`rows.jsonl` holds one row per planned case. The test is `cpg::props_matrix_tests::props_matrix_rows`.
Run it with `cargo test --lib props_matrix_rows -- --nocapture`. The checked-in `ENABLED`
constant runs `control` and `S1`. To record a baseline or test another slice, temporarily add its
group to the constant. The runner prints a status for each enabled row and reports all
failing rows together.

## Row fields

| Field | Meaning |
| --- | --- |
| `id` | Unique ID from the lane plan, or a descriptive `c-…` ID for a control. |
| `group` | `control`, `S1` (pattern formals), `S2` (object-literal arguments), or `S3` (JSX attributes). |
| `langs` | File extensions to exercise. JS-family extensions are `js`, `ts`, `tsx`, and `jsx`. Non-JS controls use `py` and `go`. |
| `files` | Map from file stem to source. For multiple independent variants, prefix each stem with `case/`, and set `case` on each assertion. Each case builds its own graph and has its own counters. An explicit suffix such as `generic-tag/m.tsx` makes the case run only for that extension, which must be in `langs`. |
| `expect` | Assertions for the graph, statistics, and trace. An omitted `case` selects the files without a case prefix. |
| `class` | The syntax, propagation rule, or limitation covered by the row. |
| `truth` | Program semantics independent of the analysis. `CORRECT-binding` denotes a genuine binding or flow, `NO-BINDING` its absence, and `MIXED` a row with both. `RUNTIME-BINDING` denotes a real binding whose static representation is outside a slice's rules, `RUNTIME-DEPENDENT` a binding dependent on runtime conditions, `NO-STATIC-BINDING` the absence of a statically established binding, and `NO-SIBLING-FLOW` independent fields whose graph may be conservatively connected. |
| `base` | Seeded expectation for the implementation base, `red` or `green`. This is metadata. The runner reports the observed result independently. |

## Assertions

Graph variable names use `function:path@line(def)` and `function:path@line(use)`.
Graph lines are one-based. Byte spans are zero-based, half-open ranges within the
fixture file. Different graph nodes can share the same spelling.

| `kind` | Fields and evaluation |
| --- | --- |
| `def` | `node` must have a Def. Optional `bytes` requires one match at that span. Optional `count` checks the total with that spelling. |
| `no_def` | No Def matches `node`. |
| `edge` | A directed DataFlow edge connects `from` and `to`. Optional `label`, `from_bytes`, and `to_bytes` filter the match. Optional `count` checks the exact number of matching edges. |
| `no_edge` | No DataFlow edge matches `from`, `to`, and any supplied label or byte filters. Optional `absent_ok: true` permits either spelling to be absent. |
| `no_edge_into` | No DataFlow edge crosses into `to` from another owner. `to` can be a function name or a node spelling. Optional `absent_ok: true` permits it to be absent. |
| `path`, `no_path` | A nonempty directed chain of DataFlow edges must connect, or must fail to connect, `from` and `to`. A `path` can specify `via` to require a two-edge chain through the same concrete intermediate node. Optional `absent_ok: true` on `no_path` permits either spelling to be absent. |
| `counter` | `key` in the serialized `dfg_props` statistics equals `count`. An absent map or key reads as zero. |
| `s1_counter` | `key` must be registered in `data_flow::s1_evidence::COUNTER_KEYS`. The case and grammar must supply an observed integer equal to `count`, including when `count` is zero. |
| `s1_ledger` | Select the `file` stem, which may be omitted for a single-file case. Count all attempts with the exact `binding_bytes` and `operand_bytes` before checking each matching `disposition` and optional `binding` verdict. |
| `trace_reaches`, `trace_misses` | Seed the normal line-based trace with `seed`. The frontier must include, or must exclude, the specified `node` at `target`. |
| `trace_boundary` | The trace records a boundary from a `seed` root ending at `node` at `target`, and the node stays outside the frontier. Optional `boundary_kind` is `cross_function` by default and also accepts `self_function_param`. Optional `from` (node spelling) and `from_bytes` filter the source of the same boundary. |

The trace `seed` and `target` are objects with a `file` stem (relative to the case)
and a one-based `line`. Both must resolve to graph variables. The supported labels
are `exact`, `killed`, `same_line`, `cfg_incomplete`, `alias_unstable`, and
`call_name_only`. Unknown assertion fields are rejected. Negative edge and path
assertions require all named endpoint spellings to exist unless `absent_ok` is true.

The S1 assertions use an uncached run of the file producer for each case and grammar. The ledger
identifies the source by revision, file, grammar and digest. Each attempt has one terminal outcome.
Signature effect sites and cap decisions have their own counts. To require exactly one edge with
a particular label, pair an unfiltered `edge` with `count` of one and a separate labeled `edge`.
