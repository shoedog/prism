# Imported/exported type identity audit — no resolution expansion

Approved after PR255 merge e298ae7c. Branch audit/imported-props-identity;
three-round SELF-PASS cap, no subagents. Scope is source audit, proof requirements,
negative fixtures and executable characterization only. No src/ edits, cache-version
change, React.FC recognition, hook inference or imported Props resolution.

## Plan and gates

1. Bind fixed source0642e72cfa2d9a71198200e52f37399384610ee3 to the measured five-file
   slice, deduplicate source spans and trace receiver→annotation→declaration edges.
   Audit tracked TypeScript module/global declarations; do not infer dependency or
   compiler-program closure from a partial source slice.
2. Compare official TypeScript semantics with compiler-backed minimal examples:
   type/value spaces, renames/reexports, member declaration scope, module/global
   augmentation, incompatible property declarations and default/named visibility.
3. Commit a shared fixture corpus, compiler verifier and TS/TSX full/direct-subset
   rejection tests with existing supported positive controls. These characterize
   unchanged behavior, not RED-first implementation of new resolution.
4. Record explicit future proof requirements and cache invalidation obligations;
   separate valid deferred syntax from ill-typed inputs and framework prerequisites.
5. Full default/MCP suites, fmt/clippy, source/fixture verification. No production
   change means no new runtime behavior claim; no rebaseline or full multicorpus.

## Initial correction and hypothesis log

Hypothesis: imported Props identity is the immediate missing proof at the six
remaining unique spans. Alternative: context/return producers are unsupported.
Source inspection found four React.FC contextual spans and two useApp/useContext
spans. The latter uses exported type alias AppClassProperties, not a Props interface,
and that shape contains a method member. Thus imported Props support alone is not a
six-site recall fix. Preserve framework exclusions and report the weaker foundation.

Prism helper lookup returned SymbolNotFound, not absent-caller evidence. No LSP tools
are available; compiler-backed examples replace unavailable type queries, not a
whole-application semantic check. Installed TypeScript6.0.3 differs from upstream's
pinned5.9.3; version scope must be explicit in the final audit.

## Completed audit

Rounds1–3 completed within the declared cap, SELF-PASS (NOT INDEPENDENT).
The [readout](../../eval/receiver-closure/2026-09-05-imported-props-identity-readout.md)
records the source trace, compiler probes and requirements P1–P8: type namespace,
module/export identity, defining-declaration scope, supported shape, augmentation
closure, class ownership, independent producer proof and persisted-evidence inputs.
All605 tracked TypeScript source blobs are pinned; this is not dependency/program
closure. The24-case shared corpus supplies96 TS/TSX full/subset comparisons;
three custody tests reject changed source bytes and added declarations. These are
baseline characterization and audit-tool mutation controls, not runtime RED/GREEN.
Default4005/0/1,MCP4195/0/1,matrix159/159 passed. No resolution expansion authorized
by these results; selection of a concrete producer/consumer remains a future decision.
