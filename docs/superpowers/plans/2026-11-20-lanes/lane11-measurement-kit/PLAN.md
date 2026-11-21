# Lane 11: a shared measurement kit — implementation plan

> **For agentic workers:** run by an orchestrator under `../ORCHESTRATION.md`. Each task goes to an implementer
> (Sonnet-5.5, high) with its section as the brief; briefs for S1 to S5 also carry `FORMATS.md` (stage D).
> In test listings, a `def` line ending in a comment is a test to write; the comment is its assertion.

**Goal:** one tested measurement kit in the repository, so a lane writes only a classifier file (and, when
needed, an oracle question); plus finding-level scoring and negatives for the security benchmark.

**Architecture:** a standard-library Python package `scripts/measure/` run as `python3 -m measure` from
`scripts/`; a Node oracle in `scripts/measure/oracle/` on the pinned TypeScript 5.9.3; benchmark work in
`eval/secbench/`; the memory guard and cloud launchers moved to `scripts/memguard/` and `scripts/cloud/`.

**Tech stack:** Python 3.9+ standard library, `unittest`; Node with the TypeScript tree that
`scripts/gate-inputs/acquire.mjs` installs; the eval project (uv, pytest). **Spec:** `DESIGN.md` here.

## Global constraints

- Worktree `~/code/prism-measure-kit` (ORCHESTRATION section 4; S6a and S6b need `eval/secbench/`, so skip
  the sparse-checkout). Evidence `~/prism-evidence/measure-kit/` (`LEDGER.md`, `HANDOFF.md`, `inventory/`,
  `repro/`, `secbench/`).
- `scripts/measure/`, `scripts/memguard/`, `scripts/cloud/`: standard library, Python 3.9 syntax
  (`from __future__ import annotations`, no `match`). Unit tests use no network, prism binary, corpus or
  TypeScript; fakes are small scripts written to a temporary directory.
- Committed files: no private corpus name, no absolute machine path, no raw dump. Private-name check of
  ORCHESTRATION section 9 before every commit; briefs name the private evidence directories as not to be
  opened and say that sentence is not to be copied into repository files.
- Every prism, `cargo` or TypeScript-checker run goes through the guard (`~/prism-evidence/tools/memguard.sh
  8`, from S1 on `scripts/memguard/memguard.sh 8`), one at a time; on exit 75 run `python3
  scripts/hostquiet/host_quiet.py wait` and retry. Counting runs take no marker.
- Cloud: 4 vCPUs, 15 GB, no swap, ~30 GB disk, no `/usr/bin/time`, REST-only GitHub, silent push failure
  for any file over 100 MB, nothing private, no kubernetes or cockroach corpus.
- Per-PR gates (ORCHESTRATION section 5): 3, 4, 8, 10 (kit README in the same PR) and 11 apply; 1 runs in CI.
  Item 2 is the slice's unit-test command. Items 5, 6, 7, 9 do not apply: no Rust change.
- Every PR body: what changed, formats as built, test totals (local and CI), the slice gate's prediction and
  result, what was not verified, the review record. Branches `feat/measure-kit-<slice>`.
- Review seats (caps: design 2 rounds, slice 3 rounds):

| Work | Reviewers | Why |
|---|---|---|
| Stage D, S2, S3 | A (Opus-5.5 xhigh) and B (gpt-6.1-sol xhigh) | formats bind every lane; a false PASS in `diff-edges` or a wrong oracle verdict passes unseen through every lane's merge gate |
| S6b | one, Opus-5.5 high | registry download and archive extraction are a security boundary |
| S1, S4, S5, S6a, S7, S8 | one, Sonnet-5.5 high (the manual's tooling reviewer) | failures are visible (refusals, receipts, reports) or proven by a gate |

## Review focus

| # | Input or condition | Required | Pinned by |
|---|---|---|---|
| 1 | Edge dump too large to hold as records (TypeScript `src`: 408,954 edges, ~108 MB raw) | streamed from gzip; per-key counts only | `test_diff_edges_streams_large_gzip_within_memory_bound` (S2) |
| 2 | Two edges differing only in byte offsets | two keys in byte mode; one key, count 2, in line mode | `test_byte_only_difference_is_a_change_in_byte_mode_and_not_in_line_mode` (S2) |
| 3 | A classifier that raises or returns an undeclared name | change unattributed, exception recorded, exit 1 | `test_classifier_exception_counts_as_unattributed` (S2) |
| 4 | Corpus pin mismatch or dirty checkout | refused before the binary runs | `test_run_refuses_corpus_with_pin_mismatch` (S1) |
| 5 | Prediction written after the first measurement | `gate` refuses | `test_gate_refuses_prediction_newer_than_first_measurement` (S5) |

## Stage M: inventory and reproduction target (2 days)

This lane builds tooling: stage T is each slice's tests-first step, Task M-2 replaces stage E (the
population is the finite set of forks), stage G is the gates named per slice.

