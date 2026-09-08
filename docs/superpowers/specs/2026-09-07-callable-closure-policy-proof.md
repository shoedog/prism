# Closure-policy proof requirements

Audit/specification successor to merged PR276, base52e72bed. No producer/schema,
closure-admission, asset, runtime/class or React.FC expansion is authorized here.

## First finding: repair completeness before considering admission

**WRONG — existing completeness predicate:** with `skipLibCheck:true`, an included
`.d.ts` containing `/// <reference path="./absent.d.ts" />` can yield zero module
requests, zero diagnostics, all closure bits true, and a Props/class candidate
reported observed despite the required declaration file being absent. The file
and failed required-path probe are in the snapshot. Exact base and current output
are identical. Turning skipLibCheck off exposes6053 and withholds the candidate.
Both runtime/class authority flags remain false; no runtime edge defect is claimed.

This is distinct from ordinary failed candidate searches. Do not refuse every
`failed_lookups` entry: optional resolution probes routinely fail before a valid
target is selected. Track required source-reference obligations and their actual
dispositions. The audit preserves a named KNOWN WRONG characterization, not a new
acceptance baseline. A later repair must replace that expectation with genuine RED.

## What current evidence establishes

There are603 null module-literal filesystem outcomes in the fixed public packet:
272 observed exact bindings,231 singleton wildcard observations,84 separate merged
pair observations,14 module source gaps,1 source-bound unsupported `require("fs")`,
and1 augmentation declaration-name request. The last two are not two augmentation
requests. An augmentation's own symbol does not resolve its missing imported target.

Original-source census adds78 path,27 type and112 lib directives outside that
module-literal ledger. A frozen-packet configured-Program cache capture resolves
26 type and112 lib targets but leaves `react-scripts` unresolved at
`packages/excalidraw/react-app-env.d.ts` UTF16/byte[22,35). This is an additional
public closure gap, not one of the14 module-literal gaps. All78 lexical path
candidates are in inventory and Program; that is candidate presence, not a general
path-resolution proof. The actual configuration has skipLibCheck=true and the
normal packet has zero diagnostics. No package/config changes are authorized.

The587 positive observations are source-binding evidence, not a closed dependency
universe. Preserve their actual context and contributor/selection proofs. A
side-effect empty/shorthand pair cannot authorize a value import. Type-only source
identity cannot establish that a declared class has a runtime implementation.

Current264 refusal digests are distinct normalized probe paths, not modules or
request occurrences. `outside_lookups` is a boolean, not a phase/cause/source-path
ledger. Identical outside paths can originate from different requests. Historic
source-backed probe classifications are useful but are not per-epoch admission
evidence in schema10; do not infer harmlessness from those aggregate fields.

## Required proof layers — keep them separate

| Layer | Required evidence | Insufficient substitute |
|---|---|---|
| Captured inputs | Quiescent inventory, safe canonical paths, compiler/options identity, original-source ownership, snapshot stability and limits | A path spelling, last run's cache or final exit status |
| Request coverage | Complete compiler input channels: module literals, triple-slash paths/types/libs, configured/automatic types/libs, config extends, references, redirects | `resolutions[]` alone or zero diagnostics |
| Binding | Per-occurrence actual checker identity and supported context/provider census; duplicates/augmentation/selection barriers | Module name, any symbol, diagnostics, asset inventory |
| Search disposition | Each refused/outside/failed search attributed to phase and request, target/fallback and approved input-universe policy | Digest prefix, outside=false by suppression, or “no successful outside read” |
| Semantic completeness | All required source obligations discharged under an explicit bounded policy; independent barriers retained | Positive binding count or an observed packet with skipped obligations |
| Runtime/class authority | Separate consumer contract, defining/runtime identity and write/duplicate/subclass/opaque-effect barriers | A .d.ts class or complete type-checking inputs alone |

The current `closure.references` only means no configured project references;
it does not certify triple-slash reference completeness. `augmentation` and
`resolution` are both the worker's `reasons.size===0` predicate, not independent
proof systems. The schema parser reproduces these constraints; it cannot add
missing obligations that the producer never recorded. `validate:true` means the
same observation reproduced, not that its completeness interpretation is sound.

## Negative proof requirements

Before any closure admission, establish tests for:

- Skipped/non-skipped missing path/type/lib directives, including in imported
  declaration files, redirects, relative/extensionless targets and excluded files.
- Required missing inputs versus harmless failed search candidates; diagnostics
  suppression (`skipLibCheck`, noCheck/checkJs/directives) cannot erase obligations.
- Binding-positive assets present/absent; transitive declaration gaps; virtual/node
  symbols with refused probes; successful targets with outside metadata probes.
- Unsupported require/dynamic/synthetic contexts and augmentation-name self-bindings.
- Configured/automatic type/lib channels, project references/plugins and config
  inheritance; no unapproved execution or config rewriting to manufacture closure.
- Same normalized probe from multiple sources/phases, source-owned causal anchors,
  overflow/truncation, forged/omitted obligations and same-byte target substitutions.
- All existing receiver writes, duplicate/augmentation barriers and snapshot epochs;
  no closure change may silently promote runtime/class authority.

## Recommended bounded sequence

1. **Correct required path-reference completeness first.** Source-backed census
   and conservative failure for an unproved required triple-slash path, independent
   of skipped diagnostics. Preserve ordinary failed-search behavior. Exclude
   admission/require expansion; add actual type/lib reference evidence separately
   unless the owner explicitly approves combining a bounded enumerable set.
2. Establish complete reference-channel observations and source-backed dispositions
   for any newly exposed public gaps. Never equate package inventory with actual
   type-reference resolution; retain redirects and actual compiler mode.
3. Add bounded request/phase provenance for refused/outside searches, with complete
   reconciliation and a separate proposed hermetic-input policy. No blanket waiver.
4. Only then propose a versioned semantic-closure policy and migration for worker,
   parser, validator and Props/class consumers together. Runtime authority remains
   a separate review/approval boundary. Current14 module gaps must not be hidden
   by resolving other channels or admitting source-backed ambient requests.

## Execution and limits

Source/compiler probes → explicit characterization/base control → fixed public
request/reference census → full observer/default/MCP/helper/authority gates → two
SELF-PASS rounds (NOT INDEPENDENT) → commit/push/PR. This audit's characterization
tests pass unchanged production; they are not implementation RED. Keep the known
WRONG visible through closeout. No fixes to applications, installs, lock/config
rewrites, no general closure expansion, no silent review-cap extension.
