# S2 repair R1b: attribute the F3 yield loss, measure a sound narrowing, fold F4–F8 (you are the repair engineer, gpt-6.1-sol)

Same clone and rules as `/Users/wesleyjinks/prism-evidence/s2/planning/repair-r1-brief.md`: `/Users/wesleyjinks/code/prism-s2-plan`, no git writes, never open private F. Your previous handback is in `REPAIR-R1.md`, and patches are in `~/prism-evidence/s2/repair-r1/`. The controller has made no owner decision on F3. **Do not adopt any F3 option.** Measure them and leave the choice open.

## 1. Attribute the F3 loss
Report exactly which X source sites trigger the conservative revocation that zeroes all 132 rows: file:line, the syntax, and why each is "opaque or unresolved". The controller's guesses are:
- `packages/excalidraw/i18n.ts:101`, `` import(`./locales/${code}.json`) ``;
- `typeof import("…")` in type positions (`setupTests.ts:36,106`), which are not runtime imports;
- non-literal or package specifiers such as `excalidraw-app/collab/Collab.tsx:508` and `TopErrorBoundary.tsx:59`.

Confirm or refute each with the evidence. Before you probe, write what you expect to see.

## 2. Measure F3 narrowings (each one separately, no adoption)
- **F3-N1 (type positions):** `import()` in TS type positions (`typeof import(...)`, `import("x").T` in a type) is not a runtime channel. Exclude it.
- **F3-N2 (template with non-code suffix):** a template or concatenated specifier whose static suffix ends in a non-JS extension (`.json`, `.css`, `.svg`, …) can only load non-code modules, which cannot be a qualifier class or namespace.
  - Argue its soundness under ESM and under bundler semantics, including `../` traversal inside the hole and query or fragment suffixes. If it is not sound, say so and do not measure it as sound.
  - Restrict any revocation to the static-prefix directory only when the soundness argument holds.
- **F3-N3 (resolved package specifiers):** a literal specifier that resolves outside the indexed universe (a node_modules package) cannot return an indexed class unless the package re-imports repo code. State the assumption, and whether it is a new accepted cost.

Report a yield table for: F3 full, F3+N1, F3+N1+N2, and F3+N1+N2+N3. Show X, installed X, R and T, with every changed row CORRECT and ownership agreeing. For each row, mark it as either **no new accepted cost** (sound) or **new accepted cost** (and name the cost).

## 3. Fold F4–F8 independently of F3
These are needed whatever the F3 decision. Fold each against the SPEC §2a channel table, using the reviewers' recommended fixes (`~/prism-evidence/s2/review/spec-r1-{opus,sol}.md`). For each one:
- add a regression in both grammars;
- add a mutant;
- measure its yield cost on top of F1+F2 (without F3).

The STOP rule still holds: if any single cut costs more than 10 X rows, or the total falls below 110, implement it in isolation and report the options without choosing.

## 4. Deliverables
- Separate patches in `~/prism-evidence/s2/repair-r1b/`:
  - `F1-F2-F4-F8.patch` (the base candidate without F3);
  - one patch per F3 variant.
- The tiered gates on the base candidate:
  - nextest `--features mcp`;
  - fmt and clippy;
  - the advisory mutgate;
  - the Tier-A matrix;
  - S1b-4 controls byte-identical;
  - lane-P rows unchanged.
- Update REPAIR-R1.md, or add REPAIR-R1b.md.

Final message:
- the attribution table;
- the F3 variant yield table with soundness labels;
- the F4–F8 per-fix yields;
- the gates;
- what you did not verify.
