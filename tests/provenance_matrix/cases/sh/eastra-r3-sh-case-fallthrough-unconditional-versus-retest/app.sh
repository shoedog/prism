#!/usr/bin/env bash
label=preview
case "$1" in
    keep) label=$2 ;;&
    reset) label=preview ;;
esac
printf '%s\n' "$label"
label=preview
case "$1" in
    keep) label=$2 ;&
    reset) label=preview ;;
esac
printf '%s\n' "$label"
