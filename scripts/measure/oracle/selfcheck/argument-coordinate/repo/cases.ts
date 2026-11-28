export {};
function target(p1: number, p2: number) { return p1 + p2; }
function cb2(p3: number, cb: (p3: number) => void) { return p3; }
export function caller(a1: number, b1: number) {
  return [target(a1, b1), cb2(a1, () => {})];
}
