# Lane 5 impact report, design amendment A1

Round 2 fold, 2026-11-24, for owner review. This amends DESIGN §§5.1, 5.2, 5.4, 5.6 and 6.1. In issue #848 the owner asked for plain words
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

Print the mode, change subject, repositories, selected trees and why each was chosen. For each commit print ID and UTC committer date.
For the counterpart print **Checked revision is {duration} earlier/later than the change** or **Both commits have the same time**, using the subject commit's date. Across repositories deployment order is unknown.

| Mode on the report | Cutoff and selection |
|---|---|
| **Live review** | Cut off at **Report time**. Capture the configured counterpart integration branch tip at that instant, or use a record of ref updates through that instant. Freeze its ID. A client added after the service commit but present at report time belongs in this review. |
| **Historical review at the change** | Cut off at **Change time**, the subject head commit’s UTC committer date, including for a range or a computed merge tree. Establish the tip at that cutoff from a snapshot or a ref-update history that covers the cutoff. For a supplied pin, a contemporaneous review record can establish its availability then. Show the record and time. Tree membership in a named commit establishes its contents. A commit date alone cannot establish branch arrival. |
| **Comparison of selected versions** | Freeze the named trees and supplied inputs at **Selection time**. Show who supplied a release pin, or the rule and frozen history tip used to reconstruct it. For date-only selection use the last commit along that tip's main line of history with a committer date at or before change time. Print **Selected by commit date. When this revision reached {branch} is unknown.** An explicitly newer pin reads **This comparison uses code from after the change.** |

**Input admission.** Admit an input only when it is **(a)** a file read from a named tree of the admitted snapshots, or **(b)** a record with independent evidence that it existed at the cutoff. Bind each snapshot identity to its mode’s cutoff evidence. Read its tree blobs directly, excluding dirty or untracked substitutes. Check every dependency by the same rule. Compute diffs, inventories and matches afresh from those inputs. For anything else, every dependent claim prints **Not checked — {required record} has no evidence of availability at {cutoff}**. An excluded input cannot select the report, supply an ID or link, change a count, or supply the text of its own warning.

| Prototype input inventory (closed) | Origin and use |
|---|---|
| Snapshot files (a) | The named service trees supply route-table and handler source, Go annotations and any registration source examined, and the committed schema and local definitions where present. Client trees supply source calls, wrappers, imports, enclosing functions and the files searched for direct callers. Record tree, path and blob for every read. |
| Cutoff records (b) | Subject/range and roles identified by a cutoff record, commit/parent/tree metadata, and any supplied observed ref, review pin or target/head/merge receipt. A later query may name the subject to look up. The subject and trees must independently qualify at the requested cutoff. Commit objects supply subject, parents and dates. The replay supplies no independent historical branch-tip log, historical merge receipt, saved binding or saved schema inventory. A commit timestamp alone proves none of these. |
| **Study-supplied** | The client configuration file(s) supply aliases, client file/export, method/call rules, URL and method argument positions, strip/add prefixes, response root and hook/fetcher entries. The route spec supplies the registration callee, method/path/handler/validation keys, mount and lookup containers (`handler_under`, `validation_under`). The study invocation supplies the consumer root. Driver defaults include `DEFAULT_SCHEMA`, the era boundary commits in `select_config` and the primary-client rule in `replay_one` (`request.ts` when configured). |
| Producer media for omitted Swagger keys | The builder loads numbered entries from the catalog named in the comparison manifest at its pinned study/config tree, path and blob. Admit this file like client configuration, including its own cutoff evidence in historical mode. The builder may also derive entries solely from admitted source and dependencies at the producer revision under DESIGN §5.1. A catalog assembled later from old files still needs its own cutoff evidence if used as an input. |

**Default for a prototype using study inputs: Comparison of selected versions.** Pin the study files and driver in a separately named study-input tree (a) for that comparison, or capture them with independent records (b) at its selection time. Under **Versions checked** print **Client configuration: {file} from {study tree or receipt}, supplied by the study** and **Route specification: {file} from {study tree or receipt}, supplied by the study** where used. Print the driver revision and how it chose the era from the named revision’s own ancestry, the consumer root, primary-client rule and schema path. Identify a mapping by file and numbered client entry. A client or service checkout cannot establish the origin of a study file. A historical or live request keeps its requested mode and marks dependencies of unadmitted configuration unchecked. A selected-version comparison has its own identity and makes no claim of availability at change time.

A release-specific or deliberately later pin belongs in a selected-version comparison. An observed live tip can postdate the change. When an as-of request lacks a ref record, keep the known subject facts and print **Not checked — no record of the tip of {branch} at {time}**. A date-based fallback needs a separate comparison. Equal dates, backdated commits and a ref moved today cannot establish earlier availability. For an unreadable admitted pin, name the revision and read failure.

For a single commit the default before revision is its first parent. For a range print the supplied base and head. Both **Before** and
**After** compare the changing side with the *same* named counterpart. The before column of a range makes no assertion about what the
counterpart contained on the base date.

| Direction | Before and after inputs to the impact step |
|---|---|
| **Service change checked against client code** | Compare the service base and head. Match the entire fixed client inventory against each service operation set, including calls with no previous match. |
| **Client change checked against service code** | Compare the two client inventories. Match both against the same service declarations, locations and schema where available. |
| **Service and client changed together** | One report, one versions block, service section first and client section second. Compare service-before/client-before and service-after/client-after. Also check each side shipping alone as shown below. |

**Behind-target pull request, in either repository setup.** In the changing repository compare **target → merge result**. Record proposed base, proposed head and target commit IDs, subjects and dates separately. In one repository read the unchanged role at target, or both roles at target and result for a joint edit. In two repositories hold the independently selected counterpart tree unchanged. Use the target and proposed inputs observed at the live or historical cutoff.

