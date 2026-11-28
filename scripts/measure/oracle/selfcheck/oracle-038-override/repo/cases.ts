class Base { m(x: number) { return x; } }
class Derived extends Base { m(y: number) { return y; } n(z: number) { return super.m(z); } }
class Priv { #q(g: number) { return g; } t(f: number) { return this.#q(f); } }
function viaBase(b: Base, p: number) { return b.m(p); }
function viaDerived(q: number) { return new Derived().m(q); }
