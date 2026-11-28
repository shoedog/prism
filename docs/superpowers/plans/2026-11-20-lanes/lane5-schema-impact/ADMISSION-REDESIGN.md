# Lane 5 amendment: admission redesign for `prism-boundary` (design of record)

Status: design only, no product code. Applies to crate `crates/prism-boundary` at the P1d head (PR #958). Every
statement about code names a repository path and function; paths are relative to `crates/prism-boundary/src`
unless they start with `crates/`, `docs/` or `scripts/`. The code at the P1d head was read, not run: every input
state below is traced from the code and becomes a RED test before anything relies on it.

## 1. Problem and goal

### 1.1 The defect class

The crate decides, for a schema-impact report, which inputs and claims are admitted (proven to come from a named
tree of an admitted snapshot, or independently evidenced at a cutoff), waived, proven or known. Six review rounds of
the P1d slice and a three-pass read-only census found one defect class again and again:

> A decision (admitted, waived, proven, known, or a derived marker) can read a value without that read becoming a
> dependency of the decision.

The census counted 131 cases over 38 settlement paths (P1-P38, listed in section 4). 105 of them bypass the
settlement gate; 89 of those come with a concrete input state in which a decision is admitted while something it
read is not. 49 of the 89 are serious (the decision certifies something that rests on an unadmitted or unchecked
value), 9 are of uncertain materiality and 31 are minor or already documented. 37 of the 38 paths lie outside
`revision/admission/gate.rs`. The census counts are approximate (about plus or minus 3) and every state was traced,
not executed.

### 1.2 What is already true

- `ClaimStatus` (`revision/admission/gate.rs`) is opaque: it has a private field and one constructor,
  `ClaimStatus::decide`, called only from `ClaimDecision::from_reasons`. A decision is admitted only when no reason
  remains and every `RequiredDependency` it was handed is admitted. Status minting is not the gap.
- Admission requests enter through one constructor: `admit_inputs` (`revision/admission.rs`),
  `admit_schema_input` (`revision/admission/projection.rs`) and `saved_binding_status` (`revision/staleness.rs`)
  all call `Ledger::new`; `Ledger` is `pub(in crate::revision)`.
- The frozen layer has one public entry, `ImpactDocument::validate` (`model/impact.rs`), which takes any typed or
  hand-edited document.

### 1.3 Why the gap exists

The dependency list is handwritten at each call site. There are 25 calls of `ClaimDecision::from_reasons` or
`AdmissionReport::settle` outside `gate.rs`, in six files: `model/impact/validation.rs` (15),
`model/impact/input_validation.rs` (4), `revision/admission/graph.rs` (3), `revision/admission/schema_claim.rs`,
`revision/admission/projection.rs` and `revision/admission.rs` (1 each). The frozen layer settles on string keys
(`format!("input:{id}")`, `"report-cutoff"`). Settlement code reads public-field request and document DTOs directly
(`AdmissionRequest`, `ImpactDocument`, `Change`, `Usage`, ... in about 2,300 lines). Derived markers
(`Coverage::Checked`, the `"proven"` provenance, presences, metric totals, `body_presence`) are plain enums or strings
set far from the gate and re-derived by each consumer. Nothing connects "this decision read X" with "X is a
dependency of this decision".

Three examples, all read in the code:

- `selection::selection` (`revision/admission/selection.rs`) tests the subject's role and repository and returns
  `Ok(())` (a waiver of the receipt requirement) without reading the subject's own selection decision, while
  `Ledger::claim` (`revision/admission/graph.rs`) adds the subject's selection as a dependency only in a historical
  review.
- `slot_cutoff` (`model/impact/input_validation.rs`) builds its merge-holder population from a map in which the last
  record with a given input id wins, so a duplicated holder is lost before any lookup, and it waives the Before input
  through `selection_required` on the raw record without an `input:` edge.
- `ElementFacts::from_index` (`revision/staleness/inventory.rs`) re-derives operation comparability from
  `message.coverage` and limits; it never reads `WireEvidence::empty_projection_with_unproved_media`, the status
  inventory or the fallback message, all of which the diff honours.

### 1.4 Why instance-by-instance folds did not converge

Each fold added the missing edge or rule for the instances named in its round. Each new review round and each census round
found new members of the same kind through another reader, another consumer pair or another record field (the
three census rounds found 81, 17 and 33: not shrinking). Under the lane's convergence rule this is an open class: the fix has to remove the
way to write the defect, not its instances.

### 1.5 Goal

> After this redesign a settlement decision cannot read a value without depending on it, for the paths the design
> lists; the design says truthfully what it leaves open.

"The paths the design lists" are the 38 settlement paths of section 4 on the certified entry points of section
3.8. Section 5 counts what the redesign turns into a compile error, what becomes a runtime rule with a test, and
what stays a disclosed limit.

## 2. Designs considered and the choice

### 2.1 Five designs

| Design | Mechanism | Cost (estimate) | How it fails | Takes from it |
|---|---|---|---|---|
| A. Mandatory read tokens | A read returns `(value, token)`; a decision must be handed the tokens of what it read. | 12-20 files, 6-9 PRs. | The value and the token separate: code uses the value and drops the token. `#[must_use]` or a non-`Copy` token does not force consumption into the right decision. A missing predicate still type-checks. | The intent: no value without a dependency. |
| B. One settle function per claim kind, DTOs hidden | Each claim kind has one settlement function with typed inputs; request and frozen adapters share it. | 15-22 files, 6-9 PRs. | A helper can collapse candidates (drop a duplicate) before handing a correctly typed input; the compiler sees the right type. Detected by tests or review. | The per-kind settle functions and hidden DTOs (section 3.3). |
| C. Dependencies recorded at read time | A per-decision reader records the key of every value before returning it; the gate uses the recorded set instead of a handwritten list. | 12-20 files, 5-8 PRs. | A policy that never asks for a value (an unconsulted match outcome, a justification never read) produces a consistent trace. A cached value detached from its source does the same. | The recording reader (section 3.2). |
| D. Mechanical CI audit | A test inventories settlement functions and raw-field reads and compares them with declared dependencies; evidence-withdrawal tests per claim. | 3-6 files, 2-4 PRs, no product change. | A new helper, an unexecuted branch or a field absent from both lists passes. Detection is by CI on represented cases only. | The import check and the probes (section 7). |
| E. Narrow the certified contract | Refuse unsupported shapes at the entry points; disclose the rest. | 5-9 files, 2-4 PRs. | It leaves 86 bypassing cases (41 serious) on shapes P4 must support (live, consumer side, saved status, membership); schema-content markers cannot be refused by a shape check; the frozen entry takes foreign documents. | Nothing beyond the existing provisional status. |

### 2.2 Choice

**B + C, plus two proof types: waiver grants and a settled-marker table; D as the regression check.**

- C removes the second, handwritten account of what a decision read. Policy code reaches values only through a
  reader that records the value's key before returning it, and the gate derives the dependency list from that
  record.
- B fixes the shape of policy code: one settle function per claim kind, taking only a reader. The DTOs are
  unreadable by policy code (section 3.3).
- A waiver is a value, not a boolean. A selection, cutoff or link waiver can be returned only as a grant type whose
  constructor takes the settled decision that justifies it. "Waive without reading the justification" then does not
  compile. This is the owner's "second type", applied to waivers.
- Derived markers are settled once, at the schema reader, into a table whose answers (presence, comparability,
  negative basis) are the only route for the saved-status and membership consumers. This is the owner's second type
  for derived markers.
- D catches what the compiler cannot: a policy module naming a DTO, a consumer naming a raw marker field, an
  optional read outside its allow-list. A seam per recording point turns each compile-time claim into a test.

### 2.3 Correction of the compile-time coverage estimate

The agreed direction estimated that about 49 of the 89 concrete-state cases (35 to 37 of the serious ones) become
compile errors, about 12 more with the marker type, and about 14 minor cases remain disclosed. Checked against the
code case by case (section 5):

- **29, not about 49, become compile errors, all of them serious**: 14 through the recording reader, 7 through
  waiver grant types, 8 through the settled-marker table.
- Of the 37 serious cases the estimate placed at compile time, 20 are compile errors after the redesign, 16 are
  runtime rules with a RED test, and 1 is a disclosed limit. A token for every read made leaves those 16 possible,
  because in each the deciding read is either never made or is made correctly on the wrong population or relation:
  - a merge-receipt holder lost before the lookup, because the input map keeps one record per id;
  - selection-conflict populations that leave out the subject head and waived inputs (both layers);
  - relation rules on admitted inputs: a non-subject After commit in the subject's repository exempted, merge base,
    target and head compared only with another request field, frozen computed Before or After inputs accepted
    without merge evidence, a live computed After without a merge receipt, an inferred frozen anchor, a scan client
    outside the slots, an independent-cutoff origin without a covered tree, the report time never compared with the
    cutoff in historical and version reviews;
  - a match outcome never consulted by the usage that names it, and a usage site id never compared with its match;
  - a media proof's cutoff flag self-asserted in the catalog bytes;
  - a published media decision that does not depend on the schema claim it promises;
  - a user claim id that collides with a generated media key.
- Of the about 12 marker and builder cases, 8 become compile errors through the marker table, 1 through the reader (a
  known metric total built from an input whose admission it never reads), and 3 stay runtime rules (an unread
  completeness flag, bindings matched by input id alone, an evidence record dropped at parse time before the
  duplicate test). A marker type constructed after any of those three operations preserves the mistake.
- The minor cases: 15 field and identity cases stay disclosed as estimated, but 13 documented known limits and
  design decisions, 4 further cases with uncertain or no consumer impact, and 1 serious case (an unevidenced
  `no_first_parent` waiver) are also disclosed, 33 in all. 4 are not defects.

The estimate's direction holds; its size does not. The difference is the population of cases whose mechanism is a
rule, not a read.

### 2.4 Private policy crate versus module privacy

One option puts policy in a private implementation crate with no dependency on the DTO crate, so the compiler forbids
policy code from naming a DTO. For this workspace it costs more than it buys:

- Policy needs the vocabulary types (`Role`, `ServiceSide`, `ReviewMode`, `SnapshotInput`, `ReceiptOrigin`,
  `SelectionReceipt` proofs, `Scope`). They would move to the new crate and be re-exported, or be duplicated as fact
  enums: an estimated 1,500 to 2,500 moved lines and a second `Cargo.toml` before any defect is fixed.
- `prism-boundary` has no dependents in the workspace and every change runs CI, so a source-scanning test rejects a
  DTO import in the same PR that adds it. The workspace already relies on such a test for the core crate
  (`tests/integration/layering_test.rs`).

The design therefore uses module privacy plus a CI import check and states the difference (section 3.3). The
private crate stays an owner option (section 9).

## 3. The design

### 3.1 Layout

```
src/settle.rs              crate-private module root
src/settle/gate.rs         moved from revision/admission/gate.rs: ClaimStatus, the gate
src/settle/key.rs          ClaimKey and its rendering to today's spellings
src/settle/store.rs        Settled<T>, the evaluator store, the output adapter
src/settle/reader.rs       recorder core, optional reads, population queries
src/settle/waiver.rs       PinProof, ExemptionGrant, SelfExemption, LinkWaiver
src/revision/admission/ingress.rs        AdmissionReader (holds the request privately)
src/revision/admission/policy/{clock,selection,receipt,merge,claim,media}.rs
src/model/impact/frozen/ingress.rs       DocumentReader (holds the document privately)
src/model/impact/frozen/policy/{inputs,cutoff,slots,status,auxiliary,records,links}.rs
src/schema/markers.rs                    MarkerTable and NegativeProof
src/model/impact/membership/admitted.rs  certified membership entry
tests/policy_imports.rs                  the CI import check
```

`ClaimStatus` keeps its public path through a re-export in `revision/admission.rs`.

### 3.2 Types and who constructs them

| Type | Fields and visibility | Constructed by | Guarantee |
|---|---|---|---|
| `ClaimKey` | closed enum, `pub(crate)`; admission: `SubjectHead`, `Clock`, `Cutoff`, `Input(id)`, `Selection(id)`, `Merge(id)`, `Receipt(id)`, `Slot(id)`, `Claim(id)`, `SchemaBasis(claim)`, `MediaProof{claim, receipt}`, `Media{claim, receipt}`; frozen: `DocumentSchema`, `DocumentClock`, `Input(id)`, `ReportCutoff`, `Slots(Role)`, `Reference`, `Comparison(id)`, `Usage{index, id}`, `ReviewAction(id)`, `BindingStatus(i)`, `CallChange(i)`, `Risk(i)`, `Membership(i)`, `Metric(name)`, `Row(id)`, `RowDuplicate(id)`, `Limit(id)`, `Warning{index, claim}`; markers: `KnownTotal(name)` | anyone in the crate | A key names a proposition; it carries no authority. An input id alone never stands for "selected for this use": that is `Selection(id)`. |
| `Recorder` / `AdmissionReader` / `DocumentReader` | private fields: the DTO reference, the store, the recorded keys (`RefCell<Vec<ClaimKey>>`, first-read order, deduplicated), the optional keys | only `Evaluator::settle(key, f)` (`settle/reader.rs`), one per decision, handed to `f` as `&Reader` and consumed when `f` returns | Every accessor records its key before returning; a withheld fact is still recorded, so reading and ignoring it refuses the decision. |
| `Settled<T>` | private: key, `ClaimDecision`, `Option<T>` | only the evaluator in `settle/store.rs`, after the gate | The value of a settled fact is reachable only through a reader, which records the key. |
| `ClaimStatus` | unchanged, opaque | only `settle/gate.rs` | As today. `from_reasons` becomes `pub(in crate::settle)`. |
| Waiver grants: `PinProof`, `ExemptionGrant`, `SelfExemption`, `LinkWaiver` | private fields holding references to the justifying `Settled` facts | only reader methods in `settle/waiver.rs` that take those facts | A selection or cutoff decision returns `SelectionOutcome::{Pinned(PinProof), Exempt(ExemptionGrant), SubjectHead(SelfExemption)}`; there is no admitted variant without a grant. |
| `MarkerTable` | fields private to `schema/markers.rs`; stored in a `pub(crate)` field of `SchemaIndex`, so other modules see only its answer methods | only `schema::markers::settle`, called once from `schema::messages::project` | One settled answer per operation and root: body authority (proven, unproved, withheld, not applicable), status set (readable or withheld), request eligibility, fallback caveat, operation readability (readable, gap, incomplete). |
| `NegativeProof` | private | only `MarkerTable::negative_basis` | A membership negative cannot be emitted on the certified entry without it. |

`ClaimDecision`, `Refusal` and `AdmissionReport` stay as they are: they are output.

### 3.3 How policy code is kept from the DTOs

**Enforced by the compiler.**

1. The DTO reference lives in a private field of `AdmissionReader` (in `revision/admission/ingress.rs`) or
   `DocumentReader` (in `model/impact/frozen/ingress.rs`). Policy modules are siblings of `ingress`, not
   descendants, so they cannot read that field.
2. Accessors return view types defined in the ingress module, and each view exposes only the fields owned by the key
   it records. `input(id)` returns identity facts (id, role, repository, side, snapshot kind, tree, revision) and
   never the selection receipt, which is reachable only through `selection(id)`. A raw DTO sub-object is exposed only
   by the accessor whose key owns all of it (for example the `SelectionReceipt` inside the settle function for
   `Selection(id)`).
3. `Settled`, the waiver grants, `MarkerTable` and `NegativeProof` have private constructors. The crate-private marker
   fields of `SchemaIndex` and `Operation` stay `pub(crate)`, because the diff (the reference consumer) reads them and
   Rust has no visibility for two sibling modules; item 5 covers them.

**Enforced by a CI import check** (`tests/policy_imports.rs`, a source scan in the style of
`tests/integration/layering_test.rs`).

4. Files under `revision/admission/policy/`, `model/impact/frozen/policy/` and the marker consumers
   (`revision/staleness/inventory.rs`, the decision parts of `revision/staleness.rs`, `model/impact/membership*`)
   may not name the multi-key DTO types (`AdmissionRequest`, `AdmissionInput`, `AdmissionReceipt`, `AdmissionClaim`,
   `InputSlot`, `ImpactDocument`, `DocumentInput`, `Change`, `RoleSlots`, `Comparison`, `MatchRecord`, `Usage`,
   `BindingStatus`, `SavedScan`, `ChangeSelection`), and policy functions take only a reader and keys as parameters.
5. Marker consumers may not name the raw marker fields, public (`.coverage`, `.limits`, `status_inventory`,
   `json_source`, `declared_media`) or crate-private (`operation_gaps`, `operation_coverage_incomplete`,
   `wire_evidence`, `request_authored_but_excluded`, `status_refusals`).
6. `from_reasons` and `RequiredDependency` may not be named outside `settle/`, and policy modules may not name
   `ClaimStatus`.
7. Every `reader.optional(..)` call site must appear in the test's closed allow-list (direct media attachments and
   inapplicable comparison links today).