Admit an immutable merge commit with cutoff evidence and use its own metadata and links. For a clean computed result, bind base, target, head, merge procedure and resulting tree ID in a cutoff receipt (for historical use, require independent evidence of that exact result at the cutoff). Print **After: computed merge tree {tree} (no commit or commit date)**. Keep computation time separate. At each site link the exact tree that supplied it. For a computed result link only the exact result blob at that path. If the host cannot serve it, print tree ID, blob ID, relative path, line range and the excerpt from that blob. Never borrow a topic-head date, subject or link for the result.
On conflict, absent receipt or unavailable tree, show the observed proposal separately and **Combined result: Not checked — {specific missing record or conflict}**. Determine changed roles from the trees including configuration. An unreadable role keeps its own section and reason.

## What counts as evidence

**Rule for every cell.** Print a sentence only if its required evidence record exists in the frozen input. Otherwise that cell reads **Not checked — {missing record and what it would establish}**. This applies to headings with claims, actions, warnings, links, counts and empty states as well as table rows. The first-cut table below identifies the records.

A claim may use absence, uniqueness, sameness or a total only when *every source region that could
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
unknown on either side. Remove each required evidence record in the first-cut table in turn. Every dependent change or zero must render **Not checked** with its reason and stay out of confirmed counts.

Name the **Source of operations** beside the result. A route table lists declarations, an annotation documents a handler, a registration
connects a path and method to code, and the committed schema describes fields. Show each source separately when they disagree. For
example, **The schema lists GET here. The registration at {location} lists POST. What handles GET is not checked.** A result about
declarations stays scoped to declarations. A negative about registered routes also needs complete registrations, applicable method/path
forms, mounted prefixes and verified address mappings through the named routing scope.

## What appears on the change

1. **Review first** states one evidenced result and one action. Surface a gap that prevents a decision. Give counts for operations,
   changed fields, client calls shown, calls with possible matches only, requests whose method and address were read but matched no declaration, and calls
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
   to check**. Show the client's own URL text, HTTP method, mapped service path and the mapping's source file, revision and entry, including an
   explicitly unchanged path. Attach each cell to its service and client versions. A changed call with an unknown method or address has
   one home, **Changed client calls with an unknown target**. Link to that row from limits. Requests with a readable method and address that matched no declaration keep their own method/path heading.
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
of `main` at report time, 5 Jun 2026, 10:00 UTC. The checked client revision is 20 hours later than the service change. **Coverage** — All registrations
under `press/` and production client calls under `desk/` were checked, including the mount and address mapping. **Changed fields: Not checked — this service has no
committed field schema.**

1 operation · 1 client call shown · 0 with possible matches only · requests whose method and address were read but matched no registration, 0 before and 1 after ·
0 calls with an unknown target.

| What changed | Before | After | Service code |
|---|---|---|---|
| GET /lunar-folios registration removed | GET and POST | POST | Before · GET registration at `press/routes.ts:27`. After · remaining POST registration at `press/routes.ts:28` |

| Client call and function | Request as written → path checked | Match before | Match after | What to check |
|---|---|---|---|---|
| `desk/list.ts:18` · `openFolios` | GET `/edge/lunar-folios` → `/lunar-folios`, `desk/map.toml` entry 1 at `73ad202` | Matches GET /lunar-folios | No GET registration matches. The path still lists POST. | This code reads the list. Check how long the old client needs GET. |

### Service change, with a schema

**GET /ink-cards · Field declaration changed.** The committed JSON schema changed at `81cc431` → `81cc432`. **1 changed field** among the
named properties read. Both schema versions declare `application/json` in document-level `consumes` and `produces`. This operation inherits both. Compared type, format, required in its parent, listed values and list item type in JSON request and successful response bodies. Parameters, error responses and other constraints were outside this check.

| What changed | Before | After | Service code |
|---|---|---|---|
| Response body (200, success) › inkCount · Type changed | Whole number | Text | Schema entry for this operation at `spec/press.json:74` before and `spec/press.json:77` after. Handler named in a comment at `press/cards.go:41`. |

| Client call and function | Request as written → path checked | Match before | Match after | What to check |
|---|---|---|---|---|
| `desk/cards.ts:22` · `loadCards` | GET `/ink-cards` → `/ink-cards`, unchanged by `desk/map.toml` entry 1 at `73ad202` | Matches GET /ink-cards | Matches GET /ink-cards | Check whether the client does arithmetic with inkCount. **Client field use: Not checked — the call was matched by method and path.** |

### Client change, checked against the service

**Client change checked against service code · Live review.** Chartdesk `73ad211` (2 Jul, 09:00) → `73ad212` (4 Jul, 10:00) adds a call. Service `91be111` (3 Jul, 10:00) was the observed tip of `main` at report time, 4 Jul 2026, 12:00 UTC. The checked service revision is one day earlier than the change.
**Coverage** — Checked the insertion in `desk/`, every registration under `press/`, and the mount and address mapping for this call. **Review first** — The new call requests POST. Only GET is registered here. Confirm the intended method before enabling this call.

| What changed | Before | After | Service code |
|---|---|---|---|
| Client call added | No call at this site | POST /star-proofs | Checked GET registration at `press/routes.ts:31` |

