if getopts 'm:' option && [ "$option" = m ]; then
  mode=$OPTARG
  printf '%s\n' "$mode"
fi
