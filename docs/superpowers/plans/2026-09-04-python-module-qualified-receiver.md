# Python module-qualified typed-receiver implementation plan

**Base:** `5e54d48381f329cae370557eeac35bc00ff7b801`
**Branch:** `py-qualified-receiver-owner`
**Design:** `docs/superpowers/specs/2026-09-04-python-module-qualified-receiver-design.md`

1. Add the qualified typed-param/constructor positives plus ambiguity, rebinding, external, wildcard, local-shadow, excluded-import-form, inheritance, and subset controls. Run the focused target on exact base and retain the admissible RED.
2. Add both incremental authority-transition REDs and a stable-authority control before implementation.
3. Correct unaliased dotted structured import locals and make import eligibility kind-neutral while retaining `MemberImport` checks at all unqualified-call consumers.
4. Extend the shared imported-class route and four-field proof-key set for exactly `alias.Class`; project only proven qualified types into receiver classification.
5. Make Python classification recognize the qualified import prefix and fail closed when the enclosing function binds that prefix.
6. Run focused GREEN and inspect the diff under a two-round review cap. Add a RED before each valid review fix.
7. Run complete Python/import-binding targets, static gates, full default and `mcp` suites with totals, release/Tier-A gates, then refresh the handoff and commit explicit paths.