- [ ] **M-1.** Worktree, evidence directory, tracking issue "Shared measurement kit (lane 11)" linking
      `DESIGN.md` and this plan; its number in `LEDGER.md`.

### Task M-2: inventory of the forks (measurement runner seat)

**Output (evidence only, never committed):** `inventory/INVENTORY.md`. Read each file (under
`~/prism-evidence/`, else the copy in `~/code/prism-evidence-repo/`; restore nothing from release assets).

| Family | Files (evidence paths unless marked) |
|---|---|
| (a) run recipe | `jsts-lane/783/scripts/{measure783.sh,gate783.sh,measure761-cloud.sh,timewrap.py}`; `jsts-lane/{599,627,664,759}/scripts/measure*.sh`; `jsts-lane/660/s*/scripts/measure660-*-cloud.sh`; `jsts-lane/693/scripts/{measure693.sh,run-measure.sh}`; `provenance-survey/slice4ab/scripts/{gate4b.sh,parity.sh}` |
| (b) edges | `jsts-lane/783/scripts/{compare783.py,compare761.py}`; `provenance-survey/slice4ab/scripts/compare4b.py`; `jsts-lane/660/*/scripts/{compare_edges.py,join_trace.py}` (7 slices); `jsts-lane/759/scripts/{join_trace.py,check_counters.py}`; `jsts-lane/693/scripts/compare693.py`; `perf-437/measure-645/{labeldiff.py,r5/labeldiff_strat.py}`; `provenance-survey/scripts/{edgediff,attrib,transitions}.py`; `nav-gaps/scripts/site_diff.py`; `provenance-survey/slice4c/scripts/run_gate_4c.py` |
| (c) counters | the `stats` modes above; `jsts-lane/783/scripts/{attribute_counters.py,attribute_return_inputs.py}` |
| (d) oracle | `js-args/{census.cjs,selfcheck.cjs}`; repository: `docs/superpowers/plans/2026-10-06-js-param-defs/probes/{adjudicate.cjs,census.cjs,oracle-read-role.cjs}`, `…/2026-10-04-workspace-package-resolution/probes/census.cjs`, `…/2026-10-03-s2-import-qualifiers/probes/census.cjs`, `eval/secbench/{inspect.mjs,bindings.mjs}` |
| (f) review output | `jsts-lane/602/m2/scripts/*.py`; `DISPATCH.md`, `population.jsonl` from `archive/prism-crossfile-docs-493ab5ee-receipts.tar.gz` (member dir `next-five-plan-fable-20261114/final-gate-campaign-preparation/`; extract to `inventory/602/`); `campaign-primary-control/drive.py` from `archive/prism-602-primary-row001-841.tar.gz` |
| (e) other-language oracles | not read: out of scope (DESIGN section 3 non-goals) |
| (g, h) cloud, guard | `tools/{cloud-measure.sh,cloud-tests.sh,cloud-probes.sh,push-cloud-input.sh,memguard.sh}`; `cloud-trial/{cloud-measure-run.sh,cloud-test-run.sh,cloud-gate.sh}` |

- [ ] One table per family, a row per behaviour (input form, key field, label rule, exit condition, output
      line, refusal): the files that have it; verdict `keep: <subcommand>`, `lane: classifier` or `drop:
      <reason>`; mark disagreements between forks (`kill_line` in the label, gzip, `[]`, failing strays) for
      stage D. Also list each trace prefix parsed (`rd-census`, `rd-census-owner`, `rd-alias-move`, `own-*`,
      `c-prune-*`) with its fields, each corpus with its pin, each machine-specific string in (g, h).

### Task M-3: reproduction target for G-repro

The design names #783's alias-kill slice. Its finished gate is slice 4b, PR #786 (squash `ae13feee`):

| Item | Value |
|---|---|
| Base / head | `4f9e3a3e` (main at merge; the PR states it is byte-identical to `ead6edf0` and the 4a tip) / `1f2ad1cc` (PR head, fold 3b; binary `prism-4b-fold3b`, SHA-256 `c52604ea…` in the evidence repository's `manifests/provenance-survey-slices.md`) |
| Recorded numbers | evidence repository `provenance-survey/slice4ab/results/gate-4b-compare-{X,EX,T}.txt`, `gate-4b-f-identity.txt`; PR #786 body, "Gates" |
| Expected | edges 80,033 / 7,445 / 388,055 (excalidraw / express / TypeScript `src`); 0 added, removed and label-moved; `dfg-stats` and `call-stats` identical; per-mechanism trace counts as in each compare file |

- [ ] Build both commits (`cargo build --release`, guarded) in detached scratch worktrees under the evidence
      directory; keep `repro/bin/prism-{base,head}`; delete the scratch `target/`s. The trace counts may come
      from an earlier fold: if S2 does not match them, try PR #786's fold commits before calling it failed.
- [ ] Fallback (DESIGN section 8), on toolchain drift only: the earlier #783 round in `jsts-lane/783/` (base
      `4c8620a2`, head `4da81c36`; `local/gate/{X,EX,T}.compare.txt`: 85,645 / 7,691 / 408,954 edges).
