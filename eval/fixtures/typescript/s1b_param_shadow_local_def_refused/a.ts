export function run(f: () => number) {
  return f();
}
export function holder() {
  const f = () => 1;
  return f;
}
