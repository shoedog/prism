# Bounded JS/TS/TSX exact parameter-token binding

Base: PR311 merged `b2b141cd5605f9b7f74b32a2fafdeb9c78619bc9`.
Publication-only predecessor commit5ce80a86 carried asb9c8c59d; no docs-only PR.

## Demonstrated defect and alternatives

Step5b uses positional names but scans all callee lines for the first same-path
Def. When a default/optional parameter has no supported parameter occurrence,
an overwrite or alias-resolved local declaration can receive the call argument.
The prior source audit classified3 public and61 private surviving observations.
Those counts are inherited until fixed-snapshot remeasurement.

Hypothesis: endpoint selection is wrong, independently of callee resolution.
RED fixtures must retain a resolved call and a later body Def while refusing the
argument edge; required-parameter controls must retain exact-token edges.
Alternative: wrong callee selection or caller occurrence collision. Inspect
resolved callee identity and exact endpoint bytes; put independent calls on
separate lines so existing same-line indexing does not confound the proof.

## Contract and architecture

For JS/TS/TSX only, compute a callee-local positional vector of optional Def nodes:

1. Identify one actual callee AST function by resolved name and full line range;
   absence/ambiguity refuses. Do not recover authority from name-only metadata.
2. Read `function_parameter_slot_occurrences`, preserving slot positions and its
   duplicate/recovery/truncation contract. No fallback to non-positional names.
3. Intersect each slot token with `function_parameter_occurrences` by exact name
   and UTF-8 byte endpoints. Unsupported slots remain holes, never compressed.
4. Select only the existing function-entry Def whose graph file/function/start,
   access path, Def kind and byte endpoints match that slot. Missing, substituted
   or mismatched nodes refuse. No body-name search or manufactured definition.
5. Memoize by complete callee FunctionId within a single immutable graph build.
   Parallel production and serial reference share this selection helper; their
   independent traversal/edge collection still must agree.

Non-JS selection retains the existing compute_param_names and body lookup exactly.
Caller-side field/base supplementation, confidence, writes, graph occurrence
indexing and return binding do not change. CPG cache80→81; navigation cache stays.
No default/optional/rest/destructured/arrow occurrence expansion; no React.FC,
closure/admission, react-scripts, or source-rewrite change.

## Verification and checkpoints

- Capture complete focused RED population before production edits, including
  required/multiline positives, default/optional/body-local negatives, positional
  holes, duplicate/recovery/destructuring and same-line refusal controls.
- Check per-field graph-node substitution and non-JS retained behavior.
- Full Rust default/MCP/audit, observers/helpers/authority, Python, all examples;
  immediate rebuild before Tier-A matrix and quick. Preserve invalid oracle
  outcomes and same-environment controls; no rebaseline.
- Replay unchanged observer/comparator on identical public/private snapshots;
  classify every added/removed flow, not just the inherited64 suspect flows.
- Review cap2. Closed bounded findings fixed in place; open-class findings at cap
  stop for design. Keep one implementation PR unless bounded evidence requires
  an explicit second slice. Commit/push/PR, no auto-merge; no policy bypass.
- Private source, identifiers and raw graph rows stay local; aggregate-only
  readout and custody metadata may accompany the implementation PR.
