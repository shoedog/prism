# Measurement kit formats of record

Stage D, lane 11. This is the contract for S1–S5 and S6a–S6b. Lane 10 takes the comparator and lane 9 takes the oracle. Adoption requires a later decision on a measured differential. Design approval, implementation and gate results are separate records.

**Changes in PLAN.md.** S1 now binds trace bytes, the effective environment and the executable. S2 defines row compression, counts, trace projection, legacy output and both adopter interfaces. S3 binds selectors to arms, adds explicit relation selection, and fixes checker, module and receipt interfaces. S4 fixes arm selection, residual counts and sampling. S5 fixes reuse, thresholds and retrospective exit. S6a fixes finding precedence and duplicate matching. S6b fixes metadata lookup, scoring inputs, exclusion precedence and the minimum of 28 pairs. Stage E records the open-class stop and the exact fixture backlog. Each slice receives this file with its brief.

## 1. Commands and exits

Run `python3 -m measure` from `scripts/`. The global `--manifest FILE` precedes the command and defaults to `measure/corpora.json` beside the package. Brackets indicate optional arguments, `...` permits repetition, and vertical bars separate alternatives. Each kit `--out` directory must be absent or empty on entry. Section 7 defines benchmark acquisition and reuse. Obtain exclusive ownership before starting, preserve existing files on refusal, stage outputs and publish the completion record last. For `diff-edges` the default output is a new `diff-edges/` under the invocation directory. The kit's argparse commands accept `-h` or `--help` (0).

```text
corpora list
corpora verify [KEY ...] [--corpora-root DIR]...
run --binary BIN --corpus KEY --out DIR [--corpora-root DIR]... [--trace-env NAME=VALUE]...
    [--no-bytes] [--sites] [--prediction FILE] [--memguard PATH] [--limit-gb N] [--expect-version TOKEN]
diff-edges BASE_RUN HEAD_RUN --classifier FILE [--key bytes|line] [--out DIR] [--list]
diff-stats BASE_RUN HEAD_RUN [--allow FILE] [--which dfg|call|both]
oracle ts-edges (--all | --changes FILE) --run RUN --out DIR [--typescript FILE] [--question FILE]
    [--relation NAME] [--side base|head] [--kind KIND]... [--class NAME]...
oracle ts-calls (--base RUN --head RUN | --census RUN) --out DIR [--typescript FILE]
    [--question FILE] [--relation NAME] [--aggregate-only]
review-diff --population FILE --clones DIR --arm NAME=BIN[::ARG...] --arm NAME=BIN[::ARG...]
    --out DIR [--review-args STRING] [--only N,M,...] [--sample N] [--seed S]
gate FILE --out DIR [--binary NAME=PATH]... [--corpora-root DIR]... [--retrospective]
```

Benchmark commands are run in `eval/secbench/`. `run.py` and `replay.py` retain their existing argument parser and defaults. S6a adds output fields and shares a scorer. In particular `--class`, `--binary-sha` and `--sut-commit` already belong to `run.py`. The command block includes the two new benchmark entry points and the oracle self-check. `--typescript` always names `lib/typescript.js`. All checker and product executions use the guard (section 2).

```text
node oracle/selfcheck.cjs [--typescript FILE] [--question FILE]
python3 run.py [--inputs DIR] [--packages DIR] [--sut-repo DIR] [--compiler FILE] [--out DIR]
    [--adjudications FILE] [--binary BIN] [--no-build] [--inspection FILE] [--workers N] [--timeout SECONDS]
    [--class NAME]... [--binary-sha SHA] [--sut-commit COMMIT]
python3 replay.py --run DIR --inspection FILE --out DIR [--adjudications FILE] [--inputs DIR]
    [--packages DIR] [--provisional]
python3 fixed.py acquire --inputs META --dest DIR [--packages VULN] [--pins FILE]
python3 fixed.py score --inputs META --packages VULN --fixed DIR --observations RUN
    --binary BIN --binary-sha SHA --sut-commit COMMIT --out DIR [--pins FILE]
    [--typescript FILE] [--adjudications FILE] [--minimum N]
```

The benchmark defaults remain two workers and 120 seconds per query. A class filter still verifies the entire input manifest. Pin overrides are 64/40 lowercase hex characters. Selecting an existing binary requires `--no-build`. Partial replay must be explicitly provisional. The self-check command above is relative to `scripts/measure/`. `--pins` defaults to `fixed-pins.json` beside `fixed.py`. The published gate uses `--minimum 28` (also the default). Fixtures may specify a smaller positive integer.

| Command | Successful exit | Other exits and observable result | Binding rows or gate |
|---|---|---|---|
| `corpora list` / `verify` | 0 lists all / all selected pins verified | 2 invalid manifest, option or unknown key. Verify prints a row for every selected key and returns 3 if any local pin is missing, dirty or unverifiable. | run-001, run-011 |
| `run` | 0 with `status="ok"` | 2 usage, unreadable prediction or occupied output. 3 corpus or version pin mismatch. 4 command/guard error, changed executable or input. 75 host quiet. Pre-analysis refusals and late failures are distinguished in section 2. | run-002, run-010, run-012 |
| `diff-edges` | 0 with every change attributed and all checks passing | 2 input, binding, transport, schema or classifier-interface refusal. 1 after comparison for an unattributed change row, ambiguous **edge record**, forbidden stray, or failed validation. A classified ambiguous **change bag** may pass. An unmatched record alone may pass. | cmp-020, cmp-053, join-002, join-003, join-004, join-008 |
| `diff-stats` | 0 all deltas allowed | 1 forbidden delta. 2 admission, decoding, ambiguous counter spelling or malformed allow-list. | cmp-045, cmp-046, join-021 |
| `oracle ts-calls` / `ts-edges` | 0 grading completed, including recorded WRONG, UNDECIDED or INADMISSIBLE verdicts | 2 malformed input, bad selector, receipt or incompatible arms. 3 checker pin or passing self-check missing/stale, before starting Node. 1 invalid module result, exception or checker execution failure. 75 host quiet. | oracle-017, oracle-018, oracle-027, oracle-029 |
| `oracle/selfcheck.cjs` | 0 only on complete expected agreement | 1 missing test coverage, disagreement or module/checker error. 2 invocation. 3 checker pin. Guard exit 75 passes through. A failure publishes no passing receipt. | oracle-018, G-selfcheck |
| `review-diff` | 0 all selected rows measured, regardless of findings | 1 if any selected ordinal failed or was refused. 2 invalid invocation/manifest before enumeration. 75 on host quiet, listing each interrupted or unstarted selected ordinal as refused. | review-008, review-011, review-015 |
| `gate` | 0 prospective PASS | 1 completed failing metric/step or completed retrospective report (never PASS). 2 malformed gate, failed admission, prediction/reuse refusal. 75 on guard refusal. Record child exit separately. | gate-001, gate-005, gate-006, gate-007, gate-010 |
| Benchmark `run.py` / `replay.py` | Existing harness success, 0, including per-case errors labelled in results | Existing argument errors 2, fatal execution/binding errors nonzero (1 for Python exceptions). S6a changes no command exit. | bf-001, bf-014, G-findings |
| `fixed.py acquire` / `score` | 0 complete acquisition accounting / complete scoring meeting minimum | 2 invalid arguments, binding, manifest or pins file. 1 incomplete batch or score below minimum. Acquisition reasons are per entry and do not abort the batch. Score guard refusal 75. | bn-001, bn-025, bn-026, G-negatives |
| `memguard.sh LIMIT_GB command ...` | Child status | 75 live marker, 76 RSS observation unavailable, 137 RSS limit enforced. The caller records the underlying exit even where its CLI maps it to 4 or 1. | run-012, sol2-run-001 |

## 2. Corpora, binding and run receipts

`corpora.json` is `{"schema":"prism.measure.corpora/1","corpora":{KEY:entry}}`. Each entry declares `name` (display string), `kind` (`git` or `tree`), `url` (public upstream or null), `commit` (full 40-hex Git commit or null), `root` (query subdirectory, `.` allowed), `history` (boolean), `local` (ordered array of relative paths to the checkout or extracted tree), `manifest_sha256` (perf-cloud regular-file manifest digest or null), `cloud_asset` (release asset basename or null), and optionally `archive_sha256` for the transport. A git entry requires a commit, a tree entry the manifest digest. A tree entry uses manifest verification even if its fixture lives inside the kit checkout. Asset names and abbreviated commits confer no pin. Each SB package is its own tree entry and run, keyed `SB.<class>.<entry>`, pinned from the acquired package binding. No aggregate package root substitutes for a package. The 4b fixture uses `R4B`, pinned to its source-only tree when S2 lands, with truth, controls and outputs outside the query root. For `R4B` set `name="4b mechanism fixture",kind="tree",root=".",local=["scripts/measure/fixtures/repro-4b/repo"],history=false,url=null,commit=null,cloud_asset=null`. For an `SB.<class>.<entry>` entry use the key as `name`, `kind="tree",root=".",local=["<class>/<entry>/src/package","pkgs/<class>/<entry>"],history=false,url=null,commit=null,cloud_asset="secbench-recall-v1"`. Bind each package's own regular-file manifest digest. The benchmark dataset revision identifies metadata independently of these tree pins.

Lookup tries `--corpora-root` in order, then `PRISM_MEASURE_CORPORA` split by `os.pathsep`, then the repository root for in-repo fixtures. Within each, try `local` in order and append `root`. The first existing directory is verified or refused. A found Git checkout must match its commit and have empty staged, unstaged, untracked **and ignored** status under the query root. Sibling dirt is outside the check. A Git-less extraction needs the manifest digest, even for an entry normally obtained from Git. Never fall back from a dirty or wrongly pinned Git checkout to archive verification. Use `manifest_hash` from `scripts/perf/cloud/fetch_corpora.py` byte for byte for archive verification (regular files, bytewise path order, GNU escaped SHA-256 lines). `history` describes availability for historical diffs. `list` prints `key<TAB>name<TAB>kind<TAB>pin<TAB>history<TAB>cloud_asset`, then one sorted row per key. `verify` prints `key<TAB>status<TAB>detail` with status `ok` or `refused`.

The following pins identify the adopter populations. `local` for each Git entry is `["<checkout>","bench-repos/<checkout>"]`. `name` is the literal checkout basename. Use the public upstream repository as `url`. Copy X, T, D and M asset names and available manifest digests from `scripts/perf/cloud/corpora.json`. EX has asset `express-18e5985b.tar.gz`. The registry does no acquisition.

| Key | Checkout / query root | Commit | History |
|---|---|---|---|
| X | excalidraw / . | `0642e72cfa2d9a71198200e52f37399384610ee3` | false |
| EX | express / . | `18e5985b8a9d5e8423db0a9121f22bdaecd5b120` | false |
| T | TypeScript / src | `7964e22f2b85f16e520f0e902c7fd7b6f0c15416` | false |
| P1 | apache-superset / . | `7519fa8b6f5a145097d51e36579240e872044266` | true |
| P2 | saleor-saleor / . | `cc4aea76e397b1dead74363c266c579ae808770f` | true |
| G1 | caddy / . | `77e9ce7404c4a76853e101a9f5687a929ee56654` | true |
| G2 | etcd / . | `61d518f55effaf5edcedcb2a696504795b4fa7bd` | false |
| R, RS (same entry) | ripgrep / . | `82313cf95849bfe425109ad9506a52154879b1b1` | false |
| G (historic 4b) | guava / guava/src | `2b2452a0f2102f820a758123b9224c685a03b9f3` | false |
| GIN (lane 9 control) | gin / . | `d75fcd4c9ab260e5225de590f1f0f8c0e0e12d11` | false |
| D | django / . | `e8cff2921ba169d806a15de18304f431f12700f4` | false |
| M | mypy / . | `5ef090270579013c337a9bcc79e2c58417f09538` | false |

For the table above the respective public upstream slugs are excalidraw/excalidraw, expressjs/express, microsoft/TypeScript, apache/superset, saleor/saleor, caddyserver/caddy, etcd-io/etcd, BurntSushi/ripgrep, google/guava, gin-gonic/gin, django/django and python/mypy. Resolve a slug to its HTTPS GitHub repository URL. The remaining pinned Git entries follow the same `local` rule. Use the keys, public repository slugs and commits below. The checkout basename is the last slug component for the first seven and the literal registry key for the last four. Query root is `.` except C (`src`). The campaign clones carry history.

| Key | Public repository | Commit |
|---|---|---|
| GO | spf13/cobra | `ad460ea8f249db69c943a365fb84f3a59042d54e` |
| PY | pallets/flask | `36e4a824f340fdee7ed50937ba8e7f6bc7d17f81` |
| C | redis/redis | `d2d3390d0c4d01ab7bfb46054ad0d5003d63c11b` |
| leveldb | google/leveldb | `7ee830d02b623e8ffe0b95d59a74db1e58da04c5` |
| click | pallets/click | `8a1b1a33d739be05b7e91251e3c0dde77c5e152f` |
| httpx | encode/httpx | `b5addb64f0161ff6bfe94c124ef76f6a1fba5254` |
| clap | clap-rs/clap | `8387c812c4b9726474b8407f81ec97f0acf570ed` |
| apache-answer | apache/answer | `6744295fa888e0621fc9f9a8ace04f1a4daf8554` |
| calcom-cal.com | calcom/cal.com | `f972feefa1376cf47631177231c671da9a9dd09f` |
| directus-directus | directus/directus | `c40ab79f2c4ac573b2c7424b2c591fd79f7b116f` |
| miniflux-v2 | miniflux/v2 | `faecfa2fac61a91f4f69550bb1afd506c4b51749` |

Public subdirectory variants use the inventory's literal variant name, pinned checkout and named suffix. Require a complete commit or extracted-tree digest before registration. Use null for unrecorded cloud assets/digests and false for history unless a complete history was verified. Use the recorded literal asset basename and transport hash where available. A source-available or unpinned inventory mention does not authorize distributing an archive. T's src-only cloud copy with a null manifest digest is unverifiable until its tree pin is supplied. Lane 10 uses P1/P2/X/T/G1/G2/R, Git-clean at those commits, for S0 against baseline, S1 against S0 and S2 against S1. Lane 9 uses X, T and individual SB packages. Its plan calls the Go control “G”. Pass registry key `GIN` for that gin checkout. The older 4b registry key `G` selects guava.

**Input digest.** Walk the query root without following symlinks. Omit every path component exactly `.git`. Include every regular file and every link, including directory and dangling links. Sort by unsigned pathname bytes (relative POSIX spelling, `os.fsencode`). Feed SHA-256 the concatenated records `type || u64be(len(path)) || path || u64be(len(value)) || value`. Type is the single ASCII byte `F` or `L`. Value is raw file bytes or the link target's raw text. Empty directories, timestamps and permissions supply no bytes. Reject unreadable members and special files. A link binds its text only. The oracle reads regular sources inside its authenticated root. The start and end checks cannot detect a transient edit restored between checks.

**Execution.** Check arguments/output, the corpus pin, starting input digest and prediction's raw SHA-256 before any invocation including `--version`. Hash the executable before version, before each analysis and after the last command. A byte change fails the run. The guard runs version and each command serially with default limit 8 GiB (positive integer override). Accept `--expect-version` only when it equals the version token or a prefix of the reported commit token, case-insensitive for hex. A version mismatch starts no analysis. Pre-analysis host-quiet (including the first analysis being refused before launch) yields 75 and no receipt. After any analysis has launched, stop at the first nonzero, keep completed/partial outputs and write a failed receipt, including on a late 75. After the last attempted analysis recheck the input. A changed digest gives status `input_changed` and exit 4 except that a late guard refusal retains exit 75. Unreadable final input, executable drift or other failure gives `failed`. A standalone run retains the prediction digest captured at entry if the prediction changes.

Each argv starts `BIN nav --no-cache` followed in order by `dfg-stats --repo ROOT`, `call-stats --repo ROOT`, `dfg-stats --repo ROOT --edges` and, with `--sites`, `call-stats --repo ROOT --dump-sites`. Start with a clean product environment: remove inherited `PRISM_*` and guard-bypass settings. Set `PRISM_EDGES_BYTES=1` only for the edge dump unless `--no-bytes`, then leave it absent. Only that command receives explicit trace overrides. Accept distinct `PRISM_[A-Z0-9_]+` names with a value (split on first `=`), except the reserved byte switch. Pass only the host runtime variables `PATH, HOME, TMPDIR, TEMP, TMP, SYSTEMROOT` when present and fix `LC_ALL=C`. Record the effective fixed and product settings. Resolve the guard from `--memguard` or `scripts/memguard/memguard.sh`. Its quiet helper is repository-relative, its log uses `MEMGUARD_LOG` or the XDG state default. Never copy the host's kill log. Exit 75 returns control to the caller for waiting. This CLI takes no quiet marker.

| Command id | stdout | stderr |
|---|---|---|
| version | `version.stdout` (full bytes), trimmed UTF-8 in `binary.version` | `version.stderr` |
| dfg-stats | `dfg-stats.json` | `dfg-stats.stderr` |
| call-stats | `call-stats.json` | `call-stats.stderr` |
| edges | `edges.jsonl.gz` | `edges.stderr.gz` when tracing, otherwise `edges.stderr` |
| sites (optional) | `sites.jsonl.gz` | `sites.stderr` |

