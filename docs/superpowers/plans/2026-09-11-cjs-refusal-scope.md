# Enumerable CJS refusal retention and source self-binding writes

Base88af6511, fifth local increment for the same future PR. Two review rounds;
root design/source writer, existing reviewer read-only. No publication.

Repair only names already extracted into temporary CJS named/conflicted facts.
When producer safety fails, retain UnprovenLocal refusal markers for these names,
except independently claimed ESM named/conflicted names. Named/star propagation
uses the existing tri-state; no new callable or wildcard authority.

Source-backed module-value-write proof must distinguish function declaration
self writes and anonymous expressions' inferred display names from genuine named
function-expression self bindings. Apply the existing source-aware guard mode to
module_value_written, and make that mode use explicit source self names only.
Keep parameter/local/catch shadows, class proof controls and CJS captured-value
ordering. General receiver lexical guards remain unchanged and separately audited.

Unknown-only computed/alias/wrapper forms without extracted names are not made
wildcard blockers. Such policy would also block otherwise-valid ESM star fallback;
it requires an explicit owner decision. Capture residual observations, don't
claim complete CJS surface knowledge, runtime snapshots or forwarding admission.

RED first on unchanged source; controls and exact changed-set full/incremental
parameter-token flow, raw serde/name retention, two same-name source epochs.
Run full Rust configurations/examples, Node/authority/Python, formatting/Clippy,
fresh-release Tier-A matrix and quick before final review (report timeout/missing
fixtures explicitly). Preserve all preceding CJS/ESM/module controls and cache
invalidation. No closure expansion, React.FC or react-scripts decision change.
