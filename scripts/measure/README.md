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
  fields are. A projected integer (`line`, `start_byte`, `end_byte`,
  `kill_line`) is converted, so admission follows the interpreter's active
  integer string conversion limit (3.11 and the 3.7.14, 3.8.14, 3.9.14 and
  3.10.7 security releases onward). 4,300 digits is CPython's default, not a
  limit the kit sets: with the limit raised to 5,000 a 4,301-digit `line`
  passes, and with it lowered to 640 a 641-digit one is refused as
  `edge input`. An interpreter without the limit, such as an earlier 3.9
  patch release, has no limit and admits it. Resident edge storage is
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

Every map the comparator reads is either **closed** (an undeclared key is
refused) or **free-form by format** (the format defines no member list, so
extras are ignored or carried):

| Map | Keys |
|---|---|
| Receipt envelope and fixed subrecords (`binary`, `corpus`, `options`, `version_command`, `guard`, `host`, each command and `guard_status`) | closed keys |
| `artifacts` (names and `{sha256, bytes}` records) | closed keys |
| Corpus manifest entries | closed keys |
| `host.env` | closed keys (the passed host allow-list and `LC_ALL`) |
| `options.trace_env` and every command `env` | closed keys: each original key must match `PRISM_[A-Z0-9_]+` (the byte switch only in a command `env`), validated before any joining |
| Trace edge keys and trace endpoint parts | closed keys |
| Raw edge lines, endpoints and paths | free-form by format (additional members ignored) |
| Trace payloads and trace node keys | free-form by format |
| Counter files | free-form by format (arbitrary member names) |
| `prediction` record | closed keys |
| Corpus manifest envelope | closed keys; its `corpora` map is keyed by corpus keys matching `[A-Za-z0-9_@+~-]+(\.[A-Za-z0-9_@+~-]+)*`; an entry's `name` is descriptive, except that an `SB.` entry's `name` must equal its key |
| Classifier `LEGACY_MECHANISM_ORDER` | free-form by classifier: corpus key to a list of distinct tags |
| Inventory `recorded_path` and `chain_path` columns | free-form in S2: FORMATS gives both the `{base, fields}` shape but the parser projects extras away (open-s2-2) |

Known limits of S2: the free-form maps above are not checked for undeclared
members, and the inventory `recorded_path` and `chain_path` shapes are not enforced (open-s2-2).
A lane-side exception raised from a lane-local SQLite connection inside a
validator's lazily evaluated result can be reported as kit storage failure
(open-s2-1).

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

## TypeScript oracle

Run the Python commands from `scripts/`.

```text
python3 -m measure oracle ts-edges (--all | --changes FILE) --run RUN --out DIR
    [--typescript FILE] [--question FILE] [--relation NAME]
    [--side base|head] [--kind KIND]... [--class NAME]...
python3 -m measure oracle ts-calls (--base RUN --head RUN | --census RUN) --out DIR
    [--typescript FILE] [--question FILE] [--relation NAME]
    [--target-confidence exact|name_only]... [--target-kind NAME]... [--aggregate-only]
node measure/oracle/selfcheck.cjs [--typescript FILE] [--question FILE]
```

TypeScript selection is explicit file, `PRISM_TYPESCRIPT`, then the
gate-inputs resolver. Supply `lib/typescript.js` from the pinned 5.9.3
package. The oracle checks both the file and containing package SHA-256
pins before launching the worker. The worker compiles the verified checker
bytes and verifies each default library from its actual read. It runs
inside the 8 GiB memory guard with the §2 host environment allow-list.
`NODE_OPTIONS` and `NODE_PATH` never reach that worker. When calling the
small `node` self-check bootstrap directly, the caller controls that
initial Node process.

The built-in self-check has 812 golden rows in 79 cases. Each
`known-limit-*` case groups programs in separate `repo/<id>/`
directories. The worker grades each program in its own checker graph.
The case names persist for the 52 member-boundary rows that now sit
outside the decided known-limit registry.

The self-check prints and atomically stores its receipt in
`PRISM_MEASURE_SELFCHECK_DIR`, or `prism/measure/selfcheck` under the XDG
cache. Grading requires a prior receipt for the complete current input
key. It replays the goldens in the guarded worker to authenticate that
they actually pass and records Node and Python versions. Use the same
versions for both commands. Edited code, unused helpers, data, fixtures or
qualification evidence requires a new receipt. Keep the question, adjacent
goldens and declared data in a dedicated directory. Put caches, output,
gate files, predictions and binary links outside that directory.

