declare const flag: boolean;
function a(p1: number) { return p1; }
function b(p2: number) { return p2; }
class Base { m(bx: number) { return bx; } }
class Derived extends Base { m(dy: number) { return dy; } }
let g = a;
g = b;
const f = flag ? a : b;
const h1 = (k1: number) => k1;
const h2 = h1;
function viaParam(cb: typeof a, r1: number) { return cb(r1); }
function run(c1: number, u1: number, u2: number, u3: number, u4: number, u5: number) {
  return [a(c1), g(u1), f(u2), (new Derived().m as typeof Base.prototype.m)(u3), h2(u4),
    ((z1: number) => z1)(u5)];
}
