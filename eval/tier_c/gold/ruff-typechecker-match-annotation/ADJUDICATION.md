# Adjudication — ruff-typechecker-match-annotation

repo: ruff  sha: 44f6d18  symbol: match_annotation (trait TypeChecker, typing.rs:615)

## STATUS: escalation — full closure fails the |gold|<=60 admission ceiling; this file ships a
proposed narrowing (is_list + is_dict sub-closure, 28 sites) for controller review, per
"propose a scope narrowing to the controller (do NOT truncate)".

## Closure walk (full picture, source-verified)

**Hop 0** — `git grep -nw match_annotation` = 4 files / 16 hits (confirms the corpus's own
stats exactly). Breakdown: trait decl (typing.rs:615) + `T::match_annotation` calls inside
`check_type` (typing.rs:646/699/712/724) + `BuiltinTypeChecker` default body + blanket-impl
forward (typing.rs:743/773/774) + 4 direct impls in typing.rs (IoBaseChecker:848,
PathlibPathChecker:945, FastApiRouteChecker:974, TypeVarLikeChecker:1013) + **3 direct impls
OUTSIDE typing.rs not in Fable's starting enumeration**: `HttpxClientChecker`
(flake8_async/rules/blocking_http_call_httpx.rs:59-60), a **locally-scoped** `PathlibPathChecker`
distinct from typing.rs's own (flake8_async/rules/blocking_path_methods.rs:161-162), and
`SameClassInstanceChecker` (flake8_self/rules/private_member_access.rs:278-280, plus a
same-impl recursive `Self::match_annotation` at line 312 — excluded, part of the impl's own
body, not an external caller). **prism's `nav_callers` on `match_annotation` independently
corroborated this: it returned `AmbiguousSymbol` listing exactly these same 7 impl sites**
(candidates.json, cross-check run) — a useful independent confirmation that the impl count
found by source-reading matches what a symbol-name lookup sees. rust-analyzer (LSP) found only
2 direct callers from ONE seed impl (typing.rs:743's blanket forward): `check_type@724` and
`private_member_access.rs:312` (the same-impl recursion) — demonstrating the LSP-fallibility
point from the methodology (single-seed call-hierarchy under-covers a fan-out trait).

**The forwarder (hop 0→1):** `check_type::<T>` (typing.rs:625) — monomorphized dispatch over
`BindingKind`, calls `T::match_annotation` or `T::match_initializer`; no domain logic of its
own. Thin, confirmed forwarder.

**Hop 1** — `git grep -nw check_type` finds **12** named wrapper fns in typing.rs (not "~10"):
is_list, is_dict, is_int, is_float, is_string, is_bytes, is_set, is_tuple, is_io_base,
is_pathlib_path, is_fastapi_route, is_type_var_like — **plus a 13th hop-0-level forwarder**,
`is_io_base_expr` (typing.rs:1116), which calls `IoBaseChecker::match_initializer` directly,
bypassing `check_type` entirely. Also at hop 1: the 3 direct-impl files' own rule fns
(`blocking_http_call_httpx`, `blocking_os_path` — both CONSUMERS, terminal, call check_type
directly with their own struct) and `is_same_class_instance` (private_member_access.rs:186,
FORWARDER — `is_method_receiver(...) || check_type::<SameClassInstanceChecker>(...)`, same
OR-aggregation-with-orthogonal-precondition pattern as is_dict/is_tuple, recurses to its one
caller `private_member_access` itself, same file).

**Hop 2+ (the real finding — depth is non-uniform, not "flat is_* -> consumer"):**
- 10 of 12 wrapper families terminate in 1 consumer hop, matching the corpus's framing —
  **but roughly half of THOSE route through an intermediate same-file private detector fn**
  before the real top-level `pub(crate) fn <rule>(checker: &Checker, ...)` entry point, adding
  a genuine extra (file,symbol) per the letter of the collapse rule: `delete_full_slice.rs::
  match_full_slice`, `repeated_append.rs::match_append` (+its own private relay
  `match_consecutive_appends`, collapsed), `slice_copy.rs::match_list_full_slice`,
  `function_signature_change_in_3.rs::is_dict_expression` (+relay
  `check_constructor_arguments`, collapsed), `unnecessary_comprehension.rs::is_dict_items`,
  `dict_index_missing_items.rs::is_inferred_dict`, `key_in_dict.rs::key_in_dict` (**shared by
  3 separate top-level rules** — key_in_dict_for/_comprehension/_compare, 4 entries alone),
  `flake8_use_pathlib/helpers.rs::is_file_descriptor` (shared by 3 more rule files:
  builtin_open.rs, os_chmod.rs, replaceable_by_pathlib.rs), `unnecessary_from_float.rs::
  is_valid_argument_type`, `unnecessary_round.rs::rounded_and_ndigits`, `readlines_in_for.rs::
  readlines_in_iter` (shared by 2 top-level rules), `invalid_pathlib_with_suffix.rs::
  is_path_with_suffix_call`, `bad_str_strip_call.rs::ValueKind::from`,
  `multiple_starts_ends_with.rs::is_bound_to_tuple`.
- **2 of 12 wrapper families have long same-file tails**, far past "2 hops":
  - `is_type_var_like`: typing.rs → `class.rs::might_be_old_style_typevar_like` →
    `class.rs::expr_might_be_old_style_typevar_like` → `class.rs::expr_might_be_typevar_like`
    → `class.rs::might_be_generic` (pub fn) → `flake8_pyi/non_self_return_type.rs::
    replace_with_self_fix` (consumer, uses the result for Fix applicability). **5 more hops.**
  - `is_fastapi_route`: typing.rs → `fastapi/rules/mod.rs::is_fastapi_route_call` (has its own
    attr-allowlist gate, judged still-thin per the is_dict/is_tuple precedent) →
    `::is_fastapi_route_decorator` → `::is_fastapi_route` (outer) → 2 terminal consumers
    (`fastapi_non_annotated_dependency.rs`, `ruff/rules/unused_async.rs`). **4 more hops.**

## Size estimate (full, unnarrowed closure)

Rigorous per-(file,symbol) collapsing across all 12 wrapper families + the 3 direct-impl files
+ the 2 long tails lands at an estimated **65-90+ distinct gold sites** — this EXCEEDS the
admission ceiling (|gold| <= 60). I did not push for an exact final number once several
independent lines of evidence (the 3 unlisted direct-impl files, the 12-not-10 wrapper count,
the 2 long tails, and the systematic "private-detector-then-top-level-rule" extra hop found in
~13 of the ~30 hop-2 consumer files I checked) made the >60 conclusion solid; further precision
would not change the admission verdict.

## Escalation / proposed narrowing

Per spec: "If the closure exceeds 60, STOP and propose a scope narrowing to the controller (do
NOT truncate)." I did not truncate the full closure — I did not attempt to freeze a 60-site
slice of it by arbitrary cutoff. Instead this `gold.json` ships a **different, smaller,
fully-independently-verified target**: the **is_list + is_dict sub-closure only** (the two
most heavily-consumed wrapper families; `unnecessary_enumerate.rs` and `literal_membership.rs`
already exercise is_set/is_tuple alongside them for free). **28 sites, D1=25 (89%)**,
comfortably inside [8,60], hops 2-3 depending on branch (private-relay branches add a hop).

