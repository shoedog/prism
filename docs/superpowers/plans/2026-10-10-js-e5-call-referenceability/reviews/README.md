# E5 spec reviews (cap: two rounds, both used)

| Round | Head reviewed | Opus-5.5 | gpt-6.1-sol |
|---|---|---|---|
| 1 | `ce6de397` | FIX (`r1-opus.md`) | FIX (`r1-sol.md`) |
| 2 (confirmation) | `c08ea98b` (source `2783899d`) | FIX, one new WRONG (`r2-opus.md`) | FIX, three new WRONG (`r2-sol.md`) |

Briefs: `~/prism-evidence/js-e5/review-brief-r1.md`, `review-brief-r2.md`. Reviewer fixtures: `~/prism-evidence/js-e5/review-r{1,2}/{opus,sol}-scratch/`.

## State at the cap (controller, 2026-10-13): PARKED for an owner decision

Every round-1 finding is closed. Round 2 found four more inputs where main's cross-file `free_single` resolution is a correct static binding and the new head drops it:

- a module's call to a script global from a site the binder grades `Position(...)` (merged namespace or enum body, decorator on a named class expression);
- Source-origin JSX whose tag is a local alias of an imported namespace member (`const F = N.F; <F />`);
- `super(...)` where the class extends an imported object's property that is literally named `super`;
- `import f = N.renamed` where `renamed` is an exported alias of `f`.

Controller probe after round 2 (`base-prism` vs `step2b/bin/head-prism`, synthetic): the ordinary-call twin of the JSX case, `const f = N.f; … f(x)` in a nested function, is also resolved by main and dropped by the head. So the shape is not specific to JSX; it is inside the population arm (b) exists to refuse (a call through a local declaration that shares a name with a unique callable elsewhere).

Classification under the convergence rule: **open-class.** Each round surfaced new inputs of one kind: a static alias chain that prism does not follow, which main resolves only because the spelling happens to match a unique callable. A rule that refuses on absent evidence cannot enumerate these one exemption at a time. No third round and no further fold is dispatched without the owner.

Measured state is unchanged by these findings: 0 CORRECT lost on Excalidraw, the TypeScript compiler, SecBench and the private corpus; none of the four shapes occurs as a CORRECT row in the roughly 500 adjudicated removals.
