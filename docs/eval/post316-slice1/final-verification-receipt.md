# Slice 1 final verification receipt

## Frozen candidate

- Commit: `07ed5ceb6098f2b5a21f5c05008a2c823a5617d7`
- Tree: `32be14a0eebecd8b936f94c289f21faeb0526716`
- Manifest: `source-manifest.md`; all ten source/test hashes matched before final gates.
- Independent cap-two review and final evidence reconciliation: APPROVE, 0 WRONG, 1 nonblocking performance SMELL, no source-change request. Records: `evidence/reviewer-round2/REVIEW-round2.md` and `evidence/reviewer-round2/FINAL-ACCEPTANCE.md`.
- Publication state: local only; no push, PR, merge, CI, rebaseline, adoption, or launch.

## Behavioral evidence

- The final eight-test ownership matrix passes 8/8. Its patch on unchanged base passes the two preservation controls and fails the six intended ownership cases, establishing fail-first attribution.
- The final navigation test passes 1/1; the same patch on unchanged base fails 0/1.
- All 16 candidate-only failures from the first repaired default suite were reproduced as 16/16 passes on unchanged base, then mapped exactly once to a bounded regression repair or an independently justified ownership-oracle update in `round1-repair-receipt.md`.
- Independent reviewer probes pass: focused ownership 8/8, navigation 1/1, cache/parity 2/2, and receiver-owner hardening 36/36.

## Final gate results

| Population | Result |
|---|---|
| Rust default | 29 suites; 4,518 passed / 0 failed / 1 ignored |
| Rust with `mcp` | 31 suites; 4,711 passed / 0 failed / 1 ignored |
| Rust with `mcp detached-owner-audit` | 31 suites; 4,734 passed / 0 failed / 1 ignored |
| Widest-feature examples | 4 suites; 32 passed / 0 failed / 0 ignored |
| Clippy / formatting / diff | clippy exit 0 with 179 warnings (134 duplicates); format and diff checks clean |
| Grammar reproduction | PASS; unchanged baselines and shipped TS/TSX parser trees reproduce byte-for-byte |
| Active Node population | 46 modules, 786 tests: 785 passed / 0 failed / 1 intentional grammar-archive skip |
| Standalone callable authority | 40 results / 0 failures using pinned TypeScript 5.9.3 and restored profiles |
| Python deterministic and adoption unit | 937 passed / 3 initially skipped; the exact three real-binary cases then passed 3/3 against a frozen binary pair, closing 940 collected cases |
| Tier-A matrix | 159/159 rows `ok` against frozen `prism` SHA-256 `947acb87e7f24ad1d64bed834d381794d717753be6fc52bdb91ee0f6d3f69810` |
| Tier-A quick | INVALID with preserved report; no accuracy or regression verdict |

Rust logs and hashes are in `evidence/rust-final/rust-gates-receipt.md`. Node, authority, Python, grammar and Tier-A logs, frozen-binary hashes, and report hashes are in `evidence/nonrust-final/nonrust-gates-receipt.md`.

## Invalid attempts and limitations

- The first widest Rust invocation omitted the explicit compiler/profile environment. All 23 `NotPresent` refusals became passes on the single diagnosed retry; the invalid log and pre-retry hypothesis are preserved.
- The first broad Node invocation omitted the freshly built native membership helper. Its one failure occurred before the test body; the single diagnosed rerun passed, and the remaining active modules passed 53/53 in a supplement.
- Tier-A quick reached the frozen candidate and emitted a report, but the result is invalid for two independent reasons: corpus SHA policy drift (`07ed5ceb6098` versus pinned `20c8490591a3`) and incomplete C-method oracle probes (4/6). It was not retried or rebaselined. A same-environment base run would retain the pin-policy mismatch and would not discriminate a candidate defect.

## Explicit exclusions and skips

- Three tests in `docs/eval/receiver-closure/audit-imported-props-source.test.mjs` were excluded because the required historical `real-sites.jsonl` custody input is absent; no replacement fixture was created.
- One Node grammar self-test intentionally skipped its archive-tamper branch because `PRISM_GRAMMAR_ARCHIVES` was not supplied. The separate restored-input grammar reproduction gate passed.
- `eval/adoption/tests/test_prism_adoption.py` was excluded because live adoption was not authorized and `PRISM_RUN_LIVE_EVALS=1` was not set.
- Full Tier-A `--corpus all` is human-triggered and was not run.

No admitted unexpected behavior failure remained, so no additional same-environment base attribution run was needed after the preserved controls above.