Alternative narrowings for the controller to pick instead (not built, estimated only):
- **(A) all 12 wrappers, drop only the 2 long tails + 3 direct-impl files** — does NOT rescue
  admission by itself (~69 sites estimated); the depth problem is systemic (the private-relay
  pattern), not isolated to the 2 tails.
- **(B) is_list + is_dict + is_set + is_tuple** (the "container" family) — ~35-40 sites
  estimated; closer to the corpus's "~10 wrapper fan-out" framing than (this file's) narrower
  pick. **Recommended if the controller wants broader wrapper coverage.**
- **(C) is_dict alone** — ~17 sites; thinner D1 illustration, least interesting.

## D-membership

D-membership is keyed to **match_annotation's own name** (per spec: "d_member ... FILE has ZERO
textual occurrence of S's name"), repo-wide total = 16 hits (well under the D2 >100 threshold,
so D2 is structurally unreachable for this task — every site is either D1 or "none"). Verified
via `git grep -c -w match_annotation <file>` for all 17 candidate files: all 16 hop-2/3 consumer
files = 0 hits (D1); typing.rs (hop 0/1, containing check_type/is_list/is_dict) = 12 hits
(d_member="none", as expected — same file as S). No test-dir files encountered.

## Dry-run (scorer sanity check)

```
perfect arm:       file_f1=1.0    d_recall=1.0  gold_size=28  d_gold_size=16  phantom=0
grep-S-only arm:   file_f1=0.111  d_recall=0.0  gold_size=28  d_gold_size=16  phantom=0
```
(`grep-S-only` simulates an off-arm that only found `check_type` via grepping `match_annotation`
itself — the exact degenerate case the task is designed to catch. It scores near-zero, as
expected: the is_list/is_dict consumer layer is genuinely invisible to a match_annotation-only
grep.)

