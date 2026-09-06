# Callable-authority foundation: observations, not runtime edges

Branch `design/callable-authority-proof`, base PR258 merge835c4fbc.
Implementation2272351 pushed; [PR259](https://github.com/shoedog/prism/pull/259)
merged2026-09-06T22:16:10Z as ebf7933bc3ac7fea1c3201eb48ca84611ea9a702;
merge tree equals published a5169697. Gates below are historical, not a merge rerun.
The approved successor is feat/configured-callable-observations; no runtime consumer.
Design, compiler characterization, regression guards and read-only source audit
only. No production change, dependency/CLI addition, cache bump or Exact-edge gain.
The contract and next-slice boundary are in
`docs/superpowers/specs/2026-09-06-callable-authority-proof.md`.

## What the evidence changes

| Construct | Compiler observation in both profiles | Consequence |
|---|---|---|
| Direct imported/namespace/renamed FC or FunctionComponent annotation | One external contextual signature; body method belongs to Client | Candidate evidence only; program closure and Prism ownership still required |
| Explicit `any` parameter | Outer contextual signature exists; body receiver is any | Contextual signature alone must not override an explicit annotation |
| Explicit decoy class parameter | Body method belongs to Decoy | Existing explicit route remains independently authoritative |
| Local/namespace homonym or paths-substituted react | Different declaration/owner despite FC spelling | Resolve actual declarations; no spelling-based React authority |
| Added callable overload | Zero diagnostics, two signatures, Client/Other union method owners | Clean compilation does not establish a unique owner |
| Added static FC member | Zero diagnostics and unchanged body owner | Even harmless-looking merging must be inventoried; not silently ignored by a bounded no-merging policy |
| Assertion/post-declaration satisfies | No contextual signature on the original function; implicit-any diagnostic | Consumer typing does not retroactively annotate implementation |
| Imported JS function assertion with checkJs disabled | Zero diagnostics; original receiver any, no contextual signature | No implementation proof from a diagnostic-free consumer cast |
| Optional/any/structural props | Optional class symbol, no owner, or interface method respectively | Symbol-at-call is not sufficient class/receiver authority |

The20 synthetic cases are shared by compiler and Prism guards. Compiler:40/40
observations (two profiles). Prism:80 combinations (20×TS/TSX×full/subset), packaged
as one integration test;18 no-Exact guards and two independent source-backed
positive controls. The current runtime intentionally passes these guards on base;
they are not captured RED for a production feature or a measured recall improvement.
UTF-16↔UTF-8 extraction is checked, including emoji and CRLF before the call.

## Dependency custody and reproduction

Public package tarballs were fetched without npm install or lifecycle scripts;
SHA512 integrity was checked against registry metadata / pinned public lock data.
Profiles contain complete extracted packages, not hand-written React stubs:

| Profile | Packages under profile/node_modules | SHA256 of sorted relative-path/source tree |
|---|---|---|
| react19 | @types/react19.0.10, csstype3.1.3 | `49c6c7a3cde29161a5af224dede5e4442295f9251ef4c694699341ed3682baad` |
| react18 | @types/react18.3.31, csstype3.2.3, @types/prop-types15.7.15 | `7b8bbdc844cd38cbf691987a229858e4006273c3d3b3c4d8c8fef70267859b34` |

Pinned compiler TypeScript5.9.3, typescript.js SHA256:
`3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675`.
The harness asserts compiler version, records its byte digest, asserts profile
versions AND fixed tree hashes, and refuses dependency symlinks. It permits only
virtual fixture files and the supplied compiler's lib directory; ambient host
node_modules cannot satisfy missing dependencies. It does not build a real app's
configured program or certify a future proof-cache key.

To reproduce, unpack the named npm tarballs into the profile layout above, without
installation/scripts; keep all package files because the full tree is pinned.
The archive retains tarballs, metadata, profiles and compiler files. Pass absolute
paths (the Node tests deliberately fail rather than skip if inputs are absent):

```sh
node docs/eval/receiver-closure/verify-callable-authority.mjs "$compiler" "$profiles"
PRISM_TYPESCRIPT="$compiler" PRISM_CALLABLE_PROFILES="$profiles" node --test docs/eval/receiver-closure/audit-callable-source.test.mjs docs/eval/receiver-closure/verify-callable-authority.test.mjs
cargo test --test integration callable_authority_audit
cargo test
cargo test --features mcp
```

`compiler` names package/lib/typescript.js; `profiles` contains react18/ and
react19/. These are task-specific shell variables, not environment overrides of
system paths. The source-census tests create and remove their own isolated Git
fixtures; on this host their Git writes require up-front sandbox escalation.

## Real public source and limits

Rechecked Excalidraw source blobs at0642e72cfa2d9a71198200e52f37399384610ee3.
The four remaining FC receiver spans are in
packages/excalidraw/components/LibraryMenuHeaderContent.tsx at155/160/184/265;
the annotation is at38 and its library property/import at41/30. Two other spans
at297/307 come from useApp/useContext, a separate producer and still out of scope.
Pinned React19 declarations show a props-P callable signature; the React18 profile
also has an optional legacy-context parameter. No generic version-independent
signature shape is presumed. Public package sources:
[React19.0.10 metadata](https://registry.npmjs.org/@types/react/19.0.10),
[React18.3.31 metadata](https://registry.npmjs.org/@types/react/18.3.31).

The initial audit reused PR258's2780 raw records/376 with an Exact target after
verifying its frozen48dfc45 binary's src/Cargo inputs equal base835c4fbc. A fresh
replay with a new cache directory is byte-identical over all2780 records. Source
anchors and tracked blob membership were rechecked against both dumps this turn.
No source-census or isolated-fixture result claims whole-application TypeScript/
dependency/augmentation closure.

Prism structural navigation reported a stale-index warning; LSP tools were absent.
Source anchors and the pinned compiler supplied the semantic fallback. This is
not an LSP-backed proof or a compiler-integrated production feature.

## Review, gates and custody

Three SELF-PASS rounds, NOT INDEPENDENT, cap3. Round1 established the complete
fixture population and froze the overload characterization. Round2 found and
fixed two WRONG items in the new audit tools: a tracked configuration symlink was
read despite the source-link fence; modified declaration bytes were accepted when
package versions stayed unchanged. Both have captured failing behavioral output
and passing regression output. Round3 completed design/evidence consistency,
private evidence separation and final gates. No open in-scope WRONG remains;
no inherited runtime WRONG was downgraded.

The census is explicitly a trusted-worktree, tracked-source syntax helper, not
complete semantic alias resolution or a secure immutable-program producer. This
limitation is a SMELL if used beyond its documented scope, not new runtime authority.
No application installation, script execution or edits are part of the audit.

Final default full suite:4017 passed/0 failed/1 existing ignored,28 summary groups;
MCP4207/0/1,30 groups. Both include two doctests. The existing ignored case is
`resolution_test::slice_elem_variant_reserved`. Builds emit test-code warnings;
no warning-clean or fresh Clippy claim is made.
Node helper regressions:4/4; compiler40/40; fmt/diff checks pass. No src/ changes:
Tier-A runtime gates not triggered. Prior quick remains INHERITED baseline-invalid
(SHA drift/C-name4/6/oracle2/30/SUT0), not a freshly green gate. No full multicorpus,
rebaseline, application tests or whole-application compiler check were run.

Evidence root: `/private/tmp/prism-callable-authority-98TLLN/public`.
Only this public subtree and scoped source checkpoints may enter the public archive.
Owner-provided private audit details are separately retained locally, never part
of this readout, the public archive or PR text.
Public archive: `/private/tmp/prism-callable-authority-public-evidence.tgz`, SHA256
`d443c2e3b217cdfee5a8737d08c6b62dd436feb082cc0ef6398450a4e1bdae41`.
It includes2272351 source custody and prior checkpoints, full gate/RED/GREEN logs,
public declaration profiles/compiler/binary and source replay; it predates this
docs-only publication closeout. Private evidence is not included.
