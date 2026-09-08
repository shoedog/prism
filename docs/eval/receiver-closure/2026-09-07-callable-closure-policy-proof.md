# Closure-policy proof audit — 2026-09-07

> Historical exact-base evidence. The known path defect is addressed by the
> [authorized bounded repair](../../superpowers/specs/2026-09-07-callable-required-path-completeness.md);
> type/lib observations and react-scripts disposition remain separate work.

**Repair completeness before considering closure admission.** This audit changes
no production worker, schema, resolver or authority. Base is merged PR276,
`52e72bed7d0685cf0a0901d053563e067dc64387`.

## WRONG: skipped required path can falsely report complete closure

An included `reference.d.ts` containing
`/// <reference path="./absent.d.ts" />` and an ordinary local Client/Props callback
can produce zero module requests, zero diagnostics, all closure bits true and a
class candidate marked observed when `skipLibCheck:true`. The required file is
absent and the failed path probe is retained. Same-environment exact base and
current packets are equal. With skipped checking disabled, TS6053 independently
withholds the candidate. Runtime/class authority remains false in both cases.

The KNOWN WRONG fixture intentionally characterizes this existing defect; it is
not an acceptance baseline or implementation RED. No fix is smuggled into this
audit. A separately approved repair must flip the expected false closure first.

Mechanism: worker.mjs144–150 derives completeness from enumerated reasons;
`closure.references` only rejects configured project references. Triple-slash
inputs do not pass through the module-literal observer, and safe failed reads mix
ordinary fallback candidates with required references. Pinned TypeScript5.9.3
`typescript.js`22819–22820 and127988–127997 suppress Program diagnostics under
skipLibCheck;128788–128815 processes path/type directives through separate channels.
Rejecting every failed lookup would wrongly reject harmless candidate searches.

## Fixed public-source result

The normal packet is byte-identical to PR276's packet. All1445 Program files were
re-read and hash-checked against original source, and all603 null-target request
coordinates were checked. Their partition is272 exact +231 singleton +84 merged
+14 no-source module requests +1 unsupported `require("fs")` +1 augmentation name.
The final two are not two augmentations; an augmentation's own name symbol is not
the missing imported target. Positive binding count587 is not completeness.

| Additional source channel | Occurrences | Bounded finding |
|---|---:|---|
| Triple-slash path | 78 | All lexical candidates in inventory and Program; not a general resolution proof |
| Triple-slash types | 27 | Compiler cache resolves26; react-scripts unresolved |
| Triple-slash lib | 112 | Compiler cache resolves112 |

The additional unresolved type request is in
`packages/excalidraw/react-app-env.d.ts`, UTF16/byte[22,35), source SHA256
`57eda4c4c04a1dca45c62857326882ce9cc948c4b52973c0e3c3b7e4c3fa3990`.
It is **outside the14 module-literal gaps**. The actual config has skipLibCheck=true;
zero diagnostics therefore does not show this input exists. This audit makes no
claim about whether the application's maintained configuration should change.

The diagnostic copy freezes the packet before querying the actual configured
Program's mode-aware type-reference cache and resolved lib cache. All other packet
fields match normal production after removing only the copied producer digest.
The139 cache rows reconcile exactly to original-source occurrences. The packaged
capture helper reproduces identical cache bytes. This is compiler-backed evidence,
not an independent compiler oracle or a general cache-authority certificate.

264 refused-path digests and outside_lookups=true remain. Historical operation
classifications in the [earlier audit](2026-09-07-callable-closure-classification.md)
are inherited, not newly recaptured here. A digest identifies neither initiating
request nor phase. Four class candidates remain unproven;30 observations and53
nested calls remain unchanged. No frontend-portal remeasurement, install or source
edit occurred. Asset presence, semantic closure and runtime authority stay separate.

## Hypothesis / probe / result

