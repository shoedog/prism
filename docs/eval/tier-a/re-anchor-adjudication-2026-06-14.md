# Tier-A re-anchor adjudication — 2026-06-14 (the S2 anchor)

Dual-adjudicator record for re-anchoring Tier-A onto **prism @ `dd60ed6`** (post-S2
node-identity merge to `main`), from the human-triggered `uv run tier-a --corpus all`
of 2026-06-13. Companion to `baseline.md`; sibling to the pre-S2
`2026-06-11/12` anchor (preserved in git history) and the
`target-c-method-flip-adjudication-2026-06-14.md` flip analysis.

## What was adjudicated

The five per-corpus reports carried **292 net-pending adjudicable diffs** (already net
of the 1,244-record prior store; the much larger raw counts were retired by S2 churn —
see "stale" below). All 292 were adjudicated, taking the store to **1,536 records**.

| Corpus | Anchor? | Pending adjudicated | Direction split | Protocol |
|---|---|---|---|---|
| prism | ✅ Rust anchor | 182 | 14 prism_only / 168 oracle_only | dual (codex + claude) |
| caddy | ✅ Go anchor | 72 | 0 / 72 | dual (codex + claude) |
| tokio | ❌ supplementary | 19 | 7 / 12 | solo (claude) |
| flask | ❌ supplementary | 8 | 0 / 8 | solo (claude) |
| click | ❌ supplementary | 11 | 0 / 11 | solo (claude) |

## Protocol (anchors)

Identical evidence to both adjudicators: each diff hydrated to `seed_def` + `seed_context`
(±2 lines) and `site` + `site_context` (±3 lines, `>`-marked exact line), judge-from-
evidence-only (no filesystem browse). Corpus source read at each report's recorded
`corpus_sha` (all working trees matched: prism `dd60ed6`, caddy `77e9ce74`, tokio
`ecb5125`, flask `36e4a82`, click `8a1b1a3`).

- **codex** gpt-5.5 xhigh — a2a-bridge `adjudicate` workflow, read-only, **no MCP**
  (pure judge-from-evidence), `examples/a2a-bridge.tier-a-adjudicate.toml` +
  `prompts/adjudicate-sample.md`, 3 batches (prism 2×91, caddy 1×72).
- **claude** opus-4-8 — operator, independent pass over the same hydrated evidence.

### Agreement

| Anchor | items | raw agreement | Cohen's κ | disagreements |
|---|---|---|---|---|
| prism | 182 | 180/182 = 0.989 | **0.923** | 2 |
| caddy | 72 | 70/72 = 0.972 | **0.900** | 2 |

All 4 disagreements were tiebroken by the operator reading source (the deciding fact
sat outside the ±3 context window codex was limited to, or was an inference miss);
**every tiebreak resolved to the claude verdict**:

| id | claude | codex | source fact → final |
|---|---|---|---|
| prism 152 | prism_fp | oracle_miss | `self.graph.edges(node)` is petgraph `Graph::edges` → **prism_fp** |
| prism 153 | prism_fp | oracle_miss | `edge.target()` is petgraph `EdgeRef::target`, not the in-repo `target` fn → **prism_fp** (the canonical collision) |
| caddy 59 | ambiguous | prism_fn | `trustedLeafCertloaders := []LeafCertificateLoader{}` (interface slice) → dispatch → **ambiguous** |
| caddy 64 | prism_fn | ambiguous | `func testParser(input string) parser` returns a concrete type → `p.Next()` real → **prism_fn** |

## Adjudication policy (this re-anchor)

- **prism_fp** — std/library/trait-on-other-type/petgraph method-name collision: the site
  calls a same-named method of a *different* type (`Vec::truncate` vs `AccessPath::truncate`,
  `BTreeMap::default` vs `*Config::default`, tuple `.cmp` vs `VarLocation::cmp`, petgraph
  `.edges()`/`.target()`, qualified `io::Write::write` / `std::time::Instant::sub`).
- **prism_fn** — a real, source-visible call prism missed: qualified free-func / package
  calls (`caddy.ProvisionContext`, `caddy.ToString`, `caddy.RegisterModule`,
  `output::*`, `crate::…`), method calls on concrete-typed receivers / constructor-locals
  (`dfg.all_defs_of`, `parsed.enclosing_function`, `provider.resolve_type`, `p.Next`,
  `rep.Set`), `super().m()` and inherited-`self` calls, and calls inside macro args
  (`assert!(detect_weak_hash_identity(...))`). A real call to external/std code with no
  in-repo def is still prism_fn (the harness measures site-level detection); the
  callee-recall narrative below flags the external-scope component.
- **ambiguous** — dynamic/interface dispatch where evidence can't fix the concrete
  receiver: Go `x.(Module).CaddyModule()` / interface-typed `loader.LoadLeafCertificates()`
  / anon-interface `annoying.SetConfig()`, and embedded-field `l.Listener.Accept()`
  (the embedded `net.Listener`, not the seed). Excluded from corrected P and R — fair to
  both analyzers (prism correctly declines these; the oracle's interface-satisfaction is
  the liberal model).
- **oracle_artifact** — not an in-repo source-level call edge: pin-project attribute-macro
  `self.project()`, enum/tuple-variant constructors counted as calls (`Ok(())`).

## What the verdicts revealed

- **prism precision stays high; method-call *recall* is the real gap, now quantified.**
  The 168 prism oracle_only were overwhelmingly real method calls on receiver-typed
  locals (mostly in tests) prism cannot resolve — the P6-lite/Phase-IP receiver-typing
  gap. Corrected `callers/C-method` recall is **0.121** (was an optimistic 1.000 when
  pending was excluded). The 14 prism_only are all name-collision **prism_fp**.
- **caddy interface over-attribution is correctly excluded.** 57 of 72 caddy diffs are the
  same ~19 interface-dispatch sites repeated across three `CaddyModule` implementers
  (StderrWriter / Filesystems / Gzip) — gopls interface-satisfaction → **ambiguous**, so
  caddy `callers/C-method` recall holds at 1.000 (residual pend=48 is non-adjudicable
  `inventory_miss` interface-method declaration seeds). Real caddy recall gaps are the
  qualified/cross-package calls (`ProvisionContext`, `RegisterModule`, `ToString`).
- **The recall gaps trace 1:1 to the G5 capability `expected_gap`s** — go/embedded_method,
  go/interface_dispatch, python/from_import_alias, python/inherited_override — i.e. the
  Phase-IP work-list. flask/click oracle_only are dominated by `super().m()` /
  inherited-`self` (python/inherited_override).
- **High stale counts confirm S2 reshaped the graphs**: caddy 471, tokio 427 prior
  line-keyed adjudications fell out of the live diff (S2 resolved/changed those sites).
  Stale records are ignored by the corrected metrics (neither pending nor correction);
  fingerprint re-anchoring remains the planned durability migration.

## Reproduction / tooling

- Hydration: `hydrate_pending.py` (built for this run; reads each report's `pending`,
  resolves contexts at the corpus SHA). Candidate to land in `eval/` — the harness has no
  built-in adjudicator-input step. Currently staged in the run scratch dir, not committed.
- Replay: `uv run tier-a --report-only runs/2026-06-13-<corpus>.json` after appending the
  292 records to `eval/adjudications.jsonl` (legal-combo validated on load).
- Supplementary (tokio/flask/click) were solo-claude and **non-anchoring** (oracle floors
  breached); recorded for signal and to drain their pending, not to anchor.