The existing Ubuntu measurement CI job discovers these tests. It does not
install the pinned checker. Real-checker tests skip with
`pinned TypeScript unavailable via PRISM_TYPESCRIPT or gate-inputs resolver`
only when both locations are absent. Hermetic protocol and wire tests need
no compiler. Run these commands from the repository root with the pinned
install available and a host where the RSS guard can observe Node.

```sh
python3 -m unittest discover -s scripts/measure -t scripts -p 'test_*.py'
python3 -m unittest discover -s scripts/memguard -p 'test_*.py'
node scripts/measure/oracle/selfcheck.cjs --typescript "$PRISM_TYPESCRIPT"
```

### Accepted inputs and refusals

Records described as closed accept exactly the named keys, with only the
listed optional keys. Opaque data is allowed only in the free-form
positions below. Strings and object keys must be Unicode scalars. Integer
wire values must fit the exact Node integer range. Booleans are separate
from integers. Digests use lowercase hexadecimal SHA-256. JSON and JSONL
are strict UTF-8 with unique object keys and no non-finite numbers. JSONL
uses byte LF delimiters. S1 gzip or plain transport and its stored-byte
validation apply.

| Accepted input | Shape and use |
|---|---|
| Run | Successful S1 `prism.measure.receipt/1` directory with all required artifacts, validated by `receipt.admit`. The receipt and its binary, corpus, prediction, guard, host, options, version command, each analysis command and each guard status use their closed S1 schemas. The closed artifacts map uses the command-derived filenames and closed `{sha256,bytes}` entries. The host environment uses the closed S1 allow-list, each command environment must match its role, and `trace_env` admits string values under allowed `PRISM_*` keys. Check hashes and lengths even for artifacts the oracle does not grade. Only the recorded, still-resolving absolute snapshot path is used. Call arms must satisfy S2 pair admission. |
| Source | Entire bound §2 snapshot, with regular `.ts,.tsx,.mts,.cts,.js,.jsx,.mjs,.cjs` source files including declarations. The host serves named regular source members and relative in-root resolutions. It omits `.git` and symlink aliases, including paths reached through directory links. A symlink's real target can participate under its own regular in-root name. An authenticated regular non-TS endpoint is out of scope for `ts-edges --all` only, after coordinate checks. The host uses no project config, plugins or external declarations. |
| Edge row | Free-form object with `from,to,confidence,doubt`, optional consumed `kill_line,relation`. Each endpoint is free-form with relative POSIX `file`, `access=def\|use`, and `line,start_byte,end_byte` integer or null when present. Each `path` is free-form with required string `base` and ordered string `fields`. `confidence=exact\|nameonly`, null `doubt` or a nonblank reason for `nameonly`, and null or positive `kill_line`. Copy original extras, including those on endpoints and paths, to the grade callback and verdict. |
| Call row | Free-form `call_site` with `caller,source_span,callee_text,origin,resolved_targets,exact_target,drop` and optional `relation`. The free-form span uses relative `file` and optional integer/null `line,start_byte,end_byte`. Caller and target IDs are free-form with relative `file`, nonblank `name`, positive `start_line` and optional null or at-least-start `end_line`. Each target object is free-form with `function_id,confidence` and optional string or null `kind`. Confidence is `exact\|name_only`. `exact_target` is null or a function ID and supplies no claims. `drop` is null or a nonblank string. Keep original extras in the callback and verdict, and include them in full-context cancellation. Free-form `go_level3_b1_candidate` objects are counted separately using just their discriminator. |
| Change selection | Closed S2 `prism.measure.certificate/1` with the §3 top-level members, verified change stream and both named run receipt hashes. The closed `inputs` map has only `base,head`, each a closed `{run,receipt_sha256}` entry. Free-form `by_class` keys name the allowed classes. Its values and the remaining diagnostic maps and counters are opaque S2 evidence. The closed change row has `id,key,kind,count,base_bag,head_bag,records,class,error`. Each closed key has `from,to`, both closed `{file,line,base,fields,start_byte,end_byte}` endpoints. Each closed bag member has `confidence,doubt,kill_line,from_access,to_access`. Attached `records` are free-form objects passed as part of `ctx.change`. |
| Filters | Use `base\|head` and S2 kinds `added,removed,label_moved,ambiguous` only with `--changes`. A class name must occur in the certificate. Union repeated values within a filter and intersect filters. Census and call differential default to all kinds at exact confidence. Target kinds are nonblank original wire names. Request `name_only` only with a checked question and explicit relation. A relation is a declared question name, or a built-in name when there is no module. |
| Question | Closed CommonJS exports with unique nonblank `relations` and synchronous `grade`. The callback receives the original row and a context with `ts,program,checker,relation,arm,target_index,change,identifier_at_bytes,declarations,enclosing_call,node_at_bytes,program_with_default_lib`. The closed result is `verdict,property,why`. Verdict must be CORRECT, WRONG, UNDECIDED or INADMISSIBLE, both strings must be nonblank, and the relation/property pair must occur in the goldens. Passing a different program or a node from another graph to a lookup is an execution error. |
| Question closure | Recursively hash all regular files under the entry's parent. The entry must be JS or CJS. Computed CommonJS requires resolve only to in-set JS, CJS or JSON and load digest-checked bytes. The worker also provides the pinned checker, the built-ins `assert,assert/strict,buffer,path,path/posix,path/win32,querystring,string_decoder,url,util` and a read-only `node:fs` interface for declared data. `question-data.json` is a free-form map from logical relative names to digests. Its values authorize only those files at those digests. The module is reviewed trusted code subject to the input-closure rule, including any transitive helper and API access. |
| Self-check | Per-case regular `repo/`, `input.jsonl`, `expected.jsonl`, with optional bound sibling fixtures. Raw wire rows use automatic dispatch. Closed envelopes use `relation,arm,row` and optional `target_index,change`. The change is null or an S2 change for a base/head envelope. The expected row is closed `key,relation,property,verdict` with optional `why`. The closed key is `{from,to}` with the projected edge endpoint keys above, or `{site,target}` with closed `{file,line,start_byte,end_byte}` and `{file,name,start_line,end_line}` projections. Omitted wire values stay omitted in the projection. Every row must agree in output order by canonical key bytes. Named built-in boundaries require the prescribed reason. Each tested grading property has all four verdicts within its own question set (kit or lane); only the built-in fallback relation `relation-not-graded` is exempt, a lane that declares the name needs the same coverage. `input-admission` is a reserved label of INADMISSIBLE lane claims: a golden under it must be INADMISSIBLE (else `selfcheck-schema`), a module result naming it refuses (`question-result`), and it counts toward no relation, property or coverage. A lane relation with several graded properties, or with a reserved golden, labels its inadmissible claims with it, and its real properties then need the three graded verdicts. |
| Self-check receipt | Closed §4 receipt plus closed `runtime` with Node and Python versions. `components` is the closed five-member key object; `question_sha256` also frames the selected entry's name, so two entries of one directory have distinct keys. The worker executes a staged copy of the kit modules whose digests Python verified, so a built-in cannot change between hashing and loading. Compiler option records have the fixed §4 options. Check schema, key, components, counts, empty failures, and runtime before relying on the result. The worker independently checks the same current goldens. |
| Qualification | A reviewed kit artifact, not a caller input. Evidence is read only from `oracle/qualifications/` inside the kit tree and its `bundle.json` SHA-256 must equal the digest in the closed `oracle/qualification-pin.json` (`{schema:"prism.measure.qualification-pin/1",bundle_sha256}`; null pins no evidence, and installed evidence under a null pin, or a pin whose bundle is absent, refuses as `qualification-pin`). A caller-supplied evidence directory outside the kit tree refuses as `qualification-provenance`. The pin bytes enter the self-check key. Hand-written evidence is out of model: the pin authenticates the reviewed builder output, it does not re-derive it. Closed bundle with `schema,builds`. The free-form builds map is keyed by executable digest. Each form map is closed to the supported form names, with closed `{path,sha256}` references. The evidence file is the builder's closed record: `binary_sha256,form` must match the reference, closed `controls` is `{match:"CORRECT",contradiction:"WRONG"}`, and it records the source tree, source state, declaration file, receipt and sites digests, the closed label below and the original call row, which must be a valid call site whose indexed exact target agrees with the label. A record without that provenance refuses as `qualification evidence lacks builder provenance`; synthetic records exist only at the test injection seam (`oracle_inputs.INJECTED_QUALIFICATIONS`). Both the bundle and all referenced evidence bytes enter the self-check key. |

