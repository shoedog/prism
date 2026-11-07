#!/bin/bash
# Cloud corpus-measurement runner. usage: cloud-measure-run.sh HEAD_SHA BASE_SHA RESULT_BRANCH RECIPE ASSET...
#   ASSET = <release asset id>:<file name>:<sha256>; each is downloaded over REST from shoedog/prism, checked, and
#   .tar.gz files are extracted into /tmp/corpora. Builds release binaries for HEAD then BASE in one target dir
#   (deps reused), copies them to /tmp/bin/prism-{head,base}, then runs:
#     bash RECIPE /tmp/bin/prism-base /tmp/bin/prism-head /tmp/corpora /tmp/measure-results/out
#   and pushes /tmp/measure-results (status, machine facts, logs, out/) to an orphan RESULT_BRANCH.
#   The controller reads the pushed files; the session's prose is not the record.
set -u
HEAD=$1 BASE=$2 BR=$3 RECIPE=$4; shift 4
REPO=$(git rev-parse --show-toplevel); cd "$REPO"
R=/tmp/measure-results; rm -rf $R; mkdir -p $R/logs $R/out /tmp/corpora /tmp/bin
st() { echo "$(date -u +%FT%TZ) $*" >> $R/status.txt; }
push() {
  cd "$REPO" && git checkout -q --orphan "$BR" && git rm -rq --cached . && mkdir -p measure-results \
    && cp -r $R/. measure-results/ && git add -f measure-results && git commit -qm "measure results $HEAD vs $BASE" \
    && git push -q origin "$BR" && st "pushed $BR" || st "FAILED push"
}
fin() { st "exit $1"; push; echo DONE >> $R/status.txt; exit "$1"; }
cp "$RECIPE" $R/recipe.sh
{ nproc; free -g; df -h /; rustc -V; cargo -V; python3 -V; } > $R/machine.txt 2>&1
for a in "$@"; do
  id=${a%%:*}; rest=${a#*:}; name=${rest%%:*}; sum=${rest#*:}
  gh api -H 'Accept: application/octet-stream' "repos/shoedog/prism/releases/assets/$id" > "/tmp/corpora/$name" 2>> $R/logs/assets.log \
    || { st "FAILED download $name"; fin 1; }
  echo "$sum  /tmp/corpora/$name" | sha256sum -c - >> $R/logs/assets.log 2>&1 || { st "FAILED sha256 $name"; fin 1; }
  case "$name" in *.tar.gz) tar xzf "/tmp/corpora/$name" -C /tmp/corpora || { st "FAILED extract $name"; fin 1; };; esac
  st "asset ok $name"
done
git fetch -q origin || { st "FAILED fetch"; fin 1; }
for arm in head base; do
  sha=$HEAD; [ $arm = base ] && sha=$BASE
  git checkout -q --detach "$sha" || { st "FAILED checkout $arm $sha"; fin 1; }
  s=$(date +%s); cargo build --release > $R/logs/build-$arm.log 2>&1; ec=$?
  st "build $arm $(git rev-parse HEAD) exit=$ec secs=$(( $(date +%s)-s ))"
  [ $ec -eq 0 ] || fin 1
  cp target/release/prism /tmp/bin/prism-$arm
done
(while sleep 30; do echo "$(date +%s) $(free -m | awk '/Mem:/{print $3}')"; done) > $R/mem-samples.txt &
SAMP=$!
s=$(date +%s); bash "$RECIPE" /tmp/bin/prism-base /tmp/bin/prism-head /tmp/corpora $R/out > $R/logs/recipe.log 2>&1; ec=$?
kill $SAMP 2>/dev/null; wait $SAMP 2>/dev/null
st "recipe exit=$ec secs=$(( $(date +%s)-s )) peak mem MiB $(awk '{print $2}' $R/mem-samples.txt | sort -n | tail -1)"
for f in $R/logs/*.log; do [ "$(wc -c < "$f")" -gt 409600 ] && { { head -c 204800 "$f"; echo; echo '[... truncated ...]'; tail -c 204800 "$f"; } > "$f.t"; mv "$f.t" "$f"; }; done
fin $ec
