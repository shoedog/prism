# Lane 5: the impact report, design amendment A1 (2026-11-24)

Status: drafted by the design seat (gpt-6.2-astra, max) on 2026-11-24 from the owner's feedback on the first five
private reports; under review. It amends `DESIGN.md` sections 5.2 (the impact document gains the fields of
"Machine-readable form") and 6.1 (the replay's labels become an evaluation companion, never part of a live report).
All examples are invented. The owner's words and requirements R1 to R5 are in the issue #848 comment of 2026-11-24:
plain words (R1), tabular layout (R2), added and changed kinds beside removals (R3), no future in a live report
(R4), direction and order of change (R5).

Orchestrator's defaults for the three owner questions at the end (applied until the owner answers): the counterpart's
integration branch, with an exact revision for release reviews; an added route with no visible client is an
informational row; five operations, three calls and three fields shown per operation in an attachment, with the full
tables one link away.


Use one section per API operation, with the service code above its client calls. Lead with a finding the reviewer can act on, such as a client still calling a removed route or a new request using a method the checked service does not offer. All examples below are invented.

## Layout choices

| Layout | Reading order | Advantage | Cost |
|---|---|---|---|
| One change list | One table with **Change**, **Service code**, **Client call**, **Match**, **What to check**. For example, a removed operation with three calls occupies three rows. | Easy to scan a small change or copy into a review comment. | Repeats the service change for every site. An addition with no known client needs its own row or it disappears. |
| **One section per operation** | A method and path, a before/after table, then one row per client use. Field changes sit with the operation that carries them. | Keeps the change, service code and affected calls together. An addition remains visible even with zero client rows. | A possible call can appear under several operations. The summary must count that site once. |
| One worklist per client function | **Client function**, then **Client call**, **Operation**, **Service code**, **What to check**. A separate **Operations with no client match** table lists the rest. | A client maintainer can visit one function and review all its affected calls together. | Fragments a service change across functions. Unmatched additions and producer-only changes are easy to miss. |

For a client change, group a unique match under its service operation. Show the requested method and path even when no service operation matches them. A call that changes targets can appear in both groups under one site ID. Put a changed call whose address cannot be read under **Client calls with an unknown target**.

## The versions a reviewer is comparing

An *operation* is an HTTP method such as GET or POST together with a path. The *service* is the producer that declares the operation. The *client* is the consumer that sends the request. In displayed paths, `{…}` means one variable part of the address. Put these explanations at the first operation heading. Expand fields as, for example, **Response body (200) › sheets › each item › shade**. The number is the response status. **Request body** means data the client sends.

| Report title | Before the change | After the change | Held fixed in both columns |
|---|---|---|---|
| **Service change checked against client code** | Service at the base of the change | Service with the change | The same client code, including calls that failed to match the old service |
| **Client change checked against service code** | Client at the base of the change | Client with the change | The same service operation set, schema when available, and service locations |

Under **Versions checked**, print **Service change** and **Client checked**, or **Client change** and **Service checked**. Give the change subject, repository, before and after commit IDs and committer dates, and the checked repository, commit ID, date and selection reason. Use UTC and revision-specific source links. Availability describes what those sources declare. For a single commit the base defaults to its first parent. A review can also supply a base and head spanning multiple commits.

| Selection | Wording beside the checked revision |
|---|---|
| Other repository, default in either direction | **Latest revision on {branch} at or before the change.** Use the last eligible commit along that branch's first-parent history, comparing committer times to the changing side's after revision. Record the branch and resolved commit. |
| Both sides in one repository | **Other side from the base of this change.** This exposes a dependency even if both sides were edited in the same commit. An explicit comparison against the joint result reads **Both sides from the changed revision** in the after column. |
| Reviewer or review system supplies an exact revision | **Revision supplied for this review.** Show who or what selected it. It takes precedence over the default. |
| Earlier, equal or later commit time | **Checked revision is {duration} earlier. Later {client/service} edits are outside this check.** / **Both commits have the same time.** / **Checked revision is {duration} later. This comparison uses later code. Compatibility on the change date is unknown.** Show both dates. Deployment order across repositories is unknown. |
| No eligible revision or the supplied revision cannot be read | **{Client/Service} revision unavailable. Only the {service/client} change is shown.** Give the cutoff or requested revision and the reason. Keep the changing side's known facts. |

The default never selects a later commit. An explicitly selected later revision answers a present comparison question about those exact versions. Label it at the top as **Comparison with a later revision**. Historical as-of reports use only the default or an eligible supplied revision that was available at the cutoff.

## What appears on the change

1. **Review first** gives the most consequential observation and an action in one sentence. Follow it with the number of operations in the review, changed fields when measured, distinct client call sites listed, and how many of those have only possible matches. Also give the number of calls with unreadable targets in the client scan. If a check did not run, print **Not checked** and its reason in place of its count.
2. **Versions checked** gives the evidence dates and selection above. **Coverage** reads **Operations only. Routes come from service code. Request and response fields were not checked.** or **Operations and fields. The schema file describes request bodies and successful response bodies.** For partial input, name the missing side and the part that could be checked.
3. **Changes by operation** uses the two tables below. **What changed** names the edit. **Before** and **After** give its values. **Service code** links each **Handler**, the function that handles the request, with its file, line and version. Label other sources **Route declaration**, **Schema entry for the operation** or **Field declaration**. Use **Possible handler** with the reason for an uncertain location, **Handler location unknown** when only a declaration was found, or **Service location unknown** when no site was found.
4. **Client call and function** links each call's file and line and names the function containing it. For an edited or removed call, identify the before site. For an added call, identify the after site. Link both when they differ. **Match before** and **Match after** use the version meanings in the title table. **What to check** states the risk and the next action. A field change gets the local caption **These calls use or may use the operation. Client use of the listed fields has not been checked.**
5. **Who calls this code** gives **Client call**, **Called from**, **How it reaches the call**. List the recorded functions and source links up to two calls away, with **Directly** or **Through {function}**. Distinguish **No caller found within two calls** from **Callers were not checked**. A recorded caller shows a link in source code.
6. **Limits of this check** lists unresolved calls and missing evidence. **Other differences at these versions** holds the standing mismatches. A reviewer can stop after the operation tables and still see the counts and important limits at the top.

### Service change, operations only

**Service change checked against client code**  
**Review first** — The service removes an operation that the checked client still calls. Update the client or keep the old route during rollout.  
1 operation · 1 client call listed · 0 with only possible matches · 1 call with an unreadable target in the client scan.

**Versions checked**  
**Service change** — Remove the lunar folio list · Printroom `91be101` (2 Jun 2026, 09:00 UTC) → `91be102` (4 Jun 2026, 12:00 UTC).  
**Client checked** — Chartdesk `73ad201` (3 Jun 2026, 12:00 UTC). Latest revision on main at or before the change. Checked revision is 1 day earlier. Later client edits are outside this check.  
**Coverage** — Operations only. Routes come from service code. Request and response fields were not checked.

**GET /lunar-folios**

| What changed | Before | After | Service code |
|---|---|---|---|
| Operation removed | GET /lunar-folios | Absent | Before · handler `listLunarFolios` at `press/folios.go:27` |

Match compares the method and path in the checked code. Both columns use the same client call.

| Client call and function | Match before | Match after | What to check |
|---|---|---|---|
| `desk/folios.ts:18` · `openLunarFolio` | Matches this operation | No matching operation | Client calls a missing route. Update this call or keep the old route during rollout. |

**Who calls this code**

| Client call | Called from | How it reaches the call |
|---|---|---|
| `desk/folios.ts:18` | `paintMoonPanel` at `desk/panel.ts:36` | Directly |

**Limits of this check** — One more client call could use the removed route. Its address is built at runtime at `desk/lookup.ts:9`.

### Service change, with a schema

This excerpt starts at the changed operation. **Coverage** reads **Operations and fields. The schema file describes request bodies and successful response bodies.**

**GET /star-proofs** · Operation fields changed

| What changed | Before | After | Service code |
|---|---|---|---|
| Response body (200) › exposureSteps · Type changed | Integer | Text | Handler `listStarProofs` at `press/proofs.go:41` in both versions. Schema entry for the operation at `spec/prints.yaml:74` before and `spec/prints.yaml:77` after. |
| Response body (200) › paperTone · Field added | Absent | Optional text | Same handler. Schema entry for the operation at `spec/prints.yaml:77` after. |

These calls use or may use the operation. Client use of the listed fields has not been checked.

| Client call and function | Match before | Match after | What to check |
|---|---|---|---|
| `desk/proofs.ts:22` · `loadStarProofs` | Matches this operation | Matches this operation | Check whether the client uses exposureSteps in arithmetic and whether its decoder accepts the extra paperTone field. |

### Client change, checked against the service

**Client change checked against service code**  
**Review first** — A new client call sends POST. The checked service lists only GET at that path. Confirm the service version needed before enabling the call.  
1 requested operation · 1 client call listed · 0 with only possible matches · 0 calls with an unreadable target in the client after the change.

**Versions checked**  
**Client change** — Add proof submission · Chartdesk `73ad211` (1 Jul 2026, 09:00 UTC) → `73ad212` (4 Jul 2026, 12:00 UTC).  
**Service checked** — Printroom `91be111` (3 Jul 2026, 12:00 UTC). Latest revision on main at or before the change. Checked revision is 1 day earlier. Later service edits are outside this check.  
**Coverage** — Operations only. Routes come from service code. Request and response fields were not checked.

**POST /star-proofs**

| What changed | Before | After | Service code |
|---|---|---|---|
| Client call added | No call at this site | POST /star-proofs | No POST operation found. The GET handler at the same path is `listStarProofs` at `press/proofs.go:41` in the checked revision. |

Match compares the method and path in the checked code. Both columns use the same service revision.

| Client call and function | Match before | Match after | What to check |
|---|---|---|---|
| After · `desk/submit.ts:14` · `sendStarProof` | No call at this site | Different method at this path. Service lists GET. | Check the intended method. If POST is intentional, make it available before enabling this call. |

### The separate review when the service adds POST

Suppose Printroom adds POST in `91be112` on 5 Jul at 12:00 UTC, with `91be111` as its base. Chartdesk `73ad212` from 4 Jul is now the fixed client. The service change has its own report and action.

**POST /star-proofs**

| What changed | Before | After | Service code |
|---|---|---|---|
| Operation added | Absent | POST /star-proofs | After · handler `submitStarProof` at `press/proofs.go:63` |

| Client call and function | Match before | Match after | What to check |
|---|---|---|---|
| `desk/submit.ts:14` · `sendStarProof` | Different method at this path. Service lists GET. | Matches this operation | Existing client call gains a route. Confirm this handler accepts what the client sends. |

If the service adds POST **first**, the service review can instead say **Added route has no client match**. The client review can then say **Client call matches the service** when checked against the version offering POST. For retirement, removing the service route first exposes remaining calls. Removing the client call first shows which other uses remain. The subsequent service review can report zero matches within its stated coverage.

## From evidence to review wording

The small key beside each client table says **Match compares the method and path in the checked code.** Use the following cell text. Append the reason in the same cell wherever there is uncertainty.

| Evidence | Cell text |
|---|---|
| One operation identified by the existing binding rules (`bound`) | **Matches this operation**. Name the operation if it differs from the section heading. |
| Several operations fit, or a variable must take one particular literal value (`candidate`) | **Possible match. Several operations fit** with their methods and paths, or **Possible match. A value chosen at runtime must equal {value}**. |
| Known method and path with no operation in the checked source (`unbound`) | **No matching operation**. When the standing comparison finds other methods there, use **Different method at this path. Service lists {methods}.** |
| The analyzer could not read the target (`unbound`) | **Target unknown.** Append the applicable sentence — **The address is built at runtime.** **The address depends on a condition.** **The lookup key is not text.** **The HTTP method is chosen at runtime.** **The client method is unrecognized.** **This form of client call is unsupported.** |
| Site absent on this side, or input unavailable | **No call at this site** for demonstrated absence. **Not checked. {reason in words}.** for an unavailable check. |

Beside a reused location, show what description of the operation or field it was checked against. For a producer edit, the §5.6 statuses compare the recorded signature with the old signature. A signature records the field's type and constraints, or the collection of these for an operation.

| Evidence about a saved match | Wording at the location |
|---|---|
| Old signature agrees (`binding_current`) or differs (`binding_stale`) | **Checked against the same description before the change** or **Checked against a different description. Recheck this location.** |
| No binding and its schema source predates the element (`element_unknown`) | **The schema used for this client predates this operation or field. Check again with a newer schema.** An earlier client commit alone does not establish this. |
| Addition, with no old signature | **New operation checked against the after version** or **New field checked against the after version** when checked. Record the new-side comparison separately. |
| Client is the changing side | Check saved evidence for both client states against the one fixed service description. Use **Checked against the selected service** on agreement and the different-description warning on disagreement. |
| Signature or source unavailable | **Description comparison unavailable.** Qualify any reused location. A fresh method/path comparison against a route table still supports an operation match at its stated coverage. |

### Operation and client changes

**Operation removed** and **Operation added** are the old-only and new-only members of the service operation sets. **Operation fields changed** means the method and path persist and at least one recorded request or response field differs. Show its field rows. For a retained route on a code-first service, say **Only the method and path were checked. Changes to this route's inputs, outputs and behavior are unknown.** A handler edit alone cannot establish an interface change. **Client call added**, **Client call removed** and **Client target changed** come from the client before/after comparison.

A route rename or move produces two independent groups, **Operation removed** and **Operation added**, with the calls and handlers for each side. A method change at one path likewise shows both methods and both groups. Place same-path entries next to one another within a review priority. Similar names, a shared handler or a simultaneous edit do not establish a replacement. Keep the two facts independently actionable. A rename of an internal type with identical recorded fields produces no field edit.

| Risk or review result printed on the row | Evidence at the selected revisions | What to check |
|---|---|---|
| **Client calls a missing route** | A fixed client matched the removed route before and has no match after, or a new or retargeted client call has a known target absent from the fixed service. | Update the call, retain the old route during rollout, or supply the needed service version. |
| **Different method at this path** | The request has no match and the service lists the path under another method. Name the requested and available methods. | Confirm the intended method and the rollout order. |
| **Existing client call gains a route** | A fixed call with a known target had no matching operation before the service addition and uniquely matches it after. | Confirm that this addition supplies the operation the client expects. |
| **Added route has no client match** / **Removed route has no client match** | A completed scan finds neither a unique nor a possible match on the relevant service side, after for an addition or before for a removal. Print the checked client revision and unresolved-call count. | For an addition, confirm the intended first client. For a removal, check users outside the inspected code before retiring it. |
| **Client target needs confirmation** | A call has only a possible match on a relevant side, or a service edit changes which operation an existing call selects. Show the targets and the reason. | Inspect the runtime value or intended destination before assigning the impact to a client. |
| **Client call matches the service** | A new or retargeted client call uniquely matches the fixed service operation. | Confirm this is the service version the changed client will use. |
| **Client stops calling this route** | A removed or retargeted client call used the route before. List any other client sites still matching it after, with unique and possible matches distinguished. | Check remaining uses before scheduling a service removal. |

Give the method-specific result precedence over the general missing-route result at a site. An identical mismatch present before and after belongs under **Other differences at these versions**. A missing route that this edit supplies belongs in the change table. A partial check can still show individual observed sites with their limitations.

### Fields on a service with a schema

Under a retained operation, identify each field by request or response, response status where applicable, and its full path in the body. Use **Field added**, **Field removed**, **Type changed**, **Format changed**, **List item type changed**, **Now required**, **Now optional**, **Allowed values added** and **Allowed values removed**. Print the before/after values, including the actual options gained or lost. **Required** means the field must be present in that message. Use **Value restriction unknown** when the source leaves this unclear. Combine changes to one field in one row. Under a whole-operation addition or removal, show its fields as detail and count the operation once.

| Risk or review result | Evidence | What to check |
|---|---|---|
| **Request now requires more** | New required input, optional input becomes required, or accepted values narrow. | Check what each listed client sends, including omitted values. Coordinate the client update before enforcing the new requirement. |
| **Request format changed** | Input removed, or its type, format or list item type changes. | Check whether clients still send the old shape and how the service handles it. |
| **More request forms accepted** | New optional input, requirement relaxed, or accepted values widen. | Existing clients may need no edit. Verify service support before clients start relying on the new form. |
| **Response field may be missing** | Output removed or now optional. | Inspect readers and their handling of missing data. |
| **Response format changed** | Output type, format, list item type or allowed values change. | Check decoding, calculations and handling of known or unfamiliar values. |
| **Response field added or guaranteed** | Output added or changed from optional to required. | Check strict decoders for an addition. Clients relying on its presence need a service version that guarantees it. |

When checking a client against a service with a schema, say **Client field use was not checked**, including when the call diff is empty. A client can edit a field read or write without changing the method or path. Identifying such a site requires evidence connecting the expression to this operation and field at the named revision.

## Incomplete checks and long changes

| Situation | Report wording and placement |
|---|---|
| Completed check, no unique match | **No client call matched this operation uniquely in {client revision}.** If candidates exist, follow with **{n} possible matches** and list them. With none, show **No client match found in the checked code** and the limit on unresolved calls. |
| Unreadable calls could hide more users | **{n} calls have an unknown target and could include other users.** Under **Limits of this check**, list **Client call and function**, **Version**, **Why the target is unknown** for every such site. List these once for the scan. |
| Missing configuration, client sources or usable imports | **Client check unavailable.** Append **Client configuration unavailable**, **Client source unavailable**, or **No import of the configured client was found**, as applicable. Show the change and the unmeasured count as **Not checked**. Use **Service check unavailable** for the equivalent reverse failure. |
| Inputs disagree | **Schema and service declarations disagree**, **Binding used a different schema from both versions**, or **Working copy includes edits outside the named commit** beside the affected source. Give the specific affected check. If an expected schema cannot be read, field comparison is unavailable. |
| Differences visible in a snapshot | Under **Other differences at these versions**, use **Where**, **What was found**, **When observed**. Use **Client call has no matching route**, **Route has no matching client call**, or **Different method at this path**. For schema cross-checks use **Schema operation has no matching route declaration**, **Route declaration is missing from the schema**, or **Schema and route declaration list different methods**. Mark **Already present**, **Appears with this change**, or **Clears with this change** only with both comparisons. Otherwise use **Seen at the checked versions**. Link to the change row for any overlap. |

Show a missing reference or unusable scan first. Order operation groups by newly unsupported requests, changed field obligations or formats, uncertain targets, then informational results. Within a priority sort by path then method, fields by message and full field path, and calls by file and line. Keep producer sites beside their operation and show possible matches after unique ones.

In the attachment, preview five operations and up to three client calls and three changed fields per operation. Use **Showing {shown} of {total} operations**, **Showing {shown} of {total} client calls** and **Showing {shown} of {total} changed fields** where a list is shortened. Link **All operations and client calls** to the complete static tables, including all callers, unresolved sites and standing differences. A viewer can reach every row without expandable sections.

Count operations by API, method and path, including known requested targets absent from the service. Count each changed field under its operation and each client site once across fields, candidate operations and caller paths. A linked before/after edit counts once. The unreadable-target count covers the fixed client for a service change or the after client for a client change. List before-only gaps with their version in the full report.

## Replay evaluation (uses later history)

A past change gets the ordinary revision comparison. The study can attach **Replay evaluation (uses later history)** for the producer-change predictions it followed. Join that companion by report, element and site IDs. Keep later edits, release tags, reader judgments, field-name searches, controls and exclusions based on future reversions in the evaluation. Withhold all of them and the same live inputs must still render identical rows, counts, grades and ordering.

An invented companion row for the lunar folio example follows. In its header, give the last revision and date examined.

| Client line followed | First recorded edit | Time from the service change | Reader's assessment and evidence |
|---|---|---|---|
| `desk/folios.ts:18` | `73ad207` · 6 Jun 2026, 12:00 UTC · Reindent folio list | 2 days | Unrelated edit. The change reformatted the call. |

**First recorded edit** means the earliest commit that modified, deleted or moved a followed line beyond what history tracking could trace. For a field this can be the call line or a followed property line. Show which line supplied the observation when known. Use these translations in the companion, inline where a value appears.

| Evaluation value | Display meaning |
|---|---|
| `same_commit`, `within_14_days`, `later` | **A followed line changed in the same commit**, **A followed line changed within 14 days**, or **A followed line changed later**. Give the commit, subject and elapsed days. Add commit distance only within one repository's first-parent history. |
| `present_at_pin`, `present_at_head` | **Followed lines remained unchanged through {revision, date}**. Identify the pinned endpoint or the end of the checked client history. |
| `coordinated` | **Changes made together for the same feature.** The reader judged that the edit responded to the service change and both first appeared in the same release. Give the feature and release evidence. |
| `fix_after_release` | **Reader identified a follow-up fix.** Say which evidence applies — a release contained the service change before the fix, or the fix message cited an issue. Claim a release gap only when the tags establish it. |
| `dead_code`, `real_mismatch` | **No client caller found for this function at {revision}** or **A client caller still reached code for an element missing at {revision}**. For fields, identify the missing field and the surviving property-name evidence. |
| `unrelated_edit`, `false_prediction` | **Unrelated edit** with the observed reason, or **Reader found this call unaffected** with the reason the prediction missed. |
| `cannot_tell`, missing assessment | **Reader could not determine the effect** or **No reader assessment recorded**. |
| `field_spelled` | For field rows only, **Property name found in the searched client files** or **Property name absent from the searched client files**. Explain there that the search covers the call's file and files importing its enclosing function by name. Spelling alone leaves actual use of the field unknown. Omit this entry for operations. |

Where tags are shown, label them **Earliest release tag containing the service change** and **Earliest release tag containing the client edit**. Mark missing evidence **Release tag unknown**. A two-day interval can accompany any applicable reader assessment. Elapsed time alone establishes only when a line was edited.

## Data needed to produce the layouts

| Data | Available to reuse | Additional input or reporting work |
|---|---|---|
| Change and reference | Service before/after IDs, dates, subject, operation sets and schema field signatures. Replay selects a client revision. | Carry both roles' dates, source revisions, reference branch, selection reason, and explicit pins in both directions. Resolve one fixed counterpart for the comparison. Report a missing reference as a check state. |
| Service change list | The schema differ already has add rows and several field differences. Both route sets exist. The replay's eligible list omits operation and field additions and consults later revisions. | Expose the live two-revision diff, including new-only operations, new fields, both directions of requiredness and allowed-value changes. Retain structured old/new shapes and group by owning operation. Preserve method/path replacements as separate edits. |
| Service locations | Operation bindings, route declarations and, for schema inputs, schema locations. | Carry all available sites on the applicable version — before for removals, after for additions, both for retained changes. Identify a handler, a route declaration and a schema entry separately. Preserve an unknown handler explicitly. |
| Client inventory and matches | The binding and standing checks can supply unique, possible and unmatched calls, reasons, containing functions and available caller links. The replay prediction list retains unique old-side matches only. | Supply the complete in-scope call inventory with requested method/path when known, all candidate targets and reasons. Compare the fixed client to both service sets, including formerly unmatched calls. Retain per-side outcomes, source links and coverage. Carry caller evidence to depth two when collected. |
| Client as the subject | A single snapshot can be checked against a service. | Supply client before/after revisions and a call-site change list with old/new source locations and targets. Check both against the same service revision. Distinguish a proved edit from two independent added/removed sites. Retain all remaining uses after a deletion. Field-only client edits need proved read/send sites to gain field-level claims. |
| Standing differences and evidence limits | Snapshot mismatch lists, recorded binding shapes, source hashes and reasons. | Associate lists with exact input versions and scan status. Two comparable snapshots support “already present” or “clears”. Include known method alternatives, deduplicate overlapping lists, and preserve completeness and unavailable reasons. |

| Layout | Data specific to that option, in addition to the common inputs above |
|---|---|
| One change list | Flatten each element/site association, retaining an explicit row for a change with no client. No additional source extraction. |
| One section per operation | Carry owning operation on field edits, a stable site reference for each before/after appearance, and the requested target for calls with no operation. Use these to list a call once per group and share caller details. No inferred rename or client field-read extractor is needed. |
| One worklist per client function | Retain containing-function identity and its revision for every relevant site, plus a group for sites with no known function. Build a reverse index from functions to operation/field changes and an inventory of service changes without client rows. |

## Machine-readable form

Extend the pilot impact document in DESIGN §5.2. Keep `schema_version`, `inputs`, `change`, `impacts`, `unobserved` and `warnings`, including API identity, schema sources and hashes, tool build, repository revisions, original binding grades and recorded callers. Join elements by API and canonical element ID. The names below describe fields in the live JSON. Reader-facing phrases are rendered from this evidence.

| Object | Field names and meaning |
|---|---|
| Report and inputs | `report_id`, `api`, `subject_role`, `inputs[].input_id`, `role`, `repo`, `rev`, `committed_at`, `dirty`, `schema`, `tool`. The subject role is producer or consumer. |
| `change` and `reference` | `change.before_input_id`, `after_input_id`, `subject`, existing old/new schema identities when applicable. `reference.role`, `input_id`, `requested_rev`, `history_ref`, `selection_reason`, `selected_by`, `time_relation`, `offset_seconds`, `check_status`, `reason`. A fixed service supplies one schema source for both client states. |
| `change.elements[]` and `change.call_changes[]` | `element_id`, `operation_id`, `change_kinds[]`, `message_side`, `response_status`, `field_path`, `before_presence`, `after_presence` (present, absent or unknown), `before_shape`, `after_shape`. Shapes expose `type`, `format`, `required`, `allowed_values`, `items_type`. Call changes carry `usage_id`, `change_kind`, `before_site`, `after_site`, `before_target`, `after_target`. Targets carry `method` and `path`. |
| `impacts[]` | `operation_id`, `requested_target`, `element_ids[]`, `producer_sites[]`, `usage_ids[]`, `risks[]` with `kind`, `element_ids[]`, `usage_ids[]`, `review_action`. A requested target may have no operation ID. Each service site has `element_id`, `input_id`, `site_kind`, `file`, `line`, `function`, `grade`, `reason`. |
| `usages[]` | `usage_id`, `before_site`, `after_site`, `match_before`, `match_after`, `field_use_status`. Sites carry `input_id`, `file`, `line`, `function`, `callers_status`, `callers[]` with source location and path through the calling functions. Each match carries `service_input_id`, `requested_target`, `check_status`, `grade`, `reason`, `operation_ids[]`, `available_methods[]`. |
| Status and limits | Keep `binding_status` per `element_id` and `consumer_input_id`, with `schema_source`, `signature_basis`, `recorded_shape`, `comparison_shape`. `coverage[]` carries `input_ids[]`, `scope`, `field_coverage`, `check_status`, `reason`, `unresolved_usage_ids[]`. `standing_mismatches[]` carries `kind`, `input_ids[]`, `when_observed`, `operation_ids[]`, `usage_ids[]`. Warnings carry a reason and affected input or element. |
| `summary` | `operation_count`, `changed_field_count`, `listed_usage_count`, `possible_only_usage_count`, `unreadable_target_count`. Use null for an unmeasured count. An empty checked list and zero express a measured absence. Preserve the existing meaning of `unobserved` for a changed element with no binding in any document. |

The separate evaluation JSON names `live_report_id`, `observation_end` (repository, revision and date) and `observations[]` keyed by `element_id` and `usage_id`. Its fields are `followed_lines`, `outcome`, `first_touched_line`, `touching_commit`, `lag_days`, `lag_commits`, `change_release_tag`, `edit_release_tag`, `reader_assessment`, `assessment_evidence`, and, for field changes only, `field_spelled`. A live report can be rendered in full without this document.

## Questions for the owner

1. Which branch or release line represents the counterpart your team actually reviews against? **Default** to its configured integration branch, with an exact revision supplied for release-specific reviews.
2. Should an added route with no visible client ask the author to name its first adopter? **Default** to an informational row below compatibility issues, with the intended client as a review prompt.
3. How much detail should appear directly on the change? **Default** to five operations, three calls and three changed fields per operation, with the complete tables one link away.