**The difference.** The compiler stops policy code from obtaining a DTO value it was not handed and from building an
admitted outcome without its grant. It does not stop someone from adding a DTO parameter to a policy function, or
from naming a marker field, because those types and fields are visible crate-wide. The CI check rejects both in the PR
that introduces them. A private crate would move the DTO and marker-field parts of items 4 and 5 to the compiler, at the cost in section 2.4.

**Residual the compiler does not see.** A scalar read in one decision could be copied into another decision's
computation. Decisions are one function per claim kind that take only `&Reader`, so such a copy must cross a function
boundary as a parameter, which the signature rule of item 4 rejects.

### 3.4 Typed keys and the output adapter

All internal storage and lookup use `ClaimKey`: `BTreeMap<ClaimKey, Settled<_>>` in the store. One adapter,
`Store::publish` (`settle/store.rs`), renders the public `AdmissionReport` and is the only place that knows the
spellings:

- `Input(id)` renders as `input:{id}`, `ReportCutoff` as `report-cutoff`, `Slots(Producer)` as `slots:producer`,
  `Comparison(id)` as the bare name for `actual_before`, `actual_after`, `service_first` and `client_first` and as
  `comparison:{id}` otherwise, `Usage{index, id}` as `usage:{index}:{id}`, `ReviewAction(id)` as `review-action`
  when the id is `review-action` and as `review-action:{id}` otherwise, `Media{claim, receipt}` as
  `media:{len}:{claim}:{receipt}`, and so on for every variant, character for character as today.