- [ ] Every recorded class count is zero: the target proves recipe, keys and trace join, not attribution of a
      real move. Supplementary target (owner question 6): #660 S1a cloud round 3, base `31703c86`, head
      `152d22bc` (`origin/feat/660-s1a-deferred-reads`); evidence repository `jsts-lane/660/s1a/cloud-r3/
      SUMMARY.md` (moved 1,398 / 49 / 722, by old label and mechanism); classifier from S1a's `join_trace.py`.
- [ ] `repro/TARGET.md` and `repro/PREDICTION.md` ("identical to the recorded files") before any kit run.

**Owner question 6 (added by this plan):** G-repro on #783 cannot show attribution of a non-zero move.
Default: G-repro stays as DESIGN states; S2 also runs the #660 S1a target, non-gating, and reports it.

### Task M-4: PR #841

- [ ] Read #841's body and the newest `archive/prism-602-*-841*` archive (`controller-status.json`). Since
      2026-11-20 #841 has its own campaign coordinator and has run primary rows, so `review-diff` is an offer
      of an independent cross-check, not its coordinator (owner decision 4). Record the state in `LEDGER.md`.

**Exit gate:** INVENTORY.md covers every file listed; TARGET.md names commits, binaries and numbers.

## Stage D: formats of record (1.5 days)

The design seat (Fable-5.2, high) gets DESIGN.md, INVENTORY.md, TARGET.md, the drafts of S1 to S5, and lane
3's planned classes (its PLAN Task M-3) as the first consumer. It writes `FORMATS.md` here, fixing every
subcommand's flags; the manifest, receipt, run-directory layout, `changes.jsonl`, certificate, classifier
contract (`context`, trace hook, whether strays fail), allow-list, gate file, report skeleton, population
manifest and `review-diff` outputs; the edge label (with or without `kill_line`); exit codes; generic
classes, if any. Drafts are inputs. Gate: both reviewers APPROVE (cap 2, then ORCHESTRATION section 8). The
orchestrator puts FORMATS.md names into each slice section before dispatch.

## Stage I: slices

Order: S1 (every subcommand reads run directories), S2 (G-repro proves the kit; lanes 3, 9, 10 need
`diff-edges` first), S3, S4, S5 (the gate composes S1 to S4), S7, S8. S6a and S6b touch only
`eval/secbench/` and run beside S3 to S5 with a second implementer. The memory guard moves in S1, not S7:
`run` needs a guard in the repository for anyone else or a cloud session to run it.

### S1: corpus manifest, `corpora`, `run`, memory guard (3 days; G-hermetic)

**Files:** create `scripts/measure/{__init__,__main__,cli,corpora,run,receipt}.py`, `corpora.json`,
`README.md`, `tests/{__init__,helpers,test_corpora,test_run,test_syntax}.py`; move the evidence `memguard.sh`
to `scripts/memguard/` with `test_memguard.py`; in `.github/workflows/ci.yml` job `build-checks`, after "Perf
tooling tests", a step "Measurement kit tests (Python)": `python3 -m unittest discover -s scripts/measure -t
scripts -p 'test_*.py'` and `python3 -m unittest discover -s scripts/memguard -p 'test_*.py'`.

**Guard changes only:** host-quiet script at `$(dirname "$0")/../hostquiet/host_quiet.py`; log at
`${MEMGUARD_LOG:-${XDG_STATE_HOME:-$HOME/.local/state}/prism/memguard-kills.log}`; exits 75, 76, 137 kept.

**Draft manifest** (X, EX, T, D, M and each public corpus M-2 found; never `private-corpus`), entry `T`:
`{"name": "TypeScript", "kind": "git", "url": "https://github.com/microsoft/TypeScript", "commit":
"7964e22f2b85f16e520f0e902c7fd7b6f0c15416", "root": "src", "history": false, "local":
["bench-repos/TypeScript"], "manifest_sha256": null, "cloud_asset": "TypeScript-7964e22f-src.tar.gz"}`
under `{"schema": "prism.measure.corpora/1", "corpora": {…}}`.

`local` is relative to each `--corpora-root` or entry of `PRISM_MEASURE_CORPORA` (`os.pathsep` list). Pin:
a git checkout needs `HEAD == commit` and empty `git status --porcelain --untracked-files=all`; another tree
must match `manifest_sha256` as `manifest_hash` in `scripts/perf/cloud/fetch_corpora.py` computes it;
neither possible is a mismatch ("unverifiable").

```
python3 -m measure [--manifest PATH] corpora list | corpora verify [KEY...] [--corpora-root DIR]...
python3 -m measure run --binary BIN --corpus KEY --out DIR [--corpora-root DIR]... [--trace-env NAME=VALUE]...
                       [--no-bytes] [--sites] [--memguard PATH] [--limit-gb 8] [--expect-version HEX]
