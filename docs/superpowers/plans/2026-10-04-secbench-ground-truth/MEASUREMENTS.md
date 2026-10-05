> Historical R0 record. R1 changes ground truth, attribution and conversions; use [R1.md](R1.md) for the current result and verification. Original evidence remains immutable.

# MEAS-A results — pinned Prism 4e592daa

**Recommendation:** provisionally choose workstream **3 before 6**. Their first-break
opportunities are 13 versus 6 entries; scenario-weighted scores are 31 versus 15.
Under the brief's broad B-to-2 mapping, workstream 2 leads with 22 opportunities
(score 50). These are planning signals, **not measured conversions**. No workstream
fix was implemented or counterfactually evaluated, and downstream barriers remain.
Consequently the demonstrated conversion lower bound is zero for every workstream;
the requested ranking by actual entries converted cannot be established here.

The acquisition and full Prism observation run are complete. Source-GT recovery and
causal/path attribution remain incomplete. Do not promote this packet to a confident
population recall or single-workstream conversion claim.

## Provenance and denominator

SUT checkout/branch: `plan/secbench-ground-truth` at
`4e592daa7858a195eb3a9eb77c83dfbc763b49fa`. Release identity
`slicing 3.1.2 (4e592daa7858)`; SHA256
`d603e7cd7f372137c254f4d7cf98ea8c33f92ac22dc510bea0e1b00e919aa455`.
SecBench is `5d362353550a8baa42bba34edd26e5fb86d41b60`.
All 583 acquired tarballs match the manifest; all package/exploit/metadata bytes
are authenticated in `eval/secbench/input-pins.json`. Its SHA256 is
`922ba44ba017253a8a7d3667495682e4659ece525d84447a39f7bf102c6a243e`.

600 entries = 583 acquired + 11 multi_or_no_dep + 6 pack_failed. Of the acquired
entries, 192 have usable GT (32.9%); 391 are unavailable (67.1%). There are 164
eligible cases with successful decisive queries and 28 Prism errors. Traced rate
is 84/192 = 43.8% among eligible cases, or 84/164 = 51.2% conditional on successful
queries. Neither is corpus-wide recall. Only 1/170 path-traversal entries is eligible,
which seriously disadvantages Node/HTTP workstream 6 in this comparison.

The original run used 220 initial GT candidates: 2,123 invocations, including
583 dfg-stats calls and 220 each of witness, frontier, callees, callers, ego, classic
taint, and chop. Bounded GT/occurrence corrections reclassify authenticated retained
observations, never invent a new source/sink invocation. The final 600-row ledger is
`/Users/wesleyjinks/prism-evidence/meas/secbench/final/entries.jsonl`. It supersedes
50 original classifications. Original `run/` artifacts are historical evidence;
`final/` and the copied summary/binding here assert the current result.

## Outcomes, attribution, syntax and sensitivity tables

Cells in attribution columns include each class's share of its eligible non-traced
cases (12 code, 30 command, 0 path, 43 pollution, 23 ReDoS; 108 overall).
Manual first-break labels replace sampled heuristics. The remainder are hypotheses,
not mechanism-confirmed findings. There are 40 unresolved cases and 26 query errors
without a resolved capability attribution; all remain in the denominator.

| Class | Total | Traced | Function only | Partial | Not reached | Prism error | GT unavailable | Acquisition excluded |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| code-injection | 40 | 9 | 5 | 7 | 0 | 0 | 17 | 2 |
| command-injection | 101 | 33 | 14 | 9 | 1 | 6 | 28 | 10 |
| path-traversal | 170 | 1 | 0 | 0 | 0 | 0 | 166 | 3 |
| prototype-pollution | 192 | 27 | 20 | 8 | 1 | 14 | 120 | 2 |
| redos | 97 | 14 | 11 | 4 | 0 | 8 | 60 | 0 |

## First-break attribution (manual labels replace sampled heuristics)

