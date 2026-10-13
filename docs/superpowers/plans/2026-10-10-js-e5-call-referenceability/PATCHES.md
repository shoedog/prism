# Delivery and custody

Base:40ad8390d6cb5544a063346033c7bb1817ad3855. The source and docs patches are under `/Users/wesleyjinks/prism-evidence/js-e5/step0/` and use ordinary git-diff a/b paths. No commit, index write, push, or merge was performed; the controller owns Git custody.

E5-src.patch contains the three runtime files, two cache mutant pins, five new regression groups and reconciled existing control fixtures. Source patch SHA256: `b21fd2a2f7309e3d0a05c52d1c49a6e267e5dd6f9f08e448ca62243ec727506f`. It applies to the clean base archive with `git apply --check`.

E5-docs.patch contains this packet, full census/samples/measurements, draft spec, handoff, controller script, reusable probes, validation and replay instructions, plus the repository-root VERIFICATION.md required by the stop hook. Its final hash is recorded outside the patch in `step0/patch-manifest.json` to avoid circular hashes. Both patches are checked against a clean base archive. Temporary Rust examples were archived and removed; their reproducible sources remain in packet/probes.

`step0/final-product-binding.json` freezes product/test/pin files and immutable binaries; `step0/final-packet-binding.json` freezes packet bytes, root verification, and patch hashes. `step0/E5-final-source-packet.tar.gz` preserves every changed/new source, packet file, and root verification. The previous delivery is preserved in verification-hook-predecessor. Earlier stable-point checkpoints and superseded/inadmissible probe receipts remain available. Raw public graph/site/native details are retained under step0/base, head, call-comparison, graphs and variants; private F is absent.

This is an owner-reviewable Step0 proposal, not a landed change. The controller should make durable Git custody, run the private paired command in CONTROLLER.md, and resolve the rule breadth/drop/D12 decisions before adoption.
