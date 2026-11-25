#!/usr/bin/env bash
read() {
  printf -v "$1" '%s' 'preview'
}

read label
builtin read -r submitted
printf '%s:%s\n' "$label" "$submitted"
