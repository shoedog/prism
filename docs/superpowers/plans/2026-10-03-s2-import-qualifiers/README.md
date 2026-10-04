# S2 — measurement-only checkpoint

[ASSUMPTION] Recommend parking the current S2 slice unless the controller's F
census shows material permitted precision gain. The public corpus measurements
and their limits are in [MEASUREMENTS.md](MEASUREMENTS.md). Dropped callables are
an opportunity count; they cannot be recovered by a rule that never adds a
target. No blanket refusal is proposed.

[READ] Start at [OQ-s2.md](OQ-s2.md) for the two owner-decision placeholders and
[HANDOFF.md](HANDOFF.md) for live custody. [PROBES.md](PROBES.md) records hypotheses,
instrument corrections, exclusions and the supplied-reference mismatch.

[READ] This packet contains a native measurement instrument and its controls,
not a product prototype. Product source and cache formats are unchanged. There
is no implementation dispatch, product lane registry or new Tier-A fixture at
this checkpoint; those are conditional on material permitted yield in the owner
brief. The controller performs Git writes and any subsequent model dispatch.

[READ] Reproduce the public census from the retained current-main binaries:

```bash
python3 docs/superpowers/plans/2026-10-03-s2-import-qualifiers/probes/public.py \
  --out target/s2-plan/replay-NEW
```

[READ] If the retained executables have been removed, rebuild the measurement
tools offline in this checkout before using that command:

```bash
CARGO_TARGET_DIR=target/s2-plan/build cargo build --offline --release --bin prism
python3 docs/superpowers/plans/2026-10-03-s2-import-qualifiers/probes/build-facts.py
```

[READ] The recorded manifest pins this run's binary bytes. A rebuild with a
different compiler can change those hashes; rebind deliberately and repeat the
source-bound controls rather than bypassing a manifest failure. The supplied
`~/prism-evidence/paths/ref-binaries/main-prism` is not the current S2 base on X.

[READ] F is controller-only. Set both environment values privately, use a **new**
evidence directory, and return only stdout's JSON object:

```bash
CORPUS_F_ROOT='<private F root>' \
PRIVATE_EVIDENCE_ROOT='<new private evidence directory>' \
bash docs/superpowers/plans/2026-10-03-s2-import-qualifiers/CONTROLLER-s2.sh \
  target/s2-plan/bin/main-prism \
  target/s2-plan/bin/main-dump_imports \
  "$HOME/prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js"
```

[READ] [FILES.md](FILES.md) lists the controller's commit set and suggested
message. Evidence and runnable binary custody remain under `target/s2-plan/`.
