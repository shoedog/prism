# Survey A: general (non-security) defects in JS/TS/Node/React open-source software

Read `/Users/wesleyjinks/prism-evidence/survey/taxonomy.md` first, and use its capability areas (A–J) and workstreams (1–7) exactly.

**Scope:** empirical studies of bugs in JavaScript, TypeScript, Node.js and React open-source projects. Use web search and read the papers or abstracts. Candidates include:
- BugsJS (Gyimesi et al.);
- "A Study of Real-World Data Races/Concurrency Bugs in Node.js" (Wang et al.);
- "To Type or Not to Type" (Gao et al., the share of bugs detectable by TS/Flow);
- TypeScript bug studies (e.g. "An Empirical Study of Bugs in TypeScript projects", Bogner/Merkel 2022);
- React-specific bug studies (hooks misuse, `useEffect` dependency bugs, state-management bugs);
- "Understanding and Detecting Callback Bugs / Promise bugs in JS" (Madsen et al., promise graph);
- JS bug taxonomy studies (Ocariza et al., client-side JS faults: DOM-related);
- npm breaking-change and dependency studies;
- and any 2023–2026 LLM-era studies of defects in JS/TS code.

**Deliver:**
1. The defect-class distribution, with sources. Examples of classes: undefined/null property access, wrong data passed across module/function boundary, async/promise/callback errors, state-management and hook bugs, DOM/UI, type errors, configuration/build, API misuse, and logic errors.
2. For each class, what a reviewer or a static tracer needs to follow to see that a diff introduces it, mapped to A–J. For example, a "value passed into a function then dereferenced" defect needs B plus C; a "stale closure in a `useEffect`" defect needs F plus H.
3. Specifically, estimate how often defects involve:
   - destructured/rest/default parameters (B);
   - member/property flow (C);
   - cross-module flow (D);
   - React props/hooks (F);
   - async/callback flow (H).
4. A ranked list of workstreams 2–6 by **defect-weighted value**, with your reasoning and confidence.

Write the report to `/Users/wesleyjinks/prism-evidence/survey/survey-defects-sonnet.md`. Your final message should be the ranked list plus three key numbers.
