# Slice 6a report: C/C++ macro census, behaviour-neutral (#630 slice 6)

Branch `perf/630-s6a-c-macro-census` (from origin/main e77cc18), head `1363df9`.
Brief sha256 verified (6ab49caa...).

## Items and commits
- `ae2d7a0` items 1-4: `CMacroCensus` (`src/call_graph/c/macro_census.rs`: params, transitive free identifiers, `control`, `poisoned`; definitions unioned; recursion poisoned, poison/control/free propagate to callers); digest persisted as `c_macro_census_digest` (storage, wire, validate, freeze, combine, views); partial-hit guard next to the Rust guard in `build_incremental_with_scope_graph_inputs` (`src/cpg/build.rs`); scoped builds read the census from the whole repo; threaded to `LexicalFacts::new` as `_c_census`, unused; cache v174 (doc line, const, in-file pin, `dfg_codec_tests.rs` x3, CLAUDE.md sentence + census note); DESIGN.md copied from Appendix A; tests (census unit tests, digest stability, cache parity cold/full/partial, scoped build, codec_review/corruption updated).
- `1363df9`: `tests/fixtures/go_blank/base.json` `*_bytes` (2 entries) gain the 8-byte empty digest.

## Tests
- `cargo fmt --all --check`: pass.
- `cargo test --lib`: 3087 pass, 2 fail (`cpg_cache::stream::tests::a_write_that_cannot_start_fails_and_leaves_the_previous_cache_usable`, `resolution::js_ts::paths_snapshot::boundary::tests::unreadable_source_and_directory_skip`). Both fail identically on origin/main in this environment (runs as root, so permission-denied setups succeed): outside my change.
- `cargo test --test ast`: initial 2 failures (go_blank python/javascript pinned bytes) fixed in `1363df9`; `go_blank` 40/40 pass after. Full `--test ast` was not re-run after the fix, only the `go_blank` filter.
- `cargo test -p prism-review --test cli`: 245 pass, 5 fail (all `owner_activation_test::owner_admission_*`); same 5 fail on origin/main here (root environment).
- RED check: with the new guard forced off (`if false && ...`), `cache_parity_cold_full_partial_when_a_header_define_changes` fails (stale incremental digest); guard restored before commit.

## Behaviour-neutral check (debug binaries, `nav --no-cache`)
origin/main vs head, byte-identical (`cmp`) for `dfg-stats`, `dfg-stats --edges`, `call-stats`:
- `tests/fixtures`: dfg 783 B, edges 2,040,823 B, call 6,390 B: identical.
- leveldb `bb74ef739973a70ca9f0d90788a1da89b788c0ad` (depth 1, parse only): dfg 778 B, edges 7,152,493 B, call 4,766 B: identical.

## Census summary
| corpus | C/C++ files | macros | control | poisoned |
|---|---|---|---|---|
| leveldb | 133 | 89 | 0 | 2 |
| prism `tests/fixtures` | 70 | 14 | 0 | 2 |

Build time on leveldb: 36 ms (release test build), ~110-145 ms (debug).

## Decisions / conservative readings
- Body parsed textually from the node's source range, not via grammar `name`/`value` fields: tree-sitter-c yields an ERROR node with a wrong `name` for `#define K 1 /* c */ real`.
- `control` also matches `__asm`, `__asm__` (superset of the brief's `asm`); more refusal only.
- Self-reference (`#define F F`) counts as recursion, so poisoned.
- A `#define` the grammar does not expose as `preproc_def`/`preproc_function_def` (inside a parse ERROR) is not seen; no per-name fail-closed is possible. Not addressed in 6a.
- Digest covers params, transitive free set, control, poisoned per name; an empty census digests to the empty string, like the Rust census.

Progress: DONE
