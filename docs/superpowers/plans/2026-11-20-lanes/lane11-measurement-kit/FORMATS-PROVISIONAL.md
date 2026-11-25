# Lane 11 measurement kit: provisional formats for the two adopters

**Historical adopter handoff, superseded by stage D.** Lane 10 and lane 9 received this draft on 2026-11-24. Implementers use the command, schema, callback and fixture decisions in [FORMATS.md](FORMATS.md). The archived examples below preserve what the adopters originally received.

## 1. Inputs the kit reads (shapes of today's binary, verified on `9d1a7d06f`)

`prism nav --no-cache dfg-stats --repo C` prints one JSON object of counters (`dfg_label_exact`, ...).
`prism nav --no-cache call-stats --repo C` prints one JSON object whose `dfg_labels` member nests the DFG counters.
`PRISM_EDGES_BYTES=1 prism nav --no-cache dfg-stats --repo C --edges` prints one JSON object per line:

```
{"from": E, "to": E, "confidence": "exact|nameonly", "doubt": null | "killed|sameline|cfg_incomplete|alias_unstable|call_nameonly", "kill_line": 13 (only with doubt "killed")}
E = {"file": "a.ts", "line": 1, "path": {"base": "o", "fields": []}, "access": "def|use", "start_byte": 11, "end_byte": 12}
```

`start_byte`/`end_byte` are present on both endpoints only with `PRISM_EDGES_BYTES=1` (UTF-8 byte offsets).
`call-stats --dump-sites` prints one `{"record_kind": "call_site", "caller": {...}, "callee_text", "source_span":
{"file", "line", "start_byte", "end_byte"}, "resolved_targets": [{"function_id": {"file", "name", "start_line",
"end_line"}, "confidence", "kind"}], "exact_target", "drop", ...}` per line.

## 2. `run` (S1): one binary on one corpus

```
python3 -m measure run --binary BIN --corpus KEY --out DIR [--corpora-root DIR]... [--trace-env NAME=VALUE]...
                       [--no-bytes] [--sites] [--prediction FILE] [--memguard PATH] [--limit-gb 8] [--expect-version HEX]
```

Exit 0 ok; 2 refused (non-empty `--out`, bad arguments); 3 pin or version mismatch (nothing else executed); 4 a command
failed or the input changed during the run (receipt written, `status` says which); 75 the guard refused (host-quiet).

Run directory `DIR/`: `dfg-stats.json`, `call-stats.json`, `edges.jsonl.gz` (the `--edges` stdout, gzip),
`edges.stderr.gz` (product trace lines, when `--trace-env` is given), `sites.jsonl.gz` (with `--sites`), one
`<command>.stderr` per command, and `receipt.json`:

```
{"schema": "prism.measure.receipt/1", "kit_commit", "start", "end", "status": "ok|failed|input_changed",
 "binary": {"path", "sha256", "version"},
 "corpus": {"key", "commit", "root", "pin_check": "git-clean|manifest", "manifest_sha256", "input_sha256"},
 "prediction": {"path", "sha256"} | null, "options",
 "commands": [{"argv", "env", "output", "output_sha256", "bytes", "exit", "start", "end"}],
 "guard": {"path", "limit_gb"}, "host": {"platform", "cpus"}}
```

Product traces (`PRISM_RD_CENSUS_TRACE=1`, `PRISM_CFG_OWNERSHIP_TRACE=1`, ...) are recorded in `edges.stderr.gz`,
the trace lines of the `--edges` command only; the kit does not interpret them (section 3, `parse_trace`).

## 3. `diff-edges` and `diff-stats` (S2): the comparator

```
python3 -m measure diff-edges BASE_RUN HEAD_RUN --classifier FILE [--key bytes|line] [--out DIR] [--list]
python3 -m measure diff-stats BASE_RUN HEAD_RUN [--allow FILE] [--which dfg|call|both]
```

Admission: both receipts must be `ok` with intact output digests, the same `corpus.input_sha256` and the same
options apart from the declared arm difference (binary, trace environment); otherwise exit 2 and nothing is compared.
Exit 0 pass; 1 any unattributed change, ambiguity nobody attributes, or a non-empty `validate` result; 2 refusal.

**Edge key** (DESIGN section 5, Edge identity): both endpoints, each `(file, path.base, path.fields as a list, line,
start_byte, end_byte)`. Byte mode when every endpoint of both dumps carries both bounds (the default then); line mode
(bytes dropped on both sides) when neither dump carries bytes or with `--key line`; mixed or partial bounds refuse
byte mode (exit 2). Attributes of a key occurrence: `(from.access, to.access, confidence, doubt, kill_line)`.
Identical attribute occurrences cancel per key; the residual is `added`, `removed`, `label_moved` (unique pairing,
each with its transition) or `ambiguous` (both bags kept, unattributed unless the classifier attributes the bag).

`changes.jsonl` (one row per changed key; `--list` also prints them):
`{"kind": "added|removed|label_moved|ambiguous", "count", "key": {"from": {"file","base","fields","line","start_byte","end_byte"}, "to": {...}}, "base_bag": [{"from_access","to_access","confidence","doubt","kill_line"}], "head_bag": [...], "records": [...], "class": str|null, "error": str|null}`.

`certificate.json`: `{"key_mode", "changed_keys", "occurrences", "attributed", "unattributed", "ambiguous", "by_class": {"<class>": {"keys", "occurrences"}}, "records": {"attached","stray","ambiguous","unmatched"}, "validation": [...], "inputs"}`.

