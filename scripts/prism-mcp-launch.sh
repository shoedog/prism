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
# Usage: prism-mcp-launch.sh <repo-path> [extra prism-mcp args...]
set -euo pipefail

repo="${1:?usage: prism-mcp-launch.sh <repo-path> [args...]}"
shift

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
