# #527 decision: escalate to the owner (rule 5)

Measured at prism 732863c0 (`prism` sha256 `e35c1845…7156`, see `dump-manifest.json`). Contract reading ratified 2026-10-28. Tables: `summary.md`.

## Inputs to §4

| quantity | value |
|---|---|
| E (pooled Rust Exact pairs, non-fixture, 7 real corpora) | 126,068 |
| A = (a) ∧ ¬implied | **120** (tokio 114, uv 4, ruff 2; others 0) |
| adjudicated (a) ∧ ¬implied | 58 (all tokio); decided 57 |
| confirmed WRONG in (a) ∧ ¬implied | **7**, all with a non-cfg cause; **0** caused by cfg availability |
| ŵ / w⁺ (any cause) | 12.3% / 23.2% → W⁺ = 27.9 |
| ŵ / w⁺ (cfg-caused) | 0% / 6.3% → W⁺ = 7.6 |
| confirmed WRONG in (b-std), (c) | 0 / not sampled (pooled (c) = 2, prism only) |
| F2 cost (= A) | 0.10% pooled; **tokio 1.93%**; uv 0.02%; ruff 0.004%; others 0 |
| F1 cost | 2.06% pooled; **tokio 33.2%**; axum 1.47%; prism 1.03%; uv 0.99% |
| F3 cost | 20.0% pooled (25,193 pairs whose T is false in C0) |
| statically_false targets on real corpora | 0 |

## The rule, in order

1. **Probe invalid?** No. Every fixture reproduces (`validation/`), and every corpus maps at least 99.86% of its pairs. Every adjudicated site has a green baseline in at least one configuration. The 20% red-baseline test is read **per site**: tokio's `C_test` (default features, `--all-targets`) is red for all 72 tokio sites, which is 77% of the sampled (site, configuration) cells in that one configuration. That reading is the owner's to confirm.
2. **F0?** No. There are confirmed WRONGs in (a).
3. **F2 slice?** The WRONG condition is met if read literally (7 confirmed, real corpus). But F2 costs 1.93% of E in tokio, over the 1% bound in every corpus. **Fails.**
4. **F1 slice?** No WRONG in (b-std), and F1 costs 33% of tokio's E. **Fails.**
5. **Escalate. This rule decides.** Its first bullet applies: a confirmed WRONG whose fence cost exceeds the 1%/2% bound in a corpus (tokio).

The 22 sampled sites that were not adjudicated (uv, ruff, prism, ripgrep, axum, clap) cannot change this outcome. Rule 2 is already excluded by confirmed WRONGs. Rules 3 and 4 fail on tokio's costs, which come from the census, not from the sample.

## What the owner is deciding

- **No WRONG in the sample is the #527 mechanism.** Every confirmed WRONG binds a different callable in every configuration that compiles the site. Five are std paths resolved to a same-named in-repo type (`std::fs::OpenOptions::new` → `tokio::fs::OpenOptions::new`; `sync::RwLock::new` with `sync = std::sync`; and a `constructor_local` `.read` on a std receiver). Two are owner-name collisions with impls inside tokio's cfg-wrapper macros, which prism does not index (`scheduler::Handle::current`, `Timer::new`). A cfg fence would hide these, but each has a direct fix (std/extern-qualified paths must not bind in-repo owners; index items in item-forwarding macros, or treat such a macro as a refusal) that would not demote the ~100 correct tokio pairs F2 would also demote.
- On the cfg question itself the sample shows 0 cfg-caused WRONGs in 57, with w⁺ = 6.3% and W⁺ = 7.6. That is above the rule-2 threshold of 5, so this evidence alone cannot close #527 as F0 (document only).
- tokio drives every cost. Most of its 114 (a) pairs come from the census matching types by name only (e.g. four distinct `Builder` types, three `Handle` types). Outside tokio, A = 6 in 120,158 pairs.
- `statically_false` (`cfg(any())`) targets: 0 on real corpora, so the zero-risk ride-along fix has nothing to catch today.
- Side finding, not cfg: 1,082 pooled Exact pairs bind a **non-Rust caller** (mostly Python, e.g. `len(...)`) to a Rust fn through `free_single`.

## Disclosures that bear on the call

- The script-repair round was spent on the census (see `summary.md`). A later adjudicator defect (cargo stopping after the first failing target) was fixed with `--keep-going`, and tokio was rerun before any verdict was used. That fix falls outside the cap; it is disclosed here for the owner to accept or reject.
- Configurations: x86_64 Linux only. windows/macos/bsd/loom are unreachable here; `tokio_unstable`, io-uring and taskdump were adjudicated.
- The contract reading was applied: a site wrong under any reachable configuration that compiles it is WRONG. All 7 WRONGs are wrong under every adjudicated configuration that compiles them.
