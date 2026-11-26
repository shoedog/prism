#!/usr/bin/env bash
read -r value < <(printenv MODE)
printf "%s\n" "$value"
