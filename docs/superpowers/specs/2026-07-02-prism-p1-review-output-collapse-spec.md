> **Status: SHIPPED — PR #149 (merged 2026-07-02).** As-executed implementer brief, including the folded codex gpt-5.5 xhigh spec-review corrections (severity vocabulary incl. `suggestion`; review-only compaction because `--format json` shares the byte-pinned serializer; emitted-sink source-licensing rule). Post-spec fix wave (also merged): cross-file `sink_to_path_sources` licensing + bash unquoted-expansion ordering. Measured: review-suite probe 13.49 MB → 552 KB (~24×).

# Task P1 — Collapse the `--format review` firehose

You work in the git worktree `/private/tmp/prism-p1-review-collapse` on branch `p1-review-output-collapse` (based on main @ 523db1b). The repo is prism (binary names: `prism`, also historically `slicing`). Follow TDD: write the failing test first for each behavior change, then implement.

## Problem (measured)

`./target/release/prism --repo . --diff <patch> --algorithm review --format review` on one modest commit (`git show 36429d7 --format= --unified=3`) emits **13.5 MB / 2,432 findings, 2,198 of them severity `info`**; 1,889 come from the taint algorithm. Root causes:

1. `taint_from_diff` (default true when no `--taint-source` is given — src/main.rs:1109, default in `TaintConfig` src/algorithms/taint.rs:1239-1248) seeds **every diff line of every file** as a taint source (src/algorithms/taint.rs:10842-10848) — including files prism cannot parse (e.g. `Cargo.toml`, `.md`), which are absent from `ctx.files` and can never be traced.
2. Every seed then unconditionally emits an `info` finding "taint source: origin of tainted data at line N" (src/algorithms/taint.rs:11169-11183), regardless of whether it reaches anything.
3. The review JSON triple-encodes each block: `slice_text` plus `slice_lines` and `diff_lines` integer arrays restating the same line numbers (`ReviewBlock` fields src/output/review.rs:27-29, rendered :138-140), and `to_review_output` (src/output/review.rs:153-181) maps ALL blocks with no cap; `MultiReviewOutput` (src/output/review.rs:69-86) aggregates all algorithms.

Line numbers verified against main @ 523db1b (a pre-implementation codex review re-confirmed them); still, read the code first.

## Changes

**Change 1 — skip unparseable seed files.** In the `taint_from_diff` seeding loop (taint.rs:10842-10848), skip a `diff_info` whose `file_path` is not a key of `ctx.files`. This key is correct: the diff parser normalizes `+++ b/...` to repo-relative paths (src/diff.rs:67-92) and the CLI inserts parsed files under the same `diff_info.file_path` (src/main.rs:624-642). Parsed-language files are unaffected.

**Change 2 — emit per-source findings only for sources tied to an emitted sink finding.** Today taint.rs:11169-11183 emits one `info` finding per source, unconditionally, BEFORE the sink loop. Sink findings are NOT all path-derived: path hits populate `sink_to_path_sources` (taint.rs:11021, :11035), but framework/source-line fallbacks add `sink_lines` entries WITHOUT a path mapping (taint.rs:11047, :11071); each emitted sink finding then selects one source via the fallback chain at taint.rs:11187-11226 (`source_location`).

Rule (exact): build `sources_with_emitted_sinks: BTreeSet<(String, usize)>` from the exact `source_location` chosen for each emitted sink finding — path-derived source if available, otherwise the fallback source if it is a member of `taint_sources`. Emit `taint_source` info findings ONLY for members of that set; when a sink finding selects no source, it contributes nothing. Restructure so source-finding emission runs AFTER sink emission. Everything else (seeding, propagation, `all_tainted`, sink detection) is unchanged; the same rule applies to explicit `--taint-source` sources.

