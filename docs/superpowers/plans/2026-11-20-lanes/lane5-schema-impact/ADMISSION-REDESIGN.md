# Lane 5 amendment: admission redesign for `prism-boundary` (design of record)

Status: design only, no product code. Revision 4, after review rounds 1 to 3 (change log in section 1.6), in two phases set by a census of real history (section 6.0). Applies to crate `crates/prism-boundary` at
the P1d head (PR #958). Every statement about code names a repository path and function. Paths are relative to
`crates/prism-boundary/src` unless they start with `crates/`, `docs/`, `tests/` or `scripts/`. The code at the P1d
head was read, not run: every input state below is traced from the code and becomes a RED test before anything
relies on it.

## 1. Problem and goal

### 1.1 The defect class

The crate decides, for a schema-impact report, which inputs and claims are admitted (proven to come from a named
tree of an admitted snapshot, or independently evidenced at a cutoff), waived, proven or known. Six review rounds of
the P1d slice and a three-round read-only census found one defect class again and again:

> A decision (admitted, waived, proven, known, or a derived marker) can read a value without that read becoming a
> dependency of the decision.

The census counted 131 cases over 38 settlement paths (P1-P38, listed in section 4). 105 of them bypass the
settlement gate. 89 of those come with a concrete input state in which a decision is admitted while something it
read is not. Of the 89, 49 are serious (the decision certifies something that rests on an unadmitted or unchecked
value), 9 are of uncertain materiality and 31 are minor or already documented. 37 of the 38 paths lie outside
`revision/admission/gate.rs`. The census counts are approximate (about plus or minus 3), and every state was traced,
not executed.

### 1.2 What is already true

- `ClaimStatus` (`revision/admission/gate.rs`) is opaque. It has a private field and one constructor,
  `ClaimStatus::decide`, which only `ClaimDecision::from_reasons` calls. A decision is admitted only when no reason
  remains and every `RequiredDependency` it was handed is admitted. Status minting is not the gap.
- Admission requests enter through one constructor. `admit_inputs` (`revision/admission.rs`),
  `admit_schema_input` (`revision/admission/projection.rs`) and `saved_binding_status` (`revision/staleness.rs`)
  all call `Ledger::new`, and `Ledger` is `pub(in crate::revision)`.
- The frozen layer has one public entry, `ImpactDocument::validate` (`model/impact.rs`). It takes any typed or
  hand-edited document.

### 1.3 Why the gap exists

The dependency list is handwritten at each call site. There are 25 calls of `ClaimDecision::from_reasons` or
`AdmissionReport::settle` outside `gate.rs`, in six files:

- `model/impact/validation.rs`: 15;
- `model/impact/input_validation.rs`: 4;
- `revision/admission/graph.rs`: 3;
- `revision/admission/schema_claim.rs`, `revision/admission/projection.rs` and `revision/admission.rs`: 1 each.

The frozen layer settles on string keys (`format!("input:{id}")`, `"report-cutoff"`). Settlement code reads
public-field request and document DTOs directly (`AdmissionRequest`, `ImpactDocument`, `Change`, `Usage`, and others,
in about 2,300 lines). Derived markers (`Coverage::Checked`, the `"proven"` provenance, presences, metric totals,
`body_presence`) are plain enums or strings set far from the gate and re-derived by each consumer. Nothing connects
"this decision read X" with "X is a dependency of this decision".

Three examples, all read in the code:

- `selection::selection` (`revision/admission/selection.rs`) tests the subject's role and repository and returns
  `Ok(())` without reading the subject's own selection decision. That return is a waiver of the receipt requirement.
  `Ledger::claim` (`revision/admission/graph.rs`) adds the subject's selection as a dependency only in a historical
  review.
- `slot_cutoff` (`model/impact/input_validation.rs`) builds its merge-holder population from a map in which the last
  record with a given input id wins, so a duplicated holder is lost before any lookup. It also waives the Before input
  through `selection_required` on the raw record, without an `input:` edge.
- `ElementFacts::from_index` (`revision/staleness/inventory.rs`) re-derives operation comparability from
  `message.coverage` and the limits. It never reads `WireEvidence::empty_projection_with_unproved_media`, the status
  inventory or the fallback message, all of which the diff honours.

### 1.4 Why instance-by-instance folds did not converge

Each fold added the missing edge or rule for the instances named in its round. Each new review round and each census
round then found new members of the same kind, through another reader, another consumer pair or another record field.
The three census rounds found 81, 17 and 33 cases: the count is not shrinking. Under the lane's convergence rule this
is an open class. The fix has to remove the way to write the defect, not its instances.

### 1.5 Goal

> After this redesign a settlement decision cannot read a value without depending on it, for the paths the design
> lists; the design says truthfully what it leaves open.

"The paths the design lists" are the 38 settlement paths of section 4 on the certified entry points of section 3.9.
Section 5 counts what the redesign turns into a compile error, what becomes a runtime rule with a test, and what
stays a disclosed limit.

The goal is about dependencies, not about evidence. On a hand-edited or foreign document, recording every read makes
`ImpactDocument::validate` say which records a decision rests on, but those records are still the document's
assertions about itself. Frozen validation certifies local invariants and reference integrity. P4 certifies a frozen
cell only for a document it produced from admission in the same run (section 3.9).

### 1.6 Revision 4 change log

One targeted fold of the round-3 findings. Nothing else was reopened.

| Finding | Kind | What changed |
|---|---|---|
| Opus W1, sol 1: `Merge(id)` re-entered itself through `input(id)` | OBSERVED | Kinds up to and including `Merge` read an identity view (`identity(id)`) that records `Input(id)` only; `Merge` reads `Input(id)`; `Merge(id)` is recorded only for kinds after `Merge`, through one recording function (3.2, 3.4, 3.6, 3.7). New S5a controls: fold3 `effective` and a live computed subject head. |
| Opus W2, sol 2: the conflict group refused a valid pin beside a waived Before | OBSERVED | Group = today's observed pins; in a historical review only, plus the topic head's binding; never an unpinned Before (3.2, 3.6). Controls: `tests/support/fold3.rs:206` and an agreeing pin beside a branch-carrying waived Before. |
| Sol 3: two projections with one receipt closure shared a marker view | CONSTRUCTED | The view key includes the admitted projection or scan-inventory identity and its distinguished entry (3.2, 3.8, S7 control). |
| Sol 5 (round 2) / sol 4 (round 3): sealed proofs do not force the right proofs | CONSTRUCTED | Exact-pair and complete-basis proofs, each from its own constructor bound to the decision key; their view and population checks are runtime rules (3.3, 3.8). Coverage recounted: compile errors 29 -> 17, runtime rules 27 -> 39 (2.3, 5, 7, 8.2). |
| Opus W3: a disagreeing duplicate refused an independent sibling | CONSTRUCTED | The conflict group classifies it as an admitted `Excluded(receipt_conflict)` and skips it; holder populations keep the refusing form (3.7). |
| Opus S1: `SegmentPopulation` was not listed as a caller contract | CONSTRUCTED | Listed in 3.9 and 8.2, bound to the saved scan's tree; `SavedScanEvidence` gains a field (3.9). |
| Opus S2: Merge seam did not cover every recording point | CONSTRUCTED | One Merge-recording function and one seam (3.7, 7.2). |
| Sol 6 (round 3, immaterial): an alias could take a retained user key | CONSTRUCTED | Aliases avoid retained user keys too; P36 says "actual collision" (3.5, 4). |
| Sol P19 (round 3, immaterial): P19 read `DocumentSchema` | CONSTRUCTED | Edge removed; the invalid-document control pins the comparison outcome (4, 7.1 S4b). |
| Opus S3: S2 preservation tally | OBSERVED | Counted as 2; preservation tests 8 -> 9 (6, 7.1). |

## 2. Designs considered and the choice

### 2.1 Five designs

| Design | Mechanism | Cost (estimate) | How it fails | Takes from it |
|---|---|---|---|---|
| A. Mandatory read tokens | A read returns `(value, token)`; a decision must be handed the tokens of what it read. | 12-20 files, 6-9 PRs. | The value and the token separate: code uses the value and drops the token. `#[must_use]` or a non-`Copy` token does not force consumption into the right decision. A missing predicate still type-checks. | The intent: no value without a dependency. |
| B. One settle function per claim kind, DTOs hidden | Each claim kind has one settlement function with typed inputs; request and frozen adapters share it. | 15-22 files, 6-9 PRs. | A helper can collapse candidates (drop a duplicate) before handing over a correctly typed input, and the compiler sees the right type. Only tests or review detect it. | The per-kind settle functions, the hidden DTOs and the shared relation rules (section 3.3, 3.7). |
| C. Dependencies recorded at read time | A per-decision reader records the key of every value before returning it; the gate uses the recorded set instead of a handwritten list. | 12-20 files, 5-8 PRs. | A policy that never asks for a value (an unconsulted match outcome, a justification never read) produces a consistent trace. A cached value detached from its source does the same. | The recording reader and closed population queries (section 3.2, 3.6). |
| D. Mechanical CI audit | A test inventories settlement functions and raw-field reads and compares them with declared dependencies; evidence-withdrawal tests per claim. | 3-6 files, 2-4 PRs, no product change. | A new helper, an unexecuted branch or a field absent from both lists passes. Detection is by CI on represented cases only. | The import check and the probes (section 7). |
| E. Narrow the certified contract | Refuse unsupported shapes at the entry points; disclose the rest. | 5-9 files, 2-4 PRs. | It leaves 86 bypassing cases (41 serious) on shapes P4 must support: live, consumer side, saved status, membership. A shape check cannot refuse schema-content markers, and the frozen entry takes foreign documents. | Nothing beyond the existing provisional status. |

### 2.2 Choice

**B + C, with three proof types (waiver grants, population proofs, membership proofs), settled marker facts, and D
as the regression check.**

- **C, the recording reader.** It removes the second, handwritten account of what a decision read. Policy code reaches
  values only through a reader, which records each value's key before returning it, and through closed population
  queries. The gate derives the dependency list from that record.
- **B, one settle function per claim kind.** Each takes only a reader and its key. One dispatcher picks the function
  from a fixed table by matching on the key, so there are no closures, and the dispatcher never names a DTO. The DTOs and the schema index are unreadable by policy code
  (section 3.3).
- **Proof types.** A waiver, a population, a comparable shape and a membership negative are values, not booleans. A
  selection or link waiver can be returned only as a grant, and the grant constructor makes the justifying reads
  itself. A known total needs a population proof. A saved current or stale outcome needs one exact-pair proof over
  the saved and the selected shape. A certified negative needs one complete-basis proof. Both are built only in
  modules separate from the decisions that consume them, by constructors bound to the decision key. The owner's
  "second type" covers waivers, populations, shapes and negatives.
- **Settled marker facts.** Derived markers are settled once per admitted view (a projection or a rebuilt scan
  inventory) into `Marker{view, root}` and `OperationFacts{view, operation}` facts, inside the ingress module that holds
  the index.
  Saved status (`SavedStatus{query}`) and membership (`Unobserved{query, region}`) become settled decisions whose
  policies read those facts through the reader and never hold the index. This is the owner's second type for derived
  markers.
- **D, the regression check.** It catches what the compiler cannot: a policy module naming a DTO or a raw marker
  field, an optional read outside its allow-list, a settle call outside the dispatcher. Single-cause probes and
  recorded-key assertions make each compile-time claim testable.

### 2.3 Correction of the compile-time coverage estimate

The agreed direction estimated three things for the 89 concrete-state cases: about 49 compile errors (35 to 37 of the
serious ones), about 12 more with the marker type, and about 14 minor cases left disclosed. Checked against the code
case by case (section 5):

- **17, not about 49, become compile errors, all of them serious.** 12 come through the recording reader and 5
  through waiver grants. None comes through a proof type alone: revision 3 counted 29, and round 3 showed that a
  sealed proof does not force a decision to obtain the right proof (section 5 lists the 12 cases that moved).
- **Of the 37 serious cases the estimate placed at compile time, 17 are compile errors after the redesign, 19 are
  runtime rules with a RED test, and 1 is a disclosed limit.** A recorded read cannot prevent 16 of those 19, because
  in each the deciding read is never made, or it is made correctly against the wrong population or relation:
  - a merge-receipt holder lost before the lookup, because the input map keeps one record per id;
  - selection-conflict populations that leave out the topic head's binding in a historical review, in both layers;
  - relation rules on admitted inputs:
    - a non-subject After commit in the subject's repository exempted;
    - merge base, target and head compared only with another request field;
    - frozen computed Before or After inputs accepted without merge evidence;
    - a live computed After accepted without a merge receipt;
    - an inferred frozen anchor;
    - a scan client outside the slots;
    - an independent-cutoff origin without a covered tree;
    - the report time never compared with the cutoff in historical and version reviews;
  - a match outcome never consulted by the usage that names it, and a usage site id never compared with its match;
  - a media proof's cutoff flag self-asserted in the catalog bytes;
  - a published media decision that does not depend on the schema claim it promises;
  - a user claim id that collides with a generated media key.

  The other 3 moved from compile errors in the round-3 recount: a receipt's read of its origin's selection and two
  media decisions whose chain passes through `SchemaBasis` reading the input's selection. Each is a read of a gate
  whose value the decision does not otherwise need, so omitting it still compiles (section 5).
- **All of the about 12 marker and builder cases are runtime rules with a RED test.** Revision 3 counted 8 compile
  errors through sealed proofs and 1 through a population proof. The exact-pair and complete-basis proofs, and the
  population proof of a known total, are bound to the decision key, but which views, which basis and which population
  their constructors take is a runtime check, so omitting the right read still compiles. The other three are an unread
  completeness flag, bindings matched by input id alone, and an evidence record dropped at parse time before the
  duplicate test. A marker type constructed after any of those three operations preserves the mistake.
- **The minor cases.** The 15 field and identity cases stay disclosed, as estimated. Four documented limits move to
  runtime rules: the saved inventory's closure, the saved negative's segment population, and the cutoff edge for
  binding status, call changes and membership. Also disclosed are 9 documented known limits and design decisions, 4
  further cases with uncertain or no consumer impact, and 1 serious case, an unevidenced `no_first_parent` waiver: 29
  disclosed in all. 4 cases are not defects.

The estimate's direction holds; its size does not. The difference is the population of cases whose mechanism is a
rule, not a read.

### 2.4 Private policy crate versus module privacy

One option puts policy in a private implementation crate that does not depend on the DTO crate, so the compiler
forbids policy code from naming a DTO. For this workspace that costs more than it buys:

- Policy needs the vocabulary types (`Role`, `ServiceSide`, `ReviewMode`, `SnapshotInput`, `ReceiptOrigin`,
  `SelectionReceipt` proofs, `Scope`). They would move to the new crate and be re-exported, or be duplicated as fact
  enums. That is an estimated 1,500 to 2,500 moved lines and a second `Cargo.toml` before any defect is fixed.
- No crate in the workspace depends on `prism-boundary`, and every change runs CI. A source-scanning test therefore
  rejects a DTO import in the same PR that adds it. The workspace already relies on such a test for the core crate
  (`tests/integration/layering_test.rs`).

The design therefore uses module privacy plus a CI import check and states the difference (section 3.3). The
private crate stays an owner option (section 9).

## 3. The design

### 3.1 Layout

```
src/settle.rs              crate-private module root
src/settle/gate.rs         moved from revision/admission/gate.rs: ClaimStatus, the gate
src/settle/key.rs          ClaimKey, PopulationQuery, refusal-id and publication rendering
src/settle/store.rs        Settled<T>, the evaluator store (arena of typed facts), the output adapter
src/settle/dispatch.rs     the one match from ClaimKey to a fixed `fn(&R, &K) -> Outcome` table; generic over the
                           ingress source handle, names no DTO
src/settle/reader.rs       recorder core, optional reads, the active-read rule
src/settle/waiver.rs       PinProof, ExemptionGrant, SelfExemption, LinkWaiver
src/settle/rules.rs        relation rules shared by both layers, over a facts trait both readers implement
src/settle/legacy.rs       temporary bridge for unmigrated callers (deleted in S6)
src/revision/admission/ingress.rs        AdmissionReader: holds the request privately; population queries
src/revision/admission/policy/{clock,selection,receipt,merge,claim,media}.rs
src/model/impact/frozen/ingress.rs       DocumentReader: holds the document privately; population queries
src/model/impact/frozen/policy/{inputs,cutoff,slots,status,auxiliary,records,links}.rs
src/schema/markers.rs                    marker answers per admitted view; ComparableOperation, ComparablePair (sealed)
src/revision/staleness/ingress.rs        SavedReader: holds the saved and selected indexes privately; settles Marker,
                                         OperationFacts and SegmentPopulation
src/revision/staleness/policy.rs         SavedStatus settle function
src/model/impact/membership/ingress.rs   MembershipReader: holds indexes and bindings privately
src/model/impact/membership/proof.rs     MembershipProof and CompleteBasis constructors (separate from the emitter)
src/model/impact/membership/admitted.rs  Unobserved settle function (the emitter)
tests/policy_imports.rs                  the CI import check
```

Each step creates the files it needs and moves only the functions it migrates (section 6). `ClaimStatus` keeps its
public path through a re-export in `revision/admission.rs`.

### 3.2 Keys

`ClaimKey` is a closed, `pub(crate)` enum. A key names a proposition and carries no authority, so anyone in the crate
may construct one.

**Admission keys.**

| Key | Settles |
|---|---|
| `Record(id)` | the record is unique among records with that id |
| `Input(id)` | `Record(id)` plus slot validity (`validate_input`); reads no selection and no merge. Its value is the identity view: intrinsic snapshot and slot identity (tree, `computed_at`, merge receipt id, slot role, side and repository) and the recorded Before revision and target branch of a computed snapshot |
| `SubjectHead` | intrinsic identity of the subject head |
| `Clock` | the mode instant |
| `PinObservation(id)` | admitted whenever `Clock` is; its value is `Observed{rev, branch}` or `NotObserved(reason)` for the input's receipt checked against `Clock` |
| `SelectionConflicts(repo, branch)` | admitted; its value is the set of input ids in conflict on that repository and branch. The group is today's: the observed pins on that branch. In a historical review only, it also holds the subject topic head's binding. It never holds an unpinned Before, or any input whose binding revision does not claim the branch tip at the cutoff |
| `Classified(query, id)` | the classification of one record by one population query; refused only when copies of the record disagree on the fields the query classifies by, except in the conflict group, where such a record is the admitted value `Excluded(receipt_conflict)` (section 3.7) |
| `SubjectSelection` | the subject head's own selection; its own type, so it cannot stand in for another input's |
| `Cutoff` | `Clock`, plus `SubjectSelection` in a historical review |
| `Selection(id)` | the selection of an input other than the subject head |
| `CutoffEvidence(id)` | an independent-cutoff receipt; reads only `Cutoff` |
| `Merge(id)` | the merge proof of a computed tree, with today's checks; reads `Input(id)` (the identity view), `SubjectHead`, `Cutoff` and `CutoffEvidence`. Only kinds after `Merge` record `Merge(id)` (section 3.6) |
| `Receipt(id)` | a named-tree or derived receipt; a named-tree receipt reads the final `Selection(origin)` |
| `Slot(id)` | a claim slot |
| `Claim(id)` | a user claim |
| `SchemaBasis(claim)` | the projection without media |
| `MediaProof{claim, receipt}` | one media proof |
| `Media{claim, receipt}` | the published media decision |

**Frozen keys.** `DocumentSchema`, `DocumentClock`, `Record(id)`, `Input(id)`, `Slots(Role)`, `Reference`,
`ReportCutoff`, `Comparison(id)`, `Usage{index, id}`, `ReviewAction(id)`, `BindingStatus(i)`, `CallChange(i)`,
`Risk(i)`, `Membership(i)`, `Metric(name)`, `Row(id)`, `RowDuplicate(id)`, `Limit(id)`, `Warning{index, claim}`.

**Marker, saved and membership keys.**

| Key | Settles |
|---|---|
| `Marker{view, root}` | the derived-marker answer for one root of one view; a view is a claim projection or a scan inventory rebuilt inside ingress, identified by input, effective scope, receipt closure, the admitted projection or scan-inventory identity and its distinguished entry |
| `OperationFacts{view, operation}` | the root population and status coverage of one operation in one view |
| `SegmentPopulation{scan}` | the segment population and scope of a saved scan, independently evidenced (a caller contract supplied by P4, section 3.9) |
| `ExtractionCoverage{input}` | expected files, uncovered regions and target-check completeness of an admitted inventory |
| `SavedStatus{query}` | a saved-status outcome |
| `Unobserved{query, region}` | a certified membership negative |
| `KnownTotal(name)` | a known metric total |

**Population keys.** `Population(PopulationQuery)`. The query is a closed enum defined in the ingress modules, with
one variant per population a policy may ask for:

- admission: `SubjectCandidates`, `PinnedInputs`, `ConfigEntries(key)`, `GeneratedMediaKeys`, and the claim, input
  and receipt id lists;
- frozen: `InputRecords`, `MergeHolders(receipt)`, `SelectionReceipts`, `SlotInputs(role)`, `ScanClients`,
  `CountedInputs(metric)`, `RequiredComparisons(metric)`, and the usage, comparison and row lists;
- membership: `EvidenceRecords(input)` and `BasisInputs(query, region)`.

### 3.3 Types and who constructs them

| Type | Fields and visibility | Constructed by | Guarantee |
|---|---|---|---|
| `Recorder` / `AdmissionReader` / `DocumentReader` | private fields: the DTO or index reference, the store, the recorded keys (first-read order, deduplicated), the optional keys | only the ingress source handle, when the dispatcher asks it for one reader per decision; the settle function receives `&Reader` and the recorder is consumed when it returns | Every accessor records its key before returning. A withheld fact is still recorded, so reading and ignoring it refuses the decision. |
| `Settled<T>` | private: key, `ClaimDecision`, `Option<T>`; stored in an arena of a typed fact enum, handed out by index | only the evaluator in `settle/store.rs`, after the gate | The value of a settled fact is reachable only through a reader, which records the key. |
| `ClaimStatus` | unchanged, opaque | only `settle/gate.rs` | As today. `from_reasons` is reachable outside `settle/` only through `settle::legacy` until S6. |
| Waiver grants: `PinProof`, `ExemptionGrant`, `SelfExemption`, `LinkWaiver` | private | only functions in `settle/waiver.rs` that take `&Reader` and make the justifying reads themselves | A selection or cutoff decision returns `SelectionOutcome::{Pinned(PinProof), Exempt(ExemptionGrant), SubjectHead(SelfExemption)}`. There is no admitted variant without a grant, and the grant cannot be built from the wrong selection. |
| `PopulationProof<Q>` | private: the query, the members | only the settle function of `Population(q)` | A decision that needs a population (holders, counted inputs, required comparisons, evidence records) takes the proof. |
| `ComparableOperation` | private: the view, the operation, the shape | only `schema/markers.rs`, from the `OperationFacts` and every root's `Marker` answer of that view | One view's shape; a withheld fallback, a withheld status set or an unproved root yields no proof. It is an input of `ComparablePair`, never an outcome's proof on its own. |
| `ComparablePair` | private: the decision key, the saved and the selected `ComparableOperation` | only `schema/markers.rs`, by one constructor that takes the `SavedStatus{query}` reader, derives both views from that key and builds both `ComparableOperation`s itself | A `binding_current` or `binding_stale` outcome takes exactly one `ComparablePair` for its own key; two single-view proofs cannot stand in for it. Whether the constructor took the right two views is a runtime rule with a test. |
| `MembershipProof` | private, three variants: `SchemaRoot`, `RouteTable`, `Consumer` | only `membership/proof.rs`, a module separate from the emitter: from `OperationFacts` and `Marker` facts (schema producer), from an admitted route-table inventory (route producer), or from `ExtractionCoverage` of an admitted consumer inventory (consumer), each tied to the query, side and population | One basis input's proof; an input of `CompleteBasis`, never an outcome's proof on its own. |
| `CompleteBasis` | private: the decision key, one `MembershipProof` per required basis input | only `membership/proof.rs`, by one constructor that takes the `Unobserved{query, region}` reader, reads `Population(BasisInputs(query, region))` and builds every member's `MembershipProof` itself | A negative cannot be emitted on the certified entry without the `CompleteBasis` of its own key, and the emitter cannot build one. Whether the population is the complete required basis is a runtime rule with a test. |

`ClaimDecision`, `Refusal` and `AdmissionReport` stay as they are: they are output. Each settle function returns an
outcome holding the output fields today's code fills (`input_id` when admitted, requested and effective scope,
`selection`, `uncovered_scope`, `dirty`, `media_root`, `evidence`). The store copies them into the published decision
unchanged.

