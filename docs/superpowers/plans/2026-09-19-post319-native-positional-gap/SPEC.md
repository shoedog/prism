# Native positional-gap characterization of twelve observed sites

Status: planning only, corrected after independent planning round1; final round2/cap2 pending. No implementation or publication of another feature is authorized by this document. The user's current request is a detailed successor plan after PR319's green merge. Implementation, if subsequently authorized, has a separate two-round review cap; controller declares that dispatch.

## 1. Decision and value

Build a small opt-in research executable and its bounded Node launcher. Characterize **exactly the twelve existing compiler-observed callables with an object-pattern parameter before a later ordinary required identifier**. Report the current native function/name inventory, raw formal-parameter ordinals, existing positional-slot prefix and existing binding occurrences. This answers whether the measured syntax has corresponding native AST API observations, and identifies the next proof needed. It does not add parameter support.

PR319 measured355object-pattern parameters,23array,13rest and2unparenthesized arrows in414files. Its destructured-before-later-optional count was0. A deterministic projection of the saved packet identifies12object-before-later-required sites, all ArrowFunction, across11files/37,040bytes (25compiler parameters). Five selected object parameters have nested defaults; seven do not. Frequency is a selection rationale, not evidence of resolved calls, CPG entries, safe slot-hole admission, runtime demand or implementation priority by itself.

A direct production repair is premature: native slots truncate at unsupported patterns, but function-name inference alone cannot prove any selected callable has a usable exact callee or entry Def. The deliverable chooses **bounded next proof for entry/call behavior** or **defer with reason**, never “implement support.” Do not revive the parked whole414/native/dual-parser census or field-provenance design.

## 2. Immutable predecessor and input custody

- PR319 merge `30e13053c9f3f9940b5526e20af0cb40f2a9fae4`, tree `961f698db1049cec42e7a8e15e716948082af9b6`; main-push CI35473373053 all green. Controller receipt SHA256 `46c0f34a04d389c5628d3c2663b025c55012e3e1fc791c4c0413630ea7688d20`.
- Planning checkout `/private/tmp/prism-post316-slice1`, branch `plan/post319-successor`; docs checkpoint `ee49d721`. Implementation must be rebound to the controller's eventual planning commit, with production/script equivalence to the merge checked.
- Excalidraw source commit `0642e72cfa2d9a71198200e52f37399384610ee3`, tree `709e9146b0fbd78c3ebf0d77e67143b2fbc43e4a`; existing authenticated root `/private/tmp/prism-post317-measurement-inputs/source`.
- Parent414 input manifest SHA256 `f8ebbdd79ca01e5cdb675616b913f1fbb549d45378ab1047db84844a3bc8e696`; saved syntax packet SHA256 `e953121bbcdda492bb651e4ef8f27b9af3de7a0221ee2a98fcda611645a7896d`.
- Normative derived `SITE-MANIFEST.json` SHA256 `789352a575d68ef672de7449299676de0c0aab8edd4abd8228e23efda65d326b`. Exactly12sites/11unique files/37,040bytes. It includes both parent hashes, upstream identity, complete selected file hashes/bytes and exact callable byte selectors. Derivation is a projection of saved JSON, not a fresh public parse.
- Ten synthetic strings/hashes are in `BASELINE-FIXTURES.json`; corresponding current native observations are in `baseline-output.log` and `BASELINE-RECEIPT.json`. These30dialect cases are **passing characterization, not RED**. The small external probe linked the existing f79 native rlib; `git diff f79bb954 30e13053 -- src Cargo.toml Cargo.lock build.rs` is empty. No public native characterization has run during planning.

Do not install dependencies, run upstream scripts, use TypeScript Program, resolve packages, or acquire private input. Reuse existing locked Rust crates and Node built-ins. Do not load the compiler or rerun the414syntax census. Other files in the public root remain out of the selected read closure; their existence is permitted, not a reason to broaden the census.

## 3. Source seams and unchanged contracts

Relevant current seams (line numbers are navigation hints, not immutable identities):

