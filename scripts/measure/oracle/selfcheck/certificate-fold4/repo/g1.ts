export {};
class GB { ['m'] = (v: number) => v; }
class GC extends GB {
  // @ts-ignore
  m(w1: number) { return w1; }
}
export function g1(x14: number) { return new GC().m(x14); }
