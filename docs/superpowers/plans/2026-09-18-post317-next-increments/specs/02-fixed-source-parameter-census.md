# P2 — fixed-source parameter and owner decision census

**Status:** planning draft for independent architecture review. Do not execute
until the exact-caller-read slice is finally accepted and the controller
separately dispatches this measurement slice.

## 1. Objective and boundary

Measure three separate source cohorts over one authenticated public package:

1. object/array/nested destructuring before a later optional identifier;
2. rest parameters and the supported slot prefix around them;
3. parenthesized and unparenthesized arrow ownership.

The output is an observer ledger, not a production admission. It may identify
at most one cohort for a later implementation proposal when that cohort has a
nonzero constructible native candidate population and a complete token,
owner, slot, entry and call-target contract. A zero result is valid only for
the complete fixed population. An unavailable or invalid input is
`input-blocked` or `incomplete`, never zero demand.

This slice does not change Prism production, tests, public APIs, parser or
compiler authority, parameter-slot behavior, CPG construction, cache schema,
call resolution or ownership. It makes no dependency, type-validity, runtime,
private-population or repository-wide claim.

## 2. Preconditions

- Exact-caller-read implementation `14e86083c2c754ed412d440742bd5763be388e42`
  (tree `a0411db6d858fe34806376ce41d2621853848d8b`) must be finally accepted,
  or the execution controller must bind a later commit with identical
  production hashes.
- This P2 spec and its implementer/reviewer prompts must receive independent
  architecture approval before observer code or measurements run.
- Observer work occurs only in an isolated archive/runner under
  `/private/tmp`; the shared repository and authenticated source archive stay
  read-only.
- No network, package manager, install, lifecycle script, project build,
  compiler Program, type checker, detached worker, or private input is used.

## 3. Fixed public population

| Property | Binding |
|---|---|
| Repository | `https://github.com/excalidraw/excalidraw` |
| Commit | `0642e72cfa2d9a71198200e52f37399384610ee3` |
| Tree | `709e9146b0fbd78c3ebf0d77e67143b2fbc43e4a` |
| Archive SHA-256 | `0a5a136c1330e87559767a7768b053dab74f7bba336e0fd7ed429377022b141d` |
| Complete extracted-source manifest SHA-256 | `afb0c3e9172fd66ff805e114a6413de6a9aef00f42c2491e8b67bb5900de0eba` |
| Population | every regular `.js`, `.jsx`, `.ts`, `.tsx` file recursively under `packages/excalidraw` |
| Members | **414** |
| Source bytes | **5,036,151** |
| Declaration-only stratum | **5** files whose basename ends `.d.ts` |
| Test-path stratum | **62** files whose basename contains `.test.` or `.spec.` |

The canonical member manifest is
`/private/tmp/prism-post317-planning/p2/input-hash-manifest.json`, SHA-256
`f8ebbdd79ca01e5cdb675616b913f1fbb549d45378ab1047db84844a3bc8e696`.
It uses sorted canonical JSON, names every relative path/hash/size/extension,
and records both strata. The optional `fractional-indexing` single-file pilot
is excluded from every decision denominator; it may be used only as a schema
smoke fixture and must be labeled as such.

### Manifest gate

Before parsing any member, the runner must validate the schema, repository,
commit/tree, population root, aggregate identity, exact member count/bytes,
unique canonical relative paths, regular-file status, allowed extensions,
per-member hashes and caps. It validates the complete manifest first, rather
than processing a valid prefix. A mismatch, duplicate, path escape, symlink,
missing member, extra member, cap breach or source mutation rejects the packet
with no census denominators.

Every valid manifest member later receives exactly one terminal file
disposition, including declaration-only files, recovered files and files with
zero relevant syntax.

## 4. Isolation and implementation budget

The runner is an archive-local observer built from the finally accepted Prism
source. It may reuse existing native Rust APIs and a standalone `.mjs`
syntactic observer that directly imports the pinned TypeScript compiler. It
must not add a crate/package dependency or edit production.

Planned source budget:

- at most 900 non-test lines for the Rust observer and JSON schema;
- at most 350 non-test lines for the compiler-syntax `.mjs` observer;
- at most 850 test/fixture lines;
- at most 2,100 total changed lines in the isolated runner.

If the formatted implementation exceeds a bound, stop with a split/options
brief before execution. Do not compress row fields or proof controls to meet a
line count.

Execution ceilings are fixed:

| Resource | Limit |
|---|---:|
| Manifest members | 512 |
| Input source bytes | 8 MiB |
| Combined ledger rows | 200,000 |
| Serialized result | 128 MiB |
| Observer wall time | 15 minutes |

