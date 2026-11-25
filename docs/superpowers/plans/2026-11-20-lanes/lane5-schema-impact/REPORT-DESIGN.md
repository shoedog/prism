# Lane 5 impact report, design amendment A1

Round 1 fold, 2026-11-24, for owner review. This amends DESIGN §§5.1, 5.2, 5.4, 5.6 and 6.1. In issue #848 the owner asked for plain words
(R1), tables (R2), additions and changes as well as removals (R3), a review without hindsight (R4) and support for either side changing
first (R5). All examples are invented. [First cut for the prototype](#first-cut-for-the-prototype)

## Layout choices

| Layout | What the reviewer sees | Benefit and cost |
|---|---|---|
| One change list | **Change**, **Service code**, **Client call**, **Match**, **What to check** | Easy to copy into a short review. Repeats the service edit for every call. Flatten the same evidence and keep a row for changes with no client listed. |
| **One section per operation** (recommended) | Service before/after and source locations, then a row per client call. Put fields under the operation that carries them. | A reviewer can follow one interface change from declaration to use. Needs the owning operation on each field and a shared reference to each call shown in multiple groups. |
| One worklist per client function | **Client function**, **Client call**, **Operation**, **Service code**, **What to check** | Convenient for a client maintainer. Needs a function-to-changes index, a group for unknown functions and a separate table for service changes without client rows. |

Start every report with **An operation is a method and path, such as GET /star-proofs. The service declares it. A client call is code that
sends a request.** Use *route* only for a service declaration or registration. Explain **{…} stands for one variable part of the address**
beside a path that uses it. These captions precede **Review first**.

## Versions checked

Print the mode, change subject, repositories, before and after commit IDs and UTC committer dates, the fixed revision and its date, and
why it was chosen. Show the branch, cutoff and evidence for its tip when a branch is used. Link code to the exact tree that supplied it.
Print **Checked revision is {duration} earlier/later than the change** or **Both commits have the same time**. These are commit dates.
Deployment order across repositories is unknown.

| Mode on the report | Cutoff and selection |
|---|---|
| **Live review** | Cut off at **Report time**. Capture the configured counterpart integration branch tip at that instant, or use a record of ref updates through that instant. Freeze its ID. A client added after the service commit but present at report time belongs in this review. |
| **Historical review at the change** | Cut off at **Change time**, the after commit's UTC committer date, including for a multi-commit range. Establish the tip at that cutoff from a snapshot or a ref-update history that covers the cutoff. For a supplied pin, a contemporaneous review record can establish its availability then. Show the record and time. Tree membership in a named commit establishes its contents. A commit date alone cannot establish branch arrival. |
| **Comparison of selected versions** | Compare the named trees. Show who supplied a release pin, or the rule and frozen history tip used to reconstruct it. For date-only selection use the last commit along that tip's main line of history with a committer date at or before change time. Print **Selected by commit date. When this revision reached {branch} is unknown.** An explicitly newer pin reads **This comparison uses code from after the change.** |

A release pin used for a version-specific comparison, an explicitly later pin and any pin with unproved availability use the third mode. A
normally observed live tip can be newer than the change. If cutoff evidence is missing, print **Not checked — which revision was available
on {branch} at {time} is unknown.** Keep the subject's known facts. A dated fallback is a separately titled version comparison. Equal
times, backdated commits and a branch moved today supply no retrospective proof. An unreadable pin gets **Not checked** with the requested
revision and read failure.

For a single commit the default before revision is its first parent. For a range print the supplied base and head. Both **Before** and
**After** compare the changing side with the *same* named counterpart. The before column of a range makes no assertion about what the
counterpart contained on the base date.

| Direction | Before and after inputs to the impact step |
|---|---|
| **Service change checked against client code** | Compare the service base and head. Match the entire fixed client inventory against each service operation set, including calls with no previous match. |
| **Client change checked against service code** | Compare the two client inventories. Match both against the same service declarations, locations and schema where available. |
| **Service and client changed together** | One report, one versions block, service section first and client section second. Compare service-before/client-before and service-after/client-after. Also check each side shipping alone as shown below. |

In one repository, use the change's before and after trees for coordinated edits. For a live pull request behind its target, record the
proposed base and head, the observed target tip and the clean merge result. Compare **target → merge result** for the effective change,
with the unchanged role at the target for a one-sided edit. The same rule uses the recorded target at a historical cutoff. If a merge
result cannot be established, retain the observed source edits and print **Combined result: Not checked — {reason}**. Do not substitute
the topic head for a merge. Determine which roles changed from their input trees, including configuration. An unreadable role gets a
section with its limit. An empty extractor result cannot make a mixed change one-sided.

## What counts as evidence

**One rule for every comparison.** A claim may use absence, uniqueness, sameness or a total only when *every source region that could
change that conclusion* is accounted for at each required revision and under the named authority. Inventory the expected tree, changed
paths (including moves), configuration and source dependencies independently of what the recognizer emits. Record extraction status per
file **and per entry or region**. A complete region has positive evidence that each relevant construct was read or falls outside the
stated scope. If a changed region ceases to match the recognizer, require evidence of its deletion or of what replaced it. An empty result
cannot certify coverage.

If a file, entry or dependency is unread, skipped, ambiguous or unaccounted for, give the affected fact the value **unknown**. Print **Not
checked — {file or scope} at {revision}: {reason in words}.** Apply this before the diff, match advice, totals, notifications and
empty-report decision. If the gap cannot be localized, widen it to the enclosing operation, file or entire check. Keep independently
observed declarations and calls with their locations. An unknown side contributes neither an addition/removal nor a zero. A confirmed file
deletion establishes absence from that file only. For each service, schema and client reader, test the rule by marking a file or entry
unknown on either side. Every dependent change or zero must render **Not checked** with its reason and stay out of confirmed counts.

Name the **Source of operations** beside the result. A route table lists declarations, an annotation documents a handler, a registration
connects a path and method to code, and the committed schema describes fields. Show each source separately when they disagree. For
example, **The schema lists GET here. The registration at {location} lists POST. What handles GET is not checked.** A result about
declarations stays scoped to declarations. A negative about registered routes also needs complete registrations, applicable method/path
forms, mounted prefixes and verified address mappings through the named routing scope.

## What appears on the change

1. **Review first** states one evidenced result and one action. Surface a gap that prevents a decision. Give counts for operations,
   changed fields, client calls shown, calls with possible matches only, known requests with no match in the declarations read, and calls
   whose target is unknown. Mark each count's scope and version. An unmeasured total reads **Not checked — {reason}**. A partial report
   can still say **{n} observed**.
2. **Versions checked** and **Coverage** name the snapshots, operation authority, client scope and measured field properties. For
   code-declared operations print **Methods and paths from route declarations. Changed fields: Not checked — this service has no committed
   field schema.** Give schema coverage beside its count as specified below.
3. **Changes by operation** gives **What changed**, **Before**, **After**, **Service code**. Link a removal to its before location, an
   addition to its after location, and a retained edit to both. Label each link **Route declaration**, **Handler** (the function that
   handles the request), **Handler named in a comment**, **Schema entry for this operation** or **Field declaration**, according to the
   evidence. Use **Possible handler — {reason}** or **Handler location: Not checked — only the declaration was found** when applicable.
4. The client table gives **Client call and function**, **Request as written → path checked**, **Match before**, **Match after**, **What
   to check**. Show the client's own URL text, HTTP method, mapped service path and the name and revision of the mapping, including an
   explicitly unchanged path. Attach each cell to its service and client versions. A changed call with an unknown method or address has
   one home, **Changed client calls with an unknown target**. Link to that row from limits. Known unmatched requests keep their own
   method/path heading.
5. **Who calls this code** gives **Client call**, **Possible caller found by name** for a one-level search, or **Calling function** and
   **Directly / Through {function}** for resolved source links. Give revision and line. When the product checks two levels, explain **Only
   calls whose destination the code analysis could identify were followed, directly or through one other function.** Distinguish **No
   caller found by this search** from **Not checked — caller search did not run at this revision**.
6. **Limits of this check** gives the affected file or site, revision, plain reason and consequence. **Other differences at these
   versions** gives **Where**, **What was found** and **When observed**. For example, **The schema lists GET /ink-cards. No match
   among the registrations read**, or **This registration has no match among the schema entries read**. Apply the same coverage rule.
   Use **Already present**, **Appears with this change** or **Clears with this change** only with both comparisons. Otherwise use
   **Seen at the checked versions**.

The following product examples show the evidence each action requires. The prototype wording for the same kinds of edits appears under
**First cut for the prototype**.

### Service change, operations only

**Service change checked against client code · Live review.** Printroom removes a list endpoint. **Review first** — The client still
requests GET. Keep the GET registration until that client is updated.

**Versions checked** — Service `91be101` (2 Jun, 09:00) → `91be102` (4 Jun, 12:00). Client `73ad202` (5 Jun, 08:00) was the observed tip
of `main` at report time, 5 Jun 2026, 10:00 UTC. The client revision is newer than the service change. **Coverage** — All registrations
and client call sites in the configured scope were checked in this product example. **Changed fields: Not checked — this service has no
committed field schema.**

1 operation · 1 client call shown · 0 with possible matches only · known requests without a matching registration, 0 before and 1 after ·
0 calls with an unknown target.

| What changed | Before | After | Service code |
|---|---|---|---|
| GET /lunar-folios registration removed | GET and POST | POST | Before · GET registration at `press/routes.ts:27`. After · remaining POST registration at `press/routes.ts:28` |

| Client call and function | Request as written → path checked | Match before | Match after | What to check |
|---|---|---|---|---|
| `desk/list.ts:18` · `openFolios` | GET `/edge/lunar-folios` → `/lunar-folios`, `desk-prefix` at `73ad202` | Matches GET /lunar-folios | No GET registration matches. The path still lists POST. | This code reads the list. Check how long the old client needs GET. |

### Service change, with a schema

**GET /ink-cards · Field declaration changed.** The committed JSON schema changed at `81cc431` → `81cc432`. **1 changed field** among the
named properties read in JSON request and successful response bodies. Compared type, format, required in its parent, listed values and
list item type. Parameters, error responses and other constraints were outside this check.

| What changed | Before | After | Service code |
|---|---|---|---|
| Response body (200, success) › inkCount · Type changed | Whole number | Text | Schema entry for this operation at `spec/press.json:74` before and `spec/press.json:77` after. Handler named in a comment at `press/cards.go:41`. |

| Client call and function | Request as written → path checked | Match before | Match after | What to check |
|---|---|---|---|---|
| `desk/cards.ts:22` · `loadCards` | GET `/ink-cards` → `/ink-cards`, unchanged by `desk-prefix` at `73ad202` | Matches GET /ink-cards | Matches GET /ink-cards | Check whether the client does arithmetic with inkCount. **Client use of this field: Not checked — only the operation was matched.** |

### Client change, checked against the service

**Client change checked against service code · Live review.** Chartdesk `73ad211` → `73ad212` adds a submission call. Service `91be111`
was the observed tip of `main` at report time, 4 Jul 2026, 12:00 UTC. **Review first** — The new call requests POST. The checked
registrations for this path list only GET. Confirm when the service will accept submissions.

| What changed | Before | After | Service code |
|---|---|---|---|
| Client call added | No call at this site | POST /star-proofs | Checked GET registration at `press/routes.ts:31` |

| Client call and function | Request as written → path checked | Match before | Match after | What to check |
|---|---|---|---|---|
| After · `desk/submit.ts:14` · `sendProof` | POST `/edge/star-proofs` → `/star-proofs`, `desk-prefix` at `73ad212` | No call at this site | No matching POST registration. GET is registered at this path. | Confirm the intended method. If POST is required, provide it before enabling this call. |

When the service subsequently adds POST, its own report can say **Existing client call gains a matching registration** and link this same
call, fixed at `73ad212`. If the service adds POST first, its review can list the addition with zero observed matches and the search
limits. The later client review can show a match to POST. For retirement in the opposite order, a client removal lists the other observed
client calls to the operation before the service is retired. Each review uses its own cutoff.

### Both sides in one change

One change replaces GET with POST at `/proof-batches` and updates the only client call. With complete registrations and mapping, the
combined report shows the following comparisons.

| Compared versions | What the reviewer sees |
|---|---|
| Before both edits | Before client requests GET. Before service registers GET. |
| Both edits included | After client requests POST. After service registers POST. |
| If the service ships first | Before client still requests GET. After service has no GET registration here. |
| If the client ships first | After client requests POST. Before service has no POST registration here. |

**Review first** — Both edited versions agree on POST. Either side shipping alone leaves one request without a matching registration.
Check whether they deploy together or need an interval supporting both methods. If both edits instead delete a route and its only call,
show the after client deletion in the service section and link to it in the client section. Direct any rollout fix to the version
combination that needs it.

## From evidence to review wording

Use the key **We compare the HTTP method and the path after applying the address mapping shown here. Among paths that fit, the check
prefers those with more matching fixed or variable parts.** Preserve the grade, operation and reason for each target on each side. A match
identifies a declaration in the checked code. Report runtime use only with separate evidence.

| Evidence | Cell or review wording and action |
|---|---|
| One highest-ranked match under §5.4, with no client variable forced to one literal value | **Matches {method path} in {source}.** For partial inputs say **Best match among declarations read: {method path}**. Only a substantiated `bound` result can notify under §5.5. |
| Tied best matches or a client variable facing a literal | **Possible match — several declarations fit: {list}** or **Possible match — the value in this address would have to be {value}.** Check that value before assigning the work. |
| Another pattern could receive the call | Keep the §5.4 top grade. List a lower-ranked overlap beside the changed route as **Could also reach {path} when {condition}. Check which route takes priority.** If overlaps were not checked, say so next to every apparent zero. A call that falls through to a different handler ranks with newly unsupported requests. |
| Known request has no same-method match among the entries read | **No {method} match in {source} read at {revision}.** If mapping, route coverage or precedence could change the answer, append **Not checked — {reason}** for whether the service registers this request. For complete routing evidence use **No matching {method} registration in {scope}**. Ask about a needed version or retention of the old route. |
| Other methods | Show a method-specific finding only for a **new or retargeted client call**, or for a fixed call whose method was removed while another was **added by this change** at the same path. Require a checked absence for the requested method, verified mapping and an unambiguous same-path alternate graded `bound`. Name the alternate and source. A possible alternate gets **{method path} might fit if {reason}**. Multiple alternatives keep their own grades and reasons. An existing POST beside a removed GET is a note on the lost GET. It gives no reason to switch methods. |
| Declaration with method `*` | At a verified matching path use **This registration accepts every method here, including {method}.** With a declaration only, say **This entry lists every method.** If its path, authority or priority is uncertain, mark the effect **Not checked** with that reason. A wildcard cannot be an alternative-method suggestion or support a missing-method claim. |
| Method or address unreadable | **Target unknown** means the method or address could not be read. Say **The address is built while the program runs**, **The address depends on a condition**, **The first argument does not give a text address**, **The method is chosen while the program runs**, **This client function has no configured HTTP method**, or **This form of request could not be read** at the call. |
| Site absent or unavailable | **No call at this site** requires evidence of absence. Otherwise **Not checked — {reason}**. An import failure reads **Not checked — no import could be linked to the configured client. Uses of this client could have been missed.** Name a missing source directory or configuration when that blocked the scan. |
| Zero matches to an added or removed operation | Report **No top-ranked client match among the {n} calls read at {revision}** for the after or before operation respectively. Beside it give **{k} known requests had no match** and **{u} calls had an unknown target**, with links. List known requests whose path ends with the operation's path on segment boundaries as **Possible match under another prefix — the leading part of the address differs**. Give the original URL and mapping. Include lower-ranked overlaps or **Not checked — only the highest-ranked matches were retained**. An addition prompts for its intended client. A removal prompts for other users. A claim of no users in the checked sources requires all relevant targets, mappings and overlaps to be resolved. |
| Remaining inputs disagree or a reason is unfamiliar | Point to each declaration. Say, for example, **Two declarations reduce to {method path}. Which one applies is not checked.** For an untranslated reason use **Not checked — the reader could not interpret {construct} at {location}. Inspect this source.** Preserve its original diagnostic in JSON. |

**Saved matches.** The product can reuse a source location previously matched to an operation or field. Label the saved schema revision
and the comparison basis in that row, **before service**, **after service** or **fixed service**. Compare the properties saved with that
location against the same element in that basis. Use **The saved type and other measured properties agree with {basis}** or **The saved
properties differ at {basis}: {old} → {new}. Recheck this location.** For an operation compare its collection of recorded field
properties. For a whole-schema mismatch say **This location was saved using schema {S}. Its list of operations and measured field
properties differs from {both service revisions / the fixed service revision}.** State the comparison for this element separately.

When no location was saved, look in the actual schema used by that scan. Use **No match was saved for this element** if it was present,
**The schema used by this scan did not list this element** for established absence, or **Not checked — the schema used by this scan is
unavailable or incomplete**. Dates do not establish presence. A saved comparison never replaces matching the request against the selected
service.

## Operation and client changes

| What changed | Evidence and review action |
|---|---|
| **Operation added / Operation removed** | Compare the named authority's before and after sets with the coverage rule above. Show calls against both versions. A previously unmatched call that now matches gains a declaration. A call that loses its registration needs a rollout check. With only possible users, show why each might use it. |
| **Route declaration changed / Handler code changed** | For a retained method/path compare the entry text, validation declaration, handler reference and, when identified, handler source at both revisions. Link the changed part. A line-number shift alone is no source edit. **Check whether this edit changes what requests the handler accepts or returns.** The row is informational until there is evidence of an interface change. |
| **Operation fields changed** | A retained method/path has at least one measured field edit. Show each changed declaration once under the operation. Fields belonging to a whole-operation addition/removal are details of that operation. |
| **Client call added / Client call removed / Client target changed** | A proved insertion, deletion or corresponding edit changes what this client requests. Show the after match for a new call, before use and any remaining after uses for a deletion, and both targets for a retarget. Ask which service version the client needs or which users still need the old method. A call may also change target through a configuration edit. |
| **Client target needs confirmation** | A possible match or a switch to another matching operation needs its runtime value or routing priority checked. Leave both targets visible. A difference already present at both versions belongs with standing differences unless it explains a change row. |

A moved path or changed HTTP method yields independently actionable removal and addition rows. Display same-path method changes next to
one another and group repeated prefix moves by shared suffix for reading. These groups imply no replacement relationship. A producer type
renamed with identical measured body properties adds no field row.

**Identity across client versions.** Within one product input identify a call by repository, revision, file/blob and the call expression's
byte span. Two calls on one line have distinct spans. Use a before/after diff to link a one-to-one corresponding file and enclosing
function and then the expression. Accept an unchanged span that maps uniquely or a single method/address edit at the same expression
position inside uniquely mapped surrounding code. Record the two spans and the mapping evidence. Copies, ambiguous moves, repeated
expressions and a shared name or line number do not establish a link.

Use **Client target changed** and one shared count only for that proved pair. Independently proved insertions and deletions get separate
rows. Otherwise keep **Observed before** and **Observed after** with their own locations and counts and **Not checked — could not tell
which call corresponds to this one**. A before/after pair may appear under two operations. The fixed client of a service change reuses the
very same inventory row.

## Fields on a service with a schema

Beside the count print **Named fields in JSON request bodies and successful JSON response bodies. Compared type, format, required in its
parent, listed values and list item type.** Name the actual message and media type read. Path, query, header, cookie and form parameters,
non-success and default responses, other media types, and constraints such as numeric bounds are outside this count. A fallback to another
media type gets **Not checked — this body is {media}** for JSON coverage. Scalar bodies and individual scalar map/list values have no
independent named-field row. A named list's item type is measured.

Print the stops for the affected operation and body path. For example, **This field refers back to an earlier type. Deeper fields were not
checked**, **The field list stops at this depth**, **The referenced definition could not be read**, or **Several different shapes can
occur here. Which fields are required together is not checked.** Overlapping definitions, ambiguous field paths and incomplete
media/status data qualify the corresponding rows and count. If related source code changes while the committed generated schema bytes stay
identical, print **The code changed. The checked-in schema file did not change. Whether it describes this edit is unknown.**

| Field row wording | What it means and what to check |
|---|---|
| **Field declaration added / removed** | Give request or response, status when relevant, full path and before/after declarations. For inputs check values clients already send, including previously allowed extra properties. For outputs check missing fields and decoders that reject unfamiliar fields. |
| **Now listed as required in its parent / No longer listed as required in its parent** | Compare the required flag in both directions, even when this is the only change. A required child of an optional object is required *when that object is present*, subject to its applicable schema branch. Check omitted inputs or code that expects an output unconditionally. |
| **Type changed / Format changed / List item type changed** | Give both values. Check what clients send, how they decode results and whether calculations still work. A format is an additional description attached to the type. Its enforcement can vary. |
| **Listed values added / Listed values removed** | Show both differences on the same row, e.g. **drops matte and adds satin**. Distinguish **No list of allowed values is declared here** from **Not checked — the scan could not distinguish an omitted list from an empty or unreadable one** when the index conflates them. Check old inputs and how clients handle new outputs. |

Report structural differences even when their compatibility effect is unknown. Adding an optional whole-number field can reject text
previously allowed under that name. An enum can both gain and lose values. The five measured properties alone give no verdict that more or
fewer messages are accepted, or that a response field will always be present. If the client links only to the operation, print **Client
field use: Not checked — the call was matched by method and path** beside its fields and on client reviews with no call edit.

| Where a term appears | Render it in that cell |
|---|---|
| Types | `string` → **text**, `integer` → **whole number**, `number` → **number (may have a fractional part)**, `boolean` → **true or false**, `array` → **list**, `object` → **group of named values**, `null` → **explicitly no value**. Spell out each member of a union. For an unknown type quote the declaration and say its meaning was not checked. |
| Formats | `date` → **calendar date**, `date-time` → **date and time**, `int32` / `int64` → **32-bit / 64-bit integer**. Keep the literal alongside its meaning. For an opaque format use **Format changed from {old} to {new}. Not checked — the schema gives no meaning the reader recognizes for {value}.** |
| Body paths | **Request body** is data sent by the client. **Response body (201)** is data returned with status 201. **2XX** means any success status from 200 through 299. Use **each list item** and **each map value (the value under each key)** at the applicable path segment. |

**Successful response status changed.** Keep §5.1's status-bearing field IDs. Compare status keys separately at each retained operation
and media type, including bodies with no named fields. With complete status lists, readable bodies and exactly one outgoing and one
incoming comparable success root, show **Successful response status changed, 200 → 201** once. Compare their measured fields by relative
body path and list only actual shape or membership differences. Ask whether clients check for 200. Keep both root IDs in JSON. If several
pairings fit, or a status range overlaps, list the observed statuses, mark the pairing **Not checked** with its reason and keep both trees
as details. A change of root alone never counts as a removal and addition of every child.

## Empty and long reports

Keep **Review first**, **Versions checked** and **Coverage** even when the tables are empty. For a complete methods/paths-only comparison
say **No method or path was added, removed or changed in the checked declarations.** For a complete comparison of the five schema
properties say **No difference in the measured field declarations.** For a complete, empty client comparison say **No client calls were
added, removed or retargeted in the checked source.** A retained declaration edit, handler edit or entry observed on only one side gets a
row even when the confirmed change count is zero or unknown. If the comparable inventories agree but coverage is partial, say **No
difference among the entries read. Additional changes: Not checked — {gap}.** Unexamined behavior and client field use get their own
**Not checked** reasons. With no usable input say **Not checked — {reason}**. An unavailable reference still permits independent facts
on the changing side.

Order new unsupported requests and calls switching destinations first, then field edits, possible targets and informational changes. Show
a blocking evidence gap before the affected claim. Within a priority sort by path/method, message/field path and client file/span. Preview
five operation groups, three calls and three fields per group, three callers per call, five limits and five change-linked standing
differences. Reserve the operation, call or field, source and limit supporting **Review first** before filling the other slots. Only
standing differences linked to a change appear in the preview.

Group identical edits from one shared schema definition once and say **Same change in {n} more operations**, retaining each operation,
client and canonical field ID in the full tables. For *each* shortened group print **Showing {shown} of {total} · See all {kind}** with a
direct, stable anchor to that group's complete table in the same Markdown appendix. Include all limits, callers and standing differences
there, including for a single `--format md` output stream.

Count operations by API/method/path and fields by owning operation and field, after status grouping. Count each observed client call once
across groups and caller paths. **Possible matches only** counts a call with at least one possible match to a reviewed operation, no
unique match to one on any applicable side, and all applicable sides assessed. A bound-before/possible-after call shows both and stays out
of that count. Incomplete sides get a separate unknown count. Report before and after counts separately if correspondence is unknown. For
a fixed client give known unmatched counts against each service version and the unknown-target count for the fixed inventory. For a client
change give both counts per client version. Counts of visible records are measurable even when the population total is unknown.

## First cut for the prototype

This is the rendering contract for the first eval prototype. Its input builder reads the selected snapshots for the requested change,
including when the study supplies the IDs of five past changes. The existing replay result contains a filtered list of changes and
old-side predictions. Build the review input before opening that result.

| Part | Render from the prototype's inputs | Product-only evidence or wording |
|---|---|---|
| Versions and both directions | Render the versions block, coverage, action, service and client tables, limits and full appendix. For a service change, read both operation sets and scan the chosen client once. For a client change, run the same client inventory at base **and** head against one chosen service revision. For a mixed change reuse those inventories for all four comparisons in one report. A dated replay gets the version-comparison title unless cutoff evidence was supplied. | Automatic collection of branch-tip records and verified merge results. With no cutoff record or usable merge tree, print **Not checked** and say which comparison it prevents. |
| Service operations and sites | Diff the method/path sets in both directions. Show **Listed only before** or **Listed only after** in the *entries read*, and link the route or annotation sites at their own revisions. Show a handler only if the input actually identifies it. Show the observed calls that lose or gain a match among these entries. | Certified additions/removals, complete registration and gateway coverage, and retained-entry, validation and handler-source comparisons. For a retained method/path print **Declaration or handler edits: Not checked — only the method and path were compared.** |
| Extraction limits | There is **no per-file or per-entry extraction status**. For example, show **GET /lunar-folios listed before at press/routes.ts:27. Removal: Not checked — the after scan did not record which entries it could read.** Apply the same rule to an apparent addition, field disappearance, client deletion or zero. Report counts of observations under that label. Keep unverified changes out of confirmed counts. | A complete source inventory and the per-file/per-entry records described below. Missing extraction evidence cannot be supplied by a later restoration or a clean exit. |
| Schema fields | Only the public schema pair has field inputs. Compare the two pinned JSON field inventories directly, including required → optional and both sets of listed values. Show observed values/paths, the five-property coverage caption, available limits and the operation's schema location when known. For a status-only shift show the observed statuses together and align comparable body paths. If the status keys cannot be verified on both sides, print **Status change: Not checked — the full response status lists were not recorded**. The code-declared pair prints **Changed fields: Not checked — no committed field schema.** | Confirmed field presence/absence and response roots, exact declaration lines, saved-schema comparisons and field uses where a product pass identifies a particular read or send. Use **Not checked** with the missing source or pass for each relevant claim. |
| Client calls and identity | Keep every observed `bound`, `candidate` and `unbound` record with method, original address, mapped pattern when known, reason, file, line and enclosing function. Re-run §5.4 matching in the impact step. Number occurrences in source order within a frozen inventory so two calls on one line retain separate rows. In a client diff show observations on changed lines and changes in matches/targets across the full inventories. Link revisions only for a uniquely mapped unchanged source line containing the complete, single call within the same uniquely mapped function. Otherwise show each side's observation and **Corresponding call: Not checked — the scan has no expression spans**. | Expression spans and proved correspondence for edited calls. A cross-revision total for ambiguous rows reads **Not checked**. Keep their per-revision counts. |
| Matches and differences | Show the highest-ranked operation(s) with their grades and reasons in plain words, including former misses. Show known unmatched requests, unknown targets and possible alternate prefixes beside an apparent zero. Report a wildcard as “every method” at its declaration. Name each actual gap, e.g. **Not checked — this scan did not check overlapping routes**. Other methods may be listed as declarations. Advice to change methods is outside this cut. | Lower-ranked overlap and dispatch checks, verified prefix/registration evidence for negatives and graded alternate-method advice. Declaration-only results cannot establish what a deployed service accepts. |
| Callers and evaluation | At the **checked client revision**, list direct uses of the enclosing function found by a **one-level name search** in its file and identified importers. Say **No direct call found by this name in the searched files** when it ran with no hits, or **Not checked — {no function name / search did not run here}**. Attach the separate evaluation only for observations the study actually followed. | Resolved caller paths at one and two levels. For a second level in the first cut print **Not checked — the search followed direct names only**. |

The prototype can show both sides of an edited client line against the same service even when it cannot prove the calls correspond.

| Client version and call | Request as written → path checked | Match to service declarations at `91be111` |
|---|---|---|
| Before · `73ad211` · `desk/queue.ts:12` · `openQueue` | GET `/proof-queue` → `/proof-queue` | Best match among declarations read · GET /proof-queue |
| After · `73ad212` · `desk/queue.ts:13` · `openQueue` | POST `/proof-queue` → `/proof-queue` | No POST match in the declarations read. Registration: **Not checked — the scan did not record routing coverage.** |

**Corresponding call: Not checked — the edited line cannot be linked across revisions.** Inspect the two source lines before treating this
as a switch to POST. The `queue-prefix` mapping in each named client revision leaves this address unchanged.

## Replay evaluation (uses later history)

Build and serialize the entire review from the frozen inputs first. Join a separate companion *to* its report, element and unambiguous
call occurrence. The companion can supply an endpoint, followed lines, edits, release evidence, controls and reader judgments. It cannot
select live rows. An ambiguous historical line link gets **Not checked — more than one call shares this line**. A client-change report
needs no invented follow-up result.

Read commit ID, date and subject directly from the selected commits. The replay's `History.detail` also consults later containing tags.
Its `usable` list excludes later reversions and restorations. Neither that object nor a stripped `ReplayResult` is a review input.
Bind source links and source bytes to the named trees. Exclude dirty or untracked substitutions. For live and historical reviews, use
configuration and mappings from the admitted snapshots or with independent evidence of availability at the cutoff. A hash of today's
configuration alone leaves the mapping **Not checked**. Verify any external change list against the fresh, cutoff-scoped diff. Determine
inclusion from the snapshots even for study-selected subjects.

**Renderer acceptance test.** For the same subject and frozen inputs, first withhold all evaluation data. Then change later history, ref
movements, restorations, releases, line searches, judgments, controls and study selection. Require byte-identical complete review JSON and
Markdown, including identity, references, links, warnings, standing differences, grades, counts, ordering, preview, appendix and
`unobserved`. The companion may differ. This test applies in both directions and to empty and partial reports.

Give the companion its own heading **Replay evaluation (uses later history)** and the last revision and date examined. The example follows
a call for two days.

| Client line followed | First recorded edit | Time from the service change | Reader's assessment and evidence |
|---|---|---|---|
| `desk/list.ts:18` | `73ad207` · 6 Jun 2026 · Reindent list call | 2 days | **Unrelated edit.** The change adjusted indentation. |

| Study value (JSON only) | Words and evidence in the companion |
|---|---|
| `same_commit`, `within_14_days`, `later` | **A followed line changed in the same commit / within 14 days / later.** Name the line, commit, subject and elapsed days. An edit includes deletion or a move history tracking lost. Use dates as observed if clocks disagree. Omit commit-distance jargon. |
| `present_at_pin`, `present_at_head` | **Followed lines remained unchanged through {revision, date}** when tracking establishes that. If the trace is missing, name the missing evidence. |
| `coordinated`, `fix_after_release` | **Reader judged the edits part of the same feature** or **Reader identified a follow-up fix**. Show the feature link, issue-citing fix message or release evidence actually available. Say **same release** or **release before the fix** only with identified releases establishing that relation. |
| `dead_code`, `real_mismatch` | **The reader found no direct call by name in {files} at {revision}**, or **The reader found a source call by name and could not find {element} in {sources} at {revision}**. For a field, identify the property-name occurrence. These searches leave runtime reach and actual field use unknown. |
| `unrelated_edit`, `false_prediction`, `cannot_tell`, missing assessment | **Unrelated edit**, **Reader judged this call unaffected**, **Reader could not determine the effect**, or **No reader assessment recorded**. Include the reader's reason where available. |
| `field_spelled` | For a field only, **Property name found / No such property name found in the searched files**. Name the call's file and files identified as importing its enclosing function by name. A spelling gives the reviewer a place to look. **Actual use of this field: Not checked.** |

Only a defined, ordered release set or release record can identify the first release containing a commit. For a cross-repository release
claim also establish which versions that release contained. An arbitrary containing tag can be labeled **Tag containing this commit**. If
the release set or correspondence is missing, print **Release: Not checked — {reason}**. A time interval alone measures when a line was
touched.

## Input documents and machine-readable form

The §5.2 bindings documents must carry the inputs below. The product impact step validates each document's API, role, repository,
revision/tree, schema provenance and configuration against its selected slot before using it. A wrong revision produces **Not checked —
expected {revision}, received {revision}** for the affected check. Preserve independently admitted evidence.

| Input | Fields needed beyond the original join |
|---|---|
| Each snapshot | `input_id`, `api`, `role`, `repo`, `rev`, `tree`, `committed_at`, `dirty`, `tool`, `schema.source`, `sha256` where applicable, `content_hash`, `config_sources[]` (name, repo, rev, path, hash, mapping/mount), `operation_authority`, `scope`. Record the actual file tree and configuration used. |
| Extraction for each input | `expected_files[]`, `file_status[]` (path, blob, expected/moved/deleted evidence, scope, status, reason), `entries[]` per file (source span or enclosing region, kind, possible keys/scope, status, reason), `dependencies[]`, `unaccounted_scopes[]`. Status distinguishes complete, partial, unreadable, skipped and unknown. Include missing expected files and regions without an element ID. A missing status defaults to unknown. |
| Producer | `operations[]` with original method/path, canonical ID, authority and per-version `sites[]` (kind, span, grade, reason, function, evidence). Keep registration, annotation, handler and schema locations distinct. `entry_digest`, `validation_digest`, `handler_ref`, `handler_digest` support retained edits. `message_roots[]` records operation, status, media and coverage. `fields[]` records ID, definition, path, declaration site when known, before/after-comparable properties and limits. |
| Consumer | `calls[]` includes unbound calls. Each has `site` (file, blob, line, byte span or local occurrence), `enclosing_function`, `function_span`, `requested_method`, `original_url_text`, `mapped_path`, `mapping_source`, `target_reason`. Optional `saved_match` carries `service_input_id`, grade, all candidate IDs/reasons, schema identity, element presence and recorded signature. `callers_status`, `caller_method`, `depth`, scope and `callers[]` give the revision, sites and any intervening functions. |
| Change and cutoff | `subject_role` (service, client, both), `before_input_id` and `after_input_id` per changed role, fixed `reference_input_id` for one-sided changes, `report_time`, `cutoff_at`, `mode`, `branch`, `observed_tip`, `observed_at`, `selection_history_tip`, `availability_evidence`, `availability_status`, `requested_rev`, `selected_by`, `selection_reason`, dates/offset and comparison basis. Keep base/head/target/merge tree IDs and optional before/after file, function and call correspondence with evidence. |

With these documents, **impact** runs §5.4 matching for each pair under **Versions checked**. The product can perform this comparison from
the documents alone. Use the raw requested target of every call, including calls the saved document could not bind. A service change needs
two operation authorities/sources and one fixed client inventory. A client change needs two client inventories and only **one** fixed
service source. A mixed change needs both pairs of snapshots. Schema-file inputs are required only for schema claims. An expected but
missing schema cannot be replaced with an empty schema.

| Live JSON | Contents |
|---|---|
| Envelope | `schema_version`, `report_id`, `api`, `mode` (`live`, `historical_as_of`, `version_comparison`), `report_time`, `cutoff_at`, `inputs`, `reference`, `comparisons[]` (purpose, service/client input IDs). Date-only or explicitly later comparisons cannot serialize as historical as-of or live. |
| `change`, `impacts[]` | `elements[]`, `operation_id`, `change_kinds[]`, `before_presence`, `after_presence` (present/absent/unknown), `before_shape`, `after_shape` (type, format, required in parent, allowed values, item type), `response_root_pairs[]`, `call_changes[]`, `producer_sites[]`, `risks[]` and `review_action` with supporting row IDs. Include declaration-only observations separately from confirmed changes. |
| `usages[]` | Revision-local `site_id`, `before_site`, `after_site`, optional shared `usage_id` and `correspondence_evidence`, `requested_target`, `match_before`, `match_after`. Each match carries service/client input IDs, mapping, status, primary grade/reason, target IDs and `alternates[]`/`overlaps[]` with operation, method, grade and reason. Include `field_use_status` and caller evidence. |
| `binding_status`, `coverage[]`, `warnings`, `standing_mismatches[]` | Saved schema identity, actual element presence, recorded properties and comparison basis. Per-claim input/file/entry references, authority, scope, status, reason code and observed versions support both change and standing rows. Warnings name their affected claims. |
| `summary`, `unobserved` | `operation_count`, `changed_field_count`, `listed_usage_count`, `possible_only_usage_count`, `known_unmatched_count`, `unknown_target_count`, with version, scope, observed count and null plus reason for an unmeasured total. `unobserved` retains §5.2's meaning: a changed element has **no binding in any admitted live binding document**. A producer binding alone keeps it out even when no client matches. |

The separate evaluation JSON carries `live_report_id`, `observation_end`, `element_id`, `usage_id` or revision-local occurrence,
`followed_lines`, `outcome`, `first_touched_line`, `touching_commit`, `lag_days`, `lag_commits`, `release_set`, `change_release`,
`edit_release`, `reader_assessment`, `assessment_evidence` and field-only `field_spelled`. Missing evidence has a reason.

## Questions for the owner

1. Which branch or release line does your team review against? **Default** to the configured integration branch, with an exact release
   revision for a version-specific review.
2. Should an added route with no observed client require a named adopter? **Default** to an informational row that prompts for the
   intended client.
3. How much detail belongs directly on the change? **Default** to five operations and three calls and three fields per operation, with
   every full table one link away.

## Review round 1: disposition

| Finding | Folded in |
|---|---|
| Sol 1 · WRONG | [Versions checked](#versions-checked) requires evidence of availability and gives date-only and later-pin comparisons their own mode. |
| Sol 2 · WRONG | [Both sides in one change](#both-sides-in-one-change) shows the joint result and both possible deployment orders. |
| Sol 3 · WRONG | [Match wording](#from-evidence-to-review-wording) keeps each alternate method's grade and condition. |
| Sol 4 · WRONG | [Match wording](#from-evidence-to-review-wording) puts lower-ranked overlaps or an unchecked-overlap notice beside each apparent zero. |
| Sol 5 · WRONG | [Evidence](#what-counts-as-evidence) and [match wording](#from-evidence-to-review-wording) bind negatives to the authority, original address, mapping and route coverage. |
| Sol 6 · WRONG | [Fields](#fields-on-a-service-with-a-schema) reports declarations, parent conditions and both sides of a mixed value-list edit. |
| Sol 7 · WRONG | [Fields](#fields-on-a-service-with-a-schema) names the five measured properties, both requiredness directions, exclusions and incomplete regions. |
| Sol 8 · WRONG | [Saved matches](#from-evidence-to-review-wording) checks actual schema presence and compares saved properties to the named service side. |
| Opus W1 · WRONG | [Match wording](#from-evidence-to-review-wording) keeps a lost GET distinct from a pre-existing POST at that path. |
| Opus W2 · WRONG | [Evidence](#what-counts-as-evidence) makes every unaccounted file or entry unknown wherever its absence would change the claim. |
| Opus W3 · WRONG | [Versions checked](#versions-checked) separates commit dates from recorded branch arrival. |
| Opus W4 · WRONG | [Both sides in one change](#both-sides-in-one-change) gives a mixed commit one report and scopes rollout advice to the compared pair. |
| Opus W5 · WRONG | [Match wording](#from-evidence-to-review-wording) treats an all-method declaration as covering the requested method where the path is verified. |
| Opus W6 · WRONG | [Fields](#fields-on-a-service-with-a-schema) compares success roots before counting field changes. |
| Sol 9 · SMELL | [Replay evaluation](#replay-evaluation-uses-later-history) builds the review before study selection can filter its rows and specifies the byte-identity test. |
| Sol 10 · SMELL | [First cut](#first-cut-for-the-prototype) and [input documents](#input-documents-and-machine-readable-form) specify both inventories, source spans, correspondence, caller evidence and notification limits. |
| Sol 11 · SMELL | [Match wording](#from-evidence-to-review-wording), [fields](#fields-on-a-service-with-a-schema) and [counts](#empty-and-long-reports) define labels at use, including mixed grades. |
| Sol 12 · SMELL | [Replay evaluation](#replay-evaluation-uses-later-history) qualifies name searches and requires an identified release set for release claims. |
| Sol 13 · SMELL | [Long reports](#empty-and-long-reports) reserves preview space for the action's evidence and links each shortened table to its appendix. |
| Opus S1 · SMELL | [Versions checked](#versions-checked) uses report time for live reviews, change time for historical ones and the integration result for a behind-base review. |
| Opus S2 · SMELL | [Operation and client changes](#operation-and-client-changes) covers retained declarations and handlers. [Empty reports](#empty-and-long-reports) states the measured scope. |
| Opus S3 · SMELL | [Fields](#fields-on-a-service-with-a-schema) names body/media coverage, unchanged generated schemas and the path where each check stopped. |
| Opus S4 · SMELL | [Match wording](#from-evidence-to-review-wording) places known unmatched requests, possible alternate prefixes and unknown targets beside an apparent zero. |
| Opus S5 · SMELL | [Layout](#layout-choices), [match wording](#from-evidence-to-review-wording), [field terms](#fields-on-a-service-with-a-schema) and [first cut](#first-cut-for-the-prototype) explain code, URL and caller evidence where displayed. |
| Opus S6 · SMELL | [Input documents](#input-documents-and-machine-readable-form) validates revisions and rematches raw calls. [Call identity](#operation-and-client-changes) and [replay evaluation](#replay-evaluation-uses-later-history) settle correspondence and hindsight inputs. |
| Opus S7 · SMELL | [Long reports](#empty-and-long-reports) caps limits, callers and standing rows and groups shared definitions and prefix moves. |
| Opus S8 · SMELL | [Worked reviews](#service-change-operations-only) show an unchecked field count and a JSON schema. [Report layout](#what-appears-on-the-change) gives unknown targets one home and [match wording](#from-evidence-to-review-wording) limits notifications to bound facts. |
