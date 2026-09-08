# H1 identity-domain hardening

H1 separates canonical source/target identities from lexical compiler-generated
lookup addresses. It adds a reviewed contract and an unwired pure helper, not a
producer/schema or closure-policy change. All current public packet fields and
producer hash remain exactly equal to S2:
`7ea9f31c2da12979fcb1d395492ed6ec0c0564fdc1b6cce27c75d79f71c0a369`.

The trigger was a real unpublished S3 regression: an in-root config directory link
caused its synthetic containing-file address to be canonicalized, while the parser
derived the correct lexical address. Exact S2 retains its ordinary packet; S3's
raw worker retains it too, but S3's parser rejects the coordinate mismatch. This
ruled out a compiler/inventory failure. The original artifact remains archived;
there was no discard/restart. S3 resumes under the closed identity contract, with
its integration tests and full gates still required.

Review history is explicit: S3 ordinary cap2, one disclosed converging numeric-index
verification extension, then another identity case caused design escalation to H1.
H1 used two bounded source/design-review rounds and independently ACCEPTed96/100.
Its seven helper/compiler characterization tests pass. They are not S3 production
RED and do not claim D1 library-hook or S4 config integration behavior.

The helper keeps case and aliases for absent synthetic filenames, normalizes lexical
dots, and rejects unsafe/refused/outside/compiler-root coordinates without I/O. Pinned
Program callbacks confirm the lexical config-directory rule for type/lib addresses.
The [contract](../../superpowers/specs/2026-09-08-callable-identity-domains.md) records
the finite integration matrix and separate canonical source/target obligations.

Full repository gates passed on clean implementation12aa795:402 observer,4017 Rust,
4207 MCP,18 helpers,40 authority controls; doctests included, one known ignored
test per Rust run. fmt/diff passed. The [gate receipt](2026-09-08-callable-identity-gates.json)
records the raw archive hash. All1229 public source files and every public packet
field remain unchanged from S2. [PR283](https://github.com/shoedog/prism/pull/283)
is published against S2; stacked feature-base
PRs do not trigger the current main-only PR workflow. Runtime/class authority remains
false; react-scripts and all14 module-literal gaps remain unresolved. No receiver
recall gain, app change, dependency install or outside-absence proof is claimed.
