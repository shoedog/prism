# WP2 timing protocol results (spec §3.3)

Protocol: P1 = `cargo clean && /usr/bin/time -l cargo test`;
P2 = `touch src/lib.rs && /usr/bin/time -l cargo test`, run immediately after P1.
Single timed pair per point, machine otherwise idle (M-series, 15 cores, 24 GB).
Gate: P2 < 8 min (G7).

| Point | P1 real | P1 user | P1 sys | P1 maxRSS | P2 real | P2 user | P2 sys | P2 maxRSS |
|---|---|---|---|---|---|---|---|---|
| pre-WP2 (tier-a @ b740b24) | 119.5 s | 167.9 s | 33.8 s | 1.22 GB | 108.8 s | 107.9 s | 33.6 s | 0.44 GB |
| post-3.1 consolidation (@ 1a8287d) | 47.9 s | 120.6 s | 17.6 s | 1.20 GB | 37.4 s | 66.9 s | 19.3 s | 0.44 GB |
| post-3.1+3.2 profile (@ cda4d6a) | 46.6 s | 111.1 s | 17.3 s | 0.88 GB | 37.1 s | 64.8 s | 17.6 s | 0.44 GB |

## Outcome (gate G7: **PASS** — P2 = 37.1 s vs < 8 min)

- **P1 −61%** (119.5 → 46.6 s), **P2 −66%** (108.8 → 37.1 s), end to end.
- Attribution: nearly all of it is **3.1 consolidation**, and the dominant mechanism is
  one the plan did *not* predict — **test-execution parallelism**, not link time.
  121 tiny test binaries run serially under `cargo test` (~103 s of execution at
  baseline); 24 fat binaries let libtest's thread-per-core scheduling bite (~35 s).
  The predicted link/compile win is real but secondary: clean build 16.35 → 11.47 s,
  lib-touch rebuild 2.0–2.3 s.
- **3.2 (`line-tables-only`)** contributes ~1 s of wall but **−27% peak RSS on clean
  builds** (1.20 → 0.88 GB) and smaller debug artifacts. Backtrace probe confirms
  file:line preserved (`panicked at tests/infra/main.rs:4:5`).
- Protocol caveats, recorded: `cargo clean`/`rm -rf target` race with the ambient
  rust-analyzer (it recreates metadata mid-delete) — P1 legs were validated by their
  full-rebuild signatures (`Finished` + user-time coherence) rather than by a
  pristine empty `target/`; rust-analyzer was equally ambient at all three points
  including baseline, so comparability holds.
- The container-verify benefit showed up during WP2 itself: each a2a verify leg builds
  24 targets instead of 121 on the Linux VM.

## Finding: the 21-minute premise does not reproduce (recorded, not buried)

The motivating observation — "full `cargo test` ≈ 21 min, compile-dominated"
(`docs/archive/plans/prism-query-layer/s1-followups.md` item 4) — is **unreproducible on a healthy machine**. Measured
breakdown at the pre-WP2 point: clean **build = 16.35 s** (cargo `Finished` line, 101
crates, all 121 test-binary links included), with the remaining ~103 s being execution
of 3,802 tests; the lib-touch loop is ~9 s rebuild/relink + execution.

Probable cause of the original number: it was observed during the S1 session in the
window when a wedged macOS `spindump` was parking every process launch at
`_dyld_start` (documented in `docs/archive/plans/prism-query-layer/s1-followups.md` item 5). 121 test binaries plus several
hundred rustc/linker launches, each delayed seconds, inflates 2 minutes to ~21. The
followups item is corrected alongside this file.

**Consequences for WP2:** G7 (P2 < 8 min) is pre-met at baseline; it will still be
reported at all three protocol points. WP2's measured value shifts to (a) the P1/P2
deltas from 121→24 links and 121→24 `tests/common` compiles, (b) test-execution wall
(24 binaries change libtest scheduling), and (c) **containerized verify wall** — every
a2a-bridge task verify rebuilds from clean on a slower Linux VM, 14 more times in WP1
alone — recorded per task as the batches land. Consolidation proceeds: the structural
case (one target per directory, uniform filters) and the container/CI case stand
independently of the dev-loop emergency that turned out not to exist.
