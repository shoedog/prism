# S2-0 measurements — exact main 4e592daa

[MEASURED] Current-main census command:
`python3 docs/superpowers/plans/2026-10-03-s2-import-qualifiers/probes/public.py --out target/s2-plan/current-main`.
Output: `target/s2-plan/current-main-run.log`; source-bound per-site records,
source hashes and receipts: `target/s2-plan/current-main/{X,installed-X,R,T}/`.
[READ] After handback, complete streams and candidate records are compressed in
`target/s2-plan/evidence.tar.gz`, indexed by `evidence-index.json`.

[MEASURED] Each cell is **low-grade or multi-target sites / native-proven
in-repo callable implementations**. Counts are disjoint and sum to the total.
F has not been measured by the planner; the controller wrapper is ready.

| Mechanism | X | Installed X | R | T | F |
|---|---:|---:|---:|---:|---|
| Default object | 7 / 0 | 7 / 0 | 0 / 0 | 0 / 0 | unmeasured |
| Default class | 0 / 0 | 2 / 0 | 0 / 0 | 0 / 0 | unmeasured |
| Named object | 0 / 0 | 0 / 0 | 0 / 0 | 24 / 0 | unmeasured |
| Named class | 299 / 299 | 302 / 299 | 0 / 0 | 365 / 365 | unmeasured |
| Class-instance exports | 3 / 0 | 3 / 0 | 0 / 0 | 16 / 0 | unmeasured |
| export = | 0 / 0 | 26 / 0 | 0 / 0 | 0 / 0 | unmeasured |
| CommonJS module.exports / other exports | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | unmeasured |
| Re-exported namespace | 0 / 0 | 42 / 0 | 0 / 0 | 157 / 157 | unmeasured |
| require object | 22 / 0 | 22 / 0 | 0 / 0 | 18 / 0 | unmeasured |
| require destructure | 0 / 0 | 0 / 0 | 0 / 0 | 0 / 0 | unmeasured |
| Call-result exports (E5) | 55 / 4 | 55 / 4 | 0 / 0 | 969 / 0 | unmeasured |
| Declared namespace | 2 / 0 | 6 / 0 | 1 / 0 | 1,485 / 1,485 | unmeasured |
| Function with members | 35 / 0 | 35 / 0 | 0 / 0 | 0 / 0 | unmeasured |
| Other / unavailable shape | 129 / 0 | 52 / 0 | 29 / 0 | 133 / 79 | unmeasured |
| **Total** | **552 / 303** | **552 / 303** | **30 / 0** | **3,167 / 2,086** | unmeasured |

[MEASURED] Total site populations are X 19,219; installed X 19,219; R 953; T
61,712. All S2-associated sites, including already-single-Exact rows: 561 / 561 /
30 / 3,168. The low populations split into **dropped / NameOnly / multi-target**:

| Corpus | Dropped | NameOnly | Multi-target | Native-certified base-R3 identity filter candidates |
|---|---:|---:|---:|---:|
| X | 546 | 0 | 6 | 0 |
| installed-X | 546 | 0 | 6 | 0 |
| R | 30 | 0 | 0 | 0 |
| T | 3,157 | 9 | 1 | 0 |

[MEASURED] The named-class bucket's proved implementations are all static:
X 299, installed X 299, T 365. X/installed X have 122 static method declarations
and 177 static function-valued fields; T has 365 static method declarations and
zero static function-valued fields. Proven instance-method/field implementations
at these imported-qualifier sites: zero. Class-instance export shapes account for
3 / 3 / 0 / 16 low rows but provide no unique implementation certificate.

[MEASURED] All 303 / 303 / 0 / 2,086 proved low callable implementations are
**dropped** on current main; none is already in a base target set. Highest raw
callable counts are T's declared-namespace exports (1,485), named classes (365),
re-exported namespaces (157), and 79 other named value shapes; X has 299 static
class members plus four call-result members. This is an opportunity census,
not an authorized recovery promise or a runtime-call guarantee.

[READ] The brief's positive-proof rule prohibits adding a target. Therefore
these dropped rows cannot be recovered by this S2 refinement. Type declarations,
opaque call results, unknown members and non-unique symbols keep base under
Option K; a lack of oracle implementation is not proof of non-callability.

[MEASURED] The complete multi-target population is enumerable: six `keyTest`
rows in X (the same source rows in installed X), plus one `IO.readFile` row in T.
[READ] The six qualifiers are exports initialized by `register({...})`
(`actions/actionCanvas.tsx:134/175/216`, `actionProperties.tsx:852/876`,
`actionExport.tsx:300`); E5 preserves their call-result bindings. T's export is
`let IO: IO` at `harness/harnessIO.ts:46`, written by `setHarnessIO` at line 48;
TypeScript binds `readFile` to the interface method signature at line 23, not
one of the two implementation arrows in the base set. E5 preserves that row.
No absence-based drop or grade promotion is proposed.

