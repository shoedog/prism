# Phase-IP Slice E — caddy re-adjudication + 5-corpus re-baseline (plan, findings, κ-protocol)

> **Status (2026-06-16):** PR-2 (#97) merged to `main` (`13c3348`). Slice E is the **owner-gated**
> follow-up. This doc records the cheap structural pre-look, the agreed ordering + its rationale, the
> step plan, and the dual-adjudicator κ-session protocol. Companion: the deferred doc
> (`…-receiver-expansion-deferred.md`) §"join-precision cluster".

## §1 — Structural pre-look (cheap, no oracle / no adjudication / no rebaseline)

`prism nav interface-manifest --repo ~/code/bench-repos/caddy` (caddy @ `77e9ce7404c4`, 312 `.go` files),
`interface_dispatch_computed = true`:

| metric | value |
|---|---|
| in-scope sites (recovered receiver ∧ method ∈ some interface ∧ Go caller) | **498** |
| **true-dispatch sites (`fanout > 0`)** | **63** |
| concrete (`fanout == 0`, owner-resolved or no live impl) | 435 |
| dispatch receiver classes | `type_assertion` 24 · `typed_param` 26 · `constructor_local` 13 |
| dispatch methods | `ServeHTTP` 23 · **`CaddyModule` 14** · `Adapt` 12 · `CertMagicStorage` 8 · `ConnectionState` 2 · `LoadConfig` 2 · `AcceptEncoding` 1 · `NewEncoder` 1 |
| fanout | max **121** (every `CaddyModule` site) · 46 sites = 1 · 14 sites > 20 |
| **same-`(file,line)` collisions (across all 498)** | **0** |

**Reading it:**
- **PR-2 lights up caddy interface recall as the spec expected.** The new `type_assertion` form contributes
  **24 of the 63** dispatch sites (incl. 12 of the 14 `CaddyModule` `x.(Module).CaddyModule()` sites; the
  other 2 are `typed_param`). This is the recall the embedding+interface work targeted.
- **The MAJOR-4 dispatch/concrete split matters:** the method-name-only denominator captured 498 sites, but
  only **63** are real interface dispatch. The §8 gate report's FP metric now runs over those 63, not 498.
- **The headline precision question:** every `CaddyModule` site mints **121** live implementers (the caddy
  module registry — RTA-live set). That is either correct recall (any registered module is reachable through
  `x.(Module).CaddyModule()`) or an over-approximation liability. **This is the central thing the κ session
  judges.**
- **0 same-line collisions** → see §2.

## §2 — What the measurement decides about ordering (the X/Y question)

The owner weighed two orderings: **X** = §8 join-precision work first, then caddy re-adjudicate + rebaseline
once; **Y** = adjudicate + rebaseline, then join-precision, then adjudicate + rebaseline *again* (double, for
measurement points). The pre-look **refutes the premise of both** for caddy:

The §8 **join-precision cluster** (byte-key + seed-scoping + fingerprint re-anchoring) is motivated by
*same-line multi-dispatch* and *one site adjudicated under multiple seeds*. **caddy exhibits neither** — 0
same-`(file,line)` collisions, and each dispatch site sits in exactly one caller. So the current **line-keyed
adjudication join is unambiguous for caddy**, and the join-precision is **not** a prerequisite for a correct
caddy re-adjudication.

**Therefore (agreed):** do the caddy re-adjudication + 5-corpus rebaseline **once, now, on the current
(PR-2) code** — no need for X's speculative precision-first, no need for Y's double cycle. The join-precision
becomes a **decoupled generality follow-up** (for corpora that *do* have same-line/multi-seed sites); doing it
later will **not** force a caddy redo (byte-keying caddy's already-unambiguous verdicts is mechanical
re-anchoring, never re-judgment). This is the measure-then-decide payoff: a free structural look replaced a
guess about whether the precision was needed.

> Caveat to confirm during adjudication: 0 *manifest* line-collisions strongly implies no multi-seed verdict
> conflicts, but if the κ session surfaces a site adjudicated under conflicting seeds, fall back to the
> join-precision fix for that case (the data says it won't).

## §3 — Agreed Slice-E plan

1. **Structural pre-look — DONE** (§1). Worklist of the 63 dispatch sites:
   `docs/eval/tier-a/slice-e-caddy-dispatch-worklist.md` (regenerate via the command in §1).
2. **Caddy re-adjudication (dual-adjudicator κ) — owner-gated, operator-coordinated.** Run the caddy tier-A
   measurement to produce the prism-vs-gopls deltas, then the κ session (§5) over them. Record Cohen's κ +
   the reconciled verdicts; re-anchor stale 2026-06-14 verdicts by `adjudication.fingerprint`.
3. **5-corpus re-baseline — owner-gated (human-triggered).** `cd eval && uv run tier-a --corpus all`; deliberate
   anchor update in `docs/eval/tier-a/` with the adjudication record. This is the PR that **moves the caddy
   metric** — recorded, not silent.
4. **§8 join-precision cluster — decoupled generality follow-up** (after caddy; see the deferred doc). No caddy
   redo required.

**Prepared now (option a):** §1 worklist + §5 κ-protocol/prompts. **Gated:** steps 2 (the measurement run that
feeds the κ session) and 3.

## §4 — The precision question (what the κ session must resolve)

Every `CaddyModule` site mints 121 implementers. The adjudication must decide, per dispatch site, whether
prism's minted edge-set is:
- **TP / correct recall** — the gopls interface-satisfaction set agrees (any registered module is a legitimate
  `CaddyModule` target), or
- **over-approximation** — prism mints edges gopls would not (a precision liability the §8 gate report should
  surface), or
- **`ambiguous`** — dynamic dispatch where evidence can't fix the concrete receiver (the 2026-06-14 verdict for
  these sites was `ambiguous`; the question is whether PR-2's type-confirmed dispatch now makes them `TP`).

The 2026-06-14 record (`docs/eval/tier-a/re-anchor-adjudication-2026-06-14.md`) adjudicated these as
`ambiguous` (gopls interface-satisfaction → can't fix the receiver). PR-2 resolves them via
signature-confirmed structural satisfaction + RTA liveness — so the κ session is specifically testing whether
that recovery is **sound recall** or **over-broad**.

## §5 — Dual-adjudicator κ-session protocol (codex gpt-5.5 high + opus 4.8 xtra-high, via a2a-bridge)

Two **independent** adjudicators judge each delta site; agreement measured by Cohen's κ; disagreements
reconciled by the operator. Verdict vocabulary (per `eval/tier_a/adjudication.py` LEGAL):

- **prism_only** sites (prism mints an edge gopls doesn't): `oracle_miss` (prism right, gopls missed) ·
  `prism_fp` (prism wrong — a real FP) · `oracle_artifact` (gopls tooling artifact) · `ambiguous`
  (dynamic/interface dispatch, evidence can't fix the receiver) · `alias_site`.
- **oracle_only** sites (gopls has an edge prism doesn't): `prism_fn` · `oracle_artifact` · `ambiguous`.

**Per-adjudicator prompt (each runs independently — no cross-talk):**

> You are one of two independent adjudicators for prism's Tier-A caddy re-adjudication (Phase-IP PR-2,
> type-confirmed Go interface dispatch). For each delta site below you are given: the call site
> (`file:line` + the source line), the **recovered receiver type** + receiver class (type_assertion /
> typed_param / var_local / constructor_local), prism's **minted implementer set** (and its size = fanout),
> and gopls's **interface-satisfaction set** for the same receiver+method. Judge whether prism's edge-set is
> correct vs the gopls oracle, and assign exactly one verdict from the legal vocabulary for the site's
> direction (above). Decision aids: (1) `x.(Module).CaddyModule()` minting all live `CaddyModule`
> implementers is **correct recall** iff gopls's satisfaction set is the same registry set — then `oracle_miss`
> (prism recovered an edge gopls's call-hierarchy missed), NOT `prism_fp`; (2) `prism_fp` only when prism mints
> a target that does **not** satisfy the interface; (3) `ambiguous` when the concrete receiver genuinely
> cannot be fixed and the dispatch is open. Output JSONL: one `{site, direction, verdict, reason}` per site,
> reason ≤ 1 sentence citing the gopls evidence. Do not consult the other adjudicator.

**Operator reconciliation:** compute Cohen's κ over the two verdict streams; for disagreements, the operator
adjudicates (citing gopls), records the final verdict + both adjudicators' calls; re-anchor any matching
stale 2026-06-14 verdict by `fingerprint`. Persist to `adjudications.jsonl` (line-keyed — adequate for caddy
per §2). Bridge: `a2a-bridge run-workflow` with codex gpt-5.5 **high** + claude/Opus **xtra-high** legs
(config derived from `slicing-adjudicate.toml` / the review configs); host-run, prism-wired.

## §6 — Decoupled generality follow-up: the §8 join-precision cluster

After caddy, for corpora with same-line/multi-seed sites: seed-scope + byte-key the adjudication join +
fingerprint re-anchoring + the two design minors (share the verdict-classification helper; centralize the
Go-dispatch eligibility predicate). Full sketch in the deferred doc. Decoupled from caddy by §2.
