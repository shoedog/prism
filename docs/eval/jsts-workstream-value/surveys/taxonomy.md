# Shared taxonomy for the JS/TS/React/Node defect and CVE surveys

## What prism is
Prism is a static code-slicing and navigation tool for defect-focused code review. Given a diff, it traces the code relevant to the change using a code property graph: AST, call graph, data-flow (def-use) graph and CFG. It supports taint-style source→sink tracing, callers/callees, and the blast radius of a change. It is aimed at reviewers and LLM agents, which use it to decide whether a change introduces a defect.

## Capability areas (map each defect/CVE class to the ones a tracer would NEED)
- **A. Intra-function data flow:** def-use within a function, loops, same-line occurrences, reassignment.
- **B. Parameter / argument binding:** values flowing from call-site arguments into callee parameters. Covers default, optional, destructured (`{a, b}`) and rest parameters, and unparenthesized arrows.
- **C. Member / property data flow:** `obj.a.b` reads and writes, dynamic keys `obj[k]`, object spread and merge. This is where prototype-pollution sinks live.
- **D. Cross-module resolution:** imports and exports, re-export barrels, default exports, CommonJS `require`/`module.exports`, ESM/CJS interop, package `exports`, tsconfig paths, dynamic `import()`.
- **E. Real-project compiler ownership:** knowing which tsconfig/project and compiler options govern a file, mixed JS/TS projects, `.mts`/`.cts`, monorepo or workspace membership.
- **F. React-specific provenance:** props flowing into components (incl. `React.FC`), hooks (`useState`, `useEffect` deps, `useContext`, custom hooks), context providers to consumers, JSX attributes (e.g. `dangerouslySetInnerHTML`, `href`).
- **G. Node runtime-specific:** `child_process`, `fs`/path handling, `http` request objects, streams, the event emitter/callback/promise async flow, `process.env`, JSON/`require` loading.
- **H. Async / callback / higher-order flow:** callbacks, promises/async-await, event handlers, closures captured by deferred callbacks, middleware chains (express/koa).
- **I. Type-level / type-driven:** bugs only visible with type information (`any` leakage, wrong narrowing, generics).
- **J. Not a tracing problem:** configuration, dependency versions, build tooling, regex complexity alone (ReDoS needs pattern analysis plus reachability), crypto parameter choice, docs, or a purely local logic error in one expression.

## Candidate workstreams (what the owner must prioritise among)
1. Exact parameter-token binding repair. **Done.**
2. Additional parameter forms: complex defaults, destructured/rest, unparenthesized arrows. Covers B.
3. Occurrence and member data-flow limits: exact loop/member-read provenance, member expressions beyond a narrow asserted shape, same-line collisions. Covers A and C.
4. Real-project compiler ownership and admission. Covers E, and helps D.
5. React.FC and hook/context provenance. Covers F.
6. Node-specific value checkpoint: ESM/CJS interop, `require`, package exports, module suffixes, JSON loading. Covers D, G and H.
7. Production readiness and accuracy measurement. Covers no capability; it is trust and packaging.

## Output requirements (both surveys)
- **Sources:** cite every quantitative claim with its URL and year. Prefer empirical studies, curated datasets (GitHub Advisory Database, NVD/CWE, Snyk/OSV, BugsJS, BugSwarm, ManyBugs-JS, "TypeScript bugs" and "React bugs" mining studies) and vendor annual reports. Mark estimates as estimates.
- **Class table:** defect/CVE class | share/prevalence (with source) | severity/impact | capability areas needed (A–J, primary first) | workstream(s) that would most help | confidence.
- **Capability roll-up:** an estimated share of defects or CVEs whose *tracing* depends mainly on each of A–J. Explain the method, including how double counting is handled.
- **Defect-risk notes:** which classes are high-severity even if rare.
- **Caveats:** dataset biases. For example, CVE databases over-represent library input validation, while bug studies over-represent crashes and test-visible bugs.
- Keep the whole thing at most about 2,500 words, plus the tables.
