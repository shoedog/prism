export {};
class D {
  m(a1: number) { return a1; }
  m(b1: number) { return b1; }
}
export function run(x2: number) { return new D().m(x2); }
