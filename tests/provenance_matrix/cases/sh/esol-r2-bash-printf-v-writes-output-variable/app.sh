#!/usr/bin/env bash
label=fixed
printf -v label '%s' "$DEPLOY_MODE"
printf '%s\n' "$label"
