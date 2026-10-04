# S2 import qualifiers — owner fail-closed packet

The owner's S2-O6 decision is implemented on the existing prototype: a closed lexical whitelist admits direct literal member calls, construction, types and own declaration/export; all other qualifier uses refuse. Refusals apply across visible files and importing/forwarding identities. Class, object and declared-namespace implicit receivers use the same whitelist; unknown receiver ownership refuses module-namespace closure. Populated main results remain byte-identical.

Fresh yield against merged-P2 main is **+132 /+132 /0 /0** on X /installed X /R /T. Every changed row is CORRECT with native module/ownership/full target span agreement. **132 of the former 179 survive**: 25 member-write rows, 11 member-value/chained-read rows and 11 namespace-argument rows keep base. X snapshots are not additive. See [MEASUREMENTS.md](MEASUREMENTS.md) for complete denominators, source witnesses, gates and exclusions.

Start with [SPEC.md](SPEC.md) §0 decisions, then [IMPLEMENTOR.md](IMPLEMENTOR.md). [OQ-s2.md](OQ-s2.md) records the owner and controller positions. [PROBES.md](PROBES.md), [HANDOFF.md](HANDOFF.md), [BUILD-MANIFEST.json](BUILD-MANIFEST.json) and [FILES.md](FILES.md) carry repair history, source/tool bindings, custody and the controller commit set. S2-W1 now keeps base in both grammars. The advisory positional-proof mutant remains a disclosed coverage SMELL; authoritative registry coverage and independent review remain controller gates.

F stays controller-only. Set these privately and use a new evidence directory; return only aggregate stdout:

```bash
CORPUS_F_ROOT='<private F root>' \
PRIVATE_EVIDENCE_ROOT='<new private evidence directory>' \
bash docs/superpowers/plans/2026-10-03-s2-import-qualifiers/CONTROLLER-s2.sh \
  target/s2-plan/bin/main-prism \
  target/s2-plan/bin/head-prism \
  target/s2-plan/bin/head-dump_imports \
  "$HOME/prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js"
```

This runs base/head complete streams, head facts and the native oracle. Every changed row requires matching native module, owner and full target span; any unproven changed row, population drift or loss fails. The fixed aggregate schema includes `head_comparison`, UNJOINABLE and reason counts; source paths, per-site keys, names, configurations and binary/source hashes stay private. The wrapper pins base/head/facts hashes to this packet and refuses an existing output directory. Rebuilt post-commit tools require a manifest rebind and replay before this command.

The controller's inherited F base census is in OQ; it is not a head result. The wrapper was tested on public positive/refusal fixtures only. No planner F read or Git write occurred. Keep immutable main tools, frozen head tools, the final snapshots and receipts before cleanup/removal. Two serial independent review rounds remain; no auto-merge. Tier-A quick is required before review, and full multi-corpus runs remain human-triggered.
