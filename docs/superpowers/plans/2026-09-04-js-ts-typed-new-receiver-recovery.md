# Plan — JS/TS typed-parameter and new-constructor receiver recovery

**Design:** `docs/superpowers/specs/2026-09-04-js-ts-typed-new-receiver-recovery-design.md`
**Exact base:** `deca1669947cd42d94d358ae80cb13cde0982750`
**Review cap:** two rounds

1. Convert the existing JS/TS no-recovery residues into discriminating positives for bare typed parameters and direct `new` locals. Add TSX, R3b-collision, same-file clean-class, direct-subset, and cache REDs. Add negative poles for imported/external/interface/generic/union/qualified/factory/annotated/reassigned/captured/after-call/active-vs-ended-scope cases. Run the focused filters before production edits and retain the exact failures.
2. Add a call-position-aware JS/TS receiver evidence query in `src/ast.rs`. It returns recovered typed-parameter/constructor-local evidence, materialized-only value binding, or no variable binding. It must select the nearest reaching lexical binding, fence unrelated nested callables, reject ambiguous writes, and fail closed on parse recovery.
3. Open `classify_simple_ident_mode` only for JS/TS/TSX through that query. Generalize recovered-materialization R3/R3b pre-emption. In `resolve_call_site_full`, route JS/TS recovered types only through caller-file `clean_class_spans` plus `recovered_receiver_direct_method`; all other outcomes fall through to residue.
4. Advance CPG cache 58 to 59 and navigation sidecar 27 to 28. Prove receiver metadata plus Exact edge presence/absence across full, subset, incremental, CPG, and sidecar paths. Update normalized custody dumps if their schema projection includes the changed fields.
5. Run focused GREEN and complete language targets. Conduct review round 1, fix its closed findings, rerun targets, then round 2 at the declared cap. If findings repeat as an open scope/mutation class, park rather than extend.
6. Run format, diff, check, configured Clippy, full default and `mcp` suites with totals, then the required release/Tier-A matrix/rebuild/quick sequence. Compare any failure with exact base in the same environment before attribution. Refresh the handoff at every stable commit.
