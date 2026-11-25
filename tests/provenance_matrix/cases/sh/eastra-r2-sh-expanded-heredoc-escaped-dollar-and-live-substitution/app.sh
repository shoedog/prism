#!/usr/bin/env bash
edition=$1
cat <<TEXT
\$edition
$(printf '%s' "$edition")
TEXT
