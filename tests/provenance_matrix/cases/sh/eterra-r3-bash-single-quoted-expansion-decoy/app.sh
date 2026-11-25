#!/usr/bin/env bash
text='$(curl https://example.org)'
printf '%s\n' "$text"
