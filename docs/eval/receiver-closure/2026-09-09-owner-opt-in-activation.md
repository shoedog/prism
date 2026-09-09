# Bounded owner activation — verification and disposition

Implemented and tested on clean `ffc974711cf4a851c614a7af461e73ec8d010919`,
base `a120220f` (merged PR297). This is explicit experimental CLI/API and eager
MCP activation, not a default resolver change. [Usage and limits](../../experimental-owner-navigation.md).

## Delivered boundary

Paired compiler/config selection, explicit cache bypass, JS/TS-only inputs, private
epoch acquisition, and fresh MCP publication before each tool call. Failed
acquisition clears active ownership and returns an error; ordinary stale fallback
is unreachable from this runtime. The accepted fixture has the intended Exact
class-member edge through API, CLI and actual MCP subprocess output. Tests cover
A→B→unproven, config/dependency/closure failure and restoration, refresh reporting,
argument/cache refusal, and unchanged default behavior. Inherited full proof,
substitution and cache-barrier tests ran again under the compiler-audit feature.

No compiler install/discovery, closure-policy expansion, React.FC expansion,
react-scripts resolution, persisted owner authority or mixed-language enrichment.
No new real-receiver recall or runtime dispatch identity claim.

## Verification

| Check | Measured result |
|---|---|
| `cargo test` | 4,030 passed; 0 failed; 1 known ignore; 28 groups |
| `cargo test --features mcp` | 4,221 passed; 0 failed; 1 known ignore; 30 groups |
| `cargo test --features 'mcp detached-owner-audit'` | 4,243 passed; 0 failed; 1 known ignore; 30 groups |
| Callable observer | 694 passed; no skips |
| Receiver helpers | corrected fixture: 18 passed on both base and candidate |
| Callable authority | 40 controls; no failures |
| Python harness, from `eval/` | 883 passed; 2 explicit matched-binary integration skips |
| Formatting/diff | passed |
| Clippy, all targets with MCP+audit | completed with warnings; three new test-only Default-field-assignment notices |
| Tier-A matrix | 159 ok, zero regressions |
| Same-environment fixed-corpus controls | 36 raw response pairs equal; all 159 complete matrix records equal/ok |

Rust totals include two doctests per run. The sole ignore is
`resolution_test::slice_elem_variant_reserved`. Python exclusions are
`test_resolve_matched_binaries_against_real_binaries` and
`test_warm_gate_real_cache_distinguishes_cold_from_warm`; their required
`PRISM_BIN`/`PRISM_MCP_BIN` matched pair was not configured. No full multi-corpus run.

## Captured failures and attribution

- Public seam RED: base plus inert options returned NameOnly, expected Exact,
  0 pass/1 fail. Source snapshot and output retained before implementation.
- Review round 1 **WRONG**, new candidate: source A→B followed by refresh reported
  `stale_before_refresh=false` due literal constants. Captured false-vs-true RED;
  fixed by retaining prior metadata stamps for reporting, never for proof reuse.
- Initial accessor compile error, concise-wire field mismatch, and incomplete
  subprocess handshake were test/probe defects, not product regressions. Corrected
  probes passed. The concise payload already contained the correct target/score.
- Original combined gate receipt remains **failed**: helpers were 15/18 because an
  old temporary slice was missing App.tsx. All 3 failures reproduced on merged base
  with the same missing path. The five-file slice was reconstructed from local
  Excalidraw commit `0642e72cfa2d9a71198200e52f37399384610ee3`; all five SHA256 values
  match the committed 2026-09-05 audit. Full helper suites then passed 18/18 on both
  revisions. No assertion/source/baseline was weakened. This supplement supersedes
  only the missing-fixture helper outcome, not the original receipt bytes.

Two self-review rounds completed; no independent-agent review. The closed WRONG is
fixed; **SMELL** limitations are portable worker custody, per-call rebuild cost,
mixed-language/lazy scope, and the test-style warnings. These are documented, not
claims of demonstrated incorrect production output.

## Tier-A quick: invalid accuracy baseline, not a new anchor

Executed on clean ffc9747 after immediate same-worktree release rebuild. Exit 2;
oracle rust-analyzer 1.94.0; oracle errors 8/30 (26.7%), SUT errors 0. Corpus differs
from pin 20c8490591a3; C-method 4/6 and C-name 0/6 successful probes. Matrix 159 ok.
`target-c-method` is a flip_candidate; module-deps/load-repo feature-gated controls
are missing; ambiguity control is ok. There are 24 pending discrepancies and no
executed M3 spot checks. These current quick discrepancies are not newly adjudicated
or attributed to this slice. No changed baseline or new accuracy-gain claim.

Matched control corpus was freshly checked clean at full pin
`20c8490591a379227b70c1d5ec4c75ff4b64ff01`. Fresh separate base/candidate caches,
actual rebuilt binaries, one inventory +30 sampled requests +5 pinned requests
produce 36 identical pairs. All 159 matrix records, including 55 direct-CLI DFG cases,
are also equal/ok. Separate Node deep-equality decoding checked the Python artifacts.
The inherited oracle inventory only selects fixed requests; it does not supply new
oracle accuracy evidence. Review readiness follows the already adopted PR297
distinction between reported quick limitations and valid accuracy anchoring.

## Custody

Evidence root `/private/tmp/prism-owner-activation-88yT3B`; RED source archives,
full original gate receipt/logs, both binaries, restored source slice, corrected
helper controls, quick reports/snapshot, and pair artifacts are retained there.
Generated quick reports were moved out of the checkout; committed Tier-A baselines
were not changed. [Machine receipt](2026-09-09-owner-opt-in-activation.json).
Archive `/private/tmp/prism-owner-activation-88yT3B-evidence.tgz`, SHA256
`779653e42c900962c987106950230bf13d8028b88ed111ec08e0f3dfaac44aef`.
It preserves the pre-publication evidence checkpoint, excluding rebuildable base
worktree and cache directories; later publication metadata lives in the Git handoff.

Navigation skills led to source fallback: Prism returned SymbolNotFound for the
new staging symbol; LSP tools were unavailable. Compiler-backed tests and actual
source/transport inspection supplied the needed consumer evidence.

Next value checkpoint: assess the experimental route's practical admission/refusal
and acquisition cost before broader activation. Portable worker custody and any
larger receiver/closure grammar remain separate decisions.
