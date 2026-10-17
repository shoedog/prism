#!/bin/sh
# A stand-in for prism-mcp in the launcher's tests (tests/mcp/launcher_test.rs): it prints each
# argument it was given on a line of its own, and nothing else.
for arg in "$@"; do
  printf '%s\n' "$arg"
done
