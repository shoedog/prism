#!/usr/bin/env bash
select selection in compact expanded; do
  printf '%s:%s\n' "$selection" "$REPLY"
  break
done
