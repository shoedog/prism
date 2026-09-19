# P2 design correction: syntax inventory, then native readiness

**Draft for split planning review, round1/cap2. Implementation remains stopped.** Controller chose a meaningful split rather than ratifying a larger monolith. This supersedes the architect's conditional1500/2600 budget recommendation; neither it nor the worker's1550/2800 proposal is authorized. Original P2 accepted spec `e6e1bb3e8a928943149e59cfa29598c1e15142d10468c32fe5704a70132c1f71` remains the full-contract reference, amended only by the staged deliverables here.

## Same artifact, preserved work

Keep `/private/tmp/prism-post317-parameter-census` and the complete `snapshots/budget-stop/` custody. Snapshot Rust SHA `795448776132ba5124c46e43cf5d3bcf96a1e1155b9941d6d6e09c5f46098cca`, compiler observer SHA `1f4b856b2059bd4089cf480648e4919b33c0f04c404ffe330775579f7f9802ee`, native characterization SHA `79fd244a2554b1f3bfac7ab1bc1d5081e5b0d8638df033ce76eea7f3e6f0142f`. Preserve the known compilation errors and all earlier RED/characterization logs as historical; do not claim a passing stopped candidate. No restart, new source baseline or discarded native scaffolding.

P2a edits the existing active runner by extracting only syntax/integrity functions; the complete old file remains in the frozen snapshot. Retain `Manifest/Member/Cli`, manifest validation/discovery, compiler invocation, syntax collection/category/span helpers, syntax joins, canonical IDs and syntax-only reconciliation. Remove `SlotBinding`, CPG/callgraph construction/imports, owner/entry/call/candidate/Step5b code and native-characterization target from the **active compile**. These remain parked in the snapshot for selective P2b reuse. Update the external Cargo bin list only as necessary; no new dependency, lockfile resolution or Prism source change. Inactive snapshot bytes are not counted toward active slice source, but every active helper/script/test counts once. Do not hide executed code in an excluded evidence directory.

## Stage contracts

|Stage|Useful independent result|Forbidden interpretation|
|---|---|---|
|P2a|Complete authenticated414-file syntax inventory; independent native-grammar/TypeScript syntax observations, exact joins, per-cohort/stratum frequencies and refusal coverage|Native support, stable owner, slots, entry, binding, constructible demand, flow, compiler semantics or closure|
|P2b|On that exact inventory, native owner/slot/entry/call-target/Step5b readiness and stage losses|Syntax expansion, runtime/production authority or unavailable private/installed-root parity|

P2a can answer how often the excluded forms occur and where observer disagreement/recovery prevents exact classification. That is a useful prioritization input, not evidence of reachable flow demand. P2b supplies the separate native-readiness question before any feature recommendation.

No public corpus run until P2a frozen core review approves its observers/tests. The existing implementation core-review count remains0; the controller must explicitly dispatch the revised P2a cap2 after split planning acceptance. P2b gets its own declared bounded review cap only when separately dispatched. One environmental retry per gate remains; inadmissible setup results never count as behavior.

## Scope/custody retained

Both stages use native Prism `14e86083c2c754ed412d440742bd5763be388e42` or verified production-equivalent accepted closeout and compiler5.9.3 SHA `3ae902c92cc44dace175c0e69e13a4b0899f6983c6121d76b9ab8dd5795e7675`. The exact source manifest stays `f8ebbdd79ca01e5cdb675616b913f1fbb549d45378ab1047db84844a3bc8e696`:414 files/5,036,151 bytes,5 declarations,62 test-path members. Validate all member hashes and exact population before parsing. No network/install, Program/typecheck, dependency closure, private inputs or shared-repository edits. P3 remains parked.
