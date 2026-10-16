#!/usr/bin/env bash
#
# Soak instrumentation (scripts/soak/soak.py): run prism-mcp with its standard error also appended
# to LOGFILE, one line per line, each prefixed with the UTC time and the pid of the prism-mcp
# process. Standard input and output (the MCP stream) are untouched. Clients that keep no log of an
# MCP server's standard error (Codex, the a2a-bridge) are pointed at this wrapper during a soak;
# Claude Code already logs it.
#
# Usage: prism-mcp-logged.sh LOGFILE BIN [prism-mcp args...]
set -euo pipefail

log="${1:?usage: prism-mcp-logged.sh LOGFILE BIN [args...]}"
bin="${2:?usage: prism-mcp-logged.sh LOGFILE BIN [args...]}"
shift 2
pid=$$
exec "$bin" "$@" 2> >(
  while IFS= read -r line || [ -n "$line" ]; do
    printf '%s %s %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$pid" "$line" >>"$log"
    printf '%s\n' "$line" >&2
  done
)