| Seam | Current behavior / permitted use |
|---|---|
| `src/ast.rs:415,438` ParsedFile::parse | Explicit path/source/language, parse_error_count; parse each selected file once. |
| `src/ast.rs:605,792` all_functions/functions | Existing native function inventory; exact byte spans. Use all_functions for selected candidates, not enclosing-name guesses. |
| `src/languages/mod.rs:1154` function_name | Assigned and one-wrapper arrow name inference; recorded name is metadata, not callee resolution. |
| `src/ast.rs:10507–10547` occurrence/slot APIs | Public function_parameter_occurrences and function_parameter_slot_occurrences have different contracts. Record both faithfully. |
| `src/parameter_slots.rs:325–421,437–480` | Whole-list duplicate guard and positional prefix stopping at object/array/rest. No edits. |
| `src/cpg/build.rs:155–239` compute_param_def_nodes | Exact callee and entry-Def checks are downstream and deliberately unmeasured. |
| `src/cpg_cache.rs:217`; `src/navigation/call_edge_cache.rs:97` | CPG97/nav53 stay unchanged; no caches are built/read/written by this tool. |

Do not alter those seams. Never enumerate destructured leaves as separate arguments or replace native slot refusal with guessed holes. Preserve JS/TS differences, unsupported/default/rest forms, aliases, duplicate names and nested-scope behavior in reported observations. A native occurrence tuple is **not** a CPG entry node. A name is **not** a resolved owner/callee. No call graph, CPG, DFG, reaching-def, public-flow or production admission changes.

## 4. Owned paths, architecture and budget

If later implemented, own only:

- `examples/parameter_slot_characterization.rs`: pure native request/observation worker using current public APIs; stdout JSON, errors stderr/nonzero.
- `scripts/parameter-slot-characterization/index.mjs`: importable launcher, input/source/binary authentication, bounded child execution and atomic new-only output.
- `scripts/parameter-slot-characterization/index.test.mjs`, and narrowly colocated cfg(test) worker assertions if useful.
- `scripts/parameter-slot-characterization/README.md`; `docs/eval/native-positional-gap/{site-manifest.json,receipt.json,readout.md}`. Large output/log custody can be external with hashes.

No src/, Cargo/lockfile, dependency, existing observer, compiler, source-root or cache changes. Do not share mutable code with the parked artifacts; preserve them unchanged. No generalized schema engine, graph IDs or reusable framework.

**Hard caps450helper/450tests/900combined executable lines**, counting Rust and Node helpers together, and all assertion/fixture code including cfg(test). No executable code hidden in docs. Allocation: native parse/inventory/APIs/formatting230–280; launcher custody/limits/publication100–130 =330–410helper; complete tests280–360. Early stop before430helper or430tests if forecast exceeds cap. Preserve snapshot and return bounded options; no silent inflation, new split or restart.

Use the Node launcher's spawnSync timeout/output cap; the synchronous Rust parser cannot enforce a Node-only event-loop timer. Source bytes are authenticated in the launcher, decoded with strict UTF8/BOM preserved, then sent to the native worker in one request. The worker parses those retained strings, never rereads source files or searches the repository. The worker has no filesystem/source-loader side effects. Parent writes successful output using unique parent-owned staging plus atomic new-only link; never delete another writer's requested output on failure.

## 5. Small request and output contract

CLI:

`node scripts/parameter-slot-characterization/index.mjs --root <upstream-root> --manifest <site-manifest> --manifest-sha256 <expected> --native <built-example> --native-sha256 <expected> --out <new.json>`

All flags required exactly once; unknown/duplicate/refused input fails nonzero, no successful packet. Validate manifest hash before use and worker binary hash before launch. A build receipt binds that binary to source; a user-supplied expected binary hash alone does not prove its source commit.

