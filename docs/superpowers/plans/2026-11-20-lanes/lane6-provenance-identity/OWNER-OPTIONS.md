# Lane 6 owner options · 25 November 2026

**Recommend (iii).** Keep the statically imported forwarding wrapper. Fund recovery against named classes of reads and keep default publication on hold through the next design fold. Decision 7 is settled. Decision 5 still needs the owner's acceptance of the residual losses.

**M** marks measured counts from the frozen census, matrix and base outputs. **E** covers all option forecasts, including head scores, prospective matrix changes and person-days. Gate percentages are policy thresholds. The baseline is the amended design at `6c32ff6d`. [Design §7](DESIGN.md#reachability-on-the-frozen-census) · [Census](CENSUS.md) · [Plan](PLAN.md)

| Decision 5 option | Benefit and cost | Increment to amended PLAN (E) | Without wrapper, input recall only (E, full / any) |
|---|---|---|---|
| (i) Drop the unproved origin | Simplest output. Removes 29 previously correct base matches, including two database matches and two config matches. | Shared work only, +4–7 days | 21.36% / 21.36% |
| (ii) Keep a suggestion at an eligible value read | Preserves 21 of those 29 as inspectable leads. Also preserves false leads on handles and locally computed values. Requires a separately scored output category. | Add legacy comparison, projection and sampling before first activation, +3–5 days. Total +7–12. | 21.36% / 70.14% |
| (iii) Drop now, fund **Project payload contracts** | Same initial loss as (i). Could restore 11 of the 29 at full strength and add 23 correct reads base missed. Requires maintained, verifiable contracts. | Add recovery after S5/S6, before G, +10–16 days. Total +14–23. | After recovery 52.90% / 52.90% |
| (iv) Suggestions now, same recovery later | Preserves leads during recovery. Promotions change target IDs again. Carries both maintenance costs. | Add both increments, +13–21 days. Total +17–28. | After recovery 52.90% / 89.32% |

### What users would see

Today a spelling can trigger an origin claim. Under every option a full claim requires the amended proof or admitted structural pattern. Compact review (`--format review`) shows `concern` for input/hardware and `warning` for database/external calls at its default `warning` floor. Parameter, environment and config are `info`. `--format json` includes all three severities. SARIF maps them respectively to `error`, `warning` and `note`. Default `prism targets` retains each finding. Today its mapper assigns hints from the description. All options require a winning library/built-in terminal in Python, JS/TS or Go for `db`/`network`. Patterns receive neither hint. Changing severity, description or related locations changes the target ID.

In (i)/(iii) a discarded claim disappears from every surface and contributes nothing to `findings_below_floor`. A proven alternative at the site can still appear. Recovery in (iii)/(iv) emits the ordinary finding for the proven origin.

In (ii)/(iv) compare the legacy classifier on the same input by file, line, public name and origin. Retain a differing legacy claim as a separate `possible_origin` suggestion only at an evaluated value read. Exclude declarations, callees, selectors and destination-only uses. A different proven origin can coexist. JSON includes the explicitly qualified claim. SARIF emits a `note`. Default targets include a new suggestion ID with a “verify possible origin” detail and no dependency hint. Compact review counts hidden suggestions in `findings_below_floor` per result and overall, omits zero, and reveals them at `--review-min-severity suggestion`. Hidden `info` never counts. Moving legacy `info` to `suggestion` actually raises its display priority. Proven matches retire the corresponding suggestion.

“Full” below means an origin assertion. The serializer's `FindingTier` grades the evidence path. Nominal resolution marks CPG findings unlabeled/candidate. Scoped asserted requires exact evidence and a clean parse. Keep suggestions unlabeled/candidate even on an exact reaching edge so `--min-tier asserted` excludes them. [Review floor](../../../../../crates/prism-review/src/output/review_compact.rs) · [Target mapping](../../../../../crates/prism-review/src/targets/mapping.rs) · [Confidence rules](../../../../../src/finding_confidence.rs)

### Reach on the frozen census

Each recall entry is **same-origin key count / weighted percent**. Denominators, base hits and loss weights are M. Option columns are E ceilings. The loss column counts base hits outside the amended reachable set and gives their inclusion weight. Initial (iii) equals (i), initial (iv) equals (ii).

| Origin | Decided keys / weight M | Base M | Base hits lost / weight | (i) full = any, (ii) full | (ii) any | (iii) full = any, (iv) full after recovery | (iv) any after recovery |
|---|---:|---:|---:|---:|---:|---:|---:|
| user_input | 44 / 242.71 | 37 / 71.38% | 18 / 81.92 | 20 / 47.07% | 34 / 70.14% | 35 / 78.61% | 40 / 89.32% |
| database | 25 / 218.70 | 2 / 0.91% | 2 / 2.00 | 0 / 0% | 2 / 0.91% | 18 / 52.67% | 19 / 53.12% |
| external_call | 5 / 88.54 | 0 / 0% | 0 / 0 | 0 / 0% | 0 / 0% | 0 / 0% | 0 / 0% |
| env_var | 3 / 15.60 | 0 / 0% | 0 / 0 | 0 / 0% | 0 / 0% | 0 / 0% | 0 / 0% |
| config | 13 / 119.27 | 6 / 78.55% | 2 / 45.84 | 6 / 41.79% | 7 / 61.01% | 7 / 61.01% | 7 / 61.01% |
| function_param | 90 / 1,444.56 | 33 / 37.61% | 7 / 122.64 | 72 / 81.16% | 76 / 84.89% | 72 / 81.16% | 76 / 84.89% |

All 180 keys, labels, weights and base hits rejoin at the scored use (M). The amendment reaches 98 keys (E). Any-tier recall adds each correct eligible legacy origin once. Eight base-hit role discrepancies comprise four input destinations/declarations, one config callee and three formal declarations (M). Restoring even those would lift any-tier input/config/formal recall to 80.82%/80.23%/89.65% (E) by reporting on excluded roles. The wrapper cohort has 12 base hits / 62.40 weight (M). It includes four header reads or aliases, two query reads, four route captures or aliases and two body reads or aliases. The pinned body route installs its parser before the callback. Snapshot and supplier checks condition the credit.

**Recovery budget.** These disjoint §7 cohorts give ceilings. Credit a contract through the normal product entry point only after independent verification of its implementation, dispatch, selected slot, terminal and fallback paths. The estimate includes the initial opt-in contract pack keyed to reviewed source digests. Its ongoing maintenance is unpriced. The pack needs broader cross-file authority than decision 7 grants. No new public evidence grade is needed. With no verified pack, only the local config tie can improve.

| Recovery increment | Target keys / weight M | Previously correct keys recovered E | Effort E |
|---|---:|---:|---:|
| Bind contracts and their resolution inputs to the reviewed snapshot. Refuse stale, ambiguous or substituted implementations. | Infrastructure | — | 3–5 days |
| Verify ingress contracts for project JSON adapters (5 / 5.00), body helpers (2 / 2.00), route helpers (2 / 20.63), tRPC installation/builder slots (5 / 26.00) and a GraphQL mutation argument (1 / 22.92). Check parser replacements and virtual dispatch. | 15 / 76.55 input | 9 | 3–5 days |
| Verify repository/DAO/store result and iteration slots (13 / 110.18) and materialized imported-model reads (5 / 5.00). A cache or local default can defeat a database claim. | 18 / 115.18 database | 1 | 3–5 days |
| Prefer a terminal-proven config source over an ordinary formal projection in an equal-risk result tie. Recheck competing winners. | 1 / 22.92 config | 1 | 1 day |

At the ceiling, (iii) still loses 18 correct base matches (E). Nine are input (five transformed, defaulted or overwritten values and four role discrepancies), one is a dynamic database attribute, one a config callee and seven are formals (four unlisted method receivers and three declarations). External clients, computed environment values and the project environment getter have no base hits to restore. Broadening formal-receiver admission could add 12 keys / 134.58 weight at the cost of three object-negative matches (reviewer estimate).

### Matrix, gates and next fold

Each cell counts distinct original IDs in **control / structural / py / js / go** order (group sizes 61 / 769 / 28 / 13 / 22 M). For each ID, start with the original wants, apply every recorded base mismatch and include unexpected names on exact lines. Compare that output with the amended sites and origins, including additions. The matching IDs reproduce the amended controls. Counts cover origins and disclosure tiers. ID churn requires emitted locations. [Row decisions](D-rows.tsv) · [Amended groups](D-groups.tsv)

| Comparison on 893 original IDs (M) | (i) | (ii) | (iii) | (iv) |
|---|---|---|---|---|
| Changed from observed base, including severity changes | 7 / 530 / 23 / 12 / 18 | same | same | same |
| Changed from original emitted wants | 7 / 142 / 3 / 3 / 6 | 7 / 377 / 26 / 13 / 16 | as (i) | as (ii) |
| Additional changes to amended wants | 0 / 0 / 0 / 0 / 0 | 6 / 286 / 23 / 12 / 13 | as (i) | as (ii) |

Base versus amended full output differs on 590 rows, with 303 matches (M). For suggestions, inspected fixture lines for differing legacy names/origins and counted rows with an eligible read (340 E, versus 393 M before role filtering). Original fixtures supply no project contracts and contain no winning formal/config operand tie of the proposed kind. Recovery needs new contract-present, missing and stale twins. These comparisons precede reviewer repairs. Thirteen named original fixtures need supplier, export or catalog corrections (1 / 12 / 0 / 0 / 0 M). The 22 proposed additions are outside the table. Final vectors await replay.

**Gate for every option.** At family activation and G, require ≥85% right among adjudicated full assertions in every populated finding-language × claimed-origin cell, including `info`. Label cells below 30 in full and apply the same threshold. Publish empty cells as unmeasured. This covers every finding in (i)/(iii). Choosing (ii)/(iv) requires accepting a narrower gate. Publish suggestion-only and pooled precision, volume and intervals. Suggestions earn zero full-recall or request-cross-check credit. Base concern/warning precision was 7.4% with objects counted wrong (M). Head precision is unmeasured. Retain the 95% per-class full-denominator objective, every frozen reachable key, raw request reconciliation and positive/negative matrix pairs for empty reachable cohorts. All options fall short of the original recall objective.

**Shared work, +4–7 days E.** Budget the next D fold and bounded repairs in T, S2, S4 and family gates. S4b's wrapper is already in PLAN (2–4 days E within scope). Move S6's hint guard to first affected activation. No slice is removed. PLAN's former 29.5-day estimate predates the split.

- All choices preserve the round-one forwarding, per-key accounting and outbound-request corrections, separate traversal budgets, split activations, collection rules, gate sequencing and qualified HCL names. The pattern catalog and disputed whole PostgreSQL result still need disposition. Defer the unsupported libpq read and shell environment operations, log the shell network reclassification, and adjudicate envelope truth under the frozen rubric or rescore both samples.
- All leave bounded repairs for snapshot-bound adapter/metadata loading, observable resolver overrides, the invalid named config export, native seed line and byte scopes, and context/handle formals. Round two projects ten false formal findings on 45 object negatives and roughly 81% formal precision in Go and JS (E). Typed exclusions help. Untyped handles and server registrations elsewhere remain risks. Only (iii)/(iv) price recovery of the config tie.
- Binding a request leaves payload supply open in every option. Require the named route capture, ordered body/cookie/upload middleware (including Nest), or a documented default. Fastify's buffered parser supplies bytes/string. Its stream overload needs a byte read. Pair each with absent, replaced and reordered suppliers. Under (ii)/(iv) even the four existing body/cookie/upload fixtures with missing suppliers could retain a false legacy suggestion. Contract recovery also needs deployed-version and ordering evidence.

Choose (iv) if consumers value historical leads enough to fund separate adjudication and tolerate ID churn. Choose (i) only with explicit acceptance of its losses. If independently verifiable contracts cannot be maintained, the recovery price buys no assured recall. Return decision 5 to the owner before any default-output merge.
