export {};
class GB { ['m'] = (v: number) => v; }
class GC extends GB {
  // @ts-ignore
  m(w1: number) { return w1; }
}
class GS { static ['m'] = 1; }
class GD extends GS { m(w2: number) { return w2; } }
export function g1(x14: number, x15: number) { return [new GC().m(x14), new GD().m(x15)]; }