The accept-list has 18 refusal classes. A refusal publishes no successful
receipt or data files. INADMISSIBLE and UNDECIDED are completed grading
outcomes and can appear on exit 0.

| Exit | Message class | Trigger |
|---|---|---|
| 2 | usage | Invalid CLI grammar, option or occupied output. |
| 2 | run binding | Failed or malformed receipt, absent or altered artifact, invalid transport, incompatible pair. |
| 2 | wire-json | JSON/type/encoding/number outside the accepted wire representation, including a duplicate member. |
| 2 | wire-coordinate | Unsafe relative file or noninteger/non-null coordinate. |
| 2 | edge-wire | Missing or malformed edge field, role, confidence, doubt or path. |
| 2 | sites-wire | Missing or malformed site discriminator, caller, function ID, target confidence or kind. |
| 2 | certificate | Wrong certificate schema, run reference, digest or gate binding. |
| 2 | changes-wire | Wrong change, key (including its path fields), bag or attached-record shape. |
| 2 | selector | Unsupported, unrelated, incomplete or impossible arm, relation, class, confidence, kind or residual selection, including a change stream whose residual bags differ from S2's residual recomputed over every edge of both bound runs (an unlisted real residual, or a listed pair over identical arms), compared per identity on the change kinds and both bags (a label move restreamed as a removal plus an addition fails). |
| 2 | ambiguous-occurrence | An unequal set of original contexts prevents a unique residual assignment. |
| 2 | source-root | The recorded snapshot cannot be located and bound. |
| 2 | cancelled-site-coordinate | A raw differential site with no emitted verdict fails source or qualified-caller admission, or belongs to a snapshot that does not match its receipt. A selected claim never refuses for this: on a snapshot that mismatches its receipt, or in a pair where either arm does, it is INADMISSIBLE `source-digest-mismatch`, while an unselected site of the valid arm is still checked. |
| 3 | checker-pin | Checker path, bytes or tree differ from the pin. |
| 3 | selfcheck-binding / selfcheck-receipt / selfcheck-runtime | Grading cannot establish a current prior receipt, its inputs or Python runtime. A `selfcheck-binding` also names `qualification-provenance` (evidence outside the kit tree) and `qualification-pin` (invalid pin, or bundle absent, unexpected or differing from the pin). |
| 1 | selfcheck-schema / selfcheck-coverage / selfcheck-disagreement | Fixture shape, coverage, order, cardinality, expected verdict or cached counts fail. An invalid closure during self-check also exits 1. |
| 1 | question-closure / question-interface / question-result | Unsupported load/read, module shape, unchecked relation/property pair, result or exception. |
| 1 | checker-execution / bound-read / source-drift / qualification-control | Guard error, child error, Node runtime mismatch, input drift during execution or a failed qualification control. |
| 75 | host-quiet | Structured guard refusal before Node launches. A launched child exiting 75 is a checker execution failure. |