| Client call and function | Request as written → path checked | Match before | Match after | What to check |
|---|---|---|---|---|
| After · `desk/submit.ts:14` · `sendProof` | POST `/edge/star-proofs` → `/star-proofs`, `desk/map.toml` entry 1 at `73ad212` | No call at this site | No matching POST registration. GET is registered at this path. | Confirm the intended method. If POST is required, provide it before enabling this call. |

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
| One highest-ranked match under §5.4, with no client variable forced to one literal value | **Matches {method path} in {source}.** For partial inputs say **Closest match among the declarations read: {method path}**. Only a substantiated `bound` result can notify under §5.5. |
| Tied best matches or a client variable facing a literal | **Possible match — several declarations fit: {list}** or **Possible match — the value in this address would have to be {value}.** Check that value before assigning the work. |
| Another pattern could receive the call | Keep the §5.4 top grade. List a lower-ranked overlap beside the changed route as **Could also reach {path} when {condition}. Check which route takes priority.** If overlaps were not checked, say so next to every apparent zero. A call that falls through to a different handler ranks with newly unsupported requests. |
| Method and address were read, no same-method match | **No {method} match among {source} declarations read at {revision}.** If mapping, route coverage or precedence could change the answer, append **Registration: Not checked — {reason}**. For complete routing evidence use **No matching {method} registration in {named directories and mounts}**. Ask about a needed version or retention of the old route. |
| Other methods | Show a method-specific finding only for a **new or retargeted client call**, or for a fixed call whose method was removed while another was **added by this change** at the same path. Require a checked absence for the requested method, verified mapping and an unambiguous same-path alternate graded `bound`. Name the alternate and source. A possible alternate gets **{method path} might fit if {reason}**. Multiple alternatives keep their own grades and reasons. An existing POST beside a removed GET is a note on the lost GET. It gives no reason to switch methods. |
| All-method declaration | In the product, `op:* {path}` is eligible for every known request method. Pool exact-method and wildcard declarations before §5.4 path scoring. Equal best scores remain candidates, including an exact/wildcard tie. A unique best with no client variable forced to a literal is bound, subject to coverage. Dispatch priority requires separate evidence. The first cut prints **This declaration lists every method. Method coverage: Not checked — the matcher compares exact methods only.** Where a wildcard could fit, withhold dependent grades, unmatched counts, alternate-method advice and notifications before summarizing. |
| Method or address unreadable | **Target unknown** means the method or address could not be read. Say **The address is built while the program runs**, **The address depends on a condition**, **The first argument does not give a text address**, **The method is chosen while the program runs**, **This client function has no configured HTTP method**, or **This form of request could not be read** at the call. |
| Site absent or unavailable | **No call at this site** requires evidence of absence. Otherwise **Not checked — {reason}**. An import failure reads **Not checked — no import could be linked to the configured client. Uses of this client could have been missed.** Name a missing source directory or configuration when that blocked the scan. |
| Every added or removed operation, including one with confirmed users | Beside the after (addition) or before (removal) row, link **{k} requests whose method and address were read had no match** and **{u} calls had an unknown target** for that comparison. Show these beside one-sided observations too. When no call selected the declaration, say **None of the {n} calls compared had this declaration among its closest matches**. Give possible overlaps or **Not checked — the scan kept only the closest matches**. Ask who needs an addition or still uses a removal. |
| Other-prefix hints on each such row | For each request use any verified mapping. Where differing leading parts remain unexplained, compare method-compatible request and declaration paths by trailing segments. Include a whole-path suffix or even one shared literal final segment after different prefixes, e.g. `/edge/ink-cards` and `/internal/ink-cards`. Print **Possible match under another prefix — both addresses end in {suffix}. Check the leading parts.** Show original URL, path compared, mapping source and call site, even if another call matched exactly. Hints never change a grade, unmatched count or notification. |
| Remaining inputs disagree or a reason is unfamiliar | Point to each declaration. Say, for example, **Two declarations reduce to {method path}. Which one applies is not checked.** For an untranslated reason use **Not checked — the reader could not interpret {construct} at {location}. Inspect this source.** Preserve its original diagnostic in JSON. |

**Saved matches.** Label **Earlier scan {scan ID} of client {tree} used schema {source, tree, hash}**. Admit the binding document, scan provenance, mapping and exact schema dependencies independently at the cutoff before using any saved fact, including in selection, a count or `unobserved`. The prototype rematches fresh. A lost dependency leaves the comparison **Not checked — {record} from the earlier scan lacks cutoff evidence**.
Label the basis **service before the change**, **service after the change** or **service held unchanged at {revision}**. Compare the properties from the earlier scan to the same element in that version. Use **The saved type and other measured properties agree with {basis}** or **The saved
properties differ at {basis}: {old} → {new}. Recheck this location.** For an operation compare its collection of recorded field
properties. For a whole-schema mismatch say **This location was saved using schema {S}. Its list of operations and measured field
properties differs from {named comparison versions}.** State the comparison for this element separately.

When no location was saved, look in the actual schema used by that scan. Use **No match was saved for this element** if it was present,
**The schema used by this scan did not list this element** for established absence, or **Not checked — {missing relevant region or record}**, where relevance follows DESIGN 5.6: only a region or saved-match segment capable of changing this element's result must be covered (an unreadable `/y` does not block a complete `/x`). Dates do not establish presence. A saved comparison never replaces matching the request against the selected
service.

## Operation and client changes

| What changed | Evidence and review action |
|---|---|
| **Operation added / Operation removed** | Compare the named authority's before and after sets with the coverage rule above. Show calls against both versions. A previously unmatched call that now matches gains a declaration. A call that loses its registration needs a rollout check. With only possible users, show why each might use it. |
| **Route declaration changed / Handler code changed** | For a retained method/path compare the entry text, validation declaration, handler reference and, when identified, handler source at both revisions. Link the changed part. A line-number shift alone is no source edit. **Check whether this edit changes what requests the handler accepts or returns.** The row is informational until there is evidence of an interface change. |
| **Operation fields changed** | A retained method/path has at least one measured field edit. Show each changed declaration once under the operation. Fields belonging to a whole-operation addition/removal are details of that operation. |
| **Successful response status added / removed** | Compare the operation's complete sets of declared success keys independently of field edits or media. Link each added or removed key to its own side, including a response with no body. Check clients that branch on that status. |
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

