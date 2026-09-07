# Public dependency acquisition — completed; observer admission declined

Owner explicitly authorized the frozen-lock, script-disabled public disposable
install and pinned tool bootstrap after PR265 merged. Measured from Prism main
3001ca58c04aa323a686968bbfb57eb7b927d730; measurement4597f6b is pushed in
[PR266](https://github.com/shoedog/prism/pull/266), targeting main.
No observer, Rust/runtime, schema, cache or application-source changes. No private
repository acquisition. Two SELF-PASS rounds, NOT INDEPENDENT; no agents.

## Outcome

The single install succeeded in24436ms, with scripts disabled. Post-install
compatibility was declined on three independent existing admission barriers:

| Measure | Acquired source plus pinned compiler | Current observer limit |
|---|---:|---:|
| Regular-file bytes | 853674175 (about814MiB) | 134217728 (128MiB) |
| Regular files plus physical directories | 68997 | 20000 |
| Symlinks | 249 | 0 |
| Maximum observed depth | 11 | 64 |

Source tree alone:59799 files,9059 directories,830105343 bytes. Compiler lib:
125 files,14 directories,23568832 bytes. Link entries are reported separately
from regular files/directories. There are240 file links and9 workspace directory
links. The non-following metadata graph classified all249 as in-root inventory
targets, with no observed cycle/missing/external target; zero special files,
invalid names or package.json parse errors. This is not a canonical-identity or
race-freedom certificate. Two complete metadata walks matched exactly.

Per the approved stop rule, produce/validate were NOT run on this predictably
inadmissible tree. No dependency pruning, bin-link suppression, symlink flattening,
config replacement or limit increase was attempted. No configured Program,
diagnostic-clean, receiver recall, class or Exact authority claim follows.

## Source and tool custody

Public source0642e72cfa2d9a71198200e52f37399384610ee3 was copied with git archive
into /private/tmp/prism-acquire-w2FtSq/source. Before and after installation,
every tracked file's bytes and executable bits matched the clean original; no
new files appeared outside node_modules directories. The original stayed clean.
The preserved root control hashes are:

| File | SHA256 |
|---|---|
| package.json | 3afad0abc6d241112e95fdd74e111e93f31bc1b7d420a4345326c456706b1d42 |
| tsconfig.json | 4f3289effdd213d3c0c8fa290645308157f8418aafc0c2cb4a3c459102d54cf6 |
| yarn.lock | a2a92a778255e83a576290948a23db0d05ec4ee994dd43c1d368a3979f16ecd0 |

Node24.15.0 on Darwin arm64, executable SHA256
3200fbd9f7fd4410426dd541e10d1ab829d3472f270d743c7fabd1696c03fe32.
Yarn1.22.22 archive matched SHA512 and SHA1 in exact-version metadata fetched over
TLS from registry.npmjs.org. Archive SHA256
c17d3797fb9a9115bf375e31bfd30058cac6bc9c3b8807a3d8cb2094794b51ca;
bin/yarn.js148e19db309ec9eaf7720b28df811337906eea8a1758deaa54afee60a6305e04;
lib/cli.js443ed69e76443b89afddccfc9faec1ff16eb5e500979cc079c696dec4c3d94ee.
This is TLS-registry integrity verification, not an independent signature verdict.
No global tool install or Corepack activation occurred.

## Execution controls and warnings

The supervisor launched the pinned Node/Yarn with:

`install --frozen-lockfile --production=false --no-default-rc --ignore-scripts --non-interactive`

Cache/global/link/temp/config directories were task-owned and outside source/;
the environment was an explicit allowlist, with no inherited credentials, HOME
override or automatic RC discovery. The unchanged project .npmrc's save-exact
and legacy-peer-deps settings were supplied explicitly. Final config output has
strict-ssl=true, ignore-scripts=true, ignore-optional=false, bin-links=true and
registry=https://registry.yarnpkg.com. CLI flags independently set ignoreScripts;
the first config-only probe showed its registry default before adding the explicit
environment setting. That was not an install attempt or evidence of script execution.

Pinned Yarn source was inspected for no-default-rc and lifecycle guards. A local
preload allowed only exact locked HTTPS tarball URLs and denied credential-bearing
requests, unknown redirects/URLs and subprocess execution. It recorded1445 allowed
requests and zero denials. This extra guard is not an OS sandbox. Yarn's completion
log explicitly reports ignored scripts; .yarn-integrity includes ignoreScripts
and an empty artifacts map. No package or application build/test/hook was invoked.

The one-attempt sentinel and15-second supervisor checks enforced operational
15-minute/5GiB-task-data/3GiB-free-floor stops. No cap fired. Final task data was
4044866771 bytes (includes cache/tools/worktree/source), with244475166720 bytes
free. These sampled controls are not hard OS quotas. Original package resolution
and peer warnings, plus Yarn's url.parse deprecation warning, remain in the log;
no engine/peer/checksum overrides or warning-driven retries were added.

## Independent archive checks and probe correction

All1445 cached archives passed cryptographic checks against applicable original
lockfile integrity records:1445 lock records,1442 distinct URLs, zero failed or
missing URLs. For each integrity value the strongest supported listed digest was
checked; every record applying to a repeated URL was retained.

WRONG (local measurement probe only), corrected: the first verifier assumed1445
lock records meant1445 unique URLs and stopped before hashing. The unchanged lock
contains three repeated URLs with identical integrity values: string-width4.2.3,
strip-ansi6.0.1, wrap-ansi7.0.0. The full duplicate population was enumerated before
correcting the verifier. That failed probe and its empty-output summary were
inadmissible, not package corruption. No install retry, lock change or production
oracle adjustment occurred. Initial and corrected probes/evidence are retained.

Installed root identities are TypeScript5.9.3, @types/react19.0.10 and react19.0.0.
The installed typescript.js SHA256 matches the observer's required
3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675.
These checks establish acquired bytes/identities, not compiler closure. Archive
integrity does not independently prove every extracted package byte or link race
safe; generated declarations and actual Program diagnostics remain unmeasured.

## Verification, next recommendation and handoff

Fresh: source/control preservation; repeated metadata;1445 archive checks; pinned
runtime/compiler hashes; effective config, network/script records; machine-checked
admission.json against current schema limits. Two self-review rounds covered
pre-install trust/source custody, then post-install evidence and admission limits.
No Rust/observer suite rerun for this docs/evidence-only increment; earlier totals
are inherited, not fresh. No Tier-A, multicorpus, rebaseline or runtime authority.

Next recommendation: a separately approved bounded acquisition design for streamed
inventory hashing/lazy verified content access and canonical in-root link identity.
Use the measured69k-entry/814MiB/249-link population and negative fixtures for
cycles/escapes/retargeting, lexical-vs-physical duplicate declarations, missing
lookups, case policy, source/config mutation and budgets before implementation.
Raising the byte cap alone leaves the entry/link barriers. Eager buffering and
compiler state need independent memory profiling; the current
--max-old-space-size=512 setting remains unchanged.
Do not silently omit packages/.bin or turn metadata link classifications into proof.

Evidence root /private/tmp/prism-acquire-w2FtSq/public/ contains supervisor/config/
install logs, source manifests, repeated inventory, archive checks and admission
summary. Source/, tools/ and cache/ remain intact locally; nothing was deleted.
The public evidence archive intentionally excludes the bulky dependency tree and
cache. The isolated-worktree skill preserved unrelated original Prism work.
