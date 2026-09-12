# Bounded ESM forwarding — implementation and value checkpoint

Base `4ffe55bf`, following the refusal-first module-binding audit on the same branch.
This implementation and its docs are intended for the same PR, not a docs-only PR.

## Added capability

Named or default ESM imports can now forward the imported function through a
plain-identifier ESM named/default export, preserving same-name and renamed
bindings. For example:

```js
// origin.js
export function work(input) { return input; }
// bridge.js
import { work as local } from './origin';
export { local as publicName };
// app.js
import { publicName as invoke } from './bridge';
function run(value) { return invoke(value); }
```

The target is the actual origin declaration, not `local` or a same-named function
in the bridge. Tests inspect exact file/name/line and argument-to-parameter edges.

## Proof and boundaries

- `ImportForward` is a distinct raw fact, never a fabricated `Local` target.
- The imported local must be a singleton named/default ESM value binding with a
  relative module path, no conflicting import/type/declaration, no visible write,
  and no parser recovery. Only top-level ESM export occurrences can mint it.
- The existing resolver enforces the unchanged two-hop bound, cycles, missing
  module/member refusals and duplicate/star conflict rules. Each forwarding frame
  then requires a terminal non-class target with separate source-backed proof.
- Terminal proof currently admits only a unique, unwritten top-level named
  function declaration with a body. Nested same-named declarations are refused;
  class, arrow, generator, ambient/declaration-only functions remain excluded.
- `module`/`exports` identifiers and `eval`/`with` conservatively revoke the proof
  throughout the file. Unrelated/shadowed occurrences can cost recall. Ordinary
  `require` use is not itself CJS export-object authority. Nearer parameter/catch
  writes are separated from writes to the module binding.
- No CommonJS forwarding, extra local alias, namespace/member-import expansion,
  package-resolution or class/receiver/React.FC authority is added.
- CPG cache 87 and navigation sidecar 48 invalidate earlier persisted semantics.

The tests describe this bounded static contract, not arbitrary dynamic module or
heap analysis. Type-only and duplicate-declaration negatives include deliberately
semantically invalid but parser-valid source; passing them is not a compiler check.

## TDD and measured value

Before production edits, the five proposed matrix promotions failed on `4ffe55bf`;
the other 58 tests selected by the broad `module_binding` filter passed. New
forwarding tests initially measured 25 passed / 10 failed, with all ten required
capability checks failing. The first implementation passes 35/35; later controls
and source-epoch tests bring the candidate forwarding suite to 46/46. Replaying
the identical final test bytes on unchanged base gives 29 passed / 17 failed.
The final isolated matrix run is 53 passed / 5 failed on base and 58 passed /
0 failed on the candidate. Hashes and counts are
recorded in [the baseline receipt](2026-09-11-esm-forwarding-baseline.json).

The original matrix has 316 matched fixture/language/build observations. Exactly
30 change from no Exact target to the correct origin: five promoted cases across
JS/TS/TSX and full/subset-plus-export-resolution builds. No other row changes.
This is synthetic fixture value, not unique real call sites or corpus recall.
The named matrix now has 18 Supported, 14 Gap, 22 Refused and one JS-supported case
with the existing TS/TSX parser gap. The original audit receipt remains historical.

Matched-row comparison must locate `MODULE_AUDIT` records even when test progress
text precedes them. An initial start-of-line parser omitted records and was
inadmissible; corrected parsing finds identical 316-key populations and 30 changes.

## Verification / review

Full Rust default: **4,294 passed**; MCP: **4,487 passed**; MCP + detached-owner-audit:
**4,510 passed**. Each run has zero failures and one existing reserved-spec ignore.
Examples: **32 passed**. After the final test-only changed-set strengthening,
targeted JS/TS/TSX controls pass **3/3** (base **0 passed / 3 failed**) and the
entire MCP + owner integration suite passes **643**, with one existing ignore.
No production bytes changed between the full suites and this final test rerun.
Node: **786 passed**, no skips; authority controls: **40 passed**; Python:
**940 passed**, one explicit live-adoption opt-in skip. Frozen executable hashes
were unchanged across these checks. Fresh-release Tier-A matrix: **159/159**.

Tier-A quick was terminated at the declared five-minute attempt cap (the Python
process had run 5m07s when signalled), without a completed verdict. Its generated
snapshot is retained with the logs. This is **incomplete, not green or INVALID**;
no quick real-corpus recall claim. Three historical imported-props helper tests
remain unavailable because their pinned `real-sites.jsonl` input is missing.

Independent source round 1/2 found WRONG 0 / SMELL 3: plan wording for
parameter/catch shadows and CJS spellings was narrowed to the actual contract,
and source-epoch controls were completed. Round 2 approved with WRONG 0 / SMELL 2:
overinclusive changed-file sets in the state tests, and incomplete Tier-A quick.
One bounded test-only review extension was disclosed and the changed sets now
use exact byte differences, asserting bridge-only and origin-only epochs. The
independent static supplied-delta review approved that fix (WRONG 0 / SMELL 0).
No production changes followed source approval. Source navigation used current files after Prism
reported 43 stale paths; no LSP tools were available in this session.

Evidence: `/private/tmp/prism-esm-forwarding-1nk0WK`; base worktree production
remains `4ffe55bf` with only identical test files added. No remote writes yet.

## Next boundary

Fourteen original gap cases remain. The next candidate is singleton const
destructured-require binding to a named CJS export, with explicit export-object
ownership and mutation/snapshot controls. Do not normalize that into this ESM
live-binding proof. General aliases and whole-module forwarding stay separate.

Follow-up: the [CJS producer-barrier increment](2026-09-11-cjs-export-barriers.md)
repairs prerequisite export-object defects before forwarding admission. The
fourteen Gap cases are unchanged. Terminal Local declaration/initialization and
require-time snapshot proof are still required; object custody alone is not enough.
