# P3 source-packet pilot architecture review — round 1 of 2

**Verdict: PARK BEFORE EXECUTION — 0 WRONG / 3 SMELL.** This is a read-only
planning review. No observer, worker, compiler, parser, build, dependency tool
or source mutation ran. The declared architecture review cap is two rounds;
this is round 1.

## Binding

- Brief: `/private/tmp/prism-post317-planning/p3/next-increment-brief.md`,
  SHA-256
  `1839b042cd7d492b07b07dddcaec600cda4cf1979b94f39699788dc19aadab26`.
- Implementor prompt SHA-256
  `86ca819e8d30c25fc16dceadc27173dc2d38a0efa3b70435828a8285bbc36e73`.
- Reviewer prompt SHA-256
  `a37159dd9b599834ed9ad0556fc50bb4f8b9d9be43a56f8bb480530f420cf1f7`.
- Production/source reference: accepted-candidate freeze
  `14e86083c2c754ed412d440742bd5763be388e42`, reviewed read-only.
- Public source: Excalidraw
  `0642e72cfa2d9a71198200e52f37399384610ee3`, tree
  `709e9146b0fbd78c3ebf0d77e67143b2fbc43e4a`, complete extracted-source
  manifest SHA-256
  `afb0c3e9172fd66ff805e114a6413de6a9aef00f42c2491e8b67bb5900de0eba`.

## WRONG

None. The brief carefully labels a source-only packet as a diagnostic and
forbids installed-root, semantic, private, production and value claims. I did
not find a constructible promised output that contradicts that stated narrow
contract.

## SMELL

### S1 — the public invocation cannot change the active WS4 decision

The proposed run applies the current worker to
`public_source_archive_without_installed_dependencies`. The active ownership
barrier is a different state: complete original-root dependency/compiler
custody, semantic closure and exact Program/input equality. The public source
archive has no installed dependency tree. Historical installed-root evidence
already found fractional-indexing's 304-file Program (2 repository, 216
dependency, 86 compiler files), while the native root held 628 files; current
production policy requires complete bounded inputs and exact Program/input
equality. Historical acquisition also exceeded existing admission at roughly
814 MiB, 68,997 entries and 249 links.

Therefore every allowed outcome of the new public run leaves the decision
unchanged:

- `spawn_failed`, `deadline_exceeded`, `capture_limit_exceeded`,
  `child_failed` or `invalid_packet` says only that this new source-only
  transport did not yield a packet;
- `packet_observed_unproven` preserves a refusal on a snapshot known to omit
  required dependencies;
- `packet_observed` still lacks installed-root equality and is expressly
  non-authorizing.

This would discriminate the mechanism behind a new source-only attempt, but it
cannot discriminate whether the historical installed snapshot could satisfy
the current gate. It also cannot explain the private `worker_failed` state.
That is useful debugging telemetry, not a WS4 decision census.

**Recommendation:** do not spend the two public invocations merely to produce
a more precise refusal label for a non-equivalent snapshot. Keep P3
`input-blocked` until dependency-root custody/policy is separately decided.

### S2 — reusable transport observability and project measurement are coupled

The sidecar addresses a real tooling gap. `scripts/callable-observations/index.mjs`
currently maps `ETIMEDOUT` and `ENOBUFS` together to `budget_exceeded`, and
maps other nonzero exits plus packet parse failures to `worker_failed`.
`membership.mjs` similarly collapses child errors into a bounded reason set.
Exact spawn/timeout/capture/status/signal/raw-byte custody would improve future
measurements.

That capability does not require an Excalidraw project run. Its behavioral
contract can be proven completely with the proposed bounded child fixtures:
spawn failure, actual timeout, actual stdout/stderr overflow, nonzero exit,
signal, invalid UTF-8/JSON/schema, valid refusal and valid observed packet.
Combining reusable process classification with a currently input-blocked
project outcome makes the deliverable look like project-boundary evidence when
its durable value is generic transport custody.

**Bounded alternative:** if transport observability is itself valuable,
re-scope to a synthetic-only external sidecar increment. Keep the ≤300
non-test /≤450 test /≤750 total limits, exact raw byte/hash/null semantics,
unchanged parser, no repo edits and the two-round implementation review cap.
Run no Excalidraw worker. Preserve the tool for the next separately authorized
installed/private measurement, where its distinctions can affect attribution.

### S3 — fail-first proves the classifier, not the proposed project value

The same-interface collapsed baseline provides meaningful behavioral RED for
the classifier: desired assertions fail on concrete timeout/capture/nonzero/
invalid distinctions while the process and schema succeed. The deterministic
child fixtures can make every output branch testable. That is a sound
classifier oracle.

No corresponding fail-first assertion can show that a source-only
fractional-indexing outcome changes readiness for installed-root ownership.
The brief correctly forbids that claim, so the project invocation has no
decision oracle beyond “the sidecar recorded this run accurately.” Cold/repeat
parity verifies reproducibility, not relevance to the blocked installed
population.

**Recommendation:** treat classifier RED/GREEN as tooling proof only. Require a
future installed-root input manifest and a decision predicate that can differ
based on the observed packet before authorizing a P3 project run.

## Higher-value sequence

1. Finish acceptance/custody for the exact-caller-read slice.
2. Execute the independently accepted P2 fixed-source parameter census. It uses
   the available authenticated 414-file package population to answer open
   destructuring/rest/arrow demand and can select one bounded successor.
3. Keep P3 project measurement `input-blocked`. Bring the existing historical
   acquisition refusal to the controller as a policy decision: retain current
   128 MiB/20,000/zero-link production admission and stop, or separately
   authorize a scripts-disabled locked dependency-custody experiment with
   explicit link/cap/effect policy. Do not rerun source-only as a proxy.
4. Optionally implement the synthetic-only transport classifier as reusable
   measurement infrastructure if that standalone capability is prioritized.

This sequence yields decision evidence from available inputs first and avoids
confusing improved wrapper telemetry with progress on compiler ownership.

## Testability if the controller retains P3

If the controller still values the exact public diagnostic outcome, the
existing plan is bounded and testable after one planning revision:

- state explicitly that its success criterion is only a reproducible
  transport/packet custody record for the source-only snapshot;
- remove any placement in the WS4 ownership decision path;
- make the synthetic classifier proof independently acceptable before public
  invocation;
- require a separate owner statement explaining why the two public runs are
  worth their cost despite leaving installed/private readiness unchanged;
- preserve all current no-install/no-Program-authority/no-retry restrictions.

No implementation or execution is authorized by this review.
