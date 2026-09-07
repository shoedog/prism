# Callable dependency acquisition — bounded preflight and execution plan

Status: planning complete; acquisition NOT executed or authorized by this document.
Published as [PR265](https://github.com/shoedog/prism/pull/265), plan commit786a019.
Owner approved this planning successor after PR264 merged. Base
f2f8a3c535fca20013ee642cec977403beec51b5. Two SELF-PASS rounds, NOT INDEPENDENT;
no agents. Documentation only: no observer/runtime/schema/cache changes.

## Decision

Recommend one public-only, disposable, lockfile-pinned acquisition measurement
before any acquisition-layer implementation or runtime authority expansion.
Installing dependencies is not equivalent to obtaining an admissible observed
Program. Keep acquisition, snapshot admission, compiler closure, lexical binding
and class/runtime authority as separate gates. Do not install into either original
application, copy fixture React declarations into it, omit dependencies to fit
budgets, replace configs or silently dereference links.

## Source-backed preflight

Public Excalidraw: clean0642e72cfa2d9a71198200e52f37399384610ee3. Actual root
tsconfig.json includes packages and excalidraw-app, explicit ambient test types,
React JSX runtime, moduleResolution=node and source workspace paths. Selecting
packages/excalidraw/tsconfig.json instead changes the Program and is not a replay
of the watched population.

| Control | Measured value |
|---|---|
| Manager | packageManager=yarn@1.22.22; Yarn v1 lockfile |
| Package manifest SHA256 | 3afad0abc6d241112e95fdd74e111e93f31bc1b7d420a4345326c456706b1d42 |
| Root config SHA256 | 4f3289effdd213d3c0c8fa290645308157f8418aafc0c2cb4a3c459102d54cf6 |
| Lockfile SHA256 | a2a92a778255e83a576290948a23db0d05ec4ee994dd43c1d368a3979f16ecd0 |
| Root-pinned compiler / React types | TypeScript5.9.3 / @types/react19.0.10 |
| Lock provenance scan | 1445 resolved entries,1445 integrity lines; all resolved URLs HTTPS registry.yarnpkg.com, no embedded URL credentials |
| Current application tree, excluding .git | 1229 files,140 directories,54407283 bytes;0 symlinks/special files/node_modules directories |
| Pinned compiler lib tree | 125 files,14 directories,23568832 bytes |
| Combined regular-file bytes | 77976115 bytes;56241613 bytes remain under128MiB before dependencies |

The metadata walk does not hash every source file and is not the observer's full
snapshot. It hashes control files and does not follow symlinks; two repeated walks
matched exactly. Git cleanliness/HEAD bind tracked source. Installed size, links,
package contents, compiler diagnostics and closure remain UNKNOWN until acquisition.
Integrity-line counts are a provenance inventory, not package verification or a
lockfile-parser proof of dependency completeness. The lock also contains another
TypeScript selector/version; do not select the compiler by first lock match.

Seven exact-version dependency edges connect local workspaces. Yarn documents
workspace symlinks; the observer's snapshot currently rejects every symlink before
building a Program. This is a demonstrated contract incompatibility for that
documented layout, not a new resolver defect. Bin-link suppression alone does not
remove workspace links. [Yarn workspaces](https://classic.yarnpkg.com/lang/en/docs/workspaces/).

Yarn is not on the current PATH. Existing local Node24.15.0, npm11.12.1 and
Corepack0.34.6 were identified; default Node is26.0.0. Merely finding Corepack does
not prove Yarn is installed, byte-verified or available offline. No manager was
bootstrapped. Original package prepare hook must not run.

Private repository preflight is retained only in private evidence: actual config,
manager/lock/runtime constraints and repeated metadata inventory were inspected.
It needs its own acquisition profile; public Yarn instructions are not portable
to it. No private package names, paths, lock details or source are published here.

## Proposed execution contract — requires explicit approval

1. Public Excalidraw only, exact SHA/control hashes above. Create a new disposable
   source copy from tracked Git bytes under a task-owned temporary directory.
   Preserve every tracked file and the actual config; verify hashes before any
   package execution. No original checkout writes, global installs or cleanup.
2. Pin the existing Node24.15.0 executable and record its hash/version/OS/arch.
   Acquire Yarn1.22.22 into a task-owned tools/cache directory if necessary;
   authenticate its registry integrity and record the executable/archive hashes
   before use. Network bootstrap is part of the requested acquisition authority,
   not an implicit Corepack side effect. No global manager activation.
3. Preserve project package-manager config, inspect effective non-secret options,
   and isolate user configuration/cache/credentials without copying secrets into
   the source snapshot. Block unexpected registries, Git/local/external dependency
   sources, credentials, redirects requiring new trust or lock mismatch. Resolve
   such failures explicitly; do not add ignore-engines/peer/checksum overrides.
4. Candidate command, ONLY in the fresh copy with verified tool/runtime:

   `yarn install --frozen-lockfile --ignore-scripts --production=false --non-interactive`

   Keep dev/optional dependencies and normal link layout for measurement; do not
   run builds, project hooks, plugins, test scripts or manager audit/HAR commands.
   Frozen lock refusal is a stop, not permission to regenerate. These controls
   follow [Yarn install documentation](https://classic.yarnpkg.com/lang/en/docs/cli/install/).
   Scripts-disabled acquisition is a measured profile, not a build-complete app.
   Missing generated declarations stay a separate gap; never enable scripts to
   make the compiler green without approval.
5. Provisional acquisition caps: one install attempt,15-minute timeout,5GiB new
   task-directory data and3GiB minimum free disk. Poll at most30-second intervals;
   interrupt on a cap and preserve diagnostics. These are operational guards, not
   hard OS quotas or archive-bomb guarantees. Reject special files and inventory
   links without following them; do not delete failed material automatically.
6. Recheck original cleanliness/HEAD and copied source/control hashes. Inventory
   acquired regular-file counts/bytes, maximum depth, link targets/cycles/escapes,
   package identities/integrities and generated/missing declaration inputs. Keep
   caches, logs and manager tools outside the candidate audited root. Do not prune
   package files or normalize symlinks into apparently authoritative source.
7. Compare the actual installed layout against current observer limits:20000
   files+directories,128MiB combined input,depth64,30s worker,512MiB heap,8MiB packet;
   every symlink refused. If incompatible, STOP with exact measured gaps. Do not
   raise defaults, filter the snapshot or run a predictably rejected whole scan.
8. Only if admitted unchanged, run produce then validate with the pinned5.9.3
   compiler and actual root config. Independently report acquisition admission,
   Program reasons/diagnostics, the four existing receiver spans and all authority
   flags. Public targets: LibraryMenuHeaderContent.tsx lines155,160,184,265 at the
   pinned source hash. No expansion of scope to manufacture receiver gains.

Even a fully installed admitted tree may remain unproven: refused compiler probes,
outside lookups, missing generated files, diagnostics and unsupported references
are independent closure barriers. Dependency completeness is an outcome to measure,
not a label earned by successful package installation.

## Subsequent implementation decision, not authorized here

If workspace links or budgets block admission, bring the measured population into
a separately approved acquisition design. Required proof obligations: canonical
in-root link target identity, cycles/escapes/retarget races, duplicate lexical vs
physical declarations, case policy, directory/negative-lookup completeness, content
and config mutation invalidation, independent reproduction and hard traversal
budgets. Do not conflate this with runtime class authority or blindly dereference.

Private acquisition is a later explicit scope choice. A clean-install manager may
remove an existing node_modules tree and requires the lock's original resolution
flags, so never run it in the original checkout. [npm ci documentation](https://docs.npmjs.com/cli/v11/commands/npm-ci/).

## Verification and custody

Two SELF-PASS rounds: (1) source/config/lock/runtime and snapshot-contract audit;
(2) challenge install-equals-closure, bin-link-only workaround, package/compiler
version inference, source mutation, lifecycle scripts and privacy boundaries.
No implementation or behavioral change; Rust/observer suites are not rerun for
this documentation slice. PR264's test counts are inherited, not fresh evidence.
Fresh checks: repeatable read-only metadata/control hashes, Git identity/cleanliness,
exact local runtime version, source/official-doc comparison and docs-only diff check.

Evidence root /private/tmp/prism-acquisition-plan-6kg7fn: public/ contains the bounded
read-only metadata probe and public results; private/ contains team-repo results,
excluded from public archives and PRs. This plan never authorizes itself to execute.
