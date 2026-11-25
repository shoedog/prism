#!/bin/sh
set -eu
here=$(CDPATH='' cd -- "$(dirname -- "$0")" && pwd)
exec python3 -B "$here/check_truth.py" "$@"
