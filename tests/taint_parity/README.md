# Taint parity matrix

Each JSON line records one source occurrence and one sink operand in a fixture.
One taxonomy test validates every loaded row. The other three check walk,
default review, and return-flow review for enabled groups and rows with
`current` cells. Run them with
`cargo test --test algo_taxonomy taint_parity_test::`.

The harness reads `rows.jsonl` (T-1), `rows-t2a.jsonl`, `rows-t2b.jsonl`, and
`rows-t2c.jsonl` in that order. IDs must be unique across all loaded files.
`PARITY_ROWS=rows.jsonl,rows-t2b.jsonl` replaces the file list at run time, in
the order given. Use comma-separated filenames relative to this directory.
`PARITY_ENABLED=control,binding` replaces the enabled groups at run time.
By default `control`, `guard` and `reasons` are enabled. For a row with `current`, the
engine tests select that cell while its group is disabled or any slice in
`current.until` is absent from `SHIPPED_SLICES`. Otherwise they select the
target `expect` of an enabled row. The runtime group override does not alter
the shipped slice list. Pending producer checks apply to their listed levels
after cell selection. An engine test reports an all-pending group and fails
an enabled group with no cells and no pending reason at that level.

The ten `option_b` rows retain `truth: flows` and their distal `BoundaryExited`
targets. Their active `current.until: ["option_b"]` cells assert the parent's
`NotReached` at the marked operand and on the line. On these rows,
`current.reasons.walk` lists live refusals to verify on the marked root's hop
ledger. Option B after I-walk supplies distal sink attribution.

## Row schema

| Field | Meaning |
| --- | --- |
| `id`, `group`, `class` | Unique row ID, implementation group, and origin of the fixture. |
| `lang`, `fixture`, `axes` | Source language, directory, and the shape of the flow or attempted crossing. |
| `src` | Relative `file`, one-based `line`, and `bytes` of the occurrence on the line marked `@SRC`. |
| `site` | `{bytes, slot}` in the source file. A numbered slot is zero-based. The string `return` marks a return statement. Use `null` when the proposal has no slot the schema can represent, such as a receiver or call result. |
| `sink` | Relative `file`, one-based `line` marked `@SINK`, `operand` byte span, and sorted `nodes` spans of all intersecting Variable nodes, including duplicates. |
| `walk_comparable` | True when the source line provides the same single occurrence to both clients and review has no implicit sources. |
| `expect` | Required cells after the row's group ships, or explicit `null` for a `tbd` row awaiting Stage D. Put the observed `walk`, `review`, and `trace_review` values in `note` when `expect` is null. |
| `expect.walk` | `{reaches, source, terminal, descent_depth}`. Witness fields are null if no witness exists. Depth counts descents on the displayed chain. |
| `expect.review`, `expect.trace_review` | Confidence of the `taint_sink` finding on the marked line with default and return-flow review, respectively. |
| `expect.catalog` | Multiset of `{file, line, rule, operand, verdict}` decisions for the one selected explicit source occurrence. `[]` expects zero decisions in that scope. A flat rule has a null operand. |
| `reasons` | Null or lists keyed by `walk`, `review`, `trace_review`, and `catalog`. Explain disagreements in flow or finding presence. |
| `current` | Optional `{until, expect, reasons}` for the active intermediate cell. `until` lists distinct prerequisite slices. `expect` and `reasons` use the target schema. Record the measured build and mechanism in `note`. |
| `knobs` | Null or `{edges, depth}`. Edge ceilings are `exact` and `nameonly`. Depth is a nonnegative crossing budget. |
| `truth`, `known_wrong` | `flows` or `no_flow` for the selected operand, and null or the issue or group responsible for a known wrong answer. |
| `pending` | Null or `{levels, issue}`. Use a nonempty list of distinct levels from `walk`, `review`, `trace_review`, `catalog`, and `reasons`, and name the missing producer in `issue`. An engine test prints the dependency and skips only its listed cell. |
| `note` | Observed base behavior, including commands that were unavailable. |

