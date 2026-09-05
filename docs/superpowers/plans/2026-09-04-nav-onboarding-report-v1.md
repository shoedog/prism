# Navigation onboarding report v1 implementation plan

Exact base: `90c522b04ff16ebc076ce85a4f8df5f7f2da4f1f`

1. Commit the design, roadmap selection, predecessor merge custody, and live handoff.
2. Add navigation and CLI contract tests; run them RED and commit the failing contract.
3. Implement the typed bounded report, renderers, CLI grammar, and create-new output;
   run all focused tests GREEN and commit.
4. Update active user/operator documentation and the handoff; run self-review round 1,
   fold the bounded findings, then run round 2. Stop at the declared cap unless the
   findings are demonstrably converging.
5. Run format, diff, all-target MCP check/Clippy, full default and MCP suites, and the
   required release/Tier-A gates. Record totals, exclusions, candidates, and custody.
6. Commit, push, open a PR, merge only when GitHub is green, verify the exact merge
   commit, and rebind root before choosing another increment.