## Uncertain sites flagged for controller

1. **`is_dict`'s kwarg early-return** (typing.rs:1040-1050) and **`is_same_class_instance`'s
   `is_method_receiver` OR-branch** (private_member_access.rs:192): both combine an orthogonal,
   S-independent precondition with the real `check_type`/`check_type::<Checker>` call via
   boolean OR. I classified both as thin FORWARDERS (matching the is_dict/is_tuple precedent
   explicit in the methodology's thinness test: "boolean aggregation over homogeneous
   forwards"). A stricter reading could call these CONSUMERS (their own domain logic partly
   insulates the contract) — this would not change is_dict's classification (still gold either
   way, just role/hop_distance shifts) but WOULD move `is_same_class_instance` and
   `SameClassInstanceChecker`'s whole downstream chain out of scope even under a broader
   narrowing. Flagged, not resolved unilaterally.
2. **`fastapi/rules/mod.rs::is_fastapi_route_call`'s attr-name allowlist gate**
   (get/post/put/delete/patch/options/head/trace) before it falls through to
   `typing::is_fastapi_route`: same ambiguity as #1, on the excluded is_fastapi_route tail.
3. **Same-file intra-function relay collapsing** (`repeated_append.rs`,
   `function_signature_change_in_3.rs`, and implicitly others): I collapsed private
   multi-step relays (e.g. `match_append` → `match_consecutive_appends` → `repeated_append`)
   down to 2 gold entries (innermost forwarder + outermost top-level consumer) rather than 3,
   reasoning that intra-file relay depth doesn't change file-level D-membership or the primary
   file-F1 metric. This is a documented interpretive choice, not a literal reading of "collapse
   to (file, symbol)" — flagged for controller override if a stricter per-symbol reading is
   wanted (would push the is_list+is_dict count from 28 to ~30-32, still well inside [8,60]).

## Exclusions (phantom bait, verified)

- `match_annotation_to_complex_bool` (flake8_boolean_trap/rules/boolean_type_hint_positional_argument.rs)
  — the corpus's named decoy; confirmed unrelated AND confirmed it never appears in the L0
  `git grep -nw match_annotation` hit list at all (word-boundary excludes the fused identifier).
- `crates/ruff_python_parser/src/parser/{mod.rs:949, pattern.rs:349,365}` — an unrelated
  `is_list(self)` method on the parser's own type; different receiver, excluded.
- `key_in_dict.rs:112,122` — a local variable named `is_dict` shadowing the import; the genuine
  `typing::is_dict` call (line 120) inside the same closure is included in gold.
- `private_member_access.rs:312` — `Self::match_annotation` recursive call inside
  `SameClassInstanceChecker`'s own impl body (Subscript case); part of the definition, not an
  external caller (also LSP-surfaced, cross-checked, and excluded here).
