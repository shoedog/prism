function f(x: number) { return x; }
function g(y: number) { return y; }
function run(a: number) { return g(f(a)) + f(a + a) + f((a as number)); }
