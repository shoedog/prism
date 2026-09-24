# Re-plan (b): native worker + request builder + controller-run pipe (2026-09-24)

**Status:** spec amendment for independent review, with a fresh 2-round spec cap. On APPROVE it enters implementation
with its own 2-round review cap, reviewing the **delta only**.

**Owner decision (2026-09-24):** "Re-plan per Fable (b)". This follows the second park, recorded in PARKED.md (r5
FIX 3 WRONG, open-class). The advisor's reasoning is in `~/prism-evidence/native-positional-gap/OBSERVER-ADVICE-fable.md`.

**What stays binding from SPEC.md:**

- §1–§3: the decision, the immutable inputs, and the unchanged seams.
- §5: the worker request schema, the output schema, the status and eligibility predicates, and the reason strings.
- §6: every **observation** control group (1–4, 6, 7).
- §7 step 3: independent row reconciliation.

This amendment replaces only the launcher-custody layer in §4–§5: the launcher CLI, binary custody, staging, and
new-only publication.

## 1. Why

- **The findings come from the launcher.** WRONG findings per round went 6, 3, 1, 1, 3, and every r5 blocker except
  D7 lives in the 513-line launcher `index.mjs`:
  - D5: ETXTBSY, a regression introduced by the r4 custody fold
  - D6: public-manifest binding
  - D8: read before size check
- **The launcher duplicates the worker's checks.** Before any parse, the worker already validates the whole request
  it receives on stdin: exact keys, safe paths and integers, source hashes, selector bounds, UTF-8 boundaries, and
  script-kind mapping.
- **The run is one-shot.** It is a single research readout over 12 pinned sites, reconciled by an independent
  reviewer. The smallest sound harness is the right one.

## 2. Trust boundary (explicit)

The operator's own account on their own machine is the trust boundary. The inputs are pinned by hash, and the
controller runs the pipe and records custody by hand.

Out of scope, and not to be reintroduced by review:

- a hostile same-account process (symlink or TOCTOU races, binary swaps)
- executable staging
- atomic new-only publication
- child timeouts and output caps

A finding that asks for this machinery is answered by citing this section. If a reviewer shows the threat is inside
the boundary, that is an owner-level spec change.

## 3. Components

### 3.1 Worker: `examples/parameter_slot_characterization.rs`

- **Unchanged**, apart from D7. D7 adds `#[cfg(test)]` predicate and ordinal rows that kill sol r5's four surviving
  mutants:
  - `R5-P1` object-shape `all`→`any`: input `function take({x}, wrong, later)`, object `[0,1]`, later `[2]`. Must
    give `selection_native_shape_mismatch`.
  - `R5-P2` later-shape `all`→`any`: input `function take({x}, good, bad = 1)`, object `[0]`, later `[1,2]`. Must
    defer.
  - `R5-P3` order directions `&&`→`||`: input `function take(early, {x}, late)`, object `[1]`, later `[0,2]`. Must
    defer.
  - `R5-O1` singular `parameter` dropped: input `const take = later => later;`. Expect one raw parameter at ordinal 0,
    with the slot at `source_ordinal:0`.
- Each row asserts the complete site record, including packet action and reason. The Rust rows may use the existing
  `observe` entry point on a constructed request.

### 3.2 Builder: `scripts/parameter-slot-characterization/request.mjs` (replaces `index.mjs`)

CLI:

```sh
node request.mjs --root <dir> --manifest <file> --manifest-sha256 <hex> --native-sha256 <hex>
```

It writes the exact `prism.native-parameter-request/1` JSON plus a newline to stdout, and exits nonzero with a
message on stderr for any refusal.

- **D6 — frozen-manifest binding.** Read the manifest bytes and require SHA-256 equal to `--manifest-sha256`.
  - If `upstream_repository` does **not** start with `synthetic://`, also require the hash to equal the frozen
    `789352a575d68ef672de7449299676de0c0aab8edd4abd8228e23efda65d326b`. Refuse otherwise.
  - Synthetic manifests may use any small population. Their schema is validated only as far as needed to build the
    request; the worker re-validates the whole request before parsing.
- **Sources.** For each member, read `<root>/<path>` with a relative, safe path: no absolute paths, no `..`, no
  backslash, no control characters. Then:
  - require byte length and SHA-256 equal to the manifest
  - decode as strict UTF-8, preserving the BOM
  - emit files and selectors in the SPEC §5 normative key and insertion order, with files in code-point path order
    and selectors in numeric order