A ceiling breach makes the receipt incomplete and requires a controller
decision. It never authorizes sampling, hit-selected files or a partial count
presented as complete.

## 5. Native-independent syntax inventories

Two syntax inventories are required and neither may depend on Prism's native
function table:

1. **Pinned-grammar walk.** Walk callable syntax nodes directly from each
   parsed tree. Do not derive the syntax denominator by iterating
   `ParsedFile::all_functions()`. Record callable kind/span, named/anonymous
   status, parameter list span, raw parameter ordinal and exact parameter
   shape for every grammar callable, including callables that later have zero
   native-owner matches.
2. **Compiler syntactic walk.** Load the already pinned TypeScript 5.9.3
   compiler from
   `/private/tmp/prism-post316-orchestration/public-inputs/typescript-5.9.3/package/package/lib/typescript.js`
   (SHA-256
   `3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675`)
   and call `createSourceFile` with the extension-appropriate script kind.
   Traverse syntax only. Do not create a `Program`, resolve imports, load
   dependencies, inspect types or call package tooling.

Both inventories retain object versus array patterns, nested/default/rest
dimensions, parenthesized versus unparenthesized arrows, exact source spans
and recovery/diagnostic state. TypeScript UTF-16 positions are converted to
UTF-8 bytes by the normative rules in section 6.8 before joining. Rows join
only by file plus canonical span/kind/shape. Zero, one and multiple
cross-observer matches are retained; the compiler inventory is an independent
syntactic observation, not semantic validity or ownership authority.

## 6. Deterministic ledgers

Emit one canonical JSON document with schema `prism.p2-census.v1`. Sort every
array by its documented key and give every row a stable `row_id`, defined as
the lowercase SHA-256 of the canonical tuple stated for that ledger. Preserve
multiplicity; never reduce facts to a set before recording rows.

### 6.1 `files`

One row per manifest member:

- relative path, member hash/bytes, language;
- declaration-only/test-path stratum booleans;
- native parse status and recovery/diagnostic counts;
- compiler syntactic parse status and diagnostic counts;
- relevant syntax counts from each observer;
- terminal disposition: `complete`, `declaration_only`,
  `recovered_quarantined`, `syntax_only`, or another enumerated refusal.

Recovered files retain syntax/recovery/file rows but cannot promote a native
candidate. Their existence does not abort later files.

### 6.2 `syntax`

Key: `(observer, file, callable_start, callable_end, parameter_start,
parameter_end, raw_ordinal, shape)`.

Record observer (`native_grammar` or `typescript_syntax`), callable kind and
span, exact parameter shape/dimensions/token bytes, raw syntactic ordinal,
parenthesization and recovery state. Object, array, nested, assignment/default,
rest and simple identifier are separate dimensions, including overlaps.

### 6.3 `native_owner_slots`

Join each native-grammar syntax callable to native inventory by exact file,
callable span and compatible kind. Record zero/one/multiple owner matches;
never infer an owner from name alone. For a unique owner retain its complete
identity: file, name, start line, start/end bytes and class owner.

Record the literal result of
`function_parameter_occurrences` and
`function_parameter_slot_occurrences`: `None`, `Some([])`, a complete vector,
or a truncated supported prefix are distinct. For every raw syntax ordinal,
record exact slot index/token or the first hole/refusal. Never invent a slot
beyond the returned prefix or compress around object/array/rest patterns.

### 6.4 `entry_defs`

For each exact simple parameter token, preserve the complete DFG and CPG entry
Def key fields, access kind and multiplicity. Record zero/one/multiple exact
entries and the reason for no unique entry. A body reference or body Def is a
separate observation and never substitutes for the signature token.

Declaration-only and recovered files are excluded from the executable/native
entry denominator but remain in syntax/file denominators.

### 6.5 `call_sites` and `resolved_targets`

Record every native `CallSite` with its **caller** identity, call and callee
spans, line, kind, origin, callee/qualifier spelling, argument count and exact
argument span/ordinal. Caller identity owns the call and argument; it is not
the parameter owner.

Resolve the call using existing native resolution. Preserve every candidate
target and confidence. A unique target joins a parameter only when its full
native callee identity equals the parameter owner by exact file/name/start
line and compatible span evidence. Ambiguous or absent targets become row
refusals and processing continues.

Controls must prove an ordinary cross-function call and a recursive call:
caller identity owns the call while the unique resolved **callee** owns the
parameter. Any rule requiring caller identity to equal the parameter owner is
wrong.

### 6.6 `binding_candidates`

A row is `constructible_native_candidate` only when all predicates are
literal observations:

- unique non-recovered native parameter owner;
- supported non-hole native slot at the raw ordinal;
- unique exact signature entry Def;
- call and argument span owned by the native caller;
- argument is present at that ordinal;
- exactly one native resolved target with retained confidence;
- resolved target identity equals the parameter's native callee identity.

Record one refusal row at the first failed predicate and retain all stage keys
needed to recompute it. Missing owner, slot, entry, target, argument or
confidence and target ambiguity are normal census results; they never abort
the packet.

### 6.7 `observed_step5b_edges`

Separately record exact existing `Use(argument) -> Def(parameter)` CPG edge,
label and full endpoints when present. Absence is `observed_edge_absent`, not a
candidate refusal and not permission to create an edge. A constructible
candidate and an emitted Step 5b edge are separate populations.

### 6.8 Normative offsets, kinds, row IDs and foreign keys

All public coordinates are zero-based, half-open UTF-8 byte offsets into the
exact manifest member. Native tree-sitter coordinates already use this form.
The compiler observer must retain raw zero-based UTF-16 `pos`, token `start`
and `end`, then convert each UTF-16 boundary by scanning the exact source from
the start and accumulating both UTF-16 code units and UTF-8 bytes. A boundary
inside a surrogate pair is `compiler_offset_unmappable` and cannot join.

Use token starts excluding leading trivia: TypeScript uses
`node.getStart(source_file, false)` and `node.end`; the native observer uses
the node's first token start and node end. Record raw enclosing/full spans
separately and never use them in place of token spans. Comments/trivia may
shift enclosing raw positions but cannot be silently absorbed into a token
join.

The small canonical callable-kind map is:

| Native/compiler form | Canonical kind |
|---|---|
| named function declaration | `function_declaration` |
| class/object method definition | `method` plus its exact class/object context |
| identifier-bound function expression | `bound_function_expression` |
| identifier-bound parenthesized arrow | `bound_arrow_parenthesized` |
| identifier-bound unparenthesized arrow | `bound_arrow_unparenthesized` |
| anonymous function expression | `anonymous_function_expression` |
| anonymous parenthesized arrow | `anonymous_arrow_parenthesized` |
| anonymous unparenthesized arrow | `anonymous_arrow_unparenthesized` |

Unsupported or observer-specific kinds remain as raw kinds and produce an
explicit `canonical_kind_unmatched` join row. No name/line fallback is
allowed. Canonical parameter shapes are `simple_identifier`,
`optional_identifier`, `assignment_identifier`, `object_pattern`,
`array_pattern`, `rest_identifier`, `rest_pattern`, `this_parameter`, and
`unknown`; nested/default dimensions are additional ordered fields, not shape
compression.

Encode each row ID as lowercase SHA-256 of a canonical JSON array beginning
with the schema name and ledger name. Strings use exact UTF-8, numbers are
base-10 JSON integers, `null` is distinct from an empty array, object members
never participate without an ordered array projection, and multiplicity is an
explicit zero-based ordinal after stable tuple sorting.

Normative ledger identities and foreign keys:

| Ledger | Canonical identity tuple | Foreign keys |
|---|---|---|
| `files` | `(path, member_sha256)` | none |
| `syntax` | `(observer, file_id, callable_token_start, callable_end, canonical_kind, parameter_start, parameter_end, raw_ordinal, canonical_shape, dimensions, multiplicity_ordinal)` | `file_id` |
| `syntax_joins` | `(native_syntax_id_or_null, compiler_syntax_id_or_null, join_status, match_ordinal)` | zero/one/many `syntax` IDs from both observers |
| `native_owner_slots` | `(native_syntax_id, raw_ordinal, owner_join_status, owner_identity_or_null, literal_slot_state, slot_index_or_null, slot_token_or_null)` | native `syntax` ID; all candidate owner identities retained on zero/many joins |
| `entry_defs` | `(owner_slot_id, entry_status, complete_def_key_or_null, multiplicity_ordinal)` | `native_owner_slots` ID |
| `call_sites` | `(file_id, caller_identity, call_start, call_end, callee_start, callee_end, call_kind, origin, multiplicity_ordinal)` | `file_id`; ordered argument subrows keyed by `(call_site_id, raw_ordinal, start, end)` |
| `resolved_targets` | `(call_site_id, target_status, complete_target_identity_or_null, confidence_or_null, target_ordinal)` | `call_sites` ID |
| `binding_candidates` | `(call_site_id, argument_ordinal, owner_slot_id_or_null, resolved_target_id_or_null, decision, reason)` | call, target, owner/slot and entry IDs when present |
| `observed_step5b_edges` | `(binding_candidate_id, edge_status, complete_from_key_or_null, complete_to_key_or_null, label_or_null, multiplicity_ordinal)` | `binding_candidates` ID |

