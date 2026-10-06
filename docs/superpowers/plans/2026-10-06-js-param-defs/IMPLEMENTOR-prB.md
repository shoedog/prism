# IMPLEMENTOR dispatch — js-param-defs PR-B callback identity, R1

Start from committed source `origin/proto/js-param-defs-prB` = `0660b3c5c1bb31a90320a74a53e1cd32cf2b3807` plus `~/prism-evidence/js-param-defs/prB/repair-r1/R1-src.patch`. Packet base is `41de010c23d21f90a906a3cccd653b07b09a2e1a` plus `R1-docs.patch` (includes eval changes). The final gate/STOP state is recorded in HANDOFF-repair-prB-r1.md and MEASUREMENTS-prB-R1.md: no measured STOP on the admitted population; clippy warning parity remains unmet (one SMELL). Both exact-base patches apply to pristine controls; independent review/adoption is not claimed. Base for product comparisons is main `da0604b3` (PR-A merged). Review cap remains two rounds; this is the R1 fold into the existing artifact, not a restart or adoption claim.

Read SPEC-prB B-D1–B-D14, §6, §8–§9 and the exact repair brief. Never open frontend-portal, execute a corpus package or perform git writes. Cache: `~/prism-evidence/js-param-defs/cache`. Exact is a static-binding grade.

## Change table and mandatory repairs
| Finding | Site / behavior | Boundary |
|---|---|---|
| F1 | data_flow pass: same-binding nested writes enter RD as kill-only Defs | No emitted Def/row for a nested write; rebound nested names do not kill |
| F2/F3 | ast_callback_identity + data_flow: binding scope for every declaration/write/member base | Both legacy and synthetic JS/TS; no wrong Def-outside-binding edges |
| W1 | reference-position-aware parameters/self/body visibility; body Def scope | Restore default/computed-key captures with original bytes and B8 label |
| W2 | full member-value read-role filter; independent oracle classifier | Plain/destructuring/loop writes are not Uses; compound/update reads retained |
| W3 | runtime enum binder and declaration-name coverage | Matching enums fence; unrelated enum captures remain |
| W4 + S5 | alias target map and raw/twin endpoint maps carry assignment-target start byte | Mandatory 12-row residue elimination; repeated lvalues, not just declarations |
| W5 | adjudicate var/function equivalence only in a proved variable environment | Static blocks/module bodies separated; re-adjudicate every corpus and report changed verdicts |
| W6 | fence-specific validated decoded identifier comparison | Matching escaped binders fence; unrelated escaped captures remain; PR-A D11 unchanged |
| S1 | synthetic spelling surfaces documented in SPEC and MCP docs | `<cb@…>` in taint/Evidence variable output is not a symbol |
| O1 | eval/secbench location witness/frontier using retained byte identities | Never require `callees --location` for anonymous source; named-source behavior retained |

Keep ownership and call-site inventories unchanged. Do not re-own nested callback rows: trace assignment propagation is same-owner. Symbol/call/module nav must be byte-identical; DataFlow `ego`/`nodes-at` changes must map exactly to removed checker-WRONG rows (owner STOP-1(a)). PR-B ships standalone (O2). Member-only support and callback invocation/outgoing-call work remain separate.

## Verification
Run touched tests with full endpoints/labels and a regression per reviewer repro that fails on `0660b3c5`; add mutants per new guard. Run one `cargo nextest run --features mcp`, advisory scoped mutgate, fmt, same-base clippy parity, fresh release rebuild followed by Tier-A matrix 178 and quick VALID. Record failures and environment exclusions; never rebaseline.

Re-measure X, Xi, T and all admitted SecBench packages: ADDED/LOST/RELABELLED/RE-OWNED split CORRECT/WRONG/UNDECIDED with fixed adjudicator. Independently sample at least 40 LOST rows by mechanism. Quantify removal of the reviewer’s up-to-284 X legacy wrong-binding rows. Non-JS byte controls and call sites must be identical. Sample ego/nodes-at/callers/callees/repo-map on X/T, retain outputs and prove each DataFlow change by exact removed-WRONG rows. Measure base/head perf on X/T/lodash. Rerun SecBench standalone and joint-counterfactual under O1.

Freeze production and byte binaries under `prB/repair-r1/bin/` with SHA256 and source manifest. Write `R1-src.patch` relative to `0660b3c5`, `R1-docs.patch` relative to `41de010c`; eval belongs in docs patch. Controller commits; no git writes here.

## STOP and report
Stop on an uneliminable LOST CORRECT row; nav change unexplained by removed WRONG rows; non-JS non-identity; a new WRONG outside disclosed classes. Do not choose a weaker contract. F is controller-only; no F claim is authorized from public measurements.
