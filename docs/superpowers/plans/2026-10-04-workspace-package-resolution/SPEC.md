# Lane PKG — workspace package-entry resolution

## 0. Decisions and authority

The owner brief is `/Users/wesleyjinks/prism-evidence/pkgres/planning/plan-brief.md`, S2-O8. Base is main `4e592daa7858a195eb3a9eb77c83dfbc763b49fa`; candidate is dirty, controller commits. Census was written before this design. No F access, S2 adoption, network, Git write, or independent-review dispatch is authorized here.

1. **Positive native binding only.** Workspace inventory does not manufacture a module binding. The flagged X writers genuinely fail native resolution. Rewriting generated package outputs to source needs additional authority and stays OQ-1. Installed-X's Next.js writer has a real canonical source winner.
2. **TS semantics, writer project.** Reuse P1's captured configuration/ownership selector and P2's caller-project export projection; never reselect a barrel's own config. The prototype supports explicit node/node10, node16/nodenext and bundler modes only in supported configurations. Ownership is checked independently with actual TS ProjectService.
3. **Entry semantics.** Legacy selects nonempty `typings`, then `types`, otherwise `main`, then index after a missing selected field. It does not select main after a missing types entry. Node10 ignores exports. Bundler static imports use require under explicit CommonJS emit and import under explicit ES/preserve emit; other/default module settings decline. This was corrected after a native falsifier (PROBE-LOG). Modern exports uses source member order and condition membership (`types`, import/require, node outside bundler, default, customConditions); literal subpaths precede patterns, patterns choose longest prefix then key length. A proved missing leaf tries the next matching condition; a declaration/opaque winner blocks it (TS toSearchResult, line 46646). Native TS does not select package.json's `module` entry; that package field supplies no proof. CompilerOptions.module separately determines emitted import form.
4. **Physical identity.** Only captured in-root package links/self-reference and final indexed regular source can supply a positive result. Broken/outside links, declaration winners, opaque/unindexed candidates and external shadows remain unresolved. Package-directory absence and metadata/link edits participate in existing snapshot topology. Ordinary installed external packages never become repo definitions.
5. **Schemes.** `node:` is a reserved external-builtin namespace; it cannot name repository source. Production exposes this classification without making Exact targets. Other colon schemes are LoaderScheme and remain unresolved. No virtual-loader policy is selected; OQ-2.
6. **Preservation.** Existing lane-P paths proof runs first. Matching paths refusals, baseUrl and unsupported ownership/options are not bypassed. Export callable/wrapper/write/shadow/ambiguity/depth guards remain unchanged. No refusal cut adopted. CPG/nav cache epoch becomes 107/63.
7. **Bounded prototype, not full feature completion.** Only indexed `.ts`/`.tsx` implementation winners are admitted. JS secondary package search, export arrays, numeric/duplicate package keys, typesVersions, versioned types conditions, generated-output source redirects, broad config/solution ownership and .mts/.cts entries remain unimplemented. They are omissions in this prototype's coverage, not accepted correctness/performance costs. See OQ and IMPLEMENTOR for the serial continuation.

## 1. Discovery versus lookup

Census inventories in-repo named package manifests and records each actual node_modules entry. Declared npm/Yarn workspaces, pnpm-workspace and lerna are discovery inputs for the planned complete inventory; they are **not** a native lookup rung. The prototype binds only a TS-reachable package entry, including captured node_modules links into indexed source or modern package self-reference. Discovery without installation never silently binds X or R imports. pnpm/lerna declaration expansion is specified but not implemented/certified in this prototype; source links still qualify independently.

## 2. Integration and snapshot contract

`js_paths::Resolver::{resolve,hop}` try existing paths then the package rung. `package_in` carries the same Config and paths/ambient guards. `js_packages` reads retained metadata, with a source-ordered parser, and `JsPathsSnapshot::package_root` admits captured canonical directory identity. Existing CallGraph import-member module tables and P2 export closure are reused; there is no global package-name callee lookup. Namespace writer joins are handled only by the scratch S2 resolver using these proofs, not by granting new namespace Exact authority on main.

Snapshot budget/read failure declines; no live package read is introduced during resolution. Existing package bytes/hashes, link inventory, candidate probes, entries and occupancy cache fingerprint bind results. Cache epochs invalidate earlier stored facts. S2 scratch caches are separate from prototype facts.

## 3. Native source binding

Pinned TS 5.9.3 `lib/typescript.js` SHA256 `3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675`: conditions 44504–44522; legacy metadata/index 45745–45813; modern exports 45851–46164; nearest node_modules priority/secondary passes and @types 46298–46407. These ranges explain why a JS-only local absence check is insufficient, why source key order matters, and why a missing exported target cannot be guessed back into source. The prototype deliberately refuses unresolved earlier authority rather than approximating the full algorithm.

## 4. Acceptance and continuation

Every changed complete row must be CORRECT_STATIC_BINDING, including native module, actual writer ownership, and terminal file/name/start/end lines. No site-population change, unproven change, or lost base edge is admissible. Measure X, installed-X, R, T and controller-only F before adopting any cut/cost. S2 overlay is measurement only. Gate requirements and exclusions are in VERIFICATION; complete public rows, controls and bound source/binary hashes are in MEASUREMENTS and BUILD-MANIFEST. No independent acceptance is inferred from self-tests or advisory mutants.