Anonymous callable names are JSON `null`, never an empty string. A
zero-parameter callable has a present empty parameter array; absence of a
parameter list is a distinct refusal. Complete Def/Use and callable identities
include file, owner/name/class, line, access and byte fields. Repeated identical
primitive facts remain separate rows through `multiplicity_ordinal`.

Permanent mapping controls place both a non-ASCII BMP character and an astral
character before an observed parameter and call, include leading/interstitial
comments, and cover parenthesized and unparenthesized arrows. They assert raw
UTF-16 positions, converted UTF-8 spans, canonical kinds/shapes, join status,
row IDs and all foreign keys.

## 7. Complete denominators and joins

Publish stage totals and reason counts for each cohort and each stratum:

1. fixed manifest files and bytes;
2. native-grammar syntax rows;
3. TypeScript compiler syntax rows;
4. exact cross-observer syntax joins (zero/one/multiple);
5. unique native-owner joins;
6. literal native slot states;
7. unique exact signature entries;
8. native call sites and argument-present sites;
9. uniquely resolved packet targets;
10. constructible native candidates;
11. observed Step 5b edges.

Destructuring, rest and arrow cohorts have separate tables and promotion
decisions. Rows may overlap cohorts, but no combined numerator is published.
Declaration-only and test-path strata are always shown separately from the
whole fixed population. Imports outside the manifest are unresolved and
outside-population; dependency or module closure is not claimed.

## 8. Fail-first harness and controls

Write the harness before observer behavior. Native Prism characterization and
observer behavioral RED are separate evidence:

- Run one external native-characterization harness unchanged on the finally
  accepted Prism predecessor. It must pass and freeze parser/function/slot,
  DFG/CPG, call-site and target facts. It is not behavioral RED.
- Before the final observer, build a test-only baseline adapter that accepts
  the final CLI and `prism.p2-census.v1` schema and exits successfully, but
  deliberately models the old table-derived syntax inventory, the erroneous
  caller=callee join, and row-abort behavior. Run the desired assertions
  against this same-schema output. Preserve the adapter source/hash and every
  wrong or missing output row. Missing CLI/schema support, missing binary,
  zero tests, invalid syntax and compilation/setup failure are inadmissible.
- After implementation, named behavioral mutants of the completed observer may
  supplement the baseline adapter. Never describe a deliberate observer mutant
  as a Prism native-source regression.

The initial fixtures must cover complete bounded row/protocol paths for:

- syntax callable absent from native inventory, proving syntax is independent
  of `all_functions()`;
- two same-name owners and an ambiguous resolved target;
- ordinary cross-function and recursive calls, proving caller/callee identity
  separation;
- zero-parameter, `Some([])`, truncated-prefix, unsupported-first-pattern and
  later-optional signatures;
- object, array, nested default, rest, parenthesized arrow and
  unparenthesized arrow rows;
- duplicate/recovery/escaped identifier, omitted argument and missing native
  call-site counterpart;
- full endpoint/access multiplicity and Step 5b present/absent separation;
- a recovered file followed by a valid file, proving row refusal continues;
- an unknown file or row category, proving fail-closed schema handling;
- every manifest member receiving one terminal file disposition;
- cold/repeat byte identity, caps and exact denominator reconciliation.
- UTF-16/UTF-8 conversion with one BMP and one astral prefix, comment/trivia
  boundaries, canonical kind/shape joins and parenthesized/unparenthesized
  arrows.

The same assertions become GREEN only on the final observer. A passing native
diagnostic inventory is characterization, not RED.

## 9. Disposable mutation controls

Synthetic controls live in a separate manifest and never enter public demand
denominators. The exact original no-trailing-newline fixture is
`function take(value){return sink(value)}\nfunction run(input){return take(input)}`
(80 bytes, SHA-256
`e9a638744cd81b7875d82519dccb5a44095d66330d97c4693325fd69d54cb86b`).
Preserve that immutable parent fixture hash.