Manifest schema is exactly `prism.native-parameter-sites/1` as frozen. Require all top-level fields, arrays and member/site scalar types; safe canonical relative paths (no absolute/backslash/control/dot/dotdot/empty components); lowercase64hex hashes; safe integers. Validate exact filename→manifest/request script_kind→native Language/output language mapping: `.js→JavaScript→JavaScript`, `.jsx→Jsx→JavaScript`, `.ts→TypeScript→TypeScript`, `.tsx→Tsx→Tsx`. Reject any script_kind disagreement before parsing; do not trust or silently ignore the supplied label. This mapping selects the existing native parser; it is not a compiler/Program claim. Member list is unique and exactly the set of paths referenced by sites, with counts/byte sum consistent. Sites are unique `(path,start_byte,end_byte)`, start<end within file, compiler_kind=`ArrowFunction` for public packet; synthetic fixtures may explicitly use FunctionDeclaration/FunctionExpression/ArrowFunction. Source ordinal arrays contain distinct increasing nonnegative integers; `object_ordinals` and `later_required_ordinals` are supplied selection evidence, not native authority. Native classification must not assume the desired prefix gap exists just because these arrays say so.

Public execution requires the exact frozen manifest; synthetic manifests use the same schema with `upstream_repository:"synthetic://..."`, their own hashes and explicit small counts. Do not silently replace fixed public12 with synthetic data.

Check root and every selected path component with lstat; no symlinks. Read selected regular files, exact byte length/hash, strict UTF8, no reopen after validation. No recursive discovery of all414members. Reject a manifest declaring a file not referenced by any selector or omitting a selected file. Validate selectors on UTF8 boundaries in the worker before slicing. Limits:16sites/16files/256KiB aggregate source,2MiB worker input,4MiB output,15seconds whole owned child. Test seams may lower but never raise ceilings. No partially successful packet on worker/protocol/limit/custody errors.

### Exact parent/worker wire request

The launcher sends one UTF8 JSON object followed by a newline, with fields in this insertion order:

`{schema:"prism.native-parameter-request/1",input_manifest_sha256,native_binary_sha256,files:[{path,sha256,bytes,script_kind,source,sites:[{path,start_byte,end_byte,compiler_kind,object_ordinals,later_required_ordinals}]}]}`

`source` is the exact retained UTF8-decoded string, BOM preserved. Every other file/site field is copied from the validated manifest, except the two top-level hashes, which are the independently authenticated manifest and native-binary byte hashes. Selectors are nested under their owning file; each selector.path must equal that file.path. There is no parallel positional sources array and no implicit basename join. Launcher file order is code-point path order and selectors numeric start/end order. Worker accepts reordered file/site arrays, identifies association by the explicit path, and emits canonical output; JSON object key order is not an authentication condition.

Both launcher and worker require exactly the listed keys at every request object level, correct scalar/array types, supported schema/script kinds/compiler kinds, canonical unique file paths and globally unique selector tuples. Every file has at least one selector; every selector is attached to the same explicit path; no missing source, duplicate file/selector, orphan selector or extra key is tolerated. Validate integer counts/caps, source byte length/hash and selector bounds/UTF8 boundaries for the **entire** request before any ParsedFile::parse. Worker validates source identity from the received strings with the existing sha2 dependency, never via files/repository reads. Check2MiB stdin cap before JSON decoding; EOF after the single JSON value plus whitespace only. Bad JSON/schema/hash/closure/limits fail nonzero with no successful output. The launcher validates the returned schema/top-level hashes/file/site identities against its request before publication; the worker copies those authenticated top-level hashes into the output. This transport integrity is not proof of the binary's source commit; the separate build receipt supplies that provenance.

Output schema `prism.native-parameter-characterization/1`, fields in this order:

1. `schema`, `measurement:"native_parameter_api_characterization"`, `authorizes_runtime_edge:false`, `native_entry_measured:false`, `callee_resolution_measured:false`, `input_manifest_sha256`, `native_binary_sha256`;
2. `files`: sorted path rows `{path,sha256,bytes,language,parse_error_count}`;
3. `sites`: sorted `(path,start_byte,end_byte)` rows described below;
4. `totals`: `{files,sites,unique_named,unique_unnamed,missing,ambiguous,recovery_quarantined}`; these statuses exclusively partition12;
5. `next_action:"bounded_entry_and_call_proof"|"defer"` and fixed reason string.

