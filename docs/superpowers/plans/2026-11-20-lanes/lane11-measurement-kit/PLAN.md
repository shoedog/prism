# Lane 11: a shared measurement kit — implementation plan

> **PROVISIONAL (owner decision, 2026-11-25).** These formats are not a design of record. Six review rounds and four enumeration passes found open-class defects in them that were not folded; the findings are test rows in [FORMATS-OPEN-ROWS.md](FORMATS-OPEN-ROWS.md), each with an owning slice. Each slice freezes the formats it implements, with executable tests, when it lands, and must close or explicitly re-scope every open row it owns. No other lane may depend on a provisional format: lanes 9 and 10 use their own tools until the slice that implements the format they need has landed.

> **For agentic workers:** run by an orchestrator under `../ORCHESTRATION.md`. Each task goes to an implementer
> (Sonnet-5.5, high) with its section and `FORMATS.md` as the brief for S1–S5 and S6a–S6b.
> Use the slice reading map in FORMATS. Section 8 is in [FORMATS-DECISIONS.md](FORMATS-DECISIONS.md) and section 9 in [FORMATS-LEDGER.md](FORMATS-LEDGER.md). Read DESIGN §§5–6 with the [replacement gate and architecture text](FORMATS.md#changes-to-design-sections-5-and-6). In test listings, a `def` line ending in a comment is a test to write; the comment
> is its assertion.

**Goal:** one tested measurement kit in the repository, so a lane writes only a classifier file (and, when
needed, an oracle question); plus finding-level accounting and verified negatives for the security benchmark.

**Architecture:** a standard-library Python package `scripts/measure/` run as `python3 -m measure` from
`scripts/`; a Node oracle in `scripts/measure/oracle/` on the pinned TypeScript 5.9.3; benchmark work in
`eval/secbench/`; the memory guard moved to `scripts/memguard/`. The cloud launchers are not moved (DESIGN
section 7, decision 3).

**Tech stack:** Python 3.9+ standard library, `unittest`; Node with the TypeScript tree that
`scripts/gate-inputs/acquire.mjs` installs; the eval project (uv, pytest). **Spec:** `DESIGN.md` here; its
section 5 supplies the architecture. `FORMATS.md` supplies the commands, files, row decisions and tests of record.

## Global constraints

- Worktrees (ORCHESTRATION section 4): the kit worktree (sparse) for M to S5 and S8;
  the benchmark worktree (no sparse-checkout: it reads and edits `eval/secbench/`) for the M-2
  harness reads, the stage-T benchmark rows, S6a and S6b, with its own branches. The two streams merge in
  their own order (S6a before S6b: both touch `report.py` and the harness README); host runs of both go one
  at a time through the guard. Evidence in the lane evidence directory (`LEDGER.md`, `HANDOFF.md`,
  `inventory/`, `repro/`, `secbench/`).
- `scripts/measure/`, `scripts/memguard/`: standard library, Python 3.9 syntax (`from __future__ import
  annotations`, no `match`). Unit tests use no network, prism binary, corpus or TypeScript; fakes are small
  scripts written to a temporary directory.
- Committed files: no private corpus name, no absolute machine path, no raw corpus dump (the lane-written
  fixture corpora and matrix cases are authored public inputs). Private-name check of ORCHESTRATION section 9
  before every commit; briefs name the private evidence directories as not to be opened and say that
  sentence is not to be copied into repository files.
- Every prism, `cargo` or TypeScript-checker run goes through the guard (the evidence guard with limit 8 GiB, from S1 on `scripts/memguard/memguard.sh 8`), one at a time; on a structured host-quiet refusal run `python3
  scripts/hostquiet/host_quiet.py wait` and retry. Counting runs take no marker.
- Cloud (the kit must run there unchanged): 4 vCPUs, 15 GB, no swap, ~30 GB disk, no system time utility,
  REST-only GitHub, silent push failure for any file over 100 MB, nothing private.
- Per-PR gates (ORCHESTRATION section 5): 3, 4, 8, 10 (kit README in the same PR) and 11 apply; 1 runs in CI.
  Item 2 is the slice's unit-test command. Items 5, 6, 7, 9 do not apply: no Rust change.
- Every PR body: what changed, formats as built, test totals (local and CI), the matrix groups enabled, the
  slice gate's prediction and result, what was not verified, the review record. Branches
  `feat/measure-kit-<slice>`.
- Review seats (ordinary design cap 2 rounds, spent; §8 escalation spent, owner extension one targeted fold and paired re-review; slice cap 3 rounds):

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
the evidence root, else its versioned evidence checkout; restore nothing from release assets).

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
| Expected | edges 80,033 / 7,445 / 388,055 (excalidraw / express / TypeScript `src`); primary pair has 0 added, removed and label-moved, with identical DFG and call counters. Measure the per-mechanism trace line for each corpus under the ordered protocol below |

