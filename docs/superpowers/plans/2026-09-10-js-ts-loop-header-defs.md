# Bounded JS/TS loop-header definition repair

Base: merged PR310 e8e4c77e7f634077f2090bf57e59f665ac0aacab.

Both loop-header lvalue helpers currently scan every direct child, so a plain
identifier iterable becomes a definition. Restrict these helpers to the grammar's
`left` field, preserving existing identifier/destructuring handling. Missing or
unsupported left forms must not fall back to scanning RHS/body. Preserve RHS Use
nodes and existing non-JS behavior. Do not expand supported member bindings or
parameter occurrence syntax. Callee-body parameter lookup is a separate slice.

Root owns AST tests/repair/cache/design. loop_cpg_tests owns synthetic CPG tests
and their module registration. Capture RED before implementation; test query and
manual paths, exact byte spans, in/of/await, declaration/bare/pattern left sides,
complex/member RHS, Unicode/multiline and non-JS controls. Cache version80 must
invalidate serialized graphs containing the old false definitions. Navigation
call-edge cache remains unchanged because call resolution is unchanged.

Verification: focused tests, full Rust default/MCP/owner-audit suites, Python,
observers/helpers/authority, all examples. Fresh release rebuild before Tier-A
matrix and quick; keep pre-existing harness failures and same-environment base
controls, no rebaseline. Review cap2; closed findings corrected in place, open-class
findings parked at cap. Commit/push/open PR; no automatic merge. Publication must
respect the aggregate-only private-corpus boundary; no raw private source.
