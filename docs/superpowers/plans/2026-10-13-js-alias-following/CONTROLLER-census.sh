#!/bin/sh
# CORPUS_ROOT=/absolute/corpus OUT_DIR=$HOME/prism-evidence/js-alias/step0b/private-run sh "$0"
# Static parser/checker + frozen Prism only. Does not execute corpus packages.
set -eu
umask 077
: "${CORPUS_ROOT:?Set CORPUS_ROOT to the corpus root}"
: "${OUT_DIR:?Set OUT_DIR to a new directory beneath the Step 0b evidence root}"
exec python3 /Users/wesleyjinks/prism-evidence/js-alias/step0b/controller.py
