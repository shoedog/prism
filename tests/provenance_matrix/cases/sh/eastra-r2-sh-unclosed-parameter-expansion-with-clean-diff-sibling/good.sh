#!/usr/bin/env bash
IFS= read -r edition || exit
printf '%s\n' "$edition"
