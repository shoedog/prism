export {};
class Base { m = (v: number) => v; }
class C extends Base {
  // @ts-ignore
  m(w1: number) { return w1; }
}
export function run(x1: number) { return new C().m(x1); }
