#!/bin/bash
cd /tmp/claude-0/-home-user-prism/817300ea-683f-51d6-b28c-8f12e9d2d739/scratchpad
c=protobuf-x
./run.sh prism-main $c main; ./run.sh prism-6d $c default; cmp out/$c.main.sorted out/$c.default.sorted && echo IDENTICAL
rm -f out/$c.log; PRISM_6D_LOG=$PWD/out/$c.log RAYON_NUM_THREADS=4 bin/prism-6d nav --no-cache dfg-stats --repo corpora/$c >/dev/null 2>&1
bin/defs6d corpora/$c > out/$c.defs.tsv
while IFS=$'\t' read tag m; do ./run.sh prism-6d $c $tag "$m"; done < configs.txt
echo DONE
