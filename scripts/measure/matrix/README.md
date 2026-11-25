# Stage T truth matrix

Each line of `rows.jsonl` is one JSON object with six fields.

| Field | Meaning |
| --- | --- |
| `id` | Unique case ID, using `run`, `cmp`, `join`, `oracle`, `review`, or `gate` followed by a hyphen and three digits |
| `group` | `run`, `comparator`, `join`, `oracle`, `review`, or `gate` |
| `input` | `case` names the directory under `cases/`. File selectors are relative to that directory. Other fields select options. `inventory` lists numbered inventory disagreements. |
| `required` | One sentence describing the inputs and the observed behavior or open alternatives |
| `truth` | Expected results for the dimensions the row pins. An `undetermined` label records what remains open for Stage D. The other truth fields still apply. |
| `source` | `probe` retains raw bytes from a named probe, `product` requires a guarded baseline capture, and `synthetic` is adjudicated from the case facts. |

For probe cases, `input.probe` identifies the raw source. Some cases use a
synthetic counterfactual alongside raw evidence. `derived_from` identifies
inspiration for synthetic inputs. A product case names its authored source file
or directory in `input.fixture` and its unedited output files in `input.raw`.
Product captures require baseline version `9d1a7d06f653`. The matrix currently has zero product rows.

| Group | Case inputs | Truth |
| --- | --- | --- |
| Run | `setup.json` describes the command, corpus, fake binary and optional environment. P5 also supplies `probe.edges.jsonl`. | `exit`, `binary_ran`, `receipt_status` (null if absent), or the check named by `input.operation` |
| Comparator | `base.edges.jsonl`, `head.edges.jsonl`, `input.key_mode` (`bytes`, `line`, or `default`), and a `classifier.py` when named. | `exit`, `changes`, `unattributed`, `collisions`, and certificate counts where pinned. Statistics comparisons pin `deltas`. |
| Join | Comparator inputs plus `head.trace` and the trace parser in `classifier.py`. | Comparator truth plus `records` (`attached`, `stray`, `ambiguous`, `unmatched`) and `class` on each change |
| Oracle | `repo/cases.ts`, `input.jsonl`, and `gen.py` to rebuild input from UTF-8 source. Call cases use call-site records and, for a diff, `base.sites.jsonl`. `probe.jsonl` preserves original records where supplied. | `verdicts`, each with `key`, `verdict`, and `why`. Admission and call-delta cases also pin the applicable outcomes. |
| Review | `base.json`, `head.json`, and `input.arms` selecting single or multi for both arms. | `presence_changes`, `payload_changes`, `ambiguous_bags`, `rule_transitions`, or the check named by `input.operation` |
| Gate | `setup.json` describes the gate, prediction contents, reused receipt, and any mutation or simulated result. | `exit` and `reason` |

Run recipes use `tracked`, `tracked_ignore`, `ignored`, and `untracked`
for the initial corpus and `second_commit` or `tracked_edits` for subsequent
changes. `initial-commit` refers to the first commit made by the test. Fake
binary mutations are tied to a named command. `binary_ran` includes a version
invocation. `out_files` seeds the output directory before invocation. Optional
`setup.json` in other groups describes temporary inputs such as receipt
overrides and statistics.

The comparator uses each uncompressed dump as input to gzip unless raw
`*_transport` files are named. A `transport_edit` describes how to corrupt a
temporary copy. The arm receipts are compatible unless `receipt_variations`
or a later transport edit describes a mismatch. Each change pins the
applicable fields from `kind`, `count`,
`from`, `to`, `base_bag`, and `head_bag`. Here `from` and `to` are
transition attributes for a label move. Attribute bags and the list of
changes are compared with multiplicity. `unattributed` counts change rows. An
unattributed ambiguous bag counts once. The certificate also reports
`changed_keys` and `occurrences`. For example, one move and one addition on
the same key make two rows and two occurrences on one changed key. A change
row can hold multiple occurrences. Ambiguous rows pin the surviving
occurrence counts by arm since the aggregate convention for an unpaired bag
is open.

Unjoined non-edge trace records remain available to validation and are
outside the four edge-join counts. In a line-mode case with multiple open
readings, `record_count_alternatives` lists the admissible count sets.
Classifiers use `context.records(change)` for attached records. Helper names
for the documented contents of `bound` are placeholders until Stage D.

An oracle edge key contains the two projected endpoints. A call verdict key
contains `site` (the byte span) and `target` (the function identifier).
For call diffs, `verdict_arm` selects the verdicts compared with truth. When
named, `input.external` is a declaration file placed beside the oracle repo
to test a callable outside the query root. Oracle `why` and gate `reason`
strings are matrix labels. For those outcomes the harness compares verdict,
exit and counts only.

Review rule transitions contain `from` and `to` rule lists, with null for
a missing field. Review inputs adapt finding payloads from the CLI review
golden for an authored fixture.
[Review golden](../../../tests/fixtures/review_no_diagrams/golden/review_unflagged.json)
