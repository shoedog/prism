# Slice 3 candidate source manifest

- Base: `f1df12e5cf6b1fc113ed6549967586b12e71651f`, tree `f74d6c8e11eb0a771a233af972ace30cff33e0cf`.
- Production equivalence control: `7fc89c98..f1df12e5` has no non-`docs/` delta; the authenticated RED patch transferred cleanly.
- Working candidate: uncommitted implementation; this file binds the exact source hashes below.
- Frozen source patch: `/private/tmp/prism-post316-orchestration/slice3/implementation/frozen-candidate-source.patch`, SHA-256 `b6d7300416fc826324bcfd061ad042841196d9deac29191509f4b7631ea00bad`.
- Cache versions: CPG `96` (was `95`); navigation call-edge cache `53` unchanged.
- Review cap: two rounds. No publication, merge, rebaseline, or live adoption is authorized.

## Changed files
src/ast_required_parameter_tests.rs
src/cpg/optional_parameter_tests.rs
src/cpg_cache.rs
src/parameter_slots.rs

## SHA-256
11020db81866c0e7b9ce55bf16052d7f11fa933788f92de5bcbed0347455fb4a  src/parameter_slots.rs
c8ac2991a358ace0ef303d7a060b8282663a1c3ae99466ba95124902ceb69559  src/cpg_cache.rs
1af98b4c678c532a87033ac2b4cf548b5ff9b7d13005c3b407c8fc1056f9fb24  src/ast_required_parameter_tests.rs
b19ca3e68be5af5c7518f91f4753edbb2b7c36029c28c6770a35ccaca9026538  src/cpg/optional_parameter_tests.rs

## Focused evidence
- Base RED patch/receipt: `/private/tmp/prism-post316-orchestration/slice3/red-prep/`.
- Candidate focused seven-control PASS log: `/private/tmp/prism-post316-orchestration/slice3/implementation/logs/focused-green-final.log` (SHA-256 `e6727d126690036f544e77f1df2329c1bffcbfe7ebabcd982abc3a920a6f8de1`).
