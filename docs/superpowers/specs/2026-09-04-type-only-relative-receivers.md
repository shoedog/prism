# Type-only and explicit-relative receiver identity

Base: merged PR #239, `862166d`. Owner: “ok, merged proceed”.
Status: implemented/pushed as `0a2edf6`, [PR #240](https://github.com/shoedog/prism/pull/240) merged as `f3bf88e`. Full suites and matrix pass; quick corpus-pin exclusion remains.
Two bounded slices; three-round self-review cap declared before implementation.

## Evidence and corrected assumptions

Pre-#240 source dropped TypeScript type-only imports before class lookup and rejected
all leading-dot Python submodule receiver proofs. Type-only imports are erased;
Python relative imports do introduce runtime value bindings. These are distinct
proof routes, not two forms of non-runtime binding.
The next increment must not turn type evidence into constructor evidence or infer
a Python source root from a filename stem. Structural navigation identified the
shared proof-key and resolver consumers; stale server lines are checked in source.

Bounded real-source sample: Excalidraw `packages/element/src/align.ts` imports
`Scene` with `import type` from `./Scene`; Scene is directly exported from that
file. Django `tests/test_runner/test_parallel.py` uses `from . import models`,
and the containing initializer is checked separately. These are syntax examples,
not corpus precision/recall evidence. Default Scene imports, structural aliases,
and executable Django initializers elsewhere remain excluded.

## A. Type-only class import proof

Support TS/TSX `import type {Client as Alias}` and `import {type Client as Alias}`.
Persist a separate per-file map of local name to module/export identity, with
explicit poisoned entries for duplicate names or conflicting module type
declarations. Consult only for TypedParam recovery. Do not add these names to
runtime import bindings, free-function exports, constructor origins or namespace
receiver resolution. Default/namespace type imports and reexports remain excluded.

Existing value imports retain their existing eligibility rules. A simultaneous
value import under the same local spelling blocks the type-only proof; ordinary
value variables/functions in the separate value namespace do not. Local generic,
type declaration and class-expression shadows, receiver writes and unsupported
type syntax retain their existing fences. Defining-file resolution still requires
one exact relative candidate, a direct Class export and clean direct instance slot.
Never fall back to same-file/global ownership after a present but failed type proof.

## B. Explicit-relative Python submodule proof

Support `from . import models`, `from .. import models`, and a dotted suffix such
as `from .nested import models`, with aliases. Resolve from the caller's containing
directory; require an indexed inert initializer at the starting package and every
package crossed while ascending. Reject escape above the indexed package anchor,
malformed path components, competing module files and executable initializers on
the resulting prefix. Keep absolute namespace-package behavior unchanged.

The shared submodule helper must be used both by call-time owner lookup and by
whole-program imported-class proof keys. Initializer/target eligibility transitions
must refresh unchanged importers through the existing full-rebuild seam. No
runtime import execution, namespace-relative inference, reexports or __getattr__.

## Verification and custody

RED on exact merged base in the same environment, positive and negative matrices,
full/subset and serialized cache/incremental parity, and served sidecar edges.
CPG 62 and navigation sidecar 31 reject the previous semantics. Full default/MCP
suites, format/check/Clippy, immediately rebuilt Tier-A matrix and quick. Carry
corpus-pin invalidity; no full multicorpus run or rebaseline. Preserve existing
untracked artifacts. Commit/push/open PR after gates; no merge.
