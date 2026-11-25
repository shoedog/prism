shopt -u lastpipe
mode=fixed
printf '%s\n' "$1" | IFS= read -r mode
printf '%s\n' "$mode"