Byte spans are zero-based, half-open offsets into the whole UTF-8 file. A
zero-width Variable node intersects the operand when it sits at the operand's
start. A line query can return several roots and terminals. The harness selects
the recorded source from the per-source results and calls
`Reachability::aggregate` over terminals that intersect the operand. It breaks
witness ties by terminal byte span. For `l01`, the source is the second `t` in
`log(t); g(t)`. The two `m01` rows pin different operands on one marked line.

The review diff lists every line of every fixture file.
`taint_from_diff=false` stops diff-derived roots and still allows framework
and IPC sources. The pre-cleansing trace from review is compared to navigation
for rows marked `walk_comparable`. Every control with identical single explicit
roots must set that flag. Review can mint extra explicit roots from call
occurrences on the source line.

Catalog expectations cover one explicit root even when the legacy command can
report decisions from several roots. When a proposed hit requires a separate
root, a file scan, or a decision across roots, record `[]` if this root has no
decisions and use `Unsupported per root` in `note` to name the dependency.
Record an unavailable catalog producer in `pending` for the catalog level.

For unresolved names or receivers, `truth` is `flows` when some feasible
binding of the unresolved name reaches the selected operand. Placeholder
receivers keep sample calls callable without choosing the binding under test.

The reason check compares flow or finding presence to `truth`. Only a reason
on a participating level whose result differs from `truth` can explain a
disagreement. Catalog participates for catalog rows or when catalog decisions
are listed, using the reached decisions on the marked sink line.

## Vocabularies

- Groups are `control`, `binding`, `same_line`, `spelling`, `knobs`, `catalog`,
  `walk`, `guard`, `reasons`, `option_b`, and `tbd`. `ENABLED` can contain any listed group
  except `tbd`. Stage D sets the remaining requirements for `tbd` rows.
- Classes are `research`, `seed`, `design_review`, and `second_review`.
  Use `research` for Sol and Astra enumeration rows. Proposal classes describe
  code shape, while the row class records provenance.
- Languages are `javascript`, `python`, `go`, `typescript`, and `tsx`.
  Use `.ts` with `typescript` for plain TypeScript and `.tsx` with `tsx` for
  JSX fixtures using the TSX grammar. Keep a historical `.tsx` path when the
  `.ts` form changes the recorded baseline, and document the difference in the
  corresponding `SOURCES-t2*.md` map.
- Mechanisms are `intra`, `argument`, `binding`, `return`, `capture`,
  `callback`, `field`, `sanitizer`, `dispatch`, and `catalog`. Layouts are
  `same_line`, `separate_lines`, and `multiline`.
- Resolution is `none`, `exact`, `nameonly`, `multi_exact`, or `unresolved`.
  The depth axis records the nominal crossing count or the requested budget.
- Sanitizer placement is `none`, `on_path`, `off_path`, or `other_category`.
  Direction is `intra`, `descent`, `ascent`, or `both`. Distance is
  `same_function`, `same_file`, or `cross_file`.
- Walk verdicts are `Reached`, `NotReached`, `BoundaryExited`, `Sanitized`, and
  `SeedError`. Catalog verdicts use the first four. Review grades are
  `exact`, `nameonly`, `unlabeled`, and `none`. Two reported findings can
  have different confidence because trace-mode crossings carry unlabeled evidence.
- Boundary reasons are `target_not_binding`, `no_matching_site`,
  `ambiguous_site`, `not_exact`, `unproven_slot`, `depth_cap`,
  `policy_ceiling`, `self_param`, and `self_return`.
- Other reasons are `return_not_entered_site`, `no_graph_edge`,
  `shortcut_suppressed`, `cfg_pruned`, `sanitizer_model`,
  `operand_granularity`, and `unmodelled`.
- Review-side reasons are `review_no_return_flow`, `review_no_recovered_def_use`,
  `review_cfg_filter`, `review_context_policy`, `uncertified_flow`, and `matcher_admission`.

The harness asserts active boundary reasons on the marked pair in
`per_sink[].sources[]` and pins comparison
reasons with paired fixtures. Catalog and knob runtime assertions arrive with I-8 and I-5. Both review modes use the default
crossing policy even on rows with explicit walk knobs. Update the matching
expectations when an enabled group's required behavior changes.