`receipt.json` has schema `prism.measure.receipt/1`. Required top-level fields are `kit_commit,start,end,status,binary,corpus,prediction,options,version_command,commands,guard,host,artifacts`. Times are UTC ISO 8601 ending in `Z`. Digests are 64 lowercase hex digits over exact bytes. `binary={path,sha256,version}`. `corpus={key,commit,root,pin_check,manifest_sha256,input_sha256,end_input_sha256}` uses the resolved query root, the bound commit or null, `git-clean` or `manifest` and nullable final digest on failure. `prediction` is null or `{path,sha256}`. `options={bytes,sites,cache,trace_env}` uses booleans, `cache="no-cache"` and a name/value object. `guard={path,limit_gb}`, `host={platform,cpus,env}` where `env` records the passed host allow-list and `LC_ALL`. `commands` contains the analysis attempts in order, each `{id,argv,env,output,output_sha256,bytes,stderr,exit,start,end}`. `env` gives the product overrides for that command. `output_sha256` and `bytes` refer to the stored transport. `artifacts` maps every produced stdout/stderr relative filename (including version stderr and compressed traces) to `{sha256,bytes}`. `version_command={argv,env,exit,start,end,stdout_sha256,stdout_bytes,stderr}` binds the guarded `BIN --version` separately, retaining its exact stdout in `version.stdout` for verification. `binary.version` is the trimmed display string. Paths printed in a *local receipt* locate the actual run, binary and corpus. Published examples use relative names.

**Admission.** Require a supported successful receipt, one successful command for each requested output, matching options/argv/outputs, every required artifact listed, sizes and hashes of **all** artifacts intact, and complete decodable selected inputs. Reject absolute or escaping artifact references. Compare the same input digest and analysis options after normalizing binary and query-root pathname to their roles. Only binary identity/version and declared trace environment may vary. Location, times, guard and prediction are provenance, outside analysis-option equality. An explicit line projection never waives differing `options.bytes`. Different wire capabilities from older binaries run with the *same* options can be compared in explicit line mode. Ordinarily data refusals produce no successful certificate or partial verdict set.

**Text and transport.** JSON documents and JSONL records are strict UTF-8, no BOM, duplicate object names, non-finite numbers or trailing garbage. Split JSONL on byte LF only, accept CRLF and an EOF-terminated last record. Ignore lines containing only ASCII JSON whitespace. Preserve Unicode code points without case folding or normalization. Validate the expected record type. Read plain or gzip by declared transport (`.gz`), concatenate every gzip member before decoding and verify all trailers through EOF. A matching compressed digest alone cannot admit an incomplete stream. Writers emit one compact object per LF-terminated line. Except for explicitly specified legacy/spec bytes, canonical JSON for sorting or hashing is UTF-8 `json.dumps(x, ensure_ascii=False, sort_keys=True, separators=(",",":"), allow_nan=False)`, without a trailing LF. JSON integer fields exclude booleans. Files with a single top-level JSON object may be pretty-printed.

## 3. Edge difference, trace join and counters

The edge wire object has `from,to,confidence,doubt` and optional `kill_line`. Each endpoint has `file,line,path:{base,fields},access` and optional `start_byte,end_byte`. File and path base are strings, fields an **ordered** string array (including literal `[]`). Access is `def` or `use`. Lines are positive integers. Supplied byte offsets are nonnegative integers or null, half-open, start ≤ end when both present. Zero-width anchors and byte zero are valid. Confidence is `exact` or `nameonly`, doubt null or a nonempty doubt string, kill line null/absent or a positive integer. Exact requires null doubt. A synthetic nameonly/null is compared literally. Additional object members, including owner, survive in the raw dump but do not enter identity or attributes. Unknown shapes of required fields refuse the stream.

**Key and cancellation.** `K={from:E,to:E}` with `E={file,base,fields,line,start_byte,end_byte}`. Automatic mode checks every endpoint across both complete streams. All carry both bounds → `bytes`. All carry neither → `line`. Anything mixed or partial → exit 2. An empty arm adds no constraint. For two empty streams choose the shared receipt option (`bytes` unless `--no-bytes`). Explicit `bytes` requires both bounds on every observed endpoint, including both ends of a zero-width span. Explicit `line` replaces both bounds with null on every endpoint and applies to both runs and edge-record match keys. A collision is a distinct projected key with multiplicity >1 on **either** arm before cancellation, including unchanged keys. Report zero collisions in byte mode.

For each ordered endpoint pair form two multisets of `A={from_access,to_access,confidence,doubt,kill_line}`, normalizing absent kill line to null. Cancel `min(B[A],H[A])` for every **whole** attribute tuple. With residual lengths `b,h`:

| Residual | Emitted change rows for K |
|---|---|
| Both zero | None. Keep the cancelled occurrences in the unchanged index. |
| One empty | One `added` or `removed` row containing the entire nonempty bag, including distinct attributes. |
| Both nonempty, larger side homogeneous (either side homogeneous on a tie) | Pair every smaller-side occurrence with the single available larger-side label. Emit one `label_moved` row per distinct ordered transition, `count` equal to its multiplicity. Put any surplus in one one-sided row. |
| Otherwise | One `ambiguous` row with both complete residual bags. Only a class returned for this entire row attributes it. |

`changes.jsonl` rows have exactly `{id,key,kind,count,base_bag,head_bag,records,class,error}`. Bags are arrays of A, including repeated copies. For a move each side contains `count` copies of its transition attribute. The fixture's `from` and `to` transition assertions read the first element on each side. `count=max(len(base_bag),len(head_bag))` for every kind, including the ambiguous case. This last number is a reporting unit, with no inferred pairing. Sort by canonical key bytes, kind order `label_moved,removed,added,ambiguous`, then canonical bags (members sorted by canonical bytes). Assign one-based integer IDs in that order. `class` and `error` are nullable strings. `records` contains attached record objects in base-then-head stream order. `--list` prints the same JSON rows before the final verdict. `classes.tsv` contains `class<TAB>keys<TAB>occurrences`, one row per declared class, sorted by name, including zeros.

**Conservation.** On each arm, at each key and for every A, cancelled multiplicity plus the sum of that arm's published bag multiplicities must equal the decoded input multiplicity. Check this before publishing, including for classified ambiguous bags. Two more checks equate the emitted row counts to `attributed+unattributed` and the sum of row `count` to `occurrences`. Failure is an internal comparison failure (exit 1, failed certificate). Stream into disk-backed sorted/grouped counts or equivalent bounded storage. The 100,000-distinct-key case requires Python `tracemalloc` peak strictly below 96 MiB. Changes, indexes and validation views may be streamed or backed by disk.

`certificate.json` has schema `prism.measure.certificate/1` and the following required fields.

| Fields | Unit and computation |
|---|---|
| `key_mode,base_edges,head_edges,changed_keys,collisions` | Mode, decoded occurrences on each arm, distinct changed keys, collided keys. |
| `attributed,unattributed,ambiguous` | Counts of change **rows** with a class, without a class, and with kind `ambiguous` (even if classified). |
| `occurrences,by_kind,base_residual,head_residual` | Sum of `count`, sums of `count` by the four kinds with zeros, and sums of each arm's bag lengths. The matrix's object-valued `truth.occurrences.{base,head}` tests the last two, not the scalar. |
| `by_class` | Map every declared class to `{keys,occurrences}`. Count a key once within that class. Keys may be shared between classes. |
| `records,unjoined,trace_by_tag,mechanisms` | `records={attached,stray,ambiguous,unmatched}` counts individual **edge records**. `unjoined` counts all other typed records. `trace_by_tag` maps each tag to those four counts plus `unjoined`. `mechanisms` is the legacy count-only head projection below. |
| `validation,failures,result` | Validator diagnostics in return order, all gate failures (attribution, join, validator, conservation), and `pass` or `fail`. |
| `inputs,changes_sha256,classifier_sha256` | `inputs={base:{run,receipt_sha256},head:{run,receipt_sha256}}`, paths relative to the certificate directory. Hash the complete receipt files, output changes stream and loaded classifier file. |

**Classifier API.** Load one local Python file. Required `CLASSES` is a list of distinct nonempty single-line class names (no tab), possibly empty. `classify(change, context)` returns exactly one declared string or `None`. An exception or undeclared result sets `class=null`, records the exception type/message or invalid result in `error` and counts one unattributed row. Continue classifying. Generic classes: **none**. Line shifts, neighbouring edges, owner changes and promotions need a lane's evidence.

Optional `parse_trace(line, arm)` receives each decoded line of that arm's edge-command stderr in file order, with LF if present, and `arm` equal to `"base"` or `"head"`. Return `None` for an unrecognized line or one JSON-compatible dict `{arm,tag,kind,key,...}` with the supplied arm, nonempty tag and kind, and a dict key. Preserve extra payload and every repeated record. An exception, malformed result or contradictory arm refuses the input (2), with arm and line, before classification. A missing parser gives zero records. `STRAYS` may be `"fail"` (default) or `"allow"`. It controls only stray edge records. An ambiguous edge record is an unresolved join and independently fails the comparison, even when all changes have classes. An unmatched record is reported and reaches validation. A lane can reject any unmatched record there.

An `edge` record's key is a nonempty subset of `{from:Epart,to:Epart}`. Use the flat E names above, require at least one supplied endpoint field, omit unknown fields, never fill one endpoint's file from the other. Match supplied fields conjunctively against **the same occurrence** on that record's own arm, before cancellation, with key and record both projected to the selected mode. If projection removes every discriminator, all occurrences on that arm are candidates. Zero matches → `unmatched`. Two or more → `ambiguous`, including identical copies or a cancelled copy beside a residual. Exactly one → `stray` if that occurrence cancelled, otherwise attach to the single row whose side bag consumed it, including a whole ambiguous bag. Never use an opposite-arm occurrence or assign the record to two transitions. Record ambiguity and change ambiguity have independent counts.

All other kinds are unjoined. `write` is per-write evidence, `owner` covers one owner or range, `counter` carries a mechanism count/prediction, `node` names one def/use span. Additional lane kinds follow the same unjoined rule. For lane 10 the parser accepts these nine tab-separated fields:

```text
subscript-site<TAB>file<TAB>owner<TAB>line<TAB>start<TAB>end<TAB>def|use<TAB>recorded path<TAB>chain path|refused|-
```

Lane 10's spelling-leak gate sets `PRISM_SUBSCRIPT_TRACE=all` on both products to emit all def/use spans, including isolated ones. The value `1` emits only subscript sites and cannot supply that inventory. Compare node inventories only when both arms supply one. In S0 against the pre-trace baseline, compare the edge and counter dumps and record the node comparison as unavailable. From S1 against S0 onward, verify `all` and trace custody on both receipts. A missing expected inventory fails the lane's validation.

Return `{arm,tag:"subscript-site",kind:"node",key:{file,owner,line,start_byte,end_byte,access},recorded_path,chain_path}`, preserving the last two strings verbatim, including `refused` and `-`. Identical spans, nodes without an incident edge, duplicates and zero-width nodes all reach validation. Do not promote a node to evidence for an edge. `rd-census` uses the destination file and both rendered places. A parser which cannot recover a structured field unambiguously omits that field. `rd-alias-cert` and `rd-alias-move` may be writes. The `rd-census-owner` guard/arrow/loop/parameter forms are owner records with their own field counts. The `own-*` and `c-prune-*` series can supply owner/counter payloads. The parser chooses the meaning for the lane.

| Python name on `context` | Value available during `classify` |
|---|---|
| `corpus_root,base_run,head_run` | `pathlib.Path` for the authenticated head query root and each run directory. On first access to `corpus_root` verify the current digest before returning the path. Recheck after validation if either callback accessed it. Drift refuses the comparison. Pure dump comparisons require no live source read. |
| `base_receipt,head_receipt,key_mode` | Decoded receipt dicts and mode string. |
| `records(change)` | Ordered sequence of the attached record dicts for `change["id"]`. |
| `all_records,unchanged_keys` | Reiterable sequence of all typed record dicts and of `{key,bag}` for common cancelled occurrences, including cancellations on changed keys. |
| `counter_deltas,call_counter_deltas,stats_deltas` | Native dotted DFG and call names mapped to numeric head-minus-base for changed leaves. `stats_deltas` gives the full rows from `diff-stats --which both` with presence, including a missing-versus-zero change. Missing deltas can be read with `.get(name,0)`. |

Call optional `validate(bound) -> list[str]` **once**, after every classification, even for zero changes, unclassified changes, strays or record ambiguities. `bound` provides `corpus_root,base_run,head_run,base_receipt,head_receipt,key_mode,unchanged_keys,counter_deltas,call_counter_deltas,stats_deltas` as above, `changes` (every final row), `records` (all parsed dicts in stream order), `joins` (parallel sequence of `{arm,line,status,change_id}`, status also admits `unjoined`, null id unless attached), and `unjoined` (the unjoined record dicts). Views are read-only as inputs to the kit. No extra fields are required of an adopter. A thrown exception or invalid validator return becomes a validation failure (1). Lane 10 vetoes an added or relabelled edge whose byte-identified endpoint component sequences differ at a static position (`cross-key`). Comparing the two inventories, a changed path at bytes with no static-key subscript occurrence is a `spelling-leak`, including on isolated spans. From its reads slice, a newly introduced zero-width use of a lane-built path, or of a path with a static-key occurrence on that line, is an `orphan-anchor`. Its validator checks those counts for zero and applies the lane's later dynamic-edge bound from its census. A failure can coexist with zero unattributed rows.

**Counter table.** `--which` defaults to `both`, DFG then call, each file in its own table. Descend objects only, include finite numeric leaves, skip booleans, null, strings and arrays. A member name containing a literal dot, an empty member or an ambiguous flattened spelling refuses a numeric path (2). Sort names by UTF-8 bytes. Compare the union of names, distinguish absence from zero, and use zero only in delta arithmetic. Each metric row is `{which,key,base,head,delta}`, null on the absent side. `--allow` is UTF-8, comments from `#` to EOL, surrounding whitespace stripped. Each nonempty line is a native dotted key or a complete component prefix followed by `.*`; `x.*` matches `x.a` and `x.a.b`. Patterns apply independently to each selected file. With no allow file, enumerate deltas without restricting them (descriptive mode). A supplied empty file permits zero changes. G-repro requires zero deltas in **both** files. Print `counters base=N head=N changed=N` per file and `  key: b -> h (+d)` per change, `missing` for absence, explicit sign on d. Use numeric decimal spelling (`str` of the decoded number). Both tables retain their `dfg` or `call` heading.

A numeric leaf present on both arms with equal value contributes no change even when one spelling was a float. Allow-list segment names exclude `#` and `*` and cannot be empty. An allow file covers presence-only moves too.

**Comparator stdout and G-repro.** The first line is always the following ASCII with single spaces, ungrouped unsigned decimal integers and LF:

```text
edges base=N head=N added=A removed=R label-moved keys=M
```

N is total occurrence count independently in each arm. For historical compatibility A and R count positive and negative **key multiplicity** differences respectively, summed separately. M counts common keys with unequal complete attribute bags. Thus the same key can contribute to both M and a surplus. `by_kind` is the residual calculation used for attribution. Then print `occurrences: added=A removed=R label_moved=M ambiguous=U`, `collisions keys=K` only in line mode, the class table, `trace records:` and one sorted `tag<TAB>attached<TAB>stray<TAB>ambiguous<TAB>unmatched<TAB>unjoined` count line per tag. The separate `trace mechanism lines:` line is specified below. Follow with any requested listing, each failure on its own diagnostic line and a final `PASS` or `FAIL` with LF.

For the legacy line count six-field `rd-census` records on **head**. Split each place at its last `@` and read the decimal line before `:`. The distinct key per mechanism is `(destination_file,norm(source_path),source_line,norm(destination_path),destination_line)` with `norm(s)=re.sub(r"[^A-Za-z0-9_$]", "", s)`. Count a key once per mechanism, including unchanged keys. The example parser supplies `mechanism` and `legacy_key` (this five-item JSON array) on eligible records, including count-only `counter` records. This lossy spelling serves display only.

The historical emitter is `print("trace mechanism lines:", dict(other))`. The suffix is a Python dictionary representation, with one space after the prefix, comma-space separators, quoted keys, colon-space before decimal counts, and LF. Empty is `trace mechanism lines: {}` plus LF. The old counter inserted tags while iterating sets, so equal counts alone cannot reproduce its bytes. Optional classifier constant `LEGACY_MECHANISM_ORDER` maps corpus key to an ordered list of tag strings. The #786 example supplies the three frozen display orders below. Format independently measured nonzero counts in that order, omit absent tags and append extra tags in lexical order. Other corpora/classifiers use lexical order. Never fill counts from the reference. These are the six literal comparison lines, arranged X, EX, T, with LF after each:

```text
edges base=80033 head=80033 added=0 removed=0 label-moved keys=0
trace mechanism lines: {'d2c': 194, 'capture-body': 894, 'capture-region': 643, 'kill:update': 32, 'loop-exit': 330, 'd3-loop': 169, 'guard-deleted:f': 2, 'arrow-body-node': 21, 'capture-param': 1}
edges base=7445 head=7445 added=0 removed=0 label-moved keys=0
trace mechanism lines: {'capture-region': 65, 'd2c': 9, 'capture-body': 36, 'loop-exit': 84, 'd3-loop': 13, 'guard-deleted:a': 4, 'kill:update': 1}
edges base=388055 head=388055 added=0 removed=0 label-moved keys=0
trace mechanism lines: {'capture-body': 1067, 'capture-region': 86489, 'd3-loop': 688, 'loop-exit': 2378, 'd2c': 769, 'd3-use': 8, 'capture-param': 114, 'kill:update': 115, 'guard-deleted:a': 18, 'do-cond': 17, 'kill:for-increment': 19, 'arrow-body-node': 10, 'owner-refuse:parse': 56, 'guard-deleted:f': 7, 'owner-refuse:one-node': 2}
```

