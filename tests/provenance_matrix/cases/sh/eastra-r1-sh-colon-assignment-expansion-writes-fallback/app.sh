#!/usr/bin/env bash
unset edition
: "${edition:=$1}"
printf '%s\n' "$edition"
