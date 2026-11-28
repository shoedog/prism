export {};
declare function use(...v: unknown[]): void;
class Box { m = 0; }
export function s1() {
  const a = new Box();
  const b = new Box();
  a.m = 1;
  use(a.m, b.m);
  return [a, b];
}
