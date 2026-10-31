# S2 repair R3b: make B's "unavailable" precise (you are the repair engineer, gpt-6.1-sol)

Same clone, rules and contract (S2-O7) as `repair-r3-brief.md`. Your R3 candidate is the current working tree; its patches are in `~/prism-evidence/s2/repair-r3/`. Keep fix A and the C housekeeping. **Nothing has been adopted, and the STOP rule still holds** (more than 10 X rows lost → report and choose nothing).

## Controller diagnosis of the R3 STOP (from `unavailable-joins-X.json`)
- Of the 5,490 unavailable joins:
  - 2,745 have a relative specifier, 2,424 of them extensionless ordinary imports such as `./helpers` or `./constants`;
  - 2,145 are workspace `@excalidraw/*` specifiers;
  - 600 are bare packages (`eslint`, `clsx`).
- All three categories independently revoke all four gain classes.
- So R3 classifies **resolved static imports whose imported name is not a qualifier identity** as `unavailable`. Under the static-binding contract, those are **`proved absent`**: the static resolver binds the specifier to a specific module, and the name to a non-qualifier export (or to no qualifier identity). That cannot be C's identity.

Before changing anything, verify or refute this diagnosis on a sample from each category. Write down your expectation first, and the evidence that would refute it.

## Required refinement of B (a single failure policy, no exception lists)
- **`proved absent`:** the writer's specifier resolves, using prism's resolver ladder with the writer's own project first (relative, tsconfig `paths`, workspace and package resolution as lane P models them), to a module M. And either the imported or written name does not resolve, within M's static export facts, to any qualifier identity, or M is not a module of any qualifier identity at all.
- **`unavailable`:** resolution genuinely fails, or the export chain past M is unprovable (beyond the cap, escaped spelling, B0 failure). The conservative target set is **only the identities that the unresolved step could reach**:
  - for an unresolved relative path, the identities in modules matching that path;
  - for a forwarding chain broken at module M, the identities that M's static source edges reach;
  - never the global identity set unless the specifier itself is fully opaque.
- Keep sol r2 W2's repros failing closed: the three-hop default forward and the `C` forward must still keep base.

## Measure, separately
1. **R3b-i:** precise absent/unavailable as specified above, where unresolved **bare** package specifiers stay `unavailable`, scoped to the identities in any in-repo module the package name could map to (workspace packages); empty if there are none.
2. **R3b-ii:** R3b-i, plus treating a bare specifier that resolves outside the indexed universe (node_modules, not workspace-linked) as `proved absent`. This is R1b's N3 package-isolation assumption. **Label it NEW ACCEPTED COST** unless you can show it follows from prism's existing modelled resolution. Measure it and do not adopt it.

For each, report X, installed X, R and T, with every changed row CORRECT and ownership agreeing, plus the residual unavailable joins that revoke gain classes, attributed by category.

## Gates (on R3b-i only if it passes the STOP rule; otherwise stop after measuring)
- nextest `--features mcp`, doctests, fmt and clippy, the advisory mutgate, the Tier-A matrix (skip quick), S1b-4 controls byte-identical, lane-P rows unchanged.
- Rebuild `target/s2-plan/bin/head-*`, rebind the BUILD-MANIFEST, and refresh VERIFICATION.md.
- Write `~/prism-evidence/s2/repair-r3b/R3b-src.patch` (relative to `75a35a5e`) and `R3b-docs.patch` (relative to `ecdb6b0f`).

## Final message
- whether the diagnosis held;
- R3b-i and R3b-ii yields with labels;
- residual attribution;
- the gates;
- files with commit messages;
- what you did not verify.
