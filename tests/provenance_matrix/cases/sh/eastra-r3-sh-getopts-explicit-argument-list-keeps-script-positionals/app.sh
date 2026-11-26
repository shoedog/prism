#!/usr/bin/env bash
while getopts 'e:' option -e preview; do
    printf '%s\n' "$OPTARG"
done
supplied=$1
printf '%s\n' "$supplied"