1. Initially expected missing directive diagnostics in skipped declarations.
   Two new expectations failed (10pass/2fail); these refuted the assumption, not
   production regressions. Alternative: the declaration never entered Program.
   Program membership plus failed required-path evidence excluded that alternative;
   toggling skipLibCheck exposed6053/2688, matching pinned compiler source.
2. Could this be introduced by PR276 or this audit? Exact merged-base/current
   control produced identical packets for path/types with both skip settings.
   Missing types retained an independent outside_lookup barrier; only the path
   case established false closure. Do not generalize the counterexample.
3. Could matching counts hide changed public evidence? Exact normal packet digest,
   non-producer field equality for instrumentation, source-before/after equality
   and full source/cache multiset reconciliation rule out that alternative within
   this fixed snapshot. No claim of complete automatic/config input coverage.
4. First SELF-PASS found two bounded audit-verifier WRONGs: a different genuine
   Program target could replace a cached target, and config whitespace could retain
   a stale reported hash. Fixed by pinning the captured cache bytes and checking
   the actual config bytes. Three controls pass: unchanged capture accepted,
   genuine target substitution rejected, same-options changed config rejected.
   These are verifier controls, not a production closure repair or base RED.
5. Second SELF-PASS checked source/capture custody, scope, test totals and next
   boundary. No additional audit WRONG/SMELL. Two-round cap not extended; reviews
   are NOT INDEPENDENT. One known production WRONG remains explicitly open.

## Verification and custody

- Characterization13/13 on current and exact base; full observer277/0/0 skipped.
- Full Rust default4017/0/1 ignored; MCP4207/0/1 ignored, including doctests.
- Helpers7/7; pinned authority40 results, failures=[]; audit controls3/3.
- Normal packet validates true/unproven; source manifests equal
  `353187a695df2683a3631e4739c173cb6d33190901093549b61e28efeda60cbb`.
- No Tier-A trigger: no call graph/AST/navigation/CPG source changes. Human-triggered
  full multi-corpus evaluation was not run. No runtime-resolution improvement claimed.
- LSP-navigation skill required a pinned compiler/source fallback because no LSP
  tools were exposed. This was not a Prism-navigation result.

Packet SHA256:`19871f6f359422afd029c22a0f1109da6d363d5b5b98e4a49ca429f13629b78a`.
Producer0.11.0/schema10 unchanged. Compiler and full217 directive anchors, null
partition and same-environment reference controls are committed in the adjacent
evidence JSON; log digests and archive custody are in the gates JSON.
Task root:`/private/tmp/prism-closure-policy-proof-OHzYXp` (raw files local-only).

Reproduce the diagnostic capture using Node24 and an options JSON from the ordinary
installed/in-root settings; keep all output outside the audited source:

```bash
node docs/eval/receiver-closure/capture-closure-reference-cache.mjs /path/to/options.json /path/to/task-root
node docs/eval/receiver-closure/audit-closure-policy-proof.mjs /path/to/pinned/typescript.js /path/to/source /path/to/normal-packet.json /path/to/copy/packet.json /path/to/copy/reference-cache.json
node docs/eval/receiver-closure/verify-closure-policy-proof-controls.mjs /path/to/pinned/typescript.js /path/to/source /path/to/task-root
```

The verifier deliberately pins this snapshot/capture, not arbitrary repositories.
It does not authenticate arbitrary instrumentation; source/compiler review and
capture reproduction establish provenance separately.

## Next bounded slice

Required-path completeness repair first: census original-source required path
directives and fail closed on an unproved target independently of diagnostics.
Keep successful fallback searches, duplicate/write/cache barriers, ordinary source
membership and validation consistency. Then separately observe type/lib channels
and classify the react-scripts gap. Broader closure-policy approval comes later;
no ambient admission, asset authority or React.FC expansion now. The
[specification](../../superpowers/specs/2026-09-07-callable-closure-policy-proof.md)
enumerates proof layers and required negative fixtures.
