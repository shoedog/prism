# 2026-10-31 nav-gap evidence

This directory holds the measurements behind the four nav-gap specs in `docs/superpowers/specs/2026-10-31-nav-gaps/` (01 find-symbol, 02 composite explore, 04 statement dependence, 06 ranked repo map; the specs' own `00-README.md` reconciles them) and behind the JS/TS/React/Node tracing-coverage issues #604 and #594 to #603. Everything was measured with the release binary `slicing 3.2.0 (533f061a942a)` on main `533f061a`.

| Directory | What it supports |
|---|---|
| `gap-analysis/` | The agent-facing capability-gap analysis (`ANALYSIS.md`: eight gaps, ranked, with the thresholds that specs 04 and 06 cite) and its probe log (`PROBES.md`: each probe's command and key numbers, cited by specs 01, 02 and 06). |
| `coverage/` | The JS/TS tracing-coverage pass behind issue #604 (the plan lives in the issue) and the findings filed as #594 to #603: `COVERAGE.md` (with a probe-index appendix and the recipe for the review-CLI diff specs used by #602), the 13 hand-written fixtures its probes ran on (`fixtures/`), and `coverage_calc.py`, which reproduces the 46% CVE and 26% defect traceable-weight figures. |
| `spec-pilots/04-gate-pilot/` | Stage A receipts for spec 04 (the offline ceiling pilot): `gate_pilot.py`, `pilot-report.json`, binary hash, corpus SHAs and commits in its `README.md`. |
| `spec-pilots/pilot-06-01/` | The scripts, seed lists and recorded results (`RESULTS.md`, with a crosswalk from each spec table row to its output row) behind the pilot tables in specs 06 and 01. `RUN.txt` says how to regenerate the inputs. |

## Private-store dependencies

Some inputs are not in this repository and are not published:

- **Raw probe outputs** (the `probes/` files named in `gap-analysis/PROBES.md` and the Appendix of `coverage/COVERAGE.md`, and the 30 raw CLI outputs of the gate pilot) are kept in a private evidence store. The probe files keep each command and its key numbers instead.
- **The tracing-trial inputs**: the 24 trial question texts, gold v2 (the gold units and the gold-file lists derived from them) and the trial harness (`match.py`, `evidence/directed-results.py`). The gate pilot's committed report therefore carries per-question coverage counts, ceilings and verdicts but no gold units, and the focused-ranking rows of `pilot-06-01/RESULTS.md` (`ppr_focus`, `ppr_focus_bidir`, `focused_2k`, `focused_4k`) can be re-derived only with the store.
- **Corpus checkouts** pinned to the SHAs in the pilot READMEs (httpx, ripgrep, cobra, excalidraw; click, flask, gin, axum, express for the gap-analysis probes).
- "COMPARATIVE" in `gap-analysis/ANALYSIS.md` and `PROBES.md` is the tracing-trial comparative write-up in the private store; "internal lane notes (private)" are the owner's lane notes, not published.

## Environment variables for the scripts

Scripts that need the store or the checkouts stop with a one-line message when a required variable is unset or points at nothing. Run them from a scratch directory and write outputs there, so the committed receipts (`pilot-report.json`, `RESULTS.md`) are never overwritten.

| Variable | Used by | Meaning |
|---|---|---|
| `PRISM_TRIAL_DIR` | `gate_pilot.py`, `goldfiles.py`, `rank2.py`, `atbudget.py` | private tracing-trial directory (`questions.jsonl`, `gold/v2/`, harness) |
| `PRISM_GOLD_DIR` | `gate_pilot.py`, `goldfiles.py` | optional, default `$PRISM_TRIAL_DIR/gold/v2` |
| `PRISM_BENCH_REPOS` | `gate_pilot.py`, `findsim2.py` | directory holding the pinned corpus checkouts |
| `PRISM_GOLDFILES` | `rank.py`, `rank2.py`, `rank3.py`, `atbudget.py` | the `goldfiles.json` that `goldfiles.py <out-dir>` writes; private, never commit it |
| `PRISM_PILOT_INPUTS` | `rank*.py`, `budget.py`, `atbudget.py`, `findsim2.py` | directory with the regenerated `rm-*.json`, `cs-*.jsonl`, `fn-*.json` (default: current directory) |
| `PRISM_PILOT_UNITS` | `gate_pilot.py` | optional, `1` writes covered gold-unit ids into the output (keep it private) |
| `PRISM_OUT_DIR` | `rank.py` | optional, directory for `per_question.json` |

`coverage/coverage_calc.py` needs nothing (`python3 -I coverage_calc.py`). With the store and checkouts in place, the pilot-06-01 scripts and `gate_pilot.py` were re-run on 2026-10-31 and reproduced `RESULTS.md` (every section) and the counts in `pilot-report.json` exactly.

## How to read the claims

- The "pre-registered" thresholds are stated to have been written before the probes ran. The only witness is the modification time of the private `ANALYSIS.md` (2026-10-31 00:47), which is a file timestamp in the private store and not a git record. It is the author's account, not an audited pre-registration. The gate pilot's T1 follow-up rules were written after exploratory probes, as its README discloses.
- Statements about third-party tools (GitNexus, codegraph, serena, Sourcegraph and others) in `gap-analysis/ANALYSIS.md` come from a private tool survey and trial write-ups. They were not re-verified.
- `coverage/fixtures/` contains deliberately vulnerable example code (for example `exec('ls ' + req.query.dir)`). It is test input for probes, not application code.
- Nothing under this directory is executed by CI (`docs/*` is docs-only), so the scripts are not kept green automatically.