- **Ordering** is the `BTreeMap<String, ClaimDecision>` order of the rendered strings, as today. Internal map order
  never reaches the output.
- **Publication conditions** stay as today: `document-schema` appears only when refused; `reference` only when its
  input is unadmitted or missing; `metric:`, `row:`, `row-duplicate:`, `limit:` and `warning:` only with a reason;
  internal facts (`SubjectHead`, `Clock`, `Cutoff`, `Selection`, `Merge`, `Receipt`, `Slot`, `SchemaBasis`,
  `MediaProof`, `DocumentClock`, `DocumentSchema` when admitted, `KnownTotal`) never.
- **Counts** are computed at publication as the per-code count over published decisions. That equals today's
  incremental `AdmissionReport::insert` bookkeeping, because each published key has one final decision.
- **The media collision.** Today `Ledger::resolve` appends `:` to a generated media key until it is free, so a user
  claim spelled like a generated key takes the key that callers read. Internally the two are distinct keys. The
  adapter keeps the `:` loop for byte compatibility, and ingress refuses a user claim whose id has the generated
  shape `media:{n}:...` (`receipt_conflict` at that claim, dimension `claim_id`). The replay has no such id.
- **Refusal presentation.** A dependency's refusal is rendered with today's record id and code where the state is not
  a census case. A fact can render its refusal under the key that used to carry it (the existing
  `RequiredDependency::with_refusal_code` mechanism), so a bad document clock still reads `missing_cutoff` at
  `report-cutoff`.

### 3.5 Acyclicity

The evaluator settles on demand with memoization and an active set. A cycle is a design defect: it refuses with
`dependency_cycle` and fails a test assertion that runs on every fixture. The graph is acyclic by construction of
the facts:

