# UMD qualifier source audit and bounded successor requirements

Status: audit and negative-fixture foundation, not authorization to expand
resolution. Owner: “267, 268, 269 are merged, proceed to next”. The accepted next
recommendation was a source/compiler-backed qualifier/ambient identity audit.
Base dc9b9e847b829bf84a43a323a9eb7e28e6aa5c16. No runtime authority.

## Settled mechanism

At LibraryDropdownMenuButton's React.FC annotation, the actual observer Program
has one compiler declaration at each hop:

| Binding | Defining syntax | Role |
|---|---|---|
| Global React alias | export as namespace React | external declaration file's globalExports table |
| Immediate module export alias | export = React | source module's export-assignment binding |
| Local React namespace | declare namespace React | module-local namespace, distinct symbol from the global alias |

The current provenance helper scans a declaration's enclosing statements using a
`local:` name key. Starting with NamespaceExportDeclaration React, it adds the
same-named local ModuleDeclaration React. Those two nodes do not declare the same
binding. The collected length becomes2 and follow() stops at ambiguous_declaration
before retaining alias anchors. A fixture that changes only the local namespace's
name still has a singleton compiler chain but stops at unsupported_declaration.
NamespaceExportDeclaration is not an accepted gateway in the current implementation.

Do not read this diagnostic as proof of an actual React namespace merge or
augmentation. Conversely, do not delete duplicate checks or accept every singleton
compiler symbol: compiler recovery and competing global providers need separate
source-population checks. The audit establishes this one qualifier's identity,
not whole-Program augmentation closure, runtime values or an Exact receiver.

## Compiler authority

TypeScript5.9.3 is checked by SHA256
3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675 before the audit
fixture loads it. Pinned typescript.js source:

- bindNamespaceExportDeclaration,48966–48975: requires top-level placement in an
  external declaration file and binds the Alias into file.symbol.globalExports.
- getTargetOfNamespaceExportDeclaration,53532–53544: resolves the source module
  symbol, preserving the immediate alias when requested.
- onSuccessfullyResolvedSymbol,52719–52723: allowUmdGlobalAccess concerns the
  value-meaning branch in an external module. Type-only qualification is not the
  same permission question. The positive and value-use diagnostic2686 controls
  exercise both false and true option settings without changing app config.

Both a canonical-root direct compiler probe and temporary diagnostics inside the
actual bounded observer produced the same singleton chain. The instrumentation
was removed; excluding only producer identity, its entire packet equals the prior
public packet. Normal main validates that original packet again. Instrumentation
is evidence tooling, not a new packet producer, schema or proof consumer.

## Proposed next implementation, separately bounded

1. Separate declaration populations by binding domain. NamespaceExportDeclaration
   belongs to the owning external declaration module's global-export domain, not
   its module-local names. Preserve existing local/export duplicate checks and
   verify global-export peers in the configured Program; never globally skip
   same-name namespace declarations.
2. Admit only an explicit eligible NamespaceExportDeclaration that belongs to the
   consulted compiler alias and a uniquely identified canonical source module.
   Require one same-named global-export provider in the bounded Program, while
   rejecting competing providers, duplicate declarations and global augmentation.
   A syntax census, a symbol's mere existence or a recovered target is insufficient.
3. Follow its immediate alias through one export= assignment to one direct,
   non-imported module-local namespace in the same source file. Retain separate
   source anchors for global export, source module, assignment and local binding.
   Imported export= gateways, merging, stars, arbitrary evaluation and cycles stay
   excluded. Charge every traversal to the existing provenance-step limit.
4. Apply the existing callable/property/class constraints after that bridge. No
   React or FC name heuristic; the unrelated Widget/Implementation fixture is
   part of acceptance. Explicit imports and lexical shadows keep their existing
   resolution paths and cannot fall back to the UMD global.
5. Preserve actual compiler options, diagnostics, include membership, canonical
   link identity and duplicate Program guards. No setting allowUmdGlobalAccess,
   synthetic imports, dependency acquisition, .git traversal or config rewriting.
6. Preserve full snapshot/write/duplicate/recomputation barriers. Both authority
   flags remain false. Incomplete Program observations can at most retain candidate
   anchors with program_unproven; the public tree still has independent outside,
   unresolved-module and unsupported-lookup barriers.
7. Before implementing: capture RED for the desired new supported bridge on this
   base, keeping these characterization expectations explicitly distinguished from
   future acceptance expectations. Then run full observer/default/MCP gates and
   replay the same fixed public tree. Do not claim runtime recall from candidate
   observations. Any runtime/CPG/nav edits are out of this scope and trigger Tier-A.

## Frozen fixture population

scripts/callable-observations/umd-qualifier.test.mjs contains15 top-level tests:
same-name singleton; differently named local target; React18 and React19 UMD
annotations; two global provider files; repeated global export in one file; local
namespace merging; external module augmentation; global namespace augmentation;
lexical namespace shadow; explicit namespace import; absent provider/unresolved
export target; invalid non-declaration-file export; type-vs-value option semantics;
and provider rename/removal/restoration epoch changes.

These are passing characterization/negative controls on unchanged production code,
not fabricated RED or implementation of the proposed bridge. The type-only UMD
positives currently remain provenance-unproven, while existing local/import
positives remain traced. Both authority flags remain false throughout.

## Review and stop conditions

Two self-review rounds, NOT INDEPENDENT. No runtime WRONG is established: the
unsupported gateway remains conservatively unproven. SMELL: the scope-insensitive
candidate collection obscures the actual cause and loses pre-refusal alias evidence.
Do not turn that diagnostic correction into permission to erase genuine merges.

Stop if canonical source identity, full provider population or target binding
ownership cannot be established. More complex UMD forms, general global merging,
new installs, private application writes and runtime authority require new scope.