| Category | Workstream | Code | Command | Path | Pollution | ReDoS | Overall | Share of  eligible non-traced |
|---|---:|---:|---:|---:|---:|---:|---:|---:|
| B-argument | 2 | 3 (25.0%) | 4 (13.3%) | 0 (n/a) | 5 (11.6%) | 2 (8.7%) | 14 | 13.0% |
| B-arguments | 2 | 0 (0.0%) | 0 (0.0%) | 0 (n/a) | 1 (2.3%) | 0 (0.0%) | 1 | 0.9% |
| B-default | 2 | 0 (0.0%) | 1 (3.3%) | 0 (n/a) | 0 (0.0%) | 0 (0.0%) | 1 | 0.9% |
| B-rest-spread | 2 | 0 (0.0%) | 0 (0.0%) | 0 (n/a) | 6 (14.0%) | 0 (0.0%) | 6 | 5.6% |
| C-member | 3 | 1 (8.3%) | 7 (23.3%) | 0 (n/a) | 2 (4.7%) | 3 (13.0%) | 13 | 12.0% |
| D-cjs | 6 | 0 (0.0%) | 2 (6.7%) | 0 (n/a) | 1 (2.3%) | 0 (0.0%) | 3 | 2.8% |
| E-admission | 4 | 0 (0.0%) | 0 (0.0%) | 0 (n/a) | 0 (0.0%) | 1 (4.3%) | 1 | 0.9% |
| H-callback/promise/event | 6 | 1 (8.3%) | 1 (3.3%) | 0 (n/a) | 0 (0.0%) | 1 (4.3%) | 3 | 2.8% |
| prism_error | unassigned | 0 (0.0%) | 6 (20.0%) | 0 (n/a) | 14 (32.6%) | 6 (26.1%) | 26 | 24.1% |
| unresolved | unassigned | 7 (58.3%) | 9 (30.0%) | 0 (n/a) | 14 (32.6%) | 10 (43.5%) | 40 | 37.0% |

## Syntax census and candidate path prevalence

| Class | Acquired packages | Packages destructuring | Packages rest/spread or Object.assign | Candidate paths | Path destructuring | Path rest/spread |
|---|---:|---:|---:|---:|---:|---:|
| code-injection | 38 | 5 | 5 | 15 | 0 | 1 |
| command-injection | 91 | 16 | 10 | 54 | 3 | 1 |
| path-traversal | 167 | 2 | 6 | 1 | 0 | 1 |
| prototype-pollution | 190 | 37 | 60 | 63 | 3 | 10 |
| redos | 97 | 14 | 24 | 33 | 1 | 1 |

Path rows concern features anywhere in candidate callable spans, not proved value dependence. Unknown paths are excluded only from this clearly stated sub-denominator. Package syntax includes shipped tests, demos, declarations and vendored sources; it does not imply server-runtime use.

## Manually inspected payload paths

| Class | Inspected eligible paths | Destructuring | Rest/spread or Object.assign |
|---|---:|---:|---:|
| code-injection | 6 | 0 | 0 |
| command-injection | 6 | 0 | 0 |
| path-traversal | 1 | 0 | 0 |
| prototype-pollution | 5 | 0 | 1 |
| redos | 6 | 0 | 0 |

## Payload-path syntax bounds (eligible cases)

| Class | Destructure present / absent / unknown | Rest-spread present / absent / unknown |
|---|---:|---:|
| code-injection | 0 / 20 / 1 | 0 / 19 / 2 |
| command-injection | 0 / 50 / 13 | 0 / 57 / 6 |
| path-traversal | 0 / 1 / 0 | 0 / 1 / 0 |
| prototype-pollution | 0 / 61 / 9 | 9 / 51 / 10 |
| redos | 0 / 33 / 4 | 0 / 31 / 6 |

