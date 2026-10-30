# Bounded ESM imported-local forwarding

Base: 4ffe55bf; same branch, bundled with the module-binding audit for one future
PR. Owner approved the next increment, not publication. Two review rounds maximum.

## Contract

Accept one top-level named/default ESM value import forwarded immediately as a
plain identifier in a top-level named/default ESM export. Same and renamed local
symbols are supported. Relative-module lookup, duplicate export poisoning,
two-hop depth, cycles, missing files/members and type/value separation remain.

The bridge must have exactly one structured import of that local, no competing
named value/type declaration, no visible write, no type-only collision and no parse recovery.
No intermediate local alias, namespace/member access, require binding, CJS export,
or type-only export is admitted. Dynamic-scope constructs are refused.

Use a distinct export-fact variant rather than treating the imported local as an
in-file declaration. Follow the existing bounded module resolver, then require
a source-backed terminal named function declaration: top-level, unique (including
same-name nested declaration refusal), body present, unmodified, no competing
value/type declarations, no mutable CJS export-object (`module`/`exports`) or
`eval`/`with` dynamic-scope spelling in the origin file. Ordinary `require` uses
are not themselves export-object authority and do not revoke an ESM declaration.
Unrelated parameter/catch shadows are not competing module declarations; writes
under their nearer lexical binding must not revoke the module binding.
This conservative first terminal proof excludes class, arrow, generator,
declaration-only/ambient and anonymous functions. No receiver authority expansion.

## TDD and value checkpoint

1. Promote five existing ESM Gap cases only after capturing their RED on base.
2. Add direct/default/list/comment forms and per-predicate refusal fixtures;
   assert exact file/name/line with decoys for JS/TS/TSX, full and subset builds.
3. Test actual argument-to-parameter edges and positive -> refused -> restored
   incremental transitions; serialize raw facts and re-run export resolution.
4. Keep the remaining CommonJS, member/namespace and extra-alias gaps unchanged.
5. Preserve base logs, count matched synthetic observations, and explicitly
   distinguish fixture value from unmeasured real-corpus recall.
6. Run full Rust configurations, Node/Python/examples/authority controls and fresh
   Tier-A matrix/quick. Freeze CLI and MCP from the same source build invocation.

## Review boundary

WRONG requires a concrete wrong target/flow; refusal-only recall losses are
SMELL unless they violate the stated positive contract. No silent scope widening
to address open-class dynamic module/heap behavior. Retain prior gate exclusions
until new evidence closes them; do not infer green from a missing oracle verdict.
