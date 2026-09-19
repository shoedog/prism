# Post-design final non-Rust and Tier-A receipt

## Binding and custody

- Candidate: `14e86083c2c754ed412d440742bd5763be388e42`; tree `a0411db6d858fe34806376ce41d2621853848d8b`.
- Source manifest: `docs/eval/post317-exact-caller-reads/source-manifest.md`, SHA-256 `310b5152d598800a7520181fe53be58cb3677d3ca7421cc900c8005e78e92aa7`.
- Final worktree check after copying/removing this lane's generated quick reports, snapshot, and temporary venv: clean (`git status --short` has 0 lines).
- Evidence root: `/private/tmp/prism-post317-implementation/final-gates/nonrust`.
- Frozen release pair: `bin/release/prism` SHA-256 `66682efbecbcd929ff4099de972f8c8c004c1ee02d37a731ae11338933937347`; `bin/release/prism-mcp` SHA-256 `8dbeec6e8c1ce66d7bba22a0eb58a4c6e2322016eb1cfb6d92f7c5e18cad63bf`.
- Frozen native helper: `bin/native/project_membership_census` SHA-256 `a6da854e34e4165fb7163a92b6b943b7e3f2e807d999549bbe2277e3b0dbbd0a`.
- Public TypeScript 5.9.3 compiler SHA-256: `3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675`.

## Results

| Gate | Command artifact | Result |
|---|---|---|
| Callable authority | `logs/authority.log` | PASS: 40 declared profile results, no failures. |
| TypeScript grammar | `logs/grammar.log` | PASS: shipped parser trees reproduce both pinned baselines byte-for-byte. |
| Node active population | `logs/node-full.log` | PASS: 785 passed, 0 failed, 1 skipped (786 total). |
| Python deterministic | `logs/python-tests-final.log` | PASS: 896 passed. |
| Python adoption units | `logs/python-adoption-unit-final.log` | PASS: 44 passed. |
| Tier-A matrix | `logs/tier-a-matrix.log` | PASS: 159/159 `ok`; fresh preceding `target-matrix` release build, hashes in `bin/matrix/sha256.txt`. |
| Tier-A quick | `reports/quick-report.{json,md}` | Terminal **INVALID**, described below; no retry or rebaseline. |

R07 cache/cost is separately PASS in `/private/tmp/prism-post317-implementation/r07/final-r07-receipt.md` (SHA-256 `00628af266e4aa4b19ab70887145c1fbb38718fdcdb50d833f2b9a8f4eab65d7`). Rust final gates are separately recorded at `/private/tmp/prism-post317-implementation/post-design-final-rust-receipt.md` (SHA-256 `4cb6ab550932b7ff9c1206267f99dd12a8cb7563a179fd39246c59050039bcf7`).

## Python setup repair custody

The first configured external-venv invocation was **INVALID before collection**: the venv had been recreated without `pytest`. The next collection was also **INVALID** (14 collection errors) because the controlled `--no-install-project` repair intentionally omitted local `prism-eval`; its adoption unit command separately passed 44. The allowed convergent final repair installed only local `prism-eval` from the unchanged locked source with `uv sync --locked --offline`, then the final preflight verified CPython 3.12.13, pytest 9.0.3, `tier_a`/`tools` imports, frozen binary paths, and candidate cwd. The final full suites above are the only behavioral Python results. Raw setup logs: `logs/python-venv-repair.log`, `logs/python-final-preflight.log`, `logs/python-tests.log`, and `logs/python-tests-retry.log`.

## Quick terminal status and custody

The quick command ran from the held, clean candidate worktree with explicit `--sut-bin bin/quick/prism`, an immediately preceding isolated `target-quick` release build, and date `2026-09-19-post317-14e86083-quick`. It completed a report in roughly four minutes, not by timeout. Its report sets `baseline_invalid: true` because:

1. `corpus_sha_drift: 14e86083c2c7 != pinned 20c8490591a3`;
2. `stratum C-method: 4/6 successful probes`.

Therefore the quick report is inadmissible for accuracy, regression, or flip claims. Its SUT error rate of 0.0 is not a clean accuracy result. External copies are `reports/quick-run.json` and `reports/quick-report.{json,md}` (run/report JSON SHA-256 `e909333b769d02028312e38d824b98e59fc7f391b039a29c9df33c12c66e25e3`; Markdown SHA-256 `801aa602c03a08f694717d08260823e139deb5d2b548bb6e86b7310232d95b20`; snapshot SHA-256 `fad3e8e40c6f2e0f63d0320a98096879f9d74ecd5155aa4a8b61f993cd09c2c3`). The same uniquely dated in-worktree files and the lane-generated venv were removed only after this external copy.

## Exclusions

- `scripts/audit-imported-props-source.test.mjs`: excluded because all five provenance inputs, including historical `real-sites.jsonl`, are absent; no reconstruction attempted.
- `eval/adoption/tests/test_prism_adoption.py`: excluded because live adoption requires `PRISM_RUN_LIVE_EVALS=1`, which was not authorized.
- `tier-a --corpus all`: excluded because full multi-corpus execution is human-triggered.

All raw-log hashes are in `log-hashes.sha256`; executable hashes are in `binary-hashes.sha256`; quick artifact hashes are in `reports/quick-artifacts.sha256`.
