# Post-PR319 native positional-gap plan

**Status:** APPROVED for later controller dispatch; implementation has **not**
started. This plan characterizes native positional gaps at exactly 12 selected
ArrowFunction sites in 11 authenticated files (37,040 bytes). It neither adds
parameter support nor measures call, entry-Def, CPG, DFG, cache, or runtime
behavior.

The immutable source base is PR319 merge `30e13053c9f3f9940b5526e20af0cb40f2a9fae4`
(tree `961f698db1049cec42e7a8e15e716948082af9b6`). PR319 pre-merge and
merge-commit CI are both green. The final planning acceptance is
[FINAL-PLANNING-ACCEPTANCE.md](FINAL-PLANNING-ACCEPTANCE.md), SHA-256
`50a6334484ce80b314e53af19b207821a1ed38d6b28431453ce66f791b91266b`.

## Durable packet

- Normative [SPEC.md](SPEC.md), SHA-256
  `e09fd1463b026bafe36b9b39a6c760163e04dab4aaabfa9ac2065ba5866d87be`.
- Separate [IMPLEMENTOR.md](IMPLEMENTOR.md) and [REVIEWER.md](REVIEWER.md)
  prompts, preserved byte-for-byte.
- Exact [SITE-MANIFEST.json](SITE-MANIFEST.json): 12 sites, 11 files, 37,040
  bytes; SHA-256 `789352a575d68ef672de7449299676de0c0aab8edd4abd8228e23efda65d326b`.
- Passing characterization, not RED: [BASELINE-FIXTURES.json](BASELINE-FIXTURES.json),
  [BASELINE-RECEIPT.json](BASELINE-RECEIPT.json),
  [BASELINE-PROBE.md](BASELINE-PROBE.md), [baseline-probe.rs](baseline-probe.rs),
  and [baseline-output.log](baseline-output.log).
- Planning review closure: [INDEPENDENT-REVIEW-round1.md](INDEPENDENT-REVIEW-round1.md),
  [ROUND1-CORRECTIONS.md](ROUND1-CORRECTIONS.md), and the final acceptance.

[COPY-MAP.md](COPY-MAP.md) binds copied file hashes to the external reviewed
packet. The two artifact manifests are retained as external-packet provenance.
The external packet's compiled `baseline-probe`, `baseline-build.log`, and
other raw build output are intentionally not copied; their paths and hashes
remain in `BASELINE-RECEIPT.json` and `artifact-manifest.sha256`.

Implementation requires a separate controller dispatch, a frozen source/test
manifest, and the declared two-round implementation review cap. Public
execution is not authorized by this planning acceptance.
