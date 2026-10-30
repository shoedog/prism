# Independent blind labelling: SecBench.js prism tracing sample (static-analysis accounting)

You are an independent labeller. This is defensive static-analysis research: you only read source code and prism output. Do **not** run any package code or test.

## Input
`/Users/wesleyjinks/prism-evidence/meas/secbench/blind-sample.jsonl`: 40 rows. Each row has `class`, `entry`, `source` (the exported function and parameter that the benchmark's test input feeds), `sink` (the target call `file:line`) and `prism_output_excerpt` (raw prism query outputs).

Package source for each entry is at `/Users/wesleyjinks/prism-evidence/inputs/secbench-pkgs/<class>/<entry>/src/package/`.

## FORBIDDEN, to keep this blind
Do not open:
- anything under `/Users/wesleyjinks/prism-evidence/meas/secbench/` except `blind-sample.jsonl` and this brief (in particular `repair-r1/`, `blind-reference.jsonl`, `final/`, `conversions*` and `entries.jsonl`);
- any file under `/Users/wesleyjinks/code/prism-secbench/docs/`;
- `eval/secbench/*.json`;
- any review in `/Users/wesleyjinks/prism-evidence/meas/review/`.

You may read prism's Rust source (`/Users/wesleyjinks/code/prism-secbench/src/`) to understand its behaviour.

## Label each row with three fields

**`gt`:**
- `available` if the source parameter and the sink are unambiguously identifiable in the package source;
- otherwise `gt_unavailable`.

**`outcome`**, judged from `prism_output_excerpt` against the source and sink:
- `traced`: prism's output reaches the sink line from the source parameter;
- `reached_function_only`: it reaches the sink's function but not the sink line or value;
- `partial`: the path starts but breaks before the sink function;
- `not_reached`: no flow from the source;
- `prism_error`: the prism query errored;
- `gt_unavailable`: when `gt` is `gt_unavailable`.

**`mechanism`**: when the outcome is not `traced`, the FIRST construct in the package source where the data flow from source to sink breaks for prism. Use exactly one of:
- `A-loop`;
- `B-arguments` (legacy `arguments` object);
- `B-default`;
- `B-destructure`;
- `B-rest-spread`;
- `B-plain-argument` (ordinary argument-to-parameter passing);
- `C-member` (`obj.a.b` member/property flow, or a parameter used only via members);
- `D-cjs` (`require` / `module.exports` hop);
- `E-admission` (file not indexed, e.g. under `dist/` or `build/`);
- `H-callback/promise/event` (callback or function-argument registration, closures, promises);
- `timeout`;
- `unresolved` (you cannot determine the mechanism);
- `unresolved-source` (the source binding itself has no prism definition and the cause is unclear).

Use `none` when traced, and `unavailable_ground_truth` when `gt` is `gt_unavailable`.

## Output
Write JSONL to `/Users/wesleyjinks/prism-evidence/meas/secbench/blind-labels-sonnet.jsonl`: one line per row, in the same order, `{"class","entry","gt","outcome","mechanism"}`, plus an optional `"note"`. Label all 40 rows.
