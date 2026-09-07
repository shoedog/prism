# Singleton exact-ambient observation closeout

Base: merged PR272, `c49d5b93e896ba699ee378e7de3027fb59b6d163`.
Implementation checkpoint: `ca2982f`. Schema8 / producer0.9.0; research observer
only. No runtime, class-authority, closure-policy or compiler-option expansion.

## Outcome

The fixed public Program now carries272 singleton exact-ambient observations:
152 imports,114 import types,5 import-equals uses and1 export-from use. Every
positive has an actual source literal, exact checker module binding, singleton
original-source provider census and zero same-name augmentations. All6893
filesystem request occurrences retain their original from/specifier/target.

| Null filesystem requests | Count | Observation disposition |
|---|---:|---|
| Supported singleton exact ambient |272| observed, not closure admission |
| Singleton wildcard |231| non_exact_binding |
| Merged wildcard |84| ambiguous_binding |
| Exact ambient require call |1| unsupported_request |
| Augmentation declaration name |1| augmentation_request |
| Missing source binding with an orphan augmentation |1| augmentation |
| Other missing source bindings |13| unresolved_symbol |
| Total |603| all filesystem targets remain null |

The14 missing-source requests match PR272's complete residual anchor population.
One is labeled augmentation because its file also declares an unresolved module
augmentation; its import still has zero checker declarations. The separate
augmentation-name symbol is not import authority. The273 exact ambient uses in
PR272 become272 supported observations plus one deliberately deferred require.

Independent source parsing checked13,990 lookup-anchor occurrences /7,600 unique
anchors and every configured-Program same-name provider/augmentation census.
The legacy packet projection is identical to the exact-main packet, including
resolution order, reads, failed/refused/outside evidence, options, diagnostics,
closure, callable observations and nested class candidates. Only schema, producer
and the additive lookup records differ. Validation reproduces unproven with
authorizes_runtime_edge=false; scope.class_authority also remains false.

Unchanged:59,924 files,9,073 directories,249 links,591 roots,1,445 Program files,
264 refusal digests, outside_lookups=true,0 diagnostics,30 callable observations,
53 nested calls and4 Library candidates still program_unproven. The original
public checkout is clean; tracked bytes/execute bits match the disposable source
copy before/after. No acquisition/install, lock/config rewrite or app build ran.

Full source/config identities, residual anchors, positive-context samples and
packet hashes are in `2026-09-07-callable-exact-ambient-evidence.json`.
Producer hash: `9675885e5e5246dad32896415f8a82e1a22e3ebe0dfabeffdc63bb03804e40a0`.

## Verification

- Initial new-behavior RED:0/16 passed on unchanged production. Final26 fixtures
  run against exact merged-main implementation:0/26 passed,26 behavioral failures.
  The control copy changes only the index import to the detached base worktree.
- Final observer suite:181 passed,0 failed,0 skipped (includes26 new fixtures).
- Default Rust:4017 passed,0 failed,1 ignored;28 groups including2 doctests.
- MCP Rust:4207 passed,0 failed,1 ignored;30 groups including2 doctests.
- Authority controls:40 results,failures=[]; all audit-helper tests7/7 passed.
- fmt and diff checks passed. Rust source/tests/Cargo files are unchanged. No
  Tier-A-triggering resolver/navigation/CPG paths changed; no full multicorpus run,
  application build, private acquisition, RSS measurement or runtime recall claim.

Commands (from repository root):

```sh
PRISM_TYPESCRIPT=/private/tmp/prism-imported-alias-O4d6E1/package/lib/typescript.js PRISM_CALLABLE_PROFILES=/private/tmp/prism-callable-authority-98TLLN/public/profiles node --test scripts/callable-observations/*.test.mjs
CARGO_TARGET_DIR=/Users/wesleyjinks/code/slicing/target cargo test --offline
CARGO_TARGET_DIR=/Users/wesleyjinks/code/slicing/target cargo test --offline --features mcp
node docs/eval/receiver-closure/verify-callable-authority.mjs /private/tmp/prism-imported-alias-O4d6E1/package/lib/typescript.js /private/tmp/prism-callable-authority-98TLLN/public/profiles
```

Public produce/validate used Node24.15.0, installed profile, in-root links and
the unchanged tsconfig.json in `/private/tmp/prism-acquire-w2FtSq/source`.
Node tests used26.0.0. Compiler remains pinned TypeScript5.9.3 SHA256
`3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675`.

## Hypothesis/probe/results and review

Two SELF-PASS rounds, NOT INDEPENDENT; no cap extension.

Round1 **WRONG, corrected**: identical-package-ID redirects share AST parents.
The new census initially attributed both Rollup estree augmentations to the
vitest copy, omitting the distinct Vite copy's path. The minimal two-package
reproducer emitted `[a/shared, a/shared]` instead of `[a/shared, b/shared]`.
The public manifest/source parse distinguished this from legitimate canonical
symlink identity. Pinned compiler createRedirectedSourceFile (28286–28307 and
128573–128583) inherits the redirectTarget and stores the original unredirected
SourceFile. The bounded fix traverses that original AST and anchors its bytes;
exact binding still requires node/source identity. Captured redirect RED became
GREEN, including a differing-bytes/same-package-ID negative. No positive binding
or closure had been promoted by the defect. Round2: no remaining WRONG or SMELL
findings after source/census replay, schema/consumer inspection and full observer
rerun. Closure/authority remain unchanged; deferred contexts are explicit scope.

Other probes:

- Synthetic fixture expected one request but got two. Same-environment main and
  branch both emitted one per source file, ruling out observer-created duplication.
  Corrected test expectation; no producer repair or false RED claim.
- Source verifier initially found7,598/7,599 anchors. The single missing literal
  was import("vite") inside JSDoc at woff2-vite-plugins.js:283–289 UTF-16. Explicit
  jsDoc traversal found the same node under the same parser settings, ruling out
  parser-mode/source mismatch. Verifier-only fix; added JSDoc producer coverage.
- Digest test retained the old implementation file list. Corrected base probe
  passed1/1, current failure matched the added exact-ambient.mjs bytes. Updated
  the explicit list and tested omission of the new helper changes the digest.
- New pre-I/O barrier fixture captured one root-getter access when a forged packet
  removed unresolved_module. Full recomputation already rejected the forgery;
  schema8 now rejects it before audited-root access.
- Initial digest-control/audit-helper probes omitted required environment inputs.
  Those observations were inadmissible; corrected commands selected1 and7 tests,
  respectively, and passed. No gate was silently skipped or re-baselined.

## Next boundary

Recommend bounded singleton wildcard lookup observations, keeping compiler binding
provenance separate from asset inventory presence. Merged wildcards and any
closure-policy admission remain separate approval boundaries. The14 real source
gaps and outside/refused acquisition requirements cannot be erased by annotations.

Raw RED/GREEN, exact-base/final packets, verifier, source manifests and logs remain
in `/private/tmp/prism-exact-ambient-rmuI9k`; archive checksum is in the handoff.
The compact evidence and executable regression fixtures are published artifacts;
the raw local archive is not a required external dependency of the implementation.
