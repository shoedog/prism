function left(value: number) { return "left" as const; }
function right(value: number) { return "right" as const; }
function caller(useLeft: boolean, actual: number) {
  const selected: typeof left | typeof right = useLeft ? left : right;
  return selected(actual);
}