**Change 3 — findings-first review output (REVIEW FORMAT ONLY).** Critical constraint discovered in spec review: `--format json` and `--format review` SHARE `to_review_output`/`MultiReviewOutput` (src/main.rs:903-924 and :977-982), and compatibility tests byte-pin the `--format json` output including `slice_lines`/`diff_lines` (tests/cli/nav_compat_test.rs:74-92 pinned against tests/fixtures/nav_compat/golden/leftflow.json:8-16). So do NOT change the shared structs' serialization. Instead add a review-only compact path — e.g. render options or a compact output type used only when `cli.format == "review"`:
   a. Compact review output omits `slice_lines` and `diff_lines` (keep `slice_text`).
   b. Severity floor: filter findings (per-result `findings` and `all_findings`) to severity ≥ minimum. Full severity vocabulary is `"info" | "suggestion" | "warning" | "concern"` — `suggestion` is emitted by PrimitiveSlice (src/algorithms/primitive_slice.rs:131-142), which IS in the `review_suite` (src/slice.rs:233-257). Ordering: info < suggestion < warning < concern. Default minimum for `--format review`: `warning`.
   c. Block retention: retain a `ReviewBlock` iff at least one RETAINED finding's `(file, line)` is present in the block's `file_line_map` (blocks are cross-file — src/output/review.rs:95-107; same-file-only matching is too broad). Findings are never dropped by block filtering — a finding matching no block keeps itself.
   d. New CLI flags in src/main.rs: `--review-min-severity <info|suggestion|warning|concern>` (default `warning`) and `--review-full-slices` (bool, default false — keep all blocks). Both affect ONLY `--format review`. `--format json`/`text`/`paper` output must be byte-identical for the same input, EXCEPT effects of Changes 1-2 (which alter findings at generation time and legitimately flow into all formats — the golden `leftflow.json` is a leftflow-only fixture and should be unaffected; verify the nav_compat test still passes unmodified, and if a golden legitimately changes due to Changes 1-2, stop and report DONE_WITH_CONCERNS rather than re-baselining silently).

**Change 4 — skill doc.** Update `skills/prism-code-slicing/SKILL.md` "Output formats" + "Gotchas": document the new review defaults (findings ≥ warning + matching slices; `--review-min-severity info` / `--review-full-slices` restore the firehose). Keep the "aggregates are large" gotcha for `--algorithm all --format json`.

## Tests (TDD)

- New CLI tests beside existing review-output tests (look in tests/cli/; keep files under 600 lines; register any new file as a module in the umbrella main.rs):
  1. Fixture repo with a parseable file + an unparseable file (e.g. `.toml`), diff touching both → no `taint_source` finding for the unparseable file even at `--review-min-severity info`.
  2. Default `--format review` contains no `info`/`suggestion` findings; `--review-min-severity info` restores them.
  3. A taint source that reaches a sink still yields its `taint_source` finding + the sink finding; a source reaching nothing yields none.
  4. Default review JSON has no `slice_lines`/`diff_lines` keys and omits finding-less blocks; `--review-full-slices` restores blocks; `--format json` retains the old shape (nav_compat golden test passes unmodified).
- Update existing tests that assert per-source findings / review shape — intent-preserving.
- Before finishing: `cargo fmt`, then full `cargo test` (expect a long first build in this worktree). No new warnings.

## Done-check (run and paste results into your report)

```
cargo build --release
git show 36429d7 --format= --unified=3 > /tmp/p1-probe.patch
./target/release/prism --repo . --diff /tmp/p1-probe.patch --algorithm review --format review | wc -c    # target: < 200_000 (was 13_490_080)
./target/release/prism --repo . --diff /tmp/p1-probe.patch --algorithm review --format review --review-min-severity info > /tmp/p1-full.json
python3 -c "
import json; d = json.load(open('/tmp/p1-full.json'))
bad = [f for f in d['all_findings'] if f.get('category')=='taint_source' and f['file']=='Cargo.toml']
print('cargo-toml taint sources:', len(bad))  # target: 0
import collections; print(collections.Counter(f['severity'] for f in d['all_findings']))"
```

If per-finding `diagrams` payloads keep the default output above 200 KB despite the floor, do not trim diagrams (out of scope) — report the number.

## Commit style

Small logical commits, conventional subjects (e.g. `fix(taint): skip unparseable files in diff-line taint seeding`). End each commit message with:
Co-Authored-By: Claude Sonnet 5 <noreply@anthropic.com>
