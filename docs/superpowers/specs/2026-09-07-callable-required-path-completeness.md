# Bounded required-path completeness repair

Successor to merged PR277, exact base `8534f4af7bafd069728faa4ebbe4d5fc062e3edd`.
Owner authorized the recommended repair. Fix the demonstrated skipLibCheck false
completeness now; keep actual type/lib channel observations and the additional
unresolved `react-scripts` directive for the next slice. No application install,
lockfile/config edit, ambient closure admission, or React.FC/runtime expansion.

## Contract and architecture

The pinned TypeScript5.9.3 Program processes triple-slash paths independently of
module literals; skipLibCheck/noCheck can suppress the corresponding diagnostics.
After observations but before the second snapshot and closure calculation, census
every original Program source's `referencedFiles`, including declaration files
and `redirectInfo.unredirected`. Verify that each consulted original SourceFile's
text equals the captured UTF-8 read and shares the canonical Program owner ID.

For each occurrence, use `program.getSourceFileFromReference(source,ref)`, not a
lexical existence test or independent resolver. Require the exact returned object
in the Program, a different canonical target ID (no self-reference), and a
`getFileIncludeReasons()` ReferenceFile entry with the owning Program source path
and directive index. This rejects incidental root membership under noResolve and
original package-redirect directives never traversed by the compiler. These are
unproved occurrences, not necessarily files absent from disk. Inspect all sources;
do not stop the census at the first unresolved directive.

Compiler-source basis (`lib/typescript.js`, hash pinned by schema):
resolveTripleslashReference126174; getSourceFileFromReference128496 and worker128499;
package redirect128691 returns before path processing128734;
addFileIncludeReason128750; processReferencedFiles128788 assigns kind4/source/index.
The cache reader does not resolve or acquire new files. Read/heap/time/file budgets,
source stability, case canonicalization and link policy remain enforced by the
existing host/inventory; no cross-Program cache is introduced.

Any unproved occurrence adds `unproven_path_reference` and withholds dependencies,
references, augmentation and resolution. The existing Program-unproven downgrade
withholds Props/class candidates. Ordinary failed module/extension candidates do
not add this reason. Project-reference/plugin/outside/unsupported/module/diagnostic
and duplicate/write barriers remain independent. Both authority flags stay false.
`closure.references=true` is still NOT complete type/lib/config reference evidence.

Producer0.11.1 extends the refusal vocabulary without changing schema10's field
shape. Parser requires the new reason's four false bits and producer0.11.1 before
audited-root I/O. Historical0.11.0 schema10 packets stay readable for pinned audits;
they cannot validate as current because full recomputation compares producer and
every field. The helper is part of producerHash. No portable per-directive ledger
is added in this bounded repair; a later versioned reference-channel slice owns it.

## RED and plan

1. Exact-base regression capture before implementation:19 tests,14 fail/5 pass.
   Behavioral failures include missing paths under suppressed diagnostics,
   extensionless/unsupported/self paths, noResolve, transitive imported files and
   original redirect sources. Non-skipped/outside cases already refuse globally;
   their new-reason failures prove reporting changes, not new false closure.
   Five base-passing positives are compatibility controls, not RED.
2. Implement the bounded Program-cache predicate and pre-I/O refusal consistency.
3. Add parser forgery/stale packet, source identity and remaining edge controls;
   re-run identical final tests on base and current with captured output.
   Final24: base7 pass/17 fail; current24 pass/0 fail. Repeated-index/BOM/noResolve
   no-directive positives stay compatible; removal of the refusal and promotion of
   every closure bit is rejected by full recomputation even when well-shaped.
4. Replay fixed public source without edits; compare all module/receiver/snapshot
   fields, report new path dispositions separately. Keep14 module source gaps and
   the unresolved react-scripts type directive explicit.
5. Full observer, Rust default/MCP, helper/authority gates; two SELF-PASS reviews
   (NOT INDEPENDENT), then commit/push/open PR. No auto-merge authority assumed.

## Hypothesis / probe / result

- Hypothesis: diagnostics suppression hides required-path failures, not just an
  optional failed search. Exact-base fixture: absent.d.ts plus zero module requests
  reports observed with skipLibCheck/noCheck; an extensionless resolved directive
  and ordinary failed module candidates remain observed. Captured RED confirms.
- Alternative: inventory or Program membership proves the path. noResolve with
  an independently included target and redirected-original source both remain
  falsely observed on base. Compiler inclusion source/index is the discriminator.
- Missing react-scripts is an additional source-environment type-reference gap,
  not the same required-path defect. Disposition remains pending the separate
  actual type/lib observation slice; no package installation inferred from this fix.
- Full-suite failure hypothesis: the expected producer file list omitted the new
  helper, rather than producerHash omitting implementation bytes. Captured actual
  hash includes required-paths.mjs; exact-base digest test passes1/1. Initial control
  without profiles was inadmissible; corrected environment passes. Updated list
  makes all301 observer tests pass; no production change was needed.

Raw evidence root: `/private/tmp/prism-required-path-o41JkY`.
