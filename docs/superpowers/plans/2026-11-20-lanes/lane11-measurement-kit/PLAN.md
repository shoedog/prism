# Lane 11: a shared measurement kit — implementation plan

> **For agentic workers:** run by an orchestrator under `../ORCHESTRATION.md`. Each task goes to an implementer
> (Sonnet-5.5, high) with its section as the brief; briefs for S1 to S5 also carry `FORMATS.md` (stage D) and
> DESIGN section 5 verbatim. In test listings, a `def` line ending in a comment is a test to write; the comment
> is its assertion.

**Goal:** one tested measurement kit in the repository, so a lane writes only a classifier file (and, when
needed, an oracle question); plus finding-level accounting and verified negatives for the security benchmark.

**Architecture:** a standard-library Python package `scripts/measure/` run as `python3 -m measure` from
`scripts/`; a Node oracle in `scripts/measure/oracle/` on the pinned TypeScript 5.9.3; benchmark work in
`eval/secbench/`; the memory guard moved to `scripts/memguard/`. The cloud launchers are not moved (DESIGN
section 7, decision 3).

**Tech stack:** Python 3.9+ standard library, `unittest`; Node with the TypeScript tree that
`scripts/gate-inputs/acquire.mjs` installs; the eval project (uv, pytest). **Spec:** `DESIGN.md` here; its
section 5 rules (Binding, Edge identity, Trace join, Oracle questions, Review comparison, the benchmark
bullets) are referred to by name below and not restated.

## Global constraints

- Worktrees (ORCHESTRATION section 4): `~/code/prism-measure-kit` (sparse) for M to S5 and S8;
  `~/code/prism-measure-kit-bench` (no sparse-checkout: it reads and edits `eval/secbench/`) for the M-2
  harness reads, the stage-T benchmark rows, S6a and S6b, with its own branches. The two streams merge in
  their own order (S6a before S6b: both touch `report.py` and the harness README); host runs of both go one
  at a time through the guard. Evidence `~/prism-evidence/measure-kit/` (`LEDGER.md`, `HANDOFF.md`,
  `inventory/`, `repro/`, `secbench/`).
- `scripts/measure/`, `scripts/memguard/`: standard library, Python 3.9 syntax (`from __future__ import
  annotations`, no `match`). Unit tests use no network, prism binary, corpus or TypeScript; fakes are small
  scripts written to a temporary directory.
- Committed files: no private corpus name, no absolute machine path, no raw corpus dump (the lane-written
  fixture corpora and matrix cases are authored public inputs). Private-name check of ORCHESTRATION section 9
  before every commit; briefs name the private evidence directories as not to be opened and say that
  sentence is not to be copied into repository files.
- Every prism, `cargo` or TypeScript-checker run goes through the guard (`~/prism-evidence/tools/memguard.sh
  8`, from S1 on `scripts/memguard/memguard.sh 8`), one at a time; on exit 75 run `python3
  scripts/hostquiet/host_quiet.py wait` and retry. Counting runs take no marker.
- Cloud (the kit must run there unchanged): 4 vCPUs, 15 GB, no swap, ~30 GB disk, no `/usr/bin/time`,
  REST-only GitHub, silent push failure for any file over 100 MB, nothing private.
- Per-PR gates (ORCHESTRATION section 5): 3, 4, 8, 10 (kit README in the same PR) and 11 apply; 1 runs in CI.
  Item 2 is the slice's unit-test command. Items 5, 6, 7, 9 do not apply: no Rust change.
- Every PR body: what changed, formats as built, test totals (local and CI), the matrix groups enabled, the
  slice gate's prediction and result, what was not verified, the review record. Branches
  `feat/measure-kit-<slice>`.
- Review seats (caps: design 2 rounds, slice 3 rounds):

