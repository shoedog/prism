#!/usr/bin/env bash
edition=preview
read -r -t 0 edition
printf '%s\n' "$edition"
if IFS= read -r edition; then
    printf '%s\n' "$edition"
fi
