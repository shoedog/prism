# Reviewer dispatch — exact caller read occurrences

Review the frozen candidate read-only against
`docs/superpowers/plans/2026-09-18-post317-next-increments/specs/01-exact-caller-read-occurrences.md`.
Bind the exact candidate/base commits, trees, source manifest and evidence hashes before
inspection. Review cap is two rounds. Do not edit candidate production or permanent tests.

## Finding discipline

Label each finding `WRONG` or `SMELL` before explaining it. Put WRONG first.

- `WRONG` requires a constructible input/state, the incorrect output, source mechanism,
  bounded fix and realistic failing regression.
- `SMELL` is a risk, ambiguity, evidence gap or maintainability concern without demonstrated
  wrong output; it never blocks by itself.
- Use same-environment unchanged-base controls before attributing regressions. An invalid
  probe changes no belief.
- Report source behavior, internal producer facts, legacy DFG, CPG and public query behavior
  separately.

## Load-bearing review questions

1. Does the new exact endpoint/edge identity include bytes in its own Eq/Ord/Hash while
   global `VarLocation`, `FlowEdge`, legacy edges/labels/adjacency and query signatures remain
   unchanged, including primitive byte payloads and multiplicities?
2. Is exact confidence produced by the existing RD solve before legacy label reduction, once
   per function rather than once per edge? Can any later edge borrow the first legacy label?
   Are only supplemental pairs absent from the legacy exact endpoint population persisted,
   with their count equal to distinct newly materialized pairs?
3. Does the admission gate accept only named owned JS/TS/TSX callables, plain parameters or
   unique non-aliased earlier-line locals, real nonzero simple identifier Uses, and permitted
   sequential syntax?
4. Do writes, redeclarations, aliases, shadows/captures, same-line locals, control flow,
   loops, members, recovery/reflection and ambiguous owners preserve old output without new
   exact expansion?
5. Does Step 4 consume only real producer facts, validate the exact endpoints, deduplicate
   the same pair and refuse/diagnose a label conflict without changing legacy rows? Probe a
   retained CPG node and Step5b edge with the producer fact removed.
6. Do exact facts survive full/subset/merge and warm cache, revoke/restore correctly on file
   removal and caller-only incremental edits, and remain deterministic? Is genuine cache96
   rejected before a cache97 rebuild/Hit while nav53 remains justified?
7. Are all exact assertions primitive tuple comparisons rather than byte-insensitive
   `VarLocation` equality, `FlowEdge` equality, or `labels.get` aliases?

## Independent controls

Independently replay at least:

- original O01 JS/TS/TSX base RED and candidate NameOnly rows;
- multiline parameter and prior-line local Exact positives;
- changed-later-binding mutation plus wrong-byte/path/owner forgeries;
- one write, shadow/capture, loop, member and non-JS preservation case;
- full/subset/incremental/warm parity and genuine old-cache rejection;
- global `VarLocation` identity and legacy first-wins query compatibility;
- exact/legacy same-pair conflict and missing exact-label refusal.
- direct `remove_files` deletion/untouched-file parity and supplemental fact cardinality.

Use isolated archives/targets and preserve raw logs/harness source. Do not substitute a
test-only exact map for the production producer, a CPG fanout for a missing DFG fact, or a
simulated cache version for genuine predecessor bytes.

## Evidence and verdict

Verify the seven proof groups, source manifest, focused totals, full Rust/non-Rust totals,
Tier-A status, exact exclusions and bounded 10/100/1,000 synthetic cost report. Historical
real JS/private inputs remain `input-blocked`; no recall claim follows from synthetic rows.
Confirm the implementation stayed within the 700 non-test production / 1,600 total changed
line guardrails or was explicitly re-sliced before freeze. Verify Tier-A used the actual
`--sut-bin <frozen-path>` binding and an immutable/locked quick corpus.

At round 1, return the complete finite finding population before requesting changes. At round
2, assess the versioned repair and give one final source verdict. If findings require global
identity, public-query, same-line-write, kill/alias, loop/member or owner expansion, classify
the design as open-class and park it rather than extending scope.

End with exact totals:

```text
WRONG: <n>
SMELL: <n>
VERDICT: APPROVE | FIX-FIRST | PARK-DESIGN
```
