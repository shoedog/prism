# Program-closure lookup dispositions: audit and successor requirements

Status: source-backed classification after merged PR271, base2abb5de0. No
production observer/schema/runtime change or permission to clear closure barriers.

## Settled distinctions

The filesystem resolver's null target, the checker's module symbol, and complete
Program closure are separate facts. The fixed Program has603 null filesystem
requests:588 bind exact/wildcard ambient symbols; one addresses an augmentation's
own declaration name;14 have no source symbol. The augmentation-name symbol does
not satisfy that same file's missing import.315 wildcard asset requests have
inventory files in this fixed tree, but fixtures show wildcard binding itself
does not establish asset existence.84 of those bindings merge two declarations.

The14 residuals remain real closure gaps under the actual Node10 configuration:
8 optional-peer requests,2 dev-only references,2 exports-only Rollup subpaths,
1 missing relative declaration and1 obsolete workspace subpath. A diagnostic-only
Bundler resolver locates the two Rollup declarations; it does not authorize an
application config rewrite or producer mode substitution. Optional metadata and
skipLibCheck do not turn the remaining gaps into complete declaration inputs.

The264 refusal digests denote distinct normalized probe paths, not distinct
modules:258 Node-prefix paths plus6 virtual-PWA paths.806 outside directory
requests comprise772 module-ancestor searches,28 type-root searches and6 canonical
workspace peer-ID metadata probes. None is a successful read or direct import
target in this capture. That distinction does not prove outside candidates absent
or remove the need for an acquisition/closure contract.

## Source/compiler authority

Pinned TypeScript5.9.3 SHA256
`3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675`:

- resolveExternalModule,54023–54030: source ambient lookup precedes the filesystem
  resolvedModule branch; tryFindAmbientModule,64013–64020, returns merged globals.
- resolveExternalModule,54132–54140: wildcard matching returns merged pattern or
  augmentation symbols; a pattern declaration is not a physical-asset proof.
- primaryLookup,44407–44413: type-root directory probes.
- module lookup,46309–46334: ancestor-directory enumeration.
- readPackageJsonPeerDependencies,45681–45706: derives a peer directory using
  lastIndexOf("node_modules") on the canonical package directory. For a workspace
  path without that segment, the fixed virtual root yields /__prism__//react and
  react-dom. The dedicated fixture reproduces this metadata probe while the
  actual relative package import resolves successfully.

Actual compiler callbacks were temporarily instrumented, then the normal packet
was frozen before lazy checker classification. Capture+normal packet equality
excludes only the temporary producer hash. All603 request occurrences reconcile
as a multiset, every use/declaration anchor matches hash/range/kind and captured
Program membership, and all264 refusal digests reconcile to normalized requests.
The instrumented production file was restored byte-for-byte before final gates.

## Proposed next bounded slice — observation provenance, not closure admission

1. Add explicit source-backed lookup dispositions to observation packets instead
   of interpreting target:null as a missing-module verdict. Keep filesystem
   results separate; freeze a strict versioned schema and pre-I/O negatives.
2. Start with singleton exact ambient modules only. Require the actual requested
   literal, source/context kind, exact checker binding, canonical declaration
   anchors, whole configured-Program provider census and duplicate/augmentation
   barriers. An exact name or node:/virtual: prefix alone is never evidence.
3. Distinguish imported/type/require requests from augmentation declaration names
   and synthetic compiler-generated requests. Never invent positions for negative-
   position JSX runtime literals. No source declaration means unproven.
4. Keep wildcard modules, merged declarations and general module augmentation
   unproven in this first slice. Retain their candidate anchors where appropriate,
   not inferred file existence or class authority.
5. Preserve all current outside/refusal/missing-Program barriers, both false
   authority flags, write/duplicate/cache/recomputation limits and exact compiler
   options. Any future decision to discharge ancestor/peer-metadata probes needs
   its own bounded acquisition proof; it is not bundled with new dispositions.
6. Capture genuine base RED for new packet observations before implementation;
   retain the14 characterization tests here as controls, not fabricated RED.
   Replay this same fixed Program and run full observer/default/MCP gates.

Not authorized here: dependency installs, lock/config rewrites, fixes to public
or private applications, global missing-module suppression, general React.FC
authority, runtime/CPG/nav/cache changes, full multicorpus evaluation.

## Review

Two SELF-PASS rounds, NOT INDEPENDENT. No runtime WRONG is demonstrated. SMELL:
current coarse labels lack the source/context distinctions required to tell
filesystem absence, ambient binding, missing source and metadata probes apart.
The producer's conservative unproven outcome remains justified by independent
closure gaps; this audit is not a reason to promote the four Library candidates.