### What can be graded

**What a lane may conclude.** CORRECT and WRONG are the pinned TypeScript
checker's verdicts under the static-binding model at exact identifier bytes.
For call targets the checker-resolved body is mapped to the qualified producer
triple. CORRECT means the binding, slot or target agrees with the claim.
WRONG means the checker binds elsewhere or the exact bytes contradict the
claimed role or position. The table describes measured cases where runtime behavior can differ.
Hand-read every WRONG against the source before attributing it to the product.
The four Excalidraw WRONGs at this revision were assignment-target anchors
tracked in #961. UNDECIDED earns no credit either way and names the missing
proof in `why`.

A real build needs the producer qualification bundle for call-target
decisions. With the empty production qualification pin, every otherwise
decidable call-target row is UNDECIDED `target-id-unproved`. Slot and lexical
verdicts require no producer qualification.

Both `lexical_binding` and `argument_binding` return UNDECIDED
`member-path-not-judged` before role or slot checks when either endpoint has
nonempty `path.fields` or lands on a member name at the claimed bytes. A member
name is a property token in member access, the right side of a qualified name,
an explicit object or pattern key, or a class or interface member name,
including fields, methods, accessors and signatures. The rule also covers any
private name (including a brand check), a JSX attribute name, the `target` in
`new.target`, and an enum member's declaration name or a bare identifier that
resolves to an enum member. Plain-name edges through shorthand keys,
destructuring elements and computed-key identifiers keep their verdicts.
Argument binding still grades a call actual's own identifier unless that
identifier is a member name under this rule.

The receiver study found 0 decided member-path edges on the authored real-use
set, Express, the Excalidraw slice and whole Excalidraw. The `structured-path`
reason then covered 1,409 rows on Express and 6,651 on whole Excalidraw. The
current reason also covers rows that were already UNDECIDED under other reasons.
Rounds 7 and 8 kept finding new receiver and pattern-key cases. A later slice
would need a positive receiver rule and its own enumeration. The starting
point is `enum-r8/price-receiver/RULE-R.md`
in the external evidence, alongside the measurements in `PRICE.md`.

