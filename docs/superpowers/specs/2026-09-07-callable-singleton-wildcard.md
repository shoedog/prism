# Bounded singleton wildcard observations

Approved successor to PR273; base `5a4b9b5eee03dc18c1d2b6cc794b0950e3f39b80`.
Schema9 / producer0.10.0. Observation only: no closure, asset-existence or runtime
authority. Existing exact-ambient statuses/reasons and filesystem results stay
unchanged; additive `resolutions[].lookup.wildcard` is independently interpreted.

## Proof requirements

The wildcard field is null unless the actual checker binding has declarations
with one uniquely named, valid single-asterisk module pattern. Otherwise it
records status/reason, the pattern, original-source providers, relevant
augmentations and every matching wildcard provider from the configured Program.
Candidate records are retained for merged/unsupported bindings without promotion.

Observed requires a supported source import/export/import-type/import-equals
literal (including JSDoc import types), null filesystem target, one checker
ValueModule declaration, and identity with one canonical original-source provider.
The declaration must be top-level with a ModuleBlock in a non-external .d.ts.
No exact-name provider, same-pattern duplicate, matching augmentation or second
matching wildcard provider is allowed. Package-ID redirects retain original
parsed bytes/file custody as in PR273. Excluded inventory files are not providers.

Match the raw specifier with TypeScript's case-sensitive single-star prefix/suffix
rule, including its minimum-length check. Pinned5.9.3 source: tryParsePattern
22689–22699; isPatternMatch3660–3662; findBestPatternMatch3638–3650; ambient
resolution54132–54140. TypeScript can select a longest-prefix winner; this slice
deliberately refuses competing matching patterns rather than expanding precedence
authority. A merged binding or augmentation is never singleton evidence.

No asset-path derivation or filesystem existence statement is emitted. Identical
wildcard binding can occur with and without a corresponding asset; inventory
presence and any future acquisition/closure proof remain separate. Both authority
flags, unresolved/refused/outside/diagnostic/class/write barriers stay unchanged.
No React.FC expansion, dependency acquisition or app/config/lock modification.

## Implementation and verification plan

1. Capture packet-behavior RED on exact merged-main before production edits.
2. Reuse the original-source Program census; build one Program-local matching
   pattern index/cache, never a cross-Program positive cache. Separate helper is
   included in producer digest; existing byte/time/heap/array budgets still apply.
3. Strict schema validates new shape, pattern match, Program anchors, positive
   identity/cardinality and unchanged closure barriers before audited-root I/O.
   Full recomputation checks every byte, declaration, pattern and candidate.
4. Negative controls: absent asset, duplicate/merged/competing patterns, concrete
   and pattern augmentations, excluded providers, invalid providers, unsupported
   requests, redirects, tampering and epoch replacement. Prior exact tests stay.
5. Full observer/default/MCP gates; fixed public replay with exact-main projection,
   source/census checks, retained14 residuals/four unproven class candidates.
6. Two SELF-PASS review rounds, NOT INDEPENDENT; no silent extension. Preserve
   evidence, reconcile handoff/roadmap, commit/push/open scoped PR.
