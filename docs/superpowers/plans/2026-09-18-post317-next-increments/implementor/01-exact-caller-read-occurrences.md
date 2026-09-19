# Implementor dispatch — exact caller read occurrences

Implement only
`docs/superpowers/plans/2026-09-18-post317-next-increments/specs/01-exact-caller-read-occurrences.md`
on the controller-provided isolated worktree based on accepted merged PR317. Read the spec,
the adjacent candidate brief, the independent architecture receipt supplied by the
controller, and applicable `AGENTS.md` completely before editing.

## Scope

Recover caller producer edges for authenticated byte-distinct same-line reads of one
uniquely defined, unaliased bare identifier in an existing named JS/TS/TSX callable.

- Admit existing parameter entries, including the original one-line O01 parameter case.
- Admit a unique non-aliased local declaration only when it is on a strictly earlier line
  than every newly expanded read.
- Use a conservative whole-callable gate for branches, loops, try/switch,
  conditional/short-circuit evaluation, nested callables/captures, recovery, reflection and
  owner ambiguity.
- Use per-binding refusal for writes, update/compound assignment, redeclaration, shadow,
  alias/destructuring participation and member paths.

Do not change global `VarLocation` Eq/Ord/Hash, `FlowEdge` equality, legacy DFG edges,
labels, forward/backward maps, public query signatures, kill/alias rules, or legacy
representatives. Do not add statement-order, loop/member, capture, alias, optional/default,
owner or compiler authority.

## Required implementation shape

1. Add an internal byte-keyed exact endpoint and edge identity. Classify all admitted pairs,
   check overlap consistency, then persist only supplemental pairs absent from the legacy
   exact endpoint population, with one independent confidence per pair.
2. Build an owned callable-local span index once. Authenticate identifier role, binding,
   path, owner and bytes; deduplicate identical raw rows. Keep public line-reference helpers
   unchanged and avoid a source-wide scan per edge.
3. Classify exact candidates within the existing per-function RD solve before the legacy
   label map collapses byte-distinct endpoints. Do not copy the first legacy label or rerun
   the solver per edge.
4. Complete `empty/build/build_subset/remove_files/merge`, deterministic serialization,
   incremental revoke/restore and cache lifecycle for the exact state.
5. Let Step 4 consume a deduplicated union of legacy rows and exact facts through the existing
   exact endpoint index. If exact and legacy views disagree for the same exact pair, refuse
   and diagnose the new binding expansion while preserving every legacy row/label. Never
   combine upward, fan out under a line key, or emit duplicate label variants.
6. Bump CPG cache 96→97 because serialized DFG state changes. Keep navigation53 unless a
   separately reviewed navigation fact change proves otherwise.

Owned production seams are limited to `src/ast.rs`, `src/data_flow.rs`,
`src/cpg/reaching.rs`, `src/cpg/build.rs`, and
`src/cpg_cache.rs`. Add focused test files/modules as described by the spec. Stop and return
to design before changing any public identity, solver equivalence, unsupported owner domain,
or source outside those seams. Minimal classifier-result factoring in the top-level
`src/cpg/reaching.rs` is allowed only to reuse the single existing per-function solve. Within
the `src/cpg/reaching/` subdirectory, only test modules are pre-authorized; production
binding/kill modules remain excluded without a separately justified helper move.

## Fail first

Before production edits, add public behavior assertions for the complete R01–R05 population
from the spec and apply the identical patch to a saved merged-base archive in the same
environment.

- The existing PASS probe is characterization only. RED requires the new assertion that the
  later caller edge exists with its exact bytes and independently classified label.
- Capture all JS/TS/TSX rows, not only the first failure in an aggregate loop.
- Assertions must use primitive endpoint fields, confidence and multiplicity. `FlowEdge ==`,
  `VarLocation` sets and byte-insensitive label lookup are inadmissible exact proof.
- Negative/preservation cases must pass base and candidate and compare complete old rows.
- Include the adversarial retained-node/missing-producer control, representation-aware absent
  fact/label and wrong-key cases, and the same-pair conflict that refuses only the target
  binding while an unrelated eligible binding remains positive. Step 4 must not recreate or
  relabel an edge.

Stage public desired-flow REDs and preservation tuples on base first. Add tests that mutate
the new crate-private table only after that seam exists; compile failure on base is not RED
and those tests do not substitute for behavioral base evidence. Directly test `remove_files`
with one deleted/changed and one untouched file.

Before every diagnostic probe, record hypothesis, expected result, falsifier and a plausible
alternative. A compile failure, zero-test filter, bad fixture or environment refusal provides
no behavioral evidence. Run an unchanged-base control in the same environment before
attributing any failure.

## Completion and review

Complete all seven proof groups before the first review freeze: one-line NameOnly,
multiline/local Exact, owner/raw identity, mutations/forgeries, exclusions, lifecycle/legacy
compatibility, and genuine cache/bounded cost. Freeze a source manifest and compact evidence
receipt. The controller owns commits. Review cap is two; apply finite findings to the same
artifact and stop on open-class identity or solver expansion.

Target at most 700 non-test production changed lines and 1,600 total source/test changed
lines. Stop for controller re-slicing before review freeze if either estimate or actual diff
exceeds the guardrail; do not compress evidence or broaden files to fit.

After source acceptance, run the full verification population in the spec and report exact
totals, base controls, retries and exclusions. Rebuild release immediately before each
Tier-A matrix/quick command and pass `--sut-bin <frozen-path>` (or a verified command-builder
equivalent); freeze the corpus/worktree or hold a coordination lock for quick. Do not run full multi-corpus or rebaseline. Historical real JS
and private corpora are `input-blocked`, not zero-yield evidence.

No push, PR, merge, publication, adoption, cleanup or successor work is authorized by this
dispatch.
