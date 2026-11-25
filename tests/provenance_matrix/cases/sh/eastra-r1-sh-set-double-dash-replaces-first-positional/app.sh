#!/usr/bin/env bash
before=$1
set -- "$EDITION" sample
after=$1
printf '%s:%s\n' "$before" "$after"