The identity control counts X=80,033, EX=7,445 and T=388,055 on each arm, both key modes, with zero changes and zero counter deltas. Use a count-only record for non-attributing mechanisms in the #786 port. The same example file exports `STRAYS="allow"` for the identity controls and vetoes each stray in `validate` when `bound.head_receipt["corpus"]["key"]=="R4B"`. In R4B only `kill:alias` and `alias-d3` census edges join. Exact→nameonly/alias_unstable requires exactly `{kill:alias}`, exact→nameonly/sameline exactly `{alias-d3}`. The fixture has 115 source files and 950 edges on each arm. Its independent table labels are 22/14/103 (those two classes/unchanged). Preserve `base_inferred` on the 14 rows whose table comments supply the base label. The 42 dump-derived siblings (1/41) contribute to the whole-run 23/55 class and attachment counts, 78 changed keys and zero unattributed. Preserve their different provenance. Controls c1–c5 respectively require (stray 1, unattributed 1), (unattributed 1), (unattributed 2), (stray 1), (unmatched 1, unattributed 1). The trace-mutated control receipts must bind the altered trace before comparison. Stale hashes exercise admission (2), not the specified join failure (1). For part (a) the bound head is `1f2ad1cc` against `4f9e3a3e`. The historical count files came from `32ad7c09b`. On a trace-count mismatch try only the predeclared count fallbacks `32ad7c09b` then `3510ed4f2`, recording each actual head. If none reproduces the counts, report the measured rows and the count reproduction as unmet. A difference in trace counts with unchanged edge totals and identity does not establish an edge-comparator regression. DESIGN section 8 separately covers a toolchain failure to build the pair. Its line-only #783 control and later byte-bearing labelled-pair replacement require the reason and a bound prediction.

## 4. TypeScript oracle

**Selection and admission.** Use TypeScript 5.9.3 from `--typescript`, then `PRISM_TYPESCRIPT`, then the gate-inputs install resolver. Before starting Node verify `lib/typescript.js` SHA-256 `3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675`, the containing package tree pin `7e02162c902e5c29ec19fefc573ad55b8ed15fcbda940e737f297c53e9c41f54` (the `treeDigest` algorithm of `scripts/gate-inputs/acquire.mjs`), and the passing self-check key below. Verify **all** artifacts of each input run, including in census mode. A bad receipt/dump is a command refusal. A successful receipt and current source with a different whole-root digest yields an INADMISSIBLE verdict for **each selected claim**, including when the unrelated file that changed supplied no endpoint. Hash before and after reading source and withhold all prior substantive verdicts if it drifts.

`ts-edges --all` grades every wire edge occurrence independently. `--changes` reads S2's `changes.jsonl` and its sibling `certificate.json`, verifies `changes_sha256`, both named receipt hashes and pair admission, and requires `--run` to be the selected arm's receipt. Infer the arm from receipt identity if unique, otherwise require `--side`. `--side`, `--kind` and `--class` apply only to `--changes`. Repeat kind/class for union within each filter, intersect the filters. Select by key **and the selected arm's residual bag, with multiplicity**. Empty selected bags contribute no claims. Moves select the corresponding arm, ambiguous bags select that arm's actual occurrences without inventing transitions. Consume each occurrence at most once in run-file order among indistinguishable wire projections. Return full original rows, including extras, with actual byte spans. If a proper subset of equal key/attribute copies must be selected and the candidate raw rows have distinct byte identities, refuse the selector (2, `ambiguous-occurrence`) before grading. A byte-mode selection or a complete census can establish that population. This avoids assigning a verdict to an invented original row. A bag demanding absent copies is refused. Use `--side base --kind removed --relation spread` on the original base run to grade lane 9's LOST rows. Use the head and `--relation binding` for added argument claims. The certificate records whether the original differential passed.

**Program and offsets.** Root names are all regular `.ts,.tsx,.mts,.cts,.js,.jsx,.mjs,.cjs` files within the bound root, including in-root declaration files. Sort paths as in section 2. Omit `.git` and skip symlinks. The compiler host serves authenticated in-root files, and only pinned compiler lib files for the default-lib program. Resolve relative in-root imports and aliases using the pinned compiler. An external resolution, including a relative import crossing the root, has no admissible local declaration. Do not load a corpus `tsconfig`, plugin, package code or unbound declarations. Compiler options are `{target:"ES2022",module:"CommonJS",moduleResolution:"Node10",jsx:"Preserve",allowJs:true,checkJs:true,noEmit:true,skipLibCheck:true,strict:true,noLib:true,types:[]}`, using the corresponding TypeScript enum values. The second program changes only `noLib` to false and gets the default libraries from the pinned checker. Keep these two program/checker/node graphs separate.

Coordinates use original UTF-8 bytes, inclusive start and exclusive end, line one-based. Decode strictly, retain BOM, CRLF and astral character widths in the byte-to-UTF-16 map. Use raw token boundaries (`getStart(sourceFile),getEnd()`) and decoded identifier spelling. Missing/partial bounds, unreadable or missing file, reversed/out-of-range bounds and middle-of-character offsets are INADMISSIBLE. On authenticated well-formed bounds a different identifier gives WRONG for a graded identifier claim. Valid punctuation, whitespace, a zero-width anchor, or a span with no unique identifier/call yields UNDECIDED with its reason. Check source binding first, then bounds, then relation-specific evidence. For structured paths requiring a property-chain proof the built-ins decline if they cannot identify the claimed token uniquely.

**Built-in proof rules.** For each admitted occurrence select one property. `use→def` proposes `argument_binding` only for an actual value in an ordinary call or construction and a formal declaration name. `def→use` proposes `lexical_binding` only for a declaration/read claim. Other role/relation pairs give UNDECIDED `relation-not-graded`. An explicit lane relation selects the registered question below. A known grammar shape alone confers no verdict.

| Relation / property | CORRECT | WRONG | UNDECIDED boundary |
|---|---|---|---|
| `argument_binding` / `selected-signature-slot` | A unique in-root callable, selected declared signature and runtime argument slot identify precisely this formal's file and identifier bytes. | Admissible unique resolution proves a different local callable, position or overload formal. | External/unresolved/ambiguous owner, no declared formal, destructuring requiring a leaf mapping, a rest formal, spread actual or preceding spread, unsupported invocation grammar. |
| `lexical_binding` / `declaration-and-read` | Both tokens identify one declaration and destination is a value read. | Distinct uniquely resolved declarations, different claimed identifier, or destination is a write, declaration target or type position. | Multiple declarations of one symbol, unresolved symbols, unknown read role, unsupported structural path. |
| `call_target` / `callee-declaration` | The exact claim names the sole matching local callee declaration by file, name and **start line**. | A single local declaration is established and its identity differs from the claim. | No declaration, external/ambiguous owner, multiple declarations matching the claimed triple, or no unique full-span call. |

Argument positions exclude the TS-only `this` parameter. Only an actual in the selected call's argument list counts; use the innermost list containing the use, stop at intervening callable bodies and type arguments. Parentheses and TS expression wrappers preserve the slot. A method, constructor, `super`, optional call, recursive call, resolved alias or anonymous arrow may meet the same proof. A later spread/rest cannot disturb an earlier fixed slot. An overload chosen by `getResolvedSignature` within **one** underlying callable binds the selected signature bytes, including a tie settled by compiler declaration order. The verdict explicitly grades that signature, leaving runtime implementation correspondence unproved. A union of possible runtime owners remains ambiguous even if the checker returns a representative signature. A flow-narrowed unique owner qualifies. Tagged-template substitutions, JSX props and destructured/literal leaf mappings require a lane relation. In lexical binding use the value-side symbol of shorthand properties, the local import binding for a local alias read, and the resolved member declaration for property reads. Compound/update targets are writes for this read-only property. Receiver and computed-key expressions used to locate a write are reads. Destructuring assignment targets and for-of/in targets are writes, their evaluated defaults are reads. `typeof` in a type query is a type position. Intervening writes, execution, reachability and path feasibility receive no lexical verdict.

**Call-site differential.** The `sites` dump contains JSONL including `record_kind="call_site"` and `"go_level3_b1_candidate"`. Keep the latter's count separately and exclude it from call denominators. Reject an unknown or missing discriminator. A call record carries `source_span={file,line,start_byte,end_byte},origin,callee_text,caller={file,name,start_line,...},resolved_targets,exact_target,drop` and any other original metadata. Use `source_span` for the **whole call**, and `resolved_targets[].confidence=="exact"` as the exact claim, regardless of the convenience field `exact_target`. Target identity is `(function_id.file,function_id.name,function_id.start_line)`. `end_line` and resolution `kind` are retained as evidence. A site key is `(file,start_byte,end_byte,callee_text,origin,caller.file,caller.name,caller.start_line)`. Missing byte values stay null for accounting and cannot prove a graded claim. Do not loosely pair different site keys by last segment or by containment.

Group by site key and arm. Keep counts of site records, drops (nonnull `drop`), and a multiset of `(target identity,confidence)` over **all** records/targets, including duplicates and non-exact entries. A newly present or vanished site is `new` or `lost`. For a retained site, first cancel identical target/confidence copies. Changed identity bags with one distinct identity on each side and a one-for-one replacement give `retargeted`. If identity counts agree but confidence differs use `confidence-only`. All other target-multiset changes on a persistent site, including gaining/losing a copy, use `multi-target`. A pure change in drops, site-record counts or empty-target state also uses `multi-target` with its changed counts. Ignore record/target ordering. Preserve both arms' lost and gained target occurrence counts in every delta. Retain the original site record and target index for each claim. When several equal claims could supply a head residual, cancel copies in original ordinal/index order and select the remaining copies in that order. A census has no deltas. In a differential grade **new head exact residual claims** for all kinds, including promotions. A lost site or demotion remains in the delta with no head verdict. Use `--census BASE` for historic target grading. Comparing negative edge claims for lane 9 uses the base-edge selector above.

**Outputs.** `verdicts.jsonl` is one row per selected edge occurrence or exact target claim. Required fields are `{key,arm,relation,property,verdict,why,row}`, plus `target_index` (zero-based original array index) for call claims. `arm` is `base,head` or `census`. Edge key is S2's endpoint pair with actual bytes whenever known. A call verdict key is `{site:source_span,target:function_id}`. `row` is the original complete JSON object. The four verdict strings are `CORRECT,WRONG,UNDECIDED,INADMISSIBLE`. `why` is a nonempty explanation. `tuples.json` maps each requested/built-in relation to the four verdict counts, including zeros, and includes `relation-not-graded` if used. `denominators.json` gives `{mode,sites,base,head,census,selected,graded,ungraded}`. `sites` counts union distinct site keys, each present arm object has `{sites,site_records,exact_claims,nonexact_claims,nonexact_sites,dropped,other_records}`, `nonexact_sites` counts keys with a non-exact claim, `dropped` counts dropped records. Absent arms are null. Edge modes use null site/arm fields and count selected occurrences. `selected` counts original edge or target occurrences in scope (both residual sides for call diffs), `graded` counts verdict rows, `ungraded=selected-graded`. Verdict ordering is stable by site/edge key, arm, original record ordinal and target index. `deltas.jsonl` in call-diff mode has one row per changed site `{site_key,kind,base,head,lost,gained}`, where each arm contains `{site_records,dropped,targets}` and target bags are arrays of `{target,confidence,count}`. `--aggregate-only` suppresses row files while keeping counts, denominators and provenance. `oracle-receipt.json` binds `{schema:"prism.measure.oracle/1",inputs,selfcheck_key,mode,relation,selection,artifacts}`. Inputs are ordered receipt hashes (and selector/certificate hashes when used), selection lists arm and filters, artifacts maps produced data files to their SHA-256.

**Question interface.** A CommonJS file exports `relations: string[]` (unique names) and synchronous `grade(row,ctx) -> {verdict,property,why}`. `row` is the original wire edge or call-site object. Read the particular claim from `ctx.target_index` for calls. `property` and `why` must be nonempty strings. No suppress/skip return exists. Invalid output or an exception fails the entire invocation (1) and discards partial accepted output. One public module per invocation may dispatch several relations or require its own local helpers. Supply `--relation NAME` to select any declared relation (lane 9 uses `binding,literal,spread,builtin,argfollow`), independent of edge-role defaults. Without it, invoke a module for the row's explicit `relation` if declared, else for an inferred built-in relation it declares, else use the built-in or return UNDECIDED. A single declared extension is selected by default when `--question` is supplied and no built-in relation applies. Reject an explicitly requested unknown name before grading. Run admission/coordinate checks precede module dispatch.

| Exact `ctx` member | Contract |
|---|---|
| `ts,program,checker` | Loaded pinned TypeScript module, `ts.Program` and its own `ts.TypeChecker`. |
| `relation,arm,target_index,change` | Selected relation string, arm string, integer claim index or null, originating S2 change dict or null. |
| `identifier_at_bytes(file,start_byte,end_byte)` | In-root relative filename and two integers → unique `ts.Identifier \| ts.PrivateIdentifier \| null` at exactly that raw-token span. |
| `declarations(identifier)` | Node from this program → array of distinct `ts.Declaration` nodes in file/start/end order, empty for unresolved. For a local alias expose its local declarations. Use `ctx.checker.getAliasedSymbol` for an ultimate callable. |
| `enclosing_call(file,start_byte,end_byte)` | A file and half-open byte range → the unique innermost `ts.CallExpression \| ts.NewExpression` containing the **entire** range, or null for invalid/no unique match. Prefer an exact whole-expression match. For site grading require its full start **and** end equal the reported site span. For an identifier require membership in the actual argument list as described above. |
| `program_with_default_lib()` | Lazily return the same `{program,checker}` pair on repeated calls within an invocation. Re-find nodes in that program by file and span before using its checker. |

**Self-check.** Each question set owns `oracle/selfcheck/<case>/{repo/,input.jsonl,expected.jsonl}` (external modules use `selfcheck/<case>/` adjacent to their entry file under their own `oracle/`). Inputs are `{relation,arm,row,target_index?}` or the original wire row for built-in automatic dispatch. Expected rows carry `{key,relation,property,verdict}`, optional explanatory `why`. Test source/byte refusal, one case per verdict per graded property and the near misses in G-selfcheck. Admission/delta cases exercise their Python wrapper as well. Compare complete ordered keys, relations, properties, verdicts and cardinality; `why` must be present in output but its prose is not a golden. Validate absence of extra verdicts and zero buckets. Never grade by calling the analyzer to manufacture expected labels.

Write `selfcheck-<key>.json` in `PRISM_MEASURE_SELFCHECK_DIR` or the XDG cache subdirectory `prism/measure/selfcheck`. `key=sha256(canonical({checker_tree_sha256,compiler_options,default_lib_options,question_sha256,fixtures_sha256}))`. `question_sha256` covers the oracle driver, byte helpers, self-check runner, all built-ins and the selected module and local JS/JSON helper closure, keyed by logical relative paths and file bytes (section 2 framing, namespaced `kit/` and `lane/`). Restrict lane imports to pinned TS, Node built-ins and static module-relative local files. Refuse unresolved or dynamic external dependencies; modifying any graded helper invalidates the receipt. Compute `fixtures_sha256` with the section 2 framing over regular files, sorted by logical path `kit/<case>/<relative>` or `lane/<case>/<relative>`, including inputs, expected answers, sibling declaration fixtures under the case and all repo files. Reject linked fixtures. Cached self-check receipts live outside that file set. `components` is exactly the five-field object hashed to obtain `key`. Its option objects use the symbolic enum spellings above. `cases` counts case directories across both participating sets, `expected` counts golden verdict rows and `agreed` counts equal verdict rows. `{schema:"prism.measure.selfcheck/1",key,status:"ok",components,cases,expected,agreed,failures:[]}` is written atomically only after the independent goldens pass with `cases>0,expected=agreed` and required coverage. Verify every component before a grading invocation launches Node. For a custom module require both built-in and its own passing checks (the combined receipt records both sets). Self-checks on installed TS are a gate outside hermetic CI; wrapper tests inject a fake checker launcher.

## 5. Review population and comparison

`populations/602-m2-120.json` is `{schema:"prism.measure.population/1",name,repos,spec_rule,diffs}`. `repos` maps public clone key to `{url,selection_head}`. `spec_rule={git_diff:["-U0","-M"],extensions:[".cjs",".cts",".go",".js",".jsx",".mjs",".mts",".py",".ts",".tsx"],exclude_test:{basename,path},modify_type:"Modified"}`. The two case-sensitive Python regexes below are the #602 selector. Search the basename for the first and the whole relative POSIX path for the second. Either match excludes the file.

```text
(\.test\.|\.spec\.|\.e2e-spec\.|_test\.go$|^test_.*\.py$|_test\.py$|^conftest\.py$)
(^|/)(tests?|__tests__|__mocks__|__snapshots__|testing)(/|$)
```

`diffs` consists solely of `{ordinal,repo,commit,parent,tree,spec_sha256,n_files,n_lines}`. Git object IDs are full 40-hex, `spec_sha256` full SHA-256. Require unique positive ordinals and known repos, bounds/counts nonnegative. Distinct ordinals may name identical diffs. The shipped file contains 1–120, 20 each from apache/answer, apache/superset, calcom/cal.com, directus/directus, miniflux/v2 and saleor/saleor. Copy the frozen selection, per-ordinal identifiers and hashes. No file contents, changed-line lists or diff payloads belong in this manifest.

