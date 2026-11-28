export {};
class D {
  // @ts-ignore
  m(a1: number) { return a1; }
  // @ts-ignore
  m(b1: number) { return b1; }
}
export function run(x2: number) { return new D().m(x2); }
