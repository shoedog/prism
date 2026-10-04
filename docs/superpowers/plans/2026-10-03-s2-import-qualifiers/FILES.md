# Controller commit set

[READ] Commit all files in this new packet as one measurement-only plan commit:

```text
docs(plan): measure S2 import qualifiers and gate dispatch on permitted yield
```

[READ] Packet files:

- `README.md`
- `MEASUREMENTS.md`
- `OQ-s2.md`
- `HANDOFF.md`
- `PROBES.md`
- `FILES.md`
- `CONTROLLER-s2.sh`
- `probes/census.cjs`
- `probes/controls.py`
- `probes/public.py`
- `probes/dump_imports.rs`
- `probes/build-facts.py`
- `probes/reference-binaries.json`
- `probes/supplied-reference-binaries.json`

[READ] No prototype commit is proposed. No tracked product file, Cargo manifest,
lockfile, test baseline, cache pin or mutation registry is changed. No Git write,
push, merge, review or model implementation dispatch was performed by the
planner. The review cap remains 2, with 0 rounds dispatched here.

[READ] Preserve `target/s2-plan/evidence.tar.gz`, `packet-snapshot.tar.gz`,
`evidence-index.json` and `bin/` in controller custody before removing this
worktree. The archives are local snapshots, not a claim of a Git commit or an
off-machine backup. `bin/` holds source-bound executables needed for F; the
cargo build directory has been removed after verification.