### 3.4 How policy code is kept from the DTOs and the index

**Enforced by the compiler.**

1. The DTO reference, and on the saved-status and membership paths the `SchemaIndex` and the `BindingsDocument`,
   live in private fields of the reader types in the ingress modules (`revision/admission/ingress.rs`,
   `model/impact/frozen/ingress.rs`, `revision/staleness/ingress.rs`, `model/impact/membership/ingress.rs`). Marker
   facts are settled inside the ingress module by `schema::markers::answer(&self.index, root)`; no reader has an
   accessor that returns an index, an `Operation` or a bindings document. Policy modules are siblings of the ingress
   modules, not descendants, so they cannot read those fields. A helper method on `Operation` or `SchemaIndex` does
   not help a policy: policy never holds either value.
2. Accessors return view types defined in the ingress module. Each view exposes only the fields owned by the key it
   records. `identity(id)` returns the identity view of `Input(id)` (id, role, repository, side, snapshot kind, tree,
   revision, `computed_at`, merge receipt id, recorded Before revision and target branch) and records `Input(id)` only;
   it is the accessor of every kind up to and including `Merge`. `input(id)` returns the same view for kinds after
   `Merge` and also records `Merge(id)` for a computed snapshot. Neither returns the selection receipt, which only
   `selection(id)` reaches. A raw DTO sub-object is exposed only by the accessor
   whose key owns all of it, for example the `SelectionReceipt` inside the settle function for `Selection(id)`.
