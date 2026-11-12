#!/bin/bash
# usage: measure.sh corpus
c=$1; R=/tmp/corp/m/$c
export RAYON_NUM_THREADS=4
B=/tmp/base-target/release/prism; H=/home/user/prism/target/release/prism
$B nav --no-cache dfg-stats --repo $R --edges > /tmp/out/$c.base.edges 2> /tmp/out/$c.base.err
PRISM_C_PRUNE_TRACE=1 $H nav --no-cache dfg-stats --repo $R --edges > /tmp/out/$c.head.edges 2> /tmp/out/$c.head.trace
$B nav --no-cache dfg-stats --repo $R > /tmp/out/$c.base.stats 2>/dev/null
$H nav --no-cache dfg-stats --repo $R > /tmp/out/$c.head.stats 2>/dev/null
$B nav --no-cache call-stats --repo $R > /tmp/out/$c.base.call 2>/dev/null
$H nav --no-cache call-stats --repo $R > /tmp/out/$c.head.call 2>/dev/null
echo $c done
