# S3 — actual type callback batches and executions

Schema14 / producer0.15.0 adds source/configured/automatic type callback occurrences,
their actual batches and resolver executions. It preserves the pinned per-batch and
shared type caches. It does not change closure, runtime/class authority or dependency
acquisition. The candidate producer is
`134739e153ee3d1ef0d87c616fcd3e009aacfb34169e531071cb2d9915032a7f`.

## Fixed public replay

| Population | Count |
|---|---:|
| Type callback batches | 23 |
| Type occurrences / executions | 29 / 29 |
| Source / configured / automatic occurrences | 27 / 2 / 0 |
| Newly type-owned boundary encounters | 30 |
| Remaining null-owned library encounters | 328 |
| Unchanged module callback occurrences | 6,893 |
| Identical ordered basic-host operations | 91,409 |

Every old packet field except schema/producer compares equal; module rows remain
equal, as do boundary occurrence order/identity except the30 intended owner changes.
The host transcript SHA256 remains
`fc4cfa95458d5521bab94c2e9bcf809c674e67acd5e62b119fd4a07a5b3d2eca`.
Full reproduction returns valid/unproven and authorizes_runtime_edge=false.
The public source remains unchanged, including unresolved react-scripts and14
module-literal gaps. New ledgers do not imply receiver recall improvement.

Seven cache controls preserve old fields and ordered host transcripts: configured
duplicates, source duplicates/shared cache, automatic duplicates, import/require
modes, noResolve, rootless config and imported-package-then-root repeated visits.
Eight identity controls do the same for config-file/directory/multihop aliases,
mixed-case config, twelve configured/automatic indices, and canonical source/type/
module targets through aliases, and combined config-directory alias plus canonical-
package-root revisit. The last control retains two batches and30 identical host
operations. Its initial alias-spelled second root hit the existing duplicate-
canonical-Program-file barrier on both base and candidate; a copied base catch
stack identified worker104, ruling out inventory/resource/candidate-only causes.
That refused transcript was inadmissible, not a regression. No barrier was changed.

## Captured RED and review corrections

The initial nine type-channel controls fail on exact S2's missing ledger. New helper
caps/context controls are helper tests, not a claim that S2 had that helper API.
Three producer/parser mistakes were constructibly found before publication:

- Global source-row uniqueness rejected actual repeated compiler callback visits
  when an imported package later became an explicit root. Exact S2 completed;
  the candidate refused. Actual callback trace ruled out a mere row-projection issue.
  Per-batch identity preserves both visits and closes duplicate-occurrence omissions.
- Twelve or more occurrences exposed lexical old-row ordering versus numeric callback
  indices: the raw worker completed but parser rejected. Source/configured/automatic
  RED3 fail before the numeric comparison correction and pass afterward.
- A config-directory alias exposed canonical source identities being wrongly used
  for compiler-generated lookup addresses. Exact S2 and the raw candidate worker
  completed; candidate parsing failed. H1 separately establishes the lexical versus
  canonical identity contract. Five directory-alias-dependent controls fail before
  wiring that contract and pass after it; file-alias/mixed-case controls already
  passing are compatibility evidence, not additional behavioral RED.

The original review cap was2; a single disclosed converging numeric-index extension
was followed by an identity case that caused explicit design escalation to H1.
The existing artifact was archived, not discarded or restarted. Archive SHA256:
`9792ac47cbb6d3c8bb1b89b1d3fd2eb21664c362645840b407cf858bfd9e6b98`.
H1 integration is separately bounded to two review rounds under its finite matrix.

The final disposable observer suite passes435/435; focused controls58/58 and digest
control1/1. An interim434/435 failed only because the digest test's manual source
list omitted the newly hashed helper. The test list was fixed, then the full suite
rerun; production bytes did not change. The disposable README was stale and is
not integrated: current S2 documentation is preserved and additively updated.

Independent H1 integration review completed round2/2, ACCEPT98/100, WRONG0/SMELL0
within the final bounded coverage check. Its first-round alias/revisit SMELL was
closed by the fifteenth transcript control above. Full [repository gates](2026-09-08-callable-type-search-gates.json)
passed on clean ea81801:435 observer,4017 Rust,4207 MCP,18 helpers,40 authority;
doctests included and one known ignored per Rust run. fmt/diff passed; Tier-A not
triggered. All1229 public sources were freshly verified unchanged. Publication follows.
The [contract](../../superpowers/specs/2026-09-08-callable-type-search-provenance.md)
and [H1 identity requirements](../../superpowers/specs/2026-09-08-callable-identity-domains.md)
define the bounded scope. D1 library searches/beneficiaries follow separately before
value checkpoint one. No default/config/source beneficiary or outside-absence proof
is inferred from a type execution or a zero-event cache result.
