# Bounded acquisition: three approved increments

Owner approval: “merged. next 3 slices approved. merge when green or stack”.
Base: PR266 merge d4f06b58b11ed09f46a80f769a94764dafa5ddf0.

The measured public installed tree exceeds the default count/byte ceilings and
contains 249 links. It has not yet supplied a configured Program observation.
Acquisition compatibility is separate from closure and runtime authority.

1. Stream every regular file into a SHA256 inventory using 64KiB chunks. Retain
   IDs, sizes, hashes and directory membership, not all file bytes. Compiler reads
   load only inventoried files and verify their size/hash before returning bytes.
   Repeat the full inventory after analysis. Keep default limits, link refusal,
   packet shape and both false authority flags. Include the acquisition module
   in the producer digest so stale packets cannot reproduce across the change.
2. Add a caller-selected installed-tree profile, recorded and validated in the
   packet. Default limits do not change. Explicitly bound inventory count/bytes,
   individual materialized compiler input, packet size and worker time. Caller
   limits may only lower the selected profile. Unknown profiles/raised limits
   reject before root I/O. Do not infer profile selection from packet content.
3. Add opt-in canonical in-root link identity. Inventory physical entries once;
   fingerprint link spellings and resolved IDs. Resolve against the captured graph,
   never expand linked directory subtrees. Refuse dangling links, escapes, cycles,
   excluded metadata, cross-root links and exhausted traversal budgets. Recheck
   link identity on recomputation. Keep actual compiler options; unsupported
   preserveSymlinks semantics must fail closed, not silently be rewritten.
   Then replay only the already-acquired public disposable tree, without installs,
   package pruning, source/config changes, link flattening or private acquisition.

## Verification and stop rules

Each implementation needs captured RED against its predecessor, a positive plus
negative/edge per new path, all observer tests and full default/MCP Rust suites.
Runtime/CPG/nav are out of scope; touching them triggers Tier-A gates. Two self-
review rounds per increment, NOT INDEPENDENT. Classify remaining findings at cap.
Only merge green current heads; dependent increments may stack pending checks.

Negative requirements: same-size byte replacement, removal/restoration, file-to-
link replacement, unused file and directory changes, count/byte/depth exhaustion,
unknown/forged profiles, over-limit single compiler inputs, link retargeting with
identical content, directory/file aliases sharing defining identity, missing
targets, chains/cycles/escapes/.git traversal and case collisions.

## Non-goals and trust

No runtime resolver/cache changes, React-name heuristics, plugin/project-script
execution or class/Exact authority. Default closure barriers remain. “Observed”
and “valid” are not runtime proof. Quiescent, trusted roots are mandatory: open-file
metadata checks, hash verification and two inventories detect observed changes,
not malicious change-and-revert races or an atomic snapshot. Worker old-space
limits are not RSS limits; packet/output and input budgets are independent.