Each site row: `{path,start_byte,end_byte,status,candidates,next_proof_eligibility}`. Find **all** native all_functions nodes whose start/end equal the selector and whose kind maps to the requested compiler kind (`function_declaration`,`function_expression`,`arrow_function`). Never line/name/containment/nearest-node fallback. Preserve every matching candidate, including duplicate captures, rather than first/dedup. Sort candidates by `(kind,start_line,end_line,name-null-first)`; equal candidates remain separate. No hashed IDs.

Candidate fields:

- `{kind,start_line,end_line,name}` using actual node kind, one-based inclusive native lines and function_name or null;
- `parameters`: top-level formal source positions, in original order, each `{ordinal,kind,start_byte,end_byte,pattern_kind,ordinary_required_identifier}`. For the function `parameters` node take named children excluding comments; or singular `parameter` field. Each object/array/rest is **one** position. TS wrappers retain their raw kind; pattern_kind comes only from the wrapper's pattern/name field, otherwise the node kind. No recursive leaf expansion, default scanning or copied production slot classification. `ordinary_required_identifier` is a shallow native-shape observation: true for a direct `identifier` parameter, or TS/TSX `required_parameter` whose pattern field is an `identifier` and whose direct children contain only that pattern, its optional type field and comments. Any extra named or anonymous child (including initializer/`=`/`?`/rest/modifier/decorator) makes it false. Do not descend into a type annotation or initializer. This bool is a cohort-shape check, not slot or binding authority. Missing list emits `[]` with APIs still recorded independently; do not invent a slot.
- `slots`: null if native API returns None, otherwise the complete returned prefix list `{name,start_byte,end_byte,source_ordinal}`;
- `bindings`: complete occurrence API list in native API order with the same fields. For each slot/binding tuple source_ordinal is the unique containing top-level parameter's ordinal, or null if zero/multiple containment. Do not compress binding list index into ordinal; use half-open byte containment. Preserve unassignable tuple and null, no guessed fallback. This mapping is observational and does not create argument positions.

For candidate status: file parse_error_count>0 => recovery_quarantined regardless of matches (retain observed candidates); otherwise zero matches=>missing, >1=>ambiguous, exactly1 with name null=>unique_unnamed, otherwise unique_named. No status depends on whether slots/bindings contain a hoped-for suffix. Recovered rows never count as clean proof.

Compute each site's `next_proof_eligibility` by this exclusive priority, independently of its unchanged match/status field:

1. Status other than unique_named => `not_clean_named`.
2. Its sole candidate has `slots:null` => `native_slot_authority_unavailable`. Null is refusal, never an empty prefix, even if binding occurrences include a selected later name.
3. Otherwise require native agreement with **all** selected labels: both ordinal arrays nonempty; every referenced ordinal exists in the emitted native parameters; each object ordinal has pattern_kind=`object_pattern` with raw kind `object_pattern` or TS/TSX `required_parameter`; every later-required ordinal has ordinary_required_identifier=true; every selected object has a selected later-required ordinal greater than it, and every selected later-required ordinal has a selected object less than it. Failure => `selection_native_shape_mismatch`. Input compiler labels alone cannot authenticate this relationship; do not promote an array-as-object, defaulted/optional later parameter or out-of-range ordinal.
4. Otherwise, if at least one selected later-required ordinal has an observed binding at that exact non-null source_ordinal and no item of the **non-null** native slots list at that ordinal => `eligible`.
5. Otherwise => `no_selected_suffix_binding_gap`.

Packet `next_action` is bounded_entry_and_call_proof iff any site is eligible, with reason `named_native_binding_outside_legacy_prefix_requires_entry_and_call_proof`. Otherwise it is defer with reason `no_eligible_native_slot_gap`; each site's reason above preserves the exact refusal/mismatch cause. A population containing only null-slot duplicate fixtures must defer with native_slot_authority_unavailable per site, not claim a positional hole. Even the positive outcome authorizes no production repair. Readout lists exact site tuples underlying the condition; distinguish missing/unnamed/recovered/ambiguous cases and preserve full denominator. All arrays use Unicode-code-point ordering and numeric offsets; one trailing newline, no timestamp/PID in canonical data. Volatile timings/source-build provenance live in receipt.

