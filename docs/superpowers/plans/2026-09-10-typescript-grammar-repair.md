# TypeScript grammar repair — design and RED checkpoint

Base: PR304 merge `9270ab43` (freshly fetched). This is an incomplete repair
checkpoint, not an implementation completion or a merge-ready artifact.
Primary owns architecture; two bounded read-only audits investigated upstream
grammar and call consumers. Repair review cap: two rounds; none dispatched yet.

## Authority decision required

The supported Rust crate consumes generated C, not grammar.js. Its two C files
total 17,515,764 bytes. A narrow local grammar edit therefore needs a distributable
patched dependency plus a generator. No tree-sitter CLI is on PATH and no CLI or
generate crate is in the inspected local Cargo registry. No ready upstream fix
was found: [upstream issue367](https://github.com/tree-sitter/tree-sitter-typescript/issues/367)
is open and matches the first-type-argument defect. Current upstream grammar
retains the same relevant precedence pairs. Search absence is not proof that no
unindexed fix exists; no verified candidate has been acquired or tested.

Owner question sent: approve pinned in-repo vendoring (~17 MiB generated C) and
isolated pinned generator bootstrap, with mechanical output reviewed separately
from the authored grammar patch? No approval received at this checkpoint.
No installation, vendoring, dependency edit or production edit has occurred.

## Corrected defect population

All probes use production source unchanged from the merged base. Initial consumer
audit used an existing pre-change rlib; primary then rebuilt `cargo build --offline
--lib` in this checkout and reproduced the findings. No regression is attributed
to this tests/docs-only checkpoint.

1. **WRONG: inline import-type generic call parse failure.** Existing PR304 cases
   await/plain/parenthesized get<typeof import("./module")>() have native ERROR
   nodes in both TS and TSX, although compiler-valid. Original bytes must survive.
2. **WRONG: await/generic call structure and callee.** The previously zero-error
   number/typeof-identifier/alias controls parse a call whose function child is
   await_expression(get). Public APIs and CallGraph record callee `await get`.
   Compiler AST has get as callee with AwaitExpression outside CallExpression;
   emitted JavaScript is await get(). Plain await get() is correctly represented.
   Zero parse errors in PR304 were never proof of correct tree/call structure.
3. **WRONG: type-only calls leak into runtime observations.** Already-parseable
   `function owner() { type T = typeof import("m"); }` and parameter type annotation
   produce owner->import sites across all six public call APIs and CallGraph.
   A mixed same-line type/runtime import emits two sites; line-based argument lookup
   returns the type argument "types" instead of runtime "runtime". Real dynamic
   import is the positive control. Do not suppress based on import/typeof spelling.
4. **SMELL / conditional design hazard:** current build.rs watches/hashes src,
   build.rs, Cargo.toml and Cargo.lock only. Same-version edits in a future vendor
   directory would evade root rebuild/input identity unless explicitly included.
   No current vendored source exists, so this is not a demonstrated current cache bug.

## Tests and probe record

`tests/lang/typescript/import_type_grammar_test.rs` promotes the nine PR304 cases
to separate TS/TSX tests. Desired structure and byte ranges are checked, not just
error counts. Fresh result: **6 passed /12 failed**, with 50 pre-existing tests
filtered out. Six failures are original error nodes; six are newly measured
await/callee structure failures. Keep these failing expectations; do not mark
ignored, weaken them, or merge the WIP test-only branch.

Existing TypeScript suite control excluding the new module: **50 passed**, with
18 new tests explicitly filtered out. Full Rust/MCP/Python/observer/Tier-A gates
have not run this turn because no implementation candidate exists. PR304 totals
are historical, not current verification. Format and whitespace checks are separate.

Evidence directory: `/private/tmp/prism-grammar-repair-BR8ivP`:
structural-red.log, typescript-control.log, await-structure-probe.log,
consumer-base.log, consumer-fresh.log, consumer_probe.rs. Initial grammar-red.log
had nine grouped tests that stopped at the first dialect failure; superseded by
18 individually executed tests in structural-red.log, not an extra defect count.

Hypotheses/probes/results:

- Invalid source versus grammar ambiguity: PR304 compiler-valid fixtures reproduce
  native failures against unchanged base; malformed control still refuses.
- Missing call node versus await nested in callee: full tree shows call_expression
  with await_expression function child. Fresh call APIs/graph output `await get`;
  compiler AST and ordinary-await control distinguish this from test-only shape taste.
- Type import token rejected by name extractor versus leaking runtime call: all six
  APIs, exact spans and CallGraph prove leakage; genuine import is retained.
- Upstream ready fix versus local generation requirement: exact open issue found;
  its reported unsuccessful experiments are not our local experiments. A two-way
  conflict replacing static precedence is only an untested candidate.

## Proposed bounded sequence after approval

1. **Dependency custody/reproduction:** pin published upstream
   f975a621f4e7f532fe322e13c4f79495e0a7b2e7, exact generator/runtime and JavaScript
   grammar inputs, retain licenses/notices; isolate bootstrap from real repositories
   and global installations. Regenerate unchanged baseline first. If baseline does
   not reproduce, classify generation drift before applying a fix. Review generated
   payload as mechanical provenance, not thousands of independently authored lines.
2. **Cache identity prerequisite:** include actual vendored build inputs in rerun
   watches, tracked/untracked and gitless enumeration, dirty detection and cache
   fingerprints. Test same-version source mutation, source restoration, cold/warm
   misses and metadata. A one-time cache-version bump alone is insufficient.
3. **Grammar candidate:** repair import-type and await/call structure, preserving
   source bytes. Compare direct/member/optional/nested/new/second-argument and TSX
   ambiguity cases. Distinguish await get<T>() from (await get)<T>()/parenthesized
   callee semantics. No blanket flattening of await callee nodes or text rewriting.
4. **Runtime-call boundary:** add a TS/TSX type-ancestry refusal at shared call-name
   extraction, verified across six APIs, spans, argument indexes/manual lookup and
   full/subset graphs. Audit direct is_call_node consumers before any broader claim.
   Preserve dynamic import and runtime typeof import, string/comment lookalikes,
   and same-line mixed types/runtime sites. Syntax queries may retain type nodes.
5. **Verification/value:** all original 628 native files, exact errors/functions/
   call facts; regular suites and fresh Tier-A matrix/quick after rebuild; original
   compiler packets and source custody unchanged. Native's 286 non-parser skip
   barriers, semantic closure and react-scripts remain unresolved. No promised
   eligible-owner or recall gain merely from a successful parse.

Keep authored work in bounded increments if prerequisite/review scope cannot fit.
Never replace the existing test artifact to escape RED or review pressure.
No upstream repository writes, dependency publication, JSON/.mts/.cts admission,
closure expansion, private reads, real source execution or edits are authorized here.
