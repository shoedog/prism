# Lane P P1 r5 owned files and proposed commits

The repairer made no Git writes. Controller commits the existing dirty artifact on `feat/tsconfig-paths-p1` at `5aa3055218b3be250ce3fef742c494f8f7a5b5a8`, after binding the final receipts in [REPAIR-R5-RESULTS.md](REPAIR-R5-RESULTS.md). Rebind any later revision before transferring a verification claim.

Proposed source/test commit:

```text
fix(paths): match native source admission and bound parallel reads

Decode ambient sources with pinned TypeScript BOM/lossy rules, preserve
lexical source names and reference bases, and scan explicitly loaded
excluded sources, including hidden-name include globs. Reserve aggregate
stat sizes before parallel reads and detect growth within reservations.
Add both-grammar and negative controls plus a roots-sensitive memo shield.
```

| File | Purpose |
|---|---|
| `src/js_paths_boundary.rs` | Native decoding; source aliases; loading hints and excluded-directory patterns; pre-read byte reservation; decoder/budget/alias units |
| `src/js_paths_snapshot.rs` | Supplementary loaded-input scan, transient excluded-path inventory, lexical-reference type closure, two-root memo regression |
| `tests/integration/js_paths_r3_test.rs` | Three new both-grammar tests covering five encodings, lexical links/references and seven excluded-input activation modes |

Proposed documentation/controller commit:

```text
docs(paths): bind P1 r5 verification and controller binary
```

| File | Purpose |
|---|---|
| `docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/BUILD-MANIFEST.md` | Current source/binary binding; earlier receipts historical |
| `docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/MEASUREMENTS.md` | Current gate pointer; earlier measurements historical |
| `docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/HANDOFF.md` | Current operational handoff pointer |
| `docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/SPEC.md` | Owner-directed decoding/loading/budget fold; accepted cost preserved |
| `docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/probes/CONTROLLER-paths.sh` | Defaults to retained r5 binary; syntax check only |
| `docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/REPAIR-R5-RESULTS.md` | Every final gate, repeat, mutant and unverified limit |
| `docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/REPAIR-R5-HANDOFF.md` | Eight-section live handoff and custody/open work |
| `docs/superpowers/plans/2026-10-01-tsconfig-paths-resolution/REPAIR-R5-FILES.md` | This exact 11-file Git inventory and proposed commits |

`VERIFICATION.md` is a local summary ignored by `.git/info/exclude:7`; it is retained in the 12-file local snapshot and is outside the proposed 11-file Git inventory.

Evidence stays local in `target/repair-r5`: frozen source, owned archive/diff, retained binary, source/receipt bindings, lean receipt archive and reproducible scripts/manifests. Build directories are excluded from the evidence bundle and removed after their producers end. No external-backup or publication claim.
