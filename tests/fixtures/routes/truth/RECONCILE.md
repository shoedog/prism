# Routes matrix reconciliation

The supplied truth contains 63 executed observations and one refusal. The seed
comparison found 58 rows in agreement and five with pin differences. All 63
executable rows agree after the edits. Counts below are by distinct row with a
compared pin in that field. Registrations include the check for extra calls.

| Field | Seed agreements | Changed rows | Final agreements |
|---|---:|---:|---:|
| Registrations | 63 | 0 | 63 |
| Framework | 60 | 0 | 60 |
| Declared methods | 47 | 2 | 47 |
| Registration path | 60 | 0 | 60 |
| Served path exposures | 55 | 0 | 55 |
| Callbacks | 57 | 1 | 58 |
| Site | 58 | 2 | 60 |

The two method edits remove pins, which is why the final method count stays at
47. Run `python3 -B tests/fixtures/routes/truth/reconcile.py` to see the compared
result and the static-only fields for each row.

## Expectation changes

| Row | Field | Author's hypothesis | Observed truth | Reason |
|---|---|---|---|---|
| ex10 | `site.ordinal` | Unpinned for GET and POST | GET 2, POST 3 ([truth line 10](truth.jsonl#L10)) | The builder's `.route()` call takes ordinal 1. Each verb leg now pins its raw call ordinal. |
| ex28 | `site.ordinal` | Unpinned for GET, HEAD and POST | GET 2, HEAD 4, POST 3 ([truth line 28](truth.jsonl#L28)) | The builder runs GET, POST, HEAD in that order after `.route()`. The expectation keeps its existing record order. |
| fl03 | `methods` | Declared GET | `null` declaration, effective GET, HEAD and OPTIONS ([truth line 32](truth.jsonl#L32)) | Flask supplies the effective set for `@app.route("/x")`. The declared method pin is omitted. |
| fl18 | `callbacks[0].name` | `status` | `create_app.<locals>.status` ([truth line 47](truth.jsonl#L47)) | The factory registers its local function, and Python reports its qualified code name. |
| fa06 | `methods` | Declared WEBSOCKET | `null` declaration, route kind `websocket`, empty effective method set ([truth line 56](truth.jsonl#L56)) | The native route table represents WebSocket as a route kind. The declared method pin is omitted and the Stage D representation stays open. |

## Provenance and coverage

- `fa12` moves from `executed` to `hand`. The runner reported
  `hand_required: untraced FastAPI mount: /s` at [truth line 62](truth.jsonl#L62).
  The orchestrator owns the reviewed provenance commit and truth regeneration.
- The matrix now has 74 hand rows. These are `fa12`, `ex30` (Express array
  mount outside literal-prefix recovery), `e01` and `e02` (core controls without
  probe entries), 41 Go rows, seven object-table rows and 22 Hapi rows.
- `fl21` exercises a lexical lookalike with no runtime registration
  ([truth line 50](truth.jsonl#L50)). The unproven record is entirely static.
  The unattached suffixes in `ex29` and `fa14` have no served path to compare
  ([line 29](truth.jsonl#L29), [line 64](truth.jsonl#L64)).
- The staging preflight also returns `missing entry for executed-truth probe`
  for `e01`, `e02` and `ex30`. Their existing hand provenance needs probe
  handling for the T-1 `check.sh` gate.

## Contradicts the design

None found in the compared observations. DESIGN 5.2's trailing-slash example
agrees with `fa02` at [truth line 52](truth.jsonl#L52). The implicit Flask
method set and WebSocket representation are Stage D decisions in the T-1 seed
table. The `fa12` refusal is consistent with the runner's documented mount limit.

## Fixture fixes

None in this reconciliation. The saved `truth.jsonl` is byte-for-byte as supplied.
