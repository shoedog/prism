# PR-A R1 measurements

`[MEASURED]` 2026-10-05, static source analysis only. No corpus package executed.

Base is `c8de720b36c24ae8a7ab274ec10c994f258eb336`; committed prototype is `1b2dfdc93a37dbd87c6359c34e7723912399b682`. Head is prototype plus `repair-r1/R1-src.patch`. Docs begin at `6b0be4ff`. Evidence is `/Users/wesleyjinks/prism-evidence/js-param-defs/repair-r1`; immutable executable hashes are in `bin/binding.json`. Prior prA3 line-only adjudication is superseded: the old oracle falsely accepted both W3 repros.

## Byte binding rows

Each record carries file, Def/Use bytes, callable bytes and parameter classification. Exact identifier `getSymbolAtLocation`, declaration-name span equality and callable identity decide Step 1. Shorthand reads use the checker value symbol. Collapsed Use anchors require unanimous byte-identified same-name occurrences within the recorded owner on that line. Missing/ambiguous owner identity and mixed bindings are UNDECIDED. Owner/span replacement appears as LOST plus ADDED. Binding CORRECT is not an independent flow proof.

| Corpus | ADDED | CORRECT | WRONG | UNDECIDED | LOST correct / wrong / undecided |
|---|---:|---:|---:|---:|---|
| X | 36 | 36 | 0 | 0 | 0 / 0 / 0 |
| Xi installed | 36 | 36 | 0 | 0 | 0 / 0 / 0 |
| T | 1,053 | 1,052 | 1 (E7) | 0 | 0 / 0 / 0 |
| SecBench (574/583 admitted) | 827 | 826 | 1 (E7) | 0 | 0 / 0 / 0 |

SecBench has three inherited E1 WRONG rows relabelled Exact→NameOnly(SameLine), with unchanged endpoint identity. These are safer labels on existing wrong bindings. The 827 gains are 503 rest and 324 bare-arrow rows; labels are 405 Exact, 351 CfgIncomplete, 61 Killed and 10 SameLine. Prototype→R1 has zero forfeited rows over the 574 successful triples; the broader D11 guard adds no observed row cost. All 574 call-site outputs are byte-identical.

X and Xi each add 25 Exact and 11 NameOnly(CfgIncomplete) rest rows, with no relabel or LOST. Their call-site output is byte-identical. No prototype gains are forfeited there. T first timed out under concurrent workload; that attempt is inadmissible and retained. Complete recapture: base byte308.6s/site382.5s, prototype byte401.4s, head byte325.8s/site316.5s, all successful. T has four inherited E1 WRONG rows relabelled Exact→NameOnly(SameLine). Prototype→R1 forfeits four checker-correct bare-arrow Exact gains in watchApi.ts afterProgramCreate(program). This is the conservative E5 refusal around cb(program), whose unresolved-referenceable inferred cb targets are NameOnly; it removes all outgoing new-source rows. The broader D11 binder guard adds no observed row cost in T. None are correct base rows. SecBench capture excludes the union of failed base/head producers from both sides, with per-side time/status retained; prototype cost uses a separate successful-triple intersection.

## Same-environment reviewer controls

Both W3 fixtures are old-oracle CORRECT and byte-oracle WRONG (`declaration_span_mismatch`). UTF-8 astral offsets, BOM-prefix offsets and export-comment trivia pass; controlled stale checker text is UNDECIDED; a wrong owner and shifted identifier span are WRONG; ambiguous/missing identity is UNDECIDED. Hidden-file census changes 0 to 1 on the review fixture. E7's property-key row is identical on plain-formal base and rest head; fix routes to PR-C. The byte checker also exposes E7 in SecBench marsdb_0.6.11 Cursor._addPipeline args@118→property-key args@128. Replacing only the rest dots by spaces in an owned copy gives byte-identical owner rows on plain base and rest head, checker-WRONG on both (`marsdb-E7-control.json`). E8's after-return rest wire/trace is identical on JS/TS/TSX plain-formal base controls. These remain disclosed pre-existing WRONG mechanisms. The current js-data_3.0.9 crud args@1403→1449 prior-write hit has the same byte row and Exact label on a full-package plain-formal base copy (rest dots replaced by spaces), checker CORRECT/EXACT_PRIOR_WRITE on both. PostCSS plugins@20→24 likewise has the identical byte row and Exact label on a full-package plain-formal base control, checker CORRECT/EXACT_PRIOR_WRITE (`postcss-E6-control.json`). Both of the two strict prior-write hits are inherited parity. Prior-write and unconditional return/throw checks are bounded syntax probes; complete independent corpus CFG reachability is not verified.

