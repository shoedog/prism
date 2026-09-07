# Explicit installed-tree profile

Second approved increment, source c723873d plus fixture correction263bf708.
Default20k entries/128MiB hashed input/30s/512MiB V8 old-space/8MiB packet remain.
Explicit installed profile:100k entries,1GiB hashed input,32MiB individual read,
120s,1024MiB V8 old-space,32MiB packet. Caller limits only lower ceilings. The
profile is strict packet v5/producer0.6.0 data and independently selected during
validation. Symlinks remain refused in this increment. No runtime authority.

RED: four profile tests fail on slice1 (unknown positive option, read ceiling
absent, profile selection/validation unavailable). Four pass after implementation.
Expanded broad run:94 passed,2 failed, then96 passed,0 failed,0 skipped after
correcting two enumerated fixture mismatches. An inherited assertion still named
v4. A new packet-cap fixture relabeled a default packet as installed but retained
its128MiB read limit; rejection was correct against the32MiB ceiling. The correction
retains that rejection as a negative, then lowers the fixture limit for the
independent packet-cap positive. No production oracle or baseline was relaxed.

Fresh full default Rust:4017 passed,0 failed,1 ignored. Full MCP:4207 passed,
0 failed,1 ignored. Both include2 doctests. cargo fmt --check and git diff --check
pass. Rust/runtime/CPG/navigation untouched; no Tier-A trigger. Authority fixture
controls are unchanged from the first increment. Raw logs in
/private/tmp/prism-acquisition-next-NoX18k/slice2-{red,node,node-final,cargo,mcp}.log.
Compiler/profile environment and target dir match the streamed-inventory readout.

Two SELF-PASS rounds, NOT INDEPENDENT. Strict string profile checks prevent coercing
array/object values into known profile names. Packet caps apply before root I/O,
including independently selected CLI stdin caps. Real configured fixture Program
production/validation/CLI, wrong-profile replay, forged profile and per-read refusal
are covered. No remaining demonstrated WRONG. Heap is not RSS; larger admission
does not establish closure, runtime class identity, or application build success.
