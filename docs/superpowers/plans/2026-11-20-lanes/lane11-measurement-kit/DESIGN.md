# Lane 11: a shared measurement kit — design

> **PROVISIONAL (owner decision, 2026-11-25).** These formats are not a design of record. Six review rounds and four enumeration passes found open-class defects in them that were not folded; the findings are test rows in [FORMATS-OPEN-ROWS.md](FORMATS-OPEN-ROWS.md), each with an owning slice. Each slice freezes the formats it implements, with executable tests, when it lands, and must close or explicitly re-scope every open row it owns. No other lane may depend on a provisional format: lanes 9 and 10 use their own tools until the slice that implements the format they need has landed.

Status: architecture final; the comparison, join, oracle and scoring rules of section 5 were amended by the
design review (2026-11-21); command names and file formats are fixed at stage D, after the inventory of
existing scripts (PLAN stage M) and the executed truth matrix (PLAN stages T and E). The plan files a tracking
issue. Research record: `~/prism-evidence/lanes-2026-11-20/research/lane11-measurement-kit.md`.

## 1. Intent

Every lane measures its change before it merges: both binaries on fixed corpora, an edge and counter
differential with every change attributed to a named class, an oracle that grades what was added, a
benchmark run, sometimes a replay over real diffs. Each lane has rebuilt those scripts in its private
evidence directory, and they are offloaded when the lane closes. The kit puts one maintained, tested copy of
the common parts in the repository, so a lane writes only what is specific to it (its classes, its oracle
question), gates are reproducible by someone else, and the same commands run unchanged in a cloud session.

It also fills two holes in what can be measured at all: the security benchmark scores reach only, never
findings, and has no negatives.

## 2. What exists today (verified on `origin/main` at 1179055f and in the evidence directories)

| Fact | Reference |
|---|---|
| Committed and tested: the call-resolution accuracy harness with language-server oracles (`eval/tier_a`), the benchmark harness (`eval/secbench`), paired timing in the cloud (`scripts/perf/cloud`), host-quiet markers, the pinned TypeScript acquisition (`scripts/gate-inputs`), a byte-comparison script over fixture diffs. CI runs only their hermetic unit tests. | `eval/`, `scripts/`, `.github/workflows/ci.yml` |
| Not committed, rebuilt per lane: (a) the "run both binaries, dump statistics and edges" recipe, at least 13 near-copies; (b) the edge comparator with class attribution, about 20 files in six or seven designs (one file byte-identical in nine places, one helper forked eight ways), whose attribution reads lane-specific trace lines the product prints under an environment variable; (c) counter comparison; (d) TypeScript-checker oracles, at least eight different `census*.cjs`; (f) review-output comparison over 120 real diffs, seven scripts, with the diff population stored only in an archive; (g) the cloud launchers; (h) the memory guard. | research section 2 |
| Edge dumps carry occurrence bytes only since #789 (`PRISM_EDGES_BYTES=1`, 2026-11-09). `DfgEdgeEndpoint` has file, line, path and access only at the #660 S1a commits `31703c86` and `152d22bc` and at the earlier #783 round `4c8620a2`/`4da81c36`; it has optional `start_byte`/`end_byte` at the #783 slice 4b commits `4f9e3a3e`/`1f2ad1cc`. | `git grep` of `src/navigation/` at each commit |
| Tooling cost per lane, from ledgers: 959 to 3,500 lines of scripts in three recent data-flow lanes; in one C/C++ lane 23,000 lines of tools against about 530 lines of product code, with four recorded budget overruns. | research section 7 |
| The benchmark harness runs review taint for every case as a "comparison channel" and discards the output. Ground truth is usable for 192 of 583 acquired packages. Its metadata names a fixed version or a fix commit for 294 of 600 entries (prototype pollution 125, ReDoS 82, command injection 47, code injection 22, path traversal 18); only vulnerable versions are acquired. | `eval/secbench/run.py`; research section 4 |
| Corpora: pinned with receipts are Excalidraw, the benchmark packages, the accuracy-harness anchors, TypeScript 5.9.3 and a cloud store of public corpora. Of 49 local clones, 42 are shallow; seven have full history, and six of those are the repositories of the 120-diff population. | research section 3 |
| Reports from three lanes share one structure: a binding receipt (binary hash, commits, corpus and oracle pins), a prediction written before the run, explicit denominators, disjoint named classes with a certificate that nothing is unattributed, verdicts CORRECT / WRONG / UNDECIDED / INADMISSIBLE, controls, limits. | research section 5 |
| Draft PR #841 compares review output over the same 120 diffs (previous, off, on) with a coordinator of its own, and has run its primary rows. The kit does not replace it; offering the kit as a cross-check is deferred out of the lane (section 7, decision 4). | PR #841 |
| Cloud sessions have 4 vCPUs and 15 GB with no swap; result pushes over 100 MB fail silently. | research section 6 |

