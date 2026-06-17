# Tier-A corpus-matrix expansion — backlog

> **Status:** backlog (not scheduled). Filed 2026-06-16 alongside the arity-disambiguation work
> (`docs/superpowers/plans/2026-06-16-prism-arity-disambiguation.md`). Human-triggered, like all
> full baselines.

## Goal

Broaden the Tier-A corpus matrix beyond the current 5 (`prism` Rust, `tokio` Rust, `caddy` Go,
`flask`/`click` Python) with **2 C, 2 C++, 2 Java, +1 Go, +2 Rust** libraries — pulled, SHA-pinned
in `eval/corpora.toml`, and baselined into `docs/eval/tier-a/`.

## Why

1. **Measure the C++/Java overload-arity locus.** The arity-disambiguation work fixes the Go
   `interface_impls` mint, but the same class has a second, *unmeasured* home: `owner_lookup`
   minting same-owner overload sets `Exact` (`src/resolution.rs:486`) for languages with genuine
   overloading — **C++ and Java**. There is no C/C++/Java corpus today, so that locus is invisible
   to the harness and cannot be gated. A corpus is the prerequisite to even decide whether the
   `owner_lookup:486` follow-up is worth doing.
2. **Steadier Rust anchor.** The Rust side currently leans on two awkward corpora: `prism` (the
   self-corpus — drifts every commit, always SHA-`baseline_invalid` until re-pinned at a merge) and
   `tokio` (large/slow, and oracle-floor-failing at 0.22 — macro/cfg density). Adding **1–2 smaller,
   stable, idiomatic** Rust libraries gives a Rust anchor that isn't a moving target or a noisy one.
3. **General multi-language precision coverage.** C/C++/Java exercise resolution paths (overloading,
   class hierarchy/CHA, implicit-`this`) that the current Rust/Go/Python matrix doesn't.

## The corpora (candidates — pick concrete repos at pull time)

Selection criteria: moderate size (not amalgamation-huge), stable/tagged release, idiomatic, and —
for C++/Java — **overload-heavy** (so the `owner_lookup:486` arity locus is actually exercised).

| Lang | Count | Candidate libs (pick 2 / 1) | Notes |
|---|---|---|---|
| C | 2 | `cJSON` (small, clean), `redis` or `git` (larger, idiomatic) | one small + one mid for range |
| C++ | 2 | `fmt`, `nlohmann/json`, `re2`, `leveldb` | pick **overload-heavy** ones (`fmt`, `nlohmann/json`) to exercise arity |
| Java | 2 | `gson`, `commons-lang`, `okhttp` | `gson`/`commons-lang` are overload-rich + moderate size |
| Go | +1 | `cobra`, `gin`, `viper` | a second Go beyond `caddy`; moderate, stable |
| Rust | +2 | `clap`, `serde`, `regex`, `anyhow` | **small + stable** — the steadier-anchor goal; avoid another `tokio`-scale repo |

## Work per corpus (the harness gaps to close first)

The runner currently wires only `rust-analyzer` (Rust), `gopls` (Go), `pyright` (Python). Adding
the new languages needs:

1. **Oracle integration** — `clangd` for C/C++ and a Java LSP (`jdtls` / Eclipse JDT) wired into
   `make_oracle` (`eval/tier_a/`), plus per-language `oracle_error_floor` entries in
   `eval/corpora.toml [defaults]` (Rust/Go 0.10, Python 0.25 today — C/C++/Java floors TBD from a
   first run, like the Python floor was). C/C++ likely needs `compile_commands.json` per repo
   (clangd + prism's `--compile-commands` enrichment).
2. **`corpora.toml` entries** — `[corpus.<name>]` with `lang`, `path`, `oracle`, `excludes`,
   `pinned_sha` (pin at first baseline, per the existing comment convention).
3. **Baseline + adjudicate** — `uv run tier-a --corpus <name>`, then the dual-adjudicator pass on
   pending diffs (per `eval/README.md`), recording validity vs the per-language floor. Expect some
   to floor-fail on first contact (the Python pattern) — record, don't hide.
4. **Anchor** — fold the valid ones into `docs/eval/tier-a/baseline.md` as additional
   per-language anchors (C/C++ and Java anchors alongside the Rust=`prism`/Go=`caddy` ones).

## Sequencing note

The C++/Java corpora are the *enabling* dependency for the C++/Java overload-arity follow-up in the
arity plan's "Backlog / related deferred work". The +1 Go / +2 Rust additions are independent
coverage improvements and can land separately.