**Admission.**

1. `SubjectHead`: intrinsic identity of the subject head (unique candidate, slot valid, not Before or Held). It does
   not read any selection.
2. `Clock`: the mode instant. Live: report time equals cutoff. Historical: the subject head committed at the cutoff
   (reads `SubjectHead`). Version: selection time equals cutoff. Every mode: cutoff not after report time (parity
   with the frozen check).
3. `Selection(subject)`: **subject self-exemption.** The subject's own selection is a `SelfExemption` built from
   `Clock`, or a `PinProof` checked against `Clock`. It never reads `Cutoff`.
4. `Cutoff`: `Clock`, plus `Selection(subject)` in a historical review (the topic head defines the cutoff and the
   merge head). This replaces the handwritten historical subject edge in `Ledger::claim`.
5. `Selection(other)`: a `PinProof` checked against `Cutoff`, or an `ExemptionGrant` that reads
   `Selection(subject)`, `Cutoff` and the input's relation to the subject.
6. `Receipt(id)` reads `Selection(origin)` and, for independent-cutoff origins, `Cutoff`. `Merge(id)` reads
   `SubjectHead`, `Cutoff` and `Receipt(cutoff evidence)`. `Slot(id)` reads the slot record and `Input(id)`.
7. **Media versus schema claim.** `SchemaBasis(claim)` is the projection without media (entry, references, config,
   root inventory; reads `Slot`, `Input`, `Selection`, `Receipt` closure, `Cutoff`). `MediaProof{claim, receipt}`
   reads `SchemaBasis(claim)` and its descendants' `MediaProof`. `Claim(claim)` reads `SchemaBasis` and every
   required `MediaProof`. The published `Media{claim, receipt}` reads `MediaProof` and `Claim(claim)`: a media
   assertion that promises a usable schema is published admitted only after that schema settles.

In a live review an input with its own pin depends on `Cutoff = Clock` only, so a failed subject selection does not
refuse independent siblings (the behaviour pinned by `fold2_version_admission_needs_proof_and_clock` and document
rows D14, A36 and A38). An exempt input depends on the subject's selection in every mode.

**Frozen.** `DocumentSchema` and `DocumentClock` (report and cutoff instants, live equality, cutoff not after report
time) read nothing. Then `Input(id)` (declared dependencies, and `DocumentClock` for an independent-cutoff origin),
then `Slots(role)`, then `ReportCutoff` (`DocumentClock`, subject, slots, holders, waivers), then `Comparison`,
then `Usage` and `ReviewAction`. `BindingStatus`, `CallChange` and `Membership` read `Slots` and `Input`. Every
accessor of a frozen record also records `DocumentSchema`, the record's wire validity.

**Markers.** `MarkerTable` is settled at index construction from reader output and reads no admission decision.
Saved status and membership read it after the admission facts they need.

### 3.6 Reader rules that are runtime rules, not compile-time

The compiler and the import check together guarantee that policy reaches values only through accessors. Four reader rules carry real semantics
and are pinned by tests (section 7):

- **Populations keep duplicates.** `inputs_where(pred)` iterates raw records, so a duplicated id is visible. Each
  match is recorded, and a duplicated id resolves to its conflict decision. Non-members are not recorded.
- **Waived reads are recorded.** A waiver's branch condition is a read (`input(id)` before deciding a slot input is
  waived records `Input(id)`).
