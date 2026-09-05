# Python and JS receiver authority repair

Status: published at `9a790419` in [PR #238](https://github.com/shoedog/prism/pull/238)
(merged at `350cc89`); full default/MCP suites and matrix pass. Tier-A quick
completed with zero oracle/SUT errors but remains baseline-invalid for corpus pin
drift. Publication is explicitly authorized by the owner follow-up; see the handoff.
Control: fetched main `10d82ca58387f030a863f75cb6f83ec2f1b9c662`; checkout
`ea2965e0237335a1c9c5c147e3aee9168e5bb84b` has the identical tracked tree.

## Handoff audit

PR #237 correctly records the earlier merges, but completion of those bounded
contracts does not establish complete Python/JS scope or receiver correctness.
The owner's current request supersedes its requirement to obtain a new scope.
Item 2, Java, write tools, and historical worktree cleanup remain separate.

WRONG 1: `outer(Foo)` containing `run` with `x = Foo()` / `new Foo()`
resolves `x.m()` to the module class despite the enclosing parameter. Python also
accepts a constructor shadow in the current function; imported Python aliases have
the same closure defect. The constructor spelling is not proof of its binding.

WRONG 2: a sole constructor assignment inside an `if` can donate a type to a call
after the conditional. The false branch does not initialize the receiver. Source
order does not prove execution order.

WRONG 3: JS `for (x of items)` does not kill a preceding constructor origin, and
`x.m(); x = item` inside a loop ignores the write from the preceding iteration.
Both retain Exact `Foo.m` when `x` can hold another class. The same loop-carried
write defect affects Python constructor origins and typed parameters.

WRONG 4: TS/TSX `run<Foo>(x: Foo)` and enclosing type aliases, interfaces, local
classes, or named class expressions resolve to an unrelated module `Foo.m`.
Named class-expression self names also shadow JS constructor lookup.

The initial nine constructor cases reproduce on unchanged production code in this
checkout; the final exact-base control reproduces all 24 cases in the completed matrix.
Tests accumulate the complete matrix before asserting. The first JS fixture had
same-line method identities and was inadmissible for Exact-edge evidence; its
corrected distinct-line run reproduced all five Exact edges.

## Architecture

Keep proof at AST classification, shared by full and subset extraction. Existing
resolution and persisted receiver fields remain the consumers; unsupported facts
materialize the receiver and preserve NameOnly residue without Exact promotion.
No general type graph, interprocedural execution model, or new serialized field.

Slice A: constructor-name visibility. JS must examine every enclosing callable and
lexical scope before accepting the module class. Python must fence constructor and
local-annotation roots against current and enclosing function bindings; parameter
bare parameter annotation lookup excludes the declaring function body. Qualified
annotations retain the predecessor's stricter current-function shadow barrier.
Retain conservative treatment of global/nonlocal uncertainty. TS type proof
examines enclosing generic parameters and lexical type declarations separately
from value bindings; module-level unsupported metadata remains resolver-checked.

Slice B: initialization and mutation. A constructor declaration must finish before
the call and have an execution region containing the call. Use a conservative
structured statement-list proof, allowing nested ordinary blocks and calls deeper
inside the same region, while rejecting conditional/loop/try escape and switch
entry without a dominating declaration. Loop assignment targets count as writes.
For a call inside a loop whose origin precedes that loop, inspect writes through
the loop's end so a later source write cannot survive a back edge. A constructor
inside the loop before the call resets the origin each iteration.
Python uses its existing binding walker to distinguish an origin in the loop
prefix from one outside the loop, and its local-binding inventory to detect later
writes when the origin must survive a back edge.

The finite authority audit covers generic parameters, aliases, interfaces, local
classes and class-expression self names. It does not establish whole-language
completion, runtime monkey-patch analysis, interprocedural writes, or import graph
coverage beyond the earlier proof boundaries.

## Acceptance and persistence

Each added behavior needs a production RED, an edge/negative pole, and GREEN.
Cover JS, TS, TSX, Python, local/imported ownership, active and ended shadows,
straight-line versus conditional initialization, and loop reset versus loop carry.
Full/subset/incremental builds and CPG/navigation round trips must agree.
Advance CPG 59 to 60 and navigation 28 to 29 because old cached classifications and
Exact topology are invalid even though the serialization shape is unchanged.

Run full default and MCP suites, format, check, configured Clippy, release build
immediately before each Tier-A matrix and quick invocation. Report totals and any
oracle/pin exclusions; do not rewrite baselines. Same-environment base control is
required before attributing any newly observed suite failure to this change.

Review cap: three self-review rounds per slice. At the cap classify findings before
extending; preserve this artifact and target bounded findings. The owner follow-up
explicitly authorizes commit, push and PR creation; merge is not requested.
