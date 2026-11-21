# Lane 11: a shared measurement kit — design

Status: architecture final; command names and file formats are fixed at stage D after the inventory of
existing scripts (PLAN stage M). The plan files a tracking issue. Research record:
`~/prism-evidence/lanes-2026-11-20/research/lane11-measurement-kit.md`.

## 1. Intent

Every lane measures its change before it merges: both binaries on fixed corpora, an edge and counter
differential with every change attributed to a named class, an oracle that grades what was added, a
benchmark run, sometimes a replay over real diffs. Each lane has rebuilt those scripts in its private
evidence directory, and they are offloaded when the lane closes. The kit puts one maintained, tested copy of
the common parts in the repository, so a lane writes only what is specific to it (its classes, its oracle
question), gates are reproducible by someone else, and the cloud runner can execute them.

It also fills two holes in what can be measured at all: the security benchmark scores reach only, never
findings, and has no negatives.

## 2. What exists today (verified on `origin/main` at 1179055f and in the evidence directories)

| Fact | Reference |
|---|---|
| Committed and tested: the call-resolution accuracy harness with language-server oracles (`eval/tier_a`), the benchmark harness (`eval/secbench`), paired timing in the cloud (`scripts/perf/cloud`), host-quiet markers, the pinned TypeScript acquisition (`scripts/gate-inputs`), a byte-comparison script over fixture diffs. CI runs only their hermetic unit tests. | `eval/`, `scripts/`, `.github/workflows/ci.yml` |
| Not committed, rebuilt per lane: (a) the "run both binaries, dump statistics and edges" recipe, at least 13 near-copies; (b) the edge comparator with class attribution, about 20 files in six or seven designs (one file byte-identical in nine places, one helper forked eight ways), whose attribution reads lane-specific trace lines the product prints under an environment variable; (c) counter comparison; (d) TypeScript-checker oracles, at least eight different `census*.cjs`; (f) review-output comparison over 120 real diffs, seven scripts, with the diff population stored only in an archive; (g) the cloud launchers; (h) the memory guard. | research section 2 |
| Tooling cost per lane, from ledgers: 959 to 3,500 lines of scripts in three recent data-flow lanes; in one C/C++ lane 23,000 lines of tools against about 530 lines of product code, with four recorded budget overruns. | research section 7 |
| The benchmark harness runs review taint for every case as a "comparison channel" and discards the output. Ground truth is usable for 192 of 583 acquired packages. Its metadata names a fixed version or a fix commit for 294 of 600 entries (prototype pollution 125, ReDoS 82, command injection 47, code injection 22, path traversal 18); only vulnerable versions are acquired. | `eval/secbench/run.py`; research section 4 |
| Corpora: pinned with receipts are Excalidraw, the benchmark packages, the accuracy-harness anchors, TypeScript 5.9.3 and a cloud store of public corpora. Of 49 local clones, 42 are shallow; seven have full history, and six of those are the repositories of the 120-diff population. | research section 3 |
| Reports from three lanes share one structure: a binding receipt (binary hash, commits, corpus and oracle pins), a prediction written before the run, explicit denominators, disjoint named classes with a certificate that nothing is unattributed, verdicts CORRECT / WRONG / UNDECIDED / INADMISSIBLE, controls, limits. | research section 5 |
| Draft PR #841 still has to build a comparison over the same 120 diffs (previous, off, on) before it merges. | PR #841 |
| Cloud sessions have 4 vCPUs and 15 GB with no swap; result pushes over 100 MB fail silently. | research section 6 |

## 3. Goals, non-goals, anti-goals

Goals:

1. One command set, in the repository, for: measuring a binary on a corpus with a receipt; comparing two
   measurements with class attribution; comparing counters; grading added call targets and edges with the
   TypeScript checker; comparing review output over a fixed population of real diffs.
2. Lane-specific knowledge stays out of the kit: a lane supplies a small classifier file and, when needed,
   an oracle question. The kit fails a gate on any unattributed change.
3. The benchmark gains finding-level scoring, and a negative set built from fixed versions.
4. A gate refuses to run without a written prediction, and every output starts with a binding receipt.
5. Proof that it works: the kit reproduces the recorded numbers of one finished lane's gate, and one new
   lane runs its measurements with it without writing a comparator.

