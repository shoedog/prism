# Implementor handoff

Start in `/Users/wesleyjinks/code/prism-pkgres`, branch plan/workspace-package-resolution, exact base 4e592daa. Read SPEC §0, CENSUS, BUILD-MANIFEST, MEASUREMENTS, VERIFICATION and HANDOFF before transferring claims. Source/tests are an uncommitted prototype; docs are a separate proposed controller commit. No Git writes or F access by workers.

Preserve this partially reviewed artifact. Repair cap three attempts per gate/measurement; enumerate first-error gates before state-changing retries. At cap classify convergence/open-class failure before acting; do not restart the implementation.

The immediate next slice should close JS secondary resolution, one location class at a time, using the existing P1 complete priority-pass machinery. Native controls must include closer JS with outer declarations, @types, custom roots, package metadata, opaque/missing ancestors and symlink identity. Then add exports arrays/version ranges and .mts/.cts/config ownership as separate increments. Do not coalesce all full-Node semantics into a large review slice. Implement declared npm/Yarn/pnpm/lerna inventory as a discovery layer that cannot bind uninstalled names.

Reuse `Resolver::package_in`, snapshot package bytes/link/topology, and P2's export-hop callback carrying caller config. A declaration or generated output is not a source implementation. Retain old paths refusals until a native earlier-rung proof allows fallback; do not use package names to route same-name functions.

For every new path, add positive plus negative/edge witnesses, same-environment pre-change RED where meaningful, full MCP suite and appropriate gates. Pin TS 5.9.3 via PRISM_TYPESCRIPT. Required source-based native diagnostics retain bytes and actual owner, never independent nearest-config guessing. Controller executes CONTROLLER-pkg.sh on private F, receives aggregates only, then decides any adoption. Scratch S2 changes are never copied into production.