## 6. Baseline, desired tool behavior and fail-first proof

Current native behavior is intentionally unchanged. The new observer should faithfully expose it. Do not write a test that demands new slots or entry nodes from the native API.

The passing baseline probe covers10fixtures in JS/TS/TSX, all parse clean. Exact sources and hashes are normative in BASELINE-FIXTURES.json; complete existing API vectors are baseline-output.log. Key vectors (byte pairs half-open):

| Fixture | Native function span/name | Current slots | Current bindings |
|---|---|---|---|
| object |0–40/take |[] |later19–24 |
| alias |0–56/take |first14–19 |first14–19;later35–40 |
| nested_default |0–49/take |[] |later28–33 |
| array |0–40/take |[] |later19–24 |
| rest |0–38/take |[] |[] |
| duplicate |0–32/take |null |JS:x19–20;TS/TSX:[] |
| assigned |13–34/take |[] |later19–24 |
| wrapped |18–39/take |[] |later24–29 |
| unnamed |8–29/null |[] |later14–19 |
| shadow |outer0–77/take;inner26–63/nested |outer[];inner later42–47 |outer later19–24;inner later42–47 |

Example desired observer distinction: alias has **three raw parameter positions**, object position1 remains one unsupported pattern; bindings map first→ordinal0 and later→ordinal2, while slots contain only first→ordinal0. No field `value` becomes an argument slot. Object fixture later→ordinal1, not compressed ordinal0. Duplicate fixture must faithfully retain JS/TS asymmetry; it is characterization, not approval of those underlying APIs for a boundary edge.

Before implementation, tests must exercise a runnable test-only same-schema observer adapter with intentionally wrong ordinal compression (alias later→1 or object later→0). Preserve source/hash/output and the desired complete-record assertion failing on that concrete value. This is observer-feature behavioral RED. Missing module/CLI, compile error, wrong fixture, setup refusal or zero tests selected is inadmissible. Passing current-native GAP output is not RED. Each implemented observation/custody path additionally needs a meaningful negative/edge control; capture bounded behavioral mutants rather than a baseline that only fails its first arbitrary field.

Complete control groups (full expected record/totals, not presence-only):

1. Object/alias/prefix/suffix: exact raw position spans, existing slots and bindings, correct source ordinals; no flattening or fabricated fields. Object with selectors object[0]/later[1] is the positive eligibility/full-output control. Plain two-identifier fixture is the no-gap control. Native array fixture falsely labeled object[0]/later[1], an out-of-range ordinal, and an optional/defaulted later native parameter must defer with selection_native_shape_mismatch; input labels cannot override observed shapes.
2. Assigned/wrapped/unnamed arrows and shadow fixture: exact byte match, names/null and only each selected callable's API observations; near-span and same-line same-name selectors cannot select an enclosing/neighbor function. Duplicate candidate assembly seam preserves both and reports ambiguous; do not claim parser naturally duplicates if it does not.
3. Rest/array/nested-default/duplicate variants preserve baseline refusal/asymmetry. Full JS/TS/TSX duplicate-fixture records must show slots:null and native_slot_authority_unavailable/defer, including JS with the surviving x binding at selected ordinal1. A direct predicate control distinguishes null from Some([]); null never promotes. Escaped identifier and malformed/recovery fixtures retain actual API rows and quarantine recovery; no new slot policy. Tests first characterize precise native results before fixing observer expected literals; that characterization alone is not RED.
4. Strict UTF8/BOM/astral/CRLF selectors: exact byte tuple identity and inside-code-point selector refusal; no UTF16 conversion or parser matching normalization. Pinned parent compiler spans are already bytes. A missing exact native match is legitimate recorded evidence, not a reason to alter the selector.
5. Manifest/source/binary identity, duplicate selectors/members, missing/unreferenced member, unsafe/symlink paths, malformed schema, stale source bytes, every cap, worker nonzero/bad JSON/output schema/timeout. Exact request controls: reordered files/sites retain full output equality by path; missing source, duplicate path, orphan/mismatched selector path, extra request field, bad JSON and oversized request refuse before parse. Accept one plain synthetic source for each of .js/.jsx/.ts/.tsx using the exact script_kind/native-language mapping; refuse mismatched .tsx/TypeScript (and do not silently coerce it). Fail before publication and preserve prior/foreign output. Child receives only authenticated retained strings; test a source mutation after validation does not change the already-formed request.
6. Canonical cold/repeat; inert ASCII prefix `/*p*/` shifts selector/candidate/parameter/occurrence byte coordinates by5, semantic fields and source ordinals unchanged, file/manifest identities refreshed; full transformed output equality. Stale identity refuses; restoring bytes/manifest reproduces original output. Meaningful alias fixture change `{key: value}`→`value` removes the unsupported shape: characterize native full slots and assert only the expected shape/prefix/ordinal differences. No generated graph edge expected.
7. Terminal partitions sum every selected site exactly once; valid zero-gap, unnamed, missing, ambiguous and recovered populations yield defer. Only a clean named, non-null-slot, native-shape-agreeing selected suffix gap yields next-proof disposition. Binding at null/unselected ordinal, unavailable slot authority or source-label mismatch cannot trigger it. No CPG-entry/caller result field may be fabricated.

