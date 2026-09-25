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
  - `R5-P1` object-shape `all`→`any`: input `function take({x}, wrong, later){return later;}`, object `[0,1]`, later `[2]`. Must
    give `selection_native_shape_mismatch`.
  - `R5-P2` later-shape `all`→`any`: input `function take({x}, good, bad = 1){return good;}`, object `[0]`, later `[1,2]`. Must
    defer.
  - `R5-P3` order directions `&&`→`||`: input `function take(early, {x}, late){return late;}`, object `[1]`, later `[0,2]`. Must
    defer.
  - `R5-O1` singular `parameter` dropped: input `const take = later => later;`. Expect one raw parameter at ordinal 0,
    with the slot at `source_ordinal:0`.
- Each row asserts the complete site record, including packet action and reason. `[r1]` The sources above are
  exact, complete fixtures. P1–P3 must additionally assert `parse_error_count:0`, `status:"unique_named"`, and
  `selection_native_shape_mismatch`, so that recovery can never mask the predicate. After implementation, re-run the
  four r5 mutants and the inherited 29. The Rust rows may use the existing
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
  - `[r2]` **Explicit amendment of SPEC §5.** SPEC §5 requires a synthetic manifest to follow the full public
    manifest schema. For synthetic manifests only, that clause is amended to **request projection**: every field the
    builder copies into the request must be valid, and manifest-only metadata may be absent or extra. Two reasons:
    synthetic manifests are test fixtures, and the public manifest is bound byte-for-byte by the frozen hash (D6),
    which covers every public field. terra flagged the contradiction in v2 WRONG 1; sol r2 judged the relaxation
    deliberate and approved. Making it explicit resolves both.
- **CLI (`[r1]`).** Each of the four flags is required exactly once. A missing, duplicate, or unknown flag refuses.
- **Sources.** For each member, first `lstat` `<root>/<path>`. `[r1]` Refuse if it is not a regular file, if its
  size differs from the declared byte count, or if it would exceed the remaining 256 KiB total. This closes D8
  cheaply, with no custody loop. Then read `<root>/<path>` with a relative, safe path: no absolute paths, no `..`, no
  backslash, no control characters. Then:
  - require byte length and SHA-256 equal to the manifest
  - decode as strict UTF-8, preserving the BOM
  - emit files and selectors in the SPEC §5 normative key and insertion order, with files in code-point path order
    and selectors in numeric order
- **`native_binary_sha256`** comes from `--native-sha256`, which the controller computes. The builder does not read
  or execute the binary.
- There is no child process, staging, publication, timeout, or file write. **D5 disappears with the launcher. D8 is closed by the pre-read `lstat` check above.**

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
  - `[r2]` CLI table: a missing, duplicate, and unknown flag each refuse with a flag-specific message.
  - `[r2]` D8 table: a non-regular source (a directory) refuses, and so does a source whose actual size differs from
    the declared size. The latter must be asserted on the **pre-read** size-specific diagnostic, distinct from the
    later hash/length refusal.
  - Invalid UTF-8 refuses.
  - The request's bytes are exactly equal to a fixture: key order, BOM preserved.
- Test 9, the raw-worker refusal matrix, stays unchanged. It exercises the worker's own pre-parse validation, which
  now carries the checks the launcher used to duplicate.

## 5. Budget (honest executable lines; strict attribution: only `#[cfg(test)]` Rust is tests)

| Bucket | Contents | Cap | Forecast |
|---|---|---|---|
| helper | worker non-test (573, unchanged) + `request.mjs` | ≤ 720 `[r1]` | 573 + ~75–105 |
| tests | Rust `#[cfg(test)]` (135 + ~30 for D7) + `request.test.mjs` | ≤ 760 `[r1]` | ~620–700 |
| combined | helper + tests | ≤ 1,480 `[r1]` | ~1,300–1,380 |

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

## 7. Spec review record

Round 1 (fresh cap):

- sol: FIX, 2 WRONG / 2 SMELL
- terra: FIX, 1 WRONG / 0 SMELL

