# Canonical links and installed public Program measurement

Third owner-approved increment. Implementation2c885b4, integrated predecessor
closeouts at eb5367a2. No runtime resolver, CPG, navigation or cache source changes.
The existing acquired public application is reused; no install or source/config
rewrite, pruning or link flattening. The original dirty Prism checkout is preserved.

## Outcome

The already-acquired Excalidraw tree now supplies a reproducible configured Program
packet with all249 links admitted. This is acquisition compatibility, not closure
or receiver recall. The independently selected installed/in-root profile produces
`unproven`; validation returns valid:true, packet_status:unproven and authority:false.

| Fixed acquired-tree measure | Result |
|---|---:|
| Regular files / physical directories / links, including compiler lib |59924 / 9073 / 249|
| Regular-file bytes hashed |853674175|
| Configured roots / Program files |591 / 1445|
| Compiler diagnostics |0|
| Annotated observations / nested calls / linked bindings |30 / 53 / 10|
| Observed Props/class records |0|
| Refused virtual-lookup digests / null module targets |264 / 603|

Reasons remain outside_lookup, unresolved_module and unsupported_lookup. Null
targets include ambient/bundler assets and Node module spellings; zero diagnostics
does not turn the observer's filesystem-resolution evidence into closed authority.
No closure barrier was suppressed. The snapshot is stable; dependency, augmentation
and resolution closure remain false.

LibraryMenuHeaderContent.tsx at source hash
c02b2e4d82f6e1916f18a07d5c8abb0666a10bb7b50f559a3fe973379f3bb711:

| Line | Receiver call | Binding | Props/class |
|---|---|---|---|
|155|library.setLibrary|linked|callable_unproven|
|160|library.updateLibrary|linked|callable_unproven|
|184|library.getLatestLibrary|linked|callable_unproven|
|265|library.setLibrary|linked|callable_unproven|

Correction to the earlier source-only explanation: installed React declarations
now supply the contextual call signature in node_modules/@types/react/index.d.ts
(hash51409be337d5cdf32915ace99a4c49bf62dbc124a49135120dfdff73236b0bad,
UTF16/byte34457–34500). The React qualifier's provenance now stops at
ambiguous_declaration, not missing types. Its empty partial alias list must not be
treated as proof that there is no alias. Next recommendation: source/compiler-
backed audit of that qualifier's actual declaration population and ambient/global
module identity, with negative fixtures before any resolution expansion. No
runtime authority or merged-declaration support follows from this measurement.

## Contract and regressions

Explicit links=in-root; default rejection unchanged. Physical regular files and
directories are inventoried once. Relative file/directory links resolve through
captured IDs, bounded to32 traversals. Original target-byte hashes and canonical
targets join the v6 snapshot digest; producer0.7.0 and strict schema reject older
packets. Targets must be in the same mapped root. Leading parent segments work;
embedded dot/parent segments, unsafe/absolute/cross-root/missing targets, link-
resolution cycles and exhaustion fail closed. No linked subtree expansion.

Compiler realpath and enumeration use captured canonical IDs. Compiler options
are not rewritten; preserveSymlinks with links is refused. Distinct Program source
files mapping to one canonical ID are refused, not collapsed into one declaration.
Canonical path identity is not hard-link/inode or runtime class identity. Case
collision, write, duplicate, recomputation and existing lookup barriers remain.

RED on slice2 acquisition:10 tests,4 fail,6 passing refusal controls. After links:
10/10. Pinned-compiler integration additionally checks package alias singleton
symbol identity, defining-source anchors, same-content retargeting, forged targets,
independent policy selection, preserveSymlinks and duplicate relative aliases.
Full observer run108/108 before final self-review; final109/109 after the BOM fix.

WRONG found and corrected in round1: decoding a BOM-prefixed link target with
TextDecoder's default BOM behavior selected a.ts instead of the literal BOM+a.ts.
Captured regression failed1/1. Decoder-vs-path-join probe showed only decoding
removed the character; ignoreBOM:true preserved literal filename bytes. Regression
now passes, as does the non-BOM target population. Round2 found no remaining
demonstrated WRONG; SELF-PASS (NOT INDEPENDENT), no round extension.

The first pinned-compiler comparison used macOS's /var temporary-root alias while
package resolution realpathed to /private/var, creating two symbols. That root-
normalization mismatch was not admissible evidence of package-link identity.
Rerunning with a canonical root, matching observer acquisition, yielded the same
symbol for both package spellings and both integration controls passed.

## Custody and verification

Public source0642e72cfa2d9a71198200e52f37399384610ee3. verify-source.mjs compared
every tracked byte and executable bit before/after; manifests match and original
source stays clean. Acquired source/tools/cache remain at /private/tmp/prism-acquire-w2FtSq.

Same-environment slice2 control, installed profile without new link support:
schema5 unproven/unsupported_input. Final slice3 production plus independent
validation both complete on the unchanged tree. Initial timed production took
23.05s; macOS time -l could not read sysctl in the sandbox, so RSS is UNKNOWN and
that probe supplies no memory-cap evidence. Final post-fix timing was not measured.

Final packet:15688997 bytes, SHA256
0c4acbb619db405af129588cd7fe992945de1c6d93ce03c5a423570752c52a04.
Snapshot SHA25690c7deb2e386c8d01e7228fb8cc300e92f8bb4232f0c4970f43e7073213f064b.
Producer SHA25647e122b154ab46335cbeb5bd6c7da6def57138d143cf8f0068c607528ca1bff5.

Raw task-root evidence: public-packet-final.json, public-validation.json,
public-summary.json, public-predecessor-control.json, source-before.json,
source-after.json, slice3-red.log, slice3-bom-red.log, slice3-integration.log,
slice3-integration-canonical.log, slice3-node-final.log. Final source-head repeats:
default Rust4017 passed,0 failed,1 ignored across28 result groups; MCP4207 passed,
0 failed,1 ignored across30 result groups. Both include2 doctests. Logs:
slice3-cargo-final.log and slice3-mcp-final.log. Observer109 passed,0 failed,
0 skipped. cargo fmt --check and git diff --check pass. Final producer byte hash
was independently rechecked against the reproduced packet after committing.

Evidence archive: /private/tmp/prism-acquisition-next-NoX18k/public-evidence.tgz,
SHA256bce0b82d5a4bedd8b75a56dc9cd39bf0d05b63f8d02757a93c0aa50b6f3168df.
It contains public packet/custody and all three slices' verification logs, not the
installed source/cache/tools. Source, artifacts and isolated worktrees retained.
Publication: PR267 → PR268 → PR269. Owner permits a stack; no merge while CI is
pending. Stacked-base PRs need main-base CI after retargeting before merge.

Replay (outputs must remain outside the audited root):

```sh
node scripts/callable-observations/index.mjs produce "$compiler" "$project" tsconfig.json installed in-root
node scripts/callable-observations/index.mjs validate "$compiler" "$project" tsconfig.json installed in-root < "$packet"
```

Measured Node24.15.0 at /Users/wesleyjinks/.local/share/mise/installs/node/24.15.0/bin/node.
Compiler/profile/target paths match the first-increment readout. No app builds,
private-repo installs, runtime recall claims, OS sandbox proof or Tier-A trigger.