```

`run`: pin check and `--expect-version` (exit 3 on mismatch, nothing else executed); refuse a non-empty `--out` (exit 2); through the guard run
`nav --no-cache dfg-stats`, `nav --no-cache call-stats`, `nav --no-cache dfg-stats --edges` (with
`PRISM_EDGES_BYTES=1` and each `--trace-env`, this command only), and with `--sites`
`nav --no-cache call-stats --dump-sites`; guard exit 75 → exit 75; any other non-zero command → exit 4 with
the receipt written. Outputs: `dfg-stats.json`, `call-stats.json`, `edges.jsonl.gz`, `edges.stderr.gz`,
`sites.jsonl.gz`, one `.stderr` per command, `receipt.json`:

`{"schema": "prism.measure.receipt/1", "kit_commit", "start", "end", "binary": {"path", "sha256",
"version"}, "corpus": {"key", "commit", "root", "pin_check": "git-clean|manifest", "manifest_sha256"},
"commands": [{"argv", "env", "output", "output_sha256", "bytes", "exit", "start", "end"}], "guard": {"path",
"limit_gb"}, "host": {"platform", "cpus"}}` (times UTC ISO 8601; `version` is the binary's `--version`).

- [ ] **Failing tests first.** `helpers.py`: a one-commit git repo named `Z` in a temp manifest, a fake prism
      that logs argv and env and prints canned JSON, a pass-through guard, `kit(*args)` running the module.

```python
def test_run_writes_dumps_and_receipt(self):
    self.assertEqual(self.run_kit().returncode, 0); r = json.loads((self.out / "receipt.json").read_text())
    self.assertEqual((r["binary"]["sha256"], r["corpus"]["commit"]), (sha256_of(self.fake), self.commit))
    self.assertEqual(r["commands"][2]["env"], {"PRISM_EDGES_BYTES": "1"})
def test_run_refuses_corpus_with_pin_mismatch(self):
    self.commit_another_file(); self.assertEqual(self.run_kit().returncode, 3); self.assertEqual(self.fake_log(), [])
def test_run_refuses_dirty_corpus(self):                 # one untracked file -> 3
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
- [ ] `grep -nE '/Users/|prism-evidence' scripts/measure scripts/memguard` prints nothing. Smoke: `run` on EX
      with `repro/bin/prism-base`; receipt path in `LEDGER.md`. PR body: formats as built, totals, smoke.

### S2: `diff-edges`, `diff-stats`, classifiers, G-repro (4 days; G-repro, G-hermetic)

**Files:** create `scripts/measure/{edges,diff_edges,diff_stats,classifier}.py`,
`examples/{classify_783_4b,classify_660_s1a}.py`, `tests/{test_edges,test_diff_edges,test_diff_stats,
test_examples}.py`.

```
python3 -m measure diff-edges BASE_RUN HEAD_RUN --classifier FILE [--key bytes|line] [--out DIR] [--list]
python3 -m measure diff-stats BASE_RUN HEAD_RUN [--allow FILE] [--which dfg|call|both]
```

Endpoint key `(file, path, line, start_byte, end_byte)`, path = `base` plus `.f` per field (`[]` kept);
`--key line` drops bytes; an endpoint without bytes (old binary) uses the line key and the change gets
`"bytes": "absent"`. Label `(confidence, doubt or "-")`. Per-key multiset difference gives `added`/`removed`
with counts; equal counts with different label multisets give `label_moved`. Stdout starts with the recorded
comparators' line `edges base=N head=N added=A removed=R label-moved keys=M` (G-repro compares it verbatim),
then the class table, `trace mechanism lines:` (per tag, the number of distinct join keys carrying it, as
`compare4b.py` counts: not raw lines), then `PASS` or `FAIL`. A `changes.jsonl`
row: `{"kind": "added|removed|label_moved", "count", "key": {"from": {"file", "path", "line", "start_byte",
"end_byte"}, "to": {…}}, "base": [["exact", "-"]], "head": [["nameonly", "alias_unstable"]], "tags", "class",
"error"}`.