Establish media for each body at its own revision. In Swagger 2 read operation-level `consumes` for requests
and `produces` for responses. Inherit the document list only when the operation omits the key. In OpenAPI 3
read the message's `content` entry. For an applicable `application/json` declaration, cite its location and
selected schema. Only when both Swagger scopes are readable and omit the directional key may an admitted
DESIGN §5.1 producer record establish JSON. Retain the omission beside the selected schema and print
**JSON media from producer record {entry} ({codec file and line or server-codec contract}) at {revision}.
The schema declares no {request/response} media type.**

Apply this rule to both sides of a replay field comparison. When both roots qualify, compare their fields
and print the media source on each side. If either lacks proof, preserve independent status rows and print
the affected side and reason, for example **JSON fields: Not checked — before, no producer record establishes
response media at {revision}**. An empty override, an unreadable list or explicitly non-JSON media cannot
use the omission rule. Print the specific reason, such as **JSON fields: Not checked — this body declares
application/xml** or **JSON fields: Not checked — the request media list could not be read**.

Beside the count for the verified bodies print **Named fields in JSON request bodies and successful JSON response bodies. Compared type, format, required in its
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

**Status and body are separate comparisons.** Emit independent added/removed status rows first. With 200 and bodyless 204 before, and the same 200 alone after, print **Successful response status removed · 204** even with zero changed fields. Preserve each row. When DESIGN §5.1 proves exactly one outgoing and one incoming root with equivalent JSON media, group those rows under **Successful response status changed, 200 → 201** and compare fields by relative path. Retain §5.1's two status-bearing IDs in JSON. If §5.1 blocks pairing (several pairings fit, a status range overlaps, or a body-eligible code moves to or from an added, removed or retained range or `default`, or one side could carry a body that the other lacks), keep each status row and print **Body comparison: Not checked — {the reason recorded under §5.1: several bodies could correspond / the status range overlaps / a body-eligible code moved to or from an added, removed or retained range or default / one side could carry a body that the other lacks, as in a JSON 200 replaced by a bodyless 204}**. A root change alone adds no child-field edits.

## Empty and long reports

Keep **Review first**, **Versions checked** and **Coverage** even when the tables are empty. For a complete methods/paths-only comparison
say **No method or path was added, removed or changed in the checked declarations.** For a complete comparison of the five schema
properties say **No difference in the measured field declarations.** For a complete, empty client comparison say **No client calls were
added, removed or retargeted in the checked source.** A retained declaration edit, handler edit or entry observed on only one side gets a
row even when the confirmed change count is zero or unknown. If the comparable inventories agree but coverage is partial, say **No
difference among the declarations read. Additional changes: Not checked — {gap}.** Unexamined behavior and client field use get their own
**Not checked** reasons. With no usable input say **Not checked — {reason}**. An unavailable reference still permits independent facts
on the changing side.

Order new unsupported requests and calls switching destinations first, then field edits, possible targets and informational changes. Show
a blocking evidence gap before the affected claim. Within a priority sort by path/method, message/field path and client file/span. Preview
five operation groups, three calls and three fields per group, three callers per call, five limits and five change-linked standing
differences. Reserve the operation, call or field, source and limit supporting **Review first** before filling the other slots. Only
standing differences linked to a change appear in the preview.

Group identical edits from one shared schema definition once and say **Same change in {n} more operations**, retaining each operation,
client and canonical field ID in the full tables. For *each* shortened group print **Showing {shown} of {total} · See all {kind}** with a
direct, stable anchor to that group's complete table in the same Markdown appendix. The same rule applies to **every list inside a preview cell**: show at most three entries and link the exact complete list, including ties, alternate methods, lower-ranked overlaps, prefix hints, values/statuses, source links, caller paths, and limits. Reserve the evidence for **Review first** inside those slots. For an incomplete population, count recorded entries and label the total in scope **Not checked**. On a review surface that may truncate, attach the complete live Markdown and target its appendix from the preview.

Count operations by API/method/path and fields by owning operation and field, after status grouping. Count each observed client call once
across groups and caller paths. **Possible matches only** counts a call with at least one possible match to a reviewed operation, no
unique match to one on any applicable side, and all applicable sides assessed. A bound-before/possible-after call shows both and stays out
of that count. Incomplete sides get a separate unknown count. Report before and after counts separately if correspondence is unknown. For
a fixed client give known unmatched counts against each service version and the unknown-target count for the fixed inventory. For a client
change give both counts per client version. Counts of visible records are measurable even when the population total is unknown.

## First cut for the prototype

Build the input for the requested subject before opening any replay result. For a service change scan the selected client once and compare it to both service trees. For a client change scan both client trees against one service. For a joint change build all four pairs. The table distinguishes records the extractors return today (**Available**), records the first-cut builder must add (**Builder**), and claims requiring additional evidence (**Product**). Partial records license only the observed part of a cell.

**Declaration coverage.** Call `producer_route_table.extract()` on each route-table service snapshot. `routes()` drops the unparsed files, substituted-template locations and wildcard diagnostics. Freeze those records and the declaration sites. Inventory expected files from each tree and changed paths from the diff. Record parsed files and every skipped entry with its location and reason: nonliteral/missing method or path, empty or holed method arrays, non-object registration items (including spreads), registration calls without an array argument, unknown spreads/computed keys, and changed code no longer recognizable as a registration. Unexplained changes, moves, renamed callees, symlinks or unreadable bytes leave the affected scope unknown. A missing diagnostics record differs from a recorded clean scan.

