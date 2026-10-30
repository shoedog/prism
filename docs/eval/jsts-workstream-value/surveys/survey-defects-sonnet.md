# Survey A: General (Non-Security) Defects in JS/TS/Node/React OSS

Scope: empirical bug studies (not CVE/vulnerability databases — see Survey B) covering
JavaScript, TypeScript, Node.js and React open-source projects, mapped to prism's
capability areas A–J and candidate workstreams 1–7 (`taxonomy.md`).

## 1. Defect-class distribution, with sources

No single study spans the whole JS/TS/Node/React surface with one taxonomy, so this
section triangulates across studies with different unit of analysis (bug report vs.
commit vs. runtime fault). Numbers are **not directly additive across studies**.

| Study | Unit / N | Headline distribution |
|---|---|---|
| BugsJS (Gyimesi et al., ICST 2019 / STVR 2021) [bugsjs.github.io/paper/ICST19.pdf](https://bugsjs.github.io/paper/ICST19.pdf), [onlinelibrary.wiley.com/doi/full/10.1002/stvr.1751](https://onlinelibrary.wiley.com/doi/full/10.1002/stvr.1751) | 453 real bugs, 10 server-side Node.js projects, ~444k LOC | First JS-specific bug taxonomy; I could not independently re-extract the exact category percentages from the PDF in this pass (binary extraction failed) — treat any specific per-category share from this paper as **unverified in this report** beyond "incomplete/incorrect feature logic dominates, input-validation and config are present but minor." Flagged for follow-up rather than asserted. |
| Ocariza et al. (ISSRE 2011 / STVR 2016 "AutoFLox" line) [people.ece.ubc.ca/frolino/docs/autoflox_journal.pdf](https://people.ece.ubc.ca/frolino/docs/autoflox_journal.pdf) | 502 bug reports, 19 client-side JS projects, >2M LOC | **68%** of client-side JS faults are DOM-interaction faults (JS↔DOM state mismatch); a related count puts DOM-related errors at **≥79%** of reported JS errors in some of the bug trackers examined. This is a client-side (browser) sample, not Node/server-side — a dataset-bias caveat (§caveats). |
| Gao, Bird & Barr, "To Type or Not to Type" (ICSE 2017) [microsoft.com/en-us/research/wp-content/uploads/2017/09/gao2017javascript.pdf](https://www.microsoft.com/en-us/research/wp-content/uploads/2017/09/gao2017javascript.pdf) | 400 fixed bugs, 389 JS projects | **15%** of shipped bugs would have been caught by adding Flow/TypeScript type annotations to the pre-fix code (static-type-detectable bugs); no significant Flow-vs-TS difference. |
| "From Logic to Toolchains" (TypeScript ecosystem study, 2026) [arxiv.org/abs/2601.21186](https://arxiv.org/abs/2601.21186) | 633 bug reports, 16 popular TS OSS repos | 11-category taxonomy: **Tooling/Config 27.8%**, **UI Bug 12.5%**, **API Misuse 14.5%**, **Type Error 12.4%**, **Logic Error 9.7%**, Test Fault 9.0%, Missing Case 8.2%, Missing Feature 3.8%, **Async/Event 3.6%**, Runtime Exception 1.2%, Error Handling 0.8%. Tooling/Config + API Misuse + Type Error together ≈ **two-thirds** of all bugs in this sample. This is the most directly usable class table for category J (tooling/config) and D/E (API/type boundary) weighting. |
| Wang et al., "A Comprehensive Study on Real World Concurrency Bugs in Node.js" (ASE 2017) [researchgate.net/publication/321260286](https://www.researchgate.net/publication/321260286_A_comprehensive_study_on_real_world_concurrency_bugs_in_Nodejs) | 57 concurrency bugs, 53 Node.js apps, ~3.5M LOC | **~67% (two-thirds)** are atomicity violations across async callback boundaries, attributed to Node's lack of locks/transactions for expressing atomic intent. Narrow slice (concurrency only), but every instance is an H-class (async/callback/event-ordering) defect by construction. |
| Madsen, Lhoták & Tip, "A Model for Reasoning About JavaScript Promises" (OOPSLA 2017) [cs.au.dk/~magnusm/papers/oopsla17/paper.pdf](https://cs.au.dk/~magnusm/papers/oopsla17/paper.pdf) | Qualitative classification + StackOverflow case study; no large-N prevalence count | Defines the promise-graph error taxonomy (e.g., forgotten `return`, swallowed rejections, double-resolution) used by later promise-bug detectors; **no percentage claim usable here** — cite as mechanism/taxonomy source only, not a prevalence number. |
| React useEffect dependency bugs — practitioner/vendor sources, not peer-reviewed | Cloudflare outage postmortem; blog-level claims of "70%"/"80%" of developers affected | These are **vendor-blog estimates, not peer-reviewed study numbers** — I found no peer-reviewed empirical paper quantifying useEffect-dependency-array bug prevalence at project scale. Treat all useEffect/hook prevalence figures in this report as **estimates**, sourced to practitioner writeups (e.g. [dev.to Cloudflare useEffect writeup](https://dev.to/vasughanta09/the-react-useeffect-object-dependency-trap-how-cloudflare-accidentally-ddosed-itself-2ge6)), not academic studies. |
| npm breaking-change studies: Mujahid et al., "I depended on you and you broke me" (TOSEM/arXiv 2301.04563, 2023) [arxiv.org/abs/2301.04563](https://arxiv.org/abs/2301.04563) | Large-scale npm client/dependency pairs | **~12%** of dependent packages and **~14%** of their releases are impacted by a breaking change on a non-major (minor/patch) dependency update; **~11.6%** of dependency updates cause client-impacting breaks; clients self-recover (mostly by re-pinning) in about half of cases. This is a **cross-module contract** defect class (closest to D), not an in-repo diff defect, and is largely outside what a diff-scoped slicer can see (the break originates in a *different* repository's release). |

**No qualifying class-distribution number found** for: React-hooks-bug prevalence from a peer-reviewed empirical study (only vendor/blog estimates, flagged above); CommonJS/ESM-interop-specific bug counts (I found detailed *mechanism* writeups — dual-package hazard, named-export synthesis — but no empirical prevalence study); destructuring/rest/default-parameter-specific bug counts as an isolated category in any study (it is submerged inside "API Misuse"/"Logic Error"/parameter-binding-adjacent categories in every taxonomy I found, never broken out). These are reported as **gaps**, not estimated.

## 2. What a reviewer/tracer needs per defect class, mapped to A–J

| Defect class | What must be followed | Capability areas (primary first) |
|---|---|---|
| Undefined/null property access (`Cannot read properties of undefined`) — anecdotally the single most common runtime JS TypeError per multiple vendor debugging guides (e.g. [rollbar.com blog](https://rollbar.com/blog/javascript-typeerror-cannot-read-property-of-undefined/) — practitioner source, not a study; treat prevalence claim as **estimate**) | Trace the value from its origin (param, import, API response, destructure) through member access to see if a guard/narrowing exists on the path | C, then B or D or H depending on origin |
| Wrong value passed across a module/function boundary then dereferenced | Bind call-site argument → callee parameter (incl. defaults/destructuring), then follow the parameter into a member read | B + C |
| Async/promise/callback errors (unhandled rejection, missing `await`, double-resolve, ordering) | Follow control/data flow across the callback/`.then`/`await` boundary and into any closures it captures | H (+ G for Node-specific event-loop/stream specifics) |
| State-management / hook bugs (stale closure in `useEffect`, missing/extra dependency, context value shape drift) | Trace prop/hook value from its declaration into the deferred callback capturing it, and separately into the dependency array that should (or shouldn't) include it | F + H |
| DOM/UI faults (the dominant class in Ocariza's client-side sample) | Trace JS state into the DOM API call/JSX attribute that reads or mutates it | F (React) or C/G (vanilla DOM) depending on stack |
| Type errors / `any`-leakage / wrong narrowing | Needs type information beyond syntax — generics, narrowing join points | I, usually *combined with* B/C for the leak path |
| Configuration/build (Tooling/Config, 27.8% of the TS-ecosystem sample above) | Not a code data-flow problem at all; needs tsconfig/project/compiler-option ownership and monorepo/workspace membership | E primarily; **J** for the parts that are genuinely untraceable (version pins, build scripts) |
| API misuse (14.5% in the same sample) | Depends on the specific API: often a parameter-binding or member-flow problem at the call site, sometimes an async-contract violation (e.g., forgetting to await a thenable-returning API) | B/C, or H, decided case by case — API Misuse is a symptom bucket, not a mechanism |
| Cross-module/dependency breaking changes (npm semver studies) | The break originates outside the diff under review (a transitive dependency's new release); a diff-scoped slicer cannot see it unless it also walks `package.json`/lockfile deltas | D, but largely **out of scope for diff-slicing** — closer to J/supply-chain tooling |
| Logic errors (single-expression, no cross-boundary flow) | By definition this is "not a tracing problem" | J |

## 3. Estimated involvement rates for B, C, D, F, H

These are **estimates** synthesized from the studies above; no single study reports all
five numbers on a common denominator, so I built each from the closest available
breakdown and flag the inference step.

- **B (destructured/rest/default parameters):** No study isolates this as its own
  bucket. Using the TS-ecosystem study's API Misuse (14.5%) and Logic Error (9.7%)
  categories as the most likely homes for parameter-binding mistakes, and discounting
  for the share of each that is unrelated to parameter shape, I estimate **~8–15%** of
  general JS/TS defects involve a destructured/rest/default parameter-binding step
  somewhere on the causal path. **Estimate, low confidence** — no direct source.
- **C (member/property flow):** Anchored to Ocariza's **68%** DOM-interaction-fault
  figure for client-side JS (a property/member-access-heavy mechanism by construction)
  and the "Cannot read properties of undefined" prevalence claims. For the broader
  JS/TS/Node population (not just browser-DOM code) I estimate **~35–50%** of defects
  have member/property flow as a necessary tracing step, since property access is the
  single most common way JS programs move and misuse data. **Estimate, medium
  confidence** (grounded in Ocariza for the browser slice, extrapolated elsewhere).
- **D (cross-module flow):** The npm breaking-change studies put **~11.6%** of
  dependency *updates* at client-impacting-break rates, but that is a different
  population (cross-repo semver breaks) from in-repo cross-module defects (wrong
  import, barrel re-export drift, CJS/ESM interop). I found no study measuring the
  latter directly. Given how central `import`/`require` wiring is to JS/TS defect
  reports generally (per anecdotal reports and the TS-ecosystem study's Tooling/Config
  bucket, 27.8%, which partially overlaps), I estimate **~10–20%** of general defects
  require crossing a module boundary to see. **Estimate, low-medium confidence.**
- **F (React props/hooks):** No peer-reviewed prevalence study found; only vendor/blog
  claims (Cloudflare `useEffect` incident, "70–80% of developers report X" blog
  figures, not peer-reviewed). For React-specific codebases specifically (not
  JS/TS/Node generally), I estimate **~15–25%** of defects in a React-heavy repo
  involve props/hooks/context provenance, based on how central hooks are to modern
  React control flow, but this is a **pure estimate** with no empirical anchor —
  flagged as the weakest-evidenced number in this report.
- **H (async/callback flow):** Triangulating the TS-ecosystem study's explicit
  Async/Event bucket (**3.6%**, likely an undercount since it excludes bugs that
  manifest as e.g. "UI Bug" or "Logic Error" but are *caused* by async ordering) against
  Wang et al.'s Node-concurrency-bug population (**~67% atomicity violations**, but that
  N=57 sample is pre-filtered to concurrency bugs only, so it cannot be reweighted to
  the general population) and Madsen's promise-error taxonomy (no prevalence number), I
  estimate **~10–20%** of general JS/TS/Node defects require following an async/
  callback/promise boundary to see. **Estimate, medium confidence** — the 3.6% figure is
  a floor (undercounts causally-async bugs filed under other symptom labels), the 67%
  figure is not a population estimate.

## 4. Ranked workstreams 2–6 by defect-weighted value

**Ranking: 3 > 6 > 2 > 5 > 4**

1. **Workstream 3 (occurrence/member data-flow limits — A+C) — highest value.**
   Member/property flow is implicated in the largest, best-evidenced share of defects
   found in this survey (Ocariza's 68% DOM-fault figure; "cannot read property of
   undefined" as the most commonly cited single JS runtime error). It is also the
   mechanism underlying a large fraction of "wrong value passed then dereferenced"
   defects regardless of how the value arrived (B, D, F, or H-sourced). **Confidence:
   medium-high** — grounded in the strongest single empirical number in this survey
   (Ocariza 68%), though that number is from a client-side-only sample.
2. **Workstream 6 (Node-specific value checkpoint — D+G+H) — second.** The TS-ecosystem
   study's Tooling/Config (27.8%) and Async/Event (3.6%, likely undercounted) categories,
   plus Wang et al.'s concurrency-bug population, together point at module-resolution
   and async/callback checkpoints as a recurring, high-severity (Cloudflare-outage-class)
   defect source. CJS/ESM interop specifically has well-documented *mechanisms*
   (dual-package hazard, named-export synthesis) but no prevalence study — this
   workstream's value estimate leans more on severity and mechanism clarity than a
   hard percentage. **Confidence: medium.**
3. **Workstream 2 (additional parameter forms — B) — third.** No study isolates
   parameter-binding defects as their own class; the estimated 8–15% share (§3) is
   real but is the least-evidenced of the "well-established" categories, and much of
   its value is likely already captured by workstream 1 ("Done") plus workstream 3's
   member-flow improvements once a value is bound. **Confidence: low-medium.**
4. **Workstream 5 (React.FC/hooks/context provenance — F) — fourth.** React defects are
   plausibly high-frequency in React-heavy repos, but this report found **no
   peer-reviewed prevalence study** to anchor that claim — only vendor blogs and one
   non-peer-reviewed "87 production apps" figure I could not verify to a citable
   source. High plausible severity (stale-closure bugs reach production silently) but
   the lowest evidence quality of the five, so it is ranked below workstreams with
   measured support despite plausible real-world impact. **Confidence: low.**
5. **Workstream 4 (compiler ownership/admission — E, helps D) — fifth, by defect-weighted
   value specifically.** Tooling/Config is the single largest bucket in the TS-ecosystem
   study (27.8%), which on its face argues for ranking this first — but per
   `taxonomy.md`'s own framing, E is explicitly about *admission/ownership*, not
   tracing a defect once admitted, and a large fraction of Tooling/Config bugs are
   **category J** (build scripts, version pins) rather than something a tracer helps
   with even after admission is solved. I rank it last among 2–6 specifically because
   its large raw share is diluted by a high proportion of untraceable (J) content,
   whereas workstreams 2/3/5/6 point at content that is tracing-addressable by
   construction. **Confidence: medium** on the dilution argument, since I could not
   get a clean J-vs-E split within "Tooling/Config" from the source.

## Three key numbers (for the final message)

- **68%** — share of client-side JS faults that are DOM-interaction (property/member-flow)
  faults (Ocariza et al., 502 bugs / 19 projects, STVR 2016).
- **27.8% + 14.5% + 12.4% ≈ two-thirds** — Tooling/Config + API Misuse + Type Error
  share of TypeScript-ecosystem bugs (633 bugs / 16 repos, arXiv:2601.21186, 2026).
- **15%** — share of shipped JS bugs that static typing (Flow/TypeScript) would have
  caught (Gao, Bird & Barr, ICSE 2017, 400 bugs / 389 projects).

## Caveats (dataset bias)

- Ocariza's 68%/79% DOM figures are **client-side-browser-only**; BugsJS is
  **server-side-Node-only**; neither generalizes cleanly to the other, and this report
  did not find a unified client+server sample.
- Bug-tracker-mined studies (BugsJS, TS-ecosystem study) over-represent bugs that got
  a filed, triaged, test-reproducible issue — they under-represent silent logic errors
  and anything fixed in the same commit without a tracked report.
- React hook/useEffect prevalence numbers in circulation are vendor-blog estimates, not
  peer-reviewed measurements; I flag every such number above rather than presenting it
  as established.
- npm breaking-change numbers measure cross-*repository* contract breaks (a different
  defect population from in-repo cross-module defects) and are only loosely informative
  for workstream 6's D-relevant share.
- I could not successfully re-extract BugsJS's own percentage table from the source PDF
  in this session (binary extraction failure) and have deliberately avoided repeating
  the web-search tool's garbled percentage summary (which summed to over 100% across
  categories and is almost certainly a synthesis error, not the paper's actual table).
  This is a named gap, not a silently dropped citation.
