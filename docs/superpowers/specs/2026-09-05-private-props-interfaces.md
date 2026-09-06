# Bounded module-private Props interfaces

Owner approved after PR253 merged eb884824. Branch feat/private-props-interfaces;
three-round SELF-PASS cap, no subagents. Cleanup separately authorized and limited
to target/debug/incremental (5.2G); source/evidence preserved, df13GiB afterward.
Later separate approval covered30 Tier-C caches and six old Prism target dirs;
no active-use references found, exact canonical paths checked, deleted36 dirs,
parents/evidence/source preserved; df10→86GiB. Nothing else removed.

## Contract

Recover TS/TSX destructured receiver ownership from a unique module-private,
non-generic interface Props with own property signatures, through explicit parameter
annotations and every already-supported contextual signature form. Required selected
property names a directly proven class; unrelated optional/readonly properties follow
existing object-shape rules. Preserve original property declaration nodes, argument
lookup nodes, implementation binding/write positions, and explicit annotation priority.

Reject script globals, nested/ambient/imported/qualified/exported interfaces,
heritage (even empty bases), merging and competing same-name declarations/imports,
generic Props/binders/inference, alias chains, overloads/call/method/constructor/index/
accessor members, selected optional/duplicate/computed properties, destructuring
defaults/rest/nesting, scope shadows and receiver/member writes. Existing callable
signature/binder restrictions remain. No React.FC, hook or imported type expansion.

## Architecture and plan

1. Save untouched base release; RED positive interfaces against alias controls in
   the same environment. Migrate obsolete Props-interface negative rows only.
2. Generalize the existing local callable declaration/body helper's name and reuse
   its unchanged private-interface/heritage proof for Props. Props consumer rejects
   any generic declaration and allows only object_type/interface_body. Existing own
   property traversal proves shape; no recursive type evaluation or string cloning.
3. CPG73→74/nav42→43; persisted valid↔invalid transitions, declaration-only A↔B
   owner replacement across explicit/contextual forms; full/subset/incremental and
   sidecar parity plus old-version misses.
4. Round2 adversarial boundaries; round3 source-to-served consumer self-review.
   Full default and MCP suites, fmt/clippy; immediate release before matrix and quick.
5. Paired saved base/candidate measurements on fixed real sites and Python/JS controls,
   separate synthetic served gains/exclusions. Archive, commit/push/open PR, reconcile
   custody. No merge, baseline rewrite or full multicorpus.

## Hypothesis/probe/result log

Expected: interface positive fixtures lack Exact ownership on base, while equivalent
local Props-alias controls pass; opposite results falsify the missing-shape hypothesis
or implicate the alternative class-owner route. Both are checked before editing.
Indexed props helper unavailable (SymbolNotFound is not no callers); current source
shows one props-shape consumer in js_ts_inline_prop_receiver_type.
RED:46 positive misses plus four explicit-module comparisons; alias controls pass.
GREEN4/4; round2 receivers26/26; cache groups2/2 plus2/2, including32 new owner-change
transitions across eight supported forms. Round2 export-value probe: TypeScript
rejects importing the same-spelled exported value as a type (TS2749), so that form
does not expose the private Props interface; no guard broadening needed.
Evidence /private/tmp/prism-props-interfaces-JPIR2j. Initial test patch failed atomically
due to hunk order before RED; source reread and patch corrected, not product evidence.
