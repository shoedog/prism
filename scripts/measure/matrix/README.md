# Stage T truth matrix

Each line of `rows.jsonl` is one JSON object with six fields.

| Field | Meaning |
| --- | --- |
| `id` | Unique case ID, using `run`, `cmp`, `join`, `oracle`, `review`, or `gate` followed by a hyphen and three digits |
| `group` | `run`, `comparator`, `join`, `oracle`, `review`, or `gate` |
| `input` | `case` names the directory under `cases/`. Other fields select files or options. `inventory` lists numbered inventory disagreements. |
| `required` | One sentence specifying the behavior and, for a synthetic case, the fact that establishes it |
| `truth` | Expected result for the dimensions the row pins. `{"undetermined": "..."}` records the alternatives for Stage D. |
| `source` | `probe` retains raw bytes from a named probe, `product` requires a guarded baseline capture, and `synthetic` is adjudicated from the case facts. |

For probe cases, `input.probe` identifies the raw source. Some cases use a
synthetic counterfactual alongside raw evidence. `derived_from` identifies
inspiration for synthetic inputs. A product case names its authored source file
or directory in `input.fixture` and its unedited output files in `input.raw`.
Product captures require baseline version `9d1a7d06f653`.

| Group | Case inputs | Truth |
| --- | --- | --- |
| Run | `setup.json` describes the command, corpus, fake binary and optional environment. P5 also supplies `probe.edges.jsonl`. | `exit`, `binary_ran`, `receipt_status` (null if absent) |
| Comparator | `base.edges.jsonl`, `head.edges.jsonl`, `input.key_mode` (`bytes`, `line`, or `default`), and a `classifier.py` when named. | `exit`, `changes`, `unattributed`, `collisions`. Only pinned fields are compared. |
| Join | Comparator inputs plus `head.trace` and the trace parser in `classifier.py`. | Comparator truth plus `records` (`attached`, `stray`, `ambiguous`, `unmatched`) and `class` on each change |
| Oracle | `repo/cases.ts`, `input.jsonl`, and `gen.py` to rebuild input from UTF-8 source. Call cases use call-site records and, for a diff, `base.sites.jsonl`. `probe.jsonl` preserves original records where supplied. | `verdicts`, each with `key`, `verdict`, and `why` |
| Review | `base.json`, `head.json`, and `input.arms` selecting single or multi for both arms. | `presence_changes`, `payload_changes`, `ambiguous_bags`, `rule_transitions` |
| Gate | `setup.json` describes the gate, prediction contents, reused receipt, and any mutation or simulated result. | `exit` and `reason` |

Run recipes use `tracked`, `tracked_ignore`, `ignored`, and `untracked`
for the initial corpus and `second_commit` or `tracked_edits` for subsequent
changes. `initial-commit` refers to the first commit made by the test. Fake
binary mutations are tied to a named command. `binary_ran` includes a version
invocation. Optional `setup.json` in other groups describes temporary inputs
or checks named by the row, such as receipt overrides, statistics, or call
delta controls.

The comparator uses each uncompressed dump as input to gzip unless raw
`*_transport` files are named. A `transport_edit` describes how to corrupt a
temporary copy. The arm receipts are compatible unless `receipt_variations`
or a later transport edit describes a mismatch. Each change pins the
applicable fields from `kind`, `count`,
`from`, `to`, `base_bag`, and `head_bag`. Here `from` and `to` are
transition attributes for a label move. Attribute bags and the list of
changes are compared with multiplicity. Ambiguous cases pin the surviving
bags and leave the occurrence count open. Unjoined non-edge trace records
remain available to validation and are outside the four edge-join counts.

An oracle edge key contains the two projected endpoints. A call verdict key
contains `site` (the byte span) and `target` (the function identifier).
For call diffs, `verdict_arm` selects the verdicts compared with truth.
Oracle `why` and gate `reason` name semantic outcomes for the matrix. Review
rule transitions contain `from` and `to` rule lists, with null for a missing
field. Review inputs adapt finding payloads from the linked repository golden
named in `input.template`. The golden is CLI review output from an authored
fixture and uses the shared finding serialization.
