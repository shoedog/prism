# P3 control expectations (written BEFORE running, 2026-09-25)
Binary: prism-wrapped-export target/release/prism @ origin/main 12ca6e8e
C01 A: Exact import_member -> lib.tsx:A (probe-validity control)
C02 Island: drop UnknownName (the measured gap)
C03 Island same-file: Exact local_def -> arrow named Island (function_name Pattern 3)
C04 export {Island}: Exact import_member (H-list: list path records Local(name) with no initializer check)
C05 export default Island: Exact import_member (same H)
C06 F4 hazard via export list: Exact import_member -> NESTED f (false Exact on main, if H-list holds). Falsifier: drop.
C07 F4 hazard via declarator: drop UnknownName (pinned by P4 test)
C08 memo(forwardRef(arrow)): drop; inner arrow absent from functions inventory (anonymous, Pattern 3 needs declarator grandparent)
C09 forwardRef(IslandInner): drop
C10 same + nested Island: drop
C11 wrapped + nested same-name elsewhere: drop
C12 local impostor forwardRef: drop
C13 zustand create(arrow): drop
C14 re-export chain: drop
C15 React.forwardRef(function Island): drop
C16 styled.div``: drop
C17 multi-declarator: drop both
C18 importer param shadow: not import_member
C19 observer(arrow): drop
C20 let reassigned wrapped: drop
C21 let reassigned plain arrow: Exact import_member (existing hazard: mutable binding not checked)
C22 default-object alias <Stack.Row>: drop
C23 debounce(arrow,100): drop
C24 createSelector(a, arrow): drop
C25 forwardRef(...) as any: drop
C26 `import React` + React.forwardRef arrow: drop on base; Exact under prototype v2+ (added r2 to the generator; the original pre-run hypothesis record for C26/C27 is P6a-debug-expectations.md)
C27 `import * as React` + React.forwardRef arrow: same as C26
C28-C46: see P18-expectations-pre-run.md (r2)