| Work | Reviewers | Why |
|---|---|---|
| Stage D, S2, S3 | A (Opus-5.5 xhigh) and B (gpt-6.1-sol xhigh) | formats bind every lane; a false PASS in `diff-edges` or a wrong oracle verdict passes unseen through every lane's merge gate |
| S6b | one, Opus-5.5 high | registry download and archive extraction are a security boundary, and the negative denominator is a truth rule |
| Stage T matrix, S1, S4, S5, S6a, S8 | one, Sonnet-5.5 high (the manual's tooling reviewer) | failures are visible (refusals, receipts, reports) or proven by a gate; the matrix PR is test-only |

## Review focus

| # | Input or condition | Required | Pinned by |
|---|---|---|---|
| 1 | Edge dump too large to hold as records (TypeScript `src`: 408,954 edges, ~108 MB raw) | streamed from gzip; per-key attribute counts only | `test_diff_edges_streams_large_gzip_within_memory_bound` (S2) |
| 2 | Two same-line reads with their labels exchanged (review probe P6) | two label moves in byte mode; none in line mode, one collision key reported | `test_byte_only_difference_is_a_change_in_byte_mode_and_not_in_line_mode` (S2) |
| 3 | One dump with bytes, the other without, same input (probe P2) | byte mode refused; explicit line mode gives 0 changes | `test_mixed_capability_refuses_byte_mode` (S2) |
| 4 | One Exact becoming two NameOnly copies on one key (probe P4) | one label move plus one addition | `test_one_exact_to_two_nameonly_is_one_move_and_one_addition` (S2) |
| 5 | A trace record naming one of two same-line reads (probe P6) | attaches in byte mode; ambiguous, never borrowed, in line mode | `test_trace_record_attaches_only_to_a_unique_changed_edge` (S2) |
| 6 | A classifier that raises or returns an undeclared name | change unattributed, exception recorded, exit 1 | `test_classifier_exception_counts_as_unattributed` (S2) |
| 7 | Corpus pin mismatch, dirty checkout, or an ignored file under the query root (probe P5) | refused before the binary runs | `test_run_refuses_ignored_file_under_root` (S1) |
| 8 | A prediction replaced after the run, its old modification time kept | `gate` refuses: the receipt does not carry its digest | `test_gate_refuses_reused_run_without_matching_prediction_digest` (S5) |

## Stage M: inventory, reproduction targets, consumer (2.5 days)

This lane builds tooling: stage T is the executed truth matrix below, stage E enumerates it, and each slice
enables its matrix group tests-first; stage G is the gates named per slice.

- [ ] **M-1.** Both worktrees, evidence directory, tracking issue "Shared measurement kit (lane 11)" linking
      `DESIGN.md` and this plan; its number in `LEDGER.md`.

### Task M-2: inventory of the forks (measurement runner seat)

**Output (evidence only, never committed):** `inventory/INVENTORY.md`. Read each file (under
`~/prism-evidence/`, else the copy in `~/code/prism-evidence-repo/`; restore nothing from release assets).

| Family | Files (evidence paths unless marked) |
|---|---|
| (a) run recipe | `jsts-lane/783/scripts/{measure783.sh,gate783.sh,measure761-cloud.sh,timewrap.py}`; `jsts-lane/{599,627,664,759}/scripts/measure*.sh`; `jsts-lane/660/s*/scripts/measure660-*-cloud.sh`; `jsts-lane/693/scripts/{measure693.sh,run-measure.sh}`; `provenance-survey/slice4ab/scripts/{gate4b.sh,parity.sh}` |
| (b) edges | `jsts-lane/783/scripts/{compare783.py,compare761.py}`; `provenance-survey/slice4ab/scripts/compare4b.py`; `jsts-lane/660/*/scripts/{compare_edges.py,join_trace.py}` (7 slices); `jsts-lane/759/scripts/{join_trace.py,check_counters.py}`; `jsts-lane/693/scripts/compare693.py`; `perf-437/measure-645/{labeldiff.py,r5/labeldiff_strat.py}`; `provenance-survey/scripts/{edgediff,attrib,transitions}.py`; `nav-gaps/scripts/site_diff.py`; `provenance-survey/slice4c/scripts/run_gate_4c.py`; repository: `docs/superpowers/plans/2026-10-06-js-param-defs/probes/{rowdiff.py,byte_dump.rs}` |
| (c) counters | the `stats` modes above; `jsts-lane/783/scripts/{attribute_counters.py,attribute_return_inputs.py}` |
| (d) oracle | `js-args/{census.cjs,selfcheck.cjs}`; repository: `docs/superpowers/plans/2026-10-06-js-param-defs/probes/{adjudicate.cjs,census.cjs,oracle-read-role.cjs}`, `…/2026-10-04-workspace-package-resolution/probes/census.cjs`, `…/2026-10-03-s2-import-qualifiers/probes/census.cjs`, `eval/tier_a/ts_syntax.cjs`, `eval/secbench/{inspect.mjs,bindings.mjs}` (bench worktree) |
| (f) review output | `jsts-lane/602/m2/scripts/*.py`; `DISPATCH.md`, `population.jsonl` from `archive/prism-crossfile-docs-493ab5ee-receipts.tar.gz` (member dir `next-five-plan-fable-20261114/final-gate-campaign-preparation/`; extract to `inventory/602/`) |
| (e) other-language oracles | not read: out of scope (DESIGN section 3 non-goals) |
| (h) guard | `tools/memguard.sh` (the cloud launchers (g) are deferred: DESIGN section 7, decision 3) |

- [ ] One table per family, a row per behaviour (input form, key field, label rule, residual pairing, exit
      condition, output line, refusal): the files that have it; verdict `keep: <subcommand>`, `lane:
      classifier` or `drop: <reason>`; mark disagreements between forks (`kill_line` and `access` in the
      label, gzip, `[]`, failing strays, owner-only replacements) for stage T rows. Also list each trace
      prefix parsed (`rd-census`, `rd-census-owner`, `rd-alias-cert`, `rd-alias-move`, `own-*`, `c-prune-*`)
      with its fields, its record kind (edge, write, owner range, counter) and the consumer decision it
      serves, each corpus with its pin, each machine-specific string in (h).

### Task M-3: reproduction targets for G-repro (DESIGN section 6)

**(a) Identity control.** #783's alias-kill slice 4b, PR #786 (squash `ae13feee`):

| Item | Value |
|---|---|
| Base / head | `4f9e3a3e` (main at merge; the PR states it is byte-identical to `ead6edf0` and the 4a tip) / `1f2ad1cc` (PR head, fold 3b; binary `prism-4b-fold3b`, SHA-256 `c52604ea…` in the evidence repository's `manifests/provenance-survey-slices.md`) |
| Recorded numbers | evidence repository `provenance-survey/slice4ab/results/gate-4b-compare-{X,EX,T}.txt`, `gate-4b-f-identity.txt`; PR #786 body, "Gates" |
| Expected | edges 80,033 / 7,445 / 388,055 (excalidraw / express / TypeScript `src`); 0 added, removed and label-moved; `dfg-stats` and `call-stats` identical; per-mechanism trace counts as in each compare file |

- [ ] Build both commits (`cargo build --release`, guarded) in detached scratch worktrees under the evidence
      directory; keep `repro/bin/prism-{base,head}`; delete the scratch `target/`s. Check that
      `PRISM_EDGES_BYTES=1` gives both bounds on every endpoint of both binaries.
- [ ] Before any kit run, identify from `gate-4b.log` and the PR's commit list which fold's head produced the
      recorded trace counts and bind it in `repro/TARGET.md`, with any alternative fold named in advance in
      order; no fold is tried after seeing kit totals.
- [ ] Fallback (DESIGN section 8), on toolchain drift only: the earlier #783 round in `jsts-lane/783/` (base
      `4c8620a2`, head `4da81c36`; `local/gate/{X,EX,T}.compare.txt`: 85,645 / 7,691 / 408,954 edges), line
      mode only (no bytes).

**(b) Labelled pair** (gating; DESIGN section 7, decision 6, changed by the design review). The #660 S1a
pair cannot serve: its serializers emit no bytes (DESIGN section 2).

- [ ] Build `repro/fixture-4b/`: one source file per case of the tables in
      `src/cpg/reaching/alias_kill_tests.rs` at `1f2ad1cc` (`CASES` and the parity pairs; extension by the
      case's language), and `truth.jsonl`, one row per table edge: `{"file", "edge"` (as the table spells it),
      `"base", "head", "class"}`, with the table's `NameOnly(AliasUnstable)` written as the dump's
      `nameonly`/`alias_unstable`; class `kill:alias` (base `exact`, head alias-unstable), `alias-d3` (base
      `exact`, head `sameline`) or `unchanged`. Written before any run.
- [ ] Raw runs (`nav --no-cache dfg-stats --edges`, bytes on, head with `PRISM_RD_CENSUS_TRACE=1`) of both
      binaries on it: a table row whose dump label differs from its column is removed and listed in
      `TARGET.md` with the reason. Required: at least two mechanisms with at least three moved rows each, and
      unchanged rows. Else go to ORCHESTRATION section 8.
- [ ] `repro/fixture-4b/controls/`: five copies of the head run, each with one trace edit: (c1) a `kill:alias`
      record moved to another edge (the same-line neighbour where the fixture has one); (c2) one record
      deleted; (c3) the tags of a `kill:alias` and an `alias-d3` record exchanged; (c4) a record naming an
      unchanged edge added; (c5) one record's start byte shifted by one. Each control's expected failure
      reason goes in the prediction.
- [ ] `repro/TARGET.md` and `repro/PREDICTION.md` ((a) "identical to the recorded files"; (b) per row and per
      class as `truth.jsonl`, and the five failures) before any kit run.

### Task M-4: the consuming lane (G-adopt)

- [ ] The owner named the consuming lanes on 2026-11-24: lane 10 adopts the comparator and lane 9 adopts the
      oracle, and lane 11 starts first. Before stage D, the orchestrator agrees with the orchestrators of lanes
      10 and 9 which differential each measures with the kit (lane 10's plan already prefers the kit's `run`
      and population when merged). `LEDGER.md` records, per lane, that differential (its plan task), the
      inputs and classes it needs, its existing comparator (for the baseline parity replay) and who hands the
      kit over. Their classes and inputs go to stage D. If neither lane has taken a decision on kit output by
      the close, FORMATS.md stays provisional and G-adopt is reported not met, as the close-out says.

**Exit gate:** INVENTORY.md covers every file listed; TARGET.md names commits, binaries, numbers and the
labelled pair's rows; PREDICTION.md exists; M-4's outcome is in LEDGER.md.

## Stage T: executed truth matrix (2 days)

Output: a test-only PR (`test/measure-kit-matrix`) committing `scripts/measure/matrix/rows.jsonl` and
`scripts/measure/matrix/cases/<id>/` (raw inputs: dump pairs, receipts, trace lines, oracle source and rows,
review outputs, benchmark case stubs with synthetic package trees), and `scripts/measure/tests/
test_matrix_rows.py` (every row well formed: `id`, `group`, `input`, `required`, `truth`, `source`; every
case directory referenced). Each row states the required behaviour and its truth, never a rule.

| Group | Enabled by | Seed rows (review probes, `~/prism-evidence/lanes-2026-11-20/reviews/astra-r1/lane11/probes/`) |
|---|---|---|
| `run` | S1 | ignored file under the root refused (P5); input changed during the run |
| `comparator` | S2 | 1 Exact → 2 NameOnly (P4); `kill_line` 13 → 14 and role change (P4); permuted key and gzip header (P6); same-line label swap (P6); mixed and partial bytes (P2); `["a.b"]` vs `["a","b"]`; unpairable residual |
| `join` | S2 | trace naming byte 154 of two reads (P6); stray with zero changes; three alias records for one edge (P1 stderr); counter delta not predicted by its mechanism |
| `oracle` | S3 | `identity(actual)` actual → formal, CORRECT (P1 row 3); def → use across an unconditional overwrite (binding CORRECT, kill not graded); `sameA`/`sameB` swapped, WRONG (P3); nested `inner` in `outer` (P3); retargeted site between runs |
| `review` | S4 | lost `related_lines` entry; added rule id; same-line distinct alerts; `findings` and `all_findings` both present |
| `gate` | S5 | prediction replaced with its old modification time; receipt without digest |
| `bench-findings` | S6a | two sinks on the ground-truth line; one match among 20 other findings; channel error |
| `bench-negatives` | S6b | sink moved to a split-out helper; unrelated reach inside the function; literal-only change; zero verified pairs |

- [ ] `source` is `probe` (raw output copied from the probe directory), `product` (an authored fixture run
      through a prism binary; receipt in evidence, fixture read by the orchestrator before its first run) or
      `synthetic` (expected result settled by the author and checked by a second reader).
- [ ] Each slice's first commit adds `scripts/measure/tests/test_matrix.py` (S1 creates it) or
      `eval/secbench/test_matrix_bench.py` (S6a creates it) with `ENABLED_GROUPS` naming its groups; the test
      runs every enabled row against the kit and compares with `truth`.

**Exit gate:** every row has a truth; the M-2 disagreements each have a row; CI green.

## Stage E: enumeration (1.5 days)

- [ ] Enumerator tiers serially (ORCHESTRATION section 6) on the cumulative rows, axes: key capability ×
      multiplicity × attribute transition; trace record kind × cardinality × arm; relation × resolution
      outcome (slot, callable, shadowing, rest and spread, type position, cross-file, same-line and nested
      declarations); review payload field; benchmark finding occurrence, function mapping and fix outcome per
      vulnerability class. New rows land in the matrix PR's successor (`test/measure-kit-matrix-e`).
- [ ] Candidate census for G-negatives (counting only, from metadata and the reach outcomes of the bound
      observation set `~/prism-evidence/meas/secbench/repair-r1/final/entries.jsonl`, no package code):
      entries with a named fixed version, usable ground truth and a vulnerable-side positive. The orchestrator sets N from it (default: half, rounded down) and writes it
      in the S6b prediction.

**Exit gate:** a full tier round adds no row (ORCHESTRATION section 3).

## Stage D: formats of record (1.5 days)

The design seat (Fable-5.2, high) gets DESIGN.md, INVENTORY.md, TARGET.md, the complete rows file with
truths, the census, the drafts of S1 to S6b, the consuming lane's classes and inputs (M-4), and
`FORMATS-PROVISIONAL.md` (the formats handed to the two adopters on 2026-11-24), whose change-log entries
stage D accepts, changes or refuses. It writes
`FORMATS.md` here, stated against every row: every subcommand's flags; the manifest, receipt, run-directory
layout, `changes.jsonl`, certificate, classifier contract (record kinds, stray policy, `validate`),
allow-list, gate file, report skeleton, population manifest, `review-diff` outputs, verdict file; exit codes;
generic classes, if any; and the accepted losses. FORMATS.md is marked provisional when M-4 secured no
consumer. Gate: both reviewers APPROVE (cap 2, then ORCHESTRATION section 8). The orchestrator puts
FORMATS.md names into each slice section before dispatch.

## Stage I: slices

Order: S1 (every subcommand reads run directories), S2 (G-repro proves the kit; the consuming lane needs
`diff-edges`), S3, S4, S5 (the gate composes S1 to S4), S8. S6a then S6b run beside S3 to S5 in the bench
worktree with a second implementer. The memory guard moves in S1: `run` needs a guard in the repository for
anyone else or a cloud session to run it.

### S1: corpus manifest, `corpora`, `run`, memory guard (3 days; G-hermetic)

**Files:** create `scripts/measure/{__init__,__main__,cli,corpora,run,receipt,inputs}.py`, `corpora.json`,
`README.md`, `tests/{__init__,helpers,test_corpora,test_run,test_syntax,test_matrix}.py`; move the evidence
`memguard.sh` to `scripts/memguard/` with `test_memguard.py`; in `.github/workflows/ci.yml` job
`build-checks`, after "Perf tooling tests", a step "Measurement kit tests (Python)": `python3 -m unittest
discover -s scripts/measure -t scripts -p 'test_*.py'` and `python3 -m unittest discover -s scripts/memguard
-p 'test_*.py'`.

**Guard changes only:** host-quiet script at `$(dirname "$0")/../hostquiet/host_quiet.py`; log at
`${MEMGUARD_LOG:-${XDG_STATE_HOME:-$HOME/.local/state}/prism/memguard-kills.log}`; exits 75, 76, 137 kept.

**Draft manifest** (X, EX, T, D, M, the labelled pair `R4B` at `scripts/measure/fixtures/repro-4b/` (S2
commits it) and each public corpus M-2 found; never `private-corpus`), entry `T`:
`{"name": "TypeScript", "kind": "git", "url": "https://github.com/microsoft/TypeScript", "commit":
"7964e22f2b85f16e520f0e902c7fd7b6f0c15416", "root": "src", "history": false, "local":
["bench-repos/TypeScript"], "manifest_sha256": null, "cloud_asset": "TypeScript-7964e22f-src.tar.gz"}`
under `{"schema": "prism.measure.corpora/1", "corpora": {…}}`.

`local` is relative to each `--corpora-root` or entry of `PRISM_MEASURE_CORPORA` (`os.pathsep` list), and
for an in-repository fixture (kind `tree`, pinned by `manifest_sha256`) to the repository root. Pin
(DESIGN section 5, Binding): a git checkout needs `HEAD == commit` and empty `git status --porcelain
--untracked-files=all --ignored -- <root>`; another tree must match `manifest_sha256` as `manifest_hash` in
`scripts/perf/cloud/fetch_corpora.py` computes it; neither possible is a mismatch ("unverifiable"; so the
`src`-only cloud archive of `T`, with a null hash, is refused until its hash is recorded). `inputs.py`
computes the input digest of DESIGN section 5, Binding.

```
python3 -m measure [--manifest PATH] corpora list | corpora verify [KEY...] [--corpora-root DIR]...
python3 -m measure run --binary BIN --corpus KEY --out DIR [--corpora-root DIR]... [--trace-env NAME=VALUE]...
                       [--no-bytes] [--sites] [--prediction FILE] [--memguard PATH] [--limit-gb 8] [--expect-version HEX]
```

`run`: pin check and `--expect-version` (exit 3 on mismatch, nothing else executed); refuse a non-empty
`--out` (exit 2); input digest and prediction digest; through the guard run `nav --no-cache dfg-stats`,
`nav --no-cache call-stats`, `nav --no-cache dfg-stats --edges` (with `PRISM_EDGES_BYTES=1` and each
`--trace-env`, this command only), and with `--sites` `nav --no-cache call-stats --dump-sites`; input digest
again (a difference: exit 4, receipt marks `input_changed`); guard exit 75 → exit 75; any other non-zero
command → exit 4 with the receipt written. Outputs: `dfg-stats.json`, `call-stats.json`, `edges.jsonl.gz`,
`edges.stderr.gz`, `sites.jsonl.gz`, one `.stderr` per command, `receipt.json`:

`{"schema": "prism.measure.receipt/1", "kit_commit", "start", "end", "status", "binary": {"path", "sha256",
"version"}, "corpus": {"key", "commit", "root", "pin_check": "git-clean|manifest", "manifest_sha256",
"input_sha256"}, "prediction": {"path", "sha256"} | null, "options", "commands": [{"argv", "env", "output",
"output_sha256", "bytes", "exit", "start", "end"}], "guard": {"path", "limit_gb"}, "host": {"platform",
"cpus"}}` (times UTC ISO 8601; `version` is the binary's `--version`).

- [ ] **Failing tests first.** `helpers.py`: a one-commit git repo named `Z` in a temp manifest, a fake prism
      that logs argv and env and prints canned JSON, a pass-through guard, `kit(*args)` running the module.
      `test_matrix.py` with `ENABLED_GROUPS = {"run"}`.

```python
def test_run_writes_dumps_and_receipt(self):
    self.assertEqual(self.run_kit().returncode, 0); r = json.loads((self.out / "receipt.json").read_text())
    self.assertEqual((r["binary"]["sha256"], r["corpus"]["commit"]), (sha256_of(self.fake), self.commit))
    self.assertEqual(r["commands"][2]["env"], {"PRISM_EDGES_BYTES": "1"})
def test_run_refuses_corpus_with_pin_mismatch(self):
    self.commit_another_file(); self.assertEqual(self.run_kit().returncode, 3); self.assertEqual(self.fake_log(), [])
def test_run_refuses_dirty_corpus(self):                 # one untracked file -> 3
def test_run_refuses_ignored_file_under_root(self):      # tracked .gitignore names generated.ts, file present -> 3, fake never ran
def test_input_change_during_run_fails(self):            # fake prism writes a file under the root -> 4, receipt "input_changed"
def test_receipt_binds_prediction_before_first_command(self):  # fake rewrites the prediction on its first call; receipt sha256 = original
def test_trace_env_applies_to_edges_run_only(self):      # [False, False, True] for PRISM_RD_CENSUS_TRACE per command
def test_guard_refusal_exits_75(self):                   # guard stub exits 75 -> 75
def test_corpora_verify_nonzero_on_mismatch(self):       # exit 3, key on stdout
def test_manifest_hash_matches_perf_cloud(self):         # equals fetch_corpora.manifest_hash on a temp tree
def test_every_module_parses_as_python_39(self):
    for f in Path(measure.__file__).parent.rglob("*.py"): ast.parse(f.read_text(), feature_version=(3, 9))
```

`test_memguard.py` (the guard copied into a temp tree with a fake `../hostquiet/host_quiet.py`, a fake `ps`
first on `PATH`): above the limit → 137 and one log line; below → the command's exit; marker
`"stale": false` → 75; failing `ps` → 76.

- [ ] RED: `python3 -m unittest discover -s scripts/measure -t scripts -p 'test_*.py'` fails on import.
      Implement; GREEN; commit `feat(measure): corpus manifest, run and receipts`.
- [ ] `git grep -nE '/Users/|prism-evidence' -- scripts/measure scripts/memguard` exits 1 (no match; exit 2
      is an invocation error, not a pass). Smoke: `run` on EX with `repro/bin/prism-base`; receipt path in
      `LEDGER.md`. PR body: formats as built, totals, smoke.

### S2: `diff-edges`, `diff-stats`, classifiers, G-repro (5 days; G-repro, G-hermetic)

S2 also implements the `node` record kind that `FORMATS-PROVISIONAL.md` adds for lane 10, in the form
stage D settles.

**Files:** create `scripts/measure/{edges,diff_edges,diff_stats,classifier,trace}.py`,
`examples/classify_783_4b.py`, `fixtures/repro-4b/` (M-3's fixture corpus, truth and control generator),
`tests/{test_edges,test_diff_edges,test_diff_stats,test_examples}.py`; enable `comparator` and `join` in
`test_matrix.py`.

```
python3 -m measure diff-edges BASE_RUN HEAD_RUN --classifier FILE [--key bytes|line] [--out DIR] [--list]
python3 -m measure diff-stats BASE_RUN HEAD_RUN [--allow FILE] [--which dfg|call|both]
```

Keys, attributes, residual pairing and the key-mode refusal are DESIGN section 5, Edge identity; record
matching, strays and `validate` are Trace join; run-pair admission is Binding (exit 2 on any refusal). The
default key mode is bytes when both dumps carry bytes, line when neither does; `--key line` forces line mode.
Stdout: first the recorded comparators' line `edges base=N head=N added=A removed=R label-moved keys=M`
(G-repro (a) compares it verbatim), then `occurrences: added=… removed=… label_moved=… ambiguous=…`,
`collisions keys=K` (line mode), the class table (keys and occurrences), `trace records:` per tag (attached,
stray, ambiguous, unmatched), the legacy `trace mechanism lines:` (per tag, the distinct join keys carrying it,
as `compare4b.py` counts), then `PASS` or `FAIL` with each failure. A `changes.jsonl` row:
`{"kind": "added|removed|label_moved|ambiguous", "count", "key": {"from": {"file", "base", "fields", "line",
"start_byte", "end_byte"}, "to": {…}}, "base_bag": [{"from_access", "to_access", "confidence", "doubt",
"kill_line"}], "head_bag": […], "records", "class", "error"}`.

`certificate.json`: `{"key_mode", "changed_keys", "occurrences", "attributed", "unattributed", "ambiguous",
"by_class": {"<class>": {"keys", "occurrences"}}, "records": {"attached", "stray", "ambiguous", "unmatched"},
"validation", "inputs"}`; exit 1 iff `unattributed > 0` or `validation` is non-empty. Classifier file:
`CLASSES = [...]`; `classify(change, context) -> Optional[str]`; optional `parse_trace(line, arm) -> record |
None`, `STRAYS = "fail" | "allow"` (default `fail`), `validate(bound) -> list[str]`. `context` has at least
`records(change)`, `corpus_root`, both run directories, both receipts and the counter deltas. Allow-list: one
dotted counter per line, `#` comments, trailing `.*` = prefix; `diff-stats` prints `  key: b -> h (+d)` per
delta, exit 1 on a delta not allowed.

- [ ] **Failing tests first** (`edge(...)` builds one line in the `nav dfg-stats --edges` shape; matrix rows
      enabled):

```python
def test_identical_dumps_pass_with_zero_changes(self):       # exit 0, certificate changed_keys == 0
def test_one_exact_to_two_nameonly_is_one_move_and_one_addition(self):
    self.diff([E1], [N1, N1]); self.assertEqual(sorted((c["kind"], c["count"]) for c in self.changes()), [("added", 1), ("label_moved", 1)])
def test_unpairable_residual_is_ambiguous_and_unattributed(self):   # {exact, killed} -> {sameline, alias_unstable}: "ambiguous", exit 1
def test_changes_rebuild_both_input_multisets(self):          # 200 seeded random pairs: unchanged + bags == each side
def test_kill_line_and_role_changes_are_attribute_moves(self):  # kill_line 13 -> 14 and access use -> def: label_moved, both bags kept
def test_permuted_rows_and_gzip_header_are_no_change(self):  # exit 0, zero changes
def test_byte_only_difference_is_a_change_in_byte_mode_and_not_in_line_mode(self):
    a, b = edge(line=6, sb=154, eb=155, label=EXACT), edge(line=6, sb=158, eb=159, label=NAMEONLY)
    self.diff([a, b], [swap_labels(a, b)], key="bytes"); self.assertEqual(len(self.changes()), 2)
    self.diff([a, b], [swap_labels(a, b)], key="line"); self.assertEqual((self.changes(), self.cert()["collisions"]), ([], 1))
def test_mixed_capability_refuses_byte_mode(self):            # bytes vs none -> exit 2; --key line -> 0 changes
def test_partial_bounds_refuse_byte_mode(self):              # one endpoint without end_byte -> exit 2
def test_path_fields_stay_structured(self):                  # fields ["a.b"] and ["a", "b"] are two keys
def test_trace_record_attaches_only_to_a_unique_changed_edge(self):  # byte mode: attached to 154; line mode: ambiguous, change unattributed
def test_stray_fails_by_default_even_with_zero_changes(self):  # exit 1; STRAYS = "allow" -> exit 0
def test_records_for_one_edge_count_one_change(self):        # cert record unjoined, two edge records on one change, changed_keys 1
def test_validate_failure_fails_the_gate(self):              # validate -> ["counter x +20, predicted +1"]: exit 1, failure printed
def test_classifier_exception_counts_as_unattributed(self):
    self.assertEqual(self.diff([E1], [], classifier=RAISING).returncode, 1)
    self.assertEqual(self.cert()["unattributed"], 1); self.assertIn("ZeroDivisionError", self.changes()[0]["error"])
def test_classifier_returning_undeclared_class_is_unattributed(self):  # exit 1, error names the class
def test_runs_with_different_input_digest_are_refused(self):  # exit 2, nothing compared
def test_diff_edges_streams_large_gzip_within_memory_bound(self):   # 100,000 distinct keys per side, tracemalloc peak < 96 MiB; truncated gzip -> exit 2
def test_diff_stats_lists_every_nested_delta(self):        # "dfg_labels.x: 1 -> 2 (+1)"
def test_diff_stats_exit_1_on_move_outside_allow_list(self):          # "x.*" allows x.a, not y
def test_783_classifier_attributes_kill_alias_and_alias_d3_and_refuses_untagged_move(self):   # classes / None
```

- [ ] RED, implement, GREEN, commit. Port `classify_783_4b.py` from `compare4b.py` (allowed: `exact` → one
      `nameonly` label whose doubt maps to tag `kill:alias` or `alias-d3`; `rd-alias-cert` records unjoined;
      strays allowed for part (a)'s mechanism counts, forbidden for part (b)).
- [ ] **G-repro** (measurement runner; M-3's `repro/PREDICTION.md` already written; DESIGN section 6):
      (a) `run` X, EX, T with both M-3 binaries (head with `--trace-env PRISM_RD_CENSUS_TRACE=1`), then
      `diff-edges --classifier examples/classify_783_4b.py` in `--key line` and in byte mode, and
      `diff-stats`. Pass: the first line equals the recorded one byte for byte, `PASS` in both modes, every
      trace tag count equals the recorded one, no counter delta. (b) `run` `R4B` with both binaries, then
      `diff-edges` in byte mode: every row and class equal `truth.jsonl`, `PASS`; then each control of
      `controls/` gives `FAIL` naming its predicted reason.
- [ ] PR body: recorded-versus-kit table per corpus for (a); truth-versus-kit per row for (b) and the five
      control results; formats as built.

### S3: TypeScript oracle and self-check (4 days; G-selfcheck, G-hermetic)

S3 also implements `ctx.enclosing_call` and `program_with_default_lib()`, which `FORMATS-PROVISIONAL.md` adds
for lane 9, in the form stage D settles.

**Files:** create `scripts/measure/oracle/{ts_oracle,bytes,selfcheck}.cjs`, `oracle/questions/{argument_binding,
lexical_binding,calls}.cjs`, `oracle/selfcheck/<case>/{repo/,input.jsonl,expected.jsonl}`,
`oracle/realuse/{repo/,truth.jsonl}`, `scripts/measure/oracle.py`, `tests/test_oracle.py`; enable `oracle`
in `test_matrix.py` (each `oracle` row must exist as a self-check case with its truth).

```
python3 -m measure oracle ts-calls (--base RUN --head RUN | --census RUN) --out DIR [--typescript PATH] [--question FILE] [--aggregate-only]
python3 -m measure oracle ts-edges (--changes CHANGES.jsonl | --all) --run RUN --out DIR [--typescript PATH] [--question FILE]
```

TypeScript: `--typescript`, else `PRISM_TYPESCRIPT`, else the `acquire.mjs` install root (as its README
resolves it) + `/typescript/7e02162c902e5c29ec19fefc573ad55b8ed15fcbda940e737f297c53e9c41f54/lib/typescript.js`;
`oracle.py` refuses (exit 3, Node never started) unless its SHA-256 is
`3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675`, and refuses grading (exit 3) unless a
passing self-check receipt exists for the current digest of checker tree, compiler options, question and
fixtures (`selfcheck.cjs` writes it). The oracle reads source only, from the run's corpus root after checking the receipt's input
digest; offsets are UTF-8 bytes as prism reports them (port `adjudicate.cjs`'s map). Questions, verdicts and
call-target deltas: DESIGN section 5, Oracle questions. Output `verdicts.jsonl` (`key`, `relation`,
`property`, `verdict`, `why`, the original row), `deltas.jsonl` (ts-calls), `tuples.json` (CORRECT, WRONG,
UNDECIDED, INADMISSIBLE per question, zeros included); `--aggregate-only` writes `tuples.json` only. A
`--question` module exports `grade(row, ctx) -> {verdict, why, property}`.

- [ ] Author self-check cases first: per question one case per verdict, plus the near-miss negatives of
      G-selfcheck and every `oracle` matrix row. `selfcheck.cjs` exits 0 only when every verdict equals
      `expected.jsonl`.
- [ ] Failing Python tests (fake `node` on `PATH`): `test_oracle_refuses_wrong_typescript_hash` (exit 3, node
      log empty), `test_oracle_refuses_without_selfcheck_receipt_for_current_digest` (a fixture edited after
      the receipt → exit 3), `test_oracle_tuples_include_zero_counts` (`{"CORRECT": 2, "WRONG": 0,
      "UNDECIDED": 1, "INADMISSIBLE": 0}`), `test_oracle_rejects_unknown_verdict_from_question_module` (exit
      1), `test_ts_calls_reports_a_retargeted_site_between_runs` (base site → A, head → B: one `retargeted`
      delta graded; `--census HEAD` reports no delta), `test_ts_edges_source_digest_mismatch_is_inadmissible`.
- [ ] RED, implement, GREEN, commit. **G-selfcheck:** `scripts/memguard/memguard.sh 8 node
      scripts/measure/oracle/selfcheck.cjs --typescript <installed path>` exits 0; output in the PR. One real
      use: `run` `oracle/realuse/repo` (authored argument- and lexical-binding shapes, truth written before
      the run) with `repro/bin/prism-head`, then `ts-edges --all`: every graded verdict equals `truth.jsonl`,
      at least ten CORRECT, no INADMISSIBLE row; report the tuple.

### S4: `review-diff` and the diff population (2.5 days; G-hermetic)

**Files:** create `scripts/measure/{population,review_diff}.py`, `populations/602-m2-120.json`,
`tests/{test_population,test_review_diff}.py`; enable `review` in `test_matrix.py`.

Draft: `{"schema": "prism.measure.population/1", "name": "602-m2-120", "repos": {"apache-answer": {"url":
"https://github.com/apache/answer", "selection_head": "6744295f…"}}, "spec_rule": {"git_diff": ["-U0",
"-M"], "extensions": […], "exclude_test": "<regex>", "modify_type": "Modified"}, "diffs": [{"ordinal": 1,
"repo": "apache-answer", "commit": "ace02c49…", "parent": "69fc3c96…", "tree": "dcb4b4c8…", "spec_sha256":
"4e9d7e7d…", "n_files": 1, "n_lines": 5}]}`, built from M-2's `population.jsonl`: ids and hashes, no content. `extensions` and `exclude_test` are
`SRC_EXT` and `is_test` of `jsts-lane/602/m2/scripts/select_rules.py` (evidence). Spec rule (as `m2run.py`):
added lines of `git diff -U0 -M parent commit`; keep a file when its extension is listed, it is not a test
and it exists at `commit`; files and lines sorted, lines unique; `json.dumps(spec)` default separators.
Repositories: apache/answer, apache/superset, calcom/cal.com, directus/directus, miniflux/v2, saleor/saleor.

```
python3 -m measure review-diff --population FILE --clones DIR --arm NAME=BIN[::ARG...] --arm NAME=BIN[::ARG...]
    --out DIR [--review-args "--algorithm taint --resolution scoped"] [--only N,M] [--sample 30 --seed 20261101]
```

Per diff: `git clone --shared --no-checkout` of the clone into scratch, `checkout --detach commit`, verify
the tree id, regenerate the spec and refuse the row on a SHA-256 mismatch; each arm runs with `--format json
--no-cache` through the guard; comparison by DESIGN section 5, Review comparison. Outputs `per-diff.jsonl`
(one accounting row per manifest ordinal: presence and payload changes, refused or failed arms),
`totals.json` (`--only` marks the totals as a subset, not the population), `sample.jsonl` (candidates sorted,
then seeded).

- [ ] Failing tests (temp repo with parent and commit): `test_spec_matches_m2_rule` (test file, `.md` file
      and deleted file excluded; equals `{"files": [{"file_path": "src/a.ts", "modify_type": "Modified",
      "diff_lines": [3, 4]}]}`), `test_spec_sha_mismatch_refuses_row`, `test_source_clone_is_untouched` (HEAD
      and `status --porcelain` equal before and after), `test_identical_payloads_cancel_with_multiplicity_and_
      order`, `test_lost_related_line_is_a_payload_change` (zero presence changes, one payload change naming
      `related_lines`), `test_added_rule_id_is_a_rule_transition_not_a_presence_change`,
      `test_findings_read_once_per_arm` (output with both `findings` and `all_findings`: each finding counted
      once), `test_failed_arm_fails_its_row_and_totals_name_it`, `test_sample_is_deterministic_for_a_seed`,
      `test_population_manifest_has_120_rows_and_no_content` (120 diffs; no `files` or `diff_lines` key).
- [ ] RED, implement, GREEN, commit. Check (not a design gate): regenerate all 120 specs from the six local
      full clones; every SHA-256 equals the manifest's; count in the PR.

### S5: `gate` and the report skeleton (2 days; G-hermetic)

**Files:** create `scripts/measure/{gate,report}.py`, `examples/gate_783_4b.json`,
`tests/{test_gate,test_report}.py`; enable `gate` in `test_matrix.py`.

```json
{"schema": "prism.measure.gate/1", "lane": "783-4b", "prediction": "PREDICTION.md",
 "binaries": {"base": {"path": "…", "expect_version": "4f9e3a3e"}, "head": {"path": "…", "expect_version": "1f2ad1cc"}},
 "steps": [{"id": "X-base", "do": "run", "binary": "base", "corpus": "X"},
  {"id": "X-head", "do": "run", "binary": "head", "corpus": "X", "trace_env": ["PRISM_RD_CENSUS_TRACE=1"]},
  {"id": "X-edges", "do": "diff-edges", "base": "X-base", "head": "X-head", "classifier": "c.py", "expect": {"unattributed": 0}}]}
```

`gate FILE --out DIR [--binary NAME=PATH]... [--corpora-root DIR]... [--retrospective]` (the overrides keep
machine paths out of committed gate files) hashes the prediction before the first step and passes it to
every `run` (DESIGN section 5, Binding); exits 2 when the prediction is missing, changes during the gate, or
a reused run's receipt lacks that digest; `--retrospective` replays legacy runs and the report's Result says
"retrospective", never a pass. It runs steps in order; writes `gate.json` (per step: pass or fail, numbers)
and `REPORT.md` with sections Receipt, Prediction (verbatim, with its SHA-256), Population and denominators,
Classes and certificate, Verdicts (C/W/U/I with zeros), Controls and Limits (for the lane to fill), Result.

- [ ] Failing tests: `test_gate_refuses_missing_prediction`; `test_gate_refuses_reused_run_without_matching_
      prediction_digest` (a receipt without a digest → 2; a receipt whose digest differs while the
      prediction keeps its old modification time → 2); `test_gate_refuses_prediction_changed_during_the_gate`;
      `test_retrospective_replay_is_never_a_pass`; `test_gate_fails_on_threshold_and_names_the_step`;
      `test_gate_rejects_unknown_step_kind`; `test_report_has_the_eight_sections_in_order`;
      `test_report_quotes_prediction_and_its_sha256`.
- [ ] RED, implement, GREEN, commit. Run `examples/gate_783_4b.json` once (paths supplied at run time); its
      numbers equal S2's part (a).

### S6a: benchmark finding-level accounting (2 days; G-findings)

**Files:** create `eval/secbench/{findings,test_findings,test_matrix_bench}.py`; modify `run.py` (keep the
parsed taint channel output instead of discarding it; add `row['finding']`), `replay.py` (score from retained
`taint.json.gz`), `report.py` (case-level line-hit table and finding-level counts beside reach), `README.md`.

`finding_outcome(row, taint_output, invocation) -> {"line_hit", "findings", "rules"}` (DESIGN section 5,
Finding-level accounting): `line_hit` is `sink_line`, `ambiguous` (the ground-truth line holds another sink
occurrence or two distinct `taint_sink` findings), `sink_file`, `elsewhere`, `none`, `error` (channel failed
or timed out) or `not_scored` (no ground truth); `findings` has one row per emitted `taint_sink` finding,
`{"file", "line", "match": "matched|ambiguous|off_target"}`, with at most one `matched`; `rules` from matching
findings' `rules` field (lane 7), else `[]`. The report gives separate denominators and calls the case figure
a line-hit rate. Reach outcomes are neither read nor changed.

- [ ] Failing tests (`bench-findings` rows enabled): `test_sink_line_finding`, `test_other_line_same_file`,
      `test_finding_elsewhere`, `test_no_taint_sink_finding` (a `taint_source` finding does not count),
      `test_channel_error`, `test_two_sinks_on_the_gt_line_is_ambiguous`,
      `test_extra_findings_are_off_target_rows_not_another_case_outcome`,
      `test_one_match_among_duplicate_findings`, `test_rules_copied_when_present`,
      `test_replay_and_live_scoring_agree` (on every `bench-findings` row),
      `test_reach_summary_unchanged_by_finding_fields` (`summarize()` equal with and without `row['finding']`).
- [ ] RED: `cd eval && uv run --locked pytest secbench/test_findings.py secbench/test_matrix_bench.py -q`.
      Implement, GREEN, commit.
- [ ] **G-findings** by replay (rescore, no prism run): raw tree `~/prism-evidence/meas/secbench/repair-r1/
      measured` (SUT `4e592daa`, binary `d603e7cd…`, 373 taint outputs). Confirm it is the observation set
      `repair-r1/final/binding.json` names and take the population from that binding (not from the research
      count); replay; require every reach outcome equal to `repair-r1/final/entries.jsonl`, a finding-level
      row on every case with ground truth, and at least one `sink_line`. PR: table per class, channel errors
      counted.

### S6b: benchmark negatives (4.5 days; G-negatives)

**Files:** create `eval/secbench/{fixed.py,test_fixed.py,locate.mjs,locate.test.mjs,fixed-pins.json}`;
modify `report.py`, `README.md`; enable `bench-negatives` in `test_matrix_bench.py`.

- `fixed.py acquire --inputs META --dest DIR`: `fixedVersion`, stripped of spaces and one leading `=`, must
  match `v?\d+\.\d+\.\d+(-[\w.]+)?` and be newer than the vulnerable version (SemVer order). Read
  `https://registry.npmjs.org/<name>/<version>` (urllib; scoped names encoded); the metadata's `name` and
  `version` must equal the request; metadata and `dist.tarball` only over HTTPS from `registry.npmjs.org`,
  redirects included; `dist.integrity` must be `sha512-` and match; an existing pin must match on
  reacquisition; at most 50 MB compressed, 200 MB expanded and 20,000 members; only regular files and
  directories under `package/`, refusing absolute paths, `..`, links, devices and duplicate or colliding
  members; extract into a fresh staging directory and rename it to `<dest>/<class>/<entry>/src/package` only
  when complete. Pin name, version, SHA-256 and integrity in `fixed-pins.json`. Never install, import or run
  anything, and no package configuration (`tsconfig`, plugins) is read when scoring.
- `locate.mjs` (pinned TypeScript, kit-chosen compiler options): the innermost named function at a file and
  line; a package's named function declarations with spans; a function's token sequence (`ts.createScanner`,
  comments and whitespace dropped, literal text kept); the call occurrences in a span with callee text and
  bytes.
- `fixed.py score`: the verified-pair rule of DESIGN section 5, Negatives. The positive control comes from
  the bound vulnerable-side observations (reach in `repair-r1/final/entries.jsonl`, findings from S6a's
replay). The source function is mapped likewise and seeded at the same
  parameter positions as the ground truth; then the `taint-reaches` frontier from the fixed source and the
  review taint channel. A verified pair gets three columns: `finding` (a `taint_sink` finding on the mapped
  occurrence's line), `reach` (a reached occurrence equal to the mapped one) and `function_change`
  (`unchanged` or `changed`); a verified removal gets `sink_removed`.
- One reason per unscored entry: `no_fixed_version`, `fix_commit_only`, `fixed_version_unparseable`,
  `fixed_not_newer`, `registry_missing`, `metadata_mismatch`, `origin_refused`, `integrity_unsupported`,
  `integrity_mismatch`, `pin_drift`, `limit_exceeded`, `extract_refused`, `gt_unavailable`,
  `no_vulnerable_positive`, `sink_function_unnamed`, `function_not_found`, `function_moved`,
  `function_ambiguous`, `sink_occurrence_unmapped`, `sink_occurrence_ambiguous`, `source_not_found`,
  `prism_error`.

- [ ] Failing tests: `test_fixed_version_parsing` (`" =1.20.0"` → `"1.20.0"`; `"n/a"` and `">=2.1.1 <3.0.1"`
      → `None`), `test_extraction_refuses_traversal_links_and_collisions`, `test_member_and_size_limits_are_
      reasons`, `test_failed_extraction_publishes_nothing`, `test_redirect_to_another_origin_is_refused`,
      `test_integrity_mismatch_is_a_reason_not_a_crash` (injected fetcher, no socket), `test_pin_drift_is_a_
      reason`, `test_locate_none_one_many`, `test_sink_moved_to_a_helper_is_a_reason_not_a_score`,
      `test_unrelated_reach_in_the_function_is_not_reach`, `test_literal_change_is_a_function_change`,
      `test_every_attempted_entry_has_outcome_or_reason`, `test_zero_verified_pairs_reports_unmet`.
- [ ] RED (`uv run --locked pytest secbench/test_fixed.py secbench/test_matrix_bench.py -q`), implement,
      GREEN, commit.
- [ ] **G-negatives** (owner decision 2: archives only, into `~/prism-evidence/inputs/secbench-fixed/`):
      acquire every entry with a named `fixedVersion` (278 by the research count), score through the guard with
      the baseline binary; every entry has an outcome or its reason, and at least N verified pairs are scored
      (N from stage E, in the prediction); fewer is reported as unmet. Report each column of `still_reported`
      within the verified subset as an upper bound on false alarms there, not a rate.

### S7: removed by the design review (2026-11-21)

The cloud launchers are a follow-up issue (DESIGN section 7, decision 3; close-out).

### S8: documentation and the manual (1 day)

- [ ] `scripts/measure/README.md`: every subcommand, FORMATS.md folded in (marked provisional if M-4 secured
      no consumer), writing a classifier with `examples/classify_783_4b.py` as the worked example, the
      matrix and its groups, the self-check rule, running in a cloud session. `CLAUDE.md`: a short
      "Measurement kit" paragraph under Build & Test with the unittest command.
- [ ] `ORCHESTRATION.md`: section 4 guard path → `scripts/memguard/memguard.sh`; section 5 item 6 and
      section 6's measurement brief → "a gate file, a prediction and a classifier for `python3 -m measure
      gate`; no comparator, run recipe, counter diff or trace join"; one sentence that this overrides a lane
      plan's own measurement-tooling task for any lane whose stage M starts after this PR merges.

## Close-out (1 day, plus the wait for G-adopt)

- [ ] **G-adopt** (DESIGN section 6): at S2's merge the orchestrator hands M-4's consuming lane the commands,
      the classifier contract and the amendment to its plan (its own comparator, recipe, counter diff and
      trace-join task replaced by a classifier file, gate files and predictions). Allowed files: one
      classifier, at most one oracle question module, gate files, predictions, and census scripts asking a
      question the kit does not answer. A lane migrating after its stage M first replays its baseline with the
      kit and shows parity with its existing comparator. Any other comparator, recipe, counter or trace-join
      script fails G-adopt. With no consumer, or none that has taken a decision on kit output by the close,
      G-adopt is reported not met and FORMATS.md stays provisional; the issue names what blocked adoption.
- [ ] Issues (filed, not started), each with the decision it would support: rule-aware finding scoring after
      lane 7 (benchmark rule columns); the cloud launchers (a lane that needs a kit gate in a cloud session);
      `review-diff` as a cross-check for PR #841 (only if #841's merge needs an independent count; same format
      and flags, so a SARIF reader); one corpus registry with `scripts/perf/cloud/corpora.json`;
      `eval/secbench/run.py`'s committed machine path and compiler default; each G-adopt blocker. Then
      `HANDOFF.md`, `LEDGER.md`, offload, remove both worktrees.

## Acceptance for the lane

G-repro (a) and (b), G-selfcheck, G-hermetic, G-findings and G-negatives hold on merged code, or a gate that
does not is reported not met with its numbers (G-negatives below N); G-adopt holds, or is reported not met
with FORMATS.md provisional (a filed gap never counts as adoption); ORCHESTRATION points lanes at the kit;
each slice approved by its seats with CI green.

## Day estimates

| M | T | E | D | S1 | S2 | S3 | S4 | S5 | S6a | S6b | S8 | Close-out | Total |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 2.5 | 2 | 1.5 | 1.5 | 3 | 5 | 4 | 2.5 | 2 | 2 | 4.5 | 1 | 1 | **32.5** (about 26 calendar days with S6a and S6b beside S3 to S5) |
