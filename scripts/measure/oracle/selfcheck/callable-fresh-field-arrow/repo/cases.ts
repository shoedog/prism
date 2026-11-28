class C { m = (v: number) => v; }
class D { k = (u: number) => u; setup() { this.k = (t: number) => t; } }
class E { n: (s: number) => number = (r: number) => r; }
function run(a: number, b: number, c: number) { return [new C().m?.(a), new D().k(b), new E().n(c)]; }
