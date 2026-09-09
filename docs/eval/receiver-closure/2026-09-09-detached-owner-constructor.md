# Detached executable-owner constructor

Implements the first checkpoint of the [approved owner design](../../superpowers/specs/2026-09-09-callable-executable-owner-proof.md), based on merged PR295 (`4e88d33b`). The constructor can prove the bounded direct imported `Handler<Props>` receiver in TS and TSX. **No production resolver edge is installed.** Real-receiver gain remains unmeasured, not positive.

## Authority and implementation

The new compiler pass derives the direct binding, actual contextual signature and whole-binder substitution, local private Props alias, required property, imported class and own executable method from one live Program/checker. It emits 21 original-source anchors in a separate `prism.detached-owner/1` observation envelope. Both this envelope and the old packet explicitly set `authorizes_runtime_edge: false`; neither deserializes into a proof.

The private Rust acquisition path accepts an independently supplied root/config/compiler and owned Prism files. It fixes the default acquisition profile and reject-links policy, pins all 23 transitive local worker assets at compile time, verifies the pinned compiler/library/source snapshot through the existing observer, and reproduces the complete observation twice. Workers have a 512 MB heap limit, 35-second deadline and 16 MiB output cap; inherited NODE_OPTIONS/NODE_PATH are removed. The trusted Node runtime and quiescent-root assumptions remain: this is not an adversarial filesystem transaction or a project-code sandbox.

Input agreement has three independent checks: the complete project Program census, supplied source identities/hashes, and Prism's actual repository load between the two snapshots. An out-of-Program file that Prism indexes refuses the route even if the caller omits it. Prism's indexing policy is unchanged. Owned JS/TS inputs are reparsed because ParsedFile exposes independently mutable source/tree fields. Input and loader source inventories are conservatively bounded to 512 files / 8 MiB; loader inventory counts all supported source languages. Declaration and compiler-library domains cannot provide executable bodies.

All anchors must agree with original SHA, UTF-8/UTF-16 boundaries and parser nodes. Class keyword normalization handles exported declaration ranges. Existing clean-class, unique relative-module target, ordinary direct-method and unproven-slot barriers must agree. Ambiguous line-based FunctionId mappings refuse. The constructor does not fall back to a printed class name.

An `AuthenticatedProgramEpoch` privately owns evidence, reparsed inputs and the exact-call-to-member map. An `ExecutableOwnerProof` holds that epoch's Arc and call key, not independently spliceable target fields. `target_for` checks owning Arc identity and the consuming call key. Equal-content epochs are distinct. Old epochs remain immutable historical objects; active-analysis replacement, graph installation and persistent cache bypass are deliberately absent.

## Refusal and substitution evidence

- 27 design scenarios in both TS/TSX run through the real constructor; only the two genuine eligible source variants receive proofs. Existing inline/explicit routes retain their independent public behavior.
- The compiler suite adds 42 focused predicate refusals plus Unicode, BOM, CRLF, non-privileged-name and ordinary-constructor controls. Some negative fixtures also have compiler diagnostics; their compiler-pass refusal is asserted independently of closure, not presented as valid typed examples.
- Genuine A/B epochs differ in class source while retaining the same call bytes. Cross-owner readout refuses both directions; a new same-root/equal-content session also refuses substitution.
- Each of the 21 anchor fields is substituted from a second genuine source epoch. Additional class/member/module field mutations, hash/domain/range/encoding errors, public ParsedFile source/tree inconsistency and unique FunctionId collisions are covered.
- Same-root positive A → positive B → unproven member and then missing configured type exercise fresh acquisition and no new-epoch authority reuse. This tests detached objects, not a production cache lifecycle.

## RED and review accounting

The merged base has no constructor API. Before implementation, a fail-closed API scaffold plus the positive acquisition test produced one captured behavioral failure (`detached_constructor_unavailable`). That is base-plus-test-seam RED, not an existing main API regression. The expanded positive variants share this gain; baseline-passing refusal cases are compatibility controls. Original scaffold output is retained, but its overwritten source was not separately archived.

Initial compiler tests exposed an undefined-name helper crash on an ordinary constructor; the bounded guard fixes it, with positive constructor and negative this-write cases. A non-privileged-name fixture initially imported the wrong file: that setup failure is inadmissible and was corrected before accepting evidence.

Two self-review rounds completed. Round 1 found one WRONG: a caller map matching the Program could omit an additional Prism-indexed prototype-effect file. The exact omitted-map test failed on the pre-fix implementation and passes after the actual loader census check. Round 2 found no further demonstrated wrong result. This was self-review, not an independent review or proof of all possible compiler inputs.

## Compatibility and verification

The ordinary worker was refactored into an internal trusted factory hook; normal CLI behavior and schema20/version0.21.0 remain unchanged. On the exact merged base and same compiler/root fixtures, all 54 observer packets are deeply equal after removing **only** producer.sha256. Base fingerprint: `32ae5bc44000af31bb7cd5994a14de5700bafb02a1089f3ac16a79687e40f88d`; current: `cfbaacfa426606f0d308b71db21c3e9d98f5e00218c825687bd68a298042ee1d`.

All seven full gates passed on clean `d0a1ce9`: observer694, default Rust4025,
MCP4215, helpers18 and authority40, with doctests/fmt/diff passing. An additional
full `mcp detached-owner-audit` run passed4221. Each Rust run has exactly one
ignored `resolution_test::slice_elem_variant_reserved`; no failures or other
required-suite exclusion. A test-only lint-helper simplification is being reverified;
the final receipt will distinguish its tested HEAD from this first checkpoint.

Reproduce the explicit real-compiler audit with the already installed pinned TypeScript 5.9.3 compiler:

```sh
PRISM_TYPESCRIPT=/absolute/path/to/typescript/lib/typescript.js cargo test --features 'mcp detached-owner-audit'
PRISM_TYPESCRIPT=/absolute/path/to/typescript/lib/typescript.js node --test scripts/callable-observations/direct-owner.test.mjs
```

The feature requires the compiler explicitly and fails if it is missing; ordinary default/MCP tests gain no TypeScript installation requirement. It is not silently skipped or claimed as an ordinary CI gate. No dependency installation was performed.

Prism and LSP navigation guidance informed seam selection. Prism's caller result was stale; LSP tools were unavailable. Source inspection and pinned compiler/parser tests supply the mapping evidence, not a claimed complete navigation blast radius.

## Value checkpoint and next boundary

Accepting this detached constructor does not ship Exact call resolution. Next, design/implement bounded opt-in shared-resolver consumption together with full/subset epoch replacement and verified cache load/write bypass; do not enable partial wiring without those lifecycle gates. Preserve explicit/local routes and terminal refusal for this new claim. Production CPG/navigation cache versions remain 77/45, with no resolver/navigation/CPG/AST changes and hence no Tier-A trigger in this slice. Served callers/callees, CPG stale-edge prevention, adversarial atomic mutation and real-corpus recall remain unverified.

Closure-policy expansion, package executable pairing, React.FC widening and resolving/installing react-scripts are not authorized here. The owner's unresolved react-scripts disposition is unchanged.
