# Experimental contextual owner navigation

This opt-in grants a static Exact edge for the bounded direct `Handler<Props>`
receiver described in the [proof contract](superpowers/specs/2026-09-09-callable-executable-owner-proof.md).
It does not establish the runtime object's identity, override behavior, or general
React.FC support. Ordinary navigation, review and targets remain unchanged.

The [real-repository value checkpoint](eval/receiver-closure/2026-09-09-owner-value-checkpoint.md)
found both approved real roots refused before compiler acquisition. Synthetic
support is measured; practical real-receiver gain is not yet established.
The [admission diagnostics verification](eval/receiver-closure/2026-09-09-owner-admission-diagnostics.md)
reproduces those refusals with bounded phase and census reports, without relaxing
admission.

Use a trusted Node executable on PATH and an explicitly selected, already-present
TypeScript 5.9.3 `lib/typescript.js`. Its required SHA256 is
`3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675`.
There is no compiler discovery, download, package install or execution of project
scripts. The selected compiler must retain its authenticated standard-library files.

```sh
prism nav --no-cache \
  --owner-compiler /absolute/path/to/typescript/lib/typescript.js \
  --owner-config tsconfig.json \
  callees --repo /absolute/path/to/project --symbol run --file src/app.ts \
  --confidence exact --format json

prism-mcp --repo /absolute/path/to/project --eager --no-cache \
  --owner-compiler /absolute/path/to/typescript/lib/typescript.js \
  --owner-config tsconfig.json
```

Both owner flags are required together; compiler path is absolute, config path is
repository-relative without traversal. Cache directories are refused, not ignored.
MCP requires eager startup and the default `warn-only` refresh-policy option; its
separate owner runtime actually reacquires **before every tool call**, including
`refresh_index`. It never uses the ordinary stale-on-auto-refresh-error fallback.
Each successful acquisition replaces the previous session. On failure no old proof
is served; the tool returns `build_failed` and the next call retries. Startup failure
is a process error. Refresh stale-path fields describe ordinary filesystem metadata
observations, not compiler/config closure authority.

## Admission diagnostics

Acquisition failures retain `owner acquisition failed: <reason>`. CLI/API errors
and eager MCP startup errors append `; owner_admission=<JSON>`. Served MCP
`build_failed` payloads instead have a separate `owner_admission` object alongside
their unchanged, bounded `cause`. The internal typed error supplies that object;
the server does not parse its text to recover authority or diagnostics.

Schema `prism.owner-admission/1` always has `authorizes_runtime_edge: false`.
`inputs` reports the loaded file count, JS/TS file and byte counts, language counts,
TypeDatabase presence, fixed limits and comparison booleans; it is null if loading
failed. Counts describe loaded inputs, not all filesystem/dependency files, and
do not certify parsing, closure or receiver ownership. Both a count excess and a
language mismatch may be visible even though only the first gate ran.

`failed_phase` and `phases` describe actual progress through load_inputs,
select_inputs, prepare_inputs, locate_inputs, compiler_evidence, owner_mapping,
and session_build. Prior phases are completed; later phases are not_reached.
The compiler_evidence phase groups worker acquisition, reproduction, closure and
census checks: failure does not prove each internal step ran, or that a compiler
process started. The report does not inventory every downstream barrier.

Only aggregate facts appear, not source paths or raw compiler diagnostics.
The optional served MCP report has a 2048-byte serialized ceiling; an oversized
report is omitted with `owner_admission_omitted: true`, never truncated into invalid
JSON. Ordinary error text keeps its existing 256-byte clamp. Selection syntax,
cache policy, MCP configuration/canonicalization and other pre-acquisition errors
remain outside this report. Neither flags nor default-path behavior changes.

## Deliberate limits

- Source-checkout-bound experiment: the exact worker assets must still exist at the
  checkout path used to build Prism. Missing or modified assets refuse. Portable
  binary packaging is not implemented by these flags.
- Only repositories whose loaded sources are JS/TS/TSX, with no TypeDatabase, are
  admitted. Mixed-language typed CPG enrichment needs a separate proof. No indexed
  source is removed to make the Program census agree.
- The existing fixed acquisition budgets, closure-v3, rejected links, and exact
  Program/index census remain mandatory. The owner census is bounded to 512 inputs
  and 8 MiB; other existing compiler-worker limits may refuse sooner.
- Unsupported receiver shapes may produce a valid session with no new owner edge.
  Incomplete acquisition, closure, missing config/compiler, or input disagreement
  is an error, never silent fallback. Error text includes a stable refusal category
  such as `compiler_unavailable`, `producer_changed`, or `owner_requires_js_ts_only`;
  worker refusals do not promise full TypeScript diagnostic text.
- Every call pays for full loading, two compiler observations and fresh CPG/index
  construction. This favors correctness for a small experimental fixture, not
  production-scale throughput. No real-receiver recall gain is claimed here.
- `react-scripts` remains intentionally unresolved. React.FC, broader provenance,
  lazy activation, incremental reuse and persisted owner results remain deferred.

Library callers set `NavOptions::owner = Some(OwnerOptions::new(compiler, config))`
and `no_cache = true`, then call `nav_session`. The returned handle is a historical
snapshot: reacquire through `nav_session` for current inputs. Options are selection
only; proofs remain private, epoch-bound and non-deserializable.