**Known limits and measured exposure.** The registry
[known_limits.json](oracle/known_limits.json) pins the checker's verdicts where Node
differs. The table gives decided edges at `d0a2b9c3` whose route touches the
listed detector or proxy. A proxy can include unrelated occurrences. The
static-initialization proxy can miss indirect calls made during initialization.
These counts measure syntactic exposure. A zero with unrecovered dependencies
leaves absence unproved. For both Annex B and eval, 4,632 Express edges and
5,775 Excalidraw edges had unrecovered dependencies.
Source measurements are in `enum-r8/price/classify-receipts.tgz` (`summary.md`).

| Known limit (example) | Measurement | Authored real-use (14 decided) | Express (5,040 CORRECT, 0 WRONG) | Excalidraw slice (14,285 CORRECT, 4 WRONG) |
|---|---|---:|---:|---:|
| Annex B block functions (B01) | Detector | 0 | 0 | 0 |
| Sloppy direct eval (E01) | Detector | 0 | 0 | 0 |
| Catch-parameter var redirection (R14) | Detector | 0 | 0 | 0 |
| Host and script vs. module bindings (M09) | Proxy | 0 | 3,694 | 661 |
| Parameter-default closure scope (N03) | Detector | 0 | 0 | 0 |
| For-let initializer environment (N10) | Detector | 0 | 0 | 0 |
| For-in/of head environment (B07, pass 3) | Detector | 0 | 0 | 0 |
| Module resolution, extension substitution (N20) | Detector | 0 | 1,289 | 3,326 |
| Module resolution, package main, JSON and declaration files (A14) | Proxy | 0 | 3,682 | 5,821 |
| HTML-like comment tokenization (N80) | Detector | 0 | 0 | 17 |
| Scope of computed-key expressions (C05) | Detector | 0 | 0 | 11 |
| Static initialization order (C01) | Proxy | 0 | 0 | 0 |
| Erased ambient declarations (B19, pass 3) | Detector | 0 | 0 | 5 |

The historical T05 count is a proxy for spread/getter and noncomputed
`__proto__` exposure (0 authored, 5 Express, 132 Excalidraw slice). A visible
spread leaves getter origin unproved. The receipts archive records the count.

The spot check reproduced all 14 enum-r8 programs it sampled
(`SPOT.md` in `enum-r8/price/spot-receipts.tgz`). Accept-list A kept 233 of 19,343
decided edges (1.2%, all 233 in Express), according to `summary.md` in
`enum-r8/price/classify-receipts.tgz`. Its stratified hand-read sampled
30 kept and 30 discarded CORRECT occurrences. All 60 were sound against runtime
(`summary.md` for the split and `enum-r8/price/HANDREAD.md` for the readings).

Argument binding checks a whole identifier or primitive literal actual against a
fixed implementation slot. Parentheses, type assertions, non-null assertions
and `satisfies` preserve an entire actual. The checker-selected body needs a
positive value-origin certificate; anything outside it is UNDECIDED
`ambiguous-dispatch` (closed world). Only parentheses, `!` and `satisfies`
are stripped from a callee or receiver, uniformly, so `o.m(x)`, `(o).m(x)`,
`o!.m(x)` and `(o satisfies C).m(x)` agree; `as` and `<T>` are never
stripped. The call's body is credited only when the callee is
(a) an identifier bound to the body's declaration (a function declaration
with its overloads and one implementation) or to an unannotated `const`
holding the body, or an alias chain of such consts (depth 8); the symbol must
never be written, and a function declaration must be the symbol's only bodied
declaration, with no `var` binding of its name that assigns in its own var scope
(an initializer, a destructuring leaf or a `for..in/of` head; nested functions
are not searched, and a plain `var f;` assigns nothing), and, when it is a
top-level function of a non-module script (no module syntax, no CommonJS
exports: a property of the global object), the closed global-scope rule below;
or `new X(..)` with `X` a certified class whose first constructor is the body;
(b) a member access with a proved receiver: `super`, a private name, a
namespace function, a static member through a certified class, an object
literal that owns the member as one plain property (no spread, no computed
key), or `new X()` (directly or as the only use of an unannotated `const`)
with `X` a certified class. A certified class is a class declaration, or an
unannotated `const` holding a class expression (alias chains allowed), never
written, without class decorators, and an `extends` that is itself certified.
A fresh `new X()` receiver also needs every constructor of the `extends` chain
to return no value; a static call through the certified class does not (a
certified static call still decides when the constructor returns an object).
The first class of the chain that declares the member must declare exactly the
proved owner (one field, or a method with its overloads and exactly one
bodied declaration); a computed name, parameter property or accessor of that
name leaves it unproved; an instance field or parameter property of the name
anywhere in the chain gives an instance method no proof (it is an own property
and wins; a computed-name instance field anywhere in the chain counts as a
possible match), and `super.f` never reaches an instance field; a namespace
function (a namespace or namespace import) needs an unwritten namespace
receiver (`N = {..}` replaces every member), its resolved export symbol never
written, and exactly one bodied function declaration for the export, which is
the owner, however many duplicate diagnostics are suppressed (the identifier
callee rule), under the same closed global-scope rule;
(c) an immediately invoked body.

