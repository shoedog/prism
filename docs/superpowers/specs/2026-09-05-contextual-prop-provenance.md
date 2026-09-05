# Contextual-prop provenance: architecture decision

Base f348779 (PR245 and246 merged). Owner approved the contextual-prop
recommendation, with React type provenance required before Exact admission.
Review cap: three rounds per implementation slice. Owner subsequently chose
"Source-backed foundation first"; route A below is the implementation contract.

## Settled observations

- Excalidraw checkout SHA0642e72cfa2d9a71198200e52f37399384610ee3 matches the
  archived receiver census. LibraryMenuHeaderContent.tsx:38 uses ambient
  React.FC with an inline required library:Library property, without a local
  React namespace import. Four target spans:155/160/184/265.
- Its root package.json lists @types/react19.0.10 and TypeScript5.9.3. Its
  node_modules/@types/react/index.d.ts is absent. tsconfig uses react-jsx and
  package-relative configuration. Dependency intent is not declaration identity.
- src/repo_loader.rs BUILTIN_SKIP_DIRS excludes node_modules. Installing types
  alone would not make them inputs to this receiver path.
- src/type_providers/typescript.rs resolve_type ignores file/line and looks up
  bare names; resolve_generic returns None. It supplies neither contextual
  callable signatures nor a compiler project/type-resolution environment.
- prism-nav returned js_ts_scope_receiver_binding_evidence as a caller of
  js_ts_parameter_receiver_binding, with StaleIndex. Current source independently
  confirms the edge; the warning prevents treating old coordinates as current.
- The shared AST receiver path can reuse TypedParam, lexical/type shadows,
  declaration timing and member-write guards once its input is proved.

Hypothesis/probe/result: an existing contextual-type provider would make this a
bounded consumer addition. Source inspection instead found the explicit generic
None and no contextual callable API. Alternative missing dependency installation
does not suffice: the loader excludes node_modules and no signature resolver is
present. These observations establish an architecture gap, not a new code defect.

## Forbidden shortcut

Do not map the first generic argument of anything spelled React.FC to props.
A legal local declaration can ignore that parameter:

```ts
declare namespace React {
  type FC<P> = (props: { library: Other }) => void;
}
const run: React.FC<{library: Library}> = ({library}) => library.m();
```

The contextual receiver is Other, not Library. This is a constructible wrong
result for the proposed spelling shortcut, not an observed WRONG on current main.
Checking package.json or an import string is not a replacement for proving the
selected declaration/call signature and handling competing bindings.

## Authority source — settled by owner: A

### A. Source-backed contextual-signature foundation (approved)

First admit only direct variable-initializer arrows/function expressions with an
explicit function-type annotation containing one required parameter and inline
object property types. Example:

```ts
const run: (props: {library: Library}) => void = ({library}) => library.m();
```

No external library semantics, alias/generic substitution, overloaded/union
signatures, call-wrapper context or assertions. Parameter count/slot mapping,
explicit conflicting parameter annotations, type binders, parse recovery,
defaults/rest/optional/duplicate shapes and existing writes must fail closed.
Keep the selected type's declaration position for lexical type provenance;
the receiver binding and write span belong to the implementation parameter.
Reuse the previous inline-property predicate without conflating these positions.

This is useful groundwork but does NOT unlock the four ambient React.FC sites.
Local declaration-backed aliases could follow separately; ambient/imported generic
call signatures still need an explicit resolver contract. Do not market this
foundation as React.FC support.

### B. Compiler-backed contextual authority (larger scope)

Resolve contextual callable/parameter/property types through the project's actual
TypeScript configuration and declarations, with source-span-backed owner identity.
Design dependency/config/declaration custody and invalidation, bounded loading,
provider failure/absence behavior, versioning and integration with cold/subset/
incremental/sidecar construction before implementation. It must not silently
start installs or make default navigation depend on network/compiler availability.

SPEC-GAP: the provider evidence interface, activation policy, supported compiler
versions and dependency-loading limits are not defined by the current request.
Do not invent signatures or a serialized evidence schema before selecting this route.

## Acceptance after the choice

RED on saved merged-base production; positives and each exclusion/edge path;
same-environment cached A↔B owner changes, full/subset/incremental/sidecar parity;
coordinated cache invalidation for changed recovered authority. Full default/MCP
suites with totals, fmt/Clippy, immediate rebuild matrix/quick; no silent baseline
rewrite or human-only full multicorpus. Keep quick FP attribution separate with
a same-environment base control. Measure unchanged archived source and served
callers/callees; source spans and caller-expanded records counted separately.

Constructor-backed this.library and useApp return destructuring remain separate
later increments; neither bypasses this decision or inherits new proof by name.

## Implementation and diagnostic record

Shared js_ts_parameter_receiver_binding consults a direct contextual annotation
only for unannotated non-simple bindings. Existing explicit annotations remain
authoritative; an unsupported explicit annotation cannot fall back to context.
js_ts_contextual_parameter_annotation admits direct variable-declarator arrow/
function-expression values, one required implementation parameter, one required
plain-identifier signature parameter, direct function_type and no generic binders
or parse recovery. Existing inline-object checks restrict the extracted type.
No new serialized field or resolver rung; CPG67/nav36 invalidate old evidence.

On untouched f348779 production, complete RED2pass/1fail: positive contextual
receiver has no type/recovery, exclusions pass. Initial green3/0. Cached
positive↔optional/write-negative and contextual A→B/B→A owner replacements pass.
Round2 adds comments, parse-recovery and further shape exclusions. Two anonymous
wrapper/generator fixtures initially selected zero call sites: inadmissible probes,
repaired with enclosing functions before asserting exclusion. No production fix
was inferred from those probe failures. Evidence: /private/tmp/prism-contextual-props-wQOxSq.
