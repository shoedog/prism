# IMPLEMENTOR dispatch — js-param-defs PR-A binding shapes

Start from committed prototype `1b2dfdc93a37dbd87c6359c34e7723912399b682` (parent/base `c8de720b36c24ae8a7ab274ec10c994f258eb336`) plus `repair-r1/R1-src.patch`. The packet docs are `6b0be4ff` plus `repair-r1/R1-docs.patch`. Historical prA3/prA3fmt have prototype-equivalent source. Read SPEC D1–D13 and its R1 disclosures before dispatch. Exact is a static-binding grade; binding CORRECT does not prove runtime flow.

## Goal and workspace
Register Defs for bare `x =>` and identifier rest formals in JS/TS/TSX, preserving existing correct rows and non-JS byte identity. The measured unmodified-package target is six rest traces and one arrow trace; two other arrows retain downstream residuals. Use the controller's implementation clone/branch from the pinned prototype and R1 patch. Use `~/prism-evidence/js-param-defs/cache`. Never open frontend-portal or execute corpus packages. In the repair sandbox no git writes are authorized; the controller commits the patches.

## Final change table
| Site | Behavior | Boundary |
|---|---|---|
| `ast.rs` shape helpers and occurrence extraction | Bare arrow yields its identifier; rest yields the inner identifier bytes | Keep `find_parameters_node`'s list contract; keep destructured/defaulted/optional rest refused |
| `parameter_slots.rs` TS occurrences | Bare identifier fast path; required_parameter/rest_pattern(identifier) under the existing child allowlist | Whole-list recovery/duplicate/escape checks remain; TS `this` stays excluded |
| New-shape D11 admissions, all four call sites | Refuse any competing binding in the owner's body: nested formals, catch, for-in/of heads, variable patterns, named functions/generators/classes, TS abstract classes/enums/namespaces; escaped binding ambiguity refuses | Apply only to new shapes; unrelated names/captures remain; PR-B needs a complete containment proof before retiring this guard |
| Rest signature guard | Final named parameter and no comma after the rest, including comments | Tree-sitter accepts these early errors without recovery; PD-12 now disables the combined admission guard, PD-25 independently checks comma refusal |
| `compute_param_def_nodes` | Incoming bare-arrow slot remains a hole | Rest remains variadic; do not change `slots()`; `(x) =>` keeps inherited incoming behavior |
| CPG assembly after argument/return flow | Reverse-reachability from inferred, unreferenceable callee argument edges; suppress only outgoing new-source edges reaching that region | Includes paths through locals and other callables. Retain static Defs, existing call/argument edges and raw DFG for incremental reconstruction. This closes sol W2's outgoing E5 exposure |
| Exact-read predicate and RD seed | Bare formal is plain required; parameter_binding_region seeds it as a formal | Rest remains excluded from byte-distinct caller-read expansion; leave introduction_is_classified unchanged |
| Cache | CPG 106→107, call-edge version stays 62 | One PR transition; P2-M11 and PD-11 intentionally share the cache-pin obligation |
| Probe/controller packet | Owner/byte-aware dump and exact checker declaration identity; hidden files admitted; explicit SecBench failure intersection | Zero-width endpoints need unanimous represented bindings; mixed/absent remains UNDECIDED; failed/partial producers never enter aggregates |

## Tests and intended re-pins
Run all `js_param_defs`, `required_parameter`, and `nested_execution_owner` tests. The shape suite includes byte Defs, labels, slots/Step 5b, captures/unrelated binders, each review repro, TS declaration variants and early-error comma comments. W3 has executable oracle controls showing the old oracle's false CORRECT and the byte oracle's WRONG. W6 has an old/new hidden-file census control. W4/E8 and F2/E7 have same-environment plain-formal base controls; those disclosed parity defects are not represented as fixed regressions.

The only four intended old assertion re-pins are `rests(...items)`, `take(...a)` (keep destructured rest unsupported), `(value?, ...rest)`, and `p=>sink(p)`. Starting from the prototype already includes them; R1 does not authorize further re-baselines. PD-01…PD-28 must be admissible and KILLED. Preserve lane P's population; its P2-M11 duplicates PD-11 and must be reconciled on every later cache transition.

## Gates and measurement
1. Touched tests, including nested_execution_owner.
2. `cargo nextest run --features mcp`; report totals and the existing ignored test. A new behavior change after a green run requires the corresponding final suite run; do not transfer source-bound evidence across it.
3. Advisory scoped mutation: `python3 scripts/mutgate/mutgate.py --since origin/main --scope fn`. The scope selects 29 obligations, including the coupled P2-M11; the authoritative PR-A lane selects 28. Authoritative lane mutation runs serially against the same final source.
4. `cargo fmt --check`; clippy `--all-targets --features mcp -- -W clippy::all`, with the same-environment base control and no new warning in touched code. `-D warnings` is not the approved gate.
5. Fresh release build in this worktree; matrix 178/178 and quick VALID, retaining the r2 TS/JS comparison frame. No re-baselining.
6. `probes/measure-bytes.py`: X, Xi, T, SecBench and Python/Go/Rust controls. Project byte records back to wire identity and compare against actual `dfg-stats --edges` output. Diff owner/span-aware multisets and adjudicate exact identifier bytes. Price R1's forfeited prototype gains separately from LOST correct base rows.
7. SecBench same `measure()` on unmodified packages, target and GT-eligible rows, plus payload-specific BFS. Exclude failed row producers by the union of failures on either side, recording per-side time/status. Explicitly list clean-css and natural as prism_error on both eligible sweep sides.
8. Controller F: use the revised six-argument `CONTROLLER-pd.sh diff TS_JS BASE_BIN HEAD_BIN BASE_BYTES_BIN HEAD_BYTES_BIN`, a new PRIVATE_EVIDENCE_ROOT and separately supplied CORPUS_F_ROOT. This repair does not access F.

## STOP and disclose
STOP on LOST checker-correct base rows, non-JS non-identity, changed call-site rows, new checker-WRONG binding outside expressly disclosed E7, or unproved/mixed binding classes. Do not silently extend the capped review loop. E7's non-computed property key is a pre-existing WRONG routed to PR-C; E8's unreachable-rest flow is a pre-existing WRONG proved by same-base plain parity as the repair brief permits. Complete independent CFG flow verification remains unperformed. F and the second capped review remain controller work; patch readiness is not merge/adoption authority.