**Closed global-scope rule.** Program-wide checks apply by spelling to script-global function names
and to namespace member-access routes. A script-global function needs no
assignment or member write of its name in any file and no other declaration
of that name in a script. A namespace member access checks identifier and
member writes to the receiver and function names, plus receiver redeclarations
in the access file and other scripts. Assignment targets include destructuring
leaves, `for` heads, compound assignments, updates and `delete`. A member write
on any receiver, including a UMD parameter or a global-object alias, counts
under the written name. These checks can yield UNDECIDED `ambiguous-dispatch`.

The callable model excludes runtime replacement or mutation of module,
namespace or global objects, programs relying on diagnostic suppression and
duplicate declarations. The detected forms below yield named UNDECIDED
reasons. The file-wide checks examine the callee's file F and, for a TypeScript
namespace member, all files contributing to the namespace.

- UNDECIDED `out-of-model-module-mutation` when F holds a write to
  `module.exports` or `exports` (an assignment, update, `delete`, destructuring
  or loop-head target rooted at `module` or `exports`, at any alias of them
  assigned in F such as `const alias = module`, or `Object.assign`,
  `Object.defineProperty`, `Object.defineProperties`, `Object.setPrototypeOf`
  or `Reflect.set`/`defineProperty`/`deleteProperty`/`setPrototypeOf` on one of
  them), or a write to a member of the global object (`globalThis`, `window`,
  `self`, `global` or an alias, any member name) or `this.<name> = ..` at the
  top level of a script, or, for a TypeScript namespace member, a write of the
  member name by any receiver spelling in any file (`(N as any).f = ..`).
- UNDECIDED `out-of-model-suppression` when F carries `@ts-nocheck`,
  `@ts-ignore` or `@ts-expect-error` (by spelling, anywhere in the file), or a
  duplicate declaration of the callee name (a `let`, `const`, `class`, `enum`,
  `namespace` or assigning `var` of the name in the same scope, in any block of a
  merged namespace; a plain `var f;` and a block-scoped namesake of another
  scope are not duplicates) or of the namespace receiver name (declared again
  anywhere, nested scopes included).

The module-object check applies to cross-file calls and calls to namespace
members. The global-object check applies to script-global functions and
namespace members. The suppression check applies to every callee.
For an ES module export reached as `ns.f()`, the member-access
rule sees a third file's `(ns as any).f = ..` and yields
UNDECIDED `ambiguous-dispatch`. For the same export reached by a bare
`f()` through a named import, that write alone permits a decision. The
all-file member-write check on the bare route applies to TypeScript
namespace members. `certificate-module-namespace-routes` pins the ES
module distinction. An ordinary CommonJS export (`module.exports = { f }`) is a write
to `module.exports`, so a call into such a file from another file is UNDECIDED
(recall cost; calls through `require` were already undecided). Reflective
writers beyond the list, computed keys and runtime re-acquisition remain out
of model (above). The exported-symbol write check of the namespace route is
kept as depth but is shadowed: an assignment to a function is TS2630 (gate) or
suppressed (belt), so no self-check row reaches it alone.
A decorated member (method, accessor or field, reached statically, through a
private name or through `super`) is never proved: a member decorator may
return a replacement and no decorator is accepted as inert, so every
certificate form refuses it (UNDECIDED `ambiguous-dispatch`) and an
`argument_binding` slot verdict through a decorated method is UNDECIDED. Only
the call-target identity of a decorated METHOD is kept (`decorated-method`,
under a matching qualification): it grades which declaration the checker
resolves under the producer's convention, not which body runs.
Erroneous programs decide nothing: when the callee's file or any file of the
owner's class chain carries a checker diagnostic TS2393 (duplicate
implementation), TS2425/TS2426 (property/method conflict across a chain),
TS2855 (`super` access to a field) or TS2630 (assignment to a function), the
verdict is UNDECIDED `ambiguous-dispatch`, argument slot and call target
alike. Only diagnostics the program reports count: a suppressed one
(`@ts-ignore`, `@ts-nocheck`) is not seen, and the structural rules above stand
alone for it. TS2300 (duplicate identifier) is not in the gate: it would also
refuse `function q(){} var q;`, which is sound, and the var-scope rule decides it.
Any assignment, update, `delete`, destructuring-assignment leaf or `for`
head target naming a member of the called name anywhere in the analysed
sources leaves the receiver unproved, and a non-`readonly` field arrow is
proved only on a fresh `new` receiver whose field has no annotation.
Computed-key writes, `Object.assign`, `defineProperty` and reflective writes
are outside the model, as for `Exact`. A class `extends` expression and a
shorthand destructuring default are reads; `implements` and interface
`extends` stay type uses. Composite expressions, earlier spreads, rest slots
and destructured formals lack a built-in fixed-slot proof. Annotated, ambient
and declaration-only callables and unresolved or ambiguous receiver origins
remain UNDECIDED.

