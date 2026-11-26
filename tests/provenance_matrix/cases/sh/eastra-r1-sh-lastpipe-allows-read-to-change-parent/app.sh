#!/usr/bin/env bash
set +m
shopt -s lastpipe
edition=sample
printf '%s\n' "$1" | read -r edition
printf '%s\n' "$edition"
