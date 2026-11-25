#!/usr/bin/env bash
IFS= read -r -d '' body
status=$?
printf '%s:%s\n' "$body" "$status"
