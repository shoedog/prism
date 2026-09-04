---
name: prism-code-slicing
description: Extract the code relevant to a diff/change with the prism `slicing` CLI for defect-focused review. Use when reviewing a diff or PR and asking "what does this change affect / what's the blast radius", "is there a taint path from user input to a dangerous sink", "what's missing (unclosed resource, broken symmetry)", "which peers should match this changed function", or "what data-flow changed vs the previous version". Diff-driven: it slices relative to changed lines, not a whole repo (for whole-repo navigation use the prism-code-navigation skill / MCP server instead). The binary is named `slicing`.
metadata:
  project: prism
  surface: cli
---

# Slicing diffs with prism

The `slicing` CLI takes a **repo + a diff** and prints the code *relevant to the changed lines* under one
of ~30 slicing algorithms. It's for **reviewing a change** — "given this patch, what should I actually
look at?" The hard part isn't running it; it's **picking the algorithm that answers the reviewer's
question**. This skill is that map.

> The binary is `slicing` (historical name; the project is Prism). It is **diff-driven** — if you want
> to navigate a whole repo with no diff, use the MCP server / `prism-code-navigation` skill instead.

## Workflow

```bash
cd /path/to/the/repo
git diff HEAD~1 > /tmp/change.patch          # or any unified diff / PR patch
slicing --repo . --diff /tmp/change.patch --algorithm <algo>
slicing --repo . --diff /tmp/change.patch --algorithm <algo> --format review   # for code-review output
```

- Default algorithm is `leftflow`. Run multiple at once: `--algorithm "leftflow,fullflow,taint"`.
- Presets: `--algorithm review` (a curated review suite) · `--algorithm all` (everything).
- `slicing --list-algorithms` prints them all; see `ALGORITHMS.md` for the per-algorithm operator's guide.

## Pick the algorithm by the reviewer's question (defaults first)

**Start with `leftflow`** (backward data-flow from the changed assignments — the default) for "what does
this change depend on / affect." Reach for a specific one only when the question is specific:

| The reviewer is asking… | Algorithm | Key flag |
|---|---|---|
| What does this change affect (backward data deps)? | `leftflow` *(default)* | — |
| …including where the new values flow forward? | `fullflow` | — |
| Just the data chain, no control-flow noise? | `thin` | — |
| Is there a path from user input to a dangerous sink? | `chop` | `--chop-source` / `--chop-sink` |
| Where do these tainted values end up? | `taint` | `--taint-source` (repeatable) |
| What does the code do when this value is null / condition holds? | `conditioned` | `--condition` |
| What data-flow paths changed vs the previous version? | `delta` | `--old-repo` |
| Which peer functions should match this changed one? | `horizontal` | `--peer-pattern` |
| The full request path, handler → database? | `vertical` | `--layers` |
| Trace a cross-cutting concern (auth, error handling)? | `angle` | `--concern` |
| Is a counterpart missing (open/close, lock/unlock, malloc/free)? | `absence` | — |
| Broken symmetry (serialize/deserialize, encode/decode)? | `symmetry` | — |
| Cross-module blast radius of a changed public API? | `membrane` | — |
| Ripple — callers that don't handle a new error? | `echo` | — |
| Async/goroutine state races around await/spawn? | `quantum` | `--quantum-var` |
| Trace callers/callees up to N levels, stop at framework internals? | `barrier` | `--barrier-depth` |
| Which functions are riskiest (coupling + churn)? | `threed` | `--temporal-days` |
| Recently deleted code resurfacing / co-change coupling? | `phantom` / `resonance` | *(needs git history)* |

When unsure, run a small set (`leftflow,fullflow,absence`) and read the union — cheaper than guessing.

## Output formats

`--format text` (default, human) · `json` (machine — pipe to `jq` or an LLM, full uncompacted shape)
· `paper` (arXiv-comparable) · `review` (grouped findings with severities — use this when producing a
code review; compact by default, see below) · `sarif` (SARIF 2.1 — upload findings as GitHub
code-scanning annotations; one rule per algorithm/category pair).

```bash
slicing --repo . --diff /tmp/change.patch --algorithm review --format json | jq '.findings[]'
```

`--format review` is compact by default: findings below `warning` severity (`info`, `suggestion`) are
dropped, `slice_lines`/`diff_lines` are omitted from each slice block (keeps `slice_text`), and any block
whose lines don't intersect a retained finding is dropped too. Each restore flag undoes only its own part
— neither restores the full pre-compaction shape by itself:

```bash
slicing --repo . --diff /tmp/change.patch --algorithm review --format review --review-min-severity info
slicing --repo . --diff /tmp/change.patch --algorithm review --format review --review-full-slices
```

`--review-min-severity <info|suggestion|warning|concern>` (default `warning`) lowers/raises the severity
floor; `--review-full-slices` restores BLOCK retention only (keeps every block regardless of whether it
has a retained finding) — it does not lower the severity floor, and neither flag restores `slice_lines`/
`diff_lines` in review output. Both flags only affect `--format review` — `--format json`/`text`/`paper`
are unaffected. For the full old (uncompacted) shape — every block plus `slice_lines`/`diff_lines` plus
every severity — use `--format json`.

**`prism targets`** is a separate subcommand, not a `--format` value: it projects findings from
five algorithms (`echo`, `absence`, `contract`, `provenance`, `membrane` by default) into a
stable, closed-schema JSON document of instrumentation sites (`docs/contracts/targets.schema.json`)
for a runtime harness — reach for it when the consumer wants "where should I fault-inject / watch /
verify," not a human-readable review:

```bash
slicing targets --repo . --diff /tmp/change.patch --strict
```

## Gotchas

- **It needs a diff.** No `--diff` ⇒ nothing to slice from. It is *not* a whole-repo navigator. For
  "who calls X" / "module deps" with no patch, use the MCP server (`prism-code-navigation` skill).
- **Lines are 1-indexed** throughout (tree-sitter's 0-indexed rows are converted internally).
- **git-history algorithms need git history.** `phantom`, `resonance`, `threed` (with `--temporal-days`)
  read the repo's commit history; they're empty/degraded on a shallow or history-less checkout.
- **C/C++ field-sensitive analysis needs type info.** Pass `--compile-commands compile_commands.json` to
  enrich struct fields, typedefs, and class hierarchy; without it, C/C++ slices are field-insensitive.
- **`review`/`all` are aggregates** — convenient, but the combined output is large and (for `review`)
  not byte-stable across runs. For a reproducible single answer, name one algorithm.
- **`--format review`'s default floor can hide what you're looking for.** If you expect an `info`-severity
  finding (e.g. a `taint_source` origin) and don't see it, that's by design — either it didn't reach a
  sink (see below) or it's below the default `warning` floor. Pass `--review-min-severity info` to see
  everything, or `--format json` for the always-uncompacted shape.
- **Taint `taint_source` findings are gated on reaching a sink.** A diff-seeded source that never flows
  into a sink no longer gets its own finding (this shrank the taint-heavy firehose considerably) — this is
  independent of the severity floor above; it happens at generation time, in all formats.
- **A slice is high-recall, not a verdict.** It's the code *relevant* to the change, surfaced for a human
  (or you) to review — not a proof of a bug. Read the surfaced sites; don't report a finding you haven't
  traced to a concrete failure.
