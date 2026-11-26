#!/bin/bash
# Run a command under a process-tree RSS limit (Linux and macOS).
# Usage: memguard.sh [--status-fd FD] LIMIT_GB command [args...]
set -u

status_fd=
if [ "${1-}" = --status-fd ]; then
    status_fd=${2-}
    if [[ ! "$status_fd" =~ ^[1-9][0-9]{0,8}$ ]] || [ "$status_fd" -lt 3 ]; then
        echo "memguard: invalid status descriptor" >&2
        exit 1
    fi
    if ! { : >&"$status_fd"; } 2>/dev/null; then
        echo "memguard: status descriptor is not open" >&2
        exit 1
    fi
    shift 2
fi

finish() {
    local code=$1 kind=$2 launched=$3 child_exit=$4
    if [ -n "$status_fd" ]; then
        printf '{"kind":"%s","launched":%s,"child_exit":%s}\n' \
            "$kind" "$launched" "$child_exit" >&"$status_fd" || exit 1
    fi
    exit "$code"
}

limit=${1-}
if [ "$#" -lt 2 ] || [[ ! "$limit" =~ ^[0-9]{1,13}$ ]] ||
    (( 10#$limit < 1 || 10#$limit > 8796093022207 )); then
    echo "memguard: expected a positive limit (GiB) and a command" >&2
    finish 1 guard_error false null
fi
limit_rss=$((10#$limit * 1048576))
shift

quiet_helper="$(dirname "$0")/../hostquiet/host_quiet.py"
kill_log=${MEMGUARD_LOG:-}
state_home=${XDG_STATE_HOME:-${HOME:+$HOME/.local/state}}
if [ -z "$kill_log" ] && [ -n "$state_home" ]; then
    kill_log="$state_home/prism/memguard-kills.log"
fi
if ! initial_rss=$(ps -o rss= -p "$$" 2>/dev/null) || [[ ! "$initial_rss" =~ [0-9] ]]; then
    echo "memguard: cannot read process memory; refusing to run unmonitored" >&2
    finish 76 rss_unavailable false null
fi
if [ "${MEMGUARD_IGNORE_QUIET:-0}" != 1 ]; then
    if ! marker=$(python3 "$quiet_helper" status); then
        echo "memguard: cannot determine host-quiet status" >&2
        finish 1 guard_error false null
    fi
    if [[ "$marker" =~ \"stale\"[[:space:]]*:[[:space:]]*false ]]; then
        echo "memguard: a measurement holds the host-quiet marker; wait (python3 $quiet_helper wait) and retry" >&2
        finish 75 host_quiet false null
    fi
    if [ "$marker" != free ] && [[ ! "$marker" =~ \"stale\"[[:space:]]*:[[:space:]]*true ]]; then
        echo "memguard: invalid host-quiet status" >&2
        finish 1 guard_error false null
    fi
fi

observe() {
    local table
    table=$(ps -ax -o pid= -o ppid= -o rss= 2>/dev/null) || return 1
    awk -v root="$child" '
        NF != 3 || $1 !~ /^[0-9]+$/ || $2 !~ /^[0-9]+$/ || $3 !~ /^[0-9]+$/ {
            invalid = 1
        }
        NF == 3 { parent[$1] = $2; rss[$1] = $3 }
        END {
            if (invalid || !(root in rss)) exit 1
            live[root] = 1
            more = 1
            while (more) {
                more = 0
                for (pid in parent)
                    if (!(pid in live) && parent[pid] in live) {
                        live[pid] = 1
                        more = 1
                    }
            }
            for (pid in live) total += rss[pid]
            printf "%.0f", total
            for (pid in live) printf " %s", pid
            printf "\n"
        }' <<< "$table"
}

stop_tree() {
    # Use the dedicated job group for new and reparented descendants, and the
    # observed PID list for descendants that changed their process group.
    kill -TERM -- "-$child" "${observed_pids[@]}" 2>/dev/null || :
    local i
    for i in 1 2 3; do
        kill -0 -- "-$child" "${observed_pids[@]}" 2>/dev/null || break
        sleep 1
    done
    kill -KILL -- "-$child" "${observed_pids[@]}" 2>/dev/null || :
    wait "$child"
    child_exit=$?
}

# Bash job control assigns a separate process group without requiring setsid.
set -m
(
    if [ -n "$status_fd" ]; then
        eval "exec ${status_fd}>&-"
        unset MEMGUARD_LOG XDG_STATE_HOME HOST_QUIET_DIR MEMGUARD_IGNORE_QUIET
        exec /usr/bin/env -u PWD -u OLDPWD -u SHLVL -u _ -- "$@"
    fi
    exec "$@"
) <&0 &
child=$!
observed_pids=("$child")
while kill -0 "$child" 2>/dev/null; do
    if ! sample=$(observe); then
        if ! kill -0 "$child" 2>/dev/null; then
            break
        fi
        echo "memguard: process memory became unavailable" >&2
        stop_tree
        finish 76 rss_unavailable true "$child_exit"
    fi
    read -r -a observation <<< "$sample"
    rss=${observation[0]}
    observed_pids=("${observation[@]:1}")
    if (( rss > limit_rss )); then
        line="$(date '+%F %T') memguard: killed '$*' at $((rss / 1048576)) GB (limit $limit GB)"
        printf '%s\n' "$line" >&2
        if [ -z "$kill_log" ] || ! mkdir -p "$(dirname "$kill_log")" ||
            ! printf '%s\n' "$line" >> "$kill_log"; then
            echo "memguard: could not write the kill log" >&2
        fi
        stop_tree
        finish 137 rss_limit true "$child_exit"
    fi
    sleep 1
done

wait "$child"
child_exit=$?
finish "$child_exit" child true "$child_exit"
