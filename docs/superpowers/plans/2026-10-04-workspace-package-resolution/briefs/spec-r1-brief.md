# Lane PKG spec review, round 1 of 2: JS/TS workspace package-entry resolution

## Your role
You are a senior/principal engineer reviewing for the project's long-term outcome. Be rigorous about finding real defects, and never invent them. Read-only: no git writes, keep disk use lean, and delete `target/` if you build.

## Goal the review serves
Prism resolves JS/TS bare specifiers to **in-repo workspace package entries** under TS 5.9.3 semantics for the writer's own project. Covered: `package.json` `exports` conditions and subpaths, `types`/`main`/`module`, the index fallback, workspace discovery, and `node:` builtins classified as external.

**Positive proof only:** an ambiguous case stays unresolved, and generated output never redirects to source. There must be **zero false module bindings, zero false Exact edges and zero lost base edges.** Exact is a static-binding grade (repo CLAUDE.md).

Lane S2 (#343, parked) consumes this. Under owner rule S2-O9, a writer import that neither prism nor TS can resolve is out of model for S2's refusal joins. **A false "unresolved" in PKG therefore becomes a silent false Exact in S2**, so both directions matter:
- a false binding;
- a missed binding that TS makes.

Tag each finding MATERIAL or IMMATERIAL to this goal.

Measured so far:
- public X, installed X, R and T streams identical to main (installed X gains one correct module proof);
- private F: 0 changed, 0 lost;
- 58 native controls: 42 CORRECT additions, 0 lost.

## What to review
- **Branch:** `review-pkg` = origin `plan/workspace-package-resolution` `c89bc5b7` (PR #344). Read SPEC (§0), IMPLEMENTOR, MEASUREMENTS, CENSUS, PROBES, OQ and VERIFICATION in `docs/superpowers/plans/2026-10-04-workspace-package-resolution/`.
- **Prototype:** `git diff origin/main origin/proto/workspace-package-resolution` (`92c1d0bc`). Main files: `src/js_packages.rs`, the `src/js_paths.rs` integration, the cache wiring, and `tests/integration/js_packages_test.rs`.

## Questions
1. **Resolution soundness against TS 5.9.3.** For each supported `moduleResolution` (node10, node16/nodenext, bundler), check:
   - `exports` condition order (`types`, `import`/`require` by the writer's mode, `default`), subpath patterns, null targets and array targets;
   - precedence of `paths` vs package resolution, and the self-reference (package name) import;
   - `typesVersions`, and extension substitution (`.js` → `.ts`/`.d.ts`).

   Can any input bind to a module TS would not choose?
2. **The unresolved boundary.** Where does PKG return "unresolved" while TS binds? Enumerate the unsupported features (OQ-3/4). For each, ask whether it can occur on a writer import S2 would need to refuse. Name the cases that need to be supported before S2 lands, versus those safely deferred.
3. **Workspace discovery.** npm/Yarn `workspaces`; pnpm/lerna (are these unsupported?); symlinked `node_modules` canonicalising into source; and competing same-name packages.
4. **Cache invalidation.** Do `package.json`/tsconfig edits invalidate the right facts?
5. **Measurement admissibility and tests.** Complete population; native-witnessed controls; would each test fail on main?
6. **IMPLEMENTOR.** Is the dispatch from the committed prototype unambiguous?

## Output
Write the review to `REVIEW_OUT`. For each finding:
- **Tag:** WRONG (name the input or state and the incorrect result) or SMELL, plus MATERIAL or IMMATERIAL.
- **Evidence:** file:line, and a minimal fixture, labelled reproduced (ideally against the TS 5.9.3 oracle at `/Users/wesleyjinks/prism-evidence/native-positional-gap/gate-inputs/typescript-5.9.3/package/lib/typescript.js`) or static.
- **Fix:** a recommended fix plus alternatives, with tradeoffs.
- **Self-critique:** when the finding would not apply.

End with a verdict:
- **APPROVE**;
- **FIX**: closed, enumerable fixes;
- **REJECT**.

List what you did not check.