3. Populations are closed queries. A policy names a `PopulationQuery` variant and receives a `PopulationProof`. It
   never passes a predicate or a closure over records.
4. The dispatcher matches on `ClaimKey` and calls an entry of a fixed table of type `fn(&R, &K) -> Outcome`. It is
   generic over an ingress-owned opaque source handle that hands out readers, so it never has a DTO in scope. The
   evaluator accepts only keys. No closure or extra argument built outside the reader reaches a decision.
5. `Settled`, the grants, `PopulationProof`, `ComparableOperation`, `ComparablePair`, `MembershipProof` and
   `CompleteBasis` have private constructors, each in a module other than the decision that consumes it.

**Enforced by a CI import check** (`tests/policy_imports.rs`, a source scan in the style of
`tests/integration/layering_test.rs`).

6. Policy files and the saved-status and membership policies may not name the multi-key DTO types or the index types:
   `AdmissionRequest`, `AdmissionInput`, `AdmissionReceipt`, `AdmissionClaim`, `InputSlot`, `ImpactDocument`,
   `DocumentInput`, `Change`, `RoleSlots`, `Comparison`, `MatchRecord`, `Usage`, `BindingStatus`, `SavedScan`,
   `ChangeSelection`, `BindingsDocument`, `SchemaIndex`, `Operation`, `MessageCoverage`, `WireEvidence`. Settle
   functions take only a reader and a key, and the check pins their signature. `settle/dispatch.rs` may name none of
   these types, and no reader may declare a method returning an index, an `Operation` or a bindings document.
7. No file other than the ingress modules, `schema/`, `diff/` and the bindings serializer may name a raw marker field:
   `coverage`, `limits`, `feasible`, `status_inventory`, `json_source`, `declared_media`, `operation_gaps`,
   `operation_coverage_incomplete`, `wire_evidence`, `request_authored_but_excluded`, `status_refusals`.
8. `settle(`, `Evaluator::` and the reader constructors may not appear outside `settle/` and the ingress modules.
   `from_reasons` and `RequiredDependency` may not appear outside `settle/`, except the closed list of legacy callers
   that `settle::legacy` serves until S6. Policy modules may not name `ClaimStatus`.
9. Every `reader.optional(..)` call site must be on the test's closed allow-list. Today that list is direct media
   attachments and inapplicable comparison links.

**The difference.** The compiler stops policy code from obtaining a DTO or index value it was not handed, from
building an admitted outcome without its proof, and from deciding on a population it did not read. It does not stop
anyone from adding a DTO parameter to a settle function or from naming a marker field, because those types and
fields are visible crate-wide. The CI check rejects both in the PR that introduces them. A private crate would move
the type and field parts of items 6 and 7 to the compiler, at the cost in section 2.4.

### 3.5 Typed keys and the output adapter

All internal storage and lookup use `ClaimKey`. One adapter, `Store::publish` (`settle/store.rs`), renders the public
`AdmissionReport` and is the only place that knows the spellings.

- **Published spellings, character for character as today.**
  - `Input(id)` renders as `input:{id}`, `ReportCutoff` as `report-cutoff`, `Slots(Producer)` as `slots:producer`.
  - `Comparison(id)` renders as the bare name for `actual_before`, `actual_after`, `service_first` and
    `client_first`, and as `comparison:{id}` otherwise.
  - `Usage{index, id}` renders as `usage:{index}:{id}`.
  - `ReviewAction(id)` renders as `review-action` when the id is `review-action`, and as `review-action:{id}`
    otherwise.
  - `Media{claim, receipt}` renders as `media:{len}:{claim}:{receipt}`.
- **Ordering** is the `BTreeMap<String, ClaimDecision>` order of the rendered strings, as today. The internal order
  never reaches the output.
- **Publication conditions stay as today.**
  - `document-schema` appears only when it is refused.
  - `reference` appears only when its input is unadmitted or missing.
  - `metric:`, `row:`, `row-duplicate:`, `limit:` and `warning:` appear only with a reason of their own. These
    refusal-only keys check their own record and do not read `DocumentSchema`, so an invalid document publishes the
    same refusal-only keys as today.
  - Internal facts are never published.
- **Counts** are computed at publication: per code, over the published decisions. That equals today's incremental
  `AdmissionReport::insert` bookkeeping, because each published key has one final decision.
- **Refusal-id rendering for internal keys.** A dependent's `missing_dependency` names a dependency the way today's
  handwritten lists did:
  - `Selection(id)`, `SubjectSelection`, `Record(id)` and `Input(id)` render as the bare input id;
  - `Receipt(id)`, `CutoffEvidence(id)`, `Merge(id)` and `MediaProof` render as the receipt id;
  - `SchemaBasis` and `Claim` render as the claim id;
  - `Clock` and `Cutoff` render as `missing_cutoff` at the reading input's id, through the existing
    `RequiredDependency::with_refusal_code`;
  - `DocumentClock` renders as `missing_cutoff` at `report-cutoff`;
  - `Population(q)` and `Classified` render as the id of the first refused record;
  - `DocumentSchema` renders as `missing_dependency` at `document-schema` without copying `invalid_document`, so
    that code is counted once, as today.
- **Closure reads keep today's direct refusals.** A claim's receipt closure is walked inside its settle function. Each
  visited `Receipt` or `CutoffEvidence` is recorded as a direct dependency: a refused receipt contributes its own
  reasons, as today's walk does, without an added `missing_dependency`. The wrong-digest variant of row A12 therefore
  keeps today's reasons and counts.
- **Canonical dependency order.** Each settle function reads in today's per-kind order: the order of today's
  handwritten list, or the `BTreeSet` order where today's code used one. The gate takes the first unadmitted
  dependency in that order. Where the redesign adds a dependency (for example an exempt input now reads the subject's
  selection), the record id of a refused decision can change. Each such change lands in the step that fixes the
  corresponding case, and the step lists the fixtures it changes (section 6).
- **The media collision.** Generated media records own their canonical keys. A user claim is refused only on an actual
  collision: its id equals a generated key of this request, `media:{len}:{claim}:{receipt}` for a schema claim and a
  media receipt in that claim's receipt closure (the `GeneratedMediaKeys` population). The refusal is
  `receipt_conflict`, dimension `claim_id`. A media-shaped id that collides with nothing (`media:2024:x`) stays
  admitted as today. The adapter publishes generated keys first and gives a refused user claim the first
  `:`-suffixed alias that is free of every generated key, every retained user key and every alias already given, so
  an alias never takes another record's published key. A canonical lookup therefore
  returns the generated record. In-crate consumers use typed lookup. The replay has no media.

### 3.6 Acyclicity and cycles

There are two kinds of edge.

**Kind edges** form a fixed DAG over key kinds. `settle/key.rs` holds a static table of which kinds may read which.
The reader asserts every recorded read against it, and a test checks the table for cycles once. Same-kind data edges
are permitted self-loops guarded by the active-read rule: `Input` to `Input` (declared dependencies), `Receipt` to
`Receipt` and `CutoffEvidence` within a closure walk, `MediaProof` to `MediaProof` (descendants).

**Admission kind order.**

1. `Record` and `Input`: intrinsic, no selection.
2. `SubjectHead`: intrinsic identity of the subject head (unique candidate from the `SubjectCandidates` population,
   slot valid, not Before or Held).
3. `Clock`: the mode instant.
   - Live: report time equals cutoff.
   - Historical: the subject head committed at the cutoff.
   - Version: selection time equals cutoff.
   - Every mode: cutoff not after report time. This is parity with the frozen check.
4. `PinObservation(id)`: settled admitted whenever `Clock` is, with the value `Observed{rev, branch}` or
   `NotObserved(reason)`. It refuses only through `Clock`, so one input's bad pin never withholds another input's
   decision.
5. `SelectionConflicts(repo, branch)`: one verdict per repository and branch, admitted, whose value is the set of
   conflicting input ids. The group is today's (`selection.rs`, `receipt_conflict`: any differing revision marks
   every member): the observed pins on that branch. In a historical review only, it also holds the subject topic
   head's binding, because the head committed at the cutoff is the branch at the cutoff. It never holds an unpinned
   Before, or any input whose binding revision does not claim the branch tip at the cutoff: a waived Before sits at
   an earlier revision of the same branch, and in a live review the subject head's revision is no claim about the
   branch tip. In a live or version review the group is the observed pins only. This stage is upstream of every
   final selection, so the historical chain from `Selection(other)` through `Cutoff` to `SubjectSelection` cannot
   close.
6. `SubjectSelection`: subject self-exemption. The subject's own selection is either a `SelfExemption`, which reads
   `Clock`, or a `PinProof` built from its `PinObservation` and its own branch's `SelectionConflicts`. It never reads
   `Cutoff`.
7. `Cutoff`: `Clock`, plus `SubjectSelection` in a historical review, because the topic head defines the cutoff and
   the merge head. This replaces the handwritten historical subject edge in `Ledger::claim`.