## 3. Goals, non-goals, anti-goals

Goals:

1. One command set, in the repository, for: measuring a binary on a corpus with a receipt; comparing two
   measurements with class attribution; comparing counters; grading added call targets and edges with the
   TypeScript checker, for the relations section 5 names; comparing review output over a fixed population
   of real diffs.
2. Lane-specific knowledge stays out of the kit: a lane supplies a small classifier file and, when needed,
   an oracle question. The kit fails a gate on any unattributed change.
3. The benchmark gains finding-level accounting (a case-level line-hit rate and one row per emitted sink
   finding, with no precision claim), and a negative set of verified fixed pairs.
4. A gate refuses to run without a prediction bound before the first measurement, and every output starts
   with a binding receipt.
5. Proof that it works: the kit reproduces the recorded numbers of one finished lane's gate, attributes an
   independently labelled non-zero set of changes row for row, fails a deliberately broken trace join, and
   one new lane runs its measurements with it without writing a comparator.

Non-goals: timing and memory measurement (they have tooling); mutation gates; the accuracy harness; oracles
for C, C++, Java, Go or Python edges (the clang labeller of the X-READ lane and the language-server oracles
stay where they are); changing how lanes are run; the cloud launchers (section 7, decision 3).

Anti-goals: a framework that lanes must fight (anything a lane cannot express in a classifier file stays
possible as a plain script that reads the kit's outputs); private corpus names or results in the
repository; a kit that only its author can run.

## 4. Approaches considered

**A. Collect the best existing scripts into the repository unchanged.** Fast, but it would commit seven
comparator designs and hard-coded paths, and the next lane would fork again. Rejected.

**B. A small library and command-line tool with plug-in classifiers** (chosen): standard-library Python,
runnable on the host and in a cloud session, with unit tests in CI; Node scripts for the TypeScript oracle
against the pinned compiler. The comparator's core (keys, multiset difference, label moves, certificates)
is common; the classes are a lane's file.

**C. Build the comparison into the product** (`prism nav diff-edges`). It would ship measurement code to
users and tie the oracle's logic to the binary under test. Rejected.

## 5. Architecture

Location: `scripts/measure/` (standard-library Python 3.9 or later, like the other script suites that CI
tests), with the Node oracle under `scripts/measure/oracle/`. One entry point, `python3 -m measure`
(run from `scripts/`), with subcommands.

| Subcommand | Does | Output |
|---|---|---|
| `corpora list` / `corpora verify` | reads the corpus manifest: name, kind, pin (commit or archive hash), how to find it locally, cloud asset name, whether history is available | a table; non-zero exit on a pin mismatch |
| `run` | runs one binary on one corpus: `nav dfg-stats`, `nav call-stats`, `nav dfg-stats --edges` (with byte offsets, gzip-compressed), optional environment variables for product traces; through the memory guard, refusing while a host-quiet marker is held; binds the input and the prediction (Binding, below) | a directory with the dumps and `receipt.json` (binary SHA-256 and `--version`, corpus pin, input digest, prediction digest, command lines, environment, output digests, start and end time) |
| `diff-edges` | compares two bound `run` directories under Edge identity and Trace join (below); calls the lane's classifier for each change and its validation hook once | `changes.jsonl`, a class table, a certificate; exit 1 on any unattributed change, unresolved ambiguity or failed validation |
| `diff-stats` | flattens the two statistics files and prints every counter delta; an allow-list file says which counters may move | a table; exit 1 on a move outside the list |
| `oracle ts-calls` / `oracle ts-edges` | grades call-target deltas or edges with the TypeScript checker under Oracle questions (below; verdicts CORRECT, WRONG, UNDECIDED, INADMISSIBLE); a lane may pass its own question module for relations the kit does not know | `verdicts.jsonl` and tuple counts; ships with self-check fixtures that must pass before any use |
| `review-diff` | for each diff of a population, runs review with two binaries (or one binary with two flag sets) and compares findings under Review comparison (below) | per-diff and total presence and payload changes; a seeded sample file for a reader |
| `gate` | runs a lane's gate file (a JSON list of the steps above with thresholds) and refuses to start unless the prediction is bound (Binding) | the report skeleton of section 2's common structure, filled with the results |

The rules below are stated once here; the plan refers to them by name.

**Binding.** `run` computes an input digest of the query root (every regular file, and every symlink as its
target text, `.git` excluded) before the first command and again after the last; a difference fails the run.
A git corpus also needs `HEAD` equal to the pinned commit and no untracked or ignored file under the query
root (the loader reads ignored files). The receipt records the input digest, every command's options and
output digest, and, when a prediction is given, its SHA-256, captured before the first command. Every
comparator and oracle entry verifies both receipts (successful, output digests intact) and refuses two runs
whose input digests or options differ beyond the declared arm difference (binary, trace environment). A gate
accepts a reused run only when its receipt carries the gate's prediction digest; modification times are not
evidence, and a receipt without a digest is replayed only as `retrospective`, never as a gate pass.

**Edge identity.** An edge's key is its two endpoints, each `(file, path base, path fields as a list, line,
start_byte, end_byte)`; the path is kept structured, never joined into a string. Its attributes are the two
endpoint roles (`access`), `confidence`, `doubt` and `kill_line`. One comparison uses one key mode for both
dumps: byte mode when every endpoint of both dumps carries both bounds; line mode (bytes dropped on both
sides) only when both dumps are line-keyed or when asked explicitly. Absent or partial bounds refuse byte
mode; modes are never mixed per endpoint. Rows are compared as decoded multisets (order and compression are
transport; compressed hashes are kept for custody). Per key, identical attribute occurrences cancel first. A
residual with one side empty is `added` or `removed` with its count. Otherwise the pairing is unique when the
larger residual side (either side when equal) holds identical copies: `min(b, h)` `label_moved` rows, each
with its transition, and the surplus as `added` or `removed`. Any other residual is `ambiguous`: both bags
are kept and count as unattributed unless the classifier attributes the whole bag. Every change carries its
residual bags, so both input multisets rebuild from the unchanged rows plus the changes. Reports give
changed keys and class occurrences in separate columns. Line mode also reports keys with multiplicity above
one on either side (collisions), even when they produce no change. Zero-width spans are valid keys.

**Trace join.** A lane's parser turns a product trace line into a typed record `{arm, tag, kind, key}`.
`kind` is `edge` for a record naming one edge (or endpoint); other kinds (per-write evidence, owner ranges,
counter predictions) reach the classifier unjoined. An `edge` record matches the edges of its arm whose key
agrees on every field the record carries (a trace that gives only `path@line:start_byte` matches on those).
It attaches only when exactly one edge of its side matches and that edge is changed. A record matching an
unchanged edge is a stray, matching several edges is `ambiguous`, and matching none is `unmatched`; none of
them is ever borrowed by a neighbouring change. A record attaches to at most one change; a change may carry
several records, and the classifier decides how many count. After `classify`, the kit calls the classifier's
`validate(bound)` once with every change, the unchanged-key index, every record (attached, stray,
ambiguous, unmatched) and the counter deltas; it returns failures, such as a forbidden stray, a counter delta
its mechanism does not predict, or a class the lane forbids. Strays fail unless the classifier allows them.
`classify` receives the corpus root and both run directories, so a lane may read source or a sidecar it
ships; the kit standardises no sidecar.

**Oracle questions.** The oracle dispatches on the edge's endpoint roles and relation, keeps the original
row, and grades only the property it proves (recorded in the verdict). Built in: (1) *argument binding*, a
use in a call argument feeding a formal's def: CORRECT when the checker resolves the call to one in-repo
callable whose parameter at the argument's position has the formal's bytes; WRONG when it resolves uniquely
to another callable or another position; (2) *lexical binding*, def to use: CORRECT when both identifiers
resolve to one declaration and the use is a read, WRONG when they resolve to different declarations or the
use is a write or type position; reachability and intervening writes are not graded (a lane question does
that). Any other relation is UNDECIDED (`relation-not-graded`); rest and spread arguments, ambiguous owners
and unresolved callables are UNDECIDED with their reason. INADMISSIBLE: absent bytes, or the oracle's source
differs from the run's input digest. On authenticated source, bytes that cover a different identifier are
WRONG. *Call targets*: `ts-calls` diffs the site-and-target multisets of two bound runs and reports new,
lost, retargeted, confidence-only and multi-target deltas with denominators, dropped and non-exact sites
included; a full census of one run stays a separate mode. A target (`resolved_targets` entry with exact
confidence; `exact_target` can be null for it) is CORRECT when the checker's declaration of the callee at
the site bytes has the target's name, file and start line and is the only declaration matching them; WRONG
when it is uniquely another in-repo declaration; UNDECIDED when none, external or not unique. Span
containment alone is never evidence. The self-check runs, per question, positive and near-miss negative
fixtures; its passing receipt is keyed by checker tree, compiler options, question and fixture digest, and
grading refuses without one.

**Review comparison.** Findings are read once per arm (`findings` of a single run or `all_findings` of a
multi-run, never both). A finding is its canonical payload: every serialised field, with set-valued fields
(rule ids, `related_lines`, `related_files`) sorted and a missing rule field distinguished from an empty
list. Per anchor `(file, line, algorithm, category)` identical payloads cancel with multiplicity. The residual
is reported in two columns: presence changes (anchor counts differ) and payload changes (fields changed, rule
transitions, text-only changes), with duplicates that cannot be paired kept as ambiguous bags. A zero means
"no payload change on any anchor". Each manifest ordinal has an accounting row, and a refused or failed arm
fails its row.

A lane's classifier is a Python file exporting `classify(change, context) -> str or None`, the list of class
names it can return, and optionally the trace parser, the stray policy and `validate`; `None` means
unattributed. The kit ships generic classes only for what is true in every lane (identical edge moved by a
line-number shift in an untouched file is not one of them: the kit does not guess).

The benchmark work is in `eval/secbench/`, not in the kit:

- **Finding-level accounting.** Kept beside the case-level reach tables, never replacing them. Per case, a
  line-hit outcome: a `taint_sink` finding on the ground-truth sink line, or `ambiguous` when that line holds
  another eligible sink occurrence. Per emitted `taint_sink` finding, one row matched one-to-one to the case's
  annotated occurrence (and rule family once findings carry rule identity, lane 7): matched, ambiguous or
  off-target. An off-target finding is unadjudicated, not a false alarm; the report names the metric a
  line-hit rate and claims no precision.
- **Negatives.** For entries whose metadata names a fixed version, the fixed package is acquired as an archive
  (never installed or executed; acquisition contract in PLAN S6b). An entry enters the negative denominator
  only as a verified fixed pair: (1) the vulnerable version reports or reaches the annotated sink (positive
  control); (2) the sink's function is found by name, uniquely, in the same file of the fixed version; (3) the
  annotated sink occurrence is mapped there (same callee text, unique) or verified removed. Moved, renamed,
  split or duplicated functions, a fix elsewhere, a partial fix and any unverified mapping are reasons, not
  scores. For a verified pair, the finding at the mapped occurrence, reach to the mapped occurrence and the
  function's change (token sequence from the pinned TypeScript scanner, so literals keep their content) are
  separate columns. "Still reported" bounds false alarms within the verified subset only.

The diff population for `review-diff` is a committed manifest of public repositories and commit ids (the
120 diffs already used for issue #602), with no content.

The memory guard moves into `scripts/memguard/` with its tests, after a check that it contains nothing
machine-specific or private; the cloud launchers stay where they are (section 7, decision 3).

## 6. Gates

| Gate | Statement |
|---|---|
| G-repro | Two parts, both gating, on the binaries built at the base and head of the alias-kill slice 4b (#786: `4f9e3a3e`, `1f2ad1cc`), which both emit occurrence bytes. (a) *Identity control:* `run` plus `diff-edges` (line and byte mode) and `diff-stats`, with a classifier ported from that lane, reproduce its recorded comparison exactly on Excalidraw, express and the TypeScript sources: 0 changes, identical counters, the recorded per-mechanism trace counts. (b) *Labelled pair* (changed by the design review, 2026-11-21): the same binaries on a lane-written public fixture corpus built from the slice's own test tables (`src/cpg/reaching/alias_kill_tests.rs` at `1f2ad1cc`, whose base and head label columns are the truth, written down before any kit run) give, in byte mode, a non-zero set of moves in at least two mechanisms whose per-row and per-class results equal the truth, with unchanged rows unchanged; and each mutated control (a trace record moved to the neighbouring occurrence, deleted, swapped between mechanisms, made a stray on an unchanged edge, byte-shifted) makes the gate fail for its named reason. The #660 S1a slice that moved edges cannot serve: its serializers emit no bytes (section 2) |
| G-selfcheck | the oracle's self-check fixtures pass: per question, a case per verdict plus near-miss negatives (wrong slot, wrong callable, shadowed binding, type-position identifier, two declarations on one line, nested declaration) |
| G-hermetic | the kit's unit tests and the matrix test run in CI without network, a prism binary or a corpus |
| G-adopt | a consuming lane named before stage D (lane 3, 9 or 10; the planning session recommends two, each for the half of the kit it exercises: lane 10 for the comparator, because its slices need both comparison modes, label moves and kill attributes in three languages and its plan already prefers the kit, and lane 9 for the oracle, because it would otherwise extend its own checker grader for the same argument-binding question; **confirmed by the owner on 2026-11-24**: lane 10 adopts the comparator, lane 9 adopts the oracle, and lane 11 starts first), with its next unmeasured differential, the inputs and classes it needs, and who hands off) runs that differential with the kit, writing a classifier file with any arm evidence sidecar registered by arm and digest under `classifier_files`, at most one oracle question module, gate files and predictions, and then takes a decision of its own on the kit's output alone. Migrating between its stage M and stage G is accepted when a bound replay of its baseline agrees with its existing comparator. A filed gap does not satisfy the gate. If no consumer is secured before stage D, the formats ship marked provisional and G-adopt is reported not met |
| G-findings | on the benchmark at the current baseline build, by replay of a bound observation set whose binding defines the population: reach outcomes unchanged, a finding-level row on every case with ground truth, at least one line hit, and live and replay scoring equal on a small independently labelled set |
| G-negatives | every attempted entry with a named fixed version has an outcome or one recorded reason, and at least N verified fixed pairs are scored, N fixed after the candidate census in stage E and written in the prediction; fewer is reported as unmet, never as a pass |

## 7. Owner decisions

| # | Question | Default applied | Cost of changing later | Latest cheap point |
|---|---|---|---|---|
| 1 | Where the kit lives | `scripts/measure/`, standard-library Python, CI unit tests | moving into the `eval` project is a packaging change | before S1 |
| 2 | Download fixed benchmark versions from the package registry | yes: archives only, never installed or run, into the evidence inputs directory, pinned by hash | without it there are no negatives | before S6 |
| 3 | Move the memory guard and the cloud launchers into the public repository | the memory guard yes (S1; `run` needs it); the cloud launchers are deferred out of the lane (changed by the design review, 2026-11-21): a follow-up issue, for the first lane that needs a kit gate in the cloud | they stay in the evidence directory | when a lane needs it |
| 4 | Offer `review-diff` to PR #841 as an independent cross-check | deferred out of the lane (changed by the design review, 2026-11-21): a follow-up issue, taken up only if #841's merge decision needs an independent count | — | — |
| 5 | Retire the forked scripts | no deletion of evidence; new lanes use the kit, old evidence stays as it is | — | — |
| 6 | Does the second reproduction (a non-zero, independently labelled change) gate the kit | yes (changed by the design review, 2026-11-21): G-repro (b), on a lane-written fixture pair the lane rebuilds with byte offsets, because the historical slice that moved edges has no bytes | the fixture pair is cheap; an unbuildable pair falls back as section 8 says | before stage T |

## 8. Risks and accepted limits

- **Class attribution depends on product traces that differ per lane.** The kit standardises only the join
  and the validation hook; it cannot make old traces uniform, and a trace that names an edge by start byte
  only can attach only where that start byte is unique.
- **A reproduction target can rot.** If the 4b commits cannot be rebuilt (toolchain drift), part (a) falls
  back to the earlier #783 round (`4c8620a2`/`4da81c36`, line mode only: no bytes) and part (b) to the
  nearest later byte-bearing slice whose test tables give per-row labels, recorded with the reason.
- **Negatives are few.** Only verified fixed pairs count; the candidate census fixes N, and a small N is
  reported as such rather than padded with unverified mappings.
- **Shallow clones.** History-based measurements work only on the seven full clones and on corpora the
  manifest marks as having history.
- **The kit can become a second product.** Its scope is the table in section 5; anything else stays a lane's
  own script.

Recorded, not pursued (design review, 2026-11-21):

- A versioned sidecar format for enriched identities (owner, parameter, site kind): the classifier can read
  source or its own sidecar; standardise when two lanes need the same one.
- A source-diff mode comparing runs on different trees through verified rename and line mappings: binary
  mode refuses mismatched trees instead.
- Blind adjudication of off-target benchmark findings for a precision estimate: the line-hit rate is reported
  as what it is.