- [ ] Build both commits (`cargo build --release`, guarded) in detached scratch worktrees under the evidence
      directory; keep `repro/bin/prism-{base,head}`; delete the scratch `target/`s. Check that
      `PRISM_EDGES_BYTES=1` gives both bounds on every endpoint of both binaries.
- [ ] Follow the ordered protocol fixed in `repro/TARGET.md` before any kit measurement. Use base `4f9e3a3e` and primary `1f2ad1cc` for edge and counter identity. For historical trace counts try the primary first. For each corpus with a count mismatch try `32ad7c09b` (head at the recorded minute), then `3510ed4f2` on a further mismatch. Stop at the first matching counts and retain every prior result. If all miss, mark trace reproduction unmet. Report each corpus, actual head, executable digest, observed numbers and the claim earned. A fallback can earn trace-count credit only. If different corpora match on different heads, report the matches separately. The toolchain fallback below is separate.
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

- [x] The owner named lane 10 for S2 and lane 9 for S3. Lane 10 was re-scoped to recall only and uses its own comparator and trace. For an adopter of the inventory profile, the comparison plan is S0 against baseline, then S1 and S2 against their respective predecessors on the seven git-clean pins in FORMATS section 2, with a classifier implementing cross-key, spelling-leak and orphan-anchor checks over edges and traced nodes. Lane 10's [S0 and rollout amendment](FORMATS.md#amendment-text-for-lane-10s-plan) is superseded by the 2026-11-25 erratum; optional for an adopter of the inventory profile; nothing in this task requires it. Lane 9 grades argument additions and spread removals on X, T and benchmark package runs, with gin as the base control (lane 9's G, registry key `GIN`), using its checked multi-relation module. The lane 11 orchestrator hands each interface to its consuming orchestrator at the slice merge. Baseline parity and the actual adopter decision remain G-adopt requirements.

**Exit gate:** INVENTORY.md covers every file listed; TARGET.md names commits, binaries, numbers and the
labelled pair's rows; PREDICTION.md exists; M-4's outcome is in LEDGER.md.

## Stage T: executed truth matrix (2 days)

The kit matrix lives in `scripts/measure/matrix/rows.jsonl` and `cases/<id>/`. The benchmark worktree supplies `scripts/measure/matrix/rows-bench.jsonl`, its own README and the `bf-` and `bn-` case directories. Its rows use repository-relative case paths. In the Stage D snapshot, the same directories are siblings of `README-bench.md` under `cases/`. The executing tests belong in `eval/secbench/`. `scripts/measure/tests/test_matrix_rows.py` checks the kit row schema and referenced directories. Each row specifies `id,group,input,required,truth,source`. FORMATS section 8 settles the fields marked `undetermined` in the inputs and the disputed readings.

| Group | Enabled by | Seed rows (review probes, `<evidence>/lanes-2026-11-20/reviews/astra-r1/lane11/probes/`) |
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
      runs every enabled row against its slice and checks the fields of `truth` with the section 8 decisions. Add each of the 108 selected enumeration IDs, 43 round-one, 20 round-two, nine round-three and six round-four fixture IDs in section 9 to its corresponding matrix before closing that slice.

**Exit gate:** every row has a truth; the M-2 disagreements each have a row; CI green.

## Stage E: enumeration (1.5 days)

- [x] Enumeration stopped as an open class after five terra rounds, two sol rounds and one astra round. A dry round was never observed. The eight files contain 300 distinct suggestions and 33 dispute mentions. Section 8 adjudicates the 33 dispute mentions. In section 9 the matrix successor takes 108 of the original IDs, including six promoted from Record, and records the remaining 192 under their rules. The sixth promotion preserves the original composite actual in `terra5-oracle-003`. The ledger adds 43 round-one, 20 round-two, nine round-three and six round-four fixture IDs. Apply the round-three and round-four changes against their stated inputs and all 24 earlier section 8 truth replacements in the matrix-successor job. Keep the two matrix streams separate.
- [x] The bound census found 57 traced positive candidates among 600 entries, 277 with parseable fixed versions and 147 with usable ground truth. Set the premeasurement G-negatives minimum to 28. S6a may add finding-only candidates.