Non-goals: timing and memory measurement (they have tooling); mutation gates; the accuracy harness; oracles
for C, C++, Java, Go or Python edges (the clang labeller of the X-READ lane and the language-server oracles
stay where they are); changing how lanes are run.

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
| `run` | runs one binary on one corpus: `nav dfg-stats`, `nav call-stats`, `nav dfg-stats --edges` (with byte offsets, gzip-compressed), optional environment variables for product traces; through the memory guard, refusing while a host-quiet marker is held | a directory with the dumps and `receipt.json` (binary SHA-256 and `--version`, corpus pin, command lines, environment, start and end time) |
| `diff-edges` | compares two `run` directories: added, removed and label-moved edges keyed by file, path, line and bytes at both ends; calls the lane's classifier for each change; joins product trace lines through a lane-supplied parser | `changes.jsonl`, a class table, a certificate; exit 1 on any unattributed change |
| `diff-stats` | flattens the two statistics files and prints every counter delta; an allow-list file says which counters may move | a table; exit 1 on a move outside the list |
| `oracle ts-calls` / `oracle ts-edges` | grades call targets or cross-function edges with the TypeScript checker (verdicts CORRECT, WRONG, UNDECIDED, INADMISSIBLE); a lane may pass its own question module for edge kinds the kit does not know | `verdicts.jsonl` and tuple counts; ships with self-check fixtures that must pass before any use |
| `review-diff` | for each diff of a population, runs review with two binaries (or one binary with two flag sets) and compares findings by file, line, algorithm, category and rule | per-diff and total added, removed, changed; a seeded sample file for a reader |
| `gate` | runs a lane's gate file (a JSON list of the steps above with thresholds) and refuses to start unless a prediction file exists and is older than the first measurement | the report skeleton of section 2's common structure, filled with the results |

A lane's classifier is a Python file exporting `classify(change, context) -> str or None` and a list of the
class names it can return; `None` means unattributed. The kit ships generic classes only for what is true in
every lane (identical edge moved by a line-number shift in an untouched file is not one of them: the kit
does not guess).

The benchmark work is in `eval/secbench/`, not in the kit:

- **Finding-level scoring.** The harness keeps the review-taint output it already produces per case and adds
  an outcome: does a `taint_sink` finding lie on the ground-truth sink line, and (once findings carry rule
  identity, lane 7) with which rule. Reported beside the reach outcome, never replacing it.
- **Negatives.** For entries whose metadata names a fixed version, the fixed package is acquired as an
  archive (never installed or executed), the function that contains the ground-truth sink is located in it
  by name, and the same queries run there. A finding or a Reached verdict in the fixed version's function is
  reported as "still reported after the fix", with the caveat that a fix elsewhere in the package leaves
  the function unchanged; such cases are separated by comparing the function's text across versions.

The diff population for `review-diff` is a committed manifest of public repositories and commit ids (the
120 diffs already used for issue #602), with no content.

The cloud launchers and the memory guard move into `scripts/` with their tests, after a check that they
contain nothing machine-specific or private.

## 6. Gates

| Gate | Statement |
|---|---|
| G-repro | with binaries built at the base and head commits of one finished gate (the alias-kill slice, #783, whose scripts and summaries survive), `run` plus `diff-edges` with a classifier ported from that lane reproduce its recorded class counts on Excalidraw and the TypeScript sources exactly |
| G-selfcheck | the oracle's self-check fixtures pass (authored cases for each verdict, including a deliberately wrong edge) |
| G-hermetic | the kit's unit tests run in CI without network, a prism binary or a corpus |
| G-adopt | one lane of this plan set that needs an edge differential (lane 3, 9 or 10, whichever starts first) runs its stage M and stage G with the kit and adds only a classifier file |
| G-findings | on the benchmark at the current baseline build, finding-level outcomes are produced for every case with ground truth, and the reach outcomes are unchanged from a run without the change |
| G-negatives | for at least 100 entries with a named fixed version, the fixed package is acquired and scored, or the reason it could not be is recorded per entry |

## 7. Owner decisions

| # | Question | Default applied | Cost of changing later | Latest cheap point |
|---|---|---|---|---|
| 1 | Where the kit lives | `scripts/measure/`, standard-library Python, CI unit tests | moving into the `eval` project is a packaging change | before S1 |
| 2 | Download fixed benchmark versions from the package registry | yes: archives only, never installed or run, into the evidence inputs directory, pinned by hash | without it there are no negatives | before S6 |
| 3 | Move the cloud launchers and memory guard into the public repository | yes, after the check for machine-specific content | they stay in the evidence directory | before S7 |
| 4 | Serve PR #841's pending 120-diff comparison with `review-diff` | offer it: build that subcommand early and tell that lane; do not block on it | — | at S4 |
| 5 | Retire the forked scripts | no deletion of evidence; new lanes use the kit, old evidence stays as it is | — | — |

## 8. Risks and accepted limits

- **Class attribution depends on product traces that differ per lane.** The kit standardises only the join
  (a parser function per lane); it cannot make old traces uniform.
- **A reproduction target can rot.** If the chosen finished gate cannot be rebuilt (toolchain drift), G-repro
  falls back to the next most recent finished gate, recorded with the reason.
- **Negatives are noisy.** A fixed version can keep the flagged function unchanged; the function-text
  comparison separates those, and the number is reported as an upper bound on false alarms, not a rate.
- **Shallow clones.** History-based measurements work only on the seven full clones and on corpora the
  manifest marks as having history.
- **The kit can become a second product.** Its scope is the table in section 5; anything else stays a lane's
  own script.