Both call it converging. Folds, marked `[r1]`:

| Finding | Source | Fold |
|---|---|---|
| D7 P1–P3 sources lacked bodies, so parse recovery masked the predicates | sol W1, terra W1 | Exact complete fixtures, plus clean-parse and shape-mismatch assertions |
| Helper forecast crossed the 95% stop (and the test margin was only 3 lines) | sol W2, terra note | Caps set before implementation: 720 / 760 / 1,480 |
| D8 read-before-size remained in the builder | sol S1 | `lstat` regular-file, size, and cap check before reading |
| Handoff not reconciled with option (b) | sol S2 | Handoff refreshed |
| CLI flag exactness unstated | sol table note | Four flags, each exactly once; unknown flags refuse |

Round 2:

- sol: **APPROVE**, 0 WRONG / 1 SMELL
- terra: FIX, 1 WRONG / 0 SMELL

| Finding | Source | Fold |
|---|---|---|
| Synthetic manifest schema relaxation contradicts SPEC §5 | terra v2 W1 (sol: deliberate) | §3.2: explicit, bounded amendment of SPEC §5 for synthetic manifests only |
| CLI and D8 branches lack regression rows | sol S1 | §4: CLI table and D8 pre-read tables |

The spec is approved, with sol gating. Implementation is authorized, with its own 2-round review cap on the delta.

## 8. Implementation budget amendment (owner-approved 2026-09-24)

The builder was trimmed to exactly the §3.2 contract; the duplicated worker validation was removed after the first
stop. The trimmed builder measures 158 honest lines under the counting rule, which counts braces and one-field-per-line
literals. The controller's 70–105 forecast was wrong. The owner chose **"Raise helper cap to 760"**:

| Bucket | Cap | 95% stop |
|---|---|---|
| helper | ≤ 760 | 722 |
| tests | ≤ 760 | unchanged |
| combined | ≤ 1,520 | — |

The 158-line draft is preserved at `~/prism-evidence/native-positional-gap/request-draft-158.mjs`; the 173-line
original is also kept there. The launcher's 507 lines are still deleted, so this remains a net reduction.

## 9. Builder/worker boundary (owner-approved 2026-09-24, after implementation review r2)

The implementation review r2 reports were sol FIX 2W/1S and terra APPROVE. Sol found the builder in an "unstable
middle": it validated some copied scalars but no relational invariants, and several of those scalar checks were
untested. The owner chose **"Worker owns schema"**. This supersedes the `[r2]` §3.2 wording "every field the builder
copies into the request must be valid".

- **Builder owns:**
  - exact CLI parsing, including a 64-lowercase-hex `--native-sha256`
  - manifest-byte identity and the D6 frozen-hash rule
  - the projection shape: `members` and `sites` arrays of objects that carry the copied keys
  - safe relative member paths and duplicate member paths (needed to open files)
  - site-to-member association (needed to nest sites)
  - the D8 pre-read `lstat`, regular-file, declared-size, and 256 KiB checks
  - source length and SHA-256 identity
  - strict UTF-8 with the BOM preserved
  - canonical emission order
- **Worker owns all request-schema validation, scalar and relational,** before any parse. This covers:
  - enums and the extension → `script_kind` mapping
  - safe integers
  - span bounds and UTF-8 boundaries
  - strictly increasing ordinals
  - selector tuple uniqueness
  - member and selector closure

  It already refuses every case sol's r2 listed.

For an invalid synthetic projection, the contract is **builder success followed by worker refusal**, with no
successful packet. A pipe-level test pins that. The builder's duplicated scalar checks are removed. The public run
is unaffected, because its manifest is bound byte-for-byte by the frozen hash.

Also folded: sol r2 W2. The group-7 `no_selected_suffix_binding_gap` packet **is** reachable through the real binary,
via a typed, escaped object binding: `function take({\u0078}: {x:number}, later: string){return later;}`. That
source becomes a complete group-7 fixture record, and the earlier inventory N/A for it is withdrawn.