- **Optional reads are explicit** (`reader.optional`, allow-listed) and do not block.
- **The grant constructors' signatures are the waiver rules.** `ExemptionGrant::new` takes the subject's settled
  selection, the settled cutoff and a settled subject relation (the input is the subject head or the computed merge
  result of it, or a sideless consumer Before of the subject's repository). Whether that relation is right is a
  runtime rule with a test.

### 3.7 Derived markers: one settled value, five consumers

`schema::markers::settle` computes the `MarkerTable` from the same reader output that today fills
`MessageCoverage`, `WireEvidence`, `OperationSite::status_inventory`, `operation_gaps`,
`operation_coverage_incomplete` and `request_authored_but_excluded`. The public fields keep their current values: the
reader's `Coverage::Checked` waiver for an empty projection with unproved media (`schema/messages.rs`, `project`) stays
in `MessageCoverage`, because the diff and the bindings wire read it and their bytes must not move. The table records
the truth (body authority unproved). Its answers are defined to agree with the diff's predicates
(`diff/fields/evidence.rs`, `diff/status/pairing.rs`, `diff/operations.rs`), and a parity test checks every marker
state against the diff.

What each consumer does with an unproved marker (the decisive case: an empty field projection with unproved media):

| Consumer | Today | After |
|---|---|---|
| Diff (`diff/*`) | Scoped `media_unverified` limit, element unknown. | Unchanged (it is the reference); pinned to the table by the parity test. Moving it onto the table is a later, optional cleanup. |
| Saved status, field branch (`ElementFacts::from_index`) | Reads the flag; misses partial coverage without a JSON source and operation gaps outside a matching gap. | Asks `MarkerTable::field_presence`; unproved gives unknown, so `evaluate` returns `membership_unchecked` or `shape_incomparable`. |
| Saved status, operation branch (`operation_shape_covered`) | Ignores the flag, the status inventory and the fallback message. | Asks `MarkerTable::operation_shape`; any unproved root, withheld status set or fallback caveat makes the shape not comparable, so `shape_incomparable` and no `binding_current`. |
| Bindings serialization (`BindingsDocument::with_schema_inventory`) | Writes coverage `checked`, strips `media_unverified`, `body_presence` from `feasible.is_some()`. | Unchanged wire bytes (the replay builds bindings documents). The serialized `coverage` is not certification evidence; P4 must not read it as such (section 8). |
| Membership (`membership()`, `covers()`) | Reads only the bindings rows, which carry neither the flag, the status inventory, the dialect exclusion nor the gaps. | The certified entry (section 3.8) needs a `NegativeProof` from the admitted index of each basis input; unproved, withheld, excluded or gap gives `membership_possible` and no negative. |

Known metric totals (`with_usage_counts`, `model/impact/membership.rs`) become a settled `KnownTotal(name)` marker
whose settle function reads the frozen inputs it counts through the document reader: a total is known only when
every counted input is admitted, and otherwise null with a reason.

### 3.8 The public DTO boundary and caller-built inputs

- **Request and document DTOs** stay public-field serde types, decoded and serialized as today. Their role becomes
  ingress and output only. Inside the crate only the ingress modules and the output builders dereference them.
- **Certified entry points**: `admit_inputs`, `admit_schema_input`, `saved_binding_status`, `ImpactDocument::validate`
  and a new `unobserved_elements_admitted(request, inventories)`. The existing `unobserved_elements` stays
  byte-compatible for current callers as an uncertified computation over bindings alone (owner question 2).
- **`index_schema` over a caller-built `AdmittedSchemaInput`** stays public. The resulting `SchemaIndex` carries a
  private provenance flag, `CallerBuilt`, unless it came from `SchemaAdmission::index` (admitted). The flag is not
  serialized. Certifying consumers (P4) refuse caller provenance. `SchemaIndex::default()` has an empty marker table,
  which answers unknown everywhere.
- **Caller `ProducerInventory`** (diff) stays a documented caller contract. P4 builds it from admitted sources.
- **`AdmissionReport` and `ClaimStatus`** are output. No in-crate code takes them as input, and P4 must not.

## 4. Settlement paths

"Reads today" is the census's route: (a) a settled claim or derived edge, (b) a handwritten `RequiredDependency`,
(c) a raw record or field, (d) no edge. Steps are defined in section 6.

| Path | Function (file) | Reads today | Afterwards | Step |
|---|---|---|---|---|
| P1 | `ClaimDecision::from_reasons` (`revision/admission/gate.rs`) | dependencies handed in | moves to `settle/gate.rs`, private to the evaluator; dependencies come from the recorder | S2 |
| P2 | `AdmissionReport::settle` (`revision/admission.rs`) | string keys from the caller (b) | replaced by `Evaluator::settle(key, f)` | S2-S4 |
| P3 | `AdmissionReport::insert` (`revision/admission.rs`) | overwrites and adjusts counts | store insert plus `Store::publish` (counts at publication) | S1 |
| P4 | `Ledger::selection_claim` (`revision/admission/graph.rs`) | `InputFacts.selection` (a) | `Selection(id)` settled fact with a `SelectionOutcome` grant | S5a |
| P5 | `Ledger::claim` (`revision/admission/graph.rs`) | slot record (c), own selection (a), subject edge in historical only (b) | claim policy reads `Slot`, `Input`, `Selection`, `Receipt` closure and `Cutoff`; the subject edge is derived through `Cutoff` | S6 |
| P6 | `Ledger::resolve` (`revision/admission/graph.rs`) | inserts decisions, media keys with `:` suffix | evaluator demand plus adapter publication | S1, S6 |
| P7 | `Ledger::resolve_schema` media loop (`revision/admission/schema_claim.rs`) | projection without the claim's edges (d) | `SchemaBasis`, `MediaProof`, published `Media` (section 3.5) | S6 |
| P8 | `selection::selection`, `selection_required` (`revision/admission/selection.rs`) | raw subject role, repository, snapshot (c) | selection policy returning `PinProof`, `ExemptionGrant` or `SelfExemption` | S5a |
| P9 | `Ledger::new`, `Ledger::validate_receipt` (`revision/admission/graph.rs`) | origin `InputFacts.selection` (a') | ingress input identity; `Receipt(id)` reads `Selection(origin)`, `Cutoff` | S5a |
| P10 | `selection::merge` (`revision/admission/selection.rs`) | merge against `expected_merge` (c) | `Merge(id)` reads `SubjectHead`, `Cutoff`, `Receipt(cutoff evidence)` plus relation rules | S5b |
| P11 | `Ledger::claim` optional media (`revision/admission/graph.rs`) | admitted attachments only (a) | optional reads (allow-listed); attached cycles still refuse | S6 |
| P12 | `admit_schema_input` (`revision/admission/projection.rs`) | report decision and projection (a) | reads `Settled<Claim>` and `Settled<SchemaBasis>` | S6 |
| P13 | `saved_binding_status`, `evaluate` (`revision/staleness.rs`) | admission decisions, raw input role (c), caller saved index | admission store through the reader; marker answers; caller saved index stays a known limit | S7 |
| P14 | `segments::check` (`revision/staleness/segments.rs`) | scan segment scope from the scan (c) | segment receipts read through the admission reader; scope-from-scan stays a known limit | S7 |
| P15 | `settle_inputs` (`model/impact/validation.rs`) | `input:{dep}` (b) | `Input(id)` reads declared dependencies through the reader | S2 |
| P16 | `slot_cutoff` (`model/impact/input_validation.rs`) | subject, holders from a deduplicated map, raw waived inputs (b, c) | `ReportCutoff` reads `DocumentClock`, `Slots`, `Input`; duplicate-preserving holders; `LinkWaiver` | S3a, S3b |
| P17 | `validate_slots` (`model/impact/input_validation.rs`) | `input:` edges (b), raw `no_first_parent` reason (c) | `Slots(role)`, `Reference`; `no_first_parent` stays a disclosed limit | S3a |
| P18 | `validate_status` (`model/impact/input_validation.rs`) | raw producer slot ids (c) | `BindingStatus(i)` reads `Slots(Producer)`, `Input`, `DocumentSchema` | S4a |
| P19 | comparisons in `validate` (`model/impact/validation.rs`) | cutoff and both inputs (b), `pair_for` over raw slots (c) | reads `ReportCutoff`, `Input`, `Slots`, `DocumentSchema` | S4b |
| P20 | usages in `validate` (`model/impact/validation.rs`) | comparison keys (b), raw presences and slots (c) | reads `Comparison`, `Slots`, `Input`, `DocumentSchema`; match outcome and site id rules | S4b |
| P21 | review action in `validate` (`model/impact/validation.rs`) | comparison key (b), row ids | `ReviewAction` reads `Comparison`; rows remain integrity links | S4b |
| P22 | `validate_auxiliary` (`model/impact/validation.rs`) | raw consumer ids (c), `input:` (b) | `CallChange` reads `Slots(Consumer)`; `Membership` and `Risk` read `Input`, rows, `DocumentSchema` | S4a |
| P23 | refusal-only keys (`model/impact/validation.rs`) | none | `DocumentSchema` always settled, published only when refused; `Metric`, `Row`, `RowDuplicate`, `Limit`, `Warning` as today | S2, S4a |
| P24 | `with_schema_inventory` provenance `"proven"` (`model/bindings.rs`) | caller index labels (c) | unchanged wire; certification requires an admitted-provenance index | S8 |
| P25 | `unobserved_elements` (`model/impact/membership.rs`) | parsed evidence, bindings by id (c) | certified entry with `NegativeProof`; population and identity rules | S8 |
| P26 | `with_usage_counts` (`model/impact/membership.rs`) | raw comparisons, first client record (c) | `KnownTotal` read through the document reader | S8 |
| P27 | `with_schema_diff`, `with_coverage_limits` (`model/impact/projection.rs`) | diff over caller indexes | unchanged; rows are not certified (documented) | none |
| P28 | `omission_proof`, `valid_media_proof` (`schema/messages/media.rs`) | catalog flag `cutoff_admitted` (c) | proof cutoff from the receipt's origin and `Cutoff`; the flag is ignored | S6 |
| P29 | `diff_boundary`, `basis::admit`, `producer::compare`, status membership (`diff*.rs`) | caller inventories | unchanged; documented caller contract | none |
| P30 | `project` waiver (`schema/messages.rs`) | sets `Coverage::Checked` for an unproved empty projection | public fields unchanged; `MarkerTable` records unproved | S7 |
| P31 | `ElementFacts::from_index`, `operation_shape_covered` (`revision/staleness/inventory.rs`) | raw markers, two ignored (c) | `MarkerTable::field_presence`, `operation_shape` | S7 |
| P32 | `validate_input`, `compare_slot`, `project_scope` (`revision/admission/slots.rs`) | slot records (c) | ingress identity and `Slot(id)` | S5a |
| P33 | `message_roots` serialization (`model/bindings.rs`) | raw coverage, limits, `feasible` | unchanged wire bytes; not certification evidence | S8 (documented) |
| P34 | `membership()`, `covers()` (`model/impact/membership.rs`) | serialized bindings rows (c) | `NegativeProof` on the certified entry | S8 |
| P35 | `input_refusals` (`model/impact/input_validation.rs`) | raw `doc.cutoff_at` (c) | part of `Input(id)`, reads `DocumentClock`; origin identity parity | S2 |
| P36 | media key placement in `Ledger::resolve` (`revision/admission/graph.rs`) | `contains_key` collision loop | typed `Media` key, adapter loop, reserved-namespace refusal | S1 |
| P37 | `Documents::new`, `document_for` (`schema/references.rs`) | admitted reference bytes | unchanged; what may enter references is ruled in `SchemaBasis` (named-tree references from the claim input only) | S6 |
| P38 | `media::openapi`, `media::swagger` (`schema/messages/media.rs`) | declared media in entry bytes | input to `MarkerTable`; unchanged semantics | S7 |

Paths P27 and P29 are accounted for and deliberately unchanged: they produce rows and diffs that the frozen
validator does not certify, or they consume documented caller inputs.

## 5. Coverage statement

Counts over the census's 89 concrete-state cases. The per-case receipt is kept with the lane evidence; these totals
equal its tally.

| Class | Cases | Serious | Uncertain | Minor |
|---|---:|---:|---:|---:|
| Compile error: recording reader | 14 | 14 | 0 | 0 |
| Compile error: waiver grant types | 7 | 7 | 0 | 0 |
| Compile error: settled-marker table (certified entry) | 8 | 8 | 0 | 0 |
| **Compile error, total** | **29** | **29** | 0 | 0 |
| Runtime rule plus RED test | 23 | 19 | 4 | 0 |
| Disclosed limit | 33 | 1 | 5 | 27 |
| Not a defect or duplicate | 4 | 0 | 0 | 4 |
| **Total** | **89** | **49** | **9** | **31** |

- **Disclosed limits by incidence in the 37-pair replay**: observed 0; constructed with replay incidence 0, 30; not
  measured, 3 (marker states whose incidence depends on schema contents). No disclosed limit was observed.
- **The 33 disclosed limits** are 15 unread frozen fields and identity relations no claim relies on, 13 documented
  known limits and recorded design decisions (caller saved inventory, scan contents, caller bindings provenance,
  uncertified rows, absence usages, record-internal path consistency, segment scope from the scan, computed-tree
  time, no cutoff edge for binding status and call changes, the copyable public status, the caller-built schema input
  and producer inventory), 4 further cases (a Held alias read differently by receipt readers, the fallback caveat
  not carried by saved or membership presence, the bindings `body_presence` misread, `content_hash` excluding
  coverage) and 1 serious case (an unevidenced `no_first_parent` slot waiver, guarded in P4).
- **The 4 that are not defects**: a duplicate, a case with a mechanism-level proof that the admitted decision rests on
  no unchecked value (owner to confirm), a missing consumer slot that is a valid shape, and a conservative
  direction in which no negative is emitted.
- **The 42 census cases without a concrete state** (26 already through the gate, 16 with a mechanism-level none-found
  or design reason) are unchanged. Four carry later corrections, which are counted under the cases they point to.
- **Unverified.** Every state is traced, not executed; each class becomes firm when its RED test fails on the
  pre-change code. Six cases rest on an owner decision or an unverified assumption (owner questions 3, 4, 7 and 8,
  and two disclosed cases: the Held alias and `content_hash`). The incidence of the 12 marker-dependent cases in the replay is not measured.
  Their fixes touch no replay output by construction (the replay has no saved status or membership, and the bindings
  wire does not change); step S7's measurement confirms it (section 6).
- **Against the agreed estimate** (section 2.3). Kept: the derived-marker type as a second type; of the about a dozen
  marker and builder cases, 9 become compile errors (8 through the marker table, 1 through the reader) and 3 runtime
  rules; and the minor unread-field cases as disclosed
  limits (15). Corrected: 29, not about 49, cases become compile errors (20 of the 37 serious cases the estimate
  placed there, plus 9 from the marker and builder set); 16 serious cases are runtime rules, because their mechanism
  is a population, relation, unconsulted value or self-asserted proof that a recorded read does not prevent.

## 6. Migration

Every step keeps the 37-pair replay byte-identical (section 7.4), runs the crate's full test suite, keeps every
file under 1,000 weighted lines (aim 600; `python3 scripts/file_size.py`), and has a review cap of two rounds (one
for S0). At the cap the round is classified under the lane's convergence rule. Converging findings are folded and
the extension is disclosed. Open-class findings go back to this design.

No file in the crate is over 650 weighted lines today, and none would pass 1,000 under this plan. Step 0 is still
needed, because Rust privacy is per module: ingress and policy cannot share a file.

| Step | Goal | Files touched / new | Functions | Weighted lines (changed) | RED / probes |
|---|---|---|---|---:|---|
| S0 | Move-only split along responsibility boundaries: `graph.rs` into `facts.rs`, `walk.rs`, `claim.rs`, `resolve.rs`; `validation.rs` into a driver plus `comparisons`, `usages`, `auxiliary`, `links`, `inputs`; `input_validation.rs` into `intrinsic`, `slots`, `cutoff`, `status` | 3 / 13 | 0 changed | about 1,650 moved, 0 changed | byte-identity of moved bodies |
| S1 | Typed keys, output adapter, publication conditions, counts at publication, reserved media namespace | 9 / 3 (`settle.rs`, `settle/key.rs`, `settle/store.rs`) | about 20 | about 550 | 1 RED, key table |
| S2 | Evaluator core (recorder, `Settled`, gate move, optional reads, cycle check); frozen `DocumentSchema`, `DocumentClock`, `Input(id)`; the import check | 5 / 5 | about 12 | about 750 | 2 RED, 1 probe, 6 core |
| S3a | Frozen cutoff and slots through the reader; `LinkWaiver`; duplicate-preserving holders | 3 / 2 | about 8 | about 550 | 5 RED, 2 probes |
| S3b | Frozen relation and parity rules: conflict population, live computed After, inferred anchor, computed Before, merge base and target; the frozen half of the waived-Before date rule (its RED test is counted in S5a) | 2 / 0 | about 6 | about 400 | 5 RED |
| S4a | Binding status, call changes, membership, risk, refusal-only keys | 3 / 3 | about 8 | about 500 | 5 RED, 1 probe |
| S4b | Comparisons, usages, review action; match outcome and site id rules | 2 / 1 | about 6 | about 450 | 4 RED |
| S5a | Admission ingress; `SubjectHead`, `Clock`, `Cutoff`, `Selection`, `Receipt`, `Slot`; waiver grants; saved status reads the store | 6 / 5 | about 15 | about 750 | 8 RED, 2 probes |
| S5b | `Merge(id)` relation rules; admission conflict population; After exemption relation | 2 / 1 | about 5 | about 350 | 4 RED |
| S6 | Claim policy, `SchemaBasis`, `MediaProof`, published `Media`; proof cutoff from origin; reference identity | 6 / 2 | about 12 | about 750 | 6 RED, 1 probe |
| S7 | `MarkerTable`, diff parity test, saved status through marker answers; marker incidence measured on the 37 pairs before merge | 6 / 1 | about 10 | about 650 | 4 RED, 2 probes |
| S8 | Certified membership entry with `NegativeProof`; population, completeness and identity rules; `KnownTotal` | 3 / 1 | about 8 | about 600 | 8 RED, 1 probe |
| S9 | Close the audit (every policy file under the import check); delete known-limit rows 12-15; add 5 known-limit pins; README and DESIGN status update | 4 / 1 | 0 product | about 300 | 5 pins |

Total: 13 PRs, about 6,600 changed weighted lines plus 1,650 moved.

**Order and dependencies.** S0 and S1 come first. S2 needs S1. S3a, S3b, S4a and S4b need S2 and touch only the
frozen layer. S5a needs S2. S5b and S6 need S5a. S7 needs S5a (saved status reads admission). S8 needs S7 (the
marker table). S9 comes last.

**Before P4.** Every step may merge before P4. P4 may implement against any of them, but it must not certify:

- any admitted or frozen cell before S1-S6 have merged;
- a saved-status cell before S7;
- a membership negative or a known total before S8.

S9 is part of P4's final gate.

**Exit gate per step.**

1. The step's RED tests fail on the pre-change code and pass after.
2. Their controls are unchanged.
3. Its probes fail with the seam on and pass with it off.
4. The full crate suite passes.
5. The replay byte comparison shows zero differences.
6. The import check passes for every migrated directory.
7. File sizes are within the rule.

An existing test expectation that encodes a census case changes only in the step that fixes that case, and the PR
lists each such change. A failure outside the step's scope is reported, not re-baselined.

## 7. Test-first plan

Every RED test is written first, fails on the pre-change code, and has a control that must stay admitted or
unchanged. Counts are per step; the private appendix maps each case to its test.

### 7.1 RED tests by shape

| Step | RED tests (shape) | Controls |
|---|---|---|
| S1 | 1: a user claim spelled like the generated media key while the real media proof is refused | the media fixture without the collision: identical bytes |
| S2 | 2: a frozen document whose report time precedes the cutoff, and an input with an independent-cutoff origin (admitted today, refused after); an independent-cutoff origin with no covered tree | report time equal to cutoff; origin covering the input tree |
| S3a | 5: a Before input with an unknown dependency in a historical computed document and in a live joint document; a second record with the Before input id; a consumer slot naming a producer input; a computed holder copied under a fresh id followed by a commit with that id (both orders) | the same documents without the defect; a single holder |
| S3b | 5: topic head and pinned Before on one branch with different revisions; a live computed After without a merge receipt; a live client-subject document without a subject head; a computed Before without merge evidence; a computed After with unrelated proposed base and target | agreeing revisions; a commit After; a recorded subject head; a commit Before; consistent base and target |
| S4a | 5: a client-subject document with producer before and after different plus a binding status; a service-subject document with the consumer before and after different plus a call change; a typed binding status outside the enum; a typed membership with an empty basis; a scan client outside the slots with no receipt | consistent slots; a valid status; a basis of one admitted input; a pinned scan client |
| S4b | 4: a usage with null match references over refused consumer slots; a typed usage with unknown presence and no reason; a usage site id absent from its match references; a usage naming an unchecked or unbound match | consistent slots; a valid usage; matching site ids; a checked, bound match |
| S5a | 8: a live subject whose pin has the wrong revision, read by a claim on an exempt input; the same read through a receipt; the same read by saved status for a consumer subject; a historical subject committed off the cutoff with a metadata claim; a live report time different from the cutoff; the same with a scan; a historical report time before the cutoff; a waived Before committed after the cutoff | live siblings with their own pins (existing pins); a subject committed at the cutoff; report time equal to cutoff (the replay shape); a Before committed before the cutoff |
| S5b | 4: a Held pin disagreeing with the unpinned topic head on one branch; a live merge head different from the subject revision; merge base and target unrelated to the Before revision; a historical After turned into an unrelated later commit in the subject's repository | agreeing pins; merge head equal to the subject; consistent base; a computed merge-result After |
| S6 | 6: a historical media request whose topic pin is wrong; the same with an independent-cutoff source in the media closure; a live media request whose subject pin is wrong; a refused required media alongside a valid one; a catalog flag claiming cutoff admission for a receipt not admitted at the cutoff; a named-tree reference from another input | valid topic and subject pins; all media valid; a named-tree media receipt of an admitted snapshot; a reference from the claim input |
| S7 | 4: a Swagger operation with an empty object response, no media declaration and no media receipt, bound saved location with an empty signature; three OAS 3.0 response shapes (a repeated `responses` member, a default-only response, an empty `responses` beside an unrelated gap); a duplicated path item beside an unrelated gap; a required Swagger body parameter without media evidence and an unknown field id | the same operations with proven JSON media or readable responses |
| S8 | 8: a waived root with a change naming a field under it; a field under a withheld status set; a request field of a dialect-excluded request; an operation behind an unreadable-operation gap; a known total over an input with an unknown dependency; evidence marked incomplete; bindings naming another tree for the same input id; two evidence records for one input, one with an unknown member | proven roots, readable status sets, eligible requests, readable operations, admitted inputs, complete evidence, matching identity, identical records |

Total 52 RED tests: 29 for compile-time cases, 23 for runtime rules. S9 adds 5 known-limit pins beside controls, for
the disclosed cases that are serious or uncertain: the `no_first_parent` waiver, the caller-built schema input, the
caller producer inventory, the Held alias, and `content_hash` with coverage.

### 7.2 Mutation-style probes for the compile-time claim

A compile-time claim says that a case is impossible because one recording point (an accessor, a grant constructor or
a marker answer) records or requires the value. Each recording point gets a `#[cfg(test)]` seam, inline in the crate
(`src/settle/probe_tests.rs`), that drops that one key or requirement. With the seam on, every RED test the point
covers must go back to admitted ("remove the dependency, the RED row goes green wrongly"); with it off they refuse.

10 probes cover the 29 compile-time cases:

| Recording point | Cases it covers |
|---|---|
| frozen `input(id)` | 4 |
| `slots(role)` | 4 |
| `DocumentSchema` | 3 |
| `DocumentClock` | 1 |
| `SchemaBasis` in a media proof | 2 |
| the subject-selection argument of `ExemptionGrant` | 4 |
| the clock and cutoff arguments of the grants | 3 |
| `operation_shape` | 2 |
| `field_presence` | 2 |
| `NegativeProof` | 4 |

A recording point with no RED test that fails under its seam has an unproven compile-time claim. The runtime rules
need no seam: each RED test fails on the pre-change code by construction.

### 7.3 Core and boundary tests (S1, S2, S7)

- **Key rendering table.** Every `ClaimKey` variant renders to today's spelling; a round trip over every key in the
  existing fixtures.
- **Evaluator core.** A read of a withheld fact refuses even when the value is unused (control: the same read of an
  admitted fact admits). A cycle refuses with `dependency_cycle` and trips the assertion. An optional read does not
  block. Publication conditions hold (`document-schema` absent when valid, `reference` only when unadmitted). Counts
  at publication equal today's on every existing fixture.
- **The import check** (`tests/policy_imports.rs`): items 4-7 of section 3.3. A planted DTO import in a scratch copy
  of a policy file must fail it, and that self-check is part of the test.
- **Compile-fail doctests on the public surface**: `ClaimStatus` cannot be constructed and `Settled` is not exported.
  Crate-internal constructors are kept private by module privacy, and the import check rejects named calls outside
  their modules.
- **Marker parity** (S7). For every marker state in the census's marker-by-consumer matrix, the table's presence and
  authority answers equal the diff's.

### 7.4 Replay byte comparison (every step)

Build the pinned 37-pair replay driver against the base and against the step head. Run both on the pinned inputs.
Compare every output byte for byte: the admission reports, the validation reports, the 111 documents, the 618
changes and the 5,645 limits, including ordering, absent optional fields and refusal presentation. Matching counts
are not enough. Any difference fails the gate and is resolved before merge, never re-baselined. A corrected marker
that would change a replay byte is a compatibility conflict that goes to the owner. Step S7 additionally counts, on
the 37 pinned schemas, the marker states the table classifies differently from the public fields, and reports where
they would surface if the replay had saved status or membership.

## 8. What P4 needs and what this does not fix

### 8.1 Minimum for P4

1. S1-S6 merged before P4 certifies any admitted or frozen cell. Both the request route and direct
   `ImpactDocument::validate` reach the shared policies. A positive cell binds its role, input, side, tree, scope,
   mode clock and receipt closure; a computed result needs its merge evidence; a waived Before needs its grant.
2. S7 merged before any saved-status cell (current, stale, no saved match, established absence) is certified, and
   S8 merged before any membership negative or known total. Until then P4 prints those cells as **Not checked** and
   retains independent observations.
3. P4 certifies only from the certified entry points (section 3.8). It refuses a `SchemaIndex` with caller provenance
   and builds `ProducerInventory` from admitted sources. It never reads the bindings wire `coverage` or
   `body_presence` as evidence, never takes an `AdmissionReport` as input, and refuses to certify a slot that uses
   `no_first_parent`.
4. The replay byte comparison of section 7.4 passes on P4's own head.
5. P4 keeps its plan's evidence-removal controls (PLAN, Slice P4, gates 1 to 5). This redesign makes those controls
   meaningful: removing a record removes a recorded dependency.

### 8.2 What it does not fix

- **Marker producers.** The table records whatever the schema reader sets. The census did not enumerate the states of
  the producer modules (`schema/wire/composition.rs`, `wire/keywords.rs`, most of `wire.rs` and `wire/constraints.rs`,
  `element/*`, `schema/servers*`, `schema/openapi/parameters.rs`, `schema/signature.rs` beyond `content_hash`). A
  producer that sets a marker wrongly is outside this redesign. The parity test pins consumers to the diff, not the
  reader to the truth.
- **The 33 disclosed limits** of section 5, including the 15 unread fields and identity relations, the bindings wire
  bytes (`coverage`, `body_presence`), and `content_hash` as defined in DESIGN 5.6.
- **Caller-built inputs.** `index_schema` over a caller-built `AdmittedSchemaInput`, a caller `ProducerInventory` and a
  caller saved inventory stay computable and uncertified; only their provenance is marked.
- **The uncertified `unobserved_elements`** keeps the four membership cases fixed on the certified entry.
- **Runtime mutation** of inputs after admission, and anything that is out of model for the lane.
- **Incidence.** The replay exercises one request shape. The redesign removes ways to write the defect; it does not
  measure how often the fixed states occur in live use.
- **Completeness of the census.** No census round called it complete. Members of the class in paths the census did
  not read are covered only to the extent that their reads go through the reader. Path P29 and the diff's own reads
  remain outside it.

## 9. Open questions for the owner

1. **Private policy crate.** Default: no. Use module privacy plus the CI import check (section 2.4). Revisit if a
   second crate ever depends on `prism-boundary` policy.
2. **The uncertified `unobserved_elements`.** Default: keep it byte-compatible and documented as uncertified; P4 uses
   `unobserved_elements_admitted`. Remove the old entry after P4.
3. **Frozen anchor inference without a subject head** (the client-only fixture used by the fold controls). Default:
   align with admission (no inferred anchor). Add `subject_head_input_id` to that fixture in S3b and list the changed
   controls.
4. **`no_first_parent`.** Default: disclose; P4 does not certify such a slot. Alternative: refuse the waiver until the
   document can record the parent-absence evidence.
5. **Marker states in the replay.** Default: S7 measures before merge, and merges only if no replay byte moves (none
   should, by construction).
6. **The diff onto the marker table.** Default: no; the parity test only.
7. **Named-tree reference blob from another input.** Default: refuse (`slot_mismatch`, dimension `input_id`).
   Independent-cutoff references keep their free identity.
8. **Waived Before committed after the cutoff.** Default: refuse in both layers (S3b, S5a).
9. **Interim P4.** Default: P4 may start building after S6, with saved-status and membership cells **Not checked**
   until S7 and S8.
10. **Refusal codes.** Default: no new codes. The reserved media namespace uses `receipt_conflict` with dimension
    `claim_id`, and new dependencies render as `missing_dependency` at their rendered key.
