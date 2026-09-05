# Indirect local default-class identity and real receiver measurement

Base: merged PR #242 at `854d53f49606`. Owner approved this bounded increment.
Implementation and verification complete with the Tier-A exclusion below.
Published as `257fb7f` in [PR #244](https://github.com/shoedog/prism/pull/244),
merged at `a70ea03` (fetch/log verified during the arrow-field continuation).

## Result

`class C { m() {} }; export default C;` now retains declaration-backed Class
identity for existing JS/TS/TSX constructor-local and TS/TSX typed receivers,
including erased default type imports. Duplicate declarations/exports, competing
imports/bindings and visible writes revoke authority. A rejected class cannot
fall back to the callable-function export lane. CPG64/nav33 invalidate old edges.

No Python, arbitrary reexport, alias-chain, class-expression, arrow-field method,
destructured-prop or this.field receiver expansion is included. Existing negative
fixtures for the newly admitted form were explicitly replaced with still-excluded
local export-list cases; unrelated baselines were not changed.

## Same-environment proof

On untouched merged-base production source, the new identity test failed with
Alias/ConstructorLocal metadata but only NameOnly m resolution. The negative
matrix passed (1 pass, 1 fail). This separates export identity from constructor
metadata recovery. Initial implementation exposed one WRONG import collision;
the explicit import check fixed it. The spec records all three self-review rounds.

- `cargo test --no-fail-fast`: **3744 passed, 0 failed, 1 ignored**.
- `cargo test --features mcp --no-fail-fast`: **3934 passed, 0 failed, 1 ignored**.
- Existing ignored test: `resolution_test::slice_elem_variant_reserved`.
- Four new tests cover full/subset identity, 23 negative cases, terminal poison
  vs callable defaults, and cached A↔B target replacement. Existing cached
  good↔bad and navigation-sidecar matrices gained indirect-default cases.
- `cargo fmt --all -- --check`, `git diff --check`, and
  `cargo clippy --all-targets --features mcp` completed; existing warnings remain.
- Immediate release rebuild + Tier-A matrix: **104/104**.
- Immediate rebuild + `uv run --directory eval tier-a --quick --allow-stale-sut
  --date 2026-09-04-indirect-default`: exit **2**, baseline-invalid solely for
  `854d53f49606 != pinned 20c8490591a3`. Oracle quiescent (rust-analyzer 1.94.0),
  oracle/SUT error rates zero, four stale adjudications. Not a green accuracy gate.
  Full multicorpus and baseline rewrite were not run/authorized.

Quick pins: target-c-method **flip_candidate** (Exact TP5/FP0/FN0, 28 additional
default candidate sites); module-deps-feature-gated **missing** literal pin,
actual main.rs:183 on both oracle and Prism, oracle-only empty and MCP tools:230
Prism-only; load-repo-feature-gated **missing** literal pin, oracle-only
resolution_test.rs:5299/5368, four MCP Prism-only sites; ambiguous-symbol **ok**.
The broader quick sample still has Exact misses and a U-free callee FP. No
same-environment base quick was rerun for this increment, so these are reported
observations, not attributed regressions or proof of whole-Rust recall closure.

## Real source, unchanged on both sides

Excalidraw SHA `0642e72cfa2d9a71198200e52f37399384610ee3`; five files archived
without edits: `packages/excalidraw/data/library.ts`, `components/App.tsx`,
`components/LibraryMenu.tsx`, `components/LibraryMenuHeaderContent.tsx`, and
`types.ts` (all under `packages/excalidraw`). Same archive, clean-base saved
release executable vs rebuilt candidate, `nav --no-cache call-stats --dump-sites`.
This is a partial-source comparison, not whole-corpus precision or recall.

**2780 call-site records; 369 Exact before and after; zero changed records.**
Relevant Library receivers occupy **11 unique source spans / 18 caller-expanded
records**, all NameOnly on both sides. Outer `.catch()` calls are excluded;
duplicate enclosing/nested caller records are not counted as unique source sites.

| File | Relevant lines | Remaining receiver boundary |
|---|---|---|
| App.tsx | 2929, 3224, 12113, 12239 | this.library, assigned in constructor at 820 |
| LibraryMenu.tsx | 114 | destructured inline-typed props at 64–85 |
| LibraryMenuHeaderContent.tsx | 155, 160, 184, 265 | destructured React.FC props at 38–56 |
| LibraryMenuHeaderContent.tsx | 297, 307 | destructured useApp() return at 286 |

Library is declared at 197 and exported via identifier at 403. Its callable
members are arrow fields (destroy249, resetLibrary261, getLatestLibrary268,
updateLibrary287, setLibrary351). Correction from the arrow-field continuation:
these already had method-owner metadata and could resolve through a simple
typed/constructor receiver; their slot/static safeguards were bypassed. The
remaining real-site miss was receiver shape, not total lack of arrow ownership.
An executable ParsedFile probe on this exact archived library reports
`default=Some(Class("Library")); conflicted={}`. Thus zero observed receiver gain
is not evidence that the new export proof failed; downstream receiver shapes
remain excluded. No added real Exact edges exist to validate in served
callers/callees; synthetic navigation sidecar tests cover the added identity.

Earlier fixed-source controls are byte-identical before/after: Black 400 sites /
79 Exact; Excalidraw 136 / 26; JavaScript 12 / 4. The zero gain corrects the prior
expectation that export identity alone would unlock Library consumers.

## Reproduce and custody

Raw logs, saved base binary, unchanged archive, probe source/executable and paired
JSONL: `/private/tmp/prism-indirect-default-VUbv13`. Compact committed evidence:
`2026-09-04-indirect-default-verification.json`. Comparator:

```sh
node docs/eval/receiver-closure/measure-indirect-default.mjs BASE.jsonl CANDIDATE.jsonl ARCHIVED_REPO
```

The comparator rejects different site universes and reports exact-target changes;
it is not an oracle. Hashes cover the four call-bearing files; types.ts has no
call records. The entire five-file archive is preserved with the raw evidence.
Raw evidence snapshot: `/private/tmp/prism-indirect-default-VUbv13-evidence.tgz`,
SHA256 `05b963a70da6348f9d0c8092d570e9400e4383e17f18662313403028803040fb`.
Only this run's newly generated quick report/snapshot were moved into that
evidence directory; existing pins and the pre-existing untracked snapshot remain.

## Next bounded recommendation

Specify arrow-field callable ownership with duplicate/member-write barriers
first. Then separately choose a receiver proof for destructured inline-typed
props (one LibraryMenu site) before generic React.FC props or this.field dataflow.
These are recommendations, not implemented or auto-authorized scope extensions.
