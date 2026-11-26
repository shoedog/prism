#!/usr/bin/env bash
argument=$1
[[ $argument =~ ^edition=(.+)$ ]]
status=$?
(( status == 0 )) || exit
value=${BASH_REMATCH[1]}
printf '%s %d\n' "$value" "$status"