## 7. Execution and verification if authorized later

1. Rebind merged source/planning commit, input/site manifest and source-equivalence; preserve parked artifacts. Record allocation and test-only adapter behavioral RED before implementing observation behavior.
2. Implement only owned files. Build isolated native example offline, record source/lockfile and binary hashes. Run complete own Node and any Rust example tests with exact native binary path/hash. Freeze source/tests, source manifest and all controls for independent implementation round1/cap2 **before public12 native execution**.
3. After core approval, authenticate selected11source files, run two cold new-output executions over exact12selectors, compare complete canonical bytes. Independently recompute statuses/gap condition from full rows and reconcile every selector/member against pinned manifest. Preserve output/readout/logs/binary and hashes. Report source diagnostic/match failures as observations; do not silently drop them or shrink population.
4. Project verification: full default Rust `cargo test --offline --no-fail-fast`; explicit example tests for the new target; full active Node inventory including the new module and predecessor parameter-frequency tests. Keep exact historical audit-input exclusions and grammar archive skip separate. Required CI feature/example/other checks must be green before any future merge. No live adoption or full Tier-A corpus. No src/callresolution/navigation/CPG change means Tier-A is not triggered; touching one is a scope stop, not permission to add Tier-A and expand production.
5. Evidence-only final independent review binds source/binary/manifest/output/gate hashes, exact counts and exclusions. Core approval, public characterization, final local acceptance and publication/merge remain separate states. User currently asked planning only; this spec does not authorize a later MR or feature merge.

Full test counts must come from logs. No new baseline for an out-of-scope failure. At each stable point snapshot/commit through controller and update handoff. Reviewer reports WRONG before SMELL with concrete input/result; no blocker without demonstrated wrong behavior. At declared cap classify before action; no silent third round, larger cap, restart or discarded partial artifact.

## 8. Handback and decision checkpoint

Handback: source/test file manifest and line counts; exact native build/source equivalence; runnable adapter RED plus mutants/negative controls; complete own/full-suite totals; exact12 output/cold-repeat hashes and independent row reconciliation; compact readout with unmatched/unnamed/quarantined totals; current blockers/exclusions and durable paths.

Readout conclusion must be one of: (a) enumerate a bounded subset needing a separate CPG-entry and resolved-call proof, with exact site identities and no implementation recommendation yet; or (b) defer with recorded reason. CPG97/nav53 and existing production parameter behavior remain unchanged. Cache invalidation is neither needed nor tested because this tool does not alter/read those caches. No completion claim that positional-hole repair, field binding, array/rest support, full dependency closure or arrow execution ownership is solved.

Stop before adding graph construction, native public API, parameter admission, production test-oracle changes, whole414native loading, dependencies/private inputs or broader syntax support. Any such request requires a separately planned slice. Do not react to budget or evidence failure by reviving the parked monolith.