For each selected ordinal use a separate local shared clone from `--clones/<repo>`, detached at `commit`. Check recorded parent and tree. Never alter the source clone. From `git diff -U0 -M parent commit` collect postimage added line numbers at the new filename, ignore deletions, include files surviving at `commit` with listed extension and outside `exclude_test`. Use `--no-color`, initialize the new filename at each surviving `+++ b/` header, extend with the postimage hunk range (implicit length one), and retain an empty line list for a surviving eligible header with zero added lines. Sort files by path and lines unique ascending. Hash the bytes of Python `json.dumps({"files":[{"file_path":p,"modify_type":"Modified","diff_lines":ls}, ...]})` with default separators, ASCII escaping, insertion order as shown and **no trailing LF**. Refuse the ordinal before either arm on a pin, spec hash or count mismatch.

Exactly two differently named `--arm` values, in base/head order. Split once at `=`, then on literal `::` into binary and individual args, no shell. `--review-args` is POSIX `shlex.split`, default `--algorithm taint --resolution scoped`. Invoke `BIN --repo SCRATCH --diff SPEC <shared args> <arm args> --format json --no-cache` through the guard. Reject attempts to override repo, diff, format or cache. A failed process or unusable/missing selected array fails the arm even if some output parses. Select by the arm's output envelope: `algorithms_run` or multi-result `results` declares multi → `all_findings`, single `algorithm` → `findings`. Use the explicit arm mode when testing the decoder. An extra inactive findings array is ignored; an absent active array is an error.

Within **one ordinal** bucket by `(file,line,algorithm,category)` from the finding, preserving a missing optional member distinctly from null. Canonicalize every serialized member recursively, ignoring object key order. Treat `rules` (string IDs or RuleRef objects), `related_files` and `related_lines` as sets of complete values, deduplicating exact members and sorting by canonical bytes. Preserve each rule's metadata with its ID. Leave all other array orders intact. Missing, null and empty remain three distinct states. Cancel identical payloads with multiplicity. At an anchor with residual counts `b,h`, `presence_changes=abs(b-h)` and `payload_changes=min(b,h)`. Count the latter even if transitions are unpairable. Pair only when one side is empty or the larger residual side is homogeneous (either on ties), as for edges. Otherwise keep one ambiguous bag. Never infer which fields or rule IDs transitioned within that bag. Sum over anchors and then ordinals. A review difference is a measurement, independent of the review command's execution status.

`per-diff.jsonl` has one row per manifest ordinal ordered by ordinal: `{ordinal,repo,commit,status,reason,arms,presence_changes,payload_changes,ambiguous_bags,rule_transitions,anchors}`. `status` is `ok,failed,refused,excluded`, with a diagnostic reason on non-ok. `arms` maps each name to `{status,exit,output_sha256,stderr_sha256}` (null exit/digests if never launched). Each residual anchor is `{anchor,base_bag,head_bag,added,removed,payload_changes,ambiguous,pairs}` with complete payload arrays, surplus counts, and `pairs=[{from,to,count,fields,text_only}]` for proved transitions. `fields` lists sorted differing top-level fields. `text_only` equals true if only `description` or `message` changed. For a uniquely paired `rules` state/ID change add `{from,to,count,from_state,to_state}` to `rule_transitions`, sorted IDs or null, states `missing,null,list`. Rule metadata alone is a payload change. Failed/refused/excluded metrics are null and their ordinals cannot contribute zeros to a comparison.

`totals.json` is `{schema:"prism.measure.review/1",population_sha256,scope,population,selected,ok,failed_ordinals,refused_ordinals,excluded_ordinals,presence_changes,payload_changes,ambiguous_bags,sample_size,seed,arms}`. `scope` is `population` or `subset`. `--only` selects distinct existing ordinals. Every unselected ordinal gets an excluded row. Bind executable hashes, versions, actual shared/arm argv and manifest digest in `arms` and record file hashes in `review-receipt.json={schema:"prism.measure.review-receipt/1",population_sha256,arms,artifacts}`. On quiet interruption, mark the attempted arm/row refused, unstarted selected ordinals refused with the interruption reason and keep the full accounting. `sample.jsonl` holds up to `--sample` distinct **successful ordinals** (default 30, including clean controls), each `{ordinal,repo,commit,presence_changes,payload_changes,ambiguous_bags}`. Start with all successful ordinals sorted ascending, choose `random.Random(seed).sample(population,min(N,len(population)))` in Python 3.9 sampling order, default seed 20261101, nonnegative N, integer seed. Sort chosen ordinals for publication. The matrix presents a five-ordinal candidate pool directly to this sampler.

## 6. Gate file and report

`{"schema":"prism.measure.gate/1","lane":"example","prediction":"PREDICTION.md","binaries":{"base":{"path":"bin/base","expect_version":"4f9e3a3e"}},"steps":[{"id":"x","do":"run","binary":"base","corpus":"X"},{"id":"prior","do":"run","reuse":"bound/head"},{"id":"edges","do":"diff-edges","base":"x","head":"prior","classifier":"classifier.py","expect":{"unattributed":0}}]}` is a complete shape. `binaries` may be empty if unused. Resolve input paths relative to the gate file, binary overrides relative to the invoking directory. An ID is a unique `[A-Za-z0-9_-]+` token. Each step is an object with `id,do`, optional `expect` and the following command-specific fields. Output paths are assigned under `OUT/<id>/`. A reference is the literal ID of an earlier step of the required type. Validate the **entire graph**, options, threshold syntax, required references and supported kinds before step one.

| `do` | Required fields | Optional fields (same defaults/semantics as command) |
|---|---|---|
| `corpora-list` / `corpora-verify` | none | `keys` array for verify |
| `run` fresh | `binary` (binaries key), `corpus` | `trace_env` array of assignments, `no_bytes,sites` booleans, `memguard,limit_gb` |
| `run` reused | `reuse` (run directory) | Sole input field. Validate original receipt and all artifacts. |
| `diff-edges` | `base,head` run IDs, `classifier` | `key,list` |
| `diff-stats` | `base,head` run IDs | `allow,which` |
| `oracle ts-edges` | `run` ID, exactly one of `all:true` or `changes` (diff-edges ID) | `typescript,question,relation,side,kind,class` (last two arrays) |
| `oracle ts-calls` | `census` run ID or `base,head` run IDs | `typescript,question,relation,aggregate_only` |
| `review-diff` | `population,clones,arms` (ordered array of two `{name,binary,args}`, binary is a binaries key) | `review_args,only,sample,seed` (only is an integer array) |

`expect` maps dotted field paths in the step's metrics to a finite number (exact equality) or an object with one or both inclusive numeric bounds `min,max`. A missing/non-numeric metric or unmet bound fails and names the step, path, observed value and threshold. Metrics are S2's certificate, `{deltas,forbidden}` **counts** for stats, `{tuples,denominators}` for an oracle, S4 totals, `{verified}` for verify, `{listed}` for list, `{commands}` (successful analysis command count) for run. A child failure cannot be overridden by a permissive threshold. Invalid syntax, unknown fields/steps, forward references or conflicting overrides refuse before execution. The Stage T `setup.json` files describe fake step results and partial reused receipts. The fixture adapter supplies valid binary/run references, intact artifacts and full receipt bindings for those fakes. Preserve the deliberate unknown `do` in gate-007. Production gate files must pass the same whole-graph checks.

Read a UTF-8 prediction, save original bytes/digest before *any* measurement, pass the same bytes/digest to every fresh run. Validate every reused run and its prediction **before** starting fresh measurements. Compare prediction digests (path and mtime are immaterial), rehash the named file before and after each step and immediately before final publication. Missing prediction, mismatch, mutation or bad evidence → 2. `--retrospective` admits a legacy **missing** digest and labels the entire report retrospective even if all digests match. It retains the prediction-file, output-integrity, option and metric checks. A receipt explicitly binding another prediction still refuses. Execute in order. On first failure leave subsequent steps `not_run` and report all checks completed by the failed step. Keep child exit; at the gate boundary an input/pin admission refusal maps to 2, other nonzero child results to 1 except 75.

`gate.json` is `{schema:"prism.measure.gate-result/1",lane,gate_sha256,prediction_sha256,retrospective,result,exit,steps}`. Per step `{id,do,status,child_exit,metrics,expect,failures,artifacts}`, `status=pass|fail|refused|not_run`, `result=pass|fail|refused|retrospective`. A retrospective execution with metric failures remains retrospective with the failures recorded. A binding refusal is refused. Each artifact entry names relative path and SHA-256. On early refusal report the reason if the output directory was allocatable. `REPORT.md` has exactly these level-two sections, in order:

1. **Receipt** gives gate, binary, corpus, options, step receipts and artifact digests.
2. **Prediction** contains the digest and the verbatim text, preserving newlines (surround with a fence longer than any run of backticks in the text).
3. **Population and denominators** gives selected/available, failed and excluded populations, explicitly naming subsets.
4. **Classes and certificate** gives key mode, changed keys, occurrences, class table, unassigned rows, records and validation failures.
5. **Verdicts** gives CORRECT, WRONG, UNDECIDED and INADMISSIBLE with zeros per question.
6. **Controls** records control IDs, expectations and observed results or `not run`.
7. **Limits** records accepted losses, refusals, unfinished steps and unmeasured quantities.
8. **Result** gives the gate result, exit and failing step/threshold if any.

## 7. Benchmark outputs and decisions

### S6a. Finding-level accounting