`certificate.json`: `{"changes", "attributed", "unattributed", "by_class", "trace_only"}`; exit 1 iff
`unattributed > 0`. Classifier file: `CLASSES = [...]` (every name it may return);
`classify(change, context) -> Optional[str]` (`None` = unattributed); optional `parse_trace(line) ->
(join_key, tag) | None` and `join_key(change)`. `context` has at least `tags(change)`, `corpus_root` and both
receipts. Allow-list: one dotted counter per line, `#` comments, trailing `.*` = prefix; `diff-stats` prints
`  key: b -> h (+d)` per delta, exit 1 on a delta not allowed.

- [ ] **Failing tests first** (`edge(...)` builds one line in the `nav dfg-stats --edges` shape):

```python
def test_identical_dumps_pass_with_zero_changes(self):       # exit 0, certificate changes == 0
def test_added_edge_counts_multiplicity(self):
    self.diff([E1], [E1, E1]); self.assertEqual([(c["kind"], c["count"]) for c in self.changes()], [("added", 1)])
def test_label_move_records_both_label_multisets(self):      # base [["exact","-"]], head [["nameonly","killed"]]
def test_byte_only_difference_is_a_change_in_byte_mode_and_not_in_line_mode(self):
    a, b = edge(line=3, sb=10, eb=11), edge(line=3, sb=20, eb=21)
    self.diff([a, a], [a, b], key="bytes"); self.assertEqual(sorted(c["kind"] for c in self.changes()), ["added", "removed"])
    self.diff([a, a], [a, b], key="line"); self.assertEqual(self.changes(), [])
def test_missing_bytes_fall_back_to_line_key_and_are_flagged(self):   # change has "bytes": "absent"
def test_classifier_exception_counts_as_unattributed(self):
    self.assertEqual(self.diff([E1], [], classifier=RAISING).returncode, 1)
    self.assertEqual(self.cert()["unattributed"], 1); self.assertIn("ZeroDivisionError", self.changes()[0]["error"])
def test_classifier_returning_undeclared_class_is_unattributed(self):  # exit 1, error names the class
def test_trace_tags_join_through_lane_parser(self):        # tags == ["kill:alias"]; a stray counts in trace_only
def test_diff_edges_streams_large_gzip_within_memory_bound(self):   # 100,000 edges per side, tracemalloc peak < 96 MiB
def test_diff_stats_lists_every_nested_delta(self):        # "dfg_labels.x: 1 -> 2 (+1)"
def test_diff_stats_exit_1_on_move_outside_allow_list(self):          # "x.*" allows x.a, not y
def test_783_classifier_attributes_kill_alias_and_refuses_untagged_move(self):   # "alias-kill" / None
```

- [ ] RED, implement, GREEN, commit. Port `classify_783_4b.py` from `compare4b.py` (allowed: `exact` → one
      `nameonly` label whose doubt maps to tag `kill:alias` or `alias-d3`) and `classify_660_s1a.py` from
      S1a's `join_trace.py`, each with its `parse_trace`.
- [ ] **G-repro** (measurement runner; `repro/PREDICTION.md` already written): `run` X, EX, T with both M-3
      binaries (head with `--trace-env PRISM_RD_CENSUS_TRACE=1`), then `diff-edges --classifier
      examples/classify_783_4b.py --key line` and `diff-stats`. Pass: each first line equals the recorded
      one byte for byte, `PASS`, every trace tag count equals the recorded one, no counter delta. Then the
      supplementary #660 S1a run: moved counts and mechanism table against `SUMMARY.md`, non-gating.
- [ ] PR body: recorded-versus-kit table per corpus for both targets; formats as built.

### S3: TypeScript oracle and self-check (3 days; G-selfcheck, G-hermetic)

**Files:** create `scripts/measure/oracle/{ts_oracle,bytes,selfcheck}.cjs`, `oracle/questions/{calls,
edges}.cjs`, `oracle/selfcheck/<case>/{repo/,input.jsonl,expected.jsonl}`, `scripts/measure/oracle.py`,
`tests/test_oracle.py`.

```
python3 -m measure oracle ts-calls --run RUN --out DIR [--typescript PATH] [--question FILE] [--aggregate-only]
python3 -m measure oracle ts-edges --changes CHANGES.jsonl --corpus-root DIR --out DIR [--typescript PATH] [--question FILE]
```

