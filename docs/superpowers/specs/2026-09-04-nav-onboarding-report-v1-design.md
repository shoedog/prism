# Navigation onboarding report v1 design

Date: 2026-09-04
Status: accepted for implementation
Exact base: PR #234 merge `90c522b04ff16ebc076ce85a4f8df5f7f2da4f1f`

## 1. Goal

Add one bounded CLI packaging command, `prism nav onboard`, that builds or warms the
normal whole-repository navigation cache and emits a compact project-orientation
report. The report combines existing repository, module-graph, and call-resolution
facts so an agent can begin with a durable map instead of issuing several large
queries.

## 2. Safety and scope

- CLI only. No MCP tool or registry-count change.
- Analysis remains read-only. Optional file output is explicit and create-new-only;
  an existing target is an error and its bytes remain unchanged.
- Stdout is the default. No implicit `.prism`, memory, config, or source file is
  created.
- No source snippets, absolute source text, edit operation, or refactor is returned.
- No cache/schema bump. `nav_session` remains the only load/build/cache path.
- Java/LSP, full write tools, symbol search, diagnostics, and persistent automatic
  session memory are out of scope.

## 3. CLI contract

```text
prism nav [--no-cache | --cache-dir DIR] onboard \
  --repo PATH [--format markdown|json] [--out NEW_PATH]
```

- `--format` defaults to `markdown`.
- Without `--out`, exactly one report is written to stdout.
- With `--out`, the parent must already exist and the target must not exist. Success
  writes the complete report and no report bytes to stdout.
- Existing global navigation cache flags retain their current meaning. `--no-cache`
  performs the analysis but intentionally does not persist a warm cache.

## 4. Data contract

The versioned `ProjectOverview` contains:

- `schema_version = "1.0"` and repository basename;
- indexed/skipped file counts, function count, and a sorted language histogram;
- module node/edge/isolated-file counts;
- at most 12 connected modules, ranked by total degree descending and path ascending,
  each with outgoing dependency and incoming dependent counts;
- total call sites, Exact edge count, NameOnly edge count, demoted edge count, and the
  four stable unresolved/drop counters already exposed by `call_stats`;
- sorted warning strings and stable follow-up command templates.

The report is derived from one `NavigationSession`. `repo_map` supplies the module
graph and therefore the same call-derived dependency relation as `module_deps`;
`call_stats` supplies resolution telemetry. A missing JSON key or non-numeric value
from the existing telemetry projection is a construction error, never silently zero.

Markdown renders every field in a fixed section/key order. JSON uses the same typed
structure. Both end with one newline and are deterministic for the same indexed
repository and binary.

## 5. Implementation seams

- `src/navigation/onboarding.rs`: typed report construction and graph ranking.
- `src/navigation/mod.rs`: export the module.
- `src/output/onboarding.rs` and `src/output/mod.rs`: Markdown/JSON rendering.
- `src/cli.rs`: additive `NavQuery::Onboard` grammar.
- `src/main.rs`: one session, render, stdout or create-new file dispatch.
- navigation and CLI integration tests; README/CLAUDE/roadmap/handoff truth pass.

Structural navigation shows `repo_map` has one other production consumer, MCP, plus
tests. The new report consumes its returned public graph and does not change the
shared projection, so MCP behavior and existing navigation JSON remain unchanged.

## 6. RED and acceptance

RED must establish all missing behavior before implementation:

1. typed report construction, deterministic ordering, and the 12-module bound;
2. empty-repository zero behavior and no fabricated connected-module entry;
3. CLI Markdown and JSON output;
4. explicit file creation plus existing-target refusal with byte preservation.

Acceptance requires focused GREEN, two self-review rounds at most, format/diff/check/
Clippy, full default and MCP suites, and the navigation accuracy harness. Tier-A
results are reported without re-baselining. Every harness invocation using
`--allow-stale-sut` has an immediately preceding release build.

## 7. Review convergence

Two self-review rounds maximum. A closed enumerable WRONG/SMELL population receives a
targeted fix on this artifact. Repeated open-class determinism, overwrite-safety, or
telemetry-contract findings park the slice for redesign.
