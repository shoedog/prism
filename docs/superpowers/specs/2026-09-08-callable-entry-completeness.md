# S5 — conservative entry-obligation completeness

Schema17 / producer0.18.0 adds `entry_obligations:{complete,reasons,rows}` separately
from old closure.references. Preserve every old field/reason/closure decision and
both false authority flags. No host operation, resolver, Program or acquisition
change. Source references and module/augmentation/diagnostic barriers stay separate;
react-scripts remains unresolved, not a configured-entry waiver.

Each type_lib_entries row maps exactly once, in existing order, to
`{kind,origin,index,disposition,reason}`. Keep duplicate occurrences and original facts.
Observed old rows become selected/null. Only old unprocessed rows can be disabled:
zero roots => no_roots for type/lib; otherwise lib plus proven noLib:true => no_lib.
Other unprocessed libs become unproven/suppression_unproven; type unprocessed and
other old refusal reasons retain their labels. Never fabricate targets or infer
suppression from diagnostics, membership or a source no-default-lib pragma.

The config predicate requires observed S4 provenance. noLib must be explicitly
present with digest canonical({present:true,value:true}); missing/unsupported config
does not prove it. Explicit types means observed types.present with S4's bounded
string[] shape, including empty arrays. With roots, absent explicit types remains
automatic_discovery_unproven even with zero listed names or all listed names resolved.
Explicit typeRoots alone is not an external-universe proof in this slice.

Reasons have fixed order: configuration_unproven if config is not observed;
automatic_discovery_unproven if observed config, nonzero roots and no explicit types;
unproven_entry if any row remains unproven. Complete iff no reasons. Rootless rows
may be disabled while aggregate config completeness remains false. Empty worker
refusals have false/configuration_unproven/empty rows. Contradictory observed old
rows are retained as selected, not rewritten using a suppression predicate; actual
compiler facts are authenticated by full reproduction.

Pinned TS5.9.3 initializes skipDefaultLib from noLib at126977; entry-lib gate127159
requires roots and !skipDefaultLib. A newly processed file's no-default-lib affects
the flag only when !ignoreNoDefaultLib at128732; lib-reference traversal ignores
that pragma, path traversal does not. Existing-file revisits at128640 and late entry
pragmas do not establish earlier causal suppression. These source distinctions are
why unsupported dynamic suppression stays unknown instead of expanding the slice.

The dependency-free classifier consumes already validated old rows, root count and
derived config predicates. Parser recomputes exact rows/reasons/complete before root
I/O; full reproduction authenticates values, AST ownership and compiler facts.
Retain schema10–16, including batch guards, and include helper bytes in producer
digest. Keep the100000 row bound before row construction; no truncation.

Acceptance: exact-S4 missing-field/digest contract RED, positive/negative per path,
rootless/default/configured/duplicate/noLib/empty-types/automatic-universe cases,
ignored-lib-first/revisit/late-pragma controls, missing types and old failure reasons,
forged populations/predicates/genuine substitutions/history/digest. Compare old public
fields and ordered host operations, preserve source, run full clean repository gates,
independent review cap2 and publish separately. This is not a historical old-bit
defect claim and cannot discharge source-reference or semantic-closure obligations.
