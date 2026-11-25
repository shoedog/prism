#!/usr/bin/env bash
edition=sample
declare -n destination=edition
IFS= read -r destination || exit 1
printf '%s\n' "$edition"
