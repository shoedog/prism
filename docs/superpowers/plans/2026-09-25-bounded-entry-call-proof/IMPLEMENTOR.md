# Implementer brief: bounded entry/call proof observer (option B)

You are implementing **exactly** `SPEC.md` in this directory. Read it fully first, then `PLANNING-PROBES.md`. Do this
only if the owner selected option B (SPEC §0 D1) and confirmed the D2 thresholds. If the dispatch does not state both,
stop and ask.

## Ground rules

- **Clone.** Use the dispatch's implementation branch, cut from the planning commit that contains this packet. Record
  the base SHA.
- **Owned paths only:**
  - `examples/entry_call_proof/main.rs`
  - `examples/entry_call_proof/fixtures/**`
  - `docs/eval/entry-call-proof/targets.json` (byte copy of this packet's `targets.json`; verify SHA-256 `b584bb4a…`)

  The public `observation.json`, `receipt.json`, and `readout.md` are written by the controller after approval, not
  by you.
- **Forbidden:**
  - any change under `src/`, or to `Cargo.toml`, `Cargo.lock`, `build.rs`, `scripts/`, or caches
  - new dependencies
  - network access
  - running the observer on the public Excalidraw tree. That is acceptance, controller-only, and happens after review
    approval.
- **Trust boundary (SPEC §4).** Build no custody machinery: no staging, self-hashing, timeouts, publication, or
  symlink or TOCTOU guards. Stdout JSON, stderr errors, exit 2.
- **The skeleton is a starting point, not authority.** `skeleton/main.rs.txt` compiled and ran on synthetic fixtures
  during planning. You own every line you keep. Where the skeleton and the SPEC differ, the SPEC wins. Known gaps:
  - resolve-once indexing
  - `resolved_targets_at_site`
  - pure `reconcile` and `prefix_only_blocked` functions
  - `decide(yields, M, U)`, with `U` making the `identity` fixture `inconclusive`

## Order of work (TDD)

1. **Check fixture loading.** Move `skeleton/fixtures/*` to `examples/entry_call_proof/fixtures/*`. Confirm that
   `load_repo` on each in-repo fixture root loads exactly its files. If the parent `.gitignore` adds or drops files,
   relocate the fixtures and report.
2. **Characterize.** Using the skeleton shape, record each fixture's actual observations. Then hand-derive each
   `expected.json` complete record, justifying every value from the source text and the SPEC §3 seams. Characterization
   is **not** RED.
3. **Write the tests** in SPEC §9.2, including the `PRISM_CAPTURE_RED` switch on `calls_fixture_complete_record`.
4. **Capture the behavioral RED.** Run `PRISM_CAPTURE_RED=1 cargo test --offline --example entry_call_proof
   calls_fixture_complete_record`. It must fail on `sites[bare].entry.status "ambiguous" ≠ "present"`. Save the log
   excerpt. Setup, compile, missing-file, or zero-test failures are inadmissible.
5. **Implement to SPEC §6–§8.** Get the tests green. Then run every mutant M1–M10 in SPEC §9.3, one at a time, and
   record each as killed along with the test that killed it. Revert each mutant after running it.
6. **Budget.** Run `cargo fmt`. Then count non-blank, non-`//` lines, with the `#[cfg(test)]` module counted as
   tests. The caps are **helper ≤ 800, tests ≤ 290, combined ≤ 1,090**. The early-stop thresholds are 760 and 275.
   Lines must be ≤ 100 columns except unsplittable literals, and each such literal must be listed. If a forecast or
   measured count would breach a cap, **stop** and return the count plus the enumerated remaining items. Do not
   compress logic to fit.
7. **Verify.** Report totals from the logs of each of these:
   - `cargo fmt --all -- --check`
   - `cargo clippy --offline --all-targets --features mcp -- -W clippy::all`
   - `cargo test --offline --example entry_call_proof`
   - `cargo test --offline --no-fail-fast`. The default suite must stay at 4,559 / 0 / 1; the example's tests do not
     run in it.
   - `node scripts/gate-inputs/acquire.mjs && node scripts/gate-inputs/gate.mjs --out target/gate-runs/<name>`

   Report any out-of-scope failure as found; do not fix or re-baseline it.
8. **Commit** on the branch. Attribution trailers follow the session's standing rule. Do not push or merge.

## Handback

Include all of the following:

- **Files and SHAs.** The file manifest, base and head SHAs, and
  `git diff --stat origin/main -- src Cargo.toml Cargo.lock build.rs` (which must be empty).
- **Budget.** Measured line counts per bucket, the counting method, and any over-100-column literals. Fixture data
  sizes.
- **RED.** The RED command and its failing excerpt, showing the concrete value.
- **Mutants.** The M1–M10 kill table.
- **Suites.** Own and full-suite totals, with log paths.
- **Unreachable arms.** Branches that no natural fixture reaches (see SPEC §9.2), and how the pure tables cover them.
- **Deviations.** Any deviation from the SPEC, with its reason. Each is a question for the owner, not a decision you
  make yourself.
