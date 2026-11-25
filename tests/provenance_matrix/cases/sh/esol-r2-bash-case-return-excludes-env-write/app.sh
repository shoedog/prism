#!/usr/bin/env bash
choose() {
    local label=fixed
    case "$1" in
        preview) label=$DEPLOY_MODE; return 0 ;;
    esac
    printf '%s\n' "$label"
}
