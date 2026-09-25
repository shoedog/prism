# Native positional-gap observer — resume advice (Fable, 2026-09-24)

**Recommendation: (b), re-plan to a controller-run pinned probe. Do not fold D5–D8.**

## Why not (a)

The WRONG trend across five rounds (6, 3, 1, 1, 3) and the r4→r5 regression (`ownedStage` fd-open-at-exec
ETXTBSY, introduced *by a custody fold*) say the open class is the launcher's custody layer, not the observation.
Every remaining blocker except D7 lives in `index.mjs`: D5 (ETXTBSY) is launcher-only, D6 (frozen-manifest
binding) is launcher-only, D8 (read-before-size) is launcher-only. The launcher (513 lines) plus its tests (608)
now outweigh the worker (716) and duplicate validation the worker already performs on the whole request before
parsing (`decode`, `safe_path`, `hash`, selector bounds, UTF-8 boundaries). Folding D5–D8 spends a fresh cap
hardening the surface that keeps producing findings; the evidence says r6 finds the next custody edge.

## Value of the 12-site answer

Modest but real, and cheap once the custody layer is gone: it decides "bounded entry/call proof next" versus
"defer" for the only positional-gap cohort PR #319 found (12 of 355 object-pattern sites, all arrows, 11 files,
37 KB). It is a one-shot research readout, not a product feature. That argues for the smallest sound harness, not
the most defended one — the inputs are pinned by hash, run once, and reconciled by an independent reviewer.

## Minimal sound version (b)

Keep the worker exactly as it stands (already validates the full request, canonical output, 30 baseline cases,
inherited mutants killed). Replace the launcher with a **request builder** and let the controller run the pipe:

1. `scripts/parameter-slot-characterization/request.mjs --manifest <path> --manifest-sha256 <frozen>` → stdout:
   verify the manifest hash equals the frozen `789352a5…` (D6 becomes two lines: the builder refuses any other hash
   unless `upstream_repository` starts with `synthetic://`), read each selected file relative to `--root`, strict
   UTF-8, hash, emit the exact `prism.native-parameter-request/1` object. ~60 lines. No staging, no atomic
   new-only link, no fd ownership, no timeouts, no binary hashing (D5, D8 disappear with the code that had them).
2. Controller runs, twice, cold:
   `node request.mjs … | ./target/debug/examples/parameter_slot_characterization > out-N.json`, records
   `shasum -a 256` of the binary, both outputs, and `git diff --stat f79bb954 HEAD -- src Cargo.toml Cargo.lock`
   (empty) in the receipt by hand. Output identity = byte-equal `out-1.json`/`out-2.json`.
3. Independent reconciliation (sol, static): every site row against the pinned manifest, status partition sums to
   12, eligibility rule re-derived from the row's own fields — this is SPEC §7 step 3 unchanged.
4. D7's four mutants are worker-logic (`eligibility` step 3 all/any and ordering, singular `parameter`): add four
   `#[cfg(test)]` rows to the example (~30 lines). That is the only worker change.

Budget: builder ≤ 80, builder tests ≤ 120 (manifest-hash refusal, synthetic pass-through, symlink/absolute-path
refusal, strict-UTF-8 refusal, request byte equality against a fixture), worker cfg(test) +30. Review: one slice,
two rounds, on the **delta only** (the worker's observation logic was reviewed five times and its findings are
closed; re-reviewing it invites re-litigation). Total new code ≈ 230 lines against the 1,121 launcher+test lines
deleted.

## What to keep from `feat/native-positional-gap`

- `examples/parameter_slot_characterization.rs` unchanged except the D7 rows.
- `SITE-MANIFEST.json`, `BASELINE-FIXTURES.json`, `BASELINE-RECEIPT.json`, and the preserved 11 public inputs at
  `~/prism-evidence/native-positional-gap/public-inputs-selected/`.
- From `index.test.mjs`: test 9's raw-worker refusal matrix and canonical-reordering rows (they exercise the
  worker through stdin and survive the launcher's removal); drop tests 1–8's launcher custody rows.
- `PARKED.md` and the r1–r5 review record as the reason the custody layer was removed.

If the owner values the readout below roughly a day of controller time, the third option is to leave it parked:
the inputs are preserved and hashed, and nothing shipped depends on it.