- **`native_binary_sha256`** comes from `--native-sha256`, which the controller computes. The builder does not read
  or execute the binary.
- There is no child process, staging, publication, timeout, or file write. **D5 and D8 disappear with the launcher.**

### 3.3 Controller-run pipe (acceptance procedure, not code)

1. Build `cargo build --offline --locked --example parameter_slot_characterization` from the frozen commit. Record
   the binary's SHA-256 and confirm `git diff --stat 30e13053 HEAD -- src Cargo.toml Cargo.lock build.rs` is empty.
2. Run twice, from cold:

   ```sh
   node request.mjs --root <public root> --manifest SITE-MANIFEST.json \
     --manifest-sha256 789352a5… --native-sha256 <bin> \
     | <bin> > out-N.json
   ```

   Record both exit statuses; each must be 0. `out-1.json` and `out-2.json` must be byte-equal.
3. Independent reconciliation (SPEC §7 step 3, unchanged) is a static review: every site row is checked against the
   pinned manifest, the status partition must sum to 12, and eligibility is re-derived from each row's own fields.
4. Durable evidence goes to `docs/eval/native-positional-gap/{site-manifest.json,receipt.json,readout.md}`. The
   receipt records the binary hash, the manifest hash, the builder SHA-256, the output SHA-256, and the exact
   commands.

## 4. Tests: `index.test.mjs` → `request.test.mjs`

- **Keep** every observation assertion in tests 1–4, 3b, 6, 7, 9, and 10: complete records, statuses, eligibility,
  the RED adapter, and the 30-case baseline replay. Re-route them through the builder and worker:
  - build a request with `request.mjs`'s exported builder function
  - pipe it to the worker binary via `spawnSync`, with `PRISM_NATIVE_PARAMETER_EXAMPLE` as today
  - assert the complete packet

  The RED adapter and its admissibility are preserved: `PRISM_CAPTURE_RED=1` fails only on alias `source_ordinal`
  1 vs 2.
- **Drop** the launcher-custody rows:
  - test 5's publication and child-custody rows
  - test 8, which covers binary staging, limits, and wire staging
  - the stage and collision controls

  Keep test 5's manifest-validation rows only where they are builder behavior: the manifest-hash mismatch, and
  unsafe and duplicate member paths.
- **Add builder controls:**
  - A non-synthetic manifest whose hash is not the frozen one refuses. This is D6, and it covers both of sol's r5
    probes: a non-frozen hash, and a non-arrow `compiler_kind`.
  - A synthetic manifest passes.
  - An absolute path, `..`, and a backslash each refuse.
  - A source-hash mismatch refuses.
  - Invalid UTF-8 refuses.
  - The request's bytes are exactly equal to a fixture: key order, BOM preserved.
- Test 9, the raw-worker refusal matrix, stays unchanged. It exercises the worker's own pre-parse validation, which
  now carries the checks the launcher used to duplicate.

## 5. Budget (honest executable lines; strict attribution: only `#[cfg(test)]` Rust is tests)

| Bucket | Contents | Cap | Forecast |
|---|---|---|---|
| helper | worker non-test (573, unchanged) + `request.mjs` | ≤ 700 | 573 + ~70–100 |
| tests | Rust `#[cfg(test)]` (135 + ~30 for D7) + `request.test.mjs` | ≤ 740, the current cap, unchanged | ~620–700 |
| combined | helper + tests | ≤ 1,440 | ~1,300–1,370 |

Early stop at 95%. The launcher's 507 helper lines are deleted; roughly 110–150 custody test lines are deleted and
about 60–80 builder test lines added. A breach is a stop and returns to the owner.

## 6. Implementation review (delta only)

- The review covers `request.mjs`, `request.test.mjs`, and the D7 rows. The worker's observation logic is closed by
  review rounds r1–r5 and is not re-reviewed, except where the D7 rows touch it.
- Two rounds, using the owner's brief shape: a recommended fix plus options for each finding, and a per-finding
  "when this would not apply" critique, from a senior/principal long-term stance.

**STOP conditions:**

- any `src/` or Cargo change
- reintroducing launcher custody machinery
- a public run before implementation approval
- a budget breach