**Exit disposition.** The original dry-round gate was unmet. Stage D supplies the proof and conservation rules plus a finite fixture backlog.

## Stage D: formats of record (1.5 days)

The paired re-review of `967dd7123` returned APPROVE from Opus and FIX from Sol. The owner authorised one final targeted design fold. The [round-four dispositions](FORMATS-DECISIONS.md#review-round-4-dispositions) and [fixture ledger](FORMATS-LEDGER.md#round-four-fixtures) record the handoff. The [DESIGN amendments](FORMATS.md#changes-to-design-sections-5-and-6) state the proposed authority for G-repro. Stage D awaits owner disposition.

| Slice while awaiting that decision | What can proceed | What requires approval and execution |
|---|---|---|
| S1 | Corpus pins, guard, paired input hashes and standalone run receipts. | Prospective registration/reuse and any measured gate claim. |
| S2 | Bag reconstruction, counters, parser/classifier/validator signatures, disk-backed node views and terminal tests. | Credit for line joins, inventory zeroes, registered classifiers and G-repro. |
| S3 | Both compiler programs, byte helpers, original-row module dispatch, census and labelled self-check fixtures. | Built-in or adopter grading, scoped quality claims and G-selfcheck/G-adopt. |

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

**Guard integration.** Add the dedicated `--status-fd` result from FORMATS §2. Keep the host-quiet script at `$(dirname "$0")/../hostquiet/host_quiet.py` and the log at
`${MEMGUARD_LOG:-${XDG_STATE_HOME:-$HOME/.local/state}/prism/memguard-kills.log}`; exits 75, 76, 137 kept.

**Contract.** Implement `corpora.json`, `corpora list|verify`, `run` and `prism.measure.receipt/1` exactly as FORMATS sections 1–2. Include the adopter pins and the S2 fixture entry when its tree is available. Per-package SB verification ships now (a caller-supplied manifest may register acquired packages); default SB registration follows authenticated acquisition (S6b). At registration pin the complete §2 type/path/value digest of the extracted query root, including symlinks. Verify Git bytes against the pinned subtree independently of index flags. Check §2 link closure, run on the verified snapshot and recheck through completion. For an extraction verify its registered digest and any supplied perf-cloud digest. Pin errors (3) precede analysis. Version mismatch runs version only. Record every produced stdout/stderr hash, including the trace, and the structured guard status per command. A fresh gate-bound receipt includes its decision digest and step ID from before version. S5 supplies independent decision registration and dependency views for prospective differentials. A pre-launch quiet refusal (75) has no receipt. A late refusal stops the run and retains its failed receipt. Output, digest, guard and Python 3.9 cases are listed by ID in FORMATS sections 8–9.

- [ ] **Failing tests first.** `helpers.py`: a one-commit git repo named `Z` in a temp manifest, a fake prism
      that logs argv and env and prints canned JSON, a pass-through guard writing §2 status to its caller's status descriptor, `kit(*args)` running the module.
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
def test_guard_refusal_exits_75(self):                   # structured host_quiet -> 75; launched analysis returning 75 -> run 4
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
- [ ] Check the kit and guard for machine paths and private names. An invocation error cannot certify a clean scan. Smoke: `run` on EX with `repro/bin/prism-base`; receipt path in
      `LEDGER.md`. PR body: formats as built, totals, smoke.

### S2: `diff-edges`, `diff-stats`, classifiers, G-repro (5 days; G-repro, G-hermetic)

Lane 10’s recall scope uses its own comparator and trace. The earlier S0 producer and rollout amendment is optional reference material. S2 owns the standalone parser, context, trace joins, complete-inventory admission helper and byte-span index with hermetic tests. A lane that elects the optional inventory profile owns its producer, measured rollout and policy for cross-key, orphan-anchor and spelling-leak. Its validator consumes both decoded path columns, checks every residual occurrence it claims to cover, and requires complete inventories and unique byte-bound candidates for endpoint claims.

**Files:** create `scripts/measure/{edges,diff_edges,diff_stats,classifier,trace}.py`,
`examples/classify_783_4b.py`, `fixtures/repro-4b/` (`repo/` holds only the measured sources, with M-3's truth and control generator alongside it),
`tests/{test_edges,test_diff_edges,test_diff_stats,test_examples}.py`; enable `comparator` and `join` in
`test_matrix.py`.

```
python3 -m measure diff-edges BASE_RUN HEAD_RUN --classifier FILE [--key bytes|line] [--line-reason TEXT] [--out DIR] [--list]
python3 -m measure diff-stats BASE_RUN HEAD_RUN [--allow FILE] [--which dfg|call|both]
```

**Contract.** FORMATS sections 1–3 and 8–9 fix admission, projection, residual bags, `changes.jsonl`, `classes.tsv`, `certificate.json`, counter tables and both legacy stdout lines. One identical transition is one counted row. One-sided residuals at a key share one bag. Check reconstruction per arm and attribute. Unattributed counts rows. Ambiguous edge records fail (1), even at a clean key. Default-forbidden strays fail (1), and unmatched records alone may pass. A whole classified ambiguous change may pass. Expose both run roots, receipts, unchanged occurrences, nodes and both counter sets to the callbacks by their published names. Grade counter allow-lists by component tuple, escaping literal dots. An erased trace key is unmatched. A sole projected candidate with contradictory known bounds is unmatched. Two projected candidates stay ambiguous. Index node candidates by bytes, file and access. Standalone callbacks see their original run paths. Prospective callbacks see only authenticated run artifacts, declared sidecar/helper bytes and the verified source. Record a reason and flag for deliberate byte-to-line projection. The example supplies a separate historical projection for the G-repro trace counts. S2 uses M-3's independently registered historical prediction and fixed inputs for this protocol. S5 later exercises the general `gate` wrapper.

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
    self.diff([a, b], [swap_labels(a, b)], key="line", line_reason="matrix line projection"); self.assertEqual((self.changes(), self.cert()["collisions"]), ([], 1))
def test_mixed_capability_refuses_byte_mode(self):            # bytes vs none -> exit 2; --key line with declared reason -> 0 changes
def test_partial_bounds_refuse_byte_mode(self):              # one endpoint without end_byte -> exit 2
def test_path_fields_stay_structured(self):                  # fields ["a.b"] and ["a", "b"] are two keys
def test_trace_record_attaches_only_to_a_unique_changed_edge(self):  # byte 154 attaches in byte mode. In join-002 line mode there are zero changes, one ambiguous record, exit 1.
def test_node_inventory_reaches_validation_with_zero_changed_edges(self):  # isolated def/use and duplicate spans from both arms remain available
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
      identity counts include all eligible mechanisms through the legacy projection, with identity strays allowed. R4B joins the two attribution mechanisms with strict strays).
- [ ] **G-repro** (measurement runner; M-3's bound prediction and the replacement DESIGN §6 text in FORMATS):
      (a) `run` X, EX, T with both M-3 binaries (head with `--trace-env PRISM_RD_CENSUS_TRACE=1`), then
      `diff-edges --classifier examples/classify_783_4b.py` with `--key line --line-reason "historical comparator used line keys"` and in byte mode, with a distinct `--out` per corpus and mode, and
      `diff-stats`. Require primary edge stdout byte equality and PASS in both modes, and zero DFG and call counter deltas. Record full `trace mechanism lines:` byte equality per corpus under M-3's ordered count trials. Publish the head that earned each claim. (b) `run` `R4B` with both binaries, then
      `diff-edges` in byte mode: the 139 independent rows match (22/14/103), the 42 dump-derived siblings are reported separately, and the whole run passes at 23/55 attributed changes. Each control rebinds its edited trace and returns 1 for the predicted join/classification reason in FORMATS section 3.
- [ ] PR body: recorded-versus-kit table per corpus for (a); truth-versus-kit per row for (b) and the five
      control results; formats as built.

### S3: TypeScript oracle and self-check (4 days; G-selfcheck, G-hermetic)

Lane 9 uses `binding`, `literal`, `spread`, `builtin` and `argfollow` through one public question module. Provide the exact checker helpers and full-span lookup in FORMATS section 4.

**Files:** create `scripts/measure/oracle/{ts_oracle,bytes,selfcheck}.cjs`, `oracle/questions/{argument_binding,
lexical_binding,calls}.cjs`, `oracle/selfcheck/<case>/{repo/,input.jsonl,expected.jsonl}`,
`oracle/realuse/{repo/,truth.jsonl}`, `scripts/measure/oracle.py`, `tests/test_oracle.py`; enable `oracle`
in `test_matrix.py`. Cover verdict rows with self-check cases and admission, selection and delta rows through the Python wrapper.

**Contract.** Implement the two oracle command grammars and self-check entry in FORMATS sections 1 and 4, including `--side`, `--kind`, `--class`, `--relation`, `--target-confidence` and `--target-kind`. Verify the checker tree/file, combined question and fixture key before launching grading. Select only the named run's residual occurrences against a bound certificate. Grade spread LOST on the base arm. For an admitted call diff retain all deltas, including changes in per-site drop reasons, with exact gained targets as the default. Cancel identical full row/index contexts before projected claim counts. A grade requiring a choice among unequal residuals refuses before dispatch. Lane 9 selects `name_only/argument_follow` in census or differential mode. Keep per-confidence/kind denominators, drops and ancillary records. Refuse ambiguous raw subsets, admit source coordinates before question-specific shape tests, grade callable claims only against a proved body, and key tuples by relation and property. Admit the producer callable's first non-decorator token line at exact formal bytes in either direction. Check the formal's decoded wire spelling for a fixed slot. Grade a declaration/write to a value read, including an old-value read in a compound/logical assignment or update. Distinguish whole and composite recursive actuals. Bind each supported target form to its build qualification. Count authenticated out-of-program edges separately under `--all`. Constructor, inferred/anonymous, computed or split-header target identity stays UNDECIDED under the built-in. Retain the slot and lane-module proofs where supported. Use program-aware lookup, the require hook and the reviewed closure for question code and declared data. Produce and bind target qualifications through the [S3 fixture builder](FORMATS-DECISIONS.md#producer-qualification). Use the prescribed `verdicts.jsonl`, `tuples.json`, `denominators.json`, `oracle-receipt.json` and call-diff `deltas.jsonl`. Aggregate mode keeps denominators and custody with the tuples. Use FORMATS section 8 decisions and materialize the oracle IDs in section 9.

- [ ] Author self-check cases first: per question one case per verdict, plus the near-miss negatives of
      G-selfcheck. Cover the wrapper rows with their own receipt, arm and denominator checks. `selfcheck.cjs` exits 0 only when every verdict equals
      `expected.jsonl` in key, relation, property and verdict, with no missing or extra rows.
- [ ] Failing Python tests (fake `node` on `PATH`): `test_oracle_refuses_wrong_typescript_hash` (exit 3, node
      log empty), `test_oracle_refuses_without_selfcheck_receipt_for_current_digest` (a fixture edited after
      the receipt → exit 3), `test_oracle_tuples_include_zero_counts` (`{"CORRECT": 2, "WRONG": 0,
      "UNDECIDED": 1, "INADMISSIBLE": 0}`), `test_oracle_rejects_unknown_verdict_from_question_module` (exit
      1), `test_ts_calls_reports_a_retargeted_site_between_runs` (base site → A, head → B: one `retargeted`
      delta, new exact head claim graded), `test_ts_edges_source_digest_mismatch_is_inadmissible`,
      `test_removed_spread_selects_the_base_bag`, `test_enclosing_call_uses_both_byte_bounds`, and
      `test_default_lib_program_uses_same_sources_and_its_own_checker`. A census has no delta output.
- [ ] RED, implement, GREEN, commit. **G-selfcheck:** `scripts/memguard/memguard.sh 8 node
      scripts/measure/oracle/selfcheck.cjs --typescript <installed path>` exits 0; output in the PR. One real
      use: `run` `oracle/realuse/repo` (authored argument- and lexical-binding shapes, truth written before
      the run) with `repro/bin/prism-head`, then `ts-edges --all`: every graded verdict equals `truth.jsonl`,
      at least ten CORRECT, no INADMISSIBLE row; report the tuple. Check the supported decorated method's actual `function_id` against a source-labelled capture and a decorator-line mutation, and check multiline formal defs in both questions. Bind the executable hash, labelled source, raw record/index, controls and qualification bundle in the self-check and grading receipts. Capture oracle-061's native `name_only/argument_follow` target. A failed producer control leaves that built-in convention unqualified and the required S3 control unmet.

### S4: `review-diff` and the diff population (2.5 days; G-hermetic)

**Files:** create `scripts/measure/{population,review_diff}.py`, `populations/602-m2-120.json`,
`tests/{test_population,test_review_diff}.py`; enable `review` in `test_matrix.py`.

**Contract.** FORMATS sections 1, 5, 8 and 9 define the public `602-m2-120.json`, exact #602 spec bytes, CLI, arm envelopes, full finding payload and sample. Reconstruct each postimage from the fixed-option patch in a detached scratch clone, decoding quoted path bytes as specified in §5. Verify the pinned worktree digest immediately before and after each arm, retain both observed hashes, and stop that ordinal on drift. Retain and hash both raw streams for each launched ordinal/arm. Round-trip the exact positional review receipt in review-018, including a failed or unlaunched arm. Record every manifest ordinal, including excluded, failed and refused rows. Sum presence surplus and overlapping payload differences after exact cancellation, including unpairable bags. Emit `per-diff.jsonl`, `totals.json`, `sample.jsonl` and `review-receipt.json`. Apply subset and successful-sample denominators explicitly.

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

**Contract.** FORMATS sections 1, 6, 8 and 9 fix `gate FILE --out DIR`, overrides, step kinds, reference types, reuse, expectations, child exits and the eight ordered report sections. Preflight the whole graph and all reused inputs before measurement. Register the decision digest independently before any result for that differential. Check the controller's `--decision-pin` on both fresh runs and reuse. Freeze the prediction, graph, every threshold and classifier dependencies through publication. Stage and verify the §6 file views and read back the exact step and review receipt shapes. Verify both the gate binding and the declared binary SHA/version on each reused run. Check coverage for each asserted relation/property and confidence/kind, including explicit intersections. For lane 9 require exact WRONG=0 and `name_only` WRONG/(CORRECT+WRONG) ≤ 1/20 on the named populations, with a positive preregistered minimum. Use integer cross multiplication. Report each scope and absence-only control. Record waivers under Limits. Publish `claim_scope` on each step and the gate, and `product_steps` on the gate. An actually used empty waiver can pass only as a control. Require a declared identity control for the same effective configuration, even on fresh runs. Exit 0 only for a prospective PASS, 1 on a failed measurement or completed retrospective, 2 on malformed gate/admission/prediction refusal (including child interface exit 2), 75 with result `host_quiet` only on structured guard refusal. Keep failed step IDs and observed metrics. Write `gate.json` and `REPORT.md`.

- [ ] Failing tests: `test_gate_refuses_missing_prediction`; `test_gate_refuses_reused_run_without_matching_
      prediction_digest` (a receipt without a digest → 2; a receipt whose digest differs while the
      prediction keeps its old modification time → 2); `test_gate_refuses_prediction_changed_during_the_gate`;
      `test_retrospective_replay_is_never_a_pass`; `test_gate_fails_on_threshold_and_names_the_step`;
      `test_gate_rejects_unknown_step_kind`; `test_report_has_the_eight_sections_in_order`;
      `test_report_quotes_prediction_and_its_sha256`.
- [ ] RED, implement, GREEN, commit. Run `examples/gate_783_4b.json` with its registered decision pin (paths supplied at run time); its
      numbers equal S2's part (a).

### S6a: benchmark finding-level accounting (2 days; G-findings)

**Files:** create `eval/secbench/{findings,test_findings,test_matrix_bench}.py`; modify `run.py` (keep the
parsed taint channel output instead of discarding it; add `row['finding']`), `replay.py` (score from retained
`taint.json.gz`), `report.py` (case-level line-hit table and finding-level counts beside reach), `README.md`.

**Contract.** FORMATS section 7 (S6a) defines `finding_outcome(row, taint_output, invocation)`, eligibility, precedence, RuleRef payloads and both denominators. Reuse the same scorer for the bound retained taint stream and live output. Keep `row.finding={line_hit,findings,rules}` and every emitted sink occurrence. Credit at most one match. Enumerate plausible emitters from the bound versioned sink-rule table. Resolve each emission by its own evidence before taking the case result. A proved match gives `sink_line` even beside an unresolved peer. Without a proved match an unresolved emitter tie gives `ambiguous`, including with zero findings. A uniquely identified different operation gets zero matched. Identical repeated hits at the annotated operation give one match and one off-target. A same-file miss outranks an elsewhere miss. Channel errors remain separate from reach. Enable the 19 finding rows plus selected S6a fixtures. Existing `--class`, `--binary-sha` and `--sut-commit` are already supplied by the harness.

For S6a publish `sink-rules/<binary_sha256>/<benchmark_revision>/rules.json` as §2 canonical JSON `{schema:"prism.secbench.sink-rules/1",sut_commit,binary_sha256,benchmark_revision,sources,rules}`. `sources` maps producer source paths to SHA-256 of raw bytes at that commit. Each rule is `{source,symbol,operation,match,pattern,family}` with source path, producer symbol, operation `call|property_write|other`, match `exact_identifier|substring_identifier|call_path|unresolved`, original selector string and verified emitted family ID or null. Sort entries by canonical bytes and remove exact duplicates. For repair-r1 derive the JS/TS candidates from the code feeding `taint_sink` at `4e592daa7858a195eb3a9eb77c83dfbc763b49fa`, including `SINK_PATTERNS`, `matches_sink` and the structured registries in `src/algorithms/taint.rs`. Preserve exact versus substring matching, property writes and all applicable families. Mark unsupported predicates, dynamic wrappers and other unclassified potential emitters `unresolved` on affected occurrences. Enumerate plausible spans conservatively. A family may narrow a finding only when authenticated on that emission. Build the table from producer rules independently of benchmark outcomes. Register its digest for the full observed executable hash and benchmark revision before replay. Reuse that table for live and fixed scoring of the same build, and fail binding on a missing or substituted replay table.

- [ ] Failing tests (`bench-findings` rows enabled): `test_sink_line_finding`, `test_other_line_same_file`,
      `test_finding_elsewhere`, `test_no_taint_sink_finding` (a `taint_source` finding does not count),
      `test_channel_error`, `test_two_sinks_on_the_gt_line_is_ambiguous`,
      `test_extra_findings_are_off_target_rows_not_another_case_outcome`,
      `test_one_match_among_duplicate_findings`, `test_rules_copied_when_present`,
      `test_replay_and_live_scoring_agree` (on every `bench-findings` row),
      `test_reach_summary_unchanged_by_finding_fields` (`summarize()` equal with and without `row['finding']`).
- [ ] RED: `cd eval && uv run --locked pytest secbench/test_findings.py secbench/test_matrix_bench.py -q`.
      Implement, GREEN, commit.
- [ ] **G-findings** by replay (rescore, no prism run): raw tree `<evidence>/meas/secbench/repair-r1/
      measured` (SUT `4e592daa`, binary `d603e7cd…`, 373 taint outputs). Confirm it is the observation set
      `repair-r1/final/binding.json` names and take the population from that binding (not from the research
      count); replay; require every reach outcome equal to `repair-r1/final/entries.jsonl`, one case result on every case with ground truth plus one finding row per emitted sink, and at least one `sink_line`. PR: table per class, channel errors
      counted.

### S6b: benchmark negatives (4.5 days; G-negatives)

**Files:** create `eval/secbench/{fixed.py,test_fixed.py,locate.mjs,locate.test.mjs,fixed-pins.json}`;
modify `report.py`, `README.md`; enable `bench-negatives` in `test_matrix_bench.py`.

**Contract.** Use FORMATS sections 1 and 7 (S6b) for the two `fixed.py` commands, registry/SRI/size/extraction checks, pin publication, bound adjudications and exact reason vocabulary with precedence. Read each entry's `package.json`. `acquire --packages VULN` binds its dependency to the vulnerable package manifest. Without that option acquisition requires a single exact dependency. On first acquisition publish the archive and tree pin after complete verification. On reacquisition verify the existing pin before reuse. `score` requires the manifest. `locate.mjs` uses the pinned TypeScript scanner and syntax tree for owning functions, actual call occurrences and byte spans. Tokens preserve raw literals. Require independently tree-bound contribution evidence for `fix_elsewhere` and `partial_fix`. An unrelated caller edit with intact seed forwarding goes through ordinary retained scoring. A direct identifier substitution in an untagged template is a supported terminal sink operand. Withhold a column when its channel cannot distinguish the annotated occurrence. Bind the fixed scorer to the vulnerable SUT SHA and version, prove the annotated occurrence's correspondence across trees or its removal, and match each reach witness to the mapped seed and a terminal sink value. An otherwise eligible candidate lacking a fixed acquisition row, pin or tree gives `fixed_input_missing`. A known move into a helper cannot earn removal credit. A failed or truncated negative observation cannot earn a false column.

Produce `acquisition.jsonl`, `negatives.jsonl`, `negatives-summary.json` and `score-receipt.json` with retained raw observations. A verified pair has `finding,reach,function_change,sink_removed`; finding/reach are null for verified removal. An exclusion has one closed reason and four nulls. Enable all 26 negative rows plus selected S6b fixtures. When materialising the successor, align the benchmark row validator’s reason enum with the closed §7 table, including fix_elsewhere, partial_fix and fixed_input_missing, and install the §8 adjudications, the independent bn-031 retained-identity control and the bound synthetic vulnerable witnesses for bn-001/002/005/006/007/008 and bn-024. Verify the nested score receipt on each completed batch, including one below the minimum.

- [ ] Failing tests: `test_fixed_version_parsing` (`" =1.20.0"` → `"1.20.0"`; `"n/a"` and `">=2.1.1 <3.0.1"`
      → `None`), `test_extraction_refuses_traversal_links_and_collisions`, `test_member_and_size_limits_are_
      reasons`, `test_failed_extraction_publishes_nothing`, `test_redirect_to_another_origin_is_refused`,
      `test_integrity_mismatch_is_a_reason_not_a_crash` (injected fetcher, no socket), `test_pin_drift_is_a_
      reason`, `test_locate_none_one_many`, `test_sink_moved_to_a_helper_is_a_reason_not_a_score`,
      `test_unrelated_reach_in_the_function_is_not_reach`, `test_literal_change_is_a_function_change`,
      `test_every_attempted_entry_has_outcome_or_reason`, `test_zero_verified_pairs_reports_unmet`.
- [ ] RED (`uv run --locked pytest secbench/test_fixed.py secbench/test_matrix_bench.py -q`), implement,
      GREEN, commit.
- [ ] **G-negatives.** Acquire archives only, at the configured evidence input destination. Account for all entries, including every attempted named version, and score using the bound baseline. Before scoring verify possession of the exact repair-r1 executable (`d603e7cd…`, SUT `4e592daa`) and a vulnerable binding carrying its full SHA/version/commit. The prediction sets `N=28` from that baseline's 57-candidate census. An unavailable build/binding or fewer verified pairs leaves historical G-negatives unmet. A separately chosen build requires new vulnerable observations, a new census and a registered minimum before any fixed query. Publish each retained observation column and the exclusion distribution within their stated denominators.

### S7: removed by the design review (2026-11-21)

The cloud launchers are a follow-up issue (DESIGN section 7, decision 3; close-out).

### S8: documentation and the manual (1 day)

- [ ] `scripts/measure/README.md`: every subcommand, the stage D formats, writing a classifier with `examples/classify_783_4b.py` as the worked example, the
      matrix and its groups, the self-check rule, running in a cloud session. `CLAUDE.md`: a short
      "Measurement kit" paragraph under Build & Test with the unittest command.
- [ ] `ORCHESTRATION.md`: section 4 guard path → `scripts/memguard/memguard.sh`; section 5 item 6 and
      section 6's measurement brief → "a gate file, a prediction and a classifier for `python3 -m measure
      gate`; no comparator, run recipe, counter diff or trace join"; one sentence that this overrides a lane
      plan's own measurement-tooling task for any lane whose stage M starts after this PR merges.

## Close-out (1 day, plus the wait for G-adopt)

- [ ] **G-adopt** (DESIGN section 6). Lane 9 remains the named adopter. Hand it the oracle contract and self-check layout when S3 merges. Lane 10’s recall decision uses its own comparator and trace and gives no G-adopt credit. A later decision by lane 10 can qualify if it uses the kit for its registered population. For an adopter migrating after stage M, first replay the baseline and show parity. Allowed files are one classifier, at most one oracle question, gate files, predictions and census scripts for questions outside the kit. Record the consuming lane, registered scope, decided denominator and limitations. If no eligible lane takes a measured decision on kit output by close, report G-adopt unmet and name the blocker.
- [ ] Issues (filed, not started), each with the decision it would support: rule-aware finding scoring after
      lane 7 (benchmark rule columns); the cloud launchers (a lane that needs a kit gate in a cloud session);
      `review-diff` as a cross-check for PR #841 (only if #841's merge needs an independent count; same format
      and flags, so a SARIF reader); one corpus registry with `scripts/perf/cloud/corpora.json`;
      `eval/secbench/run.py`'s committed machine path and compiler default; each G-adopt blocker. Then
      `HANDOFF.md`, `LEDGER.md`, offload, remove both worktrees.

## Acceptance for the lane

G-repro (a) and (b), G-selfcheck, G-hermetic, G-findings and G-negatives hold on merged code, or a gate that
does not is reported unmet with its numbers (including missing same-build custody or G-negatives below N). G-adopt needs an actual adopter decision and baseline parity where required. Report it unmet if that evidence is absent. ORCHESTRATION points lanes at the kit and
each slice approved by its seats with CI green.

## Day estimates

| M | T | E | D | S1 | S2 | S3 | S4 | S5 | S6a | S6b | S8 | Close-out | Total |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 2.5 | 2 | 1.5 | 1.5 | 3 | 5 | 4 | 2.5 | 2 | 2 | 4.5 | 1 | 1 | **32.5** (about 26 calendar days with S6a and S6b beside S3 to S5) |