## SecBench

All 583 packages were attempted. The byte table admits 574 paired base/head captures and excludes nine failed packages from both sides: command-injection/total.js_3.4.6; prototype-pollution/total.js_3.4.6; path-traversal/atropa-ide_0.2.2-2; redos/cejs_2.0.20170212; redos/clean-css_4.1.10; redos/natural_5.1.0; redos/react-native_0.63.0-rc.0; redos/three_0.122.0; redos/vant_2.12.11. Every excluded byte producer timed out on both base/head at the 300s budget. clean-css base call-site producer was killed (−9); React Native head call-site producer succeeded but its byte producer failed, so the package remains excluded on both sides. Per-side times/statuses are preserved in `sb-rows-final/excluded.json`. These failures provide no binding verdict.

Unmodified target packages: base 7 reached_function_only and 2 prism_error; head 7 traced, 1 reached_function_only (is-svg, downstream B-plain-argument), 1 partial (portprocesses, downstream D-cjs). All six rest targets and one bare-arrow target trace. Payload-specific BFS proves all seven added target credits without passing through sibling formals.

Eligible sweep: base 373 rows, 113 traced / 31 partial / 80 reached_function_only / 142 prism_error / 7 not_reached. Final head: 373 rows, 120 traced / 32 partial / 74 reached_function_only / 140 prism_error / 7 not_reached. Exactly eight outcomes improve, all 113 base traces remain traced. Both clean-css_4.1.10 and natural_5.1.0 are prism_error on both sides. wind-mvc_0.0.6 has producer timeouts on both sides in the 120s sweep; it is an explicit error/exclusion rather than behavioral evidence. A separate same-environment 600s control returns partial on both sides (base 53s, head 62s); raw sweep totals remain unchanged. Payload BFS is 107/113 base and 114/120 head; all seven new credits are payload-specific, and the same six inherited non-specific credits remain.

## Gates

| Gate | Final result |
|---|---|
| js_param_defs | 27 passed; original prototype RED has 8 failures, TS runtime declaration overlay adds 1 RED |
| required_parameter / nested_execution_owner | 23 / 11 passed |
| nextest --features mcp / separate CI doctests | 5,164 passed, 1 skipped (`slice_elem_variant_reserved`, future reserved classifier); doctests2/2 passed |
| advisory scoped / authoritative lane mutgate | 29/29 default advisory scope (includes coupled P2-M11); 28/28 authoritative PR-A lane; also 28/28 PR-A-only advisory |
| fmt | clean |
| clippy CI form, same environment | 181 lib-test warnings on each side; normalized diagnostic multiset 235 on each, added 0 / removed 0 |
| Tier-A matrix | 178/178 ok |
| Tier-A quick | TS/JS r2-frame both VALID with identical counts: TS callers raw65/7/112 exact58/1/119, callees raw203/47/14 exact178/31/39; Node callers9/0/56 callees11/2/35 (TP/FP/FN). Final run uses the same seed/sample and 1800s oracle budget after the first 600s timeout; default quick also VALID for Rust/TS/Node |
| non-JS | black/caddy/pinned Prism Rust: enriched byte records, sorted wire rows and call-site output identical. DFG rows20,539/72,681/54,150; call sites6,637/20,705/49,302. Interrupted Go wire retry was repaired and positively recaptured; zero/failed output was excluded from identity claims |

No full all-lanes mutation run or human-triggered multi-corpus Tier-A run is claimed for R1. F remains controller-only and unaccessed. Final artifacts are `R1-src.patch` (five files relative to committed prototype), `R1-docs.patch` (relative to docs HEAD), `results-summary.json`, `artifact-manifest.json` and `r1-final-snapshot.tgz`. The handoff records cleanup and controller work. There is no LOST correct base row, non-JS non-identity, call-site change or trace regression: STOP is none. The four correct prototype T gains forfeited by D12 remain explicitly priced. The controller was smoke-tested on owned fixtures: matching producers COMPLETE; deliberately mismatched producer INADMISSIBLE at byte_projection. These are fixture checks, not F measurements.

Hook follow-up: root `VERIFICATION.md` records exact commands and exclusions. Source/test/mutant hashes remain identical to the final nextest snapshot. `r1-failure-intersection.py` adds actual-old-code aggregation REDs for base failure, partial head failure and sites-only failure (3/3 old failures, repaired shell3/3 and byte consumer3/3 pass). The hook doctest target was deleted after its2/2 pass; final patches/manifest/snapshot include the root verification document.
