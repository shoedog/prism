# S6 — adopted strict semantic-completeness contract

Goal: a separately versioned configured-Program TYPE-SOURCE completeness observation,
not filesystem/asset/runtime completeness. Keep all historical packet.status/reasons,
closure bits, Props/class statuses and both authority flags unchanged. No denied-space
waiver, no outside-root read or policy inferred from event ownership. This is an
additive successor observation, not a reinterpretation of historical closure.

S6 publishes the contract and negative proof requirements first. An unwired pure
classifier may be included if its review remains bounded. S7 wires the first version
with singleton exact ambient eligibility; S8 and S9 separately add singleton wildcard
and supported merged side-effect pair eligibility. Production/runtime consumer remains
unwired; S10 specifies its authority requirements only. Strict policies must have
constructible positive cases under unchanged outside/refused barriers before adoption.

Successor semantic_closure:{policy,complete,reasons,rows}. Policy ID pins the admitted
binding kinds, never caller/packet-selected resolution options. Every sorted legacy
resolutions row gets exactly one {index,disposition,reason} in the same order. Candidate
dispositions: filesystem_selected, exact_ambient, singleton_wildcard,
merged_side_effect, unproven. No target fabrication or loss of repeated occurrences.
Filesystem_selected requires nonnull captured target in snapshot.program_files.
Semantic dispositions require targetnull and the corresponding existing exact
observed binding lane; unsupported/duplicate/augmentation/context/selection guards
are inherited, not reconstructed by name. Unproven retains explicit unsupported or
target-not-in-Program reason; global completeness requires every row discharged.

Independent conjunction, all mandatory:
- verified pinned compiler and stable snapshot;
- observed S4 config provenance, S5 entry_obligations.complete;
- noResolve must not be explicitly true (derived only from observed seven-option
  value digest); no silent unvisited-source admission;
- no compiler/config diagnostics, unsupported project refs or plugins;
- no unproven required path or source type/lib obligation;
- no outside_lookups, no refused lookup digests, no boundary_events, regardless of
  owner or whether their request selected a target;
- all existing global refusal reasons remain barriers EXCEPT unresolved_module,
  which is replaced ONLY in this new field by exact per-occurrence row coverage.

Do not use old packet.status or old resolution/augmentation closure as the new
completeness predicate: they intentionally remain false for targetnull. Conversely
do not change them when new semantic completeness is true. Diagnostics suppression
does not suppress required source/entry obligations. Automatic discovery unknown
stays S5-incomplete even with zero names or all observed names. Eventless cache hits
cannot discharge unproven rows. Lib search/beneficiary counts are not obligation
counts. Semantic provider evidence cannot certify asset presence or runtime export.

Parser structurally validates the fixed policy and recomputes exact rows/reasons/
complete from already validated packet facts; producer uses the same pure no-I/O
classifier after final snapshot and stable resolution sorting. Full reproduction
authenticates source/compiler facts and actual callback census. Include helper in
producer digest only when wired at S7; preserve all prior schema contracts.

Independent source/design review concluded at round2; vocabulary is frozen below.
Negatives: skipped missing path/type/lib/module; explicit noResolve; automatic
zero-name or resolved-name universe; outside peer metadata after successful target;
refused virtual probe despite exact binding; cache-hit unresolved zero events;
missing lib fallback/no-beneficiary; projectrefs/plugins/configunproven/diagnostics;
out-of-Program filesystem target; semantic targetnull with competing provider,
augmentation, unsupported request, wrong selected contributor, or omitted row;
positive semantic binding plus unrelated missing peer; same-genuine substitution;
asset present/absent must not alter type-source eligibility or runtime authority.

Value checkpoint after S7: report eligible rows versus complete Programs versus
runtime edges separately. Fixed public still has react-scripts/14 gaps/outside/refused
barriers, so no public complete-Program or receiver-recall gain is expected. If only
synthetic fixtures gain completeness, say so and reassess remaining scope; never
waive public barriers to hit a recall target.