Known limits of the callable certificate (accept-list line: only certificate
forms are decided). Round 3 of the S3 review found the old deny-list open:
each round added instances (aliases, casts, constructor return, destructured
writes), so S3 decides only the forms above. Each of these is a self-check
case with its UNDECIDED verdict beside a CORRECT control
(`oracle/selfcheck/certificate-*`): a `let` or `var` callee reassigned
(`let a = f; a = g; a(x)`), a conditional callee (`flag ? a : b`), a callee
typed by a parameter (`cb: typeof a`), a whole-callee assertion
(`(x as T)(a)`, `<T>x`), a constructor reached through a parameter, a call
result or a reassigned `let K = class`, a constructor or base constructor
that returns a value, a class decorator, a decorated member, a written namespace
receiver (`N = {..}`), two bodied exports of one name (diagnostics suppressed), a
function reassigned by a `var` of its name (initializer, destructuring, `for..of`
head), a global-scope function or namespace member whose name is written by any
receiver or declared again in a script file (the closed global-scope rule above:
`globalThis.N = {..}`, `var N = {..}` replacing a script namespace, a UMD `root`
parameter, `var gt; gt = globalThis; gt.f = ..`, `window.window.f = ..`, `var f`
in another script file), a module object replaced or mutated by any spelling
(`module.exports = {..}` also through `const alias = module; alias.exports =
{..}`, `module.exports` beside an ES `export function` reached by a namespace
or named import, `exports.f = ..` beside a named import, `Object.assign` on
`module.exports`; `certificate-fold7-module`), a namespace member written on any
receiver in another file or declared again as an exported `var` in another
block of the namespace and reached by a BARE call inside the namespace
(`certificate-fold7-namespace`), a function and a `const` of one name in a
namespace under `@ts-nocheck`, a suppressed diagnostic anywhere in the callee's
file, a duplicate declaration of the callee name (`certificate-fold7-suppress`),
a global-object or top-level `this` write in the callee's file for a script
function or namespace member (`certificate-fold7-global`), a same-name nested
`class`, `enum` or `namespace` declaration (by spelling,
`certificate-fold7-pins`), a computed-name instance field in the chain,
a program carrying TS2393, TS2425,
TS2426, TS2855 or TS2630 in the callee's file or class chain, a write to the
member through
`[this.m] = [..]`, `[C.prototype.m] = ..`, `({k: C.m} = ..)`,
`[C.m = d] = []` or `for (C.m of ..)`, annotated, asserted or multiply-used
receiver constants. `(x as C).m()` is UNDECIDED even when the assertion keeps
the inner expression's own type. Hand-written qualification evidence is out
of model (see Qualification above).

Lexical binding checks whether the checker resolves a declaration or write
and a read to the same unique value binding. Old-value reads in compound and
logical assignments and updates count as reads. An otherwise admitted
endpoint inside a `delete` operand is UNDECIDED `delete-operand`. The scope
covers binding identity irrespective of kill order or reachability. Source
coordinates use LF-only physical lines, full UTF-8 byte boundaries and the
formal declaration-header exception.

WRONG is returned only for a proved position, role or declaration fault at an
endpoint spelled as claimed at exact identifier bytes. Lexical cases:
- A role or position fault (source not a definition, destination write or type
  position) is WRONG only when no other occurrence of the claimed base on
  that endpoint's physical line could serve the role (source: a definition or
  write of the base; destination: a read of it). Otherwise the producer's
  line-anchored span may have landed on a neighbouring occurrence, yielding
  UNDECIDED `endpoint-coordinate-off`. Assignment
  targets such as `(objectURL = ..)` or the inner `t2` of `t1 = t2 = ..`, with
  no read of the name on the line, stay WRONG `write-position`.
