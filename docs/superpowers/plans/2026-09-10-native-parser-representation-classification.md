# Native parser and representation classification

Approved successor to PR303, merged as `17f7053df832cf0bb872aa1915899e14c01dfe91`.
Scope is source/compiler-backed classification and executable fixture requirements,
not a grammar repair, suffix expansion, input-selection change or closure admission.
Primary owns design; representation audit delegated read-only. Review cap: two rounds.

## Method and stop conditions

1. Rebind the complete seven-config public population and unchanged pinned bytes.
2. Enumerate every native ERROR/MISSING node in the two affected files; compare
   with the pinned compiler and minimized TS/TSX positive/negative controls.
3. Separate native loading, parse counts, compiler syntax, module/data identity,
   semantic closure and executable ownership. None implies the next predicate.
4. Document proof requirements and the smallest next repair checkpoint.
5. Run artifact/control verification; report fresh gates separately from inherited
   PR303 measurements. No real source edits, installs, private reads or live models.

Stop before grammar/dependency replacement, source normalization, suppressing parse
errors, changing file selection, admitting JSON/.mts/.cts, or widening ownership.
Keep react-scripts unresolved and every duplicate/write/cache/epoch barrier intact.

## Hypothesis / probe / result log

- H1: the two previously called parser refusals are loaded files with error counts,
  not loader ParseFailed skips. Falsifier: absent native entries or >30% error rate.
  Source loader/owner predicates plus fresh native census distinguish these states.
- H2: invalid source causes the counts. Alternative: pinned grammar misparses valid
  import-type generic calls. Compare exact bytes and compiler syntax, then isolated
  complete Programs with semantic checks. Both real files have zero compiler syntax
  errors; minimized valid TS/TSX have zero compiler diagnostics but native errors.
- H3: await precedence alone causes the issue. Alternative: inline import-type
  generic argument ambiguity. No-await and parentheses still fail; alias, number,
  typeof-identifier, annotation and runtime-import controls parse without errors.
  This identifies a bounded grammar limitation, not a proven grammar-production fix.
- H4: .mts fails because its contents cannot parse. Alternative: suffix dispatch
  rejects before parsing. Forced TypeScript grammar gives zero errors for exact
  vite.config.mts bytes; normal Language::from_path returns None.
- Probe setup failure: first standalone Rust link used incompatible serde feature
  artifacts; no parser executed. Serialize diagnostic enum names as strings in the
  temporary probe; corrected compilation and actual output are the evidence.
- Prism navigation returned the loader callees with a StaleIndex warning. Direct
  current source confirms collect/parse/merge and thresholds; graph is orientation,
  not current semantic authority. Two guessed nonexistent source paths were corrected
  using rg --files; their lookup failures are not evidence of missing implementation.

## Deliverables

Source-linked classification readout, machine receipt, self-contained regression
fixture matrix, and current handoff. No production code or public API changes.
