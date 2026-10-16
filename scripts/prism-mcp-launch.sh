#!/usr/bin/env bash
#
# Launch the prism-mcp navigation server for a repository, resolving the binary in this order:
#   1. a binary built inside the plugin/repo checkout ($CLAUDE_PLUGIN_ROOT/target/release/prism-mcp)
#   2. a `prism-mcp` on $PATH
# and exit with a clear "build it" message if neither is found.
#
# Used as the command for the bundled MCP server (see .claude-plugin/plugin.json). The repo to
# navigate is passed as the first argument (the plugin wires it to ${CLAUDE_PROJECT_DIR}); any
# further arguments are passed through to prism-mcp (e.g. --cache-dir, --no-cache).
#
# PRISM_MCP_SHARED=1 in the environment opts in to the shared per-repository daemon: the launcher
# adds --shared after --repo, unless the arguments already say --shared or --standalone.
# Unset, empty or 0 changes nothing. Any other value is refused before anything runs, an explicit
# --shared or --standalone included (the variable is checked first), so a typo never decides
# silently which way a client goes.
#
# Usage: prism-mcp-launch.sh <repo-path> [extra prism-mcp args...]
set -euo pipefail

repo="${1:?usage: prism-mcp-launch.sh <repo-path> [args...]}"
shift

case "${PRISM_MCP_SHARED:-}" in
  "" | 0) shared=0 ;;
  1) shared=1 ;;
  *)
    echo "prism-mcp-launch.sh: PRISM_MCP_SHARED must be 1 (use the shared daemon) or 0 or unset (an index of this process's own), not '${PRISM_MCP_SHARED}'" >&2
    exit 2
    ;;
esac
for arg in "$@"; do
  case "$arg" in
    --shared | --standalone) shared=0 ;;
  esac
done
if [ "$shared" = 1 ]; then
  set -- --shared "$@"
fi

root="${CLAUDE_PLUGIN_ROOT:-}"
if [ -n "$root" ] && [ -x "$root/target/release/prism-mcp" ]; then
  bin="$root/target/release/prism-mcp"
elif command -v prism-mcp >/dev/null 2>&1; then
  bin="prism-mcp"
else
  cat >&2 <<'EOF'
prism-mcp not found.

Build it once from a prism checkout:
    cargo build --release --bin prism-mcp --features mcp

Then either install the plugin from that built checkout (the launcher finds
target/release/prism-mcp), or put the binary on your PATH:
    cp target/release/prism-mcp /usr/local/bin/   # or anywhere on PATH

See docs/MCP.md for details.
EOF
  exit 127
fi

exec "$bin" --repo "$repo" "$@"
