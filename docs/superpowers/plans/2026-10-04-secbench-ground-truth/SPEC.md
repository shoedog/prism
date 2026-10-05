> Historical R0 record. R1 changes ground truth, attribution and conversions; use [R1.md](R1.md) for the current result and verification. Original evidence remains immutable.

# MEAS-A measurement contract

Authority: `/Users/wesleyjinks/prism-evidence/meas/secbench/plan-brief.md`.
No product changes, Git writes, network, exploit execution, or private corpus access.

Pinned SUT: clean `plan/secbench-ground-truth`, HEAD
`4e592daa7858a195eb3a9eb77c83dfbc763b49fa`. Build locally with
`CARGO_NET_OFFLINE=true cargo build --release`.
Pinned SecBench: `5d362353550a8baa42bba34edd26e5fb86d41b60`.
Input acquisition manifest: 600 entries, 583 ok, 11 multi_or_no_dep, 6 pack_failed.

Retain the 600-entry denominator. Acquisition failures and unavailable ground truth
are separate from the five Prism outcomes. Validate sink file, line and enclosing
callable. A basename repair is admissible only when unique and must be disclosed.
Exported API source derivation must cite exploit require/call and the package
export/definition with parameter ordinals and supplied argument expressions.
Ambiguity stays `gt_unavailable`; never turn a guessed source or sink into a miss.

Measure syntax with the locally acquired TypeScript 5.9.3 parser, authenticated by
SHA256 `3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675`.
Syntax counts do not establish value flow. Path prevalence requires an admissible
source-to-sink path; unavailable paths retain an unknown stratum.

Invocation debugging cap: three attempts per issue. Review/fix cap: three rounds.
Every diagnostic has an expectation, falsifier and observed result in PROBES.md.
Invocation errors are `prism_error`. Line or function inclusion alone is insufficient
to prove the requested parameter-to-sink value tracing. Unresolved attribution
must be explicit rather than guessed or credited as a convertible entry.

Workstreams: B -> 2; A/C -> 3; E -> 4; F -> 5; D/G/H -> 6; I/J -> unassigned.
Single-workstream convertibility requires evidence that no subsequent missing
capability remains; first-break counts alone are only opportunity counts.
Severity weights are a declared sensitivity model, not CVSS or literature data.

## Operational definitions and reusable entry point

`PYTHONDONTWRITEBYTECODE=1 python3 -m eval.secbench` is the complete offline
command from the repository root. Build + byte validation + inspection + full
measurement + adjudication + JSON/Markdown are included. `--sut-repo` permits a
separate exact 4e592daa SUT checkout after the controller commits the harness.
The harness revision and measured SUT revision must remain distinct.

`traced` requires per-root parameter Def identity and terminal occurrence/member
identity. Unique synthetic terminal nodes are admitted only when AST name/path
identity is unambiguous on the declared line. `reached_function_only` is a
callable-span result. `partial` is either a selected witness BoundaryExited/
Sanitized verdict or a weaker union-seed frontier continuation; it is not a
fully reconstructed parameter-specific path. `not_reached` has neither such
continuation nor a reached enclosing callable. Decisive query failures stay
`prism_error`, even where source inspection explains a capability cause.

First-break attribution uses declared A-J mapping; manual sample labels replace
heuristics and their agreement is reported. Unresolved cases/errors remain
unassigned. Complexity-of-regex conclusions (J) are outside reachability.

Path syntax presence is established by an exploit-fed entry binding or manual
inspection. A compiler-clean package without the construct proves an internal
negative. Unrelated syntax, missing paths and parser gaps retain an unknown
stratum. Broad callable-span counts and shipped-package prevalence are separate
screens and never substituted for payload-path prevalence.
