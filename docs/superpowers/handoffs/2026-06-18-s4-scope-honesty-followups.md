# S4 Scope-Honesty Follow-Ups

This branch folds the local correctness findings from the R3 review for S4 scope-honesty warnings:

- reasoning-owned nested-callable boundary classification, instead of relying on navigation function-node kinds;
- conservative line-only sink handling by unioning all same-line candidate function warnings;
- trace function mapping disambiguated by variable byte span when same-name functions share a start line.

The following R3 findings are intentionally deferred until Plan B wires a consumer:

1. Unsupported-language and unmappable-trace defaults should become explicit policy.
   Today, languages outside the S4 construct catalog and trace nodes that cannot be mapped may remain silent. Before a consumer treats these warnings as part of a verdict contract, decide whether unsupported scopes should emit a coarse "not analyzed for scope honesty" warning, and add coverage for the chosen behavior.

2. The unmodeled-construct catalog should move out of `src/languages/mod.rs`.
   `Language` should classify syntax; the reasoning layer should own which syntax classes are unmodeled by the current data-flow/reachability engine and own the user-facing labels. Moving the semantic catalog into `src/reasoning/scope_honesty.rs` should happen before Plan B freezes warning semantics.

Current validation note: `tier-a --quick --allow-stale-sut` exits 2 because the baseline is invalid for corpus SHA/probe drift, not because of SUT regressions (`sut_error_rate: 0.0`, matrix 37 ok / 2 expected gaps).