- A spelling mismatch (the span is an identifier other than the claimed base) is
  UNDECIDED `endpoint-spelling-mismatch`, never WRONG, because the producer's
  alias twins carry the alias variable's span for a def of the base and the
  oracle cannot prove such an edge wrong. When the claimed base also occurs on
  that line in the needed role the reason is `endpoint-coordinate-off` (checked
  first); matrix row oracle-010 (a span on `other` beside a read of `value`) was
  WRONG and is UNDECIDED `endpoint-coordinate-off`.
- Both endpoints at exact identifier bytes spelled as the claimed base that
  resolve to different VALUE bindings (variable, parameter, function, class,
  import, binding element by its name) are a proved different binding, WRONG
  `different-declaration`, without a neighbour check (matrix row oracle-007).
- `implements`, interface `extends` and computed property/method signatures
  in interfaces and type literals take the type role (WRONG `type-position`
  unless a neighbour serves). Erased computed keys of accessor signatures,
  abstract members and body-less method overloads are UNDECIDED
  `erased-computed-key`. Evaluated computed keys and class `extends` remain
  reads. Ambient and `declare`-field cases are recorded in the known-limit ledger.
- An endpoint that is not at exact identifier bytes is UNDECIDED
  `identifier-unproved`.

At a proved slot, a wrong actual spelling is WRONG if its line lacks a serving
read or primitive literal with the claimed spelling. For a formal claimed at
other identifier bytes, only the proved formal can serve as a neighbour. It must
have the claimed spelling and share the wire line or the physical line of the
claimed token. A different name at exactly the proved formal's bytes and a
signature formal of an overload are always WRONG (oracle-032, oracle-037).
This applies also when the overload and its implementation share a line and to
constructors, whose overloads are the containing class's constructor
declarations (`argument-fold7`). With no proved contradiction or serving
neighbour, a formal span without an identifier gives `identifier-unproved`.
Of the 36 historical Express WRONG
verdicts sampled, four landed on property keys beside the value reads and now
have reason `member-path-not-judged`. The other 32 involved alias twins or other
spelling mismatches. The `lexical-coordinate` self-check pins the shapes beside
value controls.

Call target grading supports explicitly named functions and simple
identifier-named methods whose name and first non-decorator header token
are on the same line. A matching qualification for its executable digest
and form is required. The caller triple of a site is judged only when the
caller's own form is qualified for that build (another file for the caller
always contradicts); an unqualified caller convention is no evidence. Constructor, inferred or anonymous, computed and
split-header identities remain UNDECIDED. A checked question may establish
a different property using the source and original record.

`oracle/realuse/repo` supplies a multiline formal, two fixed slots, a
supplied optional, an explicit inert default, and Unicode with a multiline
call. `oracle/realuse/truth.jsonl` labels six argument-to-formal bindings and
27 lexical def-to-read bindings directly from their source token spans.

Use `measure.oracle.build_qualification(runs, labels, output_dir)` with
already captured `--sites` run directories. Each closed label has
`run_index,record_ordinal,target_index,form,declaration,contradiction_line`.
The closed `declaration` holds `file,name,start_byte,end_byte,line` for
the source-labelled name token. Forms are `named-function`, `named-method`
and `decorated-method`. The builder runs the guarded match and wrong-line
controls before publishing the bundle. It launches no producer. Install
the reviewed bundle at `oracle/qualifications/`, record its `bundle.json`
SHA-256 in `oracle/qualification-pin.json` and rerun self-check. Synthetic
test fixture qualification does not qualify an actual product build.

The receipt binds ordered `inputs.runs` entries `{arm,receipt_sha256}`,
`inputs.runtime`, and `inputs.qualification` as null or `{path,sha256}`.
Change selections also bind `changes_sha256,certificate_sha256,certificate_result`.
The closed selection is `{arm,kind,class}` for edges or
`{arm,target_confidence,target_kind,aggregate_only}` for calls. The
receipt binds the explicit relation and gate digest/step fields.
Verdicts sort by canonical UTF-8 key bytes, arm, original record ordinal
and target index. Calls cancel full raw contexts before assigning projected
residuals. Census emits no deltas. Call differential emits all site deltas,
while defaults grade gained exact head claims. Aggregate-only retains
tuples, denominators and receipt. Edge changes count the selected arm's
residual occurrences as `selected`, independently of the arm's full size.
