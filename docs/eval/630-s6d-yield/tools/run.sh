#!/bin/bash
# usage: run.sh <bin> <corpus-dir-rel> <tag> [mute-list] ; writes out/<corpus>.<tag>.edges.sorted
S=/tmp/claude-0/-home-user-prism/817300ea-683f-51d6-b28c-8f12e9d2d739/scratchpad
bin=$1; repo=$2; tag=$3; mute=$4
name=$(echo $repo | tr / _)
if [ -n "$mute" ]; then export PRISM_6D_MUTE=$mute; else unset PRISM_6D_MUTE; fi
RAYON_NUM_THREADS=4 $S/bin/$bin nav --no-cache dfg-stats --edges --repo $S/corpora/$repo 2>$S/out/$name.$tag.err | LC_ALL=C sort > $S/out/$name.$tag.sorted
echo "$name $tag $(wc -l < $S/out/$name.$tag.sorted)"