For **Declaration removed from {file}**, require the file parsed at both revisions, a complete entry inventory and no skips in that file on either side, the before site and blob diff deleting or replacing those lines, and no remaining matching declaration there. Apply the reverse proof to an addition. Repeated sites or moves retain their individual evidence. The proof concerns that file under the named spec. A system-wide removal needs the full routing and mapping records. A proved whole-file deletion establishes only loss from that file.

Record client parse errors, unreadable imports, symlinks, missing roots/primary client and test-file exclusions against the tree. For Go, distinguish adjacent recognized annotation comments from unattached, malformed or error-region comments. A method/path suffix and handler name alone cannot certify a registration: dynamic group paths, unsupported verbs, missing handlers and unreadable arguments leave gaps. For a schema read the actual blob or record the read failure. Never use `_schema_at`'s empty-schema substitute.

| First-cut cell | Required frozen evidence | Prototype supply | Text when the required record is missing |
|---|---|---|---|
| Versions, direction, cutoff, duration, combined result | Subject/role, selected pair IDs and commit metadata, availability and target/result receipts as applicable | Available commit objects. Builder binds selections and computes commit offsets. Historical receipts need an independent source. | **Not checked — no record of the tip of {branch} at {time}** or **Combined result: Not checked — {missing result or conflict}**. A computed tree has no commit date. |
| Coverage, operation source, configuration | Source authority, roots, file selection, exclusions, input origin, config/driver/spec tree and entry IDs, era selection | Builder freezes the named sources under Versions checked. | **Not checked — {client configuration / route specification / source root} for {revision} is missing.** |
| Before/after operation, source link and action on a declaration | Method and normalized path, named authority, site at its own tree/blob/line. File-level change additionally needs the two inventories, diagnostics and diff above. | Available route and Go annotation sites. Builder adds entry ranges, original text, tree links or exact blob excerpts, and checks scoped changes. | **Listed only before [or after] among the declarations read. Removal [or addition]: Not checked — {file} has no complete {side} scan.** Without any after scan, say **Observed before at {site}. After: Not checked — {file} could not be read**. For a missing link name the unreadable path and tree. |
| Handler, validation, retained edit | Resolved function and site, or comment naming it. Entry/validation/handler edits need comparable source bytes at both versions. | Available handler text, optional resolved site, validation-presence flag and annotation. Builder may compare captured entry text. Product checks separate validation and handler bodies. | **Handler location: Not checked — only the declaration was found** or **Validation or handler edits: Not checked — the two source bodies were not compared.** A presence flag alone cannot establish an edit. |
| Client call, function, request as written and path compared | Tree, file/line, local occurrence, function, method, URL text, derived path, and identity of the mapping actually applied | Available call observations including unbound calls. Builder records the active client spec file, revision and entry at extraction (both hook and delegated rule if applicable), or the built-in rule. | **Mapping: Not checked — the scan did not record which rule produced this path.** Show the observed address. Use **Enclosing function: Not checked — the scan recorded no function name**. If the target is unreadable give the local reason from the match table. |
| Corresponding, inserted or deleted call | One-to-one diff of file, function and complete call plus scan coverage on both sides | Builder numbers occurrences per snapshot. Link only a uniquely mapped, unchanged line with one complete call in the same uniquely mapped function. Product records start/end positions for edited calls. | **Corresponding call: Not checked — the scan recorded line numbers only.** Show before and after separately. A missing deletion proof gives **Removal: Not checked — {source region} could not be compared.** |
| Matches, alternate methods, overlaps, registration and notification | Per-side matched IDs, grades and reasons, rule and authority. Stronger advice needs complete applicable methods, mapping, dispatch and overlap evidence. | Available exact-method best matches/ties from fresh matching. Builder checks for relevant wildcards, duplicate declarations and optional segments before attributing any grade. Product checks alternative methods, lower-ranked overlaps and dispatch. | **Method coverage: Not checked — the matcher compares exact methods only and this declaration lists every method.** Or **Not checked — {two declarations have the same method and path / this path segment may be omitted / the scan kept only the closest matches / no gateway mapping was checked}.** Name the affected fact. Only a supported bound match may notify. |
| Other-prefix hints and unmatched requests | Full observed request inventory, original and compared paths, method, shared trailing segments and per-side match or unknown outcome | Builder compares all observed paths for each added or removed declaration and links matching requests and hints separately. | **Other prefixes: Not checked — {request paths / declaration paths} were unavailable**. **Unmatched requests: Not checked — {specific coverage gap}.** |
| Status and JSON coverage | Complete per-operation response key sets on both sides including bodyless keys. Preserve each body root's media declarations and origins or omissions. Counting JSON requires the applicable declaration or the two readable Swagger omissions and an admitted §5.1 producer record. | Builder reads status, media and root from the raw schema. For producer-established JSON it resolves the root's `producer_media_ref` and prints the producer sentence under Fields. Status rows survive missing or non-JSON bodies. | **Status change: Not checked — {side} response status list is missing.** For unverified media, name the side, revision and specific JSON-fields reason under Fields. |
| Field change, location and schema limits | Two comparable body roots, complete region for presence, both five-property records, expansion stops, schema operation site or exact field site | Available indexed shapes and some diagnostics for the schema pair, and operation-level locations when resolved. Builder compares both directions including required → optional and mixed value edits. Product adds exact field sites. | **Changed fields: Not checked — no committed field schema**, **Field location: Not checked — only the operation was located**, or **Not checked — {field path} stops at {unreadable definition / depth limit / alternative shapes}**. |
| Client field use | A specific read of a response field or send of a request field linked to this call, schema and revision | Product. An operation match identifies the request. | **Client field use: Not checked — the call was matched by method and path.** Without an operation match use **Not checked — no recorded read or send links this call to the field**. |
| Who calls this code | Revision, enclosing function, searched files, search completion, depth and results. A resolved path needs evidence for each call. | Available one-level name search in the same file and identified importers. Builder runs it at the checked client revision and records its scope. | **Not checked — {the function name is missing / the caller search did not run at this revision}.** A completed empty search says **No direct call found by this name in the searched files**. For deeper callers: **Not checked — the search followed direct names only**. |
| Earlier scan comparison and element membership | Admitted binding and scan ID, exact schema dependency and inventory including elements with no binding, saved signature and named comparison side | Product. First cut uses fresh matches. | **Earlier match: Not checked — {the earlier scan’s binding record / the schema used by that scan / its element inventory} is unavailable.** |
| Standing differences, limits and time labels | Comparable source and match records for each pair, affected site/scope, file or entry diagnostics | Builder compares observed declarations and calls. Two observations support timing only with coverage of both sides. | **Seen at {versions}. When this difference began: Not checked — {side} comparison is missing.** For limits name the file, revision, missed construct and affected check. |
| Counts, empty results and unobserved elements | Per-version distinct occurrences, grades, unknowns, membership in admitted binding inventories, coverage of each stated population | Builder counts observed records. A producer binding alone counts for §5.2 membership. | **{n} observed. Total: Not checked — {missing inventory or unresolved comparison}.** A missing membership inventory gives **Not checked — {binding inventory} is incomplete** for claims requiring it. |
| Review first and rollout recommendation | Supported before/after change, affected call, supporting row IDs and compared pair. An observation needs its own site and scan record. | Builder chooses one supported action. With nothing confirmed, lead with the observed fact and the check that would settle it. | **Observed: {operation} was listed before at {site} and was not among the declarations read after. {call} requests it. Removal: Not checked — {missing record}. Confirm the removal, then check whether this client still needs it.** Omit ungrounded clauses. With neither observation nor proof use **Not checked — {missing record}. Check {source}.** |
| Preview, source lists and appendix | Complete ordered row/list IDs and counts, action evidence and resolvable anchors to the full delivered tables | Builder reserves action evidence and applies the same caps to every list. | **Not checked — the complete {list} was unavailable** for an incomplete inventory. Report the number actually observed. |

