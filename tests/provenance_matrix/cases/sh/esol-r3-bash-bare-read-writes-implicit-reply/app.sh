#!/usr/bin/env bash
if IFS= read -r; then
  printf '%s\n' "$REPLY"
fi
