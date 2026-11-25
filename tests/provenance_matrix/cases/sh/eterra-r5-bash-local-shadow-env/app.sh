#!/usr/bin/env bash
MODE=outer
run() {
  local MODE=fixed
  printf "%s\n" "$MODE"
}