8. `CutoffEvidence(id)` (reads only `Cutoff`), then `Merge(id)` (reads `Input(id)` through `identity(id)`,
   `SubjectHead`, `Cutoff` and `CutoffEvidence`, with today's merge checks).

   **Identity view up to `Merge`.** Every kind up to and including `Merge` (`Record`, `Input`, `SubjectHead`,
   `Clock`, `PinObservation`, `SelectionConflicts`, `SubjectSelection`, `Cutoff`, `CutoffEvidence`, `Merge`) reads a
   snapshot only through `identity(id)`, which records `Input(id)` and nothing else. So `Merge(X)` reading its own
   snapshot does not re-enter `Merge(X)`, and a computed subject head read by `SubjectHead` or a pinned computed Held
   read by `PinObservation` does not record a `Merge` key before its kind. `Merge(id)` is recorded only for readers
   whose kind comes after `Merge`: the `Selection` grants, `Receipt`, `Slot`, `SchemaBasis` and `Claim`. The kind
   table asserts it: an edge into `Merge` from a kind at or before it is a kind-order violation.
9. `Selection(other)`: a `PinProof` (its `PinObservation`, `Cutoff`, and its own branch's `SelectionConflicts`,
   refused only when its id is in the conflict set, with today's `receipt_conflict` bytes), or an `ExemptionGrant`
   (section 3.7).
10. `Receipt(id)` and `Slot(id)`.
    - A named-tree `Receipt(id)` reads the final `Selection(origin)`.
    - `Slot(id)` reads the slot record, `Input(id)` and, for a computed snapshot, `Merge(id)`.
11. Media versus schema claim.
    - `SchemaBasis(claim)` is the projection without media. It reads `Slot`, `Input`, `Selection`, the `Receipt`
      closure and `Cutoff`.
    - `MediaProof{claim, receipt}` reads `SchemaBasis(claim)` and its descendants' `MediaProof`.
    - `Claim(claim)` reads `SchemaBasis` and every required `MediaProof`.
    - The published `Media{claim, receipt}` reads `MediaProof` and `Claim(claim)`. A media assertion that promises a
      usable schema is published admitted only after that schema settles.

In a live review an input with its own pin depends on `Cutoff = Clock`, its own pin observation and its own branch's
conflict verdict, all admitted unless its own pin or the clock fails, or another observed pin on the same branch
disagrees with it (today's rule). The live group holds observed pins only, so a failed subject selection, the
subject head's revision, a waived Before on the branch or another branch's bad pin does not refuse independent
siblings. That is the behaviour pinned by `fold2_version_admission_needs_proof_and_clock` and by
document rows D14, A36 and A38. An exempt input depends on the subject's selection in every mode.

**Frozen kind order.**

1. `DocumentSchema`, `DocumentClock` and `Record(id)`.
2. `Input(id)`.
3. `Slots(role)`.
4. `ReportCutoff`.
5. `Comparison`.
6. `Usage` and `ReviewAction`.

`BindingStatus`, `CallChange` and `Membership` read `Slots`, `Input` and `ReportCutoff`. The accessors of usage,
binding-status and membership records also record `DocumentSchema`, the record's wire validity; these are the typed
records whose fields a decision reads without its own check. Inputs, slots, the cutoff and comparisons keep their
own checks and do not read `DocumentSchema`, so an invalid document still settles them independently (the control
`fold2_direct_dto_still_needs_wire_closure` stays byte-identical).

**Marker kind order.** `Marker` and `OperationFacts` read only the admitted claim and receipt closure that produced
their view. `SegmentPopulation` and `ExtractionCoverage` read the admitted evidence that supplies them.
`SavedStatus` and `Unobserved` read those facts, `Claim` and populations.

**Data edges** are defined by the records: a frozen input's declared dependencies, the receipt closure, a media
proof's descendants. They stay inside one settle function and keep today's algorithms: `Ledger::walk` for receipt
closures, and the active set of `settle_inputs` for frozen inputs. The active-read rule: reading a key that is being
settled records it and yields a refused fact carrying `dependency_cycle` at that key, with today's evidence path.
Today's cycle outputs are byte controls: row A16 of `tests/fixtures/p1d_rows/expected-admission.json`, the cycle
tests in `tests/support/fold2.rs` and `tests/support/fold3.rs` (count 1, and the optional-attachment case that stays
admitted), and the frozen input cycle path. The evaluator releases store borrows before it recurses. A kind edge that
violates the table is a design defect and fails a test assertion on every fixture. A data cycle is an input state and
refuses exactly as today.

### 3.7 Reader and grant rules that are runtime rules

The compiler and the import check guarantee that policy reaches values only through accessors, closed population
queries and grants. Five rules carry real semantics, and tests pin them (section 7).

- **Populations record what they visit.** The settle function of `Population(q)` records `Classified(q, id)` for every
  record it visits, members and non-members alike. `Classified` refuses only when copies of a record disagree on the
  fields the query classifies by, so a duplicated holder whose copies differ blocks the decision, while a duplicated
  unrelated input whose copies agree on those fields does not (today's "affects its claims only"). For a population
  whose members must be admitted (`CountedInputs`, `BasisInputs`), the settle function also records `Input(id)` for
  each member. That member recording is a recording point with its own seam (section 7.2). A non-member's full
  admission is not a dependency. This is the "trusted intrinsic selector": independent siblings stay independent,
  and the classification still depends on the records that produced it. The membership tests of each query (which
  inputs hold a merge receipt, which observations share a repository and branch) are runtime rules. The conflict
  group is the exception to refusing: a pinned record whose copies disagree on repository or branch is classified as
  the admitted value `Excluded(receipt_conflict)`, which the group skips as it skips `NotObserved`. The record itself
  is refused as today (`slots::unique`), and an independent pinned sibling stays admitted. The holder populations
  (`MergeHolders`, `CountedInputs`, `BasisInputs`) keep the refusing form, as the S3a RED requires.
- **Waived reads are recorded.** A waiver's branch condition is a read: `identity(id)` or `input(id)` records
  `Input(id)` before the policy decides that a slot input is waived.
- **Every computed snapshot read after `Merge` reads its merge.** One function in `settle/reader.rs`,
  `record_merge(reader, id)`, is the only place that records `Merge(id)`. The admission `input(id)` accessor,
  `Slot(id)` and the grant constructors that read a computed tree (`subject_repo_before`, `subject_result`,
  `consumer_before`) all call it, for every computed snapshot, whatever the selection outcome. It is callable only
  from readers of kinds after `Merge`; kinds up to `Merge` use `identity(id)` (section 3.6). A pinned Held computed
  tree, a computed tree in a version comparison and a sideless computed client After keep today's merge check. The
  one function has one seam (section 7.2), which covers every recording point at once.
- **Optional reads are explicit.** `reader.optional` is allow-listed, and an optional read does not block.
- **The grants are the waiver rules.** Each constructor takes `&Reader` and makes its reads itself:
  - `ExemptionGrant::subject_repo_before(reader, input)` covers an input at position Before with the subject's role
    and repository. Its snapshot must be a commit, or a computed tree whose `Merge(id)` it reads. It reads
    `SubjectSelection` and `Cutoff`. When the snapshot records a commit date, that date must not be after the cutoff,
    and a later Before refuses. This is the shape of the replay's service Before, which stays admitted.
  - `ExemptionGrant::subject_result(reader, input)` covers an After input that is the subject's computed merge
    result. It reads `Merge(id)` and `SubjectSelection`. A plain After commit in the subject's repository that is not
    the subject head gets no grant.
  - `ExemptionGrant::consumer_before(reader, input)` covers a sideless consumer Before in the subject's repository.
    It reads `SubjectSelection`, `Cutoff` and, for a computed snapshot, `Merge(id)`.
  - `SelfExemption::new(reader)` reads `Clock`.
  - A version comparison has no grant: every input needs a `PinProof`.
  - `LinkWaiver` (frozen) reads `Slots` and the waived input.

  Whether each relation is right is a runtime rule with a test.
- **Relation rules are shared.** `settle/rules.rs` implements the rules both layers apply over a facts trait that
  both readers implement: the selection requirement and the grants, the merge relation, the conflict population and
  the clock relation. The frozen layer adopts the module in S3a and S3b, and admission adopts it in S5a and S5b.
  Between S3b and S5b the two layers can differ on the relation rules S3b changes. That interim divergence is disclosed in
  those PRs, and a rule-by-layer parity matrix test lands in S5b.

### 3.8 Derived markers: one settled value, five consumers

`schema::markers` answers from the same reader output that today fills `MessageCoverage`, `WireEvidence`,
`OperationSite::status_inventory`, `operation_gaps`, `operation_coverage_incomplete` and
`request_authored_but_excluded`.

- The public fields keep their current values, because the diff and the bindings wire read them and their bytes must
  not move. That includes the reader's `Coverage::Checked` waiver for an empty projection with unproved media
  (`schema/messages.rs`, `project`).
- The marker answer records the truth: body authority unproved.
- `Marker{view, root}` and `OperationFacts{view, operation}` are settled inside the ingress module from one view: the
  projection of an admitted claim, or a saved inventory rebuilt from the admitted scan closure inside ingress. The
  view identity is the input, the effective scope, the receipt closure, and the admitted projection or
  scan-inventory identity together with its distinguished entry. So a saved view and a selected view with different
  closures never share an answer, and neither do two projections that share one receipt closure but name different
  entry schemas (P1d picks the directly named blob as the entry and treats the other as a reference). The facts
  record the claim (or the scan), its receipt closure and its entry.
- `schema::markers` builds `ComparableOperation` only when the operation's `OperationFacts` show every root and the
  status set readable and every root's `Marker` answer is proven. The `SavedStatus` policy's current or stale outcome
  needs the `ComparablePair` of its own key, which one constructor builds from the saved and the selected view of that
  key. The type forces the pair; that the constructor took the right two views is a runtime rule with a test.
- The answers are defined to agree with the diff's predicates (`diff/fields/evidence.rs`, `diff/status/pairing.rs`,
  `diff/operations.rs`). A parity test checks every marker state against the diff.

What each consumer does with an unproved marker. The decisive case is an empty field projection with unproved media.

| Consumer | Today | After |
|---|---|---|
| Diff (`diff/*`) | Scoped `media_unverified` limit, element unknown. | Unchanged; it is the reference, pinned to the marker answers by the parity test. Moving it onto the answers is a later, optional cleanup. |
| Saved status, field branch (`ElementFacts::from_index`) | Reads the flag. Misses partial coverage without a JSON source, and operation gaps outside a matching gap. | The `SavedStatus` policy reads `Marker` facts. Unproved gives unknown, so the outcome is `membership_unchecked` or `shape_incomparable`. |
| Saved status, operation branch (`operation_shape_covered`) | Ignores the flag, the status inventory and the fallback message. | No `ComparablePair` exists when any root of either view is unproved, a status set is withheld or a fallback caveat applies. The outcome is `shape_incomparable`, never `binding_current`. |
| Bindings serialization (`BindingsDocument::with_schema_inventory`) | Writes coverage `checked`, strips `media_unverified`, and derives `body_presence` from `feasible.is_some()`. | Wire bytes unchanged, because the replay builds bindings documents. The serialized `coverage` and `body_presence` are not certification evidence (section 8). |
| Membership (`membership()`, `covers()`) | Reads only the bindings rows. They carry neither the flag, nor the status inventory, nor the dialect exclusion, nor the gaps. | `Unobserved` needs the `CompleteBasis` of its own key, holding one `MembershipProof` per basis input. Unproved, withheld, excluded or gap gives `membership_possible` and no negative. |

**Saved inventory provenance.** The saved inventory's facts come only from an index rebuilt from the admitted closure
inside ingress: the exact receipt closure of the admitted saved scan in the same request (entry, references,
configuration and media). A caller-supplied index is compared with that rebuild and never answers itself. A genuine
admitted index from another request with a different closure does not pass.

**Saved negatives.** `no_saved_match` and `element_absent_from_scan_schema` need `SegmentPopulation{scan}`: the
segment population and scope of the scan, independently evidenced, and the match records derived from it. Without
that fact the negative is withheld (`not_checked`, reason `membership_unchecked`). The scan's own segment lists are
not evidence of their completeness. P4 supplies the fact by reconstructing the segment manifest from its own scan of
the admitted client tree that the saved scan read (the saved scan's tree, not the current one); this is a caller
contract (sections 3.9 and 8.1).

**Known totals.** `KnownTotal(name)` takes `PopulationProof`s for `CountedInputs(metric)` and
`RequiredComparisons(metric)`, reads the settled comparisons that supply the counts, and reads
`ExtractionCoverage{input}` (expected files, uncovered regions, target-check completeness) for each counted input. A
total is known only when:

- every counted input is admitted;
- every required comparison is settled admitted;
- every counted input's extraction coverage is complete;
- no waiver controls the required population. A `no_first_parent` slot is such a waiver. A total that is explicitly
  After-only has no Before requirement.

Otherwise the total is null with a reason.

### 3.9 The public DTO boundary and caller-built inputs

- **Request and document DTOs** stay public-field serde types, decoded and serialized as today. Their role becomes
  ingress and output only. Inside the crate only the ingress modules and the output builders dereference them.
- **Certified entry points** are the paths this design certifies:
  - `admit_inputs`, `admit_schema_input` and `saved_binding_status`;
  - `ImpactDocument::validate`, which certifies local invariants and reference integrity only (section 1.5);
  - a new `unobserved_elements_admitted(request, evidence)`.

  The existing `unobserved_elements` stays byte-compatible for current callers as an uncertified computation over
  bindings alone (owner question 2).
- **Frozen cells in P4.** P4 certifies a frozen cell only for a document it produced from admission in the same run.
  Saved results, population completeness and source authenticity come from P4's admitted computation, not from
  `validate` on its own. The scan-contents and absence-usage known limits stay disclosed for foreign documents.
- **`index_schema` over a caller-built `AdmittedSchemaInput`** stays public. Its `SchemaIndex` is not a view rebuilt
  inside ingress, so no `Marker` fact can be settled from it on a certified path. `SchemaIndex::default()` likewise has
  none.
- **Caller `ProducerInventory`** (diff) stays a documented caller contract. P4 builds it from admitted sources.
- **Caller segment manifest (`SegmentPopulation`)** is a documented caller contract. P4 reconstructs each saved
  scan's segment manifest from its own scan of the saved scan's admitted client tree, not the current tree. The
  redesign cannot check a P4-built manifest; it only binds it to that scan and tree. Entry change (S7):
  `SavedScanEvidence` gains one public field, an optional map from scan id to the caller's segment manifest, which
  ingress settles into `SegmentPopulation{scan}`; `saved_binding_status` keeps its two arguments. A caller that
  supplies no manifest gets today's outputs except that every saved negative is withheld (`not_checked`).
- **`AdmissionReport` and `ClaimStatus`** are output. No in-crate code takes them as input, and P4 must not.

## 4. Settlement paths

"Reads today" is the census's route:

- (a) a settled claim or derived edge;
- (b) a handwritten `RequiredDependency`;
- (c) a raw record or field;
- (d) no edge.

Steps are defined in section 6.

| Path | Function (file) | Reads today | Afterwards | Step |
|---|---|---|---|---|
| P1 | `ClaimDecision::from_reasons` (`revision/admission/gate.rs`) | dependencies handed in | moves to `settle/gate.rs`; dependencies come from the recorder; the legacy bridge serves old callers until S6 | S2, S6 |
| P2 | `AdmissionReport::settle` (`revision/admission.rs`) | string keys from the caller (b) | replaced by the dispatcher; legacy string lookups over the typed store until migrated | S2-S4b |
| P3 | `AdmissionReport::insert` (`revision/admission.rs`) | overwrites and adjusts counts | store insert plus `Store::publish`, with counts at publication | S1 |
| P4 | `Ledger::selection_claim` (`revision/admission/graph.rs`) | `InputFacts.selection` (a) | `Selection(id)` and `SubjectSelection`, each with a `SelectionOutcome` grant | S5a |
| P5 | `Ledger::claim` (`revision/admission/graph.rs`) | slot record (c), own selection (a), subject edge in historical reviews only (b) | claim policy reads `Slot`, `Input`, `Selection`, the `Receipt` closure and `Cutoff`; the subject edge comes through `Cutoff` | S6 |
| P6 | `Ledger::resolve` (`revision/admission/graph.rs`) | inserts decisions; media keys with a `:` suffix | dispatcher over the claim-id population plus adapter publication | S1, S6 |
| P7 | `Ledger::resolve_schema` media loop (`revision/admission/schema_claim.rs`) | projection without the claim's edges (d) | `SchemaBasis`, `MediaProof` and the published `Media` (section 3.6) | S6 |
| P8 | `selection::selection`, `selection_required` (`revision/admission/selection.rs`) | raw subject role, repository and snapshot (c) | selection policy returning `PinProof`, `ExemptionGrant` or `SelfExemption`; rules in `settle/rules.rs` | S5a |
| P9 | `Ledger::new`, `Ledger::validate_receipt` (`revision/admission/graph.rs`) | origin `InputFacts.selection` (a'); conflicts over checked candidates | ingress `Record` and `Input` (identity view for kinds up to `Merge`; `input(id)` records `Merge` for later kinds); `PinObservation` and per-branch `SelectionConflicts`; named-tree `Receipt(id)` reads `Selection(origin)`; `CutoffEvidence(id)` reads `Cutoff` | S5a |
| P10 | `selection::merge` (`revision/admission/selection.rs`) | merge against `expected_merge` (c) | `Merge(id)` with today's checks, reading `Input(id)` (identity view), `SubjectHead`, `Cutoff` and `CutoffEvidence` (S5a); the relation rules (S5b) | S5a, S5b |
| P11 | `Ledger::claim` optional media (`revision/admission/graph.rs`) | admitted attachments only (a) | allow-listed optional reads; attached cycles still refuse | S6 |
| P12 | `admit_schema_input` (`revision/admission/projection.rs`) | report decision and projection (a) | reads `Settled<Claim>` and `Settled<SchemaBasis>` | S6 |
| P13 | `saved_binding_status`, `evaluate` (`revision/staleness.rs`) | admission decisions, raw input role (c), caller saved index | `SavedStatus{query}` reads `Claim`, `Marker`, `OperationFacts` and `SegmentPopulation`; current or stale needs the `ComparablePair` of its key; the saved index is rebuilt from the scan closure inside ingress | S7 |
| P14 | `segments::check` (`revision/staleness/segments.rs`) | scan segment scope from the scan (c) | segment receipts are read inside `SavedStatus`; a saved negative needs `SegmentPopulation`, otherwise it is withheld | S7 |
| P15 | `settle_inputs` (`model/impact/validation.rs`) | `input:{dep}` (b) | `Input(id)` reads its declared dependencies; the active-read rule keeps today's cycle output | S2 |
| P16 | `slot_cutoff` (`model/impact/input_validation.rs`) | subject, holders from a deduplicated map, raw waived inputs (b, c) | `ReportCutoff` reads `DocumentClock`, `Slots`, `Input`, `Population(MergeHolders)` and `Population(SelectionReceipts)` with `Classified` per record; `LinkWaiver` | S3a, S3b |
| P17 | `validate_slots` (`model/impact/input_validation.rs`) | `input:` edges (b); raw `no_first_parent` reason (c) | `Slots(role)` and `Reference`; `no_first_parent` stays a disclosed limit | S3a |
| P18 | `validate_status` (`model/impact/input_validation.rs`) | raw producer slot ids (c); no cutoff | `BindingStatus(i)` reads `Slots(Producer)`, `Input`, `ReportCutoff`, `Population(ScanClients)` and `DocumentSchema` | S4a |
| P19 | comparisons in `validate` (`model/impact/validation.rs`) | cutoff and both inputs (b); `pair_for` over raw slots (c) | reads `ReportCutoff`, `Input` and `Slots`; no `DocumentSchema` edge, so an invalid document still settles comparisons independently (section 3.6) | S4b |
| P20 | usages in `validate` (`model/impact/validation.rs`) | comparison keys (b); raw presences and slots (c) | reads `Comparison`, `Slots`, `Input` and `DocumentSchema`; match-outcome and site-id rules | S4b |
| P21 | review action in `validate` (`model/impact/validation.rs`) | comparison key (b); row ids | `ReviewAction` reads `Comparison`; rows remain integrity links | S4b |
| P22 | `validate_auxiliary` (`model/impact/validation.rs`) | raw consumer ids (c); `input:` (b) | `CallChange` reads `Slots(Consumer)`, `Input` and `ReportCutoff`; `Membership` reads `Input`, `ReportCutoff` and `DocumentSchema`; `Risk` reads rows | S4a |
| P23 | refusal-only keys (`model/impact/validation.rs`) | none | `DocumentSchema` always settled and published only when refused; the refusal-only keys check their own record as today | S2, S4a |
| P24 | `with_schema_inventory` provenance `"proven"` (`model/bindings.rs`) | caller index labels (c) | wire unchanged; certification needs a `Marker`-backed admitted index | S8 |
| P25 | `unobserved_elements` (`model/impact/membership.rs`) | parsed evidence; bindings matched by id (c) | `Unobserved{query, region}` emitter with the `CompleteBasis` of its key (one `MembershipProof` per basis input), built in `membership/proof.rs`; population and identity rules | S8 |
| P26 | `with_usage_counts` (`model/impact/membership.rs`) | raw comparisons, the first client record (c) | `KnownTotal` with population proofs, settled comparisons and `ExtractionCoverage` | S8 |
| P27 | `with_schema_diff`, `with_coverage_limits` (`model/impact/projection.rs`) | diff over caller indexes | unchanged; rows are not certified (documented) | none |
| P28 | `omission_proof`, `valid_media_proof` (`schema/messages/media.rs`) | catalog flag `cutoff_admitted` (c) | proof cutoff comes from the receipt's origin and `Cutoff`; the flag is ignored | S6 |
| P29 | `diff_boundary`, `basis::admit`, `producer::compare`, status membership (`diff*.rs`) | caller inventories | unchanged; documented caller contract | none |
| P30 | `project` waiver (`schema/messages.rs`) | sets `Coverage::Checked` for an unproved empty projection | public fields unchanged; the marker answer records unproved | S7 |
| P31 | `ElementFacts::from_index`, `operation_shape_covered` (`revision/staleness/inventory.rs`) | raw markers, two of them ignored (c) | replaced inside `SavedStatus` by `Marker`, `OperationFacts` and the sealed `ComparablePair` | S7 |
| P32 | `validate_input`, `compare_slot`, `project_scope` (`revision/admission/slots.rs`) | slot records (c) | ingress `Input(id)` and `Slot(id)` | S5a |
| P33 | `message_roots` serialization (`model/bindings.rs`) | raw coverage, limits, `feasible` | wire bytes unchanged; not certification evidence | S8 (documented) |
| P34 | `membership()`, `covers()` (`model/impact/membership.rs`) | serialized bindings rows (c) | `MembershipProof`s inside the `CompleteBasis` from `membership/proof.rs`, consumed by `Unobserved` | S8 |
| P35 | `input_refusals` (`model/impact/input_validation.rs`) | raw `doc.cutoff_at` (c) | part of `Input(id)`; reads `DocumentClock`; origin identity parity | S2 |
| P36 | media key placement in `Ledger::resolve` (`revision/admission/graph.rs`) | `contains_key` collision loop | typed `Media` key; generated records own canonical keys; refusal only on an actual collision with a generated key | S1 |
| P37 | `Documents::new`, `document_for` (`schema/references.rs`) | admitted reference bytes | unchanged; `SchemaBasis` rules what may enter references (named-tree references from the claim input only) | S6 |
| P38 | `media::openapi`, `media::swagger` (`schema/messages/media.rs`) | declared media in the entry bytes | input to the marker answers; semantics unchanged | S7 |

P27 and P29 are accounted for and deliberately unchanged. They produce rows and diffs that the frozen validator does
not certify, or they consume documented caller inputs.

## 5. Coverage statement

Counts over the census's 89 concrete-state cases. A per-case receipt is kept with the lane evidence, and these totals
equal its tally.

| Class | Cases | Serious | Uncertain | Minor |
|---|---:|---:|---:|---:|
| Compile error: recording reader | 12 | 12 | 0 | 0 |
| Compile error: waiver grants | 5 | 5 | 0 | 0 |
| **Compile error, total** | **17** | **17** | 0 | 0 |
| Runtime rule plus RED test | 39 | 31 | 4 | 4 |
| Disclosed limit | 29 | 1 | 5 | 23 |
| Not a defect or duplicate | 4 | 0 | 0 | 4 |
| **Total** | **89** | **49** | **9** | **31** |

Revision 3 counted 29 compile errors (14 reader and population proofs, 7 grants, 8 sealed proofs) and 27 runtime
rules. Round 3 showed that a sealed proof does not force a decision to obtain the right proof: an outcome that accepts
two values of one proof type can take both from the same view, and an emitter can supply only the readable input's
proof. The fix (section 3.3) puts the pair and the basis in their own types, built by constructors bound to the
decision key; which views and which population those constructors take is a runtime check. Every case was then
re-decided by one question: if the deciding read is left out, does the policy still compile?

- **What "compile error" means here.** A case stays a compile error only when (a) the decision needs the deciding
  value itself to compute any admitted outcome, (b) the only route to that value is one accessor or grant
  constructor whose recorded key is fixed (a singleton key such as `DocumentClock`, `Cutoff` or `SubjectSelection`,
  or a key the record itself names), and (c) no other value of the same type, obtainable without the deciding read,
  satisfies the outcome. The compiler enforces the route through items 1-5 of section 3.4 and the CI check through
  items 6-9. Every `ExemptionGrant` constructor reads `SubjectSelection` and `Cutoff` or `Clock` unconditionally, so a
  grant for any input records them; whether the right grant applies is a relation rule (section 3.7). Where
  enforcement needs the decision to read a gate whose value it does not otherwise use, or to pick the right source
  among proofs of one type (a view, an input, a population, a metric), the case is a runtime rule with a test.
- **The 12 cases that moved from compile error to runtime rule in revision 4.**
  - Saved status, 4: the Swagger operation with an empty object response, the three response-set triggers, the
    duplicated path item and the Swagger body parameter. Their answers come only from marker facts, but that the
    marker answer is right is the parity test, and that the pair or the field marker comes from the right view is the
    `ComparablePair` constructor's view check.
  - Membership, 4: the waived root, the withheld status set, the dialect-excluded request and the
    unreadable-operation gap. That the basis is complete is the `CompleteBasis` constructor's population check. They
    hold on the certified membership entry only.
  - Known total, 1: a total over an input with an unknown dependency. The `CountedInputs` member recording records
    the input, but a population proof for another metric has the same type; placed in the runtime column
    conservatively.
  - Receipt, 1: a named-tree receipt's read of its origin's selection. The receipt uses no value from that selection
    (it compares blob identity through `input(origin)`), so omitting the gate read compiles. Decided from the text.
  - Media, 2: the historical media request with a wrong topic pin and the live media request with a wrong subject pin.
    Both chains pass through `SchemaBasis` reading the input's selection, and the text does not show that
    `SchemaBasis` needs that value. Placed in the runtime column conservatively. The media case whose closure holds an
    independent-cutoff receipt stays a compile error: each step of its chain needs the value it reads.

  The seams of section 7.2 still test every recording point, including those behind the moved cases.
- **Disclosed limits by incidence in the 37-pair replay.** Observed: 0. Constructed, with replay incidence 0: 26. Not
  measured: 3, marker states whose incidence depends on schema contents. No disclosed limit was observed.
- **The 29 disclosed limits.**
  - 15 unread frozen fields and identity relations that no claim relies on.
  - 9 documented known limits and design decisions: scan contents, caller bindings provenance, uncertified rows,
    absence usages, record-internal path consistency, computed-tree time, the copyable public status, the
    caller-built schema input, and the caller producer inventory.
  - 4 further cases: a Held alias read differently by receipt readers, the fallback caveat not carried by saved or
    membership presence, the bindings `body_presence` misread, and `content_hash` excluding coverage.
  - 1 serious case: an unevidenced `no_first_parent` slot waiver. P4 does not certify such a slot, and no known total
    may rest on it.
- **The 4 minor runtime rules.** Each was a documented limit that review showed is cheap to close:
  - saved inventory provenance bound to the scan's closure;
  - a saved negative that needs an independent segment population;
  - the `ReportCutoff` read for binding status, call changes and membership.
- **The 4 that are not defects.**
  - a duplicate;
  - a case with a mechanism-level proof that the admitted decision rests on no unchecked value (owner to confirm);
  - a missing consumer slot, which is a valid shape;
  - a conservative direction, in which no negative is emitted.
- **The 42 census cases without a concrete state** are unchanged. 26 already pass through the gate, and 16 have a
  mechanism-level none-found or design reason. Four carry later corrections, which are counted under the cases they
  point to.
- **Unverified.**
  - Every state is traced, not executed. Each class becomes firm when its RED test fails on the pre-change code.
  - Six cases rest on an owner decision or an unverified assumption: owner questions 3, 4, 7 and 8, the Held alias,
    and `content_hash`.
  - The incidence of the 12 marker-dependent cases in the replay is not measured. By construction their fixes touch
    no replay output: the replay has no saved status or membership, and the bindings wire does not change. Step S7
    measures it to confirm.
- **Against the agreed estimate (section 2.3).**
  - Kept: the derived-marker type as a second type. All of the about a dozen marker and builder cases become runtime
    rules with a test; the proof types make the right proof obligatory, not the right source. The 15 minor
    unread-field cases stay disclosed.
  - Corrected: 17 cases become compile errors, not about 49. That is 17 of the 37 serious cases the estimate placed
    at compile time, and none from the marker and builder set. The other 19 of the 37 are runtime rules: their
    mechanism is a population, a relation, an unconsulted value, a gate read or a self-asserted proof, which a
    recorded read does not prevent.

## 6. Migration

### 6.0 Phases

The twelve steps fall into two phases, set by a read-only census of real pilot history (2026-11-28) that sorted the
105 uncertified census rows by whether the request shapes they need occur in the history of the public pair and the
private pair. Counts are aggregates; the census reads git history and the settlement census's own feature columns,
with about 15 judgement calls (about plus or minus 3). This section states what each phase does; it does not
recommend either phase or any product choice.

**The census buckets.** Of the 105 uncertified rows:

- 53 are reachable by observed shapes only (every feature they need occurs in history, and historical mode is
  possible): 19 serious;
- 46 are undetermined (they need a feature or mode history cannot show: 34 a product choice only, 7 a product choice
  and a live or version mode, 5 a mode only): 29 serious;
- 6 are reachable only by unobserved shapes (a held alias, a `no_first_parent` slot, caller-built inputs): 1 serious.

"Observed" means the request shape occurs in history. No bad state was observed: every one is constructed, traced
from code and not executed, with incidence 0 in the 37-pair replay. 12 schema-marker rows depend on schema bytes, not
on the request shape, and frozen-layer rows are also reachable through a hand-edited document, which no request
shape governs.

**Phase 1: S1, S2, S3a, S3b, S4a, S4b, S5a.** Line estimates from the step table: 550 + 900 + 600 + 400 + 550 + 450 +
900 = 4,350 changed weighted lines, equal to the census's figure. It is a complete, mergeable unit under the rules of
section 6.1: each step builds in every permitted order, keeps the 37-pair replay byte-identical (section 7.4), passes
the full crate suite, and lands its RED tests, controls and seams. The legacy bridge stays in place, because S6
deletes it.

- **What is certified after phase 1.** In the README's terms (modes live, historical and version; request shapes;
  certified, accepted but not certified, withheld): the certified shape is unchanged. Admission stays certified only
  for the historical review the replay exercises (subject head the service After at the cutoff, two commit inputs of
  one API with one dependency-free named-tree blob receipt each, two schema claims, no consumer side, computed tree,
  merge, held alias, pin, media, configuration or reference receipt, and no frozen consumer record).
  `ImpactDocument::validate` stays certified in no shape. Every other mode and shape stays accepted but not
  certified. Phase 1 removes census cases from those shapes; it does not certify them, because section 8.1 item 1
  requires S1-S6.
- **Rows it closes, by the design's per-case step (section 7.1).** Observed-only: 17 of 53 (14 of 19 serious).
  Undetermined: 15 of 46 (15 of 29 serious). Unobserved-only: 0 of 6 (no step fixes any of them: 5 are disclosed
  limits, 1 serious pinned in S9, and 1 is not a defect).
- **Observed serious rows left open: 5.** Four membership and total rows (S8), and one relation rule: a historical
  After turned into an unrelated later commit in the subject's repository (S5b). The census, which mapped rows to
  steps through the paths of section 4, counted that fifth row as covered, because its paths also list S5a; its RED
  test and fix are in S5b.
- **The 36 observed-only rows phase 1 leaves open** split into 8 that phase 2 fixes (5 serious: S5b 2, S7 2, S8 4,
  counted by row), 20 disclosed limits, and 8 with no defect or no concrete state. All 36 are constructed, none
  observed as a bad state.
- **What P4 may certify after phase 1 alone.** The README's certified shape, as today, and nothing more. Against
  section 8.1: item 1 is not met (S6 is missing: the claim policy, `SchemaBasis`, media and the bridge deletion);
  item 2 is not met (S7, S8); item 3 is not met for membership (`unobserved_elements_admitted` lands in S8) and holds
  for the other entry points only as code, not as certification; items 4 and 5 are P4's own gates. P4 may not certify
  any other admitted cell, any frozen cell, a saved-status cell, a membership negative or a known total. Whether a
  phase-1-only P4 could certify more would change 8.1 item 1; this design does not make that change.

**Phase 2: S5b, S6, S7, S8, S9.** 350 + 800 + 800 + 750 + 300 = 3,000 changed weighted lines. Counted by row (section
7.1 step):

- **S5b** (relation rules, the historical topic-head binding in the conflict group, the parity matrix): observed-only
  2 rows (1 serious, the historical After relation; 1 uncertain), undetermined 2 (1 serious). Depends on S5a.
- **S6** (claim policy, `SchemaBasis`, media, reference identity, bridge deletion): observed-only 0, undetermined 6
  (5 serious; all need media or a selection pin, a product choice). It is also 8.1 item 1's last step for any
  admitted or frozen cell. Depends on S5b, S4a, S4b.
- **S7** (marker facts, saved status): observed-only 2 (0 serious: saved inventory provenance and the saved negative's
  segment population), undetermined 4 (4 serious, schema-marker states). Depends on S6.
- **S8** (membership, known totals): observed-only 4 (the four membership and total rows, all serious), undetermined 4
  (4 serious). Through its dependencies (S7, S4a, S4b) it pulls in S1 through S7.
- **S9** (audit closure, five known-limit pins, status update; about 300 lines) serves no row. It is part of P4's
  final gate.

The census's path-based mapping gives slightly different per-step counts (observed-only S5b 4 rows, S6 1, none
serious); the per-case step above is the design's.

**Product choices phase 2 depends on.** The undetermined rows it fixes need media or metadata claims, selection pins,
independent-cutoff receipts, saved scans, schema-marker states, or a live or version mode, none of which history can
show. Owner questions 12 and 13 (section 9) decide which modes and which change shapes the pilot reviews.

**The private pair.** Its producer has no schema file; its route table is its schema source. The admission module
has no route-table path, so the private pair reaches admission only after a route-table input is admitted, which is
not traced. Phase 1 therefore certifies nothing for the private pair, and neither does phase 2: its first reachable
point is after a separate route-table admission slice that this design does not specify (an open item, section 8.2).

**What phase 1 leaves open, and what is only constructed.**

- 5 observed serious rows (four membership and total rows in S8, one relation rule in S5b) and 14 undetermined
  serious rows (S5b 1, S6 5, S7 4, S8 4).
- Every saved-status and membership cell, every known total, every media decision, and the claim policy over the
  shared rules (S6).
- The interim divergence between the layers on the relation rules S3b changes, until S5b (section 3.7).
- The legacy bridge, until S6.
- All of it is constructed: no census state was observed in the replay or in history; the census shows only that
  the shapes occur.

### 6.1 Rules and steps

Rules that hold for every step:

- The 37-pair replay stays byte-identical (section 7.4).
- The crate's full test suite runs.
- Every file stays under 1,000 weighted lines (aim 600; `python3 scripts/file_size.py`).
- The review cap is two rounds. At the cap, the round is classified under the lane's convergence rule: converging
  findings are folded and the extension is disclosed; open-class findings go back to this design.
- **Every permitted order builds.** The "Depends on" column is complete: any order that respects it builds, and the
  table order is one such order.
- **S1 needs no bridge.** The store stays `AdmissionReport`, keyed by rendered strings. S1 replaces the hand-written
  spellings with `ClaimKey` rendering and centralizes the publication and alias rules.
- **The legacy bridge.** From S2, `settle::legacy` keeps `from_reasons` and string lookups over the typed store
  available to a closed list of legacy callers that the import check holds. Each step removes the entries it
  migrates. S6 deletes the bridge, and it depends on every other step that removes a caller.
- **Merge facts.** `Merge(id)` and `CutoffEvidence(id)` land in S5a with today's merge checks moved unchanged, so the
  computed-tree grants and `record_merge` (behind `input(id)`, `Slot(id)` and the grants) have their source from S5a
  on; `identity(id)` lands with them. S5b adds only the relation rules and the historical topic-head binding in the
  conflict group.
- Every behaviour change carries its own RED test in its own step.
- **Known-limit rows.** A row that pins a fixed case changes in the step that fixes it: rows 14 and 15 in S3a, row 12
  in S5a, row 13 in S6, and the saved-inventory row in S7.
- An existing expectation that encodes a census case changes only in the step that fixes that case, and the PR lists
  it.

There is no separate move-only step. No file in the crate is over 650 weighted lines today, and none passes 1,000
under this plan. Each step creates the section 3.1 files it needs and moves only the functions it migrates, so no
code moves twice.

| Step | Goal | Files touched / new | Functions | Weighted lines (changed) | Tests | Depends on |
|---|---|---|---|---:|---|---|
| S1 | Typed keys and rendering, publication conditions, refusal-id rendering, media collision and alias rules (no store change) | 9 / 2 (`settle.rs`, `settle/key.rs`) | about 20 | about 550 | 2 RED, 1 preservation, key table, byte controls | - |
| S2 | Evaluator core: arena store, ingress source handle, dispatcher table, recorder, gate move, legacy bridge, optional reads, direct closure reads, active-read rule, kind table. Frozen `DocumentSchema`, `DocumentClock`, `Record`, `Input`, `Classified`. The import check | 5 / 8 | about 15 | about 900 | 2 RED, 2 preservation, 1 seam, 8 core | S1 |
| S3a | Frozen cutoff and slots through the reader; `LinkWaiver`; `Population(MergeHolders)`; `settle/rules.rs` (frozen half) | 3 / 3 | about 9 | about 600 | 6 RED, 2 seams | S2 |
| S3b | Frozen relation and parity rules: conflict population, live computed After, inferred anchor, computed Before, merge base and target, waived-Before date | 2 / 0 | about 6 | about 400 | 6 RED | S3a |
| S4a | Binding status, call changes, membership, risk and refusal-only keys; the `ReportCutoff` reads; `DocumentSchema` reads for binding-status and membership records | 3 / 3 | about 9 | about 550 | 7 RED, 1 seam | S3a |
| S4b | Comparisons, usages, review action; match-outcome and site-id rules; `DocumentSchema` read for usage records | 2 / 1 | about 6 | about 450 | 4 RED | S3a |
| S5a | Admission ingress; `Record`, `Input` (`identity(id)` up to `Merge`; `record_merge` for later kinds), `SubjectHead`, `Clock`, `PinObservation`, `SelectionConflicts(repo, branch)`, `SubjectSelection`, `Cutoff`, `CutoffEvidence`, `Merge` (today's checks), `Selection`, `Receipt`, `Slot`; the grants; admission adopts `settle/rules.rs` | 6 / 5 | about 18 | about 900 | 8 RED, 5 preservation, 3 seams | S3b |
| S5b | Merge relation rules; the admission conflict population; the After grant; the layer parity matrix | 2 / 1 | about 5 | about 350 | 4 RED, parity matrix | S5a |
| S6 | Claim policy, `SchemaBasis`, `MediaProof`, published `Media`; proof cutoff from the origin; reference identity; the legacy bridge deleted | 7 / 2 | about 13 | about 800 | 6 RED, 1 seam | S5b, S4a, S4b |
| S7 | Saved-status ingress, marker answers, `Marker{view, root}`, `OperationFacts`, `ComparableOperation`, `ComparablePair`, `SegmentPopulation` (the `SavedScanEvidence` field); diff parity test; `SavedStatus`; saved index rebuilt from the scan closure; marker incidence measured on the 37 pairs before merge | 7 / 3 | about 13 | about 800 | 6 RED, 2 seams | S6 |
| S8 | Membership ingress, `membership/proof.rs` with the three `MembershipProof` variants and `CompleteBasis`, the `Unobserved` emitter; population, completeness and identity rules; `ExtractionCoverage`; `KnownTotal` with population proofs | 4 / 3 | about 11 | about 750 | 9 RED, 1 preservation, 1 seam | S7, S4a, S4b |
| S9 | Close the audit (every policy file under the import check); 5 known-limit pins for the remaining serious and uncertain disclosed cases; README and DESIGN status update | 4 / 1 | 0 product | about 300 | 5 pins | S8 |

Total: 12 PRs and about 7,350 changed weighted lines.

**Before P4.** Every step may merge before P4. P4 may implement against any of them, but it must not certify:

- any admitted or frozen cell before S1-S6 have merged;
- a saved-status cell before S7;
- a membership negative or a known total before S8.

S9 is part of P4's final gate. In phase terms (section 6.0): after phase 1 alone P4 certifies only the README's
existing shape; the first of these three lines is met only inside phase 2, at S6.

**Exit gate per step.**

1. The step's RED tests fail on its parent and pass after.
2. From S2 on, each RED test also asserts the decision's recorded keys. S1 asserts outputs only, because the recorder
   does not exist yet. S8's REDs run through a common fixture runner that calls today's computation on the parent and
   the admitted entry on S8, so the RED is behavioural rather than a missing-API error.
3. The controls and the preservation tests are unchanged.
4. The seams behave as section 7.2 states.
5. The full crate suite passes.
6. The byte gate of section 7.4 passes: the replay harness, plus the refusal byte controls.
7. The import check passes.
8. File sizes are within the rule.

A failure outside the step's scope is reported, not re-baselined.

## 7. Test-first plan

Every RED test is written first and fails on the pre-change code. From S2 on, each RED also asserts the decision's
recorded keys; S1 asserts outputs only. Each RED has a control at the same entry point, with the same evidence
envelope, that must stay admitted or unchanged. A preservation test passes on the parent and must still pass after:
it guards behaviour the redesign could lose. Counts are per step. The private appendix maps each case to its test.

### 7.1 RED tests by shape

| Step | RED tests (shape) | Controls and preservation tests |
|---|---|---|
| S1 | 2: a user claim equal to a generated media key while the real media proof is refused; generated media for receipts `r` and `r:` of one claim beside a user claim `media:1:c:r`, where both generated records must hold their canonical keys, the user refusal takes the first free alias, and the counts are exact | the media fixture without a collision, identical bytes; preservation: a media-shaped user id that collides with nothing (`media:2024:x`) stays admitted |
| S2 | 2: a frozen document with report time before cutoff, asserting the decision of an input with an independent-cutoff origin (the cutoff already refuses today); an independent-cutoff origin with no covered tree | report time equal to cutoff; an origin covering the input tree; preservation (2): the wrong-digest variant of row A12 keeps today's reasons and counts; `fold2_direct_dto_still_needs_wire_closure` stays byte-identical |
| S3a | 6: a Before input with an unknown dependency, in a historical computed document and in a live joint document; a second record with the Before input id; a consumer slot naming a producer input; a computed holder copied under a fresh id and followed by a commit with that id (both orders); a duplicated record whose copies disagree on the merge-receipt field | the same documents without the defect; a single holder; a non-holder with a different merge receipt and an unrelated refusal, which stays independent |
| S3b | 6: in a historical document, topic head and pinned Before on one branch with different revisions; a live computed After without a merge receipt; a live client-subject document without a subject head; a computed Before without merge evidence; a computed After with unrelated proposed base and target; a frozen waived Before committed after the cutoff | agreeing revisions; a commit After; a recorded subject head; a commit Before; consistent base and target; a Before committed before the cutoff |
| S4a | 7: a client-subject document with producer before and after different, plus a binding status; a service-subject document with consumer before and after different, plus a call change; a typed binding status outside the enum; a typed membership with an empty basis; a scan client outside the slots with no receipt; a binding status under a refused report cutoff; a call change and a membership under a refused report cutoff | consistent slots; a valid status; a basis of one admitted input; a pinned scan client; a valid cutoff |
| S4b | 4: a usage with null match references over refused consumer slots; a typed usage with unknown presence and no reason; a usage site id absent from its match references; a usage naming an unchecked or unbound match | consistent slots; a valid usage; matching site ids; a checked, bound match; an otherwise admitted typed document with only `schema_version` changed: `invalid_document`, and its comparison settles independently with today's bytes |
| S5a | 8: a live subject whose pin has the wrong revision, read by a claim on an exempt input; the same read through a receipt; the same read by saved status for a consumer subject; a historical subject committed off the cutoff, with a metadata claim; a live report time different from the cutoff; the same with a scan; a historical report time before the cutoff; a waived same-repository Before committed after the cutoff | the replay shape (same-repository producer Before commit, no pin, committed before the cutoff); live siblings with their own pins; a subject committed at the cutoff; report time equal to cutoff; claim `effective` on the computed `service-after` (`tests/support/fold3.rs:95-150`, `tests/support/fold4.rs:85-90`) and a live computed subject head, both admitted (the identity view keeps `Merge` from re-entering itself); the admission half of `fold3_frozen_receipt_checks_expected_branch` (`tests/support/fold3.rs:206`), admitted (the group is today's observed pins); a pinned input duplicated with copies that disagree on branch beside a valid pinned sibling on another branch, where the duplicate is refused as today and the sibling stays admitted. Preservation (5): a pinned Held computed tree with a wrong `result_tree`, refused; a version-comparison computed tree with a wrong merge record, refused; `sideless_computed_client_checks_merge_head` (`tests/support/fold4.rs`); a live request with a valid subject pin and one extra consumer whose pin has the wrong `observed_at`, where only that consumer is refused; the replay shape plus a duplicated unrelated consumer input, where every other decision stays admitted |
| S5b | 4: in a historical review, a Held pin disagreeing with the unpinned topic head on one branch; a live merge head different from the subject revision; merge base and target unrelated to the Before revision; a historical After turned into an unrelated later commit in the subject's repository | agreeing pins; in a historical review, a pin agreeing with the topic head beside a branch-carrying waived Before, admitted; `tests/support/fold3.rs:206` still admitted; a merge head equal to the subject; a consistent base; a computed merge-result After |
| S6 | 6: a historical media request whose topic pin is wrong; the same with an independent-cutoff source in the media closure; a live media request whose subject pin is wrong; a refused required media alongside a valid one; a catalog flag claiming cutoff admission for a receipt not admitted at the cutoff; a named-tree reference from another input | valid topic and subject pins; all media valid; a named-tree media receipt of an admitted snapshot; a reference from the claim input |
| S7 | 6: a Swagger operation with an empty object response, no media declaration and no media receipt, and a bound saved location with an empty signature; a default-only response and an empty `responses` beside an unrelated gap, at the saved entry (a repeated `responses` member is refused at receipt ingress, so it is a marker-unit test); a duplicated path item beside an unrelated gap; a required Swagger body parameter without media evidence and an unknown field id; a saved inventory index admitted in another request whose closure holds a reference that this scan's closure lacks; a scan whose matching segment was removed from its own segment lists, with no independent segment population (the negative is withheld) | the same operations with proven JSON media or readable responses; an index rebuilt from this scan's closure; a saved negative with an independent segment population; a selected and a saved view of one input with different closures, each keeping its own answer; two projections sharing one receipt closure with different entry schemas, each keeping its own view and answer |
| S8 | 9: a waived root with a change naming a field under it; a field under a withheld status set; a request field of a dialect-excluded request; an operation behind an unreadable-operation gap; a known total over an input with an unknown dependency; evidence marked incomplete; bindings naming another tree for the same input id; two evidence records for one input, one with an unknown member; a joint known total whose Before slot is waived by `no_first_parent` | proven roots, readable status sets, eligible requests, readable operations, admitted inputs, complete evidence, matching identity, identical records, an explicitly After-only total with complete extraction; certified negatives over a complete consumer basis and a route-table producer basis. Preservation: an After-only total with valid inputs and comparison but incomplete extraction stays unknown |

Total: 60 RED tests and 9 preservation tests.

- **RED tests.** 57 are for census cases: the 17 compile-time cases, and 39 runtime-rule cases, one of which needs a
  test in each layer. The other 3 cover constructions found in review: the canonical media key, disagreeing copies
  of a classified record, and a total under a waived Before.
- **Known-limit pins.** S9 adds 5 pins beside controls, for the serious and uncertain disclosed cases: the
  `no_first_parent` waiver, the caller-built schema input, the caller producer inventory, the Held alias, and
  `content_hash` with coverage.

### 7.2 Seams for the compile-time claim

A compile-time claim says a case is impossible because one recording point records or requires the value. A
recording point is:

- an accessor;
- a population settle function, including its member recording;
- a read inside a grant or proof constructor;
- a marker fact settled in ingress.

Two checks make that claim testable.

- **Recorded keys.** From S2 on, every RED test for a compile-time case asserts that the decision recorded the expected
  key, whether or not other routes also refuse it.
- **Single-cause seams.** Each recording point gets a `#[cfg(test)]` seam, inline in the crate
  (`src/settle/probe_tests.rs`), that skips that one recording or that one read.
  - With the seam on, the recorded-key assertion must fail.
  - Where the seam removes the sole refusal route, the decision must also go back to admitted.
  - Each seam has a paired control that is unaffected by it.

A `cfg(test)` seam cannot remove a type requirement. The seams therefore test the recording and the reads that a
grant or proof constructor makes, not the type rule itself, which rests on module privacy. Compile-fail tests cover
only the public surface: doctests show that `ClaimStatus` cannot be constructed and that `Settled` is not exported.
Crate-private constructors are kept private by module privacy, and the import check rejects calls to them outside
their modules. UI compile-fail tests through a cfg-gated facade are an owner option (question 11).

11 seams:

| Recording point | Compile-time cases it covers (runtime-rule cases it also guards) |
|---|---|
| frozen `input(id)` and the `CountedInputs` member recording | 3 (plus 1 runtime) |
| `slots(role)` | 4 |
| `DocumentSchema` | 3 |
| `DocumentClock` | 1 |
| `SchemaBasis` in a media proof | 1 (plus 1 runtime) |
| the `SubjectSelection` read in `ExemptionGrant` | 2 (plus 2 runtime) |
| the `Clock` and `Cutoff` reads in the grants | 3 |
| the reads of the `ComparablePair` constructor | 0 (2 runtime) |
| the `Marker` read behind field presence | 0 (2 runtime) |
| the reads of the `CompleteBasis` constructor | 0 (4 runtime) |
| `record_merge`, the one function behind every `Merge(id)` recording (`input(id)`, `Slot(id)`, the grants) | 0 (its seam must fail the three merge preservation tests of S5a) |

The compile-time column sums to 17 and the runtime column to the 12 cases that moved in revision 4 (section 5). A
recording point that has no test failing under its seam has an unproven claim. The runtime rules need no seam: each
RED test fails on the pre-change code by construction.

### 7.3 Core and boundary tests

- **S1: key rendering table.** Every `ClaimKey` variant renders to today's published spelling and refusal-id spelling,
  with a round trip over every key in the existing fixtures.
- **S2: evaluator core.**
  - A read of a withheld fact refuses even when the value is unused. Control: the same read of an admitted fact
    admits.
  - The kind table is acyclic apart from the declared same-kind self-loops, and a kind-order violation trips the
    assertion.
  - The active-read rule reproduces today's data-cycle output.
  - Direct closure reads reproduce today's walk refusals.
  - An optional read does not block.
  - The publication conditions hold, including the refusal-only keys on an invalid document.
  - Counts at publication equal today's on every existing fixture.
  - A settle call outside the dispatcher is rejected by the import check.
- **The import check** (`tests/policy_imports.rs`) enforces items 6-9 of section 3.4. Its self-test plants four
  things in scratch copies of a policy file, and each must fail the check:
  - a DTO import;
  - a raw marker field read;
  - a `settle(` call;
  - a reader method returning an index.
- **S7: marker parity.** For every marker state in the census's marker-by-consumer matrix, the marker answers'
  presence and authority equal the diff's.
- **S5b: layer parity matrix.** Each shared relation rule gives the same verdict in the admission and frozen layers on
  paired fixtures.

### 7.4 Byte gate (every step)

**Replay harness.** One harness, compiled against the base and against the step head, runs the 37 pinned pairs. For
every invocation the existing replay driver makes, it writes the complete output:

- each `admit_schema_input` call: the full admission report and the projection index;
- each `ImpactDocument::validate` call, on the built document and on the document decoded by `from_json`: the full
  validation report;
- every document file.

The harness manifest enumerates every invocation and artifact. Every output is compared byte for byte, including the
rendered key order, the evidence order, absent optional fields and the counts of the final rendered map. Matching
summary counts are not enough.

**Refusal byte controls.** The 37 pairs contain no refusal, so refused states are controlled separately by:

- `tests/fixtures/p1d_rows/expected-admission.json`;
- the expectations in `tests/fixtures/p1d_rows/rows.tsv`;
- the fold tests (`tests/support/fold*.rs`);
- the known-limit rows.

A refused state that is not a census case must stay byte-identical. A refused state that is a census case changes
only in its step, and that step lists the change. S4a and S4b list each invalid-document fixture whose usage,
binding-status or membership decision moves.

**Failures and marker incidence.** Any other difference fails the gate and is resolved before merge, never
re-baselined. A corrected marker that would change a replay byte is a compatibility conflict that goes to the owner.
Step S7 also counts, on the 37 pinned schemas, the marker states whose answers differ from the public fields.

## 8. What P4 needs and what this does not fix

### 8.1 Minimum for P4

Phase 1 (section 6.0) meets none of items 1 and 2 and only part of item 3; items 4 and 5 are P4's own. Item 1 is met
at S6, item 2 at S7 and S8, item 3 for membership at S8, all in phase 2.

1. **S1-S6 merged before P4 certifies any admitted or frozen cell.** The request route reaches the shared policies. A
   positive cell binds its role, input, side, tree, scope, mode clock and receipt closure. A computed result needs its
   merge evidence, and a waived Before needs its grant. A frozen cell is certified only for a document P4 produced
   from admission in the same run.
2. **S7 before saved-status cells, S8 before membership negatives and known totals.** P4 supplies
   `SegmentPopulation` for each saved scan it certifies, by reconstructing the segment manifest from its own scan of
   the admitted client tree that the saved scan read (not the current tree), a caller contract (section 3.9);
   without it a saved negative prints **Not checked**. S7 must merge before any
   saved-status cell (current, stale, no saved match, established absence) is certified. S8 must merge before any
   membership negative (including consumer and route-table bases) or known total is certified. Until then P4 prints
   those cells as **Not checked** and keeps independent observations.
3. **Only the certified entry points (section 3.9).** In particular, P4:
   - builds every index it certifies from through admission, with the closure of the decision it supports;
   - builds `ProducerInventory` from admitted sources;
   - never reads the bindings wire `coverage` or `body_presence` as evidence;
   - never takes an `AdmissionReport` as input;
   - refuses to certify a slot, or a total, that rests on `no_first_parent`.
4. **The byte gate of section 7.4 passes on P4's own head.**
5. **P4 keeps its plan's evidence-removal controls** (PLAN, Slice P4, gates 1 to 5). This redesign makes those
   controls meaningful: removing a record removes a recorded dependency.

### 8.2 What it does not fix

- **Marker producers.** The marker answers record whatever the schema reader sets. The census did not enumerate the
  states of the producer modules: `schema/wire/composition.rs`, `wire/keywords.rs`, most of `wire.rs` and
  `wire/constraints.rs`, `element/*`, `schema/servers*`, `schema/openapi/parameters.rs`, and `schema/signature.rs`
  beyond `content_hash`. A producer that sets a marker wrongly is outside this redesign. The parity test pins
  consumers to the diff, not the reader to the truth.
- **Frozen evidence.** `validate` on a hand-edited or foreign document certifies local invariants and reference
  integrity only. It cannot authenticate source bytes, prove a saved result, or prove population completeness.
- **The 29 disclosed limits** of section 5. They include the 15 unread fields and identity relations, the bindings
  wire bytes (`coverage`, `body_presence`), and `content_hash` as defined in DESIGN 5.6.
- **Caller-built inputs.** `index_schema` over a caller-built `AdmittedSchemaInput`, and a caller
  `ProducerInventory`, stay computable and uncertified. The caller segment manifest behind `SegmentPopulation` is a
  caller contract too: P4 builds it from the saved scan's admitted client tree, not the current one, and the
  redesign cannot check it (section 3.9).
- **Compile-time enforcement of the proof types.** 17 of the 89 cases are compile errors. The 39 runtime rules,
  including the 12 that revision 3 counted as compile errors, hold only as far as their RED tests and the
  constructors' view and population checks hold (section 5).
- **The uncertified `unobserved_elements`.** It keeps the four membership cases that are fixed on the certified
  entry.
- **Runtime mutation.** This includes mutating inputs after admission and mutating the public fields of a
  `SchemaIndex` after its marker facts were settled, and anything else out of model for the lane.
- **Saved populations P4 does not supply.** Without an independent segment population a saved negative stays **Not
  checked**; the redesign does not reconstruct segment manifests itself.
- **The private pair.** Its schema source is a route table, and the admission module has no route-table path. This
  design certifies nothing for it in either phase; its first reachable point is a separate route-table admission
  slice that this design does not specify (open item).
- **Phase 1 alone.** It leaves 5 observed serious rows and 14 undetermined serious rows to phase 2, and certifies no
  shape beyond the README's (section 6.0).
- **Incidence.** The replay exercises one request shape. The redesign removes ways to write the defect; it does not
  measure how often the fixed states occur in live use.
- **Completeness of the census.** No census round called it complete. Members of the class in paths the census did
  not read are covered only to the extent that their reads go through the reader. P29 and the diff's own reads remain
  outside it.

## 9. Open questions for the owner

1. **Private policy crate.** Default: no. Use module privacy plus the CI import check (section 2.4). Revisit if a
   second crate ever depends on `prism-boundary` policy.
2. **The uncertified `unobserved_elements`.** Default: keep it byte-compatible and documented as uncertified; P4 uses
   `unobserved_elements_admitted`. Remove the old entry after P4.
3. **Frozen anchor inference without a subject head** (the client-only fixture used by the fold controls). Default:
   align with admission, with no inferred anchor. In S3b, add `subject_head_input_id` to that fixture and list the
   changed controls.
4. **`no_first_parent`.** Default: disclose it, and P4 certifies neither such a slot nor a total resting on it.
   Alternative: refuse the waiver until the document can record the parent-absence evidence.
5. **Marker states in the replay.** Default: S7 measures before merge, and merges only if no replay byte moves. By
   construction none should.
6. **The diff onto the marker answers.** Default: no; the parity test only.
7. **A named-tree reference blob from another input.** Default: refuse it (`slot_mismatch`, dimension `input_id`).
   Independent-cutoff references keep their free identity.
8. **A waived Before committed after the cutoff.** Default: refuse it in both layers (S3b, S5a). A Before snapshot
   without a recorded commit date keeps today's behaviour.
9. **Interim P4.** Default: P4 may start building after S6, with saved-status and membership cells shown as **Not
   checked** until S7 and S8.
10. **Refusal codes.** Default: no new codes. A user claim that actually collides with a generated media key is
    refused with `receipt_conflict`, dimension `claim_id`; a media-shaped id that collides with nothing stays
    admitted. New dependencies render as `missing_dependency` at their refusal-id spelling (section 3.5).
11. **UI compile-fail tests.** Default: no. Rely on module privacy, the import check and the public doctests.
    Alternative: trybuild tests through a cfg-gated facade.
12. **Which review modes the pilot uses (live, historical, version).** Open product question; the design has no
    default. The replay used historical, and the plan names all three. Of the 46 undetermined census rows, 5 need a
    mode only, 34 need a product choice only, and 7 need a product choice and a live or version mode (section 6.0).
13. **Whether the pilot reviews merged branches or only first-parent pairs.** Open product question; the design has
    no default. A merged branch that was behind its target needs a computed tree to review: 283 of 359 merges in the
    public pair's window (35 of its 49 service-touching merges; 21 in the usable replay cohort) and 391 of 1,018 in
    the private pair's window (273 of its 716 route-touching merges; 14 in its usable cohort). The answer sets whether
    the computed-tree rows count as observed or not.