[ASSUMPTION] **Recommendation: park current S2 unless F establishes material
permitted yield.** Public permitted refinement yield is not material: zero
native-certified existing-target filters, and all seven distinct multi-target
source sites (13 measured rows across the two X snapshots and T) have an
independent E5 keep-base reason. No product prototype was built;
head-vs-base prototype gains are not applicable. This does not claim zero yield
on F, zero opportunity for a separately authorized target-adding route, or a
universal proof about every unresolved TypeScript member.

[READ] Instrument semantics and limitations:

- The census uses each caller's actual offline TypeScript ProjectService default
  project and checker; it does not use Prism's module/export resolver to certify
  a terminal. Symbol identity proves the qualifier's import/require declaration.
- It measures direct identifier-member source sites exposed by Prism, including
  its JSX member-site representation where associated. Indirect expressions,
  unmatched syntax and other bindings are reported as exclusions, never as zero
  yield. Top-level calls absent from Prism's site population are not included.
- Each candidate records the import shape, actual owner config, all qualifier
  and member declarations, callable file/name/span when unique, base drop/targets
  and whether the exact native identity is in those targets. Source hashes are
  checked against every facts file, including installed X independently.
- Native callable counts include unique plain implementations and static/instance
  method/field bodies. Declaration signatures and opaque wrapper/call values do
  not count as proved implementations. This is deliberately a certified count,
  not an upper bound on all possible implementation bodies.
- `positive_filter_ceiling` in the machine schema is an upper bound only within
  the native-certified identity population, before E5 and branch completeness;
  it is not a proof that unknown rows can never improve. The seven multi-target
  rows were separately source-audited above.
- Require provenance refuses an in-repo implementation binding shadowing
  `require`; the owner-accepted outside-repo ambient cost remains. Destructure
  defaults/rest are recorded, not granted prototype authority.

[READ] Pinned reference is TypeScript **5.9.3**, `typescript.js` SHA256
`3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675`.
Its own rules are called through the checker: import-equals/require at
53056 onward, default import at 53195 onward, named import at 53502 onward,
namespace re-export at 53333 onward, alias dispatch/resolution at 53614–53675,
export=/CommonJS at 54230–54260, and member lookup at 79674 onward.
Assignment-backed access declarations follow the exact 53608–53612 RHS rule;
shorthand property aliases use the native checker API. No product resolution
rule has been reimplemented for S2.

[MEASURED] A source-binding probe refuted the supplied reference's current-main
label on X: the supplied `main-prism` differs on 3,129 rows in each X corpus,
all bare imported-member rows (`UnknownName` → Exact `import_member` on the
fresh build). Added/removed keys: zero; S2-qualified rows changed: zero. R/T
complete streams are byte-identical. These results come from same-environment
runs of both executables, not an attribution inferred from different machines.
The old binary's exact source revision is unverified. Its hashes and complete
row diff are preserved. **All final measurements and the F wrapper use freshly
built 4e592daa binaries**, with checkout-lock package identities verified for
the facts driver. `probes/reference-binaries.json` pins them; the old hashes are
in `probes/supplied-reference-binaries.json`.

[MEASURED] Verification completed:

| Check | Result | Receipt |
|---|---|---|
| Fresh offline release main and facts driver | PASS; exact main source, seeded checkout lock, metadata package identity check | `base-build.log`, `facts-build-final.log`, binary manifest |
| Full MCP nextest | 5,137 passed, 1 existing ignored/skipped | `nextest-mcp.log` |
| MCP doctests | 2 passed | `doctests-mcp.log` |
| Source-bound measurement controls | JSX 19 / TSX 20 associated candidate sites; all assertions pass | `source-bound-controls.log`, `controls/results.json` |
| Negative instrument controls | Empty stream; compressed equality; wrapper argument/evidence-reuse/binary-drift refusal; public runner rejects F | controls logs, `public-reject.log` |
| Advisory mutation gate, `--since 4e592daa --scope fn` | selected 0; no mutation coverage claimed | `scoped-mutgate/summary.json` |
| Cargo build cleanup | Removed owned `target/s2-plan/build`; retained runnable current binaries | `cleanup.json` |

[READ] The existing ignored test is
`tests/integration/resolution_test.rs:575`: SliceElem is reserved until a future
slice. No unrelated failure was re-baselined or silently repaired. Full
pre-merge authoritative mutation, a product S2 registry, a new Tier-A fixture,
Tier-A matrix/quick, Opus review and private F acceptance were not run: no
product prototype or implementation merge is being proposed. No product source,
cache pins or committed evaluation baselines changed; no Git write occurred.
The measurement instrument's first behavioral failure, CLI setup refusal and
Python TOML-parser availability refusal are documented in PROBES as instrument
or inadmissible observations, not corpus or product failures.
