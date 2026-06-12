# Plan Review — S3 Call-Resolution Precision (codex gpt-5.5 xhigh, dual-lens)

**Process:** plan-review-codex workflow (exec-readiness + coverage lenses both on codex xhigh while the bridge claude model-override defect is open; config `examples/a2a-bridge.slicing-plan-review-codex.toml`). **Triage (owner fix-vs-defer policy): all 5 BLOCKER + 8 MAJOR + 4 MINOR findings FIXED in plan rev 2 (same commit as this record); none deferred.**

BLOCKER — Task 8 / Task 4: P6-lite receiver recovery is build-only, but incremental CPG uses build_direct_subset, so full and incremental builds would produce different call edges. Fix by populating CallSite.receiver_type in build_direct_subset too, or force a full CallGraph rebuild for this cache version.

BLOCKER — Task 8 Step 3: The snippet calls nonexistent ParsedFile::function_calls_recovery_node. Fix by calling parsed.receiver_type_in_fn(&func_node, q) directly, or define the helper before use.

BLOCKER — Task 10 Step 3: The Step 5b snippet references undefined callee_node; current code gets parameters from FunctionInfo.param_names. Fix by applying the Python self/cls skip to the existing FunctionInfo.param_names path, or first add a real callee-node lookup.

BLOCKER — Task 11 Step 5: Deleting resolve_callees_qualified as written breaks resolve_callees, which currently delegates to it, plus direct tests. Fix by reimplementing resolve_callees as the recall-biased name/static resolver before deletion, keeping a compatibility wrapper, and migrating tests.

BLOCKER — Task 11 Step 3: The plan renames the nav adapter to resolve_call_site_nav but existing callers/tests still use resolve_callees_nav. Fix every resolve_callees_nav call site, including queries.rs, module_graph.rs, and navigation tests, or keep an old-signature wrapper.

MAJOR — Task 8: receiver_type_in_fn scans the whole function without call-site ordering, so rebinding after a call can incorrectly cancel valid recovery. Fix by passing call_line and only considering bindings/rebindings before the call.

MAJOR — Task 8: Constructor-local recovery is internally inconsistent. The test expects ResolutionKind::ConstructorLocal, while the plan later says v1 collapses both recovery paths to TypedParam. Disagreement resolution: Coverage is right that this is a spec-coverage gap; Executability is right that the immediate plan/test contradiction must be fixed. Either carry recovery_kind through CallSite/ResolvedCallee, or update the test and explicitly document ConstructorLocal as deferred.

MAJOR — Tasks 11-12: Drop classification is under-decomposed. Counting qualifier.is_some() && resolve_call_site(site).is_empty() conflates multi-owner collision drops, import-qualified external calls, P6 external receiver drops, and ordinary unresolved calls. Fix with a DropReason/classification API and use it for warnings and call-stats.

MAJOR — Task 12: dropped_multi_owner is materially wrong as specified because it includes non-multi-owner unresolved receiver calls. Fix by splitting telemetry into fields such as dropped_multi_owner, dropped_external_receiver, dropped_import_external, and unresolved_receiver.

MAJOR — Task 11: Navigation helper return types discard resolution metadata. direct_callers, direct_callees, callers/callees traversal, and module graph need ResolutionConfidence and ResolutionKind to compute scores and Reason::Resolution. Fix by returning a struct carrying target, line, qualifier, confidence, kind, and score where needed.

MAJOR — Task 11: Collision warnings are specified for callers but not ego, while the spec requires callers/ego drop visibility. Fix ego_graph to emit warnings when Call/Return edges are requested around a seed and same-name receiver sites were dropped.

MAJOR — Task 11: Module graph max-score aggregation is only partially specified. Current reason storage cannot carry per-call score or resolution kind. Fix by replacing tuple reasons with a struct carrying callee, line, qualifier, score, and kind, then aggregate max score per file pair.

MAJOR — Task 9: The traversal-helper rewrite misses callers_of()/callers_of_in_file(..., None). The proposed change only resolves sites inside the target_file branch, leaving no-target caller traversal name-indexed and collision-prone. Fix by resolving each CallSite before enqueueing/filtering in both branches.

MAJOR — Task 13 Step 3: Empty caller matrix fixtures must use a concrete callers key. Fix expected.toml with [expect], callers = [], exact = true; omitted callers fails the loader, and empty [[expect.callers]] is invalid because it lacks file/line.

MINOR — Task 2: JS/TS owner extraction misses class-field arrow methods such as class Foo { handler = () => {} }. Fix owner extraction for field/property-definition arrow-function parents.

MINOR — Task 4 file table: method_traits is listed, but Task 4 only adds methods, method_owners, and receiver_vars. Fix by removing method_traits from the plan or defining the field and maintenance rules.

MINOR — Task 5/file table: src/resolution.rs is advertised as including classify_site, but no task defines it. Fix by either specifying the API or removing the promise and relying on the DropReason classifier above.

MINOR — Task 10 Step 1: The CodePropertyGraph::build(&files, None) snippet does not match the current constructor. Fix the test snippet by copying the actual setup from existing cpg_test.rs.

Verdict: not executable as-is; fix the incremental P6 path, nonexistent helpers/undefined callee_node, resolver/nav API migration, traversal no-target branch, metadata plumbing, and drop-classification gaps before building.