Group repeated causes for presentation only: the live JSON keeps every limit under its own `limit_id` (counted once per P1c key `(side, scope, pointer, claim, reason)` within a comparison basis), and a grouped line links to every preserved `limit_id`, so two roots with the same reason keep distinct records. Put a short linked **Not checked — {missing record}** in each dependent cell. A fully checked empty inventory uses the empty-report wording above. An observed match never settles a missing declaration proof. For an addition observed only after, ask the reader to confirm it and check the intended client. For a client-only observation, ask the reader to check the service version that call needs.

**Comparison of selected versions** · Service held unchanged at `91be111`. The study supplied client configuration `study/desk.toml` and route spec `study/press-routes.json`, both at study revision `aa4e203`. Both call records identify client entry 1, which leaves their addresses unchanged.

| Client version and call | Request as written → path compared | Match among the declarations read |
|---|---|---|
| Before · `73ad211` · `desk/queue.ts:12` · `openQueue` | GET `/proof-queue` → `/proof-queue` | Closest match · GET /proof-queue |
| After · `73ad212` · `desk/queue.ts:13` · `openQueue` | POST `/proof-queue` → `/proof-queue` | No POST match among the declarations read. Registration: **Not checked — the scan did not record routing coverage.** |

**Corresponding call: Not checked — the scan recorded line numbers only.** Inspect the two source lines before treating this as a switch to POST.

## Replay evaluation (uses later history)

Build and serialize the review before joining any study result. Verify any admitted external change list against the fresh diff. Neither `History.detail` (later containing tags), `usable` (later restorations), nor a stripped `ReplayResult` supplies live rows. The study may request examples by ID. For a given independently admitted review request, selection by the study cannot create, suppress or change its live report. Join later observations only in a separate companion by report, element and unambiguous call occurrence. Otherwise print **Not checked — more than one call shares this line**.

**Admission acceptance test (before freezing).** Fix the query, clock, independent cutoff receipts, bytes in the trees they identify, and analysis version. Re-run subject selection, reference resolution, admission and extraction from raw inputs each time. Withhold, then vary each unadmitted source: moved refs and backdated/equal-time commits, later pins and merge results or metadata, dirty/untracked file substitutes, study configuration/route specs and mappings, later saved bindings and their schema dependencies, external change lists, study subject lists and restoration filters, later release tags/endpoints, followed lines, property searches, controls and adjudications. Include a later binding as the only apparent binding for a changed field. Include an unselected change and a change restored later so the report set itself is checked. Compare which reports are emitted as well as their bytes. A separate positive control changes an admitted declaration and must change its supported row.

**Renderer byte-identity test (after freezing).** Feed identical frozen inputs with the companion withheld and with changed later evidence. In both tests require identical complete live JSON and Markdown, including report identity and inclusion, versions, links, rows, grades, warnings, standing differences, counts, ordering, preview, appendix and `unobserved`. Exercise both directions, a joint edit, and empty and partial reports in live and historical modes. Inspect the delivered live Markdown stream, comment and attachment: each contains only the review, with no companion section, payload, attachment or link. Deliver the companion separately to the study.

Title the companion **Replay evaluation (uses later history)** and give the last revision and date examined. **Lines the study tracked** include the client call site and, for a field, lines spelling the property in that file or in files identified as importing its enclosing function. An edit to any such line can be the first recorded edit. The **study adjudicator** is the model that assessed the later evidence separately from the change review.

| Lines the study tracked | First recorded edit | Time from service change | Study adjudicator’s assessment and evidence |
|---|---|---|---|
| `desk/list.ts:18` | `73ad207` · 6 Jun 2026 · Reindent list call | 2 days | **Unrelated edit.** Only the indentation changed. |

