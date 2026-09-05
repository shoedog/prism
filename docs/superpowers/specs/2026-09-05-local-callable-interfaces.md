# Bounded local callable interfaces

Approved after PR252 merged04bb5583. Three-round SELF-PASS cap.

## Contract

TS/TSX may recover destructured receiver ownership for direct variable-initializer
arrows/function expressions contextually annotated with a unique module-local
module-private interface containing exactly one call signature. Require a top-level
import/export module marker; reject script globals and direct/list/default interface
exports because external augmentation is not proven by this file-local lookup.
Exported consumers of a private interface remain eligible. Support non-generic inline/local
Props alias parameter types and one plain generic binder substituted only as the
entire contextual parameter type. Preserve original declaration nodes for ordinary
signatures, original use-site argument nodes for generic instantiation, and the
implementation's receiver/write anchors. Explicit annotation failure is terminal.

Reject heritage (including empty bases), merging/duplicate declarations and other
same-name type/import competitors, ambient/imported/nested/qualified interfaces,
overloads, extra members (even optional), signature generics, constraints/defaults/
variance, argument arity mismatch, alias chains, inference, and interface-shaped
Props. Existing defaults/rest/duplicate-property/shadow/write gates stay in force.
Return types do not grant authority. No React.FC/useContext or real-site gain claim.

## Architecture and plan

1. Save base release; add exhaustive positive/negative RED tables. Existing alias
   positives distinguish the missing declaration/body gate from an owner defect.
2. Factor declaration identity into a kind-bounded helper. Props lookup remains
   alias-only. Callable lookup admits aliases or heritage-free interfaces and
   normalizes the original body to a single call_signature; no string substitution.
3. Bump CPG72→73/nav41→42. Cover persisted good↔bad, full/subset/incremental parity,
   declaration/argument-only A↔B owner replacement, sidecars and previous versions.
4. Round2 boundary expansion; round3 source-to-consumer self-review. Full default
   and MCP suites, fmt/clippy, immediate release rebuild before matrix and quick.
5. Pair saved base/candidate on fixed real receiver sites and Python/JS controls;
   separately assert four synthetic gains and eight exclusions. Archive evidence,
   reconcile predecessor records, commit/push/open PR. No merge/rebaseline/multicorpus.

## Hypothesis/probe/result log

Expected RED: interface positives miss Exact owners; existing callable aliases and
new exclusions pass. Alternative owner-route defect is ruled out only if the alias
controls pass in the same environment. Installed parser node-types declare an
interface_body and extends_type_clause. Indexed helper returned SymbolNotFound;
this is not no-callers evidence. Current-source fallback shows the sole contextual
consumer inside js_ts_parameter_receiver_binding and separate alias-only Props use.
Round1: RED32 new positives; two explicit controls and both barrier groups passed.
Existing alias positive control passed; first GREEN3/3. Round2 corrected an unsafe
assumption: top-level is not module-private. Before the guard, script/global merging
minted Exact ownership in full/subset TS/TSX (four failures; module controls pass).
The first module probe stopped at one failure; the complete collector enumerated
all four before production retry. Six export-barrier probes also failed before the
private guard. No prior-main regression claim: these are defects in the proposed
interface implementation. Final results in readout.
Evidence /private/tmp/prism-callable-interfaces-mLbaAN.
