# Python imported typed-receiver implementation plan

**Base:** `c220525c6746d635d99a7a084791cfad4f0276d9`
**Branch:** `py-imported-receiver-owner`
**Design:** `docs/superpowers/specs/2026-09-04-python-imported-typed-receiver-design.md`

1. Add the alias/dotted-module positive regression and the ambiguity, external-collision, function-local, and inheritance controls. Run the focused test and retain the base RED output.
2. Move the existing structured import-binding build ahead of call-site extraction for full and subset builds. Extend `ReceiverCtx` with the per-file structured bindings and preserve the function-local/wildcard barriers.
3. Add a private imported-receiver route that proves one eligible member import, one matching indexed Python module, and one clean class. Generalize the direct-method helper to accept the proven defining file.
4. Route Python recovered receivers through `NotImported`, `Proven`, or `Blocked`; never let an imported type fall into global bare-owner Exact resolution.
5. Run focused tests, format/check, the full suite with totals, release build, Tier-A matrix-only, and Tier-A quick. Record all exclusions and regressions without re-baselining.
6. Refresh the lane handoff, commit explicit paths, and prepare the MR handoff. Review cap: two rounds; a recurring proof-completeness finding at round two parks the slice for broader design.