| Study value (JSON only) | Words and evidence in the companion |
|---|---|
| `same_commit`, `within_14_days`, `later` | **A tracked line changed in the same commit / within 14 days / later.** Name the line, commit, subject and elapsed days. Identify deletion or a move where tracking stopped. Show dates as observed if clocks disagree. |
| `present_at_pin`, `present_at_head` | **Tracked lines remained unchanged through {revision, date}** when tracking establishes that. Identify missing tracking evidence. |
| `coordinated`, `fix_after_release` | **The study adjudicator linked the edits to the same feature** or **identified a follow-up fix**. Show the feature, issue-citing fix message or release evidence. A release claim requires a defined ordered release set, and across repositories, the versions it contained. An arbitrary tag gets **Tag containing this commit**. Otherwise **Release: Not checked — {missing release record}**. |
| `dead_code`, `real_mismatch` | **The study adjudicator found no direct call by name in {files} at {revision}** or **found a source call by name and could not find {element} in {sources} at {revision}**. For a field identify the property occurrence. Runtime reach and field use remain unknown. |
| `unrelated_edit`, `false_prediction`, `cannot_tell`, missing assessment | **Unrelated edit**, **The study adjudicator judged this call unaffected**, **The study adjudicator could not determine the effect**, or **No study assessment recorded**. Give the recorded reason. |
| `field_spelled` | **Property name found / No such property name found in the searched files**. Name the files. **Actual field use: Not checked — the search recorded spelling only.** |

## Input documents and machine-readable form

The §5.2 bindings documents must carry the inputs below. The product impact step validates each document's API, role, repository,
revision/tree, schema provenance and configuration against its selected slot before using it. A wrong revision produces **Not checked —
expected {revision}, received {revision}** for the affected check. Preserve independently admitted evidence.

| Input | Fields needed beyond the original join |
|---|---|
| Each snapshot | `input_id`, `api`, `role`, `repo`, `snapshot_kind`, `rev`, `tree`, `committed_at`, `dirty`, `tool`, `schema.source`, `sha256` where applicable, `content_hash`, `operation_authority`, `scope`. For a computed tree, commit ID/date/subject are null. Each admitted input has `origin`, tree/path/blob or an independent cutoff receipt, and dependency IDs. `config_sources[]` identifies each source, revision, entry, mount/mapping, driver defaults and selected era. |
| Extraction for each input | `expected_files[]`, `file_status[]` (path, blob, expected/moved/deleted evidence, scope, status, reason), `entries[]` per file (source span or enclosing region, kind, possible keys/scope, status, reason), `dependencies[]`, `unaccounted_scopes[]`. Status distinguishes complete, partial, unreadable, skipped and unknown. Include missing expected files and regions without an element ID. A missing status defaults to unknown. |
| Producer | `operations[]` with original method/path, canonical ID, authority and per-version `sites[]` (kind, span, grade, reason, function, evidence). Keep registration, annotation, handler and schema locations distinct. `entry_digest`, `validation_digest`, `handler_ref`, `handler_digest` support retained edits. `success_statuses[]` includes bodyless declarations. `message_roots[]` records operation, direction, status, body presence, original media declarations and origins or omissions, effective media, selected schema and coverage. For producer-established JSON, `producer_media_ref` stores the frozen catalog or derived inventory input ID and numbered entry used to render the proof at this revision. `fields[]` records ID, definition, path, site when known, comparable properties and limits. |
| Consumer | `calls[]` includes unbound calls. Each has `site` (file, blob, line, byte span or local occurrence), `enclosing_function`, `function_span`, `requested_method`, `original_url_text`, `mapped_path`, `mapping_source`, `target_reason`. An optional earlier scan document carries its ID, client tree, schema inventory ID, configuration and admission receipts even for a lookup with no saved match. Each `saved_match` carries the service input ID, grade, candidates/reasons and signature. `callers_status`, `caller_method`, `depth`, scope and `callers[]` give revision, sites and intervening functions. A field-use record names the read/send site, call, field and supporting evidence. |
| Change and cutoff | `subject_role` (service, client, both), per-role `before_input_id` and `after_input_id`, `reference_input_id`, `report_time`, `cutoff_at`, `mode`, `branch`, `observed_tip`, `observed_at`, `selection_history_tip`, `availability_evidence`, `availability_status`, `requested_rev`, `selected_by`, `selection_reason`, dates/offset and comparison basis. Bind base/head/target metadata, merge procedure and tree receipt, exact-source link or blob excerpt, and any evidenced call correspondence. |

With these documents, **impact** runs §5.4 matching for each pair under **Versions checked**. The product can perform this comparison from
the documents alone. Use the raw requested target of every call, including calls the saved document could not bind. A service change needs
two operation authorities/sources and one fixed client inventory. A client change needs two client inventories and only **one** fixed
service source. A mixed change needs both pairs of snapshots. Schema-file inputs are required only for schema claims. An expected but
missing schema cannot be replaced with an empty schema.

