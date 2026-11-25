if IFS= read -r -u 3 mode <<< "$1" 3<&0; then
  printf '%s\n' "$mode"
fi
