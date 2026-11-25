#!/usr/bin/env bash
editions=("preview")
mapfile -t -n 1 -O 1 editions
(( ${#editions[@]} == 2 )) || exit
prefix=${editions[0]}
entered=${editions[1]}
printf '%s\n' "$prefix" "$entered"