1. **Inert shift.** Insert exact ASCII `/*p2*/` at byte 0. The derived fixture
   is 86 bytes, SHA-256
   `ea8d905bfd336dbe72ab21753defabb8e1df2395e6ae87ca585d7f418bc60407`.
   First require stale-member hash rejection, then provide a complete derived
   manifest with updated member/packet digests. Normalize each unchanged token
   anchor by `new = old + 6`. Recompute raw/full/enclosing spans from each
   observer's declared trivia semantics; do not apply the affine map to a span
   that includes the inserted comment. In particular, the first TypeScript
   function must record raw/token UTF-16 `(pos, start) = (0, 6)`, with token
   start converted to UTF-8 byte 6, while the native function token span starts
   at byte 6. Recompute every coordinate-derived row ID, foreign key,
   source/packet hash and dependent digest. After the declared token-anchor and
   observer-specific raw-span normalization, names, kinds, shapes, ordinals,
   owners, targets, confidences, relationships, reason codes and
   multiplicities must match complete original rows. Restore the entire
   original fixture/manifest and require byte-identical original output.
2. **Meaningful parameter mutation.** Use exact no-trailing-newline bytes
   `function take({value}){return sink(value)}\nfunction run(input){return take(input)}`
   (82 bytes, SHA-256
   `244101631c79f4f2d5c4083a9ac69c9b846ae5a4149a98777c522f9f693cc6cd`).
   The inserted `{` is at original byte 14 and `}` at original byte 19. The
   declared semantic anchor transform is: original offsets below 14 unchanged,
   offsets in `[14,19)` shift +1, and offsets at or after 19 shift +2. Thus the
   later caller, call and argument token spans all legitimately shift +2.
   Apply the piecewise map only to unchanged semantic token anchors; recompute
   raw/full/trivia-containing spans under each observer's rules. Assert the new
   object-pattern span directly from the mutated bytes rather than mechanically
   mapping the removed simple-identifier parameter node. Recompute all affected
   IDs, foreign keys, source/packet hashes, enclosing spans and digests, then
   compare complete rows under this declared correspondence. Require the exact
   simple-identifier parameter row to become an object-pattern row, literal
   `Some([])` or first-slot refusal from the native API, no exact simple-token
   entry, candidate refusal `slot_missing_at_ordinal_0`, and no observed Step5b
   argument-to-parameter edge. Every other semantic field/relationship and
   multiplicity must match after normalization; count-only equality is
   insufficient. Restore the entire original manifest and require exact
   original output.

Arbitrary-byte mutation is prohibited because it can legitimately change
syntax, identity and graph semantics.

## 10. Packet failures versus row refusals

Fatal packet/integrity states:

- manifest/path/hash/duplicate/member/count/byte mismatch;
- unsupported schema/version or unknown terminal category;
- source or output cap breach;
- tooling/compiler identity mismatch;
- output serialization failure, timeout or incomplete terminal dispositions.

Row/file observations that continue:

- syntax with no native owner;
- `None`, `Some([])`, truncated prefix or missing slot;
- missing/non-unique entry;
- omitted argument or absent native call site;
- absent/ambiguous target;
- recovered file (quarantined from candidate promotion);
- absent Step 5b edge.

The final receipt is valid only if all 414 members have terminal dispositions,
every ledger denominator reconciles, and both observers finish within caps.

## 11. Run sequence and evidence

1. Authenticate accepted Prism source, pinned compiler and input manifest.
2. Run passing unchanged-native characterization, then the runnable
   same-CLI/schema baseline adapter to capture behavioral RED rows.
3. Implement only the isolated observers/schema; rerun focused tests.
4. Run fixed public packet cold and repeat; require byte-identical canonical
   output and receipts.
5. Run inert mutation stale/refreshed/restored sequence.
6. Run meaningful mutation stale/refreshed/restored sequence.
7. Verify all denominators, joins, refusal counts, row IDs and terminal file
   dispositions independently.
8. Preserve source, binary, compiler, input, fixture, output, log and receipt
   SHA-256 hashes. Do not copy private identifiers because no private input is
   authorized.

No full Prism regression or Tier-A run is required for an external observer
that does not modify repository source. The observer's own complete tests,
cold/repeat/mutation evidence and read-only source proof are mandatory.

## 12. Decision and stop rules

For each cohort, report one of:

- `nonzero_constructible`: exact unique candidate rows exist;
- `zero_constructible`: complete fixed population ran and no candidate rows
  survived, with reason denominators;
- `input_blocked` or `incomplete`: no demand inference.

At most one cohort may be recommended for a later implementation spec, and
only when its nonzero rows support a complete contract. Counts never authorize
production. Destructuring still requires hole and alias semantics; rest still
requires a non-compressing variadic policy; arrows still require stable native
ownership. Complex defaults, optional-with-default, decorators/properties,
erased `this`, default-value flow, TDZ and new receiver authority stay outside
this census.

Stop and return an options brief on cap breach, manifest drift, unreconciled
denominator, observer disagreement that cannot be represented as rows, need
for a production/API/parser change, new dependency, compiler Program/type
authority, or private input.
