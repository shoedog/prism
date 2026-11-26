export {};
namespace N1 { export function f(o1: number) { return o1; } }
// @ts-ignore
N1 = { f: function (r1: number) { return r1; } };
namespace K1 { export function k(k1: number) { return k1; } }
export function run1(x1: number, c1: number) { return [N1.f(x1), K1.k(c1)]; }
