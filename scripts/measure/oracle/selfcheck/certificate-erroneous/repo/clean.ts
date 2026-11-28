export {};
class CB { m = (v1: number) => v1; }
class CC extends CB { n(w1: number) { return w1; } }
class CD { m(a: number): number; m(d1: number) { return d1; } }
class CB3 { g(g1: number) { return g1; } }
class CC3 extends CB3 { h(h1: number) { return super.g(h1); } }
namespace CN { export function k(k1: number) { return k1; } }
class CP { #p(p1: number) { return p1; } run(c6: number) { return this.#p(c6); } }
export function run(c1: number, c2: number, c3: number, c4: number) {
  return [new CC().n(c1), new CD().m(c2), CN.k(c3), new CC3().h(c4)];
}
