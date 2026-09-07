# Streamed inventory verification

Base PR266/d4f06b58. Implementation0ea180d4. Opt-in research observer only;
no Rust/runtime/cache/nav changes and no new dependency acquisition.

Captured RED on an extraction of the pre-change eager snapshot:5 tests,2 pass,
3 fail (retained Buffer, no same-size lazy-read revalidation, no removal/link
replacement revalidation). The eager immutable buffer behavior was not an old
wrong-output finding; these regressions specify the new lazy access contract.
An initial large Buffer diff stalled the test reporter; that probe was stopped
and excluded. The corrected discriminator-first run is the admissible RED.

Implementation streams full inventory hashes with64KiB chunks and retains only
metadata. Every materialized compiler/anchor read verifies captured hash and size;
the final full inventory preserves unused-file and directory invalidation. The
producer digest includes inventory.mjs. Default ceilings and symlink refusal are
unchanged. No atomic snapshot or hostile filesystem safety is claimed.

Fresh gates, same isolated worktree and environment:

| Gate | Result |
|---|---|
| All observer tests, explicit pinned compiler/profiles |90 passed,0 failed,0 skipped|
| Full cargo test --offline |4017 passed,0 failed,1 ignored, including2 doctests|
| Full cargo test --offline --features mcp |4207 passed,0 failed,1 ignored, including2 doctests|
| Authority fixture verifier |failures=[]|
| Authority/source-audit tests |4 passed,0 failed|
| cargo fmt --check / git diff --check |pass|

The first broad observer run was89/90: the existing producer digest test listed
the old six modules. Its same-environment predecessor control passed1/1; updating
the explicit list to include the seventh module restored90/90. No test expectation
for resolution, provenance or authority was changed. Two self-review rounds,
SELF-PASS (NOT INDEPENDENT), no remaining demonstrated WRONG. Full Tier-A was not
triggered: no call-resolution/navigation/CPG source changed.

Raw logs retained outside the repository at
/private/tmp/prism-acquisition-next-NoX18k/: slice1-red-fixed.log,
slice1-node.log, slice1-node-final.log, slice1-cargo.log, slice1-mcp.log,
authority.log and authority-tests.log. Exact test invocation uses
PRISM_TYPESCRIPT=/private/tmp/prism-imported-alias-O4d6E1/package/lib/typescript.js
and PRISM_CALLABLE_PROFILES=/private/tmp/prism-callable-authority-98TLLN/public/profiles;
Rust uses CARGO_TARGET_DIR=/Users/wesleyjinks/code/slicing/target.