Apply the same `finding_outcome(row,taint_output,invocation)` in live scoring and retained-output replay. Replay verifies the observation binding, pins, invocation/source-seed identity and stored stdout digest (hash decompressed stdout for the harness's `stdout_sha256`). Keep one case result `row.finding={line_hit,findings,rules}` beside existing reach. Preserve `entries.jsonl`, existing reach outcomes and `summarize(rows)` results. The bound repair observation population is 600 entries (583 acquired), with 373 retained taint outputs. G-findings compares every reach outcome with the bound entries, requires case accounting for every usable GT, a nonzero `sink_line` count and live/replay agreement on the independently labelled cases.

Absent usable GT gives `not_scored`, empty findings/rules. A timeout, nonzero exit, invocation or decoded application error, malformed/missing active findings array or unusable output gives `error` with empty arrays. A retained digest mismatch refuses replay rather than crediting a channel error as an observation. Otherwise filter the selected findings array to exact `category="taint_sink"`. Compare each finding's own file/line, never a related line, diagram or basename. Identify eligible operations on the annotated line from the bound source and GT (including assignment/property-write sinks). Use the annotated operation's syntactic sink form and callee/binding or write identity, excluding independently resolved ineligible lookalikes. Count distinct source spans. Where eligibility cannot be distinguished, preserve all plausible candidates as ambiguous. A single annotation supplies at most one match.

| Priority on a successful observed case | `line_hit` | Per emitted sink finding |
|---|---|---|
| Another eligible occurrence on GT line, even with zero emissions, or ≥2 distinct canonical sink payloads emitted at that file/line | `ambiguous` | On-line findings `ambiguous`, every other emission `off_target`. |
| ≥1 emission at unique GT location | `sink_line` | One on-line copy can be `matched`. Pick the first with the expected family if supplied, otherwise the first on-line copy. All others are `off_target`. If none qualifies, keep `sink_line` and zero matched. |
| Any emission elsewhere in GT file | `sink_file` | All `off_target`. |
| Any emission in another file | `elsewhere` | All `off_target`. |
| No emitted sink findings | `none` | Empty array. |

Each emitted sink occurrence, including duplicates, supplies one `{file,line,match}` in emission order. `match` is `matched,ambiguous,off_target`. The fixture check is multiset equality. `rules` is the full matched finding's `RuleRef[]`, sorted by id then canonical object, each `{id,cwe?,severity?}`; empty if absent or no match. In S6a a GT without an expected rule family is location-scored even when a finding carries rule metadata. In a family-labelled observation compare membership on that **same** finding (at least one expected ID), retain all its refs. The line-hit column remains a location outcome. Missing family mapping never silently supplies a family comparison. Reconstruction of a rule taxonomy for historic unlabelled cases is deferred as DESIGN specifies.

The finding report gives population and usable-GT counts, channel errors separately, all seven case counts by class, and `line_hit_rate = sink_line / (sink_line+ambiguous+sink_file+elsewhere+none)` (null for denominator zero). Show denominator, errors and not-scored even for empty classes. A separate table counts emitted findings and all three matches, and identifies the number of cases that emitted zero. Off-target rows are unadjudicated. Neither denominator estimates precision.

### S6b. Acquisition, mapping and negatives

Enumerate metadata `META/<class>/<entry>/package.json`, keyed by `(class,entry,id)`. Read `id,dependencies,fixedVersion,fixCommit,sink`. Resolve the package name and vulnerable version from the **bound vulnerable package/manifest** and its matching dependency, checking entry and package identity, or the single exact dependency when unambiguous. During acquisition, `--packages VULN` explicitly supplies the existing `VULN/manifest.json` and `VULN/<class>/<entry>/src/package/package.json`. Without it, only the single exact dependency can establish the vulnerable identity. `score` requires `--packages` and checks the manifest. A snapshot's `meta.name,vulnerableVersion,source` are fixture adapter inputs for that binding. Production never assumes those top-level keys in benchmark metadata. `sink` is the existing file:line:column annotation, parsed from the right. Use the bound inspection/source parameter ordinals and the S6a observation set. No inference from an arbitrary dependency or directory alone.

`acquire` trims outer whitespace, one leading `=` and surrounding whitespace, removes one optional leading `v`, then accepts only ASCII SemVer major.minor.patch with optional dot-separated prerelease identifiers (ASCII alnum/hyphen). Reject leading zeros on numeric components, empty identifiers, ranges and build suffixes. Compare numeric core, release above prerelease, numeric identifiers numerically and below nonnumeric, other identifiers by ASCII, shorter equal prefix first. Require strictly newer. Fetch npm registry metadata for the percent-encoded package (including scoped slash) and version. Require exact requested name/version, HTTPS `registry.npmjs.org` on default port on every request/redirect and tarball URL, a valid matching SHA-512 SRI token and a matching existing archive pin. Limits are 50 MiB compressed, 200 MiB uncompressed tar bytes and 20,000 archive members, inclusive. Stream enforcing limits before publication. Extract regular files/directories only below `package/` into a fresh staging directory, refusing absolute paths, dot-dot, links/devices, duplicate/colliding names, file/dir prefix conflicts and escapes. Never install, import or execute archive contents. Rename only the complete verified tree to `<dest>/<class>/<entry>/src/package`. On identical reacquisition reverify pins and existing tree before reuse. On conflict retain prior good data. Publish per-entry pins atomically. `fixed-pins.json={schema:"prism.secbench.fixed-pins/1",entries:[{class,entry,id,name,version,sha256,integrity,tree_sha256}]}`, sorted by class/entry/id; `sha256` is over archive bytes, `tree_sha256` uses section 2 input framing. `acquisition.jsonl` in dest accounts for every entry with `{class,entry,id,name,version,outcome}`, `outcome="acquired"` or a reason below.

`locate.mjs` is an internal library of the guarded benchmark scorer. It uses the pinned TypeScript parser and scanner with section 4's primary compiler options and the language kind inferred from each filename. Its inputs are bound source bytes. The scorer never loads a package configuration or executes a package module.

For `score`, validate the bound vulnerable observation run and package identity. The vulnerable positive is an unambiguous annotated finding (`matched`, or `sink_line` in a historical location-only stub) **or** `traced` reach to the annotated sink. `reached_function_only`, unrelated reach, `ambiguous` line hits and partial reach cannot establish it. Failure of one channel cannot erase an independently established positive. Locate the vulnerable sink's innermost owning callable at the annotation. Use the same plain name, same relative file, exactly one fixed callable with that name **in that file**. Include nested callables in lookup, exclude their bodies from an outer callable's occurrence inventory. Map the source by name/file/unique owner, preserve data parameter ordinals (skip TS `this`), and regenerate the fixed seed at those declaration lines. No nearest-line or first-name fallback.

Locate calls by AST and exact callee token sequence in the mapped owner. A unique retained call maps. Two plausible matches exclude the pair. A verified removal requires a retained mapped owner plus positive source evidence: the operation was deleted and has no relocated, split, renamed or alias replacement. For the direct deletion fixture, the changed owner loses the call, the callee/import is gone from the package's code and no new call/delegation in the changed code can carry it. Comments and strings are outside the call inventory. Simple absence from one owner, an unrecognized sink form or a changed spelling yields `sink_occurrence_unmapped` unless a bound independent removal determination establishes deletion. The automatic retained-call rule excludes noncall sinks even if the same assignment text persists. Independently established relocation takes precedence over a same-name decoy.

Assess `function_change` on the **sink owner** alone. Compare sequences of `(SyntaxKind,raw token text)` using the pinned TS scanner in the source's lexical context, skipping trivia, retaining literal contents and applying parser-directed rescans for regexp/template segments. A newline with automatic-semicolon consequences may preserve this token sequence. Retaining the mapped finding or reach provides an observation, never independent evidence of a partial fix. A change affecting only the caller on the annotated path with unchanged sink owner is `fix_elsewhere`. A change that addresses one bound annotated input/path while another independently demonstrable old contribution survives is `partial_fix`. For the two-parameter fixture the source diff wraps just the first contribution to the same call and leaves the second direct. A version-only metadata edit or changed literal alone supplies neither exclusion. These rules keep positive fixed findings in the verified set where the independent exclusion tests do not apply.

For cases beyond direct syntax, `--adjudications` supplies optional JSONL rows `{class,entry,id,vulnerable_tree_sha256,fixed_tree_sha256,sink,fact,evidence}`. `sink` is the vulnerable entry's original annotation string. `fact` is one of `moved,split,renamed,removed,fix_elsewhere,partial_fix`. `evidence={before:[span,...],after:[span,...],explanation}` uses nonempty arrays of `{file,start_byte,end_byte}` and a nonempty explanation. Offsets are whole UTF-8 byte boundaries in authenticated files. For deletion, the after span covers the mapped surviving owner. Validate binding, reject contradictory positive mapping facts, retain evidence beside the outcome. Independent facts can disqualify apparent matches or establish removal; they cannot waive the positive control, unique same-file owner or intact inputs. Unproved mapping stays outside the denominator. No assumption that a reported fixed-version finding proves the underlying security defect.

For a retained verified pair require usable fixed-side seed, frontier and taint results. A frontier item on the exact mapped occurrence (value-token span contained in the mapped operation, correct file/owner) supplies `reach=true`. For a line-only observation require exactly one eligible occurrence at that file/line. A finding at its own mapped anchor analogously supplies `finding=true`. Complete, successful channels with no such evidence give false separately. An unrelated in-function item or a seed at the function signature cannot count. An ambiguous line-only observation or an omitted hit in a truncated frontier does not prove false. An explicit hit in a truncated frontier still proves true. A failed/malformed/application-error channel yields `prism_error`. For a verified removal the mapped observation is inapplicable, so both finding and reach are null.

`negatives.jsonl` has one row per attempted entry, `{class,entry,id,outcome,columns,reason_detail,vulnerable_pin,fixed_pin,evidence}`. `outcome="verified_pair"` or **one** reason below. `columns={finding,reach,function_change,sink_removed}`. Retained pairs use booleans, `changed|unchanged`, false. Verified removal uses null, null, `changed|unchanged`, true. For a reason **all four columns are null**. `reason_detail` is free prose or null. `vulnerable_pin` and `fixed_pin` are the corresponding section 2 tree digests, null when unavailable. `evidence={observations_sha256,adjudication,source,sink,channels}`. `observations_sha256` binds the vulnerable observation set. `adjudication` holds the matched adjudication row or null. `source` and `sink` each map `vulnerable,fixed` to collected span arrays (empty when absent or removed). `channels` maps each used channel name to its raw stdout digest. Retain every available binding even on exclusion. `negatives-summary.json={schema:"prism.secbench.negatives/1",attempted,verified_pairs,retained,removed,still_reported,still_reached,reasons,minimum,result}` uses integer counts, zero-filled reason map, `result=met|unmet`. Keep findings, reach and function changes separately tabulated, for the verified subset only. A removed sink contributes to `verified_pairs` and `removed` and never to either retained-observation numerator. An upper bound is the number still reported within the verified subset.

**Closed unscored outcomes**, in evaluation order within each phase. Score first consumes metadata/acquisition results for a fixed candidate, then GT/control, owner/occurrence, fix evidence, source mapping and observations. Tests with several faults take the first applicable reason; an observed security-channel error does not erase an already established mapping exclusion.

| Reason | Condition |
|---|---|
| `fix_commit_only` | No fixed version (absent/null/blank), nonblank fix commit. |
| `no_fixed_version` | Neither a usable fixed version field nor a fix commit is present. |
| `metadata_mismatch` | Nonunique package/dependency, unusable vulnerable version, or registry response names a different package/version. Registry identity is checked after fetching it. |
| `fixed_version_unparseable` | Present fixed version is outside the exact-version grammar. |
| `fixed_not_newer` | Parsed version ≤ the vulnerable version. |
| `origin_refused` | Metadata request, redirect or tarball location violates the declared HTTPS registry origin. |
| `registry_missing` | No usable registry response/archive after the request (including missing version, transport/HTTP error). |
| `integrity_unsupported` | Missing or malformed SHA-512 registry integrity declaration. |
| `limit_exceeded` | Any byte/member budget exceeded while streaming. |
| `integrity_mismatch` | Fetched archive bytes disagree with declared SHA-512. |
| `pin_drift` | Verified archive identity or installed-tree digest disagrees with a stored pin. |
| `extract_refused` | Malformed archive, unsafe member, collision or extraction error before publication. |
| `gt_unavailable` | Vulnerable annotation/source unavailable in the bound inspection. |
| `no_vulnerable_positive` | Neither vulnerable channel establishes the annotated occurrence. |
| `sink_function_unnamed` | No named innermost vulnerable owner for the annotation. |
| `function_moved` | Proven relocation, or no same-file name and exactly one matching owner elsewhere. |
| `function_split` | Independent evidence establishes a split of the original owner. |
| `function_not_found` | No qualifying fixed owner by name, including a rename without a same-name owner. A bound rename excludes under this reason. |
| `function_ambiguous` | More than one matching name in the required file (or multiple candidates elsewhere with no unique move). |
| `sink_occurrence_ambiguous` | Multiple plausible matching occurrences in the mapped owner. |
| `sink_occurrence_unmapped` | No proved retained call or removal, including relocation into another helper or a retained noncall. |
| `fix_elsewhere` | Bound edit fixes/changes the annotated path outside the unchanged sink owner, or independent evidence identifies an unrelated fix. Has precedence over `partial_fix`. |
| `partial_fix` | Independent patch/path evidence establishes that only part of the annotated case was addressed. |
| `source_not_found` | Fixed source mapping or corresponding data-parameter seed is missing, ambiguous or irreproducible. |
| `prism_error` | Required invocation, decoding, artifact verification at the channel, or application result failed. |
| `observation_incomplete` | A nominally successful channel leaves the mapped occurrence undecidable (e.g. ambiguous line or truncated frontier miss). |

The batch pin and source of the positive controls are mandatory; a corrupted observation-set binding is a command refusal. Use explicit positive evidence to distinguish established exclusions (move/split/rename) before ordinary lookup. Test GT/control before requesting a fixed-side query. Evaluate fetch checks in request order and size limits as bytes arrive. Report every metadata row including commit-only/no-version rows. The recorded census has 600 entries, 277 parseable versions, 147 with usable GT and **57** with the traced vulnerable positive. Adopt `floor(57/2)=28` in the S6b prediction before scoring. S6a may add finding-only positives; the default minimum stays 28 absent a new owner decision written before measurement. Zero verified or 1–27 means unmet.

## 8. Changes to DESIGN section 5

1. **Binding.** A git-clean query root excludes tracked edits as well as untracked/ignored files. Bind all command outputs **and stderr**, check the executable across the run, and compare effective analysis options before mode projection. This makes the source, options and trace used to claim a change inspectable.
2. **Edge identity.** Compress identical transitions into one row with a count. Combine one-sided residuals at a key into one row. Give each ambiguous bag both arm totals and a scalar maximum. This preserves both observed multisets and the matrix's row unit.
3. **Trace join.** Project the trace's byte fields away along with edge bytes in line mode. Count matching arm occurrences including cancelled ones. Ambiguous edge records fail independently; unmatched records require a lane veto to fail by themselves. Include unjoined `node` records. This permits conservative line-mode joins and spelling checks on nodes with no edge.
4. **Oracle questions.** Grade a uniquely selected in-root declared signature slot. A unique external callable cannot refute a local edge using unbound declaration bytes. Grade only changed exact head target claims in call-diff mode, and provide authenticated arm-bag selection for edges. These choices specify the population and the property each verdict can support.
5. **Review comparison.** Count each overlapping unmatched payload occurrence as one payload change, even in an ambiguous bag, and report absolute anchor-count surplus as presence. This keeps a zero meaningful without assigning speculative transitions.
6. **Benchmark bullets.** Resolve case ambiguity before testing for emissions, preserve one row per duplicate, use only the sink owner's tokens for `function_change`, and give moved/split/elsewhere/partial/incomplete cases explicit exclusion rules. A verified removal carries null fixed finding/reach. This keeps all denominators reconstructible.

**Provisional handoff change log**

| Provisional entry | Stage D disposition | Effect on an adopter |
|---|---|---|
| First issue, 2026-11-24, sections 1–3 | **Accept** wire endpoints, attributes, two key modes, main CLI, classifier exports, four edge-join counts and dedicated edge stderr. **Change** “one row per changed key” to the disjoint counted rows in section 3, add custody of stderr/changes, specify fields, unit and exits and restore the legacy trace line. | Existing `classify(change,context)` and `context.records(change)` calls work. The classifier can return a class for a whole ambiguous bag. |
| First issue, 2026-11-24, section 4 | **Accept** four verdicts, checker pin, self-check-before-grading, `relations` and `grade(row,ctx)`. **Change** bare key selection to matching arm bags, add explicit relation and class/kind filters, include denominators and receipt in aggregate mode, close `ts-calls` population rules. | One public module may export all five lane 9 relations. Its local helpers and case trees are bound. |
| Adopter amendment, 2026-11-24, `node` | **Accept.** A span can exist independently of an edge. | Inventory reaches `validate(bound)` even on an identity run. |
| Adopter amendment, 2026-11-24, `enclosing_call` | **Accept.** Full-span lookup distinguishes nested and curried calls, and site claims require exact call bounds. | Use the three-argument lookup in section 4. |
| Adopter amendment, 2026-11-24, `program_with_default_lib()` | **Accept.** A separate pinned program supplies default library identities without changing the first checker. | Return the `{program,checker}` pair in section 4. |
| Deferred items in provisional section 5 | **Settle** manifest, gate/report, review artifacts and generic classes in sections 2, 3, 5 and 6. | No generic attribution is applied before lane validation. |

**Inventory disagreements.** The numbers below are the inventory's original fork disagreements. These are the materialized fixtures that name each one.

| Inventory numbers | Row IDs in number order | Adopted behavior |
|---|---|---|
| 1–3 | cmp-002 / cmp-003 / cmp-004 | Preserve the numeric kill line and both roles as attributes. Decode raw or complete gzip streams. |
| 4–6 | join-011 / join-003, join-004 / cmp-017 | Match structured paths, enforce the declared stray policy and ignore the extra wire owner in edge identity. |
| 7–9 | cmp-047, join-012 / cmp-001 / cmp-016, cmp-028 | Keep files on their own endpoints and conserve all copies through residual pairing. |
| 10–12 | cmp-018, cmp-019 / join-013, join-014, join-015, join-016 / join-017 | Skip physical blanks, refuse malformed records, and match the complete mechanism set in the 4b classifier. Parse owner layouts by prefix and arity. |
| 13–15 | cmp-046 / cmp-020 / run-012 | Exclude booleans from counters, fail unattributed rows and require both successful bound arms. |
| 16–18 | run-013 / join-018, join-019, join-020 / oracle-017 | Use no-cache for each analysis, let the classifier select the mechanism vocabulary and verify the checker pin before Node. |

### Previously undetermined materialized rows

All 24 are decided. The kit file contains 15 such rows in 152, including the appended `cmp-053`. The benchmark copy contains nine in 45. Other asserted fields of a row still bind. Each reason below identifies the distinction the competing reading would lose.

| Row ID | Truth of record | Reason for the choice |
|---|---|---|
| cmp-017 | Exit 0, zero changes. | Owner is an extra endpoint field outside the projected identity, and rejecting it would preclude a backward-compatible enrichment. |
| cmp-018 | Exit 0, two occurrences per arm, zero changes. | An empty physical line carries no JSON value and can be skipped without losing an occurrence. |
| cmp-019 | Exit 2, no comparison. | A nonblank malformed record could be a lost occurrence if skipped. |
| cmp-046 | Exit 0, no deltas. | A JSON boolean encodes a flag and coercion to integers would manufacture a counter delta. |
| cmp-053 | One ambiguous row, bags 3/1, count 3, unattributed 1, exit 1. | Choosing an exact precursor in a partly repeated bag would discard the competing killed precursor. |
| join-002 | Attached 0, stray 0, ambiguous 1, unmatched 0, collisions 1, zero change rows, exit 1. | Projecting both sides to line identity prevents hidden bytes from resolving a collision in a line-mode result. |
| join-008 | One unmatched, one independently classified move, unattributed 0, exit 0. | Requiring every unmatched diagnostic trace to fail would impose a policy the supplied classifier did not request. |
| oracle-027 | One lost site/claim in the delta, zero verdicts. | A removed base target belongs in the loss denominator and has no new exact head claim to grade. |
| oracle-029 | One multi-target delta, one head WRONG (`sameB`), zero new CORRECT. | Grading the surviving `sameA` again would inflate changed-claim coverage. |
| oracle-032 | CORRECT for `selected-signature-slot`, first overload formal. | The one callable's checker-selected signature supplies the exact claimed bytes, while interpreting them as the runtime implementation would exceed the property. |
| oracle-034 | UNDECIDED, external callable. | A refutation using declaration data beyond the authenticated source boundary would enlarge the admitted evidence. |
| review-008 | Presence 0, payload 2, ambiguous bags 1, no invented transition. | Two uncancelled occurrences overlap on each side of the anchor, and reporting zero would conceal both differences. |
| gate-005 | Result `retrospective`, exit 1. | Reserving process success for prospective PASS protects callers that gate only on the exit code. |
| gate-006 | Result `fail`, failed step `compare`, exit 1. | The measured step and its zero threshold are both valid, so the failed expectation is a gate outcome. |
| gate-007 | Refused, exit 2, zero steps launched. | Whole-file preflight prevents prior steps from changing state for an invalid graph. |
| bf-011 | `sink_line`, two findings in emission order, `matched` then `off_target`, rules empty. | A single annotation credits one duplicate and the emissions denominator requires both copies. |
| bf-018 | `sink_file`, two `off_target` rows, rules empty. | The same-file finding establishes the closest observed location while the other-file emission remains visible. |
| bf-019 | `ambiguous`, zero finding rows, rules empty. | The second eligible source occurrence makes the line ambiguous independently of channel yield. |
| bn-001 | `sink_occurrence_unmapped`, four nulls. | The named owner survives but delegates to a new helper carrying the sink, which rules out verified deletion. |
| bn-002 | `function_moved`, four nulls. | A unique same-name owner in another file supplies more specific evidence than a missing name. |
| bn-006 | `function_not_found`, four nulls. | The same-file name is gone and body resemblance cannot authorize substituting the renamed callable. |
| bn-010 | `fix_elsewhere`, four nulls. | The changed caller stops passing the original value and the sink owner itself is unchanged. |
| bn-011 | `fix_elsewhere`, four nulls. | The edit is confined to the caller and that independently established exclusion precedes any partial-fix determination. |
| bn-012 | `partial_fix`, four nulls. | The patch changes only one of two annotated input contributions, independently of what the fixed analyzer reports. |

### Truth disputes

Each mention in each round is represented below. T1–T5 are the five terra rounds, S1–S2 in this table are sol rounds, A1 is astra. T1–T3 filed no disputes. The 33 mentions address 18 IDs.

| Row ID | Rounds raising dispute | Ruling and reason |
|---|---|---|
| cmp-030 | S1, S2, A1 | Matrix wins on representation: one row with count 2 preserves both transitions and pins the classifier's call unit. |
| cmp-053 | A1 | Astra wins because both possible predecessors survive, so an unclassified 3/1 bag fails. |
| join-002 | S1, A1 | PLAN's line projection wins over the raw-byte stray reading because the selected mode governs both sides of the join. |
| join-008 | S2, A1 | Sol and astra win because one unmatched record cannot invalidate an independently classified move without a validator veto. |
| oracle-017 | S1 | Sol's refusal agrees with the folded matrix because the checker digest and self-check tree are prerequisites even at the same version. |
| oracle-020 | S1 | Sol's invocation failure agrees with the folded matrix because UNKNOWN violates the module contract and yields no verdict. |
| oracle-034 | A1 | Choose the authenticated in-root reading over astra's WRONG because the external callable is outside the proof domain. |
| review-008 | S1, A1 | Both enumerators win on nonzero payload difference because cancellation leaves two unmatched overlapping occurrences. |
| terra2-review-004 | T4, T5, S1, S2, A1 | All five disputants win because serialization preserves omitted versus explicit null, giving one payload change. |
| terra3-oracle-003 | S1, S2, A1 | Both sol rounds and astra win because a later spread leaves the already established first slot intact (CORRECT). |
| sol-review-006 | A1 | Astra wins because omission and an empty `related_lines` array remain different serialized states. |
| sol2-review-005 | A1 | Astra wins because omission and an empty `related_files` array remain different serialized states. |
| bf-011 | S2, A1 | Both win on two emitted rows because one-to-one annotation credit leaves the second identical copy off-target. |
| bf-019 | S2, A1 | Both win on `ambiguous` because occurrence eligibility is a source property even with no emitted finding. |
| bn-001 | S2, A1 | Both win on exclusion because the helper retains the former sink and the retained outer name distinguishes this from an owner move. |
| bn-010 | A1 | Astra wins on exclusion because the fix is outside the sink function. |
| bn-011 | A1 | Astra wins on exclusion because the caller-only edit establishes `fix_elsewhere` first. |
| bn-012 | S2, A1 | Both win on `partial_fix` because the second annotated contribution survives the patch. |

### Units and compared text

| Representative row IDs | Matrix assertion |
|---|---|
| cmp-001, cmp-030, cmp-051, cmp-052 | Compare change **row** multiplicities and complete per-arm bags. `unattributed` counts rows, even for a whole ambiguous bag. Check scalar occurrences and both side totals as defined in section 3. |
| join-003, join-017, join-021 | Count every typed occurrence in its own category. Compare arm, join state and required tag/mechanism; preserve order/multiplicity of the unjoined records provided to the hook. |
| oracle-010, oracle-016, oracle-020 | Stage-T `why` strings are explanatory labels. Compare verdict and keyed occurrence, arm when selected, tuple/exit and any pinned denominator. Require a nonempty explanation on every actual verdict. Question `property` is an exact field in the verdict and self-check goldens. |
| gate-001, gate-006, gate-009 | Stage-T `reason` is a diagnostic label, not a string golden. Compare exit, result, failed step, report sections, exact prediction bytes and digest where asserted. |
| bf-011, bn-002, bn-026 | Compare `line_hit,match,outcome` (including the **exact** closed exclusion name) and columns, counts and RuleRefs. `reason_detail` and exception message prose are diagnostics. |
| join-007, cmp-021 | Validation and classifier diagnostics must identify the failing counter/class or exception type when asserted. The separately predicted G-repro control failure counts/causes are compared, not punctuation in diagnostics. |

### Remaining materialized fixtures

The following 173 rows and the 24 decisions above exhaust the two input matrices. For a completed comparison, a classified result without a listed failure exits 0. Unmentioned join counts are zero. In benchmark tuples the order is finding, reach, function_change, sink_removed. Each exclusion has four nulls. The matrix checks its supplied keys, complete residual bags and pinned fields, in addition to the observables shown here.

| Row ID | Input and expected observable |
|---|---|
| run-001 | Ignored generated file under the git query root → 3 before version, no receipt. |
| run-002 | Last command rewrites a source → 4, input_changed. |
| run-003 | Clean pinned corpus and successful commands → 0, ok, binary hash and corpus commit bound. |
| run-004 | Checked-out second commit differs → 3 before version, no receipt. |
| run-005 | Untracked source under root → 3 before version, no receipt. |
| run-006 | Prediction rewritten during analysis → 0, ok, SHA-256 of the original bytes. |
| run-007 | Only the edges command gets the requested trace switch → 0, ok. |
| run-008 | Guard refuses before launch → 75, no binary invocation or receipt. |
| run-009 | Verify prints failing key and exits 3 for the second commit. |
| run-010 | Archive verifies, exit 0, perf manifest SHA-256 `1cde38ecf4ebe4cb0b36c72fbbae296fc0e34215d19116dbdb624800650b4d36`. |
| run-011 | The sample match statement fails the Python 3.9 grammar check. |
| run-012 | Middle command exits 7 → stop, 4, failed receipt with the child exit. |
| run-013 | Call-stats argv uses --no-cache → 0, ok. |
| run-014 | Link target text changes → 4, input_changed. |
| run-015 | Tracked edit with matching HEAD → 3 before version, no receipt. |
| run-016 | Only .git contents change → 0, ok. |
| run-017 | Wrong version → 3 after version only, no measurement receipt. |
| run-018 | Existing output file → 2, preserve its contents. |
| cmp-001 | One exact → two identical nameonly: one move and one addition on one key, 2 occurrences, 0 unattributed. |
| cmp-002 | Kill line 13→14 → one attributed move. |
| cmp-003 | Source access use→def → one attributed move. |
| cmp-004 | Row permutation and different gzip headers → zero. |
| cmp-005 | Same-line label swap at distinct bytes → two moves. |
| cmp-006 | Same swap projected to line → zero changes, one collision. |
| cmp-007 | Mixed dumps, explicit bytes → 2. |
| cmp-008 | Mixed dumps, auto → 2. |
| cmp-009 | Same mixed-capability data with compatible receipts and forced line → zero changes, one collision. |
| cmp-010 | Partial end bound, bytes → 2. |
| cmp-011 | Partial end bound, forced line → zero changes, zero collisions. |
| cmp-012 | Dotted single field versus two fields → one removal and one addition. |
| cmp-013 | Two distinct residuals on both sides → ambiguous 2/2, one unattributed row, exit 1. |
| cmp-014 | One identical occurrence → zero changed keys. |
| cmp-015 | A zero-width destination with a new label → one move. |
| cmp-016 | Two identical copies unchanged per arm in line mode → zero changes, one collision. |
| cmp-020 | Unclassified removal → one unattributed row, exit 1. |
| cmp-021 | Classifier divides by zero → one unattributed removal, ZeroDivisionError recorded, exit 1. |
| cmp-022 | Undeclared class → one unattributed removal, returned name recorded, exit 1. |
| cmp-023 | Different corpus input digests → 2. |
| cmp-024 | Stale output digest → 2. |
| cmp-025 | Failed receipt → 2. |
| cmp-026 | Sites option differs → 2. |
| cmp-027 | Only binary and trace environment differ → admitted, zero changes. |
| cmp-028 | Two copies to one → one removed occurrence. |
| cmp-029 | One copy to two → one added occurrence. |
| cmp-030 | Two identical transitions → one move row, count 2. |
| cmp-031 | Two distinct to three identical → two moves, one addition. |
| cmp-032 | Three identical to two distinct → two moves, one removal. |
| cmp-033 | Two distinct to two identical → two moves. |
| cmp-034 | Two identical to two distinct → two moves. |
| cmp-035 | After cancellation the residual is mixed 2/3 → one ambiguous unattributed row, exit 1. |
| cmp-036 | Reversed order, same 2/3 bag attributed as a whole → exit 0. |
| cmp-037 | Three independent keys → one addition, one removal, one move. |
| cmp-038 | Fixed case → two moves, one addition, one removal. Both bags rebuild for all 200 additional seed-731 pairs. |
| cmp-039 | Line-only data on both arms → automatic line, zero changes, one collision. |
| cmp-040 | Line-only nonempty data and explicit bytes → 2. |
| cmp-041 | 100,000 distinct keys per arm → zero changes, Python traced peak <100,663,296 bytes. |
| cmp-042 | Truncated gzip after signing → 2 on digest. |
| cmp-043 | Truncated gzip signed as-is → 2 on decoding. |
| cmp-044 | Allowed nested x 1→2 and y 2→1 → both printed with signed deltas, exit 0. |
| cmp-045 | Allow x.* only → report x.a and y, y forbidden, exit 1. |
| cmp-047 | Destination file differs → one removal and one addition in line mode, zero collisions. |
| cmp-049 | JSON member order changes → zero. |
| cmp-050 | Destination access changes → one move. |
| cmp-051 | One shared copy cancels → a single removed bag of two residual attributes, one unattributed row, exit 1. |
| cmp-052 | One versus two different successors → ambiguous 1/2, one unattributed row, exit 1. |
| join-001 | Two same-line moves, trace at byte 154 → one attached, other move unattributed, exit 1. |
| join-003 | One unchanged match → one stray, zero changes, exit 1. |
| join-004 | Same match, strays allowed → one stray, exit 0. |
| join-005 | One census edge attaches to kill:alias, two writes unjoined → one changed key, exit 0. |
| join-006 | Two edge records attach to one change, one cert unjoined → two attachments, one changed key. |
| join-007 | Attached change classifies. Observed counter +20 versus predicted +1 → validation failure, exit 1. |
| join-009 | Trace matches unchanged 158 only → one stray, one unattributed move, exit 1. |
| join-010 | Two distinct destination bytes → two moves each with one attachment, exit 0. |
| join-011 | Structured [] field disambiguates the indexed edge → one attachment and classified move. |
| join-012 | Trace names consumer file only → one cross-file attachment and classified move. |
| join-013 | Sole expected kill:alias tag → one attributed move. |
| join-014 | Sole expected alias-d3 tag for the fixture transition → one attributed move. |
| join-015 | No required tag → one unattributed move, exit 1. |
| join-016 | Both competing tags attach → one unattributed move, exit 1. |
| join-017 | Four owners retained in trace order, guard/arrow/parameter/loop predictions sum to −1 → no edge records or failures. |
| join-018 | Lane declares alias-d2c → one classified move, one attachment. |
| join-019 | Lane omits alias-d2c → one attachment, one unattributed move, exit 1. |
| join-020 | alias-d2c fires on unchanged occurrence → one stray, exit 1. |
| join-021 | Identical head copies → one ambiguous record, no attachment, move plus addition unattributed, exit 1. |
| oracle-001 | Actual → matching formal in identity → CORRECT. |
| oracle-002 | Read of same declaration after overwrite → CORRECT for binding only. |
| oracle-003 | Rest formal → UNDECIDED. |
| oracle-004 | Spread actual → UNDECIDED. |
| oracle-005 | Outer shadow claim WRONG, inner declaration to its read CORRECT. |
| oracle-006 | Identifier in type query → WRONG. |
| oracle-007 | Same-line other declaration → WRONG. |
| oracle-008 | Inner read and local declaration agree → CORRECT. |
| oracle-009 | Missing destination bounds → INADMISSIBLE. |
| oracle-010 | Authenticated span covers other identifier → WRONG. |
| oracle-011 | Assignment target → WRONG. |
| oracle-012 | Use→use with no extension → UNDECIDED. |
| oracle-013 | Different argument position → WRONG. |
| oracle-014 | Unique different in-root callable → WRONG. |
| oracle-015 | Unresolved identifier at call → UNDECIDED. |
| oracle-016 | Changed live source digest → INADMISSIBLE for selected edge. |
| oracle-017 | Correct version string, wrong checker bytes → 3 before Node, no verdicts. |
| oracle-018 | Fixture edited after passing self-check → 3 before Node, no verdicts. |
| oracle-019 | Two lexical reads → two CORRECT, all other buckets zero. |
| oracle-020 | Module returns UNKNOWN → 1, no published verdicts. |
| oracle-021 | Swapped targets at same-line calls → two WRONG. |
| oracle-022 | Unique inner declaration, exact resolved target, null convenience field → CORRECT. |
| oracle-023 | Outer declaration encloses actual inner target → WRONG for the outer claim. |
| oracle-024 | Replace outer claim by inner → one retargeted delta, one head CORRECT. |
| oracle-025 | One-run census → one CORRECT, no deltas. |
| oracle-026 | Add sameB site → one new delta, one head CORRECT. |
| oracle-028 | Demote exact to nameonly → one confidence-only delta, one site and one head nonexact site, zero verdicts. |
| oracle-030 | Dropped call → one site, one drop, zero verdicts. |
| oracle-031 | No checker declaration for callee → UNDECIDED. |
| oracle-033 | Union has two callable owners → UNDECIDED. |
| oracle-035 | Call target resolves outside root → UNDECIDED. |
| oracle-036 | Two declarations share name/file/start line → UNDECIDED. |
| review-001 | One related line disappears → presence 0, payload 1. |
| review-002 | Added rule ID → presence 0, payload 1, one sorted-ID transition. |
| review-003 | Cancel same-line peer, revised description → presence 0, payload 1. |
| review-004 | Multi arm uses aggregate once → one presence loss, zero payload changes. |
| review-005 | Permuted findings and set members → zero presence and payload. |
| review-006 | Missing rules → empty list → presence 0, payload 1, transition null→[]. |
| review-007 | Same-anchor peer removed → presence 1, payload 0. |
| review-009 | Single envelope reads only findings → zero differences. |
| review-010 | Head exits 7 → ordinal 2 failed, listed in failed totals. |
| review-011 | Spec digest differs → ordinal 1 refused before either arm. |
| review-012 | Spec includes only src/a.ts, Modified, lines [3,4] sorted and unique. |
| review-013 | Scratch reviews leave source-clone HEAD and dirty bytes intact → zero differences. |
| review-014 | Candidate ordinals [5,3,1,4,2], size 2, seed 11 → publish ordinals 4 and 5 on both draws. |
| review-015 | All 120 ordinals accounted, manifest contains neither files nor diff_lines. |
| gate-001 | Changed prediction bytes, preserved mtime → refusal 2. |
| gate-002 | Missing prediction digest in reused prospective receipt → refusal 2. |
| gate-003 | Prediction changes between steps → refusal 2. |
| gate-004 | Missing prediction file → 2 before measuring. |
| gate-008 | Matching digest → 0, all eight report sections in section 6 order. |
| gate-009 | Prediction text including final LF and SHA-256 `def6242ecf18779072eca006fc2cc09caed1a5320557d4a20a28aeb5ed8df493` appear verbatim, exit 0. |
| gate-010 | Exact bound prediction bytes → reuse admissible, exit 0. |
| bf-001 | One sink-line match, source omitted → sink_line, one matched, rules []. |
| bf-002 | Other line same file → sink_file, one off_target. |
| bf-003 | Same line number in a different file → elsewhere, one off_target. |
| bf-004 | Shell and source categories only → none, no sink rows. |
| bf-005 | Source finding only → none, no sink rows. |
| bf-006 | Child error → error, no sink rows. |
| bf-007 | Timeout → error, no sink rows. |
| bf-008 | Two eligible calls on one line → ambiguous, one ambiguous row. |
| bf-009 | Two distinct emissions on ambiguous GT line → ambiguous, two ambiguous rows. |
| bf-010 | Annotated line among 21 findings → sink_line, one matched and 20 off_target. |
| bf-012 | Match copies full js-child-process RuleRef with CWE-78 → sink_line. |
| bf-013 | Only off-target has rules → sink_line, one matched, one off_target, case rules []. |
| bf-014 | No ground truth and no invocation → not_scored, no rows. |
| bf-015 | Successful empty list → none, no rows. |
| bf-016 | GT-line source plus other-line sink → sink_file, one off_target. |
| bf-017 | Four emitted sinks → sink_line, only line 5 matched, three off_target. |
| bn-003 | Mapped sink line 3, unrelated observation on line 4 → verified_pair (false,false,changed,false). |
| bn-004 | Literal-only edit → verified_pair (true,true,changed,false). |
| bn-005 | Verified deletion → verified_pair (null,null,changed,true). |
| bn-007 | Duplicate same-file owner → function_ambiguous. |
| bn-008 | Two same-callee calls in mapped owner → sink_occurrence_ambiguous. |
| bn-009 | Version metadata alone changes → verified_pair (true,true,unchanged,false). |
| bn-013 | Trim whitespace and leading equals → 1.20.0 newer, verified_pair (true,true,changed,false). |
| bn-014 | Nonversion text → fixed_version_unparseable. |
| bn-015 | Range → fixed_version_unparseable. |
| bn-016 | Equal version → fixed_not_newer. |
| bn-017 | Earlier version → fixed_not_newer. |
| bn-018 | Prerelease below corresponding release → fixed_not_newer. |
| bn-019 | Commit without version → fix_commit_only. |
| bn-020 | Neither version nor commit → no_fixed_version. |
| bn-021 | Neither vulnerable channel positive → no_vulnerable_positive. |
| bn-022 | Function-only reach plus same-file off-target finding → no_vulnerable_positive. |
| bn-023 | Vulnerable finding alone supplies control → verified_pair (true,true,changed,false). |
| bn-024 | Vulnerable traced result alone supplies control → verified_pair (true,true,changed,false). |
| bn-025 | Missing sink/GT → gt_unavailable. |
| bn-026 | Three entries yield no_fixed_version, fixed_not_newer and no_vulnerable_positive respectively. Zero verified, unmet. |

## 9. Open-class decision and materialization ledger

The five terra rounds, two sol rounds and one astra round kept producing grammar, multiplicity and transport variants. Close enumeration by proof and conservation. For an admitted oracle occurrence, grade only a named property for which the authenticated spans, roles and checker establish or refute the claim. Every other admitted occurrence gets UNDECIDED with a concrete reason. Inadmissible evidence is identified separately. For the comparator, every decoded occurrence on each arm must reconstruct exactly from cancellations and the published bags. Every change row is classified or the comparison fails, and an unresolvable edge record cannot be borrowed. Inputs the reader cannot account for refuse the run.

Of 300 distinct suggestions, **102 require a fixture** and **198 are recorded under the rule**. Materialize is a binding task for the corresponding slice's matrix successor. Record preserves a case whose output follows an existing row or the same branch of the specified rule. Each entry gives the deciding observation. Counts exclude truth-dispute bullets, which have their own table in section 8. Neither recorded suggestions nor selected fixtures are executions.

| Round | Candidate IDs | Materialize | Record |
|---|---:|---:|---:|
| terra-1 | 49 | 12 | 37 |
| terra-2 | 30 | 4 | 26 |
| terra-3 | 22 | 4 | 18 |
| terra-4 | 14 | 2 | 12 |
| terra-5 | 14 | 2 | 12 |
| sol-1 | 60 | 23 | 37 |
| sol-2 | 59 | 32 | 27 |
| astra-1 | 52 | 23 | 29 |

| Candidate ID | Disposition | Truth and deciding reason |
|---|---|---|
| terra-cmp-001 | Record | A doubt-only transition produces one move because doubt belongs to the attribute tuple. |
| terra-cmp-002 | Record | One byte-keyed base occurrence against empty head produces one removal. |
| terra-cmp-003 | Materialize | Two empty dumps yield zero changes and use the compatible receipt byte option to break the vacuous capability tie. |
| terra-cmp-004 | Record | Three different base labels against two identical head labels form an ambiguous bag because the surplus precursor is unknown. |
| terra-cmp-005 | Record | Equal Unicode names and ordered fields cancel exactly under decoded string equality. |
| terra-cmp-006 | Materialize | Altering a non-selected bound output refuses the entire pair (2), even when both edge dumps still agree. |
| terra-join-001 | Materialize | Unique base and head records both attach to the same move with their respective arm identities. |
| terra-join-002 | Record | A partial key matching one changed and one unchanged occurrence gives one ambiguous record and exit 1. |
| terra-join-003 | Record | A head trace cannot attach to a base-only removal and is unmatched. |
| terra-join-004 | Materialize | A node at the exact changed endpoint remains available to validation with zero edge attachments. |
| terra-join-005 | Record | The counter at the changed coordinates is delivered unjoined because its kind carries a prediction. |
| terra-join-006 | Record | An owner spanning two changes stays one unjoined record because it names a range. |
| terra-oracle-001 | Record | A supplied ordinary argument to a defaulted formal is CORRECT for its selected slot. |
| terra-oracle-002 | Record | A unique method signature and matching first formal establish CORRECT. |
| terra-oracle-003 | Materialize | A new-expression resolving to the named constructor formal is CORRECT under the same slot property. |
| terra-oracle-004 | Record | The unique optional call's matched slot is CORRECT independently of whether the call executes. |
| terra-oracle-005 | Record | The in-root imported function's matched formal is CORRECT after alias resolution. |
| terra-oracle-006 | Materialize | A whole argument claimed to feed a destructured leaf is UNDECIDED because the built-in has no leaf-mapping proof. |
| terra-oracle-007 | Materialize | A claim naming the implementation's formal when the selected overload names different formal bytes is WRONG for the selected-signature property. |
| terra-oracle-008 | Materialize | Compound assignment gives WRONG for a pure-read claim because the matched token is also a write target. |
| terra-oracle-009 | Materialize | Correct raw-byte spans after a BOM, CRLF and a non-BMP character yield CORRECT because the map preserves all three. |
| terra-oracle-010 | Record | Duplicate target declaration triples are UNDECIDED because a span envelope cannot establish uniqueness. |
| terra-oracle-011 | Record | An external target is UNDECIDED under in-root target admission. |
| terra-oracle-012 | Materialize | Changed compiler options refuse grading (3) because the passing self-check belongs to the former option set. |
| terra-review-001 | Record | A severity edit gives one payload change at the retained anchor. |
| terra-review-002 | Record | Changing one related file gives one payload change after set canonicalization. |
| terra-review-003 | Record | Moving the finding to another file gives one loss and one gain across anchors. |
| terra-review-004 | Record | Moving the finding to another line gives one loss and one gain across anchors. |
| terra-review-005 | Record | Changing algorithm gives one loss and one gain across anchors. |
| terra-review-006 | Record | Changing category gives one loss and one gain across anchors. |
| terra-review-007 | Materialize | After cancelling A, residual A versus B,B gives one payload transition and one presence addition because the successors are identical. |
| terra-run-001 | Record | A persisting deletion changes the bound input digest and produces input_changed (4). |
| terra-run-002 | Materialize | A byte-for-byte restoration before the end check leaves matching digests and an otherwise successful run passes, exposing the sampling limit. |
| terra-gate-001 | Record | Corrupting a reused artifact refuses the gate (2) even with a matching prediction. |
| terra-gate-002 | Record | Changing only prediction mtime preserves eligibility because reuse compares bytes. |
| terra-bench-001 | Record | Two eligible operations on the annotated line give ambiguous case and on-line finding. |
| terra-bench-002 | Record | Duplicate emissions produce one matched and one off_target row because the annotation has capacity one. |
| terra-bench-003 | Record | A different-line finding is an unadjudicated off_target emission. |
| terra-bench-004 | Record | A missing vulnerable positive excludes the pair as no_vulnerable_positive. |
| terra-bench-005 | Record | Two fixed functions of the same name in the required file give function_ambiguous. |
| terra-bench-006 | Record | A verified retained pair reports finding, reach and sink-owner token change separately. |
| terra-bench-007 | Record | Independently verified removal gives sink_removed true with null finding and reach. |
| terra-bench-008 | Record | The sole same-name owner in a different file gives function_moved. |
| terra-bench-009 | Record | A string literal edit changes the sink owner's scanner-token sequence. |
| terra-bench-010 | Record | Independently established partial remediation gives partial_fix even with a unique map. |
| terra-bench-011 | Record | An independently established change elsewhere excludes the pair as fix_elsewhere. |
| terra-bench-012 | Record | A retained mapped finding counts as still reported only within the verified subset. |
| terra-bench-013 | Record | A rename fails same-name owner admission and gives function_not_found. |
| terra-bench-014 | Record | Two plausible retained callees in the owner give sink_occurrence_ambiguous. |
| terra2-cmp-001 | Record | Two identical head copies produce one added row of count two. |
| terra2-cmp-002 | Record | A,B versus C,C,C gives A→C and B→C plus one surplus C. |
| terra2-cmp-003 | Record | A partial endpoint refuses automatic key selection (2) even on equal data. |
| terra2-cmp-004 | Record | Two line-projected base copies versus one head copy give one removal and one collision. |
| terra2-cmp-005 | Record | Simultaneous endpoint-role changes give one move because both roles belong to the same attribute tuple. |
| terra2-join-001 | Record | A unique base trace attaches to the base-only removal. |
| terra2-join-002 | Record | Two heads sharing the traced destination make one ambiguous record and no attachment. |
| terra2-join-003 | Record | One unique record attaches while the independent miss is retained as unmatched. |
| terra2-join-004 | Record | A write at the exact span is an unjoined write by kind. |
| terra2-join-005 | Record | Two matching base copies make the base record ambiguous irrespective of head multiplicity. |
| terra2-oracle-001 | Record | Two occurrences of the same name in different actual slots grade CORRECT independently at their respective bytes. |
| terra2-oracle-002 | Materialize | The read in the nested call binds to the inner call's formal and grades CORRECT. |
| terra2-oracle-003 | Record | The read of a shadowing parameter disproves the outer declaration claim (WRONG). |
| terra2-oracle-004 | Materialize | A shorthand property reads its value binding, giving CORRECT for the local variable. |
| terra2-oracle-005 | Record | Prefix increment is WRONG for the pure-read property because it writes the operand. |
| terra2-oracle-006 | Record | A uniquely resolved local function alias proves the original formal binding (CORRECT). |
| terra2-oracle-007 | Record | A uniquely different start line disproves the claimed target (WRONG). |
| terra2-oracle-008 | Record | A different question digest refuses grading (3) because code identity participates in the self-check key. |
| terra2-review-001 | Record | Replacing one member in related_lines gives one same-anchor payload change. |
| terra2-review-002 | Record | Changing description and severity together still changes one payload occurrence. |
| terra2-review-003 | Record | Two identical payloads versus one yield one presence loss and zero payload changes after cancellation. |
| terra2-review-004 | Materialize | An omitted non-anchor member becoming null changes one serialized payload with zero presence difference. |
| terra2-run-001 | Record | A new regular file surviving to the end gives input_changed (4). |
| terra2-run-002 | Record | Equal file contents at different paths remain separate digest members, so editing either gives input_changed (4). |
| terra2-run-003 | Record | An absent mandatory output digest refuses comparison (2). |
| terra2-bench-001 | Materialize | A proven split of the original owner gives function_split, keeping the pair out of all score columns. |
| terra2-bench-002 | Record | Duplicated callees in the fixed owner give sink_occurrence_ambiguous. |
| terra2-bench-003 | Record | A verified observation with reach only records reach true and finding false. |
| terra2-bench-004 | Record | Trivia-only changes give function_change unchanged because the scanner drops trivia. |
| terra2-bench-005 | Record | Non-sink categories supply zero sink rows and no line hit. |
| terra3-cmp-001 | Record | A,A,A versus B,C gives A→B, A→C and one surplus removal. |
| terra3-cmp-002 | Record | A,B versus C stays ambiguous because either base occurrence could have been removed. |
| terra3-cmp-003 | Record | Cancelling B in line mode leaves A→C and one collided key. |
| terra3-cmp-004 | Record | Reordering path fields changes the key and produces one removal and one addition. |
| terra3-cmp-005 | Record | A matching digest of a truncated gzip still gives refusal 2 at stream validation. |
| terra3-join-001 | Record | The unique structured path attaches to the changed edge even when an unchanged edge shares its line. |
| terra3-join-002 | Record | A uniquely matched base cancellation stays a stray when some other head edge is added. |
| terra3-join-003 | Record | A parser returning None contributes no typed record while the valid neighbour attaches once. |
| terra3-oracle-001 | Record | A destructuring assignment target gives WRONG for a read claim. |
| terra3-oracle-002 | Record | A captured, uniquely bound outer lexical read is CORRECT independently of reachability. |
| terra3-oracle-003 | Materialize | An ordinary actual before the later spread binds the first fixed slot and grades CORRECT. |
| terra3-oracle-004 | Materialize | Modifying any checker-tree file invalidates the passing receipt before Node (3). |
| terra3-oracle-005 | Record | Changing only claimed end line leaves a unique matching name/file/start-line target CORRECT. |
| terra3-oracle-006 | Materialize | A valid authenticated punctuation span gives UNDECIDED because it identifies no identifier. |
| terra3-review-001 | Record | A serialized message_detail change gives one payload change under the all-fields rule. |
| terra3-review-002 | Record | Cancelling two identical copies leaves a single B→C payload transition. |
| terra3-run-001 | Record | A regular-file edit under a root that also contains a link gives input_changed (4). |
| terra3-run-002 | Record | Editing even an unrelated regular file in the oracle root gives INADMISSIBLE for the selected edge. |
| terra3-gate-001 | Record | A failed reused receipt refuses the gate (2) regardless of matching prediction and transport hashes. |
| terra3-bench-001 | Record | Traced vulnerable reach alone supplies the required positive control. |
| terra3-bench-002 | Record | One finding on the only eligible annotated occurrence gives sink_line and one matched row. |
| terra3-bench-003 | Materialize | Unverified absence of a call gives sink_occurrence_unmapped because removal needs independent source evidence. |
| terra4-cmp-001 | Materialize | Two classes on two transitions at one key give one global key, two occurrences and one key per class. |
| terra4-cmp-002 | Record | A destination-role edit gives one move at the same byte key. |
| terra4-join-001 | Record | Validation may veto an unmatched record even when an unjoined owner covers a change (1). |
| terra4-join-002 | Record | Two unique records attach to the change and the class may use just one tag. |
| terra4-oracle-001 | Record | Transparent argument parentheses preserve the uniquely proved slot (CORRECT). |
| terra4-oracle-002 | Record | The distinct property declaration disproves an equal-spelling local variable claim (WRONG). |
| terra4-oracle-003 | Record | Reversed roles on that argument/formal pair give UNDECIDED because the proposed read relation does not apply. |
| terra4-oracle-004 | Record | A non-exact target counts in denominators and yields no exact-target verdict even when its identity matches. |
| terra4-review-001 | Record | Two distinct ordinals naming the same diff still yield two clean accounting rows. |
| terra4-review-002 | Record | A serialized finding flag changing false→true gives one payload change. |
| terra4-run-001 | Materialize | Altering an external link target's contents with unchanged link text leaves the input digest unchanged because only the link text is bound. |
| terra4-bench-001 | Record | A numeric literal edit changes the scanner sequence. |
| terra4-bench-002 | Record | A bound expected rule family excludes a wrong-family finding from matched credit at the same location. |
| terra4-bench-003 | Record | Independently verified remediation of a different vulnerability gives fix_elsewhere. |
| terra5-cmp-001 | Record | One line-key copy versus two gives one surplus addition and one collision. |
| terra5-cmp-002 | Record | Different transport spellings decode to the same occurrence and preserve the one surplus removal. |
| terra5-join-001 | Record | A mismatching destination file leaves the record unmatched because every supplied field must agree. |
| terra5-join-002 | Record | An allowed stray supplies no evidence for an unrelated change, which remains unattributed (1). |
| terra5-oracle-001 | Record | The for-of assignment target is WRONG for a read claim. |
| terra5-oracle-002 | Record | A computed key is a lexical value read and grades CORRECT at its own bytes. |
| terra5-oracle-003 | Record | Recursion preserves the uniquely resolved formal slot (CORRECT). |
| terra5-oracle-004 | Materialize | A tagged-template substitution gives UNDECIDED because it is outside ordinary positional call binding. |
| terra5-oracle-005 | Record | Whole-root source mismatch takes precedence over absent bytes, giving INADMISSIBLE. |
| terra5-review-001 | Materialize | Duplicating a related_files member gives zero payload changes because the field is set-valued. |
| terra5-run-001 | Record | A rename changes the relative path in the input digest and returns input_changed (4). |
| terra5-bench-001 | Record | A verified mapped finding without reach gives finding true and reach false. |
| terra5-bench-002 | Record | One eligible occurrence and zero findings gives none and an empty findings array. |
| terra5-bench-003 | Record | A duplicate function name elsewhere leaves a uniquely mapped same-file owner eligible. |
| sol-cmp-001 | Record | A fully bound source and unbound destination refuse automatic key selection (2). |
| sol-cmp-002 | Materialize | A late line-only row makes the whole otherwise byte-bearing comparison refuse (2). |
| sol-cmp-003 | Record | Explicit line mode cancels the shared occurrence and reports one kill-line move and one collision. |
| sol-cmp-004 | Record | One predecessor versus two distinct successors yields one unclassified ambiguous bag (1). |
| sol-cmp-005 | Record | One repeated label on the larger mixed side still leaves competing surplus and an ambiguous row (1). |
| sol-cmp-006 | Record | Two identical transitions give one move row of count two and one collided line key. |
| sol-cmp-007 | Materialize | An end-only coordinate change at start byte zero gives a removal and addition because both bounds enter identity. |
| sol-cmp-008 | Materialize | Distinct NFC/NFD file spellings give a removal and addition because key strings retain their code points. |
| sol-cmp-009 | Record | LF and CRLF JSONL terminate equal decoded records. |
| sol-cmp-010 | Record | An EOF-terminated final object still supplies one occurrence. |
| sol-cmp-011 | Materialize | A leading UTF-8 BOM in JSONL refuses the stream (2) under the strict JSON transport contract. |
| sol-cmp-012 | Materialize | A bad gzip CRC refuses the dump (2) even when preceding records and its receipt digest appear valid. |
| sol-cmp-013 | Record | The x.* allow-list fails on xy.a (1) because prefix matching stops at a component boundary. |
| sol-join-001 | Record | A unique head record attaches to a head-only addition. |
| sol-join-002 | Materialize | A unique head occurrence in an ambiguous change bag may receive one record and a whole-bag class (0). |
| sol-join-003 | Record | A head trace matching both a cancellation and an addition at the same key is ambiguous. |
| sol-join-004 | Materialize | A unique base occurrence on a move-plus-surplus key attaches only to the move containing the base occurrence. |
| sol-join-005 | Record | A line-only record may attach to a unique changed line-key edge. |
| sol-join-006 | Record | An own-drop base counter and own-emit head counter retain arm identity and stay unjoined. |
| sol-join-007 | Record | An unplaced write keeps its payload and remains unjoined even when it names the changed endpoints. |
| sol-join-008 | Materialize | A parser exception refuses the trace (2) before comparison because it cannot be counted as an ignored line. |
| sol-join-009 | Record | An unread sidecar supplies zero attachments, so a classifier requiring a record leaves its change unattributed (1). |
| sol-oracle-001 | Materialize | A TS this parameter occupies no runtime argument slot, so the edge to x is CORRECT. |
| sol-oracle-002 | Record | An omitted actual has no call-argument occurrence, so the default-initializer edge is UNDECIDED. |
| sol-oracle-003 | Record | A later parameter initializer reading the earlier formal gives CORRECT for lexical binding. |
| sol-oracle-004 | Record | In-root re-export and import aliases preserve a uniquely resolved original formal (CORRECT). |
| sol-oracle-005 | Record | A super call resolves to the base method and disproves the override's formal (WRONG). |
| sol-oracle-006 | Record | A uniquely resolved computed method supplies the claimed slot (CORRECT). |
| sol-oracle-007 | Record | Optional receiver execution leaves a unique method's positional claim CORRECT. |
| sol-oracle-008 | Record | In-block same-line callable shadowing disproves the outer formal claim (WRONG). |
| sol-oracle-009 | Materialize | A representative signature for a conditional union cannot identify a single owner, yielding UNDECIDED. |
| sol-oracle-010 | Record | The callee token in f(f) is outside the actual argument list and gives UNDECIDED. |
| sol-oracle-011 | Materialize | A raw escaped identifier span and decoded identifier spelling can prove the same binding (CORRECT). |
| sol-oracle-012 | Materialize | Legal redeclarations with one symbol and multiple declarations give UNDECIDED for the one-declaration property. |
| sol-oracle-013 | Record | Two census edges with one key and different labels yield two original rows graded CORRECT. |
| sol-oracle-014 | Materialize | Losing one identical target copy at a surviving site gives one multi-target delta, one lost target, zero lost sites and no head verdict. |
| sol-oracle-015 | Materialize | A changed caller.file changes site identity, yielding one lost and one new site. |
| sol-review-001 | Record | Changing only function_name gives one same-anchor payload change. |
| sol-review-002 | Record | A newly serialized parse_quality field gives one payload change. |
| sol-review-003 | Record | The addition of serialized diagrams gives one payload change. |
| sol-review-004 | Record | Editing a nested diagram style gives one payload change. |
| sol-review-005 | Record | Adding a formerly missing anchor category produces one presence loss and one gain. |
| sol-review-006 | Record | Omission versus an empty related_lines array gives one payload change under the serialization rule. |
| sol-review-007 | Materialize | A declared multi-result envelope without all_findings fails its ordinal, even with usable inactive findings. |
| sol-review-008 | Record | A nonzero arm exit fails the ordinal despite its complete JSON stdout. |
| sol-run-001 | Record | A nonempty output directory refuses before launch (2) and keeps the prior files. |
| sol-run-002 | Materialize | Replacing a regular file by a symlink changes the type-tagged digest, giving input_changed (4). |
| sol-run-003 | Materialize | Executable replacement persisting across the next hash check gives a failed run (4), even at the same version. |
| sol-run-004 | Materialize | An unbound or replaced edges.stderr.gz refuses comparison (2) because the trace is an input to classification. |
| sol-gate-001 | Record | A mismatched prediction on either reused arm refuses the gate (2). |
| sol-gate-002 | Materialize | Retrospective mode still refuses corrupt reused output (2) because only a missing prediction digest is excused. |
| sol-bench-001 | Materialize | A retained noncall assignment without independent occurrence mapping gives sink_occurrence_unmapped. |
| sol-bench-002 | Materialize | Changed whitespace inside a regex literal changes the retained token text. |
| sol-bench-003 | Record | A hit at a different fixed call cannot establish reach at the mapped occurrence. |
| sol-bench-004 | Record | A finding timeout gives error while preserving the independently measured reach result. |
| sol-bench-005 | Record | An equal line number in another file gives elsewhere and one off_target row. |
| sol-bench-006 | Record | One annotated finding among twenty others gives one match and twenty off_target rows. |
| sol-bench-007 | Materialize | Matching callee text only in a nested helper gives sink_occurrence_unmapped because the outer owner excludes nested bodies. |
| sol-bench-008 | Record | A fix commit alone gives fix_commit_only. |
| sol-bench-009 | Materialize | An identical duplicate archive member gives extract_refused and no published tree. |
| sol2-cmp-001 | Record | Two line-key copies against an empty arm use line mode, two removed occurrences and one collided key. |
| sol2-cmp-002 | Record | A null start and numeric end is partial and refuses explicit byte mode (2). |
| sol2-cmp-003 | Record | Mixed 3/3 line-key bags remain ambiguous with a collision and one unattributed row (1). |
| sol2-cmp-004 | Materialize | The synthetic nameonly/null shape is admitted, giving one confidence-only attribute move. |
| sol2-cmp-005 | Record | Reversing two distinct endpoints changes ordered identity and yields a removal and addition. |
| sol2-cmp-006 | Record | Multiple valid gzip members concatenate to two records and compare equal to one member. |
| sol2-cmp-007 | Materialize | An embedded JSON array has the wrong record type and refuses the dump (2). |
| sol2-cmp-008 | Materialize | Conflicting duplicate JSON members refuse the stream (2) before one value can mask the other. |
| sol2-cmp-009 | Materialize | A missing counter versus explicit zero gives a presence delta (0→missing, arithmetic 0), still subject to the allow-list. |
| sol2-join-001 | Materialize | A unique cancelled base occurrence is a stray even at a key that has a head addition. |
| sol2-join-002 | Record | Repeated identical trace lines yield two attachments to the single matching change. |
| sol2-join-003 | Record | A unique base record attaches to the ambiguous change bag and the head record with two matches independently fails the join. |
| sol2-join-004 | Materialize | Two unjoined nodes at one span retain their different arm payloads alongside the one attached edge record. |
| sol2-join-005 | Materialize | Returning the opposite arm from the parser refuses the input (2) before any cross-arm attachment. |
| sol2-join-006 | Record | The traced end byte distinguishes two matching starts and attaches only to the changed head edge. |
| sol2-join-007 | Materialize | Validation runs once with both final classes and preserves its counter failure alongside one unattributed row (1). |
| sol2-oracle-001 | Record | The default initializer sees the outer declaration (CORRECT) and disproves the body-local candidate (WRONG). |
| sol2-oracle-002 | Record | Explicit undefined still occupies the supplied first slot, giving CORRECT. |
| sol2-oracle-003 | Materialize | Even a fixed tuple spread before the ordinary actual gives UNDECIDED in the built-in, which proves positions only before any spread. |
| sol2-oracle-004 | Record | A generic type argument is UNDECIDED for value binding while the uniquely matched value argument is CORRECT. |
| sol2-oracle-005 | Materialize | A disjoint selected overload formal is CORRECT and the unselected overload's formal is WRONG for selected-signature-slot. |
| sol2-oracle-006 | Record | The super constructor call proves the base formal (CORRECT) and refutes the derived formal (WRONG). |
| sol2-oracle-007 | Record | A uniquely selected inherited base constructor supplies its formal (CORRECT). |
| sol2-oracle-008 | Record | A unique arrow field on the optional receiver supplies its formal (CORRECT). |
| sol2-oracle-009 | Record | The local import alias declaration and its read prove lexical binding (CORRECT). |
| sol2-oracle-010 | Materialize | Identically spelled typeof tokens give CORRECT for the value expression and WRONG for the type query. |
| sol2-oracle-011 | Record | A property value feeding a renamed destructured leaf gives UNDECIDED because it requires a property mapping. |
| sol2-oracle-012 | Materialize | A captured variable inside an arrow body is outside the enclosing call's actual-value binding (UNDECIDED). |
| sol2-oracle-013 | Record | A partially bound endpoint is INADMISSIBLE before source grading. |
| sol2-oracle-014 | Materialize | A UTF-8-interior offset is INADMISSIBLE because it cannot represent a source position. |
| sol2-oracle-015 | Record | A non-exact resolved_targets entry with stale exact_target supplies zero exact verdicts. |
| sol2-oracle-016 | Record | Permuting exact target entries preserves their multisets and gives zero deltas. |
| sol2-oracle-017 | Materialize | The selector consumes only selected-arm residual attributes, grading both roles independently if both occur in the residual. |
| sol2-review-001 | Materialize | Changing only a RuleRef's CWE gives one payload change and zero ID transitions. |
| sol2-review-002 | Record | Adding severity to a retained RuleRef gives one payload change. |
| sol2-review-003 | Record | Reordering complete RuleRefs with stable IDs and metadata cancels exactly. |
| sol2-review-004 | Materialize | Reordering diagrams changes one payload because diagram arrays preserve sequence. |
| sol2-review-005 | Record | Omitted related_files versus an empty array gives one payload change. |
| sol2-review-006 | Materialize | Independently selecting the correct array on a single and a multi arm cancels the equal findings. |
| sol2-review-007 | Materialize | Two distinct residual base payloads versus one head payload give one presence loss, one payload change and one ambiguous bag. |
| sol2-run-001 | Materialize | Late host quiet gives 75 and a failed partial receipt, and the third analysis never starts. |
| sol2-run-002 | Materialize | Explicit --no-bytes removes the inherited PRISM_EDGES_BYTES setting, giving line-only endpoints. |
| sol2-run-003 | Record | Retargeting ./a.ts to a.ts changes the literal link-text digest and gives input_changed (4). |
| sol2-run-004 | Materialize | Persisting output files inside the initially empty query-root output directory produce input_changed (4). |
| sol2-run-005 | Record | An mtime-only touch preserves the input digest and the otherwise successful run passes. |
| sol2-gate-001 | Materialize | Mutation in the final step is caught by the closing prediction check and refuses the gate (2). |
| sol2-gate-002 | Record | The same prediction bytes under a new path pass the digest check. |
| sol2-bf-001 | Materialize | Malformed JSON after a zero exit gives line_hit error and preserves the independent reach result. |
| sol2-bf-002 | Materialize | A proved ineligible lookalike is excluded from the eligible-occurrence count, yielding sink_line and one match. |
| sol2-bf-003 | Materialize | A matched family among multiple RuleRefs qualifies that emission and preserves the full RuleRef payload. |
| sol2-bn-001 | Record | An ambiguous vulnerable line hit with no annotated reach gives no_vulnerable_positive. |
| sol2-bn-002 | Record | An anonymous sink owner with no inferred name gives sink_function_unnamed. |
| sol2-bn-003 | Materialize | Changed callee spelling with a known retained alias gives sink_occurrence_unmapped because deletion is disproved. |
| sol2-bn-004 | Materialize | Same-line competing eligible calls with only line-level positive evidence give observation_incomplete and null columns. |
| sol2-bn-005 | Materialize | A missing fixed source counterpart gives source_not_found even if the sink maps. |
| sol2-bn-006 | Materialize | An application error in a successful frontier process gives prism_error and null score columns. |
| sol2-bn-007 | Materialize | Trivia inside a template substitution leaves the scanner tokens unchanged when the raw segments agree. |
| sol2-bn-008 | Materialize | File-directory prefix collisions refuse archive extraction before publication. |
| sol2-bn-009 | Materialize | Numeric prerelease identifiers put rc.10 above rc.2 and allow the otherwise verified pair. |
| astra-cmp-001 | Materialize | Reassociating source access and doubt leaves a mixed 2/2 ambiguous bag because only whole attribute tuples cancel. |
| astra-cmp-002 | Record | Cancelling A first leaves two copies of B that uniquely supply C and D. |
| astra-cmp-003 | Materialize | One class on a moved-plus-surplus key and another added key counts two class keys and three occurrences. |
| astra-cmp-004 | Record | A collision on either or both arms counts each of the three keys once, including the unchanged key. |
| astra-cmp-005 | Materialize | Forcing line mode cannot waive the incompatible --no-bytes receipt option (2). |
| astra-cmp-006 | Materialize | A literal Unicode line separator inside a JSON string remains part of the filename and compares equal to its escape. |
| astra-cmp-007 | Materialize | A gzip boundary inside a UTF-8 character is transparent after concatenation, giving one equal occurrence per arm. |
| astra-cmp-008 | Record | A reported line shift in an unchanged file gives one unclassified loss and gain (1) because the kit ships no generic class. |
| astra-cmp-009 | Materialize | A literal dotted counter member conflicting with a nested path refuses the statistics input (2). |
| astra-join-001 | Record | Endpoints from different occurrences cannot jointly satisfy one record, so it is unmatched. |
| astra-join-002 | Record | The two partial records each have two matches and only the conjunctive third record attaches. |
| astra-join-003 | Materialize | A lossy rendered path that omits ambiguous fields matches both occurrences and cannot supply attribution. |
| astra-join-004 | Record | An incomplete arrow owner ignored by the parser leaves one valid owner prediction (−1) and one attached edge. |
| astra-join-005 | Record | A C-prune owner and three counters remain four unjoined records with zero edge matches. |
| astra-join-006 | Materialize | Both arm directories and the authenticated corpus root are separately visible to the classifier, giving one attributed removal. |
| astra-join-007 | Record | A validator may reject a successfully classified change and fail (1) with zero unattributed rows. |
| astra-oracle-001 | Materialize | A mixed census retains both one CORRECT and one INADMISSIBLE occurrence because admission is per edge for missing bounds. |
| astra-oracle-002 | Materialize | Full argument-list ownership proves a→x and b→y and disproves b→x for the two curried calls. |
| astra-oracle-003 | Record | Unique private-method resolution proves the private formal and refutes the public same-spelling candidate. |
| astra-oracle-004 | Record | A uniquely narrowed receiver proves A's formal and refutes B's at that call. |
| astra-oracle-005 | Record | The uniquely resolved anonymous IIFE needs no local function name to prove its formal slot. |
| astra-oracle-006 | Record | A destructured tuple rest remains UNDECIDED under the rest boundary. |
| astra-oracle-007 | Record | Two admitted reads of the same parameter-property declaration are both CORRECT under the binding property. |
| astra-oracle-008 | Materialize | A destructuring-assignment destination gives WRONG while its evaluated fallback gives CORRECT. |
| astra-oracle-009 | Materialize | The receiver and computed key that locate a property write are both value reads (CORRECT). |
| astra-oracle-010 | Record | The declaration later in the same scope proves the local closure read and refutes the outer candidate. |
| astra-oracle-011 | Record | JSX attribute-to-props binding gives UNDECIDED without a lane relation. |
| astra-oracle-012 | Record | Swapped exact targets at distinct sites give two retargeted sites and two new head CORRECT verdicts. |
| astra-oracle-013 | Materialize | Losing one duplicate site record preserves the site, changes 2→1 records and loses one target occurrence. |
| astra-oracle-014 | Materialize | A checked extension returns its own property and CORRECT, while a control without the extension gives UNDECIDED. |
| astra-oracle-015 | Record | A corrupt nonselected receipt artifact refuses the pair (2) before publishing call deltas. |
| astra-review-001 | Materialize | Opposite changes to an identical finding in two ordinals yield two total presence changes because ordinals never cancel each other. |
| astra-review-002 | Record | Reassociating CWE metadata between rule IDs gives one payload change with unchanged ID membership. |
| astra-review-003 | Record | Reassociated severity and description leave one ambiguous bag, two payload changes and zero presence changes. |
| astra-review-004 | Record | A changed nested mermaid string changes the complete serialized finding once. |
| astra-review-005 | Record | Sorting candidate ordinals before sampling makes a permutation of input order immaterial. |
| astra-run-001 | Materialize | Dirty files only in a sibling directory leave the pinned query root eligible (0). |
| astra-run-002 | Record | Editing .gitattributes changes the input digest because only the exact .git component is excluded. |
| astra-run-003 | Record | A new empty directory contributes no digest member and leaves the successful run ok. |
| astra-run-004 | Materialize | Only edge-command stderr supplies the typed trace stream even if other commands print recognizable traces. |
| astra-gate-001 | Record | Two fresh runs and an entitled reuse bind the same original prediction and can pass. |
| astra-gate-002 | Record | CRLF versus LF changes prediction bytes and refuses prospective reuse (2). |
| astra-bf-001 | Record | Related lines cannot promote an off-target primary anchor, so the case gives sink_file. |
| astra-bf-002 | Materialize | Two eligible property writes on one line make the case and its one line-only finding ambiguous. |
| astra-bf-003 | Materialize | Wrong family at the right location and right family elsewhere yield sink_line, two rows and zero matched. |
| astra-bf-004 | Record | A nonzero finding-channel exit gives error even after a complete JSON payload while leaving reach intact. |
| astra-bn-001 | Materialize | A newline affecting automatic semicolon insertion can leave function_change unchanged because nontrivia tokens agree. |
| astra-bn-002 | Record | Spelling left in comments and strings cannot invalidate an independently verified call removal. |
| astra-bn-003 | Materialize | Independent proof of an owner move gives function_moved even with a same-name/same-callee decoy in the old file. |
| astra-bn-004 | Materialize | A truncated frontier miss excludes as observation_incomplete, while its explicit mapped hit supplies reach true when other checks pass. |
| astra-bn-005 | Record | A duplicate callee in a sibling is outside the unique mapped owner's occurrence inventory. |
| astra-bn-006 | Materialize | Only two verified pairs contribute to the denominator and one still-reported count, while the partial fix stays in reasons. |

## 10. Accepted losses and gate ownership

| Boundary | Evidence the report can support |
|---|---|
| Source and executable binding | Pre/post hashes and per-command executable checks establish the observed snapshots. A transient change restored before the next check is invisible. Link target text is bound. A Git-less archive needs the extracted-tree pin even when a transport hash is known. |
| Wire identity | Line projection loses byte distinctions and counts its collisions. Extra owner, parameter and loop metadata are outside the edge projection. A lane may read source or its own sidecar. A shared sidecar schema and source-tree rename mapping are deferred. |
| Trace | Partial or lossy spellings can produce unmatched or ambiguous records. A node inventory describes the parsed spans. Its completeness is a lane validation question. Legacy mechanism counts are a reproducibility statistic and confer no attribution. |
| Oracle | S3 covers JS/TS source and the proven property of an authenticated claim. A binding verdict cannot establish flow, intervening kills or that a call executes. Lost call claims remain counted; the base census is available for grading them. Other languages and unproved grammar/relations stay outside the built-ins. |
| Review | The public 120-ordinal manifest contains identifiers/hashes only. Rebuilding specifications needs repository history. The sample contains successful ordinals only, and JSON payload comparison includes diagnostic text. SARIF support and a PR #841 cross-check remain deferred. |
| Benchmark | Line-only reports lose within-line identity. Unadjudicated off-target findings supply no precision estimate. The fixed subset requires independent controls, mapped occurrences and usable observations. Scanner-token equality can mask automatic semicolon insertion. Report the subset, exclusions and all four columns, including removal. |
| Operations | Timing/RSS comparisons, old evidence deletion and cloud launcher migration stay outside the kit. New source-only corpora with no verified tree pin stay unavailable. A user may run a lane-specific census for a property outside these interfaces. |

**Gate ownership.** S1–S5 supply hermetic Python matrix checks using fakes. S2 owns both parts and all five negative controls of G-repro. S3 owns G-selfcheck and the labelled real-use fixture. S6a owns G-findings and S6b owns G-negatives. The two consuming lanes own their baseline parity and their own subsequent decisions for G-adopt. A source document, named adopter or selected fixture earns no measured gate credit. The six owner decisions in DESIGN section 7 stand. No materialized truth or enumerated disposition awaits an owner decision. Changing the fixed minimum or extending scope to launchers, SARIF, precision adjudication or a different reproduction constitutes a later decision under DESIGN.

