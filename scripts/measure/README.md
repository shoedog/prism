# Measurement kit

Run from `scripts/` with Python 3.9 or later. The kit uses the Python
standard library, Git for Git-backed pins, and the repository memory guard.

```sh
python3 -m measure corpora list
python3 -m measure corpora verify EX T --corpora-root /path/to/clones
python3 -m measure run --binary /path/to/prism --corpus EX \
  --corpora-root /path/to/clones --out /path/to/new-run \
  --expect-version 1234567 --prediction /path/to/prediction.md
```

Use `--manifest FILE` before the subcommand for a different registry. Corpus
lookup tries each `--corpora-root` in order, then `PRISM_MEASURE_CORPORA`
(a path list separated by the host's path separator), then the kit checkout.
Within each location the registry supplies ordered checkout paths and a query
root. The first existing query root must pass admission. Git pins require the
registered commit, a clean query root and a match to the raw pinned blobs.
Filter-converted bytes, hard links, gitlinks and links that escape the root,
enter `.git` or form cycles are refused. Extracted trees require a complete
`tree_sha256` for that query root, including literal symlink targets. The
perf-cloud manifest is an additional check when supplied. `RS` and `R`
select the same ripgrep registration. `GIN` selects gin and `G` selects guava.

Register each `SB.<class>.<entry>` with the acquired package's own tree digest
and package path. The registry ships the public Git pins and the R4B tree pin. It holds no `SB`
entries, so `run --corpus SB.<class>.<entry>` against it exits 2 (unknown key)
until S6b registers digests computed from the authenticated acquired trees (S1 verifies any
per-package registration now; default `SB` registration follows authenticated acquisition).
A manifest supplied with `--manifest` can register them earlier. The source-only
`R4B` fixture is registered against its measured `repo/` tree, and package and extraction registrations
require digests from the actual acquired trees. An extracted tree with a
perf-cloud `manifest_sha256` must hold no `.git` file or directory, because the
reference checksum counts them and the section 2 digest does not.

`run` requires an absent or empty output directory outside the query root.
It retains a private source snapshot without Git administrative files or
sibling source directories. The receipt binds the specified file and link
bytes and also records a separate fingerprint of directories, file modes and
link resolution state. Checks before and after execution detect persistent
changes. In-between changes that are completely restored can escape these
checks. Run receipts record execution evidence. Product credit and
qualification of ambient inputs belong to the consuming gate.

By default the guard uses an 8 GiB RSS limit. `--no-bytes` disables the edge
byte switch, `--sites` adds the call-site dump, and repeatable
`--trace-env NAME=VALUE` applies only to the edge command. Version pins match
the version token or at least seven hex digits of the reported commit. A
build whose commit token carries a `-dirty` suffix cannot match a commit pin;
pin its version token or build from a clean tree. The receipt also records the
guard's SHA-256, the SHA-256 of the host-quiet helper at `../hostquiet/host_quiet.py` relative to the guard file (null when absent or not a regular file; other dependencies of a custom guard are not bound, see open-n3-21) and
`kit_dirty`, which marks Git-visible edits to the kit (tracked changes and untracked
non-ignored files under the kit directories) and, as a fallback for tracked edits that
`assume-unchanged` or `skip-worktree` index flags hide, any tracked kit path that cannot be shown to match the
recorded commit's blobs (byte or symlink-target difference, missing file, type mismatch or comparison error counts as dirty). Mode-only working-tree edits hidden from `git status` by those flags and ignored untracked files are not
seen (known-s1-2). The guard and its helper
are rechecked before each launch and after the last command; a change fails the
run (exit 4; 75 when a later attempt reports host quiet after an analysis has launched); once an analysis has launched the failed receipt is retained, before that there is none. If the version attempt reports host quiet while the guard drifts, `run` returns
75 before analysis and publishes no receipt. If input and guard both drift at
completion, the guard check overwrites `input_changed` with `failed`, retaining
the final input digest and exit 4 (or the existing late-quiet 75). See known-s1-4.
The guard must be a readable executable regular file. For ordinary rewrites
and deletions of captured artifacts, the failed receipt lists `artifact_drift` entries with the captured and
retained digests (retained is null when the file is gone) and publishes the
surviving files.
See `run --help` for the full interface.

Successful runs return 0. Usage and occupied output errors return 2, corpus
and version pin refusals return 3, and command, guard or input failures return
4. A live host-quiet marker reported by the guard returns 75. A binary's own
exit 75 is a command failure. Finalized runs keep output digests, the raw
guard exit and structured status for each attempt, and the prediction digest
captured before version. A refusal before an analysis launches leaves no
measurement receipt. Once analysis starts, a failure retains partial
artifacts and publishes the failed receipt last.

A worker still running in the command's process group after the command exits is
terminated and reported as `guard_error` (exit 1), never as completion. Standalone
use has the same effect: the guard terminates the surviving group and exits 1
even though the command itself succeeded, so a command that deliberately leaves
a background worker behind must not be run under it.

The guard also runs directly as `scripts/memguard/memguard.sh LIMIT COMMAND ...`
from the repository root. `--status-fd FD` precedes the limit when a caller
supplies a dedicated status descriptor. The kill log uses `MEMGUARD_LOG`, then
`XDG_STATE_HOME/prism/memguard-kills.log` or the corresponding path under
`~/.local/state`.

Run the hermetic tests from the repository root.

```sh
python3 -m unittest discover -s scripts/measure -t scripts -p 'test_*.py'
python3 -m unittest discover -s scripts/memguard -p 'test_*.py'
```

## Known limits of S1 (accept-list)

Round 3 of the S1 review (sol) found four more instances of one class: edge cases around trusting the guard, its quiet helper, the kit's own files and the product's staged artifacts when those are special files or edited by an adversarial party. Opus approved the S1 head before these. The boundary: S1 binds regular-file bytes of the executable, guard and helper (rechecked at each launch and at completion), the corpus snapshot, and the staged artifacts; it detects ordinary replacement, rewrite and deletion. It does not defend against a guard, helper, kit file or product that is itself adversarial (FIFO or device substitution, mode-only edits under suppressing index flags). Those four instances are the rows `known-s1-1..4` in FORMATS-OPEN-ROWS.md, each with an owner. A consumer that must resist an adversarial product or guard must wait for those rows.

## Compare two runs

```sh
python3 -m measure diff-edges BASE_RUN HEAD_RUN \
  --classifier measure/examples/classify_783_4b.py --out new-comparison --list
python3 -m measure diff-stats BASE_RUN HEAD_RUN --which both --allow allowed.txt
```

`diff-edges` publishes `changes.jsonl`, `classes.tsv` and finally
`certificate.json` into an absent or empty directory. The default is
`./diff-edges`. Without `--key`, it chooses byte or line keys from the full
edge population. Explicit `--key bytes` requires full bounds. Explicit
`--key line` requires a nonblank `--line-reason` whenever it erases a bound.
The certificate records that reason, the loss flag and line-key collisions.
The legacy stdout edge header counts key multiplicity and common keys with
unequal attribute bags. The `occurrences:` line counts residual rows weighted
by their `count`, including the larger arm's bag length on ambiguous rows.

`diff-stats` takes `--which dfg|call|both` and prints every numeric delta
from the selected tables. With no `--allow`, it reports deltas and exits 0.
An empty file requires parity. Allow entries are exact component paths or
component prefixes ending in `.*`; prefixes require an additional component.

### Input accept-list

- **Runs.** Both run directories must have successful S1 receipts, the
  registered corpus pins, the same input digest and matching analysis options
  after removing the declared trace environment. Every required artifact
  must be a regular file at its declared role with the receipt's size and
  SHA-256. Binary and trace settings may differ. For a callback that uses an
  arm's receipt or run path, or the head's `corpus_root`, that arm's absolute
  snapshot path and both receipt digests must still resolve and verify.
  Verification occurs on first access and after validation. Other callbacks
  can compare detached edge dumps. Standalone classifiers are local trusted
  Python code and may access lane sidecars on the original run paths.
- **Transport and edges.** Uses byte-LF JSONL, CRLF or an EOF-terminated last
  line. ASCII whitespace-only lines are ignored. Plain transport and the
  declared gzip transport use strict UTF-8, no BOM, no duplicate JSON names,
  no nonfinite JSON numbers and no trailing JSON value. Gzip members
  concatenate and trailers are verified through EOF. Each edge is an object
  with `from`, `to`, `confidence` and `doubt`, and optional `kill_line`.
  Each endpoint is an object with string `file`, positive integer `line`,
  `path` of string `base` and ordered string array `fields`, and
  `access` equal to `def` or `use`. Optional `start_byte` and
  `end_byte` are null or nonnegative integers with start ≤ end when both are
  present. Equal bounds and zero are admitted. Confidence is `exact` or
  `nameonly`. Doubt is null or a nonempty string. `exact` requires null.
  `kill_line` is null, absent or a positive integer. Integers exclude
  booleans. Projected strings are Unicode scalars with exact code points.
  Paths keep their strings and fields without normalization or dot splitting.
  Additional members are ignored for comparison, and a finite number in
  one (integer or float, of any size) is never converted: only projected
  fields are. Resident edge storage is
  bounded by SQLite caches and the current key's bags and attached records.
- **Counters.** Each selected stats file is one strict UTF-8 JSON object.
  Descend nested objects and count integer, decimal and exponent tokens.
  Numeric tokens may have up to 1,000 characters and a decimal quantum
  exponent from −1,000 through +1,000. Integers and decimal arithmetic are
  exact within that range. Boolean, null, string and array leaves are skipped.
  Missing leaves remain distinct from zero. Member names are Unicode scalars.
  Render tuple components joined by dots, escaping literal backslash, dot,
  star and hash as `\\`, `\.`, `\*`, and `\#`. Use `\e` for an empty
  component and `\u{HEX}` with uppercase digits and no leading zeros for
  whitespace or control characters. Allow files are UTF-8 LF or CRLF text.
  An unescaped hash begins a comment. Blank lines and surrounding whitespace
  are ignored. Remaining lines must use the canonical path spelling.
- **Trace.** The comparator decodes the complete edge stderr in strict UTF-8,
  rejects a leading BOM and passes physical lines with LF when present.
  Optional `parse_trace(line, arm)` returns `None` or a JSON-compatible
  dict with the exact input arm, a nonempty single-line `tag` and `kind`
  and a dict `key`. Payloads admit scalars, arrays and string-keyed objects.
  Edge records supply at least one field across `from` and `to`; each may
  supply a subset of flat `file,base,fields,line,start_byte,end_byte` using
  the edge field types. Unknown information is omitted. Every supplied
  discriminator matches the same occurrence on its own arm. Null bounds
  carry no discriminator. Copies count independently. A sole projected
  candidate must also agree with any supplied original bound it knows.
  Other typed kinds are unjoined. A `node` supplies `file,line,start_byte,
  end_byte,access` and optionally a string `owner`. The optional subscript
  parser accepts nine tab-separated `subscript-site` fields and the
  `subscript-inventory-end<TAB>all<TAB>D<TAB>U` terminal. Node path columns
  carry structured JSON; the chain column also admits raw `refused` and
  `-`. Its helper validates complete inventories on the requested arms.
  The 4b parser recognizes six-field `rd-census` with
  `path@positive_line:nonnegative_byte` places. It preserves the payload
  columns of `rd-alias-cert`, `rd-alias-move` and `rd-census-owner` as
  unjoined records and ignores unrelated lines. An optional historical
  `legacy_key` has five items of types string, string, positive integer,
  string, positive integer and a single-line `mechanism`.
- **Classifier.** A readable local Python file exports a `CLASSES` list of
  distinct nonempty single-line names, with an empty list allowed, and
  `classify(change, context)`. The result is one declared string or `None`.
  Optional `STRAYS` is `fail` or `allow`, and optional
  `LEGACY_MECHANISM_ORDER` maps corpus keys to distinct tag lists.
  Optional `validate(bound)` returns a list of single-line diagnostics.
  The context exposes both arms, all typed records, cancelled bags and the
  DFG and call deltas. The validator also receives final changes, joins,
  unjoined records and the `nodes(arm, file, start_byte, end_byte, access,
  owner=...)` lookup. Views can be iterated repeatedly. Every context and bound attribute is read-only; assigning or deleting one raises `AttributeError`. A diagnostic from
  validation vetoes credit even when every change has a class.

The accept-list has **17 named refusal or failure classes**, grouped below.
An ignored trace line, a counted unmatched record or a classified ambiguous
change bag can still pass.

| Exit | Message class | Cause |
|---:|---|---|
| 2 | invocation | Invalid arguments or registry |
| 2 | run binding | Unsupported, failed or incomplete receipt or unbound artifact |
| 2 | run pairing | Different input digests or analysis options |
| 2 | source binding | A requested arm's snapshot fails verification |
| 2 | edge input | Invalid edge JSONL, gzip or field schema |
| 2 | key mode | Unsupported bounds for the requested mode or missing line-loss reason |
| 2 | trace input | Invalid stderr transport, a parser exception (including an `OutputError` the callback raises) or invalid typed record, named with arm and line |
| 2 | classifier interface | Unreadable module, load exception or invalid exports |
| 2 | counter input | Invalid stats transport, object, member name or numeric token |
| 2 | allow list | Unreadable file or invalid/noncanonical component path |
| 2 | output or storage | Occupied output, staging or storage error, including a SQLite failure in the conservation query |
| 1 | unattributed change rows | Missing, undeclared or exceptional classifier result |
| 1 | ambiguous edge records | More than one projected candidate |
| 1 | forbidden stray edge records | A record joins a cancelled occurrence and either policy vetoes it |
| 1 | validation | Validator diagnostics, exception or invalid return |
| 1 | forbidden counter delta | Selected delta outside the supplied allow-list |
| 1 | conservation failure | Internal edge population or certificate accounting mismatch |

See [the formats of record](../../docs/superpowers/plans/2026-11-20-lanes/lane11-measurement-kit/FORMATS.md)
for the evidence schemas and [R4B](fixtures/repro-4b/README.md) for the
retained fixture capture and control generator.