Stdout, in order: `edges base=N head=N added=A removed=R label-moved keys=M`; `occurrences: ...`; `collisions keys=K` (line mode);
the class table; `trace records:` per tag; then `PASS` or `FAIL` with each failure.

**Classifier file** (a Python file; this is all a lane writes for the comparator):

```
CLASSES = ["name", ...]                       # every name classify may return
def classify(change, context) -> str | None   # None = unattributed; an exception or undeclared name = unattributed + error recorded
def parse_trace(line, arm) -> record | None   # optional; arm is "base" or "head"
STRAYS = "fail" | "allow"                     # optional, default "fail"
def validate(bound) -> list[str]              # optional, called once after classify; each string is a failure
```

`record = {"arm", "tag", "kind": "edge|node|write|owner|counter|...", "key": {...fields the trace names...}, ...}`. A `node` record names one def or use span (file, path, line, start_byte, end_byte) and is never joined to an edge; it reaches `validate(bound)` with every other unjoined record (lane 10's node inventory and spelling-leak gate). An `edge`
record matches the edges of its arm whose key agrees on every field the record carries; it attaches to a change only
when exactly one edge of its side matches and that edge is changed; a match on an unchanged edge is a stray, several
matches are `ambiguous`, none is `unmatched`; a record attaches to at most one change. `context` provides
`records(change)`, `corpus_root`, both run directories, both receipts and the counter deltas. `validate(bound)` receives
every change, the unchanged-key index, every record (attached, stray, ambiguous, unmatched) and the counter deltas.

`diff-stats`: flattens both statistics files (nested keys dotted: `dfg_labels.dfg_label_exact`), prints
`  key: base -> head (+delta)` per changed counter; the allow-list file has one dotted counter per line (`#`
comments, trailing `.*` = prefix); exit 1 on a delta outside the list.

## 4. `oracle` (S3): the TypeScript-checker oracle

```
python3 -m measure oracle ts-edges (--changes CHANGES.jsonl | --all) --run RUN --out DIR [--typescript PATH] [--question FILE]
python3 -m measure oracle ts-calls (--base RUN --head RUN | --census RUN) --out DIR [--typescript PATH] [--question FILE] [--aggregate-only]
```

Exit 3: the TypeScript file's SHA-256 is not `3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675`, or no passing
self-check receipt exists for the current checker tree, compiler options, question and fixture digest (Node is never
started). The oracle reads source from the run's corpus root after checking the receipt's `input_sha256`; offsets are UTF-8
bytes as `prism` reports them. Input rows are edge rows as in section 1 (a `changes.jsonl` row selects its `key`; `--all`
grades every edge of the run) and need both endpoints' bytes (absent bytes: INADMISSIBLE).

Output: `verdicts.jsonl` (one row per graded edge: `{"key", "relation", "property", "verdict": "CORRECT|WRONG|UNDECIDED|INADMISSIBLE", "why", "row": <the original edge row>}`),
`tuples.json` (`{"<question>": {"CORRECT": n, "WRONG": n, "UNDECIDED": n, "INADMISSIBLE": n}}`, zeros included), and for
`ts-calls` also `deltas.jsonl` (new, lost, retargeted, confidence-only and multi-target call-site deltas with denominators).
Built-in questions: `argument_binding` (a use in a call argument feeding a formal's def: CORRECT when the checker resolves
the call to one in-repo callable whose parameter at that argument position has the formal's bytes; WRONG when it resolves
uniquely to another callable or another position; rest and spread arguments, ambiguous owners and unresolved callables are
UNDECIDED with their reason) and `lexical_binding` (def to use: CORRECT when both identifiers resolve to one declaration and
the use is a read; WRONG for different declarations, a write use or a type position; reachability and intervening writes
are not graded). Any other relation is UNDECIDED `relation-not-graded`.

**Question module** (a `.cjs` file; one per lane at most): exports `relations` (the relation names it grades) and
`grade(row, ctx) -> {verdict, property, why}`; `ctx` gives the loaded TypeScript module, the program and checker for the
run's corpus root, and helpers to find the identifier at `(file, start_byte, end_byte)`, its declaration(s), the call
expression enclosing it (by full span), and `program_with_default_lib()`, a second program over the same sources built with the default
library (lane 9's built-in callback question). An unknown verdict string fails the run (exit 1). The built-in self-check (`oracle/selfcheck.cjs`)
must pass first; a lane's question module ships its own self-check cases in the same layout
(`oracle/selfcheck/<case>/{repo/, input.jsonl, expected.jsonl}`).

## 5. Questions left for stage D in the original handoff

The corpus manifest file name and its exact fields, `gate` files and report layout, `review-diff`, and the generic
classes (if any). Lanes do not depend on them for their classifier or question module.

## Change log

- 2026-11-24: first version, from `DESIGN.md` section 5 and `PLAN.md` S1 to S3 as folded on 2026-11-21.
- 2026-11-24 (adopter answers): record kind `node` added (lane 10 trace names spans, not edges); question-module `ctx` gains `enclosing_call` by full span and `program_with_default_lib()` (lane 9).
- Stage D accepted the original wire inputs, run-directory handoff and question-module layout, and revised the comparator, oracle selection and self-check contracts. It accepted all three adopter additions: `node`, `enclosing_call` by full span and `program_with_default_lib()`. Section 8 records each disposition and its effect. [Stage D adopter impact](FORMATS.md#adopter-impact)
