# S4 — bounded local configuration provenance

Schema16 / producer0.17.0, after D1. Observation only: preserve compiler results,
all prior fields, reasons/closure, runtime/class false and acquisition limits.
Do not install, rewrite config, execute plugins, traverse project references or
infer external absence. Identity domains follow the [H1 contract](2026-09-08-callable-identity-domains.md).

## Source and capture contract

Pinned TypeScript5.9.3 SHA3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675:
parseJsonConfigFileContent at43131 accepts extendedConfigCache as argument8;
getExtendedConfig43633–43669 uses get/set. Replace the absent cache with a recorder
whose get always returns undefined. Set never changes parsing: count writes, retain
at most33 actual results, mark overflow. Never introduce cache hits or replay a
resolver. Retain root text from the original readConfigFile callback. Parse that
retained text for AST anchors only; no second config traversal or host read.

Positive scope is one acyclic chain of at most32 configs, each extends an explicit
relative string within project. Package/absolute/backslash/array extends, cycles,
duplicates, invalid config/options and configDir substitutions are unproven.
Use actual captured extendedResult/extendedConfig and existing config read set.
The explicit candidate wins over appended.json per43609; reconcile against inventory
without I/O. Require exactly one captured write per non-root chain file, no unused,
missing or repeated records. Require all and only successful config reads in chain.

Cache keys apply the measured host case policy to lexical selected paths. Actual
SourceFile.fileName retains lexical selection; full-file anchors use canonical
physical identities. scope.config need not equal the canonical first anchor under
links. Do not conflate source, lexical lookup and cache-key coordinates.

## Packet and validation

`config_provenance:{status,reason,files,extends,options}` is strict and additive.
Observed has1..32 unique root-first full SourceFile anchors; edges exactlyfiles-1,
each source a StringLiteral in files[i] and target exactlyfiles[i+1]. Snapshot hashes,
byte lengths, start0 and config membership must agree. The parser validates bounded
structure; full reproduction binds actual root selection and compiler/source facts.

Options are exactly ordered types,lib,typeRoots,noLib,libReplacement,noResolve,target.
Each row `{name,present,value_sha256,origin}` uses the nearest direct unique
compilerOptions PropertyAssignment. Absent origin is null and hashes
canonical({present:false,value:null}); present hashes the compiler-normalized value.
Pure convertCompilerOptionsFromJson uses the declaring lexical directory and must
match actual parsed.options. Empty arrays, false and legal target enum0 are values;
explicit null resets are outside supported shapes, never omitted-value hashes.
Comments/trailing commas/Unicode use pinned AST spans. Escaped duplicate keys and
nested same-name properties cannot manufacture origins.

Unproven has empty arrays and the earliest supported reason: chain_limit,
unavailable,invalid_config,duplicate_property,unsupported_extends,cycle,missing_target,
selection_mismatch,unsupported_option,value_mismatch. Existing read/parse diagnostics
are inputs, not a new semantic diagnostic pass. Unexpected observer failure becomes
unavailable without changing the compiler result. Empty worker refusals initialize
unavailable. An observed seven-option origin is not whole-config validity, automatic
discovery completeness or Program closure. Preserve historical schemas10–15; helper
bytes participate in producer digest. No source/receiver/cache/write barrier changes.

## Acceptance

Captured exact-D1 contract RED before production edits, with helper-only evidence
labeled separately; current controls for direct/chains/overrides/normalization,
extension precedence, links/case, null/empty/false, malformed/duplicate/unsupported
configs, cap32/33, surplus reads, genuine origin swaps, schema populations/history
and digest. Preserve ordered host transcripts and all prior packet fields on fixed
public source and representative inherited/alias configs. Run full repository gates
on clean stable HEAD, independent review cap2, separate PR and custody receipts.
Historical baseline paths are optional test inputs, never mandatory local scratch
dependencies; the normal suite runs all current-behavior assertions independently.