TypeScript: `--typescript`, else `PRISM_TYPESCRIPT`, else the `acquire.mjs` install root (as its README
resolves it) + `/typescript/7e02162c902e5c29ec19fefc573ad55b8ed15fcbda940e737f297c53e9c41f54/lib/typescript.js`;
`oracle.py` refuses (exit 3, Node never started) unless its SHA-256 is `3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675`.
The oracle reads source only; offsets are UTF-8 bytes as prism reports them (port `adjudicate.cjs`'s map). Output `verdicts.jsonl` (`key`,
`verdict`, `why`) and `tuples.json` (CORRECT, WRONG, UNDECIDED, INADMISSIBLE, zeros included);
`--aggregate-only` writes `tuples.json` only. **ts-calls:** for a `call_site` with one exact target, the
checker's symbol for the callee at the site bytes is declared inside the target span (CORRECT), in another
in-repo declaration (WRONG), nowhere or outside the repository (UNDECIDED); unmappable bytes or a file
outside the program: INADMISSIBLE. **ts-edges:** for an added or moved edge, both endpoint identifiers
resolve to one symbol (CORRECT), to two (WRONG), unresolved or mixed (UNDECIDED); no bytes or text mismatch:
INADMISSIBLE. A `--question` module exports `grade(row, ctx) -> {verdict, why}`.

- [ ] Author self-check cases first: per question one case per verdict, plus a deliberately wrong edge (an
      inner `let x` use joined to the outer `x` def) and a call record naming the wrong function.
      `selfcheck.cjs` exits 0 only when every verdict equals `expected.jsonl`.
- [ ] Failing Python tests (fake `node` on `PATH`): `test_oracle_refuses_wrong_typescript_hash` (exit 3, node
      log empty), `test_oracle_tuples_include_zero_counts` (`{"CORRECT": 2, "WRONG": 0, "UNDECIDED": 1,
      "INADMISSIBLE": 0}`), `test_oracle_rejects_unknown_verdict_from_question_module` (exit 1).
- [ ] RED, implement, GREEN, commit. **G-selfcheck:** `scripts/memguard/memguard.sh 8 node
      scripts/measure/oracle/selfcheck.cjs --typescript <installed path>` exits 0; output in the PR. One real
      use: `ts-edges` on S2's #660 S1a EX moves; report the tuple.

### S4: `review-diff` and the diff population (2.5 days; G-hermetic)

**Files:** create `scripts/measure/{population,review_diff}.py`, `populations/602-m2-120.json`,
`tests/{test_population,test_review_diff}.py`.

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
--no-cache` through the guard; findings keyed `(file, line, algorithm, category, rules)` (`rules` empty
until lane 7); `added`/`removed` with multiplicity; `changed` = same key, other severity or description.
Outputs `per-diff.jsonl`, `totals.json`, `sample.jsonl` (seeded, added and removed findings).

- [ ] Failing tests (temp repo with parent and commit): `test_spec_matches_m2_rule` (test file, `.md` file
      and deleted file excluded; equals `{"files": [{"file_path": "src/a.ts", "modify_type": "Modified",
      "diff_lines": [3, 4]}]}`), `test_spec_sha_mismatch_refuses_row`, `test_source_clone_is_untouched` (HEAD
      and `status --porcelain` equal before and after), `test_findings_keyed_with_multiplicity_and_changed`,
      `test_sample_is_deterministic_for_a_seed`, `test_population_manifest_has_120_rows_and_no_content` (120
      diffs; no `files` or `diff_lines` key anywhere).
- [ ] RED, implement, GREEN, commit. Check (not a design gate): regenerate all 120 specs from the six local
      full clones; every SHA-256 equals the manifest's; count in the PR. Post the offer on #841; do not wait.

### S5: `gate` and the report skeleton (2 days; G-hermetic)

**Files:** create `scripts/measure/{gate,report}.py`, `examples/gate_783_4b.json`,
`tests/{test_gate,test_report}.py`.

```json
{"schema": "prism.measure.gate/1", "lane": "783-4b", "prediction": "PREDICTION.md",
 "binaries": {"base": {"path": "…", "expect_version": "4f9e3a3e"}, "head": {"path": "…", "expect_version": "1f2ad1cc"}},
 "steps": [{"id": "X-base", "do": "run", "binary": "base", "corpus": "X"},
  {"id": "X-head", "do": "run", "binary": "head", "corpus": "X", "trace_env": ["PRISM_RD_CENSUS_TRACE=1"]},
  {"id": "X-edges", "do": "diff-edges", "base": "X-base", "head": "X-head", "classifier": "c.py", "expect": {"unattributed": 0}}]}
