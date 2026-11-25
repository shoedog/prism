#!/usr/bin/env bash
edition=preview
assign_edition() { edition=$1; }
show() {
    local edition=local
    assign_edition "$1"
    printf '%s\n' "$edition"
}
show "$1"
printf '%s\n' "$edition"