Present is established by a test-fed entry parameter binding or manual payload-path inspection. Absent is established manually or by whole-package syntax absence with zero diagnostics/unparsed files. Syntax presence elsewhere cannot establish path presence. Bounds concern constructs inside the acquired package, excluding unpacked external dependencies.


Adjudication: {"agreement": 0.5384615384615384, "exact_category_agreements": 7, "independent": false, "method": "same-worker source and retained-output adjudication; frozen hash-stratified selection", "non_traced_compared": 13, "resolved_agreement": 0.3333333333333333, "resolved_categories": 9, "resolved_category_agreements": 3, "sample_entries": 30}


## Workstream opportunity sensitivity

| Workstream | First-break opportunities | Severity-weighted score | Adjudicated entries | Proven conversions |
|---|---:|---:|---:|---:|
| 2 | 22 | 50 | 4 | 0 |
| 3 | 13 | 31 | 3 | 0 |
| 4 | 1 | 1 | 1 | 0 |
| 5 | 0 | 0 | 0 | 0 |
| 6 | 6 | 15 | 1 | 0 |

Weights (scenario, not CVSS): command/code injection 3, prototype pollution/path traversal 2, ReDoS 1. Equal weighting is available in the raw opportunity counts.

First-break opportunities are not demonstrated single-workstream conversions.

## Ground-truth exclusions

Unavailable means the harness could not establish the contract, not that the
corpus necessarily lacks a source or that Prism failed. Reason counts are below;
they are mutually exclusive final reasons, not additive feature diagnoses.

| Reason | Entries |
|---|---:|
| ambiguous_payload_parameter | 6 |
| ambiguous_sink_basename | 9 |
| gt_parser_unsupported_returned_api | 24 |
| http_payload_not_exported_api_argument | 25 |
| missing_sink_coordinate | 7 |
| missing_sink_file | 7 |
| multiple_exported_entry_functions | 4 |
| no_bindable_data_parameter | 17 |
| no_direct_package_api_call_with_data | 152 |
| sink_coordinate_not_call_or_assignment | 1 |
| sink_not_in_callable | 1 |
| sink_out_of_range | 1 |
| unresolved_export_or_api_chain | 137 |

The frozen manual sample is six smallest SHA256(class/entry) values from each
class's initial available-GT population, selected before completed measurement.
It retains six subsequent GT exclusions rather than resampling. Of 13 non-traced
cases, nine receive resolved mechanisms, three remain unresolved and one remains
an unresolved query error. Agreement is 7/13 (53.8%); restricting to nine resolved
manual categories gives 3/9 (33.3%). The same worker performed adjudication, so this
is consistency auditing, not independent accuracy validation or a confidence bound.

## Destructuring and rest/spread interpretation

All 583 acquired packages are scanned: 74 contain destructuring (12.7%) and 105
contain rest/spread or Object.assign (18.0%). Counts include 2,041 destructured
parameters, 5,166 destructuring declarations, 4,121 rest parameters, 83 rest
bindings, 783 object spreads, 2,000 array/argument spreads, and 2,375 Object.assign
calls. These are shipped-package syntax counts, not server-runtime frequency.
Parsing covers 51,459 files / 344,494,436 bytes; 402 files have diagnostics and 66
non-UTF8 files are explicitly unparsed. Counts are parser observations, not an
exhaustive Unicode/grammar validity claim. Tests, demos, declarations, browser
bundles and vendored code are included; entries are not deduplicated across classes.

166 source-to-sink candidate callable chains are recovered: seven contain
syntactic destructuring anywhere in their callable spans (4.2%), and 14 contain
rest/spread or Object.assign (8.4%). These are **screens**, not value-path prevalence.
For example hangersteak spreads server configuration while its attack flows through
req.url; the broad screen is positive but the payload-path flag is negative.

