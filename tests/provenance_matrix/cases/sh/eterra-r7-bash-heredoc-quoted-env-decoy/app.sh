#!/usr/bin/env bash
value=$(cat <<'END'
$MODE
END
)
printf '%s\n' "$value"
