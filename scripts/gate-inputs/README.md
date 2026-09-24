# Gate inputs

`node scripts/gate-inputs/acquire.mjs` acquires the nine pinned public artifacts, verifies their byte authorities
before decompression, and installs the three derived trees below `PRISM_GATE_INPUTS_ROOT` (or
`$XDG_DATA_HOME/prism/gate-inputs`, then `~/.local/share/prism/gate-inputs`). It is supported on macOS only.

Each install path is derived from `pins.json`: `<root>/<input>/<tree_sha256>`. Installs are immutable. Re-running
`acquire` verifies an existing tree and does not download it again; a corrupt install is refused and must be removed
by the operator before re-acquiring it.

Use `node scripts/gate-inputs/acquire.mjs verify` to verify all installed inputs without downloading. Use
`node scripts/gate-inputs/acquire.mjs env` to verify then print POSIX-safe exports for the three consumers; evaluate
those exports only in a shell you control.

The tool removes only a stage directory it created after a failed invocation. It intentionally does not scavenge
other `.stage-*` directories: a stage left by a crashed process is inert. Review it and remove it manually when safe.
Roots below temporary macOS locations and roots containing control characters are refused.
