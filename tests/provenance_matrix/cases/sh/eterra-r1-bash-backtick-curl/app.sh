#!/bin/sh
value=`curl -s https://example.com`
printf '%s\n' "$value"
