# Survey B: CVEs and security advisories in the JS/TS/Node/React ecosystem

Read `/Users/wesleyjinks/prism-evidence/survey/taxonomy.md` first, and use its capability areas (A–J) and workstreams (1–7) exactly.

**Scope:** security vulnerabilities in npm packages, Node.js applications and libraries, and React/Next.js frontends, from roughly 2018 to 2026. Use web search. Useful sources:
- the GitHub Advisory Database (npm ecosystem) and OSV;
- NVD CWE distributions for npm;
- Snyk "State of Open Source Security" and similar reports;
- academic studies, e.g. prototype-pollution studies (Silent Spring / "Probe the Proto"), ReDoS in npm, "Small World with High Risks" (npm ecosystem), injection in Node.js (Synode, NodeMedic), XSS in React apps, and SSRF/path traversal in Express apps.

**Deliver:**
1. The CWE/class distribution for npm advisories: prototype pollution, ReDoS, command injection, path traversal, XSS, SSRF, code injection/eval, deserialization, auth/logic, and supply-chain/malicious packages (treat this last one separately; it is not a tracing problem). Give counts or shares with sources.
2. For each class, the **source-to-sink shape**, i.e. what a static tracer must follow to connect attacker input to the sink. For example, prototype pollution needs C (dynamic keys and recursive merge) plus B and D. Map each shape to A–J and to workstreams 2–6.
3. **Library vs application split.** Most advisories are in libraries, where the "source" is an exported API parameter. Explain what that implies: parameter binding (B) and cross-module exports (D) become the entry points.
4. **React/Next specifics:** `dangerouslySetInnerHTML`, `href` with `javascript:`, server actions/SSR injection, and the props/hooks provenance needed (F).
5. A ranked list of workstreams 2–6 by **CVE-weighted value**, with your reasoning and confidence.

Write the report to `/Users/wesleyjinks/prism-evidence/survey/survey-cve-luna.md`. Your final message should be the ranked list plus three key numbers.
