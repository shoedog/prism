# Provenance matrix

Each line in `rows.jsonl` is one case. Fixtures are small synthetic snippets that
the test parses as source. The primary file's extension selects the language
directory in `cases/<ext>/<id>/`.

| Field | Meaning |
| --- | --- |
| `id` | Unique row ID and fixture directory name. |
| `group` | Slice that enables the row. See the table below. |
| `class` | A census cause or seed family listed below. |
| `files` | Relative paths in the fixture directory. The first file is the primary diff file. |
| `diff` | Map from fixture path to changed 1-based lines. These are the files loaded into the CPG. Other listed files provide repository-local context for the repo-aware entry point when it lands. |
| `want` | Product expectations as `{file, line, var, origin}`. `origin` is an `Origin::name()` or `none` to assert absence. |
| `exact_lines` | Map from fixture path to changed lines on which the row specifies every finding. |
| `truth` | Same shape as `want`. Records the actual origin of the value. `none` covers a constant, a non-value occurrence, or a value with no reportable source. |
| `tentative` | Optional `true` for behaviour awaiting a stage D decision. Enabled rows always check `want` and `exact_lines`. |
| `accepted_loss` | `null` at stage T. Stage D can record a reason that `want` differs from `truth`. The test still checks `want` and reports a match to `truth` for adjudication. |
| `source` | Shape provenance, one of `probe`, `corpus`, `grammar`, or `enumerator:<seat>`. |

The depth boundary fixtures use two steps as the provisional boundary for
propagation and handle hops. Their adjacent rows expose that choice to stage D.

## Groups

| Group | When enabled |
| --- | --- |
| `control` | Stage T and later slices |
| `structural` | S1 |
| `py` | S3 |
| `js` | S4, including TypeScript and TSX |
| `go` | S5 |

The runner in `tests/algo/novel/provenance_matrix_test.rs` selects groups through
`ENABLED_GROUPS` and reports the ID and group of each failing row. The matrix
checks findings. Trace reasons, evidence grade, and target hints are checks for
the later trace and projection tests.

## Classes

The census causes are `non_variable_token`, `other_function_definition`,
`line_inheritance`, `request_object_itself`, `signature_substring`,
`trigger_in_name`, `killed_or_later_definition`, `one_sided_boundary`,
`precedence`, `trigger_in_comment_or_string`, `constant_masking`, and `other`.

The seed families are `pred` (predecessors), `occ` (occurrence roles), `pos`
(positions), `val` (callable value semantics), `xfer` (transfers), `rq` (request
position), `decoy` (source-like spellings), `scoping` (lexical scope), `hops`
(handle transitions and helpers), `unproven` (unproven bindings), `join` (origin
joins), `pattern` (pattern languages and parser edges), and `origin` (sources).

## Notes

- `control` rows pass on the base build and must stay green; a row that already passes on base with `want == truth` and no
  `tentative` flag is a control.
- `tentative` rows state the behaviour the design describes before stage D; `want` may differ from `truth` only on a
  `tentative` row or one with `accepted_loss`. The test checks every row's invariants (same sites in `want` and `truth`,
  fixtures present, lines in range, the tentative rule) whether or not its group is enabled.
- A listed variable with origin `none` and an omitted variable are equivalent for the runner; rows list a token as `none`
  only when the token is the point of the row (a callee, module or member name that must not be reported).
- Improvements on `accepted_loss` rows print `provenance-matrix IMPROVEMENT ...` (visible with `-- --nocapture`);
  `PROVENANCE_MATRIX_STRICT=1` makes them fail the test.
- Rows that parse sibling repository files are run through `slice(&ctx, &diff)` without a repository root until slice S2
  adds `slice_in_repo`.