## Adopted strict direction and closed S6 helper contract

Primary adopts STRICT retained boundaries, not a denied-space waiver. Source-backed
eight-case matrix on frozenS4 shows exact node:known/virtual:known (without baseUrl),
relative wildcard and supported merged pair have normal reproduced packets with
zero boundary events, clear diagnostics and complete derived S5 entries. Plain and
scoped package-like exact names have outside encounters and stay blocked. TS45062–66
returns early without baseUrl;45300–18 skips URI-like nonrelative names before ancestor
node_modules, while relative45327–38 stays local. Spelling is not authority: eligibility
still requires actual singleton binding and all conjunctions, not a prefix branch.

Independent architecture round1 ACCEPT94, WRONG0, one closed vocabulary SMELL. Freeze:
policy prism.semantic-closure/exact-ambient-v1. Aggregate reason order:
compiler_unverified,unstable_snapshot,config_unproven,entry_obligations_incomplete,
no_resolve,compiler_diagnostics,unsupported_project_references,unsupported_plugins,
required_path_unproven,type_lib_unproven,boundary_encounter,global_refusal,
resolution_unproven. Row reasons null for selected/admitted; otherwise
target_not_in_program,unresolved_module,unadmitted_binding.

S6 may add an UNWIRED dependency-free classifySemanticClosure(input,cap=100000).
Input is already validated compiler facts:
{compilerVerified,stableSnapshot,configObserved,entryComplete,noResolve,
diagnosticCount,globalReasons,outside,refusedCount,boundaryCount,programFiles,resolutions}.
Booleans required; counts safe nonnegative; programFiles array of strings;
resolutions array of existing validated resolution/lookup records; globalReasons
array of existing schema reason strings. Cap1..100000, checked before row building.
Function fixes policy exact-ambient-v1 in source, no caller policy selector yet.
Output {policy,complete,reasons,rows}; rows {index,disposition,reason}.

Row precedence: nonnull target in Program => filesystem_selected, absentProgram =>
unproven/target_not_in_program. Null target + exact lookup.status observed =>
exact_ambient. Null target + any observed wildcard/merged lane =>
unproven/unadmitted_binding. Other null target => unproven/unresolved_module.
Never clear an outside/refused/global barrier because rows qualify.

Map global reasons compiler_diagnostics,unsupported_references,unsupported_plugins,
unproven_path_reference,unproven_type_lib_reference,unstable_snapshot,compiler_mismatch,
outside_lookup,unsupported_lookup to their matching aggregate categories in addition
to the explicit evidence checks. Any OTHER existing reason except unresolved_module
adds global_refusal (including invalid_config,worker_failed,budget_exceeded,
unsupported_input). This prevents duplicate generic labels for already categorized
reasons while ensuring every old refusal except per-row-resolved unresolved_module
remains a barrier. compiler_mismatch maps compiler_unverified. noResolve contributes
only when configObserved && noResolve (unobserved config already blocks independently).
complete iff no aggregate reasons. Unknown global reason rejects helper input rather
than being ignored. No top-level code imports this helper until S7 wiring.

Enforce unchanged-worker consistency: globalReasons includes unresolved_module iff
at least one resolution target is null. Either mismatch is invalid helper input,
not a vacuously complete proof. This guard applies to the new S7 field only; do not
retroactively rewrite historical packet validation. Omitting a whole genuine request
while another remains can still need full reproduction for actual census authority;
parser structure is not a claim of original-source callback completeness.

Helper unit controls are new-contract tests, not claimed behavioral RED on a producer.
Cover every reason/row branch, duplicates/order, exact positive with oldunresolved,
unrelated nullrow, wildcard withheld, alloutside/refused/ownerless/eventlesscache cases,
falseconfig/noResolve, cap preflight and invalid inputs. S7 must separately capture
actual producer RED against frozen S5 before worker/schema edits and authenticate
all input predicates from current packet/config facts. S6 does not bump producer/schema
or include the unwired helper in current digest.