| Live JSON | Contents |
|---|---|
| Envelope | `schema_version`, `report_id`, `api`, `mode` (`live`, `historical_as_of`, `version_comparison`), `report_time`, `cutoff_at`, `inputs`, `reference`, `comparisons[]` (purpose, service/client input IDs, `matches`, and the required integer `unchecked_count`). Date-only or explicitly later comparisons cannot serialize as historical as-of or live. |
| `change`, `impacts[]` | `elements[]`, `operation_id`, `change_kinds[]`, `before_presence`, `after_presence` (present/absent/unknown), `before_shape`, `after_shape` (type, format, required in parent, allowed values, item type), `status_additions[]`, `status_removals[]`, `response_root_pairs[]`, `call_changes[]`, `producer_sites[]`, `risks[]` and `review_action` with supporting row IDs. Distinguish a certified change in one file from a one-sided observation. |
| `usages[]` | Revision-local `site_id`, `before_site`, `after_site`, optional shared `usage_id` and `correspondence_evidence`, `requested_target`, and four typed references `match_before`, `match_after`, `match_service_first`, `match_client_first`, each `{comparison_id,site_id}` or `null` when inapplicable; all four keys are always present, as are `before_presence` and `after_presence` (`present`, `absent` or `unknown`: absent is proved, unknown is unproved) and `reason` (a nonempty string naming why the correspondence is partial or unavailable, else `null`) (for example both rollout references are `null` in a client-only review). The match payload is not stored under `usages[]`: it lives under `comparisons[].matches`, where each match carries service/client input IDs, mapping, status, primary grade/reason, target IDs and `alternates[]`/`overlaps[]` with operation, method, grade and reason. `prefix_hints[]` records both paths and the common suffix. Side-local element IDs (`before_element_id`, `after_element_id`) and the paired-root shape `{operation_id,before_status,after_status,fields:[{before_element_id,after_element_id}]}` are specified in DESIGN 5.2's impact records and apply here unchanged. Include field-use status/evidence and caller evidence. Each comparison owns the full match or scoped unchecked result for every occurrence in its client input, and each usage references it by `{comparison_id,site_id}`. Use report-local comparison IDs `actual_before`, `actual_after`, `service_first`, `client_first`. Site keys are opaque nonempty strings unique within the client input. The four joint maps use old, new, old and new client occurrences respectively. `match_before`, `match_after`, `match_service_first` and `match_client_first` resolve only to their own pair. Insertion, deletion and client-only insertion follow one rule: link exactly the comparison maps that contain the occurrence and record proved absence on the other side separately (a joint-edit insertion links two maps; a client-only insertion has one resolvable match, `actual_after`, and no second link). Ambiguous correspondence leaves separate one-sided usages. A client-only review has two actual maps sharing one service ID. |
| `binding_status`, `coverage[]`, `warnings`, `standing_mismatches[]` | Saved schema identity, actual element presence, recorded properties and comparison basis. Status is keyed by scan, element, selected service input/basis and saved location when present (DESIGN 5.6); candidate grade and selected-side presence are kept independently. Per-claim input/file/entry references, authority, scope, status, reason code and observed versions support both change and standing rows. Warnings name their affected claims. |
| `summary`, `unobserved` | `operation_count`, `changed_field_count`, `listed_usage_count`, `possible_only_usage_count`, `known_unmatched_count`, `unknown_target_count`, with version, scope, observed count and null plus reason for an unmeasured total. `unobserved` retains §5.2's meaning: a changed element has **no binding in any admitted live binding document**. A producer binding alone keeps it out even when no client matches. Membership is tested across every applicable admitted live producer and consumer document on each side where the changed element exists, retaining both side-local IDs and requiring complete evidence for a negative. The count and withholding rules (required `comparisons[].unchecked_count` for a readable target with missing mapping, `unknown_target_count` for an unreadable target, `known_unmatched_count` for a relevant checked miss, withholding of `unobserved` by candidate/overlap/gap evidence, linked read/send evidence for a client field binding) are those of DESIGN 5.2's `summary, unobserved` row. |

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

## Review round 2: disposition

Both review rounds are spent. The 27 first-round findings narrowed to 16, mostly in the amended text. This fold applies the two recurring rules across the report.

| Finding | Disposition |
|---|---|
| Opus A · WRONG | Versions checked enumerates the study inputs, prints their origin and selects the comparison title. First cut identifies the mapping entry at extraction. |
| Sol 1 · WRONG | Input admission and Saved matches require cutoff evidence for the earlier binding and its schema dependencies before selection, counts or membership. |
| Sol 2 · WRONG | Versions checked and §10 distinguish the computed tree from its constituent commits and specify exact-blob links and excerpts. |
| Sol 3 · WRONG | Match wording and §10 give product wildcard eligibility, priority and grading, plus the exact-method prototype fallback before totals or notification. |
| Sol 4 · WRONG | Fields and First cut read request/response media declarations and inheritance before counting a JSON field. |
| Sol 5 · WRONG | Operation and client changes and Fields emit independent success-status rows even for a bodyless removal with zero field edits. |
| Opus B · SMELL | First cut retains diagnostics, accounts for skipped declarations, certifies a file-local change and gives Review first an observation and action when nothing is confirmed. |
| Opus C · SMELL | Match wording puts other-prefix hints and unmatched-request counts beside each addition and removal, including rows with confirmed users. |
| Opus D · SMELL | Versions checked and §10 compare target to merge result in the changing repository in either setup. |
| Opus E · SMELL | Replay evaluation identifies the model adjudicator and explains tracked call and property lines, including lines in importing files. |
| Opus F · SMELL | Saved matches, worked reviews and First cut identify the earlier scan, name the service held unchanged, use consistent cell wording, and align duration, coverage and action with the example evidence. |
| Opus G · SMELL | §10 amends §§5.1 and 5.4 for all-method IDs and matching. |
| Sol SMELL 1 · Cell evidence | What counts as evidence and the First cut table require a frozen record and give supply and fallback for each cell. |
| Sol SMELL 2 · Prefix hints | Match wording compares shared trailing segments even when both addresses have a prefix. Hints keep their own label. |
| Sol SMELL 3 · Local wording | Examples, match wording, Fields and First cut name the search, scope and concrete reason wherever a partly checked claim appears. |
| Sol SMELL 4 · Preview lists | Empty and long reports caps each in-cell list, links its complete table and specifies a complete Markdown attachment on truncating surfaces. |