Across the 192 eligible cases, payload-path destructuring is absent in 165 and
unknown in 27, with no positive adjudication. Rest/spread is present in nine, absent
in 159 and unknown in 24. Thus the available evidence bounds within-package path
prevalence at 0–14.1% for destructuring and 4.7–17.2% for rest/spread. The other 391
acquired cases lack usable source/sink GT and retain an unknown path stratum.
Nine exploit-fed entry parameters use syntactic ...rest, establishing positive
path syntax directly; this does not depend on successful Prism tracing.
The manually inspected eligible sample is 0/24 destructuring and 1/24 rest/spread;
its one positive case is decal's test-fed ...rest parameter. These results include
traced and non-traced entries. They cannot establish a point estimate of server-side
source-to-sink prevalence across the entire acquisition corpus.

## Per-class observations

- **Code injection:** 9/21 eligible traced. thenify reaches the function base but
  misses the consumed fn.name member. local-devices loses the promise result into
  arpOne before exec. value-censorship reaches the local call argument but not the
  run parameter. Later AST/serializer flow remains unproved. Class labels follow
  SecBench, even when a declared code-injection sink is a shell command.
- **Command injection:** 33/63 traced. dns-sync, node-df and im-metadata demonstrate
  observed command construction reaching runtime sink values. aaptjs's aggregate
  verdict is reached through an unrelated callback; its payload parameter is not
  reached. The earliest identified gap is template argument transfer, before the
  deferred Promise callback. lycwed remains unresolved between module, callback
  and member transfer, because union frontier already includes callback uses.
- **Path traversal:** hangersteak traces req.url to createReadStream. HTTP server
  setup arguments cannot substitute for the malicious request path. 166 unavailable
  cases make this class unsuitable for judging workstream 6's overall benefit.
- **Prototype pollution:** 27/70 traced. decal fails at its rest source binding;
  defaults-deep accesses the payload through legacy arguments[] rather than its
  unused formal. safe-flat reaches an inner helper but does not discriminate key
  enumeration, higher-order transfer and dynamic property barriers. Merely fixing
  the first binding would not establish final assignment reachability.
- **ReDoS:** 14/37 traced. locutus and npm-user-validate expose omitted receiver-only
  parameter Defs, tied to src/data_flow.rs:602's bare-reference guard. The latter
  is a source-seed query error, not a negative taint verdict. html-parse-stringify's
  shipped dist entry is excluded by src/repo_loader.rs, supporting E-admission.
  minimatch's declared sink seed is absent and remains a query error. Reaching a
  regex use does not prove regex complexity or runtime denial of service (J).

## Practical recommendation and limits

For the requested choice, prioritize a bounded **workstream 3** member-input and
receiver checkpoint next, with this baseline as the comparison artifact. Workstream
6 remains necessary and is probably under-observed because HTTP/request sources and
returned APIs are poorly recovered. For a broader backlog, the declared first-break
ranking is **2 > 3 > 6 > 4 > 5**, under equal and scenario-severity weights. The
B bucket includes ordinary argument transfer and legacy arguments binding as well
as new parameter forms; it is broader than destructuring/rest alone.

No category assignment proves a single-workstream conversion. Root/sink occurrence
filters conservatively reject unbindable metadata. Options-object inputs are seeded
at the containing formal parameter, not property-selectively. The `partial` fallback
and attribution screen use union-seed frontier evidence; a per-parameter frontier
is unavailable for unreached sinks. Function-only credit uses enclosing call-graph
spans, not a value path. Warnings/conservative graph edges can make reached verdicts
an over-approximation. Explicit declared sinks bypass sink-recognition requirements,
so zero G-sink-model attributions say nothing about native security sink coverage.
External dependency bodies and runtime behavior are not evaluated.

At the three-pass validation cap, closed GT/endpoint fixes were folded and the
artifact preserved. Broader factory/HTTP/ambiguous API recovery and root-specific
causal/path adjudication are parked as measurement-design follow-up; they must not
be silently counted as missing product capabilities or confidently ranked conversions.
No product changes, commits, network use, exploit execution, or private-corpus reads
were performed in this lane.
