#!/usr/bin/env bash
value="$1"
while :; do value="$MODE"; break; done
printf '%s\n' "$value"