```

`gate FILE --out DIR [--binary NAME=PATH]... [--corpora-root DIR]...` (the overrides keep machine paths out
of committed gate files) exits 2 when the prediction is missing or a reused run's receipt `start` is earlier
than the prediction's modification time; records the prediction's SHA-256 and time; runs steps in order;
writes `gate.json` (per step: pass or fail, numbers) and `REPORT.md` with sections Receipt, Prediction
(verbatim), Population and denominators, Classes and certificate, Verdicts (C/W/U/I with zeros), Controls
and Limits (for the lane to fill), Result.

- [ ] Failing tests: `test_gate_refuses_missing_prediction`; `test_gate_refuses_prediction_newer_than_first_
      measurement` (reused run `start` 10:00Z, prediction touched 11:00Z → exit 2);
      `test_gate_fails_on_threshold_and_names_the_step`; `test_gate_rejects_unknown_step_kind`;
      `test_report_has_the_eight_sections_in_order`; `test_report_quotes_prediction_and_its_sha256`.
- [ ] RED, implement, GREEN, commit. Run `examples/gate_783_4b.json` once (paths supplied at run time); its
      numbers equal S2's.

### S6a: benchmark finding-level scoring (1.5 days; G-findings)

**Files:** create `eval/secbench/{findings,test_findings}.py`; modify `run.py` (keep the parsed taint
channel output instead of discarding it; add `row['finding']`), `replay.py` (score from retained
`taint.json.gz`), `report.py` (findings table beside reach), `README.md`.

`finding_outcome(row, taint_output, invocation) -> {"outcome", "rules"}`: `sink_line` (a `taint_sink`
finding at `sink.file:sink.line`), `sink_file` (same file, other line), `elsewhere`, `none`, `error` (channel
failed or timed out), `not_scored` (no ground truth); `rules` from matching findings' `rules` field (lane 7),
else `[]`. Reach outcomes are neither read nor changed.

- [ ] Failing tests: `test_sink_line_finding`, `test_other_line_same_file`, `test_finding_elsewhere`,
      `test_no_taint_sink_finding` (a `taint_source` finding does not count), `test_channel_error`,
      `test_rules_copied_when_present`, `test_replay_and_live_scoring_agree`,
      `test_reach_summary_unchanged_by_finding_fields` (`summarize()` equal with and without `row['finding']`).
- [ ] RED: `cd eval && uv run --locked pytest secbench/test_findings.py -q`. Implement, GREEN, commit.
- [ ] **G-findings** by replay (rescore, no prism run): raw tree `~/prism-evidence/meas/secbench/repair-r1/
      measured` (SUT `4e592daa`, binary `d603e7cd…`, 373 taint outputs). Confirm it is the observation set
      `repair-r1/final/binding.json` names; replay; require every reach outcome equal to
      `repair-r1/final/entries.jsonl` and a finding outcome on every row with ground truth. PR: table per class.

### S6b: benchmark negatives (3.5 days; G-negatives)

**Files:** create `eval/secbench/{fixed.py,test_fixed.py,locate.mjs,locate.test.mjs,fixed-pins.json}`;
modify `report.py`, `README.md`.

- `fixed.py acquire --inputs META --dest DIR`: `fixedVersion`, stripped of spaces and one leading `=`, must
  match `v?\d+\.\d+\.\d+(-[\w.]+)?` and be newer than the vulnerable version. Read
  `https://registry.npmjs.org/<name>/<version>` (urllib), download `dist.tarball`, verify `dist.integrity`,
  extract (refusing absolute paths, `..`, links, devices) into `<dest>/<class>/<entry>/src/package`; pin
  name, version, SHA-256, integrity in `fixed-pins.json`. Never install or run anything.
- `locate.mjs` (pinned TypeScript): the innermost named function at a file and line; a package's named
  function declarations with spans and normalised text (comments dropped, whitespace runs collapsed).
- `fixed.py score`: sink function by name in the vulnerable version, then the same name in the fixed package;
  the source function likewise, seeded at the same parameter positions as the ground truth; `taint-reaches` frontier from the fixed source plus the review taint
  channel. `still_reported` when a reached occurrence or a `taint_sink` finding lies in the function's span,
  else `not_reported`; crossed with `function_text` `unchanged` or `changed`.
- One reason per unscored entry: `no_fixed_version`, `fix_commit_only`, `fixed_version_unparseable`,
  `fixed_not_newer`, `registry_missing`, `integrity_mismatch`, `extract_refused`, `gt_unavailable`,
  `sink_function_unnamed`, `function_not_found`, `function_ambiguous`, `source_not_found`, `prism_error`.

- [ ] Failing tests: `test_fixed_version_parsing` (`" =1.20.0"` → `"1.20.0"`; `"n/a"` and `">=2.1.1 <3.0.1"`
      → `None`), `test_extraction_refuses_traversal_and_links`, `test_integrity_mismatch_is_a_reason_not_a_
      crash` (injected fetcher, no socket), `test_locate_none_one_many`, `test_unchanged_function_text_is_
      separated`, `test_every_attempted_entry_has_outcome_or_reason`.
- [ ] RED (`uv run --locked pytest secbench/test_fixed.py -q`), implement, GREEN, commit.
- [ ] **G-negatives** (owner decision 2: archives only, into `~/prism-evidence/inputs/secbench-fixed/`):
      acquire every entry with a named `fixedVersion` (278 by the research count), score through the guard with
      the baseline binary; at least 100 entries scored or each with its reason. Report
      `still_reported × {changed, unchanged}` as an upper bound on false alarms, not a rate.

### S7: cloud launchers (2 days; G-hermetic)

**Files:** into `scripts/cloud/` (evidence `tools/` and `cloud-trial/`): `cloud-measure.sh` → `measure.sh`,
`cloud-measure-run.sh` → `measure-run.sh`, `cloud-tests.sh` → `tests.sh`, `cloud-test-run.sh` →
`tests-run.sh`, `cloud-probes.sh` → `probes.sh`, `cloud-gate.sh` → `gate-run.sh`, `push-cloud-input.sh` →
`push-input.sh`; create `kit-recipe.sh` (a RECIPE that runs a kit gate file on the session's base and head
binaries), `README.md`, `test_cloud_scripts.py`; CI step `python3 -m unittest discover -s scripts/cloud -p
'test_*.py'`. Replace `$HOME/code/slicing` with the script checkout's `git rev-parse --show-toplevel` and the
log directory with `${PRISM_CLOUD_LOG_DIR:-${XDG_STATE_HOME:-$HOME/.local/state}/prism-cloud}`.
`kit-recipe.sh` fails loudly on any output file over 95 MB. The evidence copies stay (owner decision 5).

- [ ] Failing tests (stub `git`, `gh`, `claude` on `PATH`): `test_measure_refuses_unpushed_head` (exit 2,
      `claude` never called), `test_assets_need_id_name_and_sha`, `test_kit_recipe_refuses_outputs_over_95_mb`,
      `test_no_machine_paths` (no `/Users/`, `prism-evidence/` or `code/slicing` in any `.sh`).
- [ ] RED, implement, GREEN, commit. One live `measure.sh` run with `kit-recipe.sh` on EX; its gate report
      equals the local one; delete the result and input branches.

### S8: documentation and the manual (1 day)

- [ ] `scripts/measure/README.md`: every subcommand, FORMATS.md folded in, writing a classifier with
      `examples/classify_783_4b.py` as the worked example, the self-check rule, cloud use. `CLAUDE.md`: a
      short "Measurement kit" paragraph under Build & Test with the unittest command.
- [ ] `ORCHESTRATION.md`: section 2 cloud row and section 4 guard path → `scripts/cloud/`,
      `scripts/memguard/memguard.sh`; section 5 item 6 and section 6's measurement brief → "a gate file, a
      prediction and a classifier for `python3 -m measure gate`; no comparator, run recipe, counter diff or
      trace join"; one sentence that this overrides a lane plan's own measurement-tooling task for any lane
      whose stage M starts after this PR merges.

## Close-out (1 day, plus the wait for G-adopt)

- [ ] **G-adopt:** the first of lanes 3, 9 and 10 whose stage M starts after S2 merges (lanes 9 and 10 have
      designs, no plans yet); tell its orchestrator at S2's merge. "Only a classifier file": the files written
      for its stage M differential and stage G are one classifier, at most one oracle question module, gate
      files and predictions; census scripts asking a question the kit does not answer are allowed. Any other
      comparator, recipe, counter or trace-join script fails G-adopt and becomes a kit issue.
- [ ] Issues (filed, not started): rule-aware finding scoring after lane 7; a SARIF reader if #841 takes the
      cross-check; one corpus registry with `scripts/perf/cloud/corpora.json`; `eval/secbench/run.py`'s
      committed machine path and compiler default; each G-adopt gap. Then `HANDOFF.md`, `LEDGER.md`, offload,
      remove the worktree.

## Acceptance for the lane

G-repro, G-selfcheck, G-hermetic, G-findings and G-negatives hold on merged code; G-adopt holds or its gaps
are filed; ORCHESTRATION points lanes at the kit; each slice approved by its seats with CI green.

## Day estimates

| M | D | S1 | S2 | S3 | S4 | S5 | S6a | S6b | S7 | S8 | Close-out | Total |
|---|---|---|---|---|---|---|---|---|---|---|---|---|
| 2 | 1.5 | 3 | 4 | 3 | 2.5 | 2 | 1.5 | 3.5 | 2 | 1 | 1 | **27** (about 22 calendar days with S6 beside S3 to S5) |